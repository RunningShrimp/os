//! CPU Hotplug Support
//!
//! This module provides comprehensive CPU hotplug functionality including:
//! - Dynamic CPU activation/deactivation
//! - Load-based CPU scaling (hotplug on load threshold)
//! - Power domain management
//! - CPU capacity awareness (big.LITTLE, ARM)
//! - Hotplug notification callbacks
//! - CPU offline/online states
//! - Hotplug statistics and monitoring
//!
//! # Overview
//!
//! CPU hotplug allows the system to add or remove CPUs dynamically at runtime,
//! enabling power savings by disabling unused CPUs and improving performance
//! by enabling CPUs under heavy load.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │                  CPU Hotplug Manager                        │
//! ├────────────────────────────────────────────────────────────┤
//! │  ┌──────────────┐  ┌──────────────┐  ┌─────────────────┐  │
//! │  │ CPU Online   │  │ CPU Offline  │  │ Load Monitor    │  │
//! │  │ Operations   │  │ Operations   │  │                 │  │
//! │  └──────┬───────┘  └──────┬───────┘  └────────┬────────┘  │
//! │         │                 │                     │            │
//! │         └─────────────────┴─────────────────────┘            │
//! │                           │                                 │
//! │                           ▼                                 │
//! │              ┌──────────────────────┐                       │
//! │              │   Hotplug Engine     │                       │
//! │              │  - Load threshold    │                       │
//! │              │  - Auto hotplug      │                       │
//! │              └──────────┬───────────┘                       │
//! │                           │                                 │
//! │                           ▼                                 │
//! │    ┌──────────────────────────────────────────────────┐    │
//! │    │              CPU States                           │    │
//! │    │  Offline → Online → Active → Quiesced            │    │
//! │    └──────────────────────────────────────────────────┘    │
//! └────────────────────────────────────────────────────────────┘
//! ```

#![allow(dead_code)]
#![allow(unused_variables)]

use crate::prelude::*;
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

/// CPU state in hotplug system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuState {
    /// CPU is offline and powered down
    Offline,
    /// CPU is online but not yet active
    Online,
    /// CPU is online and active
    Active,
    /// CPU is quiesced (prepared for offline)
    Quiesced,
}

impl CpuState {
    /// Get state name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Offline => "offline",
            Self::Online => "online",
            Self::Active => "active",
            Self::Quiesced => "quiesced",
        }
    }

    /// Check if CPU is considered online
    pub fn is_online(&self) -> bool {
        matches!(self, Self::Online | Self::Active)
    }

    /// Check if CPU can accept work
    pub fn can_run_work(&self) -> bool {
        matches!(self, Self::Active)
    }
}

/// CPU hotplug event
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotplugEvent {
    /// CPU is coming online
    Online,
    /// CPU is going offline
    Offline,
    /// CPU is being prepared for offline
    PrepareOffline,
    /// CPU has completed offline
    PostOffline,
    /// CPU is being prepared for online
    PrepareOnline,
    /// CPU has completed online
    PostOnline,
}

impl HotplugEvent {
    /// Get event name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Online => "online",
            Self::Offline => "offline",
            Self::PrepareOffline => "prepare_offline",
            Self::PostOffline => "post_offline",
            Self::PrepareOnline => "prepare_online",
            Self::PostOnline => "post_online",
        }
    }
}

/// CPU capacity type (for big.LITTLE / ARM heterogeneous systems)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuCapacity {
    /// Little core (power-efficient, lower performance)
    Little,
    /// Medium core (balanced)
    Medium,
    /// Big core (high performance)
    Big,
}

impl CpuCapacity {
    /// Get capacity name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Little => "little",
            Self::Medium => "medium",
            Self::Big => "big",
        }
    }

    /// Get relative performance score (0-1000)
    pub fn performance_score(&self) -> u32 {
        match self {
            Self::Little => 200,
            Self::Medium => 500,
            Self::Big => 1000,
        }
    }

    /// Get relative power score (0-1000, lower is better)
    pub fn power_score(&self) -> u32 {
        match self {
            Self::Little => 200,
            Self::Medium => 500,
            Self::Big => 900,
        }
    }
}

