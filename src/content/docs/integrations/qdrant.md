---
title: "Qdrant"
description: "Store indexed chunks in a Qdrant collection with dense and sparse vectors, then retrieve them with similarity or hybrid search and payload filters."
sidebar:
  order: 7
identLint:
  ignore: ["Distance::Cosine", "Distance::Manhattan", last_updated_at, collection_exists, delete_collection]
---

With the `qdrant` feature flag, an indexing pipeline writes nodes into a Qdrant collection, and a query pipeline searches that collection by similarity or with hybrid search.

```sh
cargo add swiftide@{{swiftide_version}} --features qdrant
```

Hybrid search also needs `fastembed` for sparse embeddings, and the filter snippet needs `openai`.

## Local instance and connection

Start Qdrant in Docker:

```sh
docker run -d -p 6333:6333 -p 6334:6334 qdrant/qdrant
```

Swiftide talks to Qdrant over gRPC Remote Procedure Call (gRPC) on port 6334, not over the Hypertext Transfer Protocol (HTTP) API on 6333.

## Builder

::snippet{src="@swiftide/examples/docs-qa/src/store.rs" anchor="store" title="examples/docs-qa/src/store.rs"}

`Qdrant::builder()` reads the address from `QDRANT_URL`, falls back to `http://localhost:6334`, and sends `QDRANT_API_KEY` when set. `Qdrant::try_from_url(url)` returns `Result<QdrantBuilder>`, so add `?` before calling builder methods. The builder's `client` method takes your own `qdrant_client::Qdrant`.

`vector_size` is required and must match your embedder. docs-qa uses 384, the size of FastEmbed's default dense model. The other settings have defaults:

| Setting | Default |
| --- | --- |
| `collection_name` | `swiftide` |
| `vector_distance` | `Distance::Cosine` |
| `batch_size` | 50 nodes per upsert |
| vectors | one dense vector for `EmbeddedField::Combined` |

`with_vector` adds a dense vector and `with_sparse_vector` adds a sparse one. The first `with_vector` call replaces the default, so list `EmbeddedField::Combined` yourself when you still need it. Both take an `EmbeddedField`, which names the text the vector was computed from. `Combined` is the chunk together with its metadata.

## Stored points

`Qdrant` implements `Persist`, so you hand it to `then_store_with`:

::snippet{src="@swiftide/examples/docs-qa/src/index.rs" anchor="pipeline" title="examples/docs-qa/src/index.rs"}

Before the first node, `setup` creates the collection if it is missing. An existing collection is kept as it is. The pipeline then upserts nodes in batches. Every point carries this payload:

- each metadata field of the node, under its own name
- `path`, the node's source path
- `content`, the chunk text
- `last_updated_at`, the time of the upsert

The point ID is a universally unique identifier (UUID) computed from the path and the chunk, so indexing the same chunk again overwrites its point. Debug builds wait for each upsert to apply; release builds do not.

### Captured output

A debug build of docs-qa indexed four markdown pages into 43 chunks, with metadata from `claude-haiku-5-5` and embeddings from local FastEmbed models:

```text
Indexed 43 chunks into `docs-qa` in 71.7s
```

## Named vectors

A point can hold one dense vector per embedded field:

::snippet{src="@swiftide/examples/store_multiple_vectors.rs" anchor="pipeline" title="examples/store_multiple_vectors.rs"}

`EmbedMode::PerField` embeds the chunk and every metadata field separately. `EmbedMode::Both` adds the combined text. When the collection has named vectors, Qdrant stores only the ones you configured and skips the rest. The embed mode and the configured vectors must agree. A node with exactly one dense vector and no sparse vector is always sent as the unnamed vector. With several configured vectors, use `EmbedMode::PerField` or `Both`. With one configured vector, keep the default embed mode. Sparse vectors are only stored for fields that also have a configured dense vector.

`VectorConfig::builder()` overrides the size or distance of one vector, here `Distance::Manhattan` for the title.

With one dense vector and no sparse vector, the collection uses Qdrant's unnamed vector. Otherwise vectors are named after their field: `Combined`, `Chunk` or `Metadata: <field name>`. Sparse vectors get a `_sparse` suffix, such as `Combined_sparse`.

## Similarity search

A query pipeline searches Qdrant through `then_retrieve`. This one narrows the search with a payload filter:

::snippet{src="@snippets/src/integrations/qdrant.rs" anchor="filter"}

