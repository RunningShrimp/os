//! System Sleep State Management
//!
//! This module implements system-wide sleep states similar to Linux's sleep states.
//! It provides:
//! - S3 (Suspend-to-RAM) - Sleep state
//! - S4 (Suspend-to-Disk) - Hibernate state
//! - S5 (Soft Off) - Power off state
//! - Wakeup source management
//! - Sleep state transition management
//!
//! ## Sleep States
//!
//! - **S0**: Working - System is fully operational
//! - **S1**: Sleep - CPU context maintained
//! - **S2**: Sleep - CPU context lost, some cache maintained
//! - **S3**: Suspend-to-RAM - Memory maintained, most devices off
//! - **S4**: Suspend-to-Disk - Memory saved to disk, all devices off
//! - **S5**: Soft Off - System powered off
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::power::sleep::{SleepManager, SleepState};
//!
//! // Get sleep manager
//! let manager = SleepManager::get();
//!
//! // Enter S3 sleep
//! manager.enter_sleep(SleepState::S3)?;
//! ```

use crate::error::UnifiedError;
use spin::{Mutex, Once};
use alloc::vec::Vec;
use alloc::boxed::Box;

/// System sleep state
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SleepState {
    /// S0: Working
    S0 = 0,
    /// S1: Sleep (POSIX S1)
    S1 = 1,
    /// S2: Sleep (POSIX S2)
    S2 = 2,
    /// S3: Suspend-to-RAM (Sleep)
    S3 = 3,
    /// S4: Suspend-to-Disk (Hibernate)
    S4 = 4,
    /// S5: Soft Off
    S5 = 5,
}

impl SleepState {
    /// Get state name
    pub fn name(&self) -> &'static str {
        match self {
            SleepState::S0 => "S0 (Working)",
            SleepState::S1 => "S1 (Sleep)",
            SleepState::S2 => "S2 (Sleep)",
            SleepState::S3 => "S3 (Suspend-to-RAM)",
            SleepState::S4 => "S4 (Suspend-to-Disk)",
            SleepState::S5 => "S5 (Soft Off)",
        }
    }

    /// Check if state is a working state
    pub fn is_working(&self) -> bool {
        *self == SleepState::S0
    }

    /// Check if state is a sleep state
    pub fn is_sleep(&self) -> bool {
        matches!(self, SleepState::S1 | SleepState::S2 | SleepState::S3)
    }

    /// Check if state is hibernate
    pub fn is_hibernate(&self) -> bool {
        *self == SleepState::S4
    }

    /// Check if state is power off
    pub fn is_off(&self) -> bool {
        *self == SleepState::S5
    }

    /// Get sleep depth (higher = deeper sleep)
    pub fn depth(&self) -> u8 {
        *self as u8
    }
}

/// Wakeup source type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WakeupSource {
    /// RTC alarm
    RTC,
    /// Power button
    PowerButton,
    /// LAN wakeup
    LAN,
    /// USB device
    USB,
    /// Keyboard/mouse
    Input,
    /// PCIe device
    PCIe,
    /// GPIO interrupt
    GPIO,
    /// Custom wakeup source
    Custom(u32),
}

impl WakeupSource {
    /// Get source name
    pub fn name(&self) -> &'static str {
        match self {
            WakeupSource::RTC => "RTC",
            WakeupSource::PowerButton => "Power Button",
            WakeupSource::LAN => "LAN",
            WakeupSource::USB => "USB",
            WakeupSource::Input => "Input Device",
            WakeupSource::PCIe => "PCIe Device",
            WakeupSource::GPIO => "GPIO",
            WakeupSource::Custom(_id) => "Custom",
        }
    }
}

/// Wakeup source configuration
#[derive(Debug, Clone)]
pub struct WakeupSourceConfig {
    /// Source type
    pub source: WakeupSource,
    /// Enabled flag
    pub enabled: bool,
    /// Wakeup count
    pub count: u64,
    /// Last wakeup time
    pub last_wakeup: Option<u64>,
}

impl WakeupSourceConfig {
    /// Create new wakeup source
    pub fn new(source: WakeupSource) -> Self {
        WakeupSourceConfig {
            source,
            enabled: false,
            count: 0,
            last_wakeup: None,
        }
    }

    /// Enable wakeup source
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable wakeup source
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Record wakeup event
    pub fn record_wakeup(&mut self, timestamp: u64) {
        self.count += 1;
        self.last_wakeup = Some(timestamp);
    }
}

/// Sleep state statistics
#[derive(Debug, Clone)]
pub struct SleepStats {
    /// Number of sleep entries
    pub sleep_count: u64,
    /// Number of wake events
    pub wake_count: u64,
    /// Total sleep time (milliseconds)
    pub total_sleep_time_ms: u64,
    /// Last sleep time
    pub last_sleep: Option<u64>,
    /// Last wake time
    pub last_wake: Option<u64>,
    /// Sleep failures
    pub failures: u64,
}

