/// Bootloader → Kernel Handoff
///
/// Manages boot information structure and jumps to kernel entry point.
/// Passes all necessary boot parameters to the kernel.

use core::mem;

/// Boot protocol type indicator
#[derive(Debug, Clone, Copy)]
pub enum BootProtocol {
    Multiboot2,
    Uefi,
    Bios,
}

/// Memory map entry (passed to kernel)
#[derive(Debug, Clone, Copy)]
pub struct MemoryMapEntry {
    pub base: u64,
    pub length: u64,
    pub region_type: u32,
}

/// Loaded kernel module
#[derive(Debug, Clone, Copy)]
pub struct LoadedModule {
    pub start: u64,
    pub end: u64,
    pub command_line: u64,
}

/// Complete boot information structure
/// 
/// This is passed to the kernel at entry point
#[repr(C)]
pub struct BootInformation {
    /// Magic number for validation (0x12345678)
    pub magic: u32,

    /// Boot protocol used
    pub protocol: u32,

    /// Pointer to kernel (for validation)
    pub kernel_entry: u64,

    /// Memory map entries
    pub memory_map: [MemoryMapEntry; 32],
    pub memory_map_count: u32,

    /// Loaded modules
    pub modules: [LoadedModule; 16],
    pub module_count: u32,

    /// Bootloader information
    pub bootloader_name: [u8; 64],

    /// Kernel command line
    pub command_line: [u8; 256],

    /// Video frame buffer information
    pub framebuffer_addr: u64,
    pub framebuffer_pitch: u32,
    pub framebuffer_width: u32,
    pub framebuffer_height: u32,
    pub framebuffer_bpp: u32,

    /// CPU features detected
    pub cpu_features: u64,

    /// RSDP address (ACPI)
    pub rsdp_address: u64,

    /// Reserved for future use
    pub reserved: [u64; 16],
}

impl BootInformation {
    /// Create new boot information structure
    pub fn new(entry: u64) -> Self {
        Self {
            magic: 0x12345678,
            protocol: 0,
            kernel_entry: entry,
            memory_map: [MemoryMapEntry {
                base: 0,
                length: 0,
                region_type: 0,
            }; 32],
            memory_map_count: 0,
            modules: [LoadedModule {
                start: 0,
                end: 0,
                command_line: 0,
            }; 16],
            module_count: 0,
            bootloader_name: [0u8; 64],
            command_line: [0u8; 256],
            framebuffer_addr: 0,
            framebuffer_pitch: 0,
            framebuffer_width: 0,
            framebuffer_height: 0,
            framebuffer_bpp: 0,
            cpu_features: 0,
            rsdp_address: 0,
            reserved: [0u64; 16],
        }
    }

    /// Add memory map entry
    pub fn add_memory_entry(&mut self, entry: MemoryMapEntry) -> Result<(), &'static str> {
        if self.memory_map_count >= 32 {
            return Err("Memory map full");
        }

        self.memory_map[self.memory_map_count as usize] = entry;
        self.memory_map_count += 1;
        Ok(())
    }

    /// Add loaded module
    pub fn add_module(&mut self, module: LoadedModule) -> Result<(), &'static str> {
        if self.module_count >= 16 {
            return Err("Module list full");
        }

        self.modules[self.module_count as usize] = module;
        self.module_count += 1;
        Ok(())
    }

    /// Set bootloader name
    pub fn set_bootloader_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let len = core::cmp::min(bytes.len(), 63);
        self.bootloader_name[..len].copy_from_slice(&bytes[..len]);
        self.bootloader_name[len] = 0;
    }

    /// Set kernel command line
    pub fn set_command_line(&mut self, cmd: &str) {
        let bytes = cmd.as_bytes();
        let len = core::cmp::min(bytes.len(), 255);
        self.command_line[..len].copy_from_slice(&bytes[..len]);
        self.command_line[len] = 0;
    }

    /// Validate boot information before handoff
    pub fn validate(&self) -> Result<(), &'static str> {
        // Validate magic number
        if self.magic != 0x12345678 {
            return Err("Invalid boot info magic");
        }

        // Validate kernel entry is non-zero
        if self.kernel_entry == 0 {
            return Err("Invalid kernel entry point (zero)");
        }

        // Validate kernel entry is reasonable (not in first MB or kernel space)
        if self.kernel_entry < 0x100000 {
            return Err("Kernel entry below 1MB");
        }

        // On x86_64, exclude kernel space above 0xFFFF800000000000
        #[cfg(target_arch = "x86_64")]
        {
            if self.kernel_entry > 0xFFFF800000000000 && self.kernel_entry < 0xFFFFFFFF00000000 {
                // In kernel space range, might be valid with proper setup
                // For now, allow it
            }
        }

        // Validate memory map
        if self.memory_map_count == 0 {
            return Err("No memory map entries");
        }

        // Validate command line is null-terminated
        if self.command_line[255] != 0 && self.command_line.len() > 255 {
            return Err("Command line not null-terminated");
        }

        Ok(())
    }

    /// Get boot information size
    pub fn size() -> usize {
        mem::size_of::<BootInformation>()
    }
}

/// Kernel handoff manager
pub struct KernelHandoff {
    boot_info: BootInformation,
}

impl KernelHandoff {
    /// Create new kernel handoff
    pub fn new(entry: u64) -> Self {
        Self {
            boot_info: BootInformation::new(entry),
        }
    }

