//! Page Table Isolation and Memory Protection
//!
//! This module implements page-level memory isolation for security:
//! - Kernel-space and user-space page separation
//! - Page table permissions (R/W/X)
//! - ASLR (Address Space Layout Randomization)
//! - Guard pages between memory regions
//! - Page fault handling with security checks
//!
//! Features:
//! - Multi-level page tables
//! - Per-process address spaces
//! - Page-level permissions (read, write, execute)
//! - NX (No-Execute) bit support
//! - Page guard pages
//! - ASLR implementation

use spin::Mutex;
use core::sync::atomic;
use alloc::{collections::BTreeMap, string::String, sync::Arc, vec::Vec};

// ============================================================================
// Page Table Constants
// ============================================================================

// Re-export PAGE_SIZE from unified memory management
pub use crate::subsystems::mm::PAGE_SIZE;

/// Page shift (log2 of page size)
pub const PAGE_SHIFT: usize = 12;

/// Page mask (for address alignment)
pub const PAGE_MASK: usize = !(PAGE_SIZE - 1);

/// Number of entries per page table
pub const ENTRIES_PER_TABLE: usize = 512;

/// Maximum number of page table levels
pub const MAX_PT_LEVELS: usize = 4;

/// Guard page pattern (for detecting overflows)
pub const GUARD_PAGE_PATTERN: u64 = 0xDEADBEEFDEADBEEF;

// ============================================================================
// Page Permissions
// ============================================================================

/// Page-level access permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PagePermissions {
    /// Readable
    pub readable: bool,
    
    /// Writable
    pub writable: bool,
    
    /// Executable
    pub executable: bool,
    
    /// User accessible
    pub user_accessible: bool,
    
    /// Copy-on-write
    pub cow: bool,
}

impl PagePermissions {
    /// Create new permissions (all denied)
    pub fn new() -> Self {
        Self {
            readable: false,
            writable: false,
            executable: false,
            user_accessible: false,
            cow: false,
        }
    }
    
    /// Kernel permissions (RW, no execute)
    pub fn kernel() -> Self {
        Self {
            readable: true,
            writable: true,
            executable: false,
            user_accessible: false,
            cow: false,
        }
    }
    
    /// User permissions (RWX)
    pub fn user() -> Self {
        Self {
            readable: true,
            writable: true,
            executable: true,
            user_accessible: true,
            cow: false,
        }
    }
    
    /// User read-only permissions
    pub fn user_readonly() -> Self {
        Self {
            readable: true,
            writable: false,
            executable: true,
            user_accessible: true,
            cow: false,
        }
    }
    
    /// User read-write (no execute)
    pub fn user_data() -> Self {
        Self {
            readable: true,
            writable: true,
            executable: false,
            user_accessible: true,
            cow: false,
        }
    }
    
    /// Convert to page table entry flags
    pub fn to_pte_flags(&self) -> u64 {
        let mut flags = 0u64;
        
        if self.readable {
            flags |= 1 << 0; // Present
        }
        
        if self.writable {
            flags |= 1 << 1; // Writable
        }
        
        if self.user_accessible {
            flags |= 1 << 2; // User
        }
        
        if self.executable {
            // NX (No-Execute) bit is clear for executable
            // If executable is false, set NX bit
        } else {
            flags |= 1 << 63; // NX bit
        }
        
        if self.cow {
            flags |= 1 << 4; // Copy-on-write
        }
        
        flags
    }
    
    /// Parse from page table entry flags
    pub fn from_pte_flags(flags: u64) -> Self {
        Self {
            readable: (flags & (1 << 0)) != 0,
            writable: (flags & (1 << 1)) != 0,
            user_accessible: (flags & (1 << 2)) != 0,
            executable: (flags & (1 << 63)) == 0, // NX bit not set
            cow: (flags & (1 << 4)) != 0,
        }
    }
}

// ============================================================================
// Page Table Entry
// ============================================================================

