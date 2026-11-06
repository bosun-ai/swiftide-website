---
title: "System prompts"
description: "Give an agent a system prompt as a Prompt, fill its Tera template from runtime values, change it before the first run, or leave it out."
sidebar:
  order: 6
identLint:
  ignore: [release_rules, include_str]
---

Here you build an agent whose system prompt is a Tera template, filled with values your program only knows at run time. The example is the system prompt of the [release notes agent](/showcases/release-notes-agent/).

The code on this page needs the `swiftide-agents` feature flag, and the `openai` feature flag for the model:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,swiftide-agents
```

## Prompt as system message

::snippet{src="@swiftide/examples/release-notes-agent/src/agent.rs" anchor="prompt" title="examples/release-notes-agent/src/agent.rs"}

The system prompt of an agent is a `Prompt`. `AgentBuilder::system_prompt` takes anything that converts into one, so a `&'static str`, a `String` or a finished `Prompt` all work. The showcase passes the result of this method to `system_prompt` in its `build` function.

`SYSTEM_PROMPT` is [`src/system_prompt.md`](https://github.com/bosun-ai/swiftide/blob/{{swiftide_sha}}/examples/release-notes-agent/src/system_prompt.md), included at compile time with `include_str!`. It is a Tera template, the Jinja-style template language Swiftide renders prompts with. Its last lines show how it handles optional values:

```text
{% if github %}
- Reference pull requests as Markdown links, exactly like `[#1207](https://github.com/{{ github }}/pull/1207)`. Reference a commit by its short hash only when it has no pull request.{% else %}
- Reference pull requests as `#1207`, or commits by their short hash.{% endif %}
{% if next_version %}- The version is `{{ next_version }}`.{% else %}- Choose the version yourself: before 1.0, bump the minor version of `{{ since }}` for new features or breaking changes, and the patch version for fixes only.{% endif %}
```

The template uses these constructs:

- `{{ since }}` prints the tag the release starts from. Every run has one, so the method always sets it.
- `{% if github_tools and github %}` adds the instructions for the GitHub tools only when the agent has them and the repository is on GitHub.
- `{% if github %} ... {% else %} ... {% endif %}` picks the style of pull request references, and `{{ github }}` prints the repository as `owner/name`.
- `{% if next_version %}` switches between "use this version" and "choose a version", and `{{ next_version }}` prints it.
- `{{ next_version | default(value="<version>") }}` prints the version in the heading, or a placeholder when the user gave none.

The optional values stay undefined when they are absent. Tera treats an undefined value in an `if` as false, and the `default` filter replaces it. That is why the method inserts `next_version` and `github` only when it has them.

## Template values

`with_context_value` takes the prompt by value and returns it, so it chains onto `Prompt::from`. `insert_context_value` takes `&mut self`, which suits values you add conditionally, as the showcase does. Both accept anything that converts into a `tera::Value`: strings, numbers, booleans, and `serde_json::Value`. `with_context` merges a whole `tera::Context` at once.

[Prompts and templates](/models/prompts-and-templates/#template-syntax) covers the template syntax, loops and the filters that are available. It also lists the `&str` and `format!` pitfalls.

## Values from a before_all hook

Some values are only known when the agent starts. A `before_all` hook can read them and fill them in:

::snippet{src="@snippets/src/agents/system_prompts.rs" anchor="before_all_fill"}

The hook asks the tool executor for the current git branch and inserts it as `branch`. With the default local executor, the command runs in the working directory of your program. With a Docker executor, it runs in the container.

On the first run, the `before_all` hooks run, then the agent renders the prompt and stores it in the context as a `ChatMessage::System`. A `before_all` hook is therefore the last place where a change still counts. `Agent::system_prompt_mut` returns `None` when the agent has no system prompt, so the hook only touches a prompt that exists.

Before you call `build`, `AgentBuilder::system_prompt_mut` gives the same access on the builder. It also returns `None` when no prompt is set.

A render error stops the run before the first completion. `query` returns `AgentError::FailedToRenderSystemPrompt`. The agent then stays pending, so the next `query` runs the `before_all` hooks and renders the prompt again. That gives a hook the chance to fill the missing value on a retry.

## Named templates

A system prompt that several agents share can live in a named template:

::snippet{src="@snippets/src/agents/system_prompts.rs" anchor="named_template"}

`Prompt::extend` merges your Tera instance into one template repository for the whole process. `Prompt::from_compiled_template` stores only the template name and looks it up when the agent renders the prompt. Named templates can include each other, so shared rules live in one partial, here `release_rules`.

Swiftide does not re-export Tera. Add it to your own crate:

```sh
cargo add tera@1.20 --no-default-features
```

## Prompt removal

`no_system_prompt` removes the system prompt from the builder, including one set earlier:

::snippet{src="@snippets/src/agents/system_prompts.rs" anchor="resume"}

The second agent continues a stored message history that already holds a system message. Without `no_system_prompt`, its first run appends a second one. The call is useful when a shared builder function sets a prompt you do not want for a resumed run. [Context and history](/agents/context-and-history/) shows how the history is restored.

## Default prompt

An agent has no system prompt unless you set one. Code written for 0.32 or earlier relied on a built-in default; [Upgrading](/reference/upgrading/#system-prompts) lists what to change.

The model only sees what you write. A prompt for an agent with tools usually says what the task is, which tools to use in which order, and when to call `stop`. The release notes prompt does all three in its "How to work" list.

## Gotchas

- A `Prompt::from` prompt without any context value is sent verbatim, without rendering. `{{ since }}` then reaches the model as literal text. Named templates always render.
- An undefined value outside an `if` or the `default` filter fails rendering. Guard every optional value.
- Changes to the system prompt after the first render do not reach the stored message. To change the instructions of a running conversation, replace the system message with `AgentContext::replace_history`, as [Context and history](/agents/context-and-history/) shows.
- Setup errors after the render, such as a toolbox that fails to load, also leave the agent pending. The next `query` then adds the system message a second time.
- Filters, functions and partials registered with `Prompt::extend` only work in named templates. A `Prompt::from` prompt renders with a fresh Tera instance.

## Reference

- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)
- [`AgentBuilder`](docsrs:swiftide/agents/struct.AgentBuilder.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`AgentError`](docsrs:swiftide/agents/errors/enum.AgentError.html)
- [`AgentContext`](docsrs:swiftide/traits/trait.AgentContext.html)
- [`ToolExecutor`](docsrs:swiftide/traits/trait.ToolExecutor.html)
- [`ChatMessage`](docsrs:swiftide/chat_completion/enum.ChatMessage.html)
- [Tera documentation](https://keats.github.io/tera/docs/)

## Next

[Stopping and structured results](/agents/stopping-and-structured-results/)
