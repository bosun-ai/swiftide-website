---
title: "Troubleshooting and FAQ"
description: "Fixes for the errors people hit most with Swiftide: pipeline failures, the rustls panic, missing features, unreachable stores, rejected tool schemas, runaway agents and silent streaming."
sidebar:
  order: 5
identLint:
  ignore: [HashMap, aws_lc_rs, "rustls::crypto::aws_lc_rs::default_provider"]
---

Each heading on this page names a symptom. Find yours, apply the fix, and your code runs again. Answers to frequently asked questions (FAQ) close the page.

## Pipeline errors

::snippet{src="@snippets/src/reference/troubleshooting.rs" anchor="pipeline_errors"}

An indexing pipeline stops at the first node that fails, and `run` returns that error.

`log_errors` logs each error at `ERROR` level and passes it on. `filter_errors` then drops the errors, so the other nodes still reach storage. Keep `log_errors` first: `filter_errors` on its own drops errors without a trace. Both only see errors from the steps before them, so place them after the steps you want to cover. A storage error still stops the pipeline: Qdrant rejects a node without vectors, so embed first.

`log_all` also logs every node at `DEBUG` level. `filter` keeps each `Result` its closure returns `true` for, so you can skip one kind of error. The logs need a `tracing` subscriber; see [Tracing](/observability/tracing/).

## Crypto provider panic

Building a model client panics:

```text
No rustls crypto provider is configured. When using the `rustls-no-provider` feature you must install a crypto provider before building a Client. For example: `rustls::crypto::aws_lc_rs::default_provider().install_default().unwrap();` See https://docs.rs/rustls/latest/rustls/#cryptography-providers for details.
```

Since 0.33, Swiftide's HTTP clients use rustls without a crypto provider. Install one at the start of `main`, before you build the first client:

::snippet{src="@snippets/src/reference/feature_flags.rs" anchor="crypto_provider"}

```sh
cargo add rustls@0.23 --no-default-features --features ring,std
```

