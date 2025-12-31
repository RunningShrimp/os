//! CPU Optimization Module
//!
//! This module provides comprehensive CPU optimization capabilities including:
//! - CPU frequency scaling governors (performance, powersave, ondemand, conservative)
//! - CPU topology awareness (cores, sockets, NUMA nodes)
//! - CPU idle state management (C-states: C0, C1, C1E, C3, C6)
//! - CPU load balancing across cores
//! - CPU hotplug support (add/remove CPUs)
//! - Turbo Boost/PowerNow control
//! - CPU utilization tracking

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::sync::Mutex;

use crate::prelude::*;

/// CPU frequency scaling governor types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernorType {
    /// Performance governor - always max frequency
    Performance,
    /// Powersave governor - always min frequency
    Powersave,
    /// Ondemand governor - dynamic based on load
    Ondemand {
        /// Load threshold for scaling up (0-100)
        up_threshold: u32,
        /// Load threshold for scaling down (0-100)
        down_threshold: u32,
    },
    /// Conservative governor - gradual frequency changes
    Conservative {
        /// Frequency step percentage
        step: u32,
        /// Sampling interval in milliseconds
        sampling_rate_ms: u32,
    },
    /// Schedutil governor - scheduler-driven
    Schedutil,
}

impl GovernorType {
    /// Get governor name
    pub fn name(&self) -> &str {
        match self {
            GovernorType::Performance => "performance",
            GovernorType::Powersave => "powersave",
            GovernorType::Ondemand { .. } => "ondemand",
            GovernorType::Conservative { .. } => "conservative",
            GovernorType::Schedutil => "schedutil",
        }
    }

    /// Check if governor is dynamic
    pub fn is_dynamic(&self) -> bool {
        matches!(self, GovernorType::Ondemand { .. } | GovernorType::Conservative { .. } | GovernorType::Schedutil)
    }
}

/// CPU idle state (C-state)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CState {
    /// C0 - Active state
    C0,
    /// C1 - Halt (low latency)
    C1,
    /// C1E - Enhanced Halt
    C1E,
    /// C3 - Sleep
    C3,
    /// C6 - Deep sleep
    C6,
    /// C7 - Deeper sleep
    C7,
    /// C8 - Deepest sleep
    C8,
}

impl CState {
    /// Get C-state name
    pub fn name(&self) -> &str {
        match self {
            CState::C0 => "C0",
            CState::C1 => "C1",
            CState::C1E => "C1E",
            CState::C3 => "C3",
            CState::C6 => "C6",
            CState::C7 => "C7",
            CState::C8 => "C8",
        }
    }

    /// Get exit latency in microseconds
    pub fn exit_latency_us(&self) -> u32 {
        match self {
            CState::C0 => 0,
            CState::C1 => 1,
            CState::C1E => 10,
            CState::C3 => 50,
            CState::C6 => 100,
            CState::C7 => 150,
            CState::C8 => 200,
        }
    }

    /// Get power consumption (relative)
    pub fn power_consumption(&self) -> u32 {
        match self {
            CState::C0 => 100,
            CState::C1 => 90,
            CState::C1E => 80,
            CState::C3 => 50,
            CState::C6 => 20,
            CState::C7 => 10,
            CState::C8 => 5,
        }
    }

    /// Compare C-states (deeper is greater)
    pub fn depth(&self) -> u32 {
        match self {
            CState::C0 => 0,
            CState::C1 => 1,
            CState::C1E => 2,
            CState::C3 => 3,
            CState::C6 => 4,
            CState::C7 => 5,
            CState::C8 => 6,
        }
    }
}

/// CPU topology information
#[derive(Debug, Clone)]
pub struct CpuTopology {
    /// Number of physical CPUs
    pub sockets: u32,
    /// Number of cores per socket
    pub cores_per_socket: u32,
    /// Number of threads per core (hyperthreading)
    pub threads_per_core: u32,
    /// Total number of CPUs
    pub total_cpus: u32,
    /// NUMA node count
    pub numa_nodes: u32,
    /// Cache topology
    pub cache_topology: CacheTopology,
}

