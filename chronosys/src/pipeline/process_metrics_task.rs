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
                    log::debug!("Process metrics snapshot for pid={} bytes_read={} bytes_read={} read_ops={} write_ops={} syscall_errors={} cpu_cycles_ns={}", process_snapshot.pid, process_snapshot.metrics.bytes_read , process_snapshot.metrics.bytes_written, process_snapshot.metrics.read_ops, process_snapshot.metrics.write_ops, process_snapshot.metrics.syscall_errors, process_snapshot.metrics.cpu_cycles_ns);
                    if metrics_tx.send(process_snapshot.metrics).await.is_err() {
                        return;
                    }
                }
            }
        }
    }
}
