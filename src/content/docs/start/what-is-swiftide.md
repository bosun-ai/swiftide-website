---
title: "What is Swiftide"
description: "Swiftide is a Rust library with three building blocks, agents, tasks, and indexing and query pipelines, and this page shows each one in a real program and when to pick it."
sidebar:
  order: 1
---

Swiftide is a Rust library for building LLM (large language model) applications. It has three building blocks: agents for tool loops, tasks for typed hand-offs, and pipelines for data flows. This page shows each block in a real program, so you know which one to start with.

## Building blocks

| Building block | Use it for | Entry point | Feature flag |
| --- | --- | --- | --- |
| Agents | A model that calls tools in a loop until it stops | `swiftide::agents::Agent::builder()` | `swiftide-agents` |
| Tasks | A typed graph of steps with fan-out, join and pause | `swiftide::tasks::Task::new()` | `swiftide-tasks`, also turned on by `swiftide-agents` |
| Pipelines | Streaming data into storage, and answering questions from it | `swiftide::indexing::Pipeline`, `swiftide::query::Pipeline` | always compiled; each integration has its own flag |

The three blocks share one set of core traits, such as `SimplePrompt`, `ChatCompletion`, `EmbeddingModel` and `Tool`. The `OpenAI` client implements the first three, so one client can serve all three blocks. It needs the `openai` feature flag.

### Agents

![Agent loop](/diagrams/diagram-agent-loop.svg)

The release-notes-agent showcase reads the git history since the last tag and drafts release notes. This function builds its agent:

::snippet{src="@swiftide/examples/release-notes-agent/src/agent.rs" anchor="agent" title="examples/release-notes-agent/src/agent.rs"}

The agent sends its message history to the model, calls the tools the model asks for, and adds their results to the history. The tools run their commands through the tool executor in the context, here a `DefaultContext` built from the repository's executor. `progress::report_tool_calls` prints each tool call from `before_tool` and `after_tool` hooks. `progress::stream_draft` registers an `on_stream` hook, which switches the agent to streaming completions. `add_toolbox` adds optional GitHub tools from an MCP (Model Context Protocol) server, which needs the `mcp` feature flag.

The loop ends when a tool asks the agent to stop, when a tool needs human feedback (here `ApprovalRequired` around `WriteChangelog`), when the model replies without calling a tool, or when it reaches the step limit. Each case sets a stop reason. Reaching the limit is not an error, so check the stop reason after `query` returns. This agent swaps the default `stop` tool for its own, which requires the full release notes as structured arguments. `limit(STEP_LIMIT)` caps the completions per query at 40. [Your first agent](/start/your-first-agent/) builds the first step of this agent with one tool and one hook. Read more in [Agents overview](/agents/overview/) and [Release notes agent](/showcases/release-notes-agent/).

### Tasks

![Task graph](/diagrams/diagram-task-graph.svg)

The workspace-architect showcase drafts an `ARCHITECTURE.md` for a Cargo workspace. Its task first discovers the member crates, then fans out to one analyst per crate:

::snippet{src="@swiftide/examples/workspace-architect/src/graph.rs" anchor="fan_out" title="examples/workspace-architect/src/graph.rs"}

Every task node has an input type and an output type. The compiler checks that each transition hands the next node the input it expects. The transition after `discover` starts one branch per crate on the same `analyse` node and runs the branches with `ConcurrencyModel::Parallel`. `join_with` names the join node that collects the results. The `analyse` node still needs its own transition into the join, `task.register_transition(analyse, join.join())`, or the task fails validation with a missing-transition error. An empty workspace ends the task with `Transition::error`.

A run on the swiftide workspace with claude-sonnet-5-5, through an OpenAI-compatible gateway, prints this (trimmed):

```text
[   0.0s] analyse  started  swiftide-core
[   0.0s] analyse  started  swiftide-langfuse
...
[  40.5s] analyse  finished swiftide-core in 40.5s (15 tool calls)
[  40.5s] join     10 summaries; 230s of agent work in 41s wall-clock (5.7x)
```

[Your first task](/start/your-first-task/) builds a smaller task that starts the same way, with a `discover` node and no fan-out. Read more in [How tasks work](/tasks/overview/) and [Workspace architect](/showcases/workspace-architect/).

### Pipelines

![Indexing pipeline](/diagrams/diagram-pipeline.svg)

An indexing pipeline streams nodes from a loader through transformers, a chunker and an embedder into storage. A query pipeline turns a question into an answer from retrieved chunks. The ask-your-codebase showcase indexes a codebase into LanceDB and answers questions about it with this query pipeline:

