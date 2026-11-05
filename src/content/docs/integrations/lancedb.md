---
title: "LanceDB"
description: "Store nodes in an embedded LanceDB table in a local directory and retrieve them with a similarity search, a filter or your own LanceDB query."
sidebar:
  order: 9
identLint:
  ignore: [questions_and_answers__code_, vector_combined, only_if, ExecutableQuery]
---

You end up with a vector store in a directory next to your code, with no database server to run. Swiftide persists nodes into a LanceDB table and retrieves them in a query pipeline. The integration sits behind the `lancedb` feature flag. The code on this page also uses `redb` for the node cache, `fastembed` for the embedder and `tree-sitter` for `MetadataQACode`:

```sh
cargo add swiftide@{{swiftide_version}} --features lancedb,redb,fastembed,tree-sitter
```

## Builder

::snippet{src="@swiftide/examples/ask-your-codebase/src/codebase.rs" anchor="storage" title="examples/ask-your-codebase/src/codebase.rs"}

The ask-your-codebase showcase keeps a `Redb` node cache and a `LanceDB` storage in the same directory. `uri` points at the LanceDB directory, as a Uniform Resource Identifier (URI). You need it unless you pass your own `connection_pool`. Without either, `build` returns the error `URI should be set`.

`table_name` defaults to `swiftide`. `vector_size` is the size of every vector column and must match the embedder, here 768 for a FastEmbed code model. `with_vector` adds a vector column for an embedded field. `with_metadata` adds a column for a metadata field. The storage opens a pool of up to 10 connections, which `pool_size` changes.

The table has these columns:

| Column | Type | Content |
| --- | --- | --- |
| `id` | 16 bytes | The node id, derived from the file path and the chunk |
| `chunk` | text | The chunk |
| one per `with_metadata` | text, nullable | The metadata value |
| `vector_<field>` | list of `f32` | The vector, for example `vector_combined` |

Column names are lowercased, and every character that is not a letter or a digit becomes `_`. The field `MetadataQACode` writes, `Questions and Answers (code)`, becomes the column `questions_and_answers__code_`.

## Indexing pipeline

::snippet{src="@swiftide/examples/ask-your-codebase/src/index.rs" anchor="pipeline" title="examples/ask-your-codebase/src/index.rs"}

`then_store_with` hands the nodes to `LanceDB`, which implements the `Persist` trait. When the pipeline runs, it calls `setup` once. `setup` creates an empty table with the configured columns if the table does not exist, and opens the existing table otherwise.

The storage writes 256 nodes per batch, which `batch_size` changes on the builder. Each batch is a merge insert on `id`. Ids come from the path and the chunk, so changed chunks add rows. See [Storage](/indexing/storage/) for how to clean them up.

## Metadata columns

::snippet{src="@swiftide/examples/ask-your-codebase/src/index.rs" anchor="path_metadata" title="examples/ask-your-codebase/src/index.rs"}

The closure transformer writes a `path` field into the node metadata, and `with_metadata(PATH_FIELD)` gives it a column.

Only string values are stored. A number, a list or an object in that field is written as null, as is a field the node does not have. Metadata without a column is not stored at all. In the showcase, the questions from `MetadataQACode` are embedded with the chunk but have no column, so they never reach the table.

## Similarity search

::snippet{src="@swiftide/examples/ask-your-codebase/src/query.rs" anchor="query_pipeline" title="examples/ask-your-codebase/src/query.rs"}

`LanceDB` implements the `Retrieve` trait for `SimilaritySingleEmbedding`. The search runs a nearest-neighbour query on the single vector column and returns `top_k` rows, 10 by default. `SimilaritySingleEmbedding::default()` is the search without a filter, and `with_top_k` chains onto it. :unreleased

The query needs an embedding, so an `Embed` query transformer runs before `then_retrieve`. Use the same model that embedded the chunks. A query without an embedding fails with `No embedding for query`.

::snippet{src="@swiftide/examples/ask-your-codebase/src/query.rs" anchor="sources" title="examples/ask-your-codebase/src/query.rs"}

