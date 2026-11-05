//! Snippets for `integrations/pgvector`.

use swiftide::{
    indexing::{
        self, EmbeddedField, TextNode,
        loaders::FileLoader,
        transformers::{ChunkMarkdown, Embed},
    },
    integrations::{openai::OpenAI, pgvector::PgVector},
    query::{self, answers, query_transformers, search_strategies::SimilaritySingleEmbedding},
};

/// PgVector on your own Postgres, with the file path kept in a metadata column for filters.
pub async fn own_database(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: own_database
    let pgvector = PgVector::builder()
        .db_url(std::env::var("DATABASE_URL")?)
        .vector_size(1536)
        .with_vector(EmbeddedField::Combined)
        .with_metadata("path")
        .table_name("docs_chunks")
        .build()?;

    indexing::Pipeline::from_loader(FileLoader::new("docs").with_extensions(&["md"]))
        .then_chunk(ChunkMarkdown::from_chunk_range(100..2048))
        // Every node needs each `with_metadata` field, or storing it fails
        .then(|mut node: TextNode| -> anyhow::Result<TextNode> {
            let path = node.path.to_string_lossy().into_owned();
            node.metadata.insert("path", path);
            Ok(node)
        })
        .then_in_batch(Embed::new(openai.clone()))
        .then_store_with(pgvector.clone())
        .run()
        .await?;
    // ANCHOR_END: own_database
    Ok(())
}

/// Restricts the search to chunks from one file, using the `path` field stored by `own_database`.
pub async fn filter_on_path(openai: OpenAI, pgvector: PgVector) -> anyhow::Result<()> {
    // ANCHOR: filter
    // `FileLoader::new("docs")` stores paths with the `docs/` prefix, and the match is exact
    let search =
        SimilaritySingleEmbedding::from_filter("path = docs/README.md".to_string()).with_top_k(5);

    let pipeline = query::Pipeline::from_search_strategy(search)
        .then_transform_query(query_transformers::Embed::from_client(openai.clone()))
        .then_retrieve(pgvector)
        .then_answer(answers::Simple::from_client(openai));
    // ANCHOR_END: filter

    let _answer = pipeline.query("How do I install Swiftide?").await?;
    Ok(())
}
