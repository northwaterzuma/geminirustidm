use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Shared token-bucket rate limiter covering both native HTTP segments and external tool local relay.
#[derive(Debug, Clone)]
pub struct SharedRateLimiter {
    inner: Arc<Mutex<RateLimiterState>>,
}

#[derive(Debug)]
struct RateLimiterState {
    rate_bytes_per_sec: u64,
    burst_capacity: u64,
    available_tokens: f64,
    last_refill: Instant,
    total_consumed_bytes: u64,
}

impl SharedRateLimiter {
    pub fn new(rate_bytes_per_sec: u64) -> Self {
        let burst = if rate_bytes_per_sec == 0 {
            0
        } else {
            (rate_bytes_per_sec / 2).max(16_384)
        };
        Self {
            inner: Arc::new(Mutex::new(RateLimiterState {
                rate_bytes_per_sec,
                burst_capacity: burst,
                available_tokens: burst as f64,
                last_refill: Instant::now(),
                total_consumed_bytes: 0,
            })),
        }
    }

    pub fn set_rate(&self, rate_bytes_per_sec: u64) {
        let mut guard = self.inner.lock().unwrap();
        guard.rate_bytes_per_sec = rate_bytes_per_sec;
        guard.burst_capacity = if rate_bytes_per_sec == 0 {
            0
        } else {
            (rate_bytes_per_sec / 2).max(16_384)
        };
        guard.available_tokens = guard.burst_capacity as f64;
        guard.last_refill = Instant::now();
    }

    /// Attempts to consume up to `requested` bytes, returning `(granted_bytes, wait_ms_if_zero)`.
    pub fn acquire(&self, requested: usize) -> (usize, u64) {
        let mut guard = self.inner.lock().unwrap();
        if guard.rate_bytes_per_sec == 0 {
            guard.total_consumed_bytes += requested as u64;
            return (requested, 0);
        }
        let now = Instant::now();
        let elapsed = now.duration_since(guard.last_refill).as_secs_f64();
        guard.last_refill = now;
        guard.available_tokens = (guard.available_tokens + elapsed * (guard.rate_bytes_per_sec as f64))
            .min(guard.burst_capacity as f64);

        if guard.available_tokens >= 1.0 {
            let grant = (requested as f64).min(guard.available_tokens) as usize;
            guard.available_tokens -= grant as f64;
            guard.total_consumed_bytes += grant as u64;
            (grant, 0)
        } else {
            let deficit = 1.0 - guard.available_tokens;
            let wait_ms = ((deficit / (guard.rate_bytes_per_sec as f64)) * 1000.0).ceil() as u64;
            (0, wait_ms.max(1))
        }
    }

    pub fn total_consumed(&self) -> u64 {
        self.inner.lock().unwrap().total_consumed_bytes
    }
}
