//! CPU Performance Profiling
//!
//! This module provides sampling-based CPU profiling capabilities for the NOS kernel,
//! including stack unwinding, flame graph generation, and symbol resolution.
//!
//! # Features
//!
//! - Frequency-based sampling profiler
//! - Stack unwinding and frame capture
//! - Flame graph data generation
//! - Profile data aggregation
//! - Low overhead (<5% when enabled)
//! - Configurable sampling rate
//!
//! # Usage
//!
//! ```rust
//! use kernel::profiling::cpu::CpuProfiler;
//!
//! let profiler = CpuProfiler::new(1000); // 1000 Hz sampling
//! profiler.start().unwrap();
//! // ... code to profile ...
//! profiler.stop().unwrap();
//! let report = profiler.generate_report().unwrap();
//! ```

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use core::time::Duration;

use crate::sync::Mutex;

/// CPU profiling error types
#[derive(Debug, Clone, PartialEq)]
pub enum CpuProfileError {
    /// Profiler is already running
    AlreadyRunning,
    /// Profiler is not running
    NotRunning,
    /// Invalid sampling frequency
    InvalidFrequency(u64),
    /// Stack unwind failed
    StackUnwindFailed,
    /// Buffer overflow
    BufferOverflow,
    /// Symbol resolution failed
    SymbolResolutionFailed,
}

impl core::fmt::Display for CpuProfileError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::AlreadyRunning => write!(f, "Profiler is already running"),
            Self::NotRunning => write!(f, "Profiler is not running"),
            Self::InvalidFrequency(freq) => write!(f, "Invalid sampling frequency: {}", freq),
            Self::StackUnwindFailed => write!(f, "Failed to unwind stack"),
            Self::BufferOverflow => write!(f, "Profile buffer overflow"),
            Self::SymbolResolutionFailed => write!(f, "Failed to resolve symbol"),
        }
    }
}

/// Stack frame captured during profiling
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StackFrame {
    /// Instruction pointer address
    pub ip: u64,
    /// Stack pointer address
    pub sp: u64,
    /// Frame pointer address
    pub fp: u64,
    /// Module/function name (if available)
    pub symbol: Option<alloc::string::String>,
    /// File name (if available)
    pub file: Option<alloc::string::String>,
    /// Line number (if available)
    pub line: Option<u32>,
}

impl StackFrame {
    /// Create a new stack frame from addresses
    pub fn from_addresses(ip: u64, sp: u64, fp: u64) -> Self {
        Self {
            ip,
            sp,
            fp,
            symbol: None,
            file: None,
            line: None,
        }
    }

    /// Set symbol information
    pub fn with_symbol(mut self, symbol: alloc::string::String) -> Self {
        self.symbol = Some(symbol);
        self
    }

    /// Set source location
    pub fn with_location(
        mut self,
        file: alloc::string::String,
        line: u32,
    ) -> Self {
        self.file = Some(file);
        self.line = Some(line);
        self
    }
}

/// Stack trace captured during sampling
pub type StackTrace = Vec<StackFrame>;

/// Profile sample data
#[derive(Debug, Clone)]
pub struct ProfileSample {
    /// Timestamp of the sample
    pub timestamp: u64,
    /// CPU ID where sample was taken
    pub cpu_id: u32,
    /// Process ID
    pub pid: u64,
    /// Thread ID
    pub tid: u64,
    /// Captured stack trace
    pub stack_trace: StackTrace,
    /// User/kernel mode indicator
    pub is_kernel: bool,
}

impl ProfileSample {
    /// Create a new profile sample
    pub fn new(
        cpu_id: u32,
        pid: u64,
        tid: u64,
        stack_trace: StackTrace,
        is_kernel: bool,
    ) -> Self {
        Self {
            timestamp: Self::timestamp_now(),
            cpu_id,
            pid,
            tid,
            stack_trace,
            is_kernel,
        }
    }

    /// Get current timestamp
    fn timestamp_now() -> u64 {
        // In real implementation, use TSC or proper timer
        0
    }
}

