---
title: "PgVector"
description: "Store nodes in Postgres with the pgvector extension, search them by cosine distance, filter on a metadata column or run your own SQL."
sidebar:
  order: 8
identLint:
  ignore: [swiftide_pgv_store, vector_combined, vector_chunk]
---

Your indexing pipeline writes nodes into a Postgres table, and your query pipeline finds them by cosine distance. The integration sits behind the `pgvector` feature flag. The excerpts on this page also embed with FastEmbed, prompt through Ollama or use OpenAI, so add the feature flags for the clients you use:

```sh
cargo add swiftide@{{swiftide_version}} --features pgvector,fastembed,ollama,openai
```

## Running Postgres

Any Postgres with the pgvector extension works. Swiftide's own tests use the `pgvector/pgvector:pg17` image:

```sh
docker run -d --name swiftide-pgvector -p 5432:5432 \
  -e POSTGRES_USER=myuser -e POSTGRES_PASSWORD=mypassword -e POSTGRES_DB=mydatabase \
  pgvector/pgvector:pg17
```

The connection URL for this container is `postgresql://myuser:mypassword@localhost:5432/mydatabase`.

## Builder

::snippet{src="@swiftide/examples/index_md_into_pgvector.rs" anchor="setup" title="examples/index_md_into_pgvector.rs"}

`start_postgres` is a test helper from `swiftide-test-utils`. That crate is not published, so you cannot add it as a dependency. It starts the same container through testcontainers and returns its URL. In your application, pass your own URL to `db_url`.

The example uses the `fastembed` and `ollama` feature flags. `MetadataQAText` calls `llama3.2` through a local Ollama, so pull that model first.

`db_url` and `vector_size` are required, and `build()` returns an error without them. `with_vector` is required in practice: without a vector field, setup fails with `No vector field found in configuration`. `vector_size` must match your embedder: FastEmbed's default model returns 384 dimensions. `with_metadata` adds a metadata column. The example stores the questions and answers that `MetadataQAText` generates, under the field `Questions and Answers (text)`.

| Setting | Default |
| --- | --- |
| `table_name` | `swiftide_pgv_store` |
| `batch_size` | 50 nodes per insert |
| `db_max_connections` | 10 |
| `db_max_retry` | 3 connection attempts |
| `db_conn_retry_delay` | 3 seconds |

`build()` does not connect. The connection pool opens on first use, retrying as configured above. Cloning a `PgVector` is cheap, and the clones share that pool.

In an application, read the URL from your configuration:

::snippet{src="@snippets/src/integrations/pgvector.rs" anchor="own_database"}

`1536` matches OpenAI's `text-embedding-3-small`, so build the client with `.default_embed_model("text-embedding-3-small")`. The closure transformer copies the file path into a `path` metadata field, which the [filter](#filters) below uses.

## Table layout

When the indexing pipeline starts, the persist setup runs `CREATE EXTENSION IF NOT EXISTS vector`, creates the table if it is missing, and adds a Hierarchical Navigable Small World (HNSW) index. For the example above, that comes down to:

```sql
CREATE TABLE IF NOT EXISTS swiftide_pgvector_test (
  id UUID NOT NULL,
  chunk TEXT NOT NULL,
  vector_combined VECTOR(384),
  meta_questions_and_answers JSONB,
  PRIMARY KEY (id)
);
CREATE INDEX IF NOT EXISTS swiftide_pgvector_test_embedding_idx
  ON swiftide_pgvector_test USING hnsw (vector_combined vector_cosine_ops);
```

- `id` is the node id, a universally unique identifier (UUID) derived from the node's path and chunk.
- `chunk` holds the chunk text.
- Each `with_vector` field becomes a `vector_<field>` column: `EmbeddedField::Combined` becomes `vector_combined`, `EmbeddedField::Chunk` becomes `vector_chunk`.
- Each `with_metadata` field becomes a `meta_<name>` JSONB column. It holds an object with the original field name as key, here `{"Questions and Answers (text)": "..."}`.

Column names are normalized. Swiftide keeps the text before the first `(`, `[`, `{` or `<`, takes up to three words, joins them with underscores, lowercases them and drops everything except letters, digits and underscores. The table name must start with a letter or underscore, contain only letters, digits, `_` or `$`, and not be a reserved word such as `table`. It can be at most 49 bytes, because setup appends `_embedding_idx` to name the index and Postgres identifiers stop at 63 bytes. Otherwise setup fails with `Invalid table or field name`.

