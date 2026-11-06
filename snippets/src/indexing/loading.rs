//! Snippets for `indexing/loading`.

use std::path::Path;

use swiftide::{
    agents::tools::local_executor::LocalExecutor,
    indexing::{
        Pipeline, TextNode, loaders::FileLoader, persist::MemoryStorage, transformers::ChunkText,
    },
    integrations::{parquet::Parquet, treesitter::SupportedLanguages},
    traits::ToolExecutor,
};

/// Loads Rust, Python and markdown files below `src`.
pub async fn file_loader() -> anyhow::Result<()> {
    // ANCHOR: file_loader
    let languages = [SupportedLanguages::Rust, SupportedLanguages::Python];
    let extensions: Vec<&str> = languages
        .iter()
        .flat_map(|language| language.file_extensions().iter().copied())
        .collect();

    // Extensions without the leading dot; each call adds to the list
    let loader = FileLoader::new("src")
        .with_extensions(&extensions)
        .with_extensions(&["md"]);

    Pipeline::from_loader(loader)
        .then_chunk(ChunkText::from_max_characters(2000))
        .then_store_with(MemoryStorage::default())
        .run()
        .await?;
    // ANCHOR_END: file_loader
    Ok(())
}

/// Indexes nodes that are already in memory.
pub async fn from_nodes() -> anyhow::Result<()> {
    // ANCHOR: from_nodes
    let mut faq = TextNode::new("A loader turns a data source into a stream of nodes.");
    faq.path = "faq/loading.md".into();
    faq.metadata.insert("Source", "faq");

    let nodes = vec![
        faq,
        TextNode::new("An indexing pipeline does nothing until you call run."),
    ];

    let storage = MemoryStorage::default();
    Pipeline::from_stream(nodes)
        .then_store_with(storage.clone())
        .run()
        .await?;

    println!("Stored {} nodes", storage.get_all_values().await.len());
    // ANCHOR_END: from_nodes
    Ok(())
}

/// Feeds a pipeline from a channel that another task writes to.
pub async fn from_channel() -> anyhow::Result<()> {
    // ANCHOR: from_channel
    let (sender, receiver) = tokio::sync::mpsc::channel(100);

    tokio::spawn(async move {
        for line in ["first message", "second message"] {
            if sender.send(Ok(TextNode::new(line))).await.is_err() {
                break;
            }
        }
        // Dropping the sender ends the stream, and with it the run
    });

    Pipeline::from_stream(receiver)
        .then_store_with(MemoryStorage::default())
        .run()
        .await?;
    // ANCHOR_END: from_channel
    Ok(())
}

/// Indexes the Rust files a tool executor can see.
pub async fn executor_files() -> anyhow::Result<()> {
    // ANCHOR: executor_files
    let executor = LocalExecutor::new(".");

    let files = executor
        .stream_files(Path::new("src"), Some(vec!["rs".to_string()]))
        .await?;

    Pipeline::from_stream(files)
        .then_chunk(ChunkText::from_max_characters(2000))
        .then_store_with(MemoryStorage::default())
        .run()
        .await?;
    // ANCHOR_END: executor_files
    Ok(())
}

/// Indexes the `chunk` column of a Parquet file.
pub async fn parquet() -> anyhow::Result<()> {
    // ANCHOR: parquet
    let loader = Parquet::builder()
        .path("data.parquet")
        .column_name("chunk")
        .build()?;

    Pipeline::from_loader(loader)
        .then_chunk(ChunkText::from_max_characters(2000))
        .then_store_with(MemoryStorage::default())
        .run()
        .await?;
    // ANCHOR_END: parquet
    Ok(())
}
