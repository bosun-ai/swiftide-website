---
title: "OpenAI and Azure"
description: "Build an OpenAI or Azure OpenAI client for prompts, chat completions, agents, embeddings and the Responses API, and point it at any OpenAI-compatible base URL."
sidebar:
  order: 2
---

Here you build an `OpenAI` client, or the same client for an Azure OpenAI deployment, and pass it to indexing pipelines, query pipelines and agents. The integration sits behind the `openai` feature flag:

```sh
cargo add swiftide@{{swiftide_version}} --features openai
```

## Client

::snippet{src="@swiftide/examples/agent_can_fail_custom_schema.rs" anchor="setup"}

`default_prompt_model` sets the model for `prompt`, `structured_prompt`, `complete` and `complete_stream`. `default_embed_model` sets the model for `embed`. The client has no default model. A call without one fails with a permanent error that says `Model not set`.

Unless you call `api_base`, `api_key` or `client(..)`, the builder creates an `async_openai::Client` with `OpenAIConfig::default()`. That configuration reads `OPENAI_API_KEY` and `OPENAI_BASE_URL` from the environment, and `OPENAI_ORG_ID` and `OPENAI_PROJECT_ID` when they are set. The base URL defaults to `https://api.openai.com/v1`.

`OpenAI` implements `SimplePrompt`, `ChatCompletion`, `EmbeddingModel` and `StructuredPrompt`. So one client serves a large language model (LLM) transformer, the `Embed` batch transformer, an answer generator and `Agent::builder().llm(&openai)`. Agents also need the `swiftide-agents` feature flag. Cloning the client is cheap: the clones share one HTTP client.

## Custom base URL

::snippet{src="@swiftide/examples/ask-your-codebase/src/llm.rs" anchor="llm" title="examples/ask-your-codebase/src/llm.rs"}

The ask-your-codebase showcase sends its prompts to any OpenAI-compatible endpoint. `LlmEndpoint` holds the base URL, key and model from the command line. `api_base` and `api_key` on the builder override the base URL and key from the environment. :unreleased Everything else still comes from `OpenAIConfig::default()`, so `OPENAI_ORG_ID` and `OPENAI_PROJECT_ID` are sent as headers when they are set, also to a third-party gateway. The showcase picks the Responses API or Chat Completions from a command-line flag.

The showcase wraps the client in `LanguageModelWithBackOff`. Transient errors, such as rate limits and outages, are retried with exponential backoff until `max_elapsed_time_sec` runs out. Permanent errors, including quota and billing errors that arrive as HTTP 429, return at once. `LlmEndpoint::client` returns `Arc<dyn SimplePrompt>`, so the rest of the showcase is provider-independent. [Errors, backoff, usage and tokens](/models/errors-backoff-and-usage/) covers the settings.

For anything the two setters do not cover, such as a custom HTTP client or another configuration type, pass a whole `async_openai::Client` to `client(..)`. It replaces what `api_base` and `api_key` set, and whichever you call last wins. :unreleased Swiftide re-exports `OpenAIConfig`, `AzureConfig` and `ReasoningEffort` from `async-openai` at `swiftide::integrations::openai`. It does not re-export `async_openai::Client`, so add the crate at the version Swiftide uses:

```sh
cargo add async-openai@0.41 --no-default-features
```

## Azure OpenAI

Azure OpenAI uses its own URL scheme and authentication header, so it needs `AzureConfig` instead of `OpenAIConfig`:

::snippet{src="@snippets/src/integrations/openai.rs" anchor="azure"}

`OpenAI` is a type alias for `GenericOpenAI<OpenAIConfig>`. For Azure you build a `GenericOpenAI<AzureConfig>` through `GenericOpenAIBuilder` and pass the client with `client(..)`. `api_base` and `api_key` exist only on the `OpenAI` builder. The Azure client implements the same model traits.

`AzureConfig::with_api_base` takes the resource URL without a path. Requests go to `{api_base}/openai/deployments/{deployment_id}/...`, with `api-version` as a query parameter and the key in an `api-key` header. Without `with_api_key`, `AzureConfig` falls back to `OPENAI_API_KEY`.

A deployment serves one model, so build one client per deployment, for example one for chat and one for embeddings. Set `default_prompt_model` or `default_embed_model` anyway: Swiftide refuses to send a request without a model, even though Azure routes by deployment.

