//! Snippets for `cookbook/structured-prompts`: the imports and client a binary needs
//! before it calls `structured_prompt`.

// ANCHOR: imports
use anyhow::Result;
use schemars::JsonSchema;
use serde::Deserialize;
use swiftide::{integrations, traits::StructuredPrompt as _};

#[tokio::main]
async fn main() -> Result<()> {
    let client = integrations::openai::OpenAI::builder()
        .default_prompt_model("gpt-5-mini")
        .build()?;
    // ANCHOR_END: imports

    #[derive(Debug, Deserialize, JsonSchema)]
    #[serde(deny_unknown_fields)]
    struct MyResponse {
        questions: Vec<String>,
    }

    let response = client
        .structured_prompt::<MyResponse>(
            "List three interesting questions about the Rust programming language.".into(),
        )
        .await?;
    println!("{:?}", response.questions);

    Ok(())
}

/// Keeps `main` reachable from the library so it is type checked without a dead code warning.
pub fn run() -> Result<()> {
    main()
}
