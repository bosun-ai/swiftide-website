---
title: "Testing agents"
description: "Test Swiftide agents without a provider by scripting the model, calling tools directly, or matching exact requests with MockChatCompletion."
sidebar:
  order: 13
identLint:
  ignore: [ScriptedLlm, ScriptedReviewer, count_words, reply_calling, CountWords, CountWordsArgs, deny_unknown_fields]
---

You can test an agent end to end without provider credentials or network access: a scripted model decides which tools to call, and your test asserts on the tool outputs, the requests and the stop reason.

`Agent`, `DefaultContext` and `StopReason` sit behind the `swiftide-agents` feature flag, which is off by default. The tests also need tokio for its test macros, serde_json and async-trait:

```sh
cargo add swiftide@{{swiftide_version}} --features swiftide-agents
cargo add --dev tokio --features macros,rt-multi-thread
cargo add --dev serde_json async-trait anyhow
```

Swiftide re-exports serde_json, async_trait and anyhow under `swiftide::reexports`, so you can use those paths instead of adding the crates.

## Scripted models

::snippet{src="@snippets/src/agents/testing.rs" anchor="scripted_llm"}

An agent talks to its model through the [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html) trait. Only `complete` is required. The default `complete_stream` wraps `complete` in a stream of one response, so a scripted model also works for an agent with `on_stream` hooks.

`ScriptedLlm` hands out one prepared [`ChatCompletionResponse`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponse.html) per completion and records the messages of every request. When the script runs out, it returns a permanent `LanguageModelError`, and `query` fails with `AgentError::CompletionsFailed`.

The state sits behind `Arc<Mutex<..>>` because `AgentBuilder::llm` takes a reference and stores a boxed clone. Your test keeps the original, and both share the same queue and request log.

`reply_calling` builds a response that calls one tool. The arguments are a JSON (JavaScript Object Notation) string, as a provider would send them.

## Agent tests

The tool under test is a plain function tool:

::snippet{src="@snippets/src/agents/testing.rs" anchor="count_words"}

The test scripts two turns: the model calls `count_words`, then the `stop` tool.

::snippet{src="@snippets/src/agents/testing.rs" anchor="agent_test"}

The agent runs the real tool, records its output in the message history and sends the history back to the model. `stop` ends the run with `StopReason::RequestedByTool` and no payload. The test then checks three things:

- the `ChatMessage::ToolOutput` for `count_words` holds `3`
- the stop reason names the `stop` tool
- the second request ended with the tool output, so the model saw the result

Because the agent has no system prompt by default, the first request holds only the user message. To test a system prompt, set one on the builder and assert on the first `ChatMessage::System` in `llm.requests()`.

A reply without tool calls, such as `ChatCompletionResponse::builder().message("Done.").build()`, scripts a turn where the model only answers in text. The default context stops the agent after such a turn, with `StopReason::NoNewMessages`.

## Direct tool invocation

Most tool logic needs no agent at all. Call `invoke` on the tool with a context and a [`ToolCall`](docsrs:swiftide/chat_completion/struct.ToolCall.html):

::snippet{src="@snippets/src/agents/testing.rs" anchor="invoke_tool"}

[`DefaultContext::default()`](docsrs:swiftide/agents/struct.DefaultContext.html) runs commands through a `LocalExecutor` in the current directory, so tools that call `context.executor()` work too. Point it at a temporary directory with `DefaultContext::from_executor` when the tool touches files.

Arguments that do not deserialize into the generated arguments struct return `ToolError::WrongArguments`. In the test above, the misspelled `txt` key leaves the required `text` argument missing. A call without arguments returns `ToolError::MissingArguments`. Extra keys are ignored: `deny_unknown_fields` only applies to the JSON schema the model sees. A misspelled key for an `Option` argument becomes `None` without an error. Inside an agent, both go back to the model as a failed tool output and count toward `tool_retry_limit`.

## MockChatCompletion

For tests that pin down the exact requests, swiftide-core ships `MockChatCompletion` behind its `test-utils` feature flag. The swiftide crate has a `test-utils` feature too, but it does not re-export the module, so add swiftide-core as a dev dependency and import `swiftide_core::test_utils::MockChatCompletion`:

