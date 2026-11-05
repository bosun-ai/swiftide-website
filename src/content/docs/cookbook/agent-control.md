---
title: "Agent control recipes"
description: "Resume an agent from saved message history, end a run with a typed stop or failure payload, and give a tool a hand-written JSON Schema."
sidebar:
  order: 1
identLint:
  ignore: [task_failed, "Value::String"]
---

Four recipes give you an agent that continues a saved conversation, an agent that ends with a typed report or a typed failure, and a tool whose argument schema you write yourself.

## Setup

Add Swiftide with the OpenAI integration and agents, plus the crates the recipes use:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,swiftide-agents
cargo add schemars@1 serde serde_json --features serde/derive
cargo add tokio --features full
cargo add anyhow async-trait
cargo add rustls@0.23 --no-default-features --features ring,std
```

Swiftide selects no Transport Layer Security (TLS) crypto provider. Your binary installs the rustls provider at the start of `main` (see [Crypto provider panic](/reference/troubleshooting-and-faq/#crypto-provider-panic)). The examples crate in the swiftide repository gets one through another dependency, so the examples run without that step.

Every agent below borrows an OpenAI client built like this:

::snippet{src="@swiftide/examples/agents_resume.rs" anchor="setup"}

Most recipes are examples in the swiftide repository. To run them, clone it and set an OpenAI key:

```sh
git clone https://github.com/bosun-ai/swiftide && cd swiftide
export OPENAI_API_KEY=sk-...
```

The examples crate turns on most feature flags, so the first build takes several minutes. Each recipe ends with its run command.

## Resumed agents

A conversation that has to survive a restart needs its message history stored. The first agent answers one query:

::snippet{src="@swiftide/examples/agents_resume.rs" anchor="agent"}

Then the example stores its history and loads it into a new context:

::snippet{src="@swiftide/examples/agents_resume.rs" anchor="history"}

A second agent continues from that context:

::snippet{src="@swiftide/examples/agents_resume.rs" anchor="resume"}

`Agent::history` returns the message history as a `Vec<ChatMessage>`, and `ChatMessage` round-trips through JSON (JavaScript Object Notation). `DefaultContext::with_existing_messages` replaces the context's message history with the messages you pass. [Message history](/agents/context-and-history/#message-history) explains where those messages live.

A fresh agent appends its system prompt to the context on its first run, after the restored messages. If the stored history already starts with a system message, that adds a second one. The example's first agent has no system prompt, and `no_system_prompt()` keeps the second agent the same. The comment in the `resume` excerpt dates from before 0.33, when agents had a default system prompt.

```sh
cargo run -p swiftide-examples --example agents-resume
```

### Message history backends

`DefaultContext::with_message_history` takes any `MessageHistory` backend, so you do not have to serialize the history yourself. [Redis storage](/agents/context-and-history/#redis-storage) shows the `Redis` backend.

## Typed stop payloads

When the caller needs a structured final answer, describe it as a type:

::snippet{src="@snippets/src/cookbook/agent_control.rs" anchor="stop_payload"}

Turn its schema into the stop tool, register it in place of the default one and read the payload when the agent stops:

::snippet{src="@snippets/src/cookbook/agent_control.rs" anchor="typed_stop"}

`StopWithArgs::with_parameters_schema` builds a tool named `stop` and forwards the whole argument object as the payload. The `on_stop` hook receives `StopReason::RequestedByTool(tool_call, Some(payload))`. A `&serde_json::Value` is a deserializer, so `StopPayload::deserialize(&payload)` turns it into your type. Why `without_default_stop_tool()` comes before `tools`: [Stopping and structured results](/agents/stopping-and-structured-results/#structured-results).

The OpenAI client marks every property as required when it sends a tool schema. The model sends `details` even when it is optional, which is why the system prompt asks for `null`. `query_once` runs one completion and the tool calls it returns, then hands control back.

Every field needs a schema that is an object. A field of type `serde_json::Value` or `Option<serde_json::Value>` without a description produces the schema `true`, and `tool_spec()` panics with `NodeMustBeObject`. The `stop_with_args_custom_schema.rs` example in the repository declares `details` that way, so at this version it panics before it reaches the model. Use a concrete type, or add `#[schemars(description = "...")]` to the field.

### Payload after the run

You can also read the payload after `query_once` returns. `Agent::stop_reason` returns the reason of the last stop:

::snippet{src="@snippets/src/cookbook/agent_control.rs" anchor="read_stop_payload"}

## Typed failure payloads

