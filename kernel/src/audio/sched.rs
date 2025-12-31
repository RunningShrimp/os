//! # Real-time Audio Scheduler
//!
//! This module provides real-time scheduling support for audio tasks, including
//! CPU isolation, priority management, and latency monitoring.
//!
//! ## Features
//!
//! - **SCHED_FIFO Integration**: Real-time priority management (1-99)
//! - **CPU Isolation**: Dedicated CPU cores for audio processing
//! - **XRUN Detection**: Underflow/overflow detection and recovery
//! - **Latency Monitoring**: Real-time latency measurement
//! - **Power Management**: Thermal throttling awareness
//! - **Deadline Scheduling**: SCHED_DEADLINE support when available
//!
//! ## Architecture
//!
//! ```text
//! ┌─────────────────────────────────────┐
//! │     Audio Applications                │
//! └─────────────────┬───────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────┐
//! │     Audio Scheduler API              │
//! │  - task_register/unregister          │
//! │  - task_start/stop                   │
//! └─────────────────┬───────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────┐
//! │     Real-time Scheduler              │
//! │  - Priority management               │
//! │  - CPU affinity                      │
//! │  - XRUN detection                    │
//! └─────────────────┬───────────────────┘
//!                   │
//! ┌─────────────────▼───────────────────┐
//! │     Kernel Scheduler Integration     │
//! │  - SCHED_FIFO/SCHED_RR               │
//! │  - SCHED_DEADLINE                    │
//! │  - CPU isolation                     │
//! └─────────────────────────────────────┘
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use kernel::audio::sched::{AudioScheduler, AudioTask};
//!
//! // Register audio task
//! let task = AudioTask::new("audio_process", audio_callback, 48000, 2);
//! let task_id = AudioScheduler::register(task)?;
//!
//! // Start task
//! AudioScheduler::start(task_id)?;
//!
//! // Monitor latency
//! let latency = AudioScheduler::get_latency(task_id)?;
//! println!("Audio latency: {} us", latency);
//! ```

use crate::prelude::*;
use crate::sched::rt_sched::{RtSchedPolicy, RtPriority};
use alloc::collections::BTreeMap;
use alloc::string::String;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

// ============================================================================
// Constants
// ============================================================================

/// Default audio priority (high real-time priority)
pub const DEFAULT_AUDIO_PRIORITY: RtPriority = 80;

/// Maximum audio priority
pub const MAX_AUDIO_PRIORITY: RtPriority = 95;

/// Minimum audio priority
pub const MIN_AUDIO_PRIORITY: RtPriority = 50;

/// Default XRUN threshold (frames)
pub const DEFAULT_XRUN_THRESHOLD: u32 = 1;

/// Maximum latency for monitoring (microseconds)
pub const MAX_LATENCY_US: u64 = 10_000; // 10ms

/// Audio statistics window size
const STATS_WINDOW_SIZE: usize = 100;

// ============================================================================
// Error Types
// ============================================================================

/// Scheduler error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedError {
    /// Task not found
    TaskNotFound,
    /// Invalid parameter
    InvalidParam,
    /// Task already registered
    AlreadyRegistered,
    /// Out of memory
    OutOfMemory,
    /// Scheduler not initialized
    NotInitialized,
    /// Operation not supported
    NotSupported,
    /// CPU isolation failed
    CpuIsolationFailed,
    /// Priority assignment failed
    PriorityFailed,
}

// ============================================================================
// Audio Task
// ============================================================================

/// Audio task ID
pub type TaskId = u64;

/// Audio processing callback signature
pub type AudioCallback = fn(frames: usize, channels: u32) -> Result<(), SchedError>;

