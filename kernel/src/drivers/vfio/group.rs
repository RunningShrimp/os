//! VFIO Group Management
//!
//! A VFIO group represents a set of devices that share an IOMMU domain.
//! Devices that can DMA to each other's memory must be in the same group.
//!
//! # Groups
//!
//! - Devices that are behind the same IOMMU are in the same group
//! - All devices in a group must be assigned together
//! - Groups ensure isolation - devices in different groups cannot access each other
//!
//! # Example
//!
//! ```rust,ignore
//! let group = GroupManager::global().get_group(0).unwrap();
//!
//! // Check if viable (all devices in group are available)
//! let status = group.get_status();
//!
//! // Set container
//! group.set_container(container_id)?;
//!
//! // Get device FD
//! let device_fd = group.get_device_fd("0000:01:00.0")?;
//! ```

use crate::drivers::vfio::{
    container::ContainerId,
    device::VfioDevice,
    VfioError, VfioResult,
};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use spin::{Mutex, RwLock};

/// Group status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupStatus {
    /// Group is viable (all devices available)
    Viable,

    /// Group is not viable (some devices in use)
    NotViable,
}

/// VFIO Group - Set of devices that must be used together
pub struct VfioGroup {
    id: u32,
    container_id: Mutex<Option<ContainerId>>,
    devices: RwLock<BTreeMap<alloc::string::String, Arc<VfioDevice>>>,
    status: Mutex<GroupStatus>,
    viable: AtomicU32,
    ref_count: AtomicU32,
}

impl VfioGroup {
    /// Create new VFIO group
    pub fn new(id: u32) -> Self {
        Self {
            id,
            container_id: Mutex::new(None),
            devices: RwLock::new(BTreeMap::new()),
            status: Mutex::new(GroupStatus::Viable),
            viable: AtomicU32::new(1),
            ref_count: AtomicU32::new(0),
        }
    }

    /// Get group ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get group status
    pub fn get_status(&self) -> GroupStatus {
        *self.status.lock()
    }

    /// Set group status
    pub fn set_status(&self, status: GroupStatus) {
        *self.status.lock() = status;
        self.viable.store((status == GroupStatus::Viable) as u32, Ordering::Relaxed);
    }

    /// Check if group is viable
    pub fn is_viable(&self) -> bool {
        self.viable.load(Ordering::Relaxed) != 0
    }

    /// Set container
    pub fn set_container(&self, container_id: ContainerId) -> Result<(), GroupError> {
        let mut current = self.container_id.lock();

        if current.is_some() {
            return Err(GroupError::AlreadyAttached);
        }

        *current = Some(container_id);
        Ok(())
    }

    /// Unset container
    pub fn unset_container(&self) -> Result<(), GroupError> {
        let mut current = self.container_id.lock();

        if current.is_none() {
            return Err(GroupError::NotAttached);
        }

        *current = None;
        Ok(())
    }

    /// Get container ID
    pub fn get_container_id(&self) -> Option<ContainerId> {
        *self.container_id.lock()
    }

    /// Add device to group
    pub fn add_device(&self, name: alloc::string::String, device: Arc<VfioDevice>) -> VfioResult<()> {
        let mut devices = self.devices.write();

        if devices.contains_key(&name) {
            return Err(VfioError::InvalidArgument);
        }

        devices.insert(name, device);

        Ok(())
    }

    /// Remove device from group
    pub fn remove_device(&self, name: &str) -> VfioResult<()> {
        let mut devices = self.devices.write();

        devices
            .remove(name)
            .ok_or(VfioError::InvalidArgument)?;

        Ok(())
    }

    /// Get device by name
    pub fn get_device(&self, name: &str) -> Option<Arc<VfioDevice>> {
        self.devices.read().get(name).cloned()
    }

    /// Get all devices
    pub fn get_devices(&self) -> Vec<Arc<VfioDevice>> {
        self.devices.read().values().cloned().collect()
    }

    /// Get device count
    pub fn device_count(&self) -> usize {
        self.devices.read().len()
    }

    /// Attach to IOMMU domain
    pub fn attach_to_iommu(&self, iommu: &Arc<crate::drivers::vfio::iommu::IommuDomain>) -> VfioResult<()> {
        for device in self.get_devices() {
            let info = device.info();

            iommu.attach_device(
                info.segment,
                info.bus,
                info.device,
                info.function,
            )
            .map_err(|_| VfioError::IommuError)?;
        }

        Ok(())
    }

    /// Detach from IOMMU domain
    pub fn detach_from_iommu(&self, iommu: &Arc<crate::drivers::vfio::iommu::IommuDomain>) -> VfioResult<()> {
        for device in self.get_devices() {
            let info = device.info();

            let _ = iommu.detach_device(
                info.segment,
                info.bus,
                info.device,
                info.function,
            );
        }

        Ok(())
    }

