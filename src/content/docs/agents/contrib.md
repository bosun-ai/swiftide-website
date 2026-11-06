---
title: "Contrib tools and hooks"
description: "Give an agent file tools (read_file, edit, apply_patch), an update_plan tool, hooks that truncate tool output and compress the conversation, and an executor that starts on first use, from swiftide-agents-contrib."
sidebar:
  order: 12
swiftide:
  since: "0.34"
identLint:
  ignore: [in_progress, run_tests, run_coverage]
---

With the components on this page, your agent reads and edits files, keeps a plan, and keeps its context small on long runs. They come from swiftide-agents-contrib.

::snippet{src="@snippets/src/agents/contrib.rs" anchor="file_tools_and_plan"}

`limit(50)` caps the run at 50 completions, because a fixing loop can run long. The four contrib tools sit next to the `stop` tool that every agent gets. Every file operation runs as a command through the tool executor of the context, here a `LocalExecutor` rooted at the checkout.

## Feature flags

Nothing in the crate is enabled by default. Pick one feature flag per capability:

| Feature flag | Adds |
| --- | --- |
| `agents-contrib-file-tools` | `ReadFile`, `ExactReplace`, `ApplyPatch` |
| `agents-contrib-planning` | `UpdatePlan`, `PlanItem`, `PlanStatus`, `PlanMemory` |
| `agents-contrib-context` | `ToolSummarizer`, `ContextCompression` |
| `agents-contrib-lazy-executor` | `LazyExecutor`, `ToolExecutorExt` |

