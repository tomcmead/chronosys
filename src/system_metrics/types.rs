#[derive(Default, Debug, Clone)]
pub struct GlobalMetrics {
    pub memory: MemoryMetrics,
}

#[derive(Default, Debug, Clone)]
pub struct MemoryMetrics {
    pub total: u64,
    pub free: u64,
    pub available: u64,
    pub used: u64,
    pub buffers: u64,
    pub cached: u64,
    pub swap_total: u64,
    pub swap_free: u64,
    pub slab: u64,
}
