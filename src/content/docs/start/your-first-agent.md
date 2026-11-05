---
title: "Your first agent"
description: "Build an agent with one git tool and one hook that summarizes the commits since a tag, then compare it with the release-notes-agent showcase."
sidebar:
  order: 3
---

This page builds a command-line program that asks a model to summarize the commits since a git tag. It is the first step of the [release-notes-agent](/showcases/release-notes-agent/) showcase: an agent with one tool that runs `git log` and one hook that prints each tool call.

## Dependencies

Start from the crate on [Installation](/start/installation/), or create a new one with the same dependencies plus serde:

```sh
cargo new first-agent
cd first-agent
cargo add swiftide@{{swiftide_version}} --features swiftide-agents,openai
cargo add tokio --features macros,rt-multi-thread
cargo add anyhow serde
```

`swiftide-agents` adds `swiftide::agents`. The tool attribute comes from `macros`, which is on by default. Its generated code derives serde's traits for the tool arguments, so your crate depends on serde.

## Program

The tool and the imports go at the top of `src/main.rs`:

::snippet{src="@snippets/src/start/your_first_agent.rs" anchor="tool" title="src/main.rs"}

`main` builds the agent and prints its answer:

::snippet{src="@snippets/src/start/your_first_agent.rs" anchor="main" title="src/main.rs"}

The tool runs `git` in the directory you start the program from. That directory needs a git repository with the tag you pass. The repository `cargo new` created has no commits and no tags, so run the program from another repository, such as a clone of swiftide:

```sh
export OPENAI_API_KEY=sk-...
git clone https://github.com/bosun-ai/swiftide
cd swiftide
cargo run --manifest-path ../first-agent/Cargo.toml -- v0.33.0
```

The program prints one line per tool call, such as `  → git_log_since {"tag":"v0.33.0"}`. Then it prints the model's summary. The last line is `stopped: Some(NoNewMessages)` or `stopped: Some(RequestedByTool(..))`, because `stop_reason()` returns an `Option`.

## Tool

`#[swiftide::tool]` turns the async function into a tool named `git_log_since`. The macro replaces your function: `git_log_since()` becomes a constructor that returns the tool as a `Box<dyn Tool>`, and your body moves to a method on a generated struct, so you cannot call the function directly any more. `.tools([git_log_since()])` hands the tool to the agent. The doc comment becomes the tool description the model reads. The `# Arguments` list becomes the description of each argument.

Every tool takes `&dyn AgentContext` as its first argument. `context.executor()` returns the tool executor, the part that runs commands. `Command::shell` describes a shell command, and `exec_cmd` runs it.

The model chooses `tag`, so the tool checks it before it reaches the shell. A tag that starts with `-`, or holds anything but letters, digits, `.`, `_`, `-` and `/`, gets `ToolOutput::fail`. The model gets that message as the tool's output and can try again. `--end-of-options` also stops git from reading the tag as a flag.

When git exits with a non-zero status, `exec_cmd` returns a `CommandError`, and `?` turns it into a `ToolError`. The agent also sends that error back to the model. When a call with the same arguments fails more than `tool_retry_limit` times (3 by default), the agent stops with `StopReason::ToolCallsOverLimit` and `query` returns the error. On success, `.into()` turns the `CommandOutput` into `ToolOutput::Text`, with stdout and stderr in the order the executor saw them.

## Hook

A hook is a closure that runs at a fixed point in the agent loop. `before_tool` runs before each tool call and receives `&mut Agent` and the `ToolCall`. Every hook returns a boxed future, written `Box::pin(async { Ok(()) })`. When a hook returns an error, the agent logs it with tracing and carries on.

## Agent loop

`Agent::builder()` starts with one tool already registered: `stop`, which the model calls when it is done. `.llm(&openai)` clones the client into the agent; `build` fails without it. `system_prompt` takes anything that converts into a `Prompt`, such as a `&str`.

`query` adds your message to the message history and loops. Each completion either calls tools, whose outputs go into the history for the next completion, or answers in text. In a normal run the loop ends in one of two ways:

- The model calls `stop`. `stop_reason()` then returns `StopReason::RequestedByTool`.
- The model answers without calling a tool. The default context, `DefaultContext`, ends the loop on such an answer, and `stop_reason()` returns `StopReason::NoNewMessages`.

Errors, failed tool calls and limits end it with other stop reasons. `history()` returns every `ChatMessage` of the run. The program prints the last `ChatMessage::Assistant` that has text.

`DefaultContext` runs commands through a `LocalExecutor` in your working directory, without a sandbox: the tool can reach anything your user can. It sets no timeout either, so a git command that waits for a pager or a credential prompt blocks the agent.

## Showcase version

The showcase prints its tool calls with the same kind of hook. This is the start of a real run on swiftide's own history, from its `OUTPUT.md`. It used `claude-sonnet-5-5` through an OpenAI-compatible gateway:

```text
Drafting release notes for the 7 commits since v0.33.0 with claude-sonnet-5-5 at http://127.0.0.1:8080/v1 (chat completions)
  → git_log_since v0.33.0
  git_log_since ✓ 346 lines
```

It also adds the timeout your program lacks. Every command runs at the repository root, with a one-minute `default_timeout`:

::snippet{src="@swiftide/examples/release-notes-agent/src/main.rs" anchor="executor" title="examples/release-notes-agent/src/main.rs"}

The agent gets this executor through `DefaultContext::from_executor`. Use the same pattern to run your program against a fixed repository instead of the current directory.

## Variations

To print results too, add `after_tool`. It receives `&mut Result<ToolOutput, ToolError>` and can replace what the model sees.

## Common mistakes

- **A second `.tools(..)` call drops the first list.** `.tools(..)` replaces the whole set with the default tools plus the ones you pass. Pass every tool in one call, or call `add_tool` after the last `.tools(..)`.
- **Your own stop tool is dropped.** Tools are kept by name, and a typed stop tool such as `StopWithArgs` is also named `stop`. `.tools(..)` inserts the default `stop` first, so yours is discarded and the model sees only the default. Call `without_default_stop_tool()` before `.tools(..)`, as the showcase does.
- **The model passes an argument you did not expect.** Check model-chosen arguments before they reach a shell or a path.
- **The agent loops for a long time.** `.limit(n)` caps the completions per `query`; the showcase uses 40.

## Related pages

- [Tools](/agents/tools/): `#[derive(Tool)]` for tools with state, and argument types.
- [Hooks](/agents/hooks/): every hook and what it receives.
- [Executors](/agents/executors/): `LocalExecutor`, timeouts and other executors.
- [Stopping and structured results](/agents/stopping-and-structured-results/): a stop tool that returns typed data.

## Reference

- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`AgentBuilder`](docsrs:swiftide/agents/struct.AgentBuilder.html)
- [`tool`](docsrs:swiftide/attr.tool.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)
- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)
- [`AgentContext`](docsrs:swiftide/traits/trait.AgentContext.html)
- [`ToolExecutor`](docsrs:swiftide/traits/trait.ToolExecutor.html)
- [`Command`](docsrs:swiftide/traits/enum.Command.html)
- [`CommandOutput`](docsrs:swiftide/traits/struct.CommandOutput.html)
- [`CommandError`](docsrs:swiftide/traits/enum.CommandError.html)
- [`ToolOutput`](docsrs:swiftide/chat_completion/enum.ToolOutput.html)
- [`ToolError`](docsrs:swiftide/chat_completion/errors/enum.ToolError.html)
- [`ToolCall`](docsrs:swiftide/chat_completion/struct.ToolCall.html)
- [`ChatMessage`](docsrs:swiftide/chat_completion/enum.ChatMessage.html)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`DefaultContext`](docsrs:swiftide/agents/struct.DefaultContext.html)
- [`LocalExecutor`](docsrs:swiftide/agents/tools/local_executor/struct.LocalExecutor.html)

## Next

[Your first pipeline](/start/your-first-pipeline/)
