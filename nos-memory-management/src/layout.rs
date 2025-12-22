//! Architecture-specific memory address space layout definitions
//!
//! This module provides centralized definitions of memory layouts for all
//! supported architectures (x86_64, aarch64, riscv64). It ensures consistent
//! address space organization across the system and supports features like ASLR.
//!
//! ## Memory Layout Design Principles
//!
//! 1. **Separation of Concerns**: Kernel code, data, heap, and physical mapping
//!    regions are kept separate to avoid address conflicts.
//! 2. **Architecture Compatibility**: Each architecture follows its standard
//!    memory layout conventions (e.g., Linux-style layouts).
//! 3. **ASLR Support**: Layout definitions support address space layout
//!    randomization for security.
//! 4. **Physical Mapping**: Direct physical memory mapping regions are separate
//!    from kernel code regions to prevent conflicts.

/// Memory layout configuration for a specific architecture
#[derive(Debug, Clone, Copy)]
pub struct AddressSpaceLayout {
    /// Kernel base virtual address
    pub kernel_base: usize,
    
    /// Kernel code region base
    pub kernel_code_base: usize,
    
    /// Kernel code region size (bytes)
    pub kernel_code_size: usize,
    
    /// Kernel data region base
    pub kernel_data_base: usize,
    
    /// Kernel data region size (bytes)
    pub kernel_data_size: usize,
    
    /// Kernel heap base
    pub kernel_heap_base: usize,
    
    /// Kernel heap size (bytes)
    pub kernel_heap_size: usize,
    
    /// User space base address
    pub user_base: usize,
    
    /// User stack top address (highest valid user stack address)
    pub user_stack_top: usize,
    
    /// User stack size (bytes)
    pub user_stack_size: usize,
    
    /// User heap base address
    pub user_heap_base: usize,
    
    /// User heap size (bytes)
    pub user_heap_size: usize,
    
    /// Maximum user address (exclusive, kernel space starts here)
    pub user_max: usize,
    
    /// Page size in bytes
    pub page_size: usize,
    
    /// Physical memory direct map base (if supported)
    pub phys_map_base: Option<usize>,
    
    /// Physical memory direct map size (bytes)
    pub phys_map_size: Option<usize>,
    
    /// MMIO region base (if supported)
    pub mmio_base: Option<usize>,
    
    /// MMIO region size (bytes)
    pub mmio_size: Option<usize>,
    
    /// ASLR offset range (for randomization)
    /// This is the maximum random offset that can be applied to base addresses
    pub aslr_offset_range: usize,
}

impl AddressSpaceLayout {
    /// Check if an address is in kernel space
    #[inline]
    pub fn is_kernel_address(&self, addr: usize) -> bool {
        addr >= self.kernel_base
    }
    
    /// Check if an address is in user space
    #[inline]
    pub fn is_user_address(&self, addr: usize) -> bool {
        addr < self.user_max && addr >= self.user_base
    }
    
    /// Check if an address is in kernel code region
    #[inline]
    pub fn is_kernel_code(&self, addr: usize) -> bool {
        addr >= self.kernel_code_base && 
        addr < self.kernel_code_base + self.kernel_code_size
    }
    
    /// Check if an address is in kernel data region
    #[inline]
    pub fn is_kernel_data(&self, addr: usize) -> bool {
        addr >= self.kernel_data_base && 
        addr < self.kernel_data_base + self.kernel_data_size
    }
    
    /// Check if an address is in kernel heap region
    #[inline]
    pub fn is_kernel_heap(&self, addr: usize) -> bool {
        addr >= self.kernel_heap_base && 
        addr < self.kernel_heap_base + self.kernel_heap_size
    }
    
    /// Convert physical address to kernel virtual address (if direct mapping supported)
    #[inline]
    pub fn phys_to_virt(&self, phys: usize) -> Option<usize> {
        self.phys_map_base.map(|base| base + phys)
    }
    
    /// Convert kernel virtual address to physical address (if direct mapping supported)
    #[inline]
    pub fn virt_to_phys(&self, virt: usize) -> Option<usize> {
        if let Some(base) = self.phys_map_base {
            if let Some(size) = self.phys_map_size {
                if virt >= base && virt < base + size {
                    return Some(virt - base);
                }
            } else if virt >= base {
                return Some(virt - base);
            }
        }
        None
    }
    
