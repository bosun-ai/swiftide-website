//! Snippets for `querying/answering`.

use async_trait::async_trait;
use swiftide::{
    integrations::{openai::OpenAI, qdrant::Qdrant},
    prompt::Prompt,
    query::{self, Query, answers, query_transformers, states},
    traits::Answer,
};

/// An answer prompt that shows the model both the question and the rewritten query.
pub fn current_in_prompt(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: current_in_prompt
    let answerer = answers::Simple::builder()
        .client(openai)
        .prompt_template(Prompt::from(
            "Answer this question: {{ original }}\n\
             The search looked for: {{ current }}\n\
             Cite the file of every statement in backticks.\n\n\
             {{ documents }}",
        ))
        .document_template("File: {{ metadata.path }}\n{{ content }}")
        .build()?;
    // ANCHOR_END: current_in_prompt
    let _ = answerer;
    Ok(())
}

// ANCHOR: custom_answer
/// Answers with the LLM only when the retriever found something.
#[derive(Clone)]
pub struct AnswerWhenFound {
    inner: answers::Simple,
}

#[async_trait]
impl Answer for AnswerWhenFound {
    async fn answer(
        &self,
        query: Query<states::Retrieved>,
    ) -> anyhow::Result<Query<states::Answered>> {
        if query.documents().is_empty() {
            return Ok(query.answered("No matching documents were found."));
        }
        self.inner.answer(query).await
    }
}
// ANCHOR_END: custom_answer

/// The custom answer generator at the end of a pipeline.
pub async fn custom_answer_pipeline(openai: OpenAI, qdrant: Qdrant) -> anyhow::Result<()> {
    // ANCHOR: custom_answer_pipeline
    let pipeline = query::Pipeline::default()
        .then_transform_query(query_transformers::Embed::from_client(openai.clone()))
        .then_retrieve(qdrant)
        .then_answer(AnswerWhenFound {
            inner: answers::Simple::from_client(openai),
        });

    let answered = pipeline.query("How do I configure retries?").await?;
    println!("{}", answered.answer());
    // ANCHOR_END: custom_answer_pipeline
    Ok(())
}
