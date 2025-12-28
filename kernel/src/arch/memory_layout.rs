//! Architecture-specific memory layout definitions
//!
//! This module provides architecture-agnostic abstractions for memory layout
//! constants, removing hardcoded addresses from the kernel codebase.
//! Each architecture defines its own memory layout based on its specific
//! requirements and address space constraints.

/// Memory layout configuration for a specific architecture
#[derive(Debug, Clone, Copy)]
pub struct MemoryLayout {
    /// Kernel base virtual address
    pub kernel_base: usize,
    
    /// Kernel code region base
    pub kernel_code_base: usize,
    
    /// Kernel data region base
    pub kernel_data_base: usize,
    
    /// Kernel heap base
    pub kernel_heap_base: usize,
    
    /// User space base address
    pub user_base: usize,
    
    /// User stack top address (highest valid user stack address)
    pub user_stack_top: usize,
    
    /// User heap base address
    pub user_heap_base: usize,
    
    /// Maximum user address (exclusive, kernel space starts here)
    pub user_max: usize,
    
    /// Page size in bytes
    pub page_size: usize,
    
    /// Physical memory direct map base (if supported)
    pub phys_map_base: Option<usize>,
    
    /// MMIO region base (if supported)
    pub mmio_base: Option<usize>,
    
    /// MMIO region size
    pub mmio_size: Option<usize>,
}

impl MemoryLayout {
    /// Check if an address is in kernel space
    pub fn is_kernel_address(&self, addr: usize) -> bool {
        addr >= self.kernel_base
    }
    
    /// Check if an address is in user space
    pub fn is_user_address(&self, addr: usize) -> bool {
        addr < self.user_max && addr >= self.user_base
    }
    
    /// Convert physical address to kernel virtual address (if direct mapping supported)
    pub fn phys_to_virt(&self, phys: usize) -> Option<usize> {
        self.phys_map_base.map(|base| base + phys)
    }
    
    /// Convert kernel virtual address to physical address (if direct mapping supported)
    pub fn virt_to_phys(&self, virt: usize) -> Option<usize> {
        if let Some(base) = self.phys_map_base {
            if virt >= base {
                Some(virt - base)
            } else {
                None
            }
        } else {
            None
        }
    }
    
    /// Verify that physical mapping region doesn't overlap with kernel code region
    pub fn verify_memory_layout(&self) -> Result<(), &'static str> {
        // Check if phys_map_base overlaps with kernel code region
        if let Some(phys_base) = self.phys_map_base {
            let kernel_code_end = self.kernel_code_base + 0x1000_0000; // Assume 256MB kernel code region
            
            if phys_base >= self.kernel_code_base && phys_base < kernel_code_end {
                return Err("Physical mapping region overlaps with kernel code region");
            }
            
            // Check if phys_map_base overlaps with kernel data region
            let kernel_data_end = self.kernel_data_base + 0x1000_0000; // Assume 256MB kernel data region
            if phys_base >= self.kernel_data_base && phys_base < kernel_data_end {
                return Err("Physical mapping region overlaps with kernel data region");
            }
            
            // Check if phys_map_base overlaps with kernel heap region
            let kernel_heap_end = self.kernel_heap_base + 0x1000_0000; // Assume 256MB kernel heap region
            if phys_base >= self.kernel_heap_base && phys_base < kernel_heap_end {
                return Err("Physical mapping region overlaps with kernel heap region");
            }
        }
        
        // Verify user space doesn't overlap with kernel space
        if self.user_max >= self.kernel_base {
            return Err("User space overlaps with kernel space");
        }
        
        Ok(())
    }
    
    /// Get the current architecture's memory layout
    pub fn current() -> &'static MemoryLayout {
        #[cfg(target_arch = "x86_64")]
        return &X86_64_LAYOUT;
        
        #[cfg(target_arch = "aarch64")]
        return &AARCH64_LAYOUT;
        
        #[cfg(target_arch = "riscv64")]
        return &RISCV64_LAYOUT;
        
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "riscv64")))]
        compile_error!("Unsupported architecture");
    }
}

