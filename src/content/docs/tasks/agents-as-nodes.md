---
title: "Agents as task nodes"
description: "Run an agent as a task node with TaskAgent, or implement TaskNode to turn its stop payload into typed output."
sidebar:
  order: 5
identLint:
  ignore: [add_changelog_entry]
---

You wrap an agent as a task node, first with the `TaskAgent` adapter and then as your own `TaskNode` that hands the next node a typed value read from the agent's stop payload. `TaskAgent` is the shortest form:

::snippet{src="@snippets/src/tasks/agents_as_nodes.rs" anchor="task_agent"}

`TaskAgent::from(agent)` turns the agent into a task node. Its input is a `Prompt`, its output is `()` and its error is `AgentError`. Each evaluation calls `query` on the agent with the prompt.

`TaskAgent` gives you no way to read the agent's answer. It fits agents whose work is a side effect, like the changelog entry that `add_changelog_entry` writes through the tool executor. When the next node needs data from the agent, write your own node.

The adapter keeps the agent behind an `Arc<tokio::sync::Mutex<Agent>>`. Branches that share one `TaskAgent` run one at a time and add to the same [message history](/agents/context-and-history/#message-history).

The `swiftide-agents` feature flag turns on `swiftide-tasks` as well:

```sh
cargo add swiftide@{{swiftide_version}} --features swiftide-agents,openai
```

The custom node below also needs serde and schemars 1 (the version swiftide re-exports as `swiftide::reexports::schemars`), plus `thiserror` for its error type:

```sh
cargo add serde --features derive
cargo add schemars@1 --features derive
cargo add serde_json thiserror
```

Swiftide selects no TLS (Transport Layer Security) crypto provider, so your app installs a rustls provider before it calls a model. [Troubleshooting and FAQ](/reference/troubleshooting-and-faq/#crypto-provider-panic) shows the call.

## Custom agent node

The workspace-architect showcase fans out one agent per publishable crate of a Cargo workspace, and each branch returns a `CrateSummary`. The node holds the model client, not an agent:

::snippet{src="@swiftide/examples/workspace-architect/src/analyst.rs" anchor="analyst" title="examples/workspace-architect/src/analyst.rs"}

The `TaskNode` implementation builds an agent, queries it and reads what the agent passed to `stop`:

::snippet{src="@swiftide/examples/workspace-architect/src/analyst.rs" anchor="task_node" title="examples/workspace-architect/src/analyst.rs"}

`evaluate` borrows its input and reaches the node through `&self`. Every branch of the fan-out evaluates the same node instance. That is why the agent is a local variable of `analyse` and not a field.

After `query` returns, `agent.stop_reason()` says why the agent stopped. A call to the stop tool with arguments gives `StopReason::RequestedByTool(_, Some(arguments))`. `CrateReport::deserialize` reads the report straight from the borrowed `serde_json::Value`. Any other stop reason, or a payload that does not match, becomes a `ReportProblem`.

The node then checks the report against `cargo metadata` and returns a `CrateSummary`. The transitions that follow are typed, so the compiler checks that the next node accepts it.

## Stop schema

The agent's stop tool requires a `CrateReport`:

::snippet{src="@swiftide/examples/workspace-architect/src/summary.rs" anchor="stop_schema" title="examples/workspace-architect/src/summary.rs"}

`summary_stop_tool` replaces the default `stop` tool with one whose arguments must match `CrateReport`. [Stopping and structured results](/agents/stopping-and-structured-results/) explains how the payload reaches `StopReason::RequestedByTool`.

## One agent per input

`agent_for` builds a fresh agent for every evaluation:

::snippet{src="@swiftide/examples/workspace-architect/src/analyst.rs" anchor="agent" title="examples/workspace-architect/src/analyst.rs"}

Each agent gets a [`LocalExecutor`](/agents/executors/#local-executor) whose working directory is its crate's directory, so relative paths in its tools resolve inside that crate. This is not a sandbox. Absolute paths and `..` still reach the rest of the file system. The showcase's own tools reject absolute paths and `..`, but they follow symlinks.

`without_default_stop_tool()` takes effect in the next `.tools(...)` call, which builds the tool set without the default `stop`. `summary_stop_tool()` then adds the one bound to `CrateReport`. The order matters, as the gotchas below explain.

The `before_tool` [hook](/agents/hooks/) reports progress. `limit(MAX_TURNS)` caps the completions of a single query at 30.

The branches share no agent, executor or message history. That makes it safe to run them under `ConcurrencyModel::Parallel`, as [Fan-out and join](/tasks/fan-out-and-join/#concurrency-model) shows.

## Corrections

A model sometimes answers in prose or nests its report. The node then queries the same agent again with a correction, up to `MAX_CORRECTIONS` (2) times. For a payload that does not match the schema, the correction quotes the serde error. For a missing stop, it is a fixed reminder to call `stop` now. The agent keeps its message history between the queries, so it sees its own mistake. [Workspace architect](/showcases/workspace-architect/#output) shows the captured retries.

## Shared agent

`examples/tasks.rs` keeps one agent for the life of the node instead:

::snippet{src="@swiftide/examples/tasks.rs" anchor="node" collapse="32-68"}

`BriefingAgent` locks the agent, calls `query_once` and deserializes the stop payload into a `BriefingDecision`. `query_once` runs one turn of the agent loop instead of looping until the agent stops. If the model calls another tool in that turn, the agent stops with `StopReason::NoNewMessages` and the node returns `MissingStructuredStop`. Use `query` for an agent that needs tools before it stops.

The mutex serializes evaluations, and the message history carries over from one evaluation to the next. Use this shape when the node runs once per task, or when later evaluations should see earlier ones.

## Gotchas

- Every call to `query` starts the `limit` count from zero. With corrections, one crate in the showcase can use up to `(1 + MAX_CORRECTIONS) * MAX_TURNS` completions.
- An agent that reaches its limit stops with `StopReason::NoNewMessages`, not with a stop payload. The payload itself is an `Option<Value>`, so handle `None` too.
- Call `without_default_stop_tool()` before `.tools(...)`. Called after it, or without any `.tools(...)` call, the default `stop` stays. Tools are unique by name, so your own `stop` passed later is dropped and the agent stops with `RequestedByTool(_, None)`.
- `build()` returns an `AgentBuilderError`, and `query` returns an `AgentError`. Wrap both in one error type that names the input. `AnalystError` prints "analysing `{crate_name}` failed: ...", so a failed branch says which crate it was working on.

## Reference

- [`TaskAgent`](docsrs:swiftide/agents/struct.TaskAgent.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html), [`Agent::query_once`](docsrs:swiftide/agents/struct.Agent.html#method.query_once) and [`AgentBuilder`](docsrs:swiftide/agents/struct.AgentBuilder.html)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`StopWithArgs`](docsrs:swiftide/agents/tools/control/struct.StopWithArgs.html)
- [`LocalExecutor`](docsrs:swiftide/agents/tools/local_executor/struct.LocalExecutor.html) and [`DefaultContext`](docsrs:swiftide/agents/struct.DefaultContext.html)
- [`ToolExecutor`](docsrs:swiftide/traits/trait.ToolExecutor.html)
- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)
- [`AgentError`](docsrs:swiftide/agents/errors/enum.AgentError.html)
- [`Task`](docsrs:swiftide/tasks/struct.Task.html) and [`TaskRunOutcome`](docsrs:swiftide/tasks/enum.TaskRunOutcome.html)
- [`TaskNode`](docsrs:swiftide/tasks/trait.TaskNode.html) and [`DynNodeId`](docsrs:swiftide/tasks/type.DynNodeId.html)
- [`ConcurrencyModel`](docsrs:swiftide/tasks/enum.ConcurrencyModel.html)

## Next

[Built-in nodes](/tasks/built-in-nodes/)
