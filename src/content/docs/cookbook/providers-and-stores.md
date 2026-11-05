---
title: "Other providers and stores"
description: "Short indexing recipes for Ollama, Groq, Dashscope and FastEmbed models and for Redis, PgVector and LanceDB storage."
sidebar:
  order: 11
---

Each recipe is a runnable example from the swiftide repository that indexes files into one provider or store; the PgVector, LanceDB and Dashscope recipes also query what they stored. Each section points to the integration page that covers the client options in depth.

## Prerequisites

Run an example from the root of the swiftide repository, because the examples read `README.md` and other files by relative path:

```sh
# in the swiftide repository
cargo run -p swiftide-examples --example <name>
```

To copy a recipe into your own crate, add the `swiftide` line from its section, plus the runtime and logging crates every example uses:

```sh
# in your own crate
cargo add tokio --features full
cargo add tracing-subscriber
cargo add rustls@0.23 --no-default-features --features ring,std
```

Swiftide's Hypertext Transfer Protocol (HTTP) clients use rustls without a Transport Layer Security (TLS) crypto provider. The examples get one from another dependency of the examples crate. Your own binary installs one at the start of `main`; see [Crypto provider panic](/reference/troubleshooting-and-faq/#crypto-provider-panic).

## Ollama

```sh
# in your own crate
cargo add swiftide@{{swiftide_version}} --features ollama,fastembed
```

```sh
# in the swiftide repository
ollama pull llama3.1
cargo run -p swiftide-examples --example index-ollama
```

::snippet{src="@swiftide/examples/index_ollama.rs" anchor="pipeline" title="examples/index_ollama.rs"}

FastEmbed computes the embeddings locally, so only metadata generation calls Ollama. `Ollama::default()` talks to `http://localhost:11434/v1` and sends no application programming interface (API) key.

Client options and gotchas: [OpenAI-compatible providers](/integrations/openai-compatible/).

## Groq

```sh
# in your own crate
cargo add swiftide@{{swiftide_version}} --features groq,fastembed
```

```sh
# in the swiftide repository
cargo run -p swiftide-examples --example index-groq
```

::snippet{src="@swiftide/examples/index_groq.rs" anchor="pipeline" title="examples/index_groq.rs"}

The client reads `GROQ_API_KEY`. The example sets the model `llama3-8b-8192`, which Groq has retired. Change it in the example's setup to a current model, such as `llama-3.1-8b-instant`, before you run it.

Client options and gotchas: [OpenAI-compatible providers](/integrations/openai-compatible/).

## Dashscope

```sh
# in your own crate
cargo add swiftide@{{swiftide_version}} --features dashscope,lancedb
cargo add temp-dir anyhow
```

```sh
# in the swiftide repository
cargo run -p swiftide-examples --example dashscope
```

::snippet{src="@swiftide/examples/dashscope.rs" anchor="indexing" title="examples/dashscope.rs"}

`Dashscope` reads `QWEN_API_KEY`, or `DASHSCOPE_API_KEY` when that is unset. One client sets both an embed model and a prompt model, so it serves as large language model (LLM) and embedder.

`with_default_llm_client` gives a client to every transformer added after it that has none of its own, such as `MetadataQAText::default()`. A transformer built with `new(client)` keeps its own client.

The loader picks up every markdown file under the current directory. It uses the default filters of the `ignore` crate, so it skips hidden files and directories, such as `.github/`. It also skips what `.ignore` files, the global gitignore and `.git/info/exclude` exclude, and, inside a git repository, what `.gitignore` excludes.

::snippet{src="@swiftide/examples/dashscope.rs" anchor="query" title="examples/dashscope.rs"}

The query pipeline generates subquestions, embeds them, retrieves from LanceDB, summarizes the documents with the `Summary` response transformer, and answers with `answers::Simple`.

Client options and gotchas: [OpenAI-compatible providers](/integrations/openai-compatible/) and [LanceDB](/integrations/lancedb/).

## FastEmbed

```sh
# in your own crate
cargo add swiftide@{{swiftide_version}} --features fastembed,qdrant
```

```sh
# in the swiftide repository
docker run -p 6334:6334 qdrant/qdrant
cargo run -p swiftide-examples --example fastembed
```

::snippet{src="@swiftide/examples/fastembed.rs" anchor="pipeline" title="examples/fastembed.rs"}

The recipe needs Qdrant on `QDRANT_URL`, which defaults to the gRPC port at `http://localhost:6334`. It loads, embeds and stores, with no chunker and no metadata.

FastEmbed runs embedding models in your process, with no API key. It downloads the default model from Hugging Face on first use. That model produces 384-dimensional vectors, so the Qdrant collection uses `vector_size(384)`.

`batch_size(10)` on the FastEmbed builder sets how many texts the model embeds at once (default 256). `batch_size(50)` on the Qdrant builder sets the nodes per write. [Embedding](/indexing/embedding/) explains how these relate to the pipeline's batch size.

Client options and gotchas: [FastEmbed](/integrations/fastembed/) and [Qdrant](/integrations/qdrant/).

## Redis