// ============================================================================
// x86_64 Memory Layout
// ============================================================================

#[cfg(target_arch = "x86_64")]
const X86_64_LAYOUT: MemoryLayout = MemoryLayout {
    // x86_64 uses canonical addressing with kernel in upper half
    // Kernel base: 0xFFFF_FFFF_8000_0000 (typical Linux-style layout)
    kernel_base: 0xFFFF_FFFF_8000_0000,
    
    // Kernel code region starts at kernel base
    kernel_code_base: 0xFFFF_FFFF_8000_0000,
    
    // Kernel data region (after code, typically +16MB)
    kernel_data_base: 0xFFFF_FFFF_8100_0000,
    
    // Kernel heap (after data, typically +32MB)
    kernel_heap_base: 0xFFFF_FFFF_8300_0000,
    
    // User space starts at 0x0000_0000_1000_0000 (16MB, leaving room for NULL page)
    user_base: 0x0000_0000_1000_0000,
    
    // User stack top (typical Linux layout: 0x0000_7FFF_FFFF_F000)
    user_stack_top: 0x0000_7FFF_FFFF_F000,
    
    // User heap starts after user base
    user_heap_base: 0x0000_0000_2000_0000,
    
    // Maximum user address (kernel space starts at kernel_base)
    user_max: 0xFFFF_FFFF_8000_0000,
    
    // 4KB pages
    page_size: 4096,
    
    // x86_64 supports direct physical mapping
    // Physical memory is mapped in a separate region to avoid conflicts with kernel code
    // Using the 128TB region starting at 0xFFFF_8000_0000_0000 (separate from kernel code at 0xFFFF_FFFF_8000_0000)
    phys_map_base: Some(0xFFFF_8000_0000_0000),
    
    // MMIO regions (architecture-specific)
    mmio_base: Some(0xFFFF_8000_0000_0000),
    mmio_size: Some(0x0000_8000_0000_0000), // 128TB
};

// ============================================================================
// AArch64 Memory Layout
// ============================================================================

#[cfg(target_arch = "aarch64")]
const AARCH64_LAYOUT: MemoryLayout = MemoryLayout {
    // AArch64 uses TTBR1 for kernel (upper half of address space)
    // Kernel base: 0xFFFF_0000_0000_0000 (typical Linux layout)
    kernel_base: 0xFFFF_0000_0000_0000,
    
    // Kernel code region
    kernel_code_base: 0xFFFF_0000_0000_0000,
    
    // Kernel data region
    kernel_data_base: 0xFFFF_0000_0100_0000,
    
    // Kernel heap
    kernel_heap_base: 0xFFFF_0000_0300_0000,
    
    // User space starts at 0x0000_0000_0000_0000
    user_base: 0x0000_0000_0000_0000,
    
    // User stack top (typical: 0x0000_7FFF_FFFF_F000)
    user_stack_top: 0x0000_7FFF_FFFF_F000,
    
    // User heap
    user_heap_base: 0x0000_0000_1000_0000,
    
    // Maximum user address
    user_max: 0x0000_8000_0000_0000,
    
    // 4KB pages (can also support 16KB, 64KB)
    page_size: 4096,
    
    // AArch64 supports direct physical mapping
    // Physical memory mapped in separate region from kernel code
    // Kernel code at 0xFFFF_0000_0000_0000, phys map at 0xFFFF_8000_0000_0000
    phys_map_base: Some(0xFFFF_8000_0000_0000),
    
    // MMIO regions
    mmio_base: Some(0xFFFF_0000_0000_0000),
    mmio_size: Some(0x0000_0000_8000_0000), // 32GB
};

// ============================================================================
// RISC-V 64 Memory Layout
// ============================================================================

