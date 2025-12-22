//! Fallback Boot Manager
//!
//! Provides fallback boot mechanisms including:
//! - Boot device enumeration and prioritization
//! - Boot attempt tracking and retry logic
//! - Fallback device selection
//! - Recovery boot modes

use core::fmt;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;


/// Boot device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootDeviceType {
    HardDrive,
    FloppyDrive,
    CDROM,
    Network,
    USBDevice,
    NVMeDevice,
    Unknown,
}

impl fmt::Display for BootDeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootDeviceType::HardDrive => write!(f, "Hard Drive"),
            BootDeviceType::FloppyDrive => write!(f, "Floppy Drive"),
            BootDeviceType::CDROM => write!(f, "CD-ROM"),
            BootDeviceType::Network => write!(f, "Network"),
            BootDeviceType::USBDevice => write!(f, "USB Device"),
            BootDeviceType::NVMeDevice => write!(f, "NVMe Device"),
            BootDeviceType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Boot attempt result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootAttemptResult {
    Success,
    LoadFailed,
    ValidationFailed,
    KernelNotFound,
    InvalidImage,
    MediaReadError,
    NotBootable,
    Timeout,
    UserAbort,
}

impl fmt::Display for BootAttemptResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootAttemptResult::Success => write!(f, "Success"),
            BootAttemptResult::LoadFailed => write!(f, "Load Failed"),
            BootAttemptResult::ValidationFailed => write!(f, "Validation Failed"),
            BootAttemptResult::KernelNotFound => write!(f, "Kernel Not Found"),
            BootAttemptResult::InvalidImage => write!(f, "Invalid Image"),
            BootAttemptResult::MediaReadError => write!(f, "Media Read Error"),
            BootAttemptResult::NotBootable => write!(f, "Not Bootable"),
            BootAttemptResult::Timeout => write!(f, "Timeout"),
            BootAttemptResult::UserAbort => write!(f, "User Abort"),
        }
    }
}

/// Boot device entry
#[derive(Debug, Clone)]
pub struct BootDevice {
    pub device_type: BootDeviceType,
    pub name: String,
    pub device_number: u32,
    pub is_removable: bool,
    pub is_available: bool,
    pub priority: u8,
}

impl BootDevice {
    /// Create new boot device
    pub fn new(device_type: BootDeviceType, name: &str, device_number: u32) -> Self {
        BootDevice {
            device_type,
            name: String::from(name),
            device_number,
            is_removable: false,
            is_available: true,
            priority: 0,
        }
    }

    /// Set device removable flag
    pub fn set_removable(&mut self, removable: bool) {
        self.is_removable = removable;
    }

    /// Set device priority (0=highest, 255=lowest)
    pub fn set_priority(&mut self, priority: u8) {
        self.priority = priority;
    }

    /// Check if device is bootable
    pub fn is_bootable(&self) -> bool {
        self.is_available && !self.name.is_empty()
    }
}

impl fmt::Display for BootDevice {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}({}) {{ priority: {}, available: {}, removable: {} }}",
            self.device_type, self.name, self.priority, self.is_available, self.is_removable
        )
    }
}

/// Boot attempt record
#[derive(Debug, Clone)]
pub struct BootAttempt {
    pub device: BootDevice,
    pub result: BootAttemptResult,
    pub timestamp: u64,
    pub error_message: String,
    pub retry_count: u32,
}

impl BootAttempt {
    /// Create new boot attempt record
    pub fn new(device: BootDevice, result: BootAttemptResult) -> Self {
        BootAttempt {
            device,
            result,
            timestamp: 0,
            error_message: String::new(),
            retry_count: 0,
        }
    }

    /// Set error message
    pub fn set_error(&mut self, message: &str) {
        self.error_message = String::from(message);
    }

    /// Check if attempt was successful
    pub fn is_successful(&self) -> bool {
        self.result == BootAttemptResult::Success
    }

    /// Check if attempt should be retried
    pub fn should_retry(&self) -> bool {
        matches!(
            self.result,
            BootAttemptResult::LoadFailed
                | BootAttemptResult::MediaReadError
                | BootAttemptResult::Timeout
        )
    }
}

impl fmt::Display for BootAttempt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BootAttempt {{ device: {}, result: {}, retry: {}, error: {} }}",
            self.device.name, self.result, self.retry_count, self.error_message
        )
    }
}

/// Fallback boot manager
pub struct FallbackBootManager {
    boot_devices: Vec<BootDevice>,
    boot_attempts: Vec<BootAttempt>,
    current_device_index: usize,
    max_retries: u32,
    total_attempts: u32,
    successful_attempts: u32,
    failed_attempts: u32,
}

impl FallbackBootManager {
    /// Create new fallback boot manager
    pub fn new() -> Self {
        FallbackBootManager {
            boot_devices: Vec::new(),
            boot_attempts: Vec::new(),
            current_device_index: 0,
            max_retries: 3,
            total_attempts: 0,
            successful_attempts: 0,
            failed_attempts: 0,
        }
    }

