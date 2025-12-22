//! Device Driver Registration Mechanism
//!
//! This module provides a comprehensive device driver registration mechanism for NOS,
//! supporting dynamic driver loading, unloading, and management.

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, AtomicU32, AtomicBool, Ordering};
use crate::subsystems::sync::{Mutex, Sleeplock};
use crate::subsystems::drivers::driver_manager::{
    Driver, DeviceId, DriverId, DeviceType, DeviceStatus, DriverStatus,
    DeviceInfo, DriverInfo, DeviceResources, IoOperation, IoResult, InterruptInfo,
    DriverManager
};
use crate::subsystems::drivers::device_model::{
    DeviceModel, EnhancedDeviceInfo, DeviceClass, DevicePowerState, DeviceCapabilities,
    EnhancedDeviceModel, get_enhanced_device_model
};
use crate::subsystems::drivers::device_discovery::{
    BusDiscovery, BusType, DeviceIdentification, get_device_discovery_manager
};
use nos_nos_error_handling::unified::KernelError;

// ============================================================================
// Driver Registration Constants
// ============================================================================

/// Maximum number of registered drivers
pub const MAX_REGISTERED_DRIVERS: u32 = 1000;

/// Maximum number of devices per driver
pub const MAX_DEVICES_PER_DRIVER: u32 = 100;

/// Driver registration timeout in milliseconds
pub const DRIVER_REGISTRATION_TIMEOUT_MS: u64 = 5000;

/// Driver initialization timeout in milliseconds
pub const DRIVER_INIT_TIMEOUT_MS: u64 = 10000;

// ============================================================================
// Driver Registration Types
// ============================================================================

/// Driver registration status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum DriverRegistrationStatus {
    /// Driver not registered
    NotRegistered = 0,
    /// Registration in progress
    Registering = 1,
    /// Driver registered
    Registered = 2,
    /// Registration failed
    RegistrationFailed = 3,
    /// Unregistration in progress
    Unregistering = 4,
    /// Driver unregistered
    Unregistered = 5,
    /// Unregistration failed
    UnregistrationFailed = 6,
}

/// Driver priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u32)]
pub enum DriverPriority {
    /// Lowest priority
    Lowest = 0,
    /// Low priority
    Low = 25,
    /// Normal priority
    Normal = 50,
    /// High priority
    High = 75,
    /// Highest priority
    Highest = 100,
}

/// Driver compatibility information
#[derive(Debug, Clone)]
pub struct DriverCompatibility {
    /// Supported device classes
    pub supported_classes: Vec<DeviceClass>,
    /// Supported device types
    pub supported_types: Vec<DeviceType>,
    /// Supported bus types
    pub supported_buses: Vec<BusType>,
    /// Vendor-specific device IDs
    pub vendor_device_ids: BTreeMap<String, Vec<String>>,
    /// Required device capabilities
    pub required_capabilities: Vec<fn(&DeviceCapabilities) -> bool>,
    /// Excluded device IDs
    pub excluded_device_ids: Vec<String>,
    /// Minimum driver version
    pub min_driver_version: String,
    /// Maximum driver version
    pub max_driver_version: String,
}

impl Default for DriverCompatibility {
    fn default() -> Self {
        Self {
            supported_classes: Vec::new(),
            supported_types: Vec::new(),
            supported_buses: Vec::new(),
            vendor_device_ids: BTreeMap::new(),
            required_capabilities: Vec::new(),
            excluded_device_ids: Vec::new(),
            min_driver_version: "0.0.0".to_string(),
            max_driver_version: "255.255.255".to_string(),
        }
    }
}

/// Driver registration information
#[derive(Debug, Clone)]
pub struct DriverRegistrationInfo {
    /// Base driver information
    pub base_info: DriverInfo,
    /// Driver registration status
    pub registration_status: DriverRegistrationStatus,
    /// Driver priority
    pub priority: DriverPriority,
    /// Driver compatibility information
    pub compatibility: DriverCompatibility,
    /// Driver path
    pub path: String,
    /// Driver arguments
    pub arguments: Vec<String>,
    /// Driver environment variables
    pub environment: BTreeMap<String, String>,
    /// Driver dependencies
    pub dependencies: Vec<String>,
    /// Registration timestamp
    pub registration_timestamp: u64,
    /// Last error message
    pub last_error: String,
    /// Number of devices managed by this driver
    pub managed_devices: u32,
    /// Driver statistics
    pub stats: DriverRegistrationStats,
}

