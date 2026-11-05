---
title: "How querying works"
description: "A query pipeline takes a question from Pending to Retrieved to Answered through query transformers, a retriever, response transformers and an answer generator."
sidebar:
  order: 1
---

You end up with a query pipeline that answers questions about your own code and names the files each answer came from. It is the query side of the [ask-your-codebase](/showcases/ask-your-codebase/) showcase, a retrieval augmented generation (RAG) command line tool.

The pipeline reads the `code_chunks` LanceDB table that the showcase's [indexing pipeline](/indexing/overview/) fills with the same embedding model, so run `ask-your-codebase index` first. The full program needs the `openai`, `fastembed` and `lancedb` feature flags. The helpers and prompt constants live in the showcase.

## The ask-your-codebase pipeline

::snippet{src="@swiftide/examples/ask-your-codebase/src/query.rs" anchor="query_pipeline" title="examples/ask-your-codebase/src/query.rs"}

The pipeline runs four steps for each question:

1. `GenerateSubquestions` asks the large language model (LLM) to rewrite the question into several related questions. Their text becomes the current query.
2. `Embed` embeds the current query with the local FastEmbed model that also embedded the chunks during indexing.
3. `then_retrieve` hands the query to the LanceDB storage, the retriever. It returns the `top_k` most similar chunks as documents.
4. `answers::Simple`, the answer generator, renders the documents into a prompt and asks the LLM for the answer.

`SimilaritySingleEmbedding` is the search strategy. [Retrieval and search strategies](/querying/retrieval-and-search-strategies/) explains the `let mut` binding and its type annotation.

This is the start of a real run against the `swiftide-agents` crate. The answer came from `claude-sonnet-5-5`, and the embeddings from `jina-embeddings-v2-base-code` running locally:

```text
$ ./target/debug/ask-your-codebase ask --model claude-sonnet-5-5 --path swiftide-agents "How does an agent decide to stop?"
# How an agent decides to stop

An agent stops when something calls `Agent::stop` (or `stop_with_error`), which moves the state to `State::Stopped(StopReason)`. The main loop in `run_agent` checks that state after each step. `Agent::stop` is a no-op if the agent is already stopped (`src/agent.rs`).

...

Sources:
  - src/agent.rs
  - src/tools/control.rs
  - src/lib.rs
  - src/state.rs
  - src/errors.rs
  - src/task_agent.rs
  - src/hooks.rs
  - src/tasks.rs
  - src/default_context.rs
```

The sources list comes from the `path` metadata of the documents in the answered query. [Generating answers](/querying/answering/) shows the showcase's `sources` code and the document template that puts the file path in front of each chunk.

## Query states

![Indexing and query pipelines](/diagrams/diagram-pipeline.svg)

A question travels through the pipeline as a `Query<STATE>`. The state is a type parameter, and it moves from `states::Pending` to `states::Retrieved` to `states::Answered`. A query keeps the original question in `original()` and the text of the latest step in `current()`. A query transformer sets `current()` to the rewritten question. A response transformer such as `Summary` can replace it with a summary, and `answers::Simple` then uses that text in place of the documents. The retriever adds documents, which you read with `documents()`. Once answered, `current()` holds the answer, and `answer()` returns it.

The pipeline carries the same state, with different methods per state:

| Pipeline state | Methods |
| --- | --- |
| `states::Pending` | `then_transform_query`, `then_retrieve`, `evaluate_with` |
| `states::Retrieved` | `then_transform_response`, `then_answer` |
| `states::Answered` | `query`, `query_mut`, `query_all` |

The compiler enforces this order. A pipeline without `then_retrieve` has no `then_answer`, and a pipeline without `then_answer` has no `query`.

## Pipeline steps

Each step method takes one trait:

| Step | Trait | Called |
| --- | --- | --- |
| `then_transform_query` | `TransformQuery` | any number of times, before retrieval |
| `then_retrieve` | `Retrieve` for the search strategy | once |
| `then_transform_response` | `TransformResponse` | any number of times, after retrieval |
| `then_answer` | `Answer` | once |

Every step accepts a struct that implements the trait, a `Box<dyn ..>` or `Arc<dyn ..>` of the trait, or a closure. Closures are synchronous and must be `Clone + Send + Sync`. Each step runs every query on its own Tokio task with `tokio::spawn`, so the pipeline needs a Tokio runtime.

### Closure steps

::snippet{src="@snippets/src/querying/overview.rs" anchor="closure_steps"}

