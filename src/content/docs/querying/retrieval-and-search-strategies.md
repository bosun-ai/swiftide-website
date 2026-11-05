---
title: "Retrieval and search strategies"
description: "Pick a search strategy (similarity, hybrid or custom), set top_k and filters, and pass it to a Qdrant, PgVector, LanceDB or DuckDB storage that acts as the retriever."
sidebar:
  order: 3
---

A search strategy decides how many documents come back and how a retriever finds them. You pick a strategy, set `top_k` and a filter, and hand it to your storage, which acts as the retriever: Qdrant, PgVector, LanceDB or DuckDB.

## Similarity search

::snippet{src="@swiftide/examples/ask-your-codebase/src/query.rs" anchor="query_pipeline" title="examples/ask-your-codebase/src/query.rs" mark="1"}

`SimilaritySingleEmbedding` searches with the dense embedding that the `Embed` query transformer put on the query. It returns `top_k` documents. The default is 10. ask-your-codebase searches a LanceDB table and sets `top_k` from its `--top-k` flag, which defaults to 20.

`with_top_k` takes the strategy by value and returns it, so it chains onto `default()` or `from_filter`. :unreleased `SimilaritySingleEmbedding::default()` exists only for the unfiltered strategy, so its filter type is inferred as `()`, even on stores that also retrieve with a typed filter. For a filtered search, start from `from_filter` or call `with_filter`, as in [Filters](#filters).

## Hybrid search

::snippet{src="@swiftide/examples/docs-qa/src/query.rs" anchor="hybrid" title="examples/docs-qa/src/query.rs"}

`HybridSearch` combines a keyword or sparse search with a dense search. `top_n` caps the candidates of each search. `top_k` caps the documents that come back. Both default to 10. docs-qa fetches 20 candidates and later reranks them down to 10. `with_top_n`, `with_top_k`, `with_dense_vector_field` and `with_sparse_vector_field` take the strategy by value and return it, so they chain without `.to_owned()`. :unreleased

`HybridSearch` implements `Default` for every filter type. When nothing else fixes the filter type, name it, as docs-qa does with `HybridSearch<Filter>` in its return type, or the compiler stops with E0283.

On Qdrant, the query needs a dense and a sparse embedding, so the pipeline runs both query transformers before it retrieves. The example creates two FastEmbed models in its setup: `fastembed` for dense and `fastembed_sparse` for sparse embeddings.

::snippet{src="@swiftide/examples/hybrid_search.rs" anchor="query"}

FastEmbed is the only sparse embedding model in Swiftide, so Qdrant hybrid search needs the `qdrant` and `fastembed` feature flags. The chunks need sparse embeddings too. Run the `SparseEmbed` transformer in the indexing pipeline next to `Embed`:

::snippet{src="@swiftide/examples/hybrid_search.rs" anchor="indexing"}

Qdrant runs a sparse and a dense search, each limited to `top_n`. It merges them with reciprocal rank fusion (RRF) and keeps `top_k`. The collection needs a sparse vector next to the dense one; see [Qdrant](/integrations/qdrant/).

DuckDB runs hybrid search without a sparse embedding. It scores the chunk text with BM25 full-text search over `query.current()` and the dense embedding with cosine similarity. Each score is normalized to its maximum, and the final score is 0.8 times the dense score plus 0.2 times the full-text score. After `GenerateSubquestions`, the keyword text is the whole list of generated questions.

## Filters

A filter narrows the search to documents with matching metadata. Each storage takes its own filter type. Qdrant takes a `Filter` from its client, which Swiftide re-exports at `swiftide::integrations::qdrant::qdrant_client`:

::snippet{src="@snippets/src/querying/retrieval.rs" anchor="qdrant_filter"}

PgVector and LanceDB take a `String`. PgVector expects `key = value` and compares it with the metadata column `meta_<key>`, so the key must be a metadata field on the table. Swiftide does not put the node path into metadata. To filter on `path`, insert it into each node's metadata while you index (see [Transformers and metadata](/indexing/transformers-and-metadata/)), as ask-your-codebase does. Without it, PgVector refuses to store the node with "Missing metadata field".

::snippet{src="@snippets/src/querying/retrieval.rs" anchor="pgvector_filter"}

LanceDB passes the string to its `only_if` method, which takes a SQL predicate on the table's columns, such as `path = 'README.md'`. That column has a value only if you put `path` in the node metadata and configured it with `with_metadata("path")`. DuckDB has no filter.

## Custom strategies

`CustomStrategy` hands the query to your closure, and the closure builds the query the storage runs. `from_query` takes a synchronous closure, `from_async_query` an async one. PgVector runs a `sqlx::QueryBuilder<Postgres>`, LanceDB a LanceDB query, and DuckDB a SQL string.

::snippet{src="@snippets/src/querying/retrieval.rs" anchor="pgvector_custom_strategy"}

PgVector maps each row by column name. Select `id`, `chunk` and the `meta_` columns you need. `get_table_name()` returns the table, `swiftide_pgv_store` unless you set another. Swiftide does not re-export sqlx, so add it yourself:

```sh
cargo add sqlx@0.9 --no-default-features --features postgres
```

A LanceDB custom strategy returns a query that implements `lancedb::query::ExecutableQuery`. Swiftide does not re-export lancedb either. Enable the `lancedb` feature flag and add the crate at the version Swiftide uses:

```sh
cargo add lancedb@0.33
```

DuckDB reads the first three columns of each row by position: the id, the chunk and the path.

## Retrievers per storage

| Storage (feature flag) | Strategies | Filter | Limits |
| --- | --- | --- | --- |
| `Qdrant` (`qdrant`) | `SimilaritySingleEmbedding`, `HybridSearch<Filter>` | Qdrant `Filter` | Similarity search uses the combined vector when the collection has more than one vector or a sparse vector |
| `PgVector` (`pgvector`) | `SimilaritySingleEmbedding`, `CustomStrategy<QueryBuilder<Postgres>>` | `String`, `key = value` | One vector field; orders by cosine distance |
| `LanceDB` (`lancedb`) | `SimilaritySingleEmbedding`, `CustomStrategy<Q: ExecutableQuery>` | `String`, SQL predicate | Exactly one vector field; content comes from the `chunk` column |
| `Duckdb` (`duckdb`) | `SimilaritySingleEmbedding`, `HybridSearch`, `CustomStrategy<String>` | none | Uses one vector, picked arbitrarily when several are configured; documents carry `id` and `path` metadata only |

Setup for each storage is on [Qdrant](/integrations/qdrant/), [PgVector](/integrations/pgvector/) and [LanceDB](/integrations/lancedb/). DuckDB has no integration page; see the `Duckdb` reference below.

## Custom retrievers

A retriever implements `Retrieve<S>` for a search strategy `S`. It receives the strategy and the pending query, and returns `query.retrieved_documents(documents)`. `retrieved_documents` adds the documents and records a retrieval in the query's transformation history. The trait requires `Clone`.

::snippet{src="@snippets/src/querying/retrieval.rs" anchor="custom_retriever"}

::snippet{src="@snippets/src/querying/retrieval.rs" anchor="custom_retriever_pipeline"}

A closure works too: `then_retrieve` accepts any `Fn(&S, Query<states::Pending>) -> Result<Query<states::Retrieved>>` that is `Send + Sync + Clone`. The closure is synchronous.

## Gotchas

- Retrieval fails when the query has no embedding. Qdrant and LanceDB report "No embedding for query", PgVector and DuckDB "Missing embedding in query state". Qdrant hybrid search also needs a sparse embedding ("No sparse embedding for query").
- `HybridSearchBuilder::default()` sets `top_k` and `top_n` to 0. Start from `HybridSearch::default()`, which sets both to 10.
- Qdrant only implements hybrid search for `HybridSearch<Filter>`. Write that type in signatures, as docs-qa does, even without a filter.
- In 0.33, Qdrant returns each document's content `Debug`-quoted: wrapped in `"` with escaped newlines. docs-qa wraps the retriever in `CleanQdrant` ([Qdrant content workaround](/showcases/docs-qa/#qdrant-content-workaround)), which restores the text before reranking and answering.
- DuckDB hybrid search does not rank before it cuts. Each search takes the first `top_n` rows of the table, unranked, and only rows in both lists get scored. On a table larger than `top_n`, most chunks are never considered. Set `top_n` to at least the row count, or use `SimilaritySingleEmbedding`.
- A small `top_k` hides relevant chunks. In ask-your-codebase, one answer had to infer a detail because the chunk that held it was not among the 20 retrieved. Raise `top_k` for broad questions, or retrieve more and [rerank](/querying/reranking-and-response-transformers/).

## Reference

- [`SimilaritySingleEmbedding`](docsrs:swiftide/query/search_strategies/struct.SimilaritySingleEmbedding.html)
- [`HybridSearch`](docsrs:swiftide/query/search_strategies/struct.HybridSearch.html), [`HybridSearchBuilder`](docsrs:swiftide/query/search_strategies/struct.HybridSearchBuilder.html)
- [`CustomStrategy`](docsrs:swiftide/query/search_strategies/struct.CustomStrategy.html)
- [`Retrieve`](docsrs:swiftide/traits/trait.Retrieve.html)
- [`SearchStrategy`](docsrs:swiftide/traits/trait.SearchStrategy.html)
- [`SparseEmbed`](docsrs:swiftide/indexing/transformers/struct.SparseEmbed.html), [`FastEmbed`](docsrs:swiftide/integrations/fastembed/struct.FastEmbed.html)
- [`query::Pipeline`](docsrs:swiftide/query/struct.Pipeline.html)
- [`Qdrant`](docsrs:swiftide/integrations/qdrant/struct.Qdrant.html), [`PgVector`](docsrs:swiftide/integrations/pgvector/struct.PgVector.html), [`LanceDB`](docsrs:swiftide/integrations/lancedb/struct.LanceDB.html), [`Duckdb`](docsrs:swiftide/integrations/duckdb/struct.Duckdb.html)
- Showcases: [ask-your-codebase](/showcases/ask-your-codebase/), [docs-qa](/showcases/docs-qa/)

## Next

[Reranking and response transformers](/querying/reranking-and-response-transformers/)
