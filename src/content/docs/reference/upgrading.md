---
title: "Upgrading"
description: "Code changes needed to move between Swiftide releases from 0.23 to 0.34, newest first."
sidebar:
  order: 4
identLint:
  ignore: [install_default, SystemPrompt, ChatCompletionError, Template, transitions_to_done, build_anthropic_family, impls, closures, done, current_input, current_output, node_at, node_at_index, TransitionPayload, ParamSpec, "tasks::impls", "tasks::closures::SyncFn"]
---

By the end of this page your code compiles against swiftide {{swiftide_version}}. Breaking changes land in minor releases, so upgrade one minor version at a time. Sections run newest first. Start at the section after your current release and work up. Releases without a section, such as 0.25 to 0.26 and 0.28 to 0.30, need no code changes.

```sh
cargo add swiftide@{{swiftide_version}}
```

## 0.34

### Pipeline statistics

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="pipeline_stats"}

`indexing::Pipeline::run` returns `Result<PipelineStats>` instead of `Result<()>`. `pipeline.run().await?;` keeps compiling. A function that returns `pipeline.run().await` as its `Result<()>` tail expression needs `pipeline.run().await?; Ok(())`.

`stats_collector()` returns `&Arc<StatsCollector>`, shared with the running pipeline. Clone the `Arc` before `run` to watch the pipeline while it runs. Before, cloning the collector made a copy that never changed. Each branch of `split_by` gets its own collector, seeded with the statistics so far. `merge` keeps the collector of the pipeline it is called on, so the merged run reports both branches. See [Metrics and statistics](/observability/metrics-and-statistics/).

### Search strategy setters

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="search_strategy_setters"}

`with_top_k` on `SimilaritySingleEmbedding`, and `with_top_k`, `with_top_n`, `with_dense_vector_field` and `with_sparse_vector_field` on `HybridSearch`, take `self` and return `Self`. Replace `let mut s = Strategy::default(); s.with_top_k(n);` with `let s = Strategy::default().with_top_k(n);`, and drop `.to_owned()` after a setter chain.

`SimilaritySingleEmbedding::default()` exists only for the strategy without a filter, so it no longer needs a type annotation. Build a filtered strategy with `from_filter` or `with_filter`. `HybridSearch::default()` still needs the annotation on stores that implement the retriever for more than one filter type. `SearchStrategy` no longer requires `Default`.

### TLS

Swiftide's HTTP clients no longer need a crypto provider from your application. Each one uses a process-wide rustls provider if you installed one, and otherwise its own. You can remove the `rustls` dependency and the `install_default` call from `main`, or keep them to choose the provider. See [TLS](/reference/feature-flags/#tls).

### OpenAI-compatible endpoints

`OpenAI::builder().api_base(url).api_key(key)` points the OpenAI client at another endpoint without `async-openai`'s `OpenAIConfig`. `.client(..)` still takes a full client and replaces both; whichever is called last wins. See [OpenAI-compatible providers](/integrations/openai-compatible/).

## 0.33

### Rust version and dependencies

Swiftide now needs Rust 1.97 or newer. `ReasoningItem.id` is now an `Option<String>`. Public types from SQLx, text-splitter, LanceDB and DuckDB follow the new versions of those crates. If you pass these crates' types in yourself, bump them in your `Cargo.toml`: async-openai 0.41 for `OpenAI::builder().client(..)`, rmcp 2.1 for `McpToolbox::try_from_transport`, and redis 1.0.

### TLS crypto provider

In 0.33.0, Swiftide does not pick a rustls crypto provider. Install one at the start of `main`, before you build the first model client, or the client build panics. See [Crypto provider panic](/reference/troubleshooting-and-faq/#crypto-provider-panic). Since 0.34 the clients no longer need it; see [TLS](#tls).

### System prompts

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="system_prompt"}

`SystemPrompt` and its builder are gone, with the role, guidelines and constraints fields. The agent's system prompt is now a `Prompt`. `AgentBuilder::system_prompt` takes anything that converts into a `Prompt`, such as a `&str`.

An agent without `.system_prompt(..)` sends no system prompt. Before 0.33 it sent a default one.

The `before_all` hook now runs before the agent renders the system prompt. The hook in the snippet uses `system_prompt_mut` to fill the `project` value at run time. See [System prompts](/agents/system-prompts/).

### Hooks and the agent

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="hook_mut_agent"}

Most hooks now receive `&mut Agent` instead of `&Agent`. A hook can call methods that take `&mut self`, such as `stop`. The changed hooks are `before_all`, `on_start`, `before_completion`, `after_completion`, `before_tool`, `after_tool`, `after_each` and `on_stop`. `on_new_message` and `on_stream` still receive `&Agent`. See [Hooks](/agents/hooks/).

### Custom contexts

`AgentContext` has a new required method, `replace_history`, which replaces the whole message history. Implement it if you have your own context.

### Command output

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="command_output"}

`CommandOutput.output` is gone, and so is `as_ref()` on `CommandOutput`. It keeps stdout and stderr chunks in the order they arrived. Read them with `stdout()` and `stderr()`, or get one string with `to_string_lossy()` or `Display`. `CommandOutput::from` now takes bytes, through `Into<Bytes>`.

