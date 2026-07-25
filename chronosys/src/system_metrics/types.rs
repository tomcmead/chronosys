use chronosys_common::ProcessMetrics;

#[derive(Clone, Debug)]
pub struct ProcessMetricsSnapshot {
    pub pid: u32,
    pub metrics: ProcessMetrics,
}

#[derive(Clone, Debug, Default)]
pub struct GlobalMetrics {
    pub memory: MemoryMetrics,
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
