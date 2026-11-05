//! Snippets for `agents/context-and-history`: storing the message history and rewriting it.

use swiftide::{
    agents::{Agent, DefaultContext},
    chat_completion::{ChatMessage, ToolOutput},
    integrations::{openai::OpenAI, redis::Redis},
};

/// Keeps the message history of one conversation in a Redis list.
pub async fn redis_history(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: redis_history
    let redis = Redis::try_from_url("redis://localhost:6379", "session-42")?;

    let mut context = DefaultContext::default();
    context.with_message_history(redis);

    let mut agent = Agent::builder()
        .llm(&openai)
        .context(context)
        .system_prompt("You help maintain the swiftide repository.")
        .build()?;

    agent.query("Which crates changed this week?").await?;
    // ANCHOR_END: redis_history
    Ok(())
}

/// Replaces the text of older tool outputs once the history grows long.
pub async fn replace_history(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: replace_history
    const MAX_MESSAGES: usize = 40;
    const KEEP_RECENT: usize = 10;

    let mut agent = Agent::builder()
        .llm(&openai)
        .after_each(|agent: &mut Agent| {
            Box::pin(async move {
                let history = agent.history().await?;
                if history.len() <= MAX_MESSAGES {
                    return Ok(());
                }

                let keep_from = history.len() - KEEP_RECENT;
                let trimmed = history
                    .into_iter()
                    .enumerate()
                    .map(|(index, message)| match message {
                        ChatMessage::ToolOutput(call, ToolOutput::Text(_)) if index < keep_from => {
                            ChatMessage::ToolOutput(call, ToolOutput::text("(output removed)"))
                        }
                        other => other,
                    })
                    .collect();

                agent.context().replace_history(trimmed).await?;
                Ok(())
            })
        })
        .build()?;
    // ANCHOR_END: replace_history
    agent.query("Find every TODO in the repository").await?;
    Ok(())
}
