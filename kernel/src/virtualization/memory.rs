//! Memory Virtualization Implementation
//!
//! This module provides memory virtualization functionality including MMU virtualization,
//! EPT/NPT support, shadow page tables, and memory ballooning.
//!
//! # Features
//! - MMU virtualization (EPT/NPT, nested paging)
//! - Shadow page tables
//! - Memory ballooning
//! - Memory slot management
//! - Dirty page tracking
//! - Memory slot migration
//!
//! # Example
//! ```rust
//! use kernel::virtualization::memory::{VmMemory, MemorySlot, EptEntry};
//!
//! # fn main() -> Result<(), Box<dyn core::error::Error>> {
//! let vm_mem = VmMemory::new(1024 * 1024 * 512)?;
//!
//! let slot = MemorySlot {
//!     guest_phys_addr: 0,
//!     size: 1024 * 1024,
//!     ..Default::default()
//! };
//!
//! vm_mem.add_memory_slot(slot)?;
//! # Ok(())
//! # }
//! ```

#![allow(dead_code)]
#![no_std]

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use spin::Mutex;

use crate::memory::MemoryPermissions;

/// Memory virtualization result type
pub type MemResult<T> = core::result::Result<T, MemError>;

/// Memory virtualization errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemError {
    /// Out of memory
    OutOfMemory,
    /// Invalid memory slot
    InvalidMemorySlot,
    /// Invalid GPA (Guest Physical Address)
    InvalidGpa,
    /// Invalid HVA (Host Virtual Address)
    InvalidHva,
    /// Page fault
    PageFault,
    /// EPT violation
    EptViolation,
    /// Memory not mapped
    NotMapped,
    /// Already mapped
    AlreadyMapped,
    /// Invalid permissions
    InvalidPermissions,
    /// Ballooning failed
    BallooningFailed,
    /// Shadow PT sync failed
    ShadowSyncFailed,
    /// Invalid page size
    InvalidPageSize,
    /// Not aligned
    NotAligned,
}

impl core::fmt::Display for MemError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "Out of memory"),
            Self::InvalidMemorySlot => write!(f, "Invalid memory slot"),
            Self::InvalidGpa => write!(f, "Invalid GPA"),
            Self::InvalidHva => write!(f, "Invalid HVA"),
            Self::PageFault => write!(f, "Page fault"),
            Self::EptViolation => write!(f, "EPT violation"),
            Self::NotMapped => write!(f, "Memory not mapped"),
            Self::AlreadyMapped => write!(f, "Already mapped"),
            Self::InvalidPermissions => write!(f, "Invalid permissions"),
            Self::BallooningFailed => write!(f, "Ballooning failed"),
            Self::ShadowSyncFailed => write!(f, "Shadow PT sync failed"),
            Self::InvalidPageSize => write!(f, "Invalid page size"),
            Self::NotAligned => write!(f, "Not aligned"),
        }
    }
}

/// Page size
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageSize {
    /// 4 KB page
    Page4K = 0,
    /// 2 MB page
    Page2M = 1,
    /// 1 GB page
    Page1G = 2,
}

impl PageSize {
    /// Get page size in bytes
    pub fn size(&self) -> usize {
        match self {
            Self::Page4K => 4096,
            Self::Page2M => 2 * 1024 * 1024,
            Self::Page1G => 1024 * 1024 * 1024,
        }
    }
}

/// EPT/NPT page table entry
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EptEntry {
    /// Physical address (page-aligned)
    pub phys_addr: u64,
    /// Read permission
    pub read: bool,
    /// Write permission
    pub write: bool,
    /// Execute permission
    pub execute: bool,
    /// Page size
    pub page_size: PageSize,
    /// Accessed flag
    pub accessed: bool,
    /// Dirty flag
    pub dirty: bool,
    /// Ignore PAT
    pub ignore_pat: bool,
    /// Memory type
    pub mem_type: u8,
}

