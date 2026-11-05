---
title: "OpenAI Responses API and reasoning"
description: "Switch the OpenAI client to the Responses API for prompts, chat, streaming and tool calls, and print reasoning summaries from an agent."
sidebar:
  order: 2
identLint:
  ignore: [EchoToolArgs]
---

This recipe gives you an OpenAI client that sends every kind of request through the OpenAI Responses application programming interface (API): prompts, structured prompts, chat completions, streams and tool calls. A second recipe gives you an agent that prints the reasoning summary of a reasoning model. Both recipes are runnable examples in the swiftide repository: `responses_api.rs` and `responses_api_reasoning.rs`.

## Setup

Add Swiftide with the `openai` and `swiftide-agents` feature flags. The first example also streams, defines a tool argument type and parses tool arguments, so it needs `futures-util`, `schemars`, `serde` and `serde_json`.

```sh
cargo add swiftide@{{swiftide_version}} --features openai,swiftide-agents
cargo add futures-util schemars@1 serde_json
cargo add serde --features derive
cargo add tokio --features full
cargo add anyhow
```

The examples are on Rust edition 2024. The `agent` snippet below uses a let-chain, which does not compile on edition 2021.

Set `OPENAI_API_KEY` and run the examples from the root of the swiftide repository:

```sh
export OPENAI_API_KEY=sk-...
cargo run -p swiftide-examples --example responses-api
cargo run -p swiftide-examples --example responses-api-reasoning
```

