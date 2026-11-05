---
title: "Caching"
description: "Make re-indexing incremental with a Redb, Redis or DuckDB node cache that skips files whose path and content have not changed."
sidebar:
  order: 6
identLint:
  ignore: [swiftide_cache, create_dir_all]
---

Add a node cache to your indexing pipeline and a second run skips every file whose path and content have not changed, before any large language model (LLM) call or embedding is made.

## Redb cache

The [ask-your-codebase](/showcases/ask-your-codebase/) showcase keeps its node cache in an embedded Redb file next to the code it indexes:

::snippet{src="@swiftide/examples/ask-your-codebase/src/codebase.rs" anchor="storage" title="examples/ask-your-codebase/src/codebase.rs" mark="1-9"}

`Redb::builder()` takes a `database_path` and creates the file if it does not exist. It does not create missing parent directories, so the showcase calls `create_dir_all` first. You can pass an open `redb::Database` with `database` instead of a path. `table_name` defaults to `swiftide`. The second function builds the LanceDB storage, covered on [Storage](/indexing/storage/).

The pipeline adds the cache with `filter_cached`:

::snippet{src="@swiftide/examples/ask-your-codebase/src/index.rs" anchor="pipeline" title="examples/ask-your-codebase/src/index.rs" mark="5-7,9-10"}

`filter_cached` drops every node the cache knows. Every other node continues to the chunker, the LLM and the embedder.

