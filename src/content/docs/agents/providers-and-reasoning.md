---
title: "Providers and reasoning"
description: "Run one agent definition on OpenAI, Azure, Anthropic, Ollama, AWS Bedrock or any OpenAI-compatible gateway, keep reasoning items across turns, and know how each provider treats tool schemas."
sidebar:
  order: 11
identLint:
  ignore: [block_in_place]
---

You end up with one agent definition that runs on whichever provider you pick at startup, with the model's reasoning carried from one turn to the next.

::snippet{src="@snippets/src/agents/providers.rs" anchor="build_agent"}

::snippet{src="@snippets/src/agents/providers.rs" anchor="provider_swap"}

`AgentBuilder::llm` takes a reference to any client that implements `ChatCompletion + Clone + 'static`. It stores a boxed clone, so `Agent` has no type parameter for the provider. The tools, hooks and system prompt stay the same whichever client you pass.

The Azure client needs the `async-openai` crate as a direct dependency, because Swiftide does not re-export its client type:

```sh
cargo add async-openai@0.41 --no-default-features
```

Use the same version Swiftide depends on, or the client types do not match.

## Chat providers

Each chat client sits behind its own feature flag. The snippet above needs `swiftide-agents` and three provider flags:

```sh
cargo add swiftide@{{swiftide_version}} --features swiftide-agents,openai,anthropic,ollama
```

