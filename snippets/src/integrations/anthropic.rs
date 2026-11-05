//! Snippets for `integrations/anthropic`.

use futures_util::StreamExt;
use swiftide::{
    agents::Agent,
    chat_completion::{ChatCompletionRequest, ChatMessage, ToolOutput, errors::ToolError},
    integrations::anthropic::Anthropic,
    traits::{AgentContext, ChatCompletion, SimplePrompt},
};

/// An Anthropic client with an explicit model and a usage callback, used for a single prompt.
pub async fn anthropic_client() -> anyhow::Result<()> {
    // ANCHOR: anthropic_client
    let anthropic = Anthropic::builder()
        .default_prompt_model("claude-sonnet-4-6")
        .on_usage(|usage| {
            println!(
                "{} prompt + {} completion tokens",
                usage.prompt_tokens, usage.completion_tokens
            );
            Ok(())
        })
        .build()?;

    let answer = anthropic
        .prompt("Explain the borrow checker in one sentence.".into())
        .await?;
    // ANCHOR_END: anthropic_client
    println!("{answer}");
    Ok(())
}

// ANCHOR: anthropic_tool
#[swiftide::tool(
    description = "Count the words in a piece of text",
    param(name = "text", description = "The text to count")
)]
async fn count_words(_context: &dyn AgentContext, text: &str) -> Result<ToolOutput, ToolError> {
    Ok(ToolOutput::text(
        text.split_whitespace().count().to_string(),
    ))
}
// ANCHOR_END: anthropic_tool

/// An agent that runs on Anthropic and calls a tool.
pub async fn anthropic_agent(anthropic: Anthropic) -> anyhow::Result<()> {
    // ANCHOR: anthropic_agent
    let mut agent = Agent::builder()
        .llm(&anthropic)
        .tools(vec![count_words()])
        .limit(5)
        .build()?;

    agent
        .query("How many words are in 'the quick brown fox'? Use the tool.")
        .await?;
    // ANCHOR_END: anthropic_agent
    Ok(())
}

/// Streams a chat completion from Anthropic and prints the text as it arrives.
pub async fn anthropic_stream(anthropic: Anthropic) -> anyhow::Result<()> {
    // ANCHOR: anthropic_stream
    let request = ChatCompletionRequest::builder()
        .messages(vec![
            ChatMessage::new_system("You answer in plain words."),
            ChatMessage::new_user("Explain lifetimes in Rust in three sentences."),
        ])
        .build()?;

    let mut stream = anthropic.complete_stream(&request).await;
    let mut printed = 0;
    let mut last = None;

    while let Some(response) = stream.next().await {
        let response = response?;
        let text = response.message().unwrap_or_default();
        print!("{}", &text[printed..]);
        printed = text.len();
        last = Some(response);
    }

    if let Some(usage) = last.and_then(|response| response.usage) {
        println!("\n{} tokens", usage.total_tokens);
    }
    // ANCHOR_END: anthropic_stream
    Ok(())
}