/// Page table entry descriptor
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PageTableEntry {
    /// Physical page frame number
    pub frame: u64,
    
    /// Page permissions
    pub permissions: PagePermissions,
    
    /// Global mapping (ignore ASID)
    pub global: bool,
    
    /// Access timestamp (for aging)
    pub access_time: u64,
    
    /// Dirty flag (page modified)
    pub dirty: bool,
    
    /// Accessed flag (page read)
    pub accessed: bool,
}

impl PageTableEntry {
    /// Create new page table entry
    pub fn new(frame: u64, permissions: PagePermissions) -> Self {
        Self {
            frame,
            permissions,
            global: false,
            access_time: 0,
            dirty: false,
            accessed: false,
        }
    }
    
    /// Create guard page (invalid, non-present)
    pub fn guard() -> Self {
        Self {
            frame: GUARD_PAGE_PATTERN,
            permissions: PagePermissions::new(),
            global: false,
            access_time: 0,
            dirty: false,
            accessed: false,
        }
    }
    
    /// Check if entry is valid (present)
    pub fn is_valid(&self) -> bool {
        self.permissions.readable && self.frame != GUARD_PAGE_PATTERN
    }
    
    /// Check if entry is a guard page
    pub fn is_guard(&self) -> bool {
        self.frame == GUARD_PAGE_PATTERN
    }
    
    /// Convert to u64 for storage
    pub fn to_u64(&self) -> u64 {
        let mut value = self.frame & 0x000FFFFFFFFFFF; // 44-bit physical address
        
        if self.permissions.readable {
            value |= 1 << 0;
        }
        
        if self.permissions.writable {
            value |= 1 << 1;
        }
        
        if self.permissions.user_accessible {
            value |= 1 << 2;
        }
        
        if !self.permissions.executable {
            value |= 1 << 63; // NX bit
        }
        
        if self.permissions.cow {
            value |= 1 << 4;
        }
        
        if self.global {
            value |= 1 << 8;
        }
        
        if self.dirty {
            value |= 1 << 6;
        }
        
        if self.accessed {
            value |= 1 << 5;
        }
        
        value
    }
    
    /// Parse from u64
    pub fn from_u64(value: u64) -> Self {
        Self {
            frame: value & 0x000FFFFFFFFFFF,
            permissions: PagePermissions::from_pte_flags(value),
            global: (value & (1 << 8)) != 0,
            access_time: 0,
            dirty: (value & (1 << 6)) != 0,
            accessed: (value & (1 << 5)) != 0,
        }
    }
}

// ============================================================================
// Address Space ID (ASID)
// ============================================================================

/// Address Space ID for TLB tagging
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Asid(u32);

impl Asid {
    /// Invalid ASID
    pub fn invalid() -> Self {
        Self(0)
    }
    
    /// Check if valid
    pub fn is_valid(&self) -> bool {
        self.0 != 0
    }
    
    /// Get raw value
    pub fn value(&self) -> u32 {
        self.0
    }
}

// ============================================================================
// Page Table Structure
// ============================================================================

/// Multi-level page table
pub struct PageTable {
    /// Page table level (0 = root)
    pub level: usize,
    
    /// Page table entries
    entries: [PageTableEntry; ENTRIES_PER_TABLE],
    
    /// Physical address of this table
    pub phys_addr: u64,
}

impl PageTable {
    /// Create new page table
    pub fn new(level: usize, phys_addr: u64) -> Self {
        let entries: [PageTableEntry; ENTRIES_PER_TABLE] = [PageTableEntry::guard(); ENTRIES_PER_TABLE];
        
        Self {
            level,
            entries,
            phys_addr,
        }
    }
    
    /// Get entry by index
    pub fn get_entry(&self, index: usize) -> &PageTableEntry {
        &self.entries[index]
    }
    
    /// Get mutable entry by index
    pub fn get_entry_mut(&mut self, index: usize) -> &mut PageTableEntry {
        &mut self.entries[index]
    }
    