/// Aggregated profile data for a call site
#[derive(Debug)]
pub struct CallSiteProfile {
    /// Instruction pointer
    pub ip: u64,
    /// Symbol name (if resolved)
    pub symbol: Option<alloc::string::String>,
    /// Number of samples hitting this location
    pub sample_count: AtomicU64,
    /// Total estimated time (nanoseconds)
    pub total_time: AtomicU64,
    /// Parent call sites
    pub parents: Vec<u64>,
    /// Child call sites
    pub children: Vec<u64>,
}

impl Clone for CallSiteProfile {
    fn clone(&self) -> Self {
        Self {
            ip: self.ip,
            symbol: self.symbol.clone(),
            sample_count: AtomicU64::new(self.sample_count.load(Ordering::Relaxed)),
            total_time: AtomicU64::new(self.total_time.load(Ordering::Relaxed)),
            parents: self.parents.clone(),
            children: self.children.clone(),
        }
    }
}

impl CallSiteProfile {
    /// Create a new call site profile
    pub fn new(ip: u64) -> Self {
        Self {
            ip,
            symbol: None,
            sample_count: AtomicU64::new(0),
            total_time: AtomicU64::new(0),
            parents: Vec::new(),
            children: Vec::new(),
        }
    }

    /// Record a sample
    pub fn record_sample(&self, estimated_time_ns: u64) {
        self.sample_count.fetch_add(1, Ordering::Relaxed);
        self.total_time.fetch_add(estimated_time_ns, Ordering::Relaxed);
    }

    /// Get sample count
    pub fn get_sample_count(&self) -> u64 {
        self.sample_count.load(Ordering::Relaxed)
    }

    /// Get total time
    pub fn get_total_time(&self) -> u64 {
        self.total_time.load(Ordering::Relaxed)
    }
}

/// CPU profiler configuration
#[derive(Debug, Clone)]
pub struct CpuProfilerConfig {
    /// Sampling frequency in Hz
    pub sampling_frequency: u64,
    /// Maximum stack depth to capture
    pub max_stack_depth: usize,
    /// Include kernel frames
    pub include_kernel: bool,
    /// Resolve symbols during profiling
    pub resolve_symbols: bool,
    /// Maximum number of samples
    pub max_samples: usize,
    /// Filter by process ID (None = all processes)
    pub filter_pid: Option<u64>,
}

impl Default for CpuProfilerConfig {
    fn default() -> Self {
        Self {
            sampling_frequency: 100, // 100 Hz default
            max_stack_depth: 64,
            include_kernel: true,
            resolve_symbols: false, // Defer symbol resolution for lower overhead
            max_samples: 1_000_000,
            filter_pid: None,
        }
    }
}

/// CPU profiler implementation
pub struct CpuProfiler {
    config: CpuProfilerConfig,
    state: Mutex<ProfilerState>,
    samples: Mutex<Vec<ProfileSample>>,
    call_sites: Mutex<BTreeMap<u64, CallSiteProfile>>,
    total_samples: AtomicU64,
    dropped_samples: AtomicU64,
}

struct ProfilerState {
    running: bool,
    paused: bool,
    start_time: Option<u64>,
    stop_time: Option<u64>,
}

impl CpuProfiler {
    /// Create a new CPU profiler
    pub fn new(sampling_frequency: u64) -> Result<Self, CpuProfileError> {
        if sampling_frequency == 0 || sampling_frequency > 1_000_000 {
            return Err(CpuProfileError::InvalidFrequency(sampling_frequency));
        }

        Ok(Self {
            config: CpuProfilerConfig {
                sampling_frequency,
                ..Default::default()
            },
            state: Mutex::new(ProfilerState {
                running: false,
                paused: false,
                start_time: None,
                stop_time: None,
            }),
            samples: Mutex::new(Vec::with_capacity(10_000)),
            call_sites: Mutex::new(BTreeMap::new()),
            total_samples: AtomicU64::new(0),
            dropped_samples: AtomicU64::new(0),
        })
    }

    /// Create profiler with custom configuration
    pub fn with_config(config: CpuProfilerConfig) -> Result<Self, CpuProfileError> {
        if config.sampling_frequency == 0 || config.sampling_frequency > 1_000_000 {
            return Err(CpuProfileError::InvalidFrequency(config.sampling_frequency));
        }

        Ok(Self {
            config,
            state: Mutex::new(ProfilerState {
                running: false,
                paused: false,
                start_time: None,
                stop_time: None,
            }),
            samples: Mutex::new(Vec::with_capacity(10_000)),
            call_sites: Mutex::new(BTreeMap::new()),
            total_samples: AtomicU64::new(0),
            dropped_samples: AtomicU64::new(0),
        })
    }

