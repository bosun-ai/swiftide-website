---
title: "Tool executors"
description: "Run agent tool commands through a ToolExecutor: a LocalExecutor with a working directory, timeout and environment, scoped executors, streamed output, and model arguments validated before they reach a shell."
sidebar:
  order: 4
identLint:
  ignore: [WorkspacePath, count_lines, ReadOnly]
---

Your tools run shell commands and file operations through a tool executor instead of spawning processes themselves. You end up with one that works in a chosen directory, stops slow commands, streams their output, and only receives arguments your code has checked.

The local executor needs the `swiftide-agents` feature flag:

```sh
cargo add swiftide@{{swiftide_version}} --features swiftide-agents
```

## Local executor

::snippet{src="@snippets/src/agents/executors.rs" anchor="local_executor_env"}

The builder sets where and how commands run:

- `workdir` sets the directory commands start in. It defaults to `.`. `LocalExecutor::new` only sets this field.
- `default_timeout` takes an `Option<Duration>` for commands without a timeout of their own.
- `env_remove` drops variables, here secrets that the model's commands do not need.
- `envs` sets variables. They apply after `env_clear` and `env_remove`.
- `env_clear(true)` (not shown) starts every command with an empty environment. That also drops `PATH` and `HOME`, so pass the ones your commands need back through `envs`.

The agent gets the executor through its context. [Context and message history](/agents/context-and-history/) shows how `DefaultContext::from_executor` swaps it in.

## Tool executor trait

`ToolExecutor` has three methods:

- `exec_cmd` runs a `Command` and returns its `CommandOutput` or a `CommandError`.
- `exec_cmd_streaming` also hands each piece of output to a closure. The default implementation reports all output once the command has finished.
- `stream_files` returns the files below a path as an `IndexingStream` for an indexing pipeline; see [Loading](/indexing/loading/).

A tool reaches the executor with `context.executor()`. `AgentContext::exec_cmd` is deprecated.

### Custom executor

You write your own tool executor to run commands somewhere else, such as a remote machine, or to put a policy in front of another executor. This one wraps any executor and refuses file writes:

::snippet{src="@snippets/src/agents/executors.rs" anchor="custom_executor"}

The trait uses `#[async_trait]`, so add the `async-trait` crate. The trait extends `DynClone`, so your type must implement `Clone`. `exec_cmd` and `stream_files` are required. `ReadOnly` forwards `exec_cmd_streaming` as well; without it, the default would run `exec_cmd` and report the output only at the end.

`Command` is non-exhaustive, so match it with a wildcard arm or `matches!`. Return `CommandError::ExecutorError` when the executor refuses or fails, and `NonZeroExit` when the command ran and failed. An executor that cannot stream can return `IndexingStream::from(anyhow::Error)`, so a loader reports the error instead of panicking.

Give the agent your executor the same way as a local one:

::snippet{src="@snippets/src/agents/executors.rs" anchor="custom_executor_agent"}

`ReadOnly` only blocks `Command::WriteFile`. A shell string such as `echo hi > notes.md` still writes, so a real policy also has to check or forbid `Command::Shell`.

## Output streaming

::snippet{src="@snippets/src/agents/executors.rs" anchor="stream_command_output"}

`exec_cmd_streaming` passes each chunk to a sink while the command runs. The sink is a `CommandOutputSink`: a `FnMut(&CommandOutputChunk) + Send` closure. The executor calls the sink from its read loop, so hand each chunk off instead of doing work in it. The local executor streams shell commands and reports file commands once they finish. The return value matches `exec_cmd`.

## Commands

You build a `Command` with `Command::shell`, `Command::read_file` or `Command::write_file`, as in the streaming example above and the tool further down. `Command::shell` runs a string with `sh -c`. The file commands skip the shell, so a path never needs quoting.

Two methods set options on one command:

- `with_current_dir` sets the working directory of the command.
- `with_timeout` sets the timeout of the command. It wins over `default_timeout`, also when it is longer.

How a relative directory or path resolves is up to the executor. The local executor joins a relative `current_dir` onto its `workdir` and uses an absolute one as it is. File paths resolve against the result. Its `write_file` creates missing parent directories and replaces the file's content.

## Command output

::snippet{src="@snippets/src/reference/upgrading.rs" anchor="command_output"}

`CommandOutput` keeps stdout and stderr apart. It holds `CommandOutputChunk::Stdout` and `CommandOutputChunk::Stderr` chunks in the order the executor read them. `stdout()` and `stderr()` iterate over one stream; `to_string_lossy()` and `Display` join both. A `CommandOutput` converts into a `ToolOutput` with the joined text.

