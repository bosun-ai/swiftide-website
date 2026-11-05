//! Snippets for `cookbook/responses-api-and-reasoning`: the imports and the two argument types
//! that `examples/responses_api.rs` keeps outside its anchors, so readers can rebuild the example.
#![allow(unused_imports, dead_code)]

// ANCHOR: types
use anyhow::{Context, Result};
use futures_util::StreamExt as _;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use swiftide::{
    chat_completion::{ChatCompletionRequest, ChatMessage, ToolOutput, errors::ToolError},
    integrations::openai::{OpenAI, Options},
    traits::{AgentContext, ChatCompletion, SimplePrompt, StructuredPrompt},
};

/// The type `structured_prompt` deserializes the model's answer into.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct WeatherSummary {
    description: String,
}

/// The single parameter of the echo tool.
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct EchoArgs {
    message: String,
}
// ANCHOR_END: types
