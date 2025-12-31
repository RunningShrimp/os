//! CPU Frequency Scaling (P-states)
//!
//! This module provides comprehensive CPU frequency scaling including:
//! - P-state management (P0, P1, P2...) - performance states
//! - Governor implementation (performance, powersave, ondemand, conservative, schedutil)
//! - ACPI CPPC support (Collaborative Processor Performance Control)
//! - Intel Speed Select / AMD CPPC
//! - Turbo Boost control (intel_pstate)
//! - Frequency transition optimization
//! - Frequency statistics and monitoring
//!
//! # Overview
//!
//! CPU frequency scaling allows dynamic adjustment of CPU clock speed to balance
//! performance and power consumption. Higher frequencies provide better performance
//! but consume more power and generate more heat.
//!
//! # Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────┐
//! │                 CPU Frequency Manager                       │
//! ├────────────────────────────────────────────────────────────┤
//! │  ┌────────────┐  ┌────────────┐  ┌────────────────────┐   │
//! │  │Performance │  │ Powersave  │  │ Ondemand           │   │
//! │  │ Governor   │  │ Governor   │  │ Governor           │   │
//! │  └─────┬──────┘  └─────┬──────┘  └────────┬───────────┘   │
//! │        │                │                   │              │
//! │  ┌─────┴──────┐  ┌─────┴──────┐  ┌────────┴───────────┐   │
//! │  │Conservative│  │ Schedutil  │  │                    │   │
//! │  │ Governor   │  │ Governor   │  │                    │   │
//! │  └─────┬──────┘  └─────┬──────┘  └────────┬───────────┘   │
//! │        └────────────────┴──────────────────┘              │
//! │                           │                                │
//! │                           ▼                                │
//! │              ┌────────────────────────┐                    │
//! │              │   Frequency Driver     │                    │
//! │              │  - ACPI CPPC           │                    │
//! │              │  - Intel P-State       │                    │
//! │              │  - AMD CPPC            │                    │
//! │              └───────────┬────────────┘                    │
//! │                          │                                 │
//! │                          ▼                                 │
//! │    ┌──────────────────────────────────────────────────┐   │
//! │    │              P-States                             │   │
//! │    │  P0 (max) → P1 → P2 → ... → Pn (min)             │   │
//! │    └──────────────────────────────────────────────────┘   │
//! └────────────────────────────────────────────────────────────┘
//! ```

#![allow(dead_code)]
#![allow(unused_variables)]

use crate::prelude::*;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/// Frequency governor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreqGovernor {
    /// Performance governor - always at max frequency
    Performance,
    /// Powersave governor - always at min frequency
    Powersave,
    /// Ondemand governor - dynamic based on load
    Ondemand,
    /// Conservative governor - gradual changes
    Conservative,
    /// Schedutil governor - scheduler-based
    Schedutil,
}

impl FreqGovernor {
    /// Get governor name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Performance => "performance",
            Self::Powersave => "powersave",
            Self::Ondemand => "ondemand",
            Self::Conservative => "conservative",
            Self::Schedutil => "schedutil",
        }
    }

    /// Get governor description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Performance => "CPU runs at maximum frequency",
            Self::Powersave => "CPU runs at minimum frequency",
            Self::Ondemand => "Dynamic frequency based on CPU load",
            Self::Conservative => "Gradual frequency changes for smooth transitions",
            Self::Schedutil => "Scheduler-driven frequency for optimal performance/power",
        }
    }
}

/// P-state (performance state)
#[derive(Debug, Clone, Copy)]
pub struct PState {
    /// P-state number (0 = highest performance)
    pub number: u32,
    /// Frequency in kHz
    pub frequency: u32,
    /// Voltage in millivolts
    pub voltage: u32,
    /// Power usage in milliwatts
    pub power: u32,
    /// Transition latency (microseconds)
    pub transition_latency: u32,
}

impl PState {
    /// Create new P-state
    pub fn new(number: u32, frequency: u32, voltage: u32) -> Self {
        // Estimate power based on frequency and voltage
        // P = C * V^2 * f (simplified)
        let power = (frequency as u64 * voltage as u64 * voltage as u64 / 1_000_000_000) as u32;

        Self {
            number,
            frequency,
            voltage,
            power: power.max(1),
            transition_latency: 10, // Default 10μs
        }
    }

