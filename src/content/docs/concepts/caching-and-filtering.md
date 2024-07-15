---
title: Caching and filtering nodes
description: How to cache and filter nodes in the pipeline.
sidebar:
  order: 3
---

When nodes have already been processed by the pipeline, they can often be skipped, speeding up the pipeline and saving costs. A node cache implements the `NodeCache` trait.

## The `NodeCache` trait

Which is defined as follows:

```rust
pub trait NodeCache: Send + Sync + Debug {
    async fn get(&self, node: &Node) -> bool;
    async fn set(&self, node: &Node);
}
```

Or in human language: "Given a Node, provide methods to set and get from the cache".

## Built in chunkers

<small>

| Name  | Description                                         | Feature Flag |
| ----- | --------------------------------------------------- | ------------ |
| Redis | Can get and set nodes using multiplexed connections | redis        |

</small>
