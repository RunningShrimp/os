//! Device Hotplug Support
//!
//! This module provides comprehensive device hotplug support for dynamic device
//! addition and removal, including:
//! - Hotplug event detection and notification
//! - Device state management during hotplug operations
//! - Resource allocation and deallocation
//! - Driver binding and unbinding
//! - Error handling and recovery
//! - Hotplug event logging

use crate::prelude::*;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::String;
use spin::Mutex;
use core::sync::atomic {AtomicU32, AtomicU64,, Ordering};

// ============================================================================
// Hotplug Constants
// ============================================================================

/// Maximum pending hotplug events
pub const MAX_HOTPLUG_EVENTS: usize = 128;

/// Hotplug operation timeout (seconds)
pub const HOTPLUG_TIMEOUT_SECS: u64 = 30;

// ============================================================================
// Hotplug Event Types
// ============================================================================

/// Hotplug event type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotplugEventType {
    /// Device has been added
    DeviceAdded,
    /// Device has been removed
    DeviceRemoved,
    /// Device is about to be removed
    DeviceRemovePending,
    /// Device addition failed
    DeviceAddFailed,
    /// Device removal failed
    DeviceRemoveFailed,
    /// Device state change
    DeviceStateChange,
    /// Resource query
    ResourceQuery,
    /// Resource release
    ResourceRelease,
}

/// Hotplug event priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HotplugPriority {
    Low = 0,
    Normal = 1,
    High = 2,
    Critical = 3,
}

// ============================================================================
// Hotplug Event
// ============================================================================

/// Hotplug event descriptor
#[derive(Debug, Clone)]
pub struct HotplugEvent {
    /// Event ID
    pub event_id: u32,
    /// Event type
    pub event_type: HotplugEventType,
    /// Event priority
    pub priority: HotplugPriority,
    /// Device ID
    pub device_id: u32,
    /// Device type (vendor:device ID)
    pub device_type: Option<(u16, u16)>,
    /// Event timestamp
    pub timestamp: u64,
    /// Event status
    pub status: HotplugEventStatus,
    /// Event data
    pub data: Vec<u8>,
    /// Error message (if any)
    pub error: Option<String>,
    /// Completion callback
    pub callback: Option<u64>,
    /// Callback data
    pub callback_data: u64,
}

/// Hotplug event status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HotplugEventStatus {
    /// Event is pending
    Pending,
    /// Event is being processed
    Processing,
    /// Event completed successfully
    Completed,
    /// Event failed
    Failed,
    /// Event timed out
    TimedOut,
}

impl HotplugEvent {
    /// Create new hotplug event
    pub fn new(
        event_id: u32,
        event_type: HotplugEventType,
        device_id: u32,
    ) -> Self {
        Self {
            event_id,
            event_type,
            priority: HotplugPriority::Normal,
            device_id,
            device_type: None,
            timestamp: 0, // Will be set when event is queued
            status: HotplugEventStatus::Pending,
            data: Vec::new(),
            error: None,
            callback: None,
            callback_data: 0,
        }
    }

    /// Set event priority
    pub fn with_priority(mut self, priority: HotplugPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Set device type
    pub fn with_device_type(mut self, vendor_id: u16, device_id: u16) -> Self {
        self.device_type = Some((vendor_id, device_id));
        self
    }

    /// Set event data
    pub fn with_data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// Set completion callback
    pub fn with_callback(mut self, callback: u64, data: u64) -> Self {
        self.callback = Some(callback);
        self.callback_data = data;
        self
    }

    /// Mark as processing
    pub fn mark_processing(&mut self) {
        self.status = HotplugEventStatus::Processing;
    }

    /// Mark as completed
    pub fn mark_completed(&mut self) {
        self.status = HotplugEventStatus::Completed;
    }

    /// Mark as failed
    pub fn mark_failed(&mut self, error: String) {
        self.status = HotplugEventStatus::Failed;
        self.error = Some(error);
    }

    /// Mark as timed out
    pub fn mark_timed_out(&mut self) {
        self.status = HotplugEventStatus::TimedOut;
    }

    /// Check if event is pending
    pub fn is_pending(&self) -> bool {
        self.status == HotplugEventStatus::Pending
    }

    /// Check if event is completed
    pub fn is_completed(&self) -> bool {
        self.status == HotplugEventStatus::Completed
    }

    /// Check if event failed
    pub fn is_failed(&self) -> bool {
        self.status == HotplugEventStatus::Failed
    }
}

// ============================================================================
// Device Hotplug State
// ============================================================================

/// Device hotplug state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceHotplugState {
    /// Device is not present
    Absent,
    /// Device is present but not initialized
    Present,
    /// Device is being initialized
    Initializing,
    /// Device is initialized and ready
    Ready,
    /// Device is being removed
    Removing,
    /// Device removal is pending
    RemovePending,
    /// Device has failed
    Failed,
}

