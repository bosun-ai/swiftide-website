---
title: "Human in the loop"
description: "Wrap a tool in ApprovalRequired so the agent stops with StopReason::FeedbackRequired, then approve or refuse the call and resume the agent with run() or query()."
sidebar:
  order: 8
---

An agent can stop before a tool runs, wait for a person to approve or refuse the call, and then continue where it stopped.

The agent types on this page live in `swiftide::agents`, behind the `swiftide-agents` feature flag. The example also uses the `openai` integration and the default `macros` feature flag:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,swiftide-agents
```

## Approval wrapper

::snippet{src="@swiftide/examples/agents_with_human_in_the_loop.rs" anchor="approval"}

The comment in the example gets one method name wrong: the method on the context that stores feedback is `feedback_received`.

::snippet{src="@swiftide/examples/agents_with_human_in_the_loop.rs" anchor="agent"}

Register the wrapper in place of the bare tool.

[`ApprovalRequired`](docsrs:swiftide/agents/tools/control/struct.ApprovalRequired.html) wraps any tool. It keeps the inner tool's name and spec, so the model sees the same tool. When the model calls it, the wrapper asks the context for feedback on that tool call:

- No feedback yet: it returns `ToolOutput::FeedbackRequired(None)`. The agent stops with `StopReason::FeedbackRequired { tool_call, payload }` and records no tool output, so the tool call stays open.
- Approved: it runs the inner tool and returns its output.
- Refused: it returns the text `This tool call was refused`, which the model reads as the tool output.

`query` returns `Ok(())` when the agent stops this way. Read the stop reason to find the call that waits:

::snippet{src="@swiftide/examples/agents_with_human_in_the_loop.rs" anchor="run"}

`feedback_received` on the context stores a [`ToolFeedback`](docsrs:swiftide/traits/enum.ToolFeedback.html) for that tool call. `run` adds no user message. Before it asks the model for anything, it invokes the tool calls of the last assistant message again, provided no tool output follows that message. This time the wrapper finds the approval and runs the inner tool, and the loop carries on as usual.

## Approval loop

The release notes showcase wraps its `write_changelog` tool as `OneEntryPerChange::new(ApprovalRequired::new(WriteChangelog::new(..)))` and loops until a maintainer approves a draft or gives up:

::snippet{src="@swiftide/examples/release-notes-agent/src/session.rs" anchor="approval" title="examples/release-notes-agent/src/session.rs"}

The arguments of the waiting call are the proposal. The session parses them into the arguments struct that `#[derive(Tool)]` generates and hands them to a `Reviewer`, a small trait with a terminal implementation in `terminal.rs`.

After an approval the write can still fail, so the session checks the history for a `ToolOutput::Text` answering that call before it counts the notes as written.

## Captured approval

This is the approval excerpt of a real run on the Swiftide repository. The model was `claude-sonnet-5-5` behind an OpenAI-compatible gateway, and `y` was piped into stdin, which the reviewer echoes:

```text
  → write_changelog <32 lines>
  write_changelog ⏸ waiting for approval

── proposed: 32 lines into a new file, RELEASE_NOTES.md
Write these release notes? [y/N] y
  → write_changelog <32 lines>
  write_changelog ✓ wrote RELEASE_NOTES.md
  → stop (version 0.34.0: highlights 2, breaking 1, fixes 1)
  stop ■ done
```

The `→` lines come from a `before_tool` hook and the result lines from an `after_tool` hook. `⏸ waiting for approval` is the hook seeing `ToolOutput::FeedbackRequired`. The second `→ write_changelog` is the same tool call, invoked again by `run()` after the approval.

## Refusals and revisions

A refusal is an answer, not an error. `ToolFeedback::refused()` makes the wrapper return its refusal text, and the agent records it as the tool output. `query(feedback)` queues the person's message until that output is recorded, so the history reads: tool call, refusal, feedback. The model then drafts again and calls `write_changelog` with a new tool call, which starts a new approval round.

Neither `approved()` nor `refused()` takes an argument. To attach a payload, chain `.with_payload(serde_json::Value)` after the constructor. `ApprovalRequired` ignores it; your own tools can read it.

## Draft checks

`OneEntryPerChange` in the showcase's `tools.rs` wraps the approval tool. It parses the draft and returns `ToolOutput::fail` when one pull request or commit appears in more than one entry; otherwise it calls the wrapped tool. The model reads the failure and fixes the draft, and the maintainer only sees drafts that pass. A `ToolOutput::fail` does not count toward the tool retry limit ([Tool errors and retries](/agents/tools/#tool-errors-and-retries)). Put checks outside the approval wrapper, so they run before anyone is asked.

## Custom feedback tools

A tool can ask for feedback itself by returning `ToolOutput::feedback_required(Some(payload))`. The payload reaches the caller as the `payload` of `StopReason::FeedbackRequired`. The tool needs the `ToolCall` to look up feedback, so it implements [`Tool`](docsrs:swiftide/chat_completion/trait.Tool.html) by hand; `#[swiftide::tool]` functions and `#[derive(Tool)]` structs only receive the context and their arguments.

::snippet{src="@snippets/src/agents/human_in_the_loop.rs" anchor="custom_feedback_tool"}

`DefaultContext` removes the feedback when `has_received_feedback` returns it, so each decision applies to one invocation. The caller can change what the tool does through the payload of the approval:

::snippet{src="@snippets/src/agents/human_in_the_loop.rs" anchor="approve_with_payload"}

## Gotchas

- Have the model call an approval tool on its own. `run` and `query` only resume open tool calls when no tool output follows the assistant message. If the same completion also called another tool, that tool's output is recorded and the approved call never runs. The showcase prompt says "Call `write_changelog` with the Markdown, on its own."
- The stop reason names only the first tool call that asked for feedback in a completion.
- Calling `run()` or `query()` before giving feedback invokes the wrapper again, which asks again, and the agent returns `AgentError::UnfulfilledToolCalls`. A message passed to that `query` stays queued, and the next successful `run()` or `query()` adds it to the history after the tool output. Retry with `run()` so the message does not appear twice.
- Hooks see the resumed call twice ([Lifecycle hooks](/agents/hooks/#gotchas)).
- Feedback lives in the context, not the message history. To approve in another process, see [Context and history](/agents/context-and-history/).

## Reference

- [`ApprovalRequired`](docsrs:swiftide/agents/tools/control/struct.ApprovalRequired.html)
- [`ToolFeedback`](docsrs:swiftide/traits/enum.ToolFeedback.html)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html) (`query`, `run`, `stop_reason`, `context`)
- [`AgentContext`](docsrs:swiftide/traits/trait.AgentContext.html) (`feedback_received`, `has_received_feedback`)
- [`DefaultContext`](docsrs:swiftide/agents/struct.DefaultContext.html) (`with_tool_feedback`)
- [`ToolOutput`](docsrs:swiftide/chat_completion/enum.ToolOutput.html)
- [`Tool`](docsrs:swiftide/chat_completion/trait.Tool.html), [`ToolCall`](docsrs:swiftide/chat_completion/struct.ToolCall.html), [`ToolSpec`](docsrs:swiftide/chat_completion/struct.ToolSpec.html)
- [`AgentError`](docsrs:swiftide/agents/errors/enum.AgentError.html)

## Next

[Streaming](/agents/streaming/)