Each retrieved document takes its content from the `chunk` column. Every other text column becomes a metadata entry, keyed by the column name. `path` keeps its name. Vector and id columns are left out. A metadata column that holds null for a row comes back as an empty string, not as a missing key.

The showcase's `ask` run, answered by `claude-sonnet-5-5`, listed these `path` values:

```text
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

## Filters and custom queries

`SimilaritySingleEmbedding::from_filter` takes a `String` for LanceDB. Swiftide passes it unchanged to LanceDB's `only_if`, as a Structured Query Language (SQL) filter on the column names. For example, `SimilaritySingleEmbedding::from_filter("path = 'src/agent.rs'".to_string())` searches only the chunks of that file.

For anything else, use `CustomStrategy` with a LanceDB query. `LanceDB` accepts any query that implements LanceDB's `ExecutableQuery` trait, and turns the rows into documents as above.

With `CustomStrategy::from_async_query`, the closure returns a future that awaits `get_connection` or `open_table` and builds the query. `CustomStrategy::from_query` takes a synchronous closure that cannot await, so open the `lancedb::Table` beforehand and capture it.

Swiftide does not re-export the `lancedb` crate. Add it at the version Swiftide uses to name its types:

```sh
cargo add lancedb@0.33 --no-default-features
```

Swiftide's integration test [`swiftide/tests/lancedb.rs`](https://github.com/bosun-ai/swiftide/blob/{{swiftide_sha}}/swiftide/tests/lancedb.rs) builds a custom vector search with `from_async_query`.

## Remote storage

`api_key`, `region` and `storage_options` on the builder go to every LanceDB connection. Swiftide enables the `remote` feature of the `lancedb` crate, so a `db://` URI plus `api_key` and `region` connects to LanceDB Cloud.

Object stores such as Amazon Simple Storage Service (S3) need a storage feature of the `lancedb` crate, for example `aws`, that Swiftide does not enable. Add `lancedb` with that feature to your own `Cargo.toml`, and Cargo combines it with Swiftide's.

## Gotchas

- A vector column without a size panics in `build` with `Vector size should be set either in the field or in the LanceDB builder`. Set `vector_size`.
- `setup` never changes an existing table. After a change to the vector size, the columns or the embedding model, use a new table name or delete the directory. In the showcase, delete `.ask-your-codebase/` to start over.
- `SimilaritySingleEmbedding` needs exactly one vector column. With zero or several, retrieval fails with `Zero or multiple vector fields configured in schema`. There is no `HybridSearch` support.
- LanceDB creates no vector index for you. For large tables, create one through `get_connection`.
- LanceDB's dependencies compile Protocol Buffers definitions at build time, so the build needs `protoc` on your `PATH`. Install it with your package manager, for example `brew install protobuf` or `apt install protobuf-compiler`.

## Reference

- [`LanceDB`](docsrs:swiftide/integrations/lancedb/struct.LanceDB.html)
- [`LanceDBBuilder`](docsrs:swiftide/integrations/lancedb/struct.LanceDBBuilder.html)
- [`EmbeddedField`](docsrs:swiftide/indexing/enum.EmbeddedField.html)
- [`Redb`](docsrs:swiftide/integrations/redb/struct.Redb.html)
- [`Pipeline::filter_cached`](docsrs:swiftide/indexing/struct.Pipeline.html#method.filter_cached)
- [`Pipeline::then_store_with`](docsrs:swiftide/indexing/struct.Pipeline.html#method.then_store_with)
- [`Persist`](docsrs:swiftide/traits/trait.Persist.html)
- [`Retrieve`](docsrs:swiftide/traits/trait.Retrieve.html)
- [`SimilaritySingleEmbedding`](docsrs:swiftide/query/search_strategies/struct.SimilaritySingleEmbedding.html)
- [`CustomStrategy`](docsrs:swiftide/query/search_strategies/struct.CustomStrategy.html)
- [`query::Pipeline`](docsrs:swiftide/query/struct.Pipeline.html)

## Next

[Tree-sitter](/integrations/tree-sitter/)
