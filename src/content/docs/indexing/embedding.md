---
title: "Embedding"
description: "Add dense and sparse embeddings to nodes with Embed and SparseEmbed, choose the embedded text with EmbedMode, and store each embedded field as a named vector in Qdrant."
sidebar:
  order: 5
identLint:
  ignore: [TextEmbedding, vector_combined]
---

Here every chunk in your indexing pipeline ends up with a dense and a sparse vector, stored in a Qdrant collection that a retriever can search. You add an embedder as a batch transformer, choose which text it embeds, and configure the storage to keep the vectors you need.

Add the `fastembed` and `qdrant` feature flags:

```sh
cargo add swiftide@{{swiftide_version}} --features fastembed,qdrant
```

## Dense and sparse embeddings

The docs-qa showcase gives every chunk a dense and a sparse embedding:

::snippet{src="@swiftide/examples/docs-qa/src/index.rs" anchor="pipeline" title="examples/docs-qa/src/index.rs" mark="17-18"}

`Embed::new` takes any `EmbeddingModel` and fills the node's `vectors`. `SparseEmbed::new` takes any `SparseEmbeddingModel` and fills `sparse_vectors`. A dense vector captures what the text means. A sparse vector weights the words in it, which helps keyword-like matches in hybrid search.

Both are batch transformers, so you add them with `then_in_batch`. Each batch becomes one model call. `with_batch_size` sets the nodes per batch, with a pipeline default of 256. `with_concurrency` sets how many batches run at the same time.

Both work on text nodes (`TextNode`) only. A second `Embed` step replaces the dense vectors of the first one instead of adding to them.

## Embed mode

A node's embed mode decides which texts the embedder sees, each under an embedded field:

| `EmbedMode` | Embedded fields |
| --- | --- |
| `SingleWithMetadata` (default) | `Combined` |
| `PerField` | `Chunk`, plus one `Metadata(name)` per metadata field |
| `Both` | all of the above |

`Combined` is the metadata as `key: value` lines, a newline, then the chunk. Metadata keys come in sorted order. `Chunk` is the chunk alone. `Metadata(name)` is the value of one metadata field. Values that are not strings are embedded as JSON (JavaScript Object Notation).

`Pipeline::with_embed_mode` sets the mode on every node that passes that point. Chunkers that use `Node::into_chunks` copy the mode to each chunk, so you can set it right after the loader.

Qdrant names each vector after its field: `Combined`, `Chunk` or `Metadata: Title`. Sparse vectors add `_sparse`, as in `Combined_sparse`. LanceDB prefixes the column with `vector_`, as in `vector_combined`.

## Named vectors

With `EmbedMode::PerField`, every node carries several dense vectors. Qdrant stores each one as a named vector:

::snippet{src="@swiftide/examples/store_multiple_vectors.rs" anchor="pipeline" title="examples/store_multiple_vectors.rs" mark="3,17-26"}

The example embeds the chunk and four metadata fields that a large language model (LLM) writes. Each `with_vector` call adds a named vector for one embedded field. Each metadata transformer module has a `NAME` constant, such as `metadata_title::NAME`, so the vector names match. `VectorConfig::builder()` overrides the distance or the size of one vector.

When a node has several vectors, Qdrant stores only the fields you configured and skips the rest. Without `with_vector`, the collection has one vector for `Combined`. The first `with_vector` call replaces that default.

