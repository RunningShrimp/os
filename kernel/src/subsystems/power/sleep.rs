//! System Sleep States (Suspend/Resume)
//!
//! This module provides support for system sleep states including:
//! - S3 (Suspend to RAM)
//! - S4 (Suspend to Disk)
//! - S1 (Standby)

use core::sync::atomic::{AtomicBool, AtomicU8, AtomicUsize, Ordering};

use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

use super::device_pm::{PowerManager, PowerError};

/// System sleep states (ACPI specification)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SleepState {
    /// S0: Working - Normal operation
    S0 = 0,

    /// S1: Standby - Low wake latency, CPU context maintained
    S1 = 1,

    /// S2: Deeper sleep - Longer wake latency
    S2 = 2,

    /// S3: Suspend to RAM - Memory powered, requires resume
    S3 = 3,

    /// S4: Suspend to Disk - Hibernation, memory saved to disk
    S4 = 4,

    /// S5: Soft Off - System powered off
    S5 = 5,
}

impl SleepState {
    /// Check if this is a working state
    pub fn is_working(&self) -> bool {
        matches!(self, SleepState::S0)
    }

    /// Check if this is a sleep state
    pub fn is_sleep_state(&self) -> bool {
        matches!(self, SleepState::S1 | SleepState::S2 | SleepState::S3 | SleepState::S4)
    }

    /// Check if memory is preserved in this state
    pub fn preserves_memory(&self) -> bool {
        matches!(self, SleepState::S0 | SleepState::S1 | SleepState::S2 | SleepState::S3)
    }

    /// Get the state as a string
    pub fn as_str(&self) -> &'static str {
        match self {
            SleepState::S0 => "S0",
            SleepState::S1 => "S1",
            SleepState::S2 => "S2",
            SleepState::S3 => "S3",
            SleepState::S4 => "S4",
            SleepState::S5 => "S5",
        }
    }

    /// Get wake latency estimate (in milliseconds)
    pub fn wake_latency_ms(&self) -> u64 {
        match self {
            SleepState::S0 => 0,
            SleepState::S1 => 1,
            SleepState::S2 => 50,
            SleepState::S3 => 100,
            SleepState::S4 => 5000,
            SleepState::S5 => 10000,
        }
    }

    /// Get power consumption (percentage of S0)
    pub fn power_consumption(&self) -> u8 {
        match self {
            SleepState::S0 => 100,
            SleepState::S1 => 80,
            SleepState::S2 => 50,
            SleepState::S3 => 5,
            SleepState::S4 => 0,
            SleepState::S5 => 0,
        }
    }
}

/// Result of a suspend/resume operation
#[derive(Debug)]
pub enum SuspendResult {
    /// Suspend completed successfully
    Success,

    /// Suspend was aborted
    Aborted,

    /// Suspend failed with error
    Failed(PowerError),
}

impl SuspendResult {
    /// Check if suspend was successful
    pub fn is_success(&self) -> bool {
        matches!(self, SuspendResult::Success)
    }
}

/// Sleep state manager
///
/// Manages system suspend and resume operations, coordinating
/// with device power management and CPU state.
pub struct SleepManager {
    state: Mutex<SleepState>,
    is_sleeping: AtomicBool,
    sleep_count: AtomicUsize,
    last_sleep_time: AtomicUsize,
    power_manager: Option<Arc<PowerManager>>,
    sleep_state_valid: Mutex<Vec<SleepState>>,
}

impl SleepManager {
    /// Create a new sleep manager
    ///
    /// # Arguments
    ///
    /// * `power_manager` - Optional power manager for device coordination
    pub fn new(power_manager: Option<Arc<PowerManager>>) -> Self {
        Self {
            state: Mutex::new(SleepState::S0),
            is_sleeping: AtomicBool::new(false),
            sleep_count: AtomicUsize::new(0),
            last_sleep_time: AtomicUsize::new(0),
            power_manager,
            sleep_state_valid: Mutex::new(vec![
                SleepState::S0,
                SleepState::S3,
                SleepState::S4,
                SleepState::S5,
            ]),
        }
    }

