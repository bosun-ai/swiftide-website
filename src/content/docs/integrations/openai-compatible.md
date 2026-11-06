---
title: "OpenAI-compatible providers"
description: "Build a client for Ollama, Groq, Gemini, OpenRouter, Dashscope or any other server that speaks the OpenAI API, with the feature flag, base URL and key variable each one uses."
sidebar:
  order: 3
---

Swiftide talks to Ollama, Groq, Gemini, OpenRouter and Dashscope through the same OpenAI client, each with its own base Uniform Resource Locator (URL) and key. You end up with a working client for each, local embeddings through Ollama, and a client for any other gateway that speaks the OpenAI application programming interface (API).

| Client | Feature flag | Base URL | Key variable |
| --- | --- | --- | --- |
| `Ollama` | `ollama` | `http://localhost:11434/v1` | none |
| `Groq` | `groq` | `https://api.groq.com/openai/v1` | `GROQ_API_KEY` |
| `Gemini` | `gemini` | `https://generativelanguage.googleapis.com/v1beta/openai` | `GEMINI_API_KEY` |
| `OpenRouter` | `open-router` | `https://openrouter.ai/api/v1` | `OPENROUTER_API_KEY` |
| `Dashscope` | `dashscope` | `https://dashscope.aliyuncs.com/compatible-mode/v1` | `QWEN_API_KEY`, then `DASHSCOPE_API_KEY` |

Each of these feature flags also turns on `openai`:

```sh
cargo add swiftide@{{swiftide_version}} --features ollama
```

## Ollama

::snippet{src="@swiftide/examples/index_ollama.rs" anchor="setup" title="examples/index_ollama.rs"}

Every client on this page is `GenericOpenAI` with a provider-specific configuration type. It shares the `Options` and the model traits of [`OpenAI`](/integrations/openai/).

`Ollama::default()` builds a client without a model. `with_default_prompt_model` takes `&mut self` and returns `&mut Self`, so `.to_owned()` turns the result back into a client you own. The builder, `Ollama::builder()`, avoids that step.

The example uses Ollama only for the `MetadataQAText` transformer and embeds the chunks with FastEmbed, so it also needs the `fastembed` feature flag.

### Local embeddings

Ollama also serves embeddings. Set an embed model and use the same client on both sides:

::snippet{src="@snippets/src/integrations/openai_compatible.rs" anchor="ollama_embeddings"}

The `Embed` batch transformer embeds the chunks and the `Embed` query transformer embeds the question. Both must use the same model, or the vectors do not match. The storage vector size must match the model's output too. nomic-embed-text returns 768 dimensions.

The snippet stores the vectors in Qdrant, so it also needs the `qdrant` feature flag and a running Qdrant server. [Qdrant](/integrations/qdrant/) covers that part.

Pull the models before you run it:

```sh
ollama pull nomic-embed-text
ollama pull llama3.2
```

The Ollama example above uses `llama3.1`. Pull that one to run it.

### Remote Ollama

`OllamaConfig` is public, so you can point the client at another machine:

::snippet{src="@snippets/src/integrations/openai_compatible.rs" anchor="ollama_remote"}

`OllamaConfig::with_api_base` takes `&mut self`, so build the config in a mutable variable first. `async_openai::Client` is not re-exported. Add the crate at the version Swiftide uses:

```sh
cargo add async-openai@0.41 --no-default-features
```