Swiftide does not pick a Transport Layer Security (TLS) crypto provider for its Hypertext Transfer Protocol (HTTP) clients. In your own binary, install a rustls provider at the start of `main`, as described in [Crypto provider panic](/reference/troubleshooting-and-faq/#crypto-provider-panic).

## Imports and types

The anchored excerpts from `responses_api.rs` leave out its imports and its two argument types. Here they are:

::snippet{src="@snippets/src/cookbook/responses_api.rs" anchor="types"}

`ChatCompletion` brings `complete` and `complete_stream` into scope. `StreamExt` gives you `next()` on the stream. `anyhow::Context` gives you `.context(...)` in the tool call code.

## Client

::snippet{src="@swiftide/examples/responses_api.rs" anchor="setup" title="examples/responses_api.rs"}

`use_responses_api(true)` is the only change from a Chat Completions client. The flag defaults to false. With it set, `complete`, `complete_stream`, `prompt` and `structured_prompt` all go through the Responses API. Embeddings still use the embeddings endpoint.

`default_options` takes anything that converts into `Options`, so you can pass an `OptionsBuilder` without calling `build()`. On the Responses path, the client drops some options instead of sending them. It logs a warning for `seed` and `presence_penalty`. It drops `user`, also set by `for_end_user`, without a warning. It skips `metadata` that is not a flat object, with a warning.

## Prompts

::snippet{src="@swiftide/examples/responses_api.rs" anchor="prompt" title="examples/responses_api.rs"}

`prompt` returns the model's text. `structured_prompt` returns a `WeatherSummary`, built from the JavaScript Object Notation (JSON) Schema that schemars derives for the type. The struct carries `#[serde(deny_unknown_fields)]` because OpenAI's strict mode requires `additionalProperties: false` on every object. The [schema rules](/models/structured-output/#schema-rules) list the other constraints. `SimplePrompt`, `StructuredPrompt` and `ChatCompletion` from `swiftide::traits` must be in scope, and `futures_util::StreamExt` for the stream.

## Chat completion

::snippet{src="@swiftide/examples/responses_api.rs" anchor="chat" title="examples/responses_api.rs"}

A `ChatCompletionRequest` holds the messages and, optionally, tool specs. `build()` returns a `Result`. `complete` returns a `ChatCompletionResponse`, and `message()` gives you the assistant text if there is any.

## Streaming

::snippet{src="@swiftide/examples/responses_api.rs" anchor="streaming" title="examples/responses_api.rs"}

`complete_stream` returns a stream of `ChatCompletionResponse` values. The new text of each chunk is in `delta.message_chunk`. The client's `stream_full` setting defaults to true, so every chunk also carries the message accumulated so far in `message()`. The example prints the deltas and keeps the last accumulated message. With `stream_full(false)` on the builder, every chunk but the last carries only the delta. The last one still holds the complete response, including usage, so the same logic works.

## Tool calls

::snippet{src="@swiftide/examples/responses_api.rs" anchor="tools" title="examples/responses_api.rs"}

`#[swiftide::tool]` turns the function into a tool. It generates a constructor named after the function, `echo_tool()`, which returns a boxed tool. It also generates an argument struct named after the tool in PascalCase with an `Args` suffix, here `EchoToolArgs`, with one field per parameter. The tool name defaults to the function name, and `name = "..."` in the attribute changes both. The only parameter, `payload`, is the `EchoArgs` struct from the imports excerpt.

::snippet{src="@swiftide/examples/responses_api.rs" anchor="tool_call" title="examples/responses_api.rs"}

`ChatCompletionRequest::builder().tool(...)` adds only the tool's spec to the request. Nothing runs the tool for you. The example reads the first tool call, parses its arguments into `EchoToolArgs` and builds the output itself. It then appends the assistant's tool call with `ChatMessage::new_assistant` and the result with `ChatMessage::new_tool_output`, and sends a follow-up request. An [agent](/agents/overview/) runs this loop for you, as the next recipe shows.

## Reasoning summaries

::snippet{src="@swiftide/examples/responses_api_reasoning.rs" anchor="setup" title="examples/responses_api_reasoning.rs"}

Setting `reasoning_effort` on the options turns on reasoning for the request. With the Responses API, the client then also asks for an automatic reasoning summary and for the encrypted reasoning content. `reasoning_features(false)` on the options turns both off. If your OpenAI organization is not verified for reasoning access, the response may hold no summary.

::snippet{src="@swiftide/examples/responses_api_reasoning.rs" anchor="reasoning" title="examples/responses_api_reasoning.rs"}

The helper joins the summaries of all `ReasoningItem` values into one string.

::snippet{src="@swiftide/examples/responses_api_reasoning.rs" anchor="agent" title="examples/responses_api_reasoning.rs"}

Two hooks print what happens. `after_completion` receives the `ChatCompletionResponse` before the agent takes the reasoning out of it, so `response.reasoning` is still there. `on_new_message` fires for the messages the agent adds itself: reasoning items, assistant messages and tool outputs. The system prompt and your query go into the context directly and skip it. The example prints only assistant messages.

::snippet{src="@swiftide/examples/responses_api_reasoning.rs" anchor="run" title="examples/responses_api_reasoning.rs"}

[Reasoning items](/agents/providers-and-reasoning/#reasoning-items) explains how the agent stores the items and replays them on the next request.

### Variations

- `ReasoningEffort` has the variants `None`, `Minimal`, `Low`, `Medium`, `High` and `Xhigh`. Which ones a model accepts depends on the model.
- Without an agent, call `complete` on a client with `use_responses_api(true)` and `reasoning_effort` set, and read `response.reasoning` yourself. On the Chat Completions path, `response.reasoning` stays `None`.
- The `on_new_message` hook uses a let-chain, which needs `edition = "2024"`. On edition 2021, nest the two `if let` statements.
- [Responses API and reasoning](/models/overview/#responses-api-and-reasoning) covers how other providers return reasoning.

## Reference

- [`GenericOpenAIBuilder::use_responses_api`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html#method.use_responses_api)
- [`GenericOpenAIBuilder::stream_full`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html#method.stream_full)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)
- [`Options`](docsrs:swiftide/integrations/openai/struct.Options.html)
- [`OptionsBuilder`](docsrs:swiftide/integrations/openai/struct.OptionsBuilder.html)
- [`OptionsBuilder::reasoning_features`](docsrs:swiftide/integrations/openai/struct.OptionsBuilder.html#method.reasoning_features)
- [`ReasoningEffort` (OpenAI)](docsrs:swiftide/integrations/openai/enum.ReasoningEffort.html)
- [`ChatCompletionRequest`](docsrs:swiftide/chat_completion/struct.ChatCompletionRequest.html)
- [`ChatCompletionResponse`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponse.html)
- [`ChatCompletionResponseDelta`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponseDelta.html)
- [`ReasoningItem`](docsrs:swiftide/chat_completion/struct.ReasoningItem.html)
- [`ToolOutput`](docsrs:swiftide/chat_completion/enum.ToolOutput.html)
- [`ToolCall`](docsrs:swiftide/chat_completion/struct.ToolCall.html)
- [`ChatMessage`](docsrs:swiftide/chat_completion/enum.ChatMessage.html)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`StructuredPrompt`](docsrs:swiftide/traits/trait.StructuredPrompt.html)
- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`#[swiftide::tool]`](docsrs:swiftide/attr.tool.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`AgentBuilder::after_completion`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.after_completion)
- [`AgentBuilder::on_new_message`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.on_new_message)

## Next

[Agents on AWS Bedrock](/cookbook/bedrock-agents/)
