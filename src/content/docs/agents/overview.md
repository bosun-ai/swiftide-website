---
title: "How agents work"
description: "Build a Swiftide agent, run its completion and tool-call loop, and read why it stopped."
sidebar:
  order: 1
identLint:
  ignore: [unnamed_agent]
---

After this page you can build an agent, run it, continue its conversation, and read why it stopped. The larger example is the [release-notes-agent showcase](/showcases/release-notes-agent/), which drafts release notes from git history. Agents live in `swiftide::agents`, behind the `swiftide-agents` feature flag:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,swiftide-agents
cargo add tokio --features full
cargo add anyhow
cargo add rustls@0.23 --no-default-features --features ring,std
```

## A minimal agent

::snippet{src="@snippets/src/agents/overview.rs" anchor="query_and_continue"}

Swiftide's HTTP (Hypertext Transfer Protocol) clients use rustls without a crypto provider. The snippet installs `ring` before it builds a client. [Feature flags](/reference/feature-flags/#crypto-provider) explains why. `OpenAI::builder()` reads `OPENAI_API_KEY` from the environment.

`llm` takes any `ChatCompletion` that is `Clone` and clones it into the agent. `build` fails without it. `system_prompt` takes anything that converts into a `Prompt`. The agent has no system prompt unless you set one. `limit` caps the completions in one run.

`query` adds a user message and loops until the agent stops. A stopped agent keeps its message history, so the second `query` continues the same conversation. `history` returns every message in the context.

## The agent loop

![Agent loop](/diagrams/diagram-agent-loop.svg)

The agent sends the message history and the spec of every tool to the model. When the completion holds tool calls, the agent runs the tools and records their outputs in the history. Then it asks the model again. Hooks run around every step.

The loop ends when a tool stops the agent, or when the step limit is reached. With `DefaultContext`, it also ends when the last message is an answer without a tool call. `DefaultContext::with_stop_on_assistant(false)` turns that off.

A tool that returns `ToolOutput::FeedbackRequired` stops the agent and leaves its tool call open, without an output. A call to a tool the agent does not have is skipped with a warning and gets no output either.

## State and stop reasons

`agent.state()` returns a `State`: `Pending` before the first run, `Running` during one, and `Stopped` with a `StopReason` after it. `agent.stop_reason()` tells you why it stopped:

- a tool returned `ToolOutput::Stop` (`RequestedByTool`, which holds that tool call)
- an answer without a tool call, the step limit, or the end of a `query_once` or `run_once` step (`NoNewMessages`)
- a pending approval (`FeedbackRequired`)
- a failure (`ToolCallsOverLimit`, `AgentFailed`, `Error`)

For `ToolCallsOverLimit` and `Error`, `query` also returns the error, so the `?` in the snippet returns before the `match`. `StopReason` is marked non-exhaustive, so a `match` on it needs a wildcard arm. [Stopping and structured results](/agents/stopping-and-structured-results/) covers each reason.

## Runs

`query_once` adds the message and runs one step. `run` and `run_once` do the same without a new message, which is how you resume after an approval. A run on an agent whose state is `Running` returns `AgentError::AlreadyRunning`.

Cloning an agent shares its hooks and context, so both clones write to the same message history. Each clone gets its own copy of the tools, so `add_tool` on one does not change the other. The clone copies the state and starts with an empty tool retry counter.

To run an agent as one step of a larger workflow, wrap it in a task node; see [Agents as task nodes](/tasks/agents-as-nodes/).

## A larger builder

The release-notes-agent showcase builds its agent in one function:

::snippet{src="@swiftide/examples/release-notes-agent/src/agent.rs" anchor="agent" title="examples/release-notes-agent/src/agent.rs"}

The showcase passes an `OpenAI` client pointed at any OpenAI-compatible endpoint, but the function accepts any provider.

`context` sets where the message history lives and which tool executor runs commands. `DefaultContext::from_executor` keeps the history in memory and uses the executor that `run` creates:

::snippet{src="@swiftide/examples/release-notes-agent/src/main.rs" anchor="executor" title="examples/release-notes-agent/src/main.rs"}

`Agent::builder()` starts with one tool, `stop`, which the model calls when it is done. The showcase replaces it with a `stop` tool whose arguments are the release notes, so it calls `without_default_stop_tool` before `tools`. The list mixes tools from `#[swiftide::tool]` functions, a tool wrapped in `ApprovalRequired`, and a `StopWithArgs` built from a JavaScript Object Notation (JSON) schema.

`add_toolbox` adds the GitHub tools from a Model Context Protocol (MCP) server when you pass `--mcp` in a repository hosted on GitHub. The MCP toolbox needs the `mcp` feature flag, which the showcase enables. The two `progress` functions take `&mut AgentBuilder` and register hooks, which keeps the builder short. The [hooks page](/agents/hooks/) shows a captured run of this agent with `claude-sonnet-5-5`.

### Agent names

`AgentBuilder::name` labels an agent, and `Agent::name()` reads the label back:

::snippet{src="@snippets/src/agents/overview.rs" anchor="named_agent"}

The name defaults to `unnamed_agent`. Swiftide only shows it in the agent's `Debug` output, so use it in your own logs or to tell agents apart in a multi-agent setup.

## Agent and application responsibilities

Besides running the loop, the agent:

- renders the system prompt on the first run and appends it to the history, after any messages that `BeforeAll` hooks or an existing history put there
- removes duplicate keys from the JSON arguments of a tool call
- wraps queries, steps, tools and hooks in tracing spans ([Tracing](/observability/tracing/))
- lists toolbox tools when it first starts

You provide the model, the tools, the system prompt, the tool executor, the hooks and, when you want a typed result, the schema of the stop tool.

## Gotchas

- A hook that returns an error is logged with tracing. The agent carries on.
- When a completion holds only calls to tools the agent does not have, no output is recorded. The next `query` returns `AgentError::UnfulfilledToolCalls`.
- Some errors return from `query` without setting a stop reason, for example a message history error or a tool task that fails to join. The agent then stays `Running`, and the next run returns `AgentError::AlreadyRunning`.
- A clone of a `Pending` agent renders the system prompt again on its first run and appends it to the shared history.

## Reference

- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`AgentBuilder`](docsrs:swiftide/agents/struct.AgentBuilder.html)
- [`DefaultContext`](docsrs:swiftide/agents/struct.DefaultContext.html)
- [`State`](docsrs:swiftide/agents/enum.State.html)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`AgentError`](docsrs:swiftide/agents/errors/enum.AgentError.html)
- [`StopWithArgs`](docsrs:swiftide/agents/tools/control/struct.StopWithArgs.html)
- [`ApprovalRequired`](docsrs:swiftide/agents/tools/control/struct.ApprovalRequired.html)
- [`LocalExecutor`](docsrs:swiftide/agents/tools/local_executor/struct.LocalExecutor.html)
- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)

## Next

[Tools](/agents/tools/)
