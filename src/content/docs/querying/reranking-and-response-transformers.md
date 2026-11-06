---
title: "Reranking and response transformers"
description: "Reorder and trim retrieved documents with the FastEmbed Rerank cross-encoder, condense them into an LLM Summary that the answer generator uses instead, or filter them with your own TransformResponse."
sidebar:
  order: 4
---

Your query pipeline fetches twenty candidate documents, keeps the ten a local reranker scores highest, and answers from those. You can also summarize the documents with a large language model (LLM) or filter them with your own response transformer.

## FastEmbed reranker

The docs-qa showcase builds its reranker next to its answer generator:

::snippet{src="@swiftide/examples/docs-qa/src/query.rs" anchor="answerer" title="examples/docs-qa/src/query.rs"}

It then places the reranker between retrieval and the answer:

::snippet{src="@swiftide/examples/docs-qa/src/query.rs" anchor="query_pipeline" title="examples/docs-qa/src/query.rs" mark="10-11"}

`Rerank` is a response transformer. A response transformer runs after `then_retrieve` and before `then_answer`. It takes the query in the `Retrieved` state and returns it in the same state.

`Rerank` runs a cross-encoder: a model that reads the question and one document together and returns a relevance score. That is slower than comparing embeddings, so you run it only on the candidates the retriever already found.

The builder has four fields:

- `top_k`: how many documents to keep, as a `usize`. The default is 10.
- `document_template`: an optional Tera template that renders each document before scoring. The setter takes an `Option<String>`, so pass `Some(..)`.
- `model`: the FastEmbed `TextRerank` model, wrapped in `Arc<tokio::sync::Mutex<_>>`. The default is BGE reranker base.
- `model_batch_size`: an optional batch size. The default is FastEmbed's own.

`Rerank` scores every document against `query.original()`, the question as the user asked it. Query transformers such as `GenerateSubquestions` replace `current()`, which the retriever searches with and which `answers::Simple` also sees. The reranker ignores `current()`. It replaces the query's documents with the `top_k` highest-scoring ones, best first.

`Rerank` lives in `swiftide::integrations::fastembed` behind the `fastembed` feature flag:

```sh
cargo add swiftide@{{swiftide_version}} --features fastembed
```

This is a docs-qa answer, written by `claude-sonnet-5-5` and reranked locally with BGE reranker base. The sources list the pages and sections of the kept documents, best first:

```text
Swiftide supports OpenAI and Azure OpenAI, Anthropic, Gemini, OpenRouter, AWS Bedrock Converse API, Groq, Ollama, and Dashscope. Integrations are feature-gated, so you enable a provider by adding its Cargo feature to `swiftide`.
...
Sources:
  [1] Swiftide › Swiftide capabilities for agents, typed task graphs, pipelines, and tracing  (README.md)
  [2] Swiftide › Swiftide: Rust framework for LLM agents, task graphs, and RAG  (README.md)
  [3] Swiftide › Swiftide Integrations Overview by Feature-Gated Category  (README.md)
  ...
  [9] Swiftide › Swiftide agent tool defined with the swiftide::tool macro  (README.md)
  [10] Swiftide › Installing Swiftide and Setting Up OpenAI Integration  (README.md)
```

The `openai` feature flag in that answer comes from the tenth source, the Quick Start chunk. With a `top_k` below 10 it would not have reached the model. Check a few answers before you lower `top_k`.

### Rerank templates

::snippet{src="@swiftide/examples/docs-qa/src/query.rs" anchor="answer_templates" title="examples/docs-qa/src/query.rs"}

`RERANK_TEMPLATE` puts the page name and the chunk title, both written during indexing, before the content. A chunk that is mostly code says little about its topic, so the extra context helps it score fairly. The template only changes what the model scores. The documents stay as they were, and the answer generator renders them with its own `document_template`.

