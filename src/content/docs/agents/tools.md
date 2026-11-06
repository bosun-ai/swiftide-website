---
title: "Tools"
description: "Write agent tools as async functions with #[swiftide::tool], as stateful structs with #[derive(Tool)], or by implementing Tool by hand with your own schema, and control how tool errors are retried."
sidebar:
  order: 2
identLint:
  ignore: [GitLogSince, GitLogSinceArgs, LineRange, JsonSchema, HashMap, WriteChangelogArgs]
---

You end up with three kinds of tool: async functions the model can call, a tool with its own state, and a hand-written tool with a custom argument schema. The examples come from the release notes agent in `examples/release-notes-agent`, which drafts release notes from git history.

Add the dependencies the macros expect:

```sh
cargo add swiftide@{{swiftide_version}} --features swiftide-agents,openai
cargo add serde --features derive
cargo add schemars@1 async-trait
```

::snippet{src="@swiftide/examples/release-notes-agent/src/tools.rs" anchor="tools" title="examples/release-notes-agent/src/tools.rs"}

## Function tools

`#[swiftide::tool]` turns an async function into a tool. The first argument must be `&dyn AgentContext`. Every other argument becomes a tool argument that the model fills in.

The macro generates three items from `git_log_since`:

- a struct `GitLogSince` that implements `Tool`,
- a function `git_log_since()` that returns it as a `Box<dyn Tool>`, ready for `AgentBuilder::tools`,
- an argument struct `GitLogSinceArgs` that derives serde and `JsonSchema`. Its schema forbids extra properties, but deserialization ignores keys it does not know.

The tool name the model sees is the function name, `git_log_since`. Since 0.33, the description is the doc comment text before the first heading. Each item under `# Arguments` describes one argument. A `# Parameters` heading works the same way.

The tools parse the model's strings into `Revision` and `RepoPath` before they call `context.executor()`, because the `LocalExecutor` in this example does not sandbox anything. See [Executors](/agents/executors/).

A hook printed the first tool call of a real run with `claude-sonnet-5-5`:

```text
Drafting release notes for the 7 commits since v0.33.0 with claude-sonnet-5-5 at http://127.0.0.1:8080/v1 (chat completions)
  → git_log_since v0.33.0
  git_log_since ✓ 346 lines
```

## Argument types

An argument type must implement serde's `Serialize` and `Deserialize`, `schemars::JsonSchema` and `Debug`, because the generated argument struct derives all four. The macro builds the JavaScript Object Notation (JSON) Schema of the arguments with `schemars::schema_for!`:

::snippet{src="@snippets/src/agents/tools.rs" anchor="tool_args_schema"}

- `&str`, `&[T]`, `&Vec<T>` and `&Option<T>` are deserialized as their owned types. Take any other type, such as `LineRange`, by value.
- An `Option` argument is optional. The OpenAI integration marks every property as required, so there the model sends `null` instead of leaving the argument out. The `range` description tells the model so.
- The doc comments of a struct argument become field descriptions.
- A map type such as `HashMap` compiles, but the generated `tool_spec` panics when the agent builds its first request.

Arguments that do not parse never reach your function: the model gets a `ToolError::WrongArguments` back.

## Stateful tools

When a tool needs configuration, derive `Tool` on a struct:

::snippet{src="@swiftide/examples/release-notes-agent/src/tools.rs" anchor="write_changelog" title="examples/release-notes-agent/src/tools.rs"}

The derive calls the method named after the struct in snake case, here `write_changelog`. The model never chooses the file: it comes from the struct.

- The derive does not read doc comments. `description` is required, and each argument needs a `param`.
- A `param` without a type is a `String`, so the method takes `&str`.
- The struct must be `Clone`.

Register the tool with `add_tool(WriteChangelog::new(path))`, or pass `WriteChangelog::new(path).into()` to `tools`.

## Hand-written tools

Implement `Tool` yourself to write the tool spec by hand and parse the arguments in your own code. Describe the arguments as a type:

::snippet{src="@swiftide/examples/tool_custom_schema.rs" anchor="schema"}

Then return that schema from `tool_spec`:

::snippet{src="@swiftide/examples/tool_custom_schema.rs" anchor="tools"}

