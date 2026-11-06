//! Snippets for `agents/providers-and-reasoning`: one agent definition on several providers.

use swiftide::{
    agents::Agent,
    integrations::{
        anthropic::Anthropic,
        ollama::Ollama,
        openai::{AzureConfig, GenericOpenAI, GenericOpenAIBuilder, OpenAI},
    },
    traits::ChatCompletion,
};

// ANCHOR: build_agent
/// Builds the same agent for any chat completion client.
fn changelog_agent(llm: &(impl ChatCompletion + Clone + 'static)) -> anyhow::Result<Agent> {
    let agent = Agent::builder()
        .llm(llm)
        .system_prompt("You summarize git history for release notes.")
        .limit(20)
        .build()?;
    Ok(agent)
}
// ANCHOR_END: build_agent

/// Picks the provider at runtime; the agent type stays the same.
pub async fn provider_swap() -> anyhow::Result<()> {
    // ANCHOR: provider_swap
    let mut agent = match std::env::var("LLM_PROVIDER").as_deref() {
        Ok("anthropic") => changelog_agent(
            &Anthropic::builder()
                .default_prompt_model("claude-sonnet-4-5")
                .build()?,
        )?,
        Ok("ollama") => changelog_agent(&Ollama::builder().default_prompt_model("qwen3").build()?)?,
        Ok("azure") => {
            let config = AzureConfig::new()
                .with_api_base("https://my-resource.openai.azure.com")
                .with_api_version("2024-10-21")
                .with_deployment_id("gpt-4o-mini")
                .with_api_key(std::env::var("AZURE_OPENAI_API_KEY")?);
            let azure: GenericOpenAI<AzureConfig> = GenericOpenAIBuilder::default()
                .client(async_openai::Client::with_config(config))
                .default_prompt_model("gpt-4o-mini")
                .build()?;
            changelog_agent(&azure)?
        }
        _ => changelog_agent(
            &OpenAI::builder()
                .default_prompt_model("gpt-5-mini")
                .build()?,
        )?,
    };

    agent
        .query("Summarize the commits since the last tag.")
        .await?;
    // ANCHOR_END: provider_swap
    Ok(())
}
