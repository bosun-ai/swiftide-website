//! Landing page: a typed task graph that fans out to two prompts and joins the results.
//!
//! Hand-formatted to a narrow width: the landing page shows three excerpts side by side.
#![cfg_attr(rustfmt, rustfmt_skip)]

use std::sync::Arc;

use swiftide::{
    prompt::Prompt,
    tasks::{ConcurrencyModel::Parallel, JoinInput, NodeError, Task, Transition},
    traits::SimplePrompt,
};

pub async fn review(llm: Arc<dyn SimplePrompt>, diff: String) -> anyhow::Result<()> {
    // ANCHOR: task
    let mut task = Task::<String, String>::new()
        .with_default_concurrency_model(Parallel);
    let start = task.register_node_fn(|d: &String| {
        Ok::<_, NodeError>(d.clone())
    });
    let risks = task.register_node(llm.clone());
    let summary = task.register_node(llm);
    let report = task.register_node_fn(
        |p: &JoinInput<String>| {
            let s: String = p.iter().cloned().collect();
            Ok::<_, NodeError>(s)
        },
    );
    task.starts_with(start);
    task.register_transition(start, move |d: String| {
        let ask = |q| Prompt::from(format!("{q}\n{d}"));
        Transition::fan_out(&risks, ask("List risks:"))
            .and(&summary, ask("Summarise:"))
            .join_with(report.join())
    })?;
    task.register_transition(risks, report.join())?;
    task.register_transition(summary, report.join())?;
    let done = task.transitions_to_finish();
    task.register_transition(report, done)?;

    let outcome = task.run(diff).await?;
    // ANCHOR_END: task
    println!("{outcome:?}");
    Ok(())
}
