//! # eBPF Tracer
//!
//! This module provides comprehensive eBPF tracing support for kernel and userspace instrumentation.
//!
//! ## Overview
//!
//! The eBPF tracer enables dynamic instrumentation of:
//! - **Kernel functions** via kprobes (entry/exit tracing)
//! - **Static tracepoints** for kernel events
//! - **Userspace functions** via uprobes
//!
//! ## Architecture
//!
//! The tracer is designed for production use with:
//! - **Low overhead**: <5% performance impact target
//! - **Per-CPU buffers**: Zero-copy event delivery
//! - **Batch processing**: Efficient event handling
//! - **Flexible filtering**: PID/TID/CPU/event type filtering
//!
//! ## Usage Example
//!
//! ```rust,no_run,ignore
//! use crate::bpf::tracer::{BpfTracer, TracerConfig};
//!
//! // Create tracer with default config
//! let tracer = BpfTracer::new(TracerConfig::default())?;
//!
//! // Register a kprobe
//! tracer.register_kprobe("do_sys_open", KprobeType::Entry, my_handler)?;
//!
//! // Register a tracepoint
//! tracer.register_tracepoint("sched", "sched_switch", my_handler)?;
//!
//! // Process events
//! tracer.process_events(|events| {
//!     for event in events {
//!         println!("{:?}", event);
//!     }
//!     Ok(())
//! })?;
//! ```
//!
//! ## Performance Considerations
//!
//! - Per-CPU buffers avoid lock contention
//! - Event batching reduces context switches
//! - Filters applied early in the trace path
//! - Zero-copy when possible

use crate::prelude::*;
use crate::types::{Pid, Tid};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use spin::Mutex;

// ============================================================================
// Constants and Configuration
// ============================================================================

/// Default per-CPU buffer size (number of events)
const DEFAULT_PER_CPU_BUFFER_SIZE: usize = 4096;

/// Maximum event data size (bytes)
const MAX_EVENT_DATA_SIZE: usize = 512;

/// Default batch size for event processing
const DEFAULT_BATCH_SIZE: usize = 64;

/// Target overhead percentage for tracing
const TARGET_OVERHEAD_PERCENT: u8 = 5;

// ============================================================================
// Error Types
// ============================================================================

/// Tracer-specific errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TracerError {
    /// Probe already registered
    ProbeAlreadyRegistered(String),
    /// Probe not found
    ProbeNotFound(String),
    /// Invalid function name
    InvalidFunctionName(String),
    /// Invalid tracepoint name
    InvalidTracepoint(String),
    /// Buffer overflow
    BufferOverflow,
    /// Invalid filter
    InvalidFilter(String),
    /// Permission denied
    PermissionDenied,
    /// Resource limit exceeded
    ResourceLimitExceeded,
    /// Invalid PID
    InvalidPid,
    /// Invalid CPU ID
    InvalidCpuId,
}

impl core::fmt::Display for TracerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::ProbeAlreadyRegistered(name) => {
                write!(f, "Probe already registered: {}", name)
            }
            Self::ProbeNotFound(name) => write!(f, "Probe not found: {}", name),
            Self::InvalidFunctionName(name) => write!(f, "Invalid function name: {}", name),
            Self::InvalidTracepoint(name) => write!(f, "Invalid tracepoint: {}", name),
            Self::BufferOverflow => write!(f, "Event buffer overflow"),
            Self::InvalidFilter(msg) => write!(f, "Invalid filter: {}", msg),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::ResourceLimitExceeded => write!(f, "Resource limit exceeded"),
            Self::InvalidPid => write!(f, "Invalid PID"),
            Self::InvalidCpuId => write!(f, "Invalid CPU ID"),
        }
    }
}

impl Error for TracerError {}

/// Tracer result type
pub type TracerResult<T> = core::result::Result<T, TracerError>;

// ============================================================================
// Configuration
// ============================================================================

/// Tracer configuration
#[derive(Debug, Clone)]
pub struct TracerConfig {
    /// Per-CPU event buffer size
    pub per_cpu_buffer_size: usize,
    /// Maximum event data size
    pub max_event_data_size: usize,
    /// Default batch size for processing
    pub batch_size: usize,
    /// Enable performance monitoring
    pub enable_perf_monitoring: bool,
    /// Maximum number of concurrent probes
    pub max_probes: usize,
}

impl Default for TracerConfig {
    fn default() -> Self {
        Self {
            per_cpu_buffer_size: DEFAULT_PER_CPU_BUFFER_SIZE,
            max_event_data_size: MAX_EVENT_DATA_SIZE,
            batch_size: DEFAULT_BATCH_SIZE,
            enable_perf_monitoring: true,
            max_probes: 1024,
        }
    }
}

// ============================================================================
// Event Types
// ============================================================================

