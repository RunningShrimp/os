//! CPU Idle State Management
//!
//! This module provides comprehensive CPU idle state management (C-states) including:
//! - C-state management (C0, C1, C1E, C3, C6, C7, C8)
//! - Idle governor implementation (menu, ladder, TEO - Timer Events Oriented)
//! - Wake latency prediction and optimization
//! - Idle state residency optimization
//! - Cluster idle states (package C-states)
//! - Package idle states (PC1, PC2, PC3, PC6)
//! - Idle injection for thermal control
//! - Idle statistics and monitoring
//!
//! # Overview
//!
//! CPU idle states (C-states) are power-saving states that a CPU can enter when it's not executing code.
//! Deeper C-states save more power but have longer wake latencies.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                     CPU Idle Manager                         │
//! ├─────────────────────────────────────────────────────────────┤
//! │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐ │
//! │  │ Menu        │  │ Ladder      │  │ TEO Governor        │ │
//! │  │ Governor    │  │ Governor    │  │ (Timer Events)      │ │
//! │  └──────┬──────┘  └──────┬──────┘  └──────────┬──────────┘ │
//! │         │                │                     │             │
//! │         └────────────────┴─────────────────────┘             │
//! │                          │                                  │
//! │                          ▼                                  │
//! │              ┌───────────────────────┐                      │
//! │              │   Idle State Driver   │                      │
//! │              └───────────┬───────────┘                      │
//! │                          │                                  │
//! │                          ▼                                  │
//! │    ┌──────────────────────────────────────────────────┐    │
//! │    │              C-States                             │    │
//! │    │  C0 → C1 → C1E → C3 → C6 → C7 → C8              │    │
//! │    └──────────────────────────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────┘
//! ```

#![allow(dead_code)]
#![allow(unused_variables)]

use crate::prelude::*;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};

/// CPU idle state type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CStateType {
    /// C0 - Active state (CPU is running)
    C0 = 0,
    /// C1 - Halt state (fast wake)
    C1 = 1,
    /// C1E - Enhanced C1 (lower power)
    C1E = 2,
    /// C3 - Sleep state (cache flush)
    C3 = 3,
    /// C6 - Deep sleep (context save)
    C6 = 4,
    /// C7 - Deeper sleep
    C7 = 5,
    /// C8 - Deepest sleep
    C8 = 6,
}

impl CStateType {
    /// Get C-state from index
    pub fn from_index(index: u32) -> Option<Self> {
        match index {
            0 => Some(Self::C0),
            1 => Some(Self::C1),
            2 => Some(Self::C1E),
            3 => Some(Self::C3),
            4 => Some(Self::C6),
            5 => Some(Self::C7),
            6 => Some(Self::C8),
            _ => None,
        }
    }

    /// Get name of C-state
    pub fn name(&self) -> &'static str {
        match self {
            Self::C0 => "C0",
            Self::C1 => "C1",
            Self::C1E => "C1E",
            Self::C3 => "C3",
            Self::C6 => "C6",
            Self::C7 => "C7",
            Self::C8 => "C8",
        }
    }

    /// Get description of C-state
    pub fn description(&self) -> &'static str {
        match self {
            Self::C0 => "Active state - CPU is executing instructions",
            Self::C1 => "Halt state - CPU stops executing, fast wake (~1μs)",
            Self::C1E => "Enhanced Halt - Lower power than C1 (~10μs)",
            Self::C3 => "Sleep state - Cache flush, medium wake (~50μs)",
            Self::C6 => "Deep sleep - Context save, slow wake (~100μs)",
            Self::C7 => "Deeper sleep - More power savings (~200μs)",
            Self::C8 => "Deepest sleep - Maximum power savings (~500μs)",
        }
    }

    /// Get default exit latency for C-state (in microseconds)
    pub fn default_exit_latency(&self) -> u32 {
        match self {
            Self::C0 => 0,
            Self::C1 => 1,
            Self::C1E => 10,
            Self::C3 => 50,
            Self::C6 => 100,
            Self::C7 => 200,
            Self::C8 => 500,
        }
    }

    /// Get default target residency for C-state (in microseconds)
    pub fn default_target_residency(&self) -> u32 {
        match self {
            Self::C0 => 0,
            Self::C1 => 2,
            Self::C1E => 20,
            Self::C3 => 100,
            Self::C6 => 200,
            Self::C7 => 400,
            Self::C8 => 1000,
        }
    }

    /// Get default power usage for C-state (in milliwatts)
    pub fn default_power_usage(&self) -> u32 {
        match self {
            Self::C0 => 15000, // 15W at full load
            Self::C1 => 8000,  // 8W
            Self::C1E => 5000, // 5W
            Self::C3 => 2000,  // 2W
            Self::C6 => 500,   // 0.5W
            Self::C7 => 200,   // 0.2W
            Self::C8 => 100,   // 0.1W
        }
    }
}