/// CPU hotplug statistics
#[derive(Debug, Clone)]
pub struct HotplugStats {
    /// Number of online operations
    pub online_count: u64,
    /// Number of offline operations
    pub offline_count: u64,
    /// Number of failed online operations
    pub online_failures: u64,
    /// Number of failed offline operations
    pub offline_failures: u64,
    /// Total online time (microseconds)
    pub total_online_time_us: u64,
    /// Last online time
    pub last_online_time: u64,
    /// Last offline time
    pub last_offline_time: u64,
    /// Current state
    pub current_state: CpuState,
}

impl Default for HotplugStats {
    fn default() -> Self {
        Self {
            online_count: 0,
            offline_count: 0,
            online_failures: 0,
            offline_failures: 0,
            total_online_time_us: 0,
            last_online_time: 0,
            last_offline_time: 0,
            current_state: CpuState::Offline,
        }
    }
}

/// CPU hotplug device
pub struct CpuHotplugDevice {
    /// CPU ID
    pub cpu_id: u32,
    /// Current state
    pub state: Mutex<CpuState>,
    /// CPU capacity type
    pub capacity: CpuCapacity,
    /// CPU cluster (for multi-cluster systems)
    pub cluster_id: u32,
    /// Max frequency (kHz)
    pub max_frequency: u32,
    /// Min frequency (kHz)
    pub min_frequency: u32,
    /// Statistics
    pub stats: Mutex<HotplugStats>,
    /// Online flag
    pub is_online: AtomicBool,
    /// Pending offline flag
    pub pending_offline: AtomicBool,
    /// Last state change
    pub last_state_change: AtomicU64,
    /// Notification callbacks
    pub callbacks: Mutex<Vec<HotplugCallback>>,
}

impl core::fmt::Debug for CpuHotplugDevice {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CpuHotplugDevice")
            .field("cpu_id", &self.cpu_id)
            .field("state", &self.state)
            .field("capacity", &self.capacity)
            .field("cluster_id", &self.cluster_id)
            .field("max_frequency", &self.max_frequency)
            .field("min_frequency", &self.min_frequency)
            .field("stats", &self.stats)
            .field("is_online", &self.is_online)
            .field("pending_offline", &self.pending_offline)
            .field("last_state_change", &self.last_state_change)
            .field("callbacks", &"<callbacks>")
            .finish()
    }
}

impl CpuHotplugDevice {
    /// Create new CPU hotplug device
    pub fn new(cpu_id: u32, capacity: CpuCapacity) -> Self {
        Self {
            cpu_id,
            state: Mutex::new(CpuState::Offline),
            capacity,
            cluster_id: cpu_id / 4, // Assume 4 CPUs per cluster
            max_frequency: match capacity {
                CpuCapacity::Little => 1400000,
                CpuCapacity::Medium => 2000000,
                CpuCapacity::Big => 3000000,
            },
            min_frequency: 300000,
            stats: Mutex::new(HotplugStats::default()),
            is_online: AtomicBool::new(false),
            pending_offline: AtomicBool::new(false),
            last_state_change: AtomicU64::new(0),
            callbacks: Mutex::new(Vec::new()),
        }
    }

    /// Get current state
    pub fn get_state(&self) -> CpuState {
        *self.state.lock()
    }

    /// Check if CPU is online
    pub fn check_online(&self) -> bool {
        self.is_online.load(Ordering::Relaxed)
    }

    /// Bring CPU online
    pub fn online(&self) -> Result<(), HotplugError> {
        // Check current state
        let mut state = self.state.lock();
        if state.is_online() {
            return Ok(());
        }

        // Notify callbacks
        self.notify_callbacks(HotplugEvent::PrepareOnline);

        // Perform online operation
        self.cpu_online_hw()?;

        // Update state
        *state = CpuState::Online;
        self.is_online.store(true, Ordering::Relaxed);
        self.last_state_change
            .store(self.get_time_us(), Ordering::Relaxed);

        // Update statistics
        let mut stats = self.stats.lock();
        stats.online_count += 1;
        stats.last_online_time = self.get_time_us();
        stats.current_state = CpuState::Online;

        // Notify callbacks
        drop(state);
        self.notify_callbacks(HotplugEvent::Online);
        self.notify_callbacks(HotplugEvent::PostOnline);

        log::info!("CPU {} is now online", self.cpu_id);
        Ok(())
    }

