---
title: Chunking
description: How to chunk nodes in the pipeline.
sidebar:
  order: 2
---

For quality metadata it can be important to break up text into smaller parts for both better metadata and retrieval. A chunker implements the `ChunkerTransformer` trait.

All our chunkers try to chunk in semantic blocks, i.e. a paragraph with its headers, or a full code function body.

## The `ChunkerTransformer` trait

Which is defined as follows:

```rust
pub trait ChunkerTransformer: Send + Sync + Debug {
    async fn transform_node(&self, node: Node) -> IndexingStream;

    /// Overrides the default concurrency of the pipeline
    fn concurrency(&self) -> Option<usize> {
        None
    }
}
```

Or in human language: "Given a Node, break it up into smaller parts".

## Built in chunkers

<small>

| Name          | Description                                 | Feature Flag |
| ------------- | ------------------------------------------- | ------------ |
| ChunkCode     | Given a (tree-sitter) language, chunks code | tree-sitter  |
| ChunkMarkdown | Chunks markdown                             |              |

</small>