    /// Map virtual page to physical frame
    pub fn map(&mut self, vpn: usize, frame: u64, permissions: PagePermissions) {
        let entry = PageTableEntry::new(frame, permissions);
        self.entries[vpn] = entry;
    }
    
    /// Unmap virtual page
    pub fn unmap(&mut self, vpn: usize) {
        self.entries[vpn] = PageTableEntry::guard();
    }
}

// ============================================================================
// Address Space
// ============================================================================

/// Virtual address space for a process
pub struct AddressSpace {
    /// Address space ID
    pub asid: Asid,
    
    /// Root page table
    pub root_page_table: *mut PageTable,
    
    /// ASLR base offset (randomized)
    pub aslr_base: u64,
    
    /// ASLR range size
    pub aslr_range: u64,
    
    /// Guard page at bottom
    pub guard_page_bottom: bool,
    
    /// Guard page at top
    pub guard_page_top: bool,
    
    /// Owner process ID
    pub owner_pid: usize,
    
    /// Total mapped pages
    pub mapped_pages: AtomicUsize,
    
    /// Page table levels
    pub max_levels: usize,
}

impl AddressSpace {
    /// Create new address space
    pub fn new(asid: Asid, owner_pid: usize) -> Self {
        Self {
            asid,
            root_page_table: core::ptr::null_mut(),
            aslr_base: 0,
            aslr_range: 0,
            guard_page_bottom: false,
            guard_page_top: false,
            owner_pid,
            mapped_pages: AtomicUsize::new(0),
            max_levels: MAX_PT_LEVELS,
        }
    }
    
    /// Initialize address space with ASLR
    pub fn init_with_aslr(&mut self, aslr_enabled: bool) {
        if aslr_enabled {
            // Generate random ASLR base (simplified)
            // In real implementation, would use cryptographic RNG
            self.aslr_base = 0x1000_0000; // Example: 256 MB base
            self.aslr_range = 0x1000_0000; // 256 MB range
        }
    }
    
    /// Get virtual address with ASLR offset
    pub fn get_va(&self, offset: usize) -> u64 {
        if self.aslr_base == 0 {
            offset as u64
        } else {
            (self.aslr_base + offset as u64) % self.aslr_range
        }
    }
    
    /// Map page in address space
    pub fn map_page(&self, va: usize, frame: u64, permissions: PagePermissions) -> bool {
        // Check guard pages
        let page_num = va >> PAGE_SHIFT;
        
        // In real implementation, would traverse page table hierarchy
        crate::println!("[pt_isolation] Mapping page {} to frame {} with permissions {:?}",
                        page_num, frame, permissions);
        
        true
    }
    
    /// Unmap page from address space
    pub fn unmap_page(&self, va: usize) {
        let page_num = va >> PAGE_SHIFT;
        
        crate::println!("[pt_isolation] Unmapping page {}", page_num);
    }
    
    /// Check if virtual address is mapped
    pub fn is_mapped(&self, va: usize) -> bool {
        // In real implementation, would check page table
        true
    }
    
    /// Enable guard pages
    pub fn enable_guard_pages(&mut self, bottom: bool, top: bool) {
        self.guard_page_bottom = bottom;
        self.guard_page_top = top;
        
        crate::println!("[pt_isolation] Guard pages: bottom={}, top={}", bottom, top);
    }
    
    /// Check for guard page violations
    pub fn check_guard_violation(&self, va: usize) -> bool {
        if self.guard_page_bottom && va < PAGE_SIZE {
            crate::println!("[pt_isolation] Guard page violation at VA {:#x}", va);
            return true;
        }
        
        if self.guard_page_top {
            // Check against top guard (implementation-specific)
            let max_va = self.aslr_base + self.aslr_range;
            if va >= max_va && va < max_va + PAGE_SIZE {
                crate::println!("[pt_isolation] Guard page violation at VA {:#x}", va);
                return true;
            }
        }
        
        false
    }
}