    /// Take CPU offline
    pub fn offline(&self) -> Result<(), HotplugError> {
        // Check if this is the boot CPU (can't offline)
        if self.cpu_id == 0 {
            return Err(HotplugError::BootCpu);
        }

        // Check current state
        let mut state = self.state.lock();
        if !state.is_online() {
            return Ok(());
        }

        // Notify callbacks
        self.notify_callbacks(HotplugEvent::PrepareOffline);

        // Mark as pending offline
        self.pending_offline.store(true, Ordering::Relaxed);

        // Move to quiesced state
        *state = CpuState::Quiesced;

        // Wait for work to complete (simplified)
        drop(state);
        self.quiesce();

        // Perform offline operation
        state = self.state.lock();
        self.cpu_offline_hw()?;

        // Update state
        *state = CpuState::Offline;
        self.is_online.store(false, Ordering::Relaxed);
        self.pending_offline.store(false, Ordering::Relaxed);
        self.last_state_change
            .store(self.get_time_us(), Ordering::Relaxed);

        // Update statistics
        let mut stats = self.stats.lock();
        stats.offline_count += 1;
        stats.last_offline_time = self.get_time_us();

        // Update total online time
        if stats.last_online_time > 0 {
            let online_duration = self
                .get_time_us()
                .saturating_sub(stats.last_online_time);
            stats.total_online_time_us += online_duration;
        }
        stats.current_state = CpuState::Offline;

        // Notify callbacks
        drop(state);
        self.notify_callbacks(HotplugEvent::Offline);
        self.notify_callbacks(HotplugEvent::PostOffline);

        log::info!("CPU {} is now offline", self.cpu_id);
        Ok(())
    }

    /// Hardware-specific online operation
    fn cpu_online_hw(&self) -> Result<(), HotplugError> {
        // In real implementation, this would involve:
        // 1. Powering on the CPU
        // 2. Initializing CPU state
        // 3. Starting CPU execution
        // 4. Waiting for CPU to acknowledge

        // Placeholder implementation
        Ok(())
    }

    /// Hardware-specific offline operation
    fn cpu_offline_hw(&self) -> Result<(), HotplugError> {
        // In real implementation, this would involve:
        // 1. Migrating tasks away
        // 2. Stopping CPU
        // 3. Powering down CPU

        // Placeholder implementation
        Ok(())
    }

    /// Quiesce CPU (prepare for offline)
    fn quiesce(&self) {
        // Wait for pending work to complete
        // In real implementation, would synchronize with scheduler
    }

    /// Add notification callback
    pub fn add_callback(&self, callback: HotplugCallback) {
        self.callbacks.lock().push(callback);
    }

    /// Remove callback
    pub fn remove_callback(&self, id: usize) {
        let mut callbacks = self.callbacks.lock();
        if id < callbacks.len() {
            callbacks.remove(id);
        }
    }

    /// Notify all callbacks
    fn notify_callbacks(&self, event: HotplugEvent) {
        let callbacks = self.callbacks.lock();
        for callback in callbacks.iter() {
            callback.call(self.cpu_id, event);
        }
    }

    /// Get statistics
    pub fn get_stats(&self) -> HotplugStats {
        let mut stats = self.stats.lock();
        stats.current_state = self.get_state();
        stats.clone()
    }

    /// Get current time in microseconds
    fn get_time_us(&self) -> u64 {
        nos_api::event::get_time_ns() / 1000
    }

    /// Check if CPU can be taken offline
    pub fn can_offline(&self) -> bool {
        if self.cpu_id == 0 {
            return false; // Boot CPU can't offline
        }

        let state = self.get_state();
        state.is_online() && !self.pending_offline.load(Ordering::Relaxed)
    }

