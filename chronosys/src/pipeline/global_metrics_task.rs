use crate::system_metrics::global_metrics::GlobalMetricsCollector;
use crate::system_metrics::types::GlobalMetrics;

use tokio::sync::mpsc;
use tokio::time::Duration;
use tokio_util::sync::CancellationToken;

pub async fn global_metrics_task(
    shutdown_token: CancellationToken,
    send_metrics: mpsc::Sender<GlobalMetrics>,
    poll_interval: Duration,
) {
    log::debug!("System metrics async task starting...");

    let mut interval = tokio::time::interval(poll_interval);
    let mut metrics_collector =
        GlobalMetricsCollector::new().expect("Failed to initialise GlobalMetricsCollector");

    loop {
        tokio::select! {
            biased;
            () = shutdown_token.cancelled() => break,

            _ = interval.tick() => {
                // Move collector in, get metrics, and return the collector back
                let (global_metrics, returned_collector) = tokio::task::spawn_blocking(move || {
                    let global_metrics = metrics_collector.get_metrics();
                    (global_metrics, metrics_collector)
                }).await.expect("GlobalMetricsCollector thread failed to join");

                metrics_collector = returned_collector;

                match global_metrics {
                    Ok(metrics) => {
                        log::debug!("Memory metrics: {:?}", metrics.memory);
                        if let Err(e) = send_metrics.send(metrics).await {
                            log::error!("Failed to send global metrics with error: {e:?}");
                            return;
                        }}
                    Err(e) => log::error!("GlobalMetricsCollector failed to get metrics: {e:?}"),
                }
            }
        }
    }
}
