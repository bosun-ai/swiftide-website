//! Snippets for `start/installation`.

// The first program: build a client, send one prompt.
// ANCHOR: first_run
use swiftide::{integrations::openai::OpenAI, traits::SimplePrompt};

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    // Reads OPENAI_API_KEY from the environment.
    let openai = OpenAI::builder()
        .default_prompt_model("gpt-5-mini")
        .build()?;

    let answer = openai
        .prompt("Say hello to Swiftide in five words.".into())
        .await?;
    println!("{answer}");
    Ok(())
}
// ANCHOR_END: first_run
