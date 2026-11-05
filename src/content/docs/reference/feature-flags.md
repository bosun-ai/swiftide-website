---
title: "Feature flags"
description: "Every Cargo feature flag of the swiftide crate, what each one turns on, and which ones the all bundle leaves out."
sidebar:
  order: 1
identLint:
  ignore: [JsonSchema]
---

By the end of this page you know which feature flags to pass to `cargo add` for your project. Each integration, and everything beyond the indexing pipeline and the query pipeline, sits behind a feature flag of the `swiftide` crate. You name the ones you need:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,qdrant
```

## Defaults

Only the `macros` flag is on by default. It provides the `#[tool]` attribute, the `Tool` derive and the `#[indexing_transformer]` attribute. With `default-features = false` these macros are gone, so add `macros` back if you use them:

```sh
cargo add swiftide@{{swiftide_version}} --no-default-features --features macros,openai
```

On docs.rs, each item that needs a flag shows an "Available on crate feature ... only" badge.

The core, indexing, query and integrations crates are always compiled. The integrations crate still compiles no provider or store until you turn on its flag. Without `qdrant`, for example, `swiftide::integrations::qdrant` does not exist.

## The all bundle

The `all` flag turns on exactly these ten flags: `qdrant`, `redis`, `tree-sitter`, `openai`, `fastembed`, `scraping`, `aws-bedrock`, `groq`, `ollama` and `pgvector`.

It leaves out everything else:

- agents and tasks: `swiftide-agents`, `swiftide-tasks`, `mcp`, and `agents-contrib` :unreleased with its four sub-flags
- the `anthropic` (Anthropic client), `gemini`, `dashscope` and `open-router` providers
- the `lancedb` and `duckdb` storage backends, the `redb` node cache, and the `parquet`, `kafka` and `fluvio` loaders
- `langfuse`, `metrics`, `json-schema`, `tiktoken`, `test-utils` and `tree-sitter-perl` :unreleased

To build an agent, add `swiftide-agents` next to `all`.

`all` also pulls in heavy native dependencies. `fastembed` downloads ONNX (Open Neural Network Exchange) runtime binaries at build time, and `aws-bedrock` compiles the AWS (Amazon Web Services) SDK (software development kit). Name the flags you use instead, so builds stay short.

## Flag relationships

### Agents, tasks and tool protocols

- `swiftide-tasks` adds `swiftide::tasks`, the typed task graphs.
- `swiftide-agents` adds `swiftide::agents` and turns on `swiftide-tasks`.
- `mcp` adds Model Context Protocol (MCP) tool support and turns on `swiftide-agents`.
- `langfuse` adds `swiftide::langfuse` for tracing and also turns on `swiftide-agents`.
- `json-schema` adds `JsonSchema` derives to core and agent types, and also turns on `swiftide-agents`.
- `agents-contrib` :unreleased adds `swiftide::agents_contrib` and turns on `swiftide-agents`. On its own it enables no capability.
- `agents-contrib-context`, `agents-contrib-planning`, `agents-contrib-file-tools` and `agents-contrib-lazy-executor` :unreleased each turn on `agents-contrib` plus one capability: hooks that summarize tool output and compress the conversation, a plan tool, file tools, or a lazy tool executor.

A contrib flag never registers a tool or changes an agent by itself. You still add the tool or hook to your agent.

### OpenAI-compatible providers

The `groq`, `gemini`, `ollama`, `open-router` and `dashscope` clients build on the OpenAI client. Each of these flags also compiles `swiftide::integrations::openai`.

## TLS

Integrations that make HTTP (Hypertext Transfer Protocol) calls use rustls for TLS (Transport Layer Security) and need no setup, whatever flags you pick. Each HTTP client uses the process-wide rustls crypto provider if your application installed one, and otherwise its own: aws-lc-rs, or ring for `anthropic`. `anthropic` uses rustls even when another crate turns on native TLS.

Redis over `rediss://` is the exception. The Redis client needs a process-wide provider, so before it connects Swiftide installs aws-lc-rs if your application has not installed one. To use a different provider, install it before the first Redis call.

In the {{swiftide_version}} release the clients have no provider of their own; see [Crypto provider panic](/reference/troubleshooting-and-faq/#crypto-provider-panic).

## Flag tables

The "Also enables" column lists only flags of the `swiftide` crate. A flag that turns on a dependency's feature, such as `json-schema` or `langfuse` turning on `swiftide-agents`, shows an empty cell.

::feature-flags{level="3"}

## Reference

- [`swiftide` crate root, Cargo Features section](docsrs:swiftide/index.html#cargo-features)
- [`swiftide::agents`](docsrs:swiftide/agents/index.html)
- [`swiftide::tasks`](docsrs:swiftide/tasks/index.html)
- [`swiftide::integrations`](docsrs:swiftide/integrations/index.html)
- [`swiftide::langfuse`](docsrs:swiftide/langfuse/index.html)
- [`swiftide::agents_contrib`](docsrs:swiftide/agents_contrib/index.html) :unreleased

Next: [Crates](/reference/crates/)
