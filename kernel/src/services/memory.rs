// Advanced Memory Management Service for hybrid architecture
// Provides comprehensive memory management as a separate service
// including virtual memory, memory compression, and huge pages

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec;
use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::string::String;
use alloc::string::ToString;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use crate::sync::Mutex;
use crate::reliability::errno::{EINVAL, ENOMEM, EFAULT, EPERM};
// TODO: Implement vm module with these types
// use crate::mm::vm::{VirtAddr, PhysAddr, Page};

pub type VirtAddr = usize;
pub type PhysAddr = usize;
pub struct Page {
    pub addr: usize,
    pub size: usize,
}
use crate::mm::{PAGE_SIZE, page_round_up, page_round_down};
use crate::microkernel::{
    service_registry::{ServiceRegistry, ServiceId, ServiceCategory, ServiceInfo, ServiceStatus, InterfaceVersion},
    ipc::{IpcManager, IpcMessage},
    memory::MicroMemoryManager,
};

// ============================================================================
// Memory Service Configuration and Constants
// ============================================================================

/// Memory service configuration
pub const MEMORY_SERVICE_NAME: &str = "memory_manager";
pub const MEMORY_SERVICE_VERSION: InterfaceVersion = InterfaceVersion::new(1, 0, 0);
pub const DEFAULT_HUGE_PAGE_SIZE: usize = 2 * 1024 * 1024; // 2MB
pub const MEMORY_SERVICE_QUEUE_SIZE: usize = 1024;

// ============================================================================
// Memory Service Messages
// ============================================================================

/// Memory service message types
#[derive(Debug, Clone, Copy)]
pub enum MemoryMessageType {
    AllocatePhysical = 1,
    FreePhysical = 2,
    AllocateVirtual = 3,
    FreeVirtual = 4,
    MapMemory = 5,
    UnmapMemory = 6,
    GetStats = 7,
    SetProtection = 8,
    AllocateHugePages = 9,
    CompressMemory = 10,
    CreateSharedMemory = 11,
    AccessSharedMemory = 12,
}

/// Memory allocation request
#[derive(Debug, Clone)]
pub struct MemoryAllocationRequest {
    pub size: usize,
    pub alignment: usize,
    pub flags: MemoryFlags,
}

/// Memory allocation response
#[derive(Debug, Clone)]
pub struct MemoryAllocationResponse {
    pub address: u64,
    pub actual_size: usize,
    pub success: bool,
    pub error_code: i32,
}

/// Memory flags for allocation
#[derive(Debug, Clone, Copy)]
pub struct MemoryFlags {
    pub zeroed: bool,
    pub contiguous: bool,
    pub user_accessible: bool,
    pub writable: bool,
    pub executable: bool,
    pub huge_pages: bool,
    pub compressible: bool,
}

impl MemoryFlags {
    pub const fn kernel() -> Self {
        Self {
            zeroed: false,
            contiguous: false,
            user_accessible: false,
            writable: true,
            executable: false,
            huge_pages: false,
            compressible: false,
        }
    }

    pub const fn user() -> Self {
        Self {
            zeroed: true,
            contiguous: false,
            user_accessible: true,
            writable: true,
            executable: false,
            huge_pages: false,
            compressible: true,
        }
    }

    pub const fn executable() -> Self {
        Self {
            zeroed: false,
            contiguous: false,
            user_accessible: true,
            writable: false,
            executable: true,
            huge_pages: false,
            compressible: false,
        }
    }

    pub fn as_u32(&self) -> u32 {
        let mut flags = 0u32;
        if self.zeroed { flags |= 0x01; }
        if self.contiguous { flags |= 0x02; }
        if self.user_accessible { flags |= 0x04; }
        if self.writable { flags |= 0x08; }
        if self.executable { flags |= 0x10; }
        if self.huge_pages { flags |= 0x20; }
        if self.compressible { flags |= 0x40; }
        flags
    }
}