    /// Register boot device
    pub fn register_device(&mut self, device: BootDevice) -> bool {
        if !device.name.is_empty() {
            self.boot_devices.push(device);
            self.sort_devices_by_priority();
            true
        } else {
            false
        }
    }

    /// Sort devices by priority
    fn sort_devices_by_priority(&mut self) {
        self.boot_devices.sort_by_key(|d| d.priority);
    }

    /// Get available devices
    pub fn get_available_devices(&self) -> Vec<&BootDevice> {
        self.boot_devices
            .iter()
            .filter(|d| d.is_available && d.is_bootable())
            .collect()
    }

    /// Get next boot device
    pub fn get_next_device(&mut self) -> Option<&BootDevice> {
        if self.current_device_index >= self.boot_devices.len() {
            return None;
        }

        while self.current_device_index < self.boot_devices.len() {
            let device = &self.boot_devices[self.current_device_index];
            if device.is_available && device.is_bootable() {
                return Some(device);
            }
            self.current_device_index += 1;
        }
        None
    }

    /// Record boot attempt
    pub fn record_attempt(
        &mut self,
        device: BootDevice,
        result: BootAttemptResult,
    ) -> bool {
        let mut attempt = BootAttempt::new(device, result);
        attempt.timestamp = self.total_attempts as u64;

        if result == BootAttemptResult::Success {
            self.successful_attempts += 1;
        } else {
            self.failed_attempts += 1;
        }

        self.total_attempts += 1;
        self.boot_attempts.push(attempt);
        true
    }

    /// Get last attempt
    pub fn get_last_attempt(&self) -> Option<&BootAttempt> {
        self.boot_attempts.last()
    }

    /// Get all attempts
    pub fn get_attempts(&self) -> Vec<&BootAttempt> {
        self.boot_attempts.iter().collect()
    }

    /// Check if should try fallback
    pub fn should_try_fallback(&self) -> bool {
        if let Some(last) = self.get_last_attempt() {
            last.retry_count < self.max_retries && last.should_retry()
        } else {
            true
        }
    }

    /// Try next boot device
    pub fn try_next_device(&mut self) -> Option<&BootDevice> {
        self.current_device_index += 1;
        self.get_next_device()
    }

    /// Set max retries per device
    pub fn set_max_retries(&mut self, max: u32) {
        self.max_retries = max;
    }

    /// Get device count
    pub fn device_count(&self) -> usize {
        self.boot_devices.len()
    }

    /// Get available device count
    pub fn available_device_count(&self) -> usize {
        self.get_available_devices().len()
    }

    /// Get attempt count
    pub fn attempt_count(&self) -> usize {
        self.boot_attempts.len()
    }

    /// Get success rate
    pub fn success_rate(&self) -> f64 {
        if self.total_attempts == 0 {
            return 0.0;
        }
        (self.successful_attempts as f64) / (self.total_attempts as f64)
    }

    /// Get statistics
    pub fn get_stats(&self) -> (u32, u32, u32) {
        (self.total_attempts, self.successful_attempts, self.failed_attempts)
    }

    /// Check if all devices exhausted
    pub fn all_devices_exhausted(&self) -> bool {
        self.current_device_index >= self.boot_devices.len()
    }

    /// Reset manager state
    pub fn reset(&mut self) {
        self.boot_attempts.clear();
        self.current_device_index = 0;
        self.total_attempts = 0;
        self.successful_attempts = 0;
        self.failed_attempts = 0;
    }

    /// Get detailed status report
    pub fn status_report(&self) -> String {
        format!(
            "FallbackBootManager {{ devices: {}, attempts: {}, success: {}, failed: {} }}",
            self.device_count(),
            self.total_attempts,
            self.successful_attempts,
            self.failed_attempts
        )
    }
}

