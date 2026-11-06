---
title: "Anthropic"
description: "Build an Anthropic client for Claude prompts, chat completions, streaming and agents with tools; it has no embeddings or structured output."
sidebar:
  order: 4
identLint:
  ignore: ["backoff::ExponentialBackoff"]
---

You end up with an `Anthropic` client that sends prompts and chat completions to Claude, streams responses and runs agents with tools. Enable the `anthropic` feature flag for the client and `swiftide-agents` for the agent; the `all` bundle leaves both out:

```sh
cargo add swiftide@{{swiftide_version}} --features anthropic,swiftide-agents
cargo add tokio --features full
cargo add anyhow futures-util
```

The snippets use `tokio` and `anyhow` around the calls. The streaming snippet needs `futures_util::StreamExt` for `stream.next()`, which Swiftide does not re-export.

## Client

::snippet{src="@snippets/src/integrations/anthropic.rs" anchor="anthropic_client"}

`Anthropic::builder()` returns an `AnthropicBuilder`. `default_prompt_model` sets the Claude model for every call. `build()` creates the underlying `async_anthropic::Client`, which reads your key from the `ANTHROPIC_API_KEY` environment variable. A missing key only logs a warning. The first request then fails as unauthorized.

The client implements `SimplePrompt` and `ChatCompletion`. You can pass it to indexing transformers that prompt a large language model (LLM), to query pipelines and to agents.

