---
title: "Transforming queries"
description: "Rewrite a question into subquestions with GenerateSubquestions, embed it densely and sparsely with the model your chunks were embedded with, or write your own TransformQuery."
sidebar:
  order: 2
---

A query pipeline can rewrite the user's question before the retriever runs: a large language model (LLM) widens it into related questions, and an embedding model turns the result into the vectors your chunks were stored with.

The query transformers need no feature flag. The snippets use the OpenAI client and Qdrant, which sit behind the `openai` and `qdrant` feature flags:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,qdrant
cargo add async-trait anyhow
```

The second line is for the custom query transformer further down, which uses the `#[async_trait]` attribute and `anyhow::Result`.

## Subquestions

::snippet{src="@snippets/src/querying/transforming_queries.rs" anchor="subquestions"}

A query transformer runs before retrieval. It takes the pending query and returns a new pending query. You add one with `then_transform_query`, as often as you need, and they run in the order you add them.

`GenerateSubquestions` sends the current query to an LLM through `SimplePrompt`. It asks for `num_questions` related questions, 5 by default. The reply replaces the current query, so the next step sees one text: the original question and its subquestions as a list. `GenerateSubquestions::from_client` builds it with the default prompt and count.

The default prompt is written for code. It asks the model to think about the dependencies, usage and structure of the code. Replace it when you search other content.

### Subquestion prompts

The ask-your-codebase showcase sets its own prompt on the builder:

::snippet{src="@swiftide/examples/ask-your-codebase/src/query.rs" anchor="query_pipeline" title="examples/ask-your-codebase/src/query.rs" mark="8"}

`SUBQUESTIONS_PROMPT` is a constant next to the pipeline:

::snippet{src="@swiftide/examples/ask-your-codebase/src/query.rs" anchor="prompts" title="examples/ask-your-codebase/src/query.rs" collapse="22-48"}

The field takes a `Prompt`, so convert a `&'static str` or `String` with `.into()` or `Prompt::from`. A borrowed `&str` built at runtime needs `.to_string()` first. The template receives two values: `question`, the current query, and `num_questions`.

The prompt phrases the subquestions like the questions that its indexing pipeline wrote for each chunk. Similar phrasing puts the query vector close to the chunk vectors. Keep the instruction to repeat the original question: the next step embeds only what the model returns. [Prompts and templates](/models/prompts-and-templates/) covers the template syntax.

## Dense and sparse embeddings

::snippet{src="@swiftide/examples/docs-qa/src/query.rs" anchor="query_pipeline" title="examples/docs-qa/src/query.rs" mark="2-8"}

`query_transformers::Embed` embeds the current query with an `EmbeddingModel` and stores the vector in `query.embedding`. `query_transformers::SparseEmbed` does the same with a `SparseEmbeddingModel` and fills `query.sparse_embedding`. At this release only `FastEmbed`, behind the `fastembed` feature flag, implements `SparseEmbeddingModel`.

Use the models that your [indexing pipeline embedded the chunks with](/indexing/embedding/). Vectors from different models do not compare. The docs-qa showcase creates its dense and sparse `FastEmbed` models together, and loads the same two models for its indexing and its query pipeline.

Similarity search needs `embedding`. Hybrid search on Qdrant needs both, and fails with `No sparse embedding for query` when the sparse one is missing. [Retrieval and search strategies](/querying/retrieval-and-search-strategies/) covers both strategies.

An embedding step reads the current query at the moment it runs. Put it after every step that rewrites the text. If `Embed` runs before `GenerateSubquestions`, the vector search never sees the subquestions.

## Custom query transformers

::snippet{src="@snippets/src/querying/transforming_queries.rs" anchor="custom_transformer"}

`TransformQuery` has one async method, `transform_query`. It takes the `Query<states::Pending>` by value and returns it. Implement it with the `#[async_trait]` attribute from the `async-trait` crate. The trait requires `Send`, `Sync` and `DynClone`, so derive `Clone`.

