//! CPU Frequency Scaling (CPUFreq)
//!
//! This module provides dynamic CPU frequency scaling support with multiple
//! governor strategies for balancing performance and power consumption.

use core::sync::atomic::{AtomicU8, Ordering};
use core::time::Duration;

use alloc::sync::Arc;
use alloc::string::String;
use alloc::vec::Vec;
use spin::Mutex;

/// CPU frequency governor base trait
pub trait CpufreqGovernorTrait: Send + Sync {
    /// Get governor name
    fn name(&self) -> &str;

    /// Get minimum frequency (Hz)
    fn min_freq(&self) -> u64;

    /// Get maximum frequency (Hz)
    fn max_freq(&self) -> u64;

    /// Get current frequency (Hz)
    fn current_freq(&self) -> u64;

    /// Adjust frequency based on CPU load (0-100)
    fn adjust(&self, load: u8);

    /// Set target frequency
    fn set_frequency(&self, freq: u64);
}

/// Base CPU frequency governor implementation
pub struct CpufreqGovernor {
    name: String,
    min_freq: u64,
    max_freq: u64,
    transition_latency: Duration,
    current_freq: spin::Mutex<u64>,
    target_freq: spin::Mutex<u64>,
}

impl CpufreqGovernor {
    /// Create a new CPUFreq governor
    ///
    /// # Arguments
    ///
    /// * `name` - Governor name
    /// * `min_freq` - Minimum frequency in Hz
    /// * `max_freq` - Maximum frequency in Hz
    /// * `transition_latency` - Time required for frequency transition
    pub fn new(name: &str, min_freq: u64, max_freq: u64, transition_latency: Duration) -> Self {
        Self {
            name: String::from(name),
            min_freq,
            max_freq,
            transition_latency,
            current_freq: spin::Mutex::new(min_freq),
            target_freq: spin::Mutex::new(min_freq),
        }
    }

    /// Calculate target frequency based on load
    ///
    /// Uses a linear scaling algorithm between min and max frequency
    /// based on the CPU load percentage (0-100).
    fn calculate_target_freq(&self, load: u8) -> u64 {
        if load >= 100 {
            return self.max_freq;
        }

        let range = self.max_freq - self.min_freq;
        let scaled = (range * load as u64) / 100;
        self.min_freq + scaled
    }

    /// Validate frequency is within bounds
    fn validate_frequency(&self, freq: u64) -> u64 {
        freq.clamp(self.min_freq, self.max_freq)
    }

    /// Get current frequency
    pub fn get_current_frequency(&self) -> u64 {
        *self.current_freq.lock()
    }

    /// Get target frequency
    pub fn get_target_frequency(&self) -> u64 {
        *self.target_freq.lock()
    }

    /// Get transition latency
    pub fn transition_latency(&self) -> Duration {
        self.transition_latency
    }

    /// Set frequency (internal implementation)
    fn set_frequency_internal(&self, freq: u64) {
        let validated = self.validate_frequency(freq);
        *self.target_freq.lock() = validated;

        // Simulate hardware frequency transition
        // In real implementation, this would call into CPU-specific code
        *self.current_freq.lock() = validated;
    }
}

impl CpufreqGovernorTrait for CpufreqGovernor {
    fn name(&self) -> &str {
        &self.name
    }

    fn min_freq(&self) -> u64 {
        self.min_freq
    }

    fn max_freq(&self) -> u64 {
        self.max_freq
    }

    fn current_freq(&self) -> u64 {
        self.get_current_frequency()
    }

    fn adjust(&self, load: u8) {
        let target = self.calculate_target_freq(load);
        self.set_frequency_internal(target);
    }

    fn set_frequency(&self, freq: u64) {
        self.set_frequency_internal(freq);
    }
}

/// Performance Governor
///
/// Always runs at maximum frequency for best performance.
/// This governor does not scale down frequency.
pub struct PerformanceGovernor {
    inner: Arc<CpufreqGovernor>,
}

impl PerformanceGovernor {
    /// Create a new performance governor
    ///
    /// # Arguments
    ///
    /// * `max_freq` - Maximum frequency in Hz
    pub fn new(max_freq: u64) -> Self {
        Self {
            inner: Arc::new(CpufreqGovernor::new(
                "performance",
                max_freq,
                max_freq,
                Duration::from_micros(10),
            )),
        }
    }

    /// Get inner governor
    pub fn inner(&self) -> &Arc<CpufreqGovernor> {
        &self.inner
    }
}