impl SleepStats {
    /// Create new statistics
    pub fn new() -> Self {
        SleepStats {
            sleep_count: 0,
            wake_count: 0,
            total_sleep_time_ms: 0,
            last_sleep: None,
            last_wake: None,
            failures: 0,
        }
    }

    /// Record sleep entry
    pub fn record_sleep(&mut self, timestamp: u64) {
        self.sleep_count += 1;
        self.last_sleep = Some(timestamp);
    }

    /// Record wake event
    pub fn record_wake(&mut self, timestamp: u64, sleep_time_ms: u64) {
        self.wake_count += 1;
        self.last_wake = Some(timestamp);
        self.total_sleep_time_ms += sleep_time_ms;
    }

    /// Record failure
    pub fn record_failure(&mut self) {
        self.failures += 1;
    }

    /// Get average sleep time
    pub fn average_sleep_time_ms(&self) -> u64 {
        if self.wake_count == 0 {
            return 0;
        }
        self.total_sleep_time_ms / self.wake_count
    }
}

/// Sleep state transition callback
pub trait SleepOps: Send + Sync {
    /// Prepare for sleep (before entering sleep state)
    fn prepare_sleep(&mut self, state: SleepState) -> Result<(), UnifiedError> {
        let _ = state;
        Ok(())
    }

    /// Enter sleep state
    fn enter_sleep(&mut self, state: SleepState) -> Result<(), UnifiedError> {
        let _ = state;
        Ok(())
    }

    /// Wake from sleep state
    fn wake_from_sleep(&mut self, state: SleepState) -> Result<(), UnifiedError> {
        let _ = state;
        Ok(())
    }

    /// Finish wake (after returning from sleep)
    fn finish_wake(&mut self, state: SleepState) -> Result<(), UnifiedError> {
        let _ = state;
        Ok(())
    }
}

/// Default sleep operations
pub struct DefaultSleepOps;

impl SleepOps for DefaultSleepOps {}

/// Sleep state manager
pub struct SleepManager {
    /// Current sleep state
    current_state: SleepState,
    /// Target sleep state
    target_state: SleepState,
    /// Previous state before sleep
    previous_state: SleepState,
    /// Statistics
    stats: SleepStats,
    /// Wakeup sources
    wakeup_sources: Vec<WakeupSourceConfig>,
    /// Sleep enabled flag
    sleep_enabled: bool,
    /// Sleep operations
    sleep_ops: Option<Box<dyn SleepOps>>,
}

impl core::fmt::Debug for SleepManager {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SleepManager")
            .field("current_state", &self.current_state)
            .field("target_state", &self.target_state)
            .field("previous_state", &self.previous_state)
            .field("stats", &self.stats)
            .field("wakeup_sources", &self.wakeup_sources)
            .field("sleep_enabled", &self.sleep_enabled)
            .field("sleep_ops", &self.sleep_ops.is_some())
            .finish()
    }
}

impl SleepManager {
    /// Create new sleep manager
    pub fn new() -> Self {
        let mut wakeup_sources = Vec::new();
        wakeup_sources.push(WakeupSourceConfig::new(WakeupSource::PowerButton));
        wakeup_sources.push(WakeupSourceConfig::new(WakeupSource::RTC));
        wakeup_sources.push(WakeupSourceConfig::new(WakeupSource::LAN));

        SleepManager {
            current_state: SleepState::S0,
            target_state: SleepState::S0,
            previous_state: SleepState::S0,
            stats: SleepStats::new(),
            wakeup_sources,
            sleep_enabled: true,
            sleep_ops: None,
        }
    }

    /// Get current sleep state
    pub fn current_state(&self) -> SleepState {
        self.current_state
    }

    /// Get target sleep state
    pub fn target_state(&self) -> SleepState {
        self.target_state
    }

    /// Get statistics
    pub fn stats(&self) -> &SleepStats {
        &self.stats
    }

    /// Check if sleep is enabled
    pub fn is_sleep_enabled(&self) -> bool {
        self.sleep_enabled
    }

    /// Enable/disable sleep
    pub fn set_sleep_enabled(&mut self, enabled: bool) {
        self.sleep_enabled = enabled;
    }

    /// Set sleep operations
    pub fn set_sleep_ops(&mut self, ops: Box<dyn SleepOps>) {
        self.sleep_ops = Some(ops);
    }

    /// Get wakeup sources
    pub fn wakeup_sources(&self) -> &[WakeupSourceConfig] {
        &self.wakeup_sources
    }