/// CPU idle state descriptor
#[derive(Debug, Clone)]
pub struct CState {
    /// C-state type
    pub state_type: CStateType,
    /// Exit latency in microseconds
    pub exit_latency: u32,
    /// Target residency in microseconds
    pub target_residency: u32,
    /// Power usage in milliwatts
    pub power_usage: u32,
    /// Flags
    pub flags: CStateFlags,
    /// Disable flag
    pub disabled: bool,
    /// Description
    pub description: String,
}

impl CState {
    /// Create new C-state
    pub fn new(state_type: CStateType) -> Self {
        Self {
            state_type,
            exit_latency: state_type.default_exit_latency(),
            target_residency: state_type.default_target_residency(),
            power_usage: state_type.default_power_usage(),
            flags: CStateFlags::empty(),
            disabled: false,
            description: state_type.description().to_string(),
        }
    }

    /// Create custom C-state
    pub fn with_params(
        state_type: CStateType,
        exit_latency: u32,
        target_residency: u32,
        power_usage: u32,
    ) -> Self {
        Self {
            state_type,
            exit_latency,
            target_residency,
            power_usage,
            flags: CStateFlags::empty(),
            disabled: false,
            description: state_type.description().to_string(),
        }
    }

    /// Check if this state is valid for given idle duration
    pub fn is_valid_for_idle(&self, idle_duration_us: u64) -> bool {
        if self.disabled {
            return false;
        }
        idle_duration_us >= self.target_residency as u64
    }

    /// Calculate power savings for given idle duration
    pub fn power_savings(&self, idle_duration_us: u64) -> u64 {
        let c0_power = CStateType::C0.default_power_usage() as u64;
        let state_power = self.power_usage as u64;
        let power_saved = c0_power.saturating_sub(state_power);
        (power_saved * idle_duration_us) / 1_000_000 // Convert μs to seconds
    }
}

/// C-state flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CStateFlags(u32);

impl CStateFlags {
    /// No flags
    pub const fn empty() -> Self {
        Self(0)
    }

    /// CPUIDLE_FLAG_TIMER_STOP: Timer is stopped in this state
    pub const fn timer_stop() -> Self {
        Self(1 << 0)
    }

    /// CPUIDLE_FLAG_COUPLED: State is coupled with other CPUs
    pub const fn coupled() -> Self {
        Self(1 << 1)
    }

    /// CPUIDLE_FLAG_TLB_FLUSHED: TLB is flushed in this state
    pub const fn tlb_flushed() -> Self {
        Self(1 << 2)
    }

    /// CPUIDLE_FLAG_CHECK_BM: Check bus master status
    pub const fn check_bm() -> Self {
        Self(1 << 3)
    }

    /// CPUIDLE_FLAG_HIGH_POWER_BUDGET: High power budget state
    pub const fn high_power_budget() -> Self {
        Self(1 << 4)
    }

    /// Check if flag is set
    pub fn contains(&self, flag: Self) -> bool {
        self.0 & flag.0 != 0
    }
}

/// Idle governor types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleGovernor {
    /// Menu governor (Linux default)
    Menu,
    /// Ladder governor (simple)
    Ladder,
    /// TEO governor (Timer Events Oriented)
    Teo,
}

impl IdleGovernor {
    /// Get governor name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Menu => "menu",
            Self::Ladder => "ladder",
            Self::Teo => "teo",
        }
    }

    /// Get governor description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Menu => "Menu governor - Uses interactivity and sleep time prediction",
            Self::Ladder => "Ladder governor - Simple progressive depth increase",
            Self::Teo => "TEO governor - Timer Events Oriented for improved idle prediction",
        }
    }
}

