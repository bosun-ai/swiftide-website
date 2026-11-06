//! Snippets for `tasks/pause-and-resume`.

use std::{
    convert::Infallible,
    sync::{Arc, Mutex, PoisonError},
};

use swiftide::{
    reexports::async_trait::async_trait,
    tasks::{
        CurrentNodes, DynNodeId, NodeError, Task, TaskError, TaskNode, TaskRunOutcome, Transition,
    },
};

/// What the outside world has reported for a job so far.
#[derive(Debug, Clone)]
pub enum CiStatus {
    Pending,
    Finished(String),
}

// ANCHOR: external_callback
/// Waits for a CI result that a webhook delivers from outside the task.
#[derive(Debug, Clone, Default)]
pub struct AwaitCi {
    result: Arc<Mutex<Option<String>>>,
}

impl AwaitCi {
    /// Called by the webhook handler. Clones share the slot, so any clone can deliver.
    pub fn deliver(&self, result: impl Into<String>) {
        let mut slot = self.result.lock().unwrap_or_else(PoisonError::into_inner);
        *slot = Some(result.into());
    }
}

#[async_trait]
impl TaskNode for AwaitCi {
    type Input = String;
    type Output = CiStatus;
    type Error = Infallible;

    async fn evaluate(
        &self,
        _node_id: &DynNodeId<Self>,
        commit: &String,
    ) -> Result<CiStatus, Infallible> {
        let slot = self.result.lock().unwrap_or_else(PoisonError::into_inner);
        Ok(match slot.as_deref() {
            Some(result) => CiStatus::Finished(format!("{commit}: {result}")),
            None => CiStatus::Pending,
        })
    }
}

pub async fn wait_for_ci() -> anyhow::Result<String> {
    let ci = AwaitCi::default();

    let mut task = Task::<String, String>::new();
    let push = task
        .register_node_fn(|commit: &String| -> Result<String, NodeError> { Ok(commit.clone()) });
    let wait = task.register_node(ci.clone());

    task.starts_with(push);
    task.register_transition(push, move |commit| wait.transitions_with(commit))?;
    task.register_transition(wait, {
        let finish = task.transitions_to_finish();
        move |status| match status {
            CiStatus::Finished(report) => finish(report),
            CiStatus::Pending => Transition::pause(),
        }
    })?;

    assert_eq!(task.run("4f2a91c").await?, TaskRunOutcome::Paused);
    // One task value holds one run: a second `run` while paused is refused.
    assert!(matches!(
        task.run("77b0e3d").await,
        Err(TaskError::TaskActive)
    ));

    // Later, the webhook handler that holds `ci` receives the result,
    // and the same task value continues.
    ci.deliver("all checks passed");
    let TaskRunOutcome::Completed(report) = task.resume().await? else {
        anyhow::bail!("the CI result was delivered, so the task should finish");
    };
    // ANCHOR_END: external_callback
    Ok(report)
}

// ANCHOR: current_nodes
/// Delivers a CI result through the task, for callers that hold the task instead of the node.
pub fn deliver_through_task(task: &Task<String, String>, result: &str) -> anyhow::Result<()> {
    let CurrentNodes::One(ci) = task.current_nodes::<AwaitCi>() else {
        anyhow::bail!("the task is not waiting on exactly one CI result");
    };
    ci.deliver(result);
    Ok(())
}
// ANCHOR_END: current_nodes

pub async fn reset_and_rerun() -> anyhow::Result<()> {
    // ANCHOR: reset
    let mut task = Task::<i32, i32>::new();
    let double = task.register_node_fn(|n: &i32| -> Result<i32, NodeError> { Ok(n * 2) });
    task.starts_with(double);
    task.register_transition(double, task.transitions_to_finish())?;

    assert_eq!(task.run(21).await?, TaskRunOutcome::Completed(42));
    // A completed run leaves nothing to continue.
    assert!(matches!(task.resume().await, Err(TaskError::NotResumable)));

    // `reset` queues a new run at the start node with the last input.
    task.reset();
    // That queued run counts as active work, so `run` with another input is refused.
    assert!(matches!(task.run(5).await, Err(TaskError::TaskActive)));
    assert_eq!(task.resume().await?, TaskRunOutcome::Completed(42));

    // A clone has the same graph and a fresh runtime of its own.
    let mut copy = task.clone();
    assert_eq!(copy.run(5).await?, TaskRunOutcome::Completed(10));
    // ANCHOR_END: reset
    Ok(())
}
