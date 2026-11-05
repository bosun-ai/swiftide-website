---
title: "Your first pipeline"
description: "Index a directory of Rust code into an embedded LanceDB table with local FastEmbed embeddings, then ask it one question and get an answer that cites files."
sidebar:
  order: 4
---

This page indexes one directory of Rust code into an embedded LanceDB table and answers a question about it, naming the files the answer came from. You run the [ask-your-codebase](/showcases/ask-your-codebase/) showcase and read the parts that make it work. Everything except the LLM (large language model) runs on your machine. Tree-sitter chunks the code, FastEmbed embeds it on the CPU (central processing unit), and LanceDB stores the vectors in a local directory.

## Showcase run

The showcase lives in `examples/ask-your-codebase/` on swiftide master; it is not in the v{{swiftide_version}} release tag. Clone the swiftide repository. To run exactly the code this site shows, check out the commit it is pinned to:

```sh
git clone https://github.com/bosun-ai/swiftide.git
cd swiftide
git checkout {{swiftide_sha}}  # optional: the commit this site documents
export OPENAI_API_KEY=sk-...
cargo run -p ask-your-codebase -- index swiftide-agents
cargo run -p ask-your-codebase -- ask --path swiftide-agents "How does an agent decide to stop?"
```

## Dependencies

To build the same pipelines in your own crate, add the showcase's dependencies:

```sh
cargo add swiftide@{{swiftide_version}} --no-default-features --features tree-sitter,fastembed,lancedb,redb,openai
cargo add fastembed@5 --no-default-features
cargo add tokio --features full
cargo add anyhow async-trait
```

Each integration has its own [feature flag](/reference/feature-flags/). The direct `fastembed` dependency picks a model other than the default. Only the showcase's own chunker needs `async-trait`.

## LLM client

::snippet{src="@swiftide/examples/ask-your-codebase/src/llm.rs" anchor="llm" title="examples/ask-your-codebase/src/llm.rs"}

The client is swiftide's `OpenAI` integration pointed at a base URL with `api_base` and `api_key` :unreleased. Indexing sends many prompts at once, so `LanguageModelWithBackOff` wraps it and retries transient errors, such as rate limits, with exponential backoff. A wrong key is a permanent error and fails at once. `max_elapsed_time_sec` in `BackoffConfiguration` caps how long one prompt is [retried](/models/errors-backoff-and-usage/); the showcase allows 60 seconds while indexing and 30 while asking.

## Embedder and storage

::snippet{src="@swiftide/examples/ask-your-codebase/src/embedding.rs" anchor="embedding" title="examples/ask-your-codebase/src/embedding.rs"}

The embedder runs `jina-embeddings-v2-base-code`, a model trained on code, with up to 2048 tokens per text. Its vectors have 768 dimensions, kept in `DIMENSIONS`. `BATCH_SIZE` (32) is the batch size FastEmbed uses inside one embedding call. Keep it equal to the batch size of the pipeline step that calls the embedder.

::snippet{src="@swiftide/examples/ask-your-codebase/src/codebase.rs" anchor="storage" title="examples/ask-your-codebase/src/codebase.rs"}

`Redb` is the node cache, a single embedded file. `LanceDB` is the storage: `uri` is a local directory, and `vector_size` must equal the embedder's dimensions. `EmbeddedField::Combined` stores one vector per chunk, computed from the chunk and its metadata. `with_metadata(PATH_FIELD)` keeps the file path in its own column, so the answer can cite it. Both live in `<path>/.ask-your-codebase/`, inside the indexed directory.

## Indexing pipeline

::snippet{src="@swiftide/examples/ask-your-codebase/src/index.rs" anchor="pipeline" title="examples/ask-your-codebase/src/index.rs" mark="1-3,10,14,16-22,24-27"}

The indexing pipeline is a stream of nodes. Nothing runs until `run()` is awaited. `run` returns the pipeline statistics :unreleased; the showcase reports `nodes_stored` as the chunk count. The marked lines are the core:

- The loader, `FileLoader`, reads the files under the root with the given extensions. It skips hidden files and what `.gitignore` and `.ignore` files exclude. `.gitignore` only applies inside a git repository. Each file becomes one node.
- `filter_cached` drops nodes the node cache has seen before. The cache key is a hash of the path and the content, so an edited file counts as new.
- `then_chunk` runs the chunker. `ChunkByLanguage` is a small showcase type that picks a tree-sitter `ChunkCode` per file extension. It splits along syntax boundaries into chunks of 10 to 4096 bytes.
- `then` runs a transformer on each chunk. `MetadataQACode` asks the LLM, with a custom prompt, for eight questions the chunk answers and stores them as metadata. `concurrency` caps the prompts in flight.
- `then_in_batch` runs a batch transformer. `Embed` sends 32 chunks per call to FastEmbed.
- `then_store_with` hands each chunk to the storage, here LanceDB, through the `Persist` trait.

The other lines are showcase extras that drop tests, record the relative path and count for the report. Leave them out in your own crate, and replace the showcase helpers:

- `FileLoader::new(codebase.root()).with_extensions(&languages.extensions())` becomes `FileLoader::new("./src").with_extensions(&["rs"])`.
- `ChunkByLanguage::try_new(languages, CHUNK_SIZE)?` becomes `ChunkCode::try_for_language_and_chunk_size("rust", 10..4096)?`.
- `codebase.node_cache()?` and `codebase.vector_store()?` are the builders from the storage excerpt.
- `llm` is the client from the LLM client excerpt. Without `prompt_template`, `MetadataQACode` uses its default prompt.

