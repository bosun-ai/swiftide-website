//! Snippets for `start/your-first-agent`: one git tool, one hook, one question.

// ANCHOR: tool
use anyhow::Context as _;
use swiftide::{
    agents::Agent,
    chat_completion::{ChatMessage, ToolOutput, errors::ToolError},
    integrations::openai::OpenAI,
    traits::{AgentContext, Command},
};

/// Lists the commits since a git tag, newest first, one per line.
///
/// # Arguments
/// - `tag`: The tag to start from, e.g. `v0.33.0`.
#[swiftide::tool]
async fn git_log_since(context: &dyn AgentContext, tag: &str) -> Result<ToolOutput, ToolError> {
    // The model chooses the tag; refuse anything that is not a plain tag name.
    let plain = |c: char| c.is_ascii_alphanumeric() || "._-/".contains(c);
    if tag.starts_with('-') || !tag.chars().all(plain) {
        return Ok(ToolOutput::fail(format!("`{tag}` is not a tag name")));
    }

    let log = Command::shell(format!("git log --oneline --end-of-options {tag}..HEAD"));
    Ok(context.executor().exec_cmd(&log).await?.into())
}
// ANCHOR_END: tool

/// The whole program: summarize the commits since the tag given on the command line.
// ANCHOR: main
#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("a rustls crypto provider was already installed"))?;
    let tag = std::env::args()
        .nth(1)
        .context("usage: first-agent <tag>")?;

    // Reads OPENAI_API_KEY from the environment.
    let openai = OpenAI::builder()
        .default_prompt_model("gpt-5-mini")
        .build()?;

    let mut agent = Agent::builder()
        .llm(&openai)
        .system_prompt("You summarize git history for release notes. Be brief.")
        .tools([git_log_since()])
        .before_tool(|_agent, tool_call| {
            println!(
                "  → {} {}",
                tool_call.name(),
                tool_call.args().unwrap_or_default()
            );
            Box::pin(async { Ok(()) })
        })
        .build()?;

    agent
        .query(format!("Summarize the commits since {tag}."))
        .await?;

    let history = agent.history().await?;
    let summary = history.iter().rev().find_map(|message| match message {
        ChatMessage::Assistant(Some(text), _) => Some(text.as_str()),
        _ => None,
    });
    println!("{}", summary.unwrap_or("(no summary)"));
    println!("stopped: {:?}", agent.stop_reason());
    Ok(())
}
// ANCHOR_END: main
