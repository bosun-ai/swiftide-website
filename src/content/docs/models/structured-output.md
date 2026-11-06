---
title: "Structured output"
description: "Ask a model for a Rust type with structured_prompt::<T>, where T derives schemars JsonSchema, and use DynStructuredPrompt when you need a trait object."
sidebar:
  order: 3
identLint:
  ignore: [ReleaseSummary, JsonSchema, response_format]
---

By the end of this page, a model call returns your own Rust struct. The provider shapes its answer to the struct's JavaScript Object Notation (JSON) Schema.

Bring the types and traits into scope:

::snippet{src="@snippets/src/models/structured_output.rs" anchor="imports"}

Describe the answer as a type:

::snippet{src="@snippets/src/models/structured_output.rs" anchor="release_summary_type"}

Then ask the model for it:

::snippet{src="@snippets/src/models/structured_output.rs" anchor="structured_prompt"}

## Schema and deserialization

The doc comments on the fields become descriptions in the schema, so the model reads them as instructions.

`structured_prompt::<T>` is a method of the `StructuredPrompt` trait, so the trait must be in scope. The method takes a `Prompt`, so `.into()` turns a `String` or a `&'static str` into one. Templates and template context work as on [Prompts and templates](/models/prompts-and-templates/).

Swiftide builds the schema with `schemars::schema_for!(T)` and sends it with the rendered prompt. It then deserializes the JSON the model returns into `T`. When the JSON does not fit `T`, the call returns `LanguageModelError::PermanentError`.

You need schemars, serde and serde_json in your own crate:

```sh
cargo add swiftide@{{swiftide_version}} --features openai
cargo add schemars@1 serde serde_json --features serde/derive
```

`structured_prompt` itself needs no extra feature flag. The `json-schema` feature only derives `JsonSchema` on some of Swiftide's own types.

### Schemas for Swiftide types

With `json-schema`, `ToolCall`, `ToolSpec`, `ToolFeedback`, `StopReason` and the agent `State` derive `JsonSchema`. Your own types can then hold them as fields and still derive a schema, for example the request body of an approval endpoint:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,swiftide-agents,json-schema
```

::snippet{src="@snippets/src/models/structured_output.rs" anchor="swiftide_type_schema"}

`StopReason` and `State` live in `swiftide::agents`, so they also need the `swiftide-agents` feature flag. The `json-schema` feature adds no API of its own.

## Schema rules

OpenAI clients send the schema in strict mode. This holds for Chat Completions and for the Responses application programming interface (API). Strict mode has two rules for every object in the schema.

The first rule is `additionalProperties: false`. Add `#[serde(deny_unknown_fields)]` to the struct, and to every struct it contains. Schemars then writes `additionalProperties: false` for that object. Without it, OpenAI rejects the request with an error about `additionalProperties`.

The second rule is that every field is listed in `required`. Schemars leaves `Option<T>` fields and `#[serde(default)]` fields out of `required`, so OpenAI rejects them. Use a required field with a sentinel value, or an enum, instead.

Swiftide passes the schemars schema through as it is. It does not rewrite the schema to fit a provider, so the provider's rules on schemas apply to your type directly.

## Trait objects

`StructuredPrompt` is generic over `T`, so it is not dyn safe. You cannot store a `Box<dyn StructuredPrompt>`. Use `DynStructuredPrompt` instead:

::snippet{src="@snippets/src/models/structured_output.rs" anchor="dyn_structured_prompt"}

`structured_prompt_dyn` takes a `schemars::Schema` and returns a `serde_json::Value`. You deserialize the value yourself.

Every `DynStructuredPrompt` gets `StructuredPrompt` through a blanket implementation. To add structured output to your own client, implement `DynStructuredPrompt` only.

## Provider support

| Client | Feature flag | How the schema is sent |
| --- | --- | --- |
| `OpenAI` | `openai` | Chat Completions `response_format` with a JSON Schema. With `use_responses_api(true)`, the Responses API text format. |
| `Ollama`, `Groq`, `Gemini`, `OpenRouter`, `Dashscope` | `ollama`, `groq`, `gemini`, `open-router`, `dashscope` | Same as `OpenAI`. |
| `AwsBedrock` | `aws-bedrock` | Converse `output_config` with a JSON Schema. |
| `Anthropic` | `anthropic` | Not supported. |
| `FastEmbed` | `fastembed` | Not supported. |

Whether a JSON Schema response format works on the `GenericOpenAI` aliases depends on the endpoint and the model.

A structured call is not retried. [Errors, backoff, usage and tokens](/models/errors-backoff-and-usage/) explains why.

## Agents

An agent talks to its model through chat completion, not through `structured_prompt`. To get a typed final answer from an agent, replace the default stop tool with `StopWithArgs`. Agents live behind the `swiftide-agents` feature flag:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,swiftide-agents
```

::snippet{src="@snippets/src/models/structured_output.rs" anchor="agent_stop_schema"}

The model ends the run by calling `stop` with arguments that follow the `ReleaseSummary` schema. The stop reason is then `StopReason::RequestedByTool(_, Some(payload))`, where `payload` is the arguments as a `serde_json::Value`. `serde_json::from_value` turns it into your type.

The stop tool's schema follows different rules than `structured_prompt`. Swiftide normalizes tool schemas for OpenAI: it fills in `required` and adds `additionalProperties: false`. [Stopping and structured results](/agents/stopping-and-structured-results/) covers stop tools.

Tool arguments use schemars schemas too, so the same derive works for a tool's arguments; see [Tools](/agents/tools/).

## Reference

- [`StructuredPrompt`](docsrs:swiftide/traits/trait.StructuredPrompt.html)
- [`DynStructuredPrompt`](docsrs:swiftide/traits/trait.DynStructuredPrompt.html)
- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)
- [`GenericOpenAI`](docsrs:swiftide/integrations/openai/struct.GenericOpenAI.html)
- [`AwsBedrock`](docsrs:swiftide/integrations/aws_bedrock_v2/struct.AwsBedrock.html)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`AgentBuilder`](docsrs:swiftide/agents/struct.AgentBuilder.html)
- [`StopWithArgs`](docsrs:swiftide/agents/tools/control/struct.StopWithArgs.html)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`LanguageModelError`](docsrs:swiftide/chat_completion/errors/enum.LanguageModelError.html)

## Next

[Images and multimodal input](/models/images/)
