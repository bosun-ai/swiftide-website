---
title: "Errors, backoff, usage and tokens"
description: "Tell transient from permanent model errors, retry with LanguageModelWithBackOff, record token usage with on_usage, and estimate tokens with CharEstimator or tiktoken."
sidebar:
  order: 5
identLint:
  ignore: ["chain()"]
---

By the end of this page your OpenAI-compatible or `AwsBedrock` client retries rate limits on its own, reports the tokens each call used, and you can estimate the size of a prompt before you send it.

::snippet{src="@snippets/src/models/errors_backoff_and_usage.rs" anchor="match_errors"}

A failed model call returns a `LanguageModelError`. Each of its three variants wraps the underlying error:

- `ContextLengthExceeded`: the input is longer than the model accepts. Shorten it before you send it again.
- `TransientError`: the call may succeed if you try again later.
- `PermanentError`: the call fails the same way every time.

The OpenAI-compatible clients sort provider errors into these variants for you:

- The identifier `context_length_exceeded` becomes `ContextLengthExceeded`.
- Network errors and Hypertext Transfer Protocol (HTTP) status 408, 429 and 5xx are transient.
- OpenRouter's rate limit, overload, unavailable, server and timeout error types are transient.
- Quota and billing errors are permanent, even when they arrive with status 429.
- An error inside a successful response is read and sorted the same way.
- A response that fails to parse and holds no such error is transient.
- Invalid arguments and broken streams are permanent.

`AwsBedrock` treats throttling, timeouts, unavailable services and network errors as transient. `Anthropic` treats only network errors on `prompt` as transient. Every other `Anthropic` error is permanent, including status 429.

To build a variant yourself, call `LanguageModelError::transient`, `permanent` or `context_length_exceeded`. An `anyhow::Error` converts into a `PermanentError`.

To return a variant with `?` or `.into()`, bind the whole `LanguageModelError` (`error @ ...`). The `Box<dyn Error + Send + Sync>` inside a variant does not convert into `anyhow::Error`.

### Errors from a pipeline

When a step of an indexing pipeline fails on a model call, `run` returns an `anyhow::Error` with the `LanguageModelError` still inside. So you can tell a rate limit from a broken request. :unreleased

::snippet{src="@snippets/src/models/errors_backoff_and_usage.rs" anchor="pipeline_errors"}

`downcast_ref::<LanguageModelError>()` on the error works too. Searching `chain()` also finds the model error when a step added context around it.

## Backoff

::snippet{src="@snippets/src/models/errors_backoff_and_usage.rs" anchor="backoff_configuration"}

`LanguageModelWithBackOff` wraps a client and retries calls that fail with a `TransientError`. On `prompt` and `complete`, a `ContextLengthExceeded` or `PermanentError` returns at once. The decorated client implements `SimplePrompt`, `ChatCompletion` and `EmbeddingModel` when the inner client does. You can pass it where one of those traits is expected, including `Agent::builder().llm(&openai)`.

It never retries an `Anthropic` rate limit, because `Anthropic` marks it permanent.

`BackoffConfiguration` has four fields:

- `initial_interval_sec`: the wait before the first retry, in seconds. The default is 1.
- `multiplier`: the factor the wait grows by after each attempt. The default is 2.0.
- `randomization_factor`: random jitter on each wait. The default is 0.5, plus or minus half the interval.
- `max_elapsed_time_sec`: the time budget in seconds, including time spent inside calls. The decorator returns the last error as soon as the next wait would pass it, often before the full budget. The default is 60.

`Default::default()` gives you these defaults.

Every failed attempt that the decorator retries logs a WARN event, `Language model request failed, retrying`, with the attempt number, the wait and the error with its causes. When the decorator gives up, it logs the final error once at ERROR, also for a permanent error that it never retried. :unreleased Only `prompt` and `complete` log these events.

For `complete_stream` the decorator does not send the request again. Your stream still receives every error. After each one, transient or permanent, the decorator waits for the next interval before it reads on. Once `max_elapsed_time_sec` is used up, the stream ends after the last error. The budget restarts after every successful item. OpenAI-compatible streams end after their first error, so the wait only delays that end.

Two kinds of call get no retry. `EmbeddingModel` calls the inner client once. `StructuredPrompt` and methods of the client itself, such as `estimate_tokens`, do not exist on the decorator, so call them on the inner client.

## Usage callbacks

::snippet{src="@snippets/src/models/errors_backoff_and_usage.rs" anchor="on_usage"}

