---
title: "How indexing works"
description: "An indexing pipeline streams nodes from a loader through filters, transformers, chunkers and embedders into storage, and the pipeline does its work only when you await run()."
sidebar:
  order: 1
identLint:
  ignore: [downcast_ref]
---

You can read a Swiftide indexing pipeline line by line and know when each step runs. The example comes from the [ask-your-codebase](/showcases/ask-your-codebase/) showcase. It prepares the source files of a codebase for retrieval-augmented generation (RAG): it splits them into chunks, describes each chunk with a large language model (LLM), embeds it, and stores it in LanceDB.

## The ask-your-codebase pipeline

::snippet{src="@swiftide/examples/ask-your-codebase/src/index.rs" anchor="pipeline" title="examples/ask-your-codebase/src/index.rs"}

![Indexing pipeline](/diagrams/diagram-pipeline.svg)

Each node passes through the steps from top to bottom until a step drops it:

1. `FileLoader` walks the codebase root and emits one node per file with a matching extension.
2. `filter` drops test files. The closure sees each `Result`, so errors pass through and still surface.
3. `then(test_code::without_unit_tests)` cuts unit tests out of Rust files.
4. `filter_cached` drops files whose path and content did not change since the last successful run. `codebase.node_cache()` is a showcase helper that builds a `Redb` node cache, see [Caching](/indexing/caching/).
5. `then(record_relative_path(..))` stores the file path, relative to the root, as metadata.
6. `then_chunk` splits each file along syntax boundaries with tree-sitter. `ChunkByLanguage` is the showcase's own wrapper that picks a Swiftide `ChunkCode` per language. One file becomes several nodes.
7. `MetadataQACode` asks the LLM which questions each chunk answers and stores them as metadata.
8. `then_in_batch` with `Embed` embeds each chunk and its metadata with FastEmbed, on the central processing unit (CPU). `with_batch_size` sets 32 nodes per call. Without it, a batch holds 256 nodes.
9. `then_store_with` writes each node to storage. `codebase.vector_store()` builds a `LanceDB` store, see [Storage](/indexing/storage/).
10. `run().await` drives the stream to the end and returns the run's statistics as `stats`.

## Nodes

A node is a `Node<T>`. It carries one piece of content and everything known about it.

| Field | Holds |
| --- | --- |
| `path` | The source path. |
| `chunk` | The content, of type `T`. |
| `metadata` | Key and value pairs, sorted by key. |
| `vectors`, `sparse_vectors` | Embeddings, one per embedded field. `Embed` and `SparseEmbed` fill them. |
| `embed_mode` | Whether the chunk and metadata are embedded together, each on its own, or both. |
| `offset` | A running position that `ChunkCode` counts from 0 over the chunks it keeps. |
| `parent_id` | The `id()` of the source node when the first chunker split it. |

Since 0.31 the pipeline is generic. A `Pipeline<T>` streams `Node<T>`. A chunk is any type that implements the `Chunk` marker trait, which requires `Clone + Send + Sync + Debug + AsRef<[u8]> + 'static`. Most built-in steps take `TextNode`, which is `Node<String>`.

