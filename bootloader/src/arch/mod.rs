//! Architecture abstraction layer
//!
//! This module provides architecture-specific implementations for different
//! CPU architectures, allowing the bootloader to support x86_64, AArch64,
//! and RISC-V with a unified interface.

use crate::utils::error::{BootError, Result};
use core::arch::asm;
use crate::println;

// Architecture-specific modules
#[cfg(target_arch = "x86_64")]
pub mod x86_64;
#[cfg(target_arch = "aarch64")]
pub mod aarch64;
#[cfg(target_arch = "riscv64")]
pub mod riscv64;

/// Architecture types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Architecture {
    X86_64,
    AArch64,
    RiscV64,
}

impl Architecture {
    /// Get the current architecture at compile time
    pub fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        return Architecture::X86_64;

        #[cfg(target_arch = "aarch64")]
        return Architecture::AArch64;

        #[cfg(target_arch = "riscv64")]
        return Architecture::RiscV64;

        #[allow(unreachable_code)]
        {
            panic!("Unsupported target architecture");
        }
    }

    /// Get architecture name
    pub fn name(self) -> &'static str {
        match self {
            Architecture::X86_64 => "x86_64",
            Architecture::AArch64 => "AArch64",
            Architecture::RiscV64 => "RISC-V 64",
        }
    }

    /// Get page size for this architecture
    pub fn page_size(self) -> usize {
        match self {
            Architecture::X86_64 => 4096,
            Architecture::AArch64 => 4096,
            Architecture::RiscV64 => 4096,
        }
    }

    /// Get default kernel base address for this architecture
    pub fn default_kernel_base(self) -> usize {
        match self {
            Architecture::X86_64 => 0x100000,      // 1MB
            Architecture::AArch64 => 0x40000000,   // 1GB
            Architecture::RiscV64 => 0x80000000,   // 2GB
        }
    }

    /// Get default stack size for this architecture
    pub fn default_stack_size(self) -> usize {
        match self {
            Architecture::X86_64 => 64 * 1024,   // 64KB
            Architecture::AArch64 => 64 * 1024,   // 64KB
            Architecture::RiscV64 => 64 * 1024,   // 64KB
        }
    }

    /// Get architecture-specific CPU features
    pub fn cpu_features(self) -> u64 {
        match self {
            #[cfg(target_arch = "x86_64")]
            Architecture::X86_64 => x86_64::get_cpu_features(),

            #[cfg(target_arch = "aarch64")]
            Architecture::AArch64 => aarch64::get_cpu_features(),

            #[cfg(target_arch = "riscv64")]
            Architecture::RiscV64 => riscv64::get_cpu_features(),

            #[allow(unreachable_patterns)]
            _ => 0,
        }
    }
}

/// CPU features for each architecture
#[derive(Debug, Clone, Default)]
pub struct CpuFeatures {
    /// Whether CPU supports virtualization extensions
    pub has_virtualization: bool,
    /// Whether CPU supports 64-bit operations (always true for our targets)
    pub is_64bit: bool,
    /// Cache line size in bytes
    pub cache_line_size: usize,
    /// Number of CPU cores
    pub cpu_count: usize,
    /// Architecture-specific flags
    pub arch_flags: u64,
}

/// Memory layout information for an architecture
#[derive(Debug, Clone)]
pub struct MemoryLayout {
    /// Kernel base address
    pub kernel_base: usize,
    /// Kernel size limit
    pub kernel_size_limit: usize,
    /// Stack base address (top)
    pub stack_base: usize,
    /// Stack size
    pub stack_size: usize,
    /// Heap base address
    pub heap_base: usize,
    /// Heap size
    pub heap_size: usize,
    /// Page table base address
    pub page_table_base: usize,
}

impl MemoryLayout {
    /// Create memory layout for the given architecture
    pub fn for_architecture(arch: Architecture, kernel_size: usize) -> Self {
        let kernel_base = arch.default_kernel_base();
        let stack_size = arch.default_stack_size();

        // Simple layout calculation - this would be more sophisticated in a real implementation
        let kernel_end = align_up(kernel_base + kernel_size, arch.page_size());
        let stack_base = kernel_end + stack_size;
        let heap_base = stack_base + stack_size;
        let heap_size = 512 * 1024; // 512KB heap

        Self {
            kernel_base,
            kernel_size_limit: kernel_size,
            stack_base,
            stack_size,
            heap_base,
            heap_size,
            page_table_base: heap_base + heap_size,
        }
    }