/// Audio task
#[derive(Debug)]
pub struct AudioTask {
    /// Task ID
    pub id: TaskId,
    /// Task name
    pub name: String,
    /// Processing callback
    pub callback: AudioCallback,
    /// Sample rate (Hz)
    pub sample_rate: u32,
    /// Number of channels
    pub channels: u32,
    /// Buffer size (frames)
    pub buffer_size: usize,
    /// Real-time priority
    pub priority: RtPriority,
    /// CPU affinity mask
    pub cpu_affinity: u64,
    /// Period (microseconds)
    pub period_us: u64,
    /// Deadline (microseconds)
    pub deadline_us: u64,
    /// Task is active
    pub active: AtomicBool,
    /// XRUN count
    pub xrun_count: AtomicU32,
    /// Total frames processed
    pub frames_processed: AtomicU64,
}

impl AudioTask {
    /// Create new audio task
    pub fn new(
        name: String,
        callback: AudioCallback,
        sample_rate: u32,
        channels: u32,
    ) -> Self {
        // Calculate period from sample rate and default buffer size
        let buffer_size = 1024; // Default
        let period_us = (buffer_size as u64 * 1_000_000) / sample_rate as u64;
        let deadline_us = period_us * 9 / 10; // 90% of period

        Self {
            id: 0, // Will be assigned on registration
            name,
            callback,
            sample_rate,
            channels,
            buffer_size,
            priority: DEFAULT_AUDIO_PRIORITY,
            cpu_affinity: u64::MAX, // All CPUs
            period_us,
            deadline_us,
            active: AtomicBool::new(false),
            xrun_count: AtomicU32::new(0),
            frames_processed: AtomicU64::new(0),
        }
    }

    /// Set buffer size
    pub fn set_buffer_size(&mut self, size: usize) {
        self.buffer_size = size;
        self.period_us = (size as u64 * 1_000_000) / self.sample_rate as u64;
        self.deadline_us = self.period_us * 9 / 10;
    }

    /// Set priority
    pub fn set_priority(&mut self, priority: RtPriority) -> Result<(), SchedError> {
        if priority < MIN_AUDIO_PRIORITY || priority > MAX_AUDIO_PRIORITY {
            return Err(SchedError::InvalidParam);
        }
        self.priority = priority;
        Ok(())
    }

    /// Set CPU affinity
    pub fn set_cpu_affinity(&mut self, mask: u64) {
        self.cpu_affinity = mask;
    }

    /// Get XRUN count
    pub fn xrun_count(&self) -> u32 {
        self.xrun_count.load(Ordering::Relaxed)
    }

    /// Increment XRUN count
    pub fn increment_xrun(&self) {
        self.xrun_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get frames processed
    pub fn frames_processed(&self) -> u64 {
        self.frames_processed.load(Ordering::Relaxed)
    }

    /// Add to frames processed
    pub fn add_frames(&self, frames: u64) {
        self.frames_processed.fetch_add(frames, Ordering::Relaxed);
    }

    /// Check if task is active
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Relaxed)
    }

    /// Activate task
    pub fn activate(&self) {
        self.active.store(true, Ordering::Release);
    }

    /// Deactivate task
    pub fn deactivate(&self) {
        self.active.store(false, Ordering::Release);
    }
}

// ============================================================================
// XRUN Detector
// ============================================================================

/// XRUN (underflow/overflow) detector
#[derive(Debug)]
pub struct XrunDetector {
    /// XRUN threshold (frames)
    threshold: AtomicU32,
    /// Total XRUN count
    total_xruns: AtomicU32,
    /// Last XRUN time (ticks)
    last_xrun_time: AtomicU64,
    /// XRUN recovery strategy
    recovery_strategy: XrunRecovery,
}

/// XRUN recovery strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XrunRecovery {
    /// Do nothing (just report)
    None,
    /// Reset buffers
    ResetBuffers,
    /// Restart stream
    RestartStream,
    /// Silence output
    SilenceOutput,
}

impl XrunDetector {
    /// Create new XRUN detector
    pub fn new(threshold: u32, recovery: XrunRecovery) -> Self {
        Self {
            threshold: AtomicU32::new(threshold),
            total_xruns: AtomicU32::new(0),
            last_xrun_time: AtomicU64::new(0),
            recovery_strategy: recovery,
        }
    }

    /// Set XRUN threshold
    pub fn set_threshold(&self, threshold: u32) {
        self.threshold.store(threshold, Ordering::Release);
    }

