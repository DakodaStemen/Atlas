//! Lock-free in-process metrics: histograms and counters using AtomicU64.
//! No external dependencies; all operations are wait-free.

use std::sync::atomic::{AtomicU64, Ordering};

/// Fixed histogram bucket boundaries (milliseconds).
const LATENCY_BUCKETS: [u64; 8] = [10, 50, 100, 250, 500, 1000, 5000, 30000];

/// A lock-free histogram for recording latency distributions.
pub struct Histogram {
    /// Bucket counts: buckets[i] counts values <= LATENCY_BUCKETS[i].
    /// buckets[8] counts values > LATENCY_BUCKETS[7] (overflow).
    buckets: [AtomicU64; 9],
    sum: AtomicU64,
    count: AtomicU64,
}

impl Default for Histogram {
    fn default() -> Self {
        Self::new()
    }
}

impl Histogram {
    pub const fn new() -> Self {
        Self {
            buckets: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }

    pub fn record(&self, value_ms: u64) {
        self.sum.fetch_add(value_ms, Ordering::Relaxed);
        self.count.fetch_add(1, Ordering::Relaxed);
        let idx = LATENCY_BUCKETS
            .iter()
            .position(|&b| value_ms <= b)
            .unwrap_or(LATENCY_BUCKETS.len());
        self.buckets[idx].fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> HistogramSnapshot {
        let count = self.count.load(Ordering::Relaxed);
        let sum = self.sum.load(Ordering::Relaxed);
        let mut buckets = [0u64; 9];
        for (i, b) in self.buckets.iter().enumerate() {
            buckets[i] = b.load(Ordering::Relaxed);
        }
        HistogramSnapshot {
            buckets,
            sum,
            count,
        }
    }
}

pub struct HistogramSnapshot {
    pub buckets: [u64; 9],
    pub sum: u64,
    pub count: u64,
}

impl HistogramSnapshot {
    /// Approximate percentile from bucket boundaries.
    pub fn percentile(&self, p: f64) -> u64 {
        if self.count == 0 {
            return 0;
        }
        let target = (self.count as f64 * p / 100.0).ceil() as u64;
        let mut cumulative = 0u64;
        for (i, &bucket_count) in self.buckets.iter().enumerate() {
            cumulative += bucket_count;
            if cumulative >= target {
                if i < LATENCY_BUCKETS.len() {
                    return LATENCY_BUCKETS[i];
                }
                // Overflow bucket — return sum/count as best estimate
                return if self.count > 0 {
                    self.sum / self.count
                } else {
                    0
                };
            }
        }
        if self.count > 0 {
            self.sum / self.count
        } else {
            0
        }
    }

    pub fn p50(&self) -> u64 {
        self.percentile(50.0)
    }
    pub fn p95(&self) -> u64 {
        self.percentile(95.0)
    }
    pub fn avg(&self) -> u64 {
        if self.count > 0 {
            self.sum / self.count
        } else {
            0
        }
    }
}

impl std::fmt::Display for HistogramSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "count={} avg={}ms p50={}ms p95={}ms",
            self.count,
            self.avg(),
            self.p50(),
            self.p95()
        )
    }
}

/// A simple atomic counter.
pub struct Counter(AtomicU64);

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

impl Counter {
    pub const fn new() -> Self {
        Self(AtomicU64::new(0))
    }
    pub fn inc(&self) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
    pub fn get(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

// ── Global metric instances ──

pub static TOOL_LATENCY: Histogram = Histogram::new();
pub static TOOL_CALLS_TOTAL: Counter = Counter::new();
pub static TOOL_ERRORS: Counter = Counter::new();
pub static CACHE_HITS: Counter = Counter::new();
pub static CACHE_MISSES: Counter = Counter::new();

/// Log a summary of all current metric values via tracing::info.
/// Returns the summary string for inclusion in tool responses.
pub fn log_metrics_summary() -> String {
    let latency = TOOL_LATENCY.snapshot();
    let total = TOOL_CALLS_TOTAL.get();
    let errors = TOOL_ERRORS.get();
    let hits = CACHE_HITS.get();
    let misses = CACHE_MISSES.get();

    let summary = format!(
        "Metrics: tool_calls={}, tool_errors={}, cache_hits={}, cache_misses={}, latency=[{}]",
        total, errors, hits, misses, latency
    );
    tracing::info!("{}", summary);
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn histogram_empty() {
        let h = Histogram::new();
        let s = h.snapshot();
        assert_eq!(s.count, 0);
        assert_eq!(s.p50(), 0);
        assert_eq!(s.p95(), 0);
    }

    #[test]
    fn histogram_single_value() {
        let h = Histogram::new();
        h.record(42);
        let s = h.snapshot();
        assert_eq!(s.count, 1);
        assert_eq!(s.p50(), 50); // Falls in <=50 bucket
    }

    #[test]
    fn histogram_distribution() {
        let h = Histogram::new();
        // 50 fast values (5ms each) + 50 slow values (2000ms each)
        for _ in 0..50 {
            h.record(5);
        }
        for _ in 0..50 {
            h.record(2000);
        }
        let s = h.snapshot();
        assert_eq!(s.count, 100);
        assert_eq!(s.p50(), 10); // 50th percentile in <=10 bucket
        assert_eq!(s.p95(), 5000); // 95th percentile in <=5000 bucket
    }

    #[test]
    fn counter_basic() {
        let c = Counter::new();
        assert_eq!(c.get(), 0);
        c.inc();
        c.inc();
        assert_eq!(c.get(), 2);
    }
}