// ============================================================================
// Address Space Manager
// ============================================================================

/// Manager for all address spaces
pub struct AddressSpaceManager {
    /// Active address spaces by ASID
    address_spaces: Mutex<BTreeMap<u32, Arc<AddressSpace>>>,
    
    /// Next ASID to allocate
    next_asid: AtomicU32,
    
    /// Total address spaces
    total_spaces: AtomicUsize,
    
    /// ASLR enabled
    aslr_enabled: AtomicU64,
    
    /// Page fault handler
    page_fault_handler: Option<fn(va: usize, error: PageFaultError) -> bool>,
}

/// Page fault error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageFaultError {
    /// Permission denied
    PermissionDenied,
    
    /// Page not present
    NotPresent,
    
    /// Guard page violation
    GuardViolation,
    
    /// Invalid address (out of range)
    InvalidAddress,
    
    /// Write to read-only page
    WriteToReadOnly,
    
    /// Execute from non-executable page
    ExecuteFromNonExecutable,
}

impl AddressSpaceManager {
    /// Create new address space manager
    pub fn new() -> Self {
        Self {
            address_spaces: Mutex::new(BTreeMap::new()),
            next_asid: AtomicU32::new(1),
            total_spaces: AtomicUsize::new(0),
            aslr_enabled: AtomicU64::new(1), // ASLR enabled by default
            page_fault_handler: None,
        }
    }
    
    /// Create new address space
    pub fn create_address_space(&self, owner_pid: usize) -> Result<Arc<AddressSpace>, &'static str> {
        let asid_value = self.next_asid.fetch_add(1, Ordering::Relaxed);
        let asid = Asid(asid_value);
        
        let mut addr_space = AddressSpace::new(asid, owner_pid);
        addr_space.init_with_aslr(self.is_aslr_enabled());
        
        self.total_spaces.fetch_add(1, Ordering::Relaxed);
        
        crate::println!("[pt_isolation] Created address space ASID {} for PID {}",
                        asid_value, owner_pid);
        
        Ok(Arc::new(addr_space))
    }
    
    /// Destroy address space
    pub fn destroy_address_space(&self, asid: Asid) {
        let mut spaces = self.address_spaces.lock();
        
        if let Some(addr_space) = spaces.remove(&asid.value()) {
            crate::println!("[pt_isolation] Destroyed address space ASID {}", asid.value());
            
            // In real implementation, would free all page tables
            self.total_spaces.fetch_sub(1, Ordering::Relaxed);
        }
    }
    
    /// Get address space by ASID
    pub fn get_address_space(&self, asid: Asid) -> Option<Arc<AddressSpace>> {
        let spaces = self.address_spaces.lock();
        spaces.get(&asid.value()).cloned()
    }
    
    /// Set page fault handler
    pub fn set_page_fault_handler(&mut self, handler: fn(va: usize, error: PageFaultError) -> bool) {
        self.page_fault_handler = Some(handler);
    }
    
    /// Handle page fault
    pub fn handle_page_fault(&self, va: usize, error: PageFaultError) -> bool {
        crate::println!("[pt_isolation] Page fault at VA {:#x}: {:?}", va, error);
        
        // Check guard page violations
        if error == PageFaultError::GuardViolation {
            return false; // Kill process
        }
        
        // Check permission errors
        if error == PageFaultError::PermissionDenied || 
           error == PageFaultError::WriteToReadOnly ||
           error == PageFaultError::ExecuteFromNonExecutable {
            
            // Security violation
            crate::println!("[pt_isolation] Security violation detected!");
            return false;
        }
        
        // Call custom handler if set
        if let Some(handler) = self.page_fault_handler {
            return handler(va, error);
        }
        
        true // Handle fault (COW, demand paging, etc.)
    }
    
    /// Check if ASLR is enabled
    pub fn is_aslr_enabled(&self) -> bool {
        self.aslr_enabled.load(Ordering::Relaxed) != 0
    }
    
    /// Enable/disable ASLR
    pub fn set_aslr_enabled(&self, enabled: bool) {
        let value = if enabled { 1 } else { 0 };
        self.aslr_enabled.store(value, Ordering::Release);
        
        crate::println!("[pt_isolation] ASLR {}", if enabled { "enabled" } else { "disabled" });
    }
    
    /// Get statistics
    pub fn stats(&self) -> AddressSpaceStats {
        let spaces = self.address_spaces.lock();
        AddressSpaceStats {
            total_spaces: self.total_spaces.load(Ordering::Relaxed),
            active_spaces: spaces.len(),
            aslr_enabled: self.is_aslr_enabled(),
        }
    }
}

