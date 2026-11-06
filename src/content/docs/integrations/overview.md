---
title: "Integrations"
description: "Lists every Swiftide integration with the feature flag that enables it, the traits it implements, its main types, examples and guide."
sidebar:
  order: 1
---

Each Swiftide integration sits behind its own feature flag. The matrix below shows which flag to add, what the integration plugs into, and where its guide is. An integration is a model provider, storage backend, node cache, loader, transformer, agent tool, token estimator or observability layer that lives behind a feature flag of the `swiftide` crate.

## Feature flags

Name the feature flags of the integrations you use when you add `swiftide`:

```sh
cargo add swiftide@{{swiftide_version}} --features qdrant,fastembed
```

Feature flags are additive. Each one compiles its integration and the flags it depends on. Without its flag, an integration's module does not exist: `swiftide::integrations::qdrant` needs `qdrant`. Only `macros` is on by default.

Most integrations live under `swiftide::integrations`, which re-exports the `swiftide-integrations` crate with the same feature names. A few sit elsewhere. MCP (Model Context Protocol) tools are in `swiftide::agents::tools::mcp`, Langfuse is under `swiftide::langfuse`, and the tree-sitter transformers are also re-exported under `swiftide::indexing::transformers`.

Some flags turn on other flags:

- `groq`, `gemini`, `ollama`, `open-router` and `dashscope` each turn on `openai`. Their clients build on the OpenAI client.
- `mcp` and `langfuse` each turn on `swiftide-agents`, which turns on `swiftide-tasks`. You do not need to add `swiftide-agents` next to them.

Some flags compile native code or large SDKs (software development kits) and make builds slower:

- `fastembed` downloads ONNX (Open Neural Network Exchange) Runtime binaries at build time.
- `lancedb` compiles an embedded database. It also needs a `protoc` binary on your `PATH` at build time.
- `duckdb` links a DuckDB library that is already on your system. Point `DUCKDB_LIB_DIR` at it, make it findable through pkg-config, or set `DUCKDB_DOWNLOAD_LIB=1` to download it during the build.
- `kafka` builds librdkafka from source with CMake.
- `fluvio` compiles the Fluvio client.
- `aws-bedrock` compiles the AWS (Amazon Web Services) SDK.

Turn these on only in the crates that need them.

## The all feature

The `all` flag turns on ten common integrations, but not `anthropic`, `lancedb`, `mcp` or `swiftide-agents`. It also includes `fastembed` and `aws-bedrock`, so it builds slowly. [The all bundle](/reference/feature-flags/#the-all-bundle) lists exactly what it includes.

## TLS

The HTTP (Hypertext Transfer Protocol) clients use rustls for TLS (Transport Layer Security) and need no setup: each uses the rustls crypto provider your application installed, or else its own. The exception is Redis over `rediss://`, where Swiftide installs aws-lc-rs as the process-wide provider before connecting if none is installed, so install yours before the first Redis call to use another.

The `fastembed` model download goes through native TLS, which means OpenSSL on Linux.

## Matrix columns

Every flag, type and example in the matrix matches Swiftide {{swiftide_version}}.

- **Integration** names the provider or backend, with a short note.
- **Feature flag** is the flag to pass to `cargo add`.
- **Provides** says where the integration plugs in:
  - **LLM**: a large language model (LLM) client for prompts, chat completions and agents (`SimplePrompt`, `ChatCompletion`).
  - **Embeddings** and **Sparse embeddings**: an embedder for dense or sparse vectors (`EmbeddingModel`, `SparseEmbeddingModel`).
  - **Structured output**: typed responses through `StructuredPrompt`.
  - **Storage**: a backend that the indexing pipeline stores nodes in (`Persist`).
  - **Retrieval**: a retriever for the query pipeline (`Retrieve`). The storage guide for each backend lists the search strategies it supports.
  - **Node cache**: a node cache that skips nodes an indexing pipeline has already processed (`NodeCache`).
  - **Loader**: a loader that produces nodes (`Loader`).
  - **Transformers**: transformers and chunkers for the indexing pipeline.
  - **Reranking**: a response transformer that reorders retrieved documents (`TransformResponse`).
  - **Agent tooling**: tools for agents, or token estimation.
  - **Observability**: traces or metrics.
- **Main types** are the types you build. Each links to docs.rs for the pinned release.
- **Examples** link to example files in the Swiftide repository at the pinned commit.
- **Guide** is the page on this site that shows the integration in use.

::integrations

## Guides

This section has a page for each of these integrations:

- [OpenAI and Azure](/integrations/openai/): Chat Completions, the Responses API (application programming interface), and Azure OpenAI.
- [OpenAI-compatible providers](/integrations/openai-compatible/): Groq, Gemini, Ollama, OpenRouter, Dashscope and any OpenAI-compatible gateway.
- [Anthropic](/integrations/anthropic/): prompts, chat completions, streaming and agents with Claude.
- [AWS Bedrock](/integrations/aws-bedrock/): Bedrock models through the Converse API.
- [FastEmbed](/integrations/fastembed/): dense and sparse embeddings and reranking on your CPU (central processing unit).
- [Qdrant](/integrations/qdrant/): storage with similarity and hybrid search.
- [PgVector](/integrations/pgvector/): storage in Postgres with the pgvector extension.
- [LanceDB](/integrations/lancedb/): an embedded vector store in a local directory.
- [Tree-sitter](/integrations/tree-sitter/): chunking, outlines and metadata for source code.

The other integrations are covered on the page for the feature they serve. The Guide column in the matrix links to each one: DuckDB, Redis and Redb with storage and caching, Kafka, Fluvio, Parquet and scraping with loading, MCP with agents, tiktoken with model usage, and Langfuse and metrics with observability.

## Reference

- [`swiftide::integrations`](docsrs:swiftide/integrations/index.html)
- [`swiftide` crate root, Cargo Features section](docsrs:swiftide/index.html#cargo-features)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html), [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html) and [`StructuredPrompt`](docsrs:swiftide/traits/trait.StructuredPrompt.html)
- [`EmbeddingModel`](docsrs:swiftide/traits/trait.EmbeddingModel.html) and [`SparseEmbeddingModel`](docsrs:swiftide/traits/trait.SparseEmbeddingModel.html)
- [`Persist`](docsrs:swiftide/traits/trait.Persist.html), [`Retrieve`](docsrs:swiftide/traits/trait.Retrieve.html) and [`NodeCache`](docsrs:swiftide/traits/trait.NodeCache.html)
- [`Loader`](docsrs:swiftide/traits/trait.Loader.html) and [`TransformResponse`](docsrs:swiftide/traits/trait.TransformResponse.html)
- [`swiftide::agents::tools::mcp`](docsrs:swiftide/agents/tools/mcp/index.html)
- [`swiftide::langfuse`](docsrs:swiftide/langfuse/index.html)
- [`swiftide::indexing::transformers`](docsrs:swiftide/indexing/transformers/index.html)
- [Feature flags](/reference/feature-flags/)
- [Crates](/reference/crates/)

## Next

[OpenAI and Azure](/integrations/openai/)
