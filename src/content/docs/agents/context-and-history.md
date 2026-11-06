---
title: "Context and message history"
description: "Store an agent's message history in memory or Redis, resume it in a new process, and rewrite it while the agent runs."
sidebar:
  order: 3
identLint:
  ignore: [HashMap]
---

An agent's message history can outlive the process and be rewritten while the agent runs. The `agents_resume.rs` example saves the history of one agent as JSON (JavaScript Object Notation) and loads it into a second one:

::snippet{src="@swiftide/examples/agents_resume.rs" anchor="history"}

::snippet{src="@swiftide/examples/agents_resume.rs" anchor="resume"}

`Agent::history()` returns every `ChatMessage` the context holds. `ChatMessage` implements `Serialize` and `Deserialize`, so `serde_json` turns the history into a string you can store anywhere. `DefaultContext::with_existing_messages` loads the messages back into a fresh context. The second agent gets that context through the builder's `context` method and continues the conversation with `query`. The next completion sends the whole restored history to the model.

The first agent sets no system prompt, so `no_system_prompt()` changes nothing in this example. [Prompt removal](/agents/system-prompts/#prompt-removal) explains when it matters.

`run` without a new message only continues when the history ends in a user message, a tool output or a pending tool call. When it ends in a plain assistant answer, as it does here, `run` stops at once with `StopReason::NoNewMessages`.

## Default context

The context stores the message history, decides when the next completion runs and what it contains, holds the tool executor and records feedback for tools that wait on a person. Every agent built without `.context(..)` gets a `DefaultContext`:

- the message history is an in-memory `Mutex<Vec<ChatMessage>>`;
- tools run commands through a `LocalExecutor` in the current directory;
- the agent stops when the last message is an assistant answer and no tool feedback is pending.

`DefaultContext::from_executor` keeps those defaults and swaps the tool executor. It takes a value that converts into `Arc<dyn ToolExecutor>`. [Tool executors](/agents/executors/) covers the conversion and the tool executors themselves.

`with_stop_on_assistant(false)` turns off the last rule. A text answer then no longer ends the run, and the agent asks the model again. Only a tool that stops the agent, the builder's `limit`, or a model reply with no content and no tool calls ends the loop. An agent has no limit by default, so set one ([Stop reasons](/agents/stopping-and-structured-results/#stop-reasons)).

A clone of a `DefaultContext` shares its history and counters.

## Message history

Where the messages live is the job of the `MessageHistory` trait. It has three required methods:

- `history` returns all messages in order;
- `push_owned` appends one message;
- `overwrite` replaces all messages.

The trait requires `Send + Sync + Debug` and is declared with `#[async_trait]`, so annotate your impl the same way. `push`, `extend` and `extend_owned` have default implementations on top of `push_owned`. Implement the trait for your database and pass the value to `DefaultContext::with_message_history`. The backend only stores the list. Each completion sends the full history, cut at the latest summary, and the context's counters only decide whether a new completion runs.

### Custom backend

This backend keeps one conversation in a JSON file. It uses the `async-trait` crate for the `#[async_trait]` attribute:

::snippet{src="@snippets/src/agents/context_and_history.rs" anchor="custom_history"}

::snippet{src="@snippets/src/agents/context_and_history.rs" anchor="use_custom_history"}

Every method takes `&self`, so the backend guards its own state; here a `tokio::sync::Mutex` keeps two writes from interleaving. `history` returns an empty list for a file that does not exist yet. `with_message_history` takes the value by ownership and stores it behind an `Arc`.

## Redis storage

The `redis` feature flag adds a `Redis` integration that implements `MessageHistory`:

```sh
cargo add swiftide@{{swiftide_version}} --features swiftide-agents,openai,redis
```

::snippet{src="@snippets/src/agents/context_and_history.rs" anchor="redis_history"}

The messages go into a Redis list under `<prefix>:message_history`, here `session-42:message_history`, one JSON string per message. Use one prefix per conversation. A new process that builds a context with the same prefix sees the whole conversation again. Build that agent without `.system_prompt(..)`, or with `no_system_prompt()`, as in the first example. Otherwise its first run pushes another system message onto the list.

The connection opens on first use. If Redis is unreachable, the agent returns `AgentError::MessageHistoryError`.

## Approvals across restarts

Approvals are not part of the message history. They live in the context's memory. When the agent stops with `StopReason::FeedbackRequired`, store the `ToolCall` it carries; the person's `ToolFeedback` comes later. Store each decision as a `(ToolCall, ToolFeedback)` pair, since `serde_json` cannot use a struct as a map key. Both types implement `Serialize` and `Deserialize`. Collect the pairs into a `HashMap` for `DefaultContext::with_tool_feedback` in the new process. [Human in the loop](/agents/human-in-the-loop/) covers approvals.

::snippet{src="@snippets/src/agents/context_and_history.rs" anchor="restore_approvals"}

`with_tool_feedback` accepts anything that converts into a `HashMap<ToolCall, ToolFeedback>` and adds the entries to the feedback the context already holds. The restored history still ends in the pending tool call, so `run` calls the tool again, and the tool now finds its approval.

## History rewrites

Since 0.33, `AgentContext::replace_history` swaps the whole history while the agent runs. It takes `&self`, so any hook can call it through `agent.context()`. This `after_each` hook shortens old tool outputs once the history passes `MAX_MESSAGES`:

::snippet{src="@snippets/src/agents/context_and_history.rs" anchor="replace_history"}

The hook only replaces the text of older tool outputs. Each tool call keeps its output, which the providers require. `DefaultContext` writes the new list through `overwrite` and resets its completion counters. The next completion then runs even if no new message was added since the last one.

## Summaries

A `ChatMessage::Summary` shortens what the model sees without rewriting storage. When the history holds one, `DefaultContext` sends the system messages plus everything from the latest summary on. Add one with `ChatMessage::new_summary` through `agent.context().add_message(..)`. OpenAI-compatible providers send a summary as an assistant message. `ContextCompression` in agents-contrib writes these summaries for you; see [Contrib](/agents/contrib/).

## Custom contexts

For behavior the message history cannot express, implement `AgentContext` yourself. Wrap a `DefaultContext` and forward the calls you do not change. The [upgrading guide](/reference/upgrading/) has a context that counts added messages:

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="custom_context"}