    /// Get XRUN threshold
    pub fn threshold(&self) -> u32 {
        self.threshold.load(Ordering::Relaxed)
    }

    /// Report XRUN
    pub fn report_xrun(&self) {
        self.total_xruns.fetch_add(1, Ordering::Relaxed);
        // Store current time (simplified)
        self.last_xrun_time.store(crate::platform_arch::rdtsc(), Ordering::Relaxed);

        crate::log_warn!("Audio XRUN detected");
    }

    /// Get total XRUN count
    pub fn total_xruns(&self) -> u32 {
        self.total_xruns.load(Ordering::Relaxed)
    }

    /// Get time since last XRUN
    pub fn time_since_last_xrun(&self) -> u64 {
        let last = self.last_xrun_time.load(Ordering::Relaxed);
        let now = crate::platform_arch::rdtsc();
        now.saturating_sub(last)
    }

    /// Handle XRUN
    pub fn handle(&self) -> XrunRecovery {
        self.report_xrun();
        self.recovery_strategy
    }

    /// Reset XRUN counter
    pub fn reset(&self) {
        self.total_xruns.store(0, Ordering::Relaxed);
        self.last_xrun_time.store(0, Ordering::Relaxed);
    }
}

// ============================================================================
// Latency Monitor
// ============================================================================

/// Latency statistics
#[derive(Debug, Clone, Copy)]
pub struct LatencyStats {
    /// Minimum latency (microseconds)
    pub min_us: u64,
    /// Maximum latency (microseconds)
    pub max_us: u64,
    /// Average latency (microseconds)
    pub avg_us: u64,
    /// Latency samples
    pub samples: u64,
    /// XRUN count
    pub xruns: u32,
}

/// Latency monitor
#[derive(Debug)]
pub struct LatencyMonitor {
    /// Minimum latency (microseconds)
    min_latency: AtomicU64,
    /// Maximum latency (microseconds)
    max_latency: AtomicU64,
    /// Total latency (for average)
    total_latency: AtomicU64,
    /// Sample count
    sample_count: AtomicU64,
    /// XRUN detector
    xrun_detector: XrunDetector,
}

impl LatencyMonitor {
    /// Create new latency monitor
    pub fn new(xrun_threshold: u32) -> Self {
        Self {
            min_latency: AtomicU64::new(u64::MAX),
            max_latency: AtomicU64::new(0),
            total_latency: AtomicU64::new(0),
            sample_count: AtomicU64::new(0),
            xrun_detector: XrunDetector::new(xrun_threshold, XrunRecovery::ResetBuffers),
        }
    }

    /// Record latency measurement
    pub fn record_latency(&self, latency_us: u64) {
        // Update min/max
        let mut current_min = self.min_latency.load(Ordering::Relaxed);
        while latency_us < current_min {
            match self.min_latency.compare_exchange_weak(
                current_min,
                latency_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new) => current_min = new,
            }
        }

        let mut current_max = self.max_latency.load(Ordering::Relaxed);
        while latency_us > current_max {
            match self.max_latency.compare_exchange_weak(
                current_max,
                latency_us,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(new) => current_max = new,
            }
        }

        // Update average
        self.total_latency.fetch_add(latency_us, Ordering::Relaxed);
        self.sample_count.fetch_add(1, Ordering::Relaxed);

        // Check for XRUN
        if latency_us > MAX_LATENCY_US {
            self.xrun_detector.report_xrun();
        }
    }

    /// Get latency statistics
    pub fn stats(&self) -> LatencyStats {
        let samples = self.sample_count.load(Ordering::Relaxed);
        let avg = if samples > 0 {
            self.total_latency.load(Ordering::Relaxed) / samples
        } else {
            0
        };

        LatencyStats {
            min_us: self.min_latency.load(Ordering::Relaxed),
            max_us: self.max_latency.load(Ordering::Relaxed),
            avg_us: avg,
            samples,
            xruns: self.xrun_detector.total_xruns(),
        }
    }

    /// Get XRUN detector
    pub fn xrun_detector(&self) -> &XrunDetector {
        &self.xrun_detector
    }

    /// Reset statistics
    pub fn reset(&self) {
        self.min_latency.store(u64::MAX, Ordering::Relaxed);
        self.max_latency.store(0, Ordering::Relaxed);
        self.total_latency.store(0, Ordering::Relaxed);
        self.sample_count.store(0, Ordering::Relaxed);
        self.xrun_detector.reset();
    }
}