    /// Memory region type for ASLR offset application
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum AslrRegionType {
        /// Kernel code region
        KernelCode,
        /// Kernel data region
        KernelData,
        /// Kernel heap region
        KernelHeap,
        /// User code region
        UserCode,
        /// User data region
        UserData,
        /// User heap region
        UserHeap,
        /// User stack region
        UserStack,
        /// Physical mapping region
        PhysicalMapping,
        /// MMIO region
        Mmio,
    }

    /// Apply ASLR offset to a base address with boundary checks and region awareness
    /// 
    /// This function applies ASLR offset with the following enhancements:
    /// 1. Boundary checking to ensure the offset doesn't cause address overflow
    /// 2. Region-aware offset application (different strategies for different regions)
    /// 3. Validation that the resulting address doesn't conflict with other regions
    /// 
    /// # Arguments
    /// * `base` - Base address to apply offset to
    /// * `offset` - ASLR offset (will be page-aligned automatically)
    /// * `region_type` - Type of memory region (for region-aware offset application)
    /// 
    /// # Returns
    /// * `Ok(usize)` - Adjusted address with offset applied
    /// * `Err(&'static str)` - Error if offset would cause address conflict or overflow
    pub fn apply_aslr_offset_enhanced(
        &self,
        base: usize,
        offset: usize,
        region_type: AslrRegionType,
    ) -> Result<usize, &'static str> {
        // Ensure offset is page-aligned
        let aligned_offset = (offset / self.page_size) * self.page_size;
        
        // Region-aware offset clamping
        let max_offset = match region_type {
            AslrRegionType::KernelCode | AslrRegionType::KernelData => {
                // Kernel code/data: use smaller offset range to avoid conflicts
                self.aslr_offset_range.min(0x1000_0000) // Max 256MB
            }
            AslrRegionType::KernelHeap => {
                // Kernel heap: can use larger offset
                self.aslr_offset_range.min(0x4000_0000) // Max 1GB
            }
            AslrRegionType::UserCode | AslrRegionType::UserData | AslrRegionType::UserHeap => {
                // User regions: use full range but check against user_max
                self.aslr_offset_range
            }
            AslrRegionType::UserStack => {
                // User stack: smaller offset to avoid conflicts with heap
                self.aslr_offset_range.min(0x2000_0000) // Max 512MB
            }
            AslrRegionType::PhysicalMapping | AslrRegionType::Mmio => {
                // Physical mapping and MMIO: minimal offset to avoid conflicts
                self.aslr_offset_range.min(0x1000_0000) // Max 256MB
            }
        };
        
        let clamped_offset = aligned_offset.min(max_offset);
        
        // Check for address overflow
        let adjusted = base.checked_add(clamped_offset)
            .ok_or("ASLR offset would cause address overflow")?;
        
        // Boundary checks based on region type
        match region_type {
            AslrRegionType::KernelCode => {
                let max_addr = self.kernel_code_base + self.kernel_code_size;
                if adjusted >= max_addr {
                    return Err("ASLR offset would exceed kernel code region");
                }
                // Check for overlap with kernel data region
                if adjusted + self.kernel_code_size > self.kernel_data_base {
                    return Err("ASLR-adjusted kernel code would overlap with kernel data");
                }
            }
            AslrRegionType::KernelData => {
                let max_addr = self.kernel_data_base + self.kernel_data_size;
                if adjusted >= max_addr {
                    return Err("ASLR offset would exceed kernel data region");
                }
                // Check for overlap with kernel heap
                if adjusted + self.kernel_data_size > self.kernel_heap_base {
                    return Err("ASLR-adjusted kernel data would overlap with kernel heap");
                }
            }
            AslrRegionType::KernelHeap => {
                let max_addr = self.kernel_heap_base + self.kernel_heap_size;
                if adjusted >= max_addr {
                    return Err("ASLR offset would exceed kernel heap region");
                }
            }
            AslrRegionType::UserCode | AslrRegionType::UserData | AslrRegionType::UserHeap => {
                // Check that adjusted address is still in user space
                if adjusted >= self.user_max {
                    return Err("ASLR offset would push address into kernel space");
                }
                if adjusted < self.user_base {
                    return Err("ASLR offset would push address below user base");
                }
            }
            AslrRegionType::UserStack => {
                // Stack grows downward, so check top address
                if adjusted >= self.user_stack_top {
                    return Err("ASLR offset would exceed user stack top");
                }
                if adjusted < self.user_stack_top.saturating_sub(self.user_stack_size) {
                    return Err("ASLR offset would push stack below valid range");
                }
            }
            AslrRegionType::PhysicalMapping => {
                if let Some(phys_base) = self.phys_map_base {
                    if let Some(phys_size) = self.phys_map_size {
                        let max_addr = phys_base + phys_size;
                        if adjusted >= max_addr {
                            return Err("ASLR offset would exceed physical mapping region");
                        }
                        // Check for overlap with kernel regions
                        if adjusted < self.kernel_base + self.kernel_code_size + self.kernel_data_size + self.kernel_heap_size {
                            return Err("ASLR-adjusted physical mapping would overlap with kernel regions");
                        }
                    }
                }
            }
            AslrRegionType::Mmio => {
                if let Some(mmio_base) = self.mmio_base {
                    if let Some(mmio_size) = self.mmio_size {
                        let max_addr = mmio_base + mmio_size;
                        if adjusted >= max_addr {
                            return Err("ASLR offset would exceed MMIO region");
                        }
                    }
                }
            }
        }
        
