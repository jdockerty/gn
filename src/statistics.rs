use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::{sync::atomic::AtomicU64, time::Instant};

use atomic_float::AtomicF64;

pub struct Statistics {
    start_time: Instant,
    total_bytes: Arc<AtomicU64>,
    success_count: Arc<AtomicU64>,
    failure_count: Arc<AtomicU64>,
    throughput: Arc<AtomicF64>,
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
            total_bytes: Arc::new(AtomicU64::new(0)),
            success_count: Arc::new(AtomicU64::new(0)),
            failure_count: Arc::new(AtomicU64::new(0)),
            throughput: Arc::new(AtomicF64::new(0.0)),
        }
    }

    /// Increment the total number of bytes written
    pub fn increment_total(&self, inc: u64) {
        self.total_bytes.fetch_add(inc, Ordering::Release);
    }

    /// Increment the number of successful requests
    pub fn record_success(&self) {
        self.success_count.fetch_add(1, Ordering::Release);
    }

    /// Increment the number of failed requests
    pub fn record_failure(&self) {
        self.failure_count.fetch_add(1, Ordering::Release);
    }

    pub fn successful_requests(&self) -> u64 {
        self.success_count.load(Ordering::Relaxed)
    }

    pub fn success_percentage(&self) -> f64 {
        let success = self.success_count.load(Ordering::Acquire) as f64;
        let failure = self.failure_count.load(Ordering::Relaxed) as f64;

        (success / (success + failure)) * 100.0
    }

    /// Get the total number of bytes written
    pub fn total_bytes(&self) -> u64 {
        self.total_bytes.load(Ordering::Acquire)
    }

    /// Get the total number of sent requests.
    pub fn request_count(&self) -> u64 {
        self.success_count.load(Ordering::Acquire) + self.failure_count.load(Ordering::Relaxed)
    }

    /// Retrieve the perceived bytes per second throughput that was written to
    /// the sockets.
    ///
    /// NOTE: Owing to truncation from nanosecond precision to seconds, the
    /// produced throughput may not be accurate for low write counts.
    pub fn record_throughput(&self) {
        let throughput = self.total_bytes.load(Ordering::Acquire) as f64
            / self.start_time.elapsed().as_secs() as f64;
        self.throughput.store(throughput, Ordering::Relaxed);
    }

    pub fn elapsed(&self) -> u128 {
        self.start_time.elapsed().as_millis()
    }

    /// Return the recorded throughput
    pub fn throughput(&self) -> f64 {
        self.throughput.load(Ordering::Acquire)
    }
}

#[cfg(test)]
mod test {
    use std::sync::atomic::Ordering;

    use super::Statistics;

    #[test]
    fn general() {
        let mut stats = Statistics::new();
        assert_eq!(stats.total_bytes(), 0);
        assert_eq!(stats.successful_requests(), 0);
        assert_eq!(stats.failure_count.load(Ordering::Acquire), 0);

        stats.increment_total(10);
        assert_eq!(stats.total_bytes(), 10);

        stats.record_success();
        assert_eq!(stats.successful_requests(), 1);
        assert_eq!(stats.success_percentage(), 100.0);

        stats.record_failure();
        stats.record_failure();
        stats.record_failure();
        assert_eq!(stats.failure_count.load(Ordering::Relaxed), 3);
        assert_eq!(stats.success_percentage(), 25.0);
        assert_eq!(stats.request_count(), 4);
    }
}