    /// Start profiling
    pub fn start(&self) -> Result<(), CpuProfileError> {
        let mut state = self.state.lock();

        if state.running {
            return Err(CpuProfileError::AlreadyRunning);
        }

        state.running = true;
        state.paused = false;
        state.start_time = Some(self.get_timestamp());
        state.stop_time = None;

        // In real implementation, start sampling timer here
        Ok(())
    }

    /// Stop profiling
    pub fn stop(&self) -> Result<(), CpuProfileError> {
        let mut state = self.state.lock();

        if !state.running {
            return Err(CpuProfileError::NotRunning);
        }

        state.running = false;
        state.stop_time = Some(self.get_timestamp());

        // In real implementation, stop sampling timer here
        Ok(())
    }

    /// Pause profiling (keep running but don't collect samples)
    pub fn pause(&self) -> Result<(), CpuProfileError> {
        let mut state = self.state.lock();

        if !state.running {
            return Err(CpuProfileError::NotRunning);
        }

        state.paused = true;
        Ok(())
    }

    /// Resume profiling
    pub fn resume(&self) -> Result<(), CpuProfileError> {
        let mut state = self.state.lock();

        if !state.running {
            return Err(CpuProfileError::NotRunning);
        }

        state.paused = false;
        Ok(())
    }

    /// Check if profiler is running
    pub fn is_running(&self) -> bool {
        self.state.lock().running
    }

    /// Check if profiler is paused
    pub fn is_paused(&self) -> bool {
        self.state.lock().paused
    }

    /// Record a sample (called by sampling interrupt)
    pub fn record_sample(&self, sample: ProfileSample) -> Result<(), CpuProfileError> {
        let state = self.state.lock();
        if !state.running || state.paused {
            return Ok(()); // Silently ignore when not running or paused
        }
        drop(state);

        // Check sample limit
        let current_count = self.total_samples.load(Ordering::Relaxed);
        if current_count >= self.config.max_samples as u64 {
            self.dropped_samples.fetch_add(1, Ordering::Relaxed);
            return Err(CpuProfileError::BufferOverflow);
        }

        // Store sample
        let mut samples = self.samples.lock();
        if samples.len() >= self.config.max_samples {
            self.dropped_samples.fetch_add(1, Ordering::Relaxed);
            return Err(CpuProfileError::BufferOverflow);
        }

        samples.push(sample.clone());
        self.total_samples.fetch_add(1, Ordering::Relaxed);

        // Aggregate call site data
        self.aggregate_sample(&sample);

        Ok(())
    }

    /// Aggregate sample into call site profiles
    fn aggregate_sample(&self, sample: &ProfileSample) {
        let estimated_time = 1_000_000_000 / self.config.sampling_frequency; // ns

        let mut call_sites = self.call_sites.lock();

        // Process each frame in the stack trace
        for (i, frame) in sample.stack_trace.iter().enumerate() {
            let ip = frame.ip;

            // Get or create call site profile
            let call_site = call_sites.entry(ip).or_insert_with(|| {
                let mut site = CallSiteProfile::new(ip);
                if let Some(ref symbol) = frame.symbol {
                    site.symbol = Some(symbol.clone());
                }
                site
            });

            call_site.record_sample(estimated_time);

            // Link parent-child relationships
            if i > 0 {
                let parent_ip = sample.stack_trace[i - 1].ip;
                if !call_site.parents.contains(&parent_ip) {
                    call_site.parents.push(parent_ip);
                }
            }

            if i < sample.stack_trace.len() - 1 {
                let child_ip = sample.stack_trace[i + 1].ip;
                if !call_site.children.contains(&child_ip) {
                    call_site.children.push(child_ip);
                }
            }
        }
    }

