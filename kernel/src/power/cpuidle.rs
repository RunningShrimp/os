//! CPU Idle Management Framework
//!
//! This module implements CPU idle state management (C-states) similar to Linux's cpuidle subsystem.
//! It provides:
//! - C-state management (C0, C1, C1E, C3, C6, etc.)
//! - Idle driver framework
//! - Wakeup latency calculation
//! - Power consumption estimation
//!
//! ## C-States
//!
//! - **C0**: Active state - CPU is executing instructions
//! - **C1**: Halt state - CPU stops executing but maintains context
//! - **C1E**: Enhanced C1 with lower power
//! - **C3**: Sleep state - Cache coherence maintained
//! - **C6**: Deep power down - Context saved in memory
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::power::cpuidle::{CpuIdle, CState};
//!
//! // Initialize CPU idle manager
//! let mut cpuidle = CpuIdle::new(0)?;
//!
//! // Enter idle state
//! let entered = cpuidle.enter_idle()?;
//!
//! // Get statistics
//! let stats = cpuidle.stats();
//! ```

use crate::error::UnifiedError;
use spin::{Mutex, Once};
use alloc::vec::Vec;

/// CPU identifier
pub type CpuId = u32;

/// Wakeup latency in microseconds
pub type LatencyUs = u32;

/// Power consumption in milliwatts
pub type PowerMw = u32;

/// Residency time in microseconds
pub type ResidencyUs = u32;

/// CPU idle manager instance
static CPU_IDLE_MANAGER: Once<Mutex<CpuIdleManager>> = Once::new();

/// Initialize the global CPU idle manager
pub fn init() -> Result<(), UnifiedError> {
    let manager = CpuIdleManager::new()?;
    CPU_IDLE_MANAGER.call_once(|| Mutex::new(manager));
    Ok(())
}

/// Get the global CPU idle manager
pub fn get_manager() -> Option<&'static Mutex<CpuIdleManager>> {
    CPU_IDLE_MANAGER.get()
}

/// C-State (CPU idle state) descriptor
#[derive(Debug, Clone)]
pub struct CState {
    /// State identifier (0 = C0 active, higher = deeper idle)
    pub id: u32,
    /// State name
    pub name: &'static str,
    /// State description
    pub description: &'static str,
    /// Exit latency (microseconds)
    pub exit_latency: LatencyUs,
    /// Target residency (microseconds) - minimum time to stay in this state
    pub target_residency: ResidencyUs,
    /// Power consumption (milliwatts)
    pub power_usage: PowerMw,
    /// Flags for state capabilities
    pub flags: CStateFlags,
}

bitflags::bitflags! {
    /// C-State capability flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct CStateFlags: u32 {
        /// State can be entered from any state
        const ENTER_ANY = 1 << 0;
        /// State maintains cache coherence
        const CACHE_COHERENT = 1 << 1;
        /// State timer stops
        const TIMER_STOP = 1 << 2;
        /// State is valid for all CPUs
        const VALID_FOR_ALL = 1 << 3;
        /// State disables interrupts
        const IRQ_DISABLE = 1 << 4;
    }
}

impl CState {
    /// Create new C-State
    pub fn new(
        id: u32,
        name: &'static str,
        description: &'static str,
        exit_latency: LatencyUs,
        target_residency: ResidencyUs,
        power_usage: PowerMw,
    ) -> Self {
        CState {
            id,
            name,
            description,
            exit_latency,
            target_residency,
            power_usage,
            flags: CStateFlags::empty(),
        }
    }

    /// Create new C-State with flags
    pub fn with_flags(
        id: u32,
        name: &'static str,
        description: &'static str,
        exit_latency: LatencyUs,
        target_residency: ResidencyUs,
        power_usage: PowerMw,
        flags: CStateFlags,
    ) -> Self {
        CState {
            id,
            name,
            description,
            exit_latency,
            target_residency,
            power_usage,
            flags,
        }
    }

    /// Check if state is deeper than another
    pub fn is_deeper_than(&self, other: &CState) -> bool {
        self.id > other.id
    }

    /// Calculate power savings compared to C0
    pub fn power_savings(&self, c0_power: PowerMw) -> PowerMw {
        c0_power.saturating_sub(self.power_usage)
    }
}

/// C-State usage statistics
#[derive(Debug, Clone)]
pub struct CStateStats {
    /// Number of times state was entered
    pub usage_count: u64,
    /// Total time spent in state (microseconds)
    pub total_time: u64,
    /// Time since last entry
    pub last_entry: Option<u64>,
    /// Time since last exit
    pub last_exit: Option<u64>,
}

