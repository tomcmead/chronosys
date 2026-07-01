mod pipeline;
mod system_metrics;

use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    // Setup env_logger to read from the RUST_LOG env variable
    #[cfg(feature = "logging")]
    {
        let env = env_logger::Env::default().default_filter_or("info");
        env_logger::Builder::from_env(env).init();
        log::debug!("Log Level: {:?}", log::max_level());
    }

    log::debug!("Chronosys starting...");

    let shutdown_token = CancellationToken::new();

    // Wire channel to form the pipeline
    let (metrics_tx, _metrics_rx) = mpsc::channel(256);

    // Async tasks independently scheduled by tokio
    let global_metrics_handle = tokio::spawn(pipeline::global_metrics_task(
        shutdown_token.clone(),
        metrics_tx,
        Duration::from_millis(200),
    ));

    // Block until ctrl+c
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for signal SIGINT");
    shutdown_token.cancel();

    log::debug!("Chronosys shutting down...");

    // Await all async tasks
    match global_metrics_handle.await {
        Ok(()) => log::debug!("Global metrics task succeeded"),
        Err(e) => log::error!("Global metrics task failed: {e:?}"),
    }

    log::debug!("Chronosys exited");
}
