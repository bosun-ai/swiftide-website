---
title: "Docs Q&A with evaluation"
description: "A retrieval-augmented generation app over markdown docs with a custom loader, dense and sparse embeddings in Qdrant, hybrid search, local reranking, a Ragas dataset export, and claim-based scores from a model judge."
sidebar:
  order: 4
identLint:
  ignore: [fastembed_cache, to_string]
---

This tour builds `docs-qa`, a retrieval-augmented generation (RAG) app with three commands: `index`, `ask` and `eval`. It reads markdown files or an `llms-full.txt`, and every answer lists the pages and sections it came from. The `eval` command prints a score table that you can rerun after each retrieval change.

The crate lives in the swiftide repository under `examples/docs-qa`. It is not in the {{swiftide_version}} tag: until the showcases reach master, they live on the `docs/showcases` branch. Run it with `cargo run -p docs-qa` at the repository root.

## Prerequisites

- Rust 1.97.1, which the swiftide workspace pins in `rust-toolchain.toml`.
- Qdrant in Docker. Swiftide talks to Qdrant over gRPC, so expose port 6334 as well as 6333. `QDRANT_URL` (default `http://localhost:6334`) and `QDRANT_API_KEY` point the client elsewhere.
- `OPENAI_API_KEY` for any OpenAI-compatible endpoint. `OPENAI_BASE_URL` defaults to `https://api.openai.com/v1` and `OPENAI_MODEL` to `gpt-5.4-mini`. Embeddings and reranking run locally; only model calls go to this endpoint.
- About 2.3GB of disk for the FastEmbed models, downloaded from Hugging Face into `.fastembed_cache` on first use. Set `FASTEMBED_CACHE_DIR` to share one cache.
- Optional: `LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY` and `LANGFUSE_URL` for `--langfuse`.

The crate turns off default features and enables the feature flags `openai`, `fastembed`, `qdrant` and `langfuse`. The excerpts also use `async-openai` directly and derive `schemars::JsonSchema`:

```sh
cargo add swiftide@{{swiftide_version}} --no-default-features --features openai,fastembed,qdrant,langfuse
cargo add async-openai@0.41 --no-default-features
cargo add schemars@1 --features derive
cargo add reqwest@0.13 --no-default-features --features rustls
```

