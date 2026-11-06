---
title: "Loading data"
description: "Start an indexing pipeline from files on disk, nodes you build in code, a tool executor, web pages, Kafka, Fluvio, Parquet or a custom Loader."
sidebar:
  order: 2
identLint:
  ignore: [kafka_topic, kafka_partition, kafka_offset, kafka_timestamp, kafka_key, fluvio_key]
---

Every indexing pipeline starts with a stream of nodes. You fill that stream from files, web pages, Kafka, Fluvio, Parquet or your own data, with a loader and `Pipeline::from_loader` or with any stream and `Pipeline::from_stream`.

## Files

::snippet{src="@snippets/src/indexing/loading.rs" anchor="file_loader"}

`FileLoader::new` takes a directory or a single file. Its walk uses the `ignore` crate's defaults. It skips hidden files and files excluded by `.ignore` files, and by `.gitignore` rules when the root is inside a git repository. `with_extensions` keeps only files with those extensions. Without any call the loader yields every file.

`SupportedLanguages::file_extensions` returns the extensions tree-sitter knows for a language. It needs the `tree-sitter` feature flag. `FileLoader` itself needs none.

Each file becomes one `TextNode`:

- `chunk` holds the file content, with invalid UTF-8 replaced.
- `path` is relative when the root is relative. The node id hashes path and chunk, so changing the root from relative to absolute misses the [node cache](/indexing/caching/).
- A file that cannot be read becomes an error in the stream. The first error ends `run` unless you filter errors, see [Errors](/indexing/streams-and-concurrency/#errors).

## Nodes in memory

::snippet{src="@snippets/src/indexing/loading.rs" anchor="from_nodes"}

`Pipeline::from_stream` takes anything that converts into an `IndexingStream`. That includes a `Vec<TextNode>`, a boxed `Stream` of results and a Tokio `mpsc` receiver.

A channel lets another task produce nodes while the pipeline runs:

::snippet{src="@snippets/src/indexing/loading.rs" anchor="from_channel"}

The stream ends when every sender is dropped. Until then, `run` keeps waiting for nodes.

## Files from a tool executor

::snippet{src="@snippets/src/indexing/loading.rs" anchor="executor_files"}

`ToolExecutor::stream_files` returns the files below a path as an `IndexingStream`. `LocalExecutor` uses `path` as given and does not resolve it against the executor's working directory. `LocalExecutor` needs the `swiftide-agents` feature flag.

## Web pages

::snippet{src="@swiftide/examples/scraping_index_to_markdown.rs" anchor="pipeline" title="examples/scraping_index_to_markdown.rs"}

`ScrapingLoader` crawls with the `spider` crate and needs the `scraping` feature flag. `ScrapingLoader::from_spider` takes a configured `Website`, here limited to one page. To configure it, add `spider` at the version swiftide uses (2.52), or the `Website` types do not match.

Each node carries the raw HTML of one page, with the page URL as its `path`. `HtmlToMarkdownTransformer` converts the HTML to markdown and drops `script` and `style` tags.

### Spider Cloud

Since 0.33, `ScrapingLoader` crawls through Spider Cloud when `SPIDER_CLOUD_API_KEY` is set and not empty. It reads these environment variables once per process:

| Variable | Effect |
| --- | --- |
| `SPIDER_CLOUD_API_KEY` | Turns Spider Cloud on |
| `SPIDER_CLOUD_MODE` | `proxy`, `api`, `unblocker` or `fallback`. Anything else selects smart mode, which uses the unblocker only against bot protection. |
| `SPIDER_CLOUD_API_URL` | Optional API endpoint |

Spider Cloud cannot reach hosts that only your machine can route to. A start URL on `localhost`, a `.localhost` or `.local` name, a loopback address, or a private or link-local IPv4 address is crawled directly.

## Kafka and Fluvio

::snippet{src="@swiftide/examples/kafka.rs" anchor="setup" title="examples/kafka.rs"}

::snippet{src="@swiftide/examples/kafka.rs" anchor="pipeline" title="examples/kafka.rs"}

The pipelines embed with `FastEmbed`, which needs the `fastembed` feature flag. `Kafka` needs the `kafka` feature flag, which builds librdkafka and needs cmake and a C toolchain. It takes an `rdkafka` `ClientConfig`, re-exported as `integrations::kafka::ClientConfig`. As a loader it subscribes to `topic` and turns each message payload into a node. Messages without a UTF-8 payload are skipped. The node metadata gets `kafka_topic`, `kafka_partition` and `kafka_offset`, plus `kafka_timestamp` and `kafka_key` when the message has them. `Kafka` is also a storage, here writing to a second topic.

::snippet{src="@swiftide/examples/fluvio.rs" anchor="setup" title="examples/fluvio.rs"}

`Fluvio` needs the `fluvio` feature flag and is a loader only. It takes a `ConsumerConfigExt` with the topic, partition and start offset. `Offset` comes from the `fluvio` crate, which you add at swiftide's version (0.50). Each record becomes a node with its key in the `fluvio_key` metadata field, which is null when the record has no key.

A Kafka pipeline waits for new messages, so `run` does not return on its own. For both loaders, a receive error ends the run.

## Parquet

::snippet{src="@snippets/src/indexing/loading.rs" anchor="parquet"}

`Parquet` needs the `parquet` feature flag. `batch_size` sets the rows read per batch and defaults to 1024. The loader reads one column and turns every non-null value into a node. The column must hold strings. A column of another type yields an error, and a missing column panics. Batches are flattened unordered, so node order can differ from row order. Each load prints the column descriptors to stderr.

## Custom loaders

When no loader fits, implement `Loader`. The [docs-qa](/showcases/docs-qa/) showcase loads markdown and `llms-full.txt` sources as one node per page:

::snippet{src="@swiftide/examples/docs-qa/src/loader.rs" anchor="loader" title="examples/docs-qa/src/loader.rs"}

- `type Output` is the chunk type of the nodes. Built-in transformers expect `String`.
- `into_stream` is synchronous and consumes the loader. Put slow work inside the stream, like the page downloads this loader starts through `DocsSource::pages`, so it runs when the pipeline polls it.
- A failing source becomes an `Err` item, not a panic.
- `into_stream_boxed` is only needed when you use the loader as a `Box<dyn Loader>`. Its default implementation panics.
- `Loader` requires `Send`, `Sync` and `DynClone`, which any `Clone` type implements.

## Gotchas

- `Pipeline::from_loader` calls `into_stream` right away, so work starts when you build the pipeline, not when you call `run`. `Kafka` subscribes to its topic. `Fluvio` connects. `Parquet` opens the file. `ScrapingLoader` starts crawling, and panics outside a Tokio runtime.
- `Fluvio` and `Parquet` use `tokio::task::block_in_place`, which panics on a current-thread runtime. `#[tokio::main]` uses a multi-thread runtime by default; `#[tokio::test]` does not, so use `#[tokio::test(flavor = "multi_thread")]`.
- `Kafka`, `Fluvio` and `Parquet` panic when they cannot create their consumer or open their file.

## Feature flags

Install the feature flags you need, for example:

```sh
cargo add swiftide@{{swiftide_version}} --features tree-sitter,scraping,kafka,fluvio,parquet,fastembed
```

[Feature flags](/reference/feature-flags/) lists them all.

## Reference

- [`FileLoader`](docsrs:swiftide/indexing/loaders/struct.FileLoader.html)
- [`Loader`](docsrs:swiftide/traits/trait.Loader.html)
- [`IndexingStream`](docsrs:swiftide/indexing/struct.IndexingStream.html)
- [`Pipeline::from_loader`](docsrs:swiftide/indexing/struct.Pipeline.html#method.from_loader)
- [`Pipeline::from_stream`](docsrs:swiftide/indexing/struct.Pipeline.html#method.from_stream)
- [`TextNode`](docsrs:swiftide/indexing/type.TextNode.html)
- [`SupportedLanguages`](docsrs:swiftide/integrations/treesitter/enum.SupportedLanguages.html)
- [`ToolExecutor`](docsrs:swiftide/traits/trait.ToolExecutor.html)
- [`LocalExecutor`](docsrs:swiftide/agents/tools/local_executor/struct.LocalExecutor.html)
- [`ScrapingLoader`](docsrs:swiftide/integrations/scraping/struct.ScrapingLoader.html)
- [`HtmlToMarkdownTransformer`](docsrs:swiftide/integrations/scraping/struct.HtmlToMarkdownTransformer.html)
- [`Kafka`](docsrs:swiftide/integrations/kafka/struct.Kafka.html)
- [`Fluvio`](docsrs:swiftide/integrations/fluvio/struct.Fluvio.html)
- [`Parquet`](docsrs:swiftide/integrations/parquet/struct.Parquet.html)

## Next

[Chunking](/indexing/chunking/)
