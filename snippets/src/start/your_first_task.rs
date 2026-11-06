//! Snippets for `start/your-first-task`: two nodes, one transition, one result.

/// The whole program: turn a list of crate names into a one-line report.
// ANCHOR: main
use swiftide::tasks::{NodeError, Task, TaskRunOutcome};

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    // A task from a manifest listing (one crate per line) to a one-line report.
    let mut task = Task::<String, String>::new();

    let discover = task.register_node_fn(|listing: &String| {
        let crates: Vec<String> = listing.lines().map(str::trim).map(String::from).collect();
        Ok::<_, NodeError>(crates)
    });
    let report = task.register_node_fn(|crates: &Vec<String>| {
        Ok::<_, NodeError>(format!("{} crates: {}", crates.len(), crates.join(", ")))
    });

    task.starts_with(discover);
    task.register_transition(discover, move |crates| report.transitions_with(crates))?;
    task.register_transition(report, task.transitions_to_finish())?;

    let listing = "swiftide-core\nswiftide-tasks\nswiftide-agents".to_string();
    match task.run(listing).await? {
        TaskRunOutcome::Completed(line) => println!("{line}"),
        TaskRunOutcome::Paused => println!("paused"),
    }
    Ok(())
}
// ANCHOR_END: main

#[cfg(test)]
mod tests {
    #[test]
    fn prints_the_report() {
        super::main().unwrap();
    }
}
