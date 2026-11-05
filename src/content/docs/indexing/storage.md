---
title: "Storage"
description: "Store indexed nodes with then_store_with in LanceDB, Qdrant, PgVector, DuckDB, Redis, Kafka or MemoryStorage, and write your own storage by implementing the Persist trait."
sidebar:
  order: 7
identLint:
  ignore: [last_updated_at, RefCell, swiftide_cache]
---

`then_store_with` writes every node to a storage backend. You end up with nodes in LanceDB or Qdrant, a clear picture of what each backend keeps of a node, and a storage of your own built on the `Persist` trait.

## LanceDB

The ask-your-codebase showcase keeps its vectors in an embedded LanceDB table, in a directory next to the code. There is no server to run. Enable the `lancedb` feature flag:

```sh
cargo add swiftide@{{swiftide_version}} --features lancedb
```

::snippet{src="@swiftide/examples/ask-your-codebase/src/codebase.rs" anchor="storage" title="examples/ask-your-codebase/src/codebase.rs"}

`vector_store` builds the storage; `node_cache` belongs to [Caching](/indexing/caching/). The showcase pipeline passes the storage to `then_store_with`, after the embedding step.

- `uri` is the directory that holds the table. It is required unless you pass a `connection_pool`.
- `table_name` defaults to `swiftide`.
- `vector_size` must match the output of your embedder. The showcase uses 768, the size of its FastEmbed code model.
- `with_vector(EmbeddedField::Combined)` adds a column for the embedding of the chunk together with its metadata.
- `with_metadata(PATH_FIELD)` adds a column for the `path` metadata field.

LanceDB stores only what you configure: the node id, the chunk, each configured vector and each metadata field named with `with_metadata`. The node's own `path` is not a column. The showcase stores the path relative to the codebase root as metadata, so answers can cite their sources. Metadata columns hold text, so a value that is not a JSON string is stored as null.

Before the first node arrives, setup creates the table if it does not exist. Nodes are written in batches of 256 with a merge on `id`, so storing the same node twice updates its row. See [LanceDB](/integrations/lancedb/) for retrieval and remote storage.

## Qdrant

The docs-qa showcase stores a dense and a sparse vector for every chunk in a Qdrant collection. Enable the `qdrant` feature flag:

```sh
cargo add swiftide@{{swiftide_version}} --features qdrant
```

::snippet{src="@swiftide/examples/docs-qa/src/store.rs" anchor="store" title="examples/docs-qa/src/store.rs"}

Qdrant keeps more of the node than LanceDB. Each point's payload holds every metadata field, plus `path`, `content` (the chunk) and `last_updated_at`. The point id is the node id. Nodes without vectors fail with "Node without vectors".

Setup creates the collection when it is missing. An existing collection is kept as it is, even when its vector configuration differs from the builder. The client reads `QDRANT_URL` (default `http://localhost:6334`, gRPC) and `QDRANT_API_KEY`. The default batch size is 50; docs-qa raises it to 64.

A debug build of docs-qa split four markdown pages into 43 chunks and stored them in Qdrant (metadata from `claude-haiku-5-5`, embeddings from local FastEmbed):

```text
Indexed 43 chunks into `docs-qa` in 71.7s
```

[Qdrant](/integrations/qdrant/) covers named vectors, hybrid search and filters.

## Backends

Every backend sits behind its own feature flag, except `MemoryStorage`:

| Storage | Feature flag | What it stores | Default batch size | Retrieval |
| --- | --- | --- | --- | --- |
| `LanceDB` | `lancedb` | id, chunk, configured vectors, `with_metadata` fields | 256 | yes |
| `Qdrant` | `qdrant` | vectors; payload with all metadata, path, chunk | 50 | yes |
| `PgVector` | `pgvector` | id, chunk, configured vectors, `with_metadata` fields as JSONB | 50 | yes |
| `Duckdb` | `duckdb` | `uuid`, chunk, path, configured vectors; no metadata | none, node by node | yes |
| `Redis` | `redis` | the node as JSON under `<path>:<node id>` | 10 | no |
| `Kafka` | `kafka` | the node as JSON, keyed `<path>:<node id>`, on a topic | 32 | no |
| `MemoryStorage` | none | the whole node, in memory | none, node by node | no |

PgVector fails to store a node that lacks a field named with `with_metadata`; see [PgVector](/integrations/pgvector/). Kafka's setup fails when the topic does not exist, unless you set `create_topic_if_not_exists`. Kafka is also a loader; see [Kafka and Fluvio](/indexing/loading/#kafka-and-fluvio).

DuckDB stores node by node and ignores its builder's `batch_size`. It rejects a node that lacks a vector for a configured field. When a row with the same `uuid` exists, DuckDB keeps the old row unless you set `upsert_vectors`, which needs DuckDB 1.2.1 or later.

Redis works as a key-value store for nodes. `persist_key_fn` changes the key and `persist_value_fn` the value. Both take a plain `fn` pointer, so a closure that captures variables does not compile. The example writes only the metadata and raises the batch size from 10 to 50:

::snippet{src="@swiftide/examples/index_into_redis.rs" anchor="pipeline" title="examples/index_into_redis.rs"}

### DuckDB

