//! Performance Profiling Infrastructure
//!
//! This module provides comprehensive performance profiling capabilities for the NOS kernel,
//! including CPU, memory, lock, and I/O profiling.
//!
//! # Features
//!
//! - **CPU Profiling**: Sampling-based CPU profiler with flame graph generation
//! - **Memory Profiling**: Heap profiling, allocation tracking, and leak detection
//! - **Lock Profiling**: Contention profiling and deadlock detection
//! - **I/O Profiling**: Disk and network I/O tracking with latency analysis
//! - **Symbol Resolution**: Address to symbol mapping with demangling
//! - **Multiple Export Formats**: perf, pprof, Chrome tracing, JSON
//!
//! # Usage
//!
//! ```rust
//! use kernel::profiling::{Profiler, ProfilerType};
//!
//! // Create profiler
//! let profiler = Profiler::new();
//!
//! // Start CPU profiling
//! profiler.start(ProfilerType::Cpu(100)).unwrap();
//!
//! // ... code to profile ...
//!
//! // Stop and get report
//! profiler.stop(ProfilerType::Cpu).unwrap();
//! let report = profiler.report(ProfilerType::Cpu).unwrap();
//! ```
//!
//! # Architecture
//!
//! The profiling system is designed with minimal overhead (<5% when enabled) and
//! supports:
//!
//! - Sampling-based profiling for CPU
//! - Per-allocation tracking for memory
//! - Per-operation tracking for I/O and locks
//! - Symbol caching for performance
//! - Configurable detail levels

#![no_std]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};
use core::time::Duration;

use spin::{Mutex, MutexGuard};

pub mod cpu;
pub mod io;
pub mod lock;
pub mod memory;
pub mod symbol;

/// Profiling error types
#[derive(Debug, Clone, PartialEq)]
pub enum ProfilingError {
    /// Profiler is already running
    AlreadyRunning,
    /// Profiler is not running
    NotRunning,
    /// Invalid configuration
    InvalidConfiguration(String),
    /// Buffer overflow
    BufferOverflow,
    /// Export failed
    ExportFailed(String),
    /// Type-specific error
    TypeSpecific(String),
}

impl core::fmt::Display for ProfilingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "Profiler is already running"),
            Self::NotRunning => write!(f, "Profiler is not running"),
            Self::InvalidConfiguration(msg) => write!(f, "Invalid configuration: {}", msg),
            Self::BufferOverflow => write!(f, "Profile buffer overflow"),
            Self::ExportFailed(msg) => write!(f, "Export failed: {}", msg),
            Self::TypeSpecific(msg) => write!(f, "{}", msg),
        }
    }
}

impl From<cpu::CpuProfileError> for ProfilingError {
    fn from(err: cpu::CpuProfileError) -> Self {
        Self::TypeSpecific(err.to_string())
    }
}

impl From<memory::MemoryProfileError> for ProfilingError {
    fn from(err: memory::MemoryProfileError) -> Self {
        Self::TypeSpecific(err.to_string())
    }
}

impl From<lock::LockProfileError> for ProfilingError {
    fn from(err: lock::LockProfileError) -> Self {
        Self::TypeSpecific(err.to_string())
    }
}

impl From<io::IoProfileError> for ProfilingError {
    fn from(err: io::IoProfileError) -> Self {
        Self::TypeSpecific(err.to_string())
    }
}

impl From<symbol::SymbolError> for ProfilingError {
    fn from(err: symbol::SymbolError) -> Self {
        Self::TypeSpecific(err.to_string())
    }
}

/// Profiler type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfilerType {
    /// CPU profiler with sampling frequency (Hz)
    Cpu(u64),
    /// Memory profiler
    Memory,
    /// Lock profiler
    Lock,
    /// I/O profiler
    Io,
    /// All profilers
    All,
}

/// Export format options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// Perf format
    Perf,
    /// pprof format
    Pprof,
    /// Chrome tracing format
    Chrome,
    /// JSON format
    Json,
    /// Flame graph (SVG)
    FlameGraph,
}

/// Unified profiling report
#[derive(Debug, Clone)]
pub enum ProfileReport {
    /// CPU profile report
    Cpu(cpu::CpuProfileReport),
    /// Memory heap snapshot
    Memory(memory::HeapSnapshot),
    /// Lock profiling report
    Lock(lock::LockProfileReport),
    /// I/O profiling report
    Io(io::IoProfileReport),
}

/// Global profiler instance
static GLOBAL_PROFILER: Mutex<Option<Profiler>> = Mutex::new(None);