impl CpufreqGovernorTrait for PerformanceGovernor {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn min_freq(&self) -> u64 {
        self.inner.min_freq()
    }

    fn max_freq(&self) -> u64 {
        self.inner.max_freq()
    }

    fn current_freq(&self) -> u64 {
        self.inner.current_freq()
    }

    fn adjust(&self, _load: u8) {
        // Always run at max frequency
        self.set_frequency(self.inner.max_freq());
    }

    fn set_frequency(&self, freq: u64) {
        self.inner.set_frequency(freq);
    }
}

/// Powersave Governor
///
/// Always runs at minimum frequency for best power efficiency.
/// This governor does not scale up frequency.
pub struct PowersaveGovernor {
    inner: Arc<CpufreqGovernor>,
}

impl PowersaveGovernor {
    /// Create a new powersave governor
    ///
    /// # Arguments
    ///
    /// * `min_freq` - Minimum frequency in Hz
    pub fn new(min_freq: u64) -> Self {
        Self {
            inner: Arc::new(CpufreqGovernor::new(
                "powersave",
                min_freq,
                min_freq,
                Duration::from_micros(10),
            )),
        }
    }

    /// Get inner governor
    pub fn inner(&self) -> &Arc<CpufreqGovernor> {
        &self.inner
    }
}

impl CpufreqGovernorTrait for PowersaveGovernor {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn min_freq(&self) -> u64 {
        self.inner.min_freq()
    }

    fn max_freq(&self) -> u64 {
        self.inner.max_freq()
    }

    fn current_freq(&self) -> u64 {
        self.inner.current_freq()
    }

    fn adjust(&self, _load: u8) {
        // Always run at min frequency
        self.set_frequency(self.inner.min_freq());
    }

    fn set_frequency(&self, freq: u64) {
        self.inner.set_frequency(freq);
    }
}

/// Ondemand Governor
///
/// Dynamically adjusts frequency based on CPU load.
/// - Increases frequency rapidly when load is high
/// - Decreases frequency gradually when load is low
pub struct OndemandGovernor {
    inner: Arc<CpufreqGovernor>,
    up_threshold: AtomicU8,
    down_differential: AtomicU8,
    sampling_rate: Duration,
}

impl OndemandGovernor {
    /// Create a new ondemand governor
    ///
    /// # Arguments
    ///
    /// * `min_freq` - Minimum frequency in Hz
    /// * `max_freq` - Maximum frequency in Hz
    /// * `transition_latency` - Time for frequency transition
    pub fn new(min_freq: u64, max_freq: u64, transition_latency: Duration) -> Self {
        Self {
            inner: Arc::new(CpufreqGovernor::new(
                "ondemand",
                min_freq,
                max_freq,
                transition_latency,
            )),
            up_threshold: AtomicU8::new(80),  // Default 80% threshold
            down_differential: AtomicU8::new(10), // 10% below target
            sampling_rate: Duration::from_millis(10),
        }
    }

    /// Set the load threshold for increasing frequency
    pub fn set_up_threshold(&self, threshold: u8) {
        self.up_threshold.store(threshold.clamp(1, 100), Ordering::Relaxed);
    }

    /// Set the differential for decreasing frequency
    pub fn set_down_differential(&self, diff: u8) {
        self.down_differential.store(diff.clamp(1, 100), Ordering::Relaxed);
    }

    /// Get inner governor
    pub fn inner(&self) -> &Arc<CpufreqGovernor> {
        &self.inner
    }

    /// Calculate target frequency with ondemand algorithm
    fn calculate_ondemand_freq(&self, load: u8) -> u64 {
        let up_threshold = self.up_threshold.load(Ordering::Relaxed);
        let _current = self.inner.current_freq();

        if load >= up_threshold {
            // High load: jump to max frequency immediately
            self.inner.max_freq()
        } else {
            // Low load: scale down gradually
            let _target_load = up_threshold.saturating_sub(self.down_differential.load(Ordering::Relaxed));
            let scaled = (self.inner.max_freq() * load as u64) / 100;
            scaled.max(self.inner.min_freq())
        }
    }
}

impl CpufreqGovernorTrait for OndemandGovernor {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn min_freq(&self) -> u64 {
        self.inner.min_freq()
    }

    fn max_freq(&self) -> u64 {
        self.inner.max_freq()
    }

    fn current_freq(&self) -> u64 {
        self.inner.current_freq()
    }

    fn adjust(&self, load: u8) {
        let target = self.calculate_ondemand_freq(load);
        self.inner.set_frequency(target);
    }

    fn set_frequency(&self, freq: u64) {
        self.inner.set_frequency(freq);
    }
}