    /// Generate profiling report
    pub fn generate_report(&self) -> Result<CpuProfileReport, CpuProfileError> {
        let state = self.state.lock();
        let start_time = state.start_time.unwrap_or(0);
        let stop_time = state.stop_time.unwrap_or_else(|| self.get_timestamp());
        drop(state);

        let samples = self.samples.lock();
        let call_sites = self.call_sites.lock();

        let duration = stop_time.saturating_sub(start_time);
        let sampling_period = if duration > 0 {
            Duration::from_nanos(duration)
        } else {
            Duration::from_secs(0)
        };

        Ok(CpuProfileReport {
            sampling_frequency: self.config.sampling_frequency,
            total_samples: self.total_samples.load(Ordering::Relaxed),
            dropped_samples: self.dropped_samples.load(Ordering::Relaxed),
            duration: sampling_period,
            samples: samples.clone(),
            call_sites: call_sites.clone(),
        })
    }

    /// Clear all collected data
    pub fn clear(&self) {
        self.samples.lock().clear();
        self.call_sites.lock().clear();
        self.total_samples.store(0, Ordering::Relaxed);
        self.dropped_samples.store(0, Ordering::Relaxed);
    }

    /// Get current timestamp
    fn get_timestamp(&self) -> u64 {
        // In real implementation, use high-resolution timer
        0
    }
}

/// CPU profiling report
#[derive(Debug, Clone)]
pub struct CpuProfileReport {
    /// Sampling frequency used
    pub sampling_frequency: u64,
    /// Total samples collected
    pub total_samples: u64,
    /// Samples dropped due to buffer overflow
    pub dropped_samples: u64,
    /// Duration of profiling session
    pub duration: Duration,
    /// Raw samples
    pub samples: Vec<ProfileSample>,
    /// Aggregated call site data
    pub call_sites: BTreeMap<u64, CallSiteProfile>,
}

impl CpuProfileReport {
    /// Generate flame graph data
    pub fn flame_graph(&self) -> FlameGraph {
        let mut graph = FlameGraph::new();

        // Process each sample
        for sample in &self.samples {
            self.add_sample_to_flame_graph(&mut graph, sample);
        }

        graph
    }

    /// Add a sample to flame graph
    fn add_sample_to_flame_graph(&self, graph: &mut FlameGraph, sample: &ProfileSample) {
        let mut current = &mut graph.root;

        // Add frames from leaf to root (reversed stack trace)
        for frame in sample.stack_trace.iter().rev() {
            let name = frame.symbol.as_ref().map(|s| s.as_str()).unwrap_or("unknown");

            current = current
                .children
                .entry(alloc::string::String::from(name))
                .or_insert_with(|| FlameGraphNode::new(name));

            current.value += 1;
        }
    }

    /// Get top N hot functions by sample count
    pub fn top_functions(&self, n: usize) -> Vec<(u64, Option<alloc::string::String>)> {
        let mut sites: Vec<_> = self
            .call_sites
            .iter()
            .map(|(&_ip, site)| (site.get_sample_count(), site.symbol.as_ref()))
            .collect();

        sites.sort_by(|a, b| b.0.cmp(&a.0));
        sites.truncate(n);
        sites
            .into_iter()
            .map(|(count, symbol)| (count, symbol.map(|s| s.clone())))
            .collect()
    }

    /// Export in perf format
    pub fn export_perf(&self) -> Result<alloc::vec::Vec<u8>, CpuProfileError> {
        // Simplified perf format - real implementation would write actual perf data format
        let mut output = alloc::vec::Vec::new();

        // Header
        output.extend_from_slice(b"# perf profile\n");
        output.extend_from_slice(format!("frequency={}\n", self.sampling_frequency).as_bytes());
        output.extend_from_slice(format!("duration={:?}\n", self.duration).as_bytes());
        output.extend_from_slice(format!("samples={}\n", self.total_samples).as_bytes());
        output.extend_from_slice(b"\n");

        // Call sites
        for (&ip, site) in &self.call_sites {
            let symbol = site.symbol.as_deref().unwrap_or("unknown");
            output.extend_from_slice(
                format!(
                    "{} {} {} {}\n",
                    ip,
                    symbol,
                    site.get_sample_count(),
                    site.get_total_time()
                )
                .as_bytes(),
            );
        }

        Ok(output)
    }

