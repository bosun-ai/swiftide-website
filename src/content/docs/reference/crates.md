---
title: "Crates"
description: "The crates in the Swiftide workspace, what each one contains, and when to depend on one directly instead of the swiftide facade."
sidebar:
  order: 2
---

Swiftide is a Cargo workspace: a set of crates that the `swiftide` facade crate re-exports as modules. By the end of this page you know which crate holds which part of the API, which `swiftide::` path reaches it, and which docs.rs page to open.

::snippet{src="@snippets/src/reference/crates.rs" anchor="facade_paths"}

Your application depends on the `swiftide` crate, and you switch parts on with feature flags. This excerpt needs the `swiftide-agents` and `openai` flags:

```sh
cargo add swiftide@{{swiftide_version}} --features swiftide-agents,openai
```

## The facade

The `swiftide` crate has almost no code of its own. It maps the other crates onto these modules:

- `swiftide::indexing` joins the indexing types of swiftide-core (`Node`, `TextNode`, the indexing traits) with swiftide-indexing. With the `tree-sitter` flag, `swiftide::indexing::transformers` also holds the code transformers.
- `swiftide::query` joins the query types of swiftide-core (the query state, the search strategies) with swiftide-query.
- `swiftide::traits` collects the traits of swiftide-core in one place: `AgentContext`, `MessageHistory`, the chat completion traits, the indexing and query traits, `ToolExecutor` and the command types.
- `swiftide::prompt` and `swiftide::chat_completion` come straight from swiftide-core.
- `swiftide::integrations` is swiftide-integrations. Each provider or storage module inside it exists only with its feature flag.
- `swiftide::agents` needs the `swiftide-agents` flag, `swiftide::tasks` the `swiftide-tasks` flag, and `swiftide::langfuse` the `langfuse` flag.
- `swiftide::agents_contrib` needs the `agents-contrib` flag, and each of its `tools`, `hooks` and `executors` submodules needs a capability flag on top.
- The `#[tool]` attribute, the `Tool` derive and `#[indexing_transformer]` sit at the crate root, behind the `macros` flag.

The facade always depends on swiftide-core, swiftide-indexing, swiftide-query and swiftide-integrations. Agents, tasks, Langfuse and the macros are optional dependencies. The contrib crate is optional too. [Feature flags](/reference/feature-flags/) lists every flag and what it turns on.

## Workspace crates

