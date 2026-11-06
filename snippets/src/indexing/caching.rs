//! Snippets for `indexing/caching`.

use swiftide::{
    indexing::{
        Pipeline,
        loaders::FileLoader,
        transformers::{ChunkMarkdown, Embed, MetadataQAText},
    },
    integrations::{openai::OpenAI, qdrant::Qdrant, redis::Redis},
};

/// Keeps one failing chunk from leaving the whole run uncached.
pub async fn partial_progress(openai: OpenAI, redis_url: &str) -> anyhow::Result<()> {
    // ANCHOR: partial_progress
    // Change the prefix, for example to "docs-v2", to index every file again
    let cache = Redis::try_from_url(redis_url, "docs-v1")?;

    Pipeline::from_loader(FileLoader::new("docs").with_extensions(&["md"]))
        .filter_cached(cache)
        .then_chunk(ChunkMarkdown::from_chunk_range(100..1500))
        .then(MetadataQAText::new(openai.clone()))
        // Log and drop failed chunks so the run completes. Their files stay uncached and are
        // retried next run; every other file is cached.
        .log_errors()
        .filter_errors()
        .then_in_batch(Embed::new(openai).with_batch_size(32))
        .then_store_with(
            Qdrant::builder()
                .vector_size(1536)
                .collection_name("docs")
                .build()?,
        )
        .run()
        .await?;
    // ANCHOR_END: partial_progress

    Ok(())
}

// ANCHOR: custom_cache
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use swiftide::{indexing::TextNode, traits::NodeCache};
use uuid::Uuid;

/// Remembers indexed sources in memory, for tests or one-off runs.
#[derive(Debug, Clone, Default)]
pub struct InMemoryCache {
    indexed: Arc<Mutex<HashSet<Uuid>>>,
}

/// The same key the pipeline marks: the source id, shared by all chunks of a file.
fn source_id(node: &TextNode) -> Uuid {
    node.parent_id().unwrap_or_else(|| node.id())
}

#[async_trait]
impl NodeCache for InMemoryCache {
    type Input = String;

    async fn get(&self, node: &TextNode) -> bool {
        match self.indexed.lock() {
            Ok(indexed) => indexed.contains(&source_id(node)),
            Err(error) => {
                tracing::error!("node cache lock poisoned: {error}");
                false
            }
        }
    }

    async fn set(&self, node: &TextNode) {
        self.set_by_id(source_id(node)).await;
    }

    async fn set_by_id(&self, id: Uuid) {
        match self.indexed.lock() {
            Ok(mut indexed) => {
                indexed.insert(id);
            }
            Err(error) => tracing::error!("node cache lock poisoned: {error}"),
        }
    }
}
// ANCHOR_END: custom_cache