    /// Increment reference count
    pub fn ref_count_inc(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement reference count
    pub fn ref_count_dec(&self) -> u32 {
        self.ref_count.fetch_sub(1, Ordering::Relaxed).saturating_sub(1)
    }

    /// Get reference count
    pub fn ref_count(&self) -> u32 {
        self.ref_count.load(Ordering::Relaxed)
    }

    /// Get device file descriptor
    pub fn get_device_fd(&self, name: &str) -> VfioResult<i32> {
        let device = self
            .get_device(name)
            .ok_or(VfioError::DeviceNotFound)?;

        device.ref_count_inc();

        // Return file descriptor
        // TODO: Actually allocate FD
        Ok(0)
    }
}

/// Group manager - Track all groups system-wide
pub struct GroupManager {
    groups: Mutex<BTreeMap<u32, Arc<VfioGroup>>>,
    next_id: AtomicU32,
}

impl GroupManager {
    /// Get global group manager instance
    pub fn global() -> &'static Self {
        use core::sync::atomic::{AtomicBool, Ordering};
        use spin::Once;

        static INIT: Once = Once::new();
        static mut INSTANCE: *const GroupManager = core::ptr::null();

        unsafe {
            INIT.call_once(|| {
                let manager = GroupManager {
                    groups: Mutex::new(BTreeMap::new()),
                    next_id: AtomicU32::new(0),
                };

                INSTANCE = alloc::boxed::Box::leak(alloc::boxed::Box::new(manager));
            });

            &*INSTANCE
        }
    }

    /// Create new group
    pub fn create_group(&self) -> Arc<VfioGroup> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let group = Arc::new(VfioGroup::new(id));

        self.groups.lock().insert(id, Arc::clone(&group));

        group
    }

    /// Get group by ID
    pub fn get_group(&self, id: u32) -> Option<Arc<VfioGroup>> {
        self.groups.lock().get(&id).cloned()
    }

    /// Remove group
    pub fn remove_group(&self, id: u32) -> Option<Arc<VfioGroup>> {
        self.groups.lock().remove(&id)
    }

    /// Get all group IDs
    pub fn list_groups(&self) -> Vec<u32> {
        self.groups.lock().keys().copied().collect()
    }

    /// Initialize groups from system
    ///
    /// Scans PCI bus and creates groups for devices
    pub fn init_from_system(&self) -> VfioResult<()> {
        // TODO: Scan PCI bus and create groups
        // For now, create a few test groups

        let group1 = self.create_group();

        // Add some test devices
        // let device = VfioDevice::new(...)?;
        // group1.add_device("0000:01:00.0".into(), device)?;

        Ok(())
    }
}

/// Group error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupError {
    NotAttached,
    AlreadyAttached,
    NotViable,
    DeviceNotFound,
    IommuError,
}

impl core::fmt::Display for GroupError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotAttached => write!(f, "Group not attached to container"),
            Self::AlreadyAttached => write!(f, "Group already attached to container"),
            Self::NotViable => write!(f, "Group is not viable"),
            Self::DeviceNotFound => write!(f, "Device not found in group"),
            Self::IommuError => write!(f, "IOMMU operation failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_create() {
        let group = VfioGroup::new(0);

        assert_eq!(group.id(), 0);
        assert_eq!(group.get_status(), GroupStatus::Viable);
        assert!(group.is_viable());
    }

    #[test]
    fn test_group_container() {
        let group = VfioGroup::new(0);

        // Set container
        group.set_container(123).unwrap();
        assert_eq!(group.get_container_id(), Some(123));

        // Try setting again (should fail)
        assert!(group.set_container(456).is_err());

        // Unset
        group.unset_container().unwrap();
        assert_eq!(group.get_container_id(), None);

        // Unset again (should fail)
        assert!(group.unset_container().is_err());
    }

    #[test]
    fn test_group_status() {
        let group = VfioGroup::new(0);

        group.set_status(GroupStatus::NotViable);
        assert_eq!(group.get_status(), GroupStatus::NotViable);
        assert!(!group.is_viable());

        group.set_status(GroupStatus::Viable);
        assert_eq!(group.get_status(), GroupStatus::Viable);
        assert!(group.is_viable());
    }

    #[test]
    fn test_group_ref_count() {
        let group = VfioGroup::new(0);

        assert_eq!(group.ref_count(), 0);

        group.ref_count_inc();
        assert_eq!(group.ref_count(), 1);

        group.ref_count_inc();
        assert_eq!(group.ref_count(), 2);

        assert_eq!(group.ref_count_dec(), 1);
        assert_eq!(group.ref_count(), 1);

        assert_eq!(group.ref_count_dec(), 0);
        assert_eq!(group.ref_count(), 0);
    }

    #[test]
    fn test_group_manager() {
        let manager = GroupManager::global();

        let group = manager.create_group();
        assert!(group.id() >= 0);

        let found = manager.get_group(group.id());
        assert!(found.is_some());
        assert_eq!(found.unwrap().id(), group.id());

        manager.remove_group(group.id());
        assert!(manager.get_group(group.id()).is_none());
    }

    #[test]
    fn test_group_manager_list() {
        let manager = GroupManager::global();

        let group1 = manager.create_group();
        let group2 = manager.create_group();

        let ids = manager.list_groups();
        assert!(ids.contains(&group1.id()));
        assert!(ids.contains(&group2.id()));

        // Cleanup
        manager.remove_group(group1.id());
        manager.remove_group(group2.id());
    }
}
