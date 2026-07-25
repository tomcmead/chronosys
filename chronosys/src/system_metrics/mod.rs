pub mod cpu;
pub mod memory;
pub mod types;

use cpu::CpuMetricsCollector;
use memory::MemoryMetricsCollector;
pub use types::*;

pub struct GlobalMetricsCollector {
    memory: MemoryMetricsCollector,
    cpu: CpuMetricsCollector,
}

impl GlobalMetricsCollector {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            memory: MemoryMetricsCollector::new()?,
            cpu: CpuMetricsCollector::new()?,
        })
    }

    pub fn get_metrics(&mut self) -> std::io::Result<GlobalMetrics> {
        Ok(GlobalMetrics {
            memory: self.memory.get_memory_metrics()?,
            cpu: self.cpu.get_cpu_metrics()?,
        })
    }
}
