use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::mem;

pub use super::types::{CpuCounters, CpuMetrics};

pub struct CpuMetricsCollector {
    cpu_info_file: File,
    buffer: Vec<u8>,
    per_cpu_buffer: Vec<CpuCounters>,
}

impl CpuMetricsCollector {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            cpu_info_file: File::open("/proc/stat")?,
            buffer: Vec::with_capacity(1024),
            per_cpu_buffer: Vec::with_capacity(16),
        })
    }

    pub fn get_cpu_metrics(&mut self) -> std::io::Result<CpuMetrics> {
        self.cpu_info_file.seek(SeekFrom::Start(0))?;
        self.buffer.clear();
        self.cpu_info_file.read_to_end(&mut self.buffer)?;

        let cpu_contents = std::str::from_utf8(&self.buffer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(Self::parse_cpu_info(cpu_contents, &mut self.per_cpu_buffer))
    }

    fn parse_cpu_info(contents: &str, per_cpu_buffer: &mut Vec<CpuCounters>) -> CpuMetrics {
        per_cpu_buffer.clear();

        let mut cpu_total = CpuCounters::default();
        let mut num_procs_running = 0;
        let mut num_procs_blocked = 0;

        for line in contents.lines() {
            let mut parts = line.split_whitespace();
            let Some(header) = parts.next() else {
                continue;
            };

            if header == "cpu" {
                cpu_total = Self::parse_cpu_counters_line(parts);
            } else if header.starts_with("cpu") && header[3..].chars().all(|c| c.is_ascii_digit()) {
                per_cpu_buffer.push(Self::parse_cpu_counters_line(parts));
            } else if header == "procs_running" {
                num_procs_running = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            } else if header == "procs_blocked" {
                num_procs_blocked = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            }
        }

        // Keep capacity to avoid re-allocations
        let capacity = per_cpu_buffer.capacity();
        // Move populated buffer and reset buffer to default
        let cpu_cores = mem::take(per_cpu_buffer);
        per_cpu_buffer.reserve(capacity);

        CpuMetrics {
            cpu_total,
            cpu_cores,
            num_procs_running,
            num_procs_blocked,
        }
    }

    fn parse_cpu_counters_line<'a>(mut parts: impl Iterator<Item = &'a str>) -> CpuCounters {
        let next_val = |parts: &mut dyn Iterator<Item = &'a str>| {
            parts
                .next()
                .map_or(0, |s| Self::parse_u64_fast(s.as_bytes()))
        };

        CpuCounters {
            user: next_val(&mut parts),
            nice: next_val(&mut parts),
            system: next_val(&mut parts),
            idle: next_val(&mut parts),
            iowait: next_val(&mut parts),
            irq: next_val(&mut parts),
            softirq: next_val(&mut parts),
            steal: next_val(&mut parts),
            guest: next_val(&mut parts),
            guest_nice: next_val(&mut parts),
        }
    }

    fn parse_u64_fast(bytes: &[u8]) -> u64 {
        let mut val = 0u64;
        for &byte in bytes {
            if byte.is_ascii_digit() {
                val = val.wrapping_mul(10).wrapping_add(u64::from(byte - b'0'));
            } else {
                break;
            }
        }
        val
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test /proc/stat format
    const MOCK_PROC_STAT: &str = "\
        cpu  1000 100 500 5000 50 10 5 2 0 0
        cpu0 500 50 250 2500 25 5 2 1 0 0
        cpu1 500 50 250 2500 25 5 3 1 0 0
        intr 1234567
        ctxt 987654
        btime 1710000000
        processes 1234
        procs_running 3
        procs_blocked 1
    ";

    #[test]
    fn test_parse_u64_fast() {
        assert_eq!(CpuMetricsCollector::parse_u64_fast(b"12345"), 12345);
        assert_eq!(CpuMetricsCollector::parse_u64_fast(b"0"), 0);
        assert_eq!(
            CpuMetricsCollector::parse_u64_fast(b"987654321_extra"),
            987654321
        ); // Stops at non-digit
        assert_eq!(CpuMetricsCollector::parse_u64_fast(b""), 0);
    }

    #[test]
    fn test_parse_cpu_counters_line() {
        let line = "100 200 300 400 50 10 5 2 1 0";
        let parts = line.split_whitespace();
        let counters = CpuMetricsCollector::parse_cpu_counters_line(parts);

        assert_eq!(counters.user, 100);
        assert_eq!(counters.nice, 200);
        assert_eq!(counters.system, 300);
        assert_eq!(counters.idle, 400);
        assert_eq!(counters.iowait, 50);
        assert_eq!(counters.irq, 10);
        assert_eq!(counters.softirq, 5);
        assert_eq!(counters.steal, 2);
        assert_eq!(counters.guest, 1);
        assert_eq!(counters.guest_nice, 0);
    }

    #[test]
    fn test_parse_cpu_info() {
        let mut per_cpu_buffer = Vec::new();
        let metrics = CpuMetricsCollector::parse_cpu_info(MOCK_PROC_STAT, &mut per_cpu_buffer);

        // Cpu totals
        assert_eq!(metrics.cpu_total.user, 1000);
        assert_eq!(metrics.cpu_total.idle, 5000);

        // Per-cpu extraction
        assert_eq!(metrics.cpu_cores.len(), 2);
        assert_eq!(metrics.cpu_cores[0].user, 500);
        assert_eq!(metrics.cpu_cores[1].user, 500);

        // Process states
        assert_eq!(metrics.num_procs_running, 3);
        assert_eq!(metrics.num_procs_blocked, 1);
    }

    #[test]
    fn test_buffer_reuse_and_clearing() {
        let mut per_cpu_buffer = Vec::with_capacity(4);

        // Populate buffer initially
        per_cpu_buffer.push(CpuCounters::default());
        let initial_capacity = per_cpu_buffer.capacity();

        let metrics = CpuMetricsCollector::parse_cpu_info(MOCK_PROC_STAT, &mut per_cpu_buffer);

        // Old items cleared and new ones populated
        assert_eq!(metrics.cpu_cores.len(), 2);
        assert!(per_cpu_buffer.capacity() >= initial_capacity); // Capacity preserved
    }
}
