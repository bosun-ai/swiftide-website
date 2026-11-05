---
title: "How tasks work"
description: "A task is a typed graph of nodes and transitions that runs functions, prompts and agents, with fan-out, join and pause."
sidebar:
  order: 1
---

You will be able to read a task graph, enable the `swiftide-tasks` feature flag, and decide whether a job needs a task or a single agent. A task runs a fixed graph of steps, and each step can be a function, a prompt or an agent. [Your first task](/start/your-first-task/) builds a two-node task step by step if you want a smaller start.

## Showcase graph

The workspace-architect showcase drafts an `ARCHITECTURE.md` for a Cargo workspace. This function builds its whole task:

::snippet{src="@swiftide/examples/workspace-architect/src/graph.rs" anchor="graph" title="examples/workspace-architect/src/graph.rs" collapse="15-17,26-27,40-49"}

The task has the type `Task<WorkspaceRoot, WrittenDocument>`. It takes a workspace root and returns the written document. Each `register_node*` call adds a node and returns its typed `NodeId`. Each `register_transition` call says where a node's output goes next.

- **discover** is a closure node. It runs `cargo metadata` and returns the member crates.
- **analyse** is a custom task node, `CrateAnalyst`. The fan-out sends every crate to it as its own branch. The fan-out sets `ConcurrencyModel::Parallel`, so the branches run at the same time. Each branch runs a fresh agent that reads one crate and returns a `CrateSummary`.
- **join** waits for every branch. It receives a `JoinInput<CrateSummary>` and builds the prompt for the writer.
- **render** is an `Arc<dyn SimplePrompt>` registered as a node. It sends the prompt to the model and returns Markdown.
- **review** is `ApprovalGate`. Until someone approves the draft, its transition returns `Transition::pause()` and the run stops.
- **write** is an async closure node. It saves the file, and its transition, `task.transitions_to_finish()`, ends the task.

In the captured run, ten analyst agents on `claude-sonnet-5-5` finished in the time of the slowest one. The join line from `OUTPUT.md`:

```text
[  40.5s] join     10 summaries; 230s of agent work in 41s wall-clock (5.7x)
```

Run it against the Swiftide workspace from the repository root:

```sh
export OPENAI_API_KEY=sk-...
cargo run -p workspace-architect
```

The command-line program starts the task with `task.run(WorkspaceRoot)`. That returns `TaskRunOutcome::Completed(WrittenDocument)` or `TaskRunOutcome::Paused`:

::snippet{src="@swiftide/examples/workspace-architect/src/main.rs" anchor="resume" title="examples/workspace-architect/src/main.rs"}

On a pause, the program asks you to approve the draft and then calls `resume()`.

## Task graph

![Task graph](/diagrams/diagram-task-graph.svg)

The diagram shows a smaller task with the same parts. A start node fans out to two prompt nodes that run in parallel. A join node collects both outputs, and the task finishes.

A task has these parts:

- A **task node** implements `TaskNode`. It declares an `Input`, an `Output` and an `Error` type, and its `evaluate` method turns a borrowed input into an output. [Nodes and transitions](/tasks/nodes-and-transitions/#task-nodes) covers the trait.
- A **transition** decides where a node's output goes next. It is a closure that takes the output by value, or a ready-made target such as `join.join()` or `task.transitions_to_finish()`. Each node has exactly one transition.
- The **start node** is set with `starts_with`. Its input type is the task's input type.
- A **fan-out** sends one input to each of several branches. Every fan-out names the join node that waits for all of them. By default the branches run one after another. Set `ConcurrencyModel::Parallel` on the fan-out or the task to run them at the same time.
- A **join** node receives the branch outputs as one `JoinInput<T>`. [Fan-out and join](/tasks/fan-out-and-join/) covers both.
- A **pause** stops the run, and `run` returns `TaskRunOutcome::Paused`. The paused branch keeps the node whose transition paused. `resume()` runs that node again with the same input. This is why the showcase pauses on the cheap review node and not on render. [Pause and resume](/tasks/pause-and-resume/) covers the details.

When a branch reaches the finishing transition, `run` or `resume()` returns `TaskRunOutcome::Completed` with the task's output. [Nodes and transitions](/tasks/nodes-and-transitions/#errors) covers the checks `run` makes before any node runs.

## Crate contents

The `swiftide-tasks` crate holds the graph and its runtime:

- `Task`, `TaskNode`, `NodeId` and `Transition`
- fan-out and join, with `JoinInput` and `AnyJoinInput`
- `ConcurrencyModel::Sequential` (the default) and `ConcurrencyModel::Parallel`
- pause, resume and reset, with `TaskRunOutcome`
- `TaskError` and `NodeError`
- closure nodes through `register_node_fn` and `register_node_async_fn`
- [built-in nodes](/tasks/built-in-nodes/) for `SimplePrompt`, `ChatCompletion` and `ToolExecutor` trait objects

The crate's own code spawns no tasks or threads. Parallel branches run concurrently inside the future that `run` returns. Swiftide itself still depends on tokio.

`TaskAgent`, which wraps an agent as a node, lives in swiftide-agents. Import it from `swiftide::agents::TaskAgent`.

## Feature flag

Turn on the `swiftide-tasks` feature flag:

```sh
cargo add swiftide@{{swiftide_version}} --features swiftide-tasks
```

The `swiftide-agents` flag turns it on as well. Import the graph types from `swiftide::tasks`. The old path `swiftide::agents::tasks` still works, but it is deprecated since 0.32.1. [Upgrading](/reference/upgrading/#tasks-crate) lists the 0.33 changes to tasks. A comment in the crate source says the tasks application programming interface (API) "is not stable yet" and "is subject to rapid change", so check Upgrading when you move to a new minor version.

## Tasks and agents

An [agent](/agents/overview/) runs a loop. After every model response it decides which tool to call next. It stops when a tool or the model asks it to, when it reaches its completion limit, or when it fails.

A task fixes the order of steps in code. Every node declares its input and output types. A transition to a node with the wrong input type does not compile, and neither does a fan-out without a join.

Control flow lives in Rust closures, not in model output. The model can still make decisions inside a node, but the graph decides what happens with the result. Each node does one job. You can test a closure node without a model, swap the model behind a prompt node, or give an agent node its own tools and limits.

Use a task when you know the steps in advance, when some of them need no model, or when you want to fan out work and collect it in one place. Put an agent inside a node for the open-ended part, such as reading one crate, and let the task handle the order, the parallel branches and the approval. [Agents as task nodes](/tasks/agents-as-nodes/) shows how. The [workspace-architect showcase](/showcases/workspace-architect/) walks through the whole program.

## Reference

- [`swiftide::tasks`](docsrs:swiftide/tasks/index.html)
- [`Task`](docsrs:swiftide/tasks/struct.Task.html)
- [`TaskNode`](docsrs:swiftide/tasks/trait.TaskNode.html)
- [`NodeId`](docsrs:swiftide/tasks/struct.NodeId.html)
- [`Transition`](docsrs:swiftide/tasks/struct.Transition.html)
- [`JoinInput`](docsrs:swiftide/tasks/struct.JoinInput.html)
- [`AnyJoinInput`](docsrs:swiftide/tasks/struct.AnyJoinInput.html)
- [`ConcurrencyModel`](docsrs:swiftide/tasks/enum.ConcurrencyModel.html)
- [`TaskRunOutcome`](docsrs:swiftide/tasks/enum.TaskRunOutcome.html)
- [`TaskError`](docsrs:swiftide/tasks/enum.TaskError.html)
- [`NodeError`](docsrs:swiftide/tasks/struct.NodeError.html)
- [`TaskAgent`](docsrs:swiftide/agents/struct.TaskAgent.html)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`ToolExecutor`](docsrs:swiftide/traits/trait.ToolExecutor.html)
- [Feature flags](/reference/feature-flags/)
- [Crates](/reference/crates/)

## Next

[Nodes and transitions](/tasks/nodes-and-transitions/)