[Similarity search](/integrations/qdrant/#similarity-search) on a collection with named vectors always queries `Combined`, which the example above lacks. To search it, use `EmbedMode::Both` and add `with_vector(EmbeddedField::Combined)`.

### Single vector collections

With exactly one dense vector and no sparse vector, Qdrant uses one unnamed vector, both in the collection and in each node that has a single vector. That node skips the field filter. So `with_vector(EmbeddedField::Chunk)` alone breaks in two ways:

- `PerField` or `Both` nodes with several vectors arrive as named vectors, and Qdrant rejects the write.
- Nodes in the default mode get their `Combined` vector stored without an error.

With a single configured vector, configure the field your embed mode produces.

## Hybrid collections

Hybrid search queries a dense and a sparse vector. `HybridSearch` uses `Combined` for both by default, and `with_dense_vector_field` and `with_sparse_vector_field` change each one on its own. The hybrid search example loads both models and configures both vectors for `Combined`:

::snippet{src="@swiftide/examples/hybrid_search.rs" anchor="setup" title="examples/hybrid_search.rs"}

`FastEmbed::try_default()` loads BGE (BAAI General Embedding) small, a dense model with 384 dimensions. `FastEmbed::try_default_sparse()` loads a SPLADE (Sparse Lexical and Expansion) model for sparse vectors. `vector_size` must match the dense model, here 384. Qdrant stores a sparse vector only when its field also has a dense vector configured. So `with_sparse_vector(EmbeddedField::Chunk)` without `with_vector(EmbeddedField::Chunk)` adds a sparse vector that never gets filled.

## Embedding models

[FastEmbed](/integrations/fastembed/) runs models on your central processing unit (CPU). It is the only integration that produces sparse embeddings. One instance holds one model, and passing a sparse instance to `Embed` fails with "Expected dense model, got sparse". [FastEmbed](/integrations/fastembed/#model-choice) shows how to load another model, such as a code-trained one.

Provider embeddings come from a client with an embedding model. The named vectors example uses OpenAI, which needs the `openai` feature flag:

::snippet{src="@swiftide/examples/store_multiple_vectors.rs" anchor="setup" title="examples/store_multiple_vectors.rs"}

Without `default_embed_model`, every `embed` call fails with "Model not set". The example sets `vector_size(1536)` to match `text-embedding-3-small`. [Models](/models/overview/) lists which client implements `EmbeddingModel`.

Index and query with the same model. A new model needs a new index.

## Batch sizes

Batching happens at three levels:

- `with_batch_size` on `Embed` sets the nodes per model call, and `batch_size` on `FastEmbed` the texts per model run, as [FastEmbed](/integrations/fastembed/#batch-size) explains.
- `batch_size` on the storage sets the nodes per write. Qdrant defaults to 50. [Batching](/indexing/storage/#batching) covers how the storage receives them.

With `PerField`, a node with four metadata fields gives five texts, so a batch of 10 nodes sends 50 texts in one call.

## Gotchas

- A failed model call turns the whole batch into one error. `filter_errors` then drops every node of that batch, see [Errors before a batch step](/indexing/streams-and-concurrency/#errors-before-a-batch-step).
- `LanguageModelWithBackOff` does not retry `embed` calls, as [errors and backoff](/models/errors-backoff-and-usage/) explains.
- `Combined` puts the metadata before the chunk. A model that truncates long input cuts the end of the chunk first. FastEmbed truncates at its token limit without an error. `fastembed::TextInitOptions::with_max_length` raises that limit, as ask-your-codebase does.
- `PerField` embeds every metadata field, including the ones your storage skips. Each one costs embedding time.
- A chunker that builds nodes with `TextNode::new` instead of `Node::into_chunks` resets the embed mode to the default.
- Clones of a `FastEmbed` share one model behind a lock, so a higher `with_concurrency` does not speed it up.

## Reference

- [`Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html)
- [`SparseEmbed`](docsrs:swiftide/indexing/transformers/struct.SparseEmbed.html)
- [`EmbedMode`](docsrs:swiftide/indexing/enum.EmbedMode.html)
- [`EmbeddedField`](docsrs:swiftide/indexing/enum.EmbeddedField.html)
- [`Pipeline::then_in_batch`](docsrs:swiftide/indexing/struct.Pipeline.html#method.then_in_batch)
- [`Pipeline::with_embed_mode`](docsrs:swiftide/indexing/struct.Pipeline.html#method.with_embed_mode)
- [`EmbeddingModel`](docsrs:swiftide/traits/trait.EmbeddingModel.html)
- [`SparseEmbeddingModel`](docsrs:swiftide/traits/trait.SparseEmbeddingModel.html)
- [`FastEmbed`](docsrs:swiftide/integrations/fastembed/struct.FastEmbed.html)
- [`Qdrant`](docsrs:swiftide/integrations/qdrant/struct.Qdrant.html)
- [`QdrantBuilder::with_vector`](docsrs:swiftide/integrations/qdrant/struct.QdrantBuilder.html#method.with_vector)
- [`QdrantBuilder::with_sparse_vector`](docsrs:swiftide/integrations/qdrant/struct.QdrantBuilder.html#method.with_sparse_vector)
- [`VectorConfig`](docsrs:swiftide/integrations/qdrant/struct.VectorConfig.html)
- [`HybridSearch`](docsrs:swiftide/query/search_strategies/struct.HybridSearch.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/struct.OpenAI.html)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)
- [`Node::into_chunks`](docsrs:swiftide/indexing/struct.Node.html#method.into_chunks)
- [Feature flags](/reference/feature-flags/)

## Next

[Caching](/indexing/caching/)