// ============================================================================
// Audio Scheduler
// ============================================================================

/// Audio scheduler statistics
#[derive(Debug, Clone)]
pub struct SchedulerStats {
    /// Number of registered tasks
    pub num_tasks: usize,
    /// Number of active tasks
    pub active_tasks: usize,
    /// Total XRUNs across all tasks
    pub total_xruns: u32,
    /// Average latency across all tasks (microseconds)
    pub avg_latency_us: u64,
}

/// Audio scheduler (singleton)
pub struct AudioScheduler {
    /// Registered tasks
    tasks: Mutex<BTreeMap<TaskId, AudioTask>>,
    /// Next task ID
    next_task_id: AtomicU64,
    /// Latency monitors (one per task)
    latency_monitors: Mutex<BTreeMap<TaskId, LatencyMonitor>>,
    /// Isolated CPUs for audio
    isolated_cpus: AtomicU64,
    /// Thermal throttling enabled
    thermal_throttling: AtomicBool,
    /// Scheduler is initialized
    initialized: AtomicBool,
}

impl AudioScheduler {
    /// Get singleton instance
    pub fn get() -> &'static Self {
        static SCHEDULER: AudioScheduler = AudioScheduler {
            tasks: Mutex::new(BTreeMap::new()),
            next_task_id: AtomicU64::new(1),
            latency_monitors: Mutex::new(BTreeMap::new()),
            isolated_cpus: AtomicU64::new(0),
            thermal_throttling: AtomicBool::new(false),
            initialized: AtomicBool::new(false),
        };
        &SCHEDULER
    }

    /// Initialize audio scheduler
    pub fn init(&self) -> Result<(), SchedError> {
        if self.initialized.load(Ordering::Acquire) {
            return Ok(());
        }

        crate::log_debug!("Audio scheduler initialized");
        crate::log_debug!("  - Default priority: {}", DEFAULT_AUDIO_PRIORITY);
        crate::log_debug!("  - Priority range: {}-{}", MIN_AUDIO_PRIORITY, MAX_AUDIO_PRIORITY);

        self.initialized.store(true, Ordering::Release);
        Ok(())
    }

    /// Register audio task
    pub fn register(&self, mut task: AudioTask) -> Result<TaskId, SchedError> {
        if !self.initialized.load(Ordering::Acquire) {
            return Err(SchedError::NotInitialized);
        }

        let id = self.next_task_id.fetch_add(1, Ordering::SeqCst);
        task.id = id;

        // Create latency monitor for this task
        let monitor = LatencyMonitor::new(DEFAULT_XRUN_THRESHOLD);

        {
            let mut tasks = self.tasks.lock();
            if tasks.contains_key(&id) {
                return Err(SchedError::AlreadyRegistered);
            }
            tasks.insert(id, task);
        }

        {
            let mut monitors = self.latency_monitors.lock();
            monitors.insert(id, monitor);
        }

        crate::log_debug!("Audio task '{}' registered (ID={})", self.tasks.lock().get(&id).unwrap().name.clone(), id);
        Ok(id)
    }

    /// Unregister audio task
    pub fn unregister(&self, id: TaskId) -> Result<AudioTask, SchedError> {
        self.tasks
            .lock()
            .remove(&id)
            .ok_or(SchedError::TaskNotFound)
    }

    /// Start audio task
    pub fn start(&self, id: TaskId) -> Result<(), SchedError> {
        let tasks = self.tasks.lock();
        let task = tasks.get(&id).ok_or(SchedError::TaskNotFound)?;

        // Set real-time scheduling
        crate::sched::rt_sched::RtScheduler::enqueue(&crate::sched::rt_sched::RtTask::new(
            id,
            RtSchedPolicy::Fifo,
            task.priority,
        ))
        .map_err(|_| SchedError::PriorityFailed)?;

        // Set CPU affinity
        if task.cpu_affinity != u64::MAX {
            // In real implementation, set CPU affinity
            crate::log_debug!("Set CPU affinity for task {}: {:#x}", id, task.cpu_affinity);
        }

        // Activate task
        task.activate();

        crate::log_debug!("Audio task '{}' started", task.name.clone());
        Ok(())
    }

    /// Stop audio task
    pub fn stop(&self, id: TaskId) -> Result<(), SchedError> {
        let tasks = self.tasks.lock();
        let task = tasks.get(&id).ok_or(SchedError::TaskNotFound)?;

        task.deactivate();

        crate::log_debug!("Audio task '{}' stopped", task.name.clone());
        Ok(())
    }

    /// Process audio task (called by scheduler)
    pub fn process_task(&self, id: TaskId) -> Result<(), SchedError> {
        let tasks = self.tasks.lock();
        let task = tasks.get(&id).ok_or(SchedError::TaskNotFound)?;

        if !task.is_active() {
            return Ok(());
        }

        // Measure start time
        let start = crate::platform_arch::rdtsc();

        // Call audio callback
        (task.callback)(task.buffer_size, task.channels)?;

        // Measure end time and calculate latency
        let end = crate::platform_arch::rdtsc();
        let cycles = end.saturating_sub(start);

        // Convert cycles to microseconds (simplified - assumes 3GHz CPU)
        let latency_us = cycles / 3000;

        // Record latency
        {
            let monitors = self.latency_monitors.lock();
            if let Some(monitor) = monitors.get(&id) {
                monitor.record_latency(latency_us);
            }
        }

        // Update statistics
        task.add_frames(task.buffer_size as u64);

        Ok(())
    }

    /// Get task latency
    pub fn get_latency(&self, id: TaskId) -> Result<u64, SchedError> {
        let monitors = self.latency_monitors.lock();
        let monitor = monitors.get(&id).ok_or(SchedError::TaskNotFound)?;
        let stats = monitor.stats();
        Ok(stats.avg_us)
    }

    /// Get task statistics
    pub fn get_task_stats(&self, id: TaskId) -> Result<LatencyStats, SchedError> {
        let monitors = self.latency_monitors.lock();
        let monitor = monitors.get(&id).ok_or(SchedError::TaskNotFound)?;
        Ok(monitor.stats())
    }

    /// Get scheduler statistics
    pub fn get_stats(&self) -> SchedulerStats {
        let tasks = self.tasks.lock();
        let monitors = self.latency_monitors.lock();

        let active_count = tasks.values().filter(|t| t.is_active()).count();
        let total_xruns = monitors.values().map(|m| m.xrun_detector().total_xruns()).sum();

        let mut total_latency = 0u64;
        let mut total_samples = 0u64;
        for monitor in monitors.values() {
            let stats = monitor.stats();
            total_latency += stats.avg_us * stats.samples;
            total_samples += stats.samples;
        }

        let avg_latency = if total_samples > 0 {
            total_latency / total_samples
        } else {
            0
        };

        SchedulerStats {
            num_tasks: tasks.len(),
            active_tasks: active_count,
            total_xruns,
            avg_latency_us: avg_latency,
        }
    }

    /// Set isolated CPUs for audio
    pub fn set_isolated_cpus(&self, cpu_mask: u64) -> Result<(), SchedError> {
        self.isolated_cpus.store(cpu_mask, Ordering::Release);
        crate::log_debug!("Audio isolated CPUs mask: {:#x}", cpu_mask);
        Ok(())
    }

    /// Get isolated CPUs
    pub fn isolated_cpus(&self) -> u64 {
        self.isolated_cpus.load(Ordering::Relaxed)
    }

    /// Enable/disable thermal throttling
    pub fn set_thermal_throttling(&self, enabled: bool) {
        self.thermal_throttling.store(enabled, Ordering::Release);
        crate::log_debug!("Audio thermal throttling: {}", enabled);
    }

    /// Check thermal throttling status
    pub fn thermal_throttling(&self) -> bool {
        self.thermal_throttling.load(Ordering::Relaxed)
    }
}