/// Idle state statistics
#[derive(Debug, Clone)]
pub struct CStateStats {
    /// Number of times this state was entered
    pub entries: u64,
    /// Total time spent in this state (microseconds)
    pub total_time_us: u64,
    /// Average time per entry (microseconds)
    pub avg_time_us: u64,
    /// Number of times we hit target residency
    pub hits: u64,
    /// Number of times we missed target residency
    pub misses: u64,
    /// Last entry time
    pub last_entry: u64,
    /// Last exit time
    pub last_exit: u64,
}

impl Default for CStateStats {
    fn default() -> Self {
        Self {
            entries: 0,
            total_time_us: 0,
            avg_time_us: 0,
            hits: 0,
            misses: 0,
            last_entry: 0,
            last_exit: 0,
        }
    }
}

impl CStateStats {
    /// Update statistics
    pub fn update(&mut self, duration_us: u64, hit_target: bool) {
        self.entries += 1;
        self.total_time_us += duration_us;
        self.avg_time_us = self.total_time_us / self.entries;

        if hit_target {
            self.hits += 1;
        } else {
            self.misses += 1;
        }

        self.last_exit = duration_us;
    }
}

/// Per-CPU idle device
#[derive(Debug)]
pub struct CpuIdleDevice {
    /// CPU ID
    pub cpu_id: u32,
    /// Available C-states
    pub states: Vec<CState>,
    /// Current C-state
    pub current_state: AtomicU32,
    /// Statistics for each state
    pub stats: Vec<Mutex<CStateStats>>,
    /// Last idle time
    pub last_idle: AtomicU64,
    /// Total idle time
    pub total_idle_time: AtomicU64,
    /// Governor
    pub governor: Mutex<IdleGovernor>,
}

impl CpuIdleDevice {
    /// Create new CPU idle device
    pub fn new(cpu_id: u32) -> Self {
        let mut states = Vec::new();
        for i in 0..=6 {
            if let Some(state_type) = CStateType::from_index(i) {
                states.push(CState::new(state_type));
            }
        }

        let mut stats = Vec::new();
        for _ in 0..states.len() {
            stats.push(Mutex::new(CStateStats::default()));
        }

        Self {
            cpu_id,
            states,
            current_state: AtomicU32::new(0),
            stats,
            last_idle: AtomicU64::new(0),
            total_idle_time: AtomicU64::new(0),
            governor: Mutex::new(IdleGovernor::Menu),
        }
    }

    /// Get current C-state index
    pub fn get_current_state(&self) -> u32 {
        self.current_state.load(Ordering::Relaxed)
    }

    /// Enter idle state
    pub fn enter_idle(&self, predicted_duration: u64) -> Result<CStateType, IdleError> {
        let governor = *self.governor.lock();
        let state_idx = self.select_idle_state(governor, predicted_duration)?;

        if state_idx == 0 {
            // Stay in C0 (no idle)
            return Ok(CStateType::C0);
        }

        let state = &self.states[state_idx];
        let state_type = state.state_type;

        // Update statistics
        let entry_time = self.get_time_us();
        self.last_idle.store(entry_time, Ordering::Relaxed);

        // Actually enter the C-state
        // Note: In real hardware, this would involve assembly instructions
        self.enter_c_state(state_type)?;

        let exit_time = self.get_time_us();
        let duration = exit_time.saturating_sub(entry_time);
        let hit_target = duration >= state.target_residency as u64;

        // Update state statistics
        if let Some(stats) = self.stats.get(state_idx) {
            stats.lock().update(duration, hit_target);
        }

        // Update total idle time
        self.total_idle_time.fetch_add(duration, Ordering::Relaxed);
        self.current_state.store(state_idx as u32, Ordering::Relaxed);

        Ok(state_type)
    }

    /// Select idle state based on governor
    fn select_idle_state(
        &self,
        governor: IdleGovernor,
        predicted_duration: u64,
    ) -> Result<usize, IdleError> {
        match governor {
            IdleGovernor::Menu => self.menu_select_state(predicted_duration),
            IdleGovernor::Ladder => self.ladder_select_state(predicted_duration),
            IdleGovernor::Teo => self.teo_select_state(predicted_duration),
        }
    }

