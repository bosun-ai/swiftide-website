//! Landing page: index a Rust codebase, then answer a question over it.
//!
//! Hand-formatted to a narrow width: the landing page shows three excerpts side by side.
#![cfg_attr(rustfmt, rustfmt_skip)]

use swiftide::{
    indexing::{
        Pipeline,
        loaders::FileLoader,
        transformers::{ChunkCode, Embed, MetadataQACode},
    },
    integrations::{openai::OpenAI, qdrant::Qdrant},
    query::{self, answers::Simple, query_transformers::Embed as EmbedQuery},
};

pub async fn ask_your_codebase(openai: OpenAI, qdrant: Qdrant) -> anyhow::Result<()> {
    // ANCHOR: pipeline
    let code = FileLoader::new("src")
        .with_extensions(&["rs"]);
    let chunker = ChunkCode::try_for_language("rust")?;

    Pipeline::from_loader(code)
        .with_default_llm_client(openai.clone())
        .then_chunk(chunker)
        .then(MetadataQACode::default())
        .then_in_batch(Embed::new(openai.clone()))
        .then_store_with(qdrant.clone())
        .run()
        .await?;

    let embed = EmbedQuery::from_client(openai.clone());
    let answer = Simple::from_client(openai);
    let result = query::Pipeline::default()
        .then_transform_query(embed)
        .then_retrieve(qdrant)
        .then_answer(answer)
        .query("When does an agent stop?")
        .await?;
    // ANCHOR_END: pipeline
    println!("{}", result.answer());
    Ok(())
}
