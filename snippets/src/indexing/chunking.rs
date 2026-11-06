//! Snippets for `indexing/chunking`.

use swiftide::{
    indexing::{
        IndexingStream, Pipeline, TextNode,
        loaders::FileLoader,
        persist::MemoryStorage,
        statistics::PipelineStats,
        transformers::{ChunkCode, ChunkMarkdown, ChunkText},
    },
    integrations::treesitter::SupportedLanguages,
};

/// Tree-sitter chunkers with the default size and with a byte range.
pub async fn code_chunkers() -> anyhow::Result<PipelineStats> {
    // ANCHOR: code_chunkers
    // At most 1500 bytes per chunk, the default
    let python = ChunkCode::try_for_language("python")?;

    // At most 2000 bytes per chunk; chunks of 200 bytes or less are dropped
    let rust = ChunkCode::try_for_language_and_chunk_size(SupportedLanguages::Rust, 200..2000)?
        .with_concurrency(4);

    Pipeline::from_loader(FileLoader::new("src").with_extensions(&["rs"]))
        .then_chunk(rust)
        .then_store_with(MemoryStorage::default())
        .run()
        .await?;
    // ANCHOR_END: code_chunkers

    Pipeline::from_loader(FileLoader::new("scripts").with_extensions(&["py"]))
        .then_chunk(python)
        .then_store_with(MemoryStorage::default())
        .run()
        .await
}

/// Character based chunkers for markdown and plain text.
pub fn text_chunkers() -> (ChunkMarkdown, ChunkText) {
    // ANCHOR: text_chunkers
    // Aims for 100 characters, at most 1499; chunks under 100 bytes after trimming are dropped
    let markdown = ChunkMarkdown::from_chunk_range(100..1500);

    // At most 1000 characters; only blank chunks are dropped
    let text = ChunkText::from_max_characters(1000);
    // ANCHOR_END: text_chunkers
    (markdown, text)
}

/// A closure as a chunker: one chunk per paragraph.
pub async fn closure_chunker() -> anyhow::Result<()> {
    // ANCHOR: closure_chunker
    let paragraphs = |node: TextNode| {
        let chunks: Vec<String> = node
            .chunk
            .split("\n\n")
            .map(str::trim)
            .filter(|paragraph| !paragraph.is_empty())
            .map(str::to_owned)
            .collect();

        // Each chunk keeps the path, metadata and embed mode of `node`
        IndexingStream::iter(node.into_chunks(chunks).map(Ok))
    };

    Pipeline::from_loader(FileLoader::new("notes").with_extensions(&["txt"]))
        .then_chunk(paragraphs)
        .then_store_with(MemoryStorage::default())
        .run()
        .await?;
    // ANCHOR_END: closure_chunker
    Ok(())
}

/// `then_expand`: one node per record, then chunking.
pub async fn then_expand() -> anyhow::Result<()> {
    // ANCHOR: then_expand
    // A JSON Lines file holds one record per line
    let records = |node: TextNode| {
        let lines: Vec<String> = node
            .chunk
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(str::to_owned)
            .collect();

        IndexingStream::iter(
            node.into_chunks(lines)
                .enumerate()
                .map(|(index, mut record)| {
                    record.metadata.insert("Record", index);
                    Ok(record)
                }),
        )
    };

    Pipeline::from_loader(FileLoader::new("exports").with_extensions(&["jsonl"]))
        .then_expand(records)
        .then_chunk(ChunkText::from_max_characters(1000))
        .then_store_with(MemoryStorage::default())
        .run()
        .await?;
    // ANCHOR_END: then_expand
    Ok(())
}
