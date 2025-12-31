//! VFIO Container Management
//!
//! A VFIO container represents an isolated IOMMU domain that provides:
//!
//! - **Address space isolation**: Each container has its own IOMMU page tables
//! - **Device grouping**: Multiple devices can share the same DMA address space
//! - **Security**: Devices in different containers cannot access each other's memory
//! - **Live migration**: Container state can be saved/restored
//!
//! # Example
//!
//! ```rust,ignore
//! let manager = ContainerManager::global();
//!
//! // Create new container
//! let container = manager.create_container()?;
//!
//! // Set IOMMU type
//! container.set_iommu(IommuType::Type1)?;
//!
//! // Add group
//! container.add_group(group_id)?;
//!
//! // Perform DMA operations
//! container.map_dma(iova, user_addr, size, flags)?;
//!
//! // Destroy when done
//! manager.destroy_container(container.id())?;
//! ```

use crate::drivers::vfio::{
    group::{VfioGroup, GroupError},
    iommu::{IommuDomain, IommuType, IommuError},
    VfioError, VfioResult, MAX_DEVICES_PER_CONTAINER, MAX_GROUPS_PER_CONTAINER,
};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use spin::{Mutex, RwLock};

/// Unique container identifier
pub type ContainerId = u64;

/// Container state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerState {
    /// Container created but not initialized
    Created,

    /// IOMMU set up, ready for devices
    Initialized,

    /// Active with devices attached
    Active,

    /// Container being destroyed
    Destroying,
}

/// VFIO Container - Isolated IOMMU domain
///
/// Each container maintains its own IOMMU domain and DMA address space.
/// Devices attached to the same container can DMA to each other's memory.
pub struct VfioContainer {
    id: ContainerId,
    state: Mutex<ContainerState>,
    iommu: Mutex<Option<IommuDomain>>,
    groups: RwLock<BTreeMap<u32, Arc<VfioGroup>>>,
    dma_mappings: Mutex<DmaMapTracker>,
    ref_count: AtomicUsize,
    stats: ContainerStats,
}

impl VfioContainer {
    /// Create a new VFIO container
    pub fn new(id: ContainerId) -> VfioResult<Self> {
        Ok(Self {
            id,
            state: Mutex::new(ContainerState::Created),
            iommu: Mutex::new(None),
            groups: RwLock::new(BTreeMap::new()),
            dma_mappings: Mutex::new(DmaMapTracker::new()),
            ref_count: AtomicUsize::new(1),
            stats: ContainerStats::new(),
        })
    }

    /// Get container ID
    pub fn id(&self) -> ContainerId {
        self.id
    }

    /// Get container state
    pub fn state(&self) -> ContainerState {
        *self.state.lock()
    }

    /// Set IOMMU type for this container
    pub fn set_iommu(&self, iommu_type: IommuType) -> VfioResult<()> {
        let mut state = self.state.lock();

        if *state != ContainerState::Created {
            return Err(VfioError::InvalidArgument);
        }

        // Create IOMMU domain
        let iommu = IommuDomain::new(iommu_type).map_err(|e| match e {
            IommuError::NotSupported => VfioError::NotSupported,
            IommuError::InvalidArgument => VfioError::InvalidArgument,
            _ => VfioError::IommuError,
        })?;

        *self.iommu.lock() = Some(iommu);
        *state = ContainerState::Initialized;

        Ok(())
    }

    /// Get IOMMU domain (if initialized)
    pub fn iommu(&self) -> VfioResult<Arc<IommuDomain>> {
        self.iommu
            .lock()
            .as_ref()
            .map(|iommu| Arc::clone(iommu))
            .ok_or(VfioError::InvalidArgument)
    }