impl CpuTopology {
    /// Create new CPU topology
    pub fn new() -> Self {
        Self {
            sockets: 1,
            cores_per_socket: 1,
            threads_per_core: 1,
            total_cpus: 1,
            numa_nodes: 1,
            cache_topology: CacheTopology::default(),
        }
    }

    /// Get total number of cores
    pub fn total_cores(&self) -> u32 {
        self.sockets * self.cores_per_socket
    }
}

/// Cache topology information
#[derive(Debug, Clone)]
pub struct CacheTopology {
    /// L1 cache size in bytes
    pub l1_size: u32,
    /// L1 cache line size in bytes
    pub l1_line_size: u32,
    /// L2 cache size in bytes
    pub l2_size: u32,
    /// L2 cache line size in bytes
    pub l2_line_size: u32,
    /// L3 cache size in bytes
    pub l3_size: u32,
    /// L3 cache line size in bytes
    pub l3_line_size: u32,
    /// Cache hierarchy sharing
    pub l1_shared: bool,
    pub l2_shared: bool,
    pub l3_shared: bool,
}

impl Default for CacheTopology {
    fn default() -> Self {
        Self {
            l1_size: 32 * 1024,
            l1_line_size: 64,
            l2_size: 256 * 1024,
            l2_line_size: 64,
            l3_size: 8 * 1024 * 1024,
            l3_line_size: 64,
            l1_shared: false,
            l2_shared: false,
            l3_shared: true,
        }
    }
}

/// CPU information
#[derive(Debug, Clone)]
pub struct CpuInfo {
    /// CPU ID
    pub cpu_id: u32,
    /// Physical socket ID
    pub socket_id: u32,
    /// Core ID within socket
    pub core_id: u32,
    /// Thread ID within core
    pub thread_id: u32,
    /// NUMA node ID
    pub numa_node: u32,
    /// Current frequency in MHz
    pub current_frequency: u32,
    /// Minimum frequency in MHz
    pub min_frequency: u32,
    /// Maximum frequency in MHz
    pub max_frequency: u32,
    /// Base frequency in MHz
    pub base_frequency: u32,
    /// Turbo boost enabled
    pub turbo_enabled: bool,
    /// Current C-state
    pub current_cstate: CState,
    /// CPU utilization (0-100)
    pub utilization: u32,
    /// Online status
    pub online: bool,
}

impl CpuInfo {
    /// Create new CPU info
    pub fn new(cpu_id: u32) -> Self {
        Self {
            cpu_id,
            socket_id: 0,
            core_id: cpu_id,
            thread_id: 0,
            numa_node: 0,
            current_frequency: 2000,
            min_frequency: 800,
            max_frequency: 4000,
            base_frequency: 2000,
            turbo_enabled: false,
            current_cstate: CState::C0,
            utilization: 0,
            online: true,
        }
    }

    /// Check if CPU is online
    pub fn is_online(&self) -> bool {
        self.online
    }

    /// Get normalized frequency (0.0 - 1.0)
    pub fn normalized_frequency(&self) -> f64 {
        let range = self.max_frequency - self.min_frequency;
        if range == 0 {
            return 0.5;
        }
        (self.current_frequency - self.min_frequency) as f64 / range as f64
    }
}

/// CPU load statistics
#[derive(Debug, Clone)]
pub struct CpuLoadStats {
    /// User time (ticks)
    pub user: u64,
    /// System time (ticks)
    pub system: u64,
    /// Idle time (ticks)
    pub idle: u64,
    /// I/O wait time (ticks)
    pub iowait: u64,
    /// IRQ time (ticks)
    pub irq: u64,
    /// SoftIRQ time (ticks)
    pub softirq: u64,
    /// Steal time (ticks)
    pub steal: u64,
    /// Guest time (ticks)
    pub guest: u64,
}

impl CpuLoadStats {
    /// Create new CPU load stats
    pub fn new() -> Self {
        Self {
            user: 0,
            system: 0,
            idle: 1,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            guest: 0,
        }
    }

