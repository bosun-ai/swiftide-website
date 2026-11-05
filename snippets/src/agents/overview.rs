//! Snippets for `agents/overview`: running an agent and continuing the same history.

use swiftide::{
    agents::{Agent, StopReason},
    integrations::openai::OpenAI,
};

/// Asks one question, reads how the agent stopped, then continues the conversation.
pub async fn query_and_continue() -> anyhow::Result<()> {
    // ANCHOR: query_and_continue
    // Swiftide's HTTP clients use rustls without a crypto provider; install one first.
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("a rustls crypto provider was already installed"))?;

    // Reads OPENAI_API_KEY from the environment.
    let openai = OpenAI::builder()
        .default_prompt_model("gpt-5-mini")
        .build()?;

    let mut agent = Agent::builder()
        .llm(&openai)
        .system_prompt("You answer questions about Rust crates in two sentences.")
        .limit(10)
        .build()?;

    // A failed run, including a tool that fails too often, returns `Err` here.
    agent.query("What does the tokio crate do?").await?;
    assert!(agent.is_stopped());

    match agent.stop_reason() {
        Some(StopReason::RequestedByTool(..)) => println!("a tool stopped the agent"),
        Some(StopReason::NoNewMessages) => println!("no new messages, or the limit was reached"),
        other => println!("stopped: {other:?}"),
    }

    // A second query appends to the same message history and runs the loop again.
    agent.query("And how does it compare to async-std?").await?;
    println!("{} messages in the history", agent.history().await?.len());
    // ANCHOR_END: query_and_continue
    Ok(())
}

/// Gives an agent a name that your own code can read back.
pub fn named_agent(openai: &OpenAI) -> anyhow::Result<()> {
    // ANCHOR: named_agent
    let agent = Agent::builder()
        .llm(openai)
        .name("release-checker")
        .build()?;
    assert_eq!(agent.name(), "release-checker");

    // Without `name`, every agent is called "unnamed_agent".
    let unnamed = Agent::builder().llm(openai).build()?;
    assert_eq!(unnamed.name(), "unnamed_agent");
    // ANCHOR_END: named_agent
    Ok(())
}