    /// Check if CPU can be brought online
    pub fn can_online(&self) -> bool {
        let state = self.get_state();
        !state.is_online()
    }
}

/// Hotplug callback
#[derive(Clone)]
pub struct HotplugCallback {
    /// Callback ID
    pub id: usize,
    /// Callback function
    pub callback: Arc<dyn Fn(u32, HotplugEvent) + Send + Sync>,
}

impl HotplugCallback {
    /// Create new callback
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(u32, HotplugEvent) + Send + Sync + 'static,
    {
        static NEXT_ID: AtomicUsize = AtomicUsize::new(0);

        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            callback: Arc::new(callback),
        }
    }

    /// Call the callback
    pub fn call(&self, cpu_id: u32, event: HotplugEvent) {
        (self.callback)(cpu_id, event);
    }
}

impl core::fmt::Debug for HotplugCallback {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "HotplugCallback({})", self.id)
    }
}

/// Load monitor for auto-hotplug
#[derive(Debug)]
pub struct LoadMonitor {
    /// Load samples history
    pub load_history: Mutex<VecDeque<u8>>,
    /// Number of samples to keep
    pub max_samples: usize,
    /// Auto hotplug enabled
    pub enabled: AtomicBool,
    /// High load threshold (bring CPUs online)
    pub high_threshold: AtomicU32,
    /// Low load threshold (take CPUs offline)
    pub low_threshold: AtomicU32,
    /// Sampling interval (milliseconds)
    pub sample_interval_ms: AtomicU32,
    /// Last sample time
    pub last_sample: AtomicU64,
}

impl LoadMonitor {
    /// Create new load monitor
    pub fn new() -> Self {
        Self {
            load_history: Mutex::new(VecDeque::with_capacity(10)),
            max_samples: 10,
            enabled: AtomicBool::new(false),
            high_threshold: AtomicU32::new(80), // 80%
            low_threshold: AtomicU32::new(30),  // 30%
            sample_interval_ms: AtomicU32::new(1000), // 1 second
            last_sample: AtomicU64::new(0),
        }
    }

    /// Enable auto hotplug
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::Relaxed);
    }

    /// Disable auto hotplug
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::Relaxed);
    }

    /// Set thresholds
    pub fn set_thresholds(&self, high: u32, low: u32) -> Result<(), HotplugError> {
        if high > 100 || low > 100 || high <= low {
            return Err(HotplugError::InvalidThreshold);
        }

        self.high_threshold.store(high, Ordering::Relaxed);
        self.low_threshold.store(low, Ordering::Relaxed);
        Ok(())
    }

    /// Get thresholds
    pub fn get_thresholds(&self) -> (u32, u32) {
        let high = self.high_threshold.load(Ordering::Relaxed);
        let low = self.low_threshold.load(Ordering::Relaxed);
        (high, low)
    }

    /// Sample current load
    pub fn sample_load(&self) -> u8 {
        // Placeholder: should get actual system load
        // In real implementation, would query scheduler
        50
    }

    /// Add load sample
    pub fn add_sample(&self, load: u8) {
        let mut history = self.load_history.lock();

        if history.len() >= self.max_samples {
            history.pop_front();
        }

        history.push_back(load);
        self.last_sample.store(self.get_time_ms(), Ordering::Relaxed);
    }

    /// Get average load
    pub fn get_average_load(&self) -> u8 {
        let history = self.load_history.lock();
        if history.is_empty() {
            return 0;
        }

        let sum: u32 = history.iter().map(|&l| l as u32).sum();
        (sum / history.len() as u32) as u8
    }

    /// Get trending load (increasing/decreasing)
    pub fn get_trend(&self) -> LoadTrend {
        let history = self.load_history.lock();
        if history.len() < 3 {
            return LoadTrend::Stable;
        }

        let recent: u32 = history.iter().rev().take(3).map(|&l| l as u32).sum();
        let older: u32 = history.iter().take(3).map(|&l| l as u32).sum();

        if recent > older + 10 {
            LoadTrend::Increasing
        } else if recent < older.saturating_sub(10) {
            LoadTrend::Decreasing
        } else {
            LoadTrend::Stable
        }
    }

    /// Check if should trigger hotplug
    pub fn should_hotplug(&self) -> HotplugAction {
        if !self.enabled.load(Ordering::Relaxed) {
            return HotplugAction::None;
        }

        let avg_load = self.get_average_load();
        let trend = self.get_trend();
        let (high, low) = self.get_thresholds();

        match trend {
            LoadTrend::Increasing if avg_load > high as u8 => HotplugAction::Online,
            LoadTrend::Decreasing if avg_load < low as u8 => HotplugAction::Offline,
            _ => HotplugAction::None,
        }
    }

    /// Get current time in milliseconds
    fn get_time_ms(&self) -> u64 {
        nos_api::event::get_time_ns() / 1_000_000
    }

    /// Get statistics
    pub fn get_stats(&self) -> LoadMonitorStats {
        let history = self.load_history.lock();
        LoadMonitorStats {
            enabled: self.enabled.load(Ordering::Relaxed),
            current_load: if let Some(&latest) = history.back() {
                latest
            } else {
                0
            },
            average_load: self.get_average_load(),
            trend: self.get_trend(),
            high_threshold: self.high_threshold.load(Ordering::Relaxed),
            low_threshold: self.low_threshold.load(Ordering::Relaxed),
            sample_count: history.len(),
        }
    }
}