    /// Get total time
    pub fn total(&self) -> u64 {
        self.user + self.system + self.idle + self.iowait + self.irq + self.softirq + self.steal + self.guest
    }

    /// Get utilization percentage (0-100)
    pub fn utilization(&self) -> u32 {
        let total = self.total();
        if total == 0 {
            return 0;
        }
        let idle = self.idle + self.iowait;
        ((total - idle) * 100 / total) as u32
    }
}

/// CPU errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CpuError {
    /// Invalid CPU ID
    InvalidCpuId,
    /// CPU is offline
    CpuOffline,
    /// Invalid frequency
    InvalidFrequency,
    /// Invalid governor
    InvalidGovernor,
    /// Invalid C-state
    InvalidCState,
    /// Topology detection failed
    TopologyDetectionFailed,
    /// Frequency scaling failed
    FrequencyScalingFailed,
    /// Hotplug operation failed
    HotplugFailed,
    /// Turbo boost control failed
    TurboBoostFailed,
    /// Load balancing failed
    LoadBalancingFailed,
    /// Permission denied
    PermissionDenied,
    /// Unsupported operation
    Unsupported,
}

impl core::fmt::Display for CpuError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CpuError::InvalidCpuId => write!(f, "Invalid CPU ID"),
            CpuError::CpuOffline => write!(f, "CPU is offline"),
            CpuError::InvalidFrequency => write!(f, "Invalid frequency"),
            CpuError::InvalidGovernor => write!(f, "Invalid governor"),
            CpuError::InvalidCState => write!(f, "Invalid C-state"),
            CpuError::TopologyDetectionFailed => write!(f, "Topology detection failed"),
            CpuError::FrequencyScalingFailed => write!(f, "Frequency scaling failed"),
            CpuError::HotplugFailed => write!(f, "Hotplug operation failed"),
            CpuError::TurboBoostFailed => write!(f, "Turbo boost control failed"),
            CpuError::LoadBalancingFailed => write!(f, "Load balancing failed"),
            CpuError::PermissionDenied => write!(f, "Permission denied"),
            CpuError::Unsupported => write!(f, "Unsupported operation"),
        }
    }
}

/// CPU frequency governor configuration
#[derive(Debug, Clone)]
pub struct GovernorConfig {
    /// Governor type
    pub governor_type: GovernorType,
    /// Sampling interval in milliseconds
    pub sampling_rate_ms: u32,
    /// Minimum frequency in MHz
    pub min_freq: u32,
    /// Maximum frequency in MHz
    pub max_freq: u32,
}

impl GovernorConfig {
    /// Create new governor configuration
    pub fn new(governor_type: GovernorType) -> Self {
        Self {
            governor_type,
            sampling_rate_ms: 100,
            min_freq: 800,
            max_freq: 4000,
        }
    }

    /// Create performance governor config
    pub fn performance() -> Self {
        Self::new(GovernorType::Performance)
    }

    /// Create powersave governor config
    pub fn powersave() -> Self {
        Self::new(GovernorType::Powersave)
    }

    /// Create ondemand governor config
    pub fn ondemand(up_threshold: u32, down_threshold: u32) -> Self {
        Self {
            governor_type: GovernorType::Ondemand { up_threshold, down_threshold },
            sampling_rate_ms: 100,
            min_freq: 800,
            max_freq: 4000,
        }
    }

    /// Create conservative governor config
    pub fn conservative(step: u32, sampling_rate_ms: u32) -> Self {
        Self {
            governor_type: GovernorType::Conservative { step, sampling_rate_ms },
            sampling_rate_ms,
            min_freq: 800,
            max_freq: 4000,
        }
    }

    /// Create schedutil governor config
    pub fn schedutil() -> Self {
        Self::new(GovernorType::Schedutil)
    }
}

