mod pipeline;
mod system_metrics;

use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const POLL_INTERVAL: Duration = Duration::from_millis(200);
const EBPF_CHANNEL_CAPACITY: usize = 4096;
const METRICS_CHANNEL_CAPACITY: usize = 256;

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

    // Wire channels to form the pipeline
    let (global_metrics_tx, _global_metrics_rx) = mpsc::channel(METRICS_CHANNEL_CAPACITY);
    let (process_lifecycle_tx, process_lifecycle_rx) = mpsc::channel(EBPF_CHANNEL_CAPACITY);
    let (process_snapshot_tx, process_snapshot_rx) = mpsc::channel(METRICS_CHANNEL_CAPACITY);
    let (process_metrics_tx, _process_metrics_rx) = mpsc::channel(METRICS_CHANNEL_CAPACITY);

    // Spawn pipeline tasks, each owns its sender end; receivers passed downstream
    let global_metrics_handle = tokio::spawn(pipeline::global_metrics_task(
        shutdown_token.clone(),
        global_metrics_tx,
        POLL_INTERVAL,
    ));

    let process_metrics_handle = tokio::spawn(pipeline::process_metrics_task(
        shutdown_token.clone(),
        process_lifecycle_rx,
        process_snapshot_rx,
        process_metrics_tx,
    ));

    // Spawn eBPF owner thread context entirely outside of Tokio's pool
    let shutdown_token_ebpf = shutdown_token.clone();
    let ebpf_coordinator_handle = std::thread::spawn(move || {
        let _ = pipeline::run_ebpf_coordinator(
            &shutdown_token_ebpf,
            &process_lifecycle_tx,
            &process_snapshot_tx,
            POLL_INTERVAL,
        );
    });

    // Block until ctrl+c ()
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to listen for signal SIGINT");
    shutdown_token.cancel();

    // Await all async tasks
    let (global_result, process_result) =
        tokio::join!(global_metrics_handle, process_metrics_handle);
    let ebpf_result = ebpf_coordinator_handle.join();

    if let Err(e) = global_result {
        log::error!("System global metrics task panicked: {e:?}");
    }

    if let Err(e) = ebpf_result {
        log::error!("eBPF coordinator task panicked: {e:?}");
    }

    if let Err(e) = process_result {
        log::error!("Process metrics task panicked: {e:?}");
    }

    log::debug!("Chronosys exited");
}
