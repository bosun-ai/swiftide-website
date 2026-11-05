---
title: "Agents on AWS Bedrock"
description: "Run a Swiftide agent with two tools on a Claude model hosted by Amazon Web Services (AWS) Bedrock, and read why it stopped."
sidebar:
  order: 3
identLint:
  ignore: [JsonSchema]
---

The agent on this page runs on a Claude model hosted by Amazon Web Services (AWS) Bedrock, calls two tools and then stops on its own. Client setup, credentials, region and runtime requirements are on [AWS Bedrock](/integrations/aws-bedrock/). This page covers what changes once that client drives an agent.

## Setup

Bedrock sits behind the `aws-bedrock` feature flag, and agents behind `swiftide-agents`:

```sh
cargo add swiftide@{{swiftide_version}} --features aws-bedrock,swiftide-agents
cargo add schemars@1 serde --features serde/derive
cargo add tokio --features macros,rt-multi-thread
cargo add anyhow
```

The full example lives in the swiftide repository. To run it, clone the repository and start it from the root:

```sh
git clone https://github.com/bosun-ai/swiftide && cd swiftide
cargo run -p swiftide-examples --example aws-bedrock-agent
```

The `swiftide-examples` package turns on the `all` feature flag. Its first build compiles every integration, including LanceDB, FastEmbed, Kafka and Fluvio. Those need native build tools such as `protoc` and `cmake`. If you only want Bedrock, copy the snippets below into your own crate instead.

## Tools

::snippet{src="@swiftide/examples/aws_bedrock_agent.rs" anchor="tools" title="examples/aws_bedrock_agent.rs"}

The tools import `swiftide::chat_completion::{ToolOutput, errors::ToolError}` and `swiftide::traits::{AgentContext, Command}`. The client comes from `swiftide::integrations::aws_bedrock_v2::AwsBedrock`.

`#[swiftide::tool]` turns each async function into a tool. It also generates a function with the same name that returns a `Box<dyn Tool>`. The `description` attribute becomes the tool description the model reads.

`current_utc_time` runs a shell command through the tool executor of the agent's context. The default context uses a `LocalExecutor`, so `date` runs on your machine. `exec_cmd` returns a `CommandOutput`, which keeps stdout and stderr as separate chunks since 0.33. `.into()` turns it into `ToolOutput::Text`, with all output as text in the order it arrived.

`format_timestamp` takes one parameter, `request`, of type `FormatTimestampRequest`. The model sends the arguments as a JavaScript Object Notation (JSON) object with a `request` field. That field holds `prefix` and `timestamp`. The struct derives `JsonSchema`, so its doc comments become the field descriptions in the schema. A struct used as a tool parameter needs `Serialize`, `Deserialize`, `JsonSchema` and `Debug`.

Bedrock receives every tool spec with strict schema validation turned on. That comes from the `tool_strict` option, which is `true` when you leave it unset. To turn it off, pass `Options::builder().tool_strict(false).build()?` to `AwsBedrockBuilder::default_options`.

## Agent

::snippet{src="@swiftide/examples/aws_bedrock_agent.rs" anchor="setup" title="examples/aws_bedrock_agent.rs"}

::snippet{src="@swiftide/examples/aws_bedrock_agent.rs" anchor="agent" title="examples/aws_bedrock_agent.rs"}

`llm(&bedrock)` gives the agent the Bedrock client. `tools` registers both tools next to the default `stop` tool.

The `on_new_message` hook runs before each message enters the message history. It receives the agent and the message. The future it returns may not borrow the message, so the example renders the message to a string first. The `Display` form of `ChatMessage` shows the role, the text and any tool calls.

`limit(6)` caps the agent at six completions. Turns that only run pending tool calls do not count. At the cap, Swiftide logs "Agent loop limit reached" and stops the agent with `StopReason::NoNewMessages`.

## Run

::snippet{src="@swiftide/examples/aws_bedrock_agent.rs" anchor="run" title="examples/aws_bedrock_agent.rs"}

`query` adds the prompt as a user message and runs the loop. The prompt asks the model to call `current_utc_time`, then `format_timestamp`, then report the result and stop. The model decides how it finishes, and the two ways lead to different stop reasons:

- It answers in text without a tool call. The default context then has no new messages to send, and the agent stops with `StopReason::NoNewMessages`.
- It calls the default `stop` tool. The agent stops with `StopReason::RequestedByTool`, which carries the tool call.

After `query` returns, `agent.stop_reason()` tells you which one happened. Branch on it when your code needs to know whether the model finished or the loop ran out.

## Variations

- To make the model always finish through a tool, keep the `stop` tool and say so in the prompt. To remove it, call `without_default_stop_tool` on the builder before `tools`, or register your own tool named `stop` with `add_tool`. A tool named `stop` passed to `tools` is dropped, because the default one is already in the set.
- To keep the agent going after a text answer, give it a `DefaultContext` with `with_stop_on_assistant(false)`. Then only the `stop` tool or the limit ends the run.
- A second `query` on the same agent starts with a fresh loop counter, so it gets six completions again.
- Bedrock has its own `Options` and its own `ReasoningEffort` with `Low`, `Medium`, `High` and `Max`. They are not the OpenAI types. See [AWS Bedrock settings](/models/overview/#aws-bedrock-settings).
- If your binary also builds a Swiftide client that uses Hypertext Transfer Protocol (HTTP), such as `OpenAI`, install a rustls crypto provider at the start of `main`. See [Crypto provider panic](/reference/troubleshooting-and-faq/#crypto-provider-panic).

## Gotchas

- The agent needs a model that supports tool use through the Converse API.
- `#[swiftide::tool]` reads the parameter names from the function signature. Renaming `request` changes the JSON field the model must send.

## Reference

- [AWS Bedrock integration](/integrations/aws-bedrock/)
- [`AwsBedrock`](docsrs:swiftide/integrations/aws_bedrock_v2/struct.AwsBedrock.html)
- [`AwsBedrockBuilder`](docsrs:swiftide/integrations/aws_bedrock_v2/struct.AwsBedrockBuilder.html)
- [`Options` (AWS Bedrock)](docsrs:swiftide/integrations/aws_bedrock_v2/struct.Options.html)
- [`ReasoningEffort` (AWS Bedrock)](docsrs:swiftide/integrations/aws_bedrock_v2/enum.ReasoningEffort.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`AgentBuilder`](docsrs:swiftide/agents/struct.AgentBuilder.html)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`DefaultContext`](docsrs:swiftide/agents/struct.DefaultContext.html)
- [`Stop`](docsrs:swiftide/agents/tools/control/struct.Stop.html)
- [`LocalExecutor`](docsrs:swiftide/agents/tools/local_executor/struct.LocalExecutor.html)
- [`#[swiftide::tool]`](docsrs:swiftide/attr.tool.html)
- [`AgentContext`](docsrs:swiftide/traits/trait.AgentContext.html)
- [`Command`](docsrs:swiftide/traits/enum.Command.html)
- [`CommandOutput`](docsrs:swiftide/traits/struct.CommandOutput.html)
- [`ToolOutput`](docsrs:swiftide/chat_completion/enum.ToolOutput.html)
- [`ChatMessage`](docsrs:swiftide/chat_completion/enum.ChatMessage.html)

## Next

[Structured prompts](/cookbook/structured-prompts/)
