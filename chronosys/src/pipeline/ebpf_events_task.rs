use crate::system_metrics::types::ProcessMetricsSnapshot;
use chronosys_common::{event_kind, ProcessLifecycleEvent};

use aya::maps::{HashMap, RingBuf};
use aya::programs::TracePoint;
use aya::Ebpf;
use nix::poll::{poll, PollFd, PollFlags, PollTimeout};
use std::os::fd::AsFd;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

// poll() blocking timeout to re-check shutdown flag even if no ring buffer activitiy
const MAX_WAIT: Duration = Duration::from_millis(200);

// Sends kernel-space eBPF events and metrics map snapshots to async pipeline tasks
pub fn run_ebpf_coordinator(
    shutdown: &CancellationToken,
    lifecycle_tx: &mpsc::Sender<ProcessLifecycleEvent>,
    snapshot_tx: &mpsc::Sender<Vec<ProcessMetricsSnapshot>>,
    poll_interval: Duration,
) -> anyhow::Result<()> {
    log::debug!("eBPF process lifecycle and metrics events coordinator starting...");

    let mut ebpf = load_ebpf()?;

    let raw_ring_buf = ebpf
        .take_map("PROCESS_LIFECYCLES")
        .ok_or_else(|| anyhow::anyhow!("eBPF ring buffer PROCESS_LIFECYCLES map not found"))?;
    let mut lifecycles_ring_buf = RingBuf::try_from(raw_ring_buf)?;

    let raw_metrics_map = ebpf
        .take_map("PROCESS_METRICS")
        .ok_or_else(|| anyhow::anyhow!("eBPF map PROCESS_METRICS map not found"))?;
    let mut metrics_map = HashMap::try_from(raw_metrics_map)?;

    let event_size = std::mem::size_of::<ProcessLifecycleEvent>();
    let mut last_poll = Instant::now();

    loop {
        if shutdown.is_cancelled() {
            return Ok(());
        }

        // Block until ring buffer has data or metrics poll interval or MAX_WAIT elapses
        let time_to_next_poll = poll_interval.saturating_sub(last_poll.elapsed());
        let poll_wait_duration = time_to_next_poll.min(MAX_WAIT);

        let ring_buf_fd = lifecycles_ring_buf.as_fd();
        let mut ring_buf_poll_fd = [PollFd::new(ring_buf_fd, PollFlags::POLLIN)];
        let poll_timeout = PollTimeout::try_from(poll_wait_duration).unwrap_or(PollTimeout::MAX);

        match poll(&mut ring_buf_poll_fd, poll_timeout) {
            Ok(_) => {}
            Err(nix::errno::Errno::EINTR) => continue, // crtl+c (SIGNINT), loops back to shutdown check
            Err(e) => return Err(anyhow::anyhow!("Poll eBPF ring buffer fd failed: {e}")),
        }

        // Drain all process lifecycle events from ring buffer, sending them to async pipeline
        while let Some(item) = lifecycles_ring_buf.next() {
            if item.len() != event_size {
                log::warn!(
                    "dropping malformed lifecycle event: expected {event_size} bytes, got {}",
                    item.len()
                );
                continue;
            }

            let process_event = bytemuck::pod_read_unaligned::<ProcessLifecycleEvent>(&item);

            // Remove PID from kernel-side metrics map when process exits and send final snapshot to user-space
            if process_event.event_kind == event_kind::EXIT {
                let mut final_process_snapshot = Vec::new();
                if let Ok(final_process_metrics) = metrics_map.get(&process_event.pid, 0) {
                    final_process_snapshot.push(ProcessMetricsSnapshot {
                        pid: process_event.pid,
                        metrics: final_process_metrics,
                    });
                    let _ = snapshot_tx.blocking_send(final_process_snapshot);
                }
                let _ = metrics_map.remove(&process_event.pid);
            }

            if lifecycle_tx.blocking_send(process_event).is_err() {
                return Ok(()); // Pipeline hung up
            }
        }

        // Poll kernel-side metrics map at fixed interval and send batch of snapshots to async pipeline
        if last_poll.elapsed() >= poll_interval {
            let mut batch = Vec::new();

            for entry in &metrics_map {
                match entry {
                    Ok((pid, bpf_metrics)) => batch.push(ProcessMetricsSnapshot {
                        pid,
                        metrics: bpf_metrics,
                    }),
                    Err(e) => log::warn!("error iterating metrics map entry: {e}"),
                }
            }

            if !batch.is_empty() && snapshot_tx.blocking_send(batch).is_err() {
                return Ok(());
            }
            last_poll = Instant::now();
        }
    }
}

// Load compiled eBPF binary and attach tracepoints to kernel events
pub fn load_ebpf() -> anyhow::Result<Ebpf> {
    let mut ebpf = Ebpf::load(aya::include_bytes_aligned!(concat!(
        env!("OUT_DIR"),
        "/chronosys-ebpf"
    )))?;

    // Attach exec and exit tracepoints to collect process lifecycle events
    attach_tracepoint(&mut ebpf, "handle_exec", "sched", "sched_process_exec")?;
    attach_tracepoint(&mut ebpf, "handle_exit", "sched", "sched_process_exit")?;

    // Attach syscall tracepoints to collect process metrics
    attach_tracepoint(
        &mut ebpf,
        "handle_sys_exit_read",
        "syscalls",
        "sys_exit_read",
    )?;
    attach_tracepoint(
        &mut ebpf,
        "handle_sys_exit_write",
        "syscalls",
        "sys_exit_write",
    )?;
    attach_tracepoint(
        &mut ebpf,
        "handle_generic_sys_exit",
        "raw_syscalls",
        "sys_exit",
    )?;

    Ok(ebpf)
}

fn attach_tracepoint(
    ebpf: &mut Ebpf,
    name: &str,
    category: &str,
    event: &str,
) -> anyhow::Result<()> {
    let prog: &mut TracePoint = ebpf
        .program_mut(name)
        .ok_or_else(|| anyhow::anyhow!("eBPF program '{name}' not found"))?
        .try_into()?;
    prog.load()?;
    prog.attach(category, event)?;
    log::debug!("attached tracepoint {category}:{event}");
    Ok(())
}
