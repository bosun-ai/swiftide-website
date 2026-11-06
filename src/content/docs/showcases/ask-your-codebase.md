---
title: "Ask your codebase"
description: "A CLI that indexes a Rust codebase locally with tree-sitter, FastEmbed and LanceDB, and answers questions about it citing the files it used."
sidebar:
  order: 1
identLint:
  ignore: [JinaEmbeddingsV2BaseCode, test_helpers]
---

This showcase is a command-line tool with two commands. `index` turns a codebase into searchable chunks, and `ask` answers a question from those chunks and lists the files it used. Everything except the large language model (LLM) runs on your machine. You need no Docker and no database server.

## Prerequisites

- Rust 1.97.1, which the swiftide workspace pins in `rust-toolchain.toml`.
- `OPENAI_API_KEY` for any OpenAI-compatible endpoint. `OPENAI_BASE_URL` is optional and defaults to `https://api.openai.com/v1`. `OPENAI_MODEL` is optional and defaults to `gpt-5.4-mini`. An empty `OPENAI_API_KEY` or `OPENAI_BASE_URL` counts as unset. An empty `OPENAI_MODEL` does not: it sets the model to an empty name, so unset it instead.
- About 600MB of disk for the code embedding model. FastEmbed downloads it into `.fastembed_cache/` in the current directory on first use.

The crate turns off swiftide's default features and enables the `tree-sitter`, `fastembed`, `lancedb`, `redb` and `openai` feature flags. To build the same thing in your own crate:

```sh
cargo add swiftide@{{swiftide_version}} --no-default-features --features tree-sitter,fastembed,lancedb,redb,openai
cargo add fastembed@5 tree-sitter@0.26 --no-default-features
```

The crate also depends on `fastembed` to pick the embedding model and `tree-sitter` to find unit tests, each at the version swiftide uses. TLS needs no setup: swiftide's HTTP clients bring their own rustls crypto provider ([TLS](/integrations/overview/#tls)).

## Architecture

The tool runs two pipelines that share one local embedder. The indexing pipeline loads files, chunks them, describes each chunk with the LLM, embeds the result and stores it. The query pipeline embeds your question with the same model, retrieves the closest chunks and asks the LLM to answer from them.

![Indexing and query pipelines](/diagrams/diagram-pipeline.svg)

The index lives next to the code, in `<path>/.ask-your-codebase/`. It holds `cache.redb`, the node cache, and `lancedb/`, the vector table.

## Indexing pipeline

::snippet{src="@swiftide/examples/ask-your-codebase/src/index.rs" anchor="pipeline" title="examples/ask-your-codebase/src/index.rs"}

`FileLoader` respects `.gitignore`. `filter_cached` skips files whose path and content are unchanged since the last successful run. `then_in_batch` sends 32 chunks to the embedder per call. The two `count` closures count files for the summary line. `run` returns the `PipelineStats` of the run, and its `nodes_stored` is the number of chunks in the summary.

The order of the test steps matters. The node cache keys on the file path and its content. If unit tests were removed after `filter_cached`, the content would differ on every run and the cache would never hit.

### Test filtering

::snippet{src="@swiftide/examples/ask-your-codebase/src/test_code.rs" anchor="skip_tests" title="examples/ask-your-codebase/src/test_code.rs"}

`Pipeline::filter` takes a closure over `&Result<TextNode>` and keeps the result when the closure returns true. The predicate keeps errors on purpose, so a failed load still surfaces. The second function is an ordinary transformer: any `Fn(TextNode) -> Result<TextNode>` works, and this one rewrites `chunk` for `.rs` files.

### Node cache and storage

::snippet{src="@swiftide/examples/ask-your-codebase/src/codebase.rs" anchor="storage" title="examples/ask-your-codebase/src/codebase.rs"}

`Redb` is the node cache, stored in one table of a local database file. `LanceDB` is the storage: one vector column over `EmbeddedField::Combined`, sized at 768 for the embedding model, plus a `path` metadata column. Since 0.33, the pipeline marks files in the node cache only after the whole run finishes without an error. If any file fails, or you interrupt the run, nothing from that run is cached. The next run prompts and embeds every file again, not only the one that failed.

