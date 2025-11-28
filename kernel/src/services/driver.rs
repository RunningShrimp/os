//! Device Driver Service for hybrid architecture
//! Implements device driver functionality as an independent service

use crate::drivers::{BlockDevice, RamDisk};
use crate::services::{service_register, ServiceInfo};
use crate::sync::Mutex;

// ============================================================================
// Device Driver Service State
// ============================================================================

/// Device driver service endpoint (IPC channel)
pub const DRIVER_SERVICE_ENDPOINT: usize = 0x6000;

/// Device information
pub struct DeviceInfo {
    pub device_id: u32,
    pub device_type: u32,
    pub device_name: alloc::string::String,
    pub device_class: alloc::string::String,
    pub major: u32,
    pub minor: u32,
}

static DEVICE_LIST: Mutex<alloc::vec::Vec<DeviceInfo>> = Mutex::new(alloc::vec::Vec::new());

// ============================================================================
// Public API
// ============================================================================

/// Initialize device driver service
pub fn init() {
    // Register device driver service
    service_register(
        "device_driver",
        "Device driver service for device management and abstraction",
        DRIVER_SERVICE_ENDPOINT
    );
    
    crate::println!("services/driver: initialized");
}

/// Register a new device
pub fn driver_register_device(
    device_type: u32, 
    device_name: &str, 
    device_class: &str,
    major: u32, 
    minor: u32
) -> u32 {
    let mut device_list = DEVICE_LIST.lock();
    
    let new_id = (device_list.len() as u32) + 1;
    
    let device = DeviceInfo {
        device_id: new_id,
        device_type,
        device_name: alloc::string::String::from(device_name),
        device_class: alloc::string::String::from(device_class),
        major,
        minor,
    };
    
    device_list.push(device);
    
    new_id
}

/// Unregister a device
pub fn driver_unregister_device(device_id: u32) -> bool {
    let mut device_list = DEVICE_LIST.lock();
    
    if let Some(index) = device_list.iter().position(|d| d.device_id == device_id) {
        device_list.remove(index);
        true
    } else {
        false
    }
}

/// Get device information by ID
pub fn driver_get_device_info(device_id: u32) -> Option<DeviceInfo> {
    let device_list = DEVICE_LIST.lock();
    device_list.iter().find(|d| d.device_id == device_id).cloned()
}

/// Get all devices
pub fn driver_get_all_devices() -> alloc::vec::Vec<DeviceInfo> {
    let device_list = DEVICE_LIST.lock();
    device_list.clone()
}

/// Read from block device
pub fn driver_block_read(device: &impl BlockDevice, sector: usize, buf: &mut [u8]) {
    device.read(sector, buf);
}

/// Write to block device
pub fn driver_block_write(device: &impl BlockDevice, sector: usize, buf: &[u8]) {
    device.write(sector, buf);
}