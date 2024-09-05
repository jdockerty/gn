use std::time::Instant;

pub struct Statistics {
    start_time: Instant,
    total_bytes: u64,
    throughput: f64,
}

impl Statistics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_bytes: 0,
            throughput: 0.0,
        }
    }

    pub fn increment_total(&mut self, inc: u64) {
        self.total_bytes += inc
    }

    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Retrieve the perceived bytes per second throughput that was written to
    /// the sockets.
    ///
    /// NOTE: Owing to truncation from nanosecond precision to seconds, the
    /// produced throughput may not be accurate for low write counts.
    pub fn record_throughput(&mut self) {
        self.throughput = self.total_bytes as f64 / self.start_time.elapsed().as_secs() as f64;
    }

    /// Return the recorded throughput
    pub fn throughput(&self) -> f64 {
        self.throughput
    }
}
