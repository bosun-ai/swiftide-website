//! Snippets for `agents/mcp`: naming a toolbox and hiding tools with a blacklist.

use swiftide::{
    agents::{Agent, tools::mcp::McpToolbox},
    integrations::openai::OpenAI,
};

/// Hides the GitHub server's write tools from the model and names the toolbox for the logs.
pub fn blacklist_toolbox(openai: &OpenAI, mut github: McpToolbox) -> anyhow::Result<Agent> {
    // ANCHOR: blacklist
    // `github` is an `McpToolbox` from `try_from_transport` or `from_running_service`.
    github
        .with_name("github")
        .with_blacklist(["create_or_update_file", "push_files", "merge_pull_request"]);

    let agent = Agent::builder()
        .llm(openai)
        .add_toolbox(github)
        .build()?;
    // ANCHOR_END: blacklist
    Ok(agent)
}
