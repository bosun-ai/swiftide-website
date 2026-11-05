---
title: "FastEmbed"
description: "Embed chunks and queries with dense and sparse FastEmbed models, and rerank retrieved documents, on your own CPU without an API key."
sidebar:
  order: 6
---

You embed chunks and queries with local dense and sparse models, pick a model that fits your data, and rerank retrieved documents, all on your CPU without an API key. Turn on the `fastembed` feature flag:

```sh
cargo add swiftide@{{swiftide_version}} --features fastembed
```

## Default models

The hybrid search example builds one dense and one sparse embedder with the default models:

::snippet{src="@swiftide/examples/hybrid_search.rs" anchor="setup" title="examples/hybrid_search.rs"}

The excerpt also builds an OpenAI client and Qdrant storage. To compile it as is, add the `openai` and `qdrant` feature flags next to `fastembed`.

`FastEmbed::try_default()` loads BGE (BAAI General Embedding) small en v1.5, a dense model with 384 dimensions. `FastEmbed::try_default_sparse()` loads SPLADE++ (Sparse Lexical and Expansion model, `prithivida/Splade_PP_en_v1`), a sparse model. Both return a `Result`, because loading the model can fail.

`FastEmbed` implements `EmbeddingModel` and `SparseEmbeddingModel`. You pass the dense instance to the `Embed` batch transformer in the indexing pipeline and to `query_transformers::Embed` in the query pipeline. The sparse instance goes to `SparseEmbed` on both sides. Each instance holds one model. Passing a sparse instance to `Embed` fails with the permanent error "Expected dense model, got sparse", and a dense instance in `SparseEmbed` fails with "Expected sparse model, got dense".

Your storage's vector size must match the model: 384 for BGE small.

## Model choice

The ask-your-codebase showcase indexes source code, so it picks a model trained on code:

::snippet{src="@swiftide/examples/ask-your-codebase/src/embedding.rs" anchor="embedding" title="examples/ask-your-codebase/src/embedding.rs"}

`jina-embeddings-v2-base-code` produces 768-dimensional vectors. `fastembed::TextInitOptions::with_max_length` raises the token limit to 2048 for each embedded text. In ask-your-codebase that text is a chunk of up to 4096 bytes plus its generated questions and answers. FastEmbed truncates longer texts without an error. The default limit is 512 tokens, or less if the model's own limit is lower.

Swiftide does not re-export the `fastembed` crate. To name a model, add it yourself at the major version Swiftide uses:

```sh
cargo add fastembed@5 --no-default-features
```

`FastEmbedBuilder::embedding_model` accepts a `fastembed::TextEmbedding` for dense vectors or a `fastembed::SparseTextEmbedding` for sparse ones. The fastembed documentation lists the dense models, with their dimensions, under `fastembed::EmbeddingModel`.

The excerpt uses `fastembed::InitOptions`, which fastembed 5 keeps as a deprecated alias of `fastembed::TextInitOptions`. Use `fastembed::TextInitOptions` in new code to avoid the deprecation warning. Its options also set the cache directory (`fastembed::TextInitOptions::with_cache_dir`) and cap the ONNX (Open Neural Network Exchange) Runtime threads (`fastembed::TextInitOptions::with_intra_threads`), which default to every core.

## Batch size

How the pipeline batch size and the storage batch size interact is covered in [Embedding](/indexing/embedding/#batch-sizes). The FastEmbed side has one cost to keep in mind: FastEmbed pads every text in a batch to the longest one, so long texts in a large batch cost memory.

The docs-qa showcase sets 32 on both embedding steps and leaves FastEmbed at its default of 256. FastEmbed never sees more texts than the pipeline hands over, so its own batch size only matters when it is smaller:

::snippet{src="@swiftide/examples/docs-qa/src/index.rs" anchor="pipeline" title="examples/docs-qa/src/index.rs" mark="17-18"}

ask-your-codebase sets 32 on both the pipeline and the `FastEmbed` builder, because its embedded texts run up to 2048 tokens. To limit memory, lower the pipeline batch size first.

## Reranking

`Rerank` is a response transformer that scores retrieved documents with a local cross-encoder and keeps the best ones. It implements `TransformResponse`. docs-qa builds it next to its answer generator:

::snippet{src="@swiftide/examples/docs-qa/src/query.rs" anchor="answerer" title="examples/docs-qa/src/query.rs"}

The default model is BGE reranker base. To use another one, pass a `fastembed::TextRerank` wrapped in `Arc<tokio::sync::Mutex<_>>` to `Rerank::builder().model(...)`. `model_batch_size` sets the reranker's batch size, which otherwise uses FastEmbed's default of 256.

For where `Rerank` sits in the query pipeline and how to write a rerank template, see [Reranking and response transformers](/querying/reranking-and-response-transformers/).

## First run and model cache

The first time a model loads, FastEmbed downloads it from Hugging Face into `./.fastembed_cache` in the working directory. Later runs load it from there. Set `FASTEMBED_CACHE_DIR` to share one cache between projects. If `HF_HOME` is set, it takes precedence over both, and the models land there. The three docs-qa models (dense, sparse and reranker) take about 2.3 GB. The ask-your-codebase model takes about 600 MB.

The ONNX Runtime library is downloaded once at build time, because the `fastembed` feature flag turns on fastembed's `ort-download-binaries` feature. The first build therefore needs network access, and so does the first run.

## Gotchas

- A `FastEmbed` keeps its model behind an `Arc<tokio::sync::Mutex<_>>`. Clones share the same model, so calls on one instance and its clones run one at a time. A higher `with_concurrency` on `Embed` does not speed up embedding.
- The model runs inside the async call, not on a blocking thread. A large batch occupies one Tokio worker thread until it finishes.
- Indexing and querying must use the same model. See [Embedding](/indexing/embedding/) for what to do when you change it.
- `Rerank::default()`, and `Rerank::builder().build()` without a model, panic when the default reranker cannot load, for example on a first run without network access. `FastEmbed::try_default()` returns an error instead.

## Reference

- [`FastEmbed`](docsrs:swiftide/integrations/fastembed/struct.FastEmbed.html)
- [`FastEmbedBuilder`](docsrs:swiftide/integrations/fastembed/struct.FastEmbedBuilder.html)
- [`Rerank`](docsrs:swiftide/integrations/fastembed/struct.Rerank.html)
- [`EmbeddingModel`](docsrs:swiftide/traits/trait.EmbeddingModel.html)
- [`SparseEmbeddingModel`](docsrs:swiftide/traits/trait.SparseEmbeddingModel.html)
- [`TransformResponse`](docsrs:swiftide/traits/trait.TransformResponse.html)
- [`transformers::Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html)
- [`transformers::SparseEmbed`](docsrs:swiftide/indexing/transformers/struct.SparseEmbed.html)
- [`query_transformers::Embed`](docsrs:swiftide/query/query_transformers/struct.Embed.html)
- [`query_transformers::SparseEmbed`](docsrs:swiftide/query/query_transformers/struct.SparseEmbed.html)
- [fastembed crate documentation](https://docs.rs/fastembed/5)
- [Embedding](/indexing/embedding/)
- [Reranking and response transformers](/querying/reranking-and-response-transformers/)
- [Feature flags](/reference/feature-flags/)

## Next

[Qdrant](/integrations/qdrant/)