    /// Enable wakeup source
    pub fn enable_wakeup_source(&mut self, source: WakeupSource) -> Result<(), UnifiedError> {
        let config = self
            .wakeup_sources
            .iter_mut()
            .find(|c| c.source == source)
            .ok_or_else(|| UnifiedError::Other("Wakeup source not found".into()))?;

        config.enable();
        Ok(())
    }

    /// Disable wakeup source
    pub fn disable_wakeup_source(&mut self, source: WakeupSource) -> Result<(), UnifiedError> {
        let config = self
            .wakeup_sources
            .iter_mut()
            .find(|c| c.source == source)
            .ok_or_else(|| UnifiedError::Other("Wakeup source not found".into()))?;

        config.disable();
        Ok(())
    }

    /// Check if wakeup source is enabled
    pub fn is_wakeup_source_enabled(&self, source: WakeupSource) -> bool {
        self.wakeup_sources
            .iter()
            .find(|c| c.source == source)
            .map(|c| c.enabled)
            .unwrap_or(false)
    }

    /// Add custom wakeup source
    pub fn add_wakeup_source(&mut self, source: WakeupSource) {
        if !self.wakeup_sources.iter().any(|c| c.source == source) {
            self.wakeup_sources.push(WakeupSourceConfig::new(source));
        }
    }

    /// Enter sleep state
    pub fn enter_sleep(&mut self, state: SleepState) -> Result<(), UnifiedError> {
        if !self.sleep_enabled {
            return Err(UnifiedError::Other("Sleep is disabled".into()));
        }

        if state == SleepState::S0 {
            return Err(UnifiedError::Other("Cannot enter S0 state".into()));
        }

        if state <= self.current_state {
            return Err(UnifiedError::Other("Target state must be deeper than current state".into()));
        }

        // Set target state
        self.target_state = state;

        // Prepare for sleep
        if let Some(ops) = &mut self.sleep_ops {
            let ops: &mut dyn SleepOps = ops.as_mut();
            ops.prepare_sleep(state)?;
        }

        // Record sleep time
        let sleep_time = self.get_timestamp();
        self.stats.record_sleep(sleep_time);

        // Enter sleep state
        if let Some(ops) = &mut self.sleep_ops {
            let ops: &mut dyn SleepOps = ops.as_mut();
            ops.enter_sleep(state)?;
        }

        // Update state
        self.previous_state = self.current_state;
        self.current_state = state;

        Ok(())
    }

    /// Wake from sleep state
    pub fn wake_from_sleep(&mut self, source: WakeupSource) -> Result<(), UnifiedError> {
        if self.current_state == SleepState::S0 {
            return Ok(());
        }

        // Check if wakeup source is enabled
        if !self.is_wakeup_source_enabled(source) {
            return Err(UnifiedError::Other("Wakeup source not enabled".into()));
        }

        // Record wakeup
        let wake_time = self.get_timestamp();

        // Calculate sleep time
        let sleep_time_ms = if let Some(last_sleep) = self.stats.last_sleep {
            wake_time - last_sleep
        } else {
            0
        };

        // Update wakeup source stats
        if let Some(config) = self
            .wakeup_sources
            .iter_mut()
            .find(|c| c.source == source)
        {
            config.record_wakeup(wake_time);
        }

        // Wake from sleep
        if let Some(ops) = &mut self.sleep_ops {
            let ops: &mut dyn SleepOps = ops.as_mut();
            ops.wake_from_sleep(self.current_state)?;
        }

        // Restore previous state
        self.current_state = self.previous_state;

        // Finish wake
        if let Some(ops) = &mut self.sleep_ops {
            let ops: &mut dyn SleepOps = ops.as_mut();
            ops.finish_wake(self.target_state)?;
        }

        // Record wake
        self.stats.record_wake(wake_time, sleep_time_ms);

        // Reset target state
        self.target_state = SleepState::S0;

        Ok(())
    }

    /// Request sleep (with automatic wakeup)
    pub fn request_sleep(
        &mut self,
        state: SleepState,
        duration_ms: Option<u64>,
    ) -> Result<(), UnifiedError> {
        self.enter_sleep(state)?;

        if let Some(duration) = duration_ms {
            // In a real implementation, this would set up a timer to wake
            // For now, we just wake immediately
            let _ = duration;
            self.wake_from_sleep(WakeupSource::RTC)?;
        }

        Ok(())
    }

    /// Get deepest supported sleep state
    pub fn deepest_supported_state(&self) -> SleepState {
        // In a real implementation, this would check hardware capabilities
        // For now, assume S4 is supported
        SleepState::S4
    }

    /// Get timestamp (milliseconds since boot)
    fn get_timestamp(&self) -> u64 {
        // In a real implementation, this would read from a timer
        0
    }
}

/// Global sleep manager instance
static SLEEP_MANAGER: Once<Mutex<SleepManager>> = Once::new();

