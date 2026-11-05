//! Snippets for `tasks/nodes-and-transitions`.

// ANCHOR: imports
use std::num::ParseIntError;

use swiftide::{
    reexports::async_trait::async_trait,
    tasks::{DynNodeId, NodeError, Task, TaskError, TaskNode, TaskRunOutcome, Transition},
};
// ANCHOR_END: imports

// ANCHOR: custom_node
/// Counts the words in a text.
#[derive(Clone, Debug)]
pub struct CountWords;

#[async_trait]
impl TaskNode for CountWords {
    type Input = String;
    type Output = usize;
    type Error = std::convert::Infallible;

    async fn evaluate(
        &self,
        _node_id: &DynNodeId<Self>,
        text: &Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        Ok(text.split_whitespace().count())
    }
}
// ANCHOR_END: custom_node

/// A linear task: `String` in, `String` out, with a `usize` in between.
pub async fn linear_task() -> anyhow::Result<()> {
    // ANCHOR: linear
    let mut task: Task<String, String> = Task::new();

    // A sync closure node. `Ok::<_, NodeError>` names the error type.
    let trim = task.register_node_fn(|text: &String| Ok::<_, NodeError>(text.trim().to_string()));

    // A node type that implements `TaskNode`.
    let count = task.register_node(CountWords);

    // A closure that returns a pinned, boxed future. Copy what it needs before `async move`.
    let label = task.register_node_async_fn(|words: &usize| {
        let words = *words;
        Box::pin(async move { Ok::<_, NodeError>(format!("{words} words")) })
    });

    task.starts_with(trim);
    task.register_transition(trim, move |text: String| {
        if text.is_empty() {
            return Transition::error("there is no text to count");
        }
        count.transitions_with(text).into()
    })?;
    task.register_transition(count, move |words| label.transitions_with(words))?;
    task.register_transition(label, task.transitions_to_finish())?;

    match task.run("  nodes and transitions  ").await? {
        TaskRunOutcome::Completed(summary) => println!("{summary}"),
        TaskRunOutcome::Paused => println!("paused; call task.resume() to continue"),
    }
    // ANCHOR_END: linear

    Ok(())
}

/// Telling a failing node apart from a wiring mistake.
pub async fn report_errors() -> anyhow::Result<()> {
    // ANCHOR: errors
    let mut task: Task<String, u32> = Task::new();

    let parse = task.register_node_fn(|text: &String| text.trim().parse::<u32>());
    task.starts_with(parse);
    task.register_transition(parse, task.transitions_to_finish())?;

    match task.run("forty-two").await {
        Ok(TaskRunOutcome::Completed(number)) => println!("parsed {number}"),
        Ok(TaskRunOutcome::Paused) => println!("paused"),
        Err(TaskError::NodeError(failed)) => {
            // `node_error` is the error the node returned, boxed.
            if let Some(error) = failed.node_error.downcast_ref::<ParseIntError>() {
                eprintln!("node {} could not parse the input: {error}", failed.node_id);
            }
        }
        Err(wiring @ (TaskError::NoSteps | TaskError::MissingTransition(_))) => {
            // `run` checks the graph before the first node runs.
            panic!("the task is wired wrong: {wiring}");
        }
        Err(other) => return Err(other.into()),
    }
    // ANCHOR_END: errors

    Ok(())
}
