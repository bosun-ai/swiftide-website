---
title: "Pause and resume"
description: "Pause a task at an approval gate, inspect the paused node, and resume or rerun it with the same Task value."
sidebar:
  order: 4
identLint:
  ignore: [AwaitCi, deliver_through_task]
---

You end up with a task that stops before a side effect, waits for a person or an outside event, and then continues from the node where it stopped. The workspace-architect showcase uses this to hold its generated `ARCHITECTURE.md` until someone approves it:

::snippet{src="@swiftide/examples/workspace-architect/src/graph.rs" anchor="pause" title="examples/workspace-architect/src/graph.rs"}

The `review` node returns a `Review`. When the draft is approved, the transition sends it on to the `write` node. When it still waits for approval, the transition returns `Transition::pause()`, and `run` comes back with `TaskRunOutcome::Paused` instead of an output.

The `.into()` on the first arm is the same `MarkedTransition` conversion described in [Nodes and transitions](/tasks/nodes-and-transitions/#transitions). The types live in `swiftide::tasks`, behind the `swiftide-tasks` [feature flag](/tasks/overview/#feature-flag):

```sh
cargo add swiftide@{{swiftide_version}} --features swiftide-tasks
```

A task pause is not the same as an agent pause. To stop an agent until someone approves a single tool call, see [Human in the loop](/agents/human-in-the-loop/).

## Approval gate

The `review` node is `ApprovalGate`, a task node that remembers the draft and whether someone approved it:

::snippet{src="@swiftide/examples/workspace-architect/src/approval.rs" anchor="approval" title="examples/workspace-architect/src/approval.rs" collapse="20-29,40-42"}

A paused branch keeps its current node and that node's input. On `resume()`, the runtime evaluates the same node again with the same input, and then runs its transition again. The gate relies on this. The first evaluation stores the draft and answers `AwaitingApproval`. After `approve()`, the second evaluation answers `Approved` with the same draft.

Put the pause on a cheap node. The showcase could pause right after `render`, but then `resume()` would call the model and draft the document a second time.

The gate keeps its state in `Arc<Mutex<GateState>>`. The `Mutex` is there because `evaluate` and `approve` take `&self`, so plain fields cannot change. The `Arc` is there because a task node must be `Clone`, and `Mutex` is not. With the `Arc`, clones of the gate share one state. The gate's error type is `Infallible`, because waiting is an answer, not a failure.

## Resume loop

The command-line interface (CLI) runs the task and loops until it completes or the user declines the draft:

::snippet{src="@swiftide/examples/workspace-architect/src/main.rs" anchor="resume" title="examples/workspace-architect/src/main.rs"}

On `TaskRunOutcome::Paused`, `approve_paused_draft` (in the same file) finds the gate with `task.current_nodes::<ApprovalGate>()`, shows the draft and asks for confirmation. On yes it calls `gate.approve()` and returns `true`, and the loop calls `task.resume()`. On no it returns `false`, and `draft_architecture` returns `Ok(None)` and drops the paused task. Nothing is written.

This is the end of a captured run, with `claude-sonnet-5-5` as the writer. The draft between the review line and the question is trimmed:

```text
[  68.8s] review   draft ready (11210 bytes), waiting for approval
...
Write this ARCHITECTURE.md? [y/N]
[  68.8s] review   draft approved
[  68.8s] write    .../target/ARCHITECTURE.md (11210 bytes)
Wrote .../target/ARCHITECTURE.md (11210 bytes) in 68.8s.
```

The `--yes` flag creates the gate already approved, so the first evaluation answers `Approved` and the task never pauses.

## Current nodes

`Task::current_nodes::<T>()` gives you the paused node, so you can read or change its state before you resume. This helper delivers the CI result from the next section through the task:

::snippet{src="@snippets/src/tasks/pause_and_resume.rs" anchor="current_nodes"}

The method returns the current nodes of type `T` as a `CurrentNodes`:

- `CurrentNodes::None` when no current node has that type.
- `CurrentNodes::One(&T)` for exactly one.
- `CurrentNodes::Multiple(Vec<&T>)` for more than one.

Paused branches take precedence. The method returns runnable nodes only when no branch is paused. In a parallel fan-out several branches can pause on the same node type, so handle `Multiple` there. The references point at the registered node instances.

## External callbacks

The same pattern waits for another system, such as a continuous integration (CI) webhook. The node reads a slot that the webhook handler fills, and the transition pauses while the slot is empty:

::snippet{src="@snippets/src/tasks/pause_and_resume.rs" anchor="external_callback"}

The excerpt registers `ci.clone()` and keeps `ci` itself. In a server, `ci` goes to the webhook handler, and the comment marks where that handler runs. The clone shares the `Arc`, so `deliver` fills the slot that the registered node reads. A handler that holds the task instead can use `deliver_through_task` from the previous section.

Keep the same `Task` value alive between the pause and the resume. The runtime state is not serializable. If the process restarts, the paused run is gone, so store what you need to start again, such as the commit. In a server, keep paused tasks in a map keyed by job.

One task value holds one run. While the task has paused or queued work, `run` returns `TaskError::TaskActive`. Build a new task for the next job, for example with a function like the showcase's `architect_task`.

## Reruns after reset

`reset()` discards the current run and queues a new one at the start node, with the last input passed to `run`. A `resume()` after it runs that queued run:

::snippet{src="@snippets/src/tasks/pause_and_resume.rs" anchor="reset"}

- After a completed run there is nothing to continue, so `resume()` returns `TaskError::NotResumable`.
- `reset()` also discards a paused run. Use it to start over with the same input instead of continuing.
- The queued run counts as active work, so `run(other_input)` after `reset()` returns `TaskError::TaskActive`. To start over with a different input, use `task.clone()`. Calling `starts_with` again also clears the runtime state.
- `reset()` before the first `run` has no input to queue. The `resume()` that follows returns `TaskError::NotResumable`.
- A failed run leaves no state to resume. `reset()` and `resume()` retry it with the same input.
- `task.clone()` copies the graph with a fresh runtime, without the paused run. Node state behind an `Arc` is shared between the copies.

## Pauses in parallel branches

A pause stops the whole task, not only its branch. When a branch asks to pause, the runtime starts no new branches. Parallel branches that are already running finish their current node and transition, and their next steps wait in the queue. Then `run` returns `Paused`, unless one of those running branches reaches the finish transition. In that case `run` returns `Completed` and the paused branch is discarded.

If a running branch fails after another branch asked to pause, `run` returns that error. The paused run is lost, and `resume()` has nothing to continue.

`resume()` puts the paused branches back in the queue in the order they were created and continues with all queued work. A join runs only when every branch has arrived, so a paused branch also holds back its join. [Fan-out and join](/tasks/fan-out-and-join/#join-node) covers branches and joins.

## Reference

- [`Transition::pause`](docsrs:swiftide/tasks/struct.Transition.html#method.pause)
- [`MarkedTransition`](docsrs:swiftide/tasks/struct.MarkedTransition.html)
- [`Task::run`](docsrs:swiftide/tasks/struct.Task.html#method.run)
- [`Task::resume`](docsrs:swiftide/tasks/struct.Task.html#method.resume)
- [`Task::reset`](docsrs:swiftide/tasks/struct.Task.html#method.reset)
- [`Task::current_nodes`](docsrs:swiftide/tasks/struct.Task.html#method.current_nodes)
- [`CurrentNodes`](docsrs:swiftide/tasks/enum.CurrentNodes.html)
- [`TaskRunOutcome`](docsrs:swiftide/tasks/enum.TaskRunOutcome.html)
- [`TaskError`](docsrs:swiftide/tasks/enum.TaskError.html)
- [`TaskNode`](docsrs:swiftide/tasks/trait.TaskNode.html)

## Next

[Agents as task nodes](/tasks/agents-as-nodes/)
