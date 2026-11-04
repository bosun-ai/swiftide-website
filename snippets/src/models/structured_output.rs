//! Snippets for `models/structured-output`.

// ANCHOR: imports
use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use swiftide::{
    agents::{Agent, StopReason, tools::control::StopWithArgs},
    integrations::openai::OpenAI,
    traits::{DynStructuredPrompt, StructuredPrompt as _},
};
// ANCHOR_END: imports

// ANCHOR: release_summary_type
/// What a model reads out of a changelog.
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReleaseSummary {
    /// One line that names the release theme.
    pub title: String,
    /// Changes a user notices.
    pub highlights: Vec<String>,
    /// True when upgrading needs code changes.
    pub breaking: bool,
}
// ANCHOR_END: release_summary_type

/// Ask the model for a `ReleaseSummary` directly.
pub async fn structured_prompt(changelog: String) -> anyhow::Result<()> {
    // ANCHOR: structured_prompt
    let openai = OpenAI::builder()
        .default_prompt_model("gpt-5-mini")
        .build()?;

    let summary = openai
        .structured_prompt::<ReleaseSummary>(
            format!("Summarise this changelog:\n{changelog}").into(),
        )
        .await?;

    println!("{}: {} highlights", summary.title, summary.highlights.len());
    if summary.breaking {
        println!("This release needs code changes.");
    }
    // ANCHOR_END: structured_prompt
    Ok(())
}

/// The same call through a trait object.
pub async fn dyn_structured_prompt(openai: OpenAI) -> anyhow::Result<ReleaseSummary> {
    // ANCHOR: dyn_structured_prompt
    let llm: Box<dyn DynStructuredPrompt> = Box::new(openai);

    let value = llm
        .structured_prompt_dyn(
            "Summarise the 0.33 release.".into(),
            schema_for!(ReleaseSummary),
        )
        .await?;
    let summary: ReleaseSummary = serde_json::from_value(value)?;
    // ANCHOR_END: dyn_structured_prompt
    Ok(summary)
}

/// An agent that ends by calling `stop` with a `ReleaseSummary` as its arguments.
pub async fn agent_stop_schema(openai: OpenAI) -> anyhow::Result<Option<ReleaseSummary>> {
    // ANCHOR: agent_stop_schema
    let mut agent = Agent::builder()
        .llm(&openai)
        .without_default_stop_tool()
        .tools([StopWithArgs::with_parameters_schema(schema_for!(
            ReleaseSummary
        ))])
        .build()?;

    agent.query("Summarise the 0.33 release.").await?;

    let summary: Option<ReleaseSummary> = match agent.stop_reason() {
        Some(StopReason::RequestedByTool(_, Some(payload))) => {
            Some(serde_json::from_value(payload.clone())?)
        }
        _ => None,
    };
    // ANCHOR_END: agent_stop_schema
    Ok(summary)
}
