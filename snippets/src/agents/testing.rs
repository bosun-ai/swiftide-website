//! Snippets for `agents/testing`: agent tests that run without a provider.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use swiftide::{
    chat_completion::{
        ChatCompletion, ChatCompletionRequest, ChatCompletionResponse, ChatMessage, ToolCall,
        ToolOutput,
        errors::{LanguageModelError, ToolError},
    },
    traits::AgentContext,
};

// ANCHOR: count_words
/// Counts the words in a piece of text.
///
/// # Arguments
/// - `text`: The text to count the words of.
#[swiftide::tool]
pub async fn count_words(_context: &dyn AgentContext, text: &str) -> Result<ToolOutput, ToolError> {
    Ok(text.split_whitespace().count().to_string().into())
}
// ANCHOR_END: count_words

// ANCHOR: scripted_llm
/// A model that answers with scripted responses and records every request it receives.
#[derive(Clone, Default)]
pub struct ScriptedLlm {
    replies: Arc<Mutex<VecDeque<ChatCompletionResponse>>>,
    requests: Arc<Mutex<Vec<Vec<ChatMessage>>>>,
}

impl ScriptedLlm {
    pub fn new(replies: impl IntoIterator<Item = ChatCompletionResponse>) -> Self {
        Self {
            replies: Arc::new(Mutex::new(replies.into_iter().collect())),
            requests: Arc::default(),
        }
    }

    /// The messages of every request, in the order the agent sent them.
    pub fn requests(&self) -> Vec<Vec<ChatMessage>> {
        self.requests.lock().unwrap().clone()
    }
}

#[async_trait]
impl ChatCompletion for ScriptedLlm {
    async fn complete(
        &self,
        request: &ChatCompletionRequest<'_>,
    ) -> Result<ChatCompletionResponse, LanguageModelError> {
        self.requests
            .lock()
            .unwrap()
            .push(request.messages().to_vec());
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .ok_or_else(|| LanguageModelError::permanent("the script has no more replies"))
    }
}

/// A scripted reply that calls one tool with JSON arguments.
pub fn reply_calling(
    name: &str,
    args: serde_json::Value,
) -> anyhow::Result<ChatCompletionResponse> {
    let call = ToolCall::builder()
        .id(format!("call_{name}"))
        .name(name)
        .args(args.to_string())
        .build()?;
    ChatCompletionResponse::builder()
        .tool_calls(vec![call])
        .build()
}
// ANCHOR_END: scripted_llm

#[cfg(test)]
mod tests {
    use serde_json::json;
    use swiftide::{
        agents::{Agent, DefaultContext, StopReason},
        chat_completion::{
            ChatCompletionRequest, ChatCompletionResponse, ChatMessage, Tool, ToolCall, ToolOutput,
            errors::ToolError,
        },
    };
    use swiftide_core::test_utils::MockChatCompletion;

    use super::{ScriptedLlm, count_words, reply_calling};

    // ANCHOR: agent_test
    #[tokio::test]
    async fn counts_the_words_then_stops() -> anyhow::Result<()> {
        let llm = ScriptedLlm::new([
            reply_calling("count_words", json!({ "text": "one two three" }))?,
            reply_calling("stop", json!({}))?,
        ]);
        let mut agent = Agent::builder().llm(&llm).tools([count_words()]).build()?;

        agent
            .query("How many words are in `one two three`?")
            .await?;

        let history = agent.history().await?;
        let counted = history.iter().find_map(|message| match message {
            ChatMessage::ToolOutput(call, output) if call.name() == "count_words" => {
                output.as_text()
            }
            _ => None,
        });
        assert_eq!(counted, Some("3"));
        assert!(matches!(
            agent.stop_reason(),
            Some(StopReason::RequestedByTool(call, None)) if call.name() == "stop"
        ));

        let requests = llm.requests();
        assert_eq!(requests.len(), 2);
        assert!(matches!(
            requests[1].last(),
            Some(ChatMessage::ToolOutput(..))
        ));
        Ok(())
    }
    // ANCHOR_END: agent_test

    // ANCHOR: invoke_tool
    #[tokio::test]
    async fn count_words_rejects_a_misspelled_argument() -> anyhow::Result<()> {
        let tool = count_words();
        let context = DefaultContext::default();

        let call = ToolCall::builder()
            .id("call_1")
            .name("count_words")
            .args(json!({ "text": "one two" }).to_string())
            .build()?;
        let output = tool.invoke(&context, &call).await?;
        assert_eq!(output.as_text(), Some("2"));

        let typo = ToolCall::builder()
            .id("call_2")
            .name("count_words")
            .args(json!({ "txt": "one two" }).to_string())
            .build()?;
        let error = tool.invoke(&context, &typo).await.unwrap_err();
        assert!(matches!(error, ToolError::WrongArguments(_)));
        Ok(())
    }
    // ANCHOR_END: invoke_tool

    // ANCHOR: mock_chat_completion
    #[tokio::test]
    async fn sends_the_tool_output_back_to_the_model() -> anyhow::Result<()> {
        let llm = MockChatCompletion::new();
        let question = ChatMessage::new_user("How many words are in `one two`?");
        let count = ToolCall::builder()
            .id("call_1")
            .name("count_words")
            .args(json!({ "text": "one two" }).to_string())
            .build()?;
        let stop = ToolCall::builder().id("call_2").name("stop").build()?;

        llm.expect_complete(
            ChatCompletionRequest::builder()
                .messages(vec![question.clone()])
                .tools(Agent::default_tools())
                .tool(count_words())
                .build()?,
            Ok(ChatCompletionResponse::builder()
                .tool_calls(vec![count.clone()])
                .build()?),
        );
        llm.expect_complete(
            ChatCompletionRequest::builder()
                .messages(vec![
                    question,
                    ChatMessage::new_assistant(None::<String>, Some(vec![count.clone()])),
                    ChatMessage::new_tool_output(count, ToolOutput::text("2")),
                ])
                .tools(Agent::default_tools())
                .tool(count_words())
                .build()?,
            Ok(ChatCompletionResponse::builder()
                .tool_calls(vec![stop])
                .build()?),
        );

        let mut agent = Agent::builder().llm(&llm).tools([count_words()]).build()?;
        agent.query("How many words are in `one two`?").await?;

        assert!(agent.is_stopped());
        Ok(())
    }
    // ANCHOR_END: mock_chat_completion
}