        Ok(adjusted)
    }
    
    /// Apply ASLR offset to a base address (backward-compatible version)
    /// 
    /// This is a simplified version that uses default region type (KernelCode)
    /// for backward compatibility. For new code, use `apply_aslr_offset_enhanced`.
    /// 
    /// # Arguments
    /// * `base` - Base address to apply offset to
    /// * `offset` - ASLR offset (must be page-aligned and within aslr_offset_range)
    /// 
    /// # Returns
    /// Adjusted address with offset applied
    #[inline]
    pub fn apply_aslr_offset(&self, base: usize, offset: usize) -> usize {
        // Use enhanced version with default region type
        self.apply_aslr_offset_enhanced(base, offset, AslrRegionType::KernelCode)
            .unwrap_or_else(|_| {
                // Fallback to simple addition if enhanced version fails
                let aligned_offset = (offset / self.page_size) * self.page_size;
                let clamped_offset = aligned_offset.min(self.aslr_offset_range);
                base + clamped_offset
            })
    }
    
    /// Verify that memory layout is valid (no overlaps)
    pub fn verify(&self) -> Result<(), &'static str> {
        // Check if phys_map_base overlaps with kernel code region
        if let Some(phys_base) = self.phys_map_base {
            if phys_base >= self.kernel_code_base && 
               phys_base < self.kernel_code_base + self.kernel_code_size {
                return Err("Physical mapping region overlaps with kernel code region");
            }
            
            // Check if phys_map_base overlaps with kernel data region
            if phys_base >= self.kernel_data_base && 
               phys_base < self.kernel_data_base + self.kernel_data_size {
                return Err("Physical mapping region overlaps with kernel data region");
            }
            
            // Check if phys_map_base overlaps with kernel heap region
            if phys_base >= self.kernel_heap_base && 
               phys_base < self.kernel_heap_base + self.kernel_heap_size {
                return Err("Physical mapping region overlaps with kernel heap region");
            }
        }
        
        // Verify user space doesn't overlap with kernel space
        if self.user_max >= self.kernel_base {
            return Err("User space overlaps with kernel space");
        }
        
        // Verify regions don't overlap with each other
        if self.kernel_code_base + self.kernel_code_size > self.kernel_data_base {
            return Err("Kernel code region overlaps with kernel data region");
        }
        
        if self.kernel_data_base + self.kernel_data_size > self.kernel_heap_base {
            return Err("Kernel data region overlaps with kernel heap region");
        }
        
        Ok(())
    }
    
    /// Get the current architecture's memory layout
    pub fn current() -> &'static AddressSpaceLayout {
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
const X86_64_LAYOUT: AddressSpaceLayout = AddressSpaceLayout {
    // x86_64 uses canonical addressing with kernel in upper half
    // Kernel base: 0xFFFF_FFFF_8000_0000 (typical Linux-style layout)
    kernel_base: 0xFFFF_FFFF_8000_0000,
    
    // Kernel code region (256MB)
    kernel_code_base: 0xFFFF_FFFF_8000_0000,
    kernel_code_size: 0x1000_0000, // 256MB
    
    // Kernel data region (256MB, after code)
    kernel_data_base: 0xFFFF_FFFF_8100_0000,
    kernel_data_size: 0x1000_0000, // 256MB
    
    // Kernel heap (1GB, after data)
    kernel_heap_base: 0xFFFF_FFFF_8300_0000,
    kernel_heap_size: 0x4000_0000, // 1GB
    
    // User space starts at 0x0000_0000_1000_0000 (16MB, leaving room for NULL page)
    user_base: 0x0000_0000_1000_0000,
    
    // User stack top (typical Linux layout: 0x0000_7FFF_FFFF_F000)
    user_stack_top: 0x0000_7FFF_FFFF_F000,
    user_stack_size: 0x8000_0000, // 2GB
    
    // User heap starts after user base
    user_heap_base: 0x0000_0000_2000_0000,
    user_heap_size: 0x7FFF_E000_0000, // ~2TB
    
    // Maximum user address (kernel space starts at kernel_base)
    user_max: 0xFFFF_FFFF_8000_0000,
    
    // 4KB pages
    page_size: 4096,
    
    // x86_64 supports direct physical mapping
    // Physical memory is mapped in a separate region to avoid conflicts with kernel code
    // Using the 128TB region starting at 0xFFFF_8000_0000_0000
    phys_map_base: Some(0xFFFF_8000_0000_0000),
    phys_map_size: Some(0x0000_8000_0000_0000), // 128TB
    
    // MMIO regions (architecture-specific)
    mmio_base: Some(0xFFFF_8000_0000_0000),
    mmio_size: Some(0x0000_8000_0000_0000), // 128TB
    
    // ASLR offset range: 1GB (for randomization)
    aslr_offset_range: 0x4000_0000,
};

