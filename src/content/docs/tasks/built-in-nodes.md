---
title: "Built-in nodes"
description: "Register a SimplePrompt, ChatCompletion or ToolExecutor trait object, or a plain closure, as a task node without writing a type."
sidebar:
  order: 6
identLint:
  ignore: [downcast_ref, "Box::pin(async move { .. })", "register_node(openai)"]
---

You end with a model call, a shell command and a reusable closure each running as a task node, and a rule for when to write your own `TaskNode` instead. Swiftide implements `TaskNode` for `Arc<dyn ..>` and `Box<dyn ..>` trait objects of `SimplePrompt`, `ChatCompletion` and `ToolExecutor`, and for the `SyncFn` and `AsyncFn` closure wrappers.

The snippets use the OpenAI client and the local tool executor, so enable those feature flags next to `swiftide-tasks`:

```sh
cargo add swiftide@{{swiftide_version}} --features swiftide-tasks,swiftide-agents,openai
```

## Prompt nodes

A closure builds a `Prompt`, and the model client answers it:

::snippet{src="@snippets/src/tasks/built_in_nodes.rs" anchor="prompt_node"}

The closure fills a template and returns the `Prompt`. The next node is the model client itself.

The cast matters. Swiftide implements `TaskNode` for `Arc<dyn SimplePrompt>` and `Box<dyn SimplePrompt>`, not for each client type. `register_node(openai)` on its own does not compile, because the concrete client is not a node.

A `SimplePrompt` node has `Input = Prompt`, `Output = String` and `Error = LanguageModelError`. Each evaluation clones the input `Prompt` and calls `prompt` on the client, which renders the template. See [Prompts and templates](/models/prompts-and-templates/) for building the `Prompt`.