/// Conservative Governor
///
/// Similar to ondemand but more gradual in frequency changes.
/// Better for battery life at the cost of responsiveness.
pub struct ConservativeGovernor {
    inner: Arc<CpufreqGovernor>,
    up_threshold: AtomicU8,
    down_threshold: AtomicU8,
    step: AtomicU8,
    sampling_rate: Duration,
}

impl ConservativeGovernor {
    /// Create a new conservative governor
    ///
    /// # Arguments
    ///
    /// * `min_freq` - Minimum frequency in Hz
    /// * `max_freq` - Maximum frequency in Hz
    /// * `transition_latency` - Time for frequency transition
    pub fn new(min_freq: u64, max_freq: u64, transition_latency: Duration) -> Self {
        Self {
            inner: Arc::new(CpufreqGovernor::new(
                "conservative",
                min_freq,
                max_freq,
                transition_latency,
            )),
            up_threshold: AtomicU8::new(80),
            down_threshold: AtomicU8::new(20),
            step: AtomicU8::new(5), // 5% frequency steps
            sampling_rate: Duration::from_millis(20),
        }
    }

    /// Set up threshold
    pub fn set_up_threshold(&self, threshold: u8) {
        self.up_threshold.store(threshold.clamp(1, 100), Ordering::Relaxed);
    }

    /// Set down threshold
    pub fn set_down_threshold(&self, threshold: u8) {
        self.down_threshold.store(threshold.clamp(1, 100), Ordering::Relaxed);
    }

    /// Set frequency step percentage
    pub fn set_step(&self, step: u8) {
        self.step.store(step.clamp(1, 100), Ordering::Relaxed);
    }

    /// Get inner governor
    pub fn inner(&self) -> &Arc<CpufreqGovernor> {
        &self.inner
    }

    /// Calculate target frequency with conservative algorithm
    fn calculate_conservative_freq(&self, load: u8) -> u64 {
        let up_threshold = self.up_threshold.load(Ordering::Relaxed);
        let down_threshold = self.down_threshold.load(Ordering::Relaxed);
        let current = self.inner.current_freq();
        let step = self.step.load(Ordering::Relaxed);

        let range = self.inner.max_freq() - self.inner.min_freq();
        let step_freq = (range * step as u64) / 100;

        if load >= up_threshold {
            // Increase frequency gradually
            current.saturating_add(step_freq).min(self.inner.max_freq())
        } else if load <= down_threshold {
            // Decrease frequency gradually
            current.saturating_sub(step_freq).max(self.inner.min_freq())
        } else {
            // Maintain current frequency
            current
        }
    }
}

impl CpufreqGovernorTrait for ConservativeGovernor {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn min_freq(&self) -> u64 {
        self.inner.min_freq()
    }

    fn max_freq(&self) -> u64 {
        self.inner.max_freq()
    }

    fn current_freq(&self) -> u64 {
        self.inner.current_freq()
    }

    fn adjust(&self, load: u8) {
        let target = self.calculate_conservative_freq(load);
        self.inner.set_frequency(target);
    }

    fn set_frequency(&self, freq: u64) {
        self.inner.set_frequency(freq);
    }
}

/// CPU load sample for monitoring
#[derive(Debug, Clone, Copy)]
pub struct LoadSample {
    /// CPU usage percentage (0-100)
    pub cpu_usage: u8,
    /// I/O wait percentage (0-100)
    pub io_wait: u8,
    /// System load percentage (0-100)
    pub system: u8,
    /// Timestamp of sample
    pub timestamp: u64,
}

impl LoadSample {
    /// Create a new load sample
    pub fn new(cpu_usage: u8, io_wait: u8, system: u8, timestamp: u64) -> Self {
        Self {
            cpu_usage,
            io_wait,
            system,
            timestamp,
        }
    }

    /// Create a zero load sample
    pub fn zero() -> Self {
        Self {
            cpu_usage: 0,
            io_wait: 0,
            system: 0,
            timestamp: 0,
        }
    }

    /// Get total load (max of all components)
    pub fn total_load(&self) -> u8 {
        self.cpu_usage.max(self.io_wait).max(self.system)
    }
}

/// Load monitor for tracking CPU usage
pub struct LoadMonitor {
    sample_interval: Duration,
    samples: Mutex<Vec<LoadSample>>,
    max_samples: usize,
}

