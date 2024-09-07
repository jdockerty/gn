use std::time::Instant;

pub struct Statistics {
    start_time: Instant,
    total_bytes: u64,
    success_count: u64,
    failure_count: u64,
    throughput: f64,
}

impl Default for Statistics {
    fn default() -> Self {
        Self::new()
    }
}

impl Statistics {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            total_bytes: 0,
            success_count: 0,
            failure_count: 0,
            throughput: 0.0,
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
        (self.success_count as f64 / (self.success_count + self.failure_count) as f64) * 100.0
    }

    /// Get the total number of bytes written
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes
    }

    /// Get the total number of sent requests.
    pub fn request_count(&self) -> u64 {
        self.success_count + self.failure_count
    }

    /// Retrieve the perceived bytes per second throughput that was written to
    /// the sockets.
    ///
    /// NOTE: Owing to truncation from nanosecond precision to seconds, the
    /// produced throughput may not be accurate for low write counts.
    pub fn record_throughput(&mut self) {
        self.throughput = self.total_bytes as f64 / self.start_time.elapsed().as_secs() as f64;
    }

    pub fn elapsed(&self) -> u128 {
        self.start_time.elapsed().as_millis()
    }

    /// Return the recorded throughput
    pub fn throughput(&self) -> f64 {
        self.throughput
    }
}

#[cfg(test)]
mod test {
    use super::Statistics;

    #[test]
    fn general() {
        let mut stats = Statistics::new();
        assert_eq!(stats.total_bytes(), 0);
        assert_eq!(stats.successful_requests(), 0);
        assert_eq!(stats.failure_count, 0);

        stats.increment_total(10);
        assert_eq!(stats.total_bytes(), 10);

        stats.record_success();
        assert_eq!(stats.successful_requests(), 1);
        assert_eq!(stats.success_percentage(), 100.0);

        stats.record_failure();
        stats.record_failure();
        stats.record_failure();
        assert_eq!(stats.failure_count, 3);
        assert_eq!(stats.success_percentage(), 25.0);
        assert_eq!(stats.request_count(), 4);
    }
}
