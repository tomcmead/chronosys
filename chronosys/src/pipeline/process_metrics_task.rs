use crate::system_metrics::types::ProcessMetricsSnapshot;
use chronosys_common::{event_kind, ProcessLifecycleEvent, ProcessMetrics};

use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub async fn process_metrics_task(
    shutdown: CancellationToken,
    mut lifecycle_rx: mpsc::Receiver<ProcessLifecycleEvent>,
    mut snapshot_rx: mpsc::Receiver<Vec<ProcessMetricsSnapshot>>,
    metrics_tx: mpsc::Sender<ProcessMetrics>,
) {
    log::debug!("Process metrics task starting...");

    loop {
        tokio::select! {
            biased;
            () = shutdown.cancelled() => break,

            // Process lifecycle events received from eBPF ring buffer
            event = lifecycle_rx.recv() => {
                let Some(e) = event else {
                    log::warn!("eBPF event channel closed unexpectedly");
                    break;
                };

                if e.event_kind == event_kind::EXEC {
                    log::debug!("Process exec detected for pid={} comm={}", e.pid, e.comm_str());
                }

                if e.event_kind == event_kind::EXIT {
                    log::debug!("Process exit detected for pid={} comm={}", e.pid, e.comm_str());
                }
            }

            // Process metrics received from eBPF metrics map snapshots
            process_snapshots = snapshot_rx.recv() => {
                let Some(process_snapshots) = process_snapshots else {
                    log::debug!("Process metrics snapshot channel closed, shutting down");
                    break;
                };

                for process_snapshot in process_snapshots {
                    log::debug!("Process metrics snapshot for pid={}", process_snapshot.pid);
                    if metrics_tx.send(process_snapshot.metrics).await.is_err() {
                        return;
                    }
                }
            }
        }
    }
}
