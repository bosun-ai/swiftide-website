//! Snippets for `querying/transforming-queries`.

use async_trait::async_trait;
use swiftide::{
    integrations::{openai::OpenAI, qdrant::Qdrant},
    query::{
        self, Query, answers,
        query_transformers::{Embed, GenerateSubquestions},
        states,
    },
    traits::{EmbeddingModel, TransformQuery},
};

/// Subquestions, then an embedding of the result, then retrieval; and the history of the answer.
pub async fn subquestions(
    openai: OpenAI,
    embedder: impl EmbeddingModel + 'static,
    qdrant: Qdrant,
) -> anyhow::Result<()> {
    // ANCHOR: subquestions
    let pipeline = query::Pipeline::default()
        .then_transform_query(
            GenerateSubquestions::builder()
                .client(openai.clone())
                .num_questions(3)
                .build()?,
        )
        // Embed the text that the step above produced, with the model the chunks were embedded with
        .then_transform_query(Embed::from_client(embedder))
        .then_retrieve(qdrant)
        .then_answer(answers::Simple::from_client(openai));
    // ANCHOR_END: subquestions

    // ANCHOR: history
    let answered = pipeline.query("How does an agent decide to stop?").await?;

    for event in answered.history() {
        if let Some(documents) = event.documents() {
            println!(
                "retrieved {} documents for:\n{}",
                documents.len(),
                event.before()
            );
        } else {
            println!("rewrote:\n{}\ninto:\n{}", event.before(), event.after());
        }
    }
    // ANCHOR_END: history
    Ok(())
}

// ANCHOR: custom_transformer
/// Puts the instruction that BGE embedding models expect for search queries in front of the
/// question.
#[derive(Clone)]
pub struct BgeQueryInstruction;

#[async_trait]
impl TransformQuery for BgeQueryInstruction {
    async fn transform_query(
        &self,
        mut query: Query<states::Pending>,
    ) -> anyhow::Result<Query<states::Pending>> {
        let instructed = format!(
            "Represent this sentence for searching relevant passages: {}",
            query.current()
        );
        query.transformed_query(instructed);

        Ok(query)
    }
}
// ANCHOR_END: custom_transformer

/// The custom query transformer and a closure, placed before the embedding step.
pub fn custom_transformer_pipeline(
    openai: OpenAI,
    embedder: impl EmbeddingModel + 'static,
    qdrant: Qdrant,
) {
    // ANCHOR: custom_transformer_pipeline
    let pipeline = query::Pipeline::default()
        // A closure is enough for a step without state
        .then_transform_query(|mut query: Query<states::Pending>| {
            let trimmed = query.current().trim().to_string();
            query.transformed_query(trimmed);
            Ok(query)
        })
        .then_transform_query(BgeQueryInstruction)
        .then_transform_query(Embed::from_client(embedder))
        .then_retrieve(qdrant)
        .then_answer(answers::Simple::from_client(openai));
    // ANCHOR_END: custom_transformer_pipeline
    let _ = pipeline;
}