    /// Validate the memory layout
    pub fn validate(&self) -> Result<()> {
        // Check for reasonable addresses
        if self.kernel_base == 0 {
            return Err(BootError::InvalidBootConfig);
        }

        if self.stack_size < 4096 {
            return Err(BootError::InvalidBootConfig);
        }

        if self.heap_size < 4096 {
            return Err(BootError::InvalidBootConfig);
        }

        // Check for overlaps (basic check)
        let kernel_end = self.kernel_base + self.kernel_size_limit;
        let stack_end = self.stack_base + self.stack_size;

        if kernel_end > self.stack_base {
            return Err(BootError::MemoryMapError);
        }

        if stack_end > self.heap_base {
            return Err(BootError::MemoryMapError);
        }

        Ok(())
    }
}

// Re-export unified boot parameters from nos-api
pub use nos_api::boot::BootParameters;

// Helper function to create boot parameters from boot info
pub fn create_boot_parameters(
    boot_info: &crate::protocol::BootInfo,
    kernel_image: &crate::protocol::KernelImage,
) -> BootParameters {
    let mut params = BootParameters::new();
    
    // Set architecture
    params.architecture = match boot_info.protocol_type {
        crate::protocol::BootProtocolType::Uefi => 0, // x86_64
        crate::protocol::BootProtocolType::Bios => 0,  // x86_64
        crate::protocol::BootProtocolType::Multiboot2 => 0, // x86_64
    };
    
    // Set boot protocol
    params.boot_protocol = match boot_info.protocol_type {
        crate::protocol::BootProtocolType::Uefi => nos_api::boot::BootProtocolType::UEFI as u32,
        crate::protocol::BootProtocolType::Bios => nos_api::boot::BootProtocolType::BIOS as u32,
        crate::protocol::BootProtocolType::Multiboot2 => nos_api::boot::BootProtocolType::Multiboot2 as u32,
    };
    
    // Set memory map (will be filled in by architecture-specific code)
    params.memory_map = nos_api::boot::MemoryMap {
        entry_count: 0,
        entries: 0,
    };
    
    // Set framebuffer if available
    if let Some(fb) = &boot_info.framebuffer {
        params.framebuffer = nos_api::boot::FramebufferInfo {
            address: fb.address as u64,
            width: fb.width,
            height: fb.height,
            bytes_per_pixel: fb.bpp,
            stride: fb.pitch,
            pixel_format: 0, // RGB
        };
    }
    
    // Set ACPI RSDP
    params.acpi_rsdp = boot_info.acpi_rsdp.unwrap_or(0);
    
    // Set device tree
    params.device_tree = boot_info.device_tree.unwrap_or(0);
    
    // Set command line (will be filled in by architecture-specific code)
    params.command_line = 0;
    
    // Set timestamp
    params.timestamp = boot_info.boot_timestamp;
    
    params
}

/// Early architecture-specific initialization
pub fn early_init() {
    // Disable interrupts during early initialization
    interrupt_disable();

    // Perform architecture-specific early initialization
    let arch = Architecture::current();

    match arch {
        #[cfg(target_arch = "x86_64")]
        Architecture::X86_64 => x86_64::early_init(),

        #[cfg(target_arch = "aarch64")]
        Architecture::AArch64 => aarch64::early_init(),

        #[cfg(target_arch = "riscv64")]
        Architecture::RiscV64 => riscv64::early_init(),

        #[allow(unreachable_patterns)]
        _ => panic!("Unsupported architecture"),
    }

    println!("Early architecture initialization completed for {}", arch.name());
}