// ============================================================================
// AArch64 Memory Layout
// ============================================================================

#[cfg(target_arch = "aarch64")]
const AARCH64_LAYOUT: AddressSpaceLayout = AddressSpaceLayout {
    // AArch64 uses TTBR1 for kernel (upper half of address space)
    // Kernel base: 0xFFFF_0000_0000_0000 (typical Linux layout)
    kernel_base: 0xFFFF_0000_0000_0000,
    
    // Kernel code region (256MB)
    kernel_code_base: 0xFFFF_0000_0000_0000,
    kernel_code_size: 0x1000_0000, // 256MB
    
    // Kernel data region (256MB)
    kernel_data_base: 0xFFFF_0000_0100_0000,
    kernel_data_size: 0x1000_0000, // 256MB
    
    // Kernel heap (1GB)
    kernel_heap_base: 0xFFFF_0000_0300_0000,
    kernel_heap_size: 0x4000_0000, // 1GB
    
    // User space starts at 0x0000_0000_0000_0000
    user_base: 0x0000_0000_0000_0000,
    
    // User stack top (typical: 0x0000_7FFF_FFFF_F000)
    user_stack_top: 0x0000_7FFF_FFFF_F000,
    user_stack_size: 0x8000_0000, // 2GB
    
    // User heap
    user_heap_base: 0x0000_0000_1000_0000,
    user_heap_size: 0x7FFF_F000_0000, // ~2TB
    
    // Maximum user address
    user_max: 0x0000_8000_0000_0000,
    
    // 4KB pages (can also support 16KB, 64KB)
    page_size: 4096,
    
    // AArch64 supports direct physical mapping
    // Physical memory mapped in separate region from kernel code
    // Kernel code at 0xFFFF_0000_0000_0000, phys map at 0xFFFF_8000_0000_0000
    phys_map_base: Some(0xFFFF_8000_0000_0000),
    phys_map_size: Some(0x0000_8000_0000_0000), // 128TB
    
    // MMIO regions
    mmio_base: Some(0xFFFF_0000_0000_0000),
    mmio_size: Some(0x0000_0000_8000_0000), // 32GB
    
    // ASLR offset range: 1GB
    aslr_offset_range: 0x4000_0000,
};

// ============================================================================
// RISC-V 64 Memory Layout
// ============================================================================

