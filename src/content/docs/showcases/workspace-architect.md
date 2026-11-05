---
title: "Workspace architect"
description: "A typed task graph that runs one agent per crate of a Cargo workspace in parallel, joins their reports, drafts an ARCHITECTURE.md and writes it after your approval."
sidebar:
  order: 3
identLint:
  ignore: [JsonSchema, aws_lc_rs]
---

This showcase gives you a command-line interface (CLI) that drafts an `ARCHITECTURE.md` for any Cargo workspace. A task graph discovers the crates, runs one agent per crate in parallel, joins their reports, has a model write the document, and pauses until you approve it. Only then does it write the file.

The crate lives in the swiftide repository under `examples/workspace-architect`. It is not in the {{swiftide_version}} tag: until the showcases reach master, they live on the `docs/showcases` branch. Clone the repository and run the commands below from its root:

```sh
git clone https://github.com/bosun-ai/swiftide
cd swiftide
git checkout docs/showcases
```

## Prerequisites

- Rust 1.97.1, which the swiftide workspace pins in `rust-toolchain.toml`.
- `OPENAI_API_KEY`. The CLI checks for it before any agent starts. `OPENAI_BASE_URL` points it at any OpenAI-compatible endpoint, and `OPENAI_MODEL` picks the model (default `gpt-5.4`). The CLI also reads `.env` in the current directory, but not in parent directories.
- `cargo`, `sh`, `find`, `sort` and `grep` on your `PATH`. The discover step runs `cargo metadata`. The agent tools run `find` piped into `sort` through a shell, and `grep`.
- The feature flags `swiftide-agents`, `swiftide-tasks` and `openai`, plus the default `macros`. `swiftide-agents` already turns on `swiftide-tasks`.
- A direct `schemars` dependency with the `derive` feature, for the `JsonSchema` derive on `CrateReport`.
- A direct `rustls` dependency with the `aws_lc_rs` feature.

