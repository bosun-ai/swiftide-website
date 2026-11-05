//! Snippets for `tasks/built-in-nodes`.

use std::sync::Arc;

use swiftide::{
    agents::tools::local_executor::LocalExecutor,
    chat_completion::{ChatCompletionRequest, ChatCompletionResponse, ChatMessage},
    integrations::openai::OpenAI,
    prompt::Prompt,
    tasks::{NodeError, SyncFn, Task, TaskError, TaskRunOutcome},
    traits::{ChatCompletion, Command, CommandError, CommandOutput, SimplePrompt, ToolExecutor},
};

/// A model client as a task node: prompt in, text out.
pub async fn prompt_node(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: prompt_node
    let mut task: Task<String, String> = Task::new();

    let ask = task.register_node_fn(|topic: &String| {
        Ok::<_, NodeError>(
            Prompt::from("Explain {{ topic }} in two sentences.")
                .with_context_value("topic", topic.as_str()),
        )
    });

    // The cast turns the concrete client into a node: `Prompt` in, `String` out.
    let answer = task.register_node(Arc::new(openai) as Arc<dyn SimplePrompt>);

    task.starts_with(ask);
    task.register_transition(ask, move |prompt| answer.transitions_with(prompt))?;
    task.register_transition(answer, task.transitions_to_finish())?;

    if let TaskRunOutcome::Completed(text) = task.run("a task node").await? {
        println!("{text}");
    }
    // ANCHOR_END: prompt_node
    Ok(())
}

/// A chat completion client as a task node: request in, response out.
pub async fn chat_completion_node(openai: OpenAI) -> anyhow::Result<()> {
    // ANCHOR: chat_completion_node
    let mut task: Task<String, String> = Task::new();

    let ask = task.register_node_fn(|question: &String| {
        Ok::<_, NodeError>(ChatCompletionRequest::from(vec![
            ChatMessage::new_system("Answer in one sentence."),
            ChatMessage::new_user(question.clone()),
        ]))
    });

    let chat: Arc<dyn ChatCompletion> = Arc::new(openai);
    let complete = task.register_node(chat);

    let answer = task.register_node_fn(|response: &ChatCompletionResponse| {
        Ok::<_, NodeError>(response.message().unwrap_or_default().to_string())
    });

    task.starts_with(ask);
    task.register_transition(ask, move |request| complete.transitions_with(request))?;
    task.register_transition(complete, move |response| answer.transitions_with(response))?;
    task.register_transition(answer, task.transitions_to_finish())?;

    if let TaskRunOutcome::Completed(text) = task.run("What does a task node do?").await? {
        println!("{text}");
    }
    // ANCHOR_END: chat_completion_node
    Ok(())
}

/// A tool executor as a task node: command in, command output out.
pub async fn tool_executor_node() -> anyhow::Result<()> {
    // ANCHOR: tool_executor_node
    let mut task: Task<usize, CommandOutput> = Task::new();

    let command = task.register_node_fn(|count: &usize| {
        Ok::<_, NodeError>(Command::shell(format!("git log --oneline -n {count}")))
    });

    let shell: Arc<dyn ToolExecutor> = Arc::new(LocalExecutor::new("."));
    let run = task.register_node(shell);

    task.starts_with(command);
    task.register_transition(command, move |cmd| run.transitions_with(cmd))?;
    task.register_transition(run, task.transitions_to_finish())?;

    match task.run(5_usize).await {
        Ok(TaskRunOutcome::Completed(output)) => {
            for chunk in output.stdout() {
                print!("{}", String::from_utf8_lossy(chunk));
            }
        }
        Ok(TaskRunOutcome::Paused) => unreachable!("no node in this task pauses"),
        Err(TaskError::NodeError(failed)) => {
            // An exit status, a spawn failure or an I/O error. The text can sit in either stream.
            if let Some(CommandError::NonZeroExit(output)) = failed.node_error.downcast_ref() {
                eprintln!("{}", output.to_string_lossy());
            }
            return Err(TaskError::NodeError(failed).into());
        }
        Err(other) => return Err(other.into()),
    }
    // ANCHOR_END: tool_executor_node
    Ok(())
}

/// A closure adapter built once and registered as two nodes.
pub async fn closure_adapters() -> anyhow::Result<()> {
    // ANCHOR: closure_adapters
    let mut task: Task<i32, i32> = Task::new();

    let double = SyncFn::new(|n: &i32| Ok::<_, NodeError>(n * 2));
    let first = task.register_node(double.clone());
    let second = task.register_node(double);

    task.starts_with(first);
    task.register_transition(first, move |n| second.transitions_with(n))?;
    task.register_transition(second, task.transitions_to_finish())?;

    assert_eq!(task.run(3).await?, TaskRunOutcome::Completed(12));
    // ANCHOR_END: closure_adapters
    Ok(())
}