/// Per-CPU state
struct PerCpuState {
    /// CPU information
    info: CpuInfo,
    /// Current governor
    governor: GovernorType,
    /// Governor configuration
    governor_config: GovernorConfig,
    /// Load statistics
    load_stats: CpuLoadStats,
    /// Available C-states
    available_cstates: Vec<CState>,
    /// Idle state statistics
    idle_stats: BTreeMap<CState, u64>,
    /// Frequency transition count
    frequency_transitions: AtomicU64,
    /// C-state transitions count
    cstate_transitions: AtomicU64,
}

impl PerCpuState {
    fn new(cpu_id: u32) -> Self {
        Self {
            info: CpuInfo::new(cpu_id),
            governor: GovernorType::Performance,
            governor_config: GovernorConfig::performance(),
            load_stats: CpuLoadStats::new(),
            available_cstates: vec![CState::C0, CState::C1, CState::C6],
            idle_stats: BTreeMap::new(),
            frequency_transitions: AtomicU64::new(0),
            cstate_transitions: AtomicU64::new(0),
        }
    }

    /// Update CPU utilization
    fn update_utilization(&mut self) {
        self.info.utilization = self.load_stats.utilization();
    }

    /// Select optimal frequency based on governor and load
    fn select_frequency(&self) -> u32 {
        match self.governor {
            GovernorType::Performance => self.info.max_frequency,
            GovernorType::Powersave => self.info.min_frequency,
            GovernorType::Ondemand { up_threshold, .. } => {
                let load = self.load_stats.utilization();
                if load >= up_threshold {
                    self.info.max_frequency
                } else {
                    // Scale frequency proportionally to load
                    let range = self.info.max_frequency - self.info.min_frequency;
                    self.info.min_frequency + (range * load as u32 / 100)
                }
            }
            GovernorType::Conservative { step, .. } => {
                let load = self.load_stats.utilization();
                let current = self.info.current_frequency;
                let range = self.info.max_frequency - self.info.min_frequency;
                let target = self.info.min_frequency + (range * load as u32 / 100);

                if target > current {
                    (current + (range * step / 100)).min(target).min(self.info.max_frequency)
                } else if target < current {
                    current.saturating_sub(range * step / 100).max(target).max(self.info.min_frequency)
                } else {
                    current
                }
            }
            GovernorType::Schedutil => {
                // Scheduler-driven - use utilization directly
                let load = self.load_stats.utilization() as u64;
                let range = (self.info.max_frequency - self.info.min_frequency) as u64;
                self.info.min_frequency + ((range * load / 100) as u32)
            }
        }
    }

    /// Select optimal C-state based on idle time
    fn select_cstate(&self, idle_time_us: u64) -> CState {
        for &cstate in self.available_cstates.iter().rev() {
            if cstate == CState::C0 {
                continue;
            }
            // Only enter C-state if idle time exceeds exit latency
            if idle_time_us > cstate.exit_latency_us() as u64 * 2 {
                return cstate;
            }
        }
        CState::C1
    }
}

/// CPU optimizer - manages CPU frequency scaling and idle states
pub struct CpuOptimizer {
    /// CPU topology
    topology: CpuTopology,
    /// Per-CPU state
    cpu_states: Vec<Mutex<PerCpuState>>,
    /// Hotplug enabled
    hotplug_enabled: AtomicBool,
    /// Turbo boost enabled (global)
    turbo_boost_enabled: AtomicBool,
    /// Load balancing enabled
    load_balancing_enabled: AtomicBool,
    /// Global stats
    global_stats: Mutex<CpuOptimizerStats>,
}

/// CPU optimizer statistics
#[derive(Debug, Clone)]
pub struct CpuOptimizerStats {
    /// Total frequency transitions
    pub frequency_transitions: u64,
    /// Total C-state transitions
    pub cstate_transitions: u64,
    /// Total hotplug events
    pub hotplug_events: u64,
    /// Average CPU utilization
    pub average_utilization: u32,
    /// Power savings (arbitrary units)
    pub power_savings: u64,
}

