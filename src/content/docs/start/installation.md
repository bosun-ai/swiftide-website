---
title: "Installation"
description: "Add swiftide with the feature flags you need and tokio, set your API keys, and run a first program that prints one answer from a model."
sidebar:
  order: 2
---

You end with a Rust binary on swiftide {{swiftide_version}} that prints one answer from a model. The page also lists the environment variables, feature flags and local servers the rest of the site uses.

## Dependencies

Swiftide {{swiftide_version}} needs Rust 1.97 or newer, its minimum supported Rust version (MSRV). Create a binary crate and add three dependencies:

```sh
cargo new hello-swiftide
cd hello-swiftide
cargo add swiftide@{{swiftide_version}} --features swiftide-agents,openai
cargo add tokio --features macros,rt-multi-thread
cargo add anyhow
```

`openai` compiles the OpenAI client. `swiftide-agents` is for [Your first agent](/start/your-first-agent/); this page does not use it yet. tokio runs the async `main`, and anyhow carries its errors.

## First program

Replace `src/main.rs` with this program:

::snippet{src="@snippets/src/start/installation.rs" anchor="first_run" title="src/main.rs"}

`OpenAI::builder()` without a `client(..)` call reads `OPENAI_API_KEY` from the environment, and `OPENAI_BASE_URL` when it is set. `default_prompt_model` sets the model that `prompt` uses. `prompt` comes from the `SimplePrompt` trait, which the `use` line imports. It takes a `Prompt`, which `.into()` builds from a string.

### TLS

Swiftide's Hypertext Transfer Protocol (HTTP) clients use rustls for Transport Layer Security (TLS), with the HTTP client's own crypto provider, so the program needs no TLS setup. A provider that your application installs for the whole process takes precedence. [TLS](/reference/feature-flags/#tls) covers the one exception, Redis over `rediss://`.

## API keys

Export your application programming interface (API) key and run the program:

```sh
export OPENAI_API_KEY=sk-...
cargo run
```

Cargo builds the crate and the program prints the model's one-line answer.

These environment variables come up across the site and the showcases:

| Variable | Read by | Default |
| --- | --- | --- |
| `OPENAI_API_KEY` | the OpenAI client (`openai` flag) | none |
| `OPENAI_BASE_URL` | the OpenAI client (`openai` flag) | `https://api.openai.com/v1` |
| `OPENAI_MODEL` | the showcase command-line tools, not Swiftide | `gpt-5.4` or `gpt-5.4-mini` |
| `QDRANT_URL`, `QDRANT_API_KEY` | the Qdrant builder (`qdrant` flag) | `http://localhost:6334` |
| `GROQ_API_KEY` | the Groq client (`groq` flag) | none |
| `GEMINI_API_KEY` | the Gemini client (`gemini` flag) | none |
| `OPENROUTER_API_KEY` | the OpenRouter client (`open-router` flag) | none |
| `QWEN_API_KEY`, then `DASHSCOPE_API_KEY` | the DashScope client (`dashscope` flag) | none |
| `GITHUB_PERSONAL_ACCESS_TOKEN` or `GITHUB_TOKEN` | release-notes-agent with `--mcp` | none |
| `LANGFUSE_PUBLIC_KEY`, `LANGFUSE_SECRET_KEY` | the Langfuse tracing layer (`langfuse` flag) | required; the default layer panics without them |
| `LANGFUSE_URL` | the Langfuse tracing layer (`langfuse` flag) | `http://localhost:3000` |

The Langfuse layer also accepts `LANGFUSE_INIT_PROJECT_PUBLIC_KEY` and `LANGFUSE_INIT_PROJECT_SECRET_KEY` as fallbacks for the two keys.

You can point `OPENAI_BASE_URL` at an OpenAI-compatible endpoint, such as a local gateway. The endpoint must serve the model name you pass to `default_prompt_model`, here `gpt-5-mini`. Unset the variable rather than leave it empty: an empty value becomes an empty base URL. [OpenAI-compatible providers](/integrations/openai-compatible/) and [Models](/models/overview/) cover the other clients.

## Feature flags

Only `macros` is on by default. It provides the `#[tool]` attribute, `#[derive(Tool)]` and `#[indexing_transformer]`. Agents, tasks and each integration sit behind their own feature flag, and each flag adds a module:

- `openai` adds `swiftide::integrations::openai`.
- `qdrant` adds `swiftide::integrations::qdrant`.
- `swiftide-agents` adds `swiftide::agents` and turns on `swiftide-tasks`, which adds `swiftide::tasks`.

The showcases name only the flags they use:

| Showcase | Flags |
| --- | --- |
| [release-notes-agent](/showcases/release-notes-agent/) | `macros`, `swiftide-agents`, `openai`, `mcp`, `langfuse` |
| [ask-your-codebase](/showcases/ask-your-codebase/) | `tree-sitter`, `fastembed`, `lancedb`, `redb`, `openai` |
| [workspace-architect](/showcases/workspace-architect/) | `swiftide-agents`, `swiftide-tasks`, `openai` |
| [docs-qa](/showcases/docs-qa/) | `openai`, `fastembed`, `qdrant`, `langfuse` |

Three of them turn default features off. With `--no-default-features`, add `macros` back if you use any of its macros. The `all` flag is not a shortcut to everything: it turns on only `qdrant`, `redis`, `tree-sitter`, `openai`, `fastembed`, `scraping`, `aws-bedrock`, `groq`, `ollama` and `pgvector`. [Feature flags](/reference/feature-flags/) lists every flag and what it turns on.

## Local services

LanceDB, DuckDB, FastEmbed and Redb run inside your process, so the ask-your-codebase showcase needs no server. FastEmbed downloads its model on first use. Swiftide's FastEmbed default is the fastembed crate's default model; ask-your-codebase picks `jina-embeddings-v2-base-code` instead, which is about 600MB.

Qdrant, Redis and PgVector run as servers. Docker starts each one:

```sh
# Qdrant (qdrant flag): Swiftide talks gRPC Remote Procedure Call (gRPC) on 6334;
# 6333 is the HTTP API and dashboard.
docker run -d -p 6333:6333 -p 6334:6334 qdrant/qdrant

# Redis (redis flag): the examples read REDIS_URL and default to redis://localhost:6379.
docker run -d -p 6379:6379 redis:7-alpine

# PgVector (pgvector flag): pass the connection URL to the PgVector builder with db_url.
docker run -d -p 5432:5432 -e POSTGRES_PASSWORD=postgres pgvector/pgvector:pg17
```

## Common errors

- **The program panics with "No rustls crypto provider is configured"**: you are on 0.33.0 or earlier. See [Crypto provider panic](/reference/troubleshooting-and-faq/#crypto-provider-panic).
- **Error E0432, "unresolved import"**: the feature flag behind that module is off. Add it with `cargo add swiftide@{{swiftide_version}} --features <flag>`.
- **An authentication error (HTTP 401)**: `OPENAI_API_KEY` is missing or empty. The client builds without a key, so the error shows up at the first request.

[Troubleshooting and FAQ](/reference/troubleshooting-and-faq/) covers more.

## Reference

- [`swiftide` crate root, Cargo Features section](docsrs:swiftide/index.html#cargo-features)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)
- [`GenericOpenAIBuilder`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)

## Next

[Your first agent](/start/your-first-agent/)