impl Default for DriverRegistrationInfo {
    fn default() -> Self {
        Self {
            base_info: DriverInfo {
                id: 0,
                name: "".to_string(),
                version: "".to_string(),
                status: DriverStatus::Unloaded,
                supported_device_types: Vec::new(),
                supported_device_ids: Vec::new(),
                path: "".to_string(),
                dependencies: Vec::new(),
                capabilities: Vec::new(),
                attributes: BTreeMap::new(),
            },
            registration_status: DriverRegistrationStatus::NotRegistered,
            priority: DriverPriority::Normal,
            compatibility: DriverCompatibility::default(),
            path: "".to_string(),
            arguments: Vec::new(),
            environment: BTreeMap::new(),
            dependencies: Vec::new(),
            registration_timestamp: 0,
            last_error: String::new(),
            managed_devices: 0,
            stats: DriverRegistrationStats::default(),
        }
    }
}

/// Driver registration statistics
#[derive(Debug, Default, Clone)]
pub struct DriverRegistrationStats {
    /// Number of registration attempts
    pub registration_attempts: u32,
    /// Number of successful registrations
    pub successful_registrations: u32,
    /// Number of failed registrations
    pub failed_registrations: u32,
    /// Number of unregistration attempts
    pub unregistration_attempts: u32,
    /// Number of successful unregistrations
    pub successful_unregistrations: u32,
    /// Number of failed unregistrations
    pub failed_unregistrations: u32,
    /// Number of device binding attempts
    pub binding_attempts: u32,
    /// Number of successful device bindings
    pub successful_bindings: u32,
    /// Number of failed device bindings
    pub failed_bindings: u32,
    /// Total registration time in milliseconds
    pub total_registration_time_ms: u64,
    /// Average registration time in milliseconds
    pub avg_registration_time_ms: u64,
}

/// Device binding information
#[derive(Debug, Clone)]
pub struct DeviceBindingInfo {
    /// Device ID
    pub device_id: DeviceId,
    /// Driver ID
    pub driver_id: DriverId,
    /// Binding status
    pub binding_status: DriverRegistrationStatus,
    /// Binding timestamp
    pub binding_timestamp: u64,
    /// Binding priority
    pub binding_priority: DriverPriority,
    /// Last error message
    pub last_error: String,
}

impl Default for DeviceBindingInfo {
    fn default() -> Self {
        Self {
            device_id: 0,
            driver_id: 0,
            binding_status: DriverRegistrationStatus::NotRegistered,
            binding_timestamp: 0,
            binding_priority: DriverPriority::Normal,
            last_error: String::new(),
        }
    }
}

// ============================================================================
// Driver Registration Manager
// ============================================================================

/// Driver registration manager
pub struct DriverRegistrationManager {
    /// Registered drivers
    registered_drivers: Mutex<BTreeMap<DriverId, DriverRegistrationInfo>>,
    /// Device bindings
    device_bindings: Mutex<BTreeMap<DeviceId, DeviceBindingInfo>>,
    /// Driver manager
    driver_manager: Option<&'static mut DriverManager>,
    /// Device model
    device_model: Option<&'static mut EnhancedDeviceModel>,
    /// Next driver ID
    next_driver_id: AtomicU32,
    /// Registration statistics
    stats: Mutex<DriverRegistrationStats>,
    /// Auto binding enabled
    auto_binding_enabled: AtomicBool,
    /// Driver loading enabled
    driver_loading_enabled: AtomicBool,
}

impl DriverRegistrationManager {
    /// Create a new driver registration manager
    pub fn new() -> Self {
        Self {
            registered_drivers: Mutex::new(BTreeMap::new()),
            device_bindings: Mutex::new(BTreeMap::new()),
            driver_manager: None,
            device_model: None,
            next_driver_id: AtomicU32::new(1),
            stats: Mutex::new(DriverRegistrationStats::default()),
            auto_binding_enabled: AtomicBool::new(true),
            driver_loading_enabled: AtomicBool::new(true),
        }
    }