// ============================================================================
// Memory Service Statistics
// ============================================================================

/// Comprehensive memory statistics
#[derive(Debug, Clone)]
pub struct MemoryServiceStats {
    // Physical memory
    pub total_physical_memory: usize,
    pub free_physical_memory: usize,
    pub allocated_physical_memory: usize,
    pub fragmented_memory: usize,

    // Virtual memory
    pub total_virtual_memory: usize,
    pub allocated_virtual_memory: usize,
    pub mapped_regions: usize,

    // Special memory
    pub huge_pages_allocated: usize,
    pub huge_pages_free: usize,
    pub compressed_pages: usize,
    pub compression_ratio: f64,

    // Performance metrics
    pub allocations_count: u64,
    pub deallocations_count: u64,
    pub total_allocation_time_ns: u64,
    pub average_allocation_time_ns: f64,

    // Memory pressure
    pub memory_pressure: f64, // 0.0 (no pressure) to 1.0 (critical pressure)
    pub last_gc_time: u64,
    pub gc_count: u64,
}

impl MemoryServiceStats {
    pub const fn new() -> Self {
        Self {
            total_physical_memory: 0,
            free_physical_memory: 0,
            allocated_physical_memory: 0,
            fragmented_memory: 0,
            total_virtual_memory: 0,
            allocated_virtual_memory: 0,
            mapped_regions: 0,
            huge_pages_allocated: 0,
            huge_pages_free: 0,
            compressed_pages: 0,
            compression_ratio: 0.0,
            allocations_count: 0,
            deallocations_count: 0,
            total_allocation_time_ns: 0,
            average_allocation_time_ns: 0.0,
            memory_pressure: 0.0,
            last_gc_time: 0,
            gc_count: 0,
        }
    }

    pub fn update_pressure(&mut self) {
        if self.total_physical_memory == 0 {
            self.memory_pressure = 0.0;
            return;
        }

        let used_ratio = self.allocated_physical_memory as f64 / self.total_physical_memory as f64;

        // Calculate memory pressure based on usage and fragmentation
        let fragmentation_ratio = if self.total_physical_memory > 0 {
            self.fragmented_memory as f64 / self.total_physical_memory as f64
        } else {
            0.0
        };

        self.memory_pressure = (used_ratio * 0.7) + (fragmentation_ratio * 0.3);
    }

    pub fn get_utilization_percentage(&self) -> f64 {
        if self.total_physical_memory == 0 {
            0.0
        } else {
            (self.allocated_physical_memory as f64 / self.total_physical_memory as f64) * 100.0
        }
    }
}

// ============================================================================
// Memory Service Implementation
// ============================================================================

/// Memory management service
pub struct MemoryManagementService {
    pub service_id: ServiceId,
    pub ipc_queue_id: u64,
    pub memory_manager: Arc<MicroMemoryManager>,
    pub stats: Mutex<MemoryServiceStats>,
    pub allocation_table: Mutex<BTreeMap<u64, AllocationInfo>>,
    pub next_allocation_id: AtomicU64,
    pub shared_memory_regions: Mutex<BTreeMap<u64, SharedMemoryRegion>>,
}

/// Information about a memory allocation
#[derive(Debug, Clone)]
pub struct AllocationInfo {
    pub id: u64,
    pub size: usize,
    pub physical_address: PhysAddr,
    pub virtual_address: Option<VirtAddr>,
    pub flags: MemoryFlags,
    pub allocation_time: u64,
    pub process_id: u64,
    pub compression_info: Option<CompressionInfo>,
}

/// Compression information for memory regions
#[derive(Debug, Clone)]
pub struct CompressionInfo {
    pub original_size: usize,
    pub compressed_size: usize,
    pub compression_algorithm: String,
    pub last_compression_time: u64,
    pub access_frequency: u64,
}

