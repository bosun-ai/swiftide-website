---
title: Prompting and Embedding
description: How to prompt and embed data in the pipeline.
sidebar:
  order: 2
---

Our metadata transformers are generic over the `SimplePrompt` trait. This enables different models to be used for different usecases. Similarly, the embedding transformer is generic over the `EmbeddingModel` trait.

## The `SimplePrompt` trait

Which is defined as follows:

```rust
pub trait SimplePrompt: Debug + Send + Sync {
    async fn prompt(&self, prompt: &str) -> Result<String>;
}
```

Or in human language: "Given a Prompt, give me a response".

## The `EmbeddingModel` trait

Which is defined as follows:

```rust
pub trait EmbeddingModel: Send + Sync {
    async fn embed(&self, input: Vec<String>) -> Result<Embeddings>;
}
```

Or in human language: "Given a list of things to Embed, give me embeddings". The embedding transformer will link back the embeddings to the original nodes by _order_.

## Built in inference and embedding models

<small>

| Name      | Description                                               | Feature Flag |
| --------- | --------------------------------------------------------- | ------------ |
| OpenAI    | Implements both SimplePrompt and Embed via `async_openai` | openai       |
| FastEmbed | Implements Embed via `fastembed-rs`                       | fastembed    |

</small>