This pipeline needs no model and no storage, which makes it a starting point for tests. `retrieved_documents` and `answered` are the only ways to move a `Query` to the next state, and `transformed_query` records each rewrite in `history()`.

## Search strategy

`query::Pipeline::default()` uses `SimilaritySingleEmbedding` with a `top_k` of 10 and no filter. Put an `Embed` query transformer before `then_retrieve`. The retriever searches with the embedding it sets.

`query::Pipeline::from_search_strategy` starts a pipeline with any other strategy: `SimilaritySingleEmbedding` with a different `top_k` or a filter, `HybridSearch` or `CustomStrategy`. The retriever must implement `Retrieve` for that strategy. If it does not, the pipeline does not compile. [Retrieval and search strategies](/querying/retrieval-and-search-strategies/) lists which storage supports which strategy.

## Query methods

::snippet{src="@snippets/src/querying/overview.rs" anchor="reuse"}

`query` consumes the pipeline. `query_mut` borrows the pipeline mutably, so you can call it again for the next question. `query_all` takes a `Vec` of questions, runs them through the steps concurrently and returns every answered query.

`query_all` returns results in the order they finish, not in the order you passed them. Match each result to its question with `original()`. An empty `Vec` never returns, because the pipeline waits for an answer that does not come. The first error ends the call, and the answers that already finished are lost.

Every query method accepts anything that implements `AsRef<str>`, such as `&str` or `String`.

## Gotchas

- The pipeline steps are lazy. No step runs until `query`, and errors from a model or a storage show up as the error of `query`. Code that builds a step can still do work: in the showcase, `embedding::local_model()` loads the FastEmbed model, and downloads it on first use, before the pipeline exists.
- `query` and `query_mut` log `Answered query in N seconds` for every question, and `query_mut` also logs `Sending query`. `query_all` logs `Sending queries` and `Answered all queries in N seconds` once per batch. All are at warn level, so filter the `swiftide_query` target in your tracing subscriber if that is too noisy.
- Query transformers, response transformers and answer generators must be owned and `'static`. Pass clients by value and `clone()` the ones you use in more than one step. `then_retrieve` also accepts a reference, such as `&qdrant`, and clones it.
- A closure that captures a value without `Clone`, such as a lock guard, does not implement the step trait and fails with a trait-bound error.
- Each step runs up to one query per logical central processing unit (CPU) at the same time. The pipeline has no setting to change that.
- The source of the query pipeline marks it as being in an early stage. Expect its application programming interface (API) to change between releases, and check [Upgrading](/reference/upgrading/) when you bump Swiftide.
- The query pipeline needs no feature flag. The models and retrievers do, for example `openai`, `fastembed`, `lancedb` or `qdrant`. See [Feature flags](/reference/feature-flags/).

## Reference

- [`query::Pipeline`](docsrs:swiftide/query/struct.Pipeline.html)
- [`Query`](docsrs:swiftide/query/struct.Query.html)
- [`states`](docsrs:swiftide/query/states/index.html)
- [`Document`](docsrs:swiftide/query/struct.Document.html)
- [`TransformQuery`](docsrs:swiftide/traits/trait.TransformQuery.html)
- [`Retrieve`](docsrs:swiftide/traits/trait.Retrieve.html)
- [`TransformResponse`](docsrs:swiftide/traits/trait.TransformResponse.html)
- [`Answer`](docsrs:swiftide/traits/trait.Answer.html)
- [`SimilaritySingleEmbedding`](docsrs:swiftide/query/search_strategies/struct.SimilaritySingleEmbedding.html)
- [`GenerateSubquestions`](docsrs:swiftide/query/query_transformers/struct.GenerateSubquestions.html)
- [`query_transformers::Embed`](docsrs:swiftide/query/query_transformers/struct.Embed.html)
- [`answers::Simple`](docsrs:swiftide/query/answers/struct.Simple.html)
- [`HybridSearch`](docsrs:swiftide/query/search_strategies/struct.HybridSearch.html)
- [`CustomStrategy`](docsrs:swiftide/query/search_strategies/struct.CustomStrategy.html)
- [`EvaluateQuery`](docsrs:swiftide/traits/trait.EvaluateQuery.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)
- [`Qdrant`](docsrs:swiftide/integrations/qdrant/struct.Qdrant.html)
- [`LanceDB`](docsrs:swiftide/integrations/lancedb/struct.LanceDB.html)
- [`FastEmbed`](docsrs:swiftide/integrations/fastembed/struct.FastEmbed.html)

## Next

[Transforming queries](/querying/transforming-queries/)