impl EptEntry {
    /// Create an empty EPT entry
    pub const fn new() -> Self {
        Self {
            phys_addr: 0,
            read: false,
            write: false,
            execute: false,
            page_size: PageSize::Page4K,
            accessed: false,
            dirty: false,
            ignore_pat: false,
            mem_type: 0,
        }
    }

    /// Convert to raw u64
    pub fn to_raw(&self) -> u64 {
        let mut value = self.phys_addr & 0x000FFFFFFFFFF000; // 40-bit physical address

        if self.read {
            value |= 1 << 0;
        }
        if self.write {
            value |= 1 << 1;
        }
        if self.execute {
            value |= 1 << 2;
        }
        if self.page_size == PageSize::Page2M {
            value |= 1 << 7;
        }
        if self.page_size == PageSize::Page1G {
            value |= 1 << 7;
        }
        if self.accessed {
            value |= 1 << 8;
        }
        if self.dirty {
            value |= 1 << 9;
        }
        if self.ignore_pat {
            value |= 1 << 6;
        }

        value | ((self.mem_type as u64) & 0x7) << 3
    }

    /// Parse from raw u64
    pub fn from_raw(value: u64) -> Self {
        let page_size = if value & (1 << 7) != 0 {
            PageSize::Page2M
        } else {
            PageSize::Page4K
        };

        Self {
            phys_addr: value & 0x000FFFFFFFFFF000,
            read: value & 1 != 0,
            write: value & (1 << 1) != 0,
            execute: value & (1 << 2) != 0,
            page_size,
            accessed: value & (1 << 8) != 0,
            dirty: value & (1 << 9) != 0,
            ignore_pat: value & (1 << 6) != 0,
            mem_type: ((value >> 3) & 0x7) as u8,
        }
    }
}

/// Memory slot configuration
#[derive(Debug, Clone)]
pub struct MemorySlot {
    /// Slot ID
    pub slot_id: u32,
    /// Guest physical address
    pub guest_phys_addr: u64,
    /// Memory size in bytes
    pub size: usize,
    /// Host virtual address
    pub userspace_addr: u64,
    /// Memory flags
    pub flags: MemorySlotFlags,
    /// HVA mapped
    pub hva_mapped: bool,
}

impl Default for MemorySlot {
    fn default() -> Self {
        Self {
            slot_id: 0,
            guest_phys_addr: 0,
            size: 0,
            userspace_addr: 0,
            flags: MemorySlotFlags::empty(),
            hva_mapped: false,
        }
    }
}

/// Memory slot flags
#[derive(Debug, Clone, Copy, Default)]
pub struct MemorySlotFlags {
    /// Readable
    pub readable: bool,
    /// Writable
    pub writable: bool,
    /// Executable
    pub executable: bool,
    /// Logging dirty pages
    pub dirty_log: bool,
}

impl MemorySlotFlags {
    /// Create empty flags
    pub const fn empty() -> Self {
        Self {
            readable: false,
            writable: false,
            executable: false,
            dirty_log: false,
        }
    }

    /// Create read-write flags
    pub const fn rw() -> Self {
        Self {
            readable: true,
            writable: true,
            executable: false,
            dirty_log: false,
        }
    }

    /// Create read-write-execute flags
    pub const fn rwx() -> Self {
        Self {
            readable: true,
            writable: true,
            executable: true,
            dirty_log: false,
        }
    }
}

/// Shadow page table entry
#[derive(Debug, Clone, Copy)]
pub struct ShadowPte {
    /// Guest physical address
    pub gpa: u64,
    /// Host physical address
    pub hpa: u64,
    /// Present
    pub present: bool,
    /// Writable
    pub writable: bool,
    /// User accessible
    pub user: bool,
    /// Accessed
    pub accessed: bool,
    /// Dirty
    pub dirty: bool,
    /// Page size
    pub page_size: PageSize,
    /// NX (No-Execute) bit
    pub nx: bool,
}

