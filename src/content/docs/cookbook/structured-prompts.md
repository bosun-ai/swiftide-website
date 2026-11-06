---
title: "Structured prompts"
description: "Get a typed Rust struct back from one model call with structured_prompt, or a JSON value through DynStructuredPrompt when you hold a trait object."
sidebar:
  order: 4
identLint:
  ignore: [JsonSchema, schema_for]
---

This recipe asks an OpenAI model for three questions about Rust and reads them from a `Vec<String>` inside your own struct. A second call gets the same answer through a trait object. The code comes from the `structured-prompt` example in the swiftide repository.

## Setup

Add Swiftide with the `openai` feature flag. Add schemars, serde and serde_json for the response type, and tokio and anyhow for `main`:

```sh
cargo add swiftide@{{swiftide_version}} --features openai
cargo add schemars@1 serde serde_json --features serde/derive
cargo add anyhow
cargo add tokio --features macros,rt-multi-thread
```

Set `OPENAI_API_KEY` and run the example from the root of the swiftide repository:

```sh
cargo run -p swiftide-examples --example structured-prompt
```

## Imports and client

The example needs no extra setup inside the swiftide repository. Your own binary starts like this:

::snippet{src="@snippets/src/cookbook/structured_prompts.rs" anchor="imports"}

`structured_prompt::<T>` is a method of the `StructuredPrompt` trait. The import `StructuredPrompt as _` brings the method into scope without the name. Leave it out and the call fails to compile with `no method named structured_prompt`.

`OpenAI` is a type alias of `GenericOpenAI` for the OpenAI endpoint. A structured prompt uses the default prompt model, so set one with `default_prompt_model`. Without it the call fails with `LanguageModelError::PermanentError("Model not set")`.

## Response type

::snippet{src="@swiftide/examples/structured_prompt.rs" anchor="schema" title="examples/structured_prompt.rs"}

Swiftide turns the struct into a JavaScript Object Notation (JSON) Schema with `schemars::schema_for!` and sends that schema with the prompt. OpenAI checks the schema in strict mode, which is why the struct carries `#[serde(deny_unknown_fields)]`. [Schema rules](/models/structured-output/#schema-rules) lists what strict mode rejects, including `Option<T>` fields.

## Typed call

::snippet{src="@swiftide/examples/structured_prompt.rs" anchor="prompt" title="examples/structured_prompt.rs"}

The method takes a `Prompt`. `.into()` turns a `&'static str` or a `String` into one.

The client sends the rendered prompt as one user message, with no system message. It then deserializes the JSON the model returns into `MyResponse`. A reply that does not fit the type fails with `LanguageModelError::PermanentError`.

Swiftide never retries a `structured_prompt` call, including transient and rate-limit errors. `LanguageModelWithBackOff` does not implement `StructuredPrompt`, so you call the inner client and handle retries yourself. [Errors, backoff, usage and tokens](/models/errors-backoff-and-usage/) covers the decorator.

## Templated prompts

The docs-qa showcase grades answers with a structured prompt. It renders a template and wraps the call in its own retry helper:

::snippet{src="@swiftide/examples/docs-qa/src/judge.rs" anchor="judge" title="examples/docs-qa/src/judge.rs"}

`Prompt::from` takes the template, and each `with_context_value` fills one variable. [Prompts and templates](/models/prompts-and-templates/) covers the syntax. The prompt is cloned on every attempt because `structured_prompt` takes it by value.

The response type nests structs, and each one carries `#[serde(deny_unknown_fields)]`:

::snippet{src="@swiftide/examples/docs-qa/src/judge.rs" anchor="judge_schema" title="examples/docs-qa/src/judge.rs"}

Strict mode applies to every object in the schema, not only the outer one. A nested struct without the attribute makes OpenAI reject the request.

## Trait objects

::snippet{src="@swiftide/examples/structured_prompt.rs" anchor="dynamic" title="examples/structured_prompt.rs"}

`structured_prompt` is generic over `T`, so `StructuredPrompt` is not dyn safe. You cannot hold a `Box<dyn StructuredPrompt>`.

`DynStructuredPrompt` is the object-safe variant. Its `structured_prompt_dyn` takes the prompt and a `schemars::Schema`, and returns a `serde_json::Value`. You pass the schema and decode the value yourself.

Use the trait object when the client is chosen at runtime, for example OpenAI in one deployment and Amazon Web Services (AWS) Bedrock in another.

## Variations

### Other providers

The OpenAI-compatible clients and `AwsBedrock` support structured prompts, each behind its own feature flag, and `Anthropic` does not. [Provider support](/models/structured-output/#provider-support) lists the clients and their flags.

### Responses API

With `use_responses_api(true)` on the OpenAI builder, `structured_prompt` goes through the Responses application programming interface (API) instead of Chat Completions. The schema rules stay the same. [OpenAI Responses API and reasoning](/cookbook/responses-api-and-reasoning/) shows that client.

### Agents

An agent talks to its model through chat completion, not `structured_prompt`. For a typed final answer, replace the default stop tool with `StopWithArgs`, behind the `swiftide-agents` feature flag; [Agents](/models/structured-output/#agents) shows how. Call `without_default_stop_tool()` or register it with `add_tool`, because `tools()` keeps the default `stop` tool and drops yours.

## Reference

- [`StructuredPrompt`](docsrs:swiftide/traits/trait.StructuredPrompt.html)
- [`DynStructuredPrompt`](docsrs:swiftide/traits/trait.DynStructuredPrompt.html)
- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)
- [`GenericOpenAI`](docsrs:swiftide/integrations/openai/struct.GenericOpenAI.html)
- [`AwsBedrock`](docsrs:swiftide/integrations/aws_bedrock_v2/struct.AwsBedrock.html)
- [`LanguageModelError`](docsrs:swiftide/chat_completion/errors/enum.LanguageModelError.html)
- [`LanguageModelWithBackOff`](docsrs:swiftide/indexing/struct.LanguageModelWithBackOff.html)
- [`StopWithArgs`](docsrs:swiftide/agents/tools/control/struct.StopWithArgs.html)

## Next

[Describing images](/cookbook/describe-images/)