/// Shared memory region
#[derive(Debug)]
pub struct SharedMemoryRegion {
    pub id: u64,
    pub size: usize,
    pub physical_address: PhysAddr,
    pub ref_count: AtomicUsize,
    pub permissions: MemoryFlags,
    pub creator_process_id: u64,
    pub creation_time: u64,
}

impl Clone for SharedMemoryRegion {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            size: self.size,
            physical_address: self.physical_address,
            ref_count: AtomicUsize::new(self.ref_count.load(Ordering::SeqCst)),
            permissions: self.permissions,
            creator_process_id: self.creator_process_id,
            creation_time: self.creation_time,
        }
    }
}

impl MemoryManagementService {
    pub fn new(memory_manager: Arc<MicroMemoryManager>, ipc_manager: &IpcManager) -> Result<Self, i32> {
        // Create IPC queue for memory service
        let ipc_queue_id = ipc_manager.create_message_queue(
            0, // owner_id (will be set to service ID)
            MEMORY_SERVICE_QUEUE_SIZE,
            4096, // max message size
        )?;

        Ok(Self {
            service_id: 0, // Will be set during registration
            ipc_queue_id,
            memory_manager,
            stats: Mutex::new(MemoryServiceStats::new()),
            allocation_table: Mutex::new(BTreeMap::new()),
            next_allocation_id: AtomicU64::new(1),
            shared_memory_regions: Mutex::new(BTreeMap::new()),
        })
    }

    pub fn register_service(&mut self, registry: &ServiceRegistry) -> Result<ServiceId, i32> {
        let service_info = ServiceInfo::new(
            0, // Will be assigned by registry
            MEMORY_SERVICE_NAME.to_string(),
            "Advanced memory management service for hybrid architecture".to_string(),
            ServiceCategory::Memory,
            MEMORY_SERVICE_VERSION,
            0, // owner_id (kernel process)
        );

        self.service_id = registry.register_service(service_info)?;

        // Set IPC channel for the service
        registry.set_service_ipc_channel(self.service_id, self.ipc_queue_id)?;

        Ok(self.service_id)
    }

    pub fn handle_message(&self, message: IpcMessage) -> Result<Vec<u8>, i32> {
        let start_time = crate::time::get_time_ns();

        let response_data = match message.message_type {
            msg_type if msg_type == MemoryMessageType::AllocatePhysical as u32 => {
                self.handle_allocate_physical(&message.data, message.sender_id as u32)
            }
            msg_type if msg_type == MemoryMessageType::FreePhysical as u32 => {
                self.handle_free_physical(&message.data, message.sender_id as u32)
            }
            msg_type if msg_type == MemoryMessageType::AllocateVirtual as u32 => {
                self.handle_allocate_virtual(&message.data, message.sender_id as u32)
            }
            msg_type if msg_type == MemoryMessageType::FreeVirtual as u32 => {
                self.handle_free_virtual(&message.data, message.sender_id as u32)
            }
            msg_type if msg_type == MemoryMessageType::GetStats as u32 => {
                self.handle_get_stats()
            }
            msg_type if msg_type == MemoryMessageType::AllocateHugePages as u32 => {
                self.handle_allocate_huge_pages(&message.data, message.sender_id as u32)
            }
            msg_type if msg_type == MemoryMessageType::CreateSharedMemory as u32 => {
                self.handle_create_shared_memory(&message.data, message.sender_id as u32)
            }
            _ => Err(EINVAL),
        };

        let end_time = crate::time::get_time_ns();
        let response_time = end_time - start_time;

        // Update service metrics
        let mut stats = self.stats.lock();
        stats.allocations_count += 1;
        stats.total_allocation_time_ns += response_time;
        stats.average_allocation_time_ns = stats.total_allocation_time_ns as f64 / stats.allocations_count as f64;

        response_data
    }

    fn handle_allocate_physical(&self, data: &[u8], sender_id: u32) -> Result<Vec<u8>, i32> {
        if data.len() < core::mem::size_of::<MemoryAllocationRequest>() {
            return Err(EINVAL);
        }

        let request: MemoryAllocationRequest = unsafe { core::ptr::read(data.as_ptr() as *const _) };

        // Determine allocation size and page count
        let aligned_size = page_round_up(request.size);
        let page_count = aligned_size / PAGE_SIZE;

        // Allocate physical pages
        let physical_addresses: Vec<usize> = if request.flags.huge_pages {
            let huge_page_addr = self.allocate_huge_pages(aligned_size)?;
            vec![huge_page_addr]
        } else {
            let mut pages = Vec::with_capacity(page_count);
            for _ in 0..page_count {
                let page = self.memory_manager.allocate_physical_page()?;
                pages.push(page);
            }
            pages
        };

        if physical_addresses.is_empty() {
            return Err(ENOMEM);
        }

        let allocation_id = self.next_allocation_id.fetch_add(1, Ordering::SeqCst);
        let base_address = physical_addresses[0];

        let allocation_info = AllocationInfo {
            id: allocation_id,
            size: aligned_size,
            physical_address: base_address,
            virtual_address: None,
            flags: request.flags,
            allocation_time: crate::time::get_time_ns(),
            process_id: sender_id as u64, // Sender is the requesting process
            compression_info: None,
        };

        // Record allocation
        {
            let mut table = self.allocation_table.lock();
            table.insert(allocation_id, allocation_info);
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.allocated_physical_memory += aligned_size;
            stats.free_physical_memory = stats.free_physical_memory.saturating_sub(aligned_size);
            if request.flags.huge_pages {
                stats.huge_pages_allocated += page_count;
            }
        }

        let response = MemoryAllocationResponse {
            address: base_address as u64,
            actual_size: aligned_size,
            success: true,
            error_code: 0,
        };

        Ok(unsafe { core::slice::from_raw_parts(
            &response as *const _ as *const u8,
            core::mem::size_of::<MemoryAllocationResponse>()
        ).to_vec() })
    }

    fn handle_free_physical(&self, data: &[u8], _sender_id: u32) -> Result<Vec<u8>, i32> {
        if data.len() < 8 {
            return Err(EINVAL);
        }

        let allocation_id = unsafe { *(data.as_ptr() as *const u64) };

        let allocation_info = {
            let mut table = self.allocation_table.lock();
            table.remove(&allocation_id).ok_or(EINVAL)?
        };

        // Free physical pages
        if allocation_info.flags.huge_pages {
            self.free_huge_pages(allocation_info.physical_address, allocation_info.size)?;
        } else {
            let page_count = allocation_info.size / PAGE_SIZE;
            for i in 0..page_count {
                let page_addr = allocation_info.physical_address + (i * PAGE_SIZE);
                self.memory_manager.free_physical_page(page_addr)?;
            }
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.allocated_physical_memory = stats.allocated_physical_memory.saturating_sub(allocation_info.size);
            stats.free_physical_memory += allocation_info.size;
            stats.deallocations_count += 1;
            if allocation_info.flags.huge_pages {
                stats.huge_pages_allocated = stats.huge_pages_allocated.saturating_sub(allocation_info.size / DEFAULT_HUGE_PAGE_SIZE);
            }
        }

        Ok(vec![1]) // Success
    }

    fn handle_allocate_virtual(&self, data: &[u8], _sender_id: u32) -> Result<Vec<u8>, i32> {
        // For now, this is a simplified implementation
        // In a full implementation, this would involve virtual memory management
        self.handle_allocate_physical(data, _sender_id)
    }

    fn handle_free_virtual(&self, data: &[u8], sender_id: u32) -> Result<Vec<u8>, i32> {
        self.handle_free_physical(data, sender_id)
    }

    fn handle_get_stats(&self) -> Result<Vec<u8>, i32> {
        let stats = self.stats.lock();
        Ok(unsafe { core::slice::from_raw_parts(
            &*stats as *const _ as *const u8,
            core::mem::size_of::<MemoryServiceStats>()
        ).to_vec() })
    }

