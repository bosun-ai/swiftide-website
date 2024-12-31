---
title: Storing the results
description: How to store the results of the pipeline.
sidebar:
  order: 5
---

After processing nodes in the pipeline you probably want to store the results. Pipelines support multiple storage steps, but need at least one. A storage implements the `Persist` trait.

## The `Persist` trait

Which is defined as follows:

```rust
pub trait Persist: Debug + Send + Sync {
    async fn setup(&self) -> Result<()>;
    async fn store(&self, node: Node) -> Result<Node>;
    async fn batch_store(&self, nodes: Vec<Node>) -> IndexingStream;
    fn batch_size(&self) -> Option<usize> {
        None
    }
}
```

Setup functions are run right away, asynchronously when the pipeline starts. This could include setting up collections, tables, connections etcetera. Because more might happen after storing, both `store` and `batch_store` are expected to return the nodes they processed.

If `batch_size` is implemented for the storage, the stream will always prefer `batch_store`.

## Built in storage

<small>

| Name          | Description                                   | Feature Flag |
| ------------- | --------------------------------------------- | ------------ |
| Redis         | Persists nodes by default as json             | redis        |
| Qdrant        | Persists nodes in qdrant                      | qdrant       |
| MemoryStorage | Persists nodes in memory; great for debugging |              |
| LanceDB       | Persist and retrieve in lancedb               | lancedb      |
| PGVector      | Persist and retrieve in pgvector              | pgvector     |

</small>
