//! Snippets for `integrations/qdrant`.

use swiftide::{
    integrations::{
        openai::OpenAI,
        qdrant::{
            Qdrant,
            qdrant_client::qdrant::{Condition, Filter},
        },
    },
    query::{
        self, answers, query_transformers,
        search_strategies::{HybridSearch, SimilaritySingleEmbedding},
    },
};

/// Narrows a Qdrant search with a payload filter, for similarity and hybrid search.
pub async fn filter(openai: OpenAI, qdrant: Qdrant) -> anyhow::Result<()> {
    // ANCHOR: filter
    // Payload keys are the node's metadata plus `path`, `content` and `last_updated_at`
    let guides = Filter {
        must: vec![Condition::matches("path", ["README.md", "CONTRIBUTING.md"])],
        must_not: vec![Condition::matches("deprecated", true)],
        ..Default::default()
    };

    let similarity = SimilaritySingleEmbedding::from_filter(guides.clone()).with_top_k(5);

    let pipeline = query::Pipeline::from_search_strategy(similarity)
        .then_transform_query(query_transformers::Embed::from_client(openai.clone()))
        .then_retrieve(qdrant)
        .then_answer(answers::Simple::from_client(openai));

    // On a hybrid search, the filter applies to the dense and the sparse candidates
    let hybrid = HybridSearch::from_filter(guides).with_top_n(20);
    // ANCHOR_END: filter

    let _hybrid_pipeline = query::Pipeline::from_search_strategy(hybrid);
    let _answer = pipeline.query("How do I run the tests?").await?;
    Ok(())
}