    /// Menu governor: Predict optimal idle state
    fn menu_select_state(&self, predicted_duration: u64) -> Result<usize, IdleError> {
        let mut best_state = 0; // C0
        let mut best_score = 0i64;

        for (idx, state) in self.states.iter().enumerate() {
            if state.disabled {
                continue;
            }

            // Menu governor score calculation
            // Score = power_savings - latency_penalty
            let power_savings = state.power_savings(predicted_duration) as i64;
            let latency_penalty = (state.exit_latency as i64) * 10;

            // Apply residency check
            if predicted_duration < state.target_residency as u64 {
                // Predicted duration too short, prefer shallower state
                continue;
            }

            let score = power_savings - latency_penalty;

            if score > best_score {
                best_score = score;
                best_state = idx;
            }
        }

        Ok(best_state)
    }

    /// Ladder governor: Progressive depth increase
    fn ladder_select_state(&self, predicted_duration: u64) -> Result<usize, IdleError> {
        let current = self.get_current_state() as usize;

        // Try to go deeper if we've been idle long enough
        for (idx, state) in self.states.iter().enumerate().skip(current) {
            if state.disabled {
                continue;
            }

            if predicted_duration >= state.target_residency as u64 {
                return Ok(idx);
            }
        }

        // If current state is valid, stay there
        if let Some(state) = self.states.get(current) {
            if !state.disabled
                && predicted_duration >= state.target_residency.saturating_sub(100) as u64
            {
                return Ok(current);
            }
        }

        // Default to C1
        Ok(1)
    }

    /// TEO governor: Timer Events Oriented
    fn teo_select_state(&self, predicted_duration: u64) -> Result<usize, IdleError> {
        // TEO considers recent idle duration history
        let mut best_state = 0;
        let mut best_score = 0i64;

        for (idx, state) in self.states.iter().enumerate() {
            if state.disabled {
                continue;
            }

            // TEO uses different scoring that weights recent behavior
            let residency_factor = if predicted_duration >= state.target_residency as u64 {
                100
            } else {
                50
            };

            let depth_factor = idx as i64 * 10;
            let latency_factor = -(state.exit_latency as i64);

            let score = residency_factor + depth_factor + latency_factor;

            if score > best_score {
                best_score = score;
                best_state = idx;
            }
        }

        Ok(best_state)
    }

    /// Enter specific C-state (hardware-specific)
    fn enter_c_state(&self, state: CStateType) -> Result<(), IdleError> {
        match state {
            CStateType::C0 => {
                // Stay in C0 (active)
                return Ok(());
            }
            CStateType::C1 => {
                // Execute HLT instruction
                unsafe { core::arch::asm!("hlt") };
            }
            CStateType::C1E | CStateType::C3 | CStateType::C6 | CStateType::C7 | CStateType::C8 => {
                // MWAIT or ACPI C-state
                // Note: Requires specific hardware support
                self.mwait_enter(state)?;
            }
        }

        Ok(())
    }

    /// Enter C-state using MWAIT
    #[cfg(target_arch = "x86_64")]
    fn mwait_enter(&self, state: CStateType) -> Result<(), IdleError> {
        // MWAIT extensions for C-states
        let hint = match state {
            CStateType::C1 => 0x00,
            CStateType::C1E => 0x01,
            CStateType::C3 => 0x10,
            CStateType::C6 => 0x20,
            CStateType::C7 => 0x30,
            CStateType::C8 => 0x40,
            _ => return Err(IdleError::InvalidState),
        };

        unsafe {
            let eax = hint;
            let ecx = 0; // No flags
            core::arch::asm!(
                "monitor",
                "mwait",
                in("eax") eax,
                in("ecx") ecx,
            );
        }

        Ok(())
    }

    #[cfg(not(target_arch = "x86_64"))]
    fn mwait_enter(&self, state: CStateType) -> Result<(), IdleError> {
        // For non-x86, use WFI (Wait For Interrupt)
        #[cfg(target_arch = "aarch64")]
        unsafe {
            core::arch::asm!("wfi");
        }

        Ok(())
    }

