//! Performance sampling framework
//!
//! Provides statistical sampling of operations with minimal overhead.

extern crate alloc;

use alloc::{vec::Vec, string::String, collections::BTreeMap};
use core::sync::atomic::{AtomicU64, Ordering};

use crate::subsystems::sync::Mutex;
use crate::subsystems::time;

/// Performance sampler with configurable sampling rate
pub struct PerformanceSampler {
    /// Sample rate (1 = sample every call, 100 = sample every 100th call)
    sample_rate: u32,
    /// Call counter for sampling decisions
    counter: AtomicU64,
    /// Sample buffer
    samples: Mutex<SampleBuffer>,
    /// Sampler name
    name: String,
    /// Enable/disable flag
    enabled: AtomicU64,
}

/// Sample data
#[derive(Debug, Clone)]
pub struct Sample {
    /// Timestamp (nanoseconds)
    pub timestamp: u64,
    /// Duration in nanoseconds
    pub duration_ns: u64,
    /// Call site identifier
    pub call_site: &'static str,
    /// Metadata
    pub metadata: SampleMetadata,
}

/// Sample metadata
#[derive(Debug, Clone)]
pub struct SampleMetadata {
    /// CPU ID
    pub cpu_id: u32,
    /// Thread ID
    pub thread_id: u64,
    /// Thread priority
    pub priority: u8,
}

/// Circular buffer for samples
struct SampleBuffer {
    buffer: Vec<Sample>,
    capacity: usize,
    index: usize,
    count: AtomicU64,
}

impl SampleBuffer {
    /// Create a new sample buffer with specified capacity
    fn new(capacity: usize) -> Self {
        Self {
            buffer: Vec::with_capacity(capacity),
            capacity,
            index: 0,
            count: AtomicU64::new(0),
        }
    }

    /// Push a sample into the buffer (circular)
    fn push(&mut self, sample: Sample) {
        if self.buffer.len() < self.capacity {
            self.buffer.push(sample);
        } else {
            self.buffer[self.index] = sample;
            self.index = (self.index + 1) % self.capacity;
        }
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get all samples
    fn samples(&self) -> Vec<Sample> {
        if self.buffer.len() < self.capacity {
            self.buffer.clone()
        } else {
            let mut result = Vec::with_capacity(self.capacity);
            for i in 0..self.capacity {
                result.push(self.buffer[(self.index + i) % self.capacity].clone());
            }
            result
        }
    }

    /// Get sample count
    fn total_count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }

    /// Clear buffer
    fn clear(&mut self) {
        self.buffer.clear();
        self.index = 0;
        self.count.store(0, Ordering::Relaxed);
    }
}

impl PerformanceSampler {
    /// Create a new performance sampler
    ///
    /// # Arguments
    /// * `name` - Sampler name
    /// * `sample_rate` - Sampling rate (1 = sample every call, N = sample every Nth call)
    /// * `buffer_capacity` - Maximum number of samples to keep
    pub fn new(name: &str, sample_rate: u32, buffer_capacity: usize) -> Self {
        Self {
            sample_rate: sample_rate.max(1),
            counter: AtomicU64::new(0),
            samples: Mutex::new(SampleBuffer::new(buffer_capacity)),
            name: String::from(name),
            enabled: AtomicU64::new(1),
        }
    }

    /// Sample a function call
    ///
    /// Only actually samples based on the configured rate.
    /// Minimal overhead when not sampling.
    #[inline]
    pub fn sample<F, R>(&self, call_site: &'static str, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        // Fast path: check if enabled
        if self.enabled.load(Ordering::Relaxed) == 0 {
            return f();
        }

        let count = self.counter.fetch_add(1, Ordering::Relaxed);

        // Sampling decision: only sample every Nth call
        if count % (self.sample_rate as u64) == 0 {
            let start = time::hrtime_nanos();
            let result = f();
            let duration = time::hrtime_nanos().saturating_sub(start);

            // Record sample
            let sample = Sample {
                timestamp: start,
                duration_ns: duration,
                call_site,
                metadata: SampleMetadata {
                    cpu_id: self.current_cpu_id(),
                    thread_id: self.current_thread_id(),
                    priority: self.current_priority(),
                },
            };

            self.samples.lock().push(sample);
            result
        } else {
            f()
        }
    }

    /// Enable the sampler
    pub fn enable(&self) {
        self.enabled.store(1, Ordering::Release);
    }

    /// Disable the sampler
    pub fn disable(&self) {
        self.enabled.store(0, Ordering::Release);
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Relaxed) == 1
    }

    /// Get sample summary statistics
    pub fn summary(&self) -> SampleSummary {
        let samples = self.samples.lock();
        let data = samples.samples();

        if data.is_empty() {
            return SampleSummary::empty();
        }

        let count = data.len();
        let total_duration: u64 = data.iter().map(|s| s.duration_ns).sum();
        let avg_duration = total_duration / count as u64;
        let min_duration = data.iter().map(|s| s.duration_ns).min().unwrap_or(0);
        let max_duration = data.iter().map(|s| s.duration_ns).max().unwrap_or(0);

        // Calculate percentiles
        let mut sorted_durations: Vec<u64> = data.iter().map(|s| s.duration_ns).collect();
        sorted_durations.sort_unstable();
        let p50 = Self::percentile(&sorted_durations, 50);
        let p95 = Self::percentile(&sorted_durations, 95);
        let p99 = Self::percentile(&sorted_durations, 99);

        SampleSummary {
            sampler_name: self.name.clone(),
            total_samples: samples.total_count(),
            recorded_samples: count,
            avg_duration,
            min_duration,
            max_duration,
            p50,
            p95,
            p99,
        }
    }

    /// Get all samples
    pub fn get_samples(&self) -> Vec<Sample> {
        self.samples.lock().samples()
    }

    /// Clear all samples
    pub fn clear(&self) {
        self.samples.lock().clear();
    }

    /// Set sample rate
    pub fn set_sample_rate(&mut self, rate: u32) {
        self.sample_rate = rate.max(1);
    }

    /// Get sample rate
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Calculate percentile
    fn percentile(sorted_data: &[u64], percentile: u64) -> u64 {
        if sorted_data.is_empty() {
            return 0;
        }
        let index = (sorted_data.len() as u64 * percentile) / 100;
        let index = index.min(sorted_data.len() as u64 - 1) as usize;
        sorted_data[index]
    }

    /// Get current CPU ID (placeholder)
    #[inline]
    fn current_cpu_id(&self) -> u32 {
        // TODO: Implement actual CPU ID retrieval
        0
    }

    /// Get current thread ID (placeholder)
    #[inline]
    fn current_thread_id(&self) -> u64 {
        // TODO: Implement actual thread ID retrieval
        0
    }

    /// Get current thread priority (placeholder)
    #[inline]
    fn current_priority(&self) -> u8 {
        // TODO: Implement actual priority retrieval
        0
    }
}

