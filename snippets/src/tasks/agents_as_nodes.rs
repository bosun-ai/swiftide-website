//! Snippets for `tasks/agents-as-nodes`.

use std::sync::Arc;

use swiftide::{
    agents::{Agent, DefaultContext, TaskAgent, tools::local_executor::LocalExecutor},
    chat_completion::{ToolOutput, errors::ToolError},
    integrations::openai::OpenAI,
    prompt::Prompt,
    tasks::{Task, TaskRunOutcome},
    traits::{AgentContext, Command, ToolExecutor},
};

// ANCHOR: task_agent
/// Adds an entry to the top of `CHANGELOG.md`.
///
/// # Arguments
/// - `entry`: The Markdown entry to add, including its heading.
#[swiftide::tool]
async fn add_changelog_entry(
    context: &dyn AgentContext,
    entry: &str,
) -> Result<ToolOutput, ToolError> {
    let executor = context.executor();
    let changelog = executor
        .exec_cmd(&Command::read_file("CHANGELOG.md"))
        .await?
        .into_string_lossy();
    executor
        .exec_cmd(&Command::write_file(
            "CHANGELOG.md",
            format!("{entry}\n\n{changelog}"),
        ))
        .await?;
    Ok(ToolOutput::text("Entry added."))
}

/// An agent wrapped in `TaskAgent` as the only node of a task.
pub async fn update_changelog(openai: OpenAI) -> anyhow::Result<()> {
    let checkout: Arc<dyn ToolExecutor> = Arc::new(LocalExecutor::new("/srv/checkout"));
    let agent = Agent::builder()
        .llm(&openai)
        .context(DefaultContext::from_executor(checkout))
        .system_prompt("You keep the changelog of this repository up to date.")
        .tools([add_changelog_entry()])
        .build()?;

    let mut task = Task::<Prompt, ()>::new();
    let changelog = task.register_node(TaskAgent::from(agent));

    task.starts_with(changelog);
    task.register_transition(changelog, task.transitions_to_finish())?;

    // `TaskAgent` returns `()`: the result is the file the tool wrote.
    match task.run("Add an entry for the 0.33 release.").await? {
        TaskRunOutcome::Completed(()) => println!("CHANGELOG.md is updated"),
        TaskRunOutcome::Paused => unreachable!("no node in this task pauses"),
    }
    Ok(())
}
// ANCHOR_END: task_agent