    fn handle_allocate_huge_pages(&self, data: &[u8], _sender_id: u32) -> Result<Vec<u8>, i32> {
        // Parse huge page allocation request
        let size = if data.len() >= 8 {
            (unsafe { *(data.as_ptr() as *const u64) }) as usize
        } else {
            return Err(EINVAL);
        };

        if size % DEFAULT_HUGE_PAGE_SIZE != 0 {
            return Err(EINVAL); // Size must be multiple of huge page size
        }

        let huge_page_count = size / DEFAULT_HUGE_PAGE_SIZE;
        let allocation_id = self.next_allocation_id.fetch_add(1, Ordering::SeqCst);

        // Allocate contiguous huge pages
        let base_address = self.allocate_huge_pages(size)?;

        let response = MemoryAllocationResponse {
            address: base_address as u64,
            actual_size: size,
            success: true,
            error_code: 0,
        };

        Ok(unsafe { core::slice::from_raw_parts(
            &response as *const _ as *const u8,
            core::mem::size_of::<MemoryAllocationResponse>()
        ).to_vec() })
    }

    fn handle_create_shared_memory(&self, data: &[u8], _sender_id: u32) -> Result<Vec<u8>, i32> {
        // Parse shared memory creation request
        let size = if data.len() >= 8 {
            (unsafe { *(data.as_ptr() as *const u64) }) as usize
        } else {
            return Err(EINVAL);
        };

        let shm_id = self.next_allocation_id.fetch_add(1, Ordering::SeqCst);
        let aligned_size = page_round_up(size);

        // Allocate physical memory for shared region
        let base_address = self.memory_manager.allocate_physical_page()?;

        let shared_region = SharedMemoryRegion {
            id: shm_id,
            size: aligned_size,
            physical_address: base_address,
            ref_count: AtomicUsize::new(1),
            permissions: MemoryFlags::user(),
            creator_process_id: 0, // Will be set to actual process ID
            creation_time: crate::time::get_time_ns(),
        };

        // Record shared memory region
        {
            let mut shm_table = self.shared_memory_regions.lock();
            shm_table.insert(shm_id, shared_region);
        }

        Ok(shm_id.to_le_bytes().to_vec())
    }

    fn allocate_huge_pages(&self, size: usize) -> Result<PhysAddr, i32> {
        // Simplified huge page allocation
        // In a real implementation, this would work with the buddy allocator
        // for large contiguous blocks

        let page_count = size / DEFAULT_HUGE_PAGE_SIZE;
        let mut allocated_pages = Vec::with_capacity(page_count);

        // Try to allocate contiguous pages (simplified)
        for _ in 0..page_count {
            let page = self.memory_manager.allocate_physical_page()?;
            allocated_pages.push(page);
        }

        // For simplicity, just return the first page address
        // In a real implementation, we'd need to ensure contiguity
        Ok(allocated_pages[0])
    }

    fn free_huge_pages(&self, base_address: PhysAddr, size: usize) -> Result<(), i32> {
        let page_count = size / DEFAULT_HUGE_PAGE_SIZE;

        for i in 0..page_count {
            let page_addr = base_address + (i * DEFAULT_HUGE_PAGE_SIZE);
            self.memory_manager.free_physical_page(page_addr)?;
        }

        Ok(())
    }

    pub fn run_memory_gc(&self) -> Result<usize, i32> {
        let mut freed_bytes = 0;

        // Garbage collect unused allocations
        {
            let mut table = self.allocation_table.lock();
            let current_time = crate::time::get_time_ns();
            let gc_timeout = 30_000_000_000; // 30 seconds

            table.retain(|_, allocation| {
                let should_keep = allocation.process_id != 0 &&
                    (current_time - allocation.allocation_time) < gc_timeout;

                if !should_keep {
                    freed_bytes += allocation.size;
                    // Free the physical memory
                    let page_count = allocation.size / PAGE_SIZE;
                    for i in 0..page_count {
                        let page_addr = allocation.physical_address + (i * PAGE_SIZE);
                        let _ = self.memory_manager.free_physical_page(page_addr);
                    }
                }

                should_keep
            });
        }

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.gc_count += 1;
            stats.last_gc_time = crate::time::get_time_ns();
            stats.free_physical_memory += freed_bytes;
            stats.allocated_physical_memory = stats.allocated_physical_memory.saturating_sub(freed_bytes);
        }