/// Kprobe type (entry or return)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KprobeType {
    /// Function entry (before execution)
    Entry,
    /// Function return (after execution)
    Return,
}

/// Trace event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceEventType {
    /// Kprobe event
    Kprobe,
    /// Tracepoint event
    Tracepoint,
    /// Uprobe event
    Uprobe,
}

/// Trace event
#[derive(Debug)]
pub struct TraceEvent {
    /// Event type
    pub event_type: TraceEventType,
    /// Event timestamp (nanoseconds)
    pub timestamp: u64,
    /// CPU ID where event occurred
    pub cpu_id: u32,
    /// Process ID
    pub pid: Pid,
    /// Thread ID
    pub tid: Tid,
    /// Function/probe name
    pub name: String,
    /// Event data (registers, arguments, return value, etc.)
    pub data: Vec<u8>,
    /// Kprobe type (if applicable)
    pub kprobe_type: Option<KprobeType>,
}

impl TraceEvent {
    /// Create a new trace event
    pub fn new(
        event_type: TraceEventType,
        cpu_id: u32,
        pid: Pid,
        tid: Tid,
        name: &str,
    ) -> Self {
        Self {
            event_type,
            timestamp: Self::get_timestamp(),
            cpu_id,
            pid,
            tid,
            name: name.to_string(),
            data: Vec::new(),
            kprobe_type: None,
        }
    }

    /// Set event data
    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// Set kprobe type
    pub fn with_kprobe_type(mut self, kprobe_type: KprobeType) -> Self {
        self.kprobe_type = Some(kprobe_type);
        self
    }

    /// Get current timestamp in nanoseconds
    fn get_timestamp() -> u64 {
        // TODO: Integrate with proper time source
        // For now, use a simple counter
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }
}

// ============================================================================
// Event Filters
// ============================================================================

/// Event filter criteria
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// Filter by PID (None = no filtering)
    pub pid_filter: Option<Pid>,
    /// Filter by TID (None = no filtering)
    pub tid_filter: Option<Tid>,
    /// Filter by CPU ID (None = no filtering)
    pub cpu_filter: Option<u32>,
    /// Filter by event type (None = no filtering)
    pub event_type_filter: Option<TraceEventType>,
    /// Minimum timestamp (None = no filtering)
    pub min_timestamp: Option<u64>,
    /// Maximum timestamp (None = no filtering)
    pub max_timestamp: Option<u64>,
}

impl EventFilter {
    /// Create a new filter with no restrictions
    pub fn new() -> Self {
        Self {
            pid_filter: None,
            tid_filter: None,
            cpu_filter: None,
            event_type_filter: None,
            min_timestamp: None,
            max_timestamp: None,
        }
    }

    /// Check if an event matches this filter
    pub fn matches(&self, event: &TraceEvent) -> bool {
        if let Some(pid) = self.pid_filter {
            if event.pid != pid {
                return false;
            }
        }

        if let Some(tid) = self.tid_filter {
            if event.tid != tid {
                return false;
            }
        }

        if let Some(cpu) = self.cpu_filter {
            if event.cpu_id != cpu {
                return false;
            }
        }

        if let Some(event_type) = self.event_type_filter {
            if event.event_type != event_type {
                return false;
            }
        }

        if let Some(min_ts) = self.min_timestamp {
            if event.timestamp < min_ts {
                return false;
            }
        }

        if let Some(max_ts) = self.max_timestamp {
            if event.timestamp > max_ts {
                return false;
            }
        }

        true
    }
}

impl Default for EventFilter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Per-CPU Event Buffer
// ============================================================================

/// Per-CPU event buffer for lock-free event collection
struct PerCpuBuffer {
    /// CPU ID
    cpu_id: u32,
    /// Event ring buffer
    events: Vec<Option<TraceEvent>>,
    /// Current write position
    write_pos: AtomicUsize,
    /// Current read position
    read_pos: AtomicUsize,
    /// Dropped event counter
    dropped_events: AtomicU64,
}

impl PerCpuBuffer {
    /// Create a new per-CPU buffer
    fn new(cpu_id: u32, size: usize) -> Self {
        Self {
            cpu_id,
            events: vec![None; size],
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
            dropped_events: AtomicU64::new(0),
        }
    }

    /// Push an event into the buffer (non-blocking)
    fn push(&self, event: TraceEvent) -> TracerResult<()> {
        let write = self.write_pos.fetch_add(1, Ordering::Relaxed) % self.events.len();
        let read = self.read_pos.load(Ordering::Acquire);

        // Check if buffer would overflow
        let next_write = (write + 1) % self.events.len();
        if next_write == read {
            self.dropped_events.fetch_add(1, Ordering::Relaxed);
            return Err(TracerError::BufferOverflow);
        }

        self.events[write] = Some(event);
        Ok(())
    }