## Gotchas

- `with_existing_messages` calls `overwrite` on the backend. With Redis it deletes the list under the key and writes the given messages.
- `with_existing_messages` and `with_message_history` return `&mut DefaultContext`. Use a `let mut` binding, or end the chain with `.to_owned()` as the example does.
- A builder-made `Redis` uses the key `message_history` unless you set `message_history_key(..)` on the builder or call `with_message_history_key` afterwards. Agents that share that key also share one history.
- Messages added through `agent.context()` do not trigger `on_new_message` hooks. Only `Agent::add_message` runs them.

## Reference

- [`DefaultContext`](docsrs:swiftide/agents/struct.DefaultContext.html)
- [`DefaultContext::with_existing_messages`](docsrs:swiftide/agents/struct.DefaultContext.html#method.with_existing_messages)
- [`DefaultContext::with_message_history`](docsrs:swiftide/agents/struct.DefaultContext.html#method.with_message_history)
- [`DefaultContext::with_tool_feedback`](docsrs:swiftide/agents/struct.DefaultContext.html#method.with_tool_feedback)
- [`AgentContext`](docsrs:swiftide/traits/trait.AgentContext.html)
- [`AgentContext::replace_history`](docsrs:swiftide/traits/trait.AgentContext.html#tymethod.replace_history)
- [`MessageHistory`](docsrs:swiftide/traits/trait.MessageHistory.html)
- [`Redis`](docsrs:swiftide/integrations/redis/struct.Redis.html)
- [`Redis::try_from_url`](docsrs:swiftide/integrations/redis/struct.Redis.html#method.try_from_url)
- [`ChatMessage`](docsrs:swiftide/chat_completion/enum.ChatMessage.html)
- [`ChatMessage::new_summary`](docsrs:swiftide/chat_completion/enum.ChatMessage.html#method.new_summary)
- [`ToolFeedback`](docsrs:swiftide/traits/enum.ToolFeedback.html)
- [`AgentBuilder::context`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.context)
- [`AgentBuilder::no_system_prompt`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.no_system_prompt)
- [`Agent::history`](docsrs:swiftide/agents/struct.Agent.html#method.history)

## Next

[Tool executors](/agents/executors/)
