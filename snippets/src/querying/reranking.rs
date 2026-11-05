//! Snippets for `querying/reranking-and-response-transformers`.

use std::collections::HashSet;

use anyhow::Result;
use async_trait::async_trait;
use swiftide::{
    integrations::{openai::OpenAI, qdrant::Qdrant},
    prompt::Prompt,
    query::{self, Query, answers, query_transformers, response_transformers, states},
    traits::TransformResponse,
};

// ANCHOR: custom_response_transformer
/// Keeps the first document of each file, in retrieval order, for at most `max` files.
#[derive(Clone)]
pub struct OnePerFile {
    pub max: usize,
}

#[async_trait]
impl TransformResponse for OnePerFile {
    async fn transform_response(
        &self,
        mut query: Query<states::Retrieved>,
    ) -> Result<Query<states::Retrieved>> {
        let mut seen_paths = HashSet::new();
        let documents = query.documents_mut();

        documents.retain(|document| {
            let path = document
                .metadata()
                .get("path")
                .and_then(|value| value.as_str())
                .unwrap_or_default();
            seen_paths.insert(path.to_string())
        });
        documents.truncate(self.max);

        Ok(query)
    }
}
// ANCHOR_END: custom_response_transformer

/// Places the custom response transformer after retrieval.
pub fn one_per_file_pipeline(openai: OpenAI, qdrant: Qdrant) {
    // ANCHOR: one_per_file_pipeline
    let pipeline = query::Pipeline::default()
        .then_transform_query(query_transformers::Embed::from_client(openai.clone()))
        .then_retrieve(qdrant)
        .then_transform_response(OnePerFile { max: 8 })
        .then_answer(answers::Simple::from_client(openai));
    // ANCHOR_END: one_per_file_pipeline
    let _ = pipeline;
}

/// A response transformer written as a closure.
pub fn closure_response_transformer(openai: OpenAI, qdrant: Qdrant) {
    // ANCHOR: closure_response_transformer
    let pipeline = query::Pipeline::default()
        .then_transform_query(query_transformers::Embed::from_client(openai.clone()))
        .then_retrieve(qdrant)
        .then_transform_response(|mut query: Query<states::Retrieved>| {
            query.documents_mut().truncate(5);
            Ok(query)
        })
        .then_answer(answers::Simple::from_client(openai));
    // ANCHOR_END: closure_response_transformer
    let _ = pipeline;
}

/// Replaces the built-in summary prompt.
pub fn summary_prompt(openai: OpenAI) -> Result<()> {
    // ANCHOR: summary_prompt
    let summary = response_transformers::Summary::builder()
        .client(openai)
        .prompt_template(Prompt::from(
            "Summarize these excerpts as one markdown list. Keep every type name, \
             feature flag and command exactly as written.\n\
             {% for document in documents %}\n\
             ---\n\
             {{ document.metadata.path | default(value=\"unknown\") }}:\n\
             {{ document.content }}\n\
             {% endfor %}",
        ))
        .build()?;
    // ANCHOR_END: summary_prompt
    let _ = summary;
    Ok(())
}
