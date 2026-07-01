use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

pub use super::types::{GlobalMetrics, MemoryMetrics};

pub struct GlobalMetricsCollector {
    memory_info_file: File,
    buffer: Vec<u8>,
}

impl GlobalMetricsCollector {
    pub fn new() -> std::io::Result<Self> {
        Ok(Self {
            memory_info_file: File::open("/proc/meminfo")?,
            buffer: Vec::with_capacity(2048),
        })
    }

    pub fn get_metrics(&mut self) -> std::io::Result<GlobalMetrics> {
        self.buffer.clear();
        self.memory_info_file.seek(SeekFrom::Start(0))?;
        self.memory_info_file.read_to_end(&mut self.buffer)?;

        let contents = std::str::from_utf8(&self.buffer)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        Ok(GlobalMetrics {
            memory: Self::parse_memory_metrics(contents),
        })
    }

    fn parse_memory_metrics(contents: &str) -> MemoryMetrics {
        let mut mem_metrics = MemoryMetrics::default();
        let mut num_mem_metrics = 8; // Early return after all 8 fields parsed

        // Parse /proc/meminfo line-by-line in format "MemType Value kB"
        for line in contents.lines() {
            if num_mem_metrics == 0 {
                break;
            }

            let mut parts = line.split_whitespace();
            let (Some(mem_type), Some(mem_val_str)) = (parts.next(), parts.next()) else {
                continue;
            };
            let Ok(mem_val_kb) = mem_val_str.parse::<u64>() else {
                continue;
            };
            let mem_val_bytes = mem_val_kb * 1024;

            match mem_type {
                "MemTotal:" => {
                    mem_metrics.total = mem_val_bytes;
                    num_mem_metrics -= 1;
                }
                "MemFree:" => {
                    mem_metrics.free = mem_val_bytes;
                    num_mem_metrics -= 1;
                }
                "MemAvailable:" => {
                    mem_metrics.available = mem_val_bytes;
                    num_mem_metrics -= 1;
                }
                "Buffers:" => {
                    mem_metrics.buffers = mem_val_bytes;
                    num_mem_metrics -= 1;
                }
                "Cached:" => {
                    mem_metrics.cached = mem_val_bytes;
                    num_mem_metrics -= 1;
                }
                "SwapTotal:" => {
                    mem_metrics.swap_total = mem_val_bytes;
                    num_mem_metrics -= 1;
                }
                "SwapFree:" => {
                    mem_metrics.swap_free = mem_val_bytes;
                    num_mem_metrics -= 1;
                }
                "Slab:" => {
                    mem_metrics.slab = mem_val_bytes;
                    num_mem_metrics -= 1;
                }
                _ => {}
            }
        }

        // Derive used memory once rather than making every caller do it.
        mem_metrics.used = mem_metrics.total.saturating_sub(mem_metrics.available);
        mem_metrics
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test /proc/meminfo format
    const MOCK_MEMINFO: &str = "\
        MemTotal:        3600 kB
        MemFree:          100 kB
        MemAvailable:     900 kB
        Buffers:          200 kB
        Cached:           300 kB
        SwapTotal:        400 kB
        SwapFree:         500 kB
        Slab:             600 kB
        Active:           700 kB
        Inactive:         800 kB
    ";

    #[test]
    fn test_parse_memory_metrics_success() {
        let memory_metrics = GlobalMetricsCollector::parse_memory_metrics(MOCK_MEMINFO);

        assert_eq!(memory_metrics.total, 3600 * 1024);
        assert_eq!(memory_metrics.free, 100 * 1024);
        assert_eq!(memory_metrics.available, 900 * 1024);
        assert_eq!(memory_metrics.buffers, 200 * 1024);
        assert_eq!(memory_metrics.cached, 300 * 1024);
        assert_eq!(memory_metrics.swap_total, 400 * 1024);
        assert_eq!(memory_metrics.swap_free, 500 * 1024);
        assert_eq!(memory_metrics.slab, 600 * 1024);
        assert_eq!(memory_metrics.used, (3600 - 900) * 1024);
    }

    #[test]
    fn test_parse_memory_metrics_malformed_lines() {
        let input = "\
            MemTotal:       1600 kB
            InvalidLineWithNoValue
            MemFree:        400 kB
            MemAvailable:   800 kB
            CorruptLabel:   abc kB
            Buffers:        100 kB
        ";
        let memory_metrics = GlobalMetricsCollector::parse_memory_metrics(input);

        assert_eq!(memory_metrics.total, 1600 * 1024);
        assert_eq!(memory_metrics.free, 400 * 1024);
        assert_eq!(memory_metrics.available, 800 * 1024);
        assert_eq!(memory_metrics.buffers, 100 * 1024);
        // Unparsed fields set to zero
        assert_eq!(memory_metrics.cached, 0);
        assert_eq!(memory_metrics.slab, 0);
        assert_eq!(memory_metrics.used, (1600 - 800) * 1024);
    }

    #[test]
    fn test_parse_memory_metrics_empty_input() {
        let memory_metrics = GlobalMetricsCollector::parse_memory_metrics("");
        assert_eq!(memory_metrics.total, 0);
        assert_eq!(memory_metrics.free, 0);
        assert_eq!(memory_metrics.used, 0);
    }
}
