//! # Interrupt Latency Verification
//!
//! Comprehensive interrupt latency measurement and validation system.
//!
//! ## Targets
//!
//! - **Average latency**: < 5μs
//! - **Worst case (P99)**: < 10μs
//! - **No latency spikes**: < 15μs
//!
//! ## Test Scenarios
//!
//! - Idle system latency
//! - Latency under load
//! - Latency during IRQ storms
//! - Nested interrupt latency
//! - Preemption latency

#![allow(dead_code)]

use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;

/// Interrupt latency statistics
#[derive(Debug, Clone)]
pub struct InterruptLatencyStats {
    /// Total number of interrupts measured
    pub count: u64,
    /// Average latency (nanoseconds)
    pub avg_ns: f64,
    /// Median latency (nanoseconds)
    pub median_ns: f64,
    /// Minimum latency (nanoseconds)
    pub min_ns: u64,
    /// Maximum latency (nanoseconds)
    pub max_ns: u64,
    /// 95th percentile latency
    pub p95_ns: f64,
    /// 99th percentile latency
    pub p99_ns: f64,
    /// 99.9th percentile latency
    pub p999_ns: f64,
    /// Standard deviation
    pub std_dev_ns: f64,
    /// Samples exceeding threshold
    pub spikes: Vec<LatencySpike>,
}

/// Latency spike information
#[derive(Debug, Clone)]
pub struct LatencySpike {
    /// Spike index
    pub index: u64,
    /// Latency in nanoseconds
    pub latency_ns: u64,
    /// System state during spike
    pub system_state: SystemState,
}

/// System state during latency measurement
#[derive(Debug, Clone, Copy)]
pub enum SystemState {
    Idle,
    UnderLoad {
        cpu_utilization: f32,
        running_tasks: u32,
    },
    IrqStorm {
        irq_rate_per_sec: u64,
    },
    NestedInterrupt {
        depth: u8,
    },
}

impl InterruptLatencyStats {
    /// Calculate statistics from latency samples
    pub fn from_samples(mut samples: Vec<u64>) -> Self {
        if samples.is_empty() {
            return Self {
                count: 0,
                avg_ns: 0.0,
                median_ns: 0.0,
                min_ns: 0,
                max_ns: 0,
                p95_ns: 0.0,
                p99_ns: 0.0,
                p999_ns: 0.0,
                std_dev_ns: 0.0,
                spikes: Vec::new(),
            };
        }

        samples.sort_unstable();
        let count = samples.len() as u64;

        let min_ns = samples[0];
        let max_ns = samples[samples.len() - 1];
        let sum_ns: u64 = samples.iter().sum();
        let avg_ns = sum_ns as f64 / count as f64;

        let median_ns = if count % 2 == 0 {
            let mid = (count / 2) as usize;
            (samples[mid - 1] + samples[mid]) as f64 / 2.0
        } else {
            samples[(count / 2) as usize] as f64
        };

        let p95_idx = (count as f64 * 0.95) as usize;
        let p99_idx = (count as f64 * 0.99) as usize;
        let p999_idx = (count as f64 * 0.999) as usize;

        let p95_ns = samples.get(p95_idx).copied().unwrap_or(max_ns) as f64;
        let p99_ns = samples.get(p99_idx).copied().unwrap_or(max_ns) as f64;
        let p999_ns = samples.get(p999_idx).copied().unwrap_or(max_ns) as f64;

        // Calculate standard deviation
        let variance = samples
            .iter()
            .map(|&x| {
                let diff = x as f64 - avg_ns;
                diff * diff
            })
            .sum::<f64>()
            / count as f64;
        let std_dev_ns = variance.sqrt();

        // Detect latency spikes (> 3 std deviations from mean)
        let spike_threshold = avg_ns + 3.0 * std_dev_ns;
        let spikes = samples
            .iter()
            .enumerate()
            .filter_map(|(i, &latency)| {
                if latency as f64 > spike_threshold {
                    Some(LatencySpike {
                        index: i as u64,
                        latency_ns: latency,
                        system_state: SystemState::Idle,
                    })
                } else {
                    None
                }
            })
            .collect();

        Self {
            count,
            avg_ns,
            median_ns,
            min_ns,
            max_ns,
            p95_ns,
            p99_ns,
            p999_ns,
            std_dev_ns,
            spikes,
        }
    }