impl CStateStats {
    /// Create new C-State statistics
    pub fn new() -> Self {
        CStateStats {
            usage_count: 0,
            total_time: 0,
            last_entry: None,
            last_exit: None,
        }
    }

    /// Record state entry
    pub fn record_entry(&mut self, timestamp: u64) {
        self.usage_count += 1;
        self.last_entry = Some(timestamp);
    }

    /// Record state exit
    pub fn record_exit(&mut self, timestamp: u64) {
        self.last_exit = Some(timestamp);
    }

    /// Add time spent in state
    pub fn add_time(&mut self, duration_us: u64) {
        self.total_time += duration_us;
    }

    /// Get average time per entry
    pub fn average_time(&self) -> u64 {
        if self.usage_count == 0 {
            return 0;
        }
        self.total_time / self.usage_count
    }
}

/// CPU idle state manager
#[derive(Debug)]
pub struct CpuIdle {
    /// CPU identifier
    cpu_id: CpuId,
    /// Available C-states
    states: Vec<CState>,
    /// Current state (0 = active/C0)
    current_state: u32,
    /// Per-state statistics
    stats: Vec<CStateStats>,
    /// State disabled flags
    disabled: Vec<bool>,
    /// Last idle entry time
    last_idle_time: Option<u64>,
}

impl Clone for CpuIdle {
    fn clone(&self) -> Self {
        Self {
            cpu_id: self.cpu_id,
            states: self.states.clone(),
            current_state: self.current_state,
            stats: self.stats.clone(),
            disabled: self.disabled.clone(),
            last_idle_time: self.last_idle_time,
        }
    }
}

impl CpuIdle {
    /// Create new CPU idle manager
    pub fn new(cpu_id: CpuId, states: Vec<CState>) -> Result<Self, UnifiedError> {
        if states.is_empty() {
            return Err(UnifiedError::Other("C-states list cannot be empty".into()));
        }

        // Ensure states are sorted by ID
        let mut states = states;
        states.sort_by_key(|s| s.id);

        let stats_count = states.len();
        Ok(CpuIdle {
            cpu_id,
            states,
            current_state: 0,
            stats: vec![CStateStats::new(); stats_count],
            disabled: vec![false; stats_count],
            last_idle_time: None,
        })
    }

    /// Get CPU identifier
    pub fn cpu_id(&self) -> CpuId {
        self.cpu_id
    }

    /// Get available states
    pub fn states(&self) -> &[CState] {
        &self.states
    }

    /// Get current state
    pub fn current_state(&self) -> u32 {
        self.current_state
    }

    /// Get state statistics
    pub fn stats(&self) -> &[CStateStats] {
        &self.stats
    }

    /// Check if state is disabled
    pub fn is_disabled(&self, state_id: u32) -> bool {
        self.states
            .iter()
            .position(|s| s.id == state_id)
            .map(|idx| self.disabled[idx])
            .unwrap_or(true)
    }

    /// Enable/disable state
    pub fn set_state_enabled(&mut self, state_id: u32, enabled: bool) -> Result<(), UnifiedError> {
        let idx = self
            .states
            .iter()
            .position(|s| s.id == state_id)
            .ok_or_else(|| UnifiedError::Other("State not found".into()))?;

        self.disabled[idx] = !enabled;
        Ok(())
    }

    /// Select optimal idle state based on predicted idle time
    pub fn select_state(&self, predicted_us: u64) -> Option<&CState> {
        // Start from deepest state and work backwards
        for state in self.states.iter().rev() {
            // Skip disabled states
            if let Some(idx) = self.states.iter().position(|s| s.id == state.id) {
                if self.disabled[idx] {
                    continue;
                }
            }

            // Check if predicted idle time meets target residency
            if predicted_us >= state.target_residency as u64 {
                return Some(state);
            }
        }

        // Return shallowest state (C1) if no suitable state found
        self.states.iter().find(|s| s.id == 1)
    }

    /// Enter idle state
    pub fn enter_idle(&mut self) -> Result<u32, UnifiedError> {
        // Predict idle time (simple heuristic)
        let predicted_us = self.predict_idle_time();

        // Select optimal state
        let state = self
            .select_state(predicted_us)
            .ok_or_else(|| UnifiedError::Other("No suitable idle state".into()))?;

        // Get state ID before recording entry time
        let state_id = state.id;

        // Record entry time
        let entry_time = self.get_timestamp();
        self.last_idle_time = Some(entry_time);

        // Update statistics
        let state_idx = self.states.iter().position(|s| s.id == state_id).unwrap();
        self.stats[state_idx].record_entry(entry_time);

        // Update current state
        self.current_state = state_id;

        // In a real implementation, this would execute the actual idle instruction
        // (e.g., HLT, MWAIT, WFI)

        Ok(state_id)
    }

