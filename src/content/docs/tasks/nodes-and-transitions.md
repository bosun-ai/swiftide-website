---
title: "Nodes and transitions"
description: "Register closures and TaskNode types as nodes and connect them with typed transitions from start to finish."
sidebar:
  order: 2
identLint:
  ignore: [CountWords, Infallible, ParseIntError, transitions_to_done]
---

Here you build a linear task from two closures and a node type of your own, run it, and find out which node failed when something goes wrong.

Import the task types from `swiftide::tasks`. [Tasks overview](/tasks/overview/#feature-flag) shows the feature flag to turn on.

::snippet{src="@snippets/src/tasks/nodes_and_transitions.rs" anchor="imports"}

Register the nodes, connect them, and run the task:

::snippet{src="@snippets/src/tasks/nodes_and_transitions.rs" anchor="linear"}

The run prints `3 words`.

`Task<String, String>` fixes the types at both ends. Each node declares its own input and output, so a `String` becomes a `usize` and then a `String` again.

`register_node_fn` turns a synchronous closure into a node, and `register_node_async_fn` does the same for a closure that returns a pinned, boxed future (`Box::pin(async move { .. })`). `register_node` takes any value that implements `TaskNode`. All three return a `NodeId`, the typed handle you wire the node with. `NodeId` is `Copy`, so you move it into transition closures without cloning. `id()` returns its number, assigned in registration order from 0.

`starts_with` picks the start node, and `register_transition` says where to go after a node. `transitions_with` moves on to another node with a value, and `transitions_to_finish` hands the task its output. Inside a fan-out branch, that output goes to the join instead of ending the task.

`run` returns a `TaskRunOutcome`. `Completed` carries the output. `Paused` is covered in [Pause and resume](/tasks/pause-and-resume/).

## Task nodes

A task node is a type that implements `TaskNode`:

::snippet{src="@snippets/src/tasks/nodes_and_transitions.rs" anchor="custom_node"}

`Input` is what the node receives and `Output` is what it returns. `Error` can be any `std::error::Error + Send + Sync + 'static`; `CountWords` cannot fail, so it uses `Infallible`.

`evaluate` borrows both the node and the input. The `&DynNodeId<Self>` argument is the node's own id with its concrete type erased, and most nodes ignore it. Write the impl with `#[async_trait]`, which Swiftide re-exports as `swiftide::reexports::async_trait`.

`Input` and `Output` must implement `NodeArg`, which covers every type that is `Send + Sync + Debug + Clone + 'static`. A missing derive shows up as an unmet `NodeArg` bound. `register_node` also needs the node itself to be `Clone`.

A run does not clone your node. Every evaluation calls the same instance through `&self`, so keep per-run state out of the struct.

## Closure nodes

The workspace-architect showcase registers its nodes in one block. `discover` and `write` are closure nodes:

::snippet{src="@swiftide/examples/workspace-architect/src/graph.rs" anchor="nodes" title="examples/workspace-architect/src/graph.rs"}

- `discover` is a sync closure. `Ok::<_, workspace::DiscoveryError>(members)` pins its error type, so the `?` inside converts into it.
- `write` is a plain closure that returns `Box::pin(async move { .. })`.

The other four nodes have their own pages: [Agents as task nodes](/tasks/agents-as-nodes/), [Fan-out and join](/tasks/fan-out-and-join/), [Built-in nodes](/tasks/built-in-nodes/) and [Pause and resume](/tasks/pause-and-resume/).

A closure node needs a concrete error type. Name it with `Ok::<_, E>(..)` or a return type such as `-> Result<i32, NodeError>`, unless the closure already returns a typed `Result`, as `str::parse` does.

The closure is `Fn` and runs once per evaluation, so each future needs its own copy of the captures. That is why `write` clones `output` and `progress` before `async move`. The future may still borrow the input, here `draft`.

For reusable domain logic, implement `TaskNode` instead.

## Transitions

`examples/tasks.rs` chains a prompt node, an agent node and a second prompt node:

::snippet{src="@swiftide/examples/tasks.rs" anchor="task"}

A transition closure receives the node's output by value and returns where to go next. `agent_id.transitions_with(brief)` is shorthand for `Transition::next(&agent_id, brief)`. Both return a `MarkedTransition` that carries the target's type, so the compiler checks the value against the target's `Input`.

Each node has exactly one transition, and registering a second one returns `TaskError::InvalidState`. To branch, return different transitions from that one closure. When the arms point at nodes of different types, or one arm pauses or fails, convert each `MarkedTransition` into a `Transition` with `.into()`, as the linear task does.

`Transition::error` fails the whole run, drops any in-flight fan-out branches and leaves nothing to resume. It accepts anything that converts into `Box<dyn Error + Send + Sync>`, including a `&str`. `Transition::pause()` pauses the branch.

When the decision needs to await something, use `register_transition_async`. Its closure returns a future, such as `move |value| async move { next.transitions_with(value) }`.

## Start and finish

The start node's `Input` must be the task's `Input`, or `starts_with` does not compile. `run` takes `impl Into<Input>`, so the linear task can pass a `&str` to a `Task<String, String>`.

`transitions_to_finish()` returns a closure that takes the task's `Output`. The finish constructor on `Transition` is private, so the compiler checks the value against that task's output type. A finish closure taken from a different task still compiles, and `run` then fails with `TaskError::TypeError`.

Code from 0.32 called `transitions_to_done` and got an `Option` from `run`. [Upgrading](/reference/upgrading/#tasks-crate) lists the 0.33 changes.

## Errors

Match on the result of `run` to tell a failing node from a wiring mistake:

::snippet{src="@snippets/src/tasks/nodes_and_transitions.rs" anchor="errors"}

A failing node makes `run` return `TaskError::NodeError`. The `NodeError` inside carries `node_id`, the number of the failing node, and `node_error`, the node's own error boxed. Downcast `node_error` to read your error type. The error displays as `Node error in node 0: invalid digit found in string`. A `Transition::error` produces the same variant, with the id of the node whose transition failed.

`run` and `resume` check the graph before any node runs. Without a start node you get `TaskError::NoSteps`. When any registered node has no transition, you get `TaskError::MissingTransition`, even when no path reaches that node, with a message such as `Node 3 is missing a transition`.

`TaskError::Incomplete` means all work drained without a finish or a pause.

## Reference

- [`Task`](docsrs:swiftide/tasks/struct.Task.html)
- [`TaskNode`](docsrs:swiftide/tasks/trait.TaskNode.html)
- [`NodeId`](docsrs:swiftide/tasks/struct.NodeId.html)
- [`DynNodeId`](docsrs:swiftide/tasks/type.DynNodeId.html)
- [`NodeArg`](docsrs:swiftide/tasks/trait.NodeArg.html)
- [`Transition`](docsrs:swiftide/tasks/struct.Transition.html)
- [`MarkedTransition`](docsrs:swiftide/tasks/struct.MarkedTransition.html)
- [`TaskRunOutcome`](docsrs:swiftide/tasks/enum.TaskRunOutcome.html)
- [`TaskError`](docsrs:swiftide/tasks/enum.TaskError.html)
- [`NodeError`](docsrs:swiftide/tasks/struct.NodeError.html)

## Next

[Fan-out and join](/tasks/fan-out-and-join/)