impl LoadMonitor {
    /// Create a new load monitor
    ///
    /// # Arguments
    ///
    /// * `sample_interval` - Time between samples
    /// * `max_samples` - Maximum number of samples to keep
    pub fn new(sample_interval: Duration, max_samples: usize) -> Self {
        Self {
            sample_interval,
            samples: Mutex::new(Vec::with_capacity(max_samples)),
            max_samples,
        }
    }

    /// Sample current CPU load
    pub fn sample(&self) -> LoadSample {
        // In a real implementation, this would query the scheduler
        // for actual CPU usage statistics
        LoadSample::new(
            self.get_cpu_usage(),
            self.get_io_wait(),
            self.get_system_load(),
            self.get_timestamp(),
        )
    }

    /// Record a load sample
    pub fn record_sample(&self, sample: LoadSample) {
        let mut samples = self.samples.lock();
        samples.push(sample);

        // Keep only the most recent samples
        if samples.len() > self.max_samples {
            samples.remove(0);
        }
    }

    /// Calculate average load over the last N samples
    pub fn average_load(&self, window: usize) -> LoadSample {
        let samples = self.samples.lock();
        let window = window.min(samples.len());

        if window == 0 {
            return LoadSample::zero();
        }

        let start = samples.len().saturating_sub(window);
        let relevant: &[LoadSample] = &samples[start..];

        let count = relevant.len() as u64;
        let sum_cpu: u64 = relevant.iter().map(|s| s.cpu_usage as u64).sum();
        let sum_io: u64 = relevant.iter().map(|s| s.io_wait as u64).sum();
        let sum_sys: u64 = relevant.iter().map(|s| s.system as u64).sum();

        LoadSample::new(
            (sum_cpu / count) as u8,
            (sum_io / count) as u8,
            (sum_sys / count) as u8,
            relevant.last().map(|s| s.timestamp).unwrap_or(0),
        )
    }

    /// Get current CPU usage (stub implementation)
    fn get_cpu_usage(&self) -> u8 {
        // Stub: would query scheduler in real implementation
        50
    }

    /// Get current I/O wait (stub implementation)
    fn get_io_wait(&self) -> u8 {
        // Stub: would query scheduler in real implementation
        10
    }

    /// Get current system load (stub implementation)
    fn get_system_load(&self) -> u8 {
        // Stub: would query scheduler in real implementation
        30
    }

    /// Get current timestamp (stub implementation)
    fn get_timestamp(&self) -> u64 {
        // Stub: would get real time in implementation
        0
    }

    /// Get sample interval
    pub fn sample_interval(&self) -> Duration {
        self.sample_interval
    }

    /// Clear all samples
    pub fn clear(&self) {
        self.samples.lock().clear();
    }

    /// Get number of samples
    pub fn sample_count(&self) -> usize {
        self.samples.lock().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_performance_governor() {
        let governor = PerformanceGovernor::new(3_000_000);
        assert_eq!(governor.name(), "performance");
        assert_eq!(governor.min_freq(), 3_000_000);
        assert_eq!(governor.max_freq(), 3_000_000);

        governor.adjust(50);
        assert_eq!(governor.current_freq(), 3_000_000);
    }

    #[test]
    fn test_powersave_governor() {
        let governor = PowersaveGovernor::new(1_000_000);
        assert_eq!(governor.name(), "powersave");
        assert_eq!(governor.min_freq(), 1_000_000);
        assert_eq!(governor.max_freq(), 1_000_000);

        governor.adjust(90);
        assert_eq!(governor.current_freq(), 1_000_000);
    }

    #[test]
    fn test_ondemand_governor() {
        let governor = OndemandGovernor::new(1_000_000, 3_000_000, Duration::from_micros(10));

        // Low load
        governor.adjust(30);
        assert!(governor.current_freq() < 3_000_000);

        // High load
        governor.adjust(85);
        assert_eq!(governor.current_freq(), 3_000_000);
    }

    #[test]
    fn test_load_monitor() {
        let monitor = LoadMonitor::new(Duration::from_millis(100), 10);

        monitor.record_sample(LoadSample::new(50, 10, 20, 1000));
        monitor.record_sample(LoadSample::new(60, 15, 25, 1100));
        monitor.record_sample(LoadSample::new(70, 20, 30, 1200));

        assert_eq!(monitor.sample_count(), 3);

        let avg = monitor.average_load(3);
        assert_eq!(avg.cpu_usage, 60);
        assert_eq!(avg.io_wait, 15);
        assert_eq!(avg.system, 25);
    }

    #[test]
    fn test_load_sample_total_load() {
        let sample = LoadSample::new(80, 30, 50, 1000);
        assert_eq!(sample.total_load(), 80);
    }
}