    /// Exit idle state
    pub fn exit_idle(&mut self) -> Result<(), UnifiedError> {
        if self.current_state == 0 {
            return Ok(()); // Already in active state
        }

        // Record exit time
        let exit_time = self.get_timestamp();

        // Update statistics
        let state_idx = self.states.iter().position(|s| s.id == self.current_state).unwrap();
        self.stats[state_idx].record_exit(exit_time);

        // Calculate time spent in state
        if let Some(entry_time) = self.last_idle_time {
            let duration_us = exit_time - entry_time;
            self.stats[state_idx].add_time(duration_us);
        }

        // Return to active state
        self.current_state = 0;

        Ok(())
    }

    /// Predict next idle time (microseconds)
    fn predict_idle_time(&self) -> u64 {
        // Simple heuristic: use average time in shallowest state
        // In a real implementation, this would use more sophisticated prediction
        if let Some(c1_stats) = self.stats.first() {
            if c1_stats.usage_count > 0 {
                return c1_stats.average_time();
            }
        }

        // Default prediction
        100 // 100μs default
    }

    /// Get timestamp (microseconds since boot)
    fn get_timestamp(&self) -> u64 {
        // In a real implementation, this would read from a hardware timer
        0
    }

    /// Get total idle time
    pub fn total_idle_time(&self) -> u64 {
        self.stats.iter().map(|s| s.total_time).sum()
    }

    /// Get total idle transitions
    pub fn total_idle_transitions(&self) -> u64 {
        self.stats.iter().map(|s| s.usage_count).sum()
    }

    /// Calculate power savings (milliwatt-microseconds)
    pub fn power_savings(&self, c0_power: PowerMw) -> u64 {
        let mut total_savings = 0u64;

        for (state, stats) in self.states.iter().zip(self.stats.iter()) {
            let savings_per_us = state.power_savings(c0_power) as u64;
            total_savings += savings_per_us * stats.total_time;
        }

        total_savings
    }
}

/// CPU idle manager
#[derive(Debug)]
pub struct CpuIdleManager {
    /// Per-CPU idle devices
    devices: Vec<Option<CpuIdle>>,
    /// Global idle time accumulator
    total_idle_time: u64,
    /// Total transitions
    total_transitions: u64,
}

impl CpuIdleManager {
    /// Create new CPU idle manager
    pub fn new() -> Result<Self, UnifiedError> {
        Ok(CpuIdleManager {
            devices: Vec::new(),
            total_idle_time: 0,
            total_transitions: 0,
        })
    }

    /// Add CPU idle device
    pub fn add_cpu(&mut self, cpu_id: CpuId, states: Vec<CState>) -> Result<(), UnifiedError> {
        // Ensure vector is large enough
        if cpu_id as usize >= self.devices.len() {
            self.devices.resize(cpu_id as usize + 1, None);
        }

        let device = CpuIdle::new(cpu_id, states)?;
        self.devices[cpu_id as usize] = Some(device);
        Ok(())
    }

    /// Get CPU idle device
    pub fn get_cpu(&self, cpu_id: CpuId) -> Option<&CpuIdle> {
        self.devices.get(cpu_id as usize)?.as_ref()
    }

    /// Get CPU idle device (mutable)
    pub fn get_cpu_mut(&mut self, cpu_id: CpuId) -> Option<&mut CpuIdle> {
        self.devices.get_mut(cpu_id as usize)?.as_mut()
    }

    /// Put CPU into idle state
    pub fn cpu_idle(&mut self, cpu_id: CpuId) -> Result<u32, UnifiedError> {
        let device = self
            .get_cpu_mut(cpu_id)
            .ok_or_else(|| UnifiedError::Other(format!("CPU {} not found", cpu_id)))?;

        let state_id = device.enter_idle()?;
        self.total_transitions += 1;

        Ok(state_id)
    }

    /// Wake CPU from idle state
    pub fn cpu_wake(&mut self, cpu_id: CpuId) -> Result<(), UnifiedError> {
        let device = self
            .get_cpu_mut(cpu_id)
            .ok_or_else(|| UnifiedError::Other(format!("CPU {} not found", cpu_id)))?;

        device.exit_idle()?;
        Ok(())
    }

    /// Update global statistics
    pub fn update_stats(&mut self) {
        self.total_idle_time = 0;
        for device in self.devices.iter().flatten() {
            self.total_idle_time += device.total_idle_time();
        }
    }