    /// Check if this is turbo state
    pub fn is_turbo(&self) -> bool {
        self.number == 0 && self.frequency > 3_000_000 // Assume > 3GHz is turbo
    }
}

/// CPU frequency device
#[derive(Debug)]
pub struct CpuFreqDevice {
    /// CPU ID
    pub cpu_id: u32,
    /// Current frequency (kHz)
    pub current_freq: AtomicU32,
    /// Minimum frequency (kHz)
    pub min_freq: u32,
    /// Maximum frequency (kHz)
    pub max_freq: u32,
    /// Available P-states
    pub pstates: Vec<PState>,
    /// Current governor
    pub governor: Mutex<FreqGovernor>,
    /// Turbo boost enabled
    pub turbo_enabled: AtomicBool,
    /// Statistics
    pub stats: Mutex<FreqStats>,
    /// Frequency table
    pub freq_table: Vec<u32>,
}

impl CpuFreqDevice {
    /// Create new CPU frequency device
    pub fn new(cpu_id: u32) -> Self {
        // Generate P-states
        let mut pstates = Vec::new();
        let base_freq = 3_000_000; // 3 GHz base
        let base_voltage = 1000; // 1.0V

        for i in 0..8 {
            let freq = base_freq - (i * 200_000); // 200MHz steps
            let voltage = base_voltage - (i * 50); // 50mV steps
            pstates.push(PState::new(i, freq.max(800_000), voltage.max(700)));
        }

        let mut freq_table: Vec<u32> = pstates.iter().map(|p| p.frequency).collect();
        freq_table.sort();
        freq_table.reverse();

        Self {
            cpu_id,
            current_freq: AtomicU32::new(base_freq),
            min_freq: 800_000,
            max_freq: base_freq,
            pstates,
            governor: Mutex::new(FreqGovernor::Ondemand),
            turbo_enabled: AtomicBool::new(false),
            stats: Mutex::new(FreqStats::default()),
            freq_table,
        }
    }

    /// Get current frequency
    pub fn get_frequency(&self) -> u32 {
        self.current_freq.load(Ordering::Relaxed)
    }

    /// Set frequency
    pub fn set_frequency(&self, freq_khz: u32) -> Result<(), FreqError> {
        // Clamp to range
        let freq = freq_khz.clamp(self.min_freq, self.max_freq);

        // Find matching P-state
        let pstate = self.find_pstate(freq).ok_or(FreqError::InvalidFrequency)?;

        // Transition to new P-state
        self.transition_to_pstate(&pstate)?;

        // Update current frequency
        self.current_freq.store(freq, Ordering::Relaxed);

        // Update statistics
        let mut stats = self.stats.lock();
        stats.transitions += 1;
        stats.last_transition_time = self.get_time_us();

        Ok(())
    }

    /// Find P-state for frequency
    fn find_pstate(&self, freq: u32) -> Option<PState> {
        self.pstates
            .iter()
            .find(|p| p.frequency == freq)
            .copied()
            .or_else(|| {
                // Find closest P-state
                let mut closest = self.pstates.first()?;
                for pstate in &self.pstates {
                    if (pstate.frequency as i64 - freq as i64).abs()
                        < (closest.frequency as i64 - freq as i64).abs()
                    {
                        closest = pstate;
                    }
                }
                Some(*closest)
            })
    }

    /// Transition to P-state
    fn transition_to_pstate(&self, pstate: &PState) -> Result<(), FreqError> {
        // In real implementation, this would:
        // 1. Write to MSR/ACPI registers
        // 2. Wait for transition
        // 3. Verify new state

        #[cfg(target_arch = "x86_64")]
        {
            // Intel MSR write would go here
            // wrmsrl(IA32_PERF_CTL, pstate.control);
        }

        // Placeholder
        Ok(())
    }

    /// Set governor
    pub fn set_governor(&self, governor: FreqGovernor) -> Result<(), FreqError> {
        *self.governor.lock() = governor;

        // Apply governor immediately
        self.apply_governor()?;

        Ok(())
    }

    /// Get current governor
    pub fn get_governor(&self) -> FreqGovernor {
        *self.governor.lock()
    }

