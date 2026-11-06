//! Snippets for `agents/contrib`: file tools, a plan, context hooks and a lazy executor from
//! `swiftide-agents-contrib` (unreleased, on swiftide master).

use std::sync::Arc;

use swiftide::{
    agents::{Agent, AgentBuilder, DefaultContext, tools::local_executor::LocalExecutor},
    agents_contrib::{
        executors::{LazyExecutor, ToolExecutorExt},
        hooks::{ContextCompression, ToolSummarizer},
        tools::{ApplyPatch, ExactReplace, ReadFile, UpdatePlan},
    },
    integrations::openai::OpenAI,
    traits::{ChatCompletion, SimplePrompt, ToolExecutor},
};

/// An agent that reads and edits files in a checkout and keeps a plan.
pub async fn file_tools_and_plan() -> anyhow::Result<()> {
    // ANCHOR: file_tools_and_plan
    let openai = OpenAI::builder().default_prompt_model("gpt-5.4").build()?;
    let executor: Arc<dyn ToolExecutor> = Arc::new(LocalExecutor::new("/srv/checkout"));

    let mut agent = Agent::builder()
        .llm(&openai)
        .context(DefaultContext::from_executor(executor))
        .system_prompt("You fix failing tests. Keep a plan with update_plan.")
        .add_tool(ReadFile::default().with_line_numbers())
        .add_tool(ExactReplace)
        .add_tool(ApplyPatch)
        .add_tool(UpdatePlan::default())
        .limit(50)
        .build()?;

    agent
        .query("The test `parses_empty_input` in src/parser.rs fails. Fix it.")
        .await?;
    // ANCHOR_END: file_tools_and_plan
    Ok(())
}

/// Truncates and summarizes large tool outputs and compresses the conversation.
pub fn context_hooks(builder: &mut AgentBuilder) -> anyhow::Result<()> {
    // ANCHOR: context_hooks
    let small_model = OpenAI::builder()
        .default_prompt_model("gpt-5-mini")
        .build()?;

    let summarizer = ToolSummarizer::builder()
        .llm(Arc::new(small_model.clone()) as Arc<dyn SimplePrompt>)
        .tools_to_summarize(vec!["run_tests"])
        .git_start_sha("HEAD")
        .build()?;

    let compression = ContextCompression::builder()
        .llm(Arc::new(small_model) as Arc<dyn ChatCompletion>)
        .git_start_sha("HEAD")
        .git_diff_dir(".")
        .context_limit(200_000usize)
        .build()?;

    builder.after_tool(summarizer.summarize_hook());
    compression.register_hooks(builder);
    // ANCHOR_END: context_hooks
    Ok(())
}

/// Defers executor setup until the agent runs its first command.
pub fn lazy_executor(openai: &OpenAI) -> anyhow::Result<Agent> {
    // ANCHOR: lazy_executor
    let lazy = LazyExecutor::new(|| async {
        // Clone a repository, start a container, ... before the first command runs.
        Ok(Arc::new(LocalExecutor::new("/srv/checkout")) as Arc<dyn ToolExecutorExt>)
    });
    assert!(!lazy.is_started());

    let executor: Arc<dyn ToolExecutor> = Arc::new(lazy);
    let agent = Agent::builder()
        .llm(openai)
        .context(DefaultContext::from_executor(executor))
        .add_tool(ReadFile::default())
        .build()?;
    // ANCHOR_END: lazy_executor
    Ok(agent)
}
