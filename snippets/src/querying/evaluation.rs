//! Snippets for `querying/evaluation`.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use swiftide::{
    integrations::{openai::OpenAI, qdrant::Qdrant},
    query::{
        self, QueryEvaluation, answers,
        evaluators::ragas::{EvaluationDataSet, Ragas},
        query_transformers,
    },
    traits::EvaluateQuery,
};

/// Evaluates a pipeline on questions with hand-written ground truths and writes the dataset.
pub async fn prepare_questions(openai: OpenAI, qdrant: Qdrant) -> anyhow::Result<()> {
    // ANCHOR: prepare_questions
    let ragas = Ragas::from_prepared_questions(vec![
        (
            "What does a retriever return?".to_string(),
            "A retriever returns documents that match the query.".to_string(),
        ),
        (
            "Which step turns retrieved documents into an answer?".to_string(),
            "The answer generator, for example answers::Simple.".to_string(),
        ),
    ]);

    let pipeline = query::Pipeline::default()
        // Attach the evaluator while the pipeline is still pending
        .evaluate_with(ragas.clone())
        .then_transform_query(query_transformers::Embed::from_client(openai.clone()))
        .then_retrieve(qdrant)
        .then_answer(answers::Simple::from_client(openai));

    pipeline.query_all(ragas.questions().await).await?;

    std::fs::write("ragas.json", ragas.to_json().await)?;
    // ANCHOR_END: prepare_questions
    Ok(())
}

/// Bootstraps ground truths from a first run, then reloads the dataset for later runs.
pub async fn load_dataset(openai: OpenAI, qdrant: Qdrant) -> anyhow::Result<()> {
    // ANCHOR: load_dataset
    // Runs the pipeline once over every question in the dataset
    let evaluate = async |ragas: &Ragas| -> anyhow::Result<()> {
        query::Pipeline::default()
            .evaluate_with(ragas.clone())
            .then_transform_query(query_transformers::Embed::from_client(openai.clone()))
            .then_retrieve(qdrant.clone())
            .then_answer(answers::Simple::from_client(openai.clone()))
            .query_all(ragas.questions().await)
            .await?;
        Ok(())
    };

    // First run: questions only, then keep this run's answers as the ground truth
    let questions = vec![
        "What does a retriever return?".to_string(),
        "Which step turns retrieved documents into an answer?".to_string(),
    ];
    let ragas = Ragas::from_prepared_questions(questions);
    evaluate(&ragas).await?;
    ragas.record_answers_as_ground_truth().await;
    std::fs::write("baseline.json", ragas.to_json().await)?;

    // Later runs: load the reviewed baseline and evaluate against it
    let dataset: EvaluationDataSet = std::fs::read_to_string("baseline.json")?.parse()?;
    let ragas = Ragas::from_prepared_questions(dataset);
    evaluate(&ragas).await?;
    // ANCHOR_END: load_dataset

    std::fs::write("ragas.json", ragas.to_json().await)?;
    Ok(())
}

// ANCHOR: custom_evaluator
/// Flags questions for which the retriever found nothing.
#[derive(Clone, Default)]
pub struct EmptyRetrievals {
    questions: Arc<Mutex<Vec<String>>>,
}

#[async_trait]
impl EvaluateQuery for EmptyRetrievals {
    async fn evaluate(&self, evaluation: QueryEvaluation) -> anyhow::Result<()> {
        if let QueryEvaluation::RetrieveDocuments(query) = evaluation
            && query.documents().is_empty()
        {
            self.questions
                .lock()
                .unwrap()
                .push(query.original().to_string());
        }
        Ok(())
    }
}
// ANCHOR_END: custom_evaluator

/// Runs a pipeline with the custom evaluator and prints the questions without documents.
pub async fn custom_evaluator(openai: OpenAI, qdrant: Qdrant) -> anyhow::Result<()> {
    // ANCHOR: use_custom_evaluator
    let empty = EmptyRetrievals::default();

    query::Pipeline::default()
        .evaluate_with(empty.clone())
        .then_transform_query(query_transformers::Embed::from_client(openai.clone()))
        .then_retrieve(qdrant)
        .then_answer(answers::Simple::from_client(openai))
        .query("What does a retriever return?")
        .await?;

    for question in empty.questions.lock().unwrap().iter() {
        println!("no documents for: {question}");
    }
    // ANCHOR_END: use_custom_evaluator
    Ok(())
}
