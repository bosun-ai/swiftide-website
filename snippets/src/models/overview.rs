//! Snippets for `models/overview`.

use std::sync::Arc;

use swiftide::{
    agents::Agent,
    chat_completion::ChatMessage,
    integrations::{
        anthropic::Anthropic,
        ollama::Ollama,
        openai::{OpenAI, Options, ReasoningEffort},
    },
    traits::{EmbeddingModel, SimplePrompt},
};

/// One client per provider, each with the models it uses by default.
pub fn provider_clients() -> anyhow::Result<()> {
    // ANCHOR: provider_clients
    let openai = OpenAI::builder()
        .default_prompt_model("gpt-5-mini")
        .default_embed_model("text-embedding-3-small")
        .build()?;

    let anthropic = Anthropic::builder()
        .default_prompt_model("claude-sonnet-4-5")
        .build()?;

    let ollama = Ollama::builder()
        .default_prompt_model("llama3.2")
        .default_embed_model("nomic-embed-text")
        .build()?;
    // ANCHOR_END: provider_clients
    let _ = (openai, anthropic, ollama);
    Ok(())
}

/// The same client used through three different traits.
pub async fn trait_objects(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: trait_objects
    let prompt: Arc<dyn SimplePrompt> = Arc::new(openai.clone());
    let embed: Arc<dyn EmbeddingModel> = Arc::new(openai.clone());

    let answer = prompt.prompt("Name one Rust web framework.".into()).await?;
    let vectors = embed.embed(vec![answer]).await?;

    let agent = Agent::builder().llm(&openai).build()?;
    // ANCHOR_END: trait_objects
    let _ = (vectors, agent);
    Ok(())
}

/// An OpenAI-compatible server on a custom base URL.
pub fn custom_base_url() -> anyhow::Result<()> {
    // ANCHOR: custom_base_url
    let local = OpenAI::builder()
        .api_base("http://localhost:8000/v1")
        .api_key("not-needed-locally")
        .default_prompt_model("qwen2.5-coder")
        .build()?;
    // ANCHOR_END: custom_base_url
    let _ = local;
    Ok(())
}

/// Extra top-level request fields for a provider that needs them.
pub fn extra_body() -> anyhow::Result<()> {
    // ANCHOR: extra_body
    let mut extra = serde_json::Map::new();
    extra.insert("top_k".into(), serde_json::json!(20));

    let ollama = Ollama::builder()
        .default_prompt_model("llama3.2")
        .default_options(Options::builder().extra_body(extra))
        .build()?;
    // ANCHOR_END: extra_body
    let _ = ollama;
    Ok(())
}

/// A reasoning model on the Responses API, with its reasoning kept in the message history.
pub async fn responses_api_reasoning() -> anyhow::Result<()> {
    // ANCHOR: responses_api_reasoning
    let openai = OpenAI::builder()
        .default_prompt_model("gpt-5-mini")
        .use_responses_api(true)
        .default_options(Options::builder().reasoning_effort(ReasoningEffort::Medium))
        .build()?;

    let mut agent = Agent::builder().llm(&openai).build()?;
    agent
        .query("Which crate in this workspace owns the agent loop?")
        .await?;

    for message in agent.history().await? {
        if let ChatMessage::Reasoning(item) = message {
            println!("reasoning summary: {:?}", item.summary);
        }
    }
    // ANCHOR_END: responses_api_reasoning
    Ok(())
}

/// Request settings that every request from this client carries.
pub fn request_settings() -> anyhow::Result<()> {
    // ANCHOR: request_settings
    let openai = OpenAI::builder()
        .default_prompt_model("gpt-5-mini")
        .default_embed_model("text-embedding-3-small")
        .default_options(
            Options::builder()
                .max_completion_tokens(2_048)
                .reasoning_effort(ReasoningEffort::Low)
                .seed(42)
                .metadata(serde_json::json!({ "team": "docs" }))
                .dimensions(512),
        )
        .parallel_tool_calls(Some(false))
        .for_end_user("user-1234")
        .build()?;
    // ANCHOR_END: request_settings
    let _ = openai;
    Ok(())
}

/// Change the settings of one client without touching its clones.
pub fn options_mut(openai: &OpenAI) {
    // ANCHOR: options_mut
    let mut creative = openai.clone();
    creative.options_mut().temperature = Some(1.2);
    creative.options_mut().presence_penalty = Some(0.5);
    // ANCHOR_END: options_mut
    let _ = creative;
}
