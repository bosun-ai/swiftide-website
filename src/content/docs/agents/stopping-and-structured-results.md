---
title: "Stopping and structured results"
description: "Replace the stop tool with StopWithArgs and a JSON schema to get a typed result, report failures with AgentCanFail, cap runs with limit and tool_retry_limit, and handle every StopReason."
sidebar:
  order: 7
identLint:
  ignore: [ReleaseNotes, FailureReport, JsonSchema, task_failed, deny_unknown_fields, Value, "Value::String", non_exhaustive, stop_tool]
---

Your agent ends its run by handing your code a typed value, such as `ReleaseNotes`, or a typed report of why it failed. Your code can also tell why any other run ended.

`Agent::builder()` starts with one tool, named `stop`, which takes no arguments and ends the run with `StopReason::RequestedByTool(tool_call, None)`. With the default context, an answer without a tool call also ends the run, with `StopReason::NoNewMessages`.

## Structured results

The examples need the `swiftide-agents` feature flag and your own `schemars` 1.x, `serde` and `serde_json`:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,swiftide-agents
cargo add schemars@1 serde serde_json --features serde/derive
```

The [release-notes-agent showcase](/showcases/release-notes-agent/) returns its notes as data. It describes them as a type and turns the type's JSON (JavaScript Object Notation) Schema into a stop tool:

::snippet{src="@swiftide/examples/release-notes-agent/src/notes.rs" anchor="stop_schema" title="examples/release-notes-agent/src/notes.rs"}

`StopWithArgs::with_parameters_schema` builds a tool that is also named `stop`. The model calls it with arguments that follow the schema. The doc comments on the fields end up in the schema, so the model reads them as instructions. `deny_unknown_fields` gives both objects `additionalProperties: false`.

With a custom schema, the tool passes the whole argument object on as the payload. The agent stops with `StopReason::RequestedByTool(tool_call, Some(payload))`, where `payload` is a `serde_json::Value`. Match that arm and turn the payload into your type with `ReleaseNotes::deserialize(payload)` (a `&serde_json::Value` is a deserializer) or `serde_json::from_value(payload.clone())`. [Human in the loop](/agents/human-in-the-loop/) shows the whole loop.

`StopWithArgs::default()` is the other form. Its schema has a single `output` string, and the payload is that string.

`tools` keeps the default `stop`, so call `without_default_stop_tool` before `tools`, or add yours with `add_tool`. In this example, `stop_tool` is `StopWithArgs::with_parameters_schema(stop_schema())`:

::snippet{src="@swiftide/examples/stop_with_args_custom_schema.rs" anchor="agent"}

The `on_stop` hook receives the same `StopReason`, so you can read the payload there instead of after `query` returns.

The showcase's output with `claude-sonnet-5-5`, through an OpenAI-compatible gateway, ends with the stop call and the deserialized notes:

```text
  → stop (version 0.34.0: highlights 2, breaking 1, fixes 1)
  stop ■ done

✔ RELEASE_NOTES.md