/// Load trend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadTrend {
    /// Load is increasing
    Increasing,
    /// Load is stable
    Stable,
    /// Load is decreasing
    Decreasing,
}

/// Hotplug action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotplugAction {
    /// No action needed
    None,
    /// Bring CPU online
    Online,
    /// Take CPU offline
    Offline,
}

/// Load monitor statistics
#[derive(Debug, Clone)]
pub struct LoadMonitorStats {
    /// Auto hotplug enabled
    pub enabled: bool,
    /// Current load
    pub current_load: u8,
    /// Average load
    pub average_load: u8,
    /// Load trend
    pub trend: LoadTrend,
    /// High threshold
    pub high_threshold: u32,
    /// Low threshold
    pub low_threshold: u32,
    /// Number of samples
    pub sample_count: usize,
}

/// CPU hotplug manager
#[derive(Debug)]
pub struct CpuHotplugManager {
    /// CPU devices
    pub devices: BTreeMap<u32, Arc<CpuHotplugDevice>>,
    /// Load monitor
    pub load_monitor: Arc<LoadMonitor>,
    /// Hotplug enabled
    pub enabled: AtomicBool,
    /// Minimum online CPUs
    pub min_online: AtomicU32,
    /// Maximum online CPUs
    pub max_online: AtomicU32,
    /// Total CPUs
    pub total_cpus: u32,
}

impl CpuHotplugManager {
    /// Create new CPU hotplug manager
    pub fn new(total_cpus: u32) -> Self {
        let mut devices = BTreeMap::new();

        // Create devices for all CPUs
        for cpu in 0..total_cpus {
            // Determine capacity (simple big.LITTLE setup)
            let capacity = if cpu < total_cpus / 2 {
                CpuCapacity::Big
            } else {
                CpuCapacity::Little
            };

            devices.insert(cpu, Arc::new(CpuHotplugDevice::new(cpu, capacity)));
        }

        Self {
            devices,
            load_monitor: Arc::new(LoadMonitor::new()),
            enabled: AtomicBool::new(false),
            min_online: AtomicU32::new(1), // At least 1 CPU online
            max_online: AtomicU32::new(total_cpus),
            total_cpus,
        }
    }

    /// Initialize manager
    pub fn init(&self) -> Result<(), HotplugError> {
        // Bring boot CPU online
        let boot_cpu = self.devices.get(&0).ok_or(HotplugError::DeviceNotFound)?;
        boot_cpu.online()?;

        self.enabled.store(true, Ordering::Relaxed);
        log::info!("CPU hotplug manager initialized for {} CPUs", self.total_cpus);

        Ok(())
    }