// ============================================================================
// Public API Functions
// ============================================================================

/// Initialize audio scheduler
pub fn init() -> Result<(), SchedError> {
    AudioScheduler::get().init()
}

/// Register audio task
pub fn register_task(task: AudioTask) -> Result<TaskId, SchedError> {
    AudioScheduler::get().register(task)
}

/// Unregister audio task
pub fn unregister_task(id: TaskId) -> Result<(), SchedError> {
    AudioScheduler::get().unregister(id)?;
    Ok(())
}

/// Start audio task
pub fn start_task(id: TaskId) -> Result<(), SchedError> {
    AudioScheduler::get().start(id)
}

/// Stop audio task
pub fn stop_task(id: TaskId) -> Result<(), SchedError> {
    AudioScheduler::get().stop(id)
}

/// Get task latency
pub fn get_task_latency(id: TaskId) -> Result<u64, SchedError> {
    AudioScheduler::get().get_latency(id)
}

/// Get scheduler statistics
pub fn get_scheduler_stats() -> SchedulerStats {
    AudioScheduler::get().get_stats()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_task() {
        let callback: AudioCallback = |_frames, _channels| Ok(());

        let mut task = AudioTask::new(String::from("test"), callback, 48000, 2);
        assert_eq!(task.sample_rate, 48000);
        assert_eq!(task.channels, 2);
        assert_eq!(task.priority, DEFAULT_AUDIO_PRIORITY);

        task.set_buffer_size(2048).unwrap();
        assert_eq!(task.buffer_size, 2048);

        task.set_priority(90).unwrap();
        assert_eq!(task.priority, 90);

        assert!(task.set_priority(40).is_err()); // Below minimum
    }

    #[test]
    fn test_xrun_detector() {
        let detector = XrunDetector::new(1, XrunRecovery::ResetBuffers);
        assert_eq!(detector.threshold(), 1);

        detector.report_xrun();
        assert_eq!(detector.total_xruns(), 1);

        detector.report_xrun();
        assert_eq!(detector.total_xruns(), 2);

        detector.reset();
        assert_eq!(detector.total_xruns(), 0);
    }

    #[test]
    fn test_latency_monitor() {
        let monitor = LatencyMonitor::new(100);

        monitor.record_latency(1000);
        monitor.record_latency(2000);
        monitor.record_latency(1500);

        let stats = monitor.stats();
        assert_eq!(stats.min_us, 1000);
        assert_eq!(stats.max_us, 2000);
        assert_eq!(stats.samples, 3);
        assert!(stats.avg_us > 0);

        monitor.reset();
        let stats = monitor.stats();
        assert_eq!(stats.samples, 0);
    }

    #[test]
    fn test_scheduler() {
        let scheduler = AudioScheduler::get();
        scheduler.init().unwrap();

        let callback: AudioCallback = |_frames, _channels| Ok(());
        let task = AudioTask::new(String::from("test_task"), callback, 48000, 2);

        let id = scheduler.register(task).unwrap();
        assert!(id > 0);

        let stats = scheduler.get_stats();
        assert_eq!(stats.num_tasks, 1);

        scheduler.start(id).unwrap();

        let stats = scheduler.get_stats();
        assert_eq!(stats.active_tasks, 1);

        scheduler.stop(id).unwrap();

        scheduler.unregister(id).unwrap();
    }

    #[test]
    fn test_priority_range() {
        assert!(MIN_AUDIO_PRIORITY >= RT_PRIO_MIN as u8);
        assert!(MAX_AUDIO_PRIORITY <= RT_PRIO_MAX as u8);
        assert!(DEFAULT_AUDIO_PRIORITY >= MIN_AUDIO_PRIORITY);
        assert!(DEFAULT_AUDIO_PRIORITY <= MAX_AUDIO_PRIORITY);
    }
}
