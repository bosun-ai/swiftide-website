//! Snippets for `models/prompts-and-templates`.

use swiftide::{
    agents::Agent, indexing::transformers::MetadataQAText, integrations::openai::OpenAI,
    prompt::Prompt, query::answers, traits::SimplePrompt,
};

/// A one-off template filled with two values, rendered and then sent.
pub async fn prompt_with_context(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: prompt_with_context
    let prompt = Prompt::from("Explain {{ topic }} to a {{ audience }} in two sentences.")
        .with_context_value("topic", "ownership")
        .with_context_value("audience", "Python developer");

    let rendered = prompt.render()?;
    let answer = openai.prompt(prompt).await?;
    // ANCHOR_END: prompt_with_context
    println!("{rendered}\n{answer}");
    Ok(())
}

/// A Tera for-loop over a list value.
pub fn prompt_with_loop() -> anyhow::Result<()> {
    // ANCHOR: prompt_with_loop
    let prompt =
        Prompt::from("Review these files:\n{% for file in files %}- {{ file }}\n{% endfor %}")
            .with_context_value("files", vec!["src/lib.rs", "src/main.rs"]);
    // ANCHOR_END: prompt_with_loop
    prompt.render()?;
    Ok(())
}

/// Registers a named template once and loads it by name.
pub fn named_template() -> anyhow::Result<()> {
    // ANCHOR: named_template
    let mut templates = tera::Tera::default();
    templates.add_raw_template(
        "review",
        "Review this {{ language }} diff and list risks:\n{{ diff }}",
    )?;
    Prompt::extend(&templates)?;

    let prompt = Prompt::from_compiled_template("review")
        .with_context_value("language", "Rust")
        .with_context_value("diff", "- let x = 1;\n+ let x = 2;");
    // ANCHOR_END: named_template
    prompt.render()?;
    Ok(())
}

/// Replaces the built-in prompt of an indexing transformer.
pub fn transformer_prompt(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: transformer_prompt
    let qa = MetadataQAText::builder()
        .client(openai)
        .num_questions(3_usize)
        .prompt_template(
            "Write {{ questions }} questions a support engineer would ask about:\n\
             {{ node.chunk }}",
        )
        .build()?;
    // ANCHOR_END: transformer_prompt
    let _ = qa;
    Ok(())
}

/// Replaces the built-in prompt of the simple answer generator.
pub fn answer_prompt(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: answer_prompt
    let answer = answers::Simple::builder()
        .client(openai)
        .prompt_template(Prompt::from(
            "Answer {{ question }} using only:\n{{ documents }}",
        ))
        .build()?;
    // ANCHOR_END: answer_prompt
    let _ = answer;
    Ok(())
}

/// An agent system prompt is a `Prompt` since 0.33, so it takes template values too.
pub fn agent_system_prompt(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: agent_system_prompt
    let agent = Agent::builder()
        .llm(&openai)
        .system_prompt(
            Prompt::from("You review {{ language }} code. Be brief.")
                .with_context_value("language", "Rust"),
        )
        .build()?;
    // ANCHOR_END: agent_system_prompt
    let _ = agent;
    Ok(())
}
