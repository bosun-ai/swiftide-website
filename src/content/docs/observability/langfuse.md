---
title: "Langfuse"
description: "Send agent runs, tool calls and LLM generations with their input, output and token usage to Langfuse with LangfuseLayer."
sidebar:
  order: 2
identLint:
  ignore: [LevelFilter, "LevelFilter::DEBUG", Targets, EnvFilter]
---

Turn on the `langfuse` feature flag and add one tracing layer, and your agent runs, tool calls and OpenAI-compatible or Bedrock model calls show up in [Langfuse](https://langfuse.com/), the model calls with their input, output and token usage.

## Installation

```sh
cargo add swiftide@{{swiftide_version}} --features langfuse,openai
cargo add tracing anyhow
cargo add tracing-subscriber --features env-filter
cargo add tokio --features macros,rt-multi-thread
```

The `langfuse` feature flag adds `swiftide::langfuse`. `LangfuseLayer::default()` reads its settings from the environment:

| Variable | Also read | Default |
| --- | --- | --- |
| `LANGFUSE_PUBLIC_KEY` | `LANGFUSE_INIT_PROJECT_PUBLIC_KEY` | none, required |
| `LANGFUSE_SECRET_KEY` | `LANGFUSE_INIT_PROJECT_SECRET_KEY` | none, required |
| `LANGFUSE_URL` | | `http://localhost:3000` |

Langfuse's own Docker setup uses the second names.

## Local Langfuse

Pick the keys first; the container creates its project with them:

```sh
export LANGFUSE_PUBLIC_KEY=pk-lf-local LANGFUSE_SECRET_KEY=sk-lf-local OPENAI_API_KEY=sk-...
```

The release-notes-agent README starts Langfuse v2 like this; the last `-e` line adds a login user:

```sh
docker network create langfuse
docker run -d --name langfuse-db --network langfuse \
  -e POSTGRES_USER=postgres -e POSTGRES_PASSWORD=postgres postgres:16
docker run -d --name langfuse --network langfuse -p 3000:3000 \
  -e DATABASE_URL=postgresql://postgres:postgres@langfuse-db:5432/postgres \
  -e NEXTAUTH_SECRET=secret -e SALT=salt -e NEXTAUTH_URL=http://localhost:3000 \
  -e LANGFUSE_INIT_ORG_ID=local -e LANGFUSE_INIT_PROJECT_ID=release-notes \
  -e LANGFUSE_INIT_PROJECT_PUBLIC_KEY="$LANGFUSE_PUBLIC_KEY" \
  -e LANGFUSE_INIT_PROJECT_SECRET_KEY="$LANGFUSE_SECRET_KEY" \
  -e LANGFUSE_INIT_USER_EMAIL=me@example.com -e LANGFUSE_INIT_USER_PASSWORD=password \
  langfuse/langfuse:2
```

Open `http://localhost:3000`, sign in, and pick the `release-notes` project to see its traces.

## Layer setup

The docs-qa showcase adds the layer when you pass `--langfuse`:

::snippet{src="@swiftide/examples/docs-qa/src/telemetry.rs" anchor="langfuse" title="examples/docs-qa/src/telemetry.rs"}

`LangfuseLayer::default()` panics on a missing or empty key, so docs-qa checks the keys first. The layer also starts a Tokio task that sends a batch every 5 seconds.

Swiftide records all Langfuse data at the `debug` level, so the Langfuse layer gets its own `LevelFilter::DEBUG` while the terminal layer stays quieter. Running docs-qa also needs Qdrant and an index; its [README](https://github.com/bosun-ai/swiftide/tree/{{swiftide_sha}}/examples/docs-qa) lists the steps.

## Target filter

The release-notes-agent showcase adds the layer behind `--trace`, with a `Targets` filter that passes only Swiftide's spans, shown in [Filters per layer](/observability/tracing/#filters-per-layer). Add `--trace` to any of its run commands:

```sh
cargo run -p release-notes-agent -- --version 0.34.0 --trace
```

Its key check ignores the `LANGFUSE_INIT_PROJECT_*` names and lets an empty key through to the panic. Its README states that `--trace` was never run against a live Langfuse.

## Final flush

::snippet{src="@snippets/src/observability/langfuse.rs" anchor="flush"}

A layer installed as the global subscriber is never dropped, so nothing sends the last partial batch. Events still buffered when `main` returns are lost. Clones of `LangfuseLayer` share one buffer, so keep a clone and call `flush` on it.

`flush` sends once. The layer queues events from spawned Tokio tasks, so spans that closed just before `flush` may miss it, and events requeued for retry are lost.

## Reported data

Each span that passes the layer's filter becomes an observation. Its name is the `otel.name` field when the span sets it at creation, such as `tool.git_diff` or `hook.BeforeTool`, and the span name otherwise. Agent steps record `otel.name` later, so they appear as `step`, with `step-N` in the metadata. Fields that start with `langfuse.` fill Langfuse's type, model, input, output, usage and status message; the rest go into metadata. Events add their fields to their span's observation; events outside a span are ignored.

### Agents

`agent.query` and `agent.query_once` get the rendered query as input. The [stop reason](/agents/stopping-and-structured-results/), as JavaScript Object Notation (JSON), becomes the output of the span current at the stop: `agent.query` or `agent.run` when the model has nothing more to say, a `step` observation when a tool such as `stop` ends the run.

### Tool calls

Each tool call is a `tool.<name>` observation with the arguments as input. The output is recorded after the `AfterTool` [hooks](/agents/hooks/) ran, so a hook that shortens it also shortens what Langfuse stores.

### Model calls

The OpenAI-compatible clients (`OpenAI`, `Groq`, `Gemini`, `Ollama`, `OpenRouter`, `Dashscope`) and `AwsBedrock` mark prompts, completions, structured prompts and streams as GENERATION observations with model, request, response and token usage. Each needs its own feature flag: `openai`, `groq`, `gemini`, `ollama`, `open-router`, `dashscope` or `aws-bedrock`. A failed `AwsBedrock` call records the error as the status message.

OpenAI-compatible embedding calls open no span. They set model, usage, and an empty input and output on the span they run in, such as `transformers.embed`. These replace any input and output the span had.

`Anthropic` records no Langfuse fields, so its calls appear as plain spans.

### Pipelines

Indexing pipeline step spans are TRACE, so a DEBUG filter drops them; transformer and query pipeline spans are INFO and appear. [Tracing](/observability/tracing/#spans) lists them.

## Minimal example

::snippet{src="@swiftide/examples/langfuse.rs" anchor="setup" title="examples/langfuse.rs"}

::snippet{src="@swiftide/examples/langfuse.rs" anchor="prompt" title="examples/langfuse.rs"}

Here one `EnvFilter` covers both layers, and without `RUST_LOG` nothing reaches Langfuse:

```sh
RUST_LOG=swiftide=debug,langfuse=debug cargo run --example langfuse
```

`RUST_LOG=debug` would also send the HTTP client's spans. The GENERATION observation sits under `prompt_openai`. The example never calls `flush`; add it when events do not arrive.

## Custom configuration

::snippet{src="@snippets/src/observability/langfuse.rs" anchor="from_config"}

`LangfuseLayer::from_config` takes a `Configuration` instead of the environment. It starts its own batch sender. Omitted fields come from `Configuration::default()`, whose base path is `http://localhost`.

`LangfuseLayer::from_batch_manager` builds a layer on a `LangfuseBatchManager` you started with `spawn`. Layers on one manager share its sender; each has its own trace.

## Gotchas

- **One trace per layer.** The first span creates a trace named with the current Unix timestamp, and every later span joins it. A long-running server puts all requests in one trace; wrap each request in a span of your own to group its observations; they still share the one trace.
- **Retries.** `LanguageModelWithBackOff` calls the inner client again per attempt, so each attempt is its own GENERATION observation.
- **Rejected events.** When Langfuse rejects single events in a batch with 429 or 5xx, they are sent at most three times. Other rejected events are dropped with the WARN event "Dropping Langfuse events that will not be retried". When the whole request fails (wrong keys, network error, Langfuse down, a batch over 3.5 MB), the whole batch is lost: the sender logs an ERROR, or `flush` returns the error.
- **Tokio.** `default`, `from_config` and every new span spawn Tokio tasks, and panic outside a runtime.

## Reference

- [`LangfuseLayer`](docsrs:swiftide/langfuse/struct.LangfuseLayer.html)
- [`Configuration`](docsrs:swiftide/langfuse/struct.Configuration.html)
- [`LangfuseBatchManager`](docsrs:swiftide/langfuse/struct.LangfuseBatchManager.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)
- [`AwsBedrock`](docsrs:swiftide/integrations/aws_bedrock_v2/struct.AwsBedrock.html)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)
- [`Targets`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/targets/struct.Targets.html) and [`EnvFilter`](https://docs.rs/tracing-subscriber/latest/tracing_subscriber/filter/struct.EnvFilter.html)
- [Feature flags](/reference/feature-flags/)

## Next

[Metrics and pipeline statistics](/observability/metrics-and-statistics/)