The table is created only when it is missing. An existing table is kept as it is, so a new vector size or a new column means dropping the table first. The example drops it before every run for that reason. `get_pool()` returns the `sqlx` pool for your own Structured Query Language (SQL), such as dropping the table or deleting stale rows, and `get_table_name()` returns the table name.

## Storage

::snippet{src="@swiftide/examples/index_md_into_pgvector.rs" anchor="indexing" title="examples/index_md_into_pgvector.rs"}

`then_store_with` hands nodes to PgVector in batches of `batch_size`. Each batch is one `INSERT ... SELECT FROM UNNEST(...)` statement inside a transaction, with `ON CONFLICT (id) DO UPDATE`. Indexing the same chunk again overwrites its row. A changed chunk gets a new id, so its old row stays until you delete it.

The default embed mode produces one `Combined` vector per node, which matches `with_vector(EmbeddedField::Combined)`.

## Retrieval

::snippet{src="@swiftide/examples/index_md_into_pgvector.rs" anchor="query" title="examples/index_md_into_pgvector.rs"}

`query::Pipeline::default()` uses `SimilaritySingleEmbedding` with a `top_k` of 10. Embed the query with the same model you indexed with. PgVector selects `id`, `chunk` and every metadata column, orders by cosine distance (`<=>`) and returns `top_k` documents. Each document carries its metadata under the original field names.

Retrieval needs exactly one vector column. With more than one, it fails with `Search strategy for multiple vector fields in the schema is not yet implemented`. PgVector has no hybrid search.

## Filters

`SimilaritySingleEmbedding<String>` takes a filter of the form `key = value`:

::snippet{src="@snippets/src/integrations/pgvector.rs" anchor="filter"}

PgVector splits the filter at the first `=`, trims whitespace and strips double quotes around the value. It then adds `WHERE meta_<normalized key> ->> $1 = $2`, binding the original key and the value. A filter is one exact text equality on a `with_metadata` field. The key must match the original field name, and the column must exist. The value must match the stored text exactly: `FileLoader::new("docs")` stores `docs/README.md`, not `README.md`.

Nodes do not carry a `path` metadata field on their own; add one before storing, as in the [own database excerpt](#builder).

## Custom SQL

For anything a filter cannot express, `CustomStrategy<sqlx::QueryBuilder<Postgres>>` runs your own query:

::snippet{src="@snippets/src/querying/retrieval.rs" anchor="pgvector_custom_strategy"}

Query the PgVector table: take its name from `get_table_name()` rather than writing it by hand. PgVector maps each row by column name. `id` and `chunk` are required, and every column that starts with `meta_` becomes metadata. A custom strategy does not read the query embedding, so this pipeline needs no `Embed` step. See [Retrieval and search strategies](/querying/retrieval-and-search-strategies/) for how `CustomStrategy` builds the query and which `sqlx` version to add.

## Gotchas

- Every node must carry every `with_metadata` field. A node without one fails the batch with `Missing metadata field`.
- A node without the vector named in `with_vector` is stored as an empty vector, and Postgres rejects the batch with a dimension error. Keep the embed mode and `with_vector` in sync: `EmbedMode::PerField` produces no `Combined` vector.
- Only the first vector column gets the HNSW index.
- `SimilaritySingleEmbedding` fails with `Missing embedding in query state` when the query pipeline has no `Embed` query transformer.

## Reference

- [`PgVector`](docsrs:swiftide/integrations/pgvector/struct.PgVector.html)
- [`PgVectorBuilder`](docsrs:swiftide/integrations/pgvector/struct.PgVectorBuilder.html)
- [`EmbeddedField`](docsrs:swiftide/indexing/enum.EmbeddedField.html)
- [`Persist`](docsrs:swiftide/traits/trait.Persist.html)
- [`Retrieve`](docsrs:swiftide/traits/trait.Retrieve.html)
- [`SimilaritySingleEmbedding`](docsrs:swiftide/query/search_strategies/struct.SimilaritySingleEmbedding.html)
- [`CustomStrategy`](docsrs:swiftide/query/search_strategies/struct.CustomStrategy.html)
- [`MetadataQAText`](docsrs:swiftide/indexing/transformers/struct.MetadataQAText.html)
- [`indexing::Pipeline`](docsrs:swiftide/indexing/struct.Pipeline.html)
- [`query::Pipeline`](docsrs:swiftide/query/struct.Pipeline.html)

## Next

[LanceDB](/integrations/lancedb/)
