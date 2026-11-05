---
title: "Release notes agent"
description: "An agent that reads git history through scoped tools, streams a release-notes draft, waits for your approval, and ends with structured notes."
sidebar:
  order: 2
identLint:
  ignore: [WriteChangelogArgs, schema_for, JsonSchema, TokioChildProcess, SystemPrompt, get_pull_request, get_issue]
---

You end up with a command-line agent that drafts release notes for the commits since your last tag. It streams the draft to your terminal and writes `RELEASE_NOTES.md` only after you answer `y`. The code lives in `examples/release-notes-agent` of the swiftide repository.

## Prerequisites

- Rust 1.97.1 and a git repository with at least one tag.
- `OPENAI_API_KEY`. Set `OPENAI_BASE_URL` to use any OpenAI-compatible endpoint instead of `https://api.openai.com/v1`.
- For `--mcp`: Node.js (`npx`) and `GITHUB_PERSONAL_ACCESS_TOKEN` or `GITHUB_TOKEN`.
- For `--trace`: `LANGFUSE_PUBLIC_KEY` and `LANGFUSE_SECRET_KEY`, and optionally `LANGFUSE_URL`.

The CLI also reads variables from a `.env` file, found in the current directory or the nearest parent, or from the file you pass with `--env-file`. TLS needs no setup, because swiftide's HTTP clients bring their own rustls crypto provider ([TLS](/integrations/overview/#tls)).

The crate turns off default features and enables the feature flags `macros`, `swiftide-agents`, `openai`, `mcp` and `langfuse`. The excerpts below also need `async-trait`, because `#[derive(Tool)]` expands to `#[async_trait]`, plus `schemars` and `rmcp`. The model client sets the endpoint with `api_base` and `api_key` on the `OpenAI` builder, which is on swiftide master but not in {{swiftide_version}} :unreleased, so depend on swiftide from git until the next release:

```sh
cargo add swiftide --git https://github.com/bosun-ai/swiftide --no-default-features --features macros,swiftide-agents,openai,mcp,langfuse
cargo add async-trait schemars --features schemars/derive
cargo add rmcp --features client,transport-child-process
```

The example itself builds against the swiftide workspace through a path dependency.

## Architecture

![Agent loop](/diagrams/diagram-agent-loop.svg)

The app has one agent and one tool executor. The agent reads git history with three tools, drafts the notes, and calls `write_changelog`. That tool waits for approval, which the session loop collects outside the agent. A custom `stop` tool ends the run with the notes as structured data.

## Tool executor

::snippet{src="@swiftide/examples/release-notes-agent/src/main.rs" anchor="executor" title="examples/release-notes-agent/src/main.rs"}

`LocalExecutor` runs every command with the repository root as its working directory. A command without its own timeout is stopped after one minute. It does not sandbox anything: absolute paths and `..` still reach outside the repository. The tools therefore parse revisions and paths from the model into value types, `Revision` and `RepoPath`, before they build a command. The output path comes from the CLI, not from the model.

## Tools

::snippet{src="@swiftide/examples/release-notes-agent/src/tools.rs" anchor="tools" title="examples/release-notes-agent/src/tools.rs"}

`#[swiftide::tool]` turns each async function into a tool. It also generates a constructor with the same name that returns a boxed tool, which is why the agent calls `git_log_since()`. The tool description comes from the doc comment, and the parameter descriptions come from its `# Arguments` list. Every tool sends its `Command` through `executor()` on the context, so all commands share the executor above. The git queries run as shell commands. `Command::read_file` and `Command::write_file` go straight to the filesystem.

### Stateful tool

::snippet{src="@swiftide/examples/release-notes-agent/src/tools.rs" anchor="write_changelog" title="examples/release-notes-agent/src/tools.rs"}

