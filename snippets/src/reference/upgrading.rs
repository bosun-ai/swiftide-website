//! Snippets for `reference/upgrading`: the code each breaking release from 0.23 to 0.33 needs.

use std::{borrow::Cow, sync::Arc};

use async_trait::async_trait;
use swiftide::{
    agents::{Agent, DefaultContext},
    chat_completion::{
        ChatMessage, Tool, ToolCall, ToolOutput, ToolSpec,
        errors::{LanguageModelError, ToolError},
    },
    indexing::TextNode,
    integrations::openai::OpenAI,
    prompt::Prompt,
    tasks::{NodeError, Task, TaskRunOutcome},
    traits::{
        AgentContext, Command, CommandError, CommandOutput, ExecutorExt, SimplePrompt,
        ToolExecutor, ToolFeedback, Transformer,
    },
};

/// 0.33: the system prompt is a `Prompt`; a `before_all` hook can fill its values.
pub async fn system_prompt(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: system_prompt
    let mut agent = Agent::builder()
        .llm(&openai)
        .system_prompt("You review pull requests for {{ project }}.")
        .before_all(|agent: &mut Agent| {
            Box::pin(async move {
                if let Some(prompt) = agent.system_prompt_mut() {
                    prompt.insert_context_value("project", "swiftide");
                }
                Ok(())
            })
        })
        .build()?;
    // ANCHOR_END: system_prompt
    agent.query("Review the open pull request").await?;
    Ok(())
}

/// 0.33: hooks receive `&mut Agent`, so they can call methods that take `&mut self`.
pub async fn hook_mut_agent(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: hook_mut_agent
    let mut agent = Agent::builder()
        .llm(&openai)
        .after_each(|agent: &mut Agent| {
            Box::pin(async move {
                if agent.history().await?.len() > 40 {
                    agent.stop("history too long").await;
                }
                Ok(())
            })
        })
        .build()?;
    // ANCHOR_END: hook_mut_agent
    agent.query("Summarize the repository").await?;
    Ok(())
}

/// 0.33 and 0.32: separate stdout and stderr, and a working directory per command.
pub async fn command_output(context: &dyn AgentContext) -> anyhow::Result<()> {
    // ANCHOR: command_output
    let cmd = Command::shell("cargo test").with_current_dir("crates/core");
    match context.executor().exec_cmd(&cmd).await {
        Ok(output) => {
            let stdout: Vec<u8> = output.stdout().flat_map(|b| b.iter().copied()).collect();
            println!("{}", String::from_utf8_lossy(&stdout));
        }
        Err(CommandError::NonZeroExit(output)) => {
            for chunk in output.stderr() {
                eprintln!("{}", String::from_utf8_lossy(chunk));
            }
        }
        Err(other) => return Err(other.into()),
    }
    // ANCHOR_END: command_output
    Ok(())
}

/// 0.33: task primitives live in `swiftide::tasks` and `run` returns a `TaskRunOutcome`.
pub async fn task_outcome() -> anyhow::Result<()> {
    // ANCHOR: task_outcome
    let mut task = Task::<i32, i32>::new();
    let double = task.register_node_fn(|n: &i32| Ok::<_, NodeError>(n * 2));
    task.starts_with(double);
    task.register_transition(double, task.transitions_to_finish())?;

    match task.run(21).await? {
        TaskRunOutcome::Completed(answer) => println!("{answer}"),
        TaskRunOutcome::Paused => println!("waiting to resume"),
    }
    // ANCHOR_END: task_outcome
    Ok(())
}

// ANCHOR: tool_args_schema
/// An inclusive range of lines.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
pub struct LineRange {
    /// First line, starting at 1
    pub start: u32,
    /// Last line, inclusive
    pub end: u32,
}

/// Reads line ranges from files in the workspace.
///
/// # Arguments
/// - `paths`: Files to read, relative to the workspace root.
/// - `range`: Optional line range to limit the output.
#[swiftide::tool]
async fn read_lines(
    context: &dyn AgentContext,
    paths: Vec<String>,
    range: Option<LineRange>,
) -> Result<ToolOutput, ToolError> {
    let _ = (context, range);
    Ok(ToolOutput::text(paths.join("\n")))
}
// ANCHOR_END: tool_args_schema

/// Prints the JSON schema the macro derived for `read_lines`.
pub fn print_read_lines_spec() -> anyhow::Result<()> {
    let spec = read_lines().tool_spec();
    println!("{}", serde_json::to_string_pretty(&spec)?);
    Ok(())
}

/// 0.32: run commands in a subdirectory of the executor's working directory.
///
/// A relative scope joins onto the executor's directory; a command with an absolute
/// `current_dir` runs unchanged.
pub async fn scoped_executor(executor: &dyn ToolExecutor) -> anyhow::Result<()> {
    // ANCHOR: scoped_executor
    let repo = executor.scoped("repo");
    let output = repo.exec_cmd(&Command::shell("git status --short")).await?;
    println!("{output}");
    // ANCHOR_END: scoped_executor
    Ok(())
}

// ANCHOR: text_transformer
#[derive(Clone)]
pub struct Uppercase;

