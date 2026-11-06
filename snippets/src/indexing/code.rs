//! Snippets for `indexing/code-aware-indexing`.

use swiftide::integrations::treesitter::{CodeSplitter, SupportedLanguages};

/// Splitting source code with tree-sitter, without a pipeline.
pub fn code_splitter(source: &str) -> anyhow::Result<Vec<String>> {
    // ANCHOR: code_splitter
    // Case-insensitive; "c#", "c-sharp", "cpp" and "c++" parse too
    let language: SupportedLanguages = "python".parse()?;
    assert_eq!(language.file_extensions(), ["py"]);

    // Drop chunks of 100 bytes or less, split anything above 1500 bytes
    let splitter = CodeSplitter::builder()
        .try_language(language)?
        .chunk_size(100..1500)
        .build()?;

    let chunks = splitter.split(source)?;

    // Code with a syntax error comes back whole, as one chunk
    let broken = splitter.split("def main(:\n    pass")?;
    assert_eq!(broken.len(), 1);
    // ANCHOR_END: code_splitter
    Ok(chunks)
}
