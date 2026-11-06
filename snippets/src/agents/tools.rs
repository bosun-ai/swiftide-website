//! Snippets for `agents/tools`: argument types, tool errors, soft failures and the tool retry limit.

use swiftide::{
    agents::{Agent, StopReason},
    chat_completion::{Tool, ToolOutput, errors::ToolError},
    integrations::openai::OpenAI,
    traits::{AgentContext, Command, CommandError},
};

// ANCHOR: tool_errors
/// Runs the tests of one crate in the workspace and returns their output.
///
/// # Arguments
/// - `package`: Name of the crate to test, e.g. `swiftide-core`.
#[swiftide::tool]
async fn run_tests(context: &dyn AgentContext, package: &str) -> Result<ToolOutput, ToolError> {
    let is_crate_name = !package.is_empty()
        && package
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    if !is_crate_name {
        return Ok(ToolOutput::fail(format!(
            "`{package}` is not a crate name. Use a package name from Cargo.toml."
        )));
    }

    let command = Command::shell(format!("cargo test --package {package}"));
    match context.executor().exec_cmd(&command).await {
        Ok(output) => Ok(output.into()),
        Err(CommandError::NonZeroExit(output)) => {
            Ok(ToolOutput::text(format!("Tests failed:\n{output}")))
        }
        Err(error) => Err(ToolError::execution_failed(error)),
    }
}
// ANCHOR_END: tool_errors

/// Builds an agent that stops when the same tool call keeps failing.
pub async fn retry_limit(openai: &OpenAI) -> anyhow::Result<()> {
    // ANCHOR: retry_limit
    let mut agent = Agent::builder()
        .llm(openai)
        .tools([run_tests()])
        .tool_retry_limit(2)
        .build()?;

    if let Err(error) = agent.query("Run the tests of swiftide-core.").await {
        if let Some(StopReason::ToolCallsOverLimit(call)) = agent.stop_reason() {
            eprintln!("gave up on `{}`: {error}", call.name());
        }
    }
    // ANCHOR_END: retry_limit
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
/// - `range`: Optional line range to limit the output. Use null for the whole file.
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

/// The `read_lines` tool, ready for `AgentBuilder::tools`.
pub fn read_lines_tool() -> Box<dyn Tool> {
    read_lines()
}