To use aws-lc-rs instead, call `rustls::crypto::aws_lc_rs::default_provider()` and turn on the `aws_lc_rs` feature of `rustls` instead of `ring`. [Crypto provider](/reference/feature-flags/#crypto-provider) explains why Swiftide leaves the choice to you. The Anthropic client, behind the `anthropic` flag, panics with `No provider set` instead, and the same fix applies.

## Missing features

A `use` of `swiftide::agents` or `swiftide::integrations::openai` fails with error E0432, "unresolved import", when the feature flag behind it is off. Agents, tasks and each integration have their own feature flag:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,swiftide-agents
```

[Feature flags](/reference/feature-flags/) lists what each flag turns on. If `#[tool]` is missing after `default-features = false`, add the `macros` flag back; see [Defaults](/reference/feature-flags/#defaults).

## Unreachable stores

Each store needs its feature flag: `qdrant`, `pgvector` or `redis`.

- **Qdrant**: the client reads `QDRANT_URL` and `QDRANT_API_KEY`, and falls back to `http://localhost:6334`. That is Qdrant's gRPC (gRPC Remote Procedure Calls) port, not the 6333 HTTP port. Publish both ports:

  ```sh
  docker run -p 6333:6333 -p 6334:6334 qdrant/qdrant
  ```

- **PgVector**: setup runs `CREATE EXTENSION IF NOT EXISTS vector`. The server needs pgvector installed, and your database user needs permission to create the extension. The connection pool tries 3 times, 3 seconds apart, before it gives up.
- **Redis**: pass the full URL and a key prefix to `Redis::try_from_url`, for example `Redis::try_from_url("redis://localhost:6379", "swiftide")`. This call does not connect, so a Redis that is down fails at the first store.

Qdrant and PgVector set up before the first node, so if either is down, `run` fails before any node is processed.

## Rejected tool schemas

`ToolSpecBuilder::build` rejects schemas that break the strict rules providers share: maps such as `HashMap<String, T>`, open objects and `patternProperties`. The `#[tool]` macro unwraps that result, so a bad argument type panics when `tool_spec` is called.

The OpenAI client also rejects `oneOf` and array-valued `type`, with "tool schema must be OpenAI compatible". Enums whose variants carry data or doc comments produce `oneOf`.

Model arguments as structs with named fields and unit-only enums. `Option<T>` is fine. See [Tools](/agents/tools/).

## Agent loop limits

Agents need the `swiftide-agents` feature flag.

::snippet{src="@snippets/src/reference/troubleshooting.rs" anchor="on_stream"}

With the default context, an agent stops when the model answers without calling a tool. Three settings change when it stops:

- `with_stop_on_assistant(false)` on the context keeps the loop going until a tool stops it. The default `stop` tool does that. An agent has no loop limit by default, so if you remove that tool with `without_default_stop_tool`, register your own or set `limit`.
- `limit(10)` caps the loop at 10 iterations. At the cap Swiftide logs the warning "Agent loop limit reached" and stops with `StopReason::NoNewMessages`. A normal finish has the same stop reason, so watch for the warning.
- When the same tool call keeps failing, the agent sends the error back to the model up to `tool_retry_limit` times (default 3). The next failure stops it with `StopReason::ToolCallsOverLimit`, and `query` returns the error.

`stop_reason` separates a tool stop and `ToolCallsOverLimit` from the other cases. It does not separate the loop limit from a normal finish. See [Stopping and structured results](/agents/stopping-and-structured-results/).

## Silent streaming

An agent prints nothing until the answer is complete. Register `on_stream`, as in the agent snippet on this page. The hook switches the agent to streaming and runs for every chunk. `streaming(true)` on the builder streams without a hook.

In the hook, `delta.message_chunk` holds the new text. The OpenAI client also puts the full message so far in `response.message` on each chunk, because `stream_full` defaults to `true`. In 0.33.0 the final OpenAI chunk repeats the last text fragment, so the snippet prints it twice. On master the final chunk carries no new text. :unreleased See [Streaming](/agents/streaming/).

## SimilaritySingleEmbedding type annotations

::snippet{src="@snippets/src/reference/troubleshooting.rs" anchor="similarity_annotation"}

`SimilaritySingleEmbedding::default()` fails with error E0283, "type annotations needed". The search strategy is generic over its filter type, and Qdrant, PgVector and LanceDB implement the retriever for more than one. Rust does not use the default type parameter to pick an implementation. Name the filter type, as in the snippet, or start from `query::Pipeline::default()`.

## FAQ

### Swiftide and Rig

Swiftide ships agents, task graphs, and indexing and query pipelines in one crate family. See [What is Swiftide](/start/what-is-swiftide/).

### Python

Swiftide has no Python bindings. Wrap it in your own service to use it from Python.

### Versioning

Swiftide is pre-1.0. Breaking changes land in minor releases, such as 0.32 to 0.33. Read [Upgrading](/reference/upgrading/) and the [Changelog](/reference/changelog/) before you move to a new release.

### Production use

Swiftide is part of the [bosun.ai](https://bosun.ai) project, and Bosun runs it in production.

## Reference

- [`indexing::Pipeline`](docsrs:swiftide/indexing/struct.Pipeline.html)
- [`AgentBuilder`](docsrs:swiftide/agents/struct.AgentBuilder.html)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`DefaultContext`](docsrs:swiftide/agents/struct.DefaultContext.html)
- [`SimilaritySingleEmbedding`](docsrs:swiftide/query/search_strategies/struct.SimilaritySingleEmbedding.html)
- [`ToolSpec`](docsrs:swiftide/chat_completion/struct.ToolSpec.html)
- [`ToolSpecBuilder`](docsrs:swiftide/chat_completion/struct.ToolSpecBuilder.html)
- [`Qdrant`](docsrs:swiftide/integrations/qdrant/struct.Qdrant.html)
- [`PgVector`](docsrs:swiftide/integrations/pgvector/struct.PgVector.html)
- [`Redis`](docsrs:swiftide/integrations/redis/struct.Redis.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/struct.OpenAI.html)

Next: [Glossary](/reference/glossary/)