`on_usage` receives the token counts of each call. On `prompt` and `complete` the client awaits it, and an `Err` from the callback fails the call. Use `on_usage_async` for a callback that awaits. [Usage callbacks](/models/errors-backoff-and-usage/#usage-callbacks) compares this with the other clients.

Swiftide builds the HTTP (Hypertext Transfer Protocol) client with rustls, even when another crate turns on native TLS (Transport Layer Security) for reqwest. It uses the rustls crypto provider your application installed, or else ring, so it needs no setup; see [TLS](/integrations/overview/#tls).

## Models

Always set `default_prompt_model`. Without it the client asks for `claude-3-5-sonnet-20241022`, which Anthropic has retired, so every call fails.

`Options` holds only `prompt_model`. The client sends no temperature, stop sequences or thinking settings. Every request asks for at most 2048 output tokens, the default of `async-anthropic` 0.6, and Swiftide has no setting to raise it. A longer answer is cut off, and `ChatCompletionResponse` has no field that tells you it was truncated.

`async-anthropic` 0.6 parses only text, tool use and tool result content blocks. A response that holds a thinking block fails to parse. If your model returns thinking blocks without being asked, every call fails. The snippet uses `claude-sonnet-4-6`, which does not.

## Agents with tools

Define a tool with the `#[swiftide::tool]` macro:

::snippet{src="@snippets/src/integrations/anthropic.rs" anchor="anthropic_tool"}

Then hand the client to the agent:

::snippet{src="@snippets/src/integrations/anthropic.rs" anchor="anthropic_agent"}

`AgentBuilder::llm` clones the client into the agent. Each request carries the tool specs as Anthropic tools with the tool choice set to auto, so the model decides when to call one. The default `stop` tool comes along with your own tools.

The client turns the message history into Anthropic messages:

- The first system message becomes the request's `system` field. Later system messages and summaries go out as user messages.
- Tool outputs that follow each other go out together in one user message. Swiftide groups them since 0.33.
- A tool output without content goes out as `Success`.
- `ChatMessage::Reasoning` entries are left out.
- A message with content parts may hold text parts only. Any other part, such as an image, returns an error. See [Images and multimodal input](/models/images/).

## Streaming

::snippet{src="@snippets/src/integrations/anthropic.rs" anchor="anthropic_stream"}

`complete_stream` returns a `ChatCompletionStream`. Every item is the whole response so far, and the last item is the complete response with `usage` set.

Treat streamed `usage` as an estimate. The client adds up the counts from the message start and message delta events. Anthropic reports cumulative counts in the message delta event, so the totals can come out higher than the non-streaming counts for the same request. The client has no `stream_full` setting, so you always receive the accumulated response.

`delta.message_chunk` holds the latest text, but it keeps that value on events that carry no text. Print the part of `message` you have not printed yet, as the snippet does, rather than printing every delta.

On streams the client spawns the usage callback and only logs an `Err` from it. An agent streams when you add an `on_stream` hook.

## Limitations

- No `EmbeddingModel`. Embed with [FastEmbed](/integrations/fastembed/) or [OpenAI](/integrations/openai/#embeddings) and keep Anthropic for prompts and agents.
- No `StructuredPrompt`. See [Structured output](/models/structured-output/) for clients that have it.
- No `SparseEmbeddingModel`.

To reach Claude through AWS (Amazon Web Services) with structured output and reasoning, use [AWS Bedrock](/integrations/aws-bedrock/).

## Errors and retries

`async-anthropic` retries a request that gets HTTP 429 (rate limit) or 529 (overloaded) by itself. It waits 15 seconds first, doubles the wait and gives up after 120 seconds. Streaming requests get no such retry.

After that Swiftide marks the error permanent, so `LanguageModelWithBackOff` does not retry it again. On `prompt`, a network error or a response body that fails to parse comes back as a `TransientError`. `async-anthropic` reports both as a network error. A response with a thinking block therefore gets retried by `LanguageModelWithBackOff`, and it fails the same way each time. On `complete` and `complete_stream` every error is permanent. See [Backoff](/models/errors-backoff-and-usage/#backoff).

## Custom client

`AnthropicBuilder::client` takes your own `async_anthropic::Client`. Swiftide does not re-export the crate, so add it at the version Swiftide uses:

```sh
cargo add async-anthropic@0.6 --no-default-features
cargo add backoff@0.4
```

Start from `async_anthropic::Client::from_api_key` to pass a key from your own configuration. Call `async_anthropic::Client::with_backoff` on it to change the retry timing. It takes a `backoff::ExponentialBackoff`, and `async-anthropic` does not re-export the `backoff` crate, hence the second line. `async_anthropic::Client::builder()` leaves the base URL and API version empty, so prefer those two methods. The client does not read a base URL from the environment.

## Reference

- [`Anthropic`](docsrs:swiftide/integrations/anthropic/struct.Anthropic.html)
- [`AnthropicBuilder`](docsrs:swiftide/integrations/anthropic/struct.AnthropicBuilder.html)
- [`AnthropicBuilder::client`](docsrs:swiftide/integrations/anthropic/struct.AnthropicBuilder.html#method.client)
- [`AnthropicBuilder::default_prompt_model`](docsrs:swiftide/integrations/anthropic/struct.AnthropicBuilder.html#method.default_prompt_model)
- [`AnthropicBuilder::on_usage`](docsrs:swiftide/integrations/anthropic/struct.AnthropicBuilder.html#method.on_usage)
- [`AnthropicBuilder::on_usage_async`](docsrs:swiftide/integrations/anthropic/struct.AnthropicBuilder.html#method.on_usage_async)
- [`Options` (Anthropic)](docsrs:swiftide/integrations/anthropic/struct.Options.html)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`ChatCompletionStream`](docsrs:swiftide/traits/type.ChatCompletionStream.html)
- [`ChatCompletionRequest`](docsrs:swiftide/chat_completion/struct.ChatCompletionRequest.html)
- [`ChatCompletionResponse`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponse.html)
- [`ChatCompletionResponseDelta`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponseDelta.html)
- [`ChatMessage`](docsrs:swiftide/chat_completion/enum.ChatMessage.html)
- [`Usage`](docsrs:swiftide/chat_completion/struct.Usage.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`AgentBuilder::llm`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.llm)
- [`tool` macro](docsrs:swiftide/attr.tool.html)
- [`ToolOutput`](docsrs:swiftide/chat_completion/enum.ToolOutput.html)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)

## Next

[AWS Bedrock](/integrations/aws-bedrock/)
