//! Snippets for `integrations/openai`.

use swiftide::{
    indexing::{
        self,
        loaders::FileLoader,
        persist::MemoryStorage,
        transformers::{ChunkMarkdown, Embed},
    },
    integrations::openai::{AzureConfig, GenericOpenAI, GenericOpenAIBuilder, OpenAI, Options},
    traits::{EmbeddingModel, SimplePrompt},
};

/// An Azure OpenAI deployment behind the same client type as OpenAI.
pub async fn azure() -> anyhow::Result<()> {
    // ANCHOR: azure
    let config = AzureConfig::new()
        .with_api_base("https://my-resource.openai.azure.com")
        .with_api_version("2024-10-21")
        .with_deployment_id("gpt-4o-mini")
        .with_api_key(std::env::var("AZURE_OPENAI_API_KEY")?);

    let azure: GenericOpenAI<AzureConfig> = GenericOpenAIBuilder::default()
        .client(async_openai::Client::with_config(config))
        .default_prompt_model("gpt-4o-mini")
        .build()?;

    let answer = azure.prompt("Name one Rust web framework.".into()).await?;
    // ANCHOR_END: azure
    println!("{answer}");
    Ok(())
}

/// OpenAI embeddings with fewer dimensions, called directly and from an indexing pipeline.
pub async fn embeddings() -> anyhow::Result<()> {
    // ANCHOR: embeddings
    let openai = OpenAI::builder()
        .default_embed_model("text-embedding-3-small")
        .default_options(Options::builder().dimensions(512))
        .build()?;

    let vectors = openai
        .embed(vec!["What is a borrow checker?".into()])
        .await?;
    println!("{} dimensions", vectors[0].len());

    indexing::Pipeline::from_loader(FileLoader::new("docs").with_extensions(&["md"]))
        .then_chunk(ChunkMarkdown::from_chunk_range(100..2048))
        .then_in_batch(Embed::new(openai.clone()).with_batch_size(64))
        .then_store_with(MemoryStorage::default())
        .run()
        .await?;
    // ANCHOR_END: embeddings
    Ok(())
}