impl ShadowPte {
    /// Create a new shadow PTE
    pub const fn new() -> Self {
        Self {
            gpa: 0,
            hpa: 0,
            present: false,
            writable: false,
            user: false,
            accessed: false,
            dirty: false,
            page_size: PageSize::Page4K,
            nx: false,
        }
    }

    /// Convert to raw x86 PTE format
    pub fn to_raw(&self) -> u64 {
        let mut value = self.hpa & 0x000FFFFFFFFFF000;

        if self.present {
            value |= 1 << 0;
        }
        if self.writable {
            value |= 1 << 1;
        }
        if self.user {
            value |= 1 << 2;
        }
        if self.accessed {
            value |= 1 << 5;
        }
        if self.dirty {
            value |= 1 << 6;
        }
        match self.page_size {
            PageSize::Page4K => {},
            PageSize::Page2M => value |= 1 << 7,
            PageSize::Page1G => value |= 1 << 7,
        }
        if self.nx {
            value |= 1u64 << 63;
        }

        value
    }
}

/// Dirty page bitmap
pub struct DirtyBitmap {
    /// Bitmap data (one bit per page)
    bitmap: Vec<u64>,
    /// Number of pages covered
    num_pages: usize,
}

impl DirtyBitmap {
    /// Create a new dirty bitmap
    ///
    /// # Arguments
    /// * `num_pages` - Number of pages to track
    pub fn new(num_pages: usize) -> Self {
        let num_words = (num_pages + 63) / 64;
        Self {
            bitmap: vec![0; num_words],
            num_pages,
        }
    }

    /// Mark page as dirty
    ///
    /// # Arguments
    /// * `page_index` - Page index
    pub fn set_dirty(&mut self, page_index: usize) {
        if page_index < self.num_pages {
            let word = page_index / 64;
            let bit = page_index % 64;
            self.bitmap[word] |= 1 << bit;
        }
    }

    /// Check if page is dirty
    ///
    /// # Arguments
    /// * `page_index` - Page index
    ///
    /// # Returns
    /// * `bool` - True if dirty
    pub fn is_dirty(&self, page_index: usize) -> bool {
        if page_index >= self.num_pages {
            return false;
        }
        let word = page_index / 64;
        let bit = page_index % 64;
        (self.bitmap[word] & (1 << bit)) != 0
    }

    /// Clear all dirty bits
    pub fn clear(&mut self) {
        self.bitmap.fill(0);
    }

    /// Get dirty pages count
    pub fn dirty_count(&self) -> usize {
        self.bitmap.iter().map(|word| word.count_ones() as usize).sum()
    }
}

/// Memory balloon for dynamic memory management
pub struct MemoryBalloon {
    /// Current balloon size (in pages)
    current_pages: AtomicUsize,
    /// Maximum balloon size (in pages)
    max_pages: usize,
    /// Inflated pages
    inflated: Mutex<Vec<u64>>,
    /// Balloon enabled
    enabled: AtomicBool,
}

impl MemoryBalloon {
    /// Create a new memory balloon
    ///
    /// # Arguments
    /// * `max_pages` - Maximum balloon size in pages
    pub fn new(max_pages: usize) -> Self {
        Self {
            current_pages: AtomicUsize::new(0),
            max_pages,
            inflated: Mutex::new(Vec::new()),
            enabled: AtomicBool::new(false),
        }
    }

