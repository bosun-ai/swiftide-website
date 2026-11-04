//! Snippets for `reference/crates`.

// ANCHOR: facade_paths
use swiftide::{
    agents::Agent,                         // swiftide-agents
    indexing::{self, loaders::FileLoader}, // swiftide-indexing
    integrations::openai::OpenAI,          // swiftide-integrations
    prompt::Prompt,                        // swiftide-core
    query::{self, answers},                // swiftide-query
    tasks::{NodeError, Task},              // swiftide-tasks
    traits::{AgentContext, ToolExecutor},  // swiftide-core
};
// ANCHOR_END: facade_paths

/// Touches every import above, so the excerpt stays honest under `warnings = "deny"`.
pub fn facade_paths(openai: OpenAI) -> anyhow::Result<()> {
    let _agent = Agent::builder().llm(&openai).build()?;

    let mut task = Task::<i32, i32>::new();
    let double = task.register_node_fn(|n: &i32| Ok::<_, NodeError>(n * 2));
    task.starts_with(double);

    let _indexing = indexing::Pipeline::from_loader(FileLoader::new("docs"));
    let _query = query::Pipeline::default();
    let _answers = answers::Simple::from_client(openai);
    let _prompt = Prompt::from("Summarize {{ title }}.");
    Ok(())
}

/// The context and executor traits come from `swiftide-core`, through `swiftide::traits`.
pub fn executor_of(context: &dyn AgentContext) -> &dyn ToolExecutor {
    context.executor().as_ref()
}