/// Disable interrupts
pub fn interrupt_disable() {
    match Architecture::current() {
        #[cfg(target_arch = "x86_64")]
        Architecture::X86_64 => x86_64::interrupt_disable(),

        #[cfg(target_arch = "aarch64")]
        Architecture::AArch64 => aarch64::interrupt_disable(),

        #[cfg(target_arch = "riscv64")]
        Architecture::RiscV64 => riscv64::interrupt_disable(),

        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Enable interrupts
pub fn interrupt_enable() {
    match Architecture::current() {
        #[cfg(target_arch = "x86_64")]
        Architecture::X86_64 => x86_64::interrupt_enable(),

        #[cfg(target_arch = "aarch64")]
        Architecture::AArch64 => aarch64::interrupt_enable(),

        #[cfg(target_arch = "riscv64")]
        Architecture::RiscV64 => riscv64::interrupt_enable(),

        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Check if interrupts are enabled
pub fn interrupt_enabled() -> bool {
    match Architecture::current() {
        #[cfg(target_arch = "x86_64")]
        Architecture::X86_64 => x86_64::interrupt_enabled(),

        #[cfg(target_arch = "aarch64")]
        Architecture::AArch64 => aarch64::interrupt_enabled(),

        #[cfg(target_arch = "riscv64")]
        Architecture::RiscV64 => riscv64::interrupt_enabled(),

        #[allow(unreachable_patterns)]
        _ => false,
    }
}

/// Wait for interrupt (low-power idle)
pub fn wait_for_interrupt() {
    match Architecture::current() {
        #[cfg(target_arch = "x86_64")]
        Architecture::X86_64 => x86_64::wait_for_interrupt(),

        #[cfg(target_arch = "aarch64")]
        Architecture::AArch64 => aarch64::wait_for_interrupt(),

        #[cfg(target_arch = "riscv64")]
        Architecture::RiscV64 => riscv64::wait_for_interrupt(),

        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Memory barrier
pub fn memory_barrier() {
    match Architecture::current() {
        #[cfg(target_arch = "x86_64")]
        Architecture::X86_64 => x86_64::memory_barrier(),

        #[cfg(target_arch = "aarch64")]
        Architecture::AArch64 => aarch64::memory_barrier(),

        #[cfg(target_arch = "riscv64")]
        Architecture::RiscV64 => riscv64::memory_barrier(),

        #[allow(unreachable_patterns)]
        _ => {}
    }
}

/// Get current CPU ID
pub fn cpu_id() -> usize {
    match Architecture::current() {
        #[cfg(target_arch = "x86_64")]
        Architecture::X86_64 => x86_64::cpu_id(),

        #[cfg(target_arch = "aarch64")]
        Architecture::AArch64 => aarch64::cpu_id(),

        #[cfg(target_arch = "riscv64")]
        Architecture::RiscV64 => riscv64::cpu_id(),

        #[allow(unreachable_patterns)]
        _ => 0,
    }
}

/// Jump to kernel entry point
///
/// # Safety
///
/// This function performs an unsafe transition to the kernel and should
/// only be called after all bootloader setup is complete.
pub unsafe fn jump_to_kernel(entry_point: usize, boot_params: &BootParameters) -> ! {
    // Final memory barrier before jumping to kernel
    memory_barrier();

    // Disable interrupts before kernel transition
    interrupt_disable();

    match Architecture::current() {
        #[cfg(target_arch = "x86_64")]
        Architecture::X86_64 => x86_64::jump_to_kernel(entry_point),

        #[cfg(target_arch = "aarch64")]
        Architecture::AArch64 => aarch64::jump_to_kernel(entry_point),

        #[cfg(target_arch = "riscv64")]
        Architecture::RiscV64 => riscv64::jump_to_kernel(entry_point),

        #[allow(unreachable_patterns)]
        _ => panic!("Unsupported architecture"),
    }
}

/// Reboot the system
pub fn reboot() -> ! {
    println!("Rebooting system...");

    match Architecture::current() {
        #[cfg(target_arch = "x86_64")]
        Architecture::X86_64 => x86_64::reboot(),

        #[cfg(target_arch = "aarch64")]
        Architecture::AArch64 => aarch64::reboot(),

        #[cfg(target_arch = "riscv64")]
        Architecture::RiscV64 => riscv64::reboot(),

        #[allow(unreachable_patterns)]
        _ => halt(),
    }
}

/// Shutdown the system
pub fn shutdown() -> ! {
    println!("Shutting down system...");

    match Architecture::current() {
        #[cfg(target_arch = "x86_64")]
        Architecture::X86_64 => x86_64::shutdown(),

        #[cfg(target_arch = "aarch64")]
        Architecture::AArch64 => aarch64::shutdown(),

        #[cfg(target_arch = "riscv64")]
        Architecture::RiscV64 => riscv64::shutdown(),

        #[allow(unreachable_patterns)]
        _ => halt(),
    }
}

/// Halt the system
pub fn halt() -> ! {
    interrupt_disable();
    println!("System halted.");

    loop {
        wait_for_interrupt();
    }
}

/// Utility function to align a value up to a boundary
pub fn align_up(value: usize, alignment: usize) -> usize {
    (value + alignment - 1) & !(alignment - 1)
}

/// Utility function to align a value down to a boundary
pub fn align_down(value: usize, alignment: usize) -> usize {
    value & !(alignment - 1)
}

/// Utility function to check if a value is aligned
pub fn is_aligned(value: usize, alignment: usize) -> bool {
    (value & (alignment - 1)) == 0
}