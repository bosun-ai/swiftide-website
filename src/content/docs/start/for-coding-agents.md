---
title: "Using Swiftide with coding agents"
description: "Give Claude Code, Cursor, Codex and other coding agents the Swiftide skill, Context7, llms.txt, per-page Markdown, rustdoc JSON and an AGENTS.md block pinned to the right version."
sidebar:
  order: 6
identLint:
  ignore: [SystemPrompt]
---

Your coding agent, such as Claude Code, Cursor or Codex, can write Swiftide code that compiles against {{swiftide_version}}.

Swiftide is pre-1.0, and minor releases break APIs. Release 0.33 changed system prompts, hook signatures, command output and the AWS Bedrock module. A model that learned an older release writes code that no longer compiles. Each source below gives the agent the current API, and the [AGENTS.md block](#agentsmd-block) at the end pins the rules in your project.

| Source | What the agent gets |
| --- | --- |
| Skill | Rules, gotchas and compiled snippets for 0.33 |
| Context7 | Search over the examples, the skill and the crate sources |
| llms.txt files | This site as plain text |
| Page Markdown | One page as Markdown |
| API docs | Exact signatures, as HTML or rustdoc JSON |
| Examples | Programs that compile |

## Release status

The skill, `context7.json` and the four showcase crates are in the swiftide commit these docs pin. They are not in the {{swiftide_version}} release and not yet on the swiftide `master` branch. :unreleased Until they reach `master`, the install command and the Context7 setup below find nothing, and you get the files from the [pinned commit](https://github.com/bosun-ai/swiftide/tree/e09a3e90d3eecdaffa08818a661efc3ecec08321) instead.

## Skill

The Swiftide skill is a set of instructions that the agent loads when the task needs it. Once it is on `master`, install it with the `skills` CLI from npm (needs Node.js):

```sh
npx skills add bosun-ai/swiftide
```

Until then, copy the [`skills/swiftide`](https://github.com/bosun-ai/swiftide/tree/e09a3e90d3eecdaffa08818a661efc3ecec08321/skills/swiftide) folder from the pinned commit into the skills directory of your agent.

`SKILL.md` targets 0.33. It covers feature flags, the rustls crypto provider, tools, hooks, structured stops, human approval, MCP (Model Context Protocol) toolboxes, tasks and pipelines, and ends with a checklist. Four reference files go deeper: `agents.md`, `tasks.md`, `pipelines.md` and `integrations.md`. Every snippet in them is copied from compiled code in the swiftide repository, and names its source file.

The agent picks the skill up when a Rust project depends on swiftide or one of its crates, when you ask for an agent, task graph or RAG (retrieval-augmented generation) pipeline in Rust, or when it fixes a missing feature flag or crypto provider.

## Context7

[Context7](https://context7.com) indexes the swiftide repository under the library ID `/bosun-ai/swiftide`. With the Context7 MCP server installed, ask the agent to look up Swiftide in Context7, or give it the ID.

Context7 indexes the default branch, which can hold APIs that are not released yet. Check what it returns against the version in your `Cargo.lock`.

The pinned commit adds a [`context7.json`](https://github.com/bosun-ai/swiftide/blob/e09a3e90d3eecdaffa08818a661efc3ecec08321/context7.json). :unreleased Once it is on `master`, it limits the index to `examples`, `skills` and the source folders of the crates, and adds ten rules that the AGENTS.md block below also covers. Until then, Context7 indexes the default branch without it.

## llms.txt files

The site publishes plain-text versions of itself for language models. `/llms.txt` states the version these docs describe, {{swiftide_version}}, and links the others.

| File | Contents |
| --- | --- |
| [/llms.txt](https://swiftide.rs/llms.txt) | An index with the pinned version and links to the files below |
| [/llms-full.txt](https://swiftide.rs/llms-full.txt) | Every page in one file |
| [/llms-small.txt](https://swiftide.rs/llms-small.txt) | The start pages and core guides, without recipes, showcases, integrations or reference |
| [/_llms-txt/agents.txt](https://swiftide.rs/_llms-txt/agents.txt) | Agents: tools, hooks and executors |
| [/_llms-txt/tasks.txt](https://swiftide.rs/_llms-txt/tasks.txt) | Typed task graphs |
| [/_llms-txt/indexing-and-querying.txt](https://swiftide.rs/_llms-txt/indexing-and-querying.txt) | Indexing and query pipelines |
| [/_llms-txt/integrations.txt](https://swiftide.rs/_llms-txt/integrations.txt) | Model providers, vector stores and other integrations |
| [/_llms-txt/showcases.txt](https://swiftide.rs/_llms-txt/showcases.txt) | The four showcase programs and their output |

The full and small files start with the home page, followed by the Start pages. None of the files include the changelog. A section file is the smallest useful input when the agent works on one part, such as a task graph.

## Page Markdown

Every page is also served as Markdown at its URL plus `.md`, for example [https://swiftide.rs/start/installation.md](https://swiftide.rs/start/installation.md). Snippets are expanded into code, so the agent sees the same compiled Rust as you do.

The buttons next to each page title do the same by hand. **Copy page** copies the page as Markdown, for pasting into a chat. **View as Markdown** opens the `.md` version. **Open in ChatGPT** and **Open in Claude** start a conversation about the page.

## API docs

The [API docs on docs.rs](docsrs:swiftide/index.html) are the most reliable source of exact signatures. docs.rs builds swiftide with every feature flag on and labels each item with the flag it needs.

docs.rs also serves rustdoc JSON. The `swiftide` crate only re-exports the other crates, so its JSON holds re-export stubs and no signatures. Fetch the JSON of the crate that defines the items, such as `swiftide-agents`, `swiftide-tasks`, `swiftide-core`, `swiftide-indexing`, `swiftide-query` or `swiftide-integrations`. docs.rs sends it zstd-compressed, so decompress it before the agent reads it:

```sh
curl -sSL https://docs.rs/crate/swiftide-agents/{{swiftide_version}}/json | zstd -d > swiftide-agents.json
```

Pin the version in the URL so the agent does not read a newer release. `latest` in place of the version gives the newest release instead.

## Examples

The [examples directory](https://github.com/bosun-ai/swiftide/tree/master/examples) holds the single-file examples. The four [showcases](/showcases/release-notes-agent/) are in the [pinned commit](https://github.com/bosun-ai/swiftide/tree/e09a3e90d3eecdaffa08818a661efc3ecec08321/examples) only. :unreleased Each showcase has a README, an `OUTPUT.md` with output from a real run, and source that this site excerpts.

From the root of a checkout of the pinned commit, an agent can run them. `hello-agents` uses Gemini and needs `GEMINI_API_KEY`. `ask-your-codebase` needs `OPENAI_API_KEY`, and its first run downloads an embedding model of about 600 MB.

```sh
cargo run -p swiftide-examples --example hello-agents
cargo run -p ask-your-codebase -- index swiftide-agents
```

## AGENTS.md block

Paste this block into the `AGENTS.md` or `CLAUDE.md` at the root of your project. It comes from the skill's gotchas and the rules in `context7.json`. Change the version line when you upgrade.

```md
## Swiftide

This project uses swiftide {{swiftide_version}} (Rust 1.97 or newer, edition 2024). Swiftide is
pre-1.0 and minor releases break APIs. Check the version in Cargo.lock, and read
https://swiftide.rs/llms.txt or https://docs.rs/swiftide/{{swiftide_version}}/swiftide/ before
trusting examples written for older releases.

- Feature flags: default features are only `macros` (`#[swiftide::tool]`, `#[derive(Tool)]`).
  Agents need `swiftide-agents`, which also turns on `swiftide-tasks`. Each integration needs its
  own flag (`openai`, `anthropic`, `gemini`, `qdrant`, `fastembed`, `lancedb`, `duckdb`, `redb`,
  `mcp`, ...). An unresolved import under `swiftide::agents` or `swiftide::integrations` means a
  missing flag. `all` turns on only `qdrant`, `redis`, `tree-sitter`, `openai`, `fastembed`,
  `scraping`, `aws-bedrock`, `groq`, `ollama` and `pgvector`; everything else, including agents
  and tasks, needs its own flag.
- With `default-features = false`, add `macros` back. Tools and structured stops also need
  `schemars` (with `derive`) and `serde`.
- Install a rustls crypto provider at the start of `main`, before the first client, e.g.
  `rustls::crypto::aws_lc_rs::default_provider().install_default()`. That needs a direct
  dependency on `rustls` 0.23 with the `aws_lc_rs` and `std` features. Swiftide enables rustls
  without choosing a provider. Unless another dependency selects one, building a client panics.
- Tools: `#[swiftide::tool]` on an async fn that takes `&dyn AgentContext` and returns
  `Result<ToolOutput, ToolError>`. The doc comment is the tool description and a `# Arguments`
  list describes the arguments. Use `#[derive(Tool)]` for a tool with state. Return
  `ToolOutput::fail(..)` for errors the model should see and recover from.
- Run commands with `context.executor().exec_cmd(&Command::shell(..))`; `AgentContext::exec_cmd`
  is deprecated. `CommandOutput` records stdout and stderr chunks: `stdout()` and `stderr()`
  iterate over the `Bytes` chunks of one stream, and `into_string_lossy()` merges both streams
  into one `String` in the order they arrived.
- The system prompt is a `Prompt`: `.system_prompt(..)` takes anything `Into<Prompt>`, such as a
  `&str`. `SystemPrompt` no longer exists. An agent without a system prompt sends none.
- Hooks are closures that return `Box::pin(async move { Ok(()) })`; clone captured values before
  the `async move` block. Most hooks get `&mut Agent`; `on_new_message` and `on_stream` get
  `&Agent`. A hook that returns an error is logged, not propagated.
- Structured output: call `without_default_stop_tool()` before `.tools(..)`, add
  `StopWithArgs::with_parameters_schema(schema_for!(T))`, and read
  `StopReason::RequestedByTool(_, Some(payload))`. `.tools(..)` replaces any earlier list.
- `StopReason::NoNewMessages` means the model answered without a tool call or `.limit(n)` was
  reached. `limit` counts completions per `query`.
- Human approval: wrap a tool in `ApprovalRequired::new(tool)`. On
  `StopReason::FeedbackRequired`, call
  `agent.context().feedback_received(&tool_call, &ToolFeedback::approved())`, then `agent.run()`.
- Tasks: import from `swiftide::tasks`; `swiftide::agents::tasks` is deprecated. Use `Task`,
  `TaskNode` and `Transition` instead of chaining agents by hand. Give every node one transition
  and end with `task.transitions_to_finish()`. A fan-out
  (`Transition::fan_out(..).and(..).join_with(join.join())`) ends in a join node. `task.run(..)`
  returns `TaskRunOutcome::Completed(output)` or `TaskRunOutcome::Paused`; continue a paused
  task with `task.resume()`.
- OpenAI-compatible endpoints go through the `openai` flag:
  `OpenAI::builder().client(async_openai::Client::with_config(OpenAIConfig::new().with_api_base(..).with_api_key(..)))`.
  Swiftide does not re-export `async_openai`; add `async-openai` 0.41, the version swiftide uses.
- AWS Bedrock is `swiftide::integrations::aws_bedrock_v2` (the Converse API), behind the
  `aws-bedrock` flag.
- MCP tools are named `<server>:<tool>`. OpenAI rejects `:` in tool names, so rename them.
- Pipelines: use the same embedding model for indexing and querying, and set the vector store's
  `vector_size` to that model's dimensions. Wrap LLM clients used during indexing in
  `LanguageModelWithBackOff`.
- Before finishing: run `cargo check` with the exact features, and confirm `main` installs the
  crypto provider before the first request.
```

[Installation](/start/installation/) shows the ring equivalent of the crypto provider line.

## Verification

Ask the agent to finish with the checks from the skill. `cargo check` with the exact feature flags in `Cargo.toml` catches a missing flag as an unresolved import (error E0432). Running the binary once catches a missing crypto provider, which panics when the first client is built.

## Reference

- [`swiftide` crate root](docsrs:swiftide/index.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html) and [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`ToolOutput`](docsrs:swiftide/chat_completion/enum.ToolOutput.html) and [`CommandOutput`](docsrs:swiftide/traits/struct.CommandOutput.html)
- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)
- [`ApprovalRequired`](docsrs:swiftide/agents/tools/control/struct.ApprovalRequired.html) and [`StopWithArgs`](docsrs:swiftide/agents/tools/control/struct.StopWithArgs.html)
- [`Task`](docsrs:swiftide/tasks/struct.Task.html), [`Transition`](docsrs:swiftide/tasks/struct.Transition.html) and [`TaskRunOutcome`](docsrs:swiftide/tasks/enum.TaskRunOutcome.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html) and [`OpenAIConfig`](docsrs:swiftide/integrations/openai/struct.OpenAIConfig.html)
- [`aws_bedrock_v2`](docsrs:swiftide/integrations/aws_bedrock_v2/index.html)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)
- [`skills/swiftide/SKILL.md`](https://github.com/bosun-ai/swiftide/blob/e09a3e90d3eecdaffa08818a661efc3ecec08321/skills/swiftide/SKILL.md) at the pinned commit
- [Feature flags](/reference/feature-flags/)
- [Upgrading](/reference/upgrading/)
- [Crypto provider panic](/reference/troubleshooting-and-faq/#crypto-provider-panic)

## Next

[Agents overview](/agents/overview/)