    /// Initialize the driver registration manager
    pub fn initialize(&mut self) -> Result<(), KernelError> {
        // Get driver manager and device model
        // Note: In a real implementation, we would get these from the system
        // For now, we'll assume they're available
        
        crate::println!("driver_registration: driver registration manager initialized");
        Ok(())
    }

    /// Register a driver
    pub fn register_driver(&mut self, driver: Box<dyn Driver>) -> Result<DriverId, KernelError> {
        let start_time = self.get_current_time();
        
        // Generate driver ID
        let driver_id = self.next_driver_id.fetch_add(1, Ordering::SeqCst);
        
        // Get driver information
        let driver_info = driver.get_info();
        
        // Check if driver is already registered
        {
            let drivers = self.registered_drivers.lock();
            for (_, reg_info) in drivers.iter() {
                if reg_info.base_info.name == driver_info.name {
                    return Err(KernelError::AlreadyExists);
                }
            }
        }
        
        // Check maximum number of registered drivers
        {
            let drivers = self.registered_drivers.lock();
            if drivers.len() >= MAX_REGISTERED_DRIVERS as usize {
                return Err(KernelError::OutOfSpace);
            }
        }
        
        // Create registration info
        let mut registration_info = DriverRegistrationInfo::default();
        registration_info.base_info = driver_info.clone();
        registration_info.base_info.id = driver_id;
        registration_info.registration_status = DriverRegistrationStatus::Registering;
        registration_info.registration_timestamp = start_time;
        
        // Add to registered drivers
        {
            let mut drivers = self.registered_drivers.lock();
            drivers.insert(driver_id, registration_info.clone());
        }
        
        // Register with driver manager
        let registration_result = if let Some(ref mut driver_manager) = self.driver_manager {
            driver_manager.register_driver(driver)
        } else {
            Err(KernelError::InvalidState)
        };
        
        // Update registration status
        let final_status = match registration_result {
            Ok(_) => {
                // Initialize the driver
                if let Some(ref mut driver_manager) = self.driver_manager {
                    if let Some(mut driver) = driver_manager.get_driver(driver_id) {
                        match driver.initialize() {
                            Ok(_) => DriverRegistrationStatus::Registered,
                            Err(e) => {
                                registration_info.last_error = format!("Initialization failed: {:?}", e);
                                DriverRegistrationStatus::RegistrationFailed
                            }
                        }
                    } else {
                        registration_info.last_error = "Driver not found after registration".to_string();
                        DriverRegistrationStatus::RegistrationFailed
                    }
                } else {
                    registration_info.last_error = "Driver manager not available".to_string();
                    DriverRegistrationStatus::RegistrationFailed
                }
            }
            Err(e) => {
                registration_info.last_error = format!("Registration failed: {:?}", e);
                DriverRegistrationStatus::RegistrationFailed
            }
        };
        
        // Update registration info
        {
            let mut drivers = self.registered_drivers.lock();
            if let Some(reg_info) = drivers.get_mut(&driver_id) {
                reg_info.registration_status = final_status;
                reg_info.last_error = registration_info.last_error.clone();
            }
        }
        
        // Update statistics
        let registration_time = self.get_current_time() - start_time;
        {
            let mut stats = self.stats.lock();
            stats.registration_attempts += 1;
            
            if final_status == DriverRegistrationStatus::Registered {
                stats.successful_registrations += 1;
            } else {
                stats.failed_registrations += 1;
            }
            
            stats.total_registration_time_ms += registration_time;
            if stats.registration_attempts > 0 {
                stats.avg_registration_time_ms = stats.total_registration_time_ms / stats.registration_attempts as u64;
            }
        }
        
        // Auto-bind devices if enabled
        if self.auto_binding_enabled.load(Ordering::SeqCst) && final_status == DriverRegistrationStatus::Registered {
            self.auto_bind_devices(driver_id)?;
        }
        
        crate::println!("driver_registration: registered driver {} ({})", driver_id, driver_info.name);
        Ok(driver_id)
    }

