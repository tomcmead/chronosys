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
static PROCESS_METRICS: HashMap<u32, ProcessMetrics> = HashMap::with_max_entries(1024, 0);

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
    let pid_tgid = bpf_get_current_pid_tgid();
    let pid = (pid_tgid >> 32) as u32; // Upper pid_tgid 32 bits is pid
    let ppid: u32 = unsafe { ctx.read_at(16) }.unwrap_or(0); // unsafe read of parent pid from tracepoint context at offset 16 bytes
    let uid_gid = bpf_get_current_uid_gid();
    let comm = bpf_get_current_comm().unwrap_or([0u8; TASK_COMM_LEN]);

    // Initialise process metrics from new pid
    let zeroed = ProcessMetrics {
        bytes_read: 0,
        bytes_written: 0,
        read_ops: 0,
        write_ops: 0,
        cpu_cycles_ns: 0,
        syscall_errors: 0,
    };
    let _ = PROCESS_METRICS.insert(&pid, &zeroed, 0); // Ignore failure when map full

    let event = ProcessLifecycleEvent {
        timestamp_ns,
        pid,
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

fn get_process_metrics(pid: u32) -> Option<*mut ProcessMetrics> {
    PROCESS_METRICS.get_ptr_mut(&pid)
}

// Tracepoint: syscalls/sys_exit_read
#[tracepoint]
pub fn handle_sys_exit_read(ctx: TracePointContext) -> u32 {
    let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
    let Some(metrics_ptr) = get_process_metrics(pid) else {
        return 0;
    };

    // Check return code of system call syscalls:sys_exit_read
    let Ok(ret) = (unsafe { ctx.read_at::<i64>(16) }) else {
        return 0;
    };

    unsafe {
        if ret < 0 {
            (*metrics_ptr).syscall_errors += 1;
        } else {
            (*metrics_ptr).read_ops += 1;
            (*metrics_ptr).bytes_read += ret as u64;
        }
    }
    0
}

// Tracepoint: syscalls/sys_exit_write
#[tracepoint]
pub fn handle_sys_exit_write(ctx: TracePointContext) -> u32 {
    let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
    let Some(metrics_ptr) = get_process_metrics(pid) else {
        return 0;
    };

    // Check return code of system call syscalls:sys_exit_write
    let Ok(ret) = (unsafe { ctx.read_at::<i64>(16) }) else {
        return 0;
    };

    unsafe {
        if ret < 0 {
            (*metrics_ptr).syscall_errors += 1;
        } else {
            (*metrics_ptr).write_ops += 1;
            (*metrics_ptr).bytes_written += ret as u64;
        }
    }
    0
}

// Tracepoint: raw_syscalls/sys_exit
#[tracepoint]
pub fn handle_generic_sys_exit(ctx: TracePointContext) -> u32 {
    let Ok(syscall_nr) = (unsafe { ctx.read_at::<i64>(8) }) else {
        return 0;
    };

    const EXCLUDED_SYSCALL_NUMS: [i64; 2] = [63, 64]; // read, write
    if EXCLUDED_SYSCALL_NUMS.contains(&syscall_nr) {
        return 0; // already handled by handle_sys_exit_read/write
    }

    // Check return code of system call raw_syscalls:sys_exit
    let Ok(ret) = (unsafe { ctx.read_at::<i64>(16) }) else {
        return 0;
    };

    if ret < 0 {
        let pid = (bpf_get_current_pid_tgid() >> 32) as u32;
        if let Some(metrics_ptr) = get_process_metrics(pid) {
            unsafe {
                (*metrics_ptr).syscall_errors += 1;
            }
        }
    }
    0
}

// No std library so needs custom minimal panic handler
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
