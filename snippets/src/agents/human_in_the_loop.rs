//! Snippets for `agents/human-in-the-loop`: a tool that asks a person for feedback itself.

use std::borrow::Cow;

use schemars::{JsonSchema, schema_for};
use serde::Deserialize;
use serde_json::json;
use swiftide::{
    agents::{Agent, StopReason},
    chat_completion::{Tool, ToolCall, ToolOutput, ToolSpec, errors::ToolError},
    traits::{AgentContext, ToolFeedback},
};

/// Stands in for the payment provider.
async fn send_refund(_order_id: &str, _amount_cents: u64) -> anyhow::Result<()> {
    Ok(())
}

// ANCHOR: custom_feedback_tool
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct RefundArgs {
    /// The order to refund.
    order_id: String,
    /// The amount to refund, in cents.
    amount_cents: u64,
}

#[derive(Clone)]
struct IssueRefund;

#[swiftide::reexports::async_trait::async_trait]
impl Tool for IssueRefund {
    async fn invoke(
        &self,
        context: &dyn AgentContext,
        tool_call: &ToolCall,
    ) -> Result<ToolOutput, ToolError> {
        let args = tool_call
            .args()
            .ok_or_else(|| ToolError::missing_arguments("order_id, amount_cents"))?;
        let refund: RefundArgs = serde_json::from_str(args)?;

        match context.has_received_feedback(tool_call).await {
            // No decision yet: stop the agent and show the reviewer what it asks for.
            None => Ok(ToolOutput::feedback_required(Some(json!({
                "order_id": refund.order_id,
                "amount_cents": refund.amount_cents,
            })))),
            // The reviewer can lower the amount in the approval payload.
            Some(ToolFeedback::Approved { payload }) => {
                let amount_cents = payload
                    .as_ref()
                    .and_then(|payload| payload["amount_cents"].as_u64())
                    .unwrap_or(refund.amount_cents);
                send_refund(&refund.order_id, amount_cents).await?;
                Ok(ToolOutput::text(format!(
                    "Refunded {amount_cents} cents for order {}",
                    refund.order_id
                )))
            }
            Some(ToolFeedback::Refused { payload }) => {
                let reason = payload
                    .as_ref()
                    .and_then(|payload| payload["reason"].as_str())
                    .unwrap_or("no reason given");
                Ok(ToolOutput::text(format!(
                    "A person refused the refund: {reason}"
                )))
            }
        }
    }

    fn name(&self) -> Cow<'_, str> {
        Cow::Borrowed("issue_refund")
    }

    fn tool_spec(&self) -> ToolSpec {
        ToolSpec::builder()
            .name("issue_refund")
            .description("Refund (part of) an order. A person approves every refund.")
            .parameters_schema(schema_for!(RefundArgs))
            .build()
            .expect("the refund arguments form a strict schema")
    }
}
// ANCHOR_END: custom_feedback_tool

/// Approves a pending refund at a lower amount and lets the agent continue.
pub async fn approve_with_payload(agent: &mut Agent) -> anyhow::Result<()> {
    // ANCHOR: approve_with_payload
    if let Some(StopReason::FeedbackRequired { tool_call, payload }) = agent.stop_reason() {
        println!("approve {payload:?}?");
        let tool_call = tool_call.clone();
        let feedback = ToolFeedback::approved().with_payload(json!({ "amount_cents": 2_500 }));

        agent
            .context()
            .feedback_received(&tool_call, &feedback)
            .await?;
        agent.run().await?;
    }
    // ANCHOR_END: approve_with_payload
    Ok(())
}

/// Builds an agent with the refund tool, so the excerpt above is used.
pub fn refund_agent(llm: &swiftide::integrations::openai::OpenAI) -> anyhow::Result<Agent> {
    Ok(Agent::builder().llm(llm).add_tool(IssueRefund).build()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refund_spec_is_strict() {
        assert_eq!(IssueRefund.tool_spec().name, "issue_refund");
    }
}