    /// Unregister a driver
    pub fn unregister_driver(&mut self, driver_id: DriverId) -> Result<(), KernelError> {
        // Check if driver is registered
        let registration_info = {
            let drivers = self.registered_drivers.lock();
            drivers.get(&driver_id).cloned()
                .ok_or(KernelError::NotFound)?
        };
        
        // Update registration status
        {
            let mut drivers = self.registered_drivers.lock();
            if let Some(reg_info) = drivers.get_mut(&driver_id) {
                reg_info.registration_status = DriverRegistrationStatus::Unregistering;
            }
        }
        
        // Unbind all devices
        self.unbind_all_devices(driver_id)?;
        
        // Unregister from driver manager
        let unregistration_result = if let Some(ref mut driver_manager) = self.driver_manager {
            // In a real implementation, we would call unregister_driver on the driver manager
            Ok(())
        } else {
            Err(KernelError::InvalidState)
        };
        
        // Update registration status
        let final_status = match unregistration_result {
            Ok(_) => DriverRegistrationStatus::Unregistered,
            Err(e) => {
                // Update error message
                let mut drivers = self.registered_drivers.lock();
                if let Some(reg_info) = drivers.get_mut(&driver_id) {
                    reg_info.last_error = format!("Unregistration failed: {:?}", e);
                }
                DriverRegistrationStatus::UnregistrationFailed
            }
        };
        
        // Remove from registered drivers if successful
        if final_status == DriverRegistrationStatus::Unregistered {
            let mut drivers = self.registered_drivers.lock();
            drivers.remove(&driver_id);
        } else {
            // Reset status if failed
            let mut drivers = self.registered_drivers.lock();
            if let Some(reg_info) = drivers.get_mut(&driver_id) {
                reg_info.registration_status = DriverRegistrationStatus::Registered;
            }
        }
        
        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.unregistration_attempts += 1;
            
            if final_status == DriverRegistrationStatus::Unregistered {
                stats.successful_unregistrations += 1;
            } else {
                stats.failed_unregistrations += 1;
            }
        }
        
