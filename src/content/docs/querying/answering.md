---
title: "Generating answers"
description: "Turn retrieved documents into an answer with answers::Simple, a custom prompt and a document template that lets the answer cite the file each statement comes from."
sidebar:
  order: 5
---

The last step of a query pipeline sends the question and the retrieved documents to a large language model (LLM). Here you set that step up so every answer names the file it came from, the way the [ask-your-codebase](/showcases/ask-your-codebase/) showcase does.

`answers::Simple` needs no feature flag. The snippets below use the OpenAI client and Qdrant, which sit behind the `openai` and `qdrant` feature flags:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,qdrant
```

::snippet{src="@swiftide/examples/ask-your-codebase/src/query.rs" anchor="query_pipeline" title="examples/ask-your-codebase/src/query.rs" mark="15-22"}

## The answer generator

`then_answer` takes an answer generator and moves the query from `Retrieved` to `Answered`. Swiftide ships one: `answers::Simple`. It renders a prompt, sends it through any client that implements `SimplePrompt`, and stores the reply as the answer.

`answers::Simple::from_client(llm)` builds it with the default prompt. The builder lets you replace two templates:

- `prompt_template` is the whole prompt.
- `document_template` renders each retrieved document before they are joined. It is optional.

Without a custom prompt, the model receives this text, shown here with `What is love?` as the question and `My context` as the documents:

```text
Answer the following question based on the context provided:
What is love?

## Constraints
* Do not include any information that is not in the provided context.
* If the question cannot be answered by the provided context, state that it cannot be answered.
* Answer the question completely and format it as markdown.

## Context

---
My context
---
```

The prompt template sees four variables:

- `question`: the question the user asked, from `Query::original`.
- `original`: the same text as `question`.
- `current`: the current query, from `Query::current`.
- `documents`: the retrieved documents as one string, or the text of a response transformer that changed `current` (see below).

Prompts are Tera templates; [Prompts and templates](/models/prompts-and-templates/) covers the syntax. `original` and `current` need 0.33 or later; see [Upgrading](/reference/upgrading/).

## Source citations

The answer can only name a file if the prompt contains the file name. The document template puts it there.

::snippet{src="@swiftide/examples/ask-your-codebase/src/query.rs" anchor="prompts" title="examples/ask-your-codebase/src/query.rs"}

`CHUNK_TEMPLATE` renders one document. Its template context is the serialized `Document`, so `content` is the chunk text and `metadata` holds the metadata fields your storage kept. ask-your-codebase writes more metadata while indexing, but its LanceDB table keeps only `path`, so `{{ metadata.path }}` becomes a heading above each chunk. `answers::Simple` joins the rendered documents with `---` lines and inserts the result as `{{ documents }}`.

`ANSWER_PROMPT` then asks for a citation after each statement. With `claude-sonnet-5-5`, the answers follow it:

```text
$ ./target/debug/ask-your-codebase ask --model claude-sonnet-5-5 --path swiftide-agents "How does an agent decide to stop?"
# How an agent decides to stop

An agent stops when something calls `Agent::stop` (or `stop_with_error`), which moves the state to `State::Stopped(StopReason)`. The main loop in `run_agent` checks that state after each step. `Agent::stop` is a no-op if the agent is already stopped (`src/agent.rs`).

## The stop mechanism

- `Agent::stop(reason)` fires the `OnStop` hooks with the reason, then sets `self.state = State::Stopped(reason)`. It only takes effect at the end of the current loop iteration (`src/agent.rs`).
...

Sources:
  - src/agent.rs
  - src/tools/control.rs
  - src/lib.rs
  ...
```

The `Sources:` list does not come from the model. The answered query still holds the documents it was answered from, so the showcase reads their paths back:

::snippet{src="@swiftide/examples/ask-your-codebase/src/query.rs" anchor="sources" title="examples/ask-your-codebase/src/query.rs"}

`answer` returns the text and `documents` returns the documents the answer was built from. A response transformer can reorder or trim that list first: the [docs-qa](/showcases/docs-qa/) showcase reranks its documents, so its numbered list of page and section titles comes best first.

## Original and current question

A query transformer such as `GenerateSubquestions` rewrites the current query. `question` in the prompt stays the user's question, so the model answers what was asked, not the list of subquestions. Use `current` when the model should also see what the search looked for:

::snippet{src="@snippets/src/querying/answering.rs" anchor="current_in_prompt"}

`answers::Simple` decides what goes into `documents` like this:

- If a response transformer changed `current` after retrieval, `documents` is that text and the document template is skipped. `response_transformers::Summary` works this way; see [Reranking and response transformers](/querying/reranking-and-response-transformers/).
- Otherwise each document is rendered with the document template, or as its plain content without one.

The check compares `current` with the query recorded by the `Retrieved` event, so a rewritten query from before retrieval does not count as a change. A reranker that only reorders documents leaves `current` alone, so the document template still applies. [Transforming queries](/querying/transforming-queries/) explains the transformation history.

## Custom answer generators

Implement the `Answer` trait to control the last step yourself. This one skips the LLM call when the retriever found nothing:

::snippet{src="@snippets/src/querying/answering.rs" anchor="custom_answer"}

`Query::answered` stores the text and moves the query to `Answered`. The trait requires `Clone`, so derive it. Every other case goes to the wrapped `answers::Simple`. The generator goes where `answers::Simple` went:

::snippet{src="@snippets/src/querying/answering.rs" anchor="custom_answer_pipeline"}

`then_answer` also accepts a synchronous `Fn(Query<states::Retrieved>) -> Result<Query<states::Answered>> + Send + Sync + Clone` closure, for example a fixed answer in a test. A closure that captures a value which is not `Clone` does not compile.

## Gotchas

- `prompt_template` takes a `Prompt`, so write `.into()` or `Prompt::from`. `document_template` converts for you, so a `&'static str` works directly.
- A document template that reads a missing metadata key fails to render, and the query returns that error. The [docs-qa](/showcases/docs-qa/) templates guard each key with Tera's `default` filter, as in `{{ metadata.Title | default(value="") }}`.
- After answering, `current` holds the answer; see [Transforming queries](/querying/transforming-queries/) for reading the rewritten query from `history`.
- The model can only cite files among the retrieved documents. A chunk outside the top results is invisible to it. Raise `top_k` for broad questions; see [Retrieval and search strategies](/querying/retrieval-and-search-strategies/).
- `answers::Simple` makes one LLM call and returns its error. Wrap the client in `LanguageModelWithBackOff` to retry transient failures, as docs-qa does; see [Errors, backoff and usage](/models/errors-backoff-and-usage/).

## Reference

- [`query::Pipeline::then_answer`](docsrs:swiftide/query/struct.Pipeline.html#method.then_answer)
- [`answers::Simple`](docsrs:swiftide/query/answers/struct.Simple.html)
- [`Answer`](docsrs:swiftide/traits/trait.Answer.html)
- [`Query`](docsrs:swiftide/query/struct.Query.html)
- [`Document`](docsrs:swiftide/query/struct.Document.html)
- [`query_transformers::GenerateSubquestions`](docsrs:swiftide/query/query_transformers/struct.GenerateSubquestions.html)
- [`TransformationEvent`](docsrs:swiftide/query/enum.TransformationEvent.html)
- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`response_transformers::Summary`](docsrs:swiftide/query/response_transformers/struct.Summary.html)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)

## Next

[Evaluating with Ragas](/querying/evaluation/)