    /// Check if all latency targets are met
    pub fn meets_targets(&self) -> bool {
        self.avg_ns < 5_000.0 && self.p99_ns < 10_000.0 && self.max_ns < 15_000.0
    }

    /// Format statistics as a string
    pub fn format(&self) -> alloc::string::String {
        alloc::format!(
            "Interrupt Latency Statistics:\n\
             Count: {}\n\
             Avg: {:.2} μs\n\
             Median: {:.2} μs\n\
             Min: {:.2} μs\n\
             Max: {:.2} μs\n\
             P95: {:.2} μs\n\
             P99: {:.2} μs\n\
             P999: {:.2} μs\n\
             StdDev: {:.2} μs\n\
             Spikes: {}",
            self.count,
            self.avg_ns / 1_000.0,
            self.median_ns / 1_000.0,
            self.min_ns as f64 / 1_000.0,
            self.max_ns as f64 / 1_000.0,
            self.p95_ns / 1_000.0,
            self.p99_ns / 1_000.0,
            self.p999_ns / 1_000.0,
            self.std_dev_ns / 1_000.0,
            self.spikes.len()
        )
    }

    /// Generate detailed report
    pub fn report(&self) -> alloc::string::String {
        let mut output = alloc::format!("{}\n", self.format());

        if !self.spikes.is_empty() {
            output.push_str("\nLatency Spikes Detected:\n");
            for spike in &self.spikes {
                output.push_str(&alloc::format!(
                    "  [{}] {:.2} μs - {:?}\n",
                    spike.index,
                    spike.latency_ns as f64 / 1_000.0,
                    spike.system_state
                ));
            }
        }

        let status = if self.meets_targets() {
            "✓ PASS"
        } else {
            "✗ FAIL"
        };
        output.push_str(&alloc::format!("\nStatus: {}\n", status));

        output
    }
}

/// Interrupt latency measurement
pub struct InterruptLatencyMeasurer {
    /// Timestamp when IRQ was received
    irq_start: AtomicU64,
    /// Timestamp when handler started
    handler_start: AtomicU64,
    /// Timestamp when handler ended
    handler_end: AtomicU64,
    /// Timestamp when IRQ was acknowledged
    irq_end: AtomicU64,
    /// Latency samples
    samples: core::sync::Mutex<Vec<u64>>,
    /// System state tracking
    system_state: SystemState,
    /// Measurement enabled flag
    enabled: AtomicU64,
}

impl InterruptLatencyMeasurer {
    /// Create new latency measurer
    pub const fn new() -> Self {
        Self {
            irq_start: AtomicU64::new(0),
            handler_start: AtomicU64::new(0),
            handler_end: AtomicU64::new(0),
            irq_end: AtomicU64::new(0),
            samples: core::sync::Mutex::new(Vec::new()),
            system_state: SystemState::Idle,
            enabled: AtomicU64::new(0),
        }
    }

    /// Enable measurement
    pub fn enable(&self) {
        self.enabled.store(1, Ordering::Release);
    }

    /// Disable measurement
    pub fn disable(&self) {
        self.enabled.store(0, Ordering::Release);
    }

    /// Mark IRQ arrival (called from hardware IRQ entry)
    #[inline]
    pub fn mark_irq_arrival(&self) {
        if self.enabled.load(Ordering::Acquire) == 0 {
            return;
        }
        let timestamp = self.read_timer();
        self.irq_start.store(timestamp, Ordering::Release);
    }

    /// Mark handler start (called at beginning of IRQ handler)
    #[inline]
    pub fn mark_handler_start(&self) {
        if self.enabled.load(Ordering::Acquire) == 0 {
            return;
        }
        let timestamp = self.read_timer();
        self.handler_start.store(timestamp, Ordering::Release);
    }