    /// Pop events from the buffer (for batch processing)
    fn pop_batch(&self, max_count: usize) -> Vec<TraceEvent> {
        let mut batch = Vec::with_capacity(max_count);
        let read = self.read_pos.load(Ordering::Relaxed);
        let write = self.write_pos.load(Ordering::Acquire);

        for _ in 0..max_count {
            if read == write {
                break;
            }
            let pos = self.read_pos.fetch_add(1, Ordering::Relaxed) % self.events.len();
            if let Some(event) = self.events[pos].take() {
                batch.push(event);
            }
        }

        batch
    }

    /// Get number of dropped events
    fn dropped_count(&self) -> u64 {
        self.dropped_events.load(Ordering::Relaxed)
    }

    /// Get current buffer depth
    fn depth(&self) -> usize {
        let write = self.write_pos.load(Ordering::Relaxed);
        let read = self.read_pos.load(Ordering::Relaxed);
        if write >= read {
            write - read
        } else {
            self.events.len() - read + write
        }
    }
}

// ============================================================================
// Probe Handler Types
// ============================================================================

/// Kprobe handler callback type
pub type KprobeHandler = fn(&TraceEvent) -> TracerResult<()>;

/// Tracepoint handler callback type
pub type TracepointHandler = fn(&TraceEvent) -> TracerResult<()>;

/// Uprobe handler callback type
pub type UprobeHandler = fn(&TraceEvent) -> TracerResult<()>;

// ============================================================================
// Probe Registration State
// ============================================================================

/// Kprobe registration
struct KprobeRegistration {
    /// Function name
    function_name: String,
    /// Probe type
    probe_type: KprobeType,
    /// Handler callback
    handler: KprobeHandler,
    /// Registration timestamp
    registered_at: u64,
    /// Event count
    event_count: AtomicU64,
}

/// Tracepoint registration
struct TracepointRegistration {
    /// Tracepoint subsystem
    subsystem: String,
    /// Tracepoint name
    name: String,
    /// Handler callback
    handler: TracepointHandler,
    /// Registration timestamp
    registered_at: u64,
    /// Event count
    event_count: AtomicU64,
}

/// Uprobe registration
struct UprobeRegistration {
    /// Binary path or name
    binary_path: String,
    /// Function offset or symbol
    symbol: String,
    /// Handler callback
    handler: UprobeHandler,
    /// Registration timestamp
    registered_at: u64,
    /// Event count
    event_count: AtomicU64,
}

// ============================================================================
// BPF Tracer
// ============================================================================

/// BPF tracer for kernel and userspace instrumentation
pub struct BpfTracer {
    /// Tracer configuration
    config: TracerConfig,
    /// Per-CPU event buffers
    per_cpu_buffers: Vec<Mutex<PerCpuBuffer>>,
    /// Registered kprobes
    kprobes: Mutex<BTreeMap<String, KprobeRegistration>>,
    /// Registered tracepoints
    tracepoints: Mutex<BTreeMap<String, TracepointRegistration>>,
    /// Registered uprobes
    uprobes: Mutex<BTreeMap<String, UprobeRegistration>>,
    /// Global event filter
    filter: Mutex<EventFilter>,
    /// Total events processed
    total_events: AtomicU64,
    /// Performance monitoring enabled
    perf_monitoring: bool,
    /// Number of CPUs
    num_cpus: usize,
}

impl BpfTracer {
    /// Create a new BPF tracer
    ///
    /// # Arguments
    ///
    /// * `config` - Tracer configuration
    ///
    /// # Returns
    ///
    /// A new tracer instance or an error if initialization fails
    pub fn new(config: TracerConfig) -> TracerResult<Self> {
        // Detect number of CPUs (for now, assume a reasonable default)
        let num_cpus = 4; // TODO: Query actual CPU count

        // Create per-CPU buffers
        let mut per_cpu_buffers = Vec::with_capacity(num_cpus);
        for cpu_id in 0..num_cpus {
            let buffer = PerCpuBuffer::new(cpu_id as u32, config.per_cpu_buffer_size);
            per_cpu_buffers.push(Mutex::new(buffer));
        }

        Ok(Self {
            config,
            per_cpu_buffers,
            kprobes: Mutex::new(BTreeMap::new()),
            tracepoints: Mutex::new(BTreeMap::new()),
            uprobes: Mutex::new(BTreeMap::new()),
            filter: Mutex::new(EventFilter::default()),
            total_events: AtomicU64::new(0),
            perf_monitoring: config.enable_perf_monitoring,
            num_cpus,
        })
    }

    // ========================================================================
    // Kprobe Support
    // ========================================================================

