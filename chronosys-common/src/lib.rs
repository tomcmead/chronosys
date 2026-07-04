#![cfg_attr(not(feature = "user"), no_std)] // no_std for eBPF kernel build context, allows std for userspace

use bytemuck::{Pod, Zeroable};

pub const TASK_COMM_LEN: usize = 16; // Linux task name length always 116 bytes

// Process event type enum
pub mod event_kind {
    pub const EXEC: u8 = 0;
    pub const EXIT: u8 = 1;
}

#[derive(Clone, Copy, Debug, Pod, Zeroable)]
#[repr(C)]
pub struct ProcessLifecycleEvent {
    pub timestamp_ns: u64,
    pub pid: u32,
    pub ppid: u32,
    pub uid: u32,
    pub gid: u32,
    pub exit_code: i32, // Only populated for EXIT events
    pub event_kind: u8, // 1 = EXEC, 2 = EXIT
    pub _pad: [u8; 3],  // Align struct to 8-byte boundary for eBPF map storage
    pub comm: [u8; TASK_COMM_LEN],
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for ProcessLifecycleEvent {}

// Userspace helpers, not available in no_std eBPF context
#[cfg(feature = "user")]
impl ProcessLifecycleEvent {
    pub fn kind(&self) -> Option<u8> {
        match self.event_kind {
            event_kind::EXEC | event_kind::EXIT => Some(self.event_kind),
            _ => None,
        }
    }

    // Convert null-terminated (0) comm array to str or "?" if invalid UTF-8
    pub fn comm_str(&self) -> &str {
        let end = self
            .comm
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(TASK_COMM_LEN);
        std::str::from_utf8(&self.comm[..end]).unwrap_or("?")
    }
}

#[cfg(feature = "user")]
unsafe impl aya::Pod for ProcessMetrics {}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct ProcessMetrics {
    pub bytes_read: u64,
    pub bytes_written: u64,
    pub read_ops: u64,
    pub write_ops: u64,
    pub cpu_cycles_ns: u64,
    pub syscall_errors: u64,
}