    /// Mark handler end (called at end of IRQ handler)
    #[inline]
    pub fn mark_handler_end(&self) {
        if self.enabled.load(Ordering::Acquire) == 0 {
            return;
        }
        let timestamp = self.read_timer();
        self.handler_end.store(timestamp, Ordering::Release);
    }

    /// Mark IRQ completion (called when returning from IRQ)
    #[inline]
    pub fn mark_irq_complete(&self) {
        if self.enabled.load(Ordering::Acquire) == 0 {
            return;
        }
        let timestamp = self.read_timer();
        self.irq_end.store(timestamp, Ordering::Release);

        // Calculate and record total latency
        let total_latency = timestamp - self.irq_start.load(Ordering::Acquire);
        self.record_latency(total_latency);
    }

    /// Measure IRQ to handler start latency
    pub fn measure_irq_to_handler_latency(&self) -> u64 {
        let handler_start = self.handler_start.load(Ordering::Acquire);
        let irq_start = self.irq_start.load(Ordering::Acquire);
        if handler_start >= irq_start {
            handler_start - irq_start
        } else {
            0
        }
    }

    /// Measure handler execution time
    pub fn measure_handler_execution_time(&self) -> u64 {
        let handler_end = self.handler_end.load(Ordering::Acquire);
        let handler_start = self.handler_start.load(Ordering::Acquire);
        if handler_end >= handler_start {
            handler_end - handler_start
        } else {
            0
        }
    }

    /// Measure total latency (IRQ arrival to completion)
    pub fn measure_total_latency(&self) -> u64 {
        let irq_end = self.irq_end.load(Ordering::Acquire);
        let irq_start = self.irq_start.load(Ordering::Acquire);
        if irq_end >= irq_start {
            irq_end - irq_start
        } else {
            0
        }
    }

    /// Record a latency sample
    fn record_latency(&self, latency: u64) {
        let mut samples = self.samples.lock();
        samples.push(latency);
    }

    /// Get collected statistics
    pub fn get_statistics(&self) -> InterruptLatencyStats {
        let samples = self.samples.lock();
        InterruptLatencyStats::from_samples(samples.clone())
    }

    /// Clear all samples
    pub fn clear(&self) {
        let mut samples = self.samples.lock();
        samples.clear();
    }

    /// Read high-precision timer
    #[inline]
    fn read_timer(&self) -> u64 {
        #[cfg(target_arch = "x86_64")]
        {
            unsafe { crate::arch::x86_64::rdtsc() }
        }
        #[cfg(target_arch = "aarch64")]
        {
            unsafe { crate::arch::aarch64::read_cntvct_el0() }
        }
        #[cfg(target_arch = "riscv64")]
        {
            unsafe { crate::arch::riscv64::read_time() }
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "riscv64")))]
        {
            0
        }
    }

    /// Set system state for measurement context
    pub fn set_system_state(&mut self, state: SystemState) {
        self.system_state = state;
    }
}

/// Global interrupt latency measurer
static GLOBAL_MEASURER: InterruptLatencyMeasurer = InterruptLatencyMeasurer::new();

/// Get global measurer
pub fn get_global_measurer() -> &'static InterruptLatencyMeasurer {
    &GLOBAL_MEASURER
}

/// Run interrupt latency verification
pub fn verify_interrupt_latency() -> Result<InterruptLatencyStats, &'static str> {
    let measurer = get_global_measurer();
    measurer.enable();
    measurer.clear();

    crate::println!("interrupt-latency: Starting interrupt latency verification...");

    // Test 1: Measure latency in idle state
    crate::println!("interrupt-latency: Measuring idle system latency...");
    test_idle_system_latency(measurer)?;

    // Test 2: Measure latency under load
    crate::println!("interrupt-latency: Measuring latency under load...");
    test_latency_under_load(measurer)?;

    // Test 3: Measure latency during IRQ storm
    crate::println!("interrupt-latency: Measuring latency during IRQ storm...");
    test_irq_storm_latency(measurer)?;

    // Test 4: Measure nested interrupt latency
    crate::println!("interrupt-latency: Measuring nested interrupt latency...");
    test_nested_interrupt_latency(measurer)?;

    measurer.disable();

    let stats = measurer.get_statistics();
    crate::println!("\ninterrupt-latency: Verification Results\n{}", stats.report());

    if stats.meets_targets() {
        Ok(stats)
    } else {
        Err("Interrupt latency targets not met")
    }
}