impl CpuOptimizer {
    /// Create new CPU optimizer
    pub fn new(num_cpus: u32) -> Self {
        let mut cpu_states = Vec::with_capacity(num_cpus as usize);
        for cpu_id in 0..num_cpus {
            cpu_states.push(Mutex::new(PerCpuState::new(cpu_id)));
        }

        Self {
            topology: CpuTopology::new(),
            cpu_states,
            hotplug_enabled: AtomicBool::new(true),
            turbo_boost_enabled: AtomicBool::new(true),
            load_balancing_enabled: AtomicBool::new(true),
            global_stats: Mutex::new(CpuOptimizerStats {
                frequency_transitions: 0,
                cstate_transitions: 0,
                hotplug_events: 0,
                average_utilization: 0,
                power_savings: 0,
            }),
        }
    }

    /// Initialize CPU optimizer
    pub fn init(&mut self) {
        // Detect CPU topology
        self.topology = self.detect_topology();

        log_info!("CPU optimizer initialized: {} CPUs, {} sockets, {} NUMA nodes",
                  self.topology.total_cpus,
                  self.topology.sockets,
                  self.topology.numa_nodes);
    }

    /// Detect CPU topology
    fn detect_topology(&self) -> CpuTopology {
        // Placeholder - in real implementation, use CPUID and ACPI tables
        CpuTopology {
            sockets: 1,
            cores_per_socket: self.cpu_states.len() as u32,
            threads_per_core: 1,
            total_cpus: self.cpu_states.len() as u32,
            numa_nodes: 1,
            cache_topology: CacheTopology::default(),
        }
    }

    /// Get CPU topology
    pub fn get_cpu_topology(&self) -> CpuTopology {
        self.topology.clone()
    }

    /// Set frequency governor for a CPU
    pub fn set_frequency_governor(&self, cpu_id: u32, governor: GovernorType) -> Result<(), CpuError> {
        if cpu_id as usize >= self.cpu_states.len() {
            return Err(CpuError::InvalidCpuId);
        }

        let mut state = self.cpu_states[cpu_id as usize].lock();

        // Check if CPU is online
        if !state.info.online {
            return Err(CpuError::CpuOffline);
        }

        // Update governor
        state.governor = governor;
        state.governor_config.governor_type = governor;

        log_info!("CPU {} governor set to {}", cpu_id, governor.name());
        Ok(())
    }

    /// Get current governor for a CPU
    pub fn get_frequency_governor(&self, cpu_id: u32) -> Result<GovernorType, CpuError> {
        if cpu_id as usize >= self.cpu_states.len() {
            return Err(CpuError::InvalidCpuId);
        }

        let state = self.cpu_states[cpu_id as usize].lock();
        Ok(state.governor)
    }

    /// Set CPU idle state
    pub fn set_cpu_idle_state(&self, cpu_id: u32, cstate: CState) -> Result<(), CpuError> {
        if cpu_id as usize >= self.cpu_states.len() {
            return Err(CpuError::InvalidCpuId);
        }

        let mut state = self.cpu_states[cpu_id as usize].lock();

        // Check if C-state is available
        if !state.available_cstates.contains(&cstate) {
            return Err(CpuError::InvalidCState);
        }

        // Update C-state
        let old_cstate = state.info.current_cstate;
        state.info.current_cstate = cstate;
        state.cstate_transitions.fetch_add(1, Ordering::Relaxed);

        // Track idle statistics
        *state.idle_stats.entry(cstate).or_insert(0) += 1;

        log_debug!("CPU {} C-state: {} -> {}", cpu_id, old_cstate.name(), cstate.name());
        Ok(())
    }

    /// Get CPU information
    pub fn get_cpu_info(&self, cpu_id: u32) -> Result<CpuInfo, CpuError> {
        if cpu_id as usize >= self.cpu_states.len() {
            return Err(CpuError::InvalidCpuId);
        }

        let state = self.cpu_states[cpu_id as usize].lock();
        Ok(state.info.clone())
    }

    /// Get all CPU information
    pub fn get_all_cpu_info(&self) -> Vec<CpuInfo> {
        self.cpu_states.iter()
            .map(|state| state.lock().info.clone())
            .collect()
    }

