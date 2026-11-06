---
title: "Tracing"
description: "Log and trace Swiftide agents, tasks and pipelines with tracing-subscriber, read the logs of retried model calls, and export spans to OpenTelemetry."
sidebar:
  order: 1
identLint:
  ignore: [answer_question, total_nodes, responses_stream, then_store_with_batched, "Instrument::instrument", ".or_current()"]
---

Here you set up a subscriber that prints Swiftide's logs and spans, keeps failed model calls from showing up twice, and sends the spans to an OpenTelemetry (OTel) collector.

```sh
cargo add tracing
cargo add tracing-subscriber --features env-filter
```

::snippet{src="@snippets/src/observability/tracing.rs" anchor="subscriber"}

Swiftide emits spans and events through the `tracing` crate. Your app installs a subscriber once at startup to decide where they go. Without one, events fall back to the `log` facade.

`EnvFilter` reads the `RUST_LOG` environment variable. When `RUST_LOG` is unset, the fallback directive applies: `info` for everything, nothing from `swiftide_integrations`, and only errors from `async_openai`. [Retry errors](#retry-errors) explains the last two.

Agent, tool and hook spans need the `swiftide-agents` feature flag. Task spans need `swiftide-tasks`, which `swiftide-agents` turns on. See [Feature flags](/reference/feature-flags/).

## Filters per layer

A filter added with `with_filter` applies to one layer, as `logs` and `spans` do in [OpenTelemetry export](#opentelemetry-export). A filter added with `.with(filter)` on the registry applies to every layer. [Langfuse](/observability/langfuse/) uses the same split.

The showcases pick their own terminal defaults:

- release-notes-agent: `warn,swiftide_agents=off`. The command-line interface (CLI) already prints the stops, approvals and errors the agent logs.
- ask-your-codebase: `error,swiftide_integrations=off`.
- docs-qa: `warn,swiftide_query=error,async_openai=error,swiftide_integrations=off`.
- workspace-architect: `off`. It reads its own variable, `WORKSPACE_ARCHITECT_LOG`, instead of `RUST_LOG`.

## Spans

Each span's target is its module path, such as `swiftide_agents::agent`. OTel tools show `otel.name` instead of the span name where it is set.

### Agents

| Span | Level | Fields and `otel.name` |
| --- | --- | --- |
| `agent.query`, `agent.query_once`, `agent.run`, `agent.run_once` | INFO | A returned error is logged at ERROR |
| `step` | INFO | `step_count`; `otel.name` is `step-0`, `step-1`, ... |
| `add_message` | INFO | `message`: the full text of the message |
| `tool` | INFO | `otel.name` is `tool.<tool name>` |
| `hook` | INFO, TRACE for `OnStream` | `otel.name` is `hook.<hook type>`, such as `hook.BeforeTool` |
| `exec_cmd_streaming` (`LocalExecutor`) | INFO | `cmd` |
| `complete`, `prompt`, `complete_stream` | INFO | Model calls; OpenAI-compatible and AWS Bedrock streams add a `stream` span, the OpenAI Responses API a `responses_stream` span |

Only registered [hooks](/agents/hooks/) get a `hook` span. The tools of one step run in parallel.

### Tasks

[`Task::run`](/tasks/overview/) opens `task.run` and `Task::resume` opens `task.resume`; both log a returned error. Each task node runs in an INFO span `task.step` with the fields `node` (the node index) and `branch` (the branch id). An INFO event `task.step.done` follows each one.

### Indexing pipelines

On an [indexing pipeline](/indexing/overview/), `Pipeline::run` opens the INFO span `indexing_pipeline.run` and records `total_nodes` on it when the run succeeds. The steps `then`, `then_chunk`, `then_expand`, `then_store_with` and `filter_cached` wrap each node in a TRACE span. `then_in_batch` and `then_store_with_batched` wrap each batch. Their `otel.name` adds the step's name, such as `then_in_batch.Embed`. `split_by` opens one TRACE span for the whole split when you build the pipeline, so it sits outside `indexing_pipeline.run`. The built-in transformers have their own INFO spans, such as `transformers.embed` and `transformers.metadata_qa_text`.

### Query pipelines

On a [query pipeline](/querying/overview/), `query`, `query_mut` and `query_all` open `query_pipeline.query`, `query_pipeline.query_mut` and `query_pipeline.query_all`. Each step runs in an INFO span `then_transform_query`, `then_retrieve`, `then_transform_response` or `then_answer`. Each records the whole query as the field `query`.

## Trace of one agent run

The release-notes-agent streams its replies and registers `before_tool` and `after_tool` hooks. Its first step produces this span tree (reconstructed from the source, not a capture):

```text
agent.query
├── step                    otel.name = step-0
│   ├── complete_stream
│   │   └── stream
│   ├── add_message         message = the reply and its tool calls
│   ├── hook                otel.name = hook.BeforeTool
│   ├── tool                otel.name = tool.git_log_since
│   │   └── exec_cmd_streaming   cmd = the git command
│   ├── hook                otel.name = hook.AfterTool
│   └── add_message         message = the tool output
├── step                    otel.name = step-1
└── ...
```

![Agent loop](/diagrams/diagram-agent-loop.svg)

Each turn of the [agent loop](/agents/overview/) is one `step` span. The counter starts again at `step-0` on every `query` or `run` call. Tool calls left pending after an approval or feedback stop run before the loop, so their `tool` and `hook` spans sit directly under `agent.query` or `agent.run`. With `--responses-api`, `responses_stream` replaces `stream`.

## Parent spans

Run a request inside your own span to group its work:

::snippet{src="@snippets/src/observability/tracing.rs" anchor="parent_span"}

Every Swiftide span the query opens becomes a child of `answer_question`. The same works for `Pipeline::run` and `Task::run`.

## Retry errors

Retries are opt-in: you wrap the client in `LanguageModelWithBackOff` yourself. Each failed attempt it retries logs a WARN event, `Language model request failed, retrying`, with the attempt number, the wait and the error with its causes. When it gives up, it logs the final error once at ERROR, as `Language model request failed, giving up`. Both events have the target `swiftide_core::indexing_decorators`.

The OpenAI-compatible, `Anthropic` and `AwsBedrock` clients instrument `prompt`, `complete` and, where they have it, `structured_prompt` with `err(level = "warn")`. So every failed call also logs its error at WARN under `swiftide_integrations`, with or without retries. `async_openai`, the HTTP client underneath, logs server errors at WARN.

Hide the duplicates with the directives the showcases use:

- `swiftide_integrations=off` drops the clients' per-call warnings.
- `async_openai=error` drops the client's warnings.
- `swiftide_agents=off` drops every agent event: returned errors, failed hooks, tool retries and `Agent loop limit reached`.

`off` removes a target's spans too. Put these directives on the terminal layer with `with_filter`, as the OpenTelemetry snippet does. Otherwise the exporter loses the model-call spans and, with `swiftide_agents=off`, every agent span.

The final error still reaches your code, and the ERROR from `LanguageModelWithBackOff` stays visible. [Errors, backoff, usage and tokens](/models/errors-backoff-and-usage/) covers when a call is retried.

## Noisy events

- The query pipeline logs `Sending query` and `Answered query in N seconds` at WARN; `query_all` logs `Sending queries` and `Answered all queries in N seconds`. `swiftide_query=error` hides them.
- `add_message` and the `then_*` query spans carry whole messages and queries as fields. An exporter sends them with every span.
- A successful indexing run ends with an INFO event `Pipeline completed`; [Metrics and statistics](/observability/metrics-and-statistics/) lists its fields.
- A failed agent step logs its error at ERROR three times: in `step`, as `Agent stopped with error`, and in `agent.query`.

## OpenTelemetry export

Swiftide has no OTel dependency. To export spans, add a `tracing-opentelemetry` layer and an OpenTelemetry Protocol (OTLP) exporter:

```sh
cargo add opentelemetry@0.32 opentelemetry_sdk@0.32 tracing-opentelemetry@0.33
cargo add opentelemetry-otlp@0.32 --no-default-features --features trace,http-proto,reqwest-blocking-client
```

::snippet{src="@snippets/src/observability/tracing.rs" anchor="otlp"}

The exporter posts spans to `http://localhost:4318/v1/traces`. `OTEL_EXPORTER_OTLP_TRACES_ENDPOINT` overrides that URL as is. `OTEL_EXPORTER_OTLP_ENDPOINT` takes a base URL and gets `/v1/traces` appended. `tracing-opentelemetry` names each exported span after its `otel.name` field, so a step shows up as `step-0` and a tool call as `tool.git_log_since`. `provider.shutdown()` sends buffered spans before the process exits.

## Gotchas

- Indexing step spans are TRACE. Enable them only to follow single nodes or batches.
- Swiftide instruments indexing and query pipeline steps, tools and hooks with `.or_current()`. The agent `step` and `task.step` spans do not use it. When such a span is filtered out, the spans inside it attach to the enclosing span.
- A hook that returns an error logs `Error in <hook type> hook` at ERROR, and the agent carries on. `swiftide_agents=off` hides it.
- An `https` collector needs Transport Layer Security (TLS) in the exporter's HTTP client. Enable a TLS feature of `opentelemetry-otlp`, such as `reqwest-rustls`.

## Reference

- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`Hook`](docsrs:swiftide/agents/hooks/enum.Hook.html)
- [`LocalExecutor`](docsrs:swiftide/agents/tools/local_executor/struct.LocalExecutor.html)
- [`Task`](docsrs:swiftide/tasks/struct.Task.html)
- [`Pipeline` (indexing)](docsrs:swiftide/indexing/struct.Pipeline.html)
- [`Pipeline` (query)](docsrs:swiftide/query/struct.Pipeline.html)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)
- [tracing-subscriber](https://docs.rs/tracing-subscriber/0.3)
- [tracing-opentelemetry](https://docs.rs/tracing-opentelemetry/0.33)
- [opentelemetry-otlp](https://docs.rs/opentelemetry-otlp/0.32)
- [opentelemetry_sdk](https://docs.rs/opentelemetry_sdk/0.32)

## Next

[Langfuse](/observability/langfuse/)