`invoke` receives the `ToolCall`, whose `args()` is the JSON string of the arguments. The agent drops duplicate keys and re-serializes the JSON first, so the string can differ from what the model sent. `name()` must match the `ToolSpec` name: the agent looks tools up by `name()`, the model calls the spec name.

`ToolSpec::builder().build()` returns a `Result`. It checks the schema against Swiftide's strict contract, so a `HashMap` field fails; see [Tool schema policies](/agents/providers-and-reasoning/#tool-schema-policies). The example calls `expect`, so an invalid schema panics at the first request. Cover it with a unit test that calls `tool_spec()`.

A hand-written tool can wrap another one. `OneEntryPerChange` in the same `tools.rs` delegates to `WriteChangelog` and rejects drafts that list a change twice.

## Tool output

A tool returns `Result<ToolOutput, ToolError>`. `ToolOutput` is non-exhaustive and has these variants:

- `Text(String)`: the result, added to the message history. `&str`, `String` and `CommandOutput` convert into it.
- `Fail(String)`: a failure the model can fix.
- `Stop(Option<Value>)`: stops the agent with `StopReason::RequestedByTool`. See [Stopping and structured results](/agents/stopping-and-structured-results/).
- `FeedbackRequired(Option<Value>)`: stops the agent with `StopReason::FeedbackRequired` and leaves the tool call open. See [Human in the loop](/agents/human-in-the-loop/).
- `AgentFailed(Option<Value>)`: stops the agent with `StopReason::AgentFailed`.

## Tool errors and retries

`ToolError` has four variants: `WrongArguments`, `MissingArguments`, `ExecutionFailed` and `Unknown`. It converts from `serde_json::Error`, `CommandError` and `anyhow::Error`, so `?` works.

::snippet{src="@snippets/src/agents/tools.rs" anchor="tool_errors"}

A bad crate name returns `Ok(ToolOutput::fail(..))`, and the model tries again. An executor error returns `Err`: the agent records it as a `Fail` output and counts the failure.

`tool_retry_limit` sets how many failures of the same call the agent feeds back to the model:

::snippet{src="@snippets/src/agents/tools.rs" anchor="retry_limit"}

With a limit of 2, the third identical failure stops the agent. The full rules for `tool_retry_limit` and `limit` are on [Stopping and structured results](/agents/stopping-and-structured-results/#stop-reasons).

## Attribute descriptions

Instead of doc comments you can pass `description` and `param` attributes. An explicit attribute wins over the doc comment:

::snippet{src="@swiftide/examples/aws_bedrock_agent.rs" anchor="tools"}

`description` also accepts a path to a `const`, such as `description = READ_FILE`.

## Gotchas

- The generated code refers to `::swiftide::` and `::serde` paths. The crate that uses the macros needs `swiftide` with the default `macros` feature flag and `serde` with `derive`.
- Every argument after the context must be listed under `# Arguments` or as a `param`. A doc bullet with an empty description is dropped, and a missing argument fails to compile with "parameters are missing from the spec".
- Tools are unique by name. `add_tool` replaces a tool with the same name, and `tools` replaces every tool added before it. To bring your own `stop`, see [Stopping and structured results](/agents/stopping-and-structured-results/).
- The tool calls of one completion run concurrently on separate Tokio tasks, so two calls to a tool that writes the same file can race.

## Reference

- [`#[swiftide::tool]`](docsrs:swiftide/attr.tool.html) and [`#[derive(Tool)]`](docsrs:swiftide/derive.Tool.html)
- [`Tool`](docsrs:swiftide/chat_completion/trait.Tool.html)
- [`ToolSpec`](docsrs:swiftide/chat_completion/struct.ToolSpec.html) and [`ToolSpecBuilder`](docsrs:swiftide/chat_completion/struct.ToolSpecBuilder.html)
- [`ToolCall`](docsrs:swiftide/chat_completion/struct.ToolCall.html)
- [`ToolOutput`](docsrs:swiftide/chat_completion/enum.ToolOutput.html)
- [`ToolError`](docsrs:swiftide/chat_completion/errors/enum.ToolError.html)
- [`AgentContext`](docsrs:swiftide/traits/trait.AgentContext.html)
- [`AgentBuilder::tool_retry_limit`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.tool_retry_limit)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)

## Next

[Context and message history](/agents/context-and-history/)
