---
title: "Evaluating with Ragas"
description: "Record each question's retrieved documents and answer with the Ragas evaluator, and export them with ground truths as a JSON dataset for the Python ragas toolkit."
sidebar:
  order: 6
---

You end with a JSON file that holds every question, the answer your query pipeline gave, the documents it retrieved and the answer you expected. You can score that file and compare it between runs. The scores at the end come from the evaluation in the [docs-qa](/showcases/docs-qa/) showcase.

## Evaluation run

`Ragas` needs no feature flag. The snippets below use the OpenAI client and Qdrant, which sit behind the `openai` and `qdrant` feature flags:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,qdrant
```

::snippet{src="@snippets/src/querying/evaluation.rs" anchor="prepare_questions"}

`Ragas` is an evaluator: a type that implements `EvaluateQuery`. It holds the dataset, and its clones share that dataset. Pass one clone to the pipeline with `evaluate_with` and keep the other to read the results.

`ragas.questions()` returns one pending query per question in the dataset, and `query_all` runs them all. The pipeline calls the evaluator twice per question. Right after `then_retrieve`, `Ragas` stores the content of the retrieved documents as the contexts of that question. After `then_answer`, it stores the answer. `to_json` then writes the dataset.

### Evaluation in docs-qa

The [docs-qa](/showcases/docs-qa/) showcase runs the same steps on its own question set:

::snippet{src="@swiftide/examples/docs-qa/src/eval.rs" anchor="eval" title="examples/docs-qa/src/eval.rs"}

`evaluated_pipeline` builds the same pipeline as the `ask` command and adds `evaluate_with(ragas)` to it. docs-qa also enables the `fastembed` feature flag for its hybrid search.

## Questions and ground truths

`Ragas::from_prepared_questions` takes anything that converts into an `EvaluationDataSet`. An `EvaluationDataSet` converts from:

- `Vec<String>` or `&[String]`: questions only, with an empty ground truth.
- `Vec<(String, String)>`: pairs of question and ground truth, as in the first snippet.
- a JSON string, through `parse`, in the format that `to_json` writes.

docs-qa passes `&QuestionSet`, its own YAML-backed type, which implements `From<&QuestionSet> for EvaluationDataSet`. Implement `From` for your own type in the same way.

A ground truth is the answer you expect, written in your own words. Scores that compare the answer with the ground truth need it, so write one for every question you can.

### Baseline ground truths

Writing ground truths by hand takes time. You can let a first run write them and review them afterwards.

::snippet{src="@snippets/src/querying/evaluation.rs" anchor="load_dataset"}

The `evaluate` closure builds a pipeline with `evaluate_with(ragas.clone())` and runs `query_all` over `ragas.questions()`, so each call fills the dataset with answers and contexts. `record_answers_as_ground_truth` then copies each recorded answer into its ground truth. Read the file and fix wrong answers before you treat it as the baseline. Parsing an export keeps its ground truths. The next run overwrites the answers and contexts it loaded.

## The dataset

The export is a pretty-printed JSON array with one object per question. This is the output that the unit test for `to_json` expects for one question with a ground truth that has not run yet:

```json
[
  {
    "answer": "",
    "contexts": [],
    "ground_truth": "A programming language",
    "question": "What is Rust?"
  }
]
```

The keys are in alphabetical order, unless a crate in your build enables the serde_json feature that preserves insertion order. The rows are in hash map order, which changes from run to run. docs-qa sorts the rows back into the order of its questions file, so the exports of two runs can be diffed.

`contexts` holds every retrieved document, recorded before any response transformer runs. docs-qa retrieves 20 candidates and reranks them down to 10, so its `ragas.json` has 20 contexts per question, while the answer generator saw 10. Ragas metrics computed from `contexts` therefore score the retriever's 20 candidates, not the 10 documents the answer generator saw.

## Scores

`Ragas` computes no scores. The JSON is input for the Python [ragas](https://docs.ragas.io) toolkit, which calculates metrics such as faithfulness and context recall.

docs-qa shows numbers without Python. Its judge asks a large language model (LLM) to split the ground truth and the answer into claims and to check each one, using [structured output](/models/structured-output/). Its scores are ratios modelled on the ragas metrics of the same names. In this run `claude-sonnet-5-5` both answered and judged:

```sh
./target/debug/docs-qa --model claude-sonnet-5-5 eval --judge-model claude-sonnet-5-5 \
  --out examples/docs-qa/ragas.json