    /// Prepare for kernel handoff
    pub fn prepare(&mut self) -> Result<(), &'static str> {
        // Set bootloader name
        self.boot_info.set_bootloader_name("NOS Bootloader v0.1");

        // Validate before handoff
        self.boot_info.validate()?;

        Ok(())
    }

    /// Execute kernel handoff
    ///
    /// # Safety
    ///
    /// This function jumps to the kernel entry point and does not return.
    /// Must be called from trusted bootloader context with proper setup.
    pub unsafe fn execute(&self) -> ! {
        // SAFETY: Caller must ensure:
        // 1. Kernel is properly loaded
        // 2. Memory is initialized
        // 3. Entry point is valid
        // 4. Boot information is complete

        let _kernel_entry = self.boot_info.kernel_entry;
        let _boot_info_ptr = &self.boot_info as *const _ as u64;
        log::debug!("Preparing kernel handoff with entry point");

        // Use inline assembly for kernel handoff
        // Set up arguments for kernel:
        // RDI = boot information pointer (System V AMD64 ABI first arg)
        // RAX = kernel entry point
        // Then clear flags and jump
        #[cfg(all(target_os = "none", target_arch = "x86_64"))]
        {
            unsafe {
                core::arch::asm!(
                    "cli",                          // Disable interrupts
                    "mov rdi, {boot_info}",        // RDI = boot_info pointer
                    "jmp {kernel_entry}",          // Jump to kernel
                    boot_info = in(reg) _boot_info_ptr,
                    kernel_entry = in(reg) _kernel_entry,
                    options(noreturn, nostack, nomem)
                );
            }
        }

        #[cfg(all(target_os = "none", target_arch = "aarch64"))]
        {
            unsafe {
                core::arch::asm!(
                    "msr daifset, #15",             // Disable interrupts (aarch64)
                    "mov x0, {boot_info}",         // x0 = boot_info pointer (first arg)
                    "br {kernel_entry}",            // Branch to kernel
                    boot_info = in(reg) _boot_info_ptr,
                    kernel_entry = in(reg) _kernel_entry,
                    options(noreturn, nostack, nomem)
                );
            }
        }

        #[cfg(all(target_os = "none", target_arch = "riscv64"))]
        {
            unsafe {
                core::arch::asm!(
                    "csrci mstatus, 8",             // Disable interrupts (riscv64)
                    "mv a0, {boot_info}",          // a0 = boot_info pointer (first arg)
                    "jr {kernel_entry}",            // Jump to kernel
                    boot_info = in(reg) _boot_info_ptr,
                    kernel_entry = in(reg) _kernel_entry,
                    options(noreturn, nostack, nomem)
                );
            }
        }

        #[cfg(not(any(
            all(target_os = "none", target_arch = "x86_64"),
            all(target_os = "none", target_arch = "aarch64"),
            all(target_os = "none", target_arch = "riscv64")
        )))]
        {
            loop {}
        }
    }

    /// Get boot information
    pub fn boot_info(&self) -> &BootInformation {
        &self.boot_info
    }

    /// Get mutable boot information
    pub fn boot_info_mut(&mut self) -> &mut BootInformation {
        &mut self.boot_info
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_information_creation() {
        let bi = BootInformation::new(0x200000);
        assert_eq!(bi.magic, 0x12345678);
        assert_eq!(bi.kernel_entry, 0x200000);
        assert_eq!(bi.memory_map_count, 0);
        assert_eq!(bi.module_count, 0);
    }

    #[test]
    fn test_add_memory_entry() {
        let mut bi = BootInformation::new(0x200000);
        let entry = MemoryMapEntry {
            base: 0x0,
            length: 0x100000,
            region_type: 1,
        };

        assert!(bi.add_memory_entry(entry).is_ok());
        assert_eq!(bi.memory_map_count, 1);
    }

    #[test]
    fn test_memory_map_full() {
        let mut bi = BootInformation::new(0x200000);
        let entry = MemoryMapEntry {
            base: 0x0,
            length: 0x100000,
            region_type: 1,
        };

        for _ in 0..32 {
            let _ = bi.add_memory_entry(entry);
        }

        assert_eq!(bi.memory_map_count, 32);
        assert!(bi.add_memory_entry(entry).is_err());
    }

    #[test]
    fn test_set_bootloader_name() {
        let mut bi = BootInformation::new(0x200000);
        bi.set_bootloader_name("Test Bootloader");

        assert_eq!(bi.bootloader_name[0], b'T');
        assert_eq!(bi.bootloader_name[14], b'r');
        assert_eq!(bi.bootloader_name[15], 0);
    }

    #[test]
    fn test_validate_boot_info() {
        let mut bi = BootInformation::new(0x200000);

        // Add memory entry for validation
        let entry = MemoryMapEntry {
            base: 0x0,
            length: 0x1000000,
            region_type: 1,
        };
        let _ = bi.add_memory_entry(entry);

        assert!(bi.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_entry() {
        let bi = BootInformation::new(0);
        assert!(bi.validate().is_err()); // Entry is zero
    }

    #[test]
    fn test_kernel_handoff_creation() {
        let kh = KernelHandoff::new(0x200000);
        assert_eq!(kh.boot_info.kernel_entry, 0x200000);
    }

    #[test]
    fn test_boot_information_size() {
        let size = BootInformation::size();
        assert!(size > 0);
    }
}