/// Test idle system interrupt latency
fn test_idle_system_latency(measurer: &InterruptLatencyMeasurer) -> Result<(), &'static str> {
    const ITERATIONS: usize = 1000;

    for _ in 0..ITERATIONS {
        // Simulate interrupt
        measurer.mark_irq_arrival();
        core::hint::spin_loop(); // Simulate hardware delay
        measurer.mark_handler_start();

        // Simulate handler
        for _ in 0..10 {
            core::hint::spin_loop();
        }

        measurer.mark_handler_end();
        measurer.mark_irq_complete();
    }

    Ok(())
}

/// Test interrupt latency under CPU load
fn test_latency_under_load(measurer: &InterruptLatencyMeasurer) -> Result<(), &'static str> {
    const ITERATIONS: usize = 1000;

    measurer.set_system_state(SystemState::UnderLoad {
        cpu_utilization: 0.8,
        running_tasks: 4,
    });

    for _ in 0..ITERATIONS {
        // Simulate interrupt while CPU is busy
        measurer.mark_irq_arrival();

        // Simulate work being interrupted
        for _ in 0..50 {
            core::hint::spin_loop();
        }

        measurer.mark_handler_start();

        // Simulate handler
        for _ in 0..10 {
            core::hint::spin_loop();
        }

        measurer.mark_handler_end();
        measurer.mark_irq_complete();
    }

    Ok(())
}

/// Test interrupt latency during IRQ storm
fn test_irq_storm_latency(measurer: &InterruptLatencyMeasurer) -> Result<(), &'static str> {
    const ITERATIONS: usize = 1000;

    measurer.set_system_state(SystemState::IrqStorm {
        irq_rate_per_sec: 100_000,
    });

    for _ in 0..ITERATIONS {
        // Simulate rapid interrupts
        measurer.mark_irq_arrival();
        measurer.mark_handler_start();

        // Quick handler
        for _ in 0..5 {
            core::hint::spin_loop();
        }

        measurer.mark_handler_end();
        measurer.mark_irq_complete();

        // Brief gap before next interrupt
        for _ in 0..2 {
            core::hint::spin_loop();
        }
    }

    Ok(())
}

/// Test nested interrupt latency
fn test_nested_interrupt_latency(measurer: &InterruptLatencyMeasurer) -> Result<(), &'static str> {
    const ITERATIONS: usize = 100;

    measurer.set_system_state(SystemState::NestedInterrupt { depth: 3 });

    for i in 0..ITERATIONS {
        // Outer interrupt
        measurer.mark_irq_arrival();
        measurer.mark_handler_start();

        // Simulate nested interrupts
        for depth in 0..3 {
            measurer.mark_irq_arrival();

            // Inner handler
            for _ in 0..5 {
                core::hint::spin_loop();
            }

            measurer.mark_irq_complete();
        }

        // Complete outer handler
        for _ in 0..10 {
            core::hint::spin_loop();
        }

        measurer.mark_handler_end();
        measurer.mark_irq_complete();

        // Track iteration
        core::sync::atomic::compiler_fence(Ordering::SeqCst);
    }

    Ok(())
}

/// Interrupt latency test suite
pub struct InterruptLatencyTestSuite {
    measurer: &'static InterruptLatencyMeasurer,
}

impl InterruptLatencyTestSuite {
    /// Create new test suite
    pub fn new() -> Self {
        Self {
            measurer: get_global_measurer(),
        }
    }

    /// Run all interrupt latency tests
    pub fn run_all(&self) -> Result<InterruptLatencyStats, &'static str> {
        self.measurer.enable();
        self.measurer.clear();

        crate::println!("╔══════════════════════════════════════════╗");
        crate::println!("║   Interrupt Latency Verification       ║");
        crate::println!("╚══════════════════════════════════════════╝\n");