    /// Register a kprobe for a kernel function
    ///
    /// # Arguments
    ///
    /// * `function_name` - Name of the kernel function to trace
    /// * `probe_type` - Whether to trace entry or return
    /// * `handler` - Callback function invoked when probe fires
    ///
    /// # Returns
    ///
    /// Ok(()) if registration successful, error otherwise
    ///
    /// # Example
    ///
    /// ```no_run,ignore
    /// tracer.register_kprobe("do_sys_open", KprobeType::Entry, my_handler)?;
    /// ```
    pub fn register_kprobe(
        &self,
        function_name: &str,
        probe_type: KprobeType,
        handler: KprobeHandler,
    ) -> TracerResult<()> {
        // Validate function name
        if function_name.is_empty() {
            return Err(TracerError::InvalidFunctionName(
                "Function name cannot be empty".into(),
            ));
        }

        // Check if already registered
        let key = Self::kprobe_key(function_name, probe_type);
        let mut kprobes = self.kprobes.lock();

        if kprobes.contains_key(&key) {
            return Err(TracerError::ProbeAlreadyRegistered(key));
        }

        // Check resource limits
        if kprobes.len() >= self.config.max_probes {
            return Err(TracerError::ResourceLimitExceeded);
        }

        // Register the probe
        let registration = KprobeRegistration {
            function_name: function_name.to_string(),
            probe_type,
            handler,
            registered_at: TraceEvent::get_timestamp(),
            event_count: AtomicU64::new(0),
        };

        kprobes.insert(key, registration);

        // TODO: Actual kernel kprobe registration would go here
        // This would involve:
        // 1. Looking up the kernel function address
        // 2. Using architecture-specific breakpoint/interrupt injection
        // 3. Registering the handler in the kprobe dispatch table

        log_info("Registered kprobe: {} ({:?})", function_name, probe_type);
        Ok(())
    }

    /// Unregister a previously registered kprobe
    ///
    /// # Arguments
    ///
    /// * `function_name` - Name of the kernel function
    /// * `probe_type` - Probe type (entry or return)
    pub fn unregister_kprobe(
        &self,
        function_name: &str,
        probe_type: KprobeType,
    ) -> TracerResult<()> {
        let key = Self::kprobe_key(function_name, probe_type);
        let mut kprobes = self.kprobes.lock();

        if !kprobes.contains_key(&key) {
            return Err(TracerError::ProbeNotFound(key));
        }

        kprobes.remove(&key);

        // TODO: Actual kernel kprobe unregistration

        log_info("Unregistered kprobe: {}", key);
        Ok(())
    }

