//! Snippets for `integrations/openai-compatible`.

use swiftide::{
    indexing::{
        self,
        loaders::FileLoader,
        transformers::{ChunkMarkdown, Embed},
    },
    integrations::{
        ollama::{Ollama, config::OllamaConfig},
        open_router::{OpenRouter, config::OpenRouterConfig},
        openai::{OpenAI, OpenAIConfig},
        qdrant::Qdrant,
    },
    query::{self, answers, query_transformers},
    traits::SimplePrompt,
};

/// Ollama embeds the chunks at indexing time and the question at query time.
pub async fn ollama_embeddings() -> anyhow::Result<()> {
    // ANCHOR: ollama_embeddings
    let ollama = Ollama::builder()
        .default_prompt_model("llama3.2")
        .default_embed_model("nomic-embed-text")
        .build()?;

    // nomic-embed-text returns 768 dimensions.
    let qdrant = Qdrant::builder()
        .vector_size(768)
        .collection_name("local-docs")
        .build()?;

    indexing::Pipeline::from_loader(FileLoader::new("docs").with_extensions(&["md"]))
        .then_chunk(ChunkMarkdown::from_chunk_range(100..2048))
        .then_in_batch(Embed::new(ollama.clone()).with_batch_size(32))
        .then_store_with(qdrant.clone())
        .run()
        .await?;

    let answered = query::Pipeline::default()
        .then_transform_query(query_transformers::Embed::from_client(ollama.clone()))
        .then_retrieve(qdrant)
        .then_answer(answers::Simple::from_client(ollama))
        .query("How do I configure the loader?")
        .await?;
    // ANCHOR_END: ollama_embeddings
    println!("{}", answered.answer());
    Ok(())
}

/// Ollama on another machine.
pub async fn ollama_remote() -> anyhow::Result<()> {
    // ANCHOR: ollama_remote
    let mut config = OllamaConfig::default();
    config.with_api_base("http://gpu-box:11434/v1");

    let ollama = Ollama::builder()
        .client(async_openai::Client::with_config(config))
        .default_prompt_model("llama3.2")
        .build()?;

    let answer = ollama.prompt("Name one Rust web framework.".into()).await?;
    // ANCHOR_END: ollama_remote
    println!("{answer}");
    Ok(())
}

/// OpenRouter with the key from the environment, and with a key and site URL set in code.
pub fn openrouter() -> anyhow::Result<()> {
    // ANCHOR: openrouter
    let openrouter = OpenRouter::builder()
        .default_prompt_model("anthropic/claude-sonnet-4.5")
        .build()?;

    let config = OpenRouterConfig::builder()
        .api_key(std::env::var("TEAM_OPENROUTER_KEY")?)
        .site_url("https://example.com/my-app")
        .site_name("My App")
        .build()?;

    let team_openrouter = OpenRouter::builder()
        .client(async_openai::Client::with_config(config))
        .default_prompt_model("openai/gpt-5-mini")
        .build()?;
    // ANCHOR_END: openrouter
    let _ = (openrouter, team_openrouter);
    Ok(())
}

/// Groq with a key from another variable, through the plain OpenAI client.
pub fn gateway() -> anyhow::Result<()> {
    // ANCHOR: gateway
    let config = OpenAIConfig::new()
        .with_api_base("https://api.groq.com/openai/v1")
        .with_api_key(std::env::var("TEAM_GROQ_KEY")?);

    let groq = OpenAI::builder()
        .client(async_openai::Client::with_config(config))
        .default_prompt_model("llama-3.3-70b-versatile")
        .build()?;
    // ANCHOR_END: gateway
    let _ = groq;
    Ok(())
}
