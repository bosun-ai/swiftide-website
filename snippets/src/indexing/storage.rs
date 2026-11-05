//! Snippets for `indexing/storage`.

use std::{path::PathBuf, sync::Arc};

use async_trait::async_trait;
use swiftide::{
    indexing::{IndexingStream, Pipeline, TextNode, persist::MemoryStorage},
    traits::Persist,
};
use tokio::{fs::OpenOptions, io::AsyncWriteExt as _, sync::Mutex};

/// Stores nodes in memory and reads them back after the run.
pub async fn memory_storage() -> anyhow::Result<()> {
    // ANCHOR: memory_storage
    let storage = MemoryStorage::default();

    Pipeline::from_stream(vec![
        TextNode::new("Swiftide is a Rust library for building LLM applications."),
        TextNode::new("An indexing pipeline loads, transforms and stores nodes."),
    ])
    .then(|mut node: TextNode| -> anyhow::Result<TextNode> {
        let words = node.chunk.split_whitespace().count();
        node.metadata.insert("Words", words);
        Ok(node)
    })
    // Clones share the same map, so keep one to read from later
    .then_store_with(storage.clone())
    .run()
    .await?;

    for node in storage.get_all_values().await {
        println!("{:?} words: {}", node.metadata.get("Words"), node.chunk);
    }
    // ANCHOR_END: memory_storage
    Ok(())
}

// ANCHOR: custom_persist
/// Appends every node as one line of JSON to a file.
#[derive(Debug, Clone)]
pub struct JsonLines {
    path: PathBuf,
    // Batches are stored concurrently; one writer at a time keeps lines whole
    write_lock: Arc<Mutex<()>>,
}

impl JsonLines {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            write_lock: Arc::default(),
        }
    }

    async fn append(&self, nodes: &[TextNode]) -> anyhow::Result<()> {
        let mut lines = String::new();
        for node in nodes {
            lines.push_str(&serde_json::to_string(node)?);
            lines.push('\n');
        }

        let _guard = self.write_lock.lock().await;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;
        file.write_all(lines.as_bytes()).await?;
        Ok(())
    }
}

#[async_trait]
impl Persist for JsonLines {
    type Input = String;
    type Output = String;

    // Runs once, before the first node arrives
    async fn setup(&self) -> anyhow::Result<()> {
        if let Some(dir) = self.path.parent() {
            tokio::fs::create_dir_all(dir).await?;
        }
        Ok(())
    }

    // Used when `batch_size` returns `None`
    async fn store(&self, node: TextNode) -> anyhow::Result<TextNode> {
        self.append(std::slice::from_ref(&node)).await?;
        Ok(node)
    }

    // Used when `batch_size` returns `Some`; pass the stored nodes on
    async fn batch_store(&self, nodes: Vec<TextNode>) -> IndexingStream<String> {
        self.append(&nodes).await.map(|()| nodes).into()
    }

    fn batch_size(&self) -> Option<usize> {
        Some(100)
    }
}
// ANCHOR_END: custom_persist

/// Uses the custom storage in a pipeline.
pub async fn use_custom_persist() -> anyhow::Result<()> {
    // ANCHOR: use_custom_persist
    Pipeline::from_stream(vec![TextNode::new("Stored as a line of JSON.")])
        .then_store_with(JsonLines::new("index/nodes.jsonl"))
        .run()
        .await?;
    // ANCHOR_END: use_custom_persist
    Ok(())
}
