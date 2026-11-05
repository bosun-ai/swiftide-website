---
title: "Metrics and pipeline statistics"
description: "Count token usage per model with the metrics feature flag and a metrics-rs recorder, and report what an indexing run processed from its statistics or from your own counters."
sidebar:
  order: 3
identLint:
  ignore: [HashMap, total_nodes]
---

Your metrics backend gets a token counter per model and per label you choose, and each successful indexing run reports how many nodes it handled and how long it took.

## Usage metrics

Add the `metrics` feature flag next to the integration you use, plus the `metrics` crate for the recorder:

```sh
cargo add swiftide@{{swiftide_version}} --features metrics,openai
cargo add metrics@0.24
```

::snippet{src="@swiftide/examples/usage_metrics.rs" anchor="setup" title="examples/usage_metrics.rs"}

With the `metrics` feature flag, a model client counts the tokens of every call whose response reports usage. `metric_metadata` takes a `HashMap<String, String>`. Each entry becomes a label on every counter this client emits. Swiftide always adds a `model` label with the model of the call. Two clients with different metadata, such as one per tenant or per pipeline, show up as separate series.

### Recorder

::snippet{src="@swiftide/examples/usage_metrics.rs" anchor="recorder" title="examples/usage_metrics.rs" collapse="14-32"}

Swiftide only calls the metrics-rs facade. It never installs a recorder, so your app does that once at startup with `metrics::set_global_recorder`. Without a recorder the counters do nothing. The example's recorder prints every call. An exporter crate such as `metrics-exporter-prometheus` installs a recorder the same way. It then serves the values to your backend.

Install the recorder before the first model call. Swiftide sends the counter descriptions once per process, on the first usage it emits. A recorder installed later never receives them.

### Counters

| Counter | Description |
| --- | --- |
| `swiftide.usage.prompt_tokens` | token usage for the prompt |
| `swiftide.usage.completion_tokens` | token usage for the completion |
| `swiftide.usage.total_tokens` | total token usage |

These clients emit them:

- OpenAI-compatible clients (`OpenAI`, `Groq`, `Gemini`, `Ollama`, `OpenRouter`, `Dashscope`): chat completions, streaming, `prompt`, structured prompts, embeddings and the Responses API.
- `Anthropic`: prompts, completions and streaming.
- `AwsBedrock`.

Local embedders such as FastEmbed emit nothing. These three counters are all the `metrics` feature flag emits. It adds no pipeline or agent timing metrics.

::snippet{src="@swiftide/examples/usage_metrics.rs" anchor="pipeline" title="examples/usage_metrics.rs"}

In this pipeline, the embedder sends one request per batch of 10 chunks, and each request adds to all three counters. Run it from a clone of the Swiftide repository, with Qdrant listening on `localhost:6334` and `OPENAI_API_KEY` set:

```sh
cargo run -p swiftide-examples --example usage-metrics
```

The examples package is not a default workspace member, so `-p swiftide-examples` is required from the workspace root. That package enables many integrations, including LanceDB, Kafka, Fluvio and DuckDB, so the first build takes a while.

The example keeps the output of one run in a comment at the end of `main`. These are its first lines, one per `println!`, for embeddings with `text-embedding-3-small`:

```text
(counter) registered key swiftide.usage.prompt_tokens with unit None and description "token usage for the prompt"
(counter) registered key swiftide.usage.completion_tokens with unit None and description "token usage for the completion"
(counter) registered key swiftide.usage.total_tokens with unit None and description "total token usage"
counter increment for 'Key(swiftide.usage.prompt_tokens, [example = metadata, model = text-embedding-3-small])': 356
counter increment for 'Key(swiftide.usage.completion_tokens, [example = metadata, model = text-embedding-3-small])': 0
counter increment for 'Key(swiftide.usage.total_tokens, [example = metadata, model = text-embedding-3-small])': 356
```

Embeddings have no completion, so `completion_tokens` grows by 0.

### Metrics gotchas

- `AnthropicBuilder::metric_metadata` takes an `Option<HashMap<String, String>>`, so wrap the map in `Some`. The OpenAI-compatible and `AwsBedrock` builders take the map itself.
- The counters only go up. Compute rates and per-period totals in your backend.
- To act on the usage of one call in your own code, set an `on_usage` callback instead. See [Errors, backoff, usage and tokens](/models/errors-backoff-and-usage/).

## Pipeline statistics

New in 0.33, every indexing pipeline carries a `StatsCollector`. `Pipeline::stats` returns a `PipelineStats` snapshot of it, and `Pipeline::stats_collector` returns the collector itself.

`PipelineStats` holds node counts, per-model `token_usage` and timing, with helpers such as `duration` and `nodes_per_second`.