/// Sample summary statistics
#[derive(Debug, Clone)]
pub struct SampleSummary {
    /// Sampler name
    pub sampler_name: String,
    /// Total number of calls
    pub total_samples: u64,
    /// Number of recorded samples
    pub recorded_samples: usize,
    /// Average duration in nanoseconds
    pub avg_duration: u64,
    /// Minimum duration in nanoseconds
    pub min_duration: u64,
    /// Maximum duration in nanoseconds
    pub max_duration: u64,
    /// 50th percentile (median) in nanoseconds
    pub p50: u64,
    /// 95th percentile in nanoseconds
    pub p95: u64,
    /// 99th percentile in nanoseconds
    pub p99: u64,
}

impl SampleSummary {
    /// Create an empty summary
    fn empty() -> Self {
        Self {
            sampler_name: String::new(),
            total_samples: 0,
            recorded_samples: 0,
            avg_duration: 0,
            min_duration: 0,
            max_duration: 0,
            p50: 0,
            p95: 0,
            p99: 0,
        }
    }

    /// Format summary as string
    pub fn format(&self) -> String {
        format!(
            "Sampler: {}\n\
             Total calls: {}\n\
             Recorded samples: {}\n\
             Duration (ns):\n\
             - Avg: {}\n\
             - Min: {}\n\
             - Max: {}\n\
             - P50: {}\n\
             - P95: {}\n\
             - P99: {}",
            self.sampler_name,
            self.total_samples,
            self.recorded_samples,
            self.avg_duration,
            self.min_duration,
            self.max_duration,
            self.p50,
            self.p95,
            self.p99
        )
    }
}

/// Global sampler registry
static SAMPLER_REGISTRY: Mutex<BTreeMap<String, &'static PerformanceSampler>> = Mutex::new(BTreeMap::new());

/// Register a sampler
pub fn register_sampler(name: &str, sampler: &'static PerformanceSampler) {
    let mut registry = SAMPLER_REGISTRY.lock();
    registry.insert(String::from(name), sampler);
}

/// Get a sampler by name
pub fn get_sampler(name: &str) -> Option<&'static PerformanceSampler> {
    let registry = SAMPLER_REGISTRY.lock();
    registry.get(name).copied()
}

/// Get all sampler summaries
pub fn get_all_summaries() -> Vec<SampleSummary> {
    let registry = SAMPLER_REGISTRY.lock();
    registry.values().map(|s| s.summary()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sampler_creation() {
        let sampler = PerformanceSampler::new("test", 10, 100);
        assert_eq!(sampler.name, "test");
        assert_eq!(sampler.sample_rate(), 10);
        assert!(sampler.is_enabled());
    }

    #[test]
    fn test_sampler_disable_enable() {
        let sampler = PerformanceSampler::new("test", 1, 100);
        assert!(sampler.is_enabled());

        sampler.disable();
        assert!(!sampler.is_enabled());

        sampler.enable();
        assert!(sampler.is_enabled());
    }

    #[test]
    fn test_sampler_summary() {
        let sampler = PerformanceSampler::new("test", 1, 100);

        // Sample some operations
        for _ in 0..10 {
            sampler.sample("test_call", || {
                // Simulate some work
                let _ = 1 + 1;
            });
        }

        let summary = sampler.summary();
        assert_eq!(summary.sampler_name, "test");
        assert!(summary.recorded_samples > 0);
    }

    #[test]
    fn test_sample_buffer() {
        let buffer = SampleBuffer::new(3);

        assert_eq!(buffer.total_count(), 0);

        // Test circular behavior
        let mut buffer = buffer;
        buffer.push(Sample {
            timestamp: 1,
            duration_ns: 100,
            call_site: "test1",
            metadata: SampleMetadata { cpu_id: 0, thread_id: 0, priority: 0 },
        });
        buffer.push(Sample {
            timestamp: 2,
            duration_ns: 200,
            call_site: "test2",
            metadata: SampleMetadata { cpu_id: 0, thread_id: 0, priority: 0 },
        });
        buffer.push(Sample {
            timestamp: 3,
            duration_ns: 300,
            call_site: "test3",
            metadata: SampleMetadata { cpu_id: 0, thread_id: 0, priority: 0 },
        });
        buffer.push(Sample {
            timestamp: 4,
            duration_ns: 400,
            call_site: "test4",
            metadata: SampleMetadata { cpu_id: 0, thread_id: 0, priority: 0 },
        });

        let samples = buffer.samples();
        assert_eq!(samples.len(), 3);
        assert_eq!(buffer.total_count(), 4);
    }
}