    /// Apply governor policy
    pub fn apply_governor(&self) -> Result<(), FreqError> {
        let governor = self.get_governor();
        let load = self.get_cpu_load();

        let target_freq = match governor {
            FreqGovernor::Performance => self.max_freq,
            FreqGovernor::Powersave => self.min_freq,
            FreqGovernor::Ondemand => self.ondemand_select_freq(load),
            FreqGovernor::Conservative => self.conservative_select_freq(load),
            FreqGovernor::Schedutil => self.schedutil_select_freq(load),
        };

        self.set_frequency(target_freq)
    }

    /// Ondemand governor: select frequency based on load
    fn ondemand_select_freq(&self, load: u8) -> u32 {
        const UP_THRESHOLD: u8 = 80;
        const DOWN_THRESHOLD: u8 = 20;

        let current = self.get_frequency();

        if load > UP_THRESHOLD {
            // Go to max frequency
            self.max_freq
        } else if load < DOWN_THRESHOLD {
            // Go to min frequency
            self.min_freq
        } else {
            // Proportional frequency
            let range = self.max_freq - self.min_freq;
            self.min_freq + (range * load as u32 / 100)
        }
    }

    /// Conservative governor: gradual frequency changes
    fn conservative_select_freq(&self, load: u8) -> u32 {
        const UP_THRESHOLD: u8 = 80;
        const DOWN_THRESHOLD: u8 = 20;
        const STEP: u32 = 200_000; // 200MHz steps

        let current = self.get_frequency();

        if load > UP_THRESHOLD {
            // Step up
            (current + STEP).min(self.max_freq)
        } else if load < DOWN_THRESHOLD {
            // Step down
            current.saturating_sub(STEP).max(self.min_freq)
        } else {
            current
        }
    }

    /// Schedutil governor: scheduler-driven
    fn schedutil_select_freq(&self, load: u8) -> u32 {
        // Schedutil considers:
        // 1. Current CPU utilization
        // 2. Scheduler signals (task wakeup, migration)
        // 3. Energy efficiency

        // Simplified implementation
        let current = self.get_frequency();

        if load > 90 {
            self.max_freq
        } else if load < 30 {
            self.min_freq
        } else {
            // More aggressive than ondemand
            let range = self.max_freq - self.min_freq;
            let freq = self.min_freq + (range * load as u32 * 2 / 100);
            freq.min(self.max_freq)
        }
    }

    /// Enable turbo boost
    pub fn enable_turbo(&self) -> Result<(), FreqError> {
        self.turbo_enabled.store(true, Ordering::Relaxed);

        #[cfg(target_arch = "x86_64")]
        {
            // Enable turbo via MSR
            // Intel: IA32_PERF_CTL bit 32
            // Placeholder
        }

        Ok(())
    }

    /// Disable turbo boost
    pub fn disable_turbo(&self) -> Result<(), FreqError> {
        self.turbo_enabled.store(false, Ordering::Relaxed);

        #[cfg(target_arch = "x86_64")]
        {
            // Disable turbo via MSR
            // Placeholder
        }

        Ok(())
    }

    /// Check if turbo is enabled
    pub fn is_turbo_enabled(&self) -> bool {
        self.turbo_enabled.load(Ordering::Relaxed)
    }

    /// Get CPU load (0-100)
    fn get_cpu_load(&self) -> u8 {
        // Placeholder: should get actual load from scheduler
        50
    }

    /// Get available frequencies
    pub fn available_frequencies(&self) -> &[u32] {
        &self.freq_table
    }

    /// Get statistics
    pub fn get_stats(&self) -> FreqStats {
        let mut stats = self.stats.lock();
        stats.current_freq = self.get_frequency();
        stats.turbo_enabled = self.is_turbo_enabled();
        stats.governor = self.get_governor();
        stats.clone()
    }

    /// Get current time in microseconds
    fn get_time_us(&self) -> u64 {
        nos_api::event::get_time_ns() / 1000
    }
}

