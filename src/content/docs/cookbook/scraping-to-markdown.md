---
title: "Scraping websites to markdown"
description: "Crawl a website with the spider-based ScrapingLoader, convert each page to markdown with HtmlToMarkdownTransformer and split it into markdown chunks."
sidebar:
  order: 9
identLint:
  ignore: [Website, "Website::new", with_limit]
---

This recipe crawls one page of a website, turns its HyperText Markup Language (HTML) into markdown and keeps the markdown chunks in memory. It is the `scraping_index_to_markdown.rs` example in the swiftide repository. It needs no application programming interface (API) key and no running services, only network access.

## Setup

The loader and the transformer sit behind the `scraping` feature flag. The loader takes a `Website` from the spider crate, so add spider as well. The example prints its output through `tracing`, so add a subscriber too:

```sh
cargo add swiftide@{{swiftide_version}} --features scraping
cargo add spider@2
cargo add tokio --features macros,rt-multi-thread
cargo add tracing-subscriber --features env-filter
```

Run the example from the root of the swiftide repository:

```sh
RUST_LOG=swiftide_indexing=debug cargo run -p swiftide-examples --example scraping-index
```

The example logs every node at the `DEBUG` level. Without `RUST_LOG` it prints errors only, so you see nothing on a successful run.

In your own binary, call `tracing_subscriber::fmt::init()` at the start of an async `main` that returns `Result<(), Box<dyn std::error::Error>>`, as the example does. Without a subscriber, `log_all` prints nothing at any level.

## Pipeline

::snippet{src="@swiftide/examples/scraping_index_to_markdown.rs" anchor="pipeline" title="examples/scraping_index_to_markdown.rs"}

The snippet leaves out the imports. These are the ones the example uses:

`use spider::website::Website;` and `use swiftide::{indexing, indexing::persist::MemoryStorage, indexing::transformers::ChunkMarkdown, integrations::scraping::{HtmlToMarkdownTransformer, ScrapingLoader}};`

### Loader

`Website::new` sets the start address (URL). `with_limit(1)` caps the crawl at one page. Spider's builder methods return `&mut Website`, and `ScrapingLoader::from_spider` takes the `Website` by value, which is why the example calls `.to_owned()`.

`ScrapingLoader::from_url(url)` is the short form. It builds a `Website` with spider's defaults, so it follows links on the site with no page limit, bounded only by spider's default depth of 25. Use `from_spider` whenever you want to bound or tune the crawl.

`Pipeline::from_loader` turns the loader into a stream right away. At that point the loader subscribes to the crawl and spawns two background Tokio tasks: one that crawls and one that receives pages. The crawl starts when you build the pipeline, not when you call `run`. Each page spider fetches becomes one node. The node's chunk is the raw HTML of the page, and its path is the page URL.

### Transformer

`HtmlToMarkdownTransformer` converts the HTML in each node to markdown with the htmd crate. The default skips `script` and `style` tags, so inline scripts and stylesheets do not end up in your chunks. A page that fails to convert becomes an error in the stream.

The transformer keeps the node's path, so every markdown node still carries the URL it came from.

### Chunker

`ChunkMarkdown::from_chunk_range(20..2048)` splits each page along markdown structure into chunks of fewer than 2048 characters. It drops chunks whose trimmed text is shorter than 20 bytes, which removes most leftover navigation fragments. `ChunkMarkdown::from_max_characters(n)` sets only the upper bound.

`then_chunk` turns one node into many.

### Storage

`log_all` logs errors and every node that passes. `MemoryStorage` keeps the chunks in a map in memory. Clone the `MemoryStorage` before you pass it to `then_store_with`. The clone shares the same map, so `storage.get_all_values().await` returns the chunks after `run` finishes. Swap in any other storage backend to keep the result.

## Spider Cloud

Set `SPIDER_CLOUD_API_KEY` and the loader routes the crawl through Spider Cloud, the hosted service of the spider project, with no code change. The loader reads the variables once per process, so set them before the first crawl. See [Spider Cloud](/indexing/loading/#spider-cloud) for the modes and the endpoint override.

The loader still crawls some hosts directly, because Spider Cloud cannot reach them. These are `localhost`, names ending in `.localhost` or `.local`, Internet Protocol version 4 (IPv4) loopback, private, link-local and unspecified addresses, and IPv6 loopback and unspecified addresses. Private and link-local IPv6 addresses still go through Spider Cloud, so crawl those without the key set.

## Gotchas

### Crawl limits

All crawl settings live on spider's `Website`. Besides `with_limit`, spider offers a depth limit, a delay between requests, robots.txt handling, and allow and block lists of URLs. See the spider documentation for the full list. A limit of 0 means no limit.

### Failed pages

`run` stops at the first error in the stream. On a larger crawl one malformed page would end the whole run. Add `.filter_errors()` after the transformer to drop failed nodes and keep going. Put `log_all` before it if you want those errors logged.

### Early end of a large crawl

The receive task reads pages from a spider broadcast channel and forwards them into the pipeline. If the pipeline falls behind while spider keeps fetching, the receive task lags behind the broadcast, stops reading and ends the stream. `run` then returns `Ok` with only part of the site. On a large crawl with slow steps, keep the steps after the loader fast, or check the number of stored chunks against what you expect.

### Tokio runtime

The loader calls `tokio::spawn` inside `Pipeline::from_loader`. Building the pipeline outside a Tokio runtime panics, even if you never call `run`. `#[tokio::main]` is enough.

## Embedding the chunks

The chunks carry no embeddings. To search them, add an embedder with `then_in_batch(Embed::new(...))` and store the nodes in a vector database such as Qdrant. [Rich metadata and multiple vectors](/cookbook/rich-metadata-and-multiple-vectors/) shows that shape with markdown files.

## Reference

- [`ScrapingLoader`](docsrs:swiftide/integrations/scraping/struct.ScrapingLoader.html)
- [`HtmlToMarkdownTransformer`](docsrs:swiftide/integrations/scraping/struct.HtmlToMarkdownTransformer.html)
- [`ChunkMarkdown`](docsrs:swiftide/indexing/transformers/struct.ChunkMarkdown.html)
- [`MemoryStorage`](docsrs:swiftide/indexing/persist/struct.MemoryStorage.html)
- [`indexing::Pipeline`](docsrs:swiftide/indexing/struct.Pipeline.html)
- [`Loader`](docsrs:swiftide/traits/trait.Loader.html)
- [Feature flags](/reference/feature-flags/)

## Next

[Streaming loaders](/cookbook/streaming-loaders/)