One `Arc<dyn SimplePrompt>` can back several nodes, as in the `examples/tasks.rs` excerpt in [Nodes and transitions](/tasks/nodes-and-transitions/#transitions).

The workspace-architect showcase uses the same pattern. Its join node builds one `Prompt` from every crate summary, and a `SimplePrompt` node renders it into an `ARCHITECTURE.md` draft. In the approved run from `OUTPUT.md`, the render step took 28 seconds for an 11 KB draft. The model was `claude-sonnet-5-5`, called through Chat Completions on an OpenAI-compatible gateway (Bifrost in front of AWS Bedrock):

```text
[  40.5s] join     10 summaries; 230s of agent work in 41s wall-clock (5.7x)
[  40.5s] render   drafting ARCHITECTURE.md
[  68.8s] review   draft ready (11210 bytes), waiting for approval
```

## Chat completion nodes

When a step needs a system message, a full message history or tool specs, use a `ChatCompletion` node instead:

::snippet{src="@snippets/src/tasks/built_in_nodes.rs" anchor="chat_completion_node"}

An `Arc<dyn ChatCompletion>` or `Box<dyn ChatCompletion>` node has `Input = ChatCompletionRequest<'static>`, `Output = ChatCompletionResponse` and `Error = LanguageModelError`. It calls `complete` with the request.

Task payloads must be `'static`, so the request has to own its messages. `ChatCompletionRequest::from(Vec<ChatMessage>)` does that, and so does `ChatCompletionRequest::builder()` with an owned `Vec`. To keep a borrowed request, call `to_owned()` on it first.

The response is the whole `ChatCompletionResponse`. A node after it reads `message()`, which returns `Option<&str>`, or `tool_calls()`. The node does not run tools. To act on tool calls in a loop, use an agent. [Models overview](/models/overview/) covers the clients that implement `ChatCompletion`.

## Tool executor nodes

A `ToolExecutor` node runs one command. Pair it with a closure that builds the `Command`:

::snippet{src="@snippets/src/tasks/built_in_nodes.rs" anchor="tool_executor_node"}

An `Arc<dyn ToolExecutor>` or `Box<dyn ToolExecutor>` node has `Input = Command`, `Output = CommandOutput` and `Error = CommandError`. It calls `exec_cmd` on the executor. `LocalExecutor` runs commands on the local machine. [Executors](/agents/executors/) covers the executors and their commands.

Build the input with `Command::shell`, `Command::read_file` or `Command::write_file`. Chain `with_current_dir` or `with_timeout` to set a directory or a time limit for one command.

Since 0.33, `CommandOutput` keeps standard output (stdout) and standard error (stderr) apart. `stdout()` and `stderr()` iterate over the chunks of one stream, in the order the executor read them. `to_string_lossy()` and `into_string_lossy()` merge both streams into one text.

A command that exits with a non-zero status does not produce output for the next node. `LocalExecutor` returns `CommandError::NonZeroExit` with the captured output, and the run fails with `TaskError::NodeError`. Its `node_error` field holds the original `CommandError`, so `downcast_ref` gets the output back, as the snippet does.

`NonZeroExit` also carries every I/O error, such as a command that fails to start or a missing file for `Command::read_file`. In that case the error text sits in a stdout chunk, not in stderr. The snippet prints both streams with `to_string_lossy()` for that reason. A command that runs past its timeout fails with `CommandError::TimedOut`, which the snippet passes on with the rest.

## Closure adapters

`register_node_fn` wraps a closure in `SyncFn`, and `register_node_async_fn` wraps one in `AsyncFn`. Both types are public. Build one yourself when you want to name the node's type, store it in a struct, or register the same adapter more than once:

::snippet{src="@snippets/src/tasks/built_in_nodes.rs" anchor="closure_adapters"}

`SyncFn::new` takes a closure `Fn(&I) -> Result<O, E>`. `AsyncFn::new` takes a closure that returns `Box::pin(async move { .. })`. Both are `Clone`, and every `register_node` call adds a separate node.

The error type defaults to `NodeError`, but a closure still needs its error type written out; see [Closure nodes](/tasks/nodes-and-transitions/#closure-nodes).

## Custom node or built-in

Use a built-in node when the input and output types already fit the step. A prompt in and text out is a `SimplePrompt` node. A command in and its output out is a `ToolExecutor` node. Small conversions between steps are closures.

Implement `TaskNode` yourself when the step:

- takes or returns your own domain types
- calls more than one service, or calls one in a loop
- keeps state that the caller reads from outside the task
- needs structured output from a model
- should fail with an error that names its input

The showcase's `CrateAnalyst` runs an agent and returns a typed `CrateSummary`. Its `ApprovalGate` keeps a draft that the command-line interface (CLI) approves from outside. Both are custom nodes. [Agents as task nodes](/tasks/agents-as-nodes/#custom-agent-node) and [Pause and resume](/tasks/pause-and-resume/#approval-gate) show them.

## Gotchas

- A `SimplePrompt` node clones its `Prompt` on every evaluation. Keep large values out of the template context if the node runs often.
- A `ToolExecutor` node runs commands in the executor's working directory. With `LocalExecutor`, a relative `with_current_dir` path resolves against that directory.
- To branch on a failed command instead of failing the run, write your own node that calls the executor and returns an enum. Node outputs must be `Clone`, and `CommandError` is not, so map the error to a type of your own first.

## Reference

- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`ChatCompletionRequest`](docsrs:swiftide/chat_completion/struct.ChatCompletionRequest.html)
- [`ChatCompletionResponse`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponse.html)
- [`LanguageModelError`](docsrs:swiftide/chat_completion/errors/enum.LanguageModelError.html)
- [`ToolExecutor`](docsrs:swiftide/traits/trait.ToolExecutor.html)
- [`Command`](docsrs:swiftide/traits/enum.Command.html)
- [`CommandOutput`](docsrs:swiftide/traits/struct.CommandOutput.html)
- [`CommandError`](docsrs:swiftide/traits/enum.CommandError.html)
- [`LocalExecutor`](docsrs:swiftide/agents/tools/local_executor/struct.LocalExecutor.html)
- [`SyncFn`](docsrs:swiftide/tasks/struct.SyncFn.html)
- [`AsyncFn`](docsrs:swiftide/tasks/struct.AsyncFn.html)
- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)
- [`TaskNode`](docsrs:swiftide/tasks/trait.TaskNode.html)
- [`NodeError`](docsrs:swiftide/tasks/struct.NodeError.html)

## Next

[How indexing works](/indexing/overview/)