`SimilaritySingleEmbedding` returns the 10 nearest chunks unless you set `with_top_k`. Only `then_retrieve(qdrant)` is Qdrant-specific. The query must already carry a dense embedding from `query_transformers::Embed`, made with the model you indexed with.

Swiftide picks the vector to search from the configuration of the `Qdrant` value in the query pipeline, not from the collection. With more than one vector or any sparse vector configured, it searches the vector named `Combined`. Build the retriever with the same vector configuration as the indexing side; docs-qa calls the same `store::connect` for both. The collection in the previous section has no `Combined` vector, so it cannot serve this search.

## Hybrid search

Hybrid search adds a sparse, keyword-like search to the dense one:

::snippet{src="@swiftide/examples/docs-qa/src/query.rs" anchor="hybrid" title="examples/docs-qa/src/query.rs"}

The query then needs both embeddings before retrieval:

::snippet{src="@swiftide/examples/docs-qa/src/query.rs" anchor="query_pipeline" title="examples/docs-qa/src/query.rs"}

Qdrant fetches `top_n` candidates from the dense and from the sparse vector and fuses them with Reciprocal Rank Fusion (RRF); see [search strategies](/querying/retrieval-and-search-strategies/) for `top_n` and `top_k`. Both vector fields default to `Combined`; `with_dense_vector_field` and `with_sparse_vector_field` select other named vectors.

The collection needs a sparse vector for the same field. Without a sparse embedding on the query, retrieval fails with "No sparse embedding for query".

## Payload filters

Both search strategies take a Qdrant `Filter`, as in the similarity snippet. Swiftide re-exports the client crate at `swiftide::integrations::qdrant::qdrant_client`. Conditions match payload keys, including any metadata field. An array of strings matches any of its values as keywords. An owned `String` that contains whitespace becomes a full-text match instead of a keyword match. A full-text match needs a full-text payload index on that field, which Swiftide does not create. Without one, Qdrant matches the text as an exact substring.

Qdrant implements hybrid search only for `HybridSearch<Filter>`. Write that type even when you set no filter. For `SimilaritySingleEmbedding`, Qdrant accepts both `Filter` and no filter. `SimilaritySingleEmbedding::default()` is the search without a filter and needs no type annotation. Before 0.34, it needed one.

## Gotchas

- Retrieved content is `Debug`-quoted, with escaped newlines. docs-qa wraps the retriever in [`CleanQdrant`](https://github.com/bosun-ai/swiftide/blob/{{swiftide_sha}}/examples/docs-qa/src/retrieval.rs) (the `self.qdrant` in the query pipeline snippet above), which restores the text before reranking and answering.
- A changed vector size, distance or set of vectors needs a new collection.
- A changed chunk gets a new point ID, and the old point stays. docs-qa's `--recreate` flag drops the collection first.
- Retrieved documents keep every payload key except `content` as metadata.
- `client()` returns a shared reference to the inner client, `&Arc<qdrant_client::Qdrant>`. docs-qa uses it for a health check, `collection_exists` and `delete_collection`.
- If Qdrant is down, `run` fails during setup. See [unreachable stores](/reference/troubleshooting-and-faq/#unreachable-stores).

## Reference

- [`Qdrant`](docsrs:swiftide/integrations/qdrant/struct.Qdrant.html)
- [`QdrantBuilder`](docsrs:swiftide/integrations/qdrant/struct.QdrantBuilder.html)
- [`VectorConfig`](docsrs:swiftide/integrations/qdrant/struct.VectorConfig.html)
- [`SparseVectorConfig`](docsrs:swiftide/integrations/qdrant/struct.SparseVectorConfig.html)
- [`Distance`](docsrs:swiftide/integrations/qdrant/type.Distance.html)
- [`qdrant_client`](docsrs:swiftide/integrations/qdrant/qdrant_client/index.html)
- [`EmbeddedField`](docsrs:swiftide/indexing/enum.EmbeddedField.html)
- [`EmbedMode`](docsrs:swiftide/indexing/enum.EmbedMode.html)
- [`Persist`](docsrs:swiftide/traits/trait.Persist.html)
- [`Retrieve`](docsrs:swiftide/traits/trait.Retrieve.html)
- [`SimilaritySingleEmbedding`](docsrs:swiftide/query/search_strategies/struct.SimilaritySingleEmbedding.html)
- [`HybridSearch`](docsrs:swiftide/query/search_strategies/struct.HybridSearch.html)
- [docs-qa source](https://github.com/bosun-ai/swiftide/tree/{{swiftide_sha}}/examples/docs-qa)

## Next

[PgVector](/integrations/pgvector/)