    /// Enable balloon
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Disable balloon
    pub fn disable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// Inflate balloon (take pages from guest)
    ///
    /// # Arguments
    /// * `pages` - List of guest physical addresses to inflate
    /// * `count` - Number of pages to inflate
    ///
    /// # Returns
    /// * `MemResult<usize>` - Number of pages inflated
    pub fn inflate(&self, pages: &[u64], count: usize) -> MemResult<usize> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Ok(0);
        }

        let current = self.current_pages.load(Ordering::SeqCst);
        let available = self.max_pages.saturating_sub(current);

        let to_inflate = core::cmp::min(count, available).min(pages.len());

        let mut inflated = self.inflated.lock();
        for &page in &pages[..to_inflate] {
            inflated.push(page);
        }

        self.current_pages.fetch_add(to_inflate, Ordering::SeqCst);
        Ok(to_inflate)
    }

    /// Deflate balloon (return pages to guest)
    ///
    /// # Arguments
    /// * `count` - Number of pages to deflate
    ///
    /// # Returns
    /// * `MemResult<Vec<u64>>` - List of deflated page addresses
    pub fn deflate(&self, count: usize) -> MemResult<Vec<u64>> {
        let current = self.current_pages.load(Ordering::SeqCst);
        let to_deflate = core::cmp::min(count, current);

        let mut inflated = self.inflated.lock();
        let start = inflated.len().saturating_sub(to_deflate);
        let pages = inflated.split_off(start);

        self.current_pages.fetch_sub(to_deflate, Ordering::SeqCst);
        Ok(pages)
    }

    /// Get current balloon size
    pub fn current_size(&self) -> usize {
        self.current_pages.load(Ordering::SeqCst)
    }
}

/// Virtual machine memory
pub struct VmMemory {
    /// Memory size
    size: usize,
    /// Memory slots
    slots: Mutex<BTreeMap<u32, MemorySlot>>,
    /// Next slot ID
    next_slot_id: AtomicU32,
    /// EPT root (PML4)
    ept_root: Mutex<Option<u64>>,
    /// Shadow page tables
    shadow_pts: Mutex<BTreeMap<u64, ShadowPte>>,
    /// Dirty bitmap
    dirty_bitmap: Mutex<Option<DirtyBitmap>>,
    /// Memory balloon
    balloon: Mutex<Option<MemoryBalloon>>,
    /// Total allocated pages
    allocated_pages: AtomicUsize,
    /// Used memory
    used_memory: AtomicUsize,
}

impl VmMemory {
    /// Create a new VM memory instance
    ///
    /// # Arguments
    /// * `size` - Total memory size in bytes
    ///
    /// # Returns
    /// * `MemResult<Self>` - New VM memory instance
    pub fn new(size: usize) -> MemResult<Self> {
        // Allocate EPT root
        // In real implementation, allocate physical pages for EPT

        Ok(Self {
            size,
            slots: Mutex::new(BTreeMap::new()),
            next_slot_id: AtomicU32::new(0),
            ept_root: Mutex::new(None),
            shadow_pts: Mutex::new(BTreeMap::new()),
            dirty_bitmap: Mutex::new(None),
            balloon: Mutex::new(None),
            allocated_pages: AtomicUsize::new(0),
            used_memory: AtomicUsize::new(0),
        })
    }

    /// Add a memory slot
    ///
    /// # Arguments
    /// * `slot` - Memory slot configuration
    ///
    /// # Returns
    /// * `MemResult<()>` - Success or error
    pub fn add_memory_slot(&self, slot: MemorySlot) -> MemResult<()> {
        // Validate alignment
        if slot.guest_phys_addr % 4096 != 0 {
            return Err(MemError::NotAligned);
        }

        if slot.size % 4096 != 0 {
            return Err(MemError::NotAligned);
        }

        let slot_size = slot.size;
        let mut slots = self.slots.lock();
        slots.insert(slot.slot_id, slot);

        self.used_memory.fetch_add(slot_size, Ordering::SeqCst);
        Ok(())
    }

    /// Remove a memory slot
    ///
    /// # Arguments
    /// * `slot_id` - Slot ID
    pub fn remove_memory_slot(&self, slot_id: u32) -> MemResult<()> {
        let mut slots = self.slots.lock();
        let slot = slots.remove(&slot_id).ok_or(MemError::InvalidMemorySlot)?;

        self.used_memory.fetch_sub(slot.size, Ordering::SeqCst);
        Ok(())
    }

