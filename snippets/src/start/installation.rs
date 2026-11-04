//! Snippets for `start/installation`.

use swiftide::integrations::openai::OpenAI;

/// The OpenAI client most pages start from. Reads `OPENAI_API_KEY` from the environment.
pub fn openai_client() -> anyhow::Result<OpenAI> {
    // ANCHOR: openai_client
    let openai = OpenAI::builder()
        .default_prompt_model("gpt-5-mini")
        .default_embed_model("text-embedding-3-small")
        .build()?;
    // ANCHOR_END: openai_client
    Ok(openai)
}