    /// Update CPU load statistics
    pub fn update_load_stats(&self, cpu_id: u32, stats: CpuLoadStats) -> Result<(), CpuError> {
        if cpu_id as usize >= self.cpu_states.len() {
            return Err(CpuError::InvalidCpuId);
        }

        let mut state = self.cpu_states[cpu_id as usize].lock();
        state.load_stats = stats;
        state.update_utilization();
        Ok(())
    }

    /// Get CPU load statistics
    pub fn get_load_stats(&self, cpu_id: u32) -> Result<CpuLoadStats, CpuError> {
        if cpu_id as usize >= self.cpu_states.len() {
            return Err(CpuError::InvalidCpuId);
        }

        let state = self.cpu_states[cpu_id as usize].lock();
        Ok(state.load_stats.clone())
    }

    /// Balance CPU load across all cores
    pub fn balance_cpu_load(&self) -> Result<(), CpuError> {
        if !self.load_balancing_enabled.load(Ordering::Relaxed) {
            return Err(CpuError::Unsupported);
        }

        // Calculate average utilization
        let mut total_util = 0u32;
        let mut online_count = 0u32;

        for state in &self.cpu_states {
            let state_guard = state.lock();
            if state_guard.info.online {
                total_util += state_guard.info.utilization;
                online_count += 1;
            }
        }

        if online_count == 0 {
            return Err(CpuError::LoadBalancingFailed);
        }

        let avg_util = total_util / online_count;

        // Simple load balancing: adjust frequencies based on load
        for (_cpu_id, state) in self.cpu_states.iter().enumerate() {
            let mut state_guard = state.lock();
            if !state_guard.info.online {
                continue;
            }

            let util = state_guard.info.utilization;
            let target_freq = state_guard.select_frequency();

            // Scale up if above average, down if below
            let new_freq = if util > avg_util + 20 {
                state_guard.info.max_frequency
            } else if util < avg_util - 20 {
                state_guard.info.min_frequency
            } else {
                target_freq
            };

            if new_freq != state_guard.info.current_frequency {
                state_guard.info.current_frequency = new_freq;
                state_guard.frequency_transitions.fetch_add(1, Ordering::Relaxed);
            }
        }

        Ok(())
    }

    /// Enable turbo boost
    pub fn enable_turbo_boost(&self, enable: bool) -> Result<(), CpuError> {
        if !self.turbo_boost_enabled.load(Ordering::Relaxed) {
            return Err(CpuError::Unsupported);
        }

        self.turbo_boost_enabled.store(enable, Ordering::Relaxed);

        // Update all CPUs
        for state in &self.cpu_states {
            let mut state_guard = state.lock();
            state_guard.info.turbo_enabled = enable;

            // Update max frequency based on turbo status
            if enable {
                state_guard.info.max_frequency = state_guard.info.base_frequency * 2;
            } else {
                state_guard.info.max_frequency = state_guard.info.base_frequency;
            }
        }

        log_info!("Turbo boost {}", if enable { "enabled" } else { "disabled" });
        Ok(())
    }

    /// Check if turbo boost is enabled
    pub fn is_turbo_boost_enabled(&self) -> bool {
        self.turbo_boost_enabled.load(Ordering::Relaxed)
    }

    /// CPU online operation
    pub fn cpu_online(&self, cpu_id: u32) -> Result<(), CpuError> {
        if cpu_id as usize >= self.cpu_states.len() {
            return Err(CpuError::InvalidCpuId);
        }

        if !self.hotplug_enabled.load(Ordering::Relaxed) {
            return Err(CpuError::Unsupported);
        }

        let mut state = self.cpu_states[cpu_id as usize].lock();

        if state.info.online {
            return Err(CpuError::CpuOffline); // Already online
        }

        state.info.online = true;
        state.governor = GovernorType::Performance;
        state.info.current_frequency = state.info.base_frequency;

        // Update global stats
        let mut stats = self.global_stats.lock();
        stats.hotplug_events += 1;

        log_info!("CPU {} online", cpu_id);
        Ok(())
    }

