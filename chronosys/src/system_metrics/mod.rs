pub mod memory;
pub mod types;

use memory::MemoryMetricsCollector;
pub use types::*;

pub struct GlobalMetricsCollector {
    memory: MemoryMetricsCollector,
}

impl GlobalMetricsCollector {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            memory: MemoryMetricsCollector::new()?,
        })
    }

    pub fn get_metrics(&mut self) -> std::io::Result<GlobalMetrics> {
        Ok(GlobalMetrics {
            memory: self.memory.get_memory_metrics()?,
        })
    }
}
