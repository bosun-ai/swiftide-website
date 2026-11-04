//! Landing page: an agent with one tool that reads the git history.
//!
//! Hand-formatted to a narrow width: the landing page shows three excerpts side by side.
#![cfg_attr(rustfmt, rustfmt_skip)]

use anyhow::anyhow;
use swiftide::{
    agents::Agent,
    chat_completion::{ToolOutput, errors::ToolError},
    integrations::openai::OpenAI,
    traits::{AgentContext, Command},
};

pub async fn release_notes(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: agent
    /// The model picks the tag: check it first.
    fn is_tag(tag: &str) -> bool {
        let ok = |c: char| c.is_ascii_alphanumeric()
            || "._-/".contains(c);
        !tag.starts_with('-') && tag.chars().all(ok)
    }

    #[swiftide::tool(
        description = "Commits since a git tag",
        param(name = "tag", description = "A git tag")
    )]
    async fn git_log(
        cx: &dyn AgentContext,
        tag: &str,
    ) -> Result<ToolOutput, ToolError> {
        if !is_tag(tag) {
            return Err(anyhow!("bad tag {tag}").into());
        }
        let cmd = Command::shell(format!(
            "git log --end-of-options {tag}.."
        ));
        Ok(cx.executor().exec_cmd(&cmd).await?.into())
    }

    Agent::builder()
        .llm(&openai)
        .tools([git_log()])
        .build()?
        .query("Write release notes since v0.32.0")
        .await?;
    // ANCHOR_END: agent
    Ok(())
}