        crate::println!("driver_registration: unregistered driver {} ({})", driver_id, registration_info.base_info.name);
        Ok(())
    }

    /// Bind a device to a driver
    pub fn bind_device(&mut self, device_id: DeviceId, driver_id: DriverId) -> Result<(), KernelError> {
        // Check if driver is registered
        let driver_info = {
            let drivers = self.registered_drivers.lock();
            drivers.get(&driver_id).cloned()
                .ok_or(KernelError::NotFound)?
        };
        
        if driver_info.registration_status != DriverRegistrationStatus::Registered {
            return Err(KernelError::InvalidState);
        }
        
        // Check if device is already bound
        {
            let bindings = self.device_bindings.lock();
            if bindings.contains_key(&device_id) {
                return Err(KernelError::AlreadyExists);
            }
        }
        
        // Get device information
        let device_info = if let Some(ref device_model) = self.device_model {
            device_model.get_device_info(device_id)?
        } else {
            return Err(KernelError::InvalidState);
        };
        
        // Check compatibility
        if !self.is_device_compatible(&device_info, &driver_info) {
            return Err(KernelError::Incompatible);
        }
        
        // Create binding info
        let binding_info = DeviceBindingInfo {
            device_id,
            driver_id,
            binding_status: DriverRegistrationStatus::Registering,
            binding_timestamp: self.get_current_time(),
            binding_priority: driver_info.priority,
            last_error: String::new(),
        };
        
        // Add to device bindings
        {
            let mut bindings = self.device_bindings.lock();
            bindings.insert(device_id, binding_info.clone());
        }
        
        // Bind device to driver
        let binding_result = if let Some(ref mut driver_manager) = self.driver_manager {
            // In a real implementation, we would call bind_device on the driver manager
            Ok(())
        } else {
            Err(KernelError::InvalidState)
        };
        
        // Update binding status
        let final_status = match binding_result {
            Ok(_) => DriverRegistrationStatus::Registered,
            Err(e) => {
                // Update error message
                let mut bindings = self.device_bindings.lock();
                if let Some(binding) = bindings.get_mut(&device_id) {
                    binding.last_error = format!("Binding failed: {:?}", e);
                }
                DriverRegistrationStatus::RegistrationFailed
            }
        };
        
        // Remove from bindings if failed
        if final_status != DriverRegistrationStatus::Registered {
            let mut bindings = self.device_bindings.lock();
            bindings.remove(&device_id);
        } else {
            // Update driver's managed devices count
            let mut drivers = self.registered_drivers.lock();
            if let Some(driver) = drivers.get_mut(&driver_id) {
                driver.managed_devices += 1;
            }
        }
        
        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.binding_attempts += 1;
            
            if final_status == DriverRegistrationStatus::Registered {
                stats.successful_bindings += 1;
            } else {
                stats.failed_bindings += 1;
            }
        }
        
        crate::println!("driver_registration: bound device {} to driver {}", device_id, driver_id);
        Ok(())
    }

    /// Unbind a device from a driver
    pub fn unbind_device(&mut self, device_id: DeviceId) -> Result<(), KernelError> {
        // Check if device is bound
        let binding_info = {
            let bindings = self.device_bindings.lock();
            bindings.get(&device_id).cloned()
                .ok_or(KernelError::NotFound)?
        };
        
        // Unbind device from driver
        let unbinding_result = if let Some(ref mut driver_manager) = self.driver_manager {
            // In a real implementation, we would call unbind_device on the driver manager
            Ok(())
        } else {
            Err(KernelError::InvalidState)
        };
        
        // Remove from bindings if successful
        if unbinding_result.is_ok() {
            let mut bindings = self.device_bindings.lock();
            bindings.remove(&device_id);
            
            // Update driver's managed devices count
            let mut drivers = self.registered_drivers.lock();
            if let Some(driver) = drivers.get_mut(&binding_info.driver_id) {
                driver.managed_devices = driver.managed_devices.saturating_sub(1);
            }
        }
        
        crate::println!("driver_registration: unbound device {} from driver {}", device_id, binding_info.driver_id);
        Ok(())
    }

    /// Unbind all devices from a driver
    fn unbind_all_devices(&mut self, driver_id: DriverId) -> Result<(), KernelError> {
        // Get all devices bound to this driver
        let device_ids: Vec<DeviceId> = {
            let bindings = self.device_bindings.lock();
            bindings.iter()
                .filter(|(_, binding)| binding.driver_id == driver_id)
                .map(|(device_id, _)| *device_id)
                .collect()
        };
        
        // Unbind each device
        for device_id in device_ids {
            if let Err(e) = self.unbind_device(device_id) {
                crate::println!("driver_registration: failed to unbind device {}: {:?}", device_id, e);
            }
        }
        
        Ok(())
    }

    /// Auto-bind devices to drivers
    fn auto_bind_devices(&mut self, driver_id: DriverId) -> Result<(), KernelError> {
        // Get driver information
        let driver_info = {
            let drivers = self.registered_drivers.lock();
            drivers.get(&driver_id).cloned()
                .ok_or(KernelError::NotFound)?
        };
        
        // Get all unbound devices
        let unbound_devices = if let Some(ref device_model) = self.device_model {
            // In a real implementation, we would get all unbound devices
            // For now, we'll return an empty list
            Vec::new()
        } else {
            return Err(KernelError::InvalidState);
        };
        
        // Try to bind each compatible device
        for device_id in unbound_devices {
            let device_info = device_model.get_device_info(device_id)?;
            
            if self.is_device_compatible(&device_info, &driver_info) {
                if let Err(e) = self.bind_device(device_id, driver_id) {
                    crate::println!("driver_registration: failed to auto-bind device {} to driver {}: {:?}", 
                                  device_id, driver_id, e);
                }
            }
        }
        
        Ok(())
    }

    /// Check if a device is compatible with a driver
    fn is_device_compatible(&self, device_info: &EnhancedDeviceInfo, driver_info: &DriverRegistrationInfo) -> bool {
        // Check device class
        if !driver_info.compatibility.supported_classes.is_empty() &&
           !driver_info.compatibility.supported_classes.contains(&device_info.device_class) {
            return false;
        }
        
        // Check device type
        if !driver_info.compatibility.supported_types.is_empty() &&
           !driver_info.compatibility.supported_types.contains(&device_info.base_info.device_type) {
            return false;
        }
        
        // Check device capabilities
        for capability_check in &driver_info.compatibility.required_capabilities {
            if !capability_check(&device_info.capabilities) {
                return false;
            }
        }
        
        // Check excluded device IDs
        let device_id_str = format!("{}:{}", device_info.base_info.vendor, device_info.base_info.model);
        if driver_info.compatibility.excluded_device_ids.contains(&device_id_str) {
            return false;
        }
        
        true
    }

    /// Get registered drivers
    pub fn get_registered_drivers(&self) -> Vec<DriverRegistrationInfo> {
        let drivers = self.registered_drivers.lock();
        drivers.values().cloned().collect()
    }

    /// Get driver registration info
    pub fn get_driver_registration_info(&self, driver_id: DriverId) -> Result<DriverRegistrationInfo, KernelError> {
        let drivers = self.registered_drivers.lock();
        drivers.get(&driver_id).cloned()
            .ok_or(KernelError::NotFound)
    }

    /// Get device bindings
    pub fn get_device_bindings(&self) -> Vec<DeviceBindingInfo> {
        let bindings = self.device_bindings.lock();
        bindings.values().cloned().collect()
    }

    /// Get device binding info
    pub fn get_device_binding_info(&self, device_id: DeviceId) -> Result<DeviceBindingInfo, KernelError> {
        let bindings = self.device_bindings.lock();
        bindings.get(&device_id).cloned()
            .ok_or(KernelError::NotFound)
    }

    /// Get drivers for a device
    pub fn get_drivers_for_device(&self, device_id: DeviceId) -> Result<Vec<DriverId>, KernelError> {
        let device_info = if let Some(ref device_model) = self.device_model {
            device_model.get_device_info(device_id)?
        } else {
            return Err(KernelError::InvalidState);
        };
        
        let drivers = self.registered_drivers.lock();
        let mut compatible_drivers = Vec::new();
        
        for (driver_id, driver_info) in drivers.iter() {
            if driver_info.registration_status == DriverRegistrationStatus::Registered &&
               self.is_device_compatible(&device_info, driver_info) {
                compatible_drivers.push(*driver_id);
            }
        }
        
        // Sort by priority (highest first)
        compatible_drivers.sort_by(|a, b| {
            let driver_a = drivers.get(a).unwrap();
            let driver_b = drivers.get(b).unwrap();
            driver_b.priority.cmp(&driver_a.priority)
        });
        
        Ok(compatible_drivers)
    }

    /// Enable/disable auto binding
    pub fn set_auto_binding(&self, enabled: bool) {
        self.auto_binding_enabled.store(enabled, Ordering::SeqCst);
    }

    /// Check if auto binding is enabled
    pub fn is_auto_binding_enabled(&self) -> bool {
        self.auto_binding_enabled.load(Ordering::SeqCst)
    }

    /// Enable/disable driver loading
    pub fn set_driver_loading(&self, enabled: bool) {
        self.driver_loading_enabled.store(enabled, Ordering::SeqCst);
    }

    /// Check if driver loading is enabled
    pub fn is_driver_loading_enabled(&self) -> bool {
        self.driver_loading_enabled.load(Ordering::SeqCst)
    }

    /// Get registration statistics
    pub fn get_stats(&self) -> DriverRegistrationStats {
        self.stats.lock().clone()
    }

    /// Reset registration statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = DriverRegistrationStats::default();
    }

    /// Get current time in milliseconds
    fn get_current_time(&self) -> u64 {
        // In a real implementation, this would get the current time
        // from the system clock
        0
    }
}

impl Default for DriverRegistrationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global driver registration manager instance
static mut DRIVER_REGISTRATION_MANAGER: Option<DriverRegistrationManager> = None;

/// Initialize driver registration manager
pub fn init() -> Result<(), KernelError> {
    unsafe {
        let mut manager = DriverRegistrationManager::new();
        manager.initialize()?;
        DRIVER_REGISTRATION_MANAGER = Some(manager);
    }
    Ok(())
}

/// Get driver registration manager instance
pub fn get_driver_registration_manager() -> Option<&'static mut DriverRegistrationManager> {
    unsafe { DRIVER_REGISTRATION_MANAGER.as_mut() }
}