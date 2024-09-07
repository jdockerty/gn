use std::time::Instant;

pub struct Statistics {
    start_time: Instant,
    total_bytes: u64,
    max_count: u64,
    success_count: u64,
    failure_count: u64,
    throughput: f64,
}

impl Default for Statistics {
    fn default() -> Self {
        Self::new(1)
    }
}

impl Statistics {
    pub fn new(max_count: u64) -> Self {
        Self {
            start_time: Instant::now(),
            total_bytes: 0,
            success_count: 0,
            failure_count: 0,
            throughput: 0.0,
            max_count,
        }
    }

    /// Increment the total number of bytes written
    pub fn increment_total(&mut self, inc: u64) {
        self.total_bytes += inc
    }

    /// Increment the number of successful requests
    pub fn record_success(&mut self) {
        self.success_count += 1;
    }

    /// Increment the number of failed requests
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
    }

    pub fn successful_requests(&self) -> u64 {
        self.success_count
    }

    pub fn success_percentage(&self) -> f64 {
        (self.success_count as f64 / self.max_count as f64) * 100.0
    }

    /// Get the total number of bytes written
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