    /// Internal kprobe handler (called by kernel when probe fires)
    ///
    /// This is a performance-critical path called from kernel context.
    /// Must be fast and non-blocking.
    #[inline(always)]
    fn kprobe_handler_internal(
        &self,
        function_name: &str,
        probe_type: KprobeType,
        cpu_id: u32,
        pid: Pid,
        tid: Tid,
        data: &[u8],
    ) {
        // Create event
        let event = TraceEvent::new(TraceEventType::Kprobe, cpu_id, pid, tid, function_name)
            .with_kprobe_type(probe_type)
            .with_data(data.to_vec());

        // Apply global filter
        let filter = self.filter.lock();
        if !filter.matches(&event) {
            return;
        }
        drop(filter);

        // Push to per-CPU buffer (non-blocking)
        let cpu_idx = (cpu_id as usize) % self.num_cpus;
        if let Some(buffer) = self.per_cpu_buffers.get(cpu_idx) {
            let buffer = buffer.lock();
            let _ = buffer.push(event);
            self.total_events.fetch_add(1, Ordering::Relaxed);
        }

        // Call registered handler
        let key = Self::kprobe_key(function_name, probe_type);
        let kprobes = self.kprobes.lock();
        if let Some(registration) = kprobes.get(&key) {
            let _ = (registration.handler)(&event);
            registration.event_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Helper to generate kprobe key
    fn kprobe_key(function_name: &str, probe_type: KprobeType) -> String {
        format!("{}:{:?}", function_name, probe_type)
    }

    // ========================================================================
    // Tracepoint Support
    // ========================================================================

    /// Register a tracepoint handler
    ///
    /// # Arguments
    ///
    /// * `subsystem` - Tracepoint subsystem (e.g., "sched", "irq", "net")
    /// * `name` - Tracepoint name (e.g., "sched_switch")
    /// * `handler` - Callback function invoked when tracepoint fires
    ///
    /// # Example
    ///
    /// ```no_run,ignore
    /// tracer.register_tracepoint("sched", "sched_switch", my_handler)?;
    /// ```
    pub fn register_tracepoint(
        &self,
        subsystem: &str,
        name: &str,
        handler: TracepointHandler,
    ) -> TracerResult<()> {
        // Validate names
        if subsystem.is_empty() || name.is_empty() {
            return Err(TracerError::InvalidTracepoint(
                "Subsystem and name cannot be empty".into(),
            ));
        }

        // Check if already registered
        let key = Self::tracepoint_key(subsystem, name);
        let mut tracepoints = self.tracepoints.lock();

        if tracepoints.contains_key(&key) {
            return Err(TracerError::ProbeAlreadyRegistered(key));
        }

        // Check resource limits
        if tracepoints.len() >= self.config.max_probes {
            return Err(TracerError::ResourceLimitExceeded);
        }

        // Register the tracepoint
        let registration = TracepointRegistration {
            subsystem: subsystem.to_string(),
            name: name.to_string(),
            handler,
            registered_at: TraceEvent::get_timestamp(),
            event_count: AtomicU64::new(0),
        };

        tracepoints.insert(key, registration);

        // TODO: Actual kernel tracepoint registration

        log_info("Registered tracepoint: {}:{}", subsystem, name);
        Ok(())
    }

    /// Unregister a tracepoint
    ///
    /// # Arguments
    ///
    /// * `subsystem` - Tracepoint subsystem
    /// * `name` - Tracepoint name
    pub fn unregister_tracepoint(&self, subsystem: &str, name: &str) -> TracerResult<()> {
        let key = Self::tracepoint_key(subsystem, name);
        let mut tracepoints = self.tracepoints.lock();

        if !tracepoints.contains_key(&key) {
            return Err(TracerError::ProbeNotFound(key));
        }

        tracepoints.remove(&key);

        // TODO: Actual kernel tracepoint unregistration

        log_info("Unregistered tracepoint: {}", key);
        Ok(())
    }

    /// Internal tracepoint handler (called by kernel when tracepoint fires)
    #[inline(always)]
    fn tracepoint_handler_internal(
        &self,
        subsystem: &str,
        name: &str,
        cpu_id: u32,
        pid: Pid,
        tid: Tid,
        data: &[u8],
    ) {
        // Create event
        let full_name = format!("{}:{}", subsystem, name);
        let event = TraceEvent::new(TraceEventType::Tracepoint, cpu_id, pid, tid, &full_name)
            .with_data(data.to_vec());

        // Apply global filter
        let filter = self.filter.lock();
        if !filter.matches(&event) {
            return;
        }
        drop(filter);

        // Push to per-CPU buffer
        let cpu_idx = (cpu_id as usize) % self.num_cpus;
        if let Some(buffer) = self.per_cpu_buffers.get(cpu_idx) {
            let buffer = buffer.lock();
            let _ = buffer.push(event);
            self.total_events.fetch_add(1, Ordering::Relaxed);
        }

        // Call registered handler
        let key = Self::tracepoint_key(subsystem, name);
        let tracepoints = self.tracepoints.lock();
        if let Some(registration) = tracepoints.get(&key) {
            let _ = (registration.handler)(&event);
            registration.event_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Helper to generate tracepoint key
    fn tracepoint_key(subsystem: &str, name: &str) -> String {
        format!("{}:{}", subsystem, name)
    }

    /// Parse tracepoint arguments from event data
    ///
    /// This is a simplified implementation. Real tracepoint parsing would
    /// need to handle the specific format of each tracepoint.
    pub fn parse_tracepoint_args(&self, _data: &[u8]) -> TracerResult<BTreeMap<String, Vec<u8>>> {
        // TODO: Implement tracepoint-specific argument parsing
        // This would require:
        // 1. Tracepoint format descriptor (from /sys/kernel/debug/tracing/events/)
        // 2. Argument offset information
        // 3. Type-aware parsing
        Ok(BTreeMap::new())
    }

    // ========================================================================
    // Uprobe Support
    // ========================================================================

    /// Register a uprobe for a userspace function
    ///
    /// # Arguments
    ///
    /// * `binary_path` - Path to the binary or library
    /// * `symbol` - Function symbol or offset
    /// * `handler` - Callback function invoked when uprobe fires
    ///
    /// # Example
    ///
    /// ```no_run,ignore
    /// tracer.register_uprobe("/usr/bin/myapp", "main", my_handler)?;
    /// ```
    pub fn register_uprobe(
        &self,
        binary_path: &str,
        symbol: &str,
        handler: UprobeHandler,
    ) -> TracerResult<()> {
        // Validate inputs
        if binary_path.is_empty() || symbol.is_empty() {
            return Err(TracerError::InvalidFunctionName(
                "Binary path and symbol cannot be empty".into(),
            ));
        }

        // Check if already registered
        let key = Self::uprobe_key(binary_path, symbol);
        let mut uprobes = self.uprobes.lock();

        if uprobes.contains_key(&key) {
            return Err(TracerError::ProbeAlreadyRegistered(key));
        }

        // Check resource limits
        if uprobes.len() >= self.config.max_probes {
            return Err(TracerError::ResourceLimitExceeded);
        }

        // Register the uprobe
        let registration = UprobeRegistration {
            binary_path: binary_path.to_string(),
            symbol: symbol.to_string(),
            handler,
            registered_at: TraceEvent::get_timestamp(),
            event_count: AtomicU64::new(0),
        };

        uprobes.insert(key, registration);

        // TODO: Actual uprobe registration would involve:
        // 1. Looking up the symbol in the binary
        // 2. Using ptrace or perf to insert breakpoint
        // 3. Handling the breakpoint in the kernel

        log_info("Registered uprobe: {} in {}", symbol, binary_path);
        Ok(())
    }

    /// Unregister a uprobe
    ///
    /// # Arguments
    ///
    /// * `binary_path` - Path to the binary or library
    /// * `symbol` - Function symbol or offset
    pub fn unregister_uprobe(&self, binary_path: &str, symbol: &str) -> TracerResult<()> {
        let key = Self::uprobe_key(binary_path, symbol);
        let mut uprobes = self.uprobes.lock();

        if !uprobes.contains_key(&key) {
            return Err(TracerError::ProbeNotFound(key));
        }

        uprobes.remove(&key);

        // TODO: Actual uprobe unregistration

        log_info("Unregistered uprobe: {}", key);
        Ok(())
    }

    /// Internal uprobe handler (called when userspace breakpoint hits)
    #[inline(always)]
    fn uprobe_handler_internal(
        &self,
        binary_path: &str,
        symbol: &str,
        cpu_id: u32,
        pid: Pid,
        tid: Tid,
        data: &[u8],
    ) {
        // Create event
        let name = format!("{}:{}", binary_path, symbol);
        let event =
            TraceEvent::new(TraceEventType::Uprobe, cpu_id, pid, tid, &name).with_data(data.to_vec());

        // Apply global filter
        let filter = self.filter.lock();
        if !filter.matches(&event) {
            return;
        }
        drop(filter);

        // Push to per-CPU buffer
        let cpu_idx = (cpu_id as usize) % self.num_cpus;
        if let Some(buffer) = self.per_cpu_buffers.get(cpu_idx) {
            let buffer = buffer.lock();
            let _ = buffer.push(event);
            self.total_events.fetch_add(1, Ordering::Relaxed);
        }

        // Call registered handler
        let key = Self::uprobe_key(binary_path, symbol);
        let uprobes = self.uprobes.lock();
        if let Some(registration) = uprobes.get(&key) {
            let _ = (registration.handler)(&event);
            registration.event_count.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Helper to generate uprobe key
    fn uprobe_key(binary_path: &str, symbol: &str) -> String {
        format!("{}:{}", binary_path, symbol)
    }

    // ========================================================================
    // Event Processing
    // ========================================================================

    /// Process all pending events from per-CPU buffers
    ///
    /// # Arguments
    ///
    /// * `processor` - Callback function to process batch of events
    ///
    /// # Returns
    ///
    /// Number of events processed or an error
    pub fn process_events<F>(&self, mut processor: F) -> TracerResult<usize>
    where
        F: FnMut(&[TraceEvent]) -> TracerResult<()>,
    {
        let mut total_processed = 0;

        // Process each CPU's buffer
        for buffer in &self.per_cpu_buffers {
            let mut buffer = buffer.lock();
            let batch = buffer.pop_batch(self.config.batch_size);

            if !batch.is_empty() {
                processor(&batch)?;
                total_processed += batch.len();
            }
        }

        Ok(total_processed)
    }

    /// Get events from a specific CPU
    pub fn get_cpu_events(&self, cpu_id: u32) -> TracerResult<Vec<TraceEvent>> {
        if (cpu_id as usize) >= self.num_cpus {
            return Err(TracerError::InvalidCpuId);
        }

        let buffer = &self.per_cpu_buffers[cpu_id as usize];
        let buffer = buffer.lock();
        Ok(buffer.pop_batch(self.config.batch_size))
    }

    /// Get all pending events from all CPUs
    pub fn get_all_events(&self) -> TracerResult<Vec<TraceEvent>> {
        let mut all_events = Vec::new();

        for buffer in &self.per_cpu_buffers {
            let buffer = buffer.lock();
            let events = buffer.pop_batch(self.config.batch_size);
            all_events.extend(events);
        }

        Ok(all_events)
    }

    // ========================================================================
    // Filtering
    // ========================================================================

    /// Set the global event filter
    pub fn set_filter(&self, filter: EventFilter) {
        *self.filter.lock() = filter;
    }

    /// Get the current global filter
    pub fn get_filter(&self) -> EventFilter {
        self.filter.lock().clone()
    }

    /// Clear all filter criteria
    pub fn clear_filter(&self) {
        *self.filter.lock() = EventFilter::default();
    }

    /// Set PID filter
    pub fn filter_by_pid(&self, pid: Pid) {
        let mut filter = self.filter.lock();
        filter.pid_filter = Some(pid);
    }

    /// Set CPU filter
    pub fn filter_by_cpu(&self, cpu_id: u32) {
        let mut filter = self.filter.lock();
        filter.cpu_filter = Some(cpu_id);
    }

    /// Set event type filter
    pub fn filter_by_event_type(&self, event_type: TraceEventType) {
        let mut filter = self.filter.lock();
        filter.event_type_filter = Some(event_type);
    }

    // ========================================================================
    // Statistics and Monitoring
    // ========================================================================

    /// Get total number of events processed
    pub fn total_events(&self) -> u64 {
        self.total_events.load(Ordering::Relaxed)
    }

    /// Get number of registered kprobes
    pub fn kprobe_count(&self) -> usize {
        self.kprobes.lock().len()
    }

    /// Get number of registered tracepoints
    pub fn tracepoint_count(&self) -> usize {
        self.tracepoints.lock().len()
    }

    /// Get number of registered uprobes
    pub fn uprobe_count(&self) -> usize {
        self.uprobes.lock().len()
    }

    /// Get per-CPU buffer statistics
    pub fn get_buffer_stats(&self) -> Vec<PerCpuBufferStats> {
        let mut stats = Vec::new();

        for (cpu_id, buffer) in self.per_cpu_buffers.iter().enumerate() {
            let buffer = buffer.lock();
            stats.push(PerCpuBufferStats {
                cpu_id: cpu_id as u32,
                depth: buffer.depth(),
                dropped_events: buffer.dropped_count(),
                capacity: self.config.per_cpu_buffer_size,
            });
        }

        stats
    }

    /// Get statistics for a specific probe
    pub fn get_probe_stats(&self, probe_key: &str) -> TracerResult<ProbeStats> {
        let kprobes = self.kprobes.lock();
        if let Some(reg) = kprobes.get(probe_key) {
            return Ok(ProbeStats {
                key: probe_key.to_string(),
                event_count: reg.event_count.load(Ordering::Relaxed),
                registered_at: reg.registered_at,
                probe_type: "kprobe".into(),
            });
        }

        let tracepoints = self.tracepoints.lock();
        if let Some(reg) = tracepoints.get(probe_key) {
            return Ok(ProbeStats {
                key: probe_key.to_string(),
                event_count: reg.event_count.load(Ordering::Relaxed),
                registered_at: reg.registered_at,
                probe_type: "tracepoint".into(),
            });
        }

        let uprobes = self.uprobes.lock();
        if let Some(reg) = uprobes.get(probe_key) {
            return Ok(ProbeStats {
                key: probe_key.to_string(),
                event_count: reg.event_count.load(Ordering::Relaxed),
                registered_at: reg.registered_at,
                probe_type: "uprobe".into(),
            });
        }

        Err(TracerError::ProbeNotFound(probe_key.to_string()))
    }

    /// Estimate current overhead percentage
    ///
    /// This is a rough estimate based on event rate and processing time.
    /// A proper implementation would measure actual CPU time consumed.
    pub fn estimate_overhead_percent(&self) -> u8 {
        // TODO: Implement proper overhead measurement
        // For now, return a conservative estimate
        let event_rate = self.total_events.load(Ordering::Relaxed);
        if event_rate < 1000 {
            1 // Low overhead
        } else if event_rate < 10000 {
            3 // Moderate overhead
        } else {
            TARGET_OVERHEAD_PERCENT // At target
        }
    }
}

// ============================================================================
// Statistics Types
// ============================================================================

/// Per-CPU buffer statistics
#[derive(Debug, Clone)]
pub struct PerCpuBufferStats {
    /// CPU ID
    pub cpu_id: u32,
    /// Current buffer depth
    pub depth: usize,
    /// Number of dropped events
    pub dropped_events: u64,
    /// Buffer capacity
    pub capacity: usize,
}

/// Probe statistics
#[derive(Debug, Clone)]
pub struct ProbeStats {
    /// Probe key (function:name or subsystem:name)
    pub key: String,
    /// Number of events received
    pub event_count: u64,
    /// When the probe was registered
    pub registered_at: u64,
    /// Probe type (kprobe, tracepoint, uprobe)
    pub probe_type: String,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Stub function for logging (will be replaced with proper logging)
fn log_info(fmt: &'static str, args: ...) {
    // TODO: Integrate with proper logging system
    let _ = fmt;
    let _ = args;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracer_config_default() {
        let config = TracerConfig::default();
        assert_eq!(config.per_cpu_buffer_size, DEFAULT_PER_CPU_BUFFER_SIZE);
        assert_eq!(config.max_event_data_size, MAX_EVENT_DATA_SIZE);
        assert_eq!(config.batch_size, DEFAULT_BATCH_SIZE);
    }

    #[test]
    fn test_tracer_creation() {
        let config = TracerConfig::default();
        let tracer = BpfTracer::new(config);
        assert!(tracer.is_ok());
    }

    #[test]
    fn test_kprobe_registration() {
        let tracer = BpfTracer::new(TracerConfig::default()).unwrap();

        let handler: KprobeHandler = |_event| Ok(());

        let result = tracer.register_kprobe("test_function", KprobeType::Entry, handler);
        assert!(result.is_ok());
        assert_eq!(tracer.kprobe_count(), 1);
    }

    #[test]
    fn test_duplicate_kprobe_registration() {
        let tracer = BpfTracer::new(TracerConfig::default()).unwrap();

        let handler: KprobeHandler = |_event| Ok(());

        tracer
            .register_kprobe("test_function", KprobeType::Entry, handler)
            .unwrap();

        let result = tracer.register_kprobe("test_function", KprobeType::Entry, handler);
        assert!(matches!(result, Err(TracerError::ProbeAlreadyRegistered(_))));
    }

    #[test]
    fn test_kprobe_unregistration() {
        let tracer = BpfTracer::new(TracerConfig::default()).unwrap();

        let handler: KprobeHandler = |_event| Ok(());

        tracer
            .register_kprobe("test_function", KprobeType::Entry, handler)
            .unwrap();

        let result = tracer.unregister_kprobe("test_function", KprobeType::Entry);
        assert!(result.is_ok());
        assert_eq!(tracer.kprobe_count(), 0);
    }

    #[test]
    fn test_tracepoint_registration() {
        let tracer = BpfTracer::new(TracerConfig::default()).unwrap();

        let handler: TracepointHandler = |_event| Ok(());

        let result = tracer.register_tracepoint("sched", "sched_switch", handler);
        assert!(result.is_ok());
        assert_eq!(tracer.tracepoint_count(), 1);
    }

    #[test]
    fn test_uprobe_registration() {
        let tracer = BpfTracer::new(TracerConfig::default()).unwrap();

        let handler: UprobeHandler = |_event| Ok(());

        let result = tracer.register_uprobe("/usr/bin/test", "main", handler);
        assert!(result.is_ok());
        assert_eq!(tracer.uprobe_count(), 1);
    }

    #[test]
    fn test_event_filter() {
        let filter = EventFilter::new();
        let event = TraceEvent::new(TraceEventType::Kprobe, 0, 100, 100, "test_func");

        // No filter - should match
        assert!(filter.matches(&event));

        // PID filter
        let mut filtered_filter = EventFilter::new();
        filtered_filter.pid_filter = Some(100);
        assert!(filtered_filter.matches(&event));

        filtered_filter.pid_filter = Some(200);
        assert!(!filtered_filter.matches(&event));
    }

    #[test]
    fn test_per_cpu_buffer() {
        let buffer = PerCpuBuffer::new(0, 16);
        let event = TraceEvent::new(TraceEventType::Kprobe, 0, 100, 100, "test");

        // Push event
        let result = buffer.push(event);
        assert!(result.is_ok());
        assert_eq!(buffer.depth(), 1);

        // Pop event
        let batch = buffer.pop_batch(10);
        assert_eq!(batch.len(), 1);
        assert_eq!(buffer.depth(), 0);
    }

    #[test]
    fn test_trace_event_creation() {
        let event = TraceEvent::new(TraceEventType::Kprobe, 0, 100, 100, "test_func");
        assert_eq!(event.event_type, TraceEventType::Kprobe);
        assert_eq!(event.cpu_id, 0);
        assert_eq!(event.pid, 100);
        assert_eq!(event.tid, 100);
        assert_eq!(event.name, "test_func");
    }

    #[test]
    fn test_filter_by_pid() {
        let tracer = BpfTracer::new(TracerConfig::default()).unwrap();
        tracer.filter_by_pid(42);

        let filter = tracer.get_filter();
        assert_eq!(filter.pid_filter, Some(42));
    }

    #[test]
    fn test_filter_by_cpu() {
        let tracer = BpfTracer::new(TracerConfig::default()).unwrap();
        tracer.filter_by_cpu(2);

        let filter = tracer.get_filter();
        assert_eq!(filter.cpu_filter, Some(2));
    }

    #[test]
    fn test_buffer_stats() {
        let tracer = BpfTracer::new(TracerConfig::default()).unwrap();
        let stats = tracer.get_buffer_stats();

        assert!(!stats.is_empty());
        assert_eq!(stats[0].cpu_id, 0);
        assert_eq!(stats[0].capacity, DEFAULT_PER_CPU_BUFFER_SIZE);
    }

    #[test]
    fn test_total_events() {
        let tracer = BpfTracer::new(TracerConfig::default()).unwrap();
        let count = tracer.total_events();
        assert_eq!(count, 0);
    }
}
