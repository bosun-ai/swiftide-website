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

/// Reads the statistics `run` returns at the end of a run.
pub async fn run_stats() -> anyhow::Result<()> {
    // ANCHOR: run_stats
    let pipeline =
        indexing::Pipeline::from_loader(FileLoader::new("docs").with_extensions(&["md"]))
            .then_chunk(ChunkMarkdown::from_chunk_range(100..2048))
            .then_store_with(MemoryStorage::default());

    let stats = pipeline.run().await?;
    println!(
        "Stored {} chunks in {:?} ({:.1} chunks/s)",
        stats.nodes_stored,
        stats.duration().unwrap_or_default(),
        stats.nodes_per_second().unwrap_or_default(),
    );
    // ANCHOR_END: run_stats
    Ok(())
}

/// Watches a running pipeline through a handle on its shared collector.
pub async fn watch_run() -> anyhow::Result<()> {
    // ANCHOR: watch_run
    let pipeline =
        indexing::Pipeline::from_loader(FileLoader::new("docs").with_extensions(&["md"]))
            .then_chunk(ChunkMarkdown::from_chunk_range(100..2048))
            .then_store_with(MemoryStorage::default());

    // Take the handle before `run` consumes the pipeline
    let collector = Arc::clone(pipeline.stats_collector());
    let running = tokio::spawn(pipeline.run());

    while !running.is_finished() {
        println!("Stored {} chunks so far", collector.get_stats().nodes_stored);
        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    }

    let stats = running.await??;
    println!("Stored {} chunks", stats.nodes_stored);
    // ANCHOR_END: watch_run
    Ok(())
}
