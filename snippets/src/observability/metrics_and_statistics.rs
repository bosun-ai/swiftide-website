//! Snippets for `observability/metrics-and-statistics`.

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use swiftide::indexing::{
    self, TextNode, loaders::FileLoader, persist::MemoryStorage, statistics::StatsCollector,
    transformers::ChunkMarkdown,
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

/// Records a run's numbers in a `StatsCollector` the caller owns, because `run` consumes the
/// pipeline and its own collector with it.
pub async fn own_stats_collector() -> anyhow::Result<()> {
    // ANCHOR: own_stats_collector
    let stats = Arc::new(StatsCollector::new());
    let count_chunk = {
        let stats = Arc::clone(&stats);
        move |node: TextNode| -> anyhow::Result<TextNode> {
            stats.increment_nodes_processed(1);
            Ok(node)
        }
    };

    let pipeline =
        indexing::Pipeline::from_loader(FileLoader::new("docs").with_extensions(&["md"]))
            .then_chunk(ChunkMarkdown::from_chunk_range(100..2048))
            .then_store_with(MemoryStorage::default())
            .then(count_chunk);

    // The pipeline's own collector can be read before the run; it is still empty.
    assert_eq!(pipeline.stats().nodes_processed, 0);

    stats.start();
    pipeline.run().await?;
    stats.complete();

    let run = stats.get_stats();
    println!(
        "Stored {} chunks in {:?} ({:.1} chunks/s)",
        run.nodes_processed,
        run.duration().unwrap_or_default(),
        run.nodes_per_second().unwrap_or_default(),
    );
    // ANCHOR_END: own_stats_collector
    Ok(())
}
