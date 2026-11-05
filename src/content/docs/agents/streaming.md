---
title: "Streaming"
description: "Turn on streaming with on_stream, read the text and the tool call arguments in each stream chunk, and switch OpenAI-compatible clients to deltas only."
sidebar:
  order: 9
---

You end up with an agent that prints the model's text, and the arguments of its tool calls, while the model is still writing them. Streaming changes only how a completion arrives.

The agent needs the `swiftide-agents` feature flag, and each model client needs its own: `openai`, `anthropic` or `aws-bedrock`. None of them is on by default.

```sh
cargo add swiftide@{{swiftide_version}} --features swiftide-agents,openai
```

## Streaming switch

::snippet{src="@swiftide/examples/streaming_agents.rs" anchor="agent"}

`openai` is an `OpenAI` client built with `stream_full(false)`; [Deltas only](#deltas-only) shows it.

Registering `on_stream` turns streaming on. The agent then calls `complete_stream` on the model instead of `complete`, and runs the hook once for every stream chunk. Without a hook, `streaming(true)` on the builder streams as well.

The hook receives `&Agent` and the chunk, a `ChatCompletionResponse`. The example prints `delta.message_chunk`, the text that arrived with this chunk. It never flushes stdout, so the text can arrive in bursts.

The `on_new_message` hook prints every message the agent adds to the message history. For the assistant, that is the assembled message after the stream has ended.

## Stream chunks

Each stream chunk is a whole `ChatCompletionResponse`. With the default settings it carries:

- `id`, the same for every chunk of one completion.
- `message`, the text accumulated so far.
- `tool_calls`, `usage` and `reasoning`, the tool calls rebuilt, the token counts and the reasoning items so far.
- `delta.message_chunk`, the text that arrived with this chunk.
- `delta.tool_calls_chunk`, a map from a per-client index to a `ToolCallAccum` with `id`, `name` and `arguments`. `arguments` holds everything received so far, not only the newest fragment. The map stays on every later chunk of the completion, so remember what you already showed.

The key differs by client. OpenAI chat completions use the tool call's index, the Responses API the output item index. Anthropic and Bedrock use the content block index, which also counts text blocks. Match calls on `id` or `name`, not on the key.

When the stream ends, the agent keeps the last chunk and uses it as the completion. It records that chunk's message in the message history and runs its tool calls. A stream that yields nothing fails with `AgentError::EmptyStream`. A client without its own `complete_stream` sends one chunk with the whole response and no `delta`.

## Tool argument previews

The release-notes-agent showcase streams its draft. The release notes are the `markdown` argument of the `write_changelog` tool, so they arrive as tool call arguments, not as text:

::snippet{src="@swiftide/examples/release-notes-agent/src/progress.rs" anchor="stream" title="examples/release-notes-agent/src/progress.rs"}

The hook writes `message_chunk` to stdout. It then walks the entries of `tool_calls_chunk`, keeps the `write_changelog` call, and hands its id and accumulated arguments to `DraftPreview`. It flushes stdout after every chunk.

[`DraftPreview`](https://github.com/bosun-ai/swiftide/blob/e09a3e90d3eecdaffa08818a661efc3ecec08321/examples/release-notes-agent/src/draft.rs) lives in `draft.rs`. Every chunk repeats the whole argument text, so the preview tracks how much it has shown. It decodes the complete part of the `markdown` value in the growing JSON (JavaScript Object Notation) text, stopping before an unfinished escape sequence. It returns only the text it has not returned before. A new call id starts a fresh preview.

The hook is a `Fn` closure, so it cannot change what it captures, and it must be `Clone`. State that changes from chunk to chunk goes behind an `Arc<Mutex<_>>`, as `preview` does here.

## Captured stream

These lines come from the showcase's `OUTPUT.md`, a real run with `claude-sonnet-5-5` through an OpenAI-compatible gateway. `OUTPUT.md` cuts the draft after its first lines with `...`:

```text
  git_diff ✓ 17 lines
## 0.34.0

This release adds the opt-in `swiftide-agents-contrib` crate and Perl support for tree-sitter. It also fixes duplicated text in OpenAI streaming and resolves security advisories, which removes AWS Console login from the Bedrock client.
...
  → write_changelog <32 lines>
  write_changelog ⏸ waiting for approval
```

The draft appears before the `→ write_changelog` line. `on_stream` runs while the completion arrives, and the `before_tool` hook runs only after it is complete. After the approval, the agent runs the pending tool call without a new completion, so the draft streams once.

## Deltas only

By default, the OpenAI-compatible clients clone the accumulated response, which grows with the answer, into every chunk. Turn that off with `stream_full(false)`:

::snippet{src="@swiftide/examples/streaming_agents.rs" anchor="setup"}

The last chunk is still the complete response, so the agent behaves the same. `delta.tool_calls_chunk` carries the same accumulated arguments with either setting, so the showcase's preview works with the default too. [Deltas only](/models/overview/#deltas-only) lists the clients that have the setting and the fields each chunk drops.

## Gotchas

- In 0.33.0, the `GenericOpenAI` chat completions stream does not clear `message_chunk` on a chunk without new text. That covers OpenAI, Ollama, Groq, Gemini, OpenRouter and Dashscope. Tool call fragments, the stop and usage frames, and the final chunk repeat the previous text, so a hook that prints `message_chunk` prints it more than once. The fix ([#1216](https://github.com/bosun-ai/swiftide/pull/1216)) is on master. :unreleased On 0.33.0 the first example on this page shows the repeats too.
- The fix covers only the chat completions stream. The OpenAI Responses API (`use_responses_api(true)`, which the showcase's `--responses-api` flag turns on), `Anthropic` and `AwsBedrock` still keep the previous `message_chunk` on tool call events, even on master. With them, both examples print the last text again for every tool call event. Their chunks carry the accumulated `message`, so printing the part of `message` beyond what you already printed avoids repeats.
- The agent awaits the hook for every chunk before it reads the next one. A slow hook slows the stream. Hand slow work to a channel or a spawned task.
- An error returned from `on_stream` is logged with `tracing` and does not stop the agent.
- `on_stream` receives `&Agent`, so it cannot stop the agent or change its tools. Do that from `after_completion`.
- Rust's stdout is line buffered. `print!` without a newline may show nothing until a line ends or the buffer fills, so flush after each chunk. The first example does not flush; the showcase does.

## Reference

- [`AgentBuilder::on_stream`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.on_stream)
- [`OnStreamFn`](docsrs:swiftide/agents/hooks/trait.OnStreamFn.html)
- [`ChatCompletionResponse`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponse.html)
- [`ChatCompletionResponseDelta`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponseDelta.html)
- [`ToolCallAccum`](docsrs:swiftide/chat_completion/struct.ToolCallAccum.html)
- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`GenericOpenAIBuilder::stream_full`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html#method.stream_full)
- [`AgentError`](docsrs:swiftide/agents/errors/enum.AgentError.html)
- [Feature flags](/reference/feature-flags/)

## Next

[MCP tools](/agents/mcp/)