```sh
cargo add --dev swiftide-core@{{swiftide_version}} --features test-utils
```

::snippet{src="@snippets/src/agents/testing.rs" anchor="mock_chat_completion"}

Each `expect_complete` queues an expected [`ChatCompletionRequest`](docsrs:swiftide/chat_completion/struct.ChatCompletionRequest.html) and the response to return. Expectations are consumed in the order you add them. A request that differs from the next expectation fails the test with both requests printed. An `Err` response reaches the agent as a permanent `LanguageModelError`.

Request equality covers the messages and the full set of tool specs. The agent sends the spec of every tool it has, including the default `stop` tool, so the expected request lists `Agent::default_tools()` next to your own tools.

When the last clone of the mock is dropped with expectations left over, it panics. A test that stops early fails even when every request it made matched.

## Showcase tests

The release notes showcase has 27 tests. From the swiftide repository root:

```sh
cargo test -p release-notes-agent
```

The session tests exercise the approval loop in `session.rs`:

::snippet{src="@swiftide/examples/release-notes-agent/src/session.rs" anchor="approval" title="examples/release-notes-agent/src/session.rs"}

Five of the six session tests drive this loop with a scripted model like the one above. The showcase's own `ScriptedLlm`, a simpler variant of the one above, takes `(tool name, arguments)` pairs, and an empty name scripts a turn without a tool call. A `ScriptedReviewer` plays the person who approves or asks for a revision, and those tests run against a temporary repository. They cover:

- a revision round, then approval and the write
- stopping without a write, which is an error
- one nudge after a turn without a tool call, and an error after the second one
- a draft that lists a pull request twice, which goes back to the model before review

The duplicate check also ran against a live model. The other paths ran only in these tests.

The sixth session test is a synchronous unit test. It builds a history by hand and checks that reaching the step limit is not mistaken for a turn without a tool call, although both report the same stop reason.

The remaining tests are plain unit tests in the other modules. They cover system prompt rendering, decoding a streamed draft however it is chunked, and restarting the draft preview for a new tool call. They also cover finding a pull request listed in two entries, the stop tool schema, rendering the notes, tool names, clipping tool output, shell quoting, GitHub remote parsing, and rejecting unsafe revisions and paths.

## Gotchas

- `swiftide_agents::test_utils` (`MockTool`, `MockHook`, `chat_request!`) is compiled only in test and debug builds, and its macros expand to `swiftide_core` paths. It serves swiftide's own tests; do not build on it.
- `MockChatCompletion::complete_stream` splits the response text on whitespace inside a spawned task. On a response without text that task panics, the stream closes empty, and `query` returns `AgentError::EmptyStream`. A streamed reply that only calls tools never delivers its tool calls. Script a message on every reply when the agent streams.
- The agent rewrites JSON object arguments before it stores the assistant message, to drop duplicate keys. Build expected arguments with `serde_json`, as above, rather than as hand-typed strings with their own spacing.
- The unit context `()` implements `AgentContext` for tools that never run commands. Its `executor()` panics, and it approves every feedback request.
- A tool call that names a tool the agent does not have is skipped with a warning and gets no output. With a typo in a scripted tool name, the run ends with `StopReason::NoNewMessages` instead of the scripted stop. The next `query` or `run` on that agent fails with `AgentError::UnfulfilledToolCalls`.

## Reference

- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`ChatCompletionRequest`](docsrs:swiftide/chat_completion/struct.ChatCompletionRequest.html)
- [`ChatCompletionResponse`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponse.html)
- [`ChatMessage`](docsrs:swiftide/chat_completion/enum.ChatMessage.html)
- [`ToolCall`](docsrs:swiftide/chat_completion/struct.ToolCall.html)
- [`ToolError`](docsrs:swiftide/chat_completion/errors/enum.ToolError.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`DefaultContext`](docsrs:swiftide/agents/struct.DefaultContext.html)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`MockChatCompletion`](docsrs:swiftide_core/test_utils/struct.MockChatCompletion.html)
- [`AgentError`](docsrs:swiftide/agents/errors/enum.AgentError.html)

## Next

[How tasks work](/tasks/overview/)
