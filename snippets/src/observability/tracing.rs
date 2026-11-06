//! Snippets for observability/tracing: a subscriber for Swiftide's spans and events, a parent span
//! per unit of work, and OpenTelemetry export.

use anyhow::Result;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use swiftide::agents::Agent;
use tracing::Instrument as _;
use tracing_subscriber::{
    EnvFilter, Layer as _,
    filter::{LevelFilter, Targets},
    fmt,
    layer::SubscriberExt as _,
    util::SubscriberInitExt as _,
};

// ANCHOR: subscriber
/// Prints Swiftide's logs to stderr. `RUST_LOG` replaces the default filter.
pub fn init_logging() -> Result<()> {
    let filter = EnvFilter::try_from_default_env()
        // Hide the clients' own WARN line for every failed model call and the HTTP client's warnings.
        .unwrap_or_else(|_| EnvFilter::new("info,swiftide_integrations=off,async_openai=error"));

    tracing_subscriber::registry()
        .with(fmt::layer().with_writer(std::io::stderr))
        .with(filter)
        .try_init()?;

    Ok(())
}
// ANCHOR_END: subscriber

// ANCHOR: parent_span
/// Answers one question. Every span the agent opens nests under `answer_question`.
pub async fn answer_question(agent: &mut Agent, user_id: &str, question: String) -> Result<()> {
    agent
        .query(question)
        .instrument(tracing::info_span!("answer_question", user = %user_id))
        .await?;

    Ok(())
}
// ANCHOR_END: parent_span

// ANCHOR: otlp
/// Logs to stderr and exports spans over OTLP/HTTP (default `http://localhost:4318`, or
/// `OTEL_EXPORTER_OTLP_ENDPOINT`).
pub fn init_tracing() -> Result<SdkTracerProvider> {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .build()?;
    let provider = SdkTracerProvider::builder()
        .with_resource(Resource::builder().with_service_name("my-app").build())
        .with_batch_exporter(exporter)
        .build();

    let logs = fmt::layer()
        .with_writer(std::io::stderr)
        .with_filter(EnvFilter::new("warn,swiftide_integrations=off"));

    // Swiftide's spans and your own; INFO leaves out the per-node indexing spans (TRACE).
    let spans = tracing_opentelemetry::layer()
        .with_tracer(provider.tracer("my-app"))
        .with_filter(
            Targets::new()
                .with_target("swiftide", LevelFilter::INFO)
                .with_target(env!("CARGO_CRATE_NAME"), LevelFilter::INFO),
        );

    tracing_subscriber::registry()
        .with(logs)
        .with(spans)
        .try_init()?;

    Ok(provider)
}

pub async fn run_app(agent: &mut Agent) -> Result<()> {
    let provider = init_tracing()?;

    let result = answer_question(
        agent,
        "user-1",
        "What changed since the last release?".into(),
    )
    .await;

    // Sends the spans that are still buffered.
    provider.shutdown()?;
    result
}
// ANCHOR_END: otlp