/// Frequency statistics
#[derive(Debug, Clone)]
pub struct FreqStats {
    /// Current frequency (kHz)
    pub current_freq: u32,
    /// Minimum frequency ever used (kHz)
    pub min_freq_seen: u32,
    /// Maximum frequency ever used (kHz)
    pub max_freq_seen: u32,
    /// Number of transitions
    pub transitions: u64,
    /// Total time at each frequency
    pub time_in_freq: BTreeMap<u32, u64>,
    /// Last transition time (μs)
    pub last_transition_time: u64,
    /// Turbo enabled
    pub turbo_enabled: bool,
    /// Current governor
    pub governor: FreqGovernor,
}

impl Default for FreqStats {
    fn default() -> Self {
        Self {
            current_freq: 0,
            min_freq_seen: u32::MAX,
            max_freq_seen: 0,
            transitions: 0,
            time_in_freq: BTreeMap::new(),
            last_transition_time: 0,
            turbo_enabled: false,
            governor: FreqGovernor::Ondemand,
        }
    }
}

/// ACPI CPPC (Collaborative Processor Performance Control)
#[derive(Debug)]
pub struct Cppc {
    /// Highest performance
    pub highest_perf: u32,
    /// Lowest performance
    pub lowest_perf: u32,
    /// Nominal performance
    pub nominal_perf: u32,
    /// Guaranteed performance
    pub guaranteed_perf: u32,
    /// Minimum performance
    pub min_perf: u32,
    /// Maximum performance
    pub max_perf: u32,
    /// Desired performance
    pub desired_perf: u32,
}

impl Cppc {
    /// Create new CPPC
    pub fn new() -> Self {
        Self {
            highest_perf: 255,
            lowest_perf: 0,
            nominal_perf: 200,
            guaranteed_perf: 180,
            min_perf: 0,
            max_perf: 255,
            desired_perf: 0,
        }
    }

    /// Set desired performance
    pub fn set_desired_perf(&mut self, perf: u32) -> Result<(), FreqError> {
        self.desired_perf = perf.clamp(self.lowest_perf, self.highest_perf);
        Ok(())
    }

    /// Get performance level (0-255)
    pub fn get_perf_level(&self) -> u32 {
        self.desired_perf
    }

    /// Convert to frequency (approximate)
    pub fn perf_to_freq(&self, perf: u32, max_freq: u32) -> u32 {
        if self.highest_perf == 0 {
            return max_freq;
        }

        let ratio = perf as u64 * 1000 / self.highest_perf as u64;
        (max_freq as u64 * ratio / 1000) as u32
    }
}

/// Intel Speed Select
#[derive(Debug)]
pub struct SpeedSelect {
    /// SST-CP (Control Power) enabled
    pub cp_enabled: bool,
    /// SST-TF (Turbo Frequency) enabled
    pub tf_enabled: bool,
    /// SST-BF (Base Frequency) enabled
    pub bf_enabled: bool,
    /// Current CP level
    pub cp_level: u32,
}

impl SpeedSelect {
    /// Create new Speed Select
    pub fn new() -> Self {
        Self {
            cp_enabled: false,
            tf_enabled: false,
            bf_enabled: false,
            cp_level: 0,
        }
    }

    /// Enable SST-CP (Control Power)
    pub fn enable_cp(&mut self, level: u32) -> Result<(), FreqError> {
        self.cp_enabled = true;
        self.cp_level = level.clamp(0, 4);
        Ok(())
    }

    /// Disable SST-CP
    pub fn disable_cp(&mut self) {
        self.cp_enabled = false;
        self.cp_level = 0;
    }

    /// Enable SST-TF (Turbo Frequency)
    pub fn enable_tf(&mut self) -> Result<(), FreqError> {
        self.tf_enabled = true;
        Ok(())
    }

    /// Enable SST-BF (Base Frequency)
    pub fn enable_bf(&mut self) -> Result<(), FreqError> {
        self.bf_enabled = true;
        Ok(())
    }
}

/// CPU frequency manager
#[derive(Debug)]
pub struct CpuFreqManager {
    /// CPU frequency devices
    pub devices: BTreeMap<u32, Arc<CpuFreqDevice>>,
    /// Global governor
    pub global_governor: Mutex<FreqGovernor>,
    /// CPPC
    pub cppc: Mutex<Cppc>,
    /// Speed Select
    pub speed_select: Mutex<SpeedSelect>,
    /// Enabled flag
    pub enabled: AtomicBool,
}