### Chunking per language

::snippet{src="@swiftide/examples/ask-your-codebase/src/languages.rs" anchor="chunker" title="examples/ask-your-codebase/src/languages.rs"}

`ChunkCode` is built for one language. This chunker implements `ChunkerTransformer`, holds one `ChunkCode` per language and picks one by file extension through `SupportedLanguages::file_extensions`. The chunker aims for 10 to 4096 bytes, which keeps most functions in one piece. Chunks of 10 bytes or less are dropped. When tree-sitter finds a syntax error, the whole file becomes one chunk, whatever its size.

### Question metadata

::snippet{src="@swiftide/examples/ask-your-codebase/src/index.rs" anchor="questions_prompt" title="examples/ask-your-codebase/src/index.rs"}

The default `MetadataQACode` prompt asks generic questions. This template asks for questions about behavior, phrased the way a developer asks them. It is a Tera template: `MetadataQACode` fills in `questions` with the count and `node` with the chunk. The answer is stored under the metadata key `Questions and Answers (code)`. It is embedded with the chunk, but LanceDB has no column for it.

::snippet{src="@swiftide/examples/ask-your-codebase/src/index.rs" anchor="path_metadata" title="examples/ask-your-codebase/src/index.rs"}

A closure that returns `Result<TextNode>` is a transformer too. This one writes the relative path into the metadata, where the `path` column picks it up.

### Local embeddings

::snippet{src="@swiftide/examples/ask-your-codebase/src/embedding.rs" anchor="embedding" title="examples/ask-your-codebase/src/embedding.rs"}

`JinaEmbeddingsV2BaseCode` is a code embedding model with 768 dimensions. It runs on the CPU and needs no key. Both pipelines call `local_model`, because query vectors and chunk vectors must come from the same model. After you change the model, delete the index and build it again.

## Query pipeline

::snippet{src="@swiftide/examples/ask-your-codebase/src/query.rs" anchor="query_pipeline" title="examples/ask-your-codebase/src/query.rs"}

`SimilaritySingleEmbedding` returns 10 chunks by default, and `with_top_k` sets the `--top-k` value instead. `with_top_k` takes the strategy by value and returns it, so it chains onto `default()`. `default()` is only implemented for the unfiltered strategy, so the filter type is inferred as `()`, meaning no filter. `GenerateSubquestions` rewrites the question into several related ones. The `Embed` query transformer embeds the rewritten text, the question and its subquestions together, as one vector with the local model. `query` consumes the pipeline and returns the answered query.

### Prompts and document template

::snippet{src="@swiftide/examples/ask-your-codebase/src/query.rs" anchor="prompts" title="examples/ask-your-codebase/src/query.rs"}

The subquestion template receives `question` and `num_questions`, which defaults to 5. The answer template receives `question` and `documents`. With `document_template`, each retrieved document is rendered on its own, with `content` and `metadata.path` available, and the results are joined with `---` lines. That is how the answer sees which file each chunk came from.

### Sources

::snippet{src="@swiftide/examples/ask-your-codebase/src/query.rs" anchor="sources" title="examples/ask-your-codebase/src/query.rs"}

`documents` returns the retrieved documents in relevance order. The `path` column comes back as a `path` metadata entry. The tool keeps each path once and prints the list under the answer.

## Model client

::snippet{src="@swiftide/examples/ask-your-codebase/src/llm.rs" anchor="llm" title="examples/ask-your-codebase/src/llm.rs"}

The client is always swiftide's `OpenAI` integration. `api_base` points it at `OPENAI_BASE_URL` and `api_key` sets the key, without building an `async-openai` client yourself. `use_responses_api` switches from Chat Completions to the Responses API when you pass `--responses-api`. `LanguageModelWithBackOff` retries rate limits and transient errors with exponential backoff. `index` keeps retrying a failing prompt for up to 60 seconds, because it sends many prompts at once and expects rate limits. `ask` stops retrying after 30 seconds.