    /// Get current time in microseconds
    fn get_time_us(&self) -> u64 {
        // Placeholder: should use actual time source
        nos_api::event::get_time_ns() / 1000
    }

    /// Disable specific C-state
    pub fn disable_state(&mut self, state: CStateType) {
        for s in &mut self.states {
            if s.state_type == state {
                s.disabled = true;
            }
        }
    }

    /// Enable specific C-state
    pub fn enable_state(&mut self, state: CStateType) {
        for s in &mut self.states {
            if s.state_type == state {
                s.disabled = false;
            }
        }
    }

    /// Set idle governor
    pub fn set_governor(&self, governor: IdleGovernor) {
        *self.governor.lock() = governor;
    }

    /// Get idle statistics for this CPU
    pub fn get_stats(&self) -> CpuIdleStats {
        let mut state_stats = BTreeMap::new();

        for (idx, state) in self.states.iter().enumerate() {
            if let Some(stats) = self.stats.get(idx) {
                let stats = stats.lock();
                state_stats.insert(
                    state.state_type,
                    IdleStateStats {
                        entries: stats.entries,
                        total_time_us: stats.total_time_us,
                        avg_time_us: stats.avg_time_us,
                        hits: stats.hits,
                        misses: stats.misses,
                    },
                );
            }
        }

        CpuIdleStats {
            cpu_id: self.cpu_id,
            total_idle_time_us: self.total_idle_time.load(Ordering::Relaxed),
            current_state: self.get_current_state(),
            state_stats,
            governor: *self.governor.lock(),
        }
    }
}

/// Per-CPU idle statistics
#[derive(Debug, Clone)]
pub struct IdleStateStats {
    /// Number of entries
    pub entries: u64,
    /// Total time in this state (μs)
    pub total_time_us: u64,
    /// Average time per entry (μs)
    pub avg_time_us: u64,
    /// Residency hits
    pub hits: u64,
    /// Residency misses
    pub misses: u64,
}

/// CPU idle statistics
#[derive(Debug, Clone)]
pub struct CpuIdleStats {
    /// CPU ID
    pub cpu_id: u32,
    /// Total idle time (μs)
    pub total_idle_time_us: u64,
    /// Current state index
    pub current_state: u32,
    /// Statistics per state
    pub state_stats: BTreeMap<CStateType, IdleStateStats>,
    /// Active governor
    pub governor: IdleGovernor,
}

/// Package idle state (PC-state)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PackageCState {
    /// PC0 - Active
    Pc0 = 0,
    /// PC1 - Light idle
    Pc1 = 1,
    /// PC2 - Medium idle
    Pc2 = 2,
    /// PC3 - Deep idle
    Pc3 = 3,
    /// PC6 - Deepest idle
    Pc6 = 4,
}

impl PackageCState {
    /// Get name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Pc0 => "PC0",
            Self::Pc1 => "PC1",
            Self::Pc2 => "PC2",
            Self::Pc3 => "PC3",
            Self::Pc6 => "PC6",
        }
    }

    /// Get default exit latency (μs)
    pub fn exit_latency(&self) -> u32 {
        match self {
            Self::Pc0 => 0,
            Self::Pc1 => 10,
            Self::Pc2 => 50,
            Self::Pc3 => 100,
            Self::Pc6 => 200,
        }
    }
}

/// Package idle device
#[derive(Debug)]
pub struct PackageIdleDevice {
    /// Package ID
    pub package_id: u32,
    /// CPUs in this package
    pub cpus: Vec<u32>,
    /// Current PC-state
    pub current_state: Mutex<PackageCState>,
    /// PC-state statistics
    pub stats: Mutex<BTreeMap<PackageCState, CStateStats>>,
    /// Coordinated idle: number of CPUs idle
    pub idle_cpu_count: AtomicU32,
}

