---
title: "Rich metadata and multiple vectors"
description: "Add questions, a summary, a title and keywords to every chunk with LLM transformers, and store the chunk and each metadata field as separate named vectors in Qdrant."
sidebar:
  order: 8
identLint:
  ignore: ["Distance::Manhattan"]
---

Both recipes fill a Qdrant collection in which every chunk carries four fields that a large language model (LLM) writes: questions and answers, a summary, a title and keywords. One recipe embeds each chunk together with its metadata as a single vector. The other stores the chunk and each metadata field as its own named vector.

## Setup

Add Swiftide with the `openai` and `qdrant` feature flags, plus the runtime and logging crates the examples use:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,qdrant
cargo add tokio --features macros,rt-multi-thread
cargo add tracing-subscriber --features env-filter
cargo add rustls@0.23 --no-default-features --features ring,std
```

Set `OPENAI_API_KEY` and start Qdrant on `localhost:6334`, or point `QDRANT_URL` somewhere else; [Running Qdrant](/integrations/qdrant/#local-instance-and-connection) has the Docker command. Run the examples from the root of the swiftide repository, because they load `README.md` and `LICENSE` by relative path:

```sh
cargo run -p swiftide-examples --example index-markdown-metadata
cargo run -p swiftide-examples --example store-multiple-vectors
```

Swiftide does not select a Transport Layer Security (TLS) crypto provider. In your own binary, install a rustls crypto provider at the start of `main`; see [Crypto provider panic](/reference/troubleshooting-and-faq/#crypto-provider-panic).

## Metadata transformers

Both recipes use one OpenAI client for prompts and embeddings:

::snippet{src="@swiftide/examples/index_markdown_lots_of_metadata.rs" anchor="setup" title="examples/index_markdown_lots_of_metadata.rs"}

The client sets `gpt-4o` for the metadata prompts and `text-embedding-3-small` for embeddings. That embedding model returns 1536 dimensions, which must match `vector_size(1536)` on the storage.

::snippet{src="@swiftide/examples/index_markdown_lots_of_metadata.rs" anchor="pipeline" title="examples/index_markdown_lots_of_metadata.rs"}

`ChunkMarkdown::from_chunk_range(20..2048)` splits the file into chunks of fewer than 2048 characters. It drops chunks that are under 20 bytes once whitespace is trimmed. Each metadata transformer then sends one prompt per chunk and writes the answer under a fixed metadata key:

| Transformer | Metadata key |
| --- | --- |
| `MetadataQAText` | `Questions and Answers (text)` |
| `MetadataSummary` | `Summary` |
| `MetadataTitle` | `Title` |
| `MetadataKeywords` | `Keywords` |

Each module exposes its key as a `NAME` constant, for example `metadata_qa_text::NAME`. The default `MetadataQAText` prompt asks for at most five questions, so the model may return fewer. The prompts are covered in [Transformers and metadata](/indexing/transformers-and-metadata/).

Each chunk costs four prompts. `with_concurrency(1)` sends them one node at a time. Raise the pipeline concurrency when your OpenAI rate limit allows it.

The pipeline keeps the default embed mode. `Embed` turns each node into one text: the metadata as `key: value` lines, then the chunk. With only the `Combined` field configured, Qdrant stores it as the collection's single unnamed vector. The metadata goes into the point payload next to `path` and `content`.

### Error handling

`log_all` logs errors, and logs every node at `DEBUG` level. Set `RUST_LOG=debug` to see each chunk with its metadata. `filter_errors` drops errors instead of stopping the run, but an error can take healthy nodes with it.

`then_in_batch(Embed)` collects nodes into batches before it embeds them. When a failed prompt reaches that step, the batch returns the error together with every node it had collected so far. `filter_errors` drops the whole error. One failed prompt can therefore drop up to 255 good nodes in the first recipe, which uses the default batch size of 256, and up to 9 in the second. A failed embedding request also fails its whole batch. Keep the batch size small when your prompts fail often.

## Named vectors

The second recipe stores five vectors per chunk instead of one:

::snippet{src="@swiftide/examples/store_multiple_vectors.rs" anchor="pipeline" title="examples/store_multiple_vectors.rs"}

`with_embed_mode(EmbedMode::PerField)` right after the loader makes `Embed` send the chunk and each metadata value as separate texts, so `with_batch_size(10)` keeps each request at 50 inputs. Each `with_vector` names one stored field. Qdrant names the vectors `Chunk`, `Metadata: Questions and Answers (text)`, `Metadata: Summary` and so on. The title vector uses `Distance::Manhattan` through `VectorConfig::builder()`. [Embedding](/indexing/embedding/#embed-mode) explains embed modes and named vectors in full.

## Gotchas

Qdrant keeps the vector configuration of an existing collection, as [Existing collections](/cookbook/hybrid-search-and-reranking/#existing-collections) explains. That is why the two examples use different collection names. Delete the collection or pick a new name when you change the vectors.

### Search on named vectors

A query pipeline with the default search strategy, `SimilaritySingleEmbedding`, searches the vector named `Combined` whenever the `Qdrant` storage is built with more than one `with_vector` or with any `with_sparse_vector`. The check uses the `Qdrant` value in the query pipeline, not the collection. The `store-multiple-vectors` collection has no `Combined` vector, so that search fails.

To keep similarity search working, index with `EmbedMode::Both` and add `with_vector(EmbeddedField::Combined)`. Build the query-side `Qdrant` with the same `with_vector` calls. A `Qdrant` with the default single-vector config sends an unnamed search, which fails against a collection of named vectors.

`HybridSearch` lets you pick the field: `with_dense_vector_field` and `with_sparse_vector_field` both default to `EmbeddedField::Combined`. Hybrid search also needs a sparse vector for the chosen field, from `SparseEmbed` and `with_sparse_vector`. [Hybrid search and reranking](/cookbook/hybrid-search-and-reranking/) shows that setup. For other searches on one named vector, use the Qdrant client from `Qdrant::client()` directly.

## Reference

- [`Pipeline`](docsrs:swiftide/indexing/struct.Pipeline.html)
- [`FileLoader`](docsrs:swiftide/indexing/loaders/struct.FileLoader.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)
- [`EmbedMode`](docsrs:swiftide/indexing/enum.EmbedMode.html)
- [`EmbeddedField`](docsrs:swiftide/indexing/enum.EmbeddedField.html)
- [`ChunkMarkdown`](docsrs:swiftide/indexing/transformers/struct.ChunkMarkdown.html)
- [`MetadataQAText`](docsrs:swiftide/indexing/transformers/struct.MetadataQAText.html) and [`metadata_qa_text::NAME`](docsrs:swiftide/indexing/transformers/metadata_qa_text/constant.NAME.html)
- [`MetadataSummary`](docsrs:swiftide/indexing/transformers/struct.MetadataSummary.html) and [`metadata_summary::NAME`](docsrs:swiftide/indexing/transformers/metadata_summary/constant.NAME.html)
- [`MetadataTitle`](docsrs:swiftide/indexing/transformers/struct.MetadataTitle.html) and [`metadata_title::NAME`](docsrs:swiftide/indexing/transformers/metadata_title/constant.NAME.html)
- [`MetadataKeywords`](docsrs:swiftide/indexing/transformers/struct.MetadataKeywords.html) and [`metadata_keywords::NAME`](docsrs:swiftide/indexing/transformers/metadata_keywords/constant.NAME.html)
- [`Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html)
- [`SparseEmbed`](docsrs:swiftide/indexing/transformers/struct.SparseEmbed.html)
- [`Qdrant`](docsrs:swiftide/integrations/qdrant/struct.Qdrant.html)
- [`QdrantBuilder`](docsrs:swiftide/integrations/qdrant/struct.QdrantBuilder.html)
- [`VectorConfig`](docsrs:swiftide/integrations/qdrant/struct.VectorConfig.html)
- [`Distance`](docsrs:swiftide/integrations/qdrant/type.Distance.html)
- [`SimilaritySingleEmbedding`](docsrs:swiftide/query/search_strategies/struct.SimilaritySingleEmbedding.html)
- [`HybridSearch`](docsrs:swiftide/query/search_strategies/struct.HybridSearch.html)

## Next

[Scraping websites to markdown](/cookbook/scraping-to-markdown/)
