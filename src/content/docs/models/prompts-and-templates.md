---
title: "Prompts and templates"
description: "Build prompts with Prompt::from and Tera templates, register named templates, and replace the default prompts of transformers, answer generators and agents."
sidebar:
  order: 2
identLint:
  ignore: [urlencode_strict, get_random, to_string]
---

By the end of this page you can fill a template with values, reuse named templates, and replace the built-in prompts that Swiftide sends to a Large Language Model (LLM).

`Prompt` is always available. The snippets also use the OpenAI client, an agent and the tree-sitter transformers, and each sits behind a feature flag that is off by default:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,swiftide-agents,tree-sitter
```

[Feature flags](/reference/feature-flags/) lists the rest.

::snippet{src="@snippets/src/models/prompts_and_templates.rs" anchor="prompt_with_context"}

A `Prompt` is the text you send to a model, plus the values to fill into it. `Prompt::from` takes a `&'static str` or a `String`. The text is a [Tera](https://keats.github.io/tera/docs/) template, so `{{ topic }}` is a placeholder.

`with_context_value` adds one value to the template context. It accepts anything that converts into a `serde_json::Value`, including nested objects from `serde_json::json!`.

`render` returns the final text, or an error if the template does not render. The snippet calls it only to show the result. `SimplePrompt::prompt` takes the `Prompt` by value and renders it inside the client.

## Rendering

A one-off prompt from `Prompt::from` without template context is sent verbatim. `Prompt::from("hello {{ name }}")` renders to `hello {{ name }}`, braces included. A named template always renders through Tera, so a placeholder without a value fails there.

Once you set any value, Tera renders the whole text. Autoescaping is off for one-off templates, so characters such as `<` and `&` reach the model unchanged.

A render failure is an `anyhow::Error` from `render`. Clients return it as a `LanguageModelError::PermanentError`, so a retry does not help.

`with_context_value` consumes the prompt and returns it. When you hold a `&mut Prompt`, use `insert_context_value` instead. A struct does not convert into a `tera::Context`. Pass it through `tera::Context::from_serialize(&value)?` and hand the result to `with_context`.

## Template syntax

Tera supports loops, conditions and its built-in filters in any prompt.

::snippet{src="@snippets/src/models/prompts_and_templates.rs" anchor="prompt_with_loop"}

The Tera documentation lists the full syntax. Swiftide builds Tera without its `builtins` feature. The `date`, `slugify`, `filesizeformat`, `urlencode` and `urlencode_strict` filters are missing, as are the `now` and `get_random` functions. Enable `builtins` on your own `tera` dependency if you need them.

## Named templates

A long prompt that you use in several places belongs in a named template. You compile it once and load it by name.

::snippet{src="@snippets/src/models/prompts_and_templates.rs" anchor="named_template"}

`Prompt::extend` merges your Tera instance into one repository that the whole process shares. Its templates, filters and functions carry over. `Prompt::from_compiled_template` stores only the name. The lookup happens when the prompt renders, so an unknown name fails at that point.

Swiftide does not re-export Tera. Add it to your crate to build the instance:

```sh
cargo add tera@1.20 --no-default-features
```

Tera keeps the first template it sees under a name. Calling `Prompt::extend` again with a template of the same name does not replace the registered one. Tera also autoescapes named templates whose name ends in `.html`, `.htm` or `.xml`, so give prompts another suffix.

## Transformer prompts

Every indexing transformer that prompts a model has a `prompt_template` field. You replace its default prompt on the builder.

::snippet{src="@snippets/src/models/prompts_and_templates.rs" anchor="transformer_prompt"}

The transformer fills the template context before each call. It adds the current node as `node`, so `{{ node.chunk }}` is the chunk text. It also adds its own values. `MetadataQAText` adds `questions`, which is `num_questions` and defaults to 5.

The same field exists on `MetadataSummary`, `MetadataTitle` and `MetadataKeywords`, and on `MetadataQACode` and `CompressCodeOutline` behind the `tree-sitter` feature flag. Their default templates are `.prompt.md` files in the Swiftide source. Read the default for a transformer to see which variables it uses. [Transformers and metadata](/indexing/transformers-and-metadata/) covers the transformers themselves.

## Answer prompts

The answer generator `answers::Simple` takes a `prompt_template` too.

::snippet{src="@snippets/src/models/prompts_and_templates.rs" anchor="answer_prompt"}

Its template context has four variables:

- `question`: the question the user asked.
- `original`: the same as `question`.
- `current`: the query text after the query transformers ran. After a response transformer such as `Summary`, it holds that transformer's output instead.
- `documents`: the retrieved documents, joined with `---` lines. After a response transformer, it holds the same text as `current`.

The optional `document_template` renders each document on its own before the join, with its `content` and `metadata` in the template context. The answer generator skips it when a response transformer ran. The query transformer `GenerateSubquestions` and the response transformer `Summary` also take a `prompt_template`. [Answering](/querying/answering/) shows them in a query pipeline.

## Agent system prompts

An agent's system prompt is a `Prompt`, so it takes template values like any other prompt.

::snippet{src="@snippets/src/models/prompts_and_templates.rs" anchor="agent_system_prompt"}

`system_prompt` accepts anything that converts into a `Prompt`, so a plain string works too. An agent has no system prompt unless you set one.

The system prompt renders once, when the agent first runs, after the `BeforeAll` hooks. A `BeforeAll` hook can call `system_prompt_mut` and `insert_context_value` to fill a value just in time. Later hooks run after the render, so a change there does not reach the system message. If rendering fails, the run returns `AgentError::FailedToRenderSystemPrompt`. [System prompts](/agents/system-prompts/) covers the agent side.

## Gotchas

- A borrowed `&str` that is not `'static` does not convert into a `Prompt`. Turn it into a `String` with `to_string` first.
- `format!` treats `{{` and `}}` as escaped braces. `format!("Explain {{ topic }}")` yields `Explain { topic }`, which is no longer a placeholder. Write `{{{{ topic }}}}` inside `format!`.
- Filters and functions registered through `Prompt::extend` work only in named templates. A one-off prompt from `Prompt::from` renders with a fresh Tera instance and does not see them.
- `answers::Simple::builder().prompt_template` takes a `Prompt`, not a string. Wrap the text in `Prompt::from`.
- The prompt module rustdoc mentions a Template type. It does not exist at this version, so use `Prompt`.

## Reference

- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`OpenAI`](docsrs:swiftide/integrations/openai/type.OpenAI.html)
- [`MetadataQAText`](docsrs:swiftide/indexing/transformers/struct.MetadataQAText.html)
- [`MetadataQACode`](docsrs:swiftide/indexing/transformers/struct.MetadataQACode.html)
- [`CompressCodeOutline`](docsrs:swiftide/indexing/transformers/struct.CompressCodeOutline.html)
- [`answers::Simple`](docsrs:swiftide/query/answers/struct.Simple.html)
- [`GenerateSubquestions`](docsrs:swiftide/query/query_transformers/struct.GenerateSubquestions.html)
- [`response_transformers::Summary`](docsrs:swiftide/query/response_transformers/struct.Summary.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`AgentError`](docsrs:swiftide/agents/errors/enum.AgentError.html)

## Next

[Structured output](/models/structured-output/)
