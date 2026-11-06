//! Snippets for `observability/langfuse`.

use swiftide::{
    agents::Agent,
    langfuse::{Configuration, LangfuseLayer},
};
use tracing_subscriber::{
    Layer as _, filter::LevelFilter, layer::SubscriberExt as _, util::SubscriberInitExt as _,
};

/// Answers one question and sends every buffered Langfuse event before returning.
pub async fn flush_before_exit(agent: &mut Agent) -> anyhow::Result<()> {
    // ANCHOR: flush
    // Keep a clone: the one inside the registry lives as long as the process.
    let langfuse = LangfuseLayer::default();

    tracing_subscriber::registry()
        .with(langfuse.clone().with_filter(LevelFilter::DEBUG))
        .try_init()?;

    let result = agent
        .query("Which crates changed since the last release?")
        .await;

    // Batches go out every 5 seconds; send the rest before the process exits.
    // A failed flush must not hide why the run failed.
    if let Err(error) = langfuse.flush().await {
        eprintln!("Could not send traces to Langfuse: {error:#}");
    }
    result?;
    // ANCHOR_END: flush
    Ok(())
}

/// Builds the layer from keys and a URL your app loads itself, instead of the environment.
pub fn from_config(public_key: String, secret_key: String) -> LangfuseLayer {
    // ANCHOR: from_config
    let langfuse = LangfuseLayer::from_config(Configuration {
        base_path: "http://langfuse.internal:3000".to_string(),
        user_agent: Some("my-app".to_string()),
        basic_auth: Some((public_key, Some(secret_key))),
        ..Default::default()
    });
    // ANCHOR_END: from_config
    langfuse
}