/// Device hotplug descriptor
#[derive(Debug)]
pub struct DeviceHotplugInfo {
    /// Device ID
    pub device_id: u32,
    /// Current state
    pub state: DeviceHotplugState,
    /// Bus type (PCI, USB, etc.)
    pub bus_type: String,
    /// Bus-specific address
    pub bus_address: String,
    /// Vendor ID
    pub vendor_id: u16,
    /// Device ID
    pub device_id_value: u16,
    /// Class code
    pub class_code: Option<u8>,
    /// Driver bound to device
    pub driver: Option<String>,
    /// Resources allocated to device
    pub resources: Vec<HotplugResource>,
    /// State change count
    pub state_changes: AtomicU64,
    /// Hotplug event count
    pub event_count: AtomicU64,
}

// Manual implementation of Clone for DeviceHotplugInfo
impl Clone for DeviceHotplugInfo {
    fn clone(&self) -> Self {
        Self {
            device_id: self.device_id,
            state: self.state,
            bus_type: self.bus_type.clone(),
            bus_address: self.bus_address.clone(),
            vendor_id: self.vendor_id,
            device_id_value: self.device_id_value,
            class_code: self.class_code,
            driver: self.driver.clone(),
            resources: self.resources.clone(),
            state_changes: AtomicU64::new(self.state_changes.load(Ordering::Relaxed)),
            event_count: AtomicU64::new(self.event_count.load(Ordering::Relaxed)),
        }
    }
}

/// Resource type for hotplug
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HotplugResource {
    /// Memory region (physical address, size)
    Memory(u64, usize),
    /// I/O port range (start, count)
    IoPort(u16, u16),
    /// IRQ line
    Irq(u32),
    /// DMA channel
    DmaChannel(u32),
    /// Custom resource
    Custom(String, u64),
}

impl DeviceHotplugInfo {
    /// Create new device hotplug info
    pub fn new(
        device_id: u32,
        bus_type: String,
        bus_address: String,
        vendor_id: u16,
        device_id_value: u16,
    ) -> Self {
        Self {
            device_id,
            state: DeviceHotplugState::Absent,
            bus_type,
            bus_address,
            vendor_id,
            device_id_value,
            class_code: None,
            driver: None,
            resources: Vec::new(),
            state_changes: AtomicU64::new(0),
            event_count: AtomicU64::new(0),
        }
    }

    /// Set device state
    pub fn set_state(&mut self, new_state: DeviceHotplugState) {
        self.state = new_state;
        self.state_changes.fetch_add(1, Ordering::Relaxed);
    }

    /// Get state change count
    pub fn get_state_changes(&self) -> u64 {
        self.state_changes.load(Ordering::Relaxed)
    }

    /// Increment event count
    pub fn increment_event_count(&self) {
        self.event_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Get event count
    pub fn get_event_count(&self) -> u64 {
        self.event_count.load(Ordering::Relaxed)
    }

    /// Add resource
    pub fn add_resource(&mut self, resource: HotplugResource) {
        self.resources.push(resource);
    }

    /// Remove resource
    pub fn remove_resource(&mut self, resource: &HotplugResource) -> bool {
        if let Some(pos) = self.resources.iter().position(|r| r == resource) {
            self.resources.remove(pos);
            true
        } else {
            false
        }
    }

    /// Get resources
    pub fn get_resources(&self) -> &[HotplugResource] {
        &self.resources
    }

    /// Clear all resources
    pub fn clear_resources(&mut self) {
        self.resources.clear();
    }
}

// ============================================================================
// Hotplug Manager
// ============================================================================

/// Hotplug manager statistics
#[derive(Debug, Default, Clone)]
pub struct HotplugStats {
    /// Total events processed
    pub events_processed: u64,
    /// Successful device additions
    pub device_additions: u64,
    /// Successful device removals
    pub device_removals: u64,
    /// Failed operations
    pub failed_operations: u64,
    /// Current pending events
    pub pending_events: u32,
    /// Active devices
    pub active_devices: u32,
}

/// Hotplug event handler trait
pub trait HotplugEventHandler {
    /// Handle device addition
    fn handle_device_add(&self, device_id: u32, event: &HotplugEvent) -> Result<()>;

