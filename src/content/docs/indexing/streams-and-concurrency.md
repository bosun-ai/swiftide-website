---
title: "Streams and concurrency"
description: "Set how many nodes an indexing pipeline processes at once, throttle it, split and merge its stream, and drop failed nodes without losing good ones."
sidebar:
  order: 8
identLint:
  ignore: [downcast_ref, chain]
---

You set how many nodes each step of an indexing pipeline handles at once, how fast they reach a slow step, which steps they pass through, and what happens to the ones that fail. The pipeline is a stream of nodes, and every step is a stage on that stream.

The snippets use `OpenAI`, which needs the `openai` feature flag, and `ChunkCode`, which needs the `tree-sitter` feature flag:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,tree-sitter
```

## Concurrency

::snippet{src="@snippets/src/indexing/streams.rs" anchor="concurrency"}

Each transformer, chunker and storage step spawns a Tokio task per node, or per batch for batch transformers and batch storage, and keeps up to its concurrency of them running. Results leave a step as they finish, so the order of nodes is not preserved.

`with_concurrency` sets the pipeline value. The default is the number of logical cores of the machine. A step reads the value when you add it, so call `with_concurrency` before the steps it should apply to.

A transformer, batch transformer or chunker can bring its own limit through `concurrency()`. When that returns a value, it wins over the pipeline's. The built-in large language model (LLM) transformers and `Embed` set it with `with_concurrency`, as `MetadataQAText` does above. Storage has no such override and always uses the pipeline value.

`throttle` lets at most one node through per duration, at the point where you put it. Here it caps the chunks that reach the LLM at 20 per second, whatever the concurrency.

## Split and merge

::snippet{src="@snippets/src/indexing/streams.rs" anchor="split_merge"}

`split_by` turns one pipeline into two. The predicate gets each item as a `Result`, errors included. Items for which it returns true go to the first pipeline, the rest to the second. Here, errors go to the code branch.

`merge` joins two pipelines with the same node type into one stream. Node cache registrations and recorded failures of both sides are combined, so a `filter_cached` inside either branch still marks its sources and failures in the other branch keep them out of the cache.

Each branch of `split_by` gets its own statistics collector, seeded with a copy of what was collected before the split, so a branch you run on its own reports only its own nodes. A collector handle taken before the split sees neither branch. `merge` keeps the collector of the pipeline you call it on, so `run` after the merge returns one `PipelineStats` for both branches. :unreleased

### Split gotchas

- `split_by` is not lazy. It spawns a task that starts reading the stream at once and sends each item into one of two channels that hold 1000 items each.
- When one branch is not read, its channel fills up and the split stops, which also stalls the other branch. Run both branches together: merge them, or run them at the same time with `tokio::join!`. Running one to completion before starting the other can hang once the waiting branch has 1000 items.
- If a branch is dropped, the split task panics on its next send to it. The panic stays inside that task, and the other branch then ends early without an error.
- `merge` keeps the storage setup of the pipeline you call it on and drops that of the pipeline you pass in. Add storage after the merge, as above, or to the branch you call `merge` on.

## Errors

::snippet{src="@snippets/src/indexing/streams.rs" anchor="errors_and_logging"}

`run` stops at the first error a step returns and returns that error, and a node cache marks nothing from a failed run. The steps above drop failed nodes so the run continues.

The returned error is the one the step failed with, even when a batch step follows it. A `LanguageModelError` from an LLM transformer can be recovered with `downcast_ref` or found in `chain()`, see [Errors, backoff and usage](/models/errors-backoff-and-usage/). :unreleased

- `filter` keeps an item when the closure returns true. It sees nodes and errors, so it can drop specific errors as well as nodes. This one drops empty files and keeps every error.
- `log_errors` logs each error at ERROR level and passes it on.
- `filter_errors` drops errors without logging them.
- `log_nodes` logs each node at DEBUG level. `log_all` is `log_errors` followed by `log_nodes`.

Order matters. `filter_errors` before `log_errors` leaves nothing to log. Each pair covers only the steps before it, and errors from steps added after it still abort the run. Add another pair after `then_store_with` if a failed write should not stop the run.

### Errors before a batch step

A batch step collects nodes until it has a full batch. Those are `then_in_batch`, and `then_store_with` when the storage has a batch size. When an error reaches it, the step passes the error on and drops the nodes it has collected so far, without a trace. A `filter_errors` after the batch step keeps the run going, but those good nodes are already gone. With a batch size of 64, one failed LLM call can drop up to 63 good chunks.

Those dropped nodes are not recorded as failed. When other chunks of the same source complete, the node cache marks that source and the next run skips it. Put a `filter_errors` pair before every batch step, as the snippet does before `then_in_batch`.

The source of a node that fails in a transformer stays out of the node cache, so the next run retries it. [Caching](/indexing/caching/) covers when sources are marked.

## Statistics

`run` returns the `PipelineStats` of the run, with node counts, token usage and timing, as `let stats = pipeline.run().await?;`. :unreleased It also logs them as the INFO event `Pipeline completed`. `run` takes the pipeline by value, and `stats()` before `run` returns the empty initial statistics. To watch a run while it is going, clone the handle from `stats_collector()` before you call `run`. [Pipeline statistics](/observability/metrics-and-statistics/#pipeline-statistics) shows both.

## Streams

Loaders, chunkers, batch transformers and batch storage return their nodes as an `IndexingStream`. `IndexingStream::iter` takes any iterator of results, `IndexingStream::from_nodes` a `Vec` of nodes, and `IndexingStream::empty` makes an empty stream. Use them to return nodes from your own chunkers and batch transformers.

When you build a pipeline with `Pipeline::from_stream`, a `Result<Vec<..>>` that is an error and an `anyhow::Error` each become a stream with that one error item. A Tokio channel's capacity applies backpressure: `send` waits while the pipeline is behind.

[Loading](/indexing/loading/#nodes-in-memory) shows `from_stream` with a vector and a channel.

## Reference

- [`Pipeline`](docsrs:swiftide/indexing/struct.Pipeline.html) (`with_concurrency`, `throttle`, `then`, `then_chunk`, `then_in_batch`, `then_store_with`, `split_by`, `merge`, `filter`, `filter_cached`, `filter_errors`, `log_all`, `log_errors`, `log_nodes`, `from_stream`, `run`, `stats`, `stats_collector`)
- [`IndexingStream`](docsrs:swiftide/indexing/struct.IndexingStream.html)
- [`Transformer`](docsrs:swiftide/traits/trait.Transformer.html) (`concurrency`)
- [`BatchableTransformer`](docsrs:swiftide/traits/trait.BatchableTransformer.html) (`concurrency`)
- [`ChunkerTransformer`](docsrs:swiftide/traits/trait.ChunkerTransformer.html) (`concurrency`)
- [`FileLoader`](docsrs:swiftide/indexing/loaders/struct.FileLoader.html)
- [`ChunkMarkdown`](docsrs:swiftide/indexing/transformers/struct.ChunkMarkdown.html)
- [`ChunkCode`](docsrs:swiftide/indexing/transformers/struct.ChunkCode.html)
- [`MetadataQAText`](docsrs:swiftide/indexing/transformers/struct.MetadataQAText.html)
- [`Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html)
- [`MemoryStorage`](docsrs:swiftide/indexing/persist/struct.MemoryStorage.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/struct.OpenAI.html)

## Next

[Code-aware indexing](/indexing/code-aware-indexing/)
