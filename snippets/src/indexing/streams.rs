//! Snippets for `indexing/streams-and-concurrency`.

use std::time::Duration;

use swiftide::{
    indexing::{
        self,
        loaders::FileLoader,
        persist::MemoryStorage,
        transformers::{ChunkCode, ChunkMarkdown, Embed, MetadataQAText},
    },
    integrations::openai::OpenAI,
};

/// Caps the nodes in flight per step, limits the rate into an LLM step and gives one step its
/// own concurrency.
pub async fn concurrency(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: concurrency
    indexing::Pipeline::from_loader(FileLoader::new("docs").with_extensions(&["md"]))
        // Applies to every step added after this call
        .with_concurrency(8)
        .then_chunk(ChunkMarkdown::from_chunk_range(100..2048))
        // At most one chunk every 50ms enters the next step
        .throttle(Duration::from_millis(50))
        // Its own concurrency wins over the pipeline's 8
        .then(MetadataQAText::new(openai.clone()).with_concurrency(2))
        .then_in_batch(Embed::new(openai).with_batch_size(64))
        .then_store_with(MemoryStorage::default())
        .run()
        .await?;
    // ANCHOR_END: concurrency
    Ok(())
}

/// Chunks markdown and Rust files differently and stores both through one storage step.
pub async fn split_merge() -> anyhow::Result<()> {
    // ANCHOR: split_merge
    let (markdown, code) =
        indexing::Pipeline::from_loader(FileLoader::new(".").with_extensions(&["md", "rs"]))
            .split_by(|result| {
                result
                    .as_ref()
                    .is_ok_and(|node| node.path.extension().is_some_and(|ext| ext == "md"))
            });

    let markdown = markdown.then_chunk(ChunkMarkdown::from_chunk_range(100..2048));
    let code = code.then_chunk(ChunkCode::try_for_language("rust")?);

    markdown
        .merge(code)
        .then_store_with(MemoryStorage::default())
        .run()
        .await?;
    // ANCHOR_END: split_merge
    Ok(())
}

/// Drops empty files, logs failed nodes and drops them before the batch step, then logs what is
/// stored.
pub async fn errors_and_logging(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: errors_and_logging
    indexing::Pipeline::from_loader(FileLoader::new("docs").with_extensions(&["md"]))
        .filter(|result| match result {
            Ok(node) => !node.chunk.trim().is_empty(),
            // Keep errors, so `log_errors` sees them
            Err(_) => true,
        })
        .then_chunk(ChunkMarkdown::from_chunk_range(100..2048))
        .then(MetadataQAText::new(openai.clone()))
        // Drop failed nodes before the batch step, so an error does not take its batch with it
        .log_errors()
        .filter_errors()
        .then_in_batch(Embed::new(openai).with_batch_size(64))
        .log_nodes()
        .then_store_with(MemoryStorage::default())
        .run()
        .await?;
    // ANCHOR_END: errors_and_logging
    Ok(())
}