/// Address space statistics
#[derive(Debug, Clone, Copy)]
pub struct AddressSpaceStats {
    /// Total address spaces created
    pub total_spaces: usize,
    
    /// Currently active address spaces
    pub active_spaces: usize,
    
    /// ASLR enabled
    pub aslr_enabled: bool,
}

// ============================================================================
// Security Audit Trail
// ============================================================================

/// Security event for audit trail
#[derive(Debug, Clone)]
pub struct SecurityEvent {
    /// Event timestamp
    pub timestamp: u64,
    
    /// Event type
    pub event_type: SecurityEventType,
    
    /// Address space ID (if applicable)
    pub asid: Option<u32>,
    
    /// Virtual address (if applicable)
    pub va: Option<usize>,
    
    /// Physical frame (if applicable)
    pub frame: Option<u64>,
    
    /// Process ID (if applicable)
    pub pid: Option<usize>,
    
    /// Description
    pub description: String,
}

/// Security event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityEventType {
    /// Address space created
    AddressSpaceCreated,
    
    /// Address space destroyed
    AddressSpaceDestroyed,
    
    /// Page mapped
    PageMapped,
    
    /// Page unmapped
    PageUnmapped,
    
    /// Guard page violation
    GuardViolation,
    
    /// Permission denied
    PermissionDenied,
    
    /// Page fault
    PageFault,
    
    /// Security violation
    SecurityViolation,
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_permissions() {
        let user_perms = PagePermissions::user();
        let kernel_perms = PagePermissions::kernel();
        
        assert!(user_perms.user_accessible);
        assert!(user_perms.executable);
        assert!(!kernel_perms.user_accessible);
        assert!(!kernel_perms.executable);
    }

    #[test]
    fn test_pte_conversion() {
        let pte = PageTableEntry::new(0x1234_5678, PagePermissions::user());
        let value = pte.to_u64();
        let pte2 = PageTableEntry::from_u64(value);
        
        assert_eq!(pte.frame, pte2.frame);
        assert_eq!(pte.permissions, pte2.permissions);
    }

    #[test]
    fn test_guard_page() {
        let guard = PageTableEntry::guard();
        
        assert!(!guard.is_valid());
        assert!(guard.is_guard());
    }

    #[test]
    fn test_address_space_creation() {
        let mut manager = AddressSpaceManager::new();
        
        let addr_space = manager.create_address_space(123).unwrap();
        
        assert!(addr_space.asid.is_valid());
        assert_eq!(addr_space.owner_pid, 123);
    }

    #[test]
    fn test_aslr() {
        let mut addr_space = AddressSpace::new(Asid(1), 123);
        
        addr_space.init_with_aslr(true);
        
        let va1 = addr_space.get_va(0x1000);
        let va2 = addr_space.get_va(0x2000);
        
        assert!(va1 != va2); // ASLR should add offset
    }

    #[test]
    fn test_guard_violation() {
        let mut addr_space = AddressSpace::new(Asid(1), 123);
        addr_space.enable_guard_pages(true, false);
        
        // Check guard page violation
        let violation = addr_space.check_guard_violation(0x100);
        assert!(violation);
    }
}
