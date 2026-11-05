//! Snippets for `agents/system-prompts`: run-time template values, named templates and
//! resuming without a second system message.

use swiftide::{
    agents::{Agent, DefaultContext},
    integrations::openai::OpenAI,
    prompt::Prompt,
    traits::{Command, ToolExecutor},
};

/// Fills a template value from the tool executor in a `before_all` hook, right before the render.
pub async fn before_all_fill(openai: &OpenAI) -> anyhow::Result<()> {
    // ANCHOR: before_all_fill
    let mut agent = Agent::builder()
        .llm(openai)
        .system_prompt("You review the changes on the branch `{{ branch }}`.")
        .before_all(|agent: &mut Agent| {
            Box::pin(async move {
                let branch = agent
                    .context()
                    .executor()
                    .exec_cmd(&Command::shell("git branch --show-current"))
                    .await?
                    .into_string_lossy();

                if let Some(prompt) = agent.system_prompt_mut() {
                    prompt.insert_context_value("branch", branch.trim());
                }
                Ok(())
            })
        })
        .build()?;
    // ANCHOR_END: before_all_fill
    agent.query("Review the branch").await?;
    Ok(())
}

/// Resumes a conversation whose stored history already holds the system message.
pub async fn resume_without_prompt(openai: &OpenAI) -> anyhow::Result<()> {
    // ANCHOR: resume
    let system_prompt = "You answer questions about the swiftide repository.";

    let mut first_agent = Agent::builder()
        .llm(openai)
        .system_prompt(system_prompt)
        .build()?;
    first_agent.query("Which crates does it have?").await?;
    // The stored history starts with the rendered system message.
    let history = first_agent.history().await?;

    let mut second_agent = Agent::builder()
        .llm(openai)
        .context(
            DefaultContext::default()
                .with_existing_messages(history)
                .await?
                .to_owned(),
        )
        .system_prompt(system_prompt)
        // Removes the prompt set above, so the history keeps a single system message.
        .no_system_prompt()
        .build()?;
    second_agent.query("Which one holds the agents?").await?;
    // ANCHOR_END: resume
    Ok(())
}

/// Registers a named template once, then builds an agent whose system prompt renders it.
pub fn named_template(openai: &OpenAI) -> anyhow::Result<Agent> {
    // ANCHOR: named_template
    let mut templates = tera::Tera::default();
    templates.add_raw_templates([
        (
            "release_rules",
            "Name the crate and the feature flag in every entry.",
        ),
        (
            "release_manager",
            "You write the release notes for {{ project }}.\n\n{% include \"release_rules\" %}",
        ),
    ])?;
    Prompt::extend(&templates)?;

    let agent = Agent::builder()
        .llm(openai)
        .system_prompt(
            Prompt::from_compiled_template("release_manager")
                .with_context_value("project", "swiftide"),
        )
        .build()?;
    // ANCHOR_END: named_template
    Ok(agent)
}

#[cfg(test)]
mod tests {
    use swiftide::prompt::Prompt;

    #[test]
    fn named_template_renders_with_partial() {
        let mut templates = tera::Tera::default();
        templates
            .add_raw_templates([
                ("rules_test", "Be brief."),
                (
                    "manager_test",
                    "Notes for {{ project }}. {% include \"rules_test\" %}",
                ),
            ])
            .unwrap();
        Prompt::extend(&templates).unwrap();
        let rendered = Prompt::from_compiled_template("manager_test")
            .with_context_value("project", "swiftide")
            .render()
            .unwrap();
        assert_eq!(rendered, "Notes for swiftide. Be brief.");
    }

    #[test]
    fn guarded_undefined_values_render_and_unguarded_fail() {
        let guarded = Prompt::from("{% if version %}v{{ version }}{% else %}none{% endif %}")
            .with_context_value("since", "v1");
        assert_eq!(guarded.render().unwrap(), "none");

        let unguarded = Prompt::from("v{{ version }}").with_context_value("since", "v1");
        assert!(unguarded.render().is_err());

        let verbatim = Prompt::from("v{{ version }}");
        assert_eq!(verbatim.render().unwrap(), "v{{ version }}");
    }
}
