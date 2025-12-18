// ELF kernel loader integration with boot protocols

use crate::kernel_if::elf_loader_v2::load_elf_kernel;

pub struct KernelLoader {
    kernel_data: &'static [u8],
}

impl KernelLoader {
    pub fn new(kernel_data: &'static [u8]) -> Self {
        Self { kernel_data }
    }

    /// Load kernel from buffer
    pub fn load_kernel(&self) -> Result<KernelLoadInfo, &'static str> {
        crate::drivers::console::write_str("Loading kernel from buffer\n");

        // Load ELF kernel
        let (entry_point, image_size) = load_elf_kernel(self.kernel_data)?;

        Ok(KernelLoadInfo {
            entry_point,
            image_size,
            base_address: 0x100000,
        })
    }

    /// Load kernel from UEFI filesystem
    pub fn load_kernel_from_uefi(
        &self,
        _path: &[u8],
    ) -> Result<KernelLoadInfo, &'static str> {
        crate::drivers::console::write_str("Loading kernel from UEFI filesystem\n");

        // In real implementation, would load from EFI partition
        // For now, use embedded kernel data
        self.load_kernel()
    }

    /// Load kernel from BIOS disk
    pub fn load_kernel_from_bios(
        &self,
        _drive: u8,
        _sector: u32,
    ) -> Result<KernelLoadInfo, &'static str> {
        crate::drivers::console::write_str("Loading kernel from BIOS disk\n");

        // In real implementation, would use INT 13h for disk I/O
        // For now, use embedded kernel data
        self.load_kernel()
    }

    }

pub struct KernelLoadInfo {
    pub entry_point: u64,
    pub image_size: u64,
    pub base_address: u64,
}
pub fn load_kernel_from_boot_protocol(
    protocol: BootProtocol,
) -> Result<KernelLoadInfo, &'static str> {
    match protocol {
        BootProtocol::Uefi => {
            crate::drivers::console::write_str("Loading via UEFI\n");
            // Would use UEFI boot services to load
            Err("UEFI kernel loading not yet implemented")
        }
        BootProtocol::Multiboot2 => {
            crate::drivers::console::write_str("Loading via Multiboot2\n");
            // Would parse multiboot2 info for kernel location
            Err("Multiboot2 kernel loading not yet implemented")
        }
        BootProtocol::Bios => {
            crate::drivers::console::write_str("Loading via BIOS\n");
            // Would use BIOS disk I/O
            Err("BIOS kernel loading not yet implemented")
        }
    }
}

pub enum BootProtocol {
    Uefi,
    Multiboot2,
    Bios,
}

/// Validate loaded kernel
pub fn validate_kernel(info: &KernelLoadInfo) -> Result<(), &'static str> {
    if info.entry_point == 0 {
        return Err("Invalid kernel entry point");
    }

    if info.image_size == 0 {
        return Err("Invalid kernel image size");
    }

    crate::drivers::console::write_str("Kernel validation passed\n");
    Ok(())
}

/// Setup kernel execution environment
pub fn prepare_kernel_execution(
    info: &KernelLoadInfo,
) -> Result<(), &'static str> {
    // Validate kernel
    validate_kernel(info)?;

    // Setup memory for kernel
    crate::drivers::console::write_str("Preparing kernel execution environment\n");

    // Setup stack for kernel (would be done before jump)
    // Setup GDT/IDT for kernel
    // Setup paging if needed

    Ok(())
}

/// Jump to kernel entry point
pub unsafe fn jump_to_kernel(entry_point: u64) -> ! {
    crate::drivers::console::write_str("Jumping to kernel\n");

    #[cfg(target_arch = "x86_64")]
    {
        core::arch::asm!("jmp {}", in(reg) entry_point);
    }

    #[cfg(target_arch = "aarch64")]
    {
        core::arch::asm!("br {}", in(reg) entry_point);
    }

    #[cfg(target_arch = "riscv64")]
    {
        core::arch::asm!("jr {}", in(reg) entry_point);
    }

    loop {}
}