A failed command returns an error that still carries its output:

- `CommandError::NonZeroExit` holds the output of a command that exited with a non-zero code. An input/output (I/O) error, such as reading a missing file, also becomes `NonZeroExit`, with the error message as output.
- `CommandError::TimedOut` holds the timeout and the output read so far. For a shell command, the local executor kills the process (its process group on Unix). A file command that times out returns empty output.
- `CommandError::ExecutorError` means the executor itself failed.

A tool that returns such an error with `?` sends its text to the model and counts toward the tool retry limit. See [Tool errors and retries](/agents/tools/#tool-errors-and-retries). Match `NonZeroExit` yourself when failing is a normal answer, like a failing test.

## Shebang scripts

::snippet{src="@snippets/src/agents/executors.rs" anchor="shebang_script"}

When a shell command starts with `#!`, the local executor starts the interpreter named on that line and writes the rest of the string to its stdin. Everything after the interpreter on the first line is passed as one argument, so `#!/usr/bin/env python3` runs `/usr/bin/env` with `python3`. A model can send a multi-line script without escaping it for `sh -c`.

## Scoped executors

::snippet{src="@snippets/src/agents/executors.rs" anchor="scoped_executor"}

`ExecutorExt` adds `scoped`, which borrows the executor, and `scoped_owned`, which takes it so it can live as long as an agent. Both return a `ScopedExecutor`:

- A command without a working directory runs in the scope.
- A relative `current_dir` joins onto the scope. The local executor then joins the result onto its `workdir`.
- An absolute `current_dir` is left alone, and so is an absolute file path. A scope does not keep a tool inside it.
- `stream_files` joins a relative path onto the scope. The local executor does not add its `workdir` to that path; see the gotchas below.

## Argument validation

A working directory is not a sandbox: it only chooses where a command starts. An absolute path, a `..` or a `;` in a shell string still reaches the rest of the machine. Parse model arguments into a type that cannot hold a bad value first. The release-notes-agent showcase validates every argument with `Revision` and `RepoPath` in [`src/git.rs`](https://github.com/bosun-ai/swiftide/blob/e09a3e90d3eecdaffa08818a661efc3ecec08321/examples/release-notes-agent/src/git.rs). The same pattern for your own tool:

::snippet{src="@snippets/src/agents/executors.rs" anchor="validated_path"}

Prefer `Command::read_file` and `Command::write_file` over `cat` or `>` in a shell string: no shell ever sees the path.

## Docker executor

For isolation, the [Docker executor](https://github.com/bosun-ai/swiftide-docker-executor) runs the commands in a container. It is a separate crate; pass it to `DefaultContext::from_executor`, and check that its swiftide version matches yours.

## Gotchas

- `DefaultContext::from_executor` takes `Into<Arc<dyn ToolExecutor>>`. A bare `LocalExecutor` does not convert, so wrap it in an `Arc` annotated as `Arc<dyn ToolExecutor>`, as the scoped example does.
- `LocalExecutor::stream_files` uses the path as given, not relative to `workdir`. A relative path resolves against your process's current directory, also through a scope. Scope it with an absolute path or pass an absolute path.
- Symlinks inside the working directory can still point elsewhere. Validated paths do not change that; a container does.

## Reference

- [`ToolExecutor`](docsrs:swiftide/traits/trait.ToolExecutor.html)
- [`IndexingStream`](docsrs:swiftide/indexing/struct.IndexingStream.html)
- [`Command`](docsrs:swiftide/traits/enum.Command.html)
- [`CommandOutput`](docsrs:swiftide/traits/struct.CommandOutput.html)
- [`CommandOutputChunk`](docsrs:swiftide/traits/enum.CommandOutputChunk.html)
- [`CommandOutputSink`](docsrs:swiftide/traits/type.CommandOutputSink.html)
- [`CommandError`](docsrs:swiftide/traits/enum.CommandError.html)
- [`ExecutorExt`](docsrs:swiftide/traits/trait.ExecutorExt.html)
- [`ScopedExecutor`](docsrs:swiftide/traits/struct.ScopedExecutor.html)
- [`LocalExecutor`](docsrs:swiftide/agents/tools/local_executor/struct.LocalExecutor.html)
- [`DefaultContext::from_executor`](docsrs:swiftide/agents/struct.DefaultContext.html#method.from_executor)
- [`AgentContext::executor`](docsrs:swiftide/traits/trait.AgentContext.html#tymethod.executor)

## Next

[Lifecycle hooks](/agents/hooks/)