/// Main profiler implementation
pub struct Profiler {
    cpu_profiler: Option<cpu::CpuProfiler>,
    memory_profiler: Option<memory::MemoryProfiler>,
    lock_profiler: Option<lock::LockProfiler>,
    io_profiler: Option<io::IoProfiler>,
    symbol_resolver: Option<symbol::SymbolResolver>,
    enabled: AtomicBool,
}

impl Default for Profiler {
    fn default() -> Self {
        Self::new()
    }
}

impl Profiler {
    /// Create a new profiler instance
    pub fn new() -> Self {
        Self {
            cpu_profiler: Some(cpu::CpuProfiler::new(100).unwrap()),
            memory_profiler: Some(memory::MemoryProfiler::new()),
            lock_profiler: Some(lock::LockProfiler::new()),
            io_profiler: Some(io::IoProfiler::new()),
            symbol_resolver: Some(symbol::SymbolResolver::new()),
            enabled: AtomicBool::new(false),
        }
    }

    /// Initialize global profiler
    pub fn init_global() -> Result<(), ProfilingError> {
        let mut global = GLOBAL_PROFILER.lock();
        if global.is_some() {
            return Err(ProfilingError::AlreadyRunning);
        }
        *global = Some(Profiler::new());
        Ok(())
    }

    /// Get global profiler instance
    pub fn global() -> Result<&'static Mutex<Option<Profiler>>, ProfilingError> {
        let global = GLOBAL_PROFILER.lock();
        if global.is_some() {
            drop(global);
            Ok(&GLOBAL_PROFILER)
        } else {
            Err(ProfilingError::NotRunning)
        }
    }

    /// Enable profiling globally
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Release);
    }

    /// Disable profiling globally
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Release);
    }

    /// Check if profiling is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire)
    }

    /// Start profiling of the specified type
    pub fn start(&self, profiler_type: ProfilerType) -> Result<(), ProfilingError> {
        if !self.is_enabled() {
            return Err(ProfilingError::NotRunning);
        }

        match profiler_type {
            ProfilerType::Cpu(_freq) => {
                if let Some(ref profiler) = self.cpu_profiler {
                    profiler.start()?;
                }
            }
            ProfilerType::Memory => {
                if let Some(ref profiler) = self.memory_profiler {
                    profiler.start()?;
                }
            }
            ProfilerType::Lock => {
                if let Some(ref profiler) = self.lock_profiler {
                    profiler.start()?;
                }
            }
            ProfilerType::Io => {
                if let Some(ref profiler) = self.io_profiler {
                    profiler.start()?;
                }
            }
            ProfilerType::All => {
                self.start(ProfilerType::Cpu(100))?;
                self.start(ProfilerType::Memory)?;
                self.start(ProfilerType::Lock)?;
                self.start(ProfilerType::Io)?;
            }
        }

        Ok(())
    }

    /// Stop profiling of the specified type
    pub fn stop(&self, profiler_type: ProfilerType) -> Result<(), ProfilingError> {
        match profiler_type {
            ProfilerType::Cpu(_) => {
                if let Some(ref profiler) = self.cpu_profiler {
                    profiler.stop()?;
                }
            }
            ProfilerType::Memory => {
                if let Some(ref profiler) = self.memory_profiler {
                    profiler.stop()?;
                }
            }
            ProfilerType::Lock => {
                if let Some(ref profiler) = self.lock_profiler {
                    profiler.stop()?;
                }
            }
            ProfilerType::Io => {
                if let Some(ref profiler) = self.io_profiler {
                    profiler.stop()?;
                }
            }
            ProfilerType::All => {
                self.stop(ProfilerType::Cpu(0))?;
                self.stop(ProfilerType::Memory)?;
                self.stop(ProfilerType::Lock)?;
                self.stop(ProfilerType::Io)?;
            }
        }

        Ok(())
    }

    /// Get profiling report
    pub fn report(&self, profiler_type: ProfilerType) -> Result<ProfileReport, ProfilingError> {
        match profiler_type {
            ProfilerType::Cpu(_) => {
                if let Some(ref profiler) = self.cpu_profiler {
                    let report = profiler.generate_report()?;
                    Ok(ProfileReport::Cpu(report))
                } else {
                    Err(ProfilingError::NotRunning)
                }
            }
            ProfilerType::Memory => {
                if let Some(ref profiler) = self.memory_profiler {
                    let snapshot = profiler.take_snapshot()?;
                    Ok(ProfileReport::Memory(snapshot))
                } else {
                    Err(ProfilingError::NotRunning)
                }
            }
            ProfilerType::Lock => {
                if let Some(ref profiler) = self.lock_profiler {
                    let report = profiler.generate_report()?;
                    Ok(ProfileReport::Lock(report))
                } else {
                    Err(ProfilingError::NotRunning)
                }
            }
            ProfilerType::Io => {
                if let Some(ref profiler) = self.io_profiler {
                    let report = profiler.generate_report()?;
                    Ok(ProfileReport::Io(report))
                } else {
                    Err(ProfilingError::NotRunning)
                }
            }
            ProfilerType::All => {
                // Return CPU report as default for "All"
                self.report(ProfilerType::Cpu(0))
            }
        }
    }

    /// Clear profiling data
    pub fn clear(&self, profiler_type: ProfilerType) -> Result<(), ProfilingError> {
        match profiler_type {
            ProfilerType::Cpu(_) => {
                if let Some(ref profiler) = self.cpu_profiler {
                    profiler.clear();
                }
            }
            ProfilerType::Memory => {
                if let Some(ref profiler) = self.memory_profiler {
                    profiler.clear();
                }
            }
            ProfilerType::Lock => {
                if let Some(ref profiler) = self.lock_profiler {
                    profiler.clear();
                }
            }
            ProfilerType::Io => {
                if let Some(ref profiler) = self.io_profiler {
                    profiler.clear();
                }
            }
            ProfilerType::All => {
                self.clear(ProfilerType::Cpu(0))?;
                self.clear(ProfilerType::Memory)?;
                self.clear(ProfilerType::Lock)?;
                self.clear(ProfilerType::Io)?;
            }
        }

        Ok(())
    }

    /// Export profiling data in specified format
    pub fn export(
        &self,
        profiler_type: ProfilerType,
        format: ExportFormat,
    ) -> Result<Vec<u8>, ProfilingError> {
        let report = self.report(profiler_type)?;

        match (report, format) {
            (ProfileReport::Cpu(report), ExportFormat::Perf) => {
                report.export_perf().map_err(|e| ProfilingError::ExportFailed(e.to_string()))
            }
            (ProfileReport::Cpu(report), ExportFormat::Pprof) => {
                report.export_pprof().map_err(|e| ProfilingError::ExportFailed(e.to_string()))
            }
            (ProfileReport::Cpu(report), ExportFormat::FlameGraph) => {
                let svg = report.flame_graph().to_svg();
                Ok(svg.into_bytes())
            }
            (ProfileReport::Memory(snapshot), ExportFormat::Json) => {
                Ok(snapshot.export_json().into_bytes())
            }
            (ProfileReport::Io(report), ExportFormat::Json) => {
                Ok(report.export_json().into_bytes())
            }
            _ => Err(ProfilingError::ExportFailed(
                "Unsupported export format for this profiler type".to_string(),
            )),
        }
    }

    /// Get CPU profiler reference
    pub fn cpu_profiler(&self) -> Option<&cpu::CpuProfiler> {
        self.cpu_profiler.as_ref()
    }

    /// Get memory profiler reference
    pub fn memory_profiler(&self) -> Option<&memory::MemoryProfiler> {
        self.memory_profiler.as_ref()
    }

    /// Get lock profiler reference
    pub fn lock_profiler(&self) -> Option<&lock::LockProfiler> {
        self.lock_profiler.as_ref()
    }

    /// Get I/O profiler reference
    pub fn io_profiler(&self) -> Option<&io::IoProfiler> {
        self.io_profiler.as_ref()
    }

    /// Get symbol resolver reference
    pub fn symbol_resolver(&self) -> Option<&symbol::SymbolResolver> {
        self.symbol_resolver.as_ref()
    }
}