A usage callback receives the `Usage` of each call: `prompt_tokens`, `completion_tokens` and `total_tokens`. Set it with `on_usage` on the builder of an OpenAI-compatible client (`openai` feature flag), `Anthropic` (`anthropic`) or `AwsBedrock` (`aws-bedrock`). On OpenAI-compatible clients, embedding calls report usage too.

Use `on_usage_async` when the callback needs to await:

::snippet{src="@snippets/src/models/errors_backoff_and_usage.rs" anchor="on_usage_async"}

The closure gets a borrowed `Usage` and returns a boxed future that may borrow it. Copy the numbers out first only if you hand them to a task that must be `'static`, such as one you spawn.

The providers run the callback differently:

- OpenAI-compatible clients spawn it. An `Err` is logged and the call still succeeds.
- `AwsBedrock`, and `Anthropic` on calls that do not stream, await it. An `Err` from the callback fails the call.
- `Anthropic` streaming calls spawn it and only log an `Err`.

With the `metrics` feature flag, clients also emit usage counters. See [Metrics and statistics](/observability/metrics-and-statistics/).

## Normalized usage

::snippet{src="@snippets/src/models/errors_backoff_and_usage.rs" anchor="normalized_usage"}

`Usage` has an optional `details` field with the provider's own breakdown. `Usage::normalized` returns a `NormalizedUsage` with one shape for every provider: cached and audio tokens on the input side, and reasoning, audio and prediction tokens on the output side. Its `details` is `None` when the provider reported none of them.

OpenAI fills the details from Responses application programming interface (API) calls and from Chat Completions calls that do not stream. `AwsBedrock` reports cached input tokens. Streamed Chat Completions, embeddings and `Anthropic` report the totals only.

## Token estimation

::snippet{src="@snippets/src/models/errors_backoff_and_usage.rs" anchor="estimate_tokens"}

`CharEstimator` needs no feature flag. It divides the number of characters by 4, then adds 4 tokens per message and 3 per list of messages.

`estimate` comes from the `EstimateTokens` trait. It accepts any `Estimatable`: a `&str`, a `String`, a `&Prompt` (which it renders first), a `&ChatMessage` or a `&[ChatMessage]`. For a `Vec<ChatMessage>`, pass a slice.

For estimates closer to what OpenAI models see, enable the `tiktoken` feature flag:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,tiktoken
```

::snippet{src="@snippets/src/models/errors_backoff_and_usage.rs" anchor="estimate_tiktoken"}

`TikToken` encodes the text with the Byte Pair Encoding (BPE) tokenizer of the model you name in `try_from_model`. `TikToken::default()` uses the `gpt-4o` tokenizer. With the feature on, an OpenAI client also has `estimate_tokens`, which uses the client's `TikToken`. That is `gpt-4o` whatever the prompt model, unless you set `tiktoken` on the builder.

Both return estimates, not exact counts. They skip what they cannot read as text: image, document, audio and video parts of a message, and the text of reasoning messages. Each message still adds its 4 tokens, so an image-only or reasoning message counts as 4.

## Reference

- [`LanguageModelError`](docsrs:swiftide/chat_completion/errors/enum.LanguageModelError.html)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`EmbeddingModel`](docsrs:swiftide/traits/trait.EmbeddingModel.html)
- [`StructuredPrompt`](docsrs:swiftide/traits/trait.StructuredPrompt.html)
- [`AgentBuilder`](docsrs:swiftide/agents/struct.AgentBuilder.html) (`llm`)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)
- [`BackoffConfiguration`](docsrs:swiftide/indexing/struct.BackoffConfiguration.html)
- [`GenericOpenAIBuilder`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html) (`on_usage`, `on_usage_async`)
- [`AnthropicBuilder`](docsrs:swiftide/integrations/anthropic/struct.AnthropicBuilder.html) (`on_usage`, `on_usage_async`)
- [`AwsBedrockBuilder`](docsrs:swiftide/integrations/aws_bedrock_v2/struct.AwsBedrockBuilder.html) (`on_usage`, `on_usage_async`)
- [`Usage`](docsrs:swiftide/chat_completion/struct.Usage.html)
- [`NormalizedUsage`](docsrs:swiftide/chat_completion/struct.NormalizedUsage.html)
- [`CharEstimator`](docsrs:swiftide/struct.CharEstimator.html)
- [`EstimateTokens`](docsrs:swiftide/traits/trait.EstimateTokens.html)
- [`Estimatable`](docsrs:swiftide/traits/trait.Estimatable.html)
- [`ChatMessage`](docsrs:swiftide/chat_completion/enum.ChatMessage.html)
- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)
- [`TikToken`](docsrs:swiftide/integrations/tiktoken/struct.TikToken.html)

## Next

[Tracing](/observability/tracing/)