```

```text
#   question                                              ctx faithful   answer  context
                                                                         recall   recall
1   Which crate features and environment variable do I …   10     1.00     1.00     1.00
2   What does the agents-contrib-context feature provid…   10     1.00     1.00     1.00
3   What are the default limits of the read_file tool?     10     1.00     1.00     1.00
4   Which storage and retrieval backends does Swiftide …   10     1.00     1.00     1.00
5   What should I run before opening a pull request?       10     1.00     1.00     1.00
6   What is a task in Swiftide?                            10     1.00     1.00     1.00
7   When does ContextCompression summarize the conversa…   10     1.00     1.00     1.00
8   Which commit message convention does the project fo…   10     1.00     1.00     1.00
9   How do I add a custom tool to an agent with the too…   10     1.00     1.00     1.00
    mean                                                          1.00     1.00     1.00

RAGAS dataset written to examples/docs-qa/ragas.json
```

- `ctx` is the number of chunks the LLM saw after reranking.
- `faithful` is the share of the answer's claims that those chunks support.
- `answer recall` is the share of the ground truth's claims that the answer states.
- `context recall` is the share of the ground truth's claims that those chunks support.

Before retrieval was tuned, two questions scored lower. One fix added page and title to the rerank template; see [Rerank templates](/querying/reranking-and-response-transformers/#rerank-templates). The other changed the indexing prompt so code-only chunks get titles that name the API they show; see the [docs-qa showcase](/showcases/docs-qa/).

## Custom evaluators

Any type that implements `EvaluateQuery` can take the place of `Ragas`. The trait has one async method, `evaluate`, which receives a `QueryEvaluation`: `RetrieveDocuments` with the query after retrieval, or `AnswerQuery` with the answered query. The trait requires `Clone`, so keep shared state behind an `Arc`. This evaluator collects the questions for which the retriever found no documents:

::snippet{src="@snippets/src/querying/evaluation.rs" anchor="custom_evaluator"}

::snippet{src="@snippets/src/querying/evaluation.rs" anchor="use_custom_evaluator"}

An error from `evaluate` fails the query, so return `Ok` for anything that should only be recorded.

## Gotchas

- `evaluate_with` exists only on a pending pipeline. Call it before `then_retrieve`; its position among query transformers does not matter.
- `Ragas` matches each query to its row by `original()`. A query whose question is not in the dataset fails with `Question not found`, and so does `query_all`. Query transformers change `current()`, not `original()`, so they do not affect the match.
- Rows are keyed by question text, so each question must be unique. docs-qa rejects a questions file with duplicates.
- `query_all` returns answers in the order they finish; see [Query methods](/querying/overview/#query-methods). docs-qa matches each answer back to its question with `original()`.
- Without the docs-qa workaround in [Retrieval and search strategies](/querying/retrieval-and-search-strategies/), Qdrant contexts in the export arrive `Debug`-quoted in 0.33.
- Nine questions and one LLM judge make a regression signal for one corpus, not a benchmark. Keep the question set and the judge model fixed when you compare runs.

## Reference

- [`Ragas`](docsrs:swiftide/query/evaluators/ragas/struct.Ragas.html)
- [`EvaluationDataSet`](docsrs:swiftide/query/evaluators/ragas/struct.EvaluationDataSet.html)
- [`EvaluateQuery`](docsrs:swiftide/traits/trait.EvaluateQuery.html)
- [`QueryEvaluation`](docsrs:swiftide/query/enum.QueryEvaluation.html)
- [`query::Pipeline`](docsrs:swiftide/query/struct.Pipeline.html)
- [`Query`](docsrs:swiftide/query/struct.Query.html)
- [docs-qa source on GitHub](https://github.com/bosun-ai/swiftide/tree/{{swiftide_sha}}/examples/docs-qa)

## Next

[Models](/models/overview/)
