---
title: "Usage metrics"
description: "Run the usage_metrics and langfuse examples to print token counters per model from an indexing pipeline and send one model call to Langfuse."
sidebar:
  order: 6
identLint:
  ignore: [HashMap, LevelFilter, "LevelFilter::DEBUG"]
---

After this recipe you have two examples from the swiftide repository running on your machine. `usage_metrics.rs` prints the token counters of an indexing pipeline, labelled with the model and your own metadata. `langfuse.rs` sends one model call, with its input, output and token usage, to Langfuse. [Metrics and pipeline statistics](/observability/metrics-and-statistics/) and [Langfuse](/observability/langfuse/) cover the reference details; this page is the end-to-end run.

## Setup

Token counters sit behind the `metrics` feature flag. The recorder comes from the `metrics` crate. In your own crate, add:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,qdrant,metrics
cargo add metrics@0.24
cargo add tokio --features macros,rt-multi-thread
```

Other clients that emit counters need their own feature flag as well: `anthropic`, `aws-bedrock`, `ollama` or `groq`. In your own binary, install a rustls crypto provider at the start of `main`, before you build the first client. [Crypto provider panic](/reference/troubleshooting-and-faq/#crypto-provider-panic) shows how.

To run the example itself, clone the swiftide repository, set `OPENAI_API_KEY`, and start Qdrant on `localhost:6334` (or set `QDRANT_URL`). Then run from the repository root:

```sh
export OPENAI_API_KEY=sk-...
cargo run -p swiftide-examples --example usage-metrics
```

`swiftide-examples` enables many integrations, including LanceDB, Kafka, Fluvio and DuckDB, so the first build takes a while and needs their native toolchains.

## Recorder

::snippet{src="@swiftide/examples/usage_metrics.rs" anchor="recorder" collapse="14-32,47-63,69-75" mark="79"}

Swiftide emits its counters through the `metrics` facade from the metrics-rs project. The facade sends them to whichever `Recorder` you install, and without one every counter goes nowhere. `PrintRecorder` is the basic example from metrics-rs: it prints every increment. The example's `main` calls `init_print_logger()` before it builds the client, because a counter emitted before the recorder exists is lost. In an application you install an exporter crate in its place, such as a Prometheus exporter.

## Client metadata

::snippet{src="@swiftide/examples/usage_metrics.rs" anchor="setup"}

Each entry in `metric_metadata` becomes a label on every counter this client emits, next to the `model` label Swiftide adds. Give each client a label for the tenant, feature or environment it serves, and your metrics backend can sum per label. The method only exists with the `metrics` feature flag.

## Indexing pipeline

::snippet{src="@swiftide/examples/usage_metrics.rs" anchor="pipeline"}

Each batch of 10 chunks is one embedding call, and each call increments all three counters once. `vector_size(1536)` must match `text-embedding-3-small`. `FileLoader` walks the directory with the `ignore` crate, so it skips hidden and gitignored paths such as `target/`.

## Output

The example keeps the output of one run, embedding with `text-embedding-3-small`, in a comment at the end of `main`. These are its first lines:

```text
(counter) registered key swiftide.usage.prompt_tokens with unit None and description "token usage for the prompt"
(counter) registered key swiftide.usage.completion_tokens with unit None and description "token usage for the completion"
(counter) registered key swiftide.usage.total_tokens with unit None and description "total token usage"
counter increment for 'Key(swiftide.usage.prompt_tokens, [example = metadata, model = text-embedding-3-small])': 356
counter increment for 'Key(swiftide.usage.completion_tokens, [example = metadata, model = text-embedding-3-small])': 0
counter increment for 'Key(swiftide.usage.total_tokens, [example = metadata, model = text-embedding-3-small])': 356
```

The three `registered key` lines appear once, on the first emit. After that, every embedding call prints three increment lines: prompt, completion and total. Embeddings have no completion, so `completion_tokens` grows by 0. The three counters and the clients that emit them are listed in [Counters](/observability/metrics-and-statistics/#counters).

## Langfuse traces

The `langfuse` feature flag sends traces to [Langfuse](https://langfuse.com/) through a `tracing` layer:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,langfuse
cargo add tracing anyhow
cargo add tracing-subscriber --features env-filter
```

Set `LANGFUSE_PUBLIC_KEY` and `LANGFUSE_SECRET_KEY`, or their fallbacks `LANGFUSE_INIT_PROJECT_PUBLIC_KEY` and `LANGFUSE_INIT_PROJECT_SECRET_KEY`. `LangfuseLayer::default()` panics when a key is missing or empty. `LANGFUSE_URL` defaults to `http://localhost:3000`. The example also needs `OPENAI_API_KEY`:

```sh
RUST_LOG=swiftide=debug,langfuse=debug cargo run -p swiftide-examples --example langfuse
```

::snippet{src="@swiftide/examples/langfuse.rs" anchor="setup"}

The `EnvFilter` sits on the registry itself, so it filters for every layer, the Langfuse layer included. Without `RUST_LOG` it lets only errors through, and Langfuse receives no spans. That is why the run command sets `RUST_LOG`. [Layer setup](/observability/langfuse/#layer-setup) explains the layer's own `LevelFilter::DEBUG` and its batching.

::snippet{src="@swiftide/examples/langfuse.rs" anchor="prompt"}

With the `langfuse` feature flag, the client's `prompt` call opens a span of type `GENERATION`. Langfuse shows that generation with the model, the prompt, the answer and the token usage. [Reported data](/observability/langfuse/#reported-data) covers what each client and agent records.

## Variations

### Usage callbacks

When you want the numbers in your own code rather than in a metrics backend, set `on_usage` or `on_usage_async` on the client builder. The callback receives the `Usage` of each call and needs no feature flag. See [Usage callbacks](/models/errors-backoff-and-usage/#usage-callbacks).

### Pipeline statistics

A successful indexing run logs a `Pipeline completed` event with its node counts. [Pipeline statistics](/observability/metrics-and-statistics/#pipeline-statistics) covers it and its limits.

## Gotchas

- `AnthropicBuilder::metric_metadata` takes an `Option<HashMap<String, String>>`, so wrap the map in `Some`. The OpenAI-compatible and `AwsBedrock` builders take the map itself.
- `metrics::set_global_recorder` errors on a second call. In tests that each install a recorder, use `metrics::with_local_recorder` instead.
- The Langfuse layer does not send its last partial batch on exit. A short program can end before it leaves. Keep a clone of the layer and call `flush` before `main` returns, as [Final flush](/observability/langfuse/#final-flush) shows.

## Reference

- [`GenericOpenAIBuilder`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html) (`metric_metadata`, `on_usage`)
- [`AnthropicBuilder`](docsrs:swiftide/integrations/anthropic/struct.AnthropicBuilder.html) (`metric_metadata`)
- [`AwsBedrockBuilder`](docsrs:swiftide/integrations/aws_bedrock_v2/struct.AwsBedrockBuilder.html) (`metric_metadata`)
- [`Pipeline`](docsrs:swiftide/indexing/struct.Pipeline.html)
- [`FileLoader`](docsrs:swiftide/indexing/loaders/struct.FileLoader.html)
- [`ChunkMarkdown`](docsrs:swiftide/indexing/transformers/struct.ChunkMarkdown.html)
- [`Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html)
- [`Qdrant`](docsrs:swiftide/integrations/qdrant/struct.Qdrant.html)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`LangfuseLayer`](docsrs:swiftide/langfuse/struct.LangfuseLayer.html)
- [Feature flags](/reference/feature-flags/)

## Next

[Hybrid search and reranking](/cookbook/hybrid-search-and-reranking/)
