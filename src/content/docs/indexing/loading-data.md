---
title: Loading Data
description: How to load data into the pipeline.
sidebar:
  order: 1
---

A pipeline starts with data and is only as good as the data it ingests. A loader implements the `Loader` trait.

## The `Loader` trait

Which is defined as follows:

```rust
pub trait Loader {
    fn into_stream(self) -> IndexingStream;
}
```

Or in human language: "I can be turned into a stream". The assumption under the hood is that Loaders will yield the data they load as a stream of `Nodes`. These can be files, messages, webpages and so on.

:::note
Indexing pipelines can also be created from an indexing stream with `Pipeline::from_stream`
:::

## Built in loaders

<small>

| Name           | Description                                                                     | Feature Flag |
| -------------- | ------------------------------------------------------------------------------- | ------------ |
| FileLoader     | Loads files with an optional extension filter, respecting gitignore             |              |
| ScrapingLoader | Scrapes a website using the `spider` crate and html to markdown transformations | scraping     |
| fluvio         | Load data directly from fluvio streams                                          | fluvio       |

</small>