    /// Get device for CPU
    pub fn get_device(&self, cpu_id: u32) -> Option<Arc<CpuHotplugDevice>> {
        self.devices.get(&cpu_id).cloned()
    }

    /// Bring CPU online
    pub fn cpu_online(&self, cpu_id: u32) -> Result<(), HotplugError> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Err(HotplugError::Disabled);
        }

        let device = self.get_device(cpu_id).ok_or(HotplugError::DeviceNotFound)?;

        if !device.can_online() {
            return Err(HotplugError::InvalidState);
        }

        device.online()
    }

    /// Take CPU offline
    pub fn cpu_offline(&self, cpu_id: u32) -> Result<(), HotplugError> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Err(HotplugError::Disabled);
        }

        // Check minimum online constraint
        let online_count = self.count_online_cpus();
        let min_online = self.min_online.load(Ordering::Relaxed) as usize;
        if online_count <= min_online {
            return Err(HotplugError::MinOnlineLimit);
        }

        let device = self.get_device(cpu_id).ok_or(HotplugError::DeviceNotFound)?;

        if !device.can_offline() {
            return Err(HotplugError::InvalidState);
        }

        device.offline()
    }

    /// Count online CPUs
    pub fn count_online_cpus(&self) -> usize {
        self.devices
            .values()
            .filter(|d| d.check_online())
            .count()
    }

    /// Get online CPUs list
    pub fn get_online_cpus(&self) -> Vec<u32> {
        self.devices
            .iter()
            .filter(|(_, d)| d.check_online())
            .map(|(&cpu, _)| cpu)
            .collect()
    }

    /// Get offline CPUs list
    pub fn get_offline_cpus(&self) -> Vec<u32> {
        self.devices
            .iter()
            .filter(|(_, d)| !d.check_online())
            .map(|(&cpu, _)| cpu)
            .collect()
    }

    /// Set hotplug thresholds
    pub fn set_thresholds(&self, high: u32, low: u32) -> Result<(), HotplugError> {
        self.load_monitor.set_thresholds(high, low)
    }

    /// Get hotplug thresholds
    pub fn get_thresholds(&self) -> (u32, u32) {
        self.load_monitor.get_thresholds()
    }

    /// Enable auto hotplug
    pub fn enable_auto_hotplug(&self) {
        self.load_monitor.enable();
        log::info!("Auto hotplug enabled");
    }

    /// Disable auto hotplug
    pub fn disable_auto_hotplug(&self) {
        self.load_monitor.disable();
        log::info!("Auto hotplug disabled");
    }

    /// Process auto hotplug
    pub fn process_auto_hotplug(&self) -> Result<(), HotplugError> {
        // Sample load
        let load = self.load_monitor.sample_load();
        self.load_monitor.add_sample(load);

        // Check if should hotplug
        let action = self.load_monitor.should_hotplug();

        match action {
            HotplugAction::Online => {
                // Try to bring a CPU online
                if let Some(&cpu) = self.get_offline_cpus().first() {
                    self.cpu_online(cpu)?;
                }
            }
            HotplugAction::Offline => {
                // Try to take a CPU offline
                let online_cpus = self.get_online_cpus();
                if online_cpus.len() > self.min_online.load(Ordering::Relaxed) as usize {
                    if let Some(&cpu) = online_cpus.iter().rev().next() {
                        if cpu != 0 {
                            // Don't offline boot CPU
                            let _ = self.cpu_offline(cpu);
                        }
                    }
                }
            }
            HotplugAction::None => {}
        }

        Ok(())
    }

    /// Get CPU statistics
    pub fn get_cpu_stats(&self, cpu_id: u32) -> Option<HotplugStats> {
        let device = self.get_device(cpu_id)?;
        Some(device.get_stats())
    }

    /// Get all statistics
    pub fn get_stats(&self) -> HotplugManagerStats {
        let mut cpu_stats = BTreeMap::new();

        for (&cpu_id, device) in &self.devices {
            cpu_stats.insert(cpu_id, device.get_stats());
        }

        HotplugManagerStats {
            total_cpus: self.total_cpus,
            online_count: self.count_online_cpus() as u32,
            offline_count: (self.total_cpus - self.count_online_cpus() as u32),
            cpu_stats,
            load_monitor: self.load_monitor.get_stats(),
            auto_hotplug_enabled: self.load_monitor.enabled.load(Ordering::Relaxed),
        }
    }

    /// Add global hotplug callback
    pub fn add_callback(&self, cpu_id: u32, callback: HotplugCallback) -> Result<(), HotplugError> {
        let device = self.get_device(cpu_id).ok_or(HotplugError::DeviceNotFound)?;
        device.add_callback(callback);
        Ok(())
    }

    /// Find best CPU to online based on capacity
    pub fn find_cpu_to_online(&self, preferred_capacity: CpuCapacity) -> Option<u32> {
        self.get_offline_cpus()
            .into_iter()
            .find(|&cpu| {
                self.get_device(cpu)
                    .map(|d| d.capacity == preferred_capacity)
                    .unwrap_or(false)
            })
            .or_else(|| {
                // If no preferred CPU available, get any offline CPU
                self.get_offline_cpus().into_iter().next()
            })
    }

    /// Find best CPU to offline
    pub fn find_cpu_to_offline(&self) -> Option<u32> {
        // Prefer to offline little cores first
        let mut little_cpus: Vec<_> = self
            .get_online_cpus()
            .into_iter()
            .filter(|&cpu| {
                self.get_device(cpu)
                    .map(|d| d.capacity == CpuCapacity::Little)
                    .unwrap_or(false)
            })
            .collect();

        if !little_cpus.is_empty() {
            little_cpus.pop() // Return last little CPU
        } else {
            // Otherwise, try any online CPU except boot CPU
            self.get_online_cpus()
                .into_iter()
                .filter(|&cpu| cpu != 0)
                .rev()
                .next()
        }
    }

    /// Get CPU capacity
    pub fn get_cpu_capacity(&self, cpu_id: u32) -> Option<CpuCapacity> {
        let device = self.get_device(cpu_id)?;
        Some(device.capacity)
    }

    /// Get CPUs by capacity
    pub fn get_cpus_by_capacity(&self, capacity: CpuCapacity) -> Vec<u32> {
        self.devices
            .iter()
            .filter(|(_, d)| d.capacity == capacity)
            .map(|(&cpu, _)| cpu)
            .collect()
    }
}