`AgentContext::exec_cmd` has been deprecated since 0.27. Call `context.executor().exec_cmd(&cmd)` instead.

### Tasks crate

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="task_outcome"}

Task graphs moved to the `swiftide-tasks` crate, available as `swiftide::tasks`. Its own `swiftide-tasks` feature flag lets you use tasks without agents.

`swiftide::agents::tasks` now re-exports `swiftide::tasks`, but the nested modules are gone. Import from the root: `tasks::task::Task` becomes `tasks::Task`, and `tasks::closures::SyncFn` becomes `tasks::SyncFn`. `TaskAgent` moved from `tasks::impls` to `swiftide::agents::TaskAgent`.

`Task::run` and `Task::resume` return a `TaskRunOutcome`: `Completed` with the output, or `Paused`. `transitions_to_finish()` replaces `transitions_to_done()`. `current_nodes()` replaces `current_node()`. `done()`, `current_input()`, `current_output()`, `node_at()` and `node_at_index()` are gone.

`TransitionPayload` and `NextNode` are no longer public. In a transition closure, return `Transition::next(&node, value)` or `node.transitions_with(value)` to move on. Return `Transition::pause()` to pause and `Transition::error(err)` to fail.

A fan-out ends in a join: `Transition::fan_out(..)` with `join_with(..)`, and the join node receives a `JoinInput<T>`. See [Fan-out and join](/tasks/fan-out-and-join/).

### Tool schemas

`ToolSpecBuilder::build` now validates the parameter schema against the strict tool-schema contract and returns a `ToolSpecBuildError`. A schema that built in 0.32 can fail. See [Rejected tool schemas](/reference/troubleshooting-and-faq/#rejected-tool-schemas).

The `#[tool]` macro reads the tool and argument descriptions from the doc comment and its `# Arguments` list, as in the [0.32 snippet](#tool-arguments).

### AWS Bedrock

The Amazon Web Services (AWS) Bedrock integration is now `integrations::aws_bedrock_v2`. Build it with `AwsBedrock::builder().default_prompt_model("model-id")`. The model family builders, such as `build_anthropic_family`, are gone. See [AWS Bedrock](/integrations/aws-bedrock/).

### Chat messages

`ChatMessage` has two new variants, `UserWithParts` for images and other content parts, and `Reasoning`. A `match` without a wildcard arm needs arms for both. `ChatCompletionRequest` now has a lifetime parameter. See [Images and multimodal input](/models/images/).

## 0.32

### Tool arguments

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="tool_args_schema"}

`ToolSpec.parameters` and `ParamSpec` are gone. A hand-written tool sets `parameters_schema`, an `Option<schemars::Schema>`, instead. The `#[tool]` macro derives a JSON Schema from the Rust types of the arguments. A struct argument needs `schemars::JsonSchema` next to the serde derives, so add `schemars` 1.x to your dependencies. See [Tools](/agents/tools/).

### Working directories

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="scoped_executor"}

Its variants are now structs, so patterns like `Command::Shell(script)` no longer compile. Build commands with `Command::shell`, `Command::read_file` and `Command::write_file`, and set a directory with `with_current_dir`. `scoped` on `ExecutorExt` wraps a tool executor in a `ScopedExecutor`. It joins a relative working directory onto the scope and leaves an absolute one unchanged. A custom tool executor reads the directory with `current_dir_path()`. See [Tool executors](/agents/executors/).

### MCP tool names

Model Context Protocol (MCP) tools are now named `server:tool`. The prefix is the name the server reports in its handshake, or `mcp` when it reports none. The name you set with `McpToolbox::with_name` does not change it. MCP needs the `mcp` feature flag. See [MCP](/agents/mcp/).

## 0.31

### Generic nodes

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="text_transformer"}

Indexing pipelines are generic over the content of a node. `Node<T>` replaces `Node`, and `TextNode` is the alias for `Node<String>`. The transformer, batch transformer, chunker and persist traits declare `type Input` and `type Output`. For text, set both to `String`. A loader declares only `Output`, and a node cache only `Input`.

## 0.27

### Tool invocation

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="tool_invoke"}

`Tool::invoke` receives the whole `ToolCall` instead of `raw_args: Option<&str>`. The raw arguments are `tool_call.args()`.

### Context and message history

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="restore_history"}

To store messages elsewhere, implement `MessageHistory` and pass it to `DefaultContext::with_message_history`. Loading saved messages is now the async `with_existing_messages`. Redis implements `MessageHistory` behind the `redis` feature flag. See [Context and message history](/agents/context-and-history/).

### Context methods

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="custom_context"}

The context methods that read or change messages now return a `Result`. Add a `?` where you call them, so `context.history().await` becomes `context.history().await?`. This applies to `next_completion`, `current_new_messages`, `add_message`, `add_messages`, `history` and `redrive`.

A custom context needs three new methods. `executor` returns the tool executor that runs commands for the agent. `feedback_received` stores the feedback for a tool call, such as a human approval. `has_received_feedback` returns that feedback when the agent calls the tool again. The snippet wraps a `DefaultContext` and passes each call through to it. It targets {{swiftide_version}}, so it also has `replace_history`, which arrived in 0.33.

