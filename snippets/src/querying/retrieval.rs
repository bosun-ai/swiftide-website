//! Snippets for `querying/retrieval-and-search-strategies`.

use async_trait::async_trait;
use swiftide::{
    integrations::{
        openai::OpenAI,
        pgvector::PgVector,
        qdrant::{
            Qdrant,
            qdrant_client::qdrant::{Condition, Filter},
        },
    },
    query::{
        self, Document, Query, answers, query_transformers,
        search_strategies::{CustomStrategy, HybridSearch, SimilaritySingleEmbedding},
        states,
    },
    traits::Retrieve,
};

/// Restricts a Qdrant search to chunks whose `path` payload matches.
pub async fn qdrant_filter(openai: OpenAI, qdrant: Qdrant) -> anyhow::Result<()> {
    // ANCHOR: qdrant_filter
    let only_readme = Filter::must([Condition::matches("path", "README.md".to_string())]);

    let pipeline = query::Pipeline::from_search_strategy(SimilaritySingleEmbedding::from_filter(
        only_readme.clone(),
    ))
    .then_transform_query(query_transformers::Embed::from_client(openai.clone()))
    .then_retrieve(qdrant)
    .then_answer(answers::Simple::from_client(openai));

    // The same filter on a hybrid search applies to the dense and the sparse search
    let _hybrid = HybridSearch::from_filter(only_readme).with_top_n(20);
    // ANCHOR_END: qdrant_filter

    let _answer = pipeline.query("How do I install Swiftide?").await?;
    Ok(())
}

/// Restricts a PgVector search with a `key = value` filter on a stored metadata field.
pub async fn pgvector_filter(openai: OpenAI, pgvector: PgVector) -> anyhow::Result<()> {
    // ANCHOR: pgvector_filter
    // `path` must be a metadata field on the table: `PgVector::builder().with_metadata("path")`.
    // Swiftide does not add it for you; insert `path` into each node's metadata while indexing.
    let search =
        SimilaritySingleEmbedding::from_filter("path = README.md".to_string()).with_top_k(5);

    let pipeline = query::Pipeline::from_search_strategy(search)
        .then_transform_query(query_transformers::Embed::from_client(openai.clone()))
        .then_retrieve(pgvector)
        .then_answer(answers::Simple::from_client(openai));
    // ANCHOR_END: pgvector_filter

    let _answer = pipeline.query("How do I install Swiftide?").await?;
    Ok(())
}

/// Runs a Postgres full-text search through PgVector instead of a vector search.
pub async fn pgvector_custom_strategy(openai: OpenAI, pgvector: PgVector) -> anyhow::Result<()> {
    // ANCHOR: pgvector_custom_strategy
    use sqlx::{Postgres, QueryBuilder};

    // The default table is `swiftide_pgv_store`; `meta_path` exists only with `.with_metadata("path")`
    let table = pgvector.get_table_name().to_owned();

    let keyword_search = CustomStrategy::from_query(move |query: &Query<states::Pending>| {
        // The retriever reads `id`, `chunk` and every `meta_` column from each row
        let mut sql =
            QueryBuilder::<Postgres>::new(format!("SELECT id, chunk, meta_path FROM {table}"));
        sql.push(" WHERE to_tsvector('english', chunk) @@ plainto_tsquery('english', ")
            .push_bind(query.current().to_string())
            .push(") LIMIT 10");
        Ok(sql)
    });

    let pipeline = query::Pipeline::from_search_strategy(keyword_search)
        .then_retrieve(pgvector)
        .then_answer(answers::Simple::from_client(openai));
    // ANCHOR_END: pgvector_custom_strategy

    let _answer = pipeline.query("feature flags").await?;
    Ok(())
}

// ANCHOR: custom_retriever
/// Keeps the documents that share a word with the question. No store, no embeddings.
#[derive(Clone)]
struct KeywordRetriever {
    documents: Vec<Document>,
}

#[async_trait]
impl Retrieve<SimilaritySingleEmbedding> for KeywordRetriever {
    async fn retrieve(
        &self,
        search_strategy: &SimilaritySingleEmbedding,
        query: Query<states::Pending>,
    ) -> anyhow::Result<Query<states::Retrieved>> {
        let words: Vec<String> = query
            .current()
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();

        let matches = self
            .documents
            .iter()
            .filter(|document| {
                let content = document.content().to_lowercase();
                words.iter().any(|word| content.contains(word.as_str()))
            })
            .take(usize::try_from(search_strategy.top_k())?)
            .cloned()
            .collect();

        Ok(query.retrieved_documents(matches))
    }
}
// ANCHOR_END: custom_retriever

/// Puts the keyword retriever in a pipeline.
pub async fn custom_retriever(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: custom_retriever_pipeline
    let retriever = KeywordRetriever {
        documents: vec![
            Document::from("Swiftide indexes data with an indexing pipeline."),
            Document::from("A query pipeline answers questions over indexed data."),
        ],
    };

    let pipeline = query::Pipeline::default()
        .then_retrieve(retriever)
        .then_answer(answers::Simple::from_client(openai));
    // ANCHOR_END: custom_retriever_pipeline

    let _answer = pipeline.query("What does a query pipeline do?").await?;
    Ok(())
}