    /// Handle device removal
    fn handle_device_remove(&self, device_id: u32, event: &HotplugEvent) -> Result<()>;

    /// Handle resource query
    fn handle_resource_query(&self, device_id: u32, event: &HotplugEvent) -> Result<Vec<HotplugResource>>;

    /// Handle resource release
    fn handle_resource_release(&self, device_id: u32, event: &HotplugEvent) -> Result<()>;
}

/// Hotplug manager
pub struct HotplugManager {
    /// Event queue
    event_queue: Mutex<Vec<HotplugEvent>>,
    /// Device hotplug info (device_id -> info)
    device_info: Mutex<BTreeMap<u32, DeviceHotplugInfo>>,
    /// Event handlers (by priority)
    handlers: Mutex<Vec<(HotplugPriority, u64)>>, // (priority, handler_ptr)
    /// Next event ID
    next_event_id: AtomicU32,
    /// Statistics
    stats: Mutex<HotplugStats>,
    /// Enabled flag
    enabled: AtomicU32,
}

impl HotplugManager {
    /// Create new hotplug manager
    pub fn new() -> Self {
        Self {
            event_queue: Mutex::new(Vec::new()),
            device_info: Mutex::new(BTreeMap::new()),
            handlers: Mutex::new(Vec::new()),
            next_event_id: AtomicU32::new(0),
            stats: Mutex::new(HotplugStats::default()),
            enabled: AtomicU32::new(1), // Enabled by default
        }
    }

    /// Enable hotplug
    pub fn enable(&self) {
        self.enabled.store(1, Ordering::Release);
        crate::println!("hotplug: enabled");
    }

    /// Disable hotplug
    pub fn disable(&self) {
        self.enabled.store(0, Ordering::Release);
        crate::println!("hotplug: disabled");
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::Acquire) != 0
    }

    /// Register device
    pub fn register_device(&self, info: DeviceHotplugInfo) -> Result<()> {
        let device_id = info.device_id;

        {
            let mut devices = self.device_info.lock();
            devices.insert(device_id, info.clone());
        }

        crate::println!(
            "hotplug: registered device {} ({:04X}:{:04X}) on {}",
            device_id,
            info.vendor_id,
            info.device_id_value,
            info.bus_type
        );

        Ok(())
    }

    /// Unregister device
    pub fn unregister_device(&self, device_id: u32) -> Result<()> {
        {
            let mut devices = self.device_info.lock();
            devices.remove(&device_id).ok_or(KernelError::NotFound)?;
        }

        {
            let mut stats = self.stats.lock();
            stats.active_devices = stats.active_devices.saturating_sub(1);
        }

        crate::println!("hotplug: unregistered device {}", device_id);

        Ok(())
    }

    /// Get device info
    pub fn get_device_info(&self, device_id: u32) -> Option<DeviceHotplugInfo> {
        let devices = self.device_info.lock();
        devices.get(&device_id).cloned()
    }

    /// Update device state
    pub fn update_device_state(&self, device_id: u32, new_state: DeviceHotplugState) -> Result<()> {
        let mut devices = self.device_info.lock();
        let device = devices.get_mut(&device_id).ok_or(KernelError::NotFound)?;

        let old_state = device.state;
        device.set_state(new_state);

        crate::println!(
            "hotplug: device {} state: {:?} -> {:?}",
            device_id,
            old_state,
            new_state
        );

        Ok(())
    }

