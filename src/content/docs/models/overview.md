---
title: "Models"
description: "The model traits Swiftide uses, which providers implement each one, and how to build a client for OpenAI, Anthropic, Ollama or any OpenAI-compatible endpoint."
sidebar:
  order: 1
---

By the end of this page you have a client for your model provider, and you know which trait each part of Swiftide expects from it.

::snippet{src="@snippets/src/models/overview.rs" anchor="provider_clients"}

Every provider has a client, and every client has a builder. `default_prompt_model` sets the model for prompts and chat completions. `default_embed_model` sets the model for embeddings. `OpenAI` reads its key from `OPENAI_API_KEY`. `Anthropic` reads `ANTHROPIC_API_KEY`. Ollama runs on your machine and needs no key.

OpenAI and the other OpenAI-compatible clients have no default model. A call without one fails with a permanent error that says `Model not set`. `Anthropic` falls back to `claude-3-5-sonnet-20241022`, a retired model, so always set `default_prompt_model`.

Each provider sits behind its own feature flag. Turn on the ones you use:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,anthropic,ollama
```

## Model traits

Swiftide never calls a provider by name. Each part of it asks for a trait, and you pass any client that implements that trait. The traits live in `swiftide::traits`. Import a trait to call its method on a client.

| Trait | Method | Used by |
| --- | --- | --- |
| `SimplePrompt` | `prompt`: one prompt in, text out | Large Language Model (LLM) transformers such as `MetadataQAText`, the `GenerateSubquestions` query transformer, the `answers::Simple` answer generator, and `Pipeline::with_default_llm_client` |
| `ChatCompletion` | `complete`: messages and tools in, a response out | agents, through `Agent::builder().llm(..)`, and task nodes in `swiftide::tasks` |
| `EmbeddingModel` | `embed`: a list of texts in, dense vectors out | the `Embed` batch transformer and the `Embed` query transformer |
| `SparseEmbeddingModel` | `sparse_embed`: a list of texts in, sparse vectors out | the `SparseEmbed` batch transformer and the `SparseEmbed` query transformer |
| `StructuredPrompt` | `structured_prompt::<T>`: one prompt in, a Rust value out | your own code, for typed responses |

Failed calls return a `LanguageModelError`. `complete_stream` is the exception: it returns a stream, and errors arrive as items of that stream. See [Streaming](/models/overview/#streaming).

## Providers

| Client (feature flag) | `SimplePrompt` | `ChatCompletion` | `EmbeddingModel` | `SparseEmbeddingModel` | `StructuredPrompt` |
| --- | --- | --- | --- | --- | --- |
| `OpenAI` (`openai`) | yes | yes | yes | no | yes |
| `Ollama` (`ollama`), `Groq` (`groq`), `Gemini` (`gemini`), `OpenRouter` (`open-router`), `Dashscope` (`dashscope`) | yes | yes | yes | no | yes |
| `Anthropic` (`anthropic`) | yes | yes | no | no | no |
| `AwsBedrock` (`aws-bedrock`) | yes | yes | no | no | yes |
| `FastEmbed` (`fastembed`) | no | no | yes | yes | no |

`OpenAI` is `GenericOpenAI<OpenAIConfig>`. `Ollama`, `Groq`, `Gemini`, `OpenRouter` and `Dashscope` are `GenericOpenAI` with their own configuration type. So the aliases implement every trait `OpenAI` does, and you can pass them anywhere a model trait is expected. They are still different types. A field typed `OpenAI` does not accept an `Ollama`. The endpoint and the model decide what works at runtime. Not every provider serves embeddings or structured output.

`AwsBedrock` talks to Amazon Web Services (AWS) Bedrock through the Converse application programming interface (API), in the `aws_bedrock_v2` module. `FastEmbed` runs embedding models locally. Setup for both is on [AWS Bedrock](/integrations/aws-bedrock/) and [FastEmbed](/integrations/fastembed/).

## Clients as trait objects

::snippet{src="@snippets/src/models/overview.rs" anchor="trait_objects"}

Store a client as `Arc<dyn SimplePrompt>` or `Arc<dyn EmbeddingModel>` when you pick the provider at runtime. Cloning a client is cheap. Every built-in client keeps its connection or model behind an `Arc`, so clones share it.

`Agent::builder().llm(&openai)` takes a reference. It clones the client into a `Box<dyn ChatCompletion>`, so you can keep using `openai` afterwards. `Agent` needs the `swiftide-agents` feature flag on top of the provider flags.

The `structured_prompt::<T>` method is generic, so `StructuredPrompt` cannot be a trait object. [Structured output](/models/structured-output/) covers `DynStructuredPrompt`, the variant that can.

To retry rate limits and server errors, wrap the client in `LanguageModelWithBackOff`. See [Errors, backoff, usage and tokens](/models/errors-backoff-and-usage/).

## OpenAI-compatible endpoints

A server that speaks the OpenAI API works with the `OpenAI` client. Point the client at it with `api_base` and `api_key` on the builder.

::snippet{src="@snippets/src/models/overview.rs" anchor="custom_base_url"}

With a single endpoint you can skip the code. The default `OpenAI` client reads `OPENAI_BASE_URL` and `OPENAI_API_KEY` from the environment when you build it.

For a custom HTTP (Hypertext Transfer Protocol) client or another configuration type, pass an `async_openai::Client` to `client(..)` instead. Azure OpenAI needs that route with `GenericOpenAI<AzureConfig>`; [Azure OpenAI](/integrations/openai/#azure-openai) shows it.

The hosted-provider clients have the base URL already set and read their key from the environment:

| Client | Key variable |
| --- | --- |
| `Ollama` | none |
| `Groq` | `GROQ_API_KEY` |
| `Gemini` | `GEMINI_API_KEY` |
| `OpenRouter` | `OPENROUTER_API_KEY` |
| `Dashscope` | `QWEN_API_KEY`, then `DASHSCOPE_API_KEY` |

`OpenRouter` without a key does not return an error. It panics on the first request. More on these clients is on [OpenAI-compatible providers](/integrations/openai-compatible/).

## Request settings

Each OpenAI-compatible client holds one `Options` value. Every request it sends carries those settings:

::snippet{src="@snippets/src/models/overview.rs" anchor="request_settings"}

`default_options` merges into the options the builder already has: a field you set replaces the earlier value. `for_end_user` and `parallel_tool_calls` are builder shortcuts for the `user` and `parallel_tool_calls` fields.

Apart from `reasoning_features`, every field starts unset, and Swiftide only sends the fields you set. That includes `parallel_tool_calls`: set it to `Some(false)` to stop the model from asking for several tools in one turn. Some reasoning models reject the field, so leave it at `None` for them.

| Field | Sent with |
| --- | --- |
| `temperature`, `max_completion_tokens`, `seed`, `presence_penalty`, `metadata`, `reasoning_effort` | `complete`, `prompt` and `structured_prompt` |
| `user` | `complete`, `prompt`, `structured_prompt` and `embed`; not sent on the Responses API |
| `dimensions` | `embed` only; text-embedding-3 and later models |
| `reasoning_features` | the Responses API only, and only when `reasoning_effort` is set |

`ReasoningEffort` is re-exported from `async-openai` at `swiftide::integrations::openai`. The Responses API (`use_responses_api(true)`) ignores `seed` and `presence_penalty` and logs a warning. It accepts `metadata` only as a flat map of string values. `reasoning_features` defaults to `true` on `OpenAI`. It asks for reasoning summaries and encrypted reasoning content. Turn it off with `reasoning_features(false)`.

To change a built client, clone it and edit the copy through `options_mut`. Each clone has its own `Options`:

::snippet{src="@snippets/src/models/overview.rs" anchor="options_mut"}

### Extra request fields

Some providers accept request fields that the OpenAI API does not have. `Options` has `extra_body`, a JavaScript Object Notation (JSON) map that Swiftide merges into the top level of each request:

::snippet{src="@snippets/src/models/overview.rs" anchor="extra_body"}

Only `ChatCompletion` requests carry `extra_body`. `prompt`, `structured_prompt` and `embed` calls do not. So LLM transformers such as `MetadataQAText` and the `answers::Simple` answer generator send the request without the extra fields.

### AWS Bedrock settings

`AwsBedrock` has its own `Options` in `swiftide::integrations::aws_bedrock_v2`. Set them with `AwsBedrockBuilder::default_options`, which merges the same way, or change them later with `options_mut`.

| Field | Effect |
| --- | --- |
| `max_tokens`, `temperature`, `top_p`, `stop_sequences` | the Converse inference configuration |
| `tool_strict` | strict schema validation for tool calls; `true` when unset |
| `anthropic_beta` | Anthropic beta headers, sent in the model request fields |
| `reasoning_effort` | `output_config.effort` for Claude models: `Low`, `Medium`, `High` or `Max` |
| `additional_model_request_fields` | any other model request fields, as an `aws_smithy_types::Document` |

`reasoning_effort` here is the Bedrock module's own `ReasoningEffort` enum, not the OpenAI one. For a model ID that contains `claude-opus-4-5`, Swiftide adds the `effort-2025-11-24` beta header itself. Behind an inference profile or Amazon Resource Name (ARN), add it to `anthropic_beta` yourself. With `anthropic_beta` or `reasoning_effort` set, `additional_model_request_fields` must be a JSON object, or the request fails with a permanent error.

## Responses API and reasoning

`OpenAI` uses the Chat Completions API by default. `use_responses_api(true)` switches `complete`, `complete_stream`, `prompt` and `structured_prompt` to the OpenAI Responses API. For `OpenAI`, only that API returns the reasoning of a reasoning model:

::snippet{src="@snippets/src/models/overview.rs" anchor="responses_api_reasoning"}

With `reasoning_effort` set, reasoning arrives in the `reasoning` field of `ChatCompletionResponse` as `ReasoningItem` values. An agent adds each one to its message history as a `ChatMessage::Reasoning`, even when the response holds nothing else. On the next request the client sends the items back, so the model continues from its earlier reasoning. These requests set `store` to false, so the encrypted reasoning lives in your message history, not at OpenAI.

The client sends back only items with `encrypted_content`. With `reasoning_features(false)` there is none, and nothing goes back. OpenAI returns summaries only to organizations verified for reasoning access. The Responses API does not send `extra_body`.

`AwsBedrock` returns signed Claude reasoning as `ReasoningItem` values too, and replays them on the next turn. The `Anthropic` client leaves `ChatMessage::Reasoning` out of its requests.

## Streaming

`complete_stream` sends a chat completion request and returns a `ChatCompletionStream` instead of one response. Each item is a `Result<ChatCompletionResponse, LanguageModelError>`. You read it with `StreamExt::next` from the `futures-util` crate. Swiftide does not re-export it:

```sh
cargo add futures-util@0.3
```

::snippet{src="@snippets/src/models/streaming.rs" anchor="complete_stream"}

Every item is a whole `ChatCompletionResponse`, not a fragment. `message` holds all the text so far. `tool_calls` holds the tool calls rebuilt from the fragments so far. `delta` holds the latest change: `delta.message_chunk` is the new text, and `delta.tool_calls_chunk` maps each tool call index to its fragments so far. The snippet prints the part of `message` it has not printed yet, which works the same for every client.

The last item is the complete response. Token usage comes in at the end of the stream, so read `usage` from the last item.

A request that fails before the stream starts gives a stream with a single `Err` item. A client without its own streaming keeps the default `complete_stream`. It calls `complete` and returns a stream with that one result.

An agent that streams calls `complete_stream` for you. You read its chunks in the `on_stream` hook. See [Streaming](/agents/streaming/).

### Deltas only

By default every item clones the full response, which grows with the answer. The OpenAI-compatible clients can send only the delta. Turn off `stream_full` on the builder:

::snippet{src="@snippets/src/models/streaming.rs" anchor="stream_deltas"}

With `stream_full(false)`, every item but the last carries only `id` and `delta`. `message`, `tool_calls` and `usage` are `None`. The last item is still the complete response, with `message`, `tool_calls` and `usage` set. The snippet uses `message` to tell the last item apart and prints `delta.message_chunk` for the others.

`stream_full` defaults to `true`. It is a setting of `GenericOpenAI`, so it exists on `OpenAI`, `Ollama`, `Groq`, `Gemini`, `OpenRouter` and `Dashscope`, on the Chat Completions API and the Responses API alike. `Anthropic` and `AwsBedrock` have no such setting and always send the accumulated response.

## Reference

- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`EmbeddingModel`](docsrs:swiftide/traits/trait.EmbeddingModel.html)
- [`SparseEmbeddingModel`](docsrs:swiftide/traits/trait.SparseEmbeddingModel.html)
- [`StructuredPrompt`](docsrs:swiftide/traits/trait.StructuredPrompt.html)
- [`DynStructuredPrompt`](docsrs:swiftide/traits/trait.DynStructuredPrompt.html)
- [`LanguageModelError`](docsrs:swiftide/chat_completion/errors/enum.LanguageModelError.html)
- [`GenericOpenAI`](docsrs:swiftide/integrations/openai/struct.GenericOpenAI.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)
- [`OpenAIConfig`](docsrs:swiftide/integrations/openai/struct.OpenAIConfig.html)
- [`AzureConfig`](docsrs:swiftide/integrations/openai/struct.AzureConfig.html)
- [`Options`](docsrs:swiftide/integrations/openai/struct.Options.html)
- [`OptionsBuilder`](docsrs:swiftide/integrations/openai/struct.OptionsBuilder.html)
- [`GenericOpenAIBuilder::for_end_user`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html#method.for_end_user)
- [`GenericOpenAIBuilder::parallel_tool_calls`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html#method.parallel_tool_calls)
- [`GenericOpenAI::options_mut`](docsrs:swiftide/integrations/openai/struct.GenericOpenAI.html#method.options_mut)
- [`ReasoningEffort` (OpenAI)](docsrs:swiftide/integrations/openai/enum.ReasoningEffort.html)
- [`GenericOpenAIBuilder::use_responses_api`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html#method.use_responses_api)
- [`ChatCompletionResponse`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponse.html)
- [`ChatMessage`](docsrs:swiftide/chat_completion/enum.ChatMessage.html)
- [`ReasoningItem`](docsrs:swiftide/chat_completion/struct.ReasoningItem.html)
- [`Agent::history`](docsrs:swiftide/agents/struct.Agent.html#method.history)
- [`Anthropic`](docsrs:swiftide/integrations/anthropic/struct.Anthropic.html)
- [`Ollama`](docsrs:swiftide/integrations/ollama/type.Ollama.html)
- [`Groq`](docsrs:swiftide/integrations/groq/type.Groq.html)
- [`Gemini`](docsrs:swiftide/integrations/gemini/type.Gemini.html)
- [`OpenRouter`](docsrs:swiftide/integrations/open_router/type.OpenRouter.html)
- [`Dashscope`](docsrs:swiftide/integrations/dashscope/type.Dashscope.html)
- [`AwsBedrock`](docsrs:swiftide/integrations/aws_bedrock_v2/struct.AwsBedrock.html)
- [`Options` (AWS Bedrock)](docsrs:swiftide/integrations/aws_bedrock_v2/struct.Options.html)
- [`ReasoningEffort` (AWS Bedrock)](docsrs:swiftide/integrations/aws_bedrock_v2/enum.ReasoningEffort.html)
- [`FastEmbed`](docsrs:swiftide/integrations/fastembed/struct.FastEmbed.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`AgentBuilder::llm`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.llm)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)
- [`ChatCompletion::complete_stream`](docsrs:swiftide/traits/trait.ChatCompletion.html#method.complete_stream)
- [`ChatCompletionStream`](docsrs:swiftide/traits/type.ChatCompletionStream.html)
- [`ChatCompletionResponse`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponse.html)
- [`ChatCompletionResponseDelta`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponseDelta.html)
- [`GenericOpenAIBuilder::stream_full`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html#method.stream_full)

## Next

[Prompts and templates](/models/prompts-and-templates/)