Swiftide does not select a Transport Layer Security (TLS) crypto provider, so `main` installs the rustls aws-lc-rs provider before anything else ([Crypto provider](/reference/feature-flags/#crypto-provider)). No Docker or database is needed.

## Task graph

The module comment in `graph.rs` sketches the graph:

```text
discover ──fan-out──▶ analyse × N (parallel) ──▶ join (builds prompt) ──▶ render ──▶ review ──▶ write
                                                                                     │  ▲
                                                                            pause ◀──┘  └── resume
```

The task is a `Task<WorkspaceRoot, WrittenDocument>`: a workspace root goes in, the written file comes out. It has six task nodes, registered first:

::snippet{src="@swiftide/examples/workspace-architect/src/graph.rs" anchor="nodes" title="examples/workspace-architect/src/graph.rs"}

- `discover` and `join` are sync closures registered with `register_node_fn`. A closure takes `&I` and returns `Result<O, E>`.
- `analyse` is a struct that implements `TaskNode`, registered with `register_node`.
- `render` is the model client itself, registered as `Arc<dyn SimplePrompt>`.
- `review` is another custom `TaskNode`, the approval gate.
- `write` is an async closure registered with `register_node_async_fn`.

`task.starts_with(discover)` sets the entry node. The excerpts below show most transitions. Three short ones in `graph.rs` sit between the excerpts: `starts_with`, the `render` to `review` transition covered under Join and render, and `task.register_transition(write, task.transitions_to_finish())`, which ends the run.

## Per-crate agents

Every fan-out branch evaluates the same `analyse` node with a different crate. Each evaluation builds a fresh agent:

::snippet{src="@swiftide/examples/workspace-architect/src/analyst.rs" anchor="agent" title="examples/workspace-architect/src/analyst.rs"}

`LocalExecutor::new(&member.dir)` makes the crate directory the working directory for every command the tools run. The context wraps that tool executor with `DefaultContext::from_executor`. Branches share nothing, so they can run at the same time.

`without_default_stop_tool()` comes before `tools(..)`, because `tools` merges in the default `stop` tool when you call it. The `before_tool` hook receives `&mut Agent` and the tool call. It counts the call, and with `--verbose` it also prints a `tool` line. `.limit(MAX_TURNS)` caps one query at 30 completions.

The three tools are read-only and run their commands through the agent's executor:

::snippet{src="@swiftide/examples/workspace-architect/src/tools.rs" anchor="tools" title="examples/workspace-architect/src/tools.rs"}

`#[swiftide::tool]` turns each async function into a constructor that returns `Box<dyn Tool>`. The `description` and `param` attributes set what the model sees. `CratePath::parse` rejects absolute paths and `..` before any command runs.

### Structured report

The agent finishes by calling a custom `stop` tool whose arguments must match `CrateReport`:

::snippet{src="@swiftide/examples/workspace-architect/src/summary.rs" anchor="stop_schema" title="examples/workspace-architect/src/summary.rs"}

`StopWithArgs::with_parameters_schema` works as in the [release notes agent](/showcases/release-notes-agent/#structured-stop), here with the schema of `CrateReport`. The doc comments become field descriptions in that schema, so they double as instructions. When the model calls `stop`, the agent stops with `StopReason::RequestedByTool(_, Some(arguments))`, and the arguments are the report.

### Task node

`CrateAnalyst` implements `TaskNode` by running the agent and reading the report from its stop reason:

::snippet{src="@swiftide/examples/workspace-architect/src/analyst.rs" anchor="task_node" title="examples/workspace-architect/src/analyst.rs"}

`#[serde(deny_unknown_fields)]` matters here. Some models nest the report under an `arguments` key, and serde rejects that with ``unknown field `arguments` ``. The node sends the error back with `agent.query(..)` and reads the stop reason again, at most `MAX_CORRECTIONS` (2) times. `reconciled_with` then replaces the model's dependency list with the one from `cargo metadata`, keeping the model's usage notes.

## Parallel fan-out

The `discover` transition turns the crate list into one branch per crate:

::snippet{src="@swiftide/examples/workspace-architect/src/graph.rs" anchor="fan_out" title="examples/workspace-architect/src/graph.rs"}

`Transition::fan_out(&analyse, first)` creates the first branch, and `.and(&analyse, member)` adds one per remaining crate. `fan_out` needs a first target, so an empty crate list fails the run with `Transition::error` and "the workspace has no crates to document". By default only publishable crates are selected. A workspace whose members all set `publish = false` fails the same way unless you pass `--all-members`. The default concurrency model is `ConcurrencyModel::Sequential`, so `.concurrency_model(ConcurrencyModel::Parallel)` is what runs the branches at the same time. `.join_with(join.join())` turns the `FanOutTransition` into a `Transition` and names the join node that waits for the group.

The join line in the captured run, with `claude-sonnet-5-5` as the model, shows the effect: `230s of agent work in 41s wall-clock (5.7x)`.

## Join and render

The graph registers one transition for the `analyse` node. Every fan-out branch evaluates that node, so all branches use the same transition to the join. The join then hands a prompt to `render`:

::snippet{src="@swiftide/examples/workspace-architect/src/graph.rs" anchor="join" title="examples/workspace-architect/src/graph.rs"}

Fan-out branches cannot finish the task themselves. They have to reach the join. The join runs once every branch has delivered its `CrateSummary`. `render.transitions_with(prompt)` sends the result on to the render node. The join closure calls `architecture_prompt`:

::snippet{src="@swiftide/examples/workspace-architect/src/render.rs" anchor="render" title="examples/workspace-architect/src/render.rs"}

`JoinInput<CrateSummary>` gives you `iter()`, `len()` and `is_empty()`. The prompt is a Tera template with two context values: `crates`, the serialized summaries, and `dependency_graph`. The Mermaid dependency graph is generated from `cargo metadata`, not by the model, and the template tells the model to copy it verbatim.

The `render` to `review` transition wraps the model's `String` in an `ArchitectureDraft`. `ArchitectureDraft::from_model_output` strips a code fence around the whole document, in case the model adds one.

## Pause and resume

The review transition either continues to `write` or pauses the task:

::snippet{src="@swiftide/examples/workspace-architect/src/graph.rs" anchor="pause" title="examples/workspace-architect/src/graph.rs"}

`Transition::pause()` pauses the current branch. A paused branch keeps its node, and on `resume()` that node evaluates again with the same input. The approval gate is built around that:

::snippet{src="@swiftide/examples/workspace-architect/src/approval.rs" anchor="approval" title="examples/workspace-architect/src/approval.rs"}

The first evaluation stores the draft and answers `Review::AwaitingApproval`. After `approve()`, the second evaluation lets the same draft through. Putting the pause on `render` would call the model a second time.

The CLI drives the loop:

::snippet{src="@swiftide/examples/workspace-architect/src/main.rs" anchor="resume" title="examples/workspace-architect/src/main.rs"}

`run` returns `TaskRunOutcome::Paused` instead of an output. `current_nodes::<ApprovalGate>()` returns the paused gate as `CurrentNodes::One`, so the CLI can show its draft and call `approve()` before `task.resume()`.

## Commands

From the swiftide repository root:

```sh
export OPENAI_API_KEY=sk-...
cargo run -p workspace-architect
```

To document another workspace without the approval pause:

```sh
cargo run -p workspace-architect -- --workspace ../other-repo --yes
```

| Flag | Default | Description |
| --- | --- | --- |
| `--workspace <PATH>` | `.` | Any directory inside the workspace to document |
| `--output <PATH>` | `ARCHITECTURE.md` | Output file, relative to the workspace root |
| `--yes` | off | Write the draft without pausing for approval |
| `--all-members` | off | Also document unpublished members such as examples |
| `--model <MODEL>` | `gpt-5.4` | Model for the analysts and the writer (env `OPENAI_MODEL`) |
| `--responses-api` | off | Use the OpenAI Responses application programming interface (API) instead of Chat Completions |
| `--verbose` | off | Print every tool call and each agent's raw report |

Swiftide's tracing is off unless `WORKSPACE_ARCHITECT_LOG` is set, for example to `swiftide=debug`. The CLI ignores `RUST_LOG`, so a global cargo config that sets it does not bury the draft. The client sets `max_completion_tokens` to 16,384 through `Options`, because some gateways cap an unset limit at 4,096 tokens and the document would be cut off.

The tests run the whole graph offline with a scripted model, including the pause, approval and `resume()`:

```sh
cargo test -p workspace-architect
```

## Output

This run on the swiftide repository used `claude-sonnet-5-5` for all ten analysts and the writer, through an OpenAI-compatible gateway (Bifrost in front of AWS Bedrock) with Chat Completions. The run answered `y` at the prompt and passed `--output target/ARCHITECTURE.md`. Trimmed from `OUTPUT.md`:

```text
[   0.0s] model    claude-sonnet-5-5 via Chat Completions at http://127.0.0.1:8080/v1
[   0.0s] discover 10 crates (7 other members skipped) in /Users/timonv/projects/swiftide-wt-docs-showcases
[   0.0s] analyse  started  swiftide-core
[   0.0s] analyse  started  swiftide-langfuse
...
[  14.0s] analyse  finished swiftide-langfuse in 14.0s (6 tool calls)
[  14.3s] analyse  finished swiftide-macros in 14.3s (6 tool calls)
...
[  33.9s] analyse  retry    swiftide-core: the `stop` arguments do not match the schema: unknown field `arguments`, expected one of `purpose`, `key_types`, `depends_on`, `notes`
[  40.5s] analyse  finished swiftide-core in 40.5s (15 tool calls)
[  40.5s] join     10 summaries; 230s of agent work in 41s wall-clock (5.7x)
[  40.5s] render   drafting ARCHITECTURE.md
[  68.8s] review   draft ready (11210 bytes), waiting for approval
...
[  68.8s] review   draft approved
[  68.8s] write    /Users/timonv/projects/swiftide-wt-docs-showcases/target/ARCHITECTURE.md (11210 bytes)
Wrote /Users/timonv/projects/swiftide-wt-docs-showcases/target/ARCHITECTURE.md (11210 bytes) in 68.8s.
```

A second run with `--verbose` took 217s of agent work in 33s (6.5x). Writing the document took about 28 seconds in both runs. A few lines of the generated crate map:

```text
| Crate | Responsibility | Start reading at |
|---|---|---|
| `swiftide-core` | Shared traits, data types and errors for indexing, querying, chat completion, tools and agent context. No integrations. | `Node`, `Query`, `ChatCompletion` |
...
| `swiftide-tasks` | In-process typed task graph engine with transitions, fan-out/join, pause and resume, and adapters for closures and core traits. | `Task`, `TaskNode`, `Transition` |
| `swiftide-agents` | LLM agent loop over chat completions and tool calls, with hooks, a default context, a local executor and optional MCP toolboxes. | `Agent`, `AgentBuilder`, `DefaultContext` |
```

The run documents a swiftide checkout that includes `swiftide-agents-contrib` :unreleased. That crate is on master but not in the {{swiftide_version}} release.

## Gotchas

- Retries cost time. In the approved run a retry made `swiftide-core` the slowest branch, and the join waits for it.
- Each correction is a new `query`, so it gets a fresh turn limit. One crate can use up to 90 completions.
- See [Turn limit and nudges](/showcases/release-notes-agent/#turn-limit-and-nudges) in the release notes agent for how a turn limit and a missing tool call share a stop reason. This node treats both as a missing report.
- `LocalExecutor` sets a working directory. It does not sandbox. The tools reject absolute paths and `..`, but symlinks inside a crate are followed. Treat the checks as a guard rail.

## Concept pages

- [Tasks overview](/tasks/overview/)
- [Nodes and transitions](/tasks/nodes-and-transitions/)
- [Fan-out and join](/tasks/fan-out-and-join/)
- [Pause and resume](/tasks/pause-and-resume/)
- [Agents as nodes](/tasks/agents-as-nodes/)
- [Built-in nodes](/tasks/built-in-nodes/)
- [Executors](/agents/executors/)
- [Stopping and structured results](/agents/stopping-and-structured-results/)
- [Tools](/agents/tools/)

## Reference

- [`Task`](docsrs:swiftide/tasks/struct.Task.html)
- [`TaskNode`](docsrs:swiftide/tasks/trait.TaskNode.html)
- [`Transition`](docsrs:swiftide/tasks/struct.Transition.html)
- [`FanOutTransition`](docsrs:swiftide/tasks/struct.FanOutTransition.html)
- [`ConcurrencyModel`](docsrs:swiftide/tasks/enum.ConcurrencyModel.html)
- [`JoinInput`](docsrs:swiftide/tasks/struct.JoinInput.html)
- [`JoinTarget`](docsrs:swiftide/tasks/struct.JoinTarget.html)
- [`TaskRunOutcome`](docsrs:swiftide/tasks/enum.TaskRunOutcome.html)
- [`CurrentNodes`](docsrs:swiftide/tasks/enum.CurrentNodes.html)
- [`TaskError`](docsrs:swiftide/tasks/enum.TaskError.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`DefaultContext`](docsrs:swiftide/agents/struct.DefaultContext.html)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`LocalExecutor`](docsrs:swiftide/agents/tools/local_executor/struct.LocalExecutor.html)
- [`StopWithArgs`](docsrs:swiftide/agents/tools/control/struct.StopWithArgs.html)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`Options`](docsrs:swiftide/integrations/openai/struct.Options.html)
- [`#[swiftide::tool]`](docsrs:swiftide/attr.tool.html)

## Next

[Docs Q&A with evaluation](/showcases/docs-qa/)