    /// Enter a sleep state
    ///
    /// # Arguments
    ///
    /// * `target_state` - Target sleep state
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub fn enter_sleep(&self, target_state: SleepState) -> SuspendResult {
        // Check if sleep is in progress
        if self.is_sleeping.load(Ordering::Acquire) {
            log::warn!("SleepManager: Sleep already in progress");
            return SuspendResult::Failed(PowerError::DeviceBusy);
        }

        // Validate sleep state
        if !self.is_state_supported(target_state) {
            log::warn!("SleepManager: Unsupported sleep state {:?}", target_state);
            return SuspendResult::Failed(PowerError::TransitionNotSupported);
        }

        log::info!("SleepManager: Entering {:?}", target_state);

        // Mark as sleeping
        self.is_sleeping.store(true, Ordering::Release);

        // Execute suspend sequence
        let result = match target_state {
            SleepState::S0 => {
                // No-op, already in working state
                SuspendResult::Success
            }
            SleepState::S1 => self.enter_s1(),
            SleepState::S2 => self.enter_s2(),
            SleepState::S3 => self.enter_s3(),
            SleepState::S4 => self.enter_s4(),
            SleepState::S5 => self.enter_s5(),
        };

        // Update state and count
        if result.is_success() {
            *self.state.lock() = target_state;
            self.sleep_count.fetch_add(1, Ordering::AcqRel);
            self.last_sleep_time.store(self.get_timestamp() as usize, Ordering::Release);
        }

        // Clear sleeping flag
        self.is_sleeping.store(false, Ordering::Release);

        result
    }

    /// Wake from sleep state
    ///
    /// # Returns
    ///
    /// Result indicating success or failure
    pub fn wake_from_sleep(&self) -> SuspendResult {
        let current = *self.state.lock();

        if current.is_working() {
            log::warn!("SleepManager: Already awake");
            return SuspendResult::Success;
        }

        log::info!("SleepManager: Waking from {:?}", current);

        // Mark as sleeping during resume
        self.is_sleeping.store(true, Ordering::Release);

        // Execute resume sequence
        let result = match current {
            SleepState::S1 => self.exit_s1(),
            SleepState::S2 => self.exit_s2(),
            SleepState::S3 => self.exit_s3(),
            SleepState::S4 => self.exit_s4(),
            _ => SuspendResult::Success,
        };

        // Update state
        if result.is_success() {
            *self.state.lock() = SleepState::S0;
        }

        // Clear sleeping flag
        self.is_sleeping.store(false, Ordering::Release);

        result
    }

    /// Enter S1 (Standby) state
    fn enter_s1(&self) -> SuspendResult {
        // S1: Low wake latency, CPU context maintained

        // 1. Freeze userspace
        if let Err(e) = self.freeze_userspaces() {
            log::error!("SleepManager: Failed to freeze userspaces: {:?}", e);
            return SuspendResult::Failed(e);
        }

        // 2. Minimal device suspend
        if let Some(ref pm) = self.power_manager {
            pm.suspend_all();
        }

        // 3. Enter standby (hardware specific)
        self.enter_cpu_standby();

        SuspendResult::Success
    }

    /// Exit S1 state
    fn exit_s1(&self) -> SuspendResult {
        // 1. Resume devices
        if let Some(ref pm) = self.power_manager {
            pm.resume_all();
        }

        // 2. Thaw userspaces
        if let Err(e) = self.thaw_userspaces() {
            log::error!("SleepManager: Failed to thaw userspaces: {:?}", e);
            return SuspendResult::Failed(e);
        }

        SuspendResult::Success
    }

    /// Enter S2 (Deeper sleep) state
    fn enter_s2(&self) -> SuspendResult {
        // Similar to S1 but deeper sleep

        if let Err(e) = self.freeze_userspaces() {
            return SuspendResult::Failed(e);
        }

        if let Some(ref pm) = self.power_manager {
            pm.suspend_all();
        }

        self.enter_cpu_deep_sleep();

        SuspendResult::Success
    }

    /// Exit S2 state
    fn exit_s2(&self) -> SuspendResult {
        if let Some(ref pm) = self.power_manager {
            pm.resume_all();
        }

        if let Err(e) = self.thaw_userspaces() {
            return SuspendResult::Failed(e);
        }

        SuspendResult::Success
    }

