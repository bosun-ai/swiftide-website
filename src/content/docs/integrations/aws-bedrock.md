---
title: "AWS Bedrock"
description: "Call Claude and other AWS Bedrock models through the Converse API with AwsBedrock, for prompts in indexing pipelines, agents with tools and structured output."
sidebar:
  order: 5
---

This page leaves you with an `AwsBedrock` client that sends requests to Amazon Web Services (AWS) Bedrock through the Converse application programming interface (API), and uses it in an indexing pipeline and in an agent with tools.

```sh
cargo add swiftide@{{swiftide_version}} --features aws-bedrock
cargo add tokio --features macros,rt-multi-thread
```

::snippet{src="@swiftide/examples/aws_bedrock.rs" anchor="setup" title="examples/aws_bedrock.rs"}

## Client

`AwsBedrock` lives in `swiftide::integrations::aws_bedrock_v2` behind the `aws-bedrock` feature flag. Since 0.33 it uses Bedrock's Converse and ConverseStream APIs, so one client covers every model family that Converse supports. [Upgrading](/reference/upgrading/) covers the move from the old module.

`default_prompt_model` takes a model ID or an Amazon Resource Name (ARN). The `global.` prefix in the example is a cross-region inference profile ID, which Bedrock accepts in place of a model ID. Without a model, the first call fails with a permanent `Model not set` error. Building the client does not check it.

The client implements `ChatCompletion` (`complete` and `complete_stream`), `SimplePrompt` and `DynStructuredPrompt`. It does not implement `EmbeddingModel`, so pair it with [FastEmbed](/integrations/fastembed/) or another embedder for vectors.

## Credentials and region

Without `.client(..)`, the builder loads the AWS configuration with `aws_config::load_from_env`. That is the AWS SDK (software development kit) credential chain: environment variables, shared profiles, single sign-on (SSO) and IAM (Identity and Access Management) roles. Credentials from `aws login` (AWS Console sign-in) are not supported; use a profile, SSO or environment variables, or pass your own client. The region comes from the same chain, for example `AWS_REGION` or your profile. Your account also needs access to the model you name.

The builder runs that loading step inside `tokio::task::block_in_place` when you call `build`. So `build` needs a multi-threaded Tokio runtime. `#[tokio::main]` gives you one. `#[tokio::test]` defaults to a current-thread runtime, where `build` panics; use `#[tokio::test(flavor = "multi_thread")]` there.

To control the configuration yourself, build an `aws_sdk_bedrockruntime::Client` and pass it to `AwsBedrockBuilder::client`. Swiftide does not re-export the SDK, so add `aws-config` and `aws-sdk-bedrockruntime` to your own crate at the major versions Swiftide uses (1.x). A client you pass in also skips the blocking load, so `build` also works on a current-thread runtime.

## Pipeline summaries

::snippet{src="@swiftide/examples/aws_bedrock.rs" anchor="pipeline" title="examples/aws_bedrock.rs"}

`MetadataSummary` takes any `SimplePrompt`, so `AwsBedrock` works where other examples use `OpenAI`. Each chunk costs one Converse call, so wrap the client in `LanguageModelWithBackOff` for large inputs.

`from_chunk_range(100..512)` drops chunks shorter than 100 characters, so short README sections never reach the model or the storage. The upper bound of the range is exclusive.

## Agents with Claude

An agent needs the `swiftide-agents` feature flag, and tool arguments need `schemars` and `serde` in your crate:

```sh
cargo add swiftide@{{swiftide_version}} --features aws-bedrock,swiftide-agents
cargo add schemars@1 serde anyhow tokio --features serde/derive,tokio/macros,tokio/rt-multi-thread
```

Define the tools:

::snippet{src="@swiftide/examples/aws_bedrock_agent.rs" anchor="tools" title="examples/aws_bedrock_agent.rs"}

The `use` lines sit above the excerpt in the linked example source. They bring in `anyhow::Result`, `schemars::JsonSchema`, serde's `Serialize` and `Deserialize`, and from Swiftide `agents`, `ToolOutput`, `ToolError`, `AgentContext`, `Command` and `aws_bedrock_v2::AwsBedrock`.

Build the client and the agent, then run it:

::snippet{src="@swiftide/examples/aws_bedrock_agent.rs" anchor="setup" title="examples/aws_bedrock_agent.rs"}

::snippet{src="@swiftide/examples/aws_bedrock_agent.rs" anchor="agent" title="examples/aws_bedrock_agent.rs"}

::snippet{src="@swiftide/examples/aws_bedrock_agent.rs" anchor="run" title="examples/aws_bedrock_agent.rs"}

`current_utc_time` runs `date` through the tool executor of the agent's context, which is local by default. `format_timestamp` takes a struct argument, and its doc comments become field descriptions in the tool's JavaScript Object Notation (JSON) Schema. The loop counter behind `limit(6)` resets when the agent stops, so a second `query` gets six loops again.

On the way to Bedrock, Swiftide turns the request into Converse messages:

- Every tool is sent with automatic tool choice and strict schema validation. Set `tool_strict` to `false` to turn validation off.
- Every system message becomes a Converse system block.
- Adjacent tool outputs go back together in one user message.
- Each `ChatMessage::Reasoning` in the message history becomes its own assistant message, sent before the assistant text or tool use it belongs to. Reasoning without both text and a signature, such as redacted reasoning, is dropped.

On the way back, the client returns Claude's reasoning in the `reasoning` field of `ChatCompletionResponse`. The agent then adds it to the message history as `ChatMessage::Reasoning`.

## Options

Set defaults with `AwsBedrockBuilder::default_options`, or change them later with `options_mut`. [AWS Bedrock settings](/models/overview/#aws-bedrock-settings) lists the fields, the merge rules and the `reasoning_effort` beta header. A few details only matter here:

| Field | Effect |
| --- | --- |
| `prompt_model` | model ID or ARN; `default_prompt_model` sets it |
| `max_tokens`, `temperature`, `top_p`, `stop_sequences` | unset fields are not sent |
| `additional_model_response_field_paths` | JSON Pointer paths of model-specific response fields to return |

`additional_model_request_fields` takes an `aws_smithy_types::Document`. Neither Swiftide nor the SDK re-exports that type, so add `aws-smithy-types` (1.x) to your crate to set it.

`ReasoningEffort::Max` is meant for Claude Opus 4.6 with adaptive thinking. Set `thinking.type` to `adaptive` through `additional_model_request_fields` to use it.

### Reasoning and beta headers

The swiftide examples do not set these fields, so this section names them without code. Build the value with `Options::builder()` and pass the result of `build()` to `default_options`. The setters take the value itself, not an `Option`:

- `reasoning_effort` takes `swiftide::integrations::aws_bedrock_v2::ReasoningEffort` (`Low`, `Medium`, `High` or `Max`). Swiftide writes it to `output_config.effort` in the model request fields.
- `tool_strict` takes a `bool`. `false` sends tools without strict schema validation.
- `anthropic_beta` takes a list of beta header names. Swiftide adds them to the `anthropic_beta` array in the model request fields, without duplicates.
- `additional_model_request_fields` takes the `Document` that the two fields above are merged into.

See [`OptionsBuilder` (AWS Bedrock)](docsrs:swiftide/integrations/aws_bedrock_v2/struct.OptionsBuilder.html) for every setter.

## Structured prompts

`AwsBedrock` implements `DynStructuredPrompt`, so `structured_prompt::<T>` from `StructuredPrompt` works through the blanket implementation. Swiftide sends the schema as a Converse output configuration with a JSON Schema text format, then parses the reply into JSON. Whether a model accepts that format is up to Bedrock. [Structured output](/models/structured-output/) shows the call.

## Errors and usage

Throttling, unavailable services, models that are not ready, model timeouts, internal server errors, network failures, timeouts and unreadable responses are transient. For `complete_stream`, a model stream error is transient too. An error in the middle of a stream is transient only for throttling, unavailable services, internal server errors and model stream errors. Everything else is permanent. A response that stops because the context window is full, with no text, tool calls or reasoning, returns `ContextLengthExceeded`. Wrap the client in `LanguageModelWithBackOff` to retry transient errors; see [Errors, backoff, usage and tokens](/models/errors-backoff-and-usage/). The wrapper implements `SimplePrompt` and `ChatCompletion` but not `DynStructuredPrompt`, so a wrapped client loses `structured_prompt`.

`on_usage` and `on_usage_async` receive the token `Usage` of each call. Its `cached_tokens` holds the cache-read input tokens. When there are none, it holds the cache-write tokens instead, so a first call that writes the prompt cache also reports cached input. Both callbacks run before the call returns; an `Err` from either fails the call.

## Reference

- [`AwsBedrock`](docsrs:swiftide/integrations/aws_bedrock_v2/struct.AwsBedrock.html)
- [`AwsBedrockBuilder`](docsrs:swiftide/integrations/aws_bedrock_v2/struct.AwsBedrockBuilder.html)
- [`Options` (AWS Bedrock)](docsrs:swiftide/integrations/aws_bedrock_v2/struct.Options.html)
- [`ReasoningEffort` (AWS Bedrock)](docsrs:swiftide/integrations/aws_bedrock_v2/enum.ReasoningEffort.html)
- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`DynStructuredPrompt`](docsrs:swiftide/traits/trait.DynStructuredPrompt.html)
- [`StructuredPrompt`](docsrs:swiftide/traits/trait.StructuredPrompt.html)
- [`MetadataSummary`](docsrs:swiftide/indexing/transformers/struct.MetadataSummary.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`AgentBuilder`](docsrs:swiftide/agents/struct.AgentBuilder.html)
- [`tool`](docsrs:swiftide/attr.tool.html)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)
- [`LanguageModelError`](docsrs:swiftide/chat_completion/errors/enum.LanguageModelError.html)

## Next

[FastEmbed](/integrations/fastembed/)