A model that has to give up can hand your code a report to route, for example to an on-call queue. Describe the report:

::snippet{src="@swiftide/examples/agent_can_fail_custom_schema.rs" anchor="schema"}

`AgentCanFail::with_parameters_schema` turns that schema into the arguments of the `task_failed` tool. The example prints the resulting tool spec:

::snippet{src="@swiftide/examples/agent_can_fail_custom_schema.rs" anchor="tools"}

The agent keeps the default stop tool and adds `task_failed`, so the model can either finish or fail. The `on_stop` hook prints the failure payload:

::snippet{src="@swiftide/examples/agent_can_fail_custom_schema.rs" anchor="agent"}

::snippet{src="@swiftide/examples/agent_can_fail_custom_schema.rs" anchor="run"}

The run returns `Ok` even when the agent fails, and the payload is JSON text inside a `Value::String` ([Failure reports](/agents/stopping-and-structured-results/#failure-reports) explains both). Decode it after the run:

::snippet{src="@snippets/src/cookbook/agent_control.rs" anchor="read_failure_payload"}

Your reading type can hold fewer fields than the schema type, because serde ignores unknown fields unless you deny them.

```sh
cargo run -p swiftide-examples --example agent-can-fail-custom-schema
```

## Hand-written tool schemas

`#[swiftide::tool]` cannot express every argument shape, and it gives you no say over the schema title, field descriptions or enum values. A hand-written tool can. Describe the arguments:

::snippet{src="@swiftide/examples/tool_custom_schema.rs" anchor="schema"}

Implement `Tool` and return the schema from `tool_spec`:

::snippet{src="@swiftide/examples/tool_custom_schema.rs" anchor="tools"}

::snippet{src="@swiftide/examples/tool_custom_schema.rs" anchor="run"}

The `#[schemars(description = ...)]` on `payload` matters here. Without it, the `serde_json::Value` field gets the schema `true` and `ToolSpec::builder().build()` returns an error. [Hand-written tools](/agents/tools/#hand-written-tools) lists the other schema rules and why `name()` must match the spec name.

Swiftide has no blanket conversion from a `Tool` into `Box<dyn Tool>`, so `.tools([WorkflowTool])` does not compile. Register a hand-written tool with `add_tool`, or box it yourself:

::snippet{src="@snippets/src/cookbook/agent_control.rs" anchor="register_hand_written_tool"}

This example needs no API key. It prints the tool spec as JSON:

```sh
cargo run -p swiftide-examples --example tool-custom-schema
```

## Reference

- [`Agent`](docsrs:swiftide/agents/struct.Agent.html), [`Agent::history`](docsrs:swiftide/agents/struct.Agent.html#method.history) and [`Agent::stop_reason`](docsrs:swiftide/agents/struct.Agent.html#method.stop_reason)
- [`AgentBuilder`](docsrs:swiftide/agents/struct.AgentBuilder.html): [`without_default_stop_tool`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.without_default_stop_tool), [`no_system_prompt`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.no_system_prompt), [`add_tool`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.add_tool)
- [`DefaultContext`](docsrs:swiftide/agents/struct.DefaultContext.html): [`with_existing_messages`](docsrs:swiftide/agents/struct.DefaultContext.html#method.with_existing_messages), [`with_message_history`](docsrs:swiftide/agents/struct.DefaultContext.html#method.with_message_history)
- [`MessageHistory`](docsrs:swiftide/traits/trait.MessageHistory.html) and [`Redis`](docsrs:swiftide/integrations/redis/struct.Redis.html)
- [`ChatMessage`](docsrs:swiftide/chat_completion/enum.ChatMessage.html)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`StopWithArgs`](docsrs:swiftide/agents/tools/control/struct.StopWithArgs.html) and [`AgentCanFail`](docsrs:swiftide/agents/tools/control/struct.AgentCanFail.html)
- [`Tool`](docsrs:swiftide/chat_completion/trait.Tool.html) and [`Tool::boxed`](docsrs:swiftide/chat_completion/trait.Tool.html#method.boxed), [`ToolSpec`](docsrs:swiftide/chat_completion/struct.ToolSpec.html) and [`ToolSpecBuilder`](docsrs:swiftide/chat_completion/struct.ToolSpecBuilder.html)
- [`ToolCall`](docsrs:swiftide/chat_completion/struct.ToolCall.html) and [`ToolOutput`](docsrs:swiftide/chat_completion/enum.ToolOutput.html)

## Next

[OpenAI Responses API and reasoning](/cookbook/responses-api-and-reasoning/)