    /// Enter S3 (Suspend to RAM) state
    fn enter_s3(&self) -> SuspendResult {
        // S3: Memory powered, devices off

        log::info!("SleepManager: Entering S3 (Suspend to RAM)");

        // 1. Freeze userspace processes
        if let Err(e) = self.freeze_userspaces() {
            log::error!("SleepManager: Failed to freeze userspaces: {:?}", e);
            return SuspendResult::Failed(e);
        }

        // 2. Suspend all devices
        if let Some(ref pm) = self.power_manager {
            let count = pm.suspend_all();
            log::info!("SleepManager: Suspended {} devices", count);
        }

        // 3. Save CPU state
        self.save_cpu_state();

        // 4. Flush caches
        self.flush_caches();

        // 5. Enter RAM sleep
        self.enter_ram_sleep();

        log::info!("SleepManager: S3 entry complete");

        SuspendResult::Success
    }

    /// Exit S3 state
    fn exit_s3(&self) -> SuspendResult {
        log::info!("SleepManager: Exiting S3 (Resume from RAM)");

        // 1. Restore CPU state
        self.restore_cpu_state();

        // 2. Resume all devices
        if let Some(ref pm) = self.power_manager {
            let count = pm.resume_all();
            log::info!("SleepManager: Resumed {} devices", count);
        }

        // 3. Thaw userspace processes
        if let Err(e) = self.thaw_userspaces() {
            log::error!("SleepManager: Failed to thaw userspaces: {:?}", e);
            return SuspendResult::Failed(e);
        }

        log::info!("SleepManager: S3 exit complete");

        SuspendResult::Success
    }

    /// Enter S4 (Suspend to Disk) state
    fn enter_s4(&self) -> SuspendResult {
        // S4: Hibernation, memory saved to disk

        log::info!("SleepManager: Entering S4 (Suspend to Disk)");

        // 1. Freeze userspace
        if let Err(e) = self.freeze_userspaces() {
            log::error!("SleepManager: Failed to freeze userspaces: {:?}", e);
            return SuspendResult::Failed(e);
        }

        // 2. Suspend all devices
        if let Some(ref pm) = self.power_manager {
            pm.suspend_all();
        }

        // 3. Save memory to disk
        if let Err(e) = self.save_memory_to_disk() {
            log::error!("SleepManager: Failed to save memory: {:?}", e);
            return SuspendResult::Failed(e);
        }

        // 4. Power off
        self.power_off();

        log::info!("SleepManager: S4 entry complete");

        SuspendResult::Success
    }

    /// Exit S4 state
    fn exit_s4(&self) -> SuspendResult {
        log::info!("SleepManager: Exiting S4 (Resume from Disk)");

        // 1. Restore memory from disk
        if let Err(e) = self.restore_memory_from_disk() {
            log::error!("SleepManager: Failed to restore memory: {:?}", e);
            return SuspendResult::Failed(e);
        }

        // 2. Resume devices
        if let Some(ref pm) = self.power_manager {
            pm.resume_all();
        }

        // 3. Thaw userspace
        if let Err(e) = self.thaw_userspaces() {
            log::error!("SleepManager: Failed to thaw userspaces: {:?}", e);
            return SuspendResult::Failed(e);
        }

        log::info!("SleepManager: S4 exit complete");

        SuspendResult::Success
    }

    /// Enter S5 (Soft Off) state
    fn enter_s5(&self) -> SuspendResult {
        log::info!("SleepManager: Entering S5 (Soft Off)");

        // 1. Freeze everything
        let _ = self.freeze_userspaces();

        // 2. Suspend all devices
        if let Some(ref pm) = self.power_manager {
            pm.suspend_all();
        }

        // 3. Power off system
        self.power_off();

        SuspendResult::Success
    }

    /// Get current sleep state
    pub fn current_state(&self) -> SleepState {
        *self.state.lock()
    }

    /// Check if currently sleeping
    pub fn is_sleeping(&self) -> bool {
        self.is_sleeping.load(Ordering::Acquire) || !self.current_state().is_working()
    }

    /// Get sleep count
    pub fn sleep_count(&self) -> usize {
        self.sleep_count.load(Ordering::Acquire)
    }

    /// Get last sleep time
    pub fn last_sleep_time(&self) -> usize {
        self.last_sleep_time.load(Ordering::Acquire)
    }

    /// Set supported sleep states
    pub fn set_supported_states(&self, states: Vec<SleepState>) {
        *self.sleep_state_valid.lock() = states;
    }

