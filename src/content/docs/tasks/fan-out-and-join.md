---
title: "Fan-out and join"
description: "Fan a task out into one branch per item, optionally run the branches in parallel, and collect their typed outputs in one join node."
sidebar:
  order: 3
---

You end up with a task that runs one task node per item of a list, all at the same time, and collects every result in a single join node. The task types need the `swiftide-tasks` feature flag ([Tasks overview](/tasks/overview/#feature-flag)). The code comes from the workspace-architect showcase, which analyses each crate of a Cargo workspace with its own agent.

::snippet{src="@swiftide/examples/workspace-architect/src/graph.rs" anchor="fan_out" title="examples/workspace-architect/src/graph.rs"}

::snippet{src="@swiftide/examples/workspace-architect/src/graph.rs" anchor="join" title="examples/workspace-architect/src/graph.rs"}

The `discover` node returns the workspace members, and its transition folds them into one fan-out with a branch per crate. Every branch evaluates the same `analyse` node with a different `WorkspaceCrate`, in parallel, and `join_with` names the node that waits for the whole group.

The `analyse` node registers one transition to the join target, `task.register_transition(analyse, join.join())`, and every branch uses it. Transitions belong to nodes, not branches, and each node has [exactly one](/tasks/nodes-and-transitions/#transitions). The join node is a closure over `&JoinInput<CrateSummary>`, registered with `task.register_node_fn` in the `nodes` region of `graph.rs`. It runs once every branch has delivered its `CrateSummary`, and its `Prompt` goes to the render node with an ordinary `transitions_with`. [Built-in nodes](/tasks/built-in-nodes/#prompt-nodes) covers the render node.

## Fan-out

`Transition::fan_out(&node, input)` starts a `FanOutTransition` with one branch. `.and(&node, input)` adds another. Branches can target the same node or different nodes. Each input has to convert into the target node's input type, or the code does not compile.

A `FanOutTransition` becomes a `Transition` only when you call `join_with`, so a fan-out without a join does not compile.

`fan_out` needs a first target, so handle an empty list first. The showcase returns `Transition::error(..)`, which fails the run with a `TaskError::NodeError`.

With branches known up front, write them out. This example starts two branches on two different nodes:

::snippet{src="@swiftide/examples/tasks_fanout.rs" anchor="transitions"}

A branch can pass through intermediate nodes before it joins. Only the last node on the branch registers the join target. If a branch joins a different node than the one its fan-out named, the run fails with `TaskError::InvalidState`.

## Join node

A join node takes `JoinInput<T>` as its input, where `T` is the payload every branch contributes. `NodeId::join()` exists only on nodes with that input type. `JoinInput` has three methods:

- `iter()` yields the payloads in the order the branches were created, not the order they finished.
- `len()` returns the number of payloads.
- `is_empty()` returns true when there are none.

The showcase relies on that order: the prompt lists the crates as `discover` found them, although `swiftide-core` finished last after a retry. A branch whose output does not convert into `T` does not compile.

## Payload mapping

A branch does not have to return the payload type itself. `JoinTarget::map` converts the branch output on its way into the join:

::snippet{src="@snippets/src/tasks/fan_out_and_join.rs" anchor="map_join"}

`map` turns the `usize` from `count` into the `String` the join expects. `JoinTarget::map_async` does the same with an async closure and is registered with `Task::register_transition_async`. `shout` already returns a `String`. Any output that converts into the payload type with `Into` needs no mapping.

The task returns `"2 words; 11 characters; HELLO TASKS"`, in branch creation order.

## Mixed payloads

When branches return different types and the join needs all of them, give the join node an `AnyJoinInput`. Use `join_any()` in place of `join()`, both in the fan-out and for each branch:

::snippet{src="@snippets/src/tasks/fan_out_and_join.rs" anchor="join_any"}

`AnyJoinInput::iter::<T>()` downcasts each payload and skips the ones of other types. `iter_any()` yields every payload as a type-erased value. The compiler no longer checks that a branch delivers a type the join reads, so prefer `JoinInput<T>` with `map` where you can.

## Concurrency model

`ConcurrencyModel` has two variants. `Sequential` is the default, and `Parallel` is opt-in. Set the default for the whole task with `with_default_concurrency_model`:

::snippet{src="@swiftide/examples/tasks_fanout.rs" anchor="nodes"}

To override the model from a given step on, call `concurrency_model` on a `Transition`, `MarkedTransition` or `FanOutTransition`. A branch keeps the override for every later step until another transition overrides it, and the branches of a fan-out inherit its model. The join node always runs with the task's default model.

Under `Sequential`, the runtime evaluates one node at a time. Fan-out branches take turns: after a node finishes, the branch's next step goes to the back of the queue.

Under `Parallel`, the branches run concurrently inside the future that `run` returns, as the [overview](/tasks/overview/#crate-contents) explains. Waiting on input/output (I/O), such as model calls, overlaps. Synchronous, compute-only closures do not.

This is the showcase's captured run, with `claude-sonnet-5-5` as the model for all ten analysts. The `model` and `discover` lines before it and the render steps after it are left out:

```text
[   0.0s] analyse  started  swiftide-core
[   0.0s] analyse  started  swiftide-langfuse
[   0.0s] analyse  started  swiftide-macros
[   0.0s] analyse  started  swiftide-indexing
[   0.0s] analyse  started  swiftide-integrations
[   0.0s] analyse  started  swiftide-query
[   0.0s] analyse  started  swiftide-tasks
[   0.0s] analyse  started  swiftide-agents
[   0.0s] analyse  started  swiftide-agents-contrib
[   0.0s] analyse  started  swiftide
[  14.0s] analyse  finished swiftide-langfuse in 14.0s (6 tool calls)
[  14.3s] analyse  finished swiftide-macros in 14.3s (6 tool calls)
[  14.7s] analyse  finished swiftide in 14.7s (5 tool calls)
[  19.2s] analyse  finished swiftide-query in 19.1s (10 tool calls)
[  19.7s] analyse  retry    swiftide-agents-contrib: the `stop` arguments do not match the schema: unknown field `arguments`, expected one of `purpose`, `key_types`, `depends_on`, `notes`
[  20.2s] analyse  finished swiftide-agents in 20.2s (10 tool calls)
[  24.4s] analyse  finished swiftide-indexing in 24.4s (10 tool calls)
[  24.9s] analyse  finished swiftide-tasks in 24.9s (10 tool calls)
[  27.1s] analyse  finished swiftide-agents-contrib in 27.1s (11 tool calls)
[  31.4s] analyse  finished swiftide-integrations in 31.4s (13 tool calls)
[  33.9s] analyse  retry    swiftide-core: the `stop` arguments do not match the schema: unknown field `arguments`, expected one of `purpose`, `key_types`, `depends_on`, `notes`
[  40.5s] analyse  finished swiftide-core in 40.5s (15 tool calls)
[  40.5s] join     10 summaries; 230s of agent work in 41s wall-clock (5.7x)
```

All ten branches start at 0.0s, and the fan-out takes as long as its slowest branch. The join line sums the agent time of every branch: 230 seconds of work in 41 seconds. `OUTPUT.md` estimates close to four minutes for the same analyses run one after another.

## Limits

- A branch cannot start a second fan-out while it belongs to an active join group. The run fails with `TaskError::InvalidState`. Fan out again from the join node instead.
- A branch cannot finish the task on its own. If a branch inside a group registers `task.transitions_to_finish()`, the task output counts as an arrival at the join, unmapped. When that type differs from the join's payload type, the join fails at runtime with `TaskError::InvalidState`. Route every branch to the join and finish from there.
- The first branch that fails fails the run. The other in-flight branches are dropped and never complete.
- All branches share one node instance; keep per-branch state inside `evaluate` ([Agents as task nodes](/tasks/agents-as-nodes/#custom-agent-node)).

## Reference

- [`Transition`](docsrs:swiftide/tasks/struct.Transition.html)
- [`Transition::error`](docsrs:swiftide/tasks/struct.Transition.html#method.error)
- [`FanOutTransition`](docsrs:swiftide/tasks/struct.FanOutTransition.html)
- [`MarkedTransition`](docsrs:swiftide/tasks/struct.MarkedTransition.html)
- [`JoinInput`](docsrs:swiftide/tasks/struct.JoinInput.html)
- [`JoinTarget`](docsrs:swiftide/tasks/struct.JoinTarget.html)
- [`MappedJoinTarget`](docsrs:swiftide/tasks/struct.MappedJoinTarget.html)
- [`AnyJoinInput`](docsrs:swiftide/tasks/struct.AnyJoinInput.html)
- [`AnyJoinTarget`](docsrs:swiftide/tasks/struct.AnyJoinTarget.html)
- [`ConcurrencyModel`](docsrs:swiftide/tasks/enum.ConcurrencyModel.html)
- [`NodeId`](docsrs:swiftide/tasks/struct.NodeId.html)
- [`NodeId::join_any`](docsrs:swiftide/tasks/struct.NodeId.html#method.join_any)
- [`Task`](docsrs:swiftide/tasks/struct.Task.html)
- [`Task::register_transition_async`](docsrs:swiftide/tasks/struct.Task.html#method.register_transition_async)
- [`TaskError`](docsrs:swiftide/tasks/enum.TaskError.html)

## Next

[Pause and resume](/tasks/pause-and-resume/)