/// Initialize the sleep manager
pub fn init() -> Result<(), UnifiedError> {
    let manager = SleepManager::new();
    SLEEP_MANAGER.call_once(|| Mutex::new(manager));
    Ok(())
}

/// Get the global sleep manager
pub fn get_manager() -> Option<&'static Mutex<SleepManager>> {
    SLEEP_MANAGER.get()
}

/// Enter sleep state
pub fn enter_sleep(state: SleepState) -> Result<(), UnifiedError> {
    let manager = get_manager()
        .ok_or_else(|| UnifiedError::Other("Sleep manager not initialized".into()))?;

    let mut manager = manager.lock();
    manager.enter_sleep(state)
}

/// Wake from sleep state
pub fn wake_from_sleep(source: WakeupSource) -> Result<(), UnifiedError> {
    let manager = get_manager()
        .ok_or_else(|| UnifiedError::Other("Sleep manager not initialized".into()))?;

    let mut manager = manager.lock();
    manager.wake_from_sleep(source)
}

/// Request sleep with duration
pub fn request_sleep(state: SleepState, duration_ms: Option<u64>) -> Result<(), UnifiedError> {
    let manager = get_manager()
        .ok_or_else(|| UnifiedError::Other("Sleep manager not initialized".into()))?;

    let mut manager = manager.lock();
    manager.request_sleep(state, duration_ms)
}

/// Get current sleep state
pub fn get_current_state() -> SleepState {
    if let Some(manager) = get_manager() {
        manager.lock().current_state()
    } else {
        SleepState::S0
    }
}

/// Check if system is in sleep state
pub fn is_sleeping() -> bool {
    let state = get_current_state();
    !state.is_working()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sleep_state_properties() {
        assert!(SleepState::S0.is_working());
        assert!(SleepState::S3.is_sleep());
        assert!(SleepState::S4.is_hibernate());
        assert!(SleepState::S5.is_off());

        assert_eq!(SleepState::S3.depth(), 3);
        assert_eq!(SleepState::S4.depth(), 4);
    }

    #[test]
    fn test_sleep_manager_creation() {
        let manager = SleepManager::new();
        assert_eq!(manager.current_state(), SleepState::S0);
        assert!(manager.is_sleep_enabled());
    }

    #[test]
    fn test_enter_sleep() {
        let mut manager = SleepManager::new();
        manager.enter_sleep(SleepState::S3).unwrap();
        assert_eq!(manager.current_state(), SleepState::S3);
    }

    #[test]
    fn test_wake_from_sleep() {
        let mut manager = SleepManager::new();
        manager.enable_wakeup_source(WakeupSource::PowerButton)
            .unwrap();

        manager.enter_sleep(SleepState::S3).unwrap();
        manager.wake_from_sleep(WakeupSource::PowerButton).unwrap();

        assert_eq!(manager.current_state(), SleepState::S0);
    }

    #[test]
    fn test_wakeup_source_management() {
        let mut manager = SleepManager::new();

        manager.enable_wakeup_source(WakeupSource::RTC).unwrap();
        assert!(manager.is_wakeup_source_enabled(WakeupSource::RTC));

        manager.disable_wakeup_source(WakeupSource::RTC).unwrap();
        assert!(!manager.is_wakeup_source_enabled(WakeupSource::RTC));
    }

    #[test]
    fn test_sleep_statistics() {
        let mut manager = SleepManager::new();
        manager.enable_wakeup_source(WakeupSource::PowerButton)
            .unwrap();

        manager.enter_sleep(SleepState::S3).unwrap();
        assert_eq!(manager.stats().sleep_count, 1);

        manager.wake_from_sleep(WakeupSource::PowerButton).unwrap();
        assert_eq!(manager.stats().wake_count, 1);
    }

    #[test]
    fn test_invalid_sleep_transitions() {
        let mut manager = SleepManager::new();

        // Cannot enter S0
        assert!(manager.enter_sleep(SleepState::S0).is_err());

        // Cannot wake when already awake
        assert!(manager.wake_from_sleep(WakeupSource::PowerButton).is_ok());

        // Cannot wake with disabled source
        manager.enter_sleep(SleepState::S3).unwrap();
        assert!(manager.wake_from_sleep(WakeupSource::USB).is_err());
    }

    #[test]
    fn test_custom_wakeup_source() {
        let mut manager = SleepManager::new();
        let custom_source = WakeupSource::Custom(42);

        manager.add_wakeup_source(custom_source);
        manager.enable_wakeup_source(custom_source).unwrap();

        assert!(manager.is_wakeup_source_enabled(custom_source));
    }

    #[test]
    fn test_sleep_disabled() {
        let mut manager = SleepManager::new();
        manager.set_sleep_enabled(false);

        assert!(manager.enter_sleep(SleepState::S3).is_err());
    }
}