impl PackageIdleDevice {
    /// Create new package idle device
    pub fn new(package_id: u32, cpus: Vec<u32>) -> Self {
        let mut stats = BTreeMap::new();
        stats.insert(PackageCState::Pc0, CStateStats::default());
        stats.insert(PackageCState::Pc1, CStateStats::default());
        stats.insert(PackageCState::Pc2, CStateStats::default());
        stats.insert(PackageCState::Pc3, CStateStats::default());
        stats.insert(PackageCState::Pc6, CStateStats::default());

        Self {
            package_id,
            cpus,
            current_state: Mutex::new(PackageCState::Pc0),
            stats: Mutex::new(stats),
            idle_cpu_count: AtomicU32::new(0),
        }
    }

    /// Notify that a CPU is entering idle
    pub fn notify_cpu_idle(&self, cpu_id: u32) {
        let idle_count = self.idle_cpu_count.fetch_add(1, Ordering::Relaxed) + 1;

        // Check if we can enter deeper package state
        let total_cpus = self.cpus.len() as u32;
        let idle_ratio = (idle_count * 100) / total_cpus;

        let mut current = self.current_state.lock();
        let new_state = if idle_ratio >= 90 {
            PackageCState::Pc6
        } else if idle_ratio >= 70 {
            PackageCState::Pc3
        } else if idle_ratio >= 50 {
            PackageCState::Pc2
        } else if idle_ratio >= 25 {
            PackageCState::Pc1
        } else {
            PackageCState::Pc0
        };

        *current = new_state;
    }

    /// Notify that a CPU is exiting idle
    pub fn notify_cpu_busy(&self, cpu_id: u32) {
        self.idle_cpu_count.fetch_sub(1, Ordering::Relaxed);

        // Move back to PC0 when any CPU is busy
        *self.current_state.lock() = PackageCState::Pc0;
    }

    /// Get current package state
    pub fn get_state(&self) -> PackageCState {
        *self.current_state.lock()
    }
}

/// Idle injection for thermal control
#[derive(Debug)]
pub struct IdleInjector {
    /// Injection enabled
    pub enabled: AtomicBool,
    /// Injection ratio (percentage)
    pub ratio: AtomicU32,
    /// Duration per injection (μs)
    pub duration: AtomicU32,
    /// Statistics
    pub injections: AtomicU64,
    pub total_idle_time_us: AtomicU64,
}