/// Hotplug manager statistics
#[derive(Debug, Clone)]
pub struct HotplugManagerStats {
    /// Total CPUs
    pub total_cpus: u32,
    /// Number of online CPUs
    pub online_count: u32,
    /// Number of offline CPUs
    pub offline_count: u32,
    /// Per-CPU statistics
    pub cpu_stats: BTreeMap<u32, HotplugStats>,
    /// Load monitor statistics
    pub load_monitor: LoadMonitorStats,
    /// Auto hotplug enabled
    pub auto_hotplug_enabled: bool,
}

/// CPU hotplug errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotplugError {
    /// Device not found
    DeviceNotFound,
    /// Invalid state for operation
    InvalidState,
    /// Cannot offline boot CPU
    BootCpu,
    /// Hotplug is disabled
    Disabled,
    /// Invalid threshold
    InvalidThreshold,
    /// Minimum online CPU limit reached
    MinOnlineLimit,
    /// Maximum online CPU limit reached
    MaxOnlineLimit,
    /// Hardware error
    HardwareError,
}

impl core::fmt::Display for HotplugError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DeviceNotFound => write!(f, "CPU device not found"),
            Self::InvalidState => write!(f, "Invalid CPU state for operation"),
            Self::BootCpu => write!(f, "Cannot offline boot CPU"),
            Self::Disabled => write!(f, "CPU hotplug is disabled"),
            Self::InvalidThreshold => write!(f, "Invalid threshold values"),
            Self::MinOnlineLimit => write!(f, "Minimum online CPU limit reached"),
            Self::MaxOnlineLimit => write!(f, "Maximum online CPU limit reached"),
            Self::HardwareError => write!(f, "Hardware error"),
        }
    }
}