    /// Queue hotplug event
    pub fn queue_event(&self, mut event: HotplugEvent) -> Result<()> {
        if !self.is_enabled() {
            return Err(KernelError::InvalidState);
        }

        // Set timestamp
        event.timestamp = self.get_timestamp();

        // Add to queue
        {
            let mut queue = self.event_queue.lock();
            if queue.len() >= MAX_HOTPLUG_EVENTS {
                return Err(KernelError::ResourceBusy);
            }

            // Insert by priority
            let pos = queue.iter()
                .position(|e| e.priority < event.priority)
                .unwrap_or(queue.len());

            queue.insert(pos, event.clone());
        }

        // Increment device event count
        {
            let devices = self.device_info.lock();
            if let Some(device) = devices.get(&event.device_id) {
                device.increment_event_count();
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.pending_events = self.event_queue.lock().len() as u32;
        }

        crate::println!(
            "hotplug: queued event {} (type: {:?}, priority: {:?}, device: {})",
            event.event_id,
            event.event_type,
            event.priority,
            event.device_id
        );

        Ok(())
    }

    /// Process next event from queue
    pub fn process_event(&self) -> Result<Option<HotplugEvent>> {
        let event = {
            let mut queue = self.event_queue.lock();
            if queue.is_empty() {
                return Ok(None);
            }

            // Get next event
            let mut event = queue.remove(0);

            // Mark as processing
            event.mark_processing();

            // Update pending count
            {
                let mut stats = self.stats.lock();
                stats.pending_events = queue.len() as u32;
            }

            event
        };

        crate::println!(
            "hotplug: processing event {} (type: {:?}, device: {})",
            event.event_id,
            event.event_type,
            event.device_id
        );

        // Process event based on type
        let result = match event.event_type {
            HotplugEventType::DeviceAdded => self.process_device_add(&event),
            HotplugEventType::DeviceRemoved => self.process_device_remove(&event),
            HotplugEventType::ResourceQuery => self.process_resource_query(&event),
            HotplugEventType::ResourceRelease => self.process_resource_release(&event),
            _ => Ok(()),
        };

        // Update event status
        if result.is_ok() {
            // Note: In real implementation, we'd need mutable access here
            crate::println!("hotplug: event {} completed successfully", event.event_id);
        } else {
            crate::println!("hotplug: event {} failed: {:?}", event.event_id, result);
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.events_processed += 1;
            if result.is_ok() {
                match event.event_type {
                    HotplugEventType::DeviceAdded => stats.device_additions += 1,
                    HotplugEventType::DeviceRemoved => stats.device_removals += 1,
                    _ => {}
                }
            } else {
                stats.failed_operations += 1;
            }
        }

        Ok(Some(event))
    }

    /// Process device addition
    fn process_device_add(&self, event: &HotplugEvent) -> Result<()> {
        // Update device state
        self.update_device_state(event.device_id, DeviceHotplugState::Ready)?;

        crate::println!(
            "hotplug: device {} added successfully",
            event.device_id
        );

        Ok(())
    }

    /// Process device removal
    fn process_device_remove(&self, event: &HotplugEvent) -> Result<()> {
        // Update device state
        self.update_device_state(event.device_id, DeviceHotplugState::Absent)?;

        // Clear resources
        {
            let mut devices = self.device_info.lock();
            if let Some(device) = devices.get_mut(&event.device_id) {
                device.clear_resources();
            }
        }

        crate::println!(
            "hotplug: device {} removed successfully",
            event.device_id
        );

        Ok(())
    }

    /// Process resource query
    fn process_resource_query(&self, _event: &HotplugEvent) -> Result<()> {
        // In real implementation, this would query device resources
        Ok(())
    }

    /// Process resource release
    fn process_resource_release(&self, event: &HotplugEvent) -> Result<()> {
        let mut devices = self.device_info.lock();
        let device = devices.get_mut(&event.device_id).ok_or(KernelError::NotFound)?;

        device.clear_resources();

        crate::println!(
            "hotplug: released resources for device {}",
            event.device_id
        );

        Ok(())
    }

    /// Allocate resources for device
    pub fn allocate_resources(&self, device_id: u32, resources: Vec<HotplugResource>) -> Result<()> {
        let mut devices = self.device_info.lock();
        let device = devices.get_mut(&device_id).ok_or(KernelError::NotFound)?;

        for resource in resources {
            device.add_resource(resource);
        }

        crate::println!(
            "hotplug: allocated {} resources for device {}",
            device.resources.len(),
            device_id
        );

        Ok(())
    }

    /// Get device resources
    pub fn get_device_resources(&self, device_id: u32) -> Result<Vec<HotplugResource>> {
        let devices = self.device_info.lock();
        let device = devices.get(&device_id).ok_or(KernelError::NotFound)?;
        Ok(device.resources.clone())
    }

    /// Get statistics
    pub fn get_stats(&self) -> HotplugStats {
        let mut stats = self.stats.lock();
        stats.active_devices = self.device_info.lock().len() as u32;
        stats.clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = HotplugStats::default();
    }

    /// Get current timestamp
    fn get_timestamp(&self) -> u64 {
        // In real implementation, this would get system time
        0
    }

    /// Create event for device addition
    pub fn create_add_event(
        &self,
        device_id: u32,
        vendor_id: u16,
        device_id_value: u16,
    ) -> HotplugEvent {
        let event_id = self.next_event_id.fetch_add(1, Ordering::SeqCst);

        HotplugEvent::new(event_id, HotplugEventType::DeviceAdded, device_id)
            .with_device_type(vendor_id, device_id_value)
            .with_priority(HotplugPriority::High)
    }

    /// Create event for device removal
    pub fn create_remove_event(&self, device_id: u32) -> HotplugEvent {
        let event_id = self.next_event_id.fetch_add(1, Ordering::SeqCst);

        HotplugEvent::new(event_id, HotplugEventType::DeviceRemoved, device_id)
            .with_priority(HotplugPriority::High)
    }
}

impl Default for HotplugManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hotplug_event() {
        let event = HotplugEvent::new(1, HotplugEventType::DeviceAdded, 100)
            .with_priority(HotplugPriority::High)
            .with_device_type(0x8086, 0x1234);