    /// Get total idle time across all CPUs
    pub fn total_idle_time(&self) -> u64 {
        self.total_idle_time
    }

    /// Get total transitions across all CPUs
    pub fn total_transitions(&self) -> u64 {
        self.total_transitions
    }

    /// Get idle ratio (0-100%)
    pub fn idle_ratio(&self, uptime_us: u64) -> u8 {
        if uptime_us == 0 {
            return 0;
        }

        let total_cpu_time = self.devices.len() as u64 * uptime_us;
        let ratio = (self.total_idle_time * 100 / total_cpu_time) as u8;
        ratio.min(100)
    }
}

/// Default x86_64 C-states
pub fn default_x86_cstates() -> Vec<CState> {
    vec![
        CState::new(1, "C1", "Halt", 1, 2, 500),          // C1: HLT
        CState::new(2, "C1E", "Enhanced Halt", 10, 20, 200), // C1E: Enhanced Halt
        CState::with_flags(
            3,
            "C3",
            "Sleep",
            50,
            100,
            100,
            CStateFlags::CACHE_COHERENT | CStateFlags::TIMER_STOP,
        ), // C3: Sleep
        CState::with_flags(
            4,
            "C6",
            "Deep Power Down",
            100,
            200,
            50,
            CStateFlags::TIMER_STOP,
        ), // C6: Deep Power Down
    ]
}

/// Default ARM C-states
pub fn default_arm_cstates() -> Vec<CState> {
    vec![
        CState::new(1, "WFI", "Wait For Interrupt", 1, 2, 400), // WFI: ARM's equivalent to C1
        CState::with_flags(
            2,
            "CPU_SLEEP",
            "CPU Sleep",
            10,
            50,
            150,
            CStateFlags::CACHE_COHERENT,
        ), // CPU_SLEEP
        CState::with_flags(
            3,
            "CPU_POWERDOWN",
            "CPU Power Down",
            100,
            200,
            50,
            CStateFlags::TIMER_STOP,
        ), // CPU_POWERDOWN
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpuidle_creation() {
        let states = default_x86_cstates();
        let cpuidle = CpuIdle::new(0, states).unwrap();
        assert_eq!(cpuidle.cpu_id(), 0);
        assert_eq!(cpuidle.current_state(), 0);
        assert_eq!(cpuidle.states().len(), 4);
    }

    #[test]
    fn test_state_selection() {
        let states = default_x86_cstates();
        let cpuidle = CpuIdle::new(0, states).unwrap();

        // Short idle time should select C1
        let state = cpuidle.select_state(5).unwrap();
        assert_eq!(state.id, 1);

        // Longer idle time should select deeper state
        let state = cpuidle.select_state(150).unwrap();
        assert_eq!(state.id, 3);
    }

    #[test]
    fn test_idle_entry() {
        let states = default_x86_cstates();
        let mut cpuidle = CpuIdle::new(0, states).unwrap();

        let state_id = cpuidle.enter_idle().unwrap();
        assert_ne!(state_id, 0); // Should enter some idle state
        assert_eq!(cpuidle.current_state(), state_id);
    }

    #[test]
    fn test_idle_exit() {
        let states = default_x86_cstates();
        let mut cpuidle = CpuIdle::new(0, states).unwrap();

        cpuidle.enter_idle().unwrap();
        cpuidle.exit_idle().unwrap();

        assert_eq!(cpuidle.current_state(), 0);
    }

    #[test]
    fn test_state_disable() {
        let states = default_x86_cstates();
        let mut cpuidle = CpuIdle::new(0, states).unwrap();

        cpuidle.set_state_enabled(3, false).unwrap();
        assert!(cpuidle.is_disabled(3));

        // Should not select disabled state
        let state = cpuidle.select_state(150).unwrap();
        assert_ne!(state.id, 3);
    }

    #[test]
    fn test_cstate_comparison() {
        let c1 = CState::new(1, "C1", "Halt", 1, 2, 500);
        let c3 = CState::new(3, "C3", "Sleep", 50, 100, 100);

        assert!(c3.is_deeper_than(&c1));
        assert!(!c1.is_deeper_than(&c3));

        assert_eq!(c1.power_savings(1000), 500);
        assert_eq!(c3.power_savings(1000), 900);
    }

    #[test]
    fn test_statistics() {
        let states = default_x86_cstates();
        let mut cpuidle = CpuIdle::new(0, states).unwrap();

        cpuidle.enter_idle().unwrap();
        cpuidle.exit_idle().unwrap();

        assert_eq!(cpuidle.total_idle_transitions(), 1);
        assert!(cpuidle.total_idle_time() >= 0);
    }
}
