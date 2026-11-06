---
title: "Lifecycle hooks"
description: "Register agent hooks that log tool calls, change completion requests, rewrite tool output and report why an agent stopped."
sidebar:
  order: 5
identLint:
  ignore: ["Box::pin", "mpsc::Receiver"]
---

You end up with hooks that print each tool call, change a completion request before it goes out, trim tool output before the model reads it, and report why an agent stopped.

A hook is a closure that the agent calls at a fixed point in its loop. The closure returns a boxed future that the agent awaits. Hooks need the `swiftide-agents` feature flag. The release-notes showcase prints one line per tool call and one per result:

::snippet{src="@swiftide/examples/release-notes-agent/src/progress.rs" anchor="hooks" title="examples/release-notes-agent/src/progress.rs"}

## Hook registration

You pass a closure to an `AgentBuilder` method such as `before_tool`. The closure returns `Box::pin(async { Ok(()) })`, which resolves to `anyhow::Result<()>`. An `async` closure does not satisfy the hook traits.

The closure must be `Clone`, `Send` and `Sync`. Share state through an `Arc`. Capturing a value that is not `Clone`, such as an `mpsc::Receiver`, fails to compile.

Hooks of one kind run in the order you register them. `add_hook` appends a `Hook` value, and `hooks` replaces all earlier hooks.

This is the start of a real run on the Swiftide repository, with `claude-sonnet-5-5` behind an OpenAI-compatible gateway:

```text
Drafting release notes for the 7 commits since v0.33.0 with claude-sonnet-5-5 at http://127.0.0.1:8080/v1 (chat completions)
  → git_log_since v0.33.0
  git_log_since ✓ 346 lines
  → git_diff swiftide-integrations/Cargo.toml 3b1ad22e^..3b1ad22e
  → git_diff …/src/chat_completion/chat_completion_response.rs 90bcadc7^..90bcadc7
  → git_diff swiftide-agents-contrib/Cargo.toml 71fa4587^..71fa4587
  → git_diff Cargo.toml 71fa4587^..71fa4587
  → git_diff swiftide-integrations/src/aws_bedrock_v2/mod.rs 3b1ad22e^..3b1ad22e
  → git_diff swiftide/Cargo.toml 71fa4587^..71fa4587
  git_diff ✓ 15 lines
  git_diff ✓ 72 lines
  git_diff ✓ 75 lines
  git_diff ✓ No changes to Cargo.toml in 71fa4587^..71fa4587.
  git_diff ✓ 18 lines
  git_diff ✓ 32 lines
```

The `→` lines come from `before_tool` and the `✓` lines from `after_tool`. The agent runs `before_tool` for a call and then starts that call, so the tool calls of one completion run concurrently. An earlier tool can already be running while `before_tool` runs for a later one. It then waits for the results and runs `after_tool` in call order.

## Hook points

| Builder method | When it runs | What it receives |
| --- | --- | --- |
| `before_all` | Once, on the first run, before the system prompt renders and toolboxes load | `&mut Agent` |
| `on_start` | On every `query` or `run`, before its first completion | `&mut Agent` |
| `before_completion` | Before each completion request | `&mut Agent`, `&mut ChatCompletionRequest` |
| `on_stream` | For each stream chunk, when the agent streams | `&Agent`, `&ChatCompletionResponse` |
| `after_completion` | After each completion, before the agent acts on it | `&mut Agent`, `&mut ChatCompletionResponse` |
| `on_new_message` | When `Agent::add_message` adds a message | `&Agent`, `&mut ChatMessage` |
| `before_tool` | Before each tool call starts | `&mut Agent`, `&ToolCall` |
| `after_tool` | After each tool call, before its output is recorded | `&mut Agent`, `&ToolCall`, `&mut Result<ToolOutput, ToolError>` |
| `after_each` | After a completion and its tool calls | `&mut Agent` |
| `on_stop` | When the agent records a stop reason | `&mut Agent`, `StopReason`, `Option<&AgentError>` |

Since 0.33, most hooks receive `&mut Agent`, so they can stop the agent, read its history or change its tools:

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="hook_mut_agent"}