The first run indexes the `swiftide-agents` crate from scratch, with `claude-haiku-5-5` writing the questions and FastEmbed computing the embeddings, as described in [Output](/indexing/overview/#output):

```text
$ ./target/debug/ask-your-codebase index swiftide-agents --model claude-haiku-5-5
Indexing Rust files in swiftide-agents ...
Indexed 34 chunks from 14 new or changed files (14 files found, 0 unchanged) in 91.9s
```

The second run, with no file changed:

```text
$ ./target/debug/ask-your-codebase index swiftide-agents --model claude-haiku-5-5
Indexing Rust files in swiftide-agents ...
Indexed 0 chunks from 0 new or changed files (14 files found, 14 unchanged) in 0.6s
```

The cache skips all 14 files, so no prompts are sent and nothing is embedded.

## Cache keys

The cache key is a node id: a UUID (universally unique identifier) computed from the node's path and chunk, not its metadata. The built-in chunkers set `parent_id` on every chunk to the id of the node that entered the pipeline, the first ancestor, even across several chunkers. Caches use `parent_id` when it is set. One key therefore covers a file and all of its chunks.

Each backend puts a prefix in front of the id:

| Backend | Key format | Prefix setting |
| --- | --- | --- |
| Redb | `<prefix>.<uuid>` | `cache_key_prefix`, empty by default |
| Redis | `<prefix>:<uuid>` | second argument of `Redis::try_from_url`, or `cache_key_prefix` on `Redis::builder()`, empty by default |
| DuckDB | `<prefix>.<uuid>` | `cache_key_prefix`, empty by default |

The pipeline records the id a node has when it passes `filter_cached`. At the end it marks the `parent_id` of each finished node, or its own id when `parent_id` is not set. The file is cached only when the two ids match. Three things break the match silently.

A step after `filter_cached` that rewrites `chunk` or `path` in place changes the node's id. The file is then never marked and is processed again on every run, with or without a chunker. Keep content-changing steps in front of `filter_cached`, like the showcase does with `test_code::without_unit_tests`. Steps that only add metadata are safe after it.

A custom chunker must keep the parent. The built-in chunkers build chunks with `Node::into_chunks`, which sets `parent_id`. A closure or `ChunkerTransformer` that creates fresh nodes with `TextNode::new` or `Node::builder` leaves `parent_id` empty, so the file is never marked. Derive chunks with `into_chunks` or `Node::build_from_other`.

The path is part of the id. The same file loaded through `./src` and through an absolute path has two keys. The showcase canonicalizes the codebase root for this reason.

## When nodes are cached

Since 0.33, `run` marks a file in the cache only after the stream has completed and every chunk of that file reached the end. A file with a failed chunk is left out and retried on the next run.

Two consequences follow:

- If `run` returns an error, nothing from that run is cached. The same holds when the process stops before `run` returns.
- A source whose chunker produces zero chunks never reaches the end of the stream. It is never cached, and every step up to the chunker runs for it again on each run.

To keep progress when some nodes fail, log and drop the errors so that `run` completes. This snippet needs the `redis`, `qdrant` and `openai` feature flags:

::snippet{src="@snippets/src/indexing/caching.rs" anchor="partial_progress"}

The file behind a failed chunk stays out of the cache and is retried next run. Every other file is cached. [Errors](/indexing/streams-and-concurrency/#errors) covers where to place `log_errors` and `filter_errors`.

## Backends

| Backend | Feature flag | Node type | Notes |
| --- | --- | --- | --- |
| [`Redb`](docsrs:swiftide/integrations/redb/struct.Redb.html) | `redb` | `TextNode` | Embedded file, no service to run |
| [`Redis`](docsrs:swiftide/integrations/redis/struct.Redis.html) | `redis` | any chunk type | Shared between machines; also a storage |
| [`Duckdb`](docsrs:swiftide/integrations/duckdb/struct.Duckdb.html) | `duckdb` | any chunk type | Cache table `swiftide_cache` by default; also a storage |

Turn on the feature flag for the backend you pick:

```sh
cargo add swiftide@{{swiftide_version}} --features redb
```

`Redis::try_from_url` returns a cache for `String` chunks. For another chunk type, use `Redis::try_build_from_url` or `Redis::builder()`.

With Redis, the cache is one line. This example also needs the `qdrant`, `openai` and `tree-sitter` feature flags:

::snippet{src="@swiftide/examples/index_codebase.rs" anchor="pipeline" title="examples/index_codebase.rs" mark="2"}

`MetadataQACode` runs before chunking here. That is safe, because it only adds metadata and the id does not change.

No backend fails the pipeline on a cache error. `get` logs the error and reports the node as not cached, so an unreachable Redis means every node is processed again.

## Custom cache

To write your own cache, implement [`NodeCache`](docsrs:swiftide/traits/trait.NodeCache.html). This one keeps the ids in memory:

::snippet{src="@snippets/src/indexing/caching.rs" anchor="custom_cache"}

Implement it with `#[async_trait]`, declare `type Input` as the chunk type, and make the type `Send`, `Sync`, `Debug` and `Clone`. `get` returns `bool`, and `set` and `set_by_id` return nothing, so log errors instead of returning them.

Key on `parent_id`, falling back to `id()`, so that `get`, `set` and `set_by_id` agree. The pipeline calls `set_by_id` with the source id when it marks a file. It is a required method since 0.33, so existing custom caches must add it. `clear` is optional, and its default implementation panics.

## Cache invalidation

`NodeCache::clear` empties a cache, but each backend does it differently:

- Redb deletes its table.
- DuckDB drops its cache table and does not recreate it. Every later lookup on the same `Duckdb` logs an error and misses, so build a new `Duckdb` before the next run.
- Redis sends `DEL` with the pattern `<prefix>*`. Redis treats that as a literal key name, so the call removes nothing. With an empty prefix, `clear` returns an error instead. Change the prefix, for example from `docs-v1` to `docs-v2` in the partial-progress snippet.

A new prefix works on every backend. Use it to index everything again, for example after you change a prompt or the embedding model.

Clearing the cache does not touch the storage. A changed file's new chunks get new ids and are stored next to the old ones, which stay in LanceDB until you delete the `.ask-your-codebase/` directory.

## Reference

- [`NodeCache`](docsrs:swiftide/traits/trait.NodeCache.html)
- [`Pipeline::filter_cached`](docsrs:swiftide/indexing/struct.Pipeline.html#method.filter_cached)
- [`Pipeline::log_errors`](docsrs:swiftide/indexing/struct.Pipeline.html#method.log_errors)
- [`Pipeline::filter_errors`](docsrs:swiftide/indexing/struct.Pipeline.html#method.filter_errors)
- [`Node`](docsrs:swiftide/indexing/struct.Node.html)
- [`Redb`](docsrs:swiftide/integrations/redb/struct.Redb.html)
- [`Redis`](docsrs:swiftide/integrations/redis/struct.Redis.html)
- [`Duckdb`](docsrs:swiftide/integrations/duckdb/struct.Duckdb.html)
- [Feature flags](/reference/feature-flags/)
- [ask-your-codebase source](https://github.com/bosun-ai/swiftide/tree/e09a3e90d3eecdaffa08818a661efc3ecec08321/examples/ask-your-codebase)

## Next

[Storage](/indexing/storage/)