    /// CPU offline operation
    pub fn cpu_offline(&self, cpu_id: u32) -> Result<(), CpuError> {
        if cpu_id as usize >= self.cpu_states.len() {
            return Err(CpuError::InvalidCpuId);
        }

        if !self.hotplug_enabled.load(Ordering::Relaxed) {
            return Err(CpuError::Unsupported);
        }

        let mut state = self.cpu_states[cpu_id as usize].lock();

        if !state.info.online {
            return Err(CpuError::CpuOffline); // Already offline
        }

        // Don't allow boot CPU to go offline
        if cpu_id == 0 {
            return Err(CpuError::PermissionDenied);
        }

        state.info.online = false;
        state.info.current_cstate = CState::C6;

        // Update global stats
        let mut stats = self.global_stats.lock();
        stats.hotplug_events += 1;

        log_info!("CPU {} offline", cpu_id);
        Ok(())
    }

    /// Get CPU idle state statistics
    pub fn get_idle_stats(&self, cpu_id: u32) -> Result<BTreeMap<CState, u64>, CpuError> {
        if cpu_id as usize >= self.cpu_states.len() {
            return Err(CpuError::InvalidCpuId);
        }

        let state = self.cpu_states[cpu_id as usize].lock();
        Ok(state.idle_stats.clone())
    }

    /// Get optimizer statistics
    pub fn get_stats(&self) -> CpuOptimizerStats {
        let mut stats = self.global_stats.lock();

        // Calculate average utilization
        let mut total_util = 0u32;
        let mut count = 0u32;

        for state in &self.cpu_states {
            let state_guard = state.lock();
            if state_guard.info.online {
                total_util += state_guard.info.utilization;
                count += 1;
            }

            // Aggregate transitions
            stats.frequency_transitions += state_guard.frequency_transitions.load(Ordering::Relaxed);
            stats.cstate_transitions += state_guard.cstate_transitions.load(Ordering::Relaxed);
        }

        if count > 0 {
            stats.average_utilization = total_util / count;
        }

        stats.clone()
    }

    /// Enable/disable hotplug
    pub fn set_hotplug_enabled(&self, enabled: bool) {
        self.hotplug_enabled.store(enabled, Ordering::Relaxed);
    }

    /// Enable/disable load balancing
    pub fn set_load_balancing_enabled(&self, enabled: bool) {
        self.load_balancing_enabled.store(enabled, Ordering::Relaxed);
    }

    /// Run governor update loop (should be called periodically)
    pub fn update_governors(&self) {
        for (_cpu_id, state) in self.cpu_states.iter().enumerate() {
            let mut state_guard = state.lock();

            if !state_guard.info.online {
                continue;
            }

            // Select optimal frequency
            let target_freq = state_guard.select_frequency();

            if target_freq != state_guard.info.current_frequency {
                state_guard.info.current_frequency = target_freq;
                state_guard.frequency_transitions.fetch_add(1, Ordering::Relaxed);
            }
        }
    }

    /// Get CPU utilization for all CPUs
    pub fn get_all_utilization(&self) -> Vec<u32> {
        self.cpu_states.iter()
            .map(|state| state.lock().info.utilization)
            .collect()
    }

    /// Get CPU frequencies for all CPUs
    pub fn get_all_frequencies(&self) -> Vec<u32> {
        self.cpu_states.iter()
            .map(|state| state.lock().info.current_frequency)
            .collect()
    }
}

/// Global CPU optimizer instance
static mut GLOBAL_CPU_OPTIMIZER: Option<CpuOptimizer> = None;
static CPU_OPTIMIZER_INIT: Mutex<bool> = Mutex::new(false);

/// Initialize global CPU optimizer
pub fn init_cpu_optimizer(num_cpus: u32) {
    let mut is_init = CPU_OPTIMIZER_INIT.lock();
    if *is_init {
        return;
    }

    let mut optimizer = CpuOptimizer::new(num_cpus);
    optimizer.init();

    unsafe {
        GLOBAL_CPU_OPTIMIZER = Some(optimizer);
    }
    *is_init = true;

    log_info!("Global CPU optimizer initialized");
}