`GenericOpenAI<AzureConfig>` is a different type from `OpenAI`. A field typed `OpenAI` does not accept it. Store the client as `Arc<dyn SimplePrompt>` or another trait object when the provider varies. See [Clients as trait objects](/models/overview/#clients-as-trait-objects).

## Embeddings

::snippet{src="@snippets/src/integrations/openai.rs" anchor="embeddings"}

`embed` sends a list of texts in one request and returns one vector per text. Embedding requests carry two settings from `Options`: `dimensions` and `user`. `dimensions` works with text-embedding-3 and later models. The vector size you get must match the vector size of your storage.

In an indexing pipeline, the `Embed` batch transformer calls `embed` once per batch. `with_batch_size(64)` sets the batch size. Without it, the pipeline default of 256 applies. Use the same model and dimensions for the `Embed` query transformer, so queries and chunks land in the same vector space.

## Request options

[Request settings](/models/overview/#request-settings) covers `default_options` and which call sends which field. Two details are specific to this client.

`parallel_tool_calls` is unset by default, although the builder method's documentation says it defaults to `true`. Once you set it, the client sends it on every chat request, including `prompt`, `structured_prompt` and `complete` calls without tools. OpenAI rejects `parallel_tool_calls` on a request without tools. Give your agents a separate client when you set it, and keep it off the client that serves transformers and answer generators.

`extra_body` merges extra fields into the top level of the request body. Only `complete` and `complete_stream` send it, on the Chat Completions API and the Responses API alike. `prompt`, `structured_prompt` and `embed` do not.

## Responses API

::snippet{src="@swiftide/examples/responses_api.rs" anchor="setup"}

`use_responses_api(true)` sends `complete`, `complete_stream`, `prompt` and `structured_prompt` to the OpenAI Responses API instead of the Chat Completions API. `embed` is unaffected. The flag is off by default.

With `reasoning_effort` set, the client asks for reasoning summaries and encrypted reasoning content, and sends the encrypted content back on the next turn. Summaries are not sent back. `reasoning_features(false)` turns that off. The Responses API ignores `seed` and `presence_penalty` and logs a warning. [Responses API and reasoning](/models/overview/#responses-api-and-reasoning) shows an agent that keeps its reasoning in the message history.

## Streaming and usage

`stream_full(false)` makes `complete_stream` send only the delta in every item but the last. See [Deltas only](/models/overview/#deltas-only).

`on_usage` and `on_usage_async` report the token `Usage` of every request, embeddings included. The `on_usage_async` callback returns a pinned, boxed future. See [Errors, backoff, usage and tokens](/models/errors-backoff-and-usage/).

## Gotchas

- Some OpenAI-compatible gateways cap unset `max_completion_tokens` at 4096 and truncate silently; see [OpenAI-compatible providers](/integrations/openai-compatible/#gotchas).
- A gateway's Responses API has to match OpenAI's response schema exactly. The captured docs-qa run uses Chat Completions, its default, because its gateway returns `"input_tokens_details": null`, which `async-openai` rejects.

## Reference

- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)
- [`OpenAIBuilder`](docsrs:swiftide/integrations/openai/type.OpenAIBuilder.html)
- [`GenericOpenAI`](docsrs:swiftide/integrations/openai/struct.GenericOpenAI.html)
- [`GenericOpenAIBuilder`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html)
- [`OpenAIConfig`](docsrs:swiftide/integrations/openai/struct.OpenAIConfig.html)
- [`AzureConfig`](docsrs:swiftide/integrations/openai/struct.AzureConfig.html)
- [`Options`](docsrs:swiftide/integrations/openai/struct.Options.html)
- [`OptionsBuilder`](docsrs:swiftide/integrations/openai/struct.OptionsBuilder.html)
- [`ReasoningEffort`](docsrs:swiftide/integrations/openai/enum.ReasoningEffort.html)
- [`GenericOpenAIBuilder::use_responses_api`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html#method.use_responses_api)
- [`GenericOpenAIBuilder::stream_full`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html#method.stream_full)
- [`GenericOpenAIBuilder::on_usage`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html#method.on_usage)
- [`GenericOpenAIBuilder::parallel_tool_calls`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html#method.parallel_tool_calls)
- [`Embed`](docsrs:swiftide/indexing/transformers/struct.Embed.html)
- [`EmbeddingModel`](docsrs:swiftide/traits/trait.EmbeddingModel.html)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)
- [`BackoffConfiguration`](docsrs:swiftide/indexing/struct.BackoffConfiguration.html)
- [`Usage`](docsrs:swiftide/chat_completion/struct.Usage.html)

## Next

[OpenAI-compatible providers](/integrations/openai-compatible/)
