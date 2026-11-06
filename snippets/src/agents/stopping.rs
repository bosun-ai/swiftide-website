//! Snippets for `agents/stopping-and-structured-results`: handling every stop reason and reading
//! a typed failure report.

use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use serde_json::{Value, json};
use swiftide::{
    agents::{Agent, StopReason, tools::control::AgentCanFail},
    chat_completion::ToolOutput,
    integrations::openai::OpenAI,
};

/// Runs an agent with both limits set and reports why it stopped.
pub async fn stop_reasons(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: stop_reasons
    let mut agent = Agent::builder()
        .llm(&openai)
        .system_prompt("Check that the release branch builds. Call `stop` when you are done.")
        .tools([AgentCanFail::default()])
        .limit(20)
        .tool_retry_limit(2)
        .after_each(|agent| {
            Box::pin(async move {
                if agent.history().await?.len() > 200 {
                    agent.stop("the history grew past 200 messages").await;
                }
                Ok(())
            })
        })
        .build()?;

    let result = agent.query("Is the release branch green?").await;

    match agent.stop_reason() {
        Some(StopReason::RequestedByTool(_call, payload)) => println!("done: {payload:?}"),
        Some(StopReason::AgentFailed(reason)) => println!("the model gave up: {reason:?}"),
        Some(StopReason::ToolCallsOverLimit(call)) => println!("`{}` kept failing", call.name()),
        Some(StopReason::FeedbackRequired { tool_call, .. }) => {
            println!("`{}` waits for approval", tool_call.name());
        }
        Some(StopReason::Error) => println!("the run failed"),
        Some(StopReason::NoNewMessages) => println!("turn ended without a tool call, or limit hit"),
        Some(StopReason::Other(reason)) => println!("stopped by our hook: {reason}"),
        Some(other) => println!("stopped: {other:?}"),
        None => println!("the agent did not stop; see the returned error"),
    }

    // `ToolCallsOverLimit` and `Error` also come back as an `Err`.
    result?;
    // ANCHOR_END: stop_reasons
    Ok(())
}

/// Reads the typed failure report of an agent that called `task_failed`.
pub async fn failure_payload(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: failure_payload
    /// What the agent reports when it cannot finish.
    #[derive(Debug, Deserialize, JsonSchema)]
    #[schemars(deny_unknown_fields)]
    struct FailureReport {
        /// What went wrong, in one sentence.
        summary: String,
        /// What a person has to fix before the task can be retried.
        blocked_by: Vec<String>,
    }

    let mut agent = Agent::builder()
        .llm(&openai)
        .tools([AgentCanFail::with_parameters_schema(schema_for!(
            FailureReport
        ))])
        .build()?;

    agent.query("Restore last night's database backup.").await?;

    // With a custom schema the arguments arrive as one JSON-encoded string.
    if let Some(StopReason::AgentFailed(Some(Value::String(json)))) = agent.stop_reason() {
        let report: FailureReport = serde_json::from_str(json)?;
        println!("failed: {}", report.summary);
        for blocker in &report.blocked_by {
            println!("  blocked by: {blocker}");
        }
    }
    // ANCHOR_END: failure_payload
    Ok(())
}

/// Reads tool outputs with the accessor helpers and turns a failed deploy into a failed run.
pub fn tool_output_accessors(openai: &OpenAI) -> anyhow::Result<Agent> {
    // ANCHOR: tool_output_accessors
    let agent = Agent::builder()
        .llm(openai)
        .after_tool(|_agent, tool_call, result| {
            if let Ok(output) = result {
                if let Some(text) = output.as_text() {
                    println!("`{}` returned {} bytes", tool_call.name(), text.len());
                } else if let Some(error) = output.as_fail().map(str::to_owned) {
                    // A failed deploy ends the run instead of going back to the model.
                    if tool_call.name() == "deploy" {
                        *output = ToolOutput::agent_failed(json!({ "deploy": error }));
                    }
                } else if let Some(payload) = output.as_stop() {
                    println!("stopping with {payload}");
                } else if let Some(reason) = output.as_agent_failed() {
                    println!("giving up: {reason}");
                }
            }
            Box::pin(async { Ok(()) })
        })
        .build()?;
    // ANCHOR_END: tool_output_accessors
    Ok(agent)
}