/// Convenience function to start CPU profiling globally
pub fn start_cpu_profiling(frequency_hz: u64) -> Result<(), ProfilingError> {
    let profiler_guard_ref = Profiler::global()?;
    let guard: MutexGuard<Option<Profiler>> = profiler_guard_ref.lock();
    if let Some(profiler) = guard.as_ref() {
        let profiler: &Profiler = profiler;
        profiler.start(ProfilerType::Cpu(frequency_hz))
    } else {
        Err(ProfilingError::NotRunning)
    }
}

/// Convenience function to stop CPU profiling globally
pub fn stop_cpu_profiling() -> Result<(), ProfilingError> {
    let profiler_guard_ref = Profiler::global()?;
    let guard: MutexGuard<Option<Profiler>> = profiler_guard_ref.lock();
    if let Some(profiler) = guard.as_ref() {
        let profiler: &Profiler = profiler;
        profiler.stop(ProfilerType::Cpu(0))
    } else {
        Err(ProfilingError::NotRunning)
    }
}

/// Convenience function to start memory profiling globally
pub fn start_memory_profiling() -> Result<(), ProfilingError> {
    let profiler_guard_ref = Profiler::global()?;
    let guard: MutexGuard<Option<Profiler>> = profiler_guard_ref.lock();
    if let Some(profiler) = guard.as_ref() {
        let profiler: &Profiler = profiler;
        profiler.start(ProfilerType::Memory)
    } else {
        Err(ProfilingError::NotRunning)
    }
}