    /// Check if a sleep state is supported
    pub fn is_state_supported(&self, state: SleepState) -> bool {
        self.sleep_state_valid.lock().contains(&state)
    }

    // Internal helper methods

    fn freeze_userspaces(&self) -> Result<(), PowerError> {
        log::info!("SleepManager: Freezing userspaces");
        // Stub: would freeze all userspace processes
        Ok(())
    }

    fn thaw_userspaces(&self) -> Result<(), PowerError> {
        log::info!("SleepManager: Thawing userspaces");
        // Stub: would thaw all userspace processes
        Ok(())
    }

    fn save_cpu_state(&self) {
        log::info!("SleepManager: Saving CPU state");
        // Stub: would save CPU registers and context
    }

    fn restore_cpu_state(&self) {
        log::info!("SleepManager: Restoring CPU state");
        // Stub: would restore CPU registers and context
    }

    fn flush_caches(&self) {
        log::info!("SleepManager: Flushing caches");
        // Stub: would flush CPU caches
    }

    fn enter_cpu_standby(&self) {
        log::info!("SleepManager: Entering CPU standby");
        // Stub: would execute CPU-specific standby instruction
    }

    fn enter_cpu_deep_sleep(&self) {
        log::info!("SleepManager: Entering CPU deep sleep");
        // Stub: would execute CPU-specific deep sleep instruction
    }

    fn enter_ram_sleep(&self) {
        log::info!("SleepManager: Entering RAM sleep");
        // Stub: would execute S3-specific sleep sequence
    }

    fn save_memory_to_disk(&self) -> Result<(), PowerError> {
        log::info!("SleepManager: Saving memory to disk");
        // Stub: would write memory contents to swap/hibernation file
        Ok(())
    }

    fn restore_memory_from_disk(&self) -> Result<(), PowerError> {
        log::info!("SleepManager: Restoring memory from disk");
        // Stub: would read memory contents from swap/hibernation file
        Ok(())
    }

    fn power_off(&self) {
        log::info!("SleepManager: Powering off");
        // Stub: would trigger system power off
    }

    fn get_timestamp(&self) -> u64 {
        // Stub: would get current timestamp
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sleep_state_properties() {
        assert!(SleepState::S0.is_working());
        assert!(SleepState::S3.is_sleep_state());
        assert!(SleepState::S3.preserves_memory());
        assert!(!SleepState::S4.preserves_memory());

        assert_eq!(SleepState::S0.wake_latency_ms(), 0);
        assert_eq!(SleepState::S3.wake_latency_ms(), 100);
        assert_eq!(SleepState::S4.wake_latency_ms(), 5000);

        assert_eq!(SleepState::S0.power_consumption(), 100);
        assert_eq!(SleepState::S3.power_consumption(), 5);
        assert_eq!(SleepState::S4.power_consumption(), 0);
    }

    #[test]
    fn test_sleep_manager() {
        let manager = SleepManager::new(None);

        assert_eq!(manager.current_state(), SleepState::S0);
        assert!(!manager.is_sleeping());
        assert_eq!(manager.sleep_count(), 0);

        // Test S3 suspend
        let result = manager.enter_sleep(SleepState::S3);
        assert!(result.is_success());
        assert_eq!(manager.current_state(), SleepState::S3);
        assert_eq!(manager.sleep_count(), 1);

        // Test wake
        let result = manager.wake_from_sleep();
        assert!(result.is_success());
        assert_eq!(manager.current_state(), SleepState::S0);
    }

    #[test]
    fn test_sleep_state_validation() {
        let manager = SleepManager::new(None);

        // Test unsupported state
        let result = manager.enter_sleep(SleepState::S2);
        assert!(!result.is_success());

        // Test supported state
        let result = manager.enter_sleep(SleepState::S3);
        assert!(result.is_success());
    }

    #[test]
    fn test_concurrent_sleep() {
        let manager = SleepManager::new(None);

        // Enter sleep
        let result1 = manager.enter_sleep(SleepState::S3);
        assert!(result1.is_success());

        // Try to sleep again while sleeping (should fail)
        let result2 = manager.enter_sleep(SleepState::S3);
        assert!(!result2.is_success());

        // Wake up
        manager.wake_from_sleep();

        // Now can sleep again
        let result3 = manager.enter_sleep(SleepState::S3);
        assert!(result3.is_success());
    }
}
