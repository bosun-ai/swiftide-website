//! Snippets for `agents/context-and-history`: storing the message history and rewriting it.

use std::{collections::HashMap, path::PathBuf};

use async_trait::async_trait;
use swiftide::{
    agents::{Agent, DefaultContext, StopReason},
    chat_completion::{ChatMessage, ToolCall, ToolOutput},
    integrations::{openai::OpenAI, redis::Redis},
    traits::{MessageHistory, ToolFeedback},
};
use tokio::sync::Mutex;

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

// ANCHOR: custom_history
/// Keeps the message history of one conversation in a JSON file.
#[derive(Debug)]
struct JsonFileHistory {
    path: PathBuf,
    // Serializes read-modify-write cycles on the file.
    lock: Mutex<()>,
}

impl JsonFileHistory {
    async fn read(&self) -> anyhow::Result<Vec<ChatMessage>> {
        match tokio::fs::read_to_string(&self.path).await {
            Ok(json) => Ok(serde_json::from_str(&json)?),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(err) => Err(err.into()),
        }
    }

    async fn write(&self, messages: &[ChatMessage]) -> anyhow::Result<()> {
        tokio::fs::write(&self.path, serde_json::to_vec(messages)?).await?;
        Ok(())
    }
}

#[async_trait]
impl MessageHistory for JsonFileHistory {
    async fn history(&self) -> anyhow::Result<Vec<ChatMessage>> {
        let _guard = self.lock.lock().await;
        self.read().await
    }

    async fn push_owned(&self, item: ChatMessage) -> anyhow::Result<()> {
        let _guard = self.lock.lock().await;
        let mut messages = self.read().await?;
        messages.push(item);
        self.write(&messages).await
    }

    async fn overwrite(&self, items: Vec<ChatMessage>) -> anyhow::Result<()> {
        let _guard = self.lock.lock().await;
        self.write(&items).await
    }
}
// ANCHOR_END: custom_history

/// Uses the file-backed history as the agent's message history.
pub async fn custom_history(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: use_custom_history
    let mut context = DefaultContext::default();
    context.with_message_history(JsonFileHistory {
        path: "session-42.json".into(),
        lock: Mutex::new(()),
    });

    let mut agent = Agent::builder().llm(&openai).context(context).build()?;
    // ANCHOR_END: use_custom_history
    agent.query("Which crates changed this week?").await?;
    Ok(())
}

/// Saves approval decisions in one process and restores them in the next.
pub async fn restore_approvals(openai: OpenAI, agent: &Agent) -> anyhow::Result<()> {
    // ANCHOR: restore_approvals
    // First process: the agent waits for approval; store the person's decision with its call.
    if let Some(StopReason::FeedbackRequired { tool_call, .. }) = agent.stop_reason() {
        let decisions = vec![(tool_call.clone(), ToolFeedback::approved())];
        std::fs::write("approvals.json", serde_json::to_string(&decisions)?)?;
    }

    // Second process: load the decisions next to the restored message history, then resume.
    let decisions: Vec<(ToolCall, ToolFeedback)> =
        serde_json::from_str(&std::fs::read_to_string("approvals.json")?)?;

    let mut context = DefaultContext::default();
    context.with_message_history(Redis::try_from_url("redis://localhost:6379", "session-42")?);
    context.with_tool_feedback(decisions.into_iter().collect::<HashMap<_, _>>());

    let mut resumed = Agent::builder()
        .llm(&openai)
        .context(context)
        .no_system_prompt()
        .build()?;
    resumed.run().await?;
    // ANCHOR_END: restore_approvals
    Ok(())
}