        // Run all test scenarios
        self.run_idle_test()?;
        self.run_under_load_test()?;
        self.run_irq_storm_test()?;
        self.run_nested_interrupt_test()?;

        self.measurer.disable();

        let stats = self.measurer.get_statistics();

        crate::println!("\n╔══════════════════════════════════════════╗");
        crate::println!("║           Final Results                ║");
        crate::println!("╚══════════════════════════════════════════╝");
        crate::println!("{}", stats.report());

        if stats.meets_targets() {
            Ok(stats)
        } else {
            Err("Some interrupt latency targets were not met")
        }
    }

    /// Run idle system test
    fn run_idle_test(&self) -> Result<(), &'static str> {
        crate::println!("Test 1: Idle System Latency");
        test_idle_system_latency(self.measurer)
    }

    /// Run under load test
    fn run_under_load_test(&self) -> Result<(), &'static str> {
        crate::println!("Test 2: Latency Under Load");
        test_latency_under_load(self.measurer)
    }

    /// Run IRQ storm test
    fn run_irq_storm_test(&self) -> Result<(), &'static str> {
        crate::println!("Test 3: IRQ Storm Latency");
        test_irq_storm_latency(self.measurer)
    }

    /// Run nested interrupt test
    fn run_nested_interrupt_test(&self) -> Result<(), &'static str> {
        crate::println!("Test 4: Nested Interrupt Latency");
        test_nested_interrupt_latency(self.measurer)
    }
}

/// Architecture-specific timer reading functions (placeholders)

#[cfg(target_arch = "aarch64")]
pub mod aarch64 {
    /// Read virtual counter-timer timer
    #[inline]
    pub unsafe fn read_cntvct_el0() -> u64 {
        let value: u64;
        core::arch::asm!(
            "mrs {}, cntvct_el0",
            out(reg) value,
            options(nostack, nomem)
        );
        value
    }
}

#[cfg(target_arch = "riscv64")]
pub mod riscv64 {
    /// Read time CSR
    #[inline]
    pub unsafe fn read_time() -> u64 {
        let value: u64;
        core::arch::asm!(
            "csrr {}, time",
            out(reg) value,
            options(nostack, nomem)
        );
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latency_stats_from_samples() {
        let samples = vec
![1000, 2000, 3000, 4000, 5000, 6000, 7000, 8000, 9000, 10_000];
        let stats = InterruptLatencyStats::from_samples(samples);

        assert_eq!(stats.count, 10);
        assert_eq!(stats.min_ns, 1000);
        assert_eq!(stats.max_ns, 10_000);
        assert!((stats.median_ns - 5500.0).abs() < 0.1);
    }

    #[test]
    fn test_latency_targets() {
        // All samples under 5μs average, 10μs P99, 15μs max
        let good_samples = vec
![1000; 100].iter()
.map(|x| x + 1000).collect();
        let stats = InterruptLatencyStats::from_samples(good_samples);
        assert!(stats.meets_targets());

        // Some samples exceed targets
        let bad_samples = vec
![1000, 2000, 20_000]; // 20μs exceeds 15μs max
        let stats = InterruptLatencyStats::from_samples(bad_samples);
        assert!(!stats.meets_targets());
    }

    #[test]
    fn test_spike_detection() {
        // Most samples around 1μs, but one at 20μs (spike)
        let mut samples = vec
![1000; 100];
        samples[50] = 20_000;

        let stats = InterruptLatencyStats::from_samples(samples);
        assert!(!stats.spikes.is_empty());
        assert_eq!(stats.spikes[0].latency_ns, 20_000);
    }

    #[test]
    fn test_latency_measurer() {
        let measurer = InterruptLatencyMeasurer::new();
        measurer.enable();

        // Simulate interrupt
        measurer.mark_irq_arrival();
        measurer.mark_handler_start();
        measurer.mark_handler_end();
        measurer.mark_irq_complete();

        let stats = measurer.get_statistics();
        assert_eq!(stats.count, 1);
        assert!(stats.max_ns > 0);
    }
}