impl fmt::Display for FallbackBootManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.status_report())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_device_type_display() {
        assert_eq!(BootDeviceType::HardDrive.to_string(), "Hard Drive");
        assert_eq!(BootDeviceType::USBDevice.to_string(), "USB Device");
    }

    #[test]
    fn test_boot_attempt_result_display() {
        assert_eq!(BootAttemptResult::Success.to_string(), "Success");
        assert_eq!(BootAttemptResult::LoadFailed.to_string(), "Load Failed");
    }

    #[test]
    fn test_boot_device_creation() {
        let device = BootDevice::new(BootDeviceType::HardDrive, "sda", 0x80);
        assert_eq!(device.device_type, BootDeviceType::HardDrive);
        assert_eq!(device.name, "sda");
        assert_eq!(device.device_number, 0x80);
    }

    #[test]
    fn test_boot_device_bootable() {
        let device = BootDevice::new(BootDeviceType::HardDrive, "sda", 0x80);
        assert!(device.is_bootable());
    }

    #[test]
    fn test_boot_device_priority() {
        let mut device = BootDevice::new(BootDeviceType::HardDrive, "sda", 0x80);
        device.set_priority(10);
        assert_eq!(device.priority, 10);
    }

    #[test]
    fn test_boot_device_removable() {
        let mut device = BootDevice::new(BootDeviceType::USBDevice, "usb1", 0x81);
        device.set_removable(true);
        assert!(device.is_removable);
    }

    #[test]
    fn test_boot_attempt_creation() {
        let device = BootDevice::new(BootDeviceType::HardDrive, "sda", 0x80);
        let attempt = BootAttempt::new(device, BootAttemptResult::Success);
        assert!(attempt.is_successful());
    }

    #[test]
    fn test_boot_attempt_error_message() {
        let device = BootDevice::new(BootDeviceType::HardDrive, "sda", 0x80);
        let mut attempt = BootAttempt::new(device, BootAttemptResult::LoadFailed);
        attempt.set_error("Kernel not found on disk");
        assert_eq!(attempt.error_message, "Kernel not found on disk");
    }

    #[test]
    fn test_boot_attempt_retry() {
        let device = BootDevice::new(BootDeviceType::HardDrive, "sda", 0x80);
        let attempt = BootAttempt::new(device, BootAttemptResult::MediaReadError);
        assert!(attempt.should_retry());
    }

    #[test]
    fn test_fallback_boot_manager_creation() {
        let manager = FallbackBootManager::new();
        assert_eq!(manager.device_count(), 0);
        assert_eq!(manager.attempt_count(), 0);
    }

    #[test]
    fn test_fallback_boot_manager_register_device() {
        let mut manager = FallbackBootManager::new();
        let device = BootDevice::new(BootDeviceType::HardDrive, "sda", 0x80);
        
        assert!(manager.register_device(device));
        assert_eq!(manager.device_count(), 1);
    }

    #[test]
    fn test_fallback_boot_manager_device_priority() {
        let mut manager = FallbackBootManager::new();
        
        let mut dev1 = BootDevice::new(BootDeviceType::HardDrive, "sda", 0x80);
        dev1.set_priority(20);
        
        let mut dev2 = BootDevice::new(BootDeviceType::USBDevice, "usb1", 0x81);
        dev2.set_priority(10);
        
        manager.register_device(dev1);
        manager.register_device(dev2);
        
        let available = manager.get_available_devices();
        assert_eq!(available[0].device_type, BootDeviceType::USBDevice); // USB has priority 10
    }

    #[test]
    fn test_fallback_boot_manager_get_next_device() {
        let mut manager = FallbackBootManager::new();
        let device = BootDevice::new(BootDeviceType::HardDrive, "sda", 0x80);
        manager.register_device(device.clone());
        
        let next = manager.get_next_device();
        assert!(next.is_some());
        assert_eq!(next.unwrap().name, "sda");
    }

    #[test]
    fn test_fallback_boot_manager_record_attempt() {
        let mut manager = FallbackBootManager::new();
        let device = BootDevice::new(BootDeviceType::HardDrive, "sda", 0x80);
        
        assert!(manager.record_attempt(device.clone(), BootAttemptResult::Success));
        assert_eq!(manager.attempt_count(), 1);
        assert_eq!(manager.successful_attempts, 1);
    }

    #[test]
    fn test_fallback_boot_manager_statistics() {
        let mut manager = FallbackBootManager::new();
        let device = BootDevice::new(BootDeviceType::HardDrive, "sda", 0x80);
        
        manager.record_attempt(device.clone(), BootAttemptResult::Success);
        manager.record_attempt(device.clone(), BootAttemptResult::LoadFailed);
        manager.record_attempt(device.clone(), BootAttemptResult::Success);
        
        let (total, success, failed) = manager.get_stats();
        assert_eq!(total, 3);
        assert_eq!(success, 2);
        assert_eq!(failed, 1);
    }

    #[test]
    fn test_fallback_boot_manager_success_rate() {
        let mut manager = FallbackBootManager::new();
        let device = BootDevice::new(BootDeviceType::HardDrive, "sda", 0x80);
        
        manager.record_attempt(device.clone(), BootAttemptResult::Success);
        manager.record_attempt(device.clone(), BootAttemptResult::Success);
        
        assert!((manager.success_rate() - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_fallback_boot_manager_reset() {
        let mut manager = FallbackBootManager::new();
        let device = BootDevice::new(BootDeviceType::HardDrive, "sda", 0x80);
        
        manager.register_device(device.clone());
        manager.record_attempt(device, BootAttemptResult::Success);
        
        assert!(manager.attempt_count() > 0);
        manager.reset();
        assert_eq!(manager.attempt_count(), 0);
        assert_eq!(manager.successful_attempts, 0);
    }

    #[test]
    fn test_fallback_boot_manager_try_next_device() {
        let mut manager = FallbackBootManager::new();
        
        let dev1 = BootDevice::new(BootDeviceType::HardDrive, "sda", 0x80);
        let dev2 = BootDevice::new(BootDeviceType::USBDevice, "usb1", 0x81);
        
        manager.register_device(dev1);
        manager.register_device(dev2);
        
        let first = manager.get_next_device();
        assert_eq!(first.unwrap().name, "sda");
        
        let second = manager.try_next_device();
        assert!(second.is_some());
    }
}
