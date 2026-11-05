//! Snippets for `start/installation`.

// The first program: install a crypto provider, build a client, send one prompt.
// ANCHOR: first_run
use swiftide::{integrations::openai::OpenAI, traits::SimplePrompt};

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    // Swiftide's HTTP clients use rustls without a crypto provider; pick one before any request.
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("a rustls crypto provider was already installed"))?;

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