    /// Add a VFIO group to this container
    pub fn add_group(&self, group_id: u32) -> VfioResult<()> {
        let state = *self.state.lock();
        if state == ContainerState::Destroying {
            return Err(VfioError::InvalidArgument);
        }

        let mut groups = self.groups.write();

        if groups.len() >= MAX_GROUPS_PER_CONTAINER {
            return Err(VfioError::ResourceExhausted);
        }

        if groups.contains_key(&group_id) {
            return Err(VfioError::InvalidArgument);
        }

        // Get group from group manager
        let group = super::group::GroupManager::global()
            .get_group(group_id)
            .ok_or(VfioError::GroupNotAvailable)?;

        // Set container for group
        group.set_container(self.id).map_err(|e| match e {
            GroupError::AlreadyAttached => VfioError::DeviceAttached,
            _ => VfioError::InternalError,
        })?;

        // Attach to IOMMU
        if let Some(iommu) = self.iommu.lock().as_ref() {
            group.attach_to_iommu(iommu)?;
        }

        groups.insert(group_id, group);

        self.stats.groups_added.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Remove a group from this container
    pub fn remove_group(&self, group_id: u32) -> VfioResult<()> {
        let mut groups = self.groups.write();

        let group = groups
            .remove(&group_id)
            .ok_or(VfioError::InvalidArgument)?;

        // Detach from IOMMU
        if let Ok(iommu) = self.iommu() {
            group.detach_from_iommu(&iommu)?;
        }

        group.unset_container()?;

        self.stats.groups_removed.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Get group by ID
    pub fn get_group(&self, group_id: u32) -> Option<Arc<VfioGroup>> {
        self.groups.read().get(&group_id).cloned()
    }

    /// Map DMA address
    pub fn map_dma(
        &self,
        iova: u64,
        user_addr: u64,
        size: u64,
        flags: u32,
    ) -> VfioResult<()> {
        let iommu = self.iommu()?;

        // Track mapping
        self.dma_mappings.lock().add(iova, size);

        // Map through IOMMU
        iommu.map(iova, user_addr, size, flags)?;

        self.stats.dma_mapped.fetch_add(size, Ordering::Relaxed);
        self.stats.dma_count.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Unmap DMA address
    pub fn unmap_dma(&self, iova: u64, size: u64) -> VfioResult<()> {
        let iommu = self.iommu()?;

        // Unmap through IOMMU
        iommu.unmap(iova, size)?;

        // Update tracking
        self.dma_mappings.lock().remove(iova, size);

        self.stats.dma_unmapped.fetch_add(size, Ordering::Relaxed);

        Ok(())
    }

    /// Increment reference count
    pub fn ref_count_inc(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement reference count
    pub fn ref_count_dec(&self) -> usize {
        self.ref_count.fetch_sub(1, Ordering::Relaxed).saturating_sub(1)
    }

    /// Get reference count
    pub fn get_ref_count(&self) -> usize {
        self.ref_count.load(Ordering::Relaxed)
    }

    /// Get container statistics
    pub fn get_stats(&self) -> ContainerStatsSnapshot {
        ContainerStatsSnapshot {
            groups_count: self.groups.read().len(),
            dma_mappings: self.dma_mappings.lock().count(),
            total_dma_size: self.dma_mappings.lock().total_size(),
            ref_count: self.get_ref_count(),
            groups_added: self.stats.groups_added.load(Ordering::Relaxed),
            groups_removed: self.stats.groups_removed.load(Ordering::Relaxed),
            dma_mapped: self.stats.dma_mapped.load(Ordering::Relaxed),
            dma_unmapped: self.stats.dma_unmapped.load(Ordering::Relaxed),
        }
    }

    /// Prepare for destruction
    pub fn prepare_destroy(&self) -> VfioResult<()> {
        let mut state = self.state.lock();
        *state = ContainerState::Destroying;
        Ok(())
    }

    /// Clean up resources
    pub fn cleanup(&self) -> VfioResult<()> {
        // Remove all groups
        let groups: Vec<u32> = self.groups.read().keys().copied().collect();
        for group_id in groups {
            let _ = self.remove_group(group_id);
        }

        // Clear IOMMU
        *self.iommu.lock() = None;

        Ok(())
    }
}

impl Drop for VfioContainer {
    fn drop(&mut self) {
        // Clean up resources
        let _ = self.cleanup();
    }
}

/// Container manager - Track all containers system-wide
pub struct ContainerManager {
    containers: Mutex<BTreeMap<ContainerId, Arc<VfioContainer>>>,
    next_id: AtomicU64,
    stats: Mutex<ManagerStats>,
}

impl ContainerManager {
    /// Get global container manager instance
    pub fn global() -> &'static Self {
        use core::sync::atomic::{AtomicBool, Ordering};
        use spin::Once;

        static INIT: Once = Once::new();
        static mut INSTANCE: *const ContainerManager = core::ptr::null();

        unsafe {
            INIT.call_once(|| {
                let manager = ContainerManager {
                    containers: Mutex::new(BTreeMap::new()),
                    next_id: AtomicU64::new(1),
                    stats: Mutex::new(ManagerStats::new()),
                };

                INSTANCE = alloc::boxed::Box::leak(alloc::boxed::Box::new(manager));
            });

            &*INSTANCE
        }
    }

    /// Create a new container
    pub fn create_container(&self) -> VfioResult<Arc<VfioContainer>> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);

        let container = Arc::new(VfioContainer::new(id)?);

        {
            let mut containers = self.containers.lock();
            if containers.len() >= super::MAX_CONTAINERS {
                return Err(VfioError::ResourceExhausted);
            }
            containers.insert(id, Arc::clone(&container));
        }

        self.stats.lock().containers_created.fetch_add(1, Ordering::Relaxed);

        Ok(container)
    }

    /// Destroy a container
    pub fn destroy_container(&self, id: ContainerId) -> VfioResult<()> {
        let mut containers = self.containers.lock();

        let container = containers
            .get(&id)
            .ok_or(VfioError::ContainerNotFound)?
            .clone();

        // Check if container is in use
        if container.get_ref_count() > 1 {
            return Err(VfioError::InvalidArgument);
        }

        // Prepare for destruction
        container.prepare_destroy()?;

        containers.remove(&id);

        self.stats.lock().containers_destroyed.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Get container by ID
    pub fn get_container(&self, id: ContainerId) -> Option<Arc<VfioContainer>> {
        self.containers.lock().get(&id).cloned()
    }

    /// Get all container IDs
    pub fn list_containers(&self) -> alloc::vec::Vec<ContainerId> {
        self.containers
            .lock()
            .keys()
            .copied()
            .collect()
    }

    /// Get manager statistics
    pub fn get_stats(&self) -> ManagerStatsSnapshot {
        let containers = self.containers.lock();
        let stats = self.stats.lock();

        let mut total_devices = 0;
        let mut total_dma = 0;

        for container in containers.values() {
            let snapshot = container.get_stats();
            total_devices += snapshot.groups_count; // Approximate
            total_dma += snapshot.total_dma_size;
        }

        ManagerStatsSnapshot {
            active_containers: containers.len(),
            total_devices,
            total_dma_mappings: total_dma,
            containers_created: stats.containers_created.load(Ordering::Relaxed),
            containers_destroyed: stats.containers_destroyed.load(Ordering::Relaxed),
        }
    }
}

/// DMA map tracker
struct DmaMapTracker {
    mappings: BTreeMap<u64, u64>, // iova -> size
}

impl DmaMapTracker {
    fn new() -> Self {
        Self {
            mappings: BTreeMap::new(),
        }
    }

    fn add(&mut self, iova: u64, size: u64) {
        self.mappings.insert(iova, size);
    }

    fn remove(&mut self, iova: u64, size: u64) {
        self.mappings.remove(&iova);
    }

    fn count(&self) -> usize {
        self.mappings.len()
    }

    fn total_size(&self) -> u64 {
        self.mappings.values().copied().sum()
    }
}

/// Container statistics
struct ContainerStats {
    groups_added: AtomicUsize,
    groups_removed: AtomicUsize,
    dma_mapped: AtomicU64,
    dma_unmapped: AtomicU64,
}

impl ContainerStats {
    fn new() -> Self {
        Self {
            groups_added: AtomicUsize::new(0),
            groups_removed: AtomicUsize::new(0),
            dma_mapped: AtomicU64::new(0),
            dma_unmapped: AtomicU64::new(0),
        }
    }
}

/// Container statistics snapshot
#[derive(Debug, Clone, Copy)]
pub struct ContainerStatsSnapshot {
    pub groups_count: usize,
    pub dma_mappings: usize,
    pub total_dma_size: u64,
    pub ref_count: usize,
    pub groups_added: usize,
    pub groups_removed: usize,
    pub dma_mapped: u64,
    pub dma_unmapped: u64,
}

/// Manager statistics
struct ManagerStats {
    containers_created: AtomicUsize,
    containers_destroyed: AtomicUsize,
}

impl ManagerStats {
    fn new() -> Self {
        Self {
            containers_created: AtomicUsize::new(0),
            containers_destroyed: AtomicUsize::new(0),
        }
    }
}

/// Manager statistics snapshot
#[derive(Debug, Clone, Copy)]
pub struct ManagerStatsSnapshot {
    pub active_containers: usize,
    pub total_devices: usize,
    pub total_dma_mappings: u64,
    pub containers_created: usize,
    pub containers_destroyed: usize,
}

/// Container error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerError {
    NotFound,
    AlreadyExists,
    InvalidState,
    ResourceExhausted,
    GroupNotAvailable,
    IommuError,
}

impl core::fmt::Display for ContainerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotFound => write!(f, "Container not found"),
            Self::AlreadyExists => write!(f, "Container already exists"),
            Self::InvalidState => write!(f, "Container in invalid state"),
            Self::ResourceExhausted => write!(f, "Container resources exhausted"),
            Self::GroupNotAvailable => write!(f, "Group not available"),
            Self::IommuError => write!(f, "IOMMU operation failed"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_create() {
        let container = VfioContainer::new(1).unwrap();
        assert_eq!(container.id(), 1);
        assert_eq!(container.state(), ContainerState::Created);
    }

    #[test]
    fn test_container_set_iommu() {
        let container = VfioContainer::new(1).unwrap();
        container.set_iommu(IommuType::Type1).unwrap();
        assert_eq!(container.state(), ContainerState::Initialized);
    }

    #[test]
    fn test_container_ref_count() {
        let container = VfioContainer::new(1).unwrap();
        assert_eq!(container.get_ref_count(), 1);

        container.ref_count_inc();
        assert_eq!(container.get_ref_count(), 2);

        assert_eq!(container.ref_count_dec(), 1);
        assert_eq!(container.get_ref_count(), 1);
    }

    #[test]
    fn test_manager_create_container() {
        let manager = ContainerManager::global();
        let container = manager.create_container().unwrap();

        assert!(container.id() > 0);

        // Cleanup
        manager.destroy_container(container.id()).ok();
    }

    #[test]
    fn test_manager_get_container() {
        let manager = ContainerManager::global();
        let container = manager.create_container().unwrap();

        let found = manager.get_container(container.id());
        assert!(found.is_some());
        assert_eq!(found.unwrap().id(), container.id());

        // Cleanup
        manager.destroy_container(container.id()).ok();
    }

    #[test]
    fn test_dma_tracker() {
        let mut tracker = DmaMapTracker::new();

        tracker.add(0x1000, 0x1000);
        tracker.add(0x2000, 0x2000);

        assert_eq!(tracker.count(), 2);
        assert_eq!(tracker.total_size(), 0x3000);

        tracker.remove(0x1000, 0x1000);

        assert_eq!(tracker.count(), 1);
        assert_eq!(tracker.total_size(), 0x2000);
    }
}