`id()` returns a universally unique identifier (UUID) computed from the path and the chunk, so it changes when either changes. The node cache keys on this id, see [Cache keys](/indexing/caching/#cache-keys).

## Steps

| Step | Method | Trait |
| --- | --- | --- |
| loader | `Pipeline::from_loader`, `Pipeline::from_stream` | `Loader` |
| filter | `filter` | a closure over `&Result<Node<T>>` |
| node cache | `filter_cached` | `NodeCache` |
| transformer | `then` | `Transformer`, or a closure |
| chunker | `then_chunk`, `then_expand` | `ChunkerTransformer` |
| batch transformer | `then_in_batch` | `BatchableTransformer` |
| embedder | `then_in_batch(Embed::new(model))` | `EmbeddingModel` |
| storage | `then_store_with` | `Persist` |

A chunker returns a stream of nodes for each node. `then_expand` behaves like `then_chunk` with a different name in traces. Use it when you split a node for a reason other than chunking.

A step can change the node type. `then` with a transformer whose `Output` is `U` turns a `Pipeline<T>` into a `Pipeline<U>`.

Nodes keep flowing after storage.

## Execution

The pipeline methods only describe the stream, but their arguments run while you build the chain. In the snippet, `codebase.node_cache()?` creates the Redb file and `embedding::local_model()?` loads the FastEmbed model, which downloads it on first use.

`run` takes the pipeline by value and does the work:

1. It calls `setup` on every storage backend. LanceDB creates its table here if it is missing.
2. It pulls nodes through the stream until the stream ends. Some steps spawn Tokio tasks, so `run` needs a Tokio runtime.
3. It marks the source nodes in the node cache, but only after the stream ended without an error.
4. It logs `Pipeline completed` at info level and returns the same numbers as `PipelineStats`, see [Statistics](/indexing/streams-and-concurrency/#statistics).

`run` returns `Result<PipelineStats>`, not the nodes. The showcase reads the number of stored chunks from `stats.nodes_stored`. It counts files with pass-through closures such as `.then(files_found.count())`, because the statistics count nodes at the end of the stream, after chunking.

`then`, `then_chunk`, `then_in_batch` and storage work on several nodes at once and pass them on as they finish, see [Streams and concurrency](/indexing/streams-and-concurrency/). The showcase passes `.concurrency(concurrent_requests)` to `MetadataQACode` to bound requests to the LLM.

### Errors and the node cache

The first error stops the run, and `run` returns it, see [Errors](/indexing/streams-and-concurrency/#errors). It is the error the step failed with, so a `LanguageModelError` can be recovered with `downcast_ref`. A source node is cached only when at least one of its chunks reached the end and none of them failed, and [When nodes are cached](/indexing/caching/#when-nodes-are-cached) covers the consequences.

## Output

Two runs on the `swiftide-agents` crate. `claude-haiku-5-5` wrote the questions, through an OpenAI-compatible gateway to Amazon Web Services (AWS) Bedrock. FastEmbed `jina-embeddings-v2-base-code` computed the embeddings.

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

The node cache drops all 14 files at step 4, so no prompt is sent.

## Feature flags

`Pipeline`, `FileLoader` and `Embed` need no feature flag. The showcase turns off the defaults and turns on `tree-sitter`, `fastembed`, `lancedb`, `redb` and `openai`. To add the same features to your crate:

```sh
cargo add swiftide@{{swiftide_version}} --no-default-features --features tree-sitter,fastembed,lancedb,redb,openai
```

TLS needs no setup, because swiftide's HTTP clients bring their own rustls crypto provider ([TLS](/integrations/overview/#tls)). [Feature flags](/reference/feature-flags/) lists every flag.

## Reference

- [`indexing::Pipeline`](docsrs:swiftide/indexing/struct.Pipeline.html)
- [`PipelineStats`](docsrs:swiftide/indexing/statistics/struct.PipelineStats.html)
- [`Node`](docsrs:swiftide/indexing/struct.Node.html)
- [`TextNode`](docsrs:swiftide/indexing/type.TextNode.html)
- [`EmbedMode`](docsrs:swiftide/indexing/enum.EmbedMode.html)
- [`FileLoader`](docsrs:swiftide/indexing/loaders/struct.FileLoader.html)
- [`Loader`](docsrs:swiftide/traits/trait.Loader.html)
- [`Transformer`](docsrs:swiftide/traits/trait.Transformer.html)
- [`ChunkerTransformer`](docsrs:swiftide/traits/trait.ChunkerTransformer.html)
- [`BatchableTransformer`](docsrs:swiftide/traits/trait.BatchableTransformer.html)
- [`EmbeddingModel`](docsrs:swiftide/traits/trait.EmbeddingModel.html)
- [`Persist`](docsrs:swiftide/traits/trait.Persist.html)
- [`NodeCache`](docsrs:swiftide/traits/trait.NodeCache.html)
- [`Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html)
- [`SparseEmbed`](docsrs:swiftide/indexing/transformers/struct.SparseEmbed.html)
- [`ChunkCode`](docsrs:swiftide/indexing/transformers/struct.ChunkCode.html)
- [`MetadataQACode`](docsrs:swiftide/indexing/transformers/struct.MetadataQACode.html)
- [`Redb`](docsrs:swiftide/integrations/redb/struct.Redb.html)
- [`LanceDB`](docsrs:swiftide/integrations/lancedb/struct.LanceDB.html)
- [`FastEmbed`](docsrs:swiftide/integrations/fastembed/struct.FastEmbed.html)

## Next

[Loading data](/indexing/loading/)