Swiftide 0.33 enables rustls without selecting a crypto provider, and the default `OPENAI_BASE_URL` is https. docs-qa gets a provider through its own `reqwest` dependency with the `rustls` feature. Without it, add that dependency or install a rustls crypto provider in `main` before you build any client, as the other showcases do ([Crypto provider](/reference/feature-flags/#crypto-provider)).

## Architecture

`index` writes chunks into one Qdrant collection. `ask` and `eval` build the same query pipeline over that collection; `eval` only adds an evaluator.

![Indexing and query pipelines](/diagrams/diagram-pipeline.svg)

## Custom loader

::snippet{src="@swiftide/examples/docs-qa/src/loader.rs" anchor="loader" title="examples/docs-qa/src/loader.rs"}

A loader is the start of an indexing pipeline. `Loader` has one associated type, `Output`, and `into_stream` returns an `IndexingStream` of nodes. Here every page becomes one `TextNode`, with its origin as `node.path` and its title under the `Page` metadata key. A futures stream becomes an `IndexingStream` with `.boxed().into()`.

`into_stream_boxed` matters only when you use the loader as a `Box<dyn Loader>`. The default implementation panics, so implement it on the concrete type.

`DocsSource` decides how a page is read. `MarkdownDir` walks a directory with `FileLoader` and respects `.gitignore`. `MarkdownFile` reads one file. `LlmsFullFile` and `LlmsFullUrl` split an `llms-full.txt` into one page per top-level heading.

## Hybrid indexing

::snippet{src="@swiftide/examples/docs-qa/src/index.rs" anchor="pipeline" title="examples/docs-qa/src/index.rs"}

`ChunkMarkdown::from_chunk_range(120..1500)` splits each page into chunks of at most 1,499 characters, because the range end is exclusive. It drops chunks whose trimmed text is under 120 bytes, which are mostly badges and stray headings. `MetadataTitle` stores a title under `Title` and `MetadataKeywords` stores keywords under `Keywords`, so indexing makes two large language model (LLM) calls per chunk. Two batch transformers follow: `Embed` writes the dense vector and `SparseEmbed` the sparse one, both from local FastEmbed models.

The metadata transformers use their own Tera templates:

::snippet{src="@swiftide/examples/docs-qa/src/index.rs" anchor="metadata_prompts" title="examples/docs-qa/src/index.rs"}

The default prompts show the expected format inside a code fence, and models tend to copy the fence. These templates ask for plain text and read the page title from `node.metadata.Page`. A code-only chunk is titled after the API it shows, because the reranker scores the title too.

### Store

::snippet{src="@swiftide/examples/docs-qa/src/store.rs" anchor="store" title="examples/docs-qa/src/store.rs"}

The collection holds one dense and one sparse vector per chunk. Both use `EmbeddedField::Combined`, which embeds the chunk together with its metadata, so the title and keywords reach the keyword side of the search. The vector size is 384 because the dense model is BGE small, the default of `FastEmbed::try_default`. The sparse model comes from `FastEmbed::try_default_sparse`, which loads SPLADE.

## Hybrid search and reranking

::snippet{src="@swiftide/examples/docs-qa/src/query.rs" anchor="hybrid" title="examples/docs-qa/src/query.rs"}

`HybridSearch` defaults to 10 for both `top_n` and `top_k`. `with_top_n` and `with_top_k` take `&mut self` and return `&mut Self`, so `.to_owned()` turns the result back into a value for the pipeline.

::snippet{src="@swiftide/examples/docs-qa/src/query.rs" anchor="query_pipeline" title="examples/docs-qa/src/query.rs"}

Qdrant's hybrid retriever needs both a dense and a sparse embedding of the question and fails without either. It runs a dense and a sparse search, each limited to `top_n`, fuses them with reciprocal rank fusion (RRF), and returns `top_k` documents: 20 candidates here.

The reranker is a response transformer. It runs between retrieval and the answer generator:

::snippet{src="@swiftide/examples/docs-qa/src/query.rs" anchor="answerer" title="examples/docs-qa/src/query.rs"}

`Rerank` scores each candidate against the original question with a local cross-encoder and keeps `top_k` of them (default 10). With a `document_template`, each document is rendered with Tera first, with `metadata` and `content` available. `answers::Simple` renders every kept document with its own `document_template` and joins them with `---` lines into `{{ documents }}`. Its `prompt_template` takes a `Prompt`, hence the `.into()`.

::snippet{src="@swiftide/examples/docs-qa/src/query.rs" anchor="answer_templates" title="examples/docs-qa/src/query.rs"}

### Qdrant content workaround

In 0.33, Qdrant retrieval turns the stored `content` payload into a string with `to_string` on Qdrant's own `Value` type, which formats strings with `Debug`. Every chunk then comes back wrapped in quotes, with escaped newlines. The example wraps the store in `CleanQdrant` (`src/retrieval.rs`), which implements `Retrieve` for `HybridSearch`, calls Qdrant, and restores the original text. The reranker, the LLM and the evaluator then see clean markdown.

## Evaluation

::snippet{src="@swiftide/examples/docs-qa/src/eval.rs" anchor="eval" title="examples/docs-qa/src/eval.rs"}

Retrieval-Augmented Generation Assessment (Ragas) is a Python toolkit. Swiftide's `Ragas` evaluator only exports data for it and computes no scores.

`Ragas::from_prepared_questions` takes anything that converts into an `EvaluationDataSet`. The example implements that conversion for `&QuestionSet`, which is why the snippet passes `&questions`. `qa.evaluated_pipeline` builds the same query pipeline and attaches the evaluator with `evaluate_with`. `query_all` runs every question, and `to_json` exports the dataset with `question`, `answer`, `contexts` and `ground_truth`.

The evaluator records contexts at retrieval, before any response transformer. `ragas.json` therefore holds all 20 candidates per question, not the 10 the reranker kept.

### LLM judge

To print scores without leaving Rust, a judge splits the ground truth and the answer into claims and checks each one:

::snippet{src="@swiftide/examples/docs-qa/src/judge.rs" anchor="judge_schema" title="examples/docs-qa/src/judge.rs"}

::snippet{src="@swiftide/examples/docs-qa/src/judge.rs" anchor="judge" title="examples/docs-qa/src/judge.rs"}

`structured_prompt` sends the JSON schema of `Judgement` with the prompt and parses the reply into it. The scores are plain ratios over the claims: faithful (answer claims the reranked context supports), answer recall and context recall. `LanguageModelWithBackOff` does not wrap `StructuredPrompt`, so the judge calls the plain `OpenAI` client and retries transient errors itself, up to five attempts.

## Model client

::snippet{src="@swiftide/examples/docs-qa/src/models.rs" anchor="llm" title="examples/docs-qa/src/models.rs"}

Answer and metadata calls go through `LanguageModelWithBackOff`, as in [Ask your codebase](/showcases/ask-your-codebase/#model-client). Here it retries with a longer backoff than the defaults (1 s start, 60 s cap).

## Tracing

::snippet{src="@swiftide/examples/docs-qa/src/telemetry.rs" anchor="langfuse" title="examples/docs-qa/src/telemetry.rs"}

The Langfuse layer is set up the same way as in the [release notes agent](/showcases/release-notes-agent/#tracing). Langfuse data is emitted at debug level, so its layer gets its own debug-level filter.

## Commands

```sh
git clone --branch docs/showcases https://github.com/bosun-ai/swiftide && cd swiftide
docker run -d --name docs-qa-qdrant -p 6333:6333 -p 6334:6334 qdrant/qdrant
export OPENAI_API_KEY=sk-...

cargo run -p docs-qa -- index --recreate \
  README.md CONTRIBUTING.md AGENTS.md swiftide-agents-contrib/README.md
cargo run -p docs-qa -- ask "How do I give an agent tools to read and edit files?"
cargo run -p docs-qa -- eval --out examples/docs-qa/ragas.json
```

| Flag | Where | Default | What it does |
| --- | --- | --- | --- |
| `--model` | global | `gpt-5.4-mini` (`OPENAI_MODEL`) | Answers questions; default for the other models |
| `--responses-api` | global | off | Responses API instead of chat completions |
| `--collection` | global | `docs-qa` | Qdrant collection |
| `--langfuse` | global | off | Send traces to Langfuse (`LANGFUSE_URL`) |
| `--recreate` | `index` | off | Drop the collection first |
| `--metadata-model` | `index` | `--model` | Writes titles and keywords; pick a cheaper model |
| `--questions` | `eval` | bundled `questions.yaml` | Questions with ground truths |
| `--out` | `eval` | `ragas.json` | Where the dataset goes |
| `--judge-model` | `eval` | `--model` | Grades the answers |

## Output

The captured run used an OpenAI-compatible gateway in front of AWS Bedrock. `claude-haiku-5-5` wrote titles and keywords, and `claude-sonnet-5-5` wrote the answers and acted as judge. FastEmbed ran BGE small en v1.5, SPLADE++ and BGE reranker base locally.

```text
Indexed 43 chunks into `docs-qa` in 71.7s
```

The answer below describes the agents-contrib file tools :unreleased, which are on swiftide master but not in {{swiftide_version}}. The CLI prints the answer followed by its sources.

```text
Enable the `agents-contrib-file-tools` feature on `swiftide`, then register `ReadFile`, `ExactReplace`, and `ApplyPatch` on your agent with `add_tool`.
...
**Requirement:** the executor must provide a POSIX-like shell with `cat`, `mkdir`, and `rm` commands.

Sources:
  [1] Swiftide › Agent Tools and Hooks: Enabling Contrib File, Planning, and Context Features  (README.md)
  [2] swiftide-agents-contrib › read_file tool: arguments, defaults, and offset/limit behavior  (swiftide-agents-contrib/README.md)
  [3] swiftide-agents-contrib › File Tools: Line-Numbered Reads and Exact-Match Edits  (swiftide-agents-contrib/README.md)
  ...
  [10] Swiftide › Swiftide capabilities for agents, typed task graphs, pipelines, and tracing  (README.md)
```

```text
#   question                                              ctx faithful   answer  context
                                                                         recall   recall
1   Which crate features and environment variable do I …   10     1.00     1.00     1.00
2   What does the agents-contrib-context feature provid…   10     1.00     1.00     1.00
3   What are the default limits of the read_file tool?     10     1.00     1.00     1.00
4   Which storage and retrieval backends does Swiftide …   10     1.00     1.00     1.00
5   What should I run before opening a pull request?       10     1.00     1.00     1.00
6   What is a task in Swiftide?                            10     1.00     1.00     1.00
7   When does ContextCompression summarize the conversa…   10     1.00     1.00     1.00
8   Which commit message convention does the project fo…   10     1.00     1.00     1.00
9   How do I add a custom tool to an agent with the too…   10     1.00     1.00     1.00
    mean                                                          1.00     1.00     1.00

RAGAS dataset written to examples/docs-qa/ragas.json
```

`ctx` is the number of chunks the LLM saw after reranking.

## Gotchas

- A chunk that is mostly code ranks poorly when the reranker scores only its text. Render page and chunk title into the reranker's `document_template`, and title code-only chunks after the API they show.
- The scores come from one judge over nine questions. Treat them as a regression signal for this corpus, not as a benchmark.
- `--responses-api` failed against the gateway used for the captured run, which returns a field the `async-openai` client rejects.
- Ragas exports rows in hash-map order, so `eval` sorts them back into question order before writing.

## Concept pages

- Indexing: [Loading](/indexing/loading/), [Chunking](/indexing/chunking/), [Transformers and metadata](/indexing/transformers-and-metadata/), [Embedding](/indexing/embedding/)
- Storage: [Storage](/indexing/storage/), [Qdrant](/integrations/qdrant/), [FastEmbed](/integrations/fastembed/)
- Querying: [Retrieval and search strategies](/querying/retrieval-and-search-strategies/), [Reranking and response transformers](/querying/reranking-and-response-transformers/), [Answering](/querying/answering/), [Evaluation](/querying/evaluation/)
- Models and tracing: [Structured output](/models/structured-output/), [Langfuse](/observability/langfuse/)
- Recipe: [Hybrid search and reranking](/cookbook/hybrid-search-and-reranking/)

## Reference

- [`Loader`](docsrs:swiftide/traits/trait.Loader.html)
- [`FileLoader`](docsrs:swiftide/indexing/loaders/struct.FileLoader.html)
- [`TextNode`](docsrs:swiftide/indexing/type.TextNode.html)
- [`EmbeddedField`](docsrs:swiftide/indexing/enum.EmbeddedField.html)
- [`IndexingStream`](docsrs:swiftide/indexing/struct.IndexingStream.html)
- [`ChunkMarkdown`](docsrs:swiftide/indexing/transformers/struct.ChunkMarkdown.html)
- [`MetadataTitle`](docsrs:swiftide/indexing/transformers/struct.MetadataTitle.html)
- [`MetadataKeywords`](docsrs:swiftide/indexing/transformers/struct.MetadataKeywords.html)
- [`Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html) and [`SparseEmbed`](docsrs:swiftide/indexing/transformers/struct.SparseEmbed.html) (indexing)
- [`Qdrant`](docsrs:swiftide/integrations/qdrant/struct.Qdrant.html)
- [`FastEmbed`](docsrs:swiftide/integrations/fastembed/struct.FastEmbed.html)
- [`Rerank`](docsrs:swiftide/integrations/fastembed/struct.Rerank.html)
- [`HybridSearch`](docsrs:swiftide/query/search_strategies/struct.HybridSearch.html)
- [`Embed`](docsrs:swiftide/query/query_transformers/struct.Embed.html) and [`SparseEmbed`](docsrs:swiftide/query/query_transformers/struct.SparseEmbed.html) (query)
- [`answers::Simple`](docsrs:swiftide/query/answers/struct.Simple.html)
- [`Ragas`](docsrs:swiftide/query/evaluators/ragas/struct.Ragas.html) and [`EvaluationDataSet`](docsrs:swiftide/query/evaluators/ragas/struct.EvaluationDataSet.html)
- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)
- [`Retrieve`](docsrs:swiftide/traits/trait.Retrieve.html)
- [`StructuredPrompt`](docsrs:swiftide/traits/trait.StructuredPrompt.html)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)
- [`BackoffConfiguration`](docsrs:swiftide/indexing/struct.BackoffConfiguration.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)
- [`LangfuseLayer`](docsrs:swiftide/langfuse/struct.LangfuseLayer.html)

## Next

[Agent control recipes](/cookbook/agent-control/)