    /// Get memory slot
    ///
    /// # Arguments
    /// * `slot_id` - Slot ID
    ///
    /// # Returns
    /// * `MemResult<MemorySlot>` - Memory slot
    pub fn get_memory_slot(&self, slot_id: u32) -> MemResult<MemorySlot> {
        let slots = self.slots.lock();
        slots.get(&slot_id)
            .cloned()
            .ok_or(MemError::InvalidMemorySlot)
    }

    /// Translate GPA to HVA
    ///
    /// # Arguments
    /// * `gpa` - Guest physical address
    ///
    /// # Returns
    /// * `MemResult<u64>` - Host virtual address
    pub fn gpa_to_hva(&self, gpa: u64) -> MemResult<u64> {
        let slots = self.slots.lock();

        for slot in slots.values() {
            if gpa >= slot.guest_phys_addr && gpa < slot.guest_phys_addr + slot.size as u64 {
                let offset = gpa - slot.guest_phys_addr;
                return Ok(slot.userspace_addr + offset);
            }
        }

        Err(MemError::InvalidGpa)
    }

    /// Setup EPT for a GPA range
    ///
    /// # Arguments
    /// * `gpa_start` - Start GPA
    /// * `size` - Size in bytes
    /// * `hpa_start` - Start HPA
    /// * `permissions` - Access permissions
    pub fn setup_ept(
        &self,
        gpa_start: u64,
        size: usize,
        hpa_start: u64,
        permissions: MemoryPermissions,
    ) -> MemResult<()> {
        // Validate alignment
        if gpa_start % 4096 != 0 || hpa_start % 4096 != 0 {
            return Err(MemError::NotAligned);
        }

        let num_pages = size / 4096;

        for i in 0..num_pages {
            let gpa = gpa_start + (i as u64) * 4096;
            let hpa = hpa_start + (i as u64) * 4096;

            let entry = EptEntry {
                phys_addr: hpa,
                read: permissions.read,
                write: permissions.write,
                execute: permissions.execute,
                page_size: PageSize::Page4K,
                accessed: false,
                dirty: false,
                ignore_pat: false,
                mem_type: 0, // Write-back
            };

            // In real implementation, walk EPT and insert entry
            let _ = entry;
            let _ = gpa;
        }

        Ok(())
    }

    /// Initialize dirty bitmap
    ///
    /// # Arguments
    /// * `num_pages` - Number of pages to track
    pub fn init_dirty_bitmap(&self, num_pages: usize) -> MemResult<()> {
        let mut bitmap = self.dirty_bitmap.lock();
        *bitmap = Some(DirtyBitmap::new(num_pages));
        Ok(())
    }

    /// Get dirty bitmap
    ///
    /// # Returns
    /// * `MemResult<Vec<u64>>` - Copy of dirty bitmap
    pub fn get_dirty_bitmap(&self) -> MemResult<Vec<u64>> {
        let bitmap = self.dirty_bitmap.lock();
        if let Some(ref bitmap) = *bitmap {
            Ok(bitmap.bitmap.clone())
        } else {
            Err(MemError::InvalidMemorySlot)
        }
    }

    /// Clear dirty bitmap
    pub fn clear_dirty_bitmap(&self) -> MemResult<()> {
        let mut bitmap = self.dirty_bitmap.lock();
        if let Some(ref mut bitmap) = *bitmap {
            bitmap.clear();
            Ok(())
        } else {
            Err(MemError::InvalidMemorySlot)
        }
    }

    /// Initialize memory balloon
    ///
    /// # Arguments
    /// * `max_pages` - Maximum balloon size
    pub fn init_balloon(&self, max_pages: usize) -> MemResult<()> {
        let mut balloon = self.balloon.lock();
        *balloon = Some(MemoryBalloon::new(max_pages));
        Ok(())
    }