## Commands

The crate lives in `examples/ask-your-codebase` of the swiftide repository. Clone the repository and run from its root:

```sh
git clone https://github.com/bosun-ai/swiftide && cd swiftide
export OPENAI_API_KEY=sk-...
cargo run -p ask-your-codebase -- index swiftide-agents
cargo run -p ask-your-codebase -- ask --path swiftide-agents "How does an agent decide to stop?"
```

`index` and `ask` are separate runs, so each can use its own model. Through a gateway that serves Anthropic models, a small model writes the indexing prompts and a stronger one writes the answers:

```sh
export OPENAI_BASE_URL=http://127.0.0.1:8080/v1  # an OpenAI-compatible gateway that serves these models
cargo run -p ask-your-codebase -- index swiftide-agents --model claude-haiku-5-5
cargo run -p ask-your-codebase -- ask --path swiftide-agents --model claude-sonnet-5-5 "How does an agent decide to stop?"
```

| Flag | Applies to | Default | Description |
| --- | --- | --- | --- |
| `--model <name>` | both | `gpt-5.4-mini` | Model for chunk metadata or answers. Also read from `OPENAI_MODEL`. |
| `--responses-api` | both | off | Send prompts to the Responses API. |
| `--lang <languages>` | `index` | `rust` | Comma-separated tree-sitter languages, such as `rust,python`. |
| `--concurrency <n>` | `index` | `16` | Prompts sent at once while indexing. |
| `--path <dir>` | `ask` | `.` | Root of the codebase, as given to `index`. |
| `--top-k <n>` | `ask` | `20` | Number of chunks to retrieve. |

By default the tool logs only errors and turns off all logging from `swiftide_integrations`, which hides errors from LanceDB, FastEmbed, Redb, tree-sitter and OpenAI. Each retried attempt is logged at WARN, so it stays hidden too. Set `RUST_LOG=info` or `RUST_LOG=debug` to see what the pipelines do.

## Output

The captured run indexed the `swiftide-agents` crate with a debug build. It went through an OpenAI-compatible gateway that forwards to Anthropic models on AWS Bedrock. `claude-haiku-5-5` wrote the per-chunk questions, `claude-sonnet-5-5` wrote the answers, and FastEmbed ran `jina-embeddings-v2-base-code` on the CPU.

The first run indexes everything. The second finds nothing changed, so the node cache skips every file and no prompt is sent:

```text
$ ./target/debug/ask-your-codebase index swiftide-agents --model claude-haiku-5-5
Indexing Rust files in swiftide-agents ...
Indexed 34 chunks from 14 new or changed files (14 files found, 0 unchanged) in 91.9s

$ ./target/debug/ask-your-codebase index swiftide-agents --model claude-haiku-5-5
Indexing Rust files in swiftide-agents ...
Indexed 0 chunks from 0 new or changed files (14 files found, 14 unchanged) in 0.6s
```

The start of one answer, and its sources:

```text
$ ./target/debug/ask-your-codebase ask --model claude-sonnet-5-5 --path swiftide-agents "How does an agent decide to stop?"
# How an agent decides to stop

An agent stops when something calls `Agent::stop` (or `stop_with_error`), which moves the state to `State::Stopped(StopReason)`. The main loop in `run_agent` checks that state after each step. `Agent::stop` is a no-op if the agent is already stopped (`src/agent.rs`).

[...]

## Triggers, by `StopReason` variant (`src/state.rs`)

| Variant | When it applies |
|---|---|
| `RequestedByTool(ToolCall, Option<Value>)` | A tool returned `ToolOutput::Stop`. |
| `FeedbackRequired { tool_call, payload }` | A tool returned `ToolOutput::FeedbackRequired`. |
| `AgentFailed(Option<Value>)` | A tool returned `ToolOutput::AgentFailed`. |
| `ToolCallsOverLimit(ToolCall)` | The same tool call failed too many times. |
| `Error` | `step` returned an `Err`. |
| `NoNewMessages` | The completion loop ended without another reason. |
| `Other(String)` | Anything convertible from `AsRef<str>`, e.g. `agent.stop("done")`. |

[...]

Sources:
  - src/agent.rs
  - src/tools/control.rs
  - src/lib.rs
  - src/state.rs
  - src/errors.rs
  - src/task_agent.rs
  - src/hooks.rs
  - src/tasks.rs
  - src/default_context.rs
```