`Pipeline::run` takes the pipeline by value, so you cannot call `stats` during or after a run, although its API docs say you can. Read the numbers of a run from tracing instead. At the end of a successful run, `run` logs one INFO event, `Pipeline completed`, with the fields `nodes_processed`, `nodes_stored`, `total_tokens`, `total_requests`, `elapsed_secs` and `nodes_per_sec`.

Enable INFO for the `swiftide_indexing` target to see them, for example with `RUST_LOG=swiftide_indexing=info`. [Tracing](/observability/tracing/) shows the subscriber setup.

### Your own collector

`StatsCollector` is thread-safe and works outside a pipeline. To read the numbers of a run in code, create one yourself, share it through an `Arc`, and fill it from a closure transformer:

::snippet{src="@snippets/src/observability/metrics_and_statistics.rs" anchor="own_stats_collector"}

`pipeline.stats()` works on the built pipeline until `run` takes it, and before the run every count is 0. Your collector outlives the run. `start` and `complete` set the times that `duration` and `nodes_per_second` use, and `nodes_per_second` returns `None` when no node was counted.

### Statistics gotchas

- `nodes_processed` and `nodes_stored` both count the nodes that reach the end of the pipeline. They are the same number, even when the last step is not storage.
- Nothing in Swiftide records token usage, failed nodes or transformations into the collector during a run. `total_tokens` and `total_requests` in the `Pipeline completed` event are always 0. So are `nodes_failed` and `transformations_applied`.
- Each step you add copies the collector instead of sharing it, so a transformer cannot write into it during the run.
- The event and the `total_nodes` span field only appear when the run succeeds. If a node error reaches the end of the stream, `run` returns the error and logs neither.
- For token totals per pipeline, give the client of each pipeline its own `metric_metadata` label. The counters add up across all runs of the process, so take per-run numbers as differences in your backend.

## Node counters

The ask-your-codebase showcase reports its counts without `stats`. It threads three counting transformers through its pipeline:

::snippet{src="@swiftide/examples/ask-your-codebase/src/index.rs" anchor="pipeline" title="examples/ask-your-codebase/src/index.rs"}

`files_found.count()` sees every source file that passes the test-file filter. `files_changed.count()` sits after `filter_cached`, so it only sees files the [node cache](/indexing/caching/) did not skip. `chunks_indexed.count()` sits after storage. Each `Tally` wraps an `Arc<AtomicU64>`, and the showcase measures the elapsed time with its own `Instant`.

The same pattern as a closure [transformer](/indexing/transformers-and-metadata/):

::snippet{src="@snippets/src/observability/metrics_and_statistics.rs" anchor="count_nodes"}

A closure from `TextNode` to `anyhow::Result<TextNode>` is a transformer. This one adds 1 to a shared counter and returns the node unchanged. Steps run nodes concurrently, so the counter is an atomic behind an `Arc`, and the closure holds its own clone of it.

The showcase prints these lines from its counters. It indexed the `swiftide-agents` crate with `claude-haiku-5-5` through an OpenAI-compatible gateway, and FastEmbed embeddings on the CPU:

```text
Indexed 34 chunks from 14 new or changed files (14 files found, 0 unchanged) in 91.9s
```

On the second run the Redb node cache skipped every file:

```text
Indexed 0 chunks from 0 new or changed files (14 files found, 14 unchanged) in 0.6s
```

The docs-qa showcase takes another route: after the run, it asks Qdrant how many points the collection holds. That counts the whole collection, not only this run. The number matches the run here because the captured run used `--recreate`, which drops the collection first. With `claude-haiku-5-5` writing chunk titles and keywords, it printed:

```text
Indexed 43 chunks into `docs-qa` in 71.7s
```

## Reference

- [`Pipeline`](docsrs:swiftide/indexing/struct.Pipeline.html) (`stats`, `stats_collector`, `run`)
- [`PipelineStats`](docsrs:swiftide/indexing/statistics/struct.PipelineStats.html)
- [`StatsCollector`](docsrs:swiftide/indexing/statistics/struct.StatsCollector.html)
- [`ModelUsage`](docsrs:swiftide/indexing/statistics/struct.ModelUsage.html)
- [`GenericOpenAIBuilder`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html) (`metric_metadata`)
- [`AnthropicBuilder`](docsrs:swiftide/integrations/anthropic/struct.AnthropicBuilder.html)
- [`AwsBedrockBuilder`](docsrs:swiftide/integrations/aws_bedrock_v2/struct.AwsBedrockBuilder.html)
- [Feature flags](/reference/feature-flags/)
- [metrics-rs](https://docs.rs/metrics/0.24)

## Next

[Integrations overview](/integrations/overview/)
