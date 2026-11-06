//! Snippets for `cookbook/agent-control`: a typed stop tool, reading typed stop and failure
//! payloads after a run, and registering a hand-written tool on an agent.

use std::borrow::Cow;

use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use serde_json::Value;
use swiftide::{
    agents::{Agent, StopReason, tools::control::StopWithArgs},
    chat_completion::{Tool, ToolCall, ToolOutput, ToolSpec, errors::ToolError},
    integrations::openai::OpenAI,
    traits::AgentContext,
};

// ANCHOR: stop_payload
#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum TaskStatus {
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(deny_unknown_fields)]
struct StopPayload {
    status: TaskStatus,
    summary: String,
    /// Extra context for the reader, or null when there is nothing to add.
    details: Option<String>,
}
// ANCHOR_END: stop_payload

/// Ends a run with a typed stop payload and prints it from the `on_stop` hook.
pub async fn typed_stop(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: typed_stop
    let stop_tool = StopWithArgs::with_parameters_schema(schema_for!(StopPayload));

    let mut agent = Agent::builder()
        .llm(&openai)
        .system_prompt(
            "Summarize the completed work. When done, call `stop` with the JSON schema. \
             Use null for `details` when there is nothing to add.",
        )
        .without_default_stop_tool()
        .tools([stop_tool])
        .on_stop(|_agent, reason, _| {
            Box::pin(async move {
                if let StopReason::RequestedByTool(_call, Some(payload)) = reason {
                    let report = StopPayload::deserialize(&payload)?;
                    println!("{:?}: {}", report.status, report.summary);
                    if let Some(details) = report.details {
                        println!("{details}");
                    }
                }
                Ok(())
            })
        })
        .build()?;

    agent
        .query_once("You onboarded five merchants today. Write the handoff report and stop.")
        .await?;
    // ANCHOR_END: typed_stop
    Ok(())
}

/// Reads the typed stop payload after the run instead of inside an `on_stop` hook.
pub async fn read_stop_payload(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: read_stop_payload
    #[derive(Debug, Deserialize, JsonSchema)]
    #[schemars(deny_unknown_fields)]
    struct HandoffReport {
        summary: String,
        next_actions: Vec<String>,
    }

    let mut agent = Agent::builder()
        .llm(&openai)
        .without_default_stop_tool()
        .tools([StopWithArgs::with_parameters_schema(schema_for!(
            HandoffReport
        ))])
        .build()?;

    agent
        .query_once("Write the handoff report for today's onboarding and stop.")
        .await?;

    if let Some(StopReason::RequestedByTool(_call, Some(payload))) = agent.stop_reason() {
        let report: HandoffReport = serde_json::from_value(payload.clone())?;
        println!("{}: {:?}", report.summary, report.next_actions);
    }
    // ANCHOR_END: read_stop_payload
    Ok(())
}

/// Decodes the failure payload of an agent that called `task_failed` with a custom schema.
pub fn read_failure_payload(agent: &Agent) -> anyhow::Result<()> {
    // ANCHOR: read_failure_payload
    #[derive(Debug, Deserialize)]
    struct FailureReport {
        summary: String,
        impact: String,
    }

    if let Some(StopReason::AgentFailed(Some(Value::String(json)))) = agent.stop_reason() {
        let report: FailureReport = serde_json::from_str(json)?;
        println!("failed: {} ({})", report.summary, report.impact);
    }
    // ANCHOR_END: read_failure_payload
    Ok(())
}

/// Registers a hand-written tool on an agent.
pub fn register_hand_written_tool(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: register_hand_written_tool
    #[derive(Clone)]
    struct CurrentBranch;

    #[async_trait::async_trait]
    impl Tool for CurrentBranch {
        async fn invoke(
            &self,
            _context: &dyn AgentContext,
            _tool_call: &ToolCall,
        ) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput::text("main"))
        }

        fn name(&self) -> Cow<'_, str> {
            "current_branch".into()
        }

        fn tool_spec(&self) -> ToolSpec {
            ToolSpec::builder()
                .name("current_branch")
                .description("Returns the name of the checked out git branch")
                .build()
                .expect("tool spec should be valid")
        }
    }

    let agent = Agent::builder()
        .llm(&openai)
        .add_tool(CurrentBranch)
        .build()?;

    let same_agent = Agent::builder()
        .llm(&openai)
        .tools([CurrentBranch.boxed()])
        .build()?;
    // ANCHOR_END: register_hand_written_tool
    drop((agent, same_agent));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stop_payload_schema_passes_the_strict_tool_schema_check() {
        let spec = StopWithArgs::with_parameters_schema(schema_for!(StopPayload)).tool_spec();
        assert_eq!(spec.name, "stop");
    }
}