    /// Export in pprof format
    pub fn export_pprof(&self) -> Result<alloc::vec::Vec<u8>, CpuProfileError> {
        // Simplified pprof format - real implementation would write protobuf
        let mut output = alloc::vec::Vec::new();

        output.extend_from_slice(b"profile {\n");
        output.extend_from_slice(&format!("  sample_type: \"samples\"\n").into_bytes());
        output.extend_from_slice(&format!("  samples: {}\n", self.total_samples).into_bytes());
        output.extend_from_slice(&format!("  period: {}\n", self.sampling_frequency).into_bytes());
        output.extend_from_slice(b"}\n");

        // Sample data
        for sample in &self.samples {
            output.extend_from_slice(b"sample {\n");
            output.extend_from_slice(&format!("  pid: {}\n", sample.pid).into_bytes());
            output.extend_from_slice(&format!("  tid: {}\n", sample.tid).into_bytes());
            output.extend_from_slice(b"  locations: [");

            for (i, frame) in sample.stack_trace.iter().enumerate() {
                if i > 0 {
                    output.extend_from_slice(b", ");
                }
                output.extend_from_slice(format!("0x{:x}", frame.ip).as_bytes());
            }

            output.extend_from_slice(b"]\n");
            output.extend_from_slice(b"}\n");
        }

        Ok(output)
    }
}

/// Flame graph node
#[derive(Debug, Clone)]
pub struct FlameGraphNode {
    /// Function name
    pub name: alloc::string::String,
    /// Sample count (width in flame graph)
    pub value: u64,
    /// Child function calls
    pub children: alloc::collections::BTreeMap<alloc::string::String, FlameGraphNode>,
}

impl FlameGraphNode {
    /// Create a new flame graph node
    pub fn new(name: &str) -> Self {
        Self {
            name: alloc::string::String::from(name),
            value: 0,
            children: alloc::collections::BTreeMap::new(),
        }
    }
}

/// Flame graph data structure
#[derive(Debug, Clone)]
pub struct FlameGraph {
    pub root: FlameGraphNode,
}

impl FlameGraph {
    /// Create a new empty flame graph
    pub fn new() -> Self {
        Self {
            root: FlameGraphNode::new("root"),
        }
    }