/// Get global CPU optimizer
pub fn get_cpu_optimizer() -> Option<&'static CpuOptimizer> {
    unsafe {
        GLOBAL_CPU_OPTIMIZER.as_ref()
    }
}

/// Set frequency governor for a CPU (convenience function)
pub fn set_frequency_governor(cpu_id: u32, governor: GovernorType) -> Result<(), CpuError> {
    let optimizer = get_cpu_optimizer().ok_or(CpuError::Unsupported)?;
    optimizer.set_frequency_governor(cpu_id, governor)
}

/// Set CPU idle state (convenience function)
pub fn set_cpu_idle_state(cpu_id: u32, cstate: CState) -> Result<(), CpuError> {
    let optimizer = get_cpu_optimizer().ok_or(CpuError::Unsupported)?;
    optimizer.set_cpu_idle_state(cpu_id, cstate)
}

/// Get CPU topology (convenience function)
pub fn get_cpu_topology() -> Option<CpuTopology> {
    let optimizer = get_cpu_optimizer()?;
    Some(optimizer.get_cpu_topology())
}

/// Balance CPU load (convenience function)
pub fn balance_cpu_load() -> Result<(), CpuError> {
    let optimizer = get_cpu_optimizer().ok_or(CpuError::Unsupported)?;
    optimizer.balance_cpu_load()
}

/// Get CPU information (convenience function)
pub fn get_cpu_info(cpu_id: u32) -> Result<CpuInfo, CpuError> {
    let optimizer = get_cpu_optimizer().ok_or(CpuError::Unsupported)?;
    optimizer.get_cpu_info(cpu_id)
}

/// Get all CPU information (convenience function)
pub fn get_all_cpu_info() -> Option<Vec<CpuInfo>> {
    let optimizer = get_cpu_optimizer()?;
    Some(optimizer.get_all_cpu_info())
}

/// Get optimizer statistics (convenience function)
pub fn get_cpu_optimizer_stats() -> Option<CpuOptimizerStats> {
    let optimizer = get_cpu_optimizer()?;
    Some(optimizer.get_stats())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_governor_types() {
        let perf = GovernorType::Performance;
        let save = GovernorType::Powersave;
        let ondemand = GovernorType::Ondemand { up_threshold: 80, down_threshold: 20 };

        assert!(!perf.is_dynamic());
        assert!(!save.is_dynamic());
        assert!(ondemand.is_dynamic());
    }

    #[test]
    fn test_cstate_properties() {
        assert_eq!(CState::C0.depth(), 0);
        assert_eq!(CState::C8.depth(), 6);
        assert!(CState::C8.exit_latency_us() > CState::C1.exit_latency_us());
    }

    #[test]
    fn test_load_stats() {
        let stats = CpuLoadStats {
            user: 100,
            system: 50,
            idle: 50,
            iowait: 0,
            irq: 0,
            softirq: 0,
            steal: 0,
            guest: 0,
        };

        assert_eq!(stats.total(), 200);
        assert_eq!(stats.utilization(), 75);
    }

    #[test]
    fn test_cpu_topology() {
        let topo = CpuTopology {
            sockets: 2,
            cores_per_socket: 4,
            threads_per_core: 2,
            total_cpus: 16,
            numa_nodes: 2,
            cache_topology: CacheTopology::default(),
        };

        assert_eq!(topo.total_cores(), 8);
        assert_eq!(topo.total_cpus, 16);
    }

    #[test]
    fn test_governor_configs() {
        let perf = GovernorConfig::performance();
        assert!(matches!(perf.governor_type, GovernorType::Performance));

        let ondemand = GovernorConfig::ondemand(80, 20);
        match ondemand.governor_type {
            GovernorType::Ondemand { up_threshold, down_threshold } => {
                assert_eq!(up_threshold, 80);
                assert_eq!(down_threshold, 20);
            }
            _ => panic!("Expected Ondemand governor"),
        }
    }
}
