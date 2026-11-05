//! Snippets for `observability/metrics-and-statistics`.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use swiftide::indexing::{
    self, TextNode, loaders::FileLoader, persist::MemoryStorage, transformers::ChunkMarkdown,
};

/// Counts the chunks that reach storage with a pass-through closure transformer.
pub async fn count_nodes() -> anyhow::Result<u64> {
    // ANCHOR: count_nodes
    let chunks_stored = Arc::new(AtomicU64::new(0));
    let count_chunk = {
        let chunks_stored = Arc::clone(&chunks_stored);
        move |node: TextNode| -> anyhow::Result<TextNode> {
            chunks_stored.fetch_add(1, Ordering::Relaxed);
            Ok(node)
        }
    };

    indexing::Pipeline::from_loader(FileLoader::new("docs").with_extensions(&["md"]))
        .then_chunk(ChunkMarkdown::from_chunk_range(100..2048))
        .then_store_with(MemoryStorage::default())
        .then(count_chunk)
        .run()
        .await?;

    let chunks_stored = chunks_stored.load(Ordering::Relaxed);
    println!("Stored {chunks_stored} chunks");
    // ANCHOR_END: count_nodes
    Ok(chunks_stored)
}
