---
title: "Hybrid search and reranking"
description: "Retrieve with dense and sparse vectors fused in Qdrant, and rerank retrieved documents with a local FastEmbed cross-encoder before answering."
sidebar:
  order: 7
---

You build two query pipelines over Qdrant: one that fuses dense and sparse vector search, and one that reranks retrieved documents with a local cross-encoder before the large language model (LLM) answers. Both are runnable examples in the swiftide repository: `hybrid-search` and `reranking`.

## Setup

To run the examples, clone the repository and start Qdrant with its gRPC port 6334 exposed ([Running Qdrant](/integrations/qdrant/#local-instance-and-connection) covers the options). Set `OPENAI_API_KEY`. Run from the repository root, because the examples read `swiftide-core/` and `README.md` by relative path:

```sh
git clone https://github.com/bosun-ai/swiftide && cd swiftide
docker run -d -p 6333:6333 -p 6334:6334 qdrant/qdrant
cargo run -p swiftide-examples --example hybrid-search
cargo run -p swiftide-examples --example reranking
```

To use the same code in your own project, add Swiftide with the feature flags the examples use, plus the async runtime and logging they call from `main`:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,qdrant,fastembed,tree-sitter
cargo add tokio --features full
cargo add tracing-subscriber
```

`tree-sitter` is only needed for the code chunker in the hybrid example.

## Hybrid indexing

::snippet{src="@swiftide/examples/hybrid_search.rs" anchor="setup" title="examples/hybrid_search.rs"}

`FastEmbed::try_default()` loads a local dense embedding model with 384 dimensions, which is why the collection uses `vector_size(384)`. `FastEmbed::try_default_sparse()` loads SPLADE, a sparse model that weights individual terms. A dense vector matches on meaning. A sparse vector also rewards exact words, such as a type or function name.

`with_vector` and `with_sparse_vector` both name `EmbeddedField::Combined`: the chunk and its metadata embedded as one text. Qdrant stores the two vectors on the same point.

::snippet{src="@swiftide/examples/hybrid_search.rs" anchor="indexing" title="examples/hybrid_search.rs"}

`ChunkCode` splits the Rust files in `swiftide-core/` with tree-sitter. `MetadataQACode` asks `gpt-4o-mini` for five questions and answers per chunk and stores them under the `Questions and Answers (code)` metadata key. That metadata is part of the combined text, so both vectors include it.

`SparseEmbed` and `Embed` are batch transformers. The example uses one `batch_size` of 64 for both of them and for the Qdrant storage.

## Hybrid query

::snippet{src="@swiftide/examples/hybrid_search.rs" anchor="query" title="examples/hybrid_search.rs"}

`HybridSearch::default()` searches the `Combined` dense and sparse vectors. `with_top_n(20)` limits each sub-search to 20 candidates. `with_top_k(20)` limits the fused result.

The Qdrant retriever needs both query embeddings. `query_transformers::Embed` sets the dense one and `query_transformers::SparseEmbed` the sparse one, each with the model used for indexing. If either is missing, retrieval fails with `No embedding for query` or `No sparse embedding for query`.

`GenerateSubquestions` runs first. Its default prompt asks `gpt-4o` for the original question plus five extra questions. That list becomes the current query, and both embeddings are made from it.

Qdrant fuses the two searches with Reciprocal Rank Fusion (RRF); see [Retrieval and search strategies](/querying/retrieval-and-search-strategies/).

::snippet{src="@swiftide/examples/hybrid_search.rs" anchor="run" title="examples/hybrid_search.rs"}

### Stores and filters

`Qdrant` implements `Retrieve` for `HybridSearch` with a Qdrant `Filter` type, so type inference picks that filter type for `HybridSearch::default()`. Pass a filter with `HybridSearch::from_filter` or `with_filter`, and Qdrant applies it to both sub-searches. [Retrieval and search strategies](/querying/retrieval-and-search-strategies/) shows a filter.

`Duckdb`, behind the `duckdb` feature flag, also implements `Retrieve` for `HybridSearch`, without a filter. It combines cosine similarity with a full-text keyword score on the query text, so it needs only the dense query embedding. `LanceDB` and `PgVector` do not implement it.

### Existing collections

Qdrant creates the collection with its named vectors only when the collection does not exist yet. For an existing collection it logs a warning and keeps the old configuration. A collection created without `with_sparse_vector` stays without a sparse vector. Use a new collection name, as the example does with `swiftide-hybrid-example`, or delete the old collection first.

## Reranking

::snippet{src="@swiftide/examples/reranking.rs" anchor="setup" title="examples/reranking.rs"}

`Rerank` is a response transformer: `top_k(5)` keeps the five best of the retrieved documents. [Reranking and response transformers](/querying/reranking-and-response-transformers/) covers its builder fields and templates.

::snippet{src="@swiftide/examples/reranking.rs" anchor="indexing" title="examples/reranking.rs"}

The indexing pipeline chunks `README.md` as markdown, embeds the chunks with FastEmbed and stores them in Qdrant. With one dense vector and no sparse vector, Qdrant stores the embedding as the point's single unnamed vector.

::snippet{src="@swiftide/examples/reranking.rs" anchor="query" title="examples/reranking.rs"}

`query::Pipeline::default()` uses `SimilaritySingleEmbedding`, which retrieves at most 10 documents, its default `top_k`. `then_transform_response(reranker)` runs after retrieval and before the answer. It scores the retrieved documents and passes the best 5 to `answers::Simple`.

::snippet{src="@swiftide/examples/reranking.rs" anchor="run" title="examples/reranking.rs"}

### Gotchas

- Retrieve more documents than you keep. Here retrieval returns up to 10 and `Rerank` keeps 5. If retrieval returns `top_k` documents or fewer, `Rerank` only reorders them.
- FastEmbed loads its models when you construct them, in `FastEmbed::try_default()`, `try_default_sparse()` and `Rerank::builder().build()`, and downloads them on the first run. A `Rerank` built without `model` loads the default model with `expect`, so a failed download panics at `build()` instead of returning an error.
- The plain `Qdrant` retriever returns each document's content `Debug`-quoted: wrapped in `"` with escaped newlines. In both examples, `Rerank` scores and `answers::Simple` renders that quoted text. docs-qa works around it with a `CleanQdrant` wrapper; see [Qdrant content workaround](/showcases/docs-qa/#qdrant-content-workaround).

## Combined pipeline

The docs-qa showcase puts a reranker behind a hybrid search. Its strategy fetches 20 candidates per sub-search and keeps 20 after fusion:

::snippet{src="@swiftide/examples/docs-qa/src/query.rs" anchor="hybrid" title="examples/docs-qa/src/query.rs"}

Its reranker keeps 10 of the 20 candidates and scores each one with the page name and chunk title in front of the content:

::snippet{src="@swiftide/examples/docs-qa/src/query.rs" anchor="answerer" title="examples/docs-qa/src/query.rs"}

The query pipeline embeds the question twice, retrieves the fused candidates and reranks them before answering:

::snippet{src="@swiftide/examples/docs-qa/src/query.rs" anchor="query_pipeline" title="examples/docs-qa/src/query.rs"}

`self.qdrant` is the `CleanQdrant` wrapper around `Qdrant`, so the reranker and the answer see the text as it was indexed. The [docs-qa showcase](/showcases/docs-qa/) covers the indexing side and shows captured answers.

## Reference

- [`HybridSearch`](docsrs:swiftide/query/search_strategies/struct.HybridSearch.html)
- [`SimilaritySingleEmbedding`](docsrs:swiftide/query/search_strategies/struct.SimilaritySingleEmbedding.html)
- [`Qdrant`](docsrs:swiftide/integrations/qdrant/struct.Qdrant.html) and [`QdrantBuilder`](docsrs:swiftide/integrations/qdrant/struct.QdrantBuilder.html)
- [`Duckdb`](docsrs:swiftide/integrations/duckdb/struct.Duckdb.html)
- [`FastEmbed`](docsrs:swiftide/integrations/fastembed/struct.FastEmbed.html)
- [`Rerank`](docsrs:swiftide/integrations/fastembed/struct.Rerank.html) and [`RerankBuilder`](docsrs:swiftide/integrations/fastembed/struct.RerankBuilder.html)
- [`indexing::Pipeline`](docsrs:swiftide/indexing/struct.Pipeline.html)
- [`FileLoader`](docsrs:swiftide/indexing/loaders/struct.FileLoader.html)
- [`EmbeddedField`](docsrs:swiftide/indexing/enum.EmbeddedField.html)
- [`ChunkCode`](docsrs:swiftide/indexing/transformers/struct.ChunkCode.html), [`ChunkMarkdown`](docsrs:swiftide/indexing/transformers/struct.ChunkMarkdown.html) and [`MetadataQACode`](docsrs:swiftide/indexing/transformers/struct.MetadataQACode.html)
- [`transformers::Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html) and [`transformers::SparseEmbed`](docsrs:swiftide/indexing/transformers/struct.SparseEmbed.html)
- [`query_transformers::Embed`](docsrs:swiftide/query/query_transformers/struct.Embed.html), [`query_transformers::SparseEmbed`](docsrs:swiftide/query/query_transformers/struct.SparseEmbed.html) and [`GenerateSubquestions`](docsrs:swiftide/query/query_transformers/struct.GenerateSubquestions.html)
- [`answers::Simple`](docsrs:swiftide/query/answers/struct.Simple.html)
- [`query::Pipeline`](docsrs:swiftide/query/struct.Pipeline.html)
- [`Retrieve`](docsrs:swiftide/traits/trait.Retrieve.html) and [`TransformResponse`](docsrs:swiftide/traits/trait.TransformResponse.html)

## Next

[Rich metadata and multiple vectors](/cookbook/rich-metadata-and-multiple-vectors/)
