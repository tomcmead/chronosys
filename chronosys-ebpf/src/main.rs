#![no_std]
#![no_main]

use aya_ebpf::{
    helpers::{
        bpf_get_current_comm, bpf_get_current_pid_tgid, bpf_get_current_uid_gid, bpf_ktime_get_ns,
    },
    macros::{map, tracepoint},
    maps::{HashMap, RingBuf},
    programs::TracePointContext,
};
use chronosys_common::{ProcessLifecycleEvent, ProcessMetrics, TASK_COMM_LEN, event_kind};

// 1Mb ring buffer stores Kernel-space eBPF event structs, user-space reads from other end
#[map]
static PROCESS_LIFECYCLES: RingBuf = RingBuf::with_byte_size(1024 * 1024, 0);

// Map PID to ProcessMetrics for kernel-side metrics collection
#[map]
static mut PROCESS_METRICS: HashMap<u32, ProcessMetrics> = HashMap::with_max_entries(1024, 0);

// Tracepoint eBPF handler pushes new process exec events (creation) to ring buffer for userspace
#[tracepoint]
pub fn handle_exec(ctx: TracePointContext) -> u32 {
    match try_handle_exec(&ctx) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

fn try_handle_exec(ctx: &TracePointContext) -> Result<(), i64> {
    let mut entry = match PROCESS_LIFECYCLES.reserve::<ProcessLifecycleEvent>(0) {
        Some(e) => e,
        None => return Ok(()),
    };

    // eBPF aya context helper functions
    let timestamp_ns: u64 = unsafe { bpf_ktime_get_ns() };
    let ppid: u32 = unsafe { ctx.read_at(16) }.unwrap_or(0); // unsafe read of parent pid from tracepoint context at offset 16 bytes
    let pid_tgid = bpf_get_current_pid_tgid();
    let uid_gid = bpf_get_current_uid_gid();
    let comm = bpf_get_current_comm().unwrap_or([0u8; TASK_COMM_LEN]);

    let event = ProcessLifecycleEvent {
        timestamp_ns,
        pid: (pid_tgid >> 32) as u32, // Upper pid_tgid 32 bits is pid
        ppid,
        uid: uid_gid as u32,
        gid: (uid_gid >> 32) as u32,
        exit_code: 0,
        event_kind: event_kind::EXEC,
        _pad: [0u8; 3],
        comm,
    };

    entry.write(event);
    entry.submit(0);
    Ok(())
}

// Tracepoint eBPF handler pushes process exit events (deletion) to ring buffer for userspace
#[tracepoint]
pub fn handle_exit(ctx: TracePointContext) -> u32 {
    match try_handle_exit(&ctx) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

fn try_handle_exit(ctx: &TracePointContext) -> Result<(), i64> {
    let mut entry = match PROCESS_LIFECYCLES.reserve::<ProcessLifecycleEvent>(0) {
        Some(e) => e,
        None => return Ok(()),
    };

    let timestamp_ns: u64 = unsafe { bpf_ktime_get_ns() };
    let raw_exit: i32 = unsafe { ctx.read_at(8) }.unwrap_or(0);
    let pid_tgid: u64 = bpf_get_current_pid_tgid();
    let uid_gid: u64 = bpf_get_current_uid_gid();
    let comm: [u8; 16] = bpf_get_current_comm().unwrap_or([0u8; TASK_COMM_LEN]);

    let event = ProcessLifecycleEvent {
        timestamp_ns,
        pid: (pid_tgid >> 32) as u32,
        ppid: 0,
        uid: uid_gid as u32,
        gid: (uid_gid >> 32) as u32,
        exit_code: raw_exit >> 8,
        event_kind: event_kind::EXIT,
        _pad: [0u8; 3],
        comm,
    };

    entry.write(event);
    entry.submit(0);
    Ok(())
}

// No std library so needs custom minimal panic handler
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
