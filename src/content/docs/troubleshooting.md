---
title: Troubleshooting
description: Debugging and troubleshooting your pipeline.
---

When building a pipeline, things can go wrong. We provide several tools to help you debug and troubleshoot your pipeline.

By default, if _any_ node fails, the pipeline will stop. This is to prevent cascading failures. You can change this behaviour by using `filter_errors` after any step, or using other available helpers.

## The `tracing` crate

[Tracing](https://github.com/tokio-rs/tracing) quickly became the standard for logging in Rust. We use it throughout the pipeline. When you run your pipeline, you can set the log level to `debug` or `trace` to get detailed logs of what is happening.

To enable tracing you need to configure add a subscriber.

```bash
cargo add tracing tracing-subscriber
```

Then in the entry point of your program, you can add the following code:

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    ...
}
```

When you then set `RUST_LOG=debug` or `RUST_LOG=trace` you will get detailed logs. Depending on the size of the data, it might be a lot

### OpenTelemetry support

Tracing has best-in-class opentelemetry support. See the [tracing-opentelemetry](https://github.com/tokio-rs/tracing-opentelemetry) crate for more information.

Note that currently the Node is attached to every transformation step. Beware of large amounts of tracing data.

## Helpers and utility functions

There are several helpers and utility functions available on the pipeline to help you debug and handle errors.

- `log_all` Logs both passing and failed nodes
- `log_errors` Logs errors only
- `log_nodes` Logs nodes only
- `filter_errors` Filters out errors, only passing nodes
- `filter` Filter out `Result<Node>` based on a predicate