No swiftide example uses DuckDB, so this section describes the builder without code. `Duckdb::builder()` returns a `DuckdbBuilder` with these methods:

- `connection` takes anything that converts into a `duckdb::Connection`. Swiftide does not re-export the `duckdb` crate, so add it to your crate yourself.
- `with_vector(field, size)` adds a vector column of `size` floats for an `EmbeddedField`, such as `EmbeddedField::Combined`.
- `table_name` defaults to `swiftide`, and `cache_table` to `swiftide_cache`. `cache_key_prefix` prefixes the node cache keys, which also lets you invalidate the cache by hand.
- `schema` replaces the SQL that loads the extensions and creates the table. The upsert statement is built from the table name and vectors, so keep the column names when you change it. `upsert_vectors` is the flag described above.

`Duckdb` implements `Persist` for `then_store_with`, `NodeCache` for `filter_cached`, and `Retrieve` for three search strategies: `SimilaritySingleEmbedding`, `HybridSearch` and `CustomStrategy<String>`. The setup installs and loads the `vss` and `fts` extensions before it creates the table. Retrieved documents carry `id` and `path` as metadata, since no other metadata is stored. See [`DuckdbBuilder`](docsrs:swiftide/integrations/duckdb/struct.DuckdbBuilder.html) for every method.

## MemoryStorage

`MemoryStorage` lives in `swiftide::indexing::persist` and needs no feature flag. It suits tests and experiments:

::snippet{src="@snippets/src/indexing/storage.rs" anchor="memory_storage"}

Its keys are an incrementing counter, not the node id, so storing the same node twice keeps two copies. `get(key)` reads one node, `get_all_values()` returns all nodes, and `get_all()` returns them with their keys. It has no retrieval for query pipelines.

## Batching

A storage chooses how it receives nodes through `batch_size`. When it returns `Some(n)`, the pipeline groups up to `n` nodes and calls `batch_store`. When it returns `None`, the pipeline calls `store` for each node. In both cases, up to the pipeline's concurrency of these calls run at once. Storage has no concurrency setting of its own, and [Concurrency](/indexing/streams-and-concurrency/#concurrency) explains when the pipeline value is read. For how this differs from the embedder's batch size, see [Batch sizes](/indexing/embedding/#batch-sizes).

Stored nodes flow on, so you can chain a second `then_store_with` to write to two backends. If `setup` fails, for example because Qdrant is unreachable, `run` returns that error. To keep going after a failed write, see [Errors](/indexing/streams-and-concurrency/#errors).

## Custom storage

Implement `Persist` to write nodes anywhere else. This one appends each node as a line of JSON:

::snippet{src="@snippets/src/indexing/storage.rs" anchor="custom_persist"}

- `Input` is the chunk type the storage accepts, and `Output` the chunk type it passes on. Both are `String` for text nodes.
- `batch_store` returns an `IndexingStream`. Return the stored nodes so later steps receive them, or an error.
- `Persist` requires `Debug`, `Clone`, `Send` and `Sync`, so fields like `Rc` or `RefCell` do not compile. Implement it with `#[async_trait]`.

Batches are stored concurrently, so guard anything they share. Use the storage like any other:

::snippet{src="@snippets/src/indexing/storage.rs" anchor="use_custom_persist"}

## Re-indexing

A node's id is computed from its path and its chunk, as [Cache keys](/indexing/caching/#cache-keys) explains. When you index an unchanged chunk again, LanceDB, Qdrant, PgVector and DuckDB find the same id and keep one row. DuckDB keeps the old row as it is unless `upsert_vectors` is set. When a chunk changes, it gets a new id and a new row, and the row of the old version stays. Swiftide does not delete rows for you.

docs-qa's `--recreate` flag drops the Qdrant collection before indexing. For incremental updates, delete a file's rows yourself before you index its new version, and skip unchanged files with a [node cache](/indexing/caching/).

## Reference

- [`Persist`](docsrs:swiftide/traits/trait.Persist.html)
- [`Pipeline::then_store_with`](docsrs:swiftide/indexing/struct.Pipeline.html#method.then_store_with)
- [`IndexingStream`](docsrs:swiftide/indexing/struct.IndexingStream.html)
- [`MemoryStorage`](docsrs:swiftide/indexing/persist/struct.MemoryStorage.html)
- [`LanceDB`](docsrs:swiftide/integrations/lancedb/struct.LanceDB.html)
- [`Qdrant`](docsrs:swiftide/integrations/qdrant/struct.Qdrant.html)
- [`PgVector`](docsrs:swiftide/integrations/pgvector/struct.PgVector.html)
- [`Duckdb`](docsrs:swiftide/integrations/duckdb/struct.Duckdb.html)
- [`Redis`](docsrs:swiftide/integrations/redis/struct.Redis.html)
- [`Kafka`](docsrs:swiftide/integrations/kafka/struct.Kafka.html)
- [`EmbeddedField`](docsrs:swiftide/indexing/enum.EmbeddedField.html)
- [ask-your-codebase source](https://github.com/bosun-ai/swiftide/tree/{{swiftide_sha}}/examples/ask-your-codebase)
- [docs-qa source](https://github.com/bosun-ai/swiftide/tree/{{swiftide_sha}}/examples/docs-qa)

## Next

[Streams and concurrency](/indexing/streams-and-concurrency/)