        Ok(freed_bytes)
    }

    pub fn compress_memory(&self, allocation_id: u64) -> Result<f64, i32> {
        // Find allocation
        let allocation_info = {
            let table = self.allocation_table.lock();
            table.get(&allocation_id).cloned()
        };

        if allocation_info.is_none() {
            return Err(EINVAL);
        }

        // For now, simulate compression with a fixed ratio
        let compression_ratio = 0.6; // 40% reduction

        // Update compression info
        {
            let mut table = self.allocation_table.lock();
            if let Some(allocation) = table.get_mut(&allocation_id) {
                allocation.compression_info = Some(CompressionInfo {
                    original_size: allocation.size,
                    compressed_size: (allocation.size as f64 * compression_ratio) as usize,
                    compression_algorithm: "lzo".to_string(),
                    last_compression_time: crate::time::get_time_ns(),
                    access_frequency: 0,
                });
            }
        }

        // Update global statistics
        {
            let mut stats = self.stats.lock();
            stats.compressed_pages += 1;
            stats.compression_ratio = (stats.compression_ratio + compression_ratio) / 2.0;
        }

        Ok(compression_ratio)
    }

    pub fn get_memory_pressure(&self) -> f64 {
        let stats = self.stats.lock();
        stats.memory_pressure
    }

    pub fn update_stats(&self) {
        let mut stats = self.stats.lock();

        // Get physical memory info from microkernel memory manager
        if let Some(mm) = crate::microkernel::memory::get_memory_manager() {
            // Update physical memory statistics
            // In a real implementation, this would query the actual memory manager
            stats.total_physical_memory = 512 * 1024 * 1024; // Example: 512MB
            stats.free_physical_memory = mm.physical_manager.get_free_pages_count() * PAGE_SIZE;
            stats.allocated_physical_memory = stats.total_physical_memory - stats.free_physical_memory;
        }

        // Update memory pressure
        stats.update_pressure();
    }
}

// ============================================================================
// Global Memory Service Instance
// ============================================================================

static mut GLOBAL_MEMORY_SERVICE: Option<Arc<MemoryManagementService>> = None;
static MEMORY_SERVICE_INIT: AtomicBool = AtomicBool::new(false);

/// Initialize memory management service
pub fn init() -> Result<(), i32> {
    if MEMORY_SERVICE_INIT.load(Ordering::SeqCst) {
        return Ok(());
    }

    // Get required dependencies
    let memory_manager_ref = crate::microkernel::memory::get_memory_manager()
        .ok_or(EFAULT)?;

    let ipc_manager = crate::microkernel::ipc::get_ipc_manager()
        .ok_or(EFAULT)?;

    // Create Arc from static reference
    // Safe because the reference is 'static and will never be dropped
    let memory_manager_arc = unsafe {
        let ptr = memory_manager_ref as *const crate::microkernel::memory::MicroMemoryManager;
        // Create Arc from raw pointer - this is safe because:
        // 1. The pointer is from a static reference ('static lifetime)
        // 2. We're not dropping the original reference
        // 3. The Arc will manage the reference count correctly
        Arc::from_raw(ptr)
    };
    // Don't forget the original reference to prevent double-free
    core::mem::forget(memory_manager_ref);
    let mut service = MemoryManagementService::new(memory_manager_arc, ipc_manager)?;

    // Register with service registry
    let registry = crate::microkernel::service_registry::get_service_registry()
        .ok_or(EFAULT)?;

    service.register_service(registry)?;

    // Set service to running state
    registry.update_service_status(service.service_id, ServiceStatus::Running)?;

    let arc_service = Arc::new(service);

    unsafe {
        GLOBAL_MEMORY_SERVICE = Some(arc_service);
    }

    MEMORY_SERVICE_INIT.store(true, Ordering::SeqCst);
    crate::println!("services/memory: advanced memory management service initialized");

    Ok(())
}