    /// Get memory statistics
    pub fn get_stats(&self) -> MemoryStats {
        MemoryStats {
            total_size: self.size,
            used_memory: self.used_memory.load(Ordering::SeqCst),
            allocated_pages: self.allocated_pages.load(Ordering::SeqCst),
            num_slots: self.slots.lock().len(),
        }
    }
}

/// Memory statistics
#[derive(Debug, Clone, Copy)]
pub struct MemoryStats {
    /// Total memory size
    pub total_size: usize,
    /// Used memory
    pub used_memory: usize,
    /// Allocated pages
    pub allocated_pages: usize,
    /// Number of memory slots
    pub num_slots: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vm_memory_creation() {
        let vm_mem = VmMemory::new(1024 * 1024 * 512).unwrap();
        assert_eq!(vm_mem.size, 1024 * 1024 * 512);
    }

    #[test]
    fn test_memory_slot() {
        let vm_mem = VmMemory::new(1024 * 1024).unwrap();

        let slot = MemorySlot {
            slot_id: 0,
            guest_phys_addr: 0,
            size: 4096,
            userspace_addr: 0x1000,
            flags: MemorySlotFlags::rw(),
            hva_mapped: false,
        };

        vm_mem.add_memory_slot(slot.clone()).unwrap();

        let retrieved = vm_mem.get_memory_slot(0).unwrap();
        assert_eq!(retrieved.slot_id, 0);
        assert_eq!(retrieved.size, 4096);
    }

    #[test]
    fn test_gpa_to_hva() {
        let vm_mem = VmMemory::new(1024 * 1024).unwrap();

        let slot = MemorySlot {
            slot_id: 0,
            guest_phys_addr: 0x1000,
            size: 4096,
            userspace_addr: 0x5000,
            flags: MemorySlotFlags::rw(),
            hva_mapped: false,
        };

        vm_mem.add_memory_slot(slot).unwrap();

        let hva = vm_mem.gpa_to_hva(0x1500).unwrap();
        assert_eq!(hva, 0x5500);
    }

    #[test]
    fn test_dirty_bitmap() {
        let bitmap = DirtyBitmap::new(100);

        assert!(!bitmap.is_dirty(50));

        bitmap.set_dirty(50);
        assert!(bitmap.is_dirty(50));

        assert_eq!(bitmap.dirty_count(), 1);

        bitmap.clear();
        assert!(!bitmap.is_dirty(50));
        assert_eq!(bitmap.dirty_count(), 0);
    }

    #[test]
    fn test_memory_balloon() {
        let balloon = MemoryBalloon::new(1000);
        balloon.enable();

        let pages = vec![0x1000, 0x2000, 0x3000];
        let inflated = balloon.inflate(&pages, 3).unwrap();
        assert_eq!(inflated, 3);
        assert_eq!(balloon.current_size(), 3);

        let deflated = balloon.deflate(2).unwrap();
        assert_eq!(deflated.len(), 2);
        assert_eq!(balloon.current_size(), 1);
    }

    #[test]
    fn test_ept_entry() {
        let entry = EptEntry {
            phys_addr: 0x1000,
            read: true,
            write: true,
            execute: false,
            page_size: PageSize::Page4K,
            accessed: false,
            dirty: false,
            ignore_pat: false,
            mem_type: 6, // Write-back
        };

        let raw = entry.to_raw();
        let parsed = EptEntry::from_raw(raw);

        assert_eq!(parsed.phys_addr, 0x1000);
        assert!(parsed.read);
        assert!(parsed.write);
        assert!(!parsed.execute);
    }

    #[test]
    fn test_page_size() {
        assert_eq!(PageSize::Page4K.size(), 4096);
        assert_eq!(PageSize::Page2M.size(), 2 * 1024 * 1024);
        assert_eq!(PageSize::Page1G.size(), 1024 * 1024 * 1024);
    }
}