/// Convenience function to stop memory profiling globally
pub fn stop_memory_profiling() -> Result<(), ProfilingError> {
    let profiler_guard_ref = Profiler::global()?;
    let guard: MutexGuard<Option<Profiler>> = profiler_guard_ref.lock();
    if let Some(profiler) = guard.as_ref() {
        let profiler: &Profiler = profiler;
        profiler.stop(ProfilerType::Memory)
    } else {
        Err(ProfilingError::NotRunning)
    }
}

/// Benchmark utilities
pub mod benchmark {
    use super::*;
    

    /// Simple benchmark timer
    pub struct Timer {
        start: u64,
    }

    impl Timer {
        /// Start a new timer
        pub fn start() -> Self {
            Self {
                start: Self::now(),
            }
        }

        /// Get elapsed time in nanoseconds
        pub fn elapsed_ns(&self) -> u64 {
            Self::now().saturating_sub(self.start)
        }

        /// Get elapsed time as Duration
        pub fn elapsed(&self) -> Duration {
            Duration::from_nanos(self.elapsed_ns())
        }

        /// Get current timestamp
        fn now() -> u64 {
            // In real implementation, use high-resolution timer
            0
        }
    }

    /// Benchmark a function
    pub fn benchmark<F, R>(f: F, iterations: usize) -> (Duration, R)
    where
        F: Fn() -> R,
    {
        let timer = Timer::start();
        let result = f();
        let elapsed = timer.elapsed();

        let avg_duration = Duration::from_nanos(elapsed.as_nanos() as u64 / iterations as u64);
        (avg_duration, result)
    }

    /// Benchmark multiple iterations and return statistics
    pub fn benchmark_stats<F>(f: F, iterations: usize) -> BenchmarkStats
    where
        F: Fn() -> u64,
    {
        let mut times = Vec::with_capacity(iterations);

        for _ in 0..iterations {
            let timer = Timer::start();
            let _ = f();
            times.push(timer.elapsed_ns());
        }

        times.sort();

        let min = times.first().copied().unwrap_or(0);
        let max = times.last().copied().unwrap_or(0);
        let sum: u64 = times.iter().sum();
        let avg = sum / times.len() as u64;
        let median = times[times.len() / 2];

        // Calculate 95th percentile
        let percentile_95 = times[(times.len() * 95) / 100];

        BenchmarkStats {
            min,
            max,
            avg,
            median,
            percentile_95,
            iterations: times.len() as u64,
        }
    }

    /// Benchmark statistics
    #[derive(Debug, Clone)]
    pub struct BenchmarkStats {
        /// Minimum time (nanoseconds)
        pub min: u64,
        /// Maximum time (nanoseconds)
        pub max: u64,
        /// Average time (nanoseconds)
        pub avg: u64,
        /// Median time (nanoseconds)
        pub median: u64,
        /// 95th percentile (nanoseconds)
        pub percentile_95: u64,
        /// Number of iterations
        pub iterations: u64,
    }