In docs-qa this template fixed one evaluation result. For "What is a task in Swiftide?", the chunk that lists what a task supports ranked 14th when the reranker scored only chunk text, so it was dropped. The answer missed two features, with answer recall 0.71 and context recall 0.57. With page and title in the template, the chunk made the kept ten and both scores reached 1.00. [Evaluating with Ragas](/querying/evaluation/#scores) explains these scores.

Model download, cache location and load failures are covered in [FastEmbed](/integrations/fastembed/).

### Minimal pipeline

The single-file example builds its reranker in its setup:

::snippet{src="@swiftide/examples/reranking.rs" anchor="setup"}

It then generates subquestions for retrieval, while the reranker still scores against the original question:

::snippet{src="@swiftide/examples/reranking.rs" anchor="query"}

## Summaries

::snippet{src="@swiftide/examples/query_pipeline.rs" anchor="query"}

`response_transformers::Summary` sends the retrieved documents to an LLM and asks for one summary of them. It stores the reply with `transformed_response`, which records a transformation event and replaces `query.current()`. It leaves `documents()` untouched.

`answers::Simple` then answers from the summary instead of the rendered documents; see [Generating answers](/querying/answering/). The documents stay on the query, so you can still list sources.

To change the instruction, pass your own `prompt_template`:

::snippet{src="@snippets/src/querying/reranking.rs" anchor="summary_prompt"}

The template sees only `documents`, a list of documents with `content` and `metadata`. The question is not in its context.

You can chain response transformers. Put `Rerank` before `Summary` and the LLM summarizes only the kept documents.

## Custom response transformers

::snippet{src="@snippets/src/querying/reranking.rs" anchor="custom_response_transformer"}

`TransformResponse` has one async method, `transform_response`. Implement it with `#[async_trait]`, from the `async-trait` crate or `swiftide::reexports::async_trait`. The trait requires `Clone` through `DynClone`, so derive it.

This transformer edits the list in place through `documents_mut`. It keeps the first document per `path` metadata value, in the order the documents arrive, and caps the list at `max`. Documents without a `path` share one empty key, so only the first of them is kept. It does not call `transformed_response`, so `answers::Simple` still renders the documents one by one. Call `transformed_response` only when your transformer produces text for the answer to use in place of the documents, as `Summary` does.

Place it after `then_retrieve`:

::snippet{src="@snippets/src/querying/reranking.rs" anchor="one_per_file_pipeline"}

For a small change, a closure works too. It must be synchronous, take a `Query<states::Retrieved>` and return `anyhow::Result<Query<states::Retrieved>>`. It must also be `Clone + Send + Sync`, so capture only values that are themselves `Clone`:

::snippet{src="@snippets/src/querying/reranking.rs" anchor="closure_response_transformer"}

## Gotchas

- In 0.33 the plain `Qdrant` retriever returns `Debug`-quoted content, so `Rerank` scores the quoted text. See the gotcha in [Retrieval and search strategies](/querying/retrieval-and-search-strategies/#gotchas) for the docs-qa workaround.
- `Rerank` changes neither `current()` nor the transformation history. The `Retrieved` events in `history()` hold copies of the documents as the retriever returned them. Comparing them with `documents()` shows what was dropped only if no wrapper edited the documents after retrieval and the query retrieved once.
- The `Ragas` evaluator records contexts before `Rerank` runs, so its dataset holds the retrieved candidates, not the kept documents; see [The dataset](/querying/evaluation/#the-dataset). A custom evaluator can read the reranked documents at its second call, after the answer.
- To use another rerank model, build a `TextRerank` yourself, wrap it in `Arc::new(tokio::sync::Mutex::new(..))` and pass it to `model`. That needs direct `fastembed` and `tokio` dependencies, because Swiftide does not re-export `fastembed`.
- Clones of a `Rerank` share one model behind a Tokio mutex. Scoring is synchronous CPU work, so concurrent queries, such as those from `query_all`, take turns at the rerank step.
- If retrieval returns `top_k` documents or fewer, `Rerank` only reorders them. Retrieve more candidates than you keep.

## Reference

- [`Rerank`](docsrs:swiftide/integrations/fastembed/struct.Rerank.html)
- [`RerankBuilder`](docsrs:swiftide/integrations/fastembed/struct.RerankBuilder.html)
- [`response_transformers::Summary`](docsrs:swiftide/query/response_transformers/struct.Summary.html)
- [`TransformResponse`](docsrs:swiftide/traits/trait.TransformResponse.html)
- [`Query`](docsrs:swiftide/query/struct.Query.html)
- [`Document`](docsrs:swiftide/query/struct.Document.html)
- [`query::Pipeline`](docsrs:swiftide/query/struct.Pipeline.html)
- [`answers::Simple`](docsrs:swiftide/query/answers/struct.Simple.html)
- The [docs-qa showcase](/showcases/docs-qa/) and the [FastEmbed integration](/integrations/fastembed/)

## Next

[Generating answers](/querying/answering/)