The current query has no setter of its own. `transformed_query` replaces it and records a `Transformed` event with the text before and after. The original question never changes. The fields `embedding`, `sparse_embedding` and `documents` are public, so a transformer that computes a vector sets it directly, as `Embed` does.

This example prepares a question for an embedding model of the BGE (BAAI General Embedding) family, which suggests an instruction in front of short search queries. The chunks are embedded without it.

::snippet{src="@snippets/src/querying/transforming_queries.rs" anchor="custom_transformer_pipeline"}

A closure is a query transformer too. It must be `Fn(Query<states::Pending>) -> Result<Query<states::Pending>>` and `Clone`, `Send` and `Sync`. Closures are synchronous, so use a struct for a step that awaits.

The pipeline logs `Transformed query` at debug level after each step, with the transformer's `name()` and the new current query. `name()` defaults to the last path segment of the type name, so a closure logs a generic closure name instead of a useful one. Override `name()` on a struct to get a clearer log line.

## Transformed query after retrieval

::snippet{src="@snippets/src/querying/transforming_queries.rs" anchor="history"}

Since 0.33 the current query keeps the transformed text after retrieval. The `Retrieved` event records the current query at the moment of retrieval, as both `before()` and `after()`, and the documents it returned. Before 0.33, retrieval cleared the current query.

That text is not always what the search used. A vector retriever such as Qdrant searches with `query.embedding`, which `Embed` computed when it ran. A step after `Embed` that rewrites the text changes the event, not the vector.

`history()` lists the events in order. Each `transformed_query` call adds a `Transformed` event, and each retrieval adds a `Retrieved` event. `Embed` and `SparseEmbed` add nothing, because they leave the text alone.

Once the query is answered, `current()` returns the answer. Read the transformed query from the history instead.

[Generating answers](/querying/answering/) shows how the answer generator uses the original and the transformed query.

## Gotchas

- Two types are called `Embed`. `query_transformers::Embed` belongs in a query pipeline, `indexing::transformers::Embed` in an indexing pipeline. The same goes for `SparseEmbed`. Import them by module path.
- After `GenerateSubquestions`, `Embed` turns the whole list into one vector. The pipeline does not run one search per subquestion.
- DuckDB hybrid search, behind the `duckdb` feature flag, uses the subquestion list as its keyword text; see [Retrieval and search strategies](/querying/retrieval-and-search-strategies/).
- The FastEmbed [`Rerank` response transformer](/querying/reranking-and-response-transformers/) scores documents against `original()`, and the [`Ragas` evaluator](/querying/evaluation/) matches rows by `original()`. Rewriting the question changes neither.
- `Query::new` sets only the original question and leaves the current query empty, so an embedding step would embed an empty string. Pass a `&str` or `String` to `query`, or build the query with `Query::from`, which sets both.

## Reference

- [`Pipeline`](docsrs:swiftide/query/struct.Pipeline.html) (`then_transform_query`)
- [`TransformQuery`](docsrs:swiftide/traits/trait.TransformQuery.html)
- [`GenerateSubquestions`](docsrs:swiftide/query/query_transformers/struct.GenerateSubquestions.html)
- [`query_transformers::Embed`](docsrs:swiftide/query/query_transformers/struct.Embed.html)
- [`query_transformers::SparseEmbed`](docsrs:swiftide/query/query_transformers/struct.SparseEmbed.html)
- [`Query`](docsrs:swiftide/query/struct.Query.html)
- [`TransformationEvent`](docsrs:swiftide/query/enum.TransformationEvent.html)
- [`states::Pending`](docsrs:swiftide/query/states/struct.Pending.html)
- [`EmbeddingModel`](docsrs:swiftide/traits/trait.EmbeddingModel.html)
- [`SparseEmbeddingModel`](docsrs:swiftide/traits/trait.SparseEmbeddingModel.html)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)
- [`answers::Simple`](docsrs:swiftide/query/answers/struct.Simple.html)
- Showcases: [ask-your-codebase](/showcases/ask-your-codebase/) and [docs-qa](/showcases/docs-qa/)

## Next

[Retrieval and search strategies](/querying/retrieval-and-search-strategies/)