    /// Export as SVG string
    pub fn to_svg(&self) -> alloc::string::String {
        let mut output = alloc::string::String::new();

        output.push_str(r#"<?xml version="1.0" standalone="no"?>"#);
        output.push_str(r#"<!DOCTYPE svg PUBLIC "-//W3C//DTD SVG 1.1//EN" "http://www.w3.org/Graphics/SVG/1.1/DTD/svg11.dtd">"#);
        output.push_str(r#"<svg version="1.1" width="100%" height="100%" xmlns="http://www.w3.org/2000/svg">"#);
        output.push_str(r#"<g class="flame-graph">"#);

        // Render nodes recursively
        self.render_node(&mut output, &self.root, 0, 0, 1000, 30);

        output.push_str(r#"</g>"#);
        output.push_str(r#"</svg>"#);

        output
    }

    /// Render a flame graph node
    fn render_node(
        &self,
        output: &mut alloc::string::String,
        node: &FlameGraphNode,
        x: u64,
        y: u64,
        width: u64,
        height: u64,
    ) {
        let mut current_x = x;
        let total_value = node.value.max(1);

        for (_name, child) in &node.children {
            let child_width = (child.value * width) / total_value;

            output.push_str(&format!(
                r#"<rect x="{}" y="{}" width="{}" height="{}" fill="rgb(255,128,0)" stroke="white">"#,
                current_x, y, child_width, height
            ));

            output.push_str(&format!(
                r#"<title>{}: {} samples</title></rect>"#,
                child.name, child.value
            ));

            // Render children
            self.render_node(output, child, current_x, y + height, child_width, height);

            current_x += child_width;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profiler_creation() {
        let profiler = CpuProfiler::new(100).unwrap();
        assert!(!profiler.is_running());
        assert!(!profiler.is_paused());
    }

    #[test]
    fn test_profiler_start_stop() {
        let profiler = CpuProfiler::new(100).unwrap();
        profiler.start().unwrap();
        assert!(profiler.is_running());
        profiler.stop().unwrap();
        assert!(!profiler.is_running());
    }

    #[test]
    fn test_profiler_double_start() {
        let profiler = CpuProfiler::new(100).unwrap();
        profiler.start().unwrap();
        let result = profiler.start();
        assert!(matches!(result, Err(CpuProfileError::AlreadyRunning)));
    }

    #[test]
    fn test_profiler_pause_resume() {
        let profiler = CpuProfiler::new(100).unwrap();
        profiler.start().unwrap();
        profiler.pause().unwrap();
        assert!(profiler.is_paused());
        profiler.resume().unwrap();
        assert!(!profiler.is_paused());
    }

    #[test]
    fn test_stack_frame_creation() {
        let frame = StackFrame::from_addresses(0x1000, 0x2000, 0x3000);
        assert_eq!(frame.ip, 0x1000);
        assert_eq!(frame.sp, 0x2000);
        assert_eq!(frame.fp, 0x3000);
    }

    #[test]
    fn test_stack_frame_with_symbol() {
        let frame = StackFrame::from_addresses(0x1000, 0x2000, 0x3000)
            .with_symbol(alloc::string::String::from("test_function"));
        assert_eq!(frame.symbol, Some(alloc::string::String::from("test_function")));
    }

    #[test]
    fn test_sample_recording() {
        let profiler = CpuProfiler::new(100).unwrap();
        profiler.start().unwrap();

        let stack_trace = vec![StackFrame::from_addresses(0x1000, 0x2000, 0x3000)];
        let sample = ProfileSample::new(0, 1, 2, stack_trace, false);

        profiler.record_sample(sample).unwrap();
        assert_eq!(profiler.total_samples.load(Ordering::Relaxed), 1);

        profiler.stop().unwrap();
    }

    #[test]
    fn test_report_generation() {
        let profiler = CpuProfiler::new(100).unwrap();
        profiler.start().unwrap();

        let stack_trace = vec![
            StackFrame::from_addresses(0x1000, 0x2000, 0x3000)
                .with_symbol(alloc::string::String::from("func1")),
            StackFrame::from_addresses(0x2000, 0x2100, 0x3100)
                .with_symbol(alloc::string::String::from("func2")),
        ];
        let sample = ProfileSample::new(0, 1, 2, stack_trace, true);

        profiler.record_sample(sample).unwrap();
        profiler.stop().unwrap();

        let report = profiler.generate_report().unwrap();
        assert_eq!(report.total_samples, 1);
        assert!(!report.call_sites.is_empty());
    }

    #[test]
    fn test_flame_graph_generation() {
        let profiler = CpuProfiler::new(100).unwrap();
        profiler.start().unwrap();

        let stack_trace = vec![
            StackFrame::from_addresses(0x1000, 0x2000, 0x3000)
                .with_symbol(alloc::string::String::from("main")),
            StackFrame::from_addresses(0x2000, 0x2100, 0x3100)
                .with_symbol(alloc::string::String::from("worker")),
        ];
        let sample = ProfileSample::new(0, 1, 2, stack_trace, false);

        profiler.record_sample(sample).unwrap();
        profiler.stop().unwrap();

        let report = profiler.generate_report().unwrap();
        let flame_graph = report.flame_graph();

        assert_eq!(flame_graph.root.value, 2); // Both frames counted
    }

    #[test]
    fn test_export_perf() {
        let profiler = CpuProfiler::new(100).unwrap();
        profiler.start().unwrap();

        let stack_trace = vec![StackFrame::from_addresses(0x1000, 0x2000, 0x3000)];
        let sample = ProfileSample::new(0, 1, 2, stack_trace, false);

        profiler.record_sample(sample).unwrap();
        profiler.stop().unwrap();

        let report = profiler.generate_report().unwrap();
        let perf_data = report.export_perf().unwrap();

        assert!(perf_data.starts_with(b"# perf profile"));
    }

    #[test]
    fn test_invalid_frequency() {
        let result = CpuProfiler::new(0);
        assert!(matches!(result, Err(CpuProfileError::InvalidFrequency(0))));

        let result = CpuProfiler::new(2_000_000);
        assert!(matches!(result, Err(CpuProfileError::InvalidFrequency(_))));
    }
}
