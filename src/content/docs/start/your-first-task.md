---
title: "Your first task"
description: "Build a typed task with two nodes and one transition, run it to completion and print a one-line report of crate names."
sidebar:
  order: 5
---

You end this page with a `Task<String, String>` that turns a list of crate names into a one-line report. It has two task nodes, one transition between them and a finish. The [workspace-architect](/showcases/workspace-architect/) showcase starts the same way, with a `discover` node, before it fans out to one agent per crate.

## Dependencies

The task needs no model, so the `swiftide-tasks` feature flag is enough, and with no client to build there is no crypto provider to install. `swiftide-agents` also turns `swiftide-tasks` on.

```sh
cargo add swiftide@{{swiftide_version}} --features swiftide-tasks
cargo add tokio --features macros,rt-multi-thread
cargo add anyhow
```

## Program

`main` builds the task and runs it:

::snippet{src="@snippets/src/start/your_first_task.rs" anchor="main" title="src/main.rs"}

## Task nodes

`Task::<String, String>::new()` creates an empty task. The first type parameter is the input you pass to `run`. The second is the output it returns.

`register_node_fn` turns a synchronous closure into a task node. The closure takes a reference to its input and returns a `Result`. The error type must implement `std::error::Error + Send + Sync + 'static`, so `anyhow::Error` does not fit. The closures never fail, so `Ok::<_, NodeError>(..)` only names a type the compiler can infer. The method returns a typed `NodeId`, which you use to wire transitions.

`discover` splits the listing into a `Vec<String>`. `report` takes that vector and formats a line. Every input and output type must be `Clone + Debug + Send + Sync + 'static`. The `NodeArg` trait covers exactly that bound, so plain owned types work.

## Transitions

`starts_with(discover)` marks the node that receives the task input.

`register_transition` connects a node to what runs after it. The closure receives the output of `discover` and returns `report.transitions_with(crates)`. The compiler checks that the input type of `report` matches the output type of `discover`. A mismatch is a compile error, not a runtime surprise.

`task.transitions_to_finish()` ends the task with the output of `report`. The finishing node's output type must equal the task's output type.

Each node gets exactly one transition. A second `register_transition` for the same node returns an error. A node without a transition makes `run` fail with `TaskError::MissingTransition`, and a task without a start node fails with `TaskError::NoSteps`.

## Run outcome

`task.run(input)` returns `Result<TaskRunOutcome<O>, TaskError>`. `TaskRunOutcome::Completed` carries the output. `TaskRunOutcome::Paused` means a node asked the task to pause, which this graph never does.

`cargo run` prints:

```text
3 crates: swiftide-core, swiftide-tasks, swiftide-agents
```

A node error stops the run and comes back as `TaskError::NodeError`. Its message gives the numeric id of the node that failed, as in `Node error in node 0: ...`. The id is the order in which you registered the node, not the name of the variable that holds it.

To run the same graph again with the last input, call `reset` and then `resume`.

## Fan-out

A transition can also fan out. It schedules one branch per input on the same node, and a join node collects the outputs. The [workspace-architect showcase](/showcases/workspace-architect/) uses this to run one agent per crate. [Fan-out and join](/tasks/fan-out-and-join/) covers the API.

## Common mistakes

- Returning `anyhow::Result` from a `register_node_fn` closure does not compile. Use an error type that implements `std::error::Error`, such as your own `thiserror` enum.
- `run` on a task that is still active or paused returns `TaskError::TaskActive`. Continue a paused task with `resume`.
- A human approval step is `Transition::pause()` followed by `task.resume()`. See [Pause and resume](/tasks/pause-and-resume/).
- For nodes that call a model, see [Nodes and transitions](/tasks/nodes-and-transitions/) and [Agents as nodes](/tasks/agents-as-nodes/). An agent node also needs the `swiftide-agents` feature flag and a provider feature flag such as `openai`.

## Reference

- [`Task`](docsrs:swiftide/tasks/struct.Task.html)
- [`TaskRunOutcome`](docsrs:swiftide/tasks/enum.TaskRunOutcome.html)
- [`NodeId`](docsrs:swiftide/tasks/struct.NodeId.html)
- [`NodeError`](docsrs:swiftide/tasks/struct.NodeError.html)
- [`TaskError`](docsrs:swiftide/tasks/enum.TaskError.html)
- [`Transition`](docsrs:swiftide/tasks/struct.Transition.html)

## Next

[Using Swiftide with coding agents](/start/for-coding-agents/)