0.34.0 — This release adds the opt-in `swiftide-agents-contrib` crate and Perl support for tree-sitter. It also fixes duplicated text in OpenAI streaming and resolves security advisories, which removes AWS Console login from the Bedrock client.
  highlights (2)
    • Agent tools, hooks and executors in `swiftide-agents-contrib` [#1207]
    • Perl support for tree-sitter [#1198]
  breaking changes (1)
    • AWS Console login removed from the Bedrock client [#1215]
  fixes (1)
    • Repeated text in OpenAI streaming [#1216]
```

## Failure reports

`AgentCanFail` is a second control tool, named `task_failed`. The agent does not add it unless you pass it to `tools`. `AgentCanFail::default()` takes a `reason` string, and the agent stops with `StopReason::AgentFailed(Some(reason))`. `query` still returns `Ok`, so you check the stop reason.

`AgentCanFail::with_parameters_schema` accepts your own schema:

::snippet{src="@snippets/src/agents/stopping.rs" anchor="failure_payload"}

With a custom schema the tool serializes the arguments back to text. The payload is a `Value::String` that holds JSON, so you parse it a second time with `serde_json::from_str`.

## Stop reasons

This run caps itself at 20 completions, lets one tool call fail twice, adds `task_failed`, and stops itself from a hook when the message history grows too long:

::snippet{src="@snippets/src/agents/stopping.rs" anchor="stop_reasons"}

`limit` caps the number of completions in one run. The counter starts at zero with every `query` or `run`. When it is reached, the agent logs a warning and stops with `StopReason::NoNewMessages`, and `query` returns `Ok`. No limit is set by default.

`tool_retry_limit` caps how often the same tool call may fail before the agent stops with `StopReason::ToolCallsOverLimit` and `query` returns `AgentError::ToolError`. [Tools](/agents/tools/) explains how failures are counted.

Each variant means something different for your code:

- `RequestedByTool(tool_call, payload)`: a tool returned `ToolOutput::Stop`, usually a `stop` tool. `payload` is `None` for the default tool, the `output` value for `StopWithArgs::default()`, and the argument object for a custom schema. Treat it as success.
- `AgentFailed(payload)`: a tool returned `ToolOutput::AgentFailed`, usually `task_failed`. Show the reason to a person, or retry with more context.
- `ToolCallsOverLimit(tool_call)`: one call failed too often. `query` also returns the error. Fix the tool or its description, or raise `tool_retry_limit`.
- `FeedbackRequired { tool_call, payload }`: a tool waits for approval. Record a `ToolFeedback` on the context and call `run`, as [Human in the loop](/agents/human-in-the-loop/) shows.
- `Error`: a step failed, for example a completion request, and `query` returns the `AgentError`.
- `NoNewMessages`: the model answered without a tool call, sent an empty reply, or the run reached `limit`. `query_once` and `run_once` also end with it after one step, unless a tool stopped the agent first. A new `query` continues the same message history.
- `Other(text)`: your code stopped the agent. `agent.stop("text")` accepts any string. Hooks that receive `&mut Agent`, such as `after_each` and `after_tool`, can call it.

`StopReason` is marked `#[non_exhaustive]`, so a `match` needs a wildcard arm. `agent.stop_reason()` returns `None` while the agent is `Pending` or `Running`. That includes a run that returned an error without recording a stop. Each variant also has an `is_*` method, and most have an `as_*` accessor such as `as_requested_by_tool`.

## Gotchas

- To tell the cases of `NoNewMessages` apart, check whether the last assistant message has tool calls, or turn off the text-answer stop with `with_stop_on_assistant(false)` (see [Context and history](/agents/context-and-history/)).
- `without_default_stop_tool` takes effect when you call `tools`, so call it first.
- A `StopWithArgs` or `AgentCanFail` call whose arguments are not valid JSON is a tool error. It goes back to the model and counts toward `tool_retry_limit`. The default `Stop` ignores its arguments.
- `to_string_pretty` on an `AgentCanFail` payload prints a quoted, escaped string; parse it with `serde_json::from_str` first.
- Once the agent has stopped, `Agent::stop` does nothing. The first stop reason stays, and `on_stop` fires once. Tools the model called in the same completion as `stop` still run.
- The payload is whatever the model sent. Handle a failed deserialization, as the showcase does.

## Reference

- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`AgentBuilder`](docsrs:swiftide/agents/struct.AgentBuilder.html)
- [`DefaultContext`](docsrs:swiftide/agents/struct.DefaultContext.html)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`State`](docsrs:swiftide/agents/enum.State.html)
- [`Stop`](docsrs:swiftide/agents/tools/control/struct.Stop.html)
- [`StopWithArgs`](docsrs:swiftide/agents/tools/control/struct.StopWithArgs.html)
- [`AgentCanFail`](docsrs:swiftide/agents/tools/control/struct.AgentCanFail.html)
- [`ToolCall`](docsrs:swiftide/chat_completion/struct.ToolCall.html)
- [`ToolOutput`](docsrs:swiftide/chat_completion/enum.ToolOutput.html)
- [`ToolFeedback`](docsrs:swiftide/traits/enum.ToolFeedback.html)
- [`AgentError`](docsrs:swiftide/agents/errors/enum.AgentError.html)

## Next

[Human in the loop](/agents/human-in-the-loop/)