The `Tool` derive macro builds a tool from a struct. Here the struct holds the output file, which the CLI fixes, so the model cannot choose where to write. The macro calls the method named after the tool, `write_changelog`, and generates the argument type `WriteChangelogArgs`. The session deserializes the pending call into that type to report how many lines the draft has and where it goes.

## System prompt

::snippet{src="@swiftide/examples/release-notes-agent/src/agent.rs" anchor="prompt" title="examples/release-notes-agent/src/agent.rs"}

The system prompt is a `Prompt`, a Tera template read from `src/system_prompt.md`. In 0.33 `Prompt` replaces `SystemPrompt`; see [Upgrading](/reference/upgrading/). `with_context_value` fills a template variable.

## Agent

::snippet{src="@swiftide/examples/release-notes-agent/src/agent.rs" anchor="agent" title="examples/release-notes-agent/src/agent.rs"}

`DefaultContext::from_executor` gives the agent's context the executor, so tools get it through `executor()`. Call `without_default_stop_tool` before `tools`, because `tools` adds the default `stop` tool at the moment you call it. The `write_changelog` tool sits inside two wrappers. `ApprovalRequired` holds the call until you decide. `OneEntryPerChange`, defined in the example, sends a draft that lists one pull request twice back to the model before you see it. `limit(STEP_LIMIT)` caps one run at 40 completions.

## Hooks and streaming

::snippet{src="@swiftide/examples/release-notes-agent/src/progress.rs" anchor="hooks" title="examples/release-notes-agent/src/progress.rs"}

In 0.33 `before_tool` and `after_tool` receive `&mut Agent`. `after_tool` also gets the tool result as `&mut`, so a hook could rewrite it.

::snippet{src="@swiftide/examples/release-notes-agent/src/progress.rs" anchor="stream" title="examples/release-notes-agent/src/progress.rs"}

Registering `on_stream` turns on streaming completions for the agent. The notes arrive as the JSON argument of `write_changelog`, so the hook reads `tool_calls_chunk` and decodes the argument as it grows. The client in `src/llm.rs` sets `stream_full(false)` on the OpenAI builder. Each chunk then leaves out the accumulated message, tool calls and usage. `message_chunk` holds only the new text, but `tool_calls_chunk` still holds the argument accumulated so far.