Only the `on_stream` builder method turns streaming on. A `Hook::OnStream` passed to `add_hook` or `hooks` never fires unless streaming is on. [Streaming](/agents/streaming/) covers stream chunks.

## Completion request and response edits

::snippet{src="@snippets/src/agents/hooks.rs" anchor="before_completion"}

A change to `messages` or `tools_spec` applies to this one request. The reminder reaches the model but never enters the message history. Removing an entry from `tools_spec` hides a tool for one turn.

In `after_completion`, changing `response.message` or `response.tool_calls` changes what the agent records and which tools it runs. `response.usage` is `None` when the provider reports no usage.

## Tool output rewrites

::snippet{src="@snippets/src/agents/hooks.rs" anchor="after_tool_rewrite"}

Whatever you leave in the result is what the agent records in the message history and its tracing spans. The hook runs before the agent looks at errors. Replacing an `Err` with `Ok(ToolOutput::fail(..))` still tells the model what went wrong, but no longer counts toward `tool_retry_limit`. [Tool errors and retries](/agents/tools/#tool-errors-and-retries) explains the retry limit.

## Stop notifications

::snippet{src="@snippets/src/agents/hooks.rs" anchor="on_stop"}

`on_stop` runs once per stop and before the agent records its new state, so read the reason from the argument. [Stopping and structured results](/agents/stopping-and-structured-results/) lists every stop reason.

`on_stop` is not a failure signal. The error is `Some` only when a completion step fails. Errors before the loop, such as a failed toolbox load, skip `on_stop`. A tool over `tool_retry_limit` fires `on_stop` with `StopReason::ToolCallsOverLimit` and no error, yet `query` returns `Err`. Check the result of `query` for failures.

## Hooks from methods

::snippet{src="@snippets/src/agents/hooks.rs" anchor="hook_from_method"}

Builder methods only accept `'static` hooks. In Rust 2024, `fn hook(&self) -> impl AfterToolFn` captures the lifetime of `&self`. Clone the `Arc` into the closure and add `use<>` to the return type.

## Gotchas

- The future may only borrow the agent. Touch the request, response, tool call or message before `Box::pin`. The `after_tool` result is the exception.
- A hook that returns `Err` is logged with `tracing::error!` and the agent carries on.
- `agent.stop(..)` lets the current step finish. A stop in `before_completion` still sends that request, and one in `before_tool` still runs the tool. In `before_all` and `on_start`, the agent fires `on_stop` but runs the loop anyway.
- In `before_all`, `agent.tools()` does not yet hold Model Context Protocol (MCP) toolbox tools. It can fill template values through `system_prompt_mut`, as [System prompts](/agents/system-prompts/) shows.
- `on_new_message` skips the system prompt and user queries, because the agent writes those to the context directly.
- A resumed tool call that waited for approval runs again, so a counter like the one above counts it twice. Its tool hooks run before `on_start` while the agent is still stopped, so a stop from them is ignored. [Human in the loop](/agents/human-in-the-loop/) covers approvals.
- A call to a tool the agent does not have is skipped, and no tool hook runs for it.

## Reference

- [`hooks` module](docsrs:swiftide/agents/hooks/index.html)
- [`Hook`](docsrs:swiftide/agents/hooks/enum.Hook.html)
- [`AfterToolFn`](docsrs:swiftide/agents/hooks/trait.AfterToolFn.html)
- [`AgentBuilder`](docsrs:swiftide/agents/struct.AgentBuilder.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`ChatCompletionRequest`](docsrs:swiftide/chat_completion/struct.ChatCompletionRequest.html)
- [`ChatCompletionResponse`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponse.html)
- [`ChatMessage`](docsrs:swiftide/chat_completion/enum.ChatMessage.html)
- [`ToolCall`](docsrs:swiftide/chat_completion/struct.ToolCall.html)
- [`ToolOutput`](docsrs:swiftide/chat_completion/enum.ToolOutput.html)
- [`ToolError`](docsrs:swiftide/chat_completion/errors/enum.ToolError.html)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`AgentError`](docsrs:swiftide/agents/errors/enum.AgentError.html)
- [Feature flags](/reference/feature-flags/)

## Next

[System prompts](/agents/system-prompts/)
