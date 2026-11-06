//! Snippets for `agents/executors`: running commands through a tool executor.

use std::{
    collections::HashMap,
    path::{Component, Path, PathBuf},
    str::FromStr,
    sync::Arc,
    time::Duration,
};

use anyhow::{bail, ensure};
use async_trait::async_trait;
use swiftide::{
    agents::{Agent, DefaultContext, tools::local_executor::LocalExecutor},
    chat_completion::{ToolOutput, errors::ToolError},
    indexing::IndexingStream,
    integrations::openai::OpenAI,
    traits::{
        AgentContext, Command, CommandError, CommandOutput, CommandOutputChunk, CommandOutputSink,
        ExecutorExt, ToolExecutor,
    },
};

/// A local executor with a working directory, a default timeout and a trimmed environment.
pub fn local_executor_env() -> anyhow::Result<LocalExecutor> {
    // ANCHOR: local_executor_env
    let executor = LocalExecutor::builder()
        .workdir("/srv/checkout")
        .default_timeout(Some(Duration::from_secs(30)))
        .env_remove(vec!["OPENAI_API_KEY".to_owned(), "GITHUB_TOKEN".to_owned()])
        .envs(HashMap::from([(
            "CARGO_TERM_COLOR".to_owned(),
            "never".to_owned(),
        )]))
        .build()?;
    // ANCHOR_END: local_executor_env
    Ok(executor)
}

/// A shell command that starts with `#!` runs as a script of that interpreter.
pub async fn shebang_script(executor: &dyn ToolExecutor) -> anyhow::Result<()> {
    // ANCHOR: shebang_script
    let script = "#!/usr/bin/env python3\nimport sys\nprint(sys.version_info.major)";
    let output = executor.exec_cmd(&Command::shell(script)).await?;
    println!("python {}", output.to_string_lossy().trim());
    // ANCHOR_END: shebang_script
    Ok(())
}

/// Borrowed and owned scopes on top of one local executor.
pub async fn scoped_executor(openai: OpenAI) -> anyhow::Result<Agent> {
    // ANCHOR: scoped_executor
    let checkout = LocalExecutor::new("/srv/checkout");

    // Borrowed: one command in /srv/checkout/docs.
    let docs = checkout.scoped("docs");
    let listing = docs.exec_cmd(&Command::shell("ls")).await?;
    println!("{listing}");

    // Owned: commands without an absolute directory or path run in /srv/checkout/crates/core.
    let executor: Arc<dyn ToolExecutor> = Arc::new(checkout.scoped_owned("crates/core"));
    let agent = Agent::builder()
        .llm(&openai)
        .context(DefaultContext::from_executor(executor))
        .build()?;
    // ANCHOR_END: scoped_executor
    Ok(agent)
}

/// Prints the output of a long command while it runs.
pub async fn stream_command_output(executor: &dyn ToolExecutor) -> anyhow::Result<()> {
    // ANCHOR: stream_command_output
    let (chunks, mut received) = tokio::sync::mpsc::unbounded_channel();
    let printer = tokio::spawn(async move {
        while let Some(chunk) = received.recv().await {
            match chunk {
                CommandOutputChunk::Stdout(bytes) => print!("{}", String::from_utf8_lossy(&bytes)),
                CommandOutputChunk::Stderr(bytes) => eprint!("{}", String::from_utf8_lossy(&bytes)),
            }
        }
    });

    let cmd = Command::shell("cargo test").with_timeout(Duration::from_secs(600));
    let result = executor
        .exec_cmd_streaming(&cmd, &mut |chunk: &CommandOutputChunk| {
            // The sink runs inside the executor's read loop: hand the chunk off, never block.
            let _ = chunks.send(chunk.clone());
        })
        .await;
    drop(chunks);
    printer.await?;

    let output = result?;
    println!("{} bytes of output", output.len());
    // ANCHOR_END: stream_command_output
    Ok(())
}

// ANCHOR: validated_path
/// A path inside the executor's working directory: relative and without `..`.
#[derive(Debug, Clone)]
pub struct WorkspacePath(PathBuf);

impl FromStr for WorkspacePath {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> anyhow::Result<Self> {
        let path = Path::new(raw.trim());
        ensure!(!path.as_os_str().is_empty(), "path is empty");
        for component in path.components() {
            match component {
                Component::Normal(_) | Component::CurDir => {}
                Component::ParentDir => bail!("`{raw}` must not contain `..`"),
                Component::RootDir | Component::Prefix(_) => {
                    bail!("`{raw}` must be relative to the workspace root")
                }
            }
        }
        Ok(Self(path.to_path_buf()))
    }
}

/// Counts the lines of a file in the workspace.
///
/// # Arguments
/// - `path`: File path relative to the workspace root.
#[swiftide::tool]
pub async fn count_lines(context: &dyn AgentContext, path: &str) -> Result<ToolOutput, ToolError> {
    let path: WorkspacePath = path.parse()?;
    let content = context
        .executor()
        .exec_cmd(&Command::read_file(&path.0))
        .await?;

    Ok(ToolOutput::text(
        content.to_string_lossy().lines().count().to_string(),
    ))
}
// ANCHOR_END: validated_path

// ANCHOR: custom_executor
/// Runs commands through another executor, but refuses to write files.
#[derive(Clone)]
pub struct ReadOnly<E> {
    inner: E,
}

impl<E> ReadOnly<E> {
    fn check(cmd: &Command) -> Result<(), CommandError> {
        if matches!(cmd, Command::WriteFile { .. }) {
            return Err(CommandError::ExecutorError(anyhow::anyhow!(
                "this executor is read-only"
            )));
        }
        Ok(())
    }
}

#[async_trait]
impl<E: ToolExecutor + Clone> ToolExecutor for ReadOnly<E> {
    async fn exec_cmd(&self, cmd: &Command) -> Result<CommandOutput, CommandError> {
        Self::check(cmd)?;
        self.inner.exec_cmd(cmd).await
    }

    async fn exec_cmd_streaming(
        &self,
        cmd: &Command,
        output: &mut CommandOutputSink<'_>,
    ) -> Result<CommandOutput, CommandError> {
        Self::check(cmd)?;
        self.inner.exec_cmd_streaming(cmd, output).await
    }

    async fn stream_files(
        &self,
        path: &Path,
        extensions: Option<Vec<String>>,
    ) -> anyhow::Result<IndexingStream<String>> {
        self.inner.stream_files(path, extensions).await
    }
}
// ANCHOR_END: custom_executor

/// An agent whose tools can read the checkout but not write to it.
pub fn read_only_agent(openai: OpenAI) -> anyhow::Result<Agent> {
    // ANCHOR: custom_executor_agent
    let executor: Arc<dyn ToolExecutor> = Arc::new(ReadOnly {
        inner: LocalExecutor::new("/srv/checkout"),
    });
    let agent = Agent::builder()
        .llm(&openai)
        .context(DefaultContext::from_executor(executor))
        .build()?;
    // ANCHOR_END: custom_executor_agent
    Ok(agent)
}