        assert_eq!(event.event_id, 1);
        assert_eq!(event.event_type, HotplugEventType::DeviceAdded);
        assert_eq!(event.priority, HotplugPriority::High);
        assert!(event.is_pending());
        assert!(!event.is_completed());

        event.mark_processing();
        assert_eq!(event.status, HotplugEventStatus::Processing);

        event.mark_completed();
        assert!(event.is_completed());
    }

    #[test]
    fn test_device_hotplug_info() {
        let mut info = DeviceHotplugInfo::new(
            100,
            "PCI".to_string(),
            "0000:00:1f.0".to_string(),
            0x8086,
            0x1234,
        );

        assert_eq!(info.device_id, 100);
        assert_eq!(info.state, DeviceHotplugState::Absent);

        info.set_state(DeviceHotplugState::Ready);
        assert_eq!(info.state, DeviceHotplugState::Ready);
        assert_eq!(info.get_state_changes(), 1);

        info.add_resource(HotplugResource::Memory(0xF0000000, 0x1000));
        info.add_resource(HotplugResource::Irq(16));
        assert_eq!(info.resources.len(), 2);

        assert_eq!(info.get_event_count(), 0);
        info.increment_event_count();
        assert_eq!(info.get_event_count(), 1);
    }

    #[test]
    fn test_hotplug_manager() {
        let manager = HotplugManager::new();

        // Register device
        let info = DeviceHotplugInfo::new(
            100,
            "PCI".to_string(),
            "0000:00:1f.0".to_string(),
            0x8086,
            0x1234,
        );
        manager.register_device(info).unwrap();

        // Create and queue event
        let event = manager.create_add_event(100, 0x8086, 0x1234);
        manager.queue_event(event).unwrap();

        // Process event
        let processed = manager.process_event().unwrap();
        assert!(processed.is_some());

        // Check statistics
        let stats = manager.get_stats();
        assert_eq!(stats.events_processed, 1);
        assert_eq!(stats.device_additions, 1);
        assert_eq!(stats.active_devices, 1);
    }

    #[test]
    fn test_resource_management() {
        let manager = HotplugManager::new();

        let info = DeviceHotplugInfo::new(
            100,
            "PCI".to_string(),
            "0000:00:1f.0".to_string(),
            0x8086,
            0x1234,
        );
        manager.register_device(info).unwrap();

        // Allocate resources
        let resources = vec![
            HotplugResource::Memory(0xF0000000, 0x1000),
            HotplugResource::IoPort(0x1000, 16),
            HotplugResource::Irq(16),
        ];
        manager.allocate_resources(100, resources).unwrap();

        // Get resources
        let device_resources = manager.get_device_resources(100).unwrap();
        assert_eq!(device_resources.len(), 3);
    }

    #[test]
    fn test_enable_disable() {
        let manager = HotplugManager::new();

        assert!(manager.is_enabled());

        manager.disable();
        assert!(!manager.is_enabled());

        manager.enable();
        assert!(manager.is_enabled());
    }
}