impl CpuFreqManager {
    /// Create new CPU frequency manager
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            global_governor: Mutex::new(FreqGovernor::Ondemand),
            cppc: Mutex::new(Cppc::new()),
            speed_select: Mutex::new(SpeedSelect::new()),
            enabled: AtomicBool::new(false),
        }
    }

    /// Initialize manager
    pub fn init(&mut self) -> Result<(), FreqError> {
        if self.enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Detect CPUs
        let num_cpus = self.detect_num_cpus();

        for cpu in 0..num_cpus {
            let device = Arc::new(CpuFreqDevice::new(cpu));
            self.devices.insert(cpu, device);
        }

        self.enabled.store(true, Ordering::Relaxed);
        log::info!("CPU frequency manager initialized for {} CPUs", num_cpus);

        Ok(())
    }

    /// Detect number of CPUs
    fn detect_num_cpus(&self) -> u32 {
        // Placeholder: should detect from hardware
        4
    }

    /// Get device for CPU
    pub fn get_device(&self, cpu_id: u32) -> Option<Arc<CpuFreqDevice>> {
        self.devices.get(&cpu_id).cloned()
    }

    /// Set CPU frequency
    pub fn set_frequency(&self, cpu: u32, freq_khz: u32) -> Result<(), FreqError> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Err(FreqError::Disabled);
        }

        let device = self.get_device(cpu).ok_or(FreqError::DeviceNotFound)?;
        device.set_frequency(freq_khz)
    }

    /// Get CPU frequency
    pub fn get_frequency(&self, cpu: u32) -> Result<u32, FreqError> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Err(FreqError::Disabled);
        }

        let device = self.get_device(cpu).ok_or(FreqError::DeviceNotFound)?;
        Ok(device.get_frequency())
    }

    /// Set frequency governor
    pub fn set_governor(&self, governor: FreqGovernor) -> Result<(), FreqError> {
        *self.global_governor.lock() = governor;

        // Apply to all CPUs
        for device in self.devices.values() {
            device.set_governor(governor)?;
        }

        Ok(())
    }

    /// Get frequency governor
    pub fn get_governor(&self) -> FreqGovernor {
        *self.global_governor.lock()
    }

    /// Enable turbo boost
    pub fn enable_turbo_boost(&self) -> Result<(), FreqError> {
        for device in self.devices.values() {
            device.enable_turbo()?;
        }

        log::info!("Turbo boost enabled for all CPUs");
        Ok(())
    }

    /// Disable turbo boost
    pub fn disable_turbo_boost(&self) -> Result<(), FreqError> {
        for device in self.devices.values() {
            device.disable_turbo()?;
        }

        log::info!("Turbo boost disabled for all CPUs");
        Ok(())
    }

    /// Get frequency statistics
    pub fn get_stats(&self) -> FreqManagerStats {
        let mut cpu_stats = BTreeMap::new();

        for (cpu_id, device) in &self.devices {
            cpu_stats.insert(*cpu_id, device.get_stats());
        }

        FreqManagerStats {
            cpu_stats,
            global_governor: self.get_governor(),
            turbo_enabled: self.devices.values().next().map_or(false, |d| d.is_turbo_enabled()),
        }
    }

    /// Update all governors (called periodically)
    pub fn update_governors(&self) -> Result<(), FreqError> {
        for device in self.devices.values() {
            device.apply_governor()?;
        }

        Ok(())
    }
}

/// Frequency manager statistics
#[derive(Debug, Clone)]
pub struct FreqManagerStats {
    /// Per-CPU statistics
    pub cpu_stats: BTreeMap<u32, FreqStats>,
    /// Global governor
    pub global_governor: FreqGovernor,
    /// Turbo enabled
    pub turbo_enabled: bool,
}

/// CPU frequency errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreqError {
    /// Device not found
    DeviceNotFound,
    /// Invalid frequency
    InvalidFrequency,
    /// Transition failed
    TransitionFailed,
    /// Governor error
    GovernorError,
    /// Not supported
    NotSupported,
    /// Disabled
    Disabled,
}