::snippet{src="@swiftide/examples/ask-your-codebase/src/query.rs" anchor="query_pipeline" title="examples/ask-your-codebase/src/query.rs"}

Query transformers rewrite and embed the question, the retriever searches the LanceDB table with the search strategy, and the answer generator writes the answer from the retrieved chunks. [Your first pipeline](/start/your-first-pipeline/) runs this showcase and walks through each step, along with the indexing pipeline that fills the table. The showcase needs the `lancedb`, `fastembed`, `tree-sitter` and `openai` feature flags.

The docs-qa showcase adds hybrid search in Qdrant (the `qdrant` feature flag), reranking and evaluation. Read more in [Indexing overview](/indexing/overview/), [How querying works](/querying/overview/), [Ask your codebase](/showcases/ask-your-codebase/) and [Docs Q&A](/showcases/docs-qa/).

## Composition

The blocks nest. In workspace-architect, every fan-out branch runs an agent. `CrateAnalyst` implements `TaskNode` with a `WorkspaceCrate` in and a `CrateSummary` out. Each evaluation builds a fresh agent for one crate:

::snippet{src="@swiftide/examples/workspace-architect/src/analyst.rs" anchor="task_node" title="examples/workspace-architect/src/analyst.rs"}

Two built-in node types need no impl of your own. `Arc<dyn SimplePrompt>` takes a `Prompt` and returns a `String`; the showcase's render step is one. `TaskAgent` wraps an `Agent` as a node that takes a `Prompt` and returns `()`. To pass an answer to the next node, write your own `TaskNode`. `TaskAgent` also locks the agent, so branches that share one `TaskAgent` run one at a time. For parallel work, build one agent per branch as `CrateAnalyst` does.

A task node or a tool is plain async Rust, so its body can run an indexing pipeline or a query pipeline.

## Block selection

| You need | Use |
| --- | --- |
| A model that decides which tools to call and when to stop | an agent |
| Known steps with typed inputs and outputs, parallel branches, or a pause for human approval | a task |
| Documents or code loaded, chunked, embedded and stored, then searched to answer questions | an indexing pipeline and a query pipeline |
| Several agents in a fixed order, or one agent per item in parallel | a task with agent nodes |

Start with an agent when the model should choose the steps. Move to a task when you know the steps and want the compiler to check the hand-offs between them.

## Project status

Swiftide is pre-1.0. APIs change while the agent and task APIs settle, and breaking changes land in minor releases. Upgrade one minor version at a time with [Upgrading](/reference/upgrading/), and check the [Changelog](/reference/changelog/) for each release.

These docs describe swiftide {{swiftide_version}}. Some APIs are only on master, such as the `agents-contrib` feature flag :unreleased and the Perl tree-sitter grammar :unreleased. Pages mark them with that badge.

Swiftide is MIT licensed and part of the [bosun.ai](https://bosun.ai) project.

## Reference

- [`swiftide`](docsrs:swiftide/index.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`AgentBuilder`](docsrs:swiftide/agents/struct.AgentBuilder.html)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`DefaultContext`](docsrs:swiftide/agents/struct.DefaultContext.html)
- [`TaskAgent`](docsrs:swiftide/agents/struct.TaskAgent.html)
- [`Task`](docsrs:swiftide/tasks/struct.Task.html)
- [`TaskNode`](docsrs:swiftide/tasks/trait.TaskNode.html)
- [`Transition`](docsrs:swiftide/tasks/struct.Transition.html)
- [`ConcurrencyModel`](docsrs:swiftide/tasks/enum.ConcurrencyModel.html)
- [`indexing::Pipeline`](docsrs:swiftide/indexing/struct.Pipeline.html)
- [`query::Pipeline`](docsrs:swiftide/query/struct.Pipeline.html)
- [`SimilaritySingleEmbedding`](docsrs:swiftide/query/search_strategies/struct.SimilaritySingleEmbedding.html)
- [`GenerateSubquestions`](docsrs:swiftide/query/query_transformers/struct.GenerateSubquestions.html)
- [`Embed`](docsrs:swiftide/query/query_transformers/struct.Embed.html)
- [`answers::Simple`](docsrs:swiftide/query/answers/struct.Simple.html)
- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`EmbeddingModel`](docsrs:swiftide/traits/trait.EmbeddingModel.html)
- [`Tool`](docsrs:swiftide/traits/trait.Tool.html)
- [`ToolExecutor`](docsrs:swiftide/traits/trait.ToolExecutor.html)
- [Feature flags](/reference/feature-flags/)

## Next

[Installation](/start/installation/)
