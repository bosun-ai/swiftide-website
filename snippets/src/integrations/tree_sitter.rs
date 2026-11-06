//! Snippets for `integrations/tree-sitter`.

use swiftide::{
    indexing::{
        self, TextNode,
        loaders::FileLoader,
        persist::MemoryStorage,
        transformers::{
            ChunkCode, MetadataRefsDefsCode,
            metadata_refs_defs_code::{NAME_DEFINITIONS, NAME_REFERENCES},
        },
    },
    integrations::treesitter::SupportedLanguages,
    traits::Transformer,
};

/// A language parsed from a string, used to pick both the files and the grammar.
pub async fn languages() -> anyhow::Result<()> {
    // ANCHOR: languages
    // Case-insensitive: "python", "Python" and "PYTHON" all parse
    let language: SupportedLanguages = "python".parse()?;

    indexing::Pipeline::from_loader(
        FileLoader::new("src").with_extensions(language.file_extensions()),
    )
    .then_chunk(ChunkCode::try_for_language(language)?)
    .then_store_with(MemoryStorage::default())
    .run()
    .await?;
    // ANCHOR_END: languages
    Ok(())
}

/// References and definitions as metadata, in a pipeline and on a single node.
pub async fn refs_defs() -> anyhow::Result<()> {
    // ANCHOR: refs_defs
    indexing::Pipeline::from_loader(FileLoader::new("src").with_extensions(&["rs"]))
        .then_chunk(ChunkCode::try_for_language_and_chunk_size(
            "rust",
            10..2048,
        )?)
        .then(MetadataRefsDefsCode::try_from_language("rust")?)
        .then_store_with(MemoryStorage::default())
        .run()
        .await?;

    // The same transformer on one node
    let refs_defs = MetadataRefsDefsCode::try_from_language("rust")?;
    let node = refs_defs
        .transform_node(TextNode::new(r#"fn main() { println!("Hello"); }"#))
        .await?;

    assert_eq!(node.metadata.get(NAME_REFERENCES).unwrap(), "println");
    assert_eq!(node.metadata.get(NAME_DEFINITIONS).unwrap(), "main");
    // ANCHOR_END: refs_defs
    Ok(())
}
