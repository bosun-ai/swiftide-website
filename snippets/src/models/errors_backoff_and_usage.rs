//! Snippets for `models/errors-backoff-and-usage`.

use swiftide::{
    CharEstimator,
    chat_completion::{ChatCompletionRequest, ChatMessage, Usage, errors::LanguageModelError},
    indexing::{BackoffConfiguration, LanguageModelWithBackOff},
    integrations::{openai::OpenAI, tiktoken::TikToken},
    prompt::Prompt,
    traits::{ChatCompletion, EstimateTokens, SimplePrompt},
};

/// Reacts to each kind of model error.
pub async fn match_errors(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: match_errors
    match openai.prompt("Summarise the attached log.".into()).await {
        Ok(answer) => println!("{answer}"),
        Err(LanguageModelError::ContextLengthExceeded(e)) => {
            eprintln!("prompt too long, shorten it: {e}");
        }
        Err(LanguageModelError::TransientError(e)) => {
            eprintln!("rate limit or outage, try again later: {e}");
        }
        Err(error @ LanguageModelError::PermanentError(_)) => return Err(error.into()),
    }
    // ANCHOR_END: match_errors
    Ok(())
}

/// Wraps a client so transient errors are retried with exponential backoff.
pub async fn backoff_configuration(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: backoff_configuration
    let openai = LanguageModelWithBackOff::new(
        openai,
        BackoffConfiguration {
            initial_interval_sec: 2,
            multiplier: 2.0,
            randomization_factor: 0.5,
            max_elapsed_time_sec: 120,
        },
    );

    let answer = openai.prompt("Name one Rust web framework.".into()).await?;
    // ANCHOR_END: backoff_configuration
    println!("{answer}");
    Ok(())
}

/// Logs the token usage of every call.
pub fn on_usage() -> anyhow::Result<OpenAI> {
    // ANCHOR: on_usage
    let openai = OpenAI::builder()
        .default_prompt_model("gpt-5-mini")
        .on_usage(|usage| {
            println!(
                "{} tokens in, {} out",
                usage.prompt_tokens, usage.completion_tokens
            );
            Ok(())
        })
        .build()?;
    // ANCHOR_END: on_usage
    Ok(openai)
}

/// Hands the token usage of every call to async code.
pub fn on_usage_async() -> anyhow::Result<OpenAI> {
    // ANCHOR: on_usage_async
    let openai = OpenAI::builder()
        .default_prompt_model("gpt-5-mini")
        .on_usage_async(|usage| {
            Box::pin(async move {
                // Write to a database or a metrics service here.
                println!("{} tokens", usage.total_tokens);
                Ok(())
            })
        })
        .build()?;
    // ANCHOR_END: on_usage_async
    Ok(openai)
}

/// Reads the provider-independent usage breakdown from a chat completion response.
pub async fn normalized_usage(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: normalized_usage
    let request = ChatCompletionRequest::from(vec![ChatMessage::new_user("Hello")]);
    let response = openai.complete(&request).await?;

    if let Some(usage) = response.usage.as_ref().map(Usage::normalized) {
        let cached = usage
            .details
            .as_ref()
            .and_then(|details| details.input.cached_tokens)
            .unwrap_or(0);
        println!("{} tokens, {cached} from cache", usage.total_tokens);
    }
    // ANCHOR_END: normalized_usage
    Ok(())
}

/// Estimates the size of a message history without calling a model.
pub async fn estimate_tokens() -> anyhow::Result<usize> {
    // ANCHOR: estimate_tokens
    let history = vec![
        ChatMessage::new_system("You are a helpful assistant."),
        ChatMessage::new_user("What is a borrow checker?"),
    ];

    let rough = CharEstimator.estimate(history.as_slice()).await?;
    // ANCHOR_END: estimate_tokens
    Ok(rough)
}

/// Estimates tokens with the tokenizer an OpenAI model uses.
pub async fn estimate_tiktoken(openai: OpenAI) -> anyhow::Result<(usize, usize)> {
    // ANCHOR: estimate_tiktoken
    let tokenizer = TikToken::try_from_model("gpt-4o")?;
    let prompt = Prompt::from("Explain lifetimes in one paragraph.");
    let from_tokenizer = tokenizer.estimate(&prompt).await?;

    // The client's tokenizer is gpt-4o unless you set `tiktoken` on its builder.
    let from_client = openai.estimate_tokens(&prompt).await?;
    // ANCHOR_END: estimate_tiktoken
    Ok((from_tokenizer, from_client))
}