Each one turns on `agents-contrib`, which turns on `swiftide-agents`. The plain `agents-contrib` flag only exposes the crate as `swiftide::agents_contrib`:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,agents-contrib-file-tools,agents-contrib-planning
cargo add tokio --features full
cargo add anyhow
```

A feature flag never registers a tool or a hook. You add each component to the builder yourself. [Feature flags](/reference/feature-flags/) lists every flag.

## File tools

The model sees these tool names:

| Type | Tool name | Arguments |
| --- | --- | --- |
| `ReadFile::default()` | `read_file` | `file_name`, optional `offset` and `limit` |
| `ReadFile::default().with_line_numbers()` | `read_file_with_line_numbers` | same, output prefixed with `N\|` line numbers |
| `ExactReplace` | `edit` | `file_path`, `old_string`, `new_string`, optional `replace_all` |
| `ApplyPatch` | `apply_patch` | `patch` |

`read_file` returns up to 2,000 lines by default. It cuts single lines at 2,000 characters and refuses a window over 100,000 characters. When it stops early, the output tells the model which offset to read next. `edit` replaces a match only when it is unique, unless the model sets `replace_all`.

`apply_patch` takes a patch in this format:

```text
*** Begin Patch
*** Add File: hello.txt
+Hello
*** Update File: src/main.rs
@@ fn greet()
-    println!("Hi");
+    println!("Hello");
*** End Patch
```

`*** Move to: new/path` after an update header moves a file. `*** Delete File: path` deletes one. Lines starting with a space are context, `-` removes a line and `+` adds one.

The file tools need a tool executor with a POSIX (Portable Operating System Interface) shell and the `cat`, `mkdir` and `rm` commands.

## Plans

`UpdatePlan` registers `update_plan`. The model sends `items`, each with a `step` and a `status` of `pending`, `in_progress` or `completed`, plus an `explanation` string. Both arguments are required. An empty `items` list returns the current plan without changing it.

The tool answers with a `ToolOutput::fail` when a step is empty or spans several lines, or when more than one step is in progress. The stored plan stays as it was. Clones of one `UpdatePlan` share the plan, while two `UpdatePlan::default()` values each have their own. The plan lives in memory and is not persisted.

## Context management

Two hooks keep the message history short:

::snippet{src="@snippets/src/agents/contrib.rs" anchor="context_hooks"}

Call both on the same `AgentBuilder` before `build()`. `tools_to_summarize` only applies to tools the agent has registered. A listed tool the agent lacks is skipped with a warning. `run_tests` here stands for a tool you define yourself.

`ToolSummarizer::summarize_hook` returns an `after_tool` hook. It truncates any tool output over an estimated 10,000 tokens to its first 40 and last 160 lines. It then cuts the result to a character budget of four characters per token, 40,000 by default, so a long line can still end mid-line. Finally, it summarizes the output of the tools named in `tools_to_summarize` with the model you pass to `llm`. Set `summarize_all_at_token_cutoff` to summarize any large output. By default, `skip_summarize_all` exempts the file-read tools from truncation and from `summarize_all_at_token_cutoff`. For the tools in `tools_needing_diff` (default `run_tests` and `run_coverage`), the summary prompt includes `git diff` against `git_start_sha`.

`ContextCompression::register_hooks` adds an `after_completion` and an `after_each` hook. They check three triggers in this order:

1. A turn that ends with a file read postpones any summary until the history reaches 90% of `context_limit`.
2. The sixth turn after the last summary triggers a new summary (`num_completions_for_summary`).
3. A history estimated above 70% of `context_limit` triggers one, unless the agent has stopped.

The model you pass to `llm` writes the summary from the history, a Git diff and the plan progress. The hook stores the summary as a `ChatMessage::Summary`. A `DefaultContext` then sends only the system messages and everything from the latest summary on ([Summaries](/agents/context-and-history/#summaries)). When the summary interrupts a tool turn, the hook also appends a user message `Continue` to the message history. After the first summary, the hook adds a `read_original_instructions` tool so the model can recover the first user message. It adds the tool only when the history holds a user message.

`ContextCompression`, and `ToolSummarizer` for the tools in `tools_needing_diff`, run `git diff` through the agent's tool executor. Point that tool executor at a Git repository.

## Lazy executor

`LazyExecutor` defers expensive setup, such as cloning a repository or starting a container, until the agent first needs it:

::snippet{src="@snippets/src/agents/contrib.rs" anchor="lazy_executor"}

`LazyExecutor::new` takes a closure whose future resolves to an `anyhow::Result<Arc<dyn ToolExecutorExt>>`. The first command, file stream or download calls it. Clones share the started executor. A failed start is retried on the next use. `is_started` returns true once the closure has succeeded. `ToolExecutorExt` adds `is_started`, `cleanup` and `download` to a tool executor, so bring the trait into scope to call them. It is implemented for `LocalExecutor`, for `LazyExecutor` itself and for a `ScopedExecutor` whose inner executor implements `ToolExecutorExt` and `Clone`. `cleanup` before the start does nothing. `download` runs `dd`, `base64` and `sha256sum` in the tool executor.

## Gotchas

- `LocalExecutor` does not isolate the file system. `read_file` and `edit` pass absolute paths to the tool executor unchanged. `apply_patch` only takes relative paths without `..` components, but it follows symlinks like the other tools. Use an isolated executor, such as the separate swiftide-docker-executor crate, for repositories you do not trust.
- Patches are not transactions. `apply_patch` runs its operations in order and can partly succeed, so the model has to read the result report.
- Tools are unique by name. If you already have a tool called `read_file` or `edit`, `add_tool` replaces it with the contrib tool, or the other way around.
- The default `context_limit` is 400,000,000. Set one that matches your model, or compression only triggers on the turn count.
- `ContextCompression` keeps its turn counter behind an `Arc`, shared with clones. Build a fresh one for every independent agent.
- Summaries lose detail. Keep facts the model must not forget in the system prompt.
- Debug logs contain the summary prompts and summaries, including file contents.

## Reference

The links point at the source on GitHub.

- [swiftide-agents-contrib README](https://github.com/bosun-ai/swiftide/blob/{{swiftide_sha}}/swiftide-agents-contrib/README.md)
- [`ReadFile`](https://github.com/bosun-ai/swiftide/blob/{{swiftide_sha}}/swiftide-agents-contrib/src/tools/read_file.rs), [`ExactReplace`](https://github.com/bosun-ai/swiftide/blob/{{swiftide_sha}}/swiftide-agents-contrib/src/tools/exact_replace.rs), [`ApplyPatch`](https://github.com/bosun-ai/swiftide/blob/{{swiftide_sha}}/swiftide-agents-contrib/src/tools/apply_patch/mod.rs)
- [`UpdatePlan`](https://github.com/bosun-ai/swiftide/blob/{{swiftide_sha}}/swiftide-agents-contrib/src/tools/update_plan.rs)
- [`ToolSummarizer`](https://github.com/bosun-ai/swiftide/blob/{{swiftide_sha}}/swiftide-agents-contrib/src/hooks/tool_summarizer.rs), [`ContextCompression`](https://github.com/bosun-ai/swiftide/blob/{{swiftide_sha}}/swiftide-agents-contrib/src/hooks/context_compression.rs)
- [`LazyExecutor`](https://github.com/bosun-ai/swiftide/blob/{{swiftide_sha}}/swiftide-agents-contrib/src/executors/lazy.rs), [`ToolExecutorExt`](https://github.com/bosun-ai/swiftide/blob/{{swiftide_sha}}/swiftide-agents-contrib/src/executors/traits.rs)
- [contrib examples](https://github.com/bosun-ai/swiftide/tree/{{swiftide_sha}}/swiftide-agents-contrib/examples)
- [`AgentBuilder::add_tool`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.add_tool), [`AgentBuilder::after_tool`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.after_tool)
- [`DefaultContext::from_executor`](docsrs:swiftide/agents/struct.DefaultContext.html#method.from_executor)
- [`LocalExecutor`](docsrs:swiftide/agents/tools/local_executor/struct.LocalExecutor.html)
- [Feature flags](/reference/feature-flags/)

## Next

[Testing agents](/agents/testing/)
