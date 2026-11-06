//! Snippets for `tasks/fan-out-and-join`.

use swiftide::tasks::{AnyJoinInput, JoinInput, NodeError, Task, TaskRunOutcome, Transition};

/// Three branches with different outputs, mapped into one `JoinInput<String>`.
pub async fn map_join() -> anyhow::Result<String> {
    // ANCHOR: map_join
    let mut task = Task::<String, String>::new();

    let start = task.register_node_fn(|text: &String| -> Result<String, NodeError> {
        Ok(text.trim().to_string())
    });
    let count = task.register_node_fn(|text: &String| -> Result<usize, NodeError> {
        Ok(text.split_whitespace().count())
    });
    let length = task
        .register_node_fn(|text: &String| -> Result<usize, NodeError> { Ok(text.chars().count()) });
    let shout = task
        .register_node_fn(|text: &String| -> Result<String, NodeError> { Ok(text.to_uppercase()) });
    let join = task.register_node_fn(|parts: &JoinInput<String>| -> Result<String, NodeError> {
        Ok(parts.iter().cloned().collect::<Vec<_>>().join("; "))
    });

    task.starts_with(start);
    task.register_transition(start, move |text: String| {
        Transition::fan_out(&count, text.clone())
            .and(&length, text.clone())
            .and(&shout, text)
            .join_with(join.join())
    })?;

    // `count` returns a usize; `map` turns it into the join's String payload.
    task.register_transition(
        count,
        join.join().map(|words: usize| format!("{words} words")),
    )?;
    // An async mapping is registered with `register_transition_async`.
    task.register_transition_async(
        length,
        join.join()
            .map_async(|chars: usize| async move { format!("{chars} characters") }),
    )?;
    // `shout` already returns a String, so it joins as is.
    task.register_transition(shout, join.join())?;
    task.register_transition(join, task.transitions_to_finish())?;

    // "2 words; 11 characters; HELLO TASKS"
    let TaskRunOutcome::Completed(summary) = task.run(" hello tasks ").await? else {
        anyhow::bail!("nothing in this task pauses");
    };
    // ANCHOR_END: map_join
    Ok(summary)
}

/// Branches with different payload types, collected by an `AnyJoinInput` join.
pub async fn join_any() -> anyhow::Result<String> {
    // ANCHOR: join_any
    let mut task = Task::<String, String>::new();

    let start = task.register_node_fn(|text: &String| -> Result<String, NodeError> {
        Ok(text.trim().to_string())
    });
    let count = task.register_node_fn(|text: &String| -> Result<usize, NodeError> {
        Ok(text.split_whitespace().count())
    });
    let shout = task
        .register_node_fn(|text: &String| -> Result<String, NodeError> { Ok(text.to_uppercase()) });
    let join = task.register_node_fn(|parts: &AnyJoinInput| -> Result<String, NodeError> {
        let words: usize = parts.iter::<usize>().sum();
        let shouted = parts.iter::<String>().cloned().collect::<Vec<_>>();
        Ok(format!("{} ({words} words)", shouted.join(" ")))
    });

    task.starts_with(start);
    task.register_transition(start, move |text: String| {
        Transition::fan_out(&count, text.clone())
            .and(&shout, text)
            .join_with(join.join_any())
    })?;
    task.register_transition(count, join.join_any())?;
    task.register_transition(shout, join.join_any())?;
    task.register_transition(join, task.transitions_to_finish())?;

    // "HELLO TASKS (2 words)"
    let TaskRunOutcome::Completed(summary) = task.run(" hello tasks ").await? else {
        anyhow::bail!("nothing in this task pauses");
    };
    // ANCHOR_END: join_any
    Ok(summary)
}
