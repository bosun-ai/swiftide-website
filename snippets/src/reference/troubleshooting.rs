//! Snippets for `reference/troubleshooting-and-faq`.

use swiftide::{
    agents::Agent,
    indexing::{
        self,
        loaders::FileLoader,
        transformers::{ChunkMarkdown, Embed},
    },
    integrations::{openai::OpenAI, pgvector::PgVector, qdrant::Qdrant},
    query::{self, answers, search_strategies::SimilaritySingleEmbedding},
};

/// Logs failed nodes and keeps indexing instead of stopping at the first error.
pub async fn pipeline_errors(openai: OpenAI, qdrant: Qdrant) -> anyhow::Result<()> {
    // ANCHOR: pipeline_errors
    indexing::Pipeline::from_loader(FileLoader::new("docs").with_extensions(&["md"]))
        .then_chunk(ChunkMarkdown::from_chunk_range(100..2048))
        .then_in_batch(Embed::new(openai))
        .log_errors()
        .filter_errors()
        .then_store_with(qdrant)
        .run()
        .await?;
    // ANCHOR_END: pipeline_errors
    Ok(())
}

/// Streams the answer as it arrives, caps the loop and reports why the agent stopped.
pub async fn on_stream(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: on_stream
    let mut agent = Agent::builder()
        .llm(&openai)
        .on_stream(|_agent, response| {
            if let Some(chunk) = response
                .delta
                .as_ref()
                .and_then(|d| d.message_chunk.as_deref())
            {
                print!("{chunk}");
            }
            Box::pin(async move { Ok(()) })
        })
        .limit(10)
        .build()?;

    agent.query("Explain Rust lifetimes").await?;

    if let Some(reason) = agent.stop_reason() {
        println!("stopped: {reason:?}");
    }
    // ANCHOR_END: on_stream
    Ok(())
}

/// Names the filter type of the search strategy so the retriever impl can be picked.
pub async fn similarity_annotation(openai: OpenAI, pgvector: PgVector) -> anyhow::Result<()> {
    // ANCHOR: similarity_annotation
    let pipeline =
        query::Pipeline::from_search_strategy(SimilaritySingleEmbedding::<()>::default())
            .then_transform_query(query::query_transformers::Embed::from_client(
                openai.clone(),
            ))
            .then_retrieve(pgvector)
            .then_answer(answers::Simple::from_client(openai));
    // ANCHOR_END: similarity_annotation
    let _answer = pipeline.query("What is Swiftide?").await?;
    Ok(())
}
