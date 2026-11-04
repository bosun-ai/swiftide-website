//! Snippets for the streaming section of `models/overview`.

use futures_util::StreamExt;
use swiftide::{
    chat_completion::{ChatCompletionRequest, ChatMessage},
    integrations::openai::OpenAI,
    traits::{ChatCompletion, ChatCompletionStream},
};

/// Streams a chat completion and prints the text as it arrives.
pub async fn complete_stream(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: complete_stream
    let request = ChatCompletionRequest::builder()
        .messages(vec![ChatMessage::new_user("Explain borrowing in Rust in three sentences.")])
        .build()?;

    let mut stream: ChatCompletionStream = openai.complete_stream(&request).await;
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
    // ANCHOR_END: complete_stream
    Ok(())
}

/// Streams only the new text on each item, without the accumulated response.
pub async fn stream_deltas(request: ChatCompletionRequest<'_>) -> anyhow::Result<()> {
    // ANCHOR: stream_deltas
    let openai = OpenAI::builder()
        .default_prompt_model("gpt-5-mini")
        .stream_full(false)
        .build()?;

    let mut stream = openai.complete_stream(&request).await;
    let mut complete = None;

    while let Some(response) = stream.next().await {
        let response = response?;
        if response.message.is_some() {
            complete = Some(response);
            continue;
        }
        if let Some(chunk) = response.delta.as_ref().and_then(|d| d.message_chunk.as_deref()) {
            print!("{chunk}");
        }
    }
    // ANCHOR_END: stream_deltas
    let _ = complete;
    Ok(())
}