The answers in [`OUTPUT.md`](https://github.com/bosun-ai/swiftide/blob/{{swiftide_sha}}/examples/ask-your-codebase/OUTPUT.md) were checked against the source of `swiftide-agents`. Where a needed chunk was not among the 20 retrieved, the answer marked its claim as an inference.

## Limitations

- When a file changes, its new chunks are added, but the chunks of the old version stay in LanceDB. Delete `.ask-your-codebase/` to start over.
- Files under a `tests` directory, or named `tests`, `test_utils` or `test_helpers`, are dropped in every language. Unit tests inside a file are only removed from `.rs` files.
- Retrieval is a single vector search. Raise `--top-k` for broad questions.
- A file that produces no chunks is never marked in the node cache, so every run loads it again. It costs no LLM calls.

## Concept pages

- Loading and caching: [Loading](/indexing/loading/), [Caching](/indexing/caching/)
- Chunking: [Code-aware indexing](/indexing/code-aware-indexing/), [Chunking](/indexing/chunking/), [Tree-sitter](/integrations/tree-sitter/)
- Metadata and embeddings: [Transformers and metadata](/indexing/transformers-and-metadata/), [Embedding](/indexing/embedding/), [FastEmbed](/integrations/fastembed/)
- Storage: [Storage](/indexing/storage/), [LanceDB](/integrations/lancedb/)
- Querying: [Transforming queries](/querying/transforming-queries/), [Retrieval and search strategies](/querying/retrieval-and-search-strategies/), [Answering](/querying/answering/)
- Models: [OpenAI-compatible endpoints](/integrations/openai-compatible/), [Errors, backoff and usage](/models/errors-backoff-and-usage/)

## Reference

- [`indexing::Pipeline`](docsrs:swiftide/indexing/struct.Pipeline.html)
- [`FileLoader`](docsrs:swiftide/indexing/loaders/struct.FileLoader.html)
- [`ChunkCode`](docsrs:swiftide/indexing/transformers/struct.ChunkCode.html)
- [`ChunkerTransformer`](docsrs:swiftide/traits/trait.ChunkerTransformer.html)
- [`SupportedLanguages`](docsrs:swiftide/integrations/treesitter/enum.SupportedLanguages.html)
- [`MetadataQACode`](docsrs:swiftide/indexing/transformers/struct.MetadataQACode.html)
- [`Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html)
- [`EmbeddedField`](docsrs:swiftide/indexing/enum.EmbeddedField.html)
- [`Redb`](docsrs:swiftide/integrations/redb/struct.Redb.html)
- [`LanceDB`](docsrs:swiftide/integrations/lancedb/struct.LanceDB.html)
- [`FastEmbed`](docsrs:swiftide/integrations/fastembed/struct.FastEmbed.html)
- [`query::Pipeline`](docsrs:swiftide/query/struct.Pipeline.html)
- [`SimilaritySingleEmbedding`](docsrs:swiftide/query/search_strategies/struct.SimilaritySingleEmbedding.html)
- [`GenerateSubquestions`](docsrs:swiftide/query/query_transformers/struct.GenerateSubquestions.html)
- [`query_transformers::Embed`](docsrs:swiftide/query/query_transformers/struct.Embed.html)
- [`answers::Simple`](docsrs:swiftide/query/answers/struct.Simple.html)
- [`query::Query`](docsrs:swiftide/query/struct.Query.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)
- [`BackoffConfiguration`](docsrs:swiftide/indexing/struct.BackoffConfiguration.html)

## Next

[Release notes agent](/showcases/release-notes-agent/)