```sh
# in your own crate
cargo add swiftide@{{swiftide_version}} --features redis,tree-sitter
cargo add serde_json
```

```sh
# in the swiftide repository
cargo run -p swiftide-examples --example index-redis
```

::snippet{src="@swiftide/examples/index_into_redis.rs" anchor="pipeline" title="examples/index_into_redis.rs"}

The recipe needs Redis on `REDIS_URL`, which defaults to `redis://localhost:6379`. It chunks Rust files with `ChunkCode` and stores the chunks in Redis, with no LLM or embedder.

Redis is storage only here. It does not implement retrieval, but it also works as a node cache and as a message history backend. Keys, values and the other backends: [Storage](/indexing/storage/).

## PgVector

```sh
# in your own crate
cargo add swiftide@{{swiftide_version}} --features pgvector,ollama,fastembed
```

```sh
# in the swiftide repository
ollama pull llama3.2
cargo run -p swiftide-examples --example index-md-pgvector
```

The example starts a `pgvector/pgvector` container through Docker with a test helper, so Docker must be running. In your application, pass your own database URL to `db_url`.

::snippet{src="@swiftide/examples/index_md_into_pgvector.rs" anchor="indexing" title="examples/index_md_into_pgvector.rs"}

::snippet{src="@swiftide/examples/index_md_into_pgvector.rs" anchor="query" title="examples/index_md_into_pgvector.rs"}

`query_all` sends several questions through one pipeline and returns an answered query for each. The `SimilaritySingleEmbedding` search strategy needs exactly one vector field; with more, it fails with "Search strategy for multiple vector fields in the schema is not yet implemented". A `CustomStrategy` does not have this limit.

Table setup, columns and indexes: [PgVector](/integrations/pgvector/).

## LanceDB

```sh
# in your own crate
cargo add swiftide@{{swiftide_version}} --features lancedb,openai
cargo add temp-dir
```

```sh
# in the swiftide repository
cargo run -p swiftide-examples --example lancedb
```

::snippet{src="@swiftide/examples/lancedb.rs" anchor="setup" title="examples/lancedb.rs"}

::snippet{src="@swiftide/examples/lancedb.rs" anchor="indexing" title="examples/lancedb.rs"}

::snippet{src="@swiftide/examples/lancedb.rs" anchor="query" title="examples/lancedb.rs"}

LanceDB is embedded: `uri` points at a local directory, and the table is created on the first run. The example uses a temporary directory, so the data is gone when it exits. The recipe needs `OPENAI_API_KEY`.

With `SimilaritySingleEmbedding`, retrieval also needs exactly one vector field, and fails with "Zero or multiple vector fields configured in schema" otherwise. A `CustomStrategy` does not have this limit.

Remote storage and vector indexes: [LanceDB](/integrations/lancedb/).

## Reference

- [`Ollama`](docsrs:swiftide/integrations/ollama/type.Ollama.html), [`Groq`](docsrs:swiftide/integrations/groq/type.Groq.html), [`Dashscope`](docsrs:swiftide/integrations/dashscope/type.Dashscope.html), [`GenericOpenAI`](docsrs:swiftide/integrations/openai/struct.GenericOpenAI.html)
- [`FastEmbed`](docsrs:swiftide/integrations/fastembed/struct.FastEmbed.html)
- [`Redis`](docsrs:swiftide/integrations/redis/struct.Redis.html), [`RedisBuilder`](docsrs:swiftide/integrations/redis/struct.RedisBuilder.html)
- [`PgVector`](docsrs:swiftide/integrations/pgvector/struct.PgVector.html), [`PgVectorBuilder`](docsrs:swiftide/integrations/pgvector/struct.PgVectorBuilder.html)
- [`LanceDB`](docsrs:swiftide/integrations/lancedb/struct.LanceDB.html), [`LanceDBBuilder`](docsrs:swiftide/integrations/lancedb/struct.LanceDBBuilder.html)
- [`Qdrant`](docsrs:swiftide/integrations/qdrant/struct.Qdrant.html)
- [`MetadataQAText`](docsrs:swiftide/indexing/transformers/struct.MetadataQAText.html), [`Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html), [`ChunkCode`](docsrs:swiftide/indexing/transformers/struct.ChunkCode.html)
- [indexing `Pipeline`](docsrs:swiftide/indexing/struct.Pipeline.html), [query `Pipeline`](docsrs:swiftide/query/struct.Pipeline.html)
- [`SimilaritySingleEmbedding`](docsrs:swiftide/query/search_strategies/struct.SimilaritySingleEmbedding.html), [`CustomStrategy`](docsrs:swiftide/query/search_strategies/struct.CustomStrategy.html), [`Summary`](docsrs:swiftide/query/response_transformers/struct.Summary.html)
- [OpenAI-compatible providers](/integrations/openai-compatible/), [FastEmbed](/integrations/fastembed/), [Qdrant](/integrations/qdrant/), [PgVector](/integrations/pgvector/), [LanceDB](/integrations/lancedb/), [Storage](/indexing/storage/)

## Next

[Feature flags](/reference/feature-flags/)
