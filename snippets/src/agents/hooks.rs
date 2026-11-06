//! Snippets for `agents/hooks`: changing requests, rewriting tool output, stop notifications and
//! hooks returned from methods.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex, PoisonError},
};

use swiftide::{
    agents::{Agent, hooks::AfterToolFn},
    chat_completion::{ChatMessage, ToolOutput},
    integrations::openai::OpenAI,
};

/// Adds a reminder to every request and reports token usage after every completion.
pub fn before_completion(openai: &OpenAI) -> anyhow::Result<Agent> {
    // ANCHOR: before_completion
    let agent = Agent::builder()
        .llm(openai)
        .before_completion(|_agent, request| {
            println!(
                "sending {} messages and {} tools",
                request.messages().len(),
                request.tools_spec().len()
            );
            // Only this request gets the reminder; the message history stays as it is.
            request
                .messages
                .to_mut()
                .push(ChatMessage::new_user("Reply in at most three sentences."));
            Box::pin(async { Ok(()) })
        })
        .after_completion(|_agent, response| {
            if let Some(usage) = &response.usage {
                println!("used {} tokens", usage.total_tokens);
            }
            Box::pin(async { Ok(()) })
        })
        .build()?;
    // ANCHOR_END: before_completion
    Ok(agent)
}

/// Keeps only the last lines of long tool output before it reaches the message history.
pub fn after_tool_rewrite(openai: &OpenAI) -> anyhow::Result<Agent> {
    // ANCHOR: after_tool_rewrite
    const KEEP_LINES: usize = 200;

    let agent = Agent::builder()
        .llm(openai)
        .after_tool(|_agent, tool_call, result| {
            if let Ok(ToolOutput::Text(text)) = result {
                let lines: Vec<&str> = text.lines().collect();
                if lines.len() > KEEP_LINES {
                    let dropped = lines.len() - KEEP_LINES;
                    let tail = lines[dropped..].join("\n");
                    *text = format!(
                        "[{dropped} lines of {} output cut]\n{tail}",
                        tool_call.name()
                    );
                }
            }
            Box::pin(async { Ok(()) })
        })
        .build()?;
    // ANCHOR_END: after_tool_rewrite
    Ok(agent)
}

/// Reports why the agent stopped, and the error if it failed.
pub fn on_stop(openai: &OpenAI) -> anyhow::Result<Agent> {
    // ANCHOR: on_stop
    let agent = Agent::builder()
        .llm(openai)
        .on_stop(|_agent, reason, error| {
            match error {
                Some(error) => eprintln!("agent failed: {error}"),
                None => println!("agent stopped: {reason:?}"),
            }
            Box::pin(async { Ok(()) })
        })
        .build()?;
    // ANCHOR_END: on_stop
    Ok(agent)
}

// ANCHOR: hook_from_method
/// Counts tool calls per tool, shared between the hook and the code that reads it.
#[derive(Clone, Default)]
pub struct ToolCallCounter {
    counts: Arc<Mutex<HashMap<String, usize>>>,
}

impl ToolCallCounter {
    /// `use<>` keeps `&self` out of the returned type, so the hook is `'static`.
    pub fn hook(&self) -> impl AfterToolFn + use<> {
        let counts = Arc::clone(&self.counts);
        move |_agent, tool_call, _result| {
            let mut counts = counts.lock().unwrap_or_else(PoisonError::into_inner);
            *counts.entry(tool_call.name().to_owned()).or_default() += 1;
            Box::pin(async { Ok(()) })
        }
    }

    pub fn count(&self, tool_name: &str) -> usize {
        let counts = self.counts.lock().unwrap_or_else(PoisonError::into_inner);
        counts.get(tool_name).copied().unwrap_or_default()
    }
}

pub async fn count_tool_calls(openai: &OpenAI) -> anyhow::Result<()> {
    let counter = ToolCallCounter::default();
    let mut agent = Agent::builder()
        .llm(openai)
        .after_tool(counter.hook())
        .build()?;

    agent.query("List the files in this directory").await?;
    println!("stop was called {} times", counter.count("stop"));
    Ok(())
}
// ANCHOR_END: hook_from_method