### Tool output

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="tool_output_match"}

`ToolOutput` has a new `FeedbackRequired` variant. A tool returns it when it needs feedback before it runs. The agent then stops with the new `StopReason::FeedbackRequired`. Both enums are marked non-exhaustive, so your `match` already has a wildcard arm. That arm now catches `FeedbackRequired`, which compiles but can hide a paused agent. Add an arm for it. To ask for approval before a tool runs, wrap the tool in `ApprovalRequired`. See [Human in the loop](/agents/human-in-the-loop/).

## 0.24

### Model errors

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="language_model_error"}

The model traits now return a `LanguageModelError`. This breaks every model client you wrote yourself. `SimplePrompt::prompt` returns `Result<String, LanguageModelError>` instead of `anyhow::Result<String>`. `EmbeddingModel::embed` and `SparseEmbeddingModel::sparse_embed` changed the same way. `ChatCompletion::complete` returns `LanguageModelError` in place of `ChatCompletionError`, which is gone.

Pick the variant that tells callers what to do next. `LanguageModelError::transient` marks an error worth a retry, such as a rate limit. `permanent` marks one that fails again on retry. `context_length_exceeded` marks a request too long for the model. The `?` operator still works on an `anyhow::Error`, which becomes a `PermanentError`. See [Errors, backoff, usage and tokens](/models/errors-backoff-and-usage/).

## 0.23

### Templates

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="prompt_template"}

`Template` is gone. `Prompt` holds either a one-off template string or the name of a compiled template. Use `Prompt::from` for a string and `Prompt::from_compiled_template` for a name. `render` is no longer async. See [Prompts and templates](/models/prompts-and-templates/).

### Stopping an agent

`stop` is now async and takes a stop reason, so `agent.stop()` becomes `agent.stop("done").await`. A `&str` converts into `StopReason::Other`.

## Reference

- [`Prompt`](docsrs:swiftide/prompt/struct.Prompt.html)
- [`Agent`](docsrs:swiftide/agents/struct.Agent.html)
- [`AgentBuilder`](docsrs:swiftide/agents/struct.AgentBuilder.html)
- [`hooks`](docsrs:swiftide/agents/hooks/index.html)
- [`StopReason`](docsrs:swiftide/agents/enum.StopReason.html)
- [`AgentContext`](docsrs:swiftide/traits/trait.AgentContext.html)
- [`CommandOutput`](docsrs:swiftide/traits/struct.CommandOutput.html)
- [`CommandError`](docsrs:swiftide/traits/enum.CommandError.html)
- [`Command`](docsrs:swiftide/traits/enum.Command.html)
- [`ExecutorExt`](docsrs:swiftide/traits/trait.ExecutorExt.html)
- [`ScopedExecutor`](docsrs:swiftide/traits/struct.ScopedExecutor.html)
- [`Task`](docsrs:swiftide/tasks/struct.Task.html)
- [`TaskRunOutcome`](docsrs:swiftide/tasks/enum.TaskRunOutcome.html)
- [`Transition`](docsrs:swiftide/tasks/struct.Transition.html)
- [`JoinInput`](docsrs:swiftide/tasks/struct.JoinInput.html)
- [`TaskAgent`](docsrs:swiftide/agents/struct.TaskAgent.html)
- [`AwsBedrock`](docsrs:swiftide/integrations/aws_bedrock_v2/struct.AwsBedrock.html)
- [`ChatMessage`](docsrs:swiftide/chat_completion/enum.ChatMessage.html)
- [`LanguageModelError`](docsrs:swiftide/chat_completion/errors/enum.LanguageModelError.html)
- [`SimplePrompt`](docsrs:swiftide/traits/trait.SimplePrompt.html)
- [`EmbeddingModel`](docsrs:swiftide/traits/trait.EmbeddingModel.html)
- [`ChatCompletion`](docsrs:swiftide/traits/trait.ChatCompletion.html)
- [`Tool`](docsrs:swiftide/chat_completion/trait.Tool.html)
- [`ToolCall`](docsrs:swiftide/chat_completion/struct.ToolCall.html)
- [`ToolSpec`](docsrs:swiftide/chat_completion/struct.ToolSpec.html)
- [`Node`](docsrs:swiftide/indexing/struct.Node.html)
- [`TextNode`](docsrs:swiftide/indexing/type.TextNode.html)
- [`Transformer`](docsrs:swiftide/traits/trait.Transformer.html)
- [`MessageHistory`](docsrs:swiftide/traits/trait.MessageHistory.html)
- [`DefaultContext`](docsrs:swiftide/agents/struct.DefaultContext.html)
- [`ToolOutput`](docsrs:swiftide/chat_completion/enum.ToolOutput.html)
- [`ToolFeedback`](docsrs:swiftide/traits/enum.ToolFeedback.html)
- [`ApprovalRequired`](docsrs:swiftide/agents/tools/control/struct.ApprovalRequired.html)

Next: [Troubleshooting and FAQ](/reference/troubleshooting-and-faq/)