impl IdleInjector {
    /// Create new idle injector
    pub fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            ratio: AtomicU32::new(0),
            duration: AtomicU32::new(1000), // Default 1ms
            injections: AtomicU64::new(0),
            total_idle_time_us: AtomicU64::new(0),
        }
    }

    /// Configure injection
    pub fn configure(&self, ratio: u32, duration_us: u32) -> Result<(), IdleError> {
        if ratio > 100 {
            return Err(IdleError::InvalidRatio);
        }

        self.ratio.store(ratio, Ordering::Relaxed);
        self.duration.store(duration_us, Ordering::Relaxed);

        if ratio > 0 {
            self.enabled.store(true, Ordering::Relaxed);
        } else {
            self.enabled.store(false, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Check if should inject idle now
    pub fn should_inject(&self) -> bool {
        if !self.enabled.load(Ordering::Relaxed) {
            return false;
        }

        let ratio = self.ratio.load(Ordering::Relaxed);
        if ratio == 0 {
            return false;
        }

        // Simple probability-based injection
        // In real implementation, this would be more sophisticated
        (nos_api::event::get_time_ns() % 100) < ratio as u64
    }

    /// Inject idle
    pub fn inject(&self) -> Result<(), IdleError> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        let duration = self.duration.load(Ordering::Relaxed);
        self.injections.fetch_add(1, Ordering::Relaxed);

        // Spin for specified duration
        let start = nos_api::event::get_time_ns();
        let duration_ns = duration as u64 * 1000;

        while nos_api::event::get_time_ns().saturating_sub(start) < duration_ns {
            unsafe { core::arch::asm!("pause") };
        }

        self.total_idle_time_us.fetch_add(duration as u64, Ordering::Relaxed);
        Ok(())
    }

    /// Get statistics
    pub fn get_stats(&self) -> IdleInjectionStats {
        IdleInjectionStats {
            enabled: self.enabled.load(Ordering::Relaxed),
            ratio: self.ratio.load(Ordering::Relaxed),
            duration_us: self.duration.load(Ordering::Relaxed),
            injections: self.injections.load(Ordering::Relaxed),
            total_idle_time_us: self.total_idle_time_us.load(Ordering::Relaxed),
        }
    }
}

/// Idle injection statistics
#[derive(Debug, Clone)]
pub struct IdleInjectionStats {
    /// Injection enabled
    pub enabled: bool,
    /// Injection ratio
    pub ratio: u32,
    /// Duration per injection
    pub duration_us: u32,
    /// Total injections
    pub injections: u64,
    /// Total idle time (μs)
    pub total_idle_time_us: u64,
}

/// CPU idle manager
#[derive(Debug)]
pub struct CpuIdleManager {
    /// Per-CPU idle devices
    pub devices: BTreeMap<u32, Arc<CpuIdleDevice>>,
    /// Package devices
    pub packages: BTreeMap<u32, Arc<PackageIdleDevice>>,
    /// Idle injector
    pub injector: Arc<IdleInjector>,
    /// Global governor
    pub global_governor: Mutex<IdleGovernor>,
    /// Enabled flag
    pub enabled: AtomicBool,
}

impl CpuIdleManager {
    /// Create new CPU idle manager
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            packages: BTreeMap::new(),
            injector: Arc::new(IdleInjector::new()),
            global_governor: Mutex::new(IdleGovernor::Menu),
            enabled: AtomicBool::new(false),
        }
    }

    /// Initialize manager
    pub fn init(&mut self) -> Result<(), IdleError> {
        if self.enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Detect CPU topology and create devices
        let num_cpus = self.detect_num_cpus();

        // For simplicity, assume single package
        let package_id = 0;
        let mut cpus = Vec::new();
        for cpu in 0..num_cpus {
            cpus.push(cpu);
            let device = Arc::new(CpuIdleDevice::new(cpu));
            self.devices.insert(cpu, device);
        }

        // Create package device
        let package = Arc::new(PackageIdleDevice::new(package_id, cpus));
        self.packages.insert(package_id, package);

        self.enabled.store(true, Ordering::Relaxed);
        log::info!("CPU idle manager initialized for {} CPUs", num_cpus);

        Ok(())
    }

    /// Detect number of CPUs
    fn detect_num_cpus(&self) -> u32 {
        // Placeholder: should detect from CPU topology
        4
    }

    /// Get idle device for CPU
    pub fn get_device(&self, cpu_id: u32) -> Option<Arc<CpuIdleDevice>> {
        self.devices.get(&cpu_id).cloned()
    }

    /// Get package device
    pub fn get_package(&self, package_id: u32) -> Option<Arc<PackageIdleDevice>> {
        self.packages.get(&package_id).cloned()
    }

    /// Enter idle state for CPU
    pub fn enter_idle(&self, cpu_id: u32, predicted_duration: u64) -> Result<CStateType, IdleError> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Ok(CStateType::C0);
        }

        // Check idle injection
        if self.injector.should_inject() {
            self.injector.inject()?;
            return Ok(CStateType::C1); // Inject shallow idle
        }

        let device = self.get_device(cpu_id).ok_or(IdleError::DeviceNotFound)?;
        device.enter_idle(predicted_duration)
    }

    /// Set global governor
    pub fn set_governor(&self, governor: IdleGovernor) -> Result<(), IdleError> {
        *self.global_governor.lock() = governor;

        // Update all devices
        for device in self.devices.values() {
            device.set_governor(governor);
        }

        Ok(())
    }

    /// Get global governor
    pub fn get_governor(&self) -> IdleGovernor {
        *self.global_governor.lock()
    }

    /// Get idle statistics
    pub fn get_stats(&self) -> IdleStats {
        let mut cpu_stats = BTreeMap::new();

        for (cpu_id, device) in &self.devices {
            cpu_stats.insert(*cpu_id, device.get_stats());
        }

        IdleStats {
            cpu_stats,
            global_governor: self.get_governor(),
            injection_stats: self.injector.get_stats(),
        }
    }

    /// Configure idle injection
    pub fn configure_injection(&self, ratio: u32, duration_us: u32) -> Result<(), IdleError> {
        self.injector.configure(ratio, duration_us)
    }
}