On {{swiftide_version}} the OpenAI chat completions client repeats the previous `message_chunk` on tool-call and final chunks, so this hook prints some text twice. The fix (#1216) is on master, which the showcase builds against. :unreleased

## Structured stop

::snippet{src="@swiftide/examples/release-notes-agent/src/notes.rs" anchor="stop_schema" title="examples/release-notes-agent/src/notes.rs"}

`StopWithArgs::with_parameters_schema` creates a tool named `stop` whose arguments must match the schema of `ReleaseNotes`. With a custom schema the whole argument payload is forwarded, without an `output` wrapper. It arrives in `StopReason::RequestedByTool`, where the session deserializes it into `ReleaseNotes`. `render` in `src/notes.rs` prints that value as the summary at the end of a run.

## Approval

::snippet{src="@swiftide/examples/release-notes-agent/src/session.rs" anchor="approval" title="examples/release-notes-agent/src/session.rs"}

`ApprovalRequired` returns `ToolOutput::FeedbackRequired` until the context holds feedback for that tool call. The agent then stops with `StopReason::FeedbackRequired`, which carries the pending tool call. To approve, record `ToolFeedback::approved()` with `feedback_received` and call `run()`. The pending call then runs the inner tool. To ask for changes, record `ToolFeedback::refused()` and send your feedback with `query()`. The model sees "This tool call was refused" as the tool output, followed by your message.

The session reports success only when the approved write returned text. A write that fails goes back to the model, and the loop continues.

### Turn limit and nudges

The step counter resets on every `query()` and `run()`. When the agent reaches the limit, it stops with `StopReason::NoNewMessages`, the same stop reason as a turn without tool calls. The session tells the two apart from the message history. After a turn without a tool call it asks the model once to continue. At the limit, or after a second empty turn, it gives up.

## MCP tools

::snippet{src="@swiftide/examples/release-notes-agent/src/mcp.rs" anchor="mcp" title="examples/release-notes-agent/src/mcp.rs"}

`--mcp` starts the GitHub Model Context Protocol (MCP) server as a child process. `McpToolbox::try_from_transport` connects to it, and `with_whitelist` keeps two read-only tools, `get_pull_request` and `get_issue`. Swiftide registers MCP tools as `<server>:<tool>`, and OpenAI rejects `:` in tool names. The example wraps the toolbox to replace those characters with `_`, so the model sees the two tools with the server name and `_` in front. Without `npx`, a token or a GitHub `origin`, the CLI continues without these tools.

## Tracing

::snippet{src="@swiftide/examples/release-notes-agent/src/telemetry.rs" anchor="langfuse" title="examples/release-notes-agent/src/telemetry.rs"}

`LangfuseLayer::default()` reads the keys and `LANGFUSE_URL` (default `http://localhost:3000`) from the environment. It panics when a key is missing or empty. `init` checks that the keys are set first, but an empty value still passes that check. The `Targets` filter matches by prefix, so the layer receives spans and events from every target that starts with `swiftide`, at debug level and above. `main` calls `flush` before the process exits, so buffered events reach Langfuse.

## Commands

The crate is on master, not in the {{swiftide_version}} tag. Clone the repository:

```sh
git clone https://github.com/bosun-ai/swiftide
cd swiftide
```

From the checkout root, this drafts notes for swiftide itself and writes `RELEASE_NOTES.md` there:

```sh
cargo run -p release-notes-agent -- --version 0.34.0
```

Point it at another OpenAI-compatible endpoint, or at another repository:

```sh
OPENAI_BASE_URL=http://127.0.0.1:8080/v1 OPENAI_MODEL=claude-sonnet-5-5 \
  cargo run -p release-notes-agent -- --version 0.34.0
cargo run -p release-notes-agent -- --repo ../my-crate --since v1.2.0 --mcp --trace
```

Answer `y` or `yes` to write the notes. Any other answer, including an empty one, asks what should change. An empty answer to that second question stops without writing.

`cargo test -p release-notes-agent` runs the revise round and the nudge against a scripted model ([Testing](/agents/testing/)), and checks the step-limit detection on a hand-built message history.

| Flag | Default | |
| --- | --- | --- |
| `--repo <path>` | `.` | The git repository. |
| `--since <tag>` | latest tag | The previous release. |
| `--version <next>` | proposed by the agent | The version being released. |
| `--output <file>` | `RELEASE_NOTES.md` | File the notes are prepended to, relative to the repository root. |
| `--mcp` | off | Read pull requests through the GitHub MCP server. |
| `--trace` | off | Send traces to Langfuse. |
| `--model <name>` | `gpt-5.4` | Model name, also read from `OPENAI_MODEL`. |
| `--responses-api` | off | Use the OpenAI Responses API instead of chat completions. |
| `--env-file <path>` | nearest `.env` | Load environment variables from this file. |

## Output

This run used `claude-sonnet-5-5` through an OpenAI-compatible gateway in front of AWS Bedrock, with chat completions. It covers the 7 commits since v0.33.0, released as 0.34.0, with `y` piped into stdin. The tool calls in a group run in parallel, but the agent awaits them in the order they were issued, so their results print in call order.

```text
Drafting release notes for the 7 commits since v0.33.0 with claude-sonnet-5-5 at http://127.0.0.1:8080/v1 (chat completions)
  → git_log_since v0.33.0
  git_log_since ✓ 346 lines
  → git_diff swiftide-integrations/Cargo.toml 3b1ad22e^..3b1ad22e
  → git_diff …/src/chat_completion/chat_completion_response.rs 90bcadc7^..90bcadc7
  ...
  git_diff ✓ 15 lines
  git_diff ✓ 72 lines
  ...
## 0.34.0

This release adds the opt-in `swiftide-agents-contrib` crate and Perl support for tree-sitter. It also fixes duplicated text in OpenAI streaming and resolves security advisories, which removes AWS Console login from the Bedrock client.
...
  → write_changelog <32 lines>
  write_changelog ⏸ waiting for approval

── proposed: 32 lines into a new file, RELEASE_NOTES.md
Write these release notes? [y/N] y
  → write_changelog <32 lines>
  write_changelog ✓ wrote RELEASE_NOTES.md
  → stop (version 0.34.0: highlights 2, breaking 1, fixes 1)
  stop ■ done

✔ RELEASE_NOTES.md

0.34.0 — This release adds the opt-in `swiftide-agents-contrib` crate and Perl support for tree-sitter. It also fixes duplicated text in OpenAI streaming and resolves security advisories, which removes AWS Console login from the Bedrock client.
  highlights (2)
    • Agent tools, hooks and executors in `swiftide-agents-contrib` [#1207]
    • Perl support for tree-sitter [#1198]
  breaking changes (1)
    • AWS Console login removed from the Bedrock client [#1215]
  fixes (1)
    • Repeated text in OpenAI streaming [#1216]
```

Everything these notes describe, `swiftide-agents-contrib`, Perl tree-sitter support, the Bedrock change (#1215) and the streaming fix (#1216), is on swiftide master and not in {{swiftide_version}}. :unreleased

## Gotchas

- The notes are a draft. Check each entry against `git show` before you approve.
- `--responses-api` failed against the gateway used for this run, because the gateway streams Responses events that `async-openai` cannot parse. Use it with OpenAI itself.
- `--trace` is untested against a live Langfuse; check that traces arrive before relying on them.

## Concept pages

- [Tools](/agents/tools/) and [Executors](/agents/executors/)
- [System prompts](/agents/system-prompts/)
- [Hooks](/agents/hooks/) and [Streaming](/agents/streaming/)
- [Stopping and structured results](/agents/stopping-and-structured-results/) and [Structured output](/models/structured-output/)
- [Human in the loop](/agents/human-in-the-loop/)
- [MCP](/agents/mcp/)
- [Langfuse](/observability/langfuse/)

## Reference

- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`AgentBuilder`](docsrs:swiftide/agents/struct.AgentBuilder.html)
- [`DefaultContext`](docsrs:swiftide/agents/struct.DefaultContext.html)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`LocalExecutor`](docsrs:swiftide/agents/tools/local_executor/struct.LocalExecutor.html)
- [`ApprovalRequired`](docsrs:swiftide/agents/tools/control/struct.ApprovalRequired.html)
- [`StopWithArgs`](docsrs:swiftide/agents/tools/control/struct.StopWithArgs.html)
- [`McpToolbox`](docsrs:swiftide/agents/tools/mcp/struct.McpToolbox.html)
- [`ToolFeedback`](docsrs:swiftide/traits/enum.ToolFeedback.html)
- [`ToolExecutor`](docsrs:swiftide/traits/trait.ToolExecutor.html)
- [`AgentContext`](docsrs:swiftide/traits/trait.AgentContext.html)
- [`ToolBox`](docsrs:swiftide/traits/trait.ToolBox.html)
- [`Command`](docsrs:swiftide/traits/enum.Command.html)
- [`CommandOutput`](docsrs:swiftide/traits/struct.CommandOutput.html)
- [`ToolOutput`](docsrs:swiftide/chat_completion/enum.ToolOutput.html)
- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)
- [`tool`](docsrs:swiftide/attr.tool.html) and [`Tool`](docsrs:swiftide/derive.Tool.html) macros
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)
- [`LangfuseLayer`](docsrs:swiftide/langfuse/struct.LangfuseLayer.html)

## Next

[Workspace architect](/showcases/workspace-architect/)