#[cfg(target_arch = "riscv64")]
const RISCV64_LAYOUT: MemoryLayout = MemoryLayout {
    // RISC-V uses Sv39/Sv48 paging with kernel in upper half
    // Kernel base: 0xFFFF_FFFF_0000_0000 (typical layout)
    kernel_base: 0xFFFF_FFFF_0000_0000,
    
    // Kernel code region
    kernel_code_base: 0xFFFF_FFFF_0000_0000,
    
    // Kernel data region
    kernel_data_base: 0xFFFF_FFFF_0100_0000,
    
    // Kernel heap
    kernel_heap_base: 0xFFFF_FFFF_0300_0000,
    
    // User space starts at 0x0000_0000_0000_0000
    user_base: 0x0000_0000_0000_0000,
    
    // User stack top
    user_stack_top: 0x0000_7FFF_FFFF_F000,
    
    // User heap
    user_heap_base: 0x0000_0000_1000_0000,
    
    // Maximum user address
    user_max: 0x0000_8000_0000_0000,
    
    // 4KB pages
    page_size: 4096,
    
    // RISC-V supports direct physical mapping
    // Physical memory mapped in separate region from kernel code
    // Kernel code at 0xFFFF_FFFF_0000_0000, phys map at 0xFFFF_FFFF_8000_0000 (separate 2GB region)
    // Note: RISC-V has limited address space, so we use a closer region but still separate
    phys_map_base: Some(0xFFFF_FFFF_8000_0000),
    
    // MMIO regions
    mmio_base: Some(0xFFFF_FFFF_0000_0000),
    mmio_size: Some(0x0000_0000_8000_0000), // 32GB
};

// ============================================================================
// Public API - Architecture-agnostic constants
// ============================================================================

/// Get kernel base address for current architecture
pub const fn kernel_base() -> usize {
    MemoryLayout::current().kernel_base
}

/// Get user base address for current architecture
pub const fn user_base() -> usize {
    MemoryLayout::current().user_base
}

/// Get user stack top address for current architecture
pub const fn user_stack_top() -> usize {
    MemoryLayout::current().user_stack_top
}

/// Get maximum user address for current architecture
pub const fn user_max() -> usize {
    MemoryLayout::current().user_max
}

/// Get page size for current architecture
pub const fn page_size() -> usize {
    MemoryLayout::current().page_size
}

/// Check if address is in kernel space
pub fn is_kernel_address(addr: usize) -> bool {
    MemoryLayout::current().is_kernel_address(addr)
}

/// Check if address is in user space
pub fn is_user_address(addr: usize) -> bool {
    MemoryLayout::current().is_user_address(addr)
}

/// Convert physical address to kernel virtual address (if supported)
pub fn phys_to_virt(phys: usize) -> Option<usize> {
    MemoryLayout::current().phys_to_virt(phys)
}

/// Convert kernel virtual address to physical address (if supported)
pub fn virt_to_phys(virt: usize) -> Option<usize> {
    MemoryLayout::current().virt_to_phys(virt)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_layout_access() {
        let layout = MemoryLayout::current();
        assert!(layout.kernel_base > 0);
        assert!(layout.user_base < layout.user_max);
        assert!(layout.page_size > 0);
    }
    
    #[test]
    fn test_address_checks() {
        let layout = MemoryLayout::current();
        
        // Kernel addresses should be detected correctly
        assert!(layout.is_kernel_address(layout.kernel_base));
        assert!(layout.is_kernel_address(layout.kernel_base + 0x1000));
        
        // User addresses should be detected correctly
        assert!(layout.is_user_address(layout.user_base));
        assert!(layout.is_user_address(layout.user_base + 0x1000));
        assert!(!layout.is_user_address(layout.user_max));
    }
    
    #[test]
    fn test_phys_virt_conversion() {
        let layout = MemoryLayout::current();
        
        if let Some(phys_base) = layout.phys_map_base {
            // Test conversion
            let phys = 0x100000;
            if let Some(virt) = layout.phys_to_virt(phys) {
                assert_eq!(virt, phys_base + phys);
                
                // Test reverse conversion
                if let Some(converted_phys) = layout.virt_to_phys(virt) {
                    assert_eq!(converted_phys, phys);
                }
            }
        }
    }
}