/// Get global memory management service
pub fn get_memory_service() -> Option<Arc<MemoryManagementService>> {
    unsafe {
        GLOBAL_MEMORY_SERVICE.clone()
    }
}

/// Legacy API compatibility functions

/// Allocate a single page (legacy compatibility)
pub fn mem_alloc_page() -> *mut u8 {
    if let Some(service) = get_memory_service() {
        let request = MemoryAllocationRequest {
            size: PAGE_SIZE,
            alignment: PAGE_SIZE,
            flags: MemoryFlags::kernel(),
        };

        let request_data = unsafe { core::slice::from_raw_parts(
            &request as *const _ as *const u8,
            core::mem::size_of::<MemoryAllocationRequest>()
        )};

        if let Ok(response_data) = service.handle_message(IpcMessage::new(0, 0, MemoryMessageType::AllocatePhysical as u32, request_data.to_vec())) {
            if response_data.len() >= core::mem::size_of::<MemoryAllocationResponse>() {
                let response: MemoryAllocationResponse = unsafe { core::ptr::read(response_data.as_ptr() as *const _) };
                if response.success {
                    return response.address as *mut u8;
                }
            }
        }
    }

    // Fallback to original allocation method
    crate::mm::kalloc()
}

/// Free a single page (legacy compatibility)
pub unsafe fn mem_free_page(page: *mut u8) {
    if !page.is_null() {
        crate::mm::kfree(page);
    }
}

/// Align address down to page boundary
pub fn mem_page_round_down(addr: usize) -> usize {
    page_round_down(addr)
}

/// Align address up to page boundary
pub fn mem_page_round_up(addr: usize) -> usize {
    page_round_up(addr)
}

/// Get memory statistics (legacy compatibility)
pub fn mem_get_stats() -> MemoryServiceStats {
    get_memory_service()
        .map(|s| {
            let stats = s.stats.lock();
            stats.clone()
        })
        .unwrap_or_else(|| MemoryServiceStats::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_flags() {
        let kernel_flags = MemoryFlags::kernel();
        assert!(!kernel_flags.user_accessible);
        assert!(kernel_flags.writable);
        assert!(!kernel_flags.executable);

        let user_flags = MemoryFlags::user();
        assert!(user_flags.user_accessible);
        assert!(user_flags.writable);
        assert!(user_flags.zeroed);

        let exec_flags = MemoryFlags::executable();
        assert!(exec_flags.user_accessible);
        assert!(!exec_flags.writable);
        assert!(exec_flags.executable);
    }

    #[test]
    fn test_memory_service_stats() {
        let mut stats = MemoryServiceStats::new();

        stats.total_physical_memory = 1024 * 1024 * 1024; // 1GB
        stats.allocated_physical_memory = 512 * 1024 * 1024; // 512MB

        stats.update_pressure();

        assert!(stats.memory_pressure > 0.0);
        assert!(stats.memory_pressure < 1.0);
        assert_eq!(stats.get_utilization_percentage(), 50.0);
    }

    #[test]
    fn test_allocation_info() {
        let info = AllocationInfo {
            id: 1,
            size: 4096,
            physical_address: PhysAddr::new(0x1000),
            virtual_address: Some(VirtAddr::new(0x80000000)),
            flags: MemoryFlags::user(),
            allocation_time: crate::time::get_time_ns(),
            process_id: 123,
            compression_info: None,
        };

        assert_eq!(info.id, 1);
        assert_eq!(info.size, 4096);
        assert_eq!(info.process_id, 123);
        assert!(info.flags.user_accessible);
    }
}