#[async_trait]
impl Transformer for Uppercase {
    type Input = String;
    type Output = String;

    async fn transform_node(&self, mut node: TextNode) -> anyhow::Result<TextNode> {
        node.chunk = node.chunk.to_uppercase();
        Ok(node)
    }
}
// ANCHOR_END: text_transformer

// ANCHOR: tool_invoke
#[derive(Clone)]
pub struct Echo;

#[async_trait]
impl Tool for Echo {
    async fn invoke(
        &self,
        _context: &dyn AgentContext,
        tool_call: &ToolCall,
    ) -> Result<ToolOutput, ToolError> {
        Ok(ToolOutput::text(tool_call.args().unwrap_or_default()))
    }

    fn name(&self) -> Cow<'_, str> {
        Cow::Borrowed("echo")
    }

    fn tool_spec(&self) -> ToolSpec {
        ToolSpec::builder()
            .name("echo")
            .description("Repeats its arguments")
            .build()
            .expect("valid tool spec")
    }
}
// ANCHOR_END: tool_invoke

// ANCHOR: custom_context
/// Counts the messages an agent adds and delegates everything else.
#[derive(Clone)]
pub struct CountingContext {
    inner: DefaultContext,
    added: Arc<std::sync::atomic::AtomicUsize>,
}

#[async_trait]
impl AgentContext for CountingContext {
    async fn next_completion(&self) -> anyhow::Result<Option<Vec<ChatMessage>>> {
        self.inner.next_completion().await
    }

    async fn current_new_messages(&self) -> anyhow::Result<Vec<ChatMessage>> {
        self.inner.current_new_messages().await
    }

    async fn add_messages(&self, items: Vec<ChatMessage>) -> anyhow::Result<()> {
        self.added
            .fetch_add(items.len(), std::sync::atomic::Ordering::Relaxed);
        self.inner.add_messages(items).await
    }

    async fn add_message(&self, item: ChatMessage) -> anyhow::Result<()> {
        self.added
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.inner.add_message(item).await
    }

    #[allow(deprecated)]
    async fn exec_cmd(&self, cmd: &Command) -> Result<CommandOutput, CommandError> {
        self.inner.exec_cmd(cmd).await
    }

    fn executor(&self) -> &Arc<dyn ToolExecutor> {
        self.inner.executor()
    }

    async fn history(&self) -> anyhow::Result<Vec<ChatMessage>> {
        self.inner.history().await
    }

    async fn replace_history(&self, items: Vec<ChatMessage>) -> anyhow::Result<()> {
        self.inner.replace_history(items).await
    }

    async fn redrive(&self) -> anyhow::Result<()> {
        self.inner.redrive().await
    }

    async fn has_received_feedback(&self, tool_call: &ToolCall) -> Option<ToolFeedback> {
        self.inner.has_received_feedback(tool_call).await
    }

    async fn feedback_received(
        &self,
        tool_call: &ToolCall,
        feedback: &ToolFeedback,
    ) -> anyhow::Result<()> {
        self.inner.feedback_received(tool_call, feedback).await
    }
}
// ANCHOR_END: custom_context

/// 0.27: tools can ask for feedback, and the agent stops until it arrives.
pub fn describe_tool_output(output: &ToolOutput) -> String {
    // ANCHOR: tool_output_match
    match output {
        ToolOutput::Text(text) => text.clone(),
        ToolOutput::FeedbackRequired(payload) => format!("waiting for approval: {payload:?}"),
        ToolOutput::Fail(error) => format!("tool failed: {error}"),
        other => format!("agent stops: {other:?}"),
    }
    // ANCHOR_END: tool_output_match
}

/// 0.27: restore saved messages into the default context.
pub async fn restore_history(openai: OpenAI, saved: Vec<ChatMessage>) -> anyhow::Result<()> {
    // ANCHOR: restore_history
    let mut context = DefaultContext::default();
    context.with_existing_messages(saved).await?;

    let mut agent = Agent::builder().llm(&openai).context(context).build()?;
    agent.query("Continue where we left off").await?;
    // ANCHOR_END: restore_history
    Ok(())
}

// ANCHOR: language_model_error
#[derive(Debug, Clone)]
pub struct CannedModel {
    pub overloaded: bool,
}

#[async_trait]
impl SimplePrompt for CannedModel {
    async fn prompt(&self, prompt: Prompt) -> Result<String, LanguageModelError> {
        if self.overloaded {
            return Err(LanguageModelError::transient("model is overloaded"));
        }
        // An `anyhow::Error` converts into `LanguageModelError::PermanentError`.
        let rendered = prompt.render()?;
        Ok(format!("You asked: {rendered}"))
    }
}
// ANCHOR_END: language_model_error

/// 0.23: a one-off prompt with a template value.
pub fn prompt_template() -> anyhow::Result<String> {
    // ANCHOR: prompt_template
    let prompt = Prompt::from("Summarize {{ title }} in one sentence.")
        .with_context_value("title", "the changelog");
    let rendered = prompt.render()?;
    // ANCHOR_END: prompt_template
    Ok(rendered)
}