## Query pipeline

::snippet{src="@swiftide/examples/ask-your-codebase/src/query.rs" anchor="query_pipeline" title="examples/ask-your-codebase/src/query.rs"}

`GenerateSubquestions` rewrites the question into several related ones, to widen what the search can match. The query transformer `Embed` turns them into one vector with the same local model that embedded the chunks. The search strategy, `SimilaritySingleEmbedding`, runs one vector search with that embedding and returns the `top_k` closest chunks. Its default is 10; the showcase passes 20. `then_retrieve` uses the same `LanceDB` store as the retriever. The answer generator, `answers::Simple`, answers from the retrieved chunks; its document template renders each chunk under its file path. [Answering](/querying/answering/) covers other answer generators.

`query` consumes the pipeline and returns the answered query. To ask more than once, call `query_mut` or `query_all`.

## Output

This output is from the showcase's `OUTPUT.md`. The chunk questions came from claude-haiku-5-5 and the answer from claude-sonnet-5-5, both through an OpenAI-compatible gateway. The embeddings came from FastEmbed on the CPU.

The first run indexes everything:

```text
Indexing Rust files in swiftide-agents ...
Indexed 34 chunks from 14 new or changed files (14 files found, 0 unchanged) in 91.9s
```

A second run finds every file in the node cache and sends no prompts:

```text
Indexing Rust files in swiftide-agents ...
Indexed 0 chunks from 0 new or changed files (14 files found, 14 unchanged) in 0.6s
```

Asking "How does an agent decide to stop?" gives, trimmed:

```text
# How an agent decides to stop

An agent stops when something calls `Agent::stop` (or `stop_with_error`), which moves the state to `State::Stopped(StopReason)`. The main loop in `run_agent` checks that state after each step. `Agent::stop` is a no-op if the agent is already stopped (`src/agent.rs`).

## The stop mechanism

- `Agent::stop(reason)` fires the `OnStop` hooks with the reason, then sets `self.state = State::Stopped(reason)`. It only takes effect at the end of the current loop iteration (`src/agent.rs`).
- `Agent::stop_with_error(&AgentError)` fires `OnStop` with `StopReason::Error` and the error, then sets `Stopped(StopReason::Error)` (`src/agent.rs`).
- You can inspect the result with `stop_reason()`, `is_stopped()` and `state()` (`src/agent.rs`, `src/state.rs`).
...
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

## Variations

### OpenAI embeddings

The LanceDB example embeds with OpenAI's `text-embedding-3-small` instead, so its `vector_size` is 1536:

::snippet{src="@swiftide/examples/lancedb.rs" anchor="setup"}

### Qdrant with the default FastEmbed model

`FastEmbed::builder()` without a model uses the default model, with 384 dimensions. This pipeline stores into Qdrant. It needs the `qdrant` feature flag and a Qdrant server at `QDRANT_URL`, which defaults to `http://localhost:6334`. See [Qdrant](/integrations/qdrant/):

::snippet{src="@swiftide/examples/fastembed.rs" anchor="pipeline"}

For hybrid search and reranking, see the [docs-qa](/showcases/docs-qa/) showcase.

## Gotchas

- Use the same embedding model for indexing and querying, and set `vector_size` to its dimensions. After you change the model, delete `<path>/.ask-your-codebase/` in the indexed directory and index again.
- The first run downloads `jina-embeddings-v2-base-code`, about 600MB, into `.fastembed_cache/` in the current directory.
- When a file changes, its new chunks are added, but the chunks of the old version stay in LanceDB.
- Files enter the node cache only after `run()` finishes without an error. One failed chunk ends the run, and every file of that run is prompted and embedded again next time. The same holds for a run you interrupt.
- A file that produces no chunks is never cached, so it is loaded again on every run.

## Reference

- [`indexing::Pipeline`](docsrs:swiftide/indexing/struct.Pipeline.html)
- [`NodeCache`](docsrs:swiftide/traits/trait.NodeCache.html)
- [`Persist`](docsrs:swiftide/traits/trait.Persist.html)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)
- [`BackoffConfiguration`](docsrs:swiftide/indexing/struct.BackoffConfiguration.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)
- [`FileLoader`](docsrs:swiftide/indexing/loaders/struct.FileLoader.html)
- [`ChunkCode`](docsrs:swiftide/indexing/transformers/struct.ChunkCode.html)
- [`MetadataQACode`](docsrs:swiftide/indexing/transformers/struct.MetadataQACode.html)
- [`transformers::Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html)
- [`EmbeddedField`](docsrs:swiftide/indexing/enum.EmbeddedField.html)
- [`LanceDB`](docsrs:swiftide/integrations/lancedb/struct.LanceDB.html)
- [`FastEmbed`](docsrs:swiftide/integrations/fastembed/struct.FastEmbed.html)
- [`Redb`](docsrs:swiftide/integrations/redb/struct.Redb.html)
- [`query::Pipeline`](docsrs:swiftide/query/struct.Pipeline.html)
- [`SimilaritySingleEmbedding`](docsrs:swiftide/query/search_strategies/struct.SimilaritySingleEmbedding.html)
- [`query_transformers::Embed`](docsrs:swiftide/query/query_transformers/struct.Embed.html)
- [`GenerateSubquestions`](docsrs:swiftide/query/query_transformers/struct.GenerateSubquestions.html)
- [`answers::Simple`](docsrs:swiftide/query/answers/struct.Simple.html)

## Next

[Your first task](/start/your-first-task/)
