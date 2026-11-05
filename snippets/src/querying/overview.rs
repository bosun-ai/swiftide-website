//! Snippets for `querying/overview`.

use swiftide::{
    integrations::{openai::OpenAI, qdrant::Qdrant},
    query::{
        self, Document, Query, answers, query_transformers,
        search_strategies::SimilaritySingleEmbedding, states,
    },
};

/// A query pipeline with a closure at every step. It needs no model and no store.
pub async fn closure_steps() -> anyhow::Result<()> {
    // ANCHOR: closure_steps
    let pipeline = query::Pipeline::default()
        .then_transform_query(|mut query: Query<states::Pending>| {
            let lowercased = query.current().to_lowercase();
            query.transformed_query(lowercased);
            Ok(query)
        })
        .then_retrieve(
            |_: &SimilaritySingleEmbedding, query: Query<states::Pending>| {
                let documents = vec![Document::from(
                    "Swiftide is a Rust library for LLM applications.",
                )];
                Ok(query.retrieved_documents(documents))
            },
        )
        .then_transform_response(Ok)
        .then_answer(|query: Query<states::Retrieved>| {
            let answer = format!("Found {} document(s)", query.documents().len());
            Ok(query.answered(answer))
        });

    let answered = pipeline.query("What is Swiftide?").await?;
    println!("{}", answered.answer());
    // ANCHOR_END: closure_steps
    Ok(())
}

/// Runs one pipeline for several questions in a row, and another for a batch of questions.
pub async fn reuse(openai: OpenAI, qdrant: Qdrant) -> anyhow::Result<()> {
    // ANCHOR: reuse
    let mut pipeline = query::Pipeline::default()
        .then_transform_query(query_transformers::Embed::from_client(openai.clone()))
        .then_retrieve(qdrant.clone())
        .then_answer(answers::Simple::from_client(openai.clone()));

    let first = pipeline.query_mut("What is a query pipeline?").await?;
    let second = pipeline.query_mut("What does a retriever return?").await?;
    println!("{}\n\n{}", first.answer(), second.answer());

    let answered = query::Pipeline::default()
        .then_transform_query(query_transformers::Embed::from_client(openai.clone()))
        .then_retrieve(qdrant)
        .then_answer(answers::Simple::from_client(openai))
        .query_all(vec!["What is a node?", "What is a chunk?"])
        .await?;

    for query in answered {
        println!("{}: {}", query.original(), query.answer());
    }
    // ANCHOR_END: reuse
    Ok(())
}