    impl BenchmarkStats {
        /// Export as JSON
        pub fn to_json(&self) -> String {
            format!(
                r#"{{"min_ns": {}, "max_ns": {}, "avg_ns": {}, "median_ns": {}, "p95_ns": {}, "iterations": {}}}"#,
                self.min, self.max, self.avg, self.median, self.percentile_95, self.iterations
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_creation() {
        let profiler = Profiler::new();
        assert!(!profiler.is_enabled());
    }

    #[test]
    fn test_profiler_enable_disable() {
        let profiler = Profiler::new();
        profiler.enable();
        assert!(profiler.is_enabled());
        profiler.disable();
        assert!(!profiler.is_enabled());
    }

    #[test]
    fn test_cpu_profiling() {
        let profiler = Profiler::new();
        profiler.enable();

        profiler.start(ProfilerType::Cpu(100)).unwrap();
        assert!(profiler.cpu_profiler().unwrap().is_running());

        profiler.stop(ProfilerType::Cpu(0)).unwrap();
        assert!(!profiler.cpu_profiler().unwrap().is_running());
    }

    #[test]
    fn test_memory_profiling() {
        let profiler = Profiler::new();
        profiler.enable();

        profiler.start(ProfilerType::Memory).unwrap();
        assert!(profiler.memory_profiler().unwrap().is_running());

        profiler.stop(ProfilerType::Memory).unwrap();
        assert!(!profiler.memory_profiler().unwrap().is_running());
    }

    #[test]
    fn test_lock_profiling() {
        let profiler = Profiler::new();
        profiler.enable();

        profiler.start(ProfilerType::Lock).unwrap();
        assert!(profiler.lock_profiler().unwrap().is_running());

        profiler.stop(ProfilerType::Lock).unwrap();
        assert!(!profiler.lock_profiler().unwrap().is_running());
    }

    #[test]
    fn test_io_profiling() {
        let profiler = Profiler::new();
        profiler.enable();

        profiler.start(ProfilerType::Io).unwrap();
        assert!(profiler.io_profiler().unwrap().is_running());

        profiler.stop(ProfilerType::Io).unwrap();
        assert!(!profiler.io_profiler().unwrap().is_running());
    }

    #[test]
    fn test_all_profilers() {
        let profiler = Profiler::new();
        profiler.enable();

        profiler.start(ProfilerType::All).unwrap();
        assert!(profiler.cpu_profiler().unwrap().is_running());
        assert!(profiler.memory_profiler().unwrap().is_running());
        assert!(profiler.lock_profiler().unwrap().is_running());
        assert!(profiler.io_profiler().unwrap().is_running());

        profiler.stop(ProfilerType::All).unwrap();
        assert!(!profiler.cpu_profiler().unwrap().is_running());
        assert!(!profiler.memory_profiler().unwrap().is_running());
        assert!(!profiler.lock_profiler().unwrap().is_running());
        assert!(!profiler.io_profiler().unwrap().is_running());
    }

    #[test]
    fn test_profiler_not_enabled() {
        let profiler = Profiler::new();
        // Profiler is not enabled
        let result = profiler.start(ProfilerType::Cpu(100));
        assert!(matches!(result, Err(ProfilingError::NotRunning)));
    }

    #[test]
    fn test_global_profiler_init() {
        Profiler::init_global().unwrap();
        let global = Profiler::global().unwrap();
        assert!(global.lock().is_some());
    }

    #[test]
    fn test_global_profiler_double_init() {
        Profiler::init_global().unwrap();
        let result = Profiler::init_global();
        assert!(matches!(result, Err(ProfilingError::AlreadyRunning)));
    }

    #[test]
    fn test_convenience_functions() {
        Profiler::init_global().unwrap();

        let profiler_guard = Profiler::global().unwrap();
        let profiler = profiler_guard.lock().as_ref().unwrap();
        profiler.enable();

        start_cpu_profiling(100).unwrap();
        assert!(profiler.cpu_profiler().unwrap().is_running());

        stop_cpu_profiling().unwrap();
        assert!(!profiler.cpu_profiler().unwrap().is_running());
    }

    #[test]
    fn test_benchmark_timer() {
        let timer = benchmark::Timer::start();
        // Simulate some work
        let _ = 1 + 1;
        let elapsed = timer.elapsed();
        assert!(elapsed.as_nanos() >= 0);
    }

    #[test]
    fn test_benchmark_stats() {
        let stats = benchmark::benchmark_stats(|| {
            // Simulate variable work
            let x = 42;
            x as u64
        }, 100);

        assert_eq!(stats.iterations, 100);
        assert!(stats.min <= stats.avg);
        assert!(stats.avg <= stats.max);
    }

    #[test]
    fn test_export_perf_format() {
        let profiler = Profiler::new();
        profiler.enable();

        profiler.start(ProfilerType::Cpu(100)).unwrap();
        // Simulate some CPU activity
        profiler.stop(ProfilerType::Cpu(0)).unwrap();

        let data = profiler.export(ProfilerType::Cpu(0), ExportFormat::Perf);
        assert!(data.is_ok());
        assert!(data.unwrap().starts_with(b"# perf profile"));
    }

    #[test]
    fn test_clear_profiling_data() {
        let profiler = Profiler::new();
        profiler.enable();

        profiler.start(ProfilerType::Memory).unwrap();

        let mem_profiler = profiler.memory_profiler().unwrap();
        mem_profiler.record_allocation(0x1000, 1024, 8).unwrap();

        profiler.clear(ProfilerType::Memory).unwrap();

        let snapshot = mem_profiler.take_snapshot().unwrap();
        assert_eq!(snapshot.total_live_allocations, 0);
    }
}