impl core::fmt::Display for FreqError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DeviceNotFound => write!(f, "CPU frequency device not found"),
            Self::InvalidFrequency => write!(f, "Invalid frequency"),
            Self::TransitionFailed => write!(f, "Frequency transition failed"),
            Self::GovernorError => write!(f, "Governor error"),
            Self::NotSupported => write!(f, "Operation not supported"),
            Self::Disabled => write!(f, "CPU frequency scaling is disabled"),
        }
    }
}

/// Global CPU frequency manager
static GLOBAL_FREQ_MANAGER: Mutex<Option<CpuFreqManager>> = Mutex::new(None);

/// Initialize CPU frequency subsystem
pub fn init_cpufreq() -> Result<(), FreqError> {
    let mut manager = GLOBAL_FREQ_MANAGER.lock();

    if manager.is_some() {
        return Ok(());
    }

    let mut freq_manager = CpuFreqManager::new();
    freq_manager.init()?;

    *manager = Some(freq_manager);

    log::info!("CPU frequency subsystem initialized");
    Ok(())
}

/// Set CPU frequency
pub fn set_cpu_frequency(cpu: u32, freq_khz: u32) -> Result<(), FreqError> {
    let manager = GLOBAL_FREQ_MANAGER.lock();
    let manager = manager.as_ref().ok_or(FreqError::Disabled)?;
    manager.set_frequency(cpu, freq_khz)
}

/// Get CPU frequency
pub fn get_cpu_frequency(cpu: u32) -> Result<u32, FreqError> {
    let manager = GLOBAL_FREQ_MANAGER.lock();
    let manager = manager.as_ref().ok_or(FreqError::Disabled)?;
    manager.get_frequency(cpu)
}

/// Set frequency governor
pub fn set_frequency_governor(governor: FreqGovernor) -> Result<(), FreqError> {
    let manager = GLOBAL_FREQ_MANAGER.lock();
    let manager = manager.as_ref().ok_or(FreqError::Disabled)?;
    manager.set_governor(governor)
}

/// Enable turbo boost
pub fn enable_turbo_boost() -> Result<(), FreqError> {
    let manager = GLOBAL_FREQ_MANAGER.lock();
    let manager = manager.as_ref().ok_or(FreqError::Disabled)?;
    manager.enable_turbo_boost()
}

/// Disable turbo boost
pub fn disable_turbo_boost() -> Result<(), FreqError> {
    let manager = GLOBAL_FREQ_MANAGER.lock();
    let manager = manager.as_ref().ok_or(FreqError::Disabled)?;
    manager.disable_turbo_boost()
}

/// Get frequency statistics
pub fn get_freq_stats() -> Option<FreqManagerStats> {
    let manager = GLOBAL_FREQ_MANAGER.lock();
    manager.as_ref()?.get_stats().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_governor_names() {
        assert_eq!(FreqGovernor::Performance.name(), "performance");
        assert_eq!(FreqGovernor::Powersave.name(), "powersave");
        assert_eq!(FreqGovernor::Ondemand.name(), "ondemand");
    }

    #[test]
    fn test_pstate() {
        let p0 = PState::new(0, 3_000_000, 1000);
        assert_eq!(p0.number, 0);
        assert_eq!(p0.frequency, 3_000_000);
        assert!(!p0.is_turbo()); // 3GHz is not considered turbo in test
    }

    #[test]
    fn test_cpu_freq_device() {
        let device = CpuFreqDevice::new(0);
        assert_eq!(device.cpu_id, 0);
        assert_eq!(device.get_frequency(), 3_000_000);
        assert!(!device.is_turbo_enabled());
    }

    #[test]
    fn test_cppc() {
        let mut cppc = Cppc::new();
        assert_eq!(cppc.highest_perf, 255);

        cppc.set_desired_perf(200).unwrap();
        assert_eq!(cppc.get_perf_level(), 200);

        let freq = cppc.perf_to_freq(200, 3_000_000);
        assert!(freq > 2_000_000);
    }

    #[test]
    fn test_freq_manager() {
        let mut manager = CpuFreqManager::new();
        manager.init().unwrap();

        assert_eq!(manager.devices.len(), 4);
        assert!(manager.enabled.load(Ordering::Relaxed));
    }
}