Every chat client implements `ChatCompletion`. [Providers](/models/overview/#providers) lists each client with its feature flag, and [OpenAI-compatible endpoints](/models/overview/#openai-compatible-endpoints) covers gateways and base URLs.

The ask-your-codebase showcase builds its client from a base URL and a flag for the Responses API, so the same binary runs against OpenAI or a gateway:

::snippet{src="@swiftide/examples/ask-your-codebase/src/llm.rs" anchor="llm" title="examples/ask-your-codebase/src/llm.rs"}

## Bedrock agents

`AwsBedrock` talks to the Bedrock Converse API. It reads AWS credentials and the region from your usual Amazon Web Services (AWS) configuration, such as a command-line interface (CLI) profile or environment variables:

::snippet{src="@swiftide/examples/aws_bedrock_agent.rs" anchor="setup"}

The model ID here is a global inference profile. The agent itself looks like any other:

::snippet{src="@swiftide/examples/aws_bedrock_agent.rs" anchor="agent"}

Setup and credentials are on [AWS Bedrock](/integrations/aws-bedrock/).

## Reasoning items

On `OpenAI`, reasoning models return their reasoning only through the Responses API. `AwsBedrock` returns signed Claude reasoning as `ReasoningItem` values without extra setup. Reasoning on `OpenAI` needs the Responses API and a reasoning effort; [Responses API and reasoning](/models/overview/#responses-api-and-reasoning) covers the client options.

::snippet{src="@swiftide/examples/responses_api_reasoning.rs" anchor="setup"}

Each `ReasoningItem` has a `summary` list and an optional `encrypted_content`. This helper joins the summaries:

::snippet{src="@swiftide/examples/responses_api_reasoning.rs" anchor="reasoning"}

The agent reads them in an `after_completion` hook:

::snippet{src="@swiftide/examples/responses_api_reasoning.rs" anchor="agent"}

After the `after_completion` hooks run, the agent adds every item in `response.reasoning` to the message history as a `ChatMessage::Reasoning`. It adds them before the assistant message, through `Agent::add_message`, so `on_new_message` hooks see them too. A hook that empties `response.reasoning` keeps the items out of the history.

## Reasoning replay

What happens to those items on the next request depends on the client:

- The Responses API client replays them only when `reasoning_effort` is set on that client. It then sends back every item with a non-empty `encrypted_content` and sets `store` to false, so the reasoning lives in your message history and survives a saved and restored history (see [Context and history](/agents/context-and-history/)). Without `reasoning_effort`, it drops every `ChatMessage::Reasoning`.
- `AwsBedrock` replays an item only when it has non-empty `content` and a non-empty `encrypted_content`, which it sends as the signature. It drops anything else.
- The Chat Completions client and `Anthropic` leave `ChatMessage::Reasoning` out of their requests.

## Bedrock reasoning effort

`AwsBedrock` has its own `Options` and its own `ReasoningEffort` enum in `swiftide::integrations::aws_bedrock_v2`. Set `reasoning_effort` in those options and pass them to `default_options` on the builder. For the Opus 4.5 beta header and the other fields, see [AWS Bedrock settings](/models/overview/#aws-bedrock-settings).

## Tool schema policies

`ToolSpec::builder().build()` checks every schema against Swiftide's strict contract (see [Tools](/agents/tools/)) and rewrites what it can: a nullable type such as `["string", "null"]` becomes an `anyOf` with a null branch, a two-entry `oneOf` with a null branch becomes an `anyOf`, annotations next to a `$ref` are dropped, and every object gets `additionalProperties: false`.

Each provider then applies its own policy when it builds a request. Only `AwsBedrock` lets you change it.

- `OpenAI` and the other `GenericOpenAI` clients remove `$schema` and Rust integer formats such as `int32`, and list every property in `required`. They reject any remaining `oneOf` or array-valued `type`, and send the schema with `strict: true`.
- `Anthropic` and `AwsBedrock` send the strict schema unchanged. `AwsBedrock` marks tools strict unless you set `tool_strict` to `false` in its `Options`.

## Gotchas

- The OpenAI schema policy runs when the request is built, not when the tool is built. A schema it rejects fails the first completion with `AgentError::CompletionsFailed`, wrapping a permanent error that says `tool schema must be OpenAI compatible`.
- Gateways do not always speak the Responses API. Against the gateway used for the showcase runs, the Responses API fails because the gateway streams events with `"output": null`, which the client cannot parse. Use it with OpenAI itself.
- OpenAI rejects the `:` in Model Context Protocol (MCP) tool names; see [Tool names](/agents/mcp/#tool-names).
- Reasoning items are tied to the provider that made them. Bedrock items carry a Bedrock signature in `encrypted_content`, and an OpenAI Responses client with `reasoning_effort` set sends that to OpenAI as encrypted reasoning. OpenAI items usually have no `content`, so `AwsBedrock` drops them silently. Remove `ChatMessage::Reasoning` entries from the message history before you switch providers.
- `AwsBedrock::builder().build()` loads the AWS configuration with `block_in_place`, so it needs a multi-threaded Tokio runtime. A default `#[tokio::test]` or `flavor = "current_thread"` panics. Pass your own client with `.client(..)` otherwise.
- A client's `on_usage` callback counts every call on that client, including other agents that share it; see [Usage callbacks](/models/errors-backoff-and-usage/#usage-callbacks).

## Reference

- [`AgentBuilder::llm`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.llm)
- [`AgentBuilder::after_completion`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.after_completion)
- [`AgentBuilder::on_new_message`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.on_new_message)
- [`Agent::add_message`](docsrs:swiftide/agents/struct.Agent.html#method.add_message)
- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`GenericOpenAI`](docsrs:swiftide/integrations/openai/struct.GenericOpenAI.html)
- [`GenericOpenAIBuilder::use_responses_api`](docsrs:swiftide/integrations/openai/struct.GenericOpenAIBuilder.html#method.use_responses_api)
- [`Options` (OpenAI)](docsrs:swiftide/integrations/openai/struct.Options.html)
- [`AzureConfig`](docsrs:swiftide/integrations/openai/struct.AzureConfig.html)
- [`Anthropic`](docsrs:swiftide/integrations/anthropic/struct.Anthropic.html)
- [`Ollama`](docsrs:swiftide/integrations/ollama/struct.Ollama.html)
- [`AwsBedrock`](docsrs:swiftide/integrations/aws_bedrock_v2/struct.AwsBedrock.html)
- [`Options` (AWS Bedrock)](docsrs:swiftide/integrations/aws_bedrock_v2/struct.Options.html)
- [`ReasoningEffort` (AWS Bedrock)](docsrs:swiftide/integrations/aws_bedrock_v2/enum.ReasoningEffort.html)
- [`ChatCompletionResponse`](docsrs:swiftide/chat_completion/struct.ChatCompletionResponse.html)
- [`ChatMessage`](docsrs:swiftide/chat_completion/enum.ChatMessage.html)
- [`ReasoningItem`](docsrs:swiftide/chat_completion/struct.ReasoningItem.html)
- [`ToolSpec`](docsrs:swiftide/chat_completion/struct.ToolSpec.html)
- [`AgentError`](docsrs:swiftide/agents/errors/enum.AgentError.html)

## Next

[Contrib tools and hooks](/agents/contrib/)
