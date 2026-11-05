---
title: "MCP tools"
description: "Start a Model Context Protocol (MCP) server as a child process, wrap it in an McpToolbox with a whitelist, and add its tools to a Swiftide agent."
sidebar:
  order: 10
identLint:
  ignore: [TokioChildProcess, Stdio, "Stdio::null()", get_pull_request, get_issue, ClientInfo, "ClientInfo::serve", ServiceExt, RunningService]
---

Here you give an agent a few tools from a Model Context Protocol (MCP) server that runs as a child process and hide the rest from the model. Swiftide speaks MCP through the [`rmcp`](https://docs.rs/rmcp/2.1) crate. Turn on the `mcp` feature flag, which also enables `swiftide-agents`, and add `rmcp` with a client and the transport you use:

```sh
cargo add swiftide@{{swiftide_version}} --features openai,mcp
cargo add rmcp@2.1 --features client,transport-child-process
cargo add tokio --features macros,rt-multi-thread,process,time
```

## Child process toolbox

The release-notes-agent showcase reads pull requests through the GitHub MCP server when you pass `--mcp`:

::snippet{src="@swiftide/examples/release-notes-agent/src/mcp.rs" anchor="mcp" title="examples/release-notes-agent/src/mcp.rs"}

`McpToolbox::try_from_transport` takes any `rmcp` client transport, such as a child process or a Transmission Control Protocol (TCP) stream. It performs the MCP handshake with a default client info named `swiftide` and returns the toolbox. The handshake waits for `npx` to download the package on a first run, so the showcase puts a one-minute timeout around it.

Calling `available_tools` up front turns a missing tool into a startup error instead of a failure in the middle of a run. `src/agent.rs` adds the toolbox with `builder.add_toolbox(github_tools)`.

Run it from the swiftide repository. You need `OPENAI_API_KEY`, Node.js, a token in `GITHUB_PERSONAL_ACCESS_TOKEN` or `GITHUB_TOKEN`, and an `origin` remote on GitHub. Without the remote, the showcase skips the GitHub tools:

```sh
cargo run -p release-notes-agent -- --mcp
```

## Running service toolbox

When you need your own client info or capabilities, start the `rmcp` service yourself and hand it over:

::snippet{src="@swiftide/examples/agents_mcp_tools.rs" anchor="tools"}

`ClientInfo::serve` comes from `rmcp`'s `ServiceExt` and returns a `RunningService`. `McpToolbox::from_running_service` wraps it without contacting the server. Serve from a `ClientInfo`, not from `()`.

The filter methods return `&mut Self`, so finish a chain on a temporary with `.to_owned()`.

::snippet{src="@swiftide/examples/agents_mcp_tools.rs" anchor="agent"}

`add_toolbox` accepts any owned `ToolBox + 'static`. You can add as many toolboxes as you like. A `Vec<Box<dyn Tool>>` is a toolbox too.

The agent lists a toolbox's tools when it first starts, after the `before_all` hooks and the system prompt, not when you call `build`. If listing fails, `query` returns `AgentError::ToolBoxFailedToLoad`. After a successful listing the agent keeps the list, so tools the server adds later do not reach it. A failed listing is retried on the next `query`, and the `before_all` hooks run again.

The example needs `OPENAI_API_KEY` and Node.js:

```sh
cargo run -p swiftide-examples --example agents-mcp
```

It sends the prefixed name `<server name>:add` to OpenAI unchanged. If OpenAI rejects it, wrap the toolbox as described under [Tool names](/agents/mcp/#tool-names).

## Filters

A toolbox holds one filter, and each call replaces it:

- `with_whitelist(names)` keeps only the named tools.
- `with_blacklist(names)` keeps every tool except the named ones.
- `with_filter(ToolFilter::Whitelist(..))` or `with_filter(ToolFilter::Blacklist(..))` sets a filter built elsewhere, such as from configuration. `ToolFilter` implements `Serialize` and `Deserialize`.

Names match the tool names the server reports, exactly and without the server prefix. A whitelisted name that the server does not offer is skipped without an error.

A blacklist suits a server where most tools are safe to expose. This toolbox keeps the GitHub server's read tools and hides three that write:

::snippet{src="@snippets/src/agents/mcp.rs" anchor="blacklist"}

`with_name` and `with_blacklist` both return `&mut Self`, so they chain on a `let mut` binding, and the toolbox itself then moves into `add_toolbox`. A tool that a new version of the server adds passes a blacklist, but not a whitelist.

## Tool names

Since 0.32, Swiftide registers each MCP tool as `<server name>:<tool>`. The server name comes from the handshake, and falls back to `mcp` when it is missing or empty. The model sees the prefixed name, and Swiftide calls the server with the original one.

`with_name` only names the toolbox in logs and does not change the prefix.

OpenAI accepts tool names made of letters, digits, `_` and `-`, so it rejects the `:`. Swiftide does not rewrite the names. The showcase wraps its toolbox in `OpenAiToolNames`, a `ToolBox` in `src/mcp.rs` that replaces every other character with `_`. The model then sees `github-mcp-server_get_pull_request`.

Tool names are unique per agent. When a toolbox tool has the same name as a tool the agent already has, the agent keeps the existing one.

## Results

Since 0.33, Swiftide reads the content blocks of `rmcp` 2 tool results:

- Text blocks are joined with newlines into a text tool output. Other block kinds, such as images, are dropped.
- Only a result with no content blocks at all uses its `structured_content`, serialized as JSON (JavaScript Object Notation).
- A successful result with no text left becomes `Tool executed successfully`. That includes a result with only image blocks, even when it carries `structured_content`.
- A result with `is_error` set becomes `ToolError::Unknown` with the server's text, or `Unknown error` when there is none. The model reads the error as the tool output, and the failure counts toward the tool retry limit (`tool_retry_limit`). See [Tool errors and retries](/agents/tools/#tool-errors-and-retries).
- Arguments that are not valid JSON return `ToolError::WrongArguments` before the server is called.

## Shutdown

`McpToolbox::cancel` disconnects from the server and stops the `rmcp` service. A toolbox is `Clone`, and clones share one connection with the tools they listed. Bind a clone with `let mut` before you pass the toolbox to `add_toolbox`, and call `cancel().await` on it once the agent is done.

After `cancel`, calls to the server's tools return a tool error that says the server is not running, and listing the tools fails. A second `cancel` logs a warning and returns `Ok`.

## Gotchas

- Swiftide supports MCP tools only. Prompts, resources and sampling from the server are not used.
- Use `rmcp` 2. Another major version brings its own `RunningService` type, which `from_running_service` does not accept.
- Every tool that passes the filter needs a description, and its input schema must parse as a JSON Schema that Swiftide accepts. Otherwise `query` returns `AgentError::ToolBoxFailedToLoad` for the whole toolbox, so filter out the tools you do not need.
- A whitelist limits what the model can call, not what the server can do. Pick read-only tools when the model's arguments are untrusted, as the showcase does.

## Reference

- [`McpToolbox`](docsrs:swiftide/agents/tools/mcp/struct.McpToolbox.html)
- [`ToolFilter`](docsrs:swiftide/agents/tools/mcp/enum.ToolFilter.html)
- [`AgentBuilder::add_toolbox`](docsrs:swiftide/agents/struct.AgentBuilder.html#method.add_toolbox)
- [`ToolBox`](docsrs:swiftide/traits/trait.ToolBox.html)
- [`AgentError`](docsrs:swiftide/agents/errors/enum.AgentError.html)
- [`ToolError`](docsrs:swiftide/chat_completion/errors/enum.ToolError.html)
- [`rmcp` transports](https://docs.rs/rmcp/2.1/rmcp/transport/index.html)

## Next

[Providers and reasoning](/agents/providers-and-reasoning/)