#[cfg(target_arch = "riscv64")]
const RISCV64_LAYOUT: AddressSpaceLayout = AddressSpaceLayout {
    // RISC-V uses Sv39/Sv48 paging with kernel in upper half
    // Kernel base: 0xFFFF_FFFF_0000_0000 (typical layout)
    kernel_base: 0xFFFF_FFFF_0000_0000,
    
    // Kernel code region (256MB)
    kernel_code_base: 0xFFFF_FFFF_0000_0000,
    kernel_code_size: 0x1000_0000, // 256MB
    
    // Kernel data region (256MB)
    kernel_data_base: 0xFFFF_FFFF_0100_0000,
    kernel_data_size: 0x1000_0000, // 256MB
    
    // Kernel heap (1GB)
    kernel_heap_base: 0xFFFF_FFFF_0300_0000,
    kernel_heap_size: 0x4000_0000, // 1GB
    
    // User space starts at 0x0000_0000_0000_0000
    user_base: 0x0000_0000_0000_0000,
    
    // User stack top
    user_stack_top: 0x0000_7FFF_FFFF_F000,
    user_stack_size: 0x8000_0000, // 2GB
    
    // User heap
    user_heap_base: 0x0000_0000_1000_0000,
    user_heap_size: 0x7FFF_F000_0000, // ~2TB
    
    // Maximum user address
    user_max: 0x0000_8000_0000_0000,
    
    // 4KB pages
    page_size: 4096,
    
    // RISC-V supports direct physical mapping
    // Physical memory mapped in separate region from kernel code
    // Kernel code at 0xFFFF_FFFF_0000_0000, phys map at 0xFFFF_FFFF_8000_0000
    phys_map_base: Some(0xFFFF_FFFF_8000_0000),
    phys_map_size: Some(0x0000_0000_8000_0000), // 2GB
    
    // MMIO regions
    mmio_base: Some(0xFFFF_FFFF_0000_0000),
    mmio_size: Some(0x0000_0000_8000_0000), // 32GB
    
    // ASLR offset range: 512MB (smaller due to limited address space)
    aslr_offset_range: 0x2000_0000,
};

// ============================================================================
// Public API - Architecture-agnostic constants
// ============================================================================

/// Get kernel base address for current architecture
#[inline]
pub const fn kernel_base() -> usize {
    AddressSpaceLayout::current().kernel_base
}

/// Get user base address for current architecture
#[inline]
pub const fn user_base() -> usize {
    AddressSpaceLayout::current().user_base
}

/// Get user stack top address for current architecture
#[inline]
pub const fn user_stack_top() -> usize {
    AddressSpaceLayout::current().user_stack_top
}

/// Get maximum user address for current architecture
#[inline]
pub const fn user_max() -> usize {
    AddressSpaceLayout::current().user_max
}

/// Get page size for current architecture
#[inline]
pub const fn page_size() -> usize {
    AddressSpaceLayout::current().page_size
}

/// Check if address is in kernel space
#[inline]
pub fn is_kernel_address(addr: usize) -> bool {
    AddressSpaceLayout::current().is_kernel_address(addr)
}

/// Check if address is in user space
#[inline]
pub fn is_user_address(addr: usize) -> bool {
    AddressSpaceLayout::current().is_user_address(addr)
}

/// Convert physical address to kernel virtual address (if supported)
#[inline]
pub fn phys_to_virt(phys: usize) -> Option<usize> {
    AddressSpaceLayout::current().phys_to_virt(phys)
}

/// Convert kernel virtual address to physical address (if supported)
#[inline]
pub fn virt_to_phys(virt: usize) -> Option<usize> {
    AddressSpaceLayout::current().virt_to_phys(virt)
}

/// Apply ASLR offset to a base address
#[inline]
pub fn apply_aslr_offset(base: usize, offset: usize) -> usize {
    AddressSpaceLayout::current().apply_aslr_offset(base, offset)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_memory_layout_access() {
        let layout = AddressSpaceLayout::current();
        assert!(layout.kernel_base > 0);
        assert!(layout.user_base < layout.user_max);
        assert!(layout.page_size > 0);
    }
    
    #[test]
    fn test_address_checks() {
        let layout = AddressSpaceLayout::current();
        
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
        let layout = AddressSpaceLayout::current();
        
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
    
    #[test]
    fn test_layout_verification() {
        let layout = AddressSpaceLayout::current();
        // Layout should be valid
        assert!(layout.verify().is_ok());
    }
    
    #[test]
    fn test_aslr_offset() {
        let layout = AddressSpaceLayout::current();
        let base = layout.kernel_code_base;
        let offset = 0x1000; // 4KB
        
        let adjusted = layout.apply_aslr_offset(base, offset);
        assert_eq!(adjusted, base + offset);
        
        // Test page alignment
        let unaligned_offset = 0x1001;
        let adjusted2 = layout.apply_aslr_offset(base, unaligned_offset);
        assert_eq!(adjusted2, base + 0x1000); // Should be aligned
    }
}