`OllamaConfig` sends no `Authorization` header and reads no environment variable. For an Ollama server behind a proxy that checks a bearer token, use `OpenAI` with `api_base` and `api_key` instead. See [Other gateways](#other-gateways).

## Groq

::snippet{src="@swiftide/examples/index_groq.rs" anchor="setup" title="examples/index_groq.rs"}

`Groq::builder()` gives a builder with the Groq base URL and the key from `GROQ_API_KEY`. The `.to_owned()` in the middle clones the builder and changes nothing. Like the Ollama example, this one embeds with FastEmbed and needs the `fastembed` feature flag.

Groq has retired `llama3-8b-8192`, the model in the example. Pick a current model from Groq's model list.

## Gemini

::snippet{src="@swiftide/examples/hello_agents.rs" anchor="setup" title="examples/hello_agents.rs"}

The variable is named `openai`, but its type is `Gemini`. Later in the example it goes to `Agent::builder().llm(&openai)`, which works because `Gemini` implements `ChatCompletion`. The agent needs the `swiftide-agents` feature flag as well. The key comes from `GEMINI_API_KEY`.

The embed model in the example, `gemini-embedding-exp-03-07`, is experimental. Check Google's model list for a current embedding model before you copy it.

## OpenRouter

::snippet{src="@snippets/src/integrations/openai_compatible.rs" anchor="openrouter"}

The first client reads `OPENROUTER_API_KEY`. The second takes the key from an `OpenRouterConfig`, built with `OpenRouterConfig::builder()`. That builder requires `api_key`, `site_url` and `site_name`. Leave one out and `build()` returns an error at runtime.

`site_url` sets the `HTTP-Referer` header, which OpenRouter uses for its app rankings. At this release the `X-Title` header also takes the `site_url` value, so `site_name` is required but not sent. `OpenRouterConfig::default()` leaves both unset. It then sends `https://github.com/bosun-ai/swiftide` as `HTTP-Referer` and `Swiftide` as `X-Title`.

## Dashscope

::snippet{src="@swiftide/examples/dashscope.rs" anchor="setup" title="examples/dashscope.rs"}

`DashscopeBuilder::default()` is the same as `Dashscope::builder()`. The client prompts with `qwen-long` and embeds with `text-embedding-v2`. The example stores the vectors in LanceDB with a vector size of 1536, so it also needs the `lancedb` feature flag. [LanceDB](/integrations/lancedb/) covers that part.

## Other gateways

The configuration types of `Groq`, `Gemini` and `Dashscope` are private. To use another key variable or another base URL with them, use `OpenAI` with `api_base` and `api_key`.

::snippet{src="@snippets/src/integrations/openai_compatible.rs" anchor="gateway"}

For vLLM, LiteLLM or a company gateway, see [Custom base URL](/integrations/openai/#custom-base-url).

## Gotchas

- None of the clients has a default model. Set one with `default_prompt_model` or `default_embed_model`, or calls fail with `Model not set`.
- A client reads its key when you build it. For `Groq`, `Gemini` and `Dashscope`, a missing key becomes an empty one, and the provider rejects the first request. `OpenRouter` panics on the first request instead.
- Each alias is its own type. To pick the provider at runtime, see [Clients as trait objects](/models/overview/#clients-as-trait-objects).
- The endpoint decides what works. Every client implements `EmbeddingModel` and `StructuredPrompt`, but a provider without embeddings or structured output returns an error at runtime.
- Every client uses the Chat Completions API by default. `use_responses_api(true)` exists on every builder, but only some providers serve the Responses API.
- For request fields that only one provider knows, set `extra_body` in `Options`. Only `complete` and `complete_stream` send it. `prompt`, `structured_prompt` and `embed` do not. See [Extra request fields](/models/overview/#extra-request-fields).
- Some gateways cap the output at 4096 tokens when `max_completion_tokens` is unset. Set it in `Options`, because a cut-off answer looks complete.

## Reference

- [`GenericOpenAI`](docsrs:swiftide/integrations/openai/struct.GenericOpenAI.html)
- [`GenericOpenAI::with_default_prompt_model`](docsrs:swiftide/integrations/openai/struct.GenericOpenAI.html#method.with_default_prompt_model)
- [`GenericOpenAIBuilder`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html)
- [`Options`](docsrs:swiftide/integrations/openai/struct.Options.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)
- [`Ollama`](docsrs:swiftide/integrations/ollama/type.Ollama.html)
- [`OllamaConfig`](docsrs:swiftide/integrations/ollama/config/struct.OllamaConfig.html)
- [`OllamaConfig::with_api_base`](docsrs:swiftide/integrations/ollama/config/struct.OllamaConfig.html#method.with_api_base)
- [`Groq`](docsrs:swiftide/integrations/groq/type.Groq.html)
- [`Gemini`](docsrs:swiftide/integrations/gemini/type.Gemini.html)
- [`OpenRouter`](docsrs:swiftide/integrations/open_router/type.OpenRouter.html)
- [`OpenRouterConfig`](docsrs:swiftide/integrations/open_router/config/struct.OpenRouterConfig.html)
- [`OpenRouterConfigBuilder`](docsrs:swiftide/integrations/open_router/config/struct.OpenRouterConfigBuilder.html)
- [`Dashscope`](docsrs:swiftide/integrations/dashscope/type.Dashscope.html)
- [`DashscopeBuilder`](docsrs:swiftide/integrations/dashscope/type.DashscopeBuilder.html)
- [`MetadataQAText`](docsrs:swiftide/indexing/transformers/struct.MetadataQAText.html)
- [`Embed` (indexing)](docsrs:swiftide/indexing/transformers/struct.Embed.html)
- [`Embed` (query)](docsrs:swiftide/query/query_transformers/struct.Embed.html)
- [`FastEmbed`](docsrs:swiftide/integrations/fastembed/struct.FastEmbed.html)
- [`Qdrant`](docsrs:swiftide/integrations/qdrant/struct.Qdrant.html)
- [`LanceDB`](docsrs:swiftide/integrations/lancedb/struct.LanceDB.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`EmbeddingModel`](docsrs:swiftide/traits/trait.EmbeddingModel.html)
- [`StructuredPrompt`](docsrs:swiftide/traits/trait.StructuredPrompt.html)

## Next

[Anthropic](/integrations/anthropic/)