/// Global CPU hotplug manager
static GLOBAL_HOTPLUG_MANAGER: Mutex<Option<CpuHotplugManager>> = Mutex::new(None);

/// Initialize CPU hotplug subsystem
pub fn init_cpuhotplug() -> Result<(), HotplugError> {
    let mut manager = GLOBAL_HOTPLUG_MANAGER.lock();

    if manager.is_some() {
        return Ok(());
    }

    // Detect number of CPUs
    let num_cpus = detect_num_cpus();
    let hotplug_manager = CpuHotplugManager::new(num_cpus);
    hotplug_manager.init()?;

    *manager = Some(hotplug_manager);

    log::info!("CPU hotplug subsystem initialized");
    Ok(())
}

/// Get CPU hotplug manager
pub fn get_hotplug_manager() -> Option<&'static CpuHotplugManager> {
    // Note: Can't return reference from Mutex
    // In real implementation, would use different approach
    None
}

/// Bring CPU online
pub fn cpu_online(cpu: u32) -> Result<(), HotplugError> {
    let manager = GLOBAL_HOTPLUG_MANAGER.lock();
    let manager = manager.as_ref().ok_or(HotplugError::Disabled)?;
    manager.cpu_online(cpu)
}

/// Take CPU offline
pub fn cpu_offline(cpu: u32) -> Result<(), HotplugError> {
    let manager = GLOBAL_HOTPLUG_MANAGER.lock();
    let manager = manager.as_ref().ok_or(HotplugError::Disabled)?;
    manager.cpu_offline(cpu)
}

/// Set hotplug threshold
pub fn set_hotplug_threshold(high_percent: u8, low_percent: u8) -> Result<(), HotplugError> {
    let manager = GLOBAL_HOTPLUG_MANAGER.lock();
    let manager = manager.as_ref().ok_or(HotplugError::Disabled)?;
    manager.set_thresholds(high_percent as u32, low_percent as u32)
}

/// Detect number of CPUs
fn detect_num_cpus() -> u32 {
    // Placeholder: should detect from hardware
    4
}

/// Get hotplug statistics
pub fn get_hotplug_stats() -> Option<HotplugManagerStats> {
    let manager = GLOBAL_HOTPLUG_MANAGER.lock();
    manager.as_ref()?.get_stats().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_state() {
        assert_eq!(CpuState::Online.is_online(), true);
        assert_eq!(CpuState::Active.is_online(), true);
        assert_eq!(CpuState::Offline.is_online(), false);
        assert_eq!(CpuState::Active.can_run_work(), true);
        assert_eq!(CpuState::Online.can_run_work(), false);
    }

    #[test]
    fn test_cpu_capacity() {
        assert_eq!(CpuCapacity::Big.performance_score(), 1000);
        assert_eq!(CpuCapacity::Little.performance_score(), 200);
        assert_eq!(CpuCapacity::Medium.power_score(), 500);
    }

    #[test]
    fn test_hotplug_device() {
        let device = CpuHotplugDevice::new(0, CpuCapacity::Big);
        assert_eq!(device.cpu_id, 0);
        assert_eq!(device.capacity, CpuCapacity::Big);
        assert_eq!(device.get_state(), CpuState::Offline);
        assert!(!device.check_online());
    }

    #[test]
    fn test_load_monitor() {
        let monitor = LoadMonitor::new();
        monitor.add_sample(50);
        monitor.add_sample(60);
        monitor.add_sample(70);

        assert_eq!(monitor.get_average_load(), 60);

        let stats = monitor.get_stats();
        assert_eq!(stats.current_load, 70);
        assert_eq!(stats.sample_count, 3);
    }

    #[test]
    fn test_hotplug_manager() {
        let manager = CpuHotplugManager::new(4);
        assert_eq!(manager.total_cpus, 4);
        assert_eq!(manager.devices.len(), 4);

        let boot_cpu = manager.get_device(0).unwrap();
        assert_eq!(boot_cpu.capacity, CpuCapacity::Big);
    }
}
