use chronosys_common::ProcessMetrics;

#[derive(Clone, Debug)]
pub struct ProcessMetricsSnapshot {
    pub pid: u32,
    pub metrics: ProcessMetrics,
}

#[derive(Clone, Debug, Default)]
pub struct GlobalMetrics {
    pub memory: MemoryMetrics,
    pub cpu: CpuMetrics,
}

#[derive(Clone, Debug, Default)]
pub struct MemoryMetrics {
    pub total: u64,      // Total physical RAM (MemTotal)
    pub free: u64,       // Unused RAM (MemFree)
    pub available: u64,  // RAM usable for new apps without swapping (MemAvailable)
    pub used: u64,       // Active RAM in use (total - available)
    pub buffers: u64,    // Raw disk block cache (Buffers)
    pub cached: u64,     // Regular file page cache (Cached)
    pub swap_total: u64, // Total allocated swap space (SwapTotal)
    pub swap_free: u64,  // Unused swap space available (SwapFree)
    pub slab: u64,       // Kernel-internal data structure cache (Slab)
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct CpuCounters {
    pub user: u64,       // Time spent in normal processes in user mode
    pub nice: u64,       // Time spent in niced (low priority) processes in user mode
    pub system: u64,     // Time spent in kernel mode
    pub idle: u64,       // Time spent doing nothing
    pub iowait: u64,     // Time spent waiting for I/O to complete
    pub irq: u64,        // Time spent servicing hardware interrupts
    pub softirq: u64,    // Time spent servicing software interrupts
    pub steal: u64,      // Stolen time in virtualized environments
    pub guest: u64,      // Time spent running a virtual CPU for guest OS
    pub guest_nice: u64, // Time spent running a niced guest OS
}

#[allow(dead_code)]
#[derive(Clone, Debug, Default)]
pub struct CpuMetrics {
    pub cpu_total: CpuCounters,
    pub cpu_cores: Vec<CpuCounters>,
    pub num_procs_running: u64,
    pub num_procs_blocked: u64,
}