| Crate | Contains | Facade path | API docs |
| --- | --- | --- | --- |
| swiftide-core | The traits the other crates implement, `Node` and `TextNode`, `Prompt`, chat completion types, `ToolExecutor`, `CommandOutput`, search strategies | `swiftide::traits`, `swiftide::prompt`, `swiftide::chat_completion`; `Node` and `TextNode` through `swiftide::indexing`; search strategies through `swiftide::query::search_strategies` | [swiftide_core](docsrs:swiftide_core) |
| swiftide-indexing | The indexing pipeline, loaders, transformers, chunkers, `MemoryStorage` | `swiftide::indexing` | [swiftide_indexing](docsrs:swiftide_indexing) |
| swiftide-query | The query pipeline, query transformers, response transformers, answer generators, evaluators | `swiftide::query` | [swiftide_query](docsrs:swiftide_query) |
| swiftide-integrations | One module per provider or storage, each behind its feature flag | `swiftide::integrations` | [swiftide_integrations](docsrs:swiftide_integrations) |
| swiftide-agents | `Agent`, `DefaultContext`, hooks, `StopReason`, `LocalExecutor`, Model Context Protocol (MCP) tools (with the `mcp` flag), `TaskAgent` | `swiftide::agents` | [swiftide_agents](docsrs:swiftide_agents) |
| swiftide-tasks | `Task`, `TaskNode`, `Transition`, fan-out and join | `swiftide::tasks` | [swiftide::tasks](docsrs:swiftide/tasks/index.html) |
| swiftide-langfuse | `LangfuseLayer`, a tracing layer that reports agent and completion calls to Langfuse | `swiftide::langfuse` | [swiftide_langfuse](docsrs:swiftide_langfuse) |
| swiftide-macros | `#[tool]`, the `Tool` derive, `#[indexing_transformer]` | crate root | [swiftide_macros](docsrs:swiftide_macros) |
| swiftide-agents-contrib | `ReadFile`, `ExactReplace`, `ApplyPatch`, `UpdatePlan`, `ContextCompression`, `ToolSummarizer`, `LazyExecutor` | `swiftide::agents_contrib` | [source on GitHub](https://github.com/bosun-ai/swiftide/tree/{{swiftide_sha}}/swiftide-agents-contrib) |

swiftide-tasks is new in 0.33. It depends on swiftide-core only. swiftide-agents depends on swiftide-tasks and swiftide-indexing, so the `swiftide-agents` flag also turns on `swiftide-tasks`. The old `swiftide::agents::tasks` module still compiles, but it is deprecated since 0.32.1 and only re-exports the new crate plus `TaskAgent`.

swiftide-agents-contrib enables nothing by default, and each capability has its own facade flag. [Feature flags](/reference/feature-flags/#agents-tasks-and-tool-protocols) lists them.

swiftide-test-utils also lives in the workspace. It is internal (`publish = false`) and not on crates.io.

## Direct dependencies

Applications depend on the facade. A direct dependency on a workspace crate pays off in two cases:

- You write a library that only implements Swiftide traits, such as a loader, a transformer, a tool or a tool executor. swiftide-core defines every one of those traits and keeps your dependency tree small.
- You write an integration for a provider or storage that Swiftide does not ship.

```sh
cargo add swiftide-core@{{swiftide_version}}
```

### Matching versions

All crates share one workspace version, {{swiftide_version}}, and each one requires its swiftide dependencies at 0.33. Keep every swiftide crate in your dependency tree on the same minor version. Cargo treats 0.32 and 0.33 as incompatible, so a mix builds two copies of swiftide-core, and a trait from one copy does not accept a type from the other.

### Macros and the facade

`#[tool]` and the `Tool` derive expand to paths in the `swiftide` crate, such as `swiftide::chat_completion::ToolSpec`. A crate that uses them needs the `swiftide` crate with its `macros` flag, even when the rest of its code imports from swiftide-core.

`#[indexing_transformer]` works the other way round. It expands to paths in `swiftide_core`, `anyhow` and `derive_builder`, not `swiftide`. A crate that uses it needs those three crates as direct dependencies, next to the facade:

```sh
cargo add swiftide-core@{{swiftide_version}} anyhow derive_builder
```

### Tasks without agents

The `swiftide-tasks` flag adds `swiftide::tasks` without compiling swiftide-agents:

```sh
cargo add swiftide@{{swiftide_version}} --features swiftide-tasks
```

The `mcp`, `langfuse` and `json-schema` flags and every `agents-contrib` flag turn on swiftide-agents. Enable one of them and agents compile again. [Flag relationships](/reference/feature-flags/#flag-relationships) shows which flag turns on which.

## Versioning

Only the `swiftide` crate gets a git tag (`v{{swiftide_version}}`) and a GitHub release, and its changelog covers every crate. Before you move to a new minor version, read [Upgrading](/reference/upgrading/).

## Reference

- [`swiftide`](docsrs:swiftide)
- [`swiftide_core`](docsrs:swiftide_core)
- [`swiftide_indexing`](docsrs:swiftide_indexing)
- [`swiftide_query`](docsrs:swiftide_query)
- [`swiftide_integrations`](docsrs:swiftide_integrations)
- [`swiftide_agents`](docsrs:swiftide_agents)
- [`swiftide::tasks`](docsrs:swiftide/tasks/index.html)
- [`swiftide_langfuse`](docsrs:swiftide_langfuse)
- [`swiftide_macros`](docsrs:swiftide_macros)
- [swiftide-agents-contrib source](https://github.com/bosun-ai/swiftide/tree/{{swiftide_sha}}/swiftide-agents-contrib)

Next: [Changelog](/reference/changelog/)