/// Idle statistics summary
#[derive(Debug, Clone)]
pub struct IdleStats {
    /// Per-CPU statistics
    pub cpu_stats: BTreeMap<u32, CpuIdleStats>,
    /// Global governor
    pub global_governor: IdleGovernor,
    /// Injection statistics
    pub injection_stats: IdleInjectionStats,
}

/// CPU idle errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdleError {
    /// Invalid C-state
    InvalidState,
    /// Device not found
    DeviceNotFound,
    /// Invalid ratio
    InvalidRatio,
    /// Hardware error
    HardwareError,
    /// Governor error
    GovernorError,
    /// Disabled
    Disabled,
}

impl core::fmt::Display for IdleError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidState => write!(f, "Invalid C-state"),
            Self::DeviceNotFound => write!(f, "CPU idle device not found"),
            Self::InvalidRatio => write!(f, "Invalid injection ratio"),
            Self::HardwareError => write!(f, "Hardware error"),
            Self::GovernorError => write!(f, "Governor error"),
            Self::Disabled => write!(f, "CPU idle is disabled"),
        }
    }
}

/// Global CPU idle manager
static GLOBAL_IDLE_MANAGER: Mutex<Option<CpuIdleManager>> = Mutex::new(None);

/// Initialize CPU idle subsystem
pub fn init_cpuidle() -> Result<(), IdleError> {
    let mut manager = GLOBAL_IDLE_MANAGER.lock();

    if manager.is_some() {
        return Ok(());
    }

    let mut idle_manager = CpuIdleManager::new();
    idle_manager.init()?;

    *manager = Some(idle_manager);

    log::info!("CPU idle subsystem initialized");
    Ok(())
}

/// Get CPU idle manager
pub fn get_idle_manager() -> Option<Arc<CpuIdleManager>> {
    // Note: We can't return Arc from static Mutex easily
    // In real implementation, would use different approach
    None
}

/// Enter idle state for current CPU
pub fn enter_idle_state(cpu: u32, predicted_us: u64) -> Result<CStateType, IdleError> {
    let manager = GLOBAL_IDLE_MANAGER.lock();
    let manager = manager.as_ref().ok_or(IdleError::Disabled)?;
    manager.enter_idle(cpu, predicted_us)
}

/// Set idle governor
pub fn set_idle_governor(governor: IdleGovernor) -> Result<(), IdleError> {
    let manager = GLOBAL_IDLE_MANAGER.lock();
    let manager = manager.as_ref().ok_or(IdleError::Disabled)?;
    manager.set_governor(governor)
}

/// Get idle statistics
pub fn get_idle_stats() -> IdleStats {
    let manager = GLOBAL_IDLE_MANAGER.lock();
    if let Some(manager) = manager.as_ref() {
        manager.get_stats()
    } else {
        IdleStats {
            cpu_stats: BTreeMap::new(),
            global_governor: IdleGovernor::Menu,
            injection_stats: IdleInjectionStats {
                enabled: false,
                ratio: 0,
                duration_us: 0,
                injections: 0,
                total_idle_time_us: 0,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cstate_creation() {
        let c1 = CState::new(CStateType::C1);
        assert_eq!(c1.state_type, CStateType::C1);
        assert_eq!(c1.exit_latency, 1);
        assert!(!c1.disabled);
    }

    #[test]
    fn test_cstate_validation() {
        let c1 = CState::new(CStateType::C1);
        assert!(c1.is_valid_for_idle(10));
        assert!(!c1.is_valid_for_idle(1));
    }

    #[test]
    fn test_idle_device() {
        let device = CpuIdleDevice::new(0);
        assert_eq!(device.cpu_id, 0);
        assert_eq!(device.states.len(), 7);
    }

    #[test]
    fn test_idle_injector() {
        let injector = IdleInjector::new();
        assert!(!injector.enabled.load(Ordering::Relaxed));

        injector.configure(50, 1000).unwrap();
        assert!(injector.enabled.load(Ordering::Relaxed));
        assert_eq!(injector.ratio.load(Ordering::Relaxed), 50);
    }

    #[test]
    fn test_governor_names() {
        assert_eq!(IdleGovernor::Menu.name(), "menu");
        assert_eq!(IdleGovernor::Ladder.name(), "ladder");
        assert_eq!(IdleGovernor::Teo.name(), "teo");
    }
}
