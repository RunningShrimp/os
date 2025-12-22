//! x86_64 Architecture Support
//!
//! This module provides architecture-specific functionality for x86_64 systems,
//! including CPU detection, memory management, and low-level hardware access.

use crate::utils::error::{BootError, Result};

pub mod cpuid;
pub mod gdt;
pub mod idt;
pub mod long_mode;
pub mod msr;

#[cfg(feature = "bios_support")]
pub mod bios;

#[cfg(feature = "uefi_support")]
pub mod uefi;

/// x86_64-specific constants
pub mod consts {
    /// Page size in bytes
    pub const PAGE_SIZE: usize = 4096;
    /// Page shift (log2 of page size)
    pub const PAGE_SHIFT: usize = 12;
    /// Page mask
    pub const PAGE_MASK: usize = PAGE_SIZE - 1;

    /// Kernel base address (typical)
    pub const KERNEL_BASE: usize = 0xFFFF800000000000;
    /// Kernel load address (physical)
    pub const KERNEL_LOAD_ADDR: usize = 0x100000;
    /// Bootloader base address
    pub const BOOTLOADER_BASE: usize = 0x10000;

    /// VGA text mode buffer
    pub const VGA_BUFFER: usize = 0xB8000;
    /// VGA text mode dimensions
    pub const VGA_WIDTH: usize = 80;
    pub const VGA_HEIGHT: usize = 25;

    /// BIOS Data Area
    pub const BIOS_DATA_AREA: usize = 0x400;
    /// EBDA (Extended BIOS Data Area)
    pub const EBDA_BASE: usize = 0x9FC00;

    /// Port addresses
    pub mod ports {
        /// Keyboard controller status
        pub const KB_STATUS: u16 = 0x64;
        /// Keyboard controller data
        pub const KB_DATA: u16 = 0x60;
        /// Fast A20 gate
        pub const FAST_A20: u16 = 0x92;
        /// COM1 serial port
        pub const COM1: u16 = 0x3F8;
    }

    /// Interrupt vectors
    pub mod interrupts {
        /// BIOS video interrupt
        pub const INT10: u8 = 0x10;
        /// BIOS disk services
        pub const INT13: u8 = 0x13;
        /// BIOS system services
        pub const INT15: u8 = 0x15;
        /// BIOS keyboard services
        pub const INT16: u8 = 0x16;
        /// BIOS time services
        pub const INT1A: u8 = 0x1A;
    }

    /// CPUID functions
    pub mod cpuid {
        /// Highest basic function
        pub const HIGHEST_BASIC: u32 = 0x0;
        /// Processor info and feature bits
        pub const PROCESSOR_INFO: u32 = 0x1;
        /// Cache and TLB info
        pub const CACHE_INFO: u32 = 0x2;
        /// Serial number
        pub const SERIAL_NUMBER: u32 = 0x3;
        /// Highest extended function
        pub const HIGHEST_EXTENDED: u32 = 0x80000000;
        /// Extended processor info and feature bits
        pub const EXT_PROCESSOR_INFO: u32 = 0x80000001;
        /// Processor brand string
        pub const BRAND_STRING_START: u32 = 0x80000002;
        pub const BRAND_STRING_END: u32 = 0x80000004;
    }

    /// MSRs (Model Specific Registers)
    pub mod msr {
        /// IA32_EFER (Extended Feature Enable Register)
        pub const IA32_EFER: u32 = 0xC0000080;
        /// IA32_LSTAR (Long Mode SYSCALL Target)
        pub const IA32_LSTAR: u32 = 0xC0000082;
        /// IA32_STAR (System Call Target Address)
        pub const IA32_STAR: u32 = 0xC0000081;
        /// IA32_FMASK (System Call Flag Mask)
        pub const IA32_FMASK: u32 = 0xC0000084;
    }

    /// CR0 control register bits
    pub mod cr0 {
        /// Protection Enable
        pub const PE: u64 = 1 << 0;
        /// Paging Enable
        pub const PG: u64 = 1 << 31;
        /// Cache Disable
        pub const CD: u64 = 1 << 30;
        /// Not Write-through
        pub const NW: u64 = 1 << 29;
        /// Alignment Mask
        pub const AM: u64 = 1 << 18;
        /// Write Protect
        pub const WP: u64 = 1 << 16;
        /// Numeric Error
        pub const NE: u64 = 1 << 5;
        /// Extension Type
        pub const ET: u64 = 1 << 4;
        /// Task Switched
        pub const TS: u64 = 1 << 3;
        /// Emulation
        pub const EM: u64 = 1 << 2;
        /// Monitor Coprocessor
        pub const MP: u64 = 1 << 1;
    }

    /// CR4 control register bits
    pub mod cr4 {
        /// Virtual Mode Extensions
        pub const VME: u64 = 1 << 0;
        /// Protected-mode Virtual Interrupts
        pub const PVI: u64 = 1 << 1;
        /// Time Stamp Disable
        pub const TSD: u64 = 1 << 2;
        /// Debugging Extensions
        pub const DE: u64 = 1 << 3;
        /// Page Size Extension
        pub const PSE: u64 = 1 << 4;
        /// Physical Address Extension
        pub const PAE: u64 = 1 << 5;
        /// Machine Check Enable
        pub const MCE: u64 = 1 << 6;
        /// Page Global Enable
        pub const PGE: u64 = 1 << 7;
        /// Performance-Monitoring Counter Enable
        pub const PCE: u64 = 1 << 8;
        /// OSFXSR (FXSAVE/FXRSTOR support)
        pub const OSFXSR: u64 = 1 << 9;
        /// OSXMMEXCPT (SIMD floating-point exception)
        pub const OSXMMEXCPT: u64 = 1 << 10;
        /// User-Mode Instruction Prevention
        pub const UMIP: u64 = 1 << 11;
        /// VMX Enable
        pub const VMXE: u64 = 1 << 13;
        /// SMX Enable
        pub const SMXE: u64 = 1 << 14;
        /// FSGSBASE Enable
        pub const FSGSBASE: u64 = 1 << 16;
        /// PCID Enable
        pub const PCIDE: u64 = 1 << 17;
        /// OS XSAVE Enable
        pub const OSXSAVE: u64 = 1 << 18;
        /// SMEP Enable
        pub const SMEP: u64 = 1 << 20;
        /// SMAP Enable
        pub const SMAP: u64 = 1 << 21;
        /// Protection Key Enable
        pub const PKE: u64 = 1 << 22;
    }

    /// EFER bits
    pub mod efer {
        /// SYSCALL Enable
        pub const SCE: u64 = 1 << 0;
        /// Long Mode Enable
        pub const LME: u64 = 1 << 8;
        /// Long Mode Active
        pub const LMA: u64 = 1 << 10;
        /// NX Enable
        pub const NXE: u64 = 1 << 11;
        /// System Call Extensions
        pub const SVME: u64 = 1 << 12;
        /// Long Mode Segment Limit Enable
        pub const LMSLE: u64 = 1 << 13;
        /// Fast FXSAVE/FXRSTOR
        pub const FFXSR: u64 = 1 << 14;
        /// Translation Cache Extension
        pub const TCE: u64 = 1 << 15;
    }
}

/// x86_64 CPU information
#[derive(Debug, Clone)]
pub struct X86_64CpuInfo {
    /// CPU vendor string
    pub vendor: [u8; 12],
    /// CPU model
    pub model: u8,
    /// CPU family
    pub family: u8,
    /// CPU stepping
    pub stepping: u8,
    /// CPU brand string
    pub brand: [u8; 48],
    /// Feature flags (EDX)
    pub features_edx: u32,
    /// Feature flags (ECX)
    pub features_ecx: u32,
    /// Extended feature flags (EDX)
    pub ext_features_edx: u32,
    /// Extended feature flags (ECX)
    pub ext_features_ecx: u32,
    /// Cache line size
    pub cache_line_size: u8,
    /// Number of logical processors
    pub logical_processors: u8,
    /// APIC ID
    pub apic_id: u8,
}

impl X86_64CpuInfo {
    /// Create a new CPU info structure
    pub fn new() -> Self {
        Self {
            vendor: [0; 12],
            model: 0,
            family: 0,
            stepping: 0,
            brand: [0; 48],
            features_edx: 0,
            features_ecx: 0,
            ext_features_edx: 0,
            ext_features_ecx: 0,
            cache_line_size: 0,
            logical_processors: 0,
            apic_id: 0,
        }
    }

    /// Detect CPU information
    pub fn detect(&mut self) -> Result<()> {
        // Check if CPUID is supported
        if !self.cpuid_supported() {
            return Err(BootError::HardwareError("CPUID not supported"));
        }

        // Get basic CPU info
        self.detect_basic_info()?;

        // Get extended CPU info
        self.detect_extended_info()?;

        // Get cache info
        self.detect_cache_info()?;

        Ok(())
    }

    /// Check if CPUID is supported
    fn cpuid_supported(&self) -> bool {
        let mut flags: u32;
        unsafe {
            core::arch::asm!(
                "pushfq",
                "pop {0}",
                out(reg) flags,
                out("rax") _,  // Tell the compiler that rax is clobbered
                out("rcx") _,  // Tell the compiler that rcx is clobbered
                out("rdx") _,  // Tell the compiler that rdx is clobbered
            );
        }

        let flags1 = flags;

        unsafe {
            core::arch::asm!(
                "pushfq",
                "pop {0}",
                out(reg) flags,
                out("rax") _,  // Tell the compiler that rax is clobbered
                out("rcx") _,  // Tell the compiler that rcx is clobbered
                out("rdx") _,  // Tell the compiler that rdx is clobbered
            );
        }

        // Flip ID bit
        flags ^= 1 << 21;

        unsafe {
            core::arch::asm!(
                "push {0}",
                "popfq",
                in(reg) flags,
                out("rax") _,  // Tell the compiler that rax is clobbered
                out("rcx") _,  // Tell the compiler that rcx is clobbered
                out("rdx") _,  // Tell the compiler that rdx is clobbered
            );
        }

        unsafe {
            core::arch::asm!(
                "pushfq",
                "pop {0}",
                out(reg) flags,
                out("rax") _,  // Tell the compiler that rax is clobbered
                out("rcx") _,  // Tell the compiler that rcx is clobbered
                out("rdx") _,  // Tell the compiler that rdx is clobbered
            );
        }

        flags == flags1
    }

    /// Detect basic CPU information
    fn detect_basic_info(&mut self) -> Result<()> {
        let mut eax: u32;
        let mut ebx: u32;
        let mut ecx: u32;
        let mut edx: u32;

        unsafe {
            core::arch::asm!(
                "cpuid",
                in("eax") consts::cpuid::PROCESSOR_INFO,
                out("eax") eax,
                out("ebx") ebx,
                out("ecx") ecx,
                out("edx") edx,
            );
        }

        // Extract basic info
        self.stepping = (eax & 0xF) as u8;
        self.model = ((eax >> 4) & 0xF) as u8;
        self.family = ((eax >> 8) & 0xF) as u8;
        self.apic_id = ((ebx >> 24) & 0xFF) as u8;

        // Store feature flags
        self.features_edx = edx;
        self.features_ecx = ecx;

        // Get vendor string
        let vendor_bytes = [
            (ebx & 0xFF) as u8, ((ebx >> 8) & 0xFF) as u8, ((ebx >> 16) & 0xFF) as u8, ((ebx >> 24) & 0xFF) as u8,
            (edx & 0xFF) as u8, ((edx >> 8) & 0xFF) as u8, ((edx >> 16) & 0xFF) as u8, ((edx >> 24) & 0xFF) as u8,
            (ecx & 0xFF) as u8, ((ecx >> 8) & 0xFF) as u8, ((ecx >> 16) & 0xFF) as u8, ((ecx >> 24) & 0xFF) as u8,
        ];
        self.vendor.copy_from_slice(&vendor_bytes);

        Ok(())
    }

    /// Detect extended CPU information
    fn detect_extended_info(&mut self) -> Result<()> {
        let mut eax: u32;

        unsafe {
            core::arch::asm!(
                "cpuid",
                in("eax") consts::cpuid::HIGHEST_EXTENDED,
                out("eax") eax,
                out("ebx") _,
                out("ecx") _,
                out("edx") _,
            );
        }

        if eax < consts::cpuid::EXT_PROCESSOR_INFO {
            return Err(BootError::HardwareError("Extended CPUID not supported"));
        }

        let mut ebx: u32;
        let mut ecx: u32;
        let mut edx: u32;

        unsafe {
            core::arch::asm!(
                "cpuid",
                in("eax") consts::cpuid::EXT_PROCESSOR_INFO,
                out("eax") _,
                out("ebx") ebx,
                out("ecx") ecx,
                out("edx") edx,
            );
        }

        self.ext_features_edx = edx;
        self.ext_features_ecx = ecx;

        // Get brand string
        for (i, func) in (consts::cpuid::BRAND_STRING_START..=consts::cpuid::BRAND_STRING_END).enumerate() {
            unsafe {
                core::arch::asm!(
                    "cpuid",
                    in("eax") func,
                    out("eax") eax,
                    out("ebx") ebx,
                    out("ecx") ecx,
                    out("edx") edx,
                );
            }

            let brand_bytes = [
                (eax & 0xFF) as u8, ((eax >> 8) & 0xFF) as u8, ((eax >> 16) & 0xFF) as u8, ((eax >> 24) & 0xFF) as u8,
                (ebx & 0xFF) as u8, ((ebx >> 8) & 0xFF) as u8, ((ebx >> 16) & 0xFF) as u8, ((ebx >> 24) & 0xFF) as u8,
                (ecx & 0xFF) as u8, ((ecx >> 8) & 0xFF) as u8, ((ecx >> 16) & 0xFF) as u8, ((ecx >> 24) & 0xFF) as u8,
                (edx & 0xFF) as u8, ((edx >> 8) & 0xFF) as u8, ((edx >> 16) & 0xFF) as u8, ((edx >> 24) & 0xFF) as u8,
            ];

            let start = i * 16;
            if start + 16 <= self.brand.len() {
                self.brand[start..start + 16].copy_from_slice(&brand_bytes);
            }
        }

        Ok(())
    }

    /// Detect cache information
    fn detect_cache_info(&mut self) -> Result<()> {
        let mut eax: u32;
        let mut ebx: u32;
        let mut ecx: u32;
        let mut edx: u32;

        unsafe {
            core::arch::asm!(
                "cpuid",
                in("eax") consts::cpuid::CACHE_INFO,
                out("eax") eax,
                out("ebx") ebx,
                out("ecx") ecx,
                out("edx") edx,
            );
        }

        // Extract cache line size (simplified)
        self.cache_line_size = ((ebx & 0xFFF) + 1) as u8;

        Ok(())
    }

    /// Get vendor as string
    pub fn vendor_str(&self) -> &str {
        core::str::from_utf8(&self.vendor).unwrap_or("Unknown")
    }

    /// Get brand as string
    pub fn brand_str(&self) -> &str {
        let end = self.brand.iter().position(|&b| b == 0).unwrap_or(self.brand.len());
        core::str::from_utf8(&self.brand[..end]).unwrap_or("Unknown")
    }

    /// Check if long mode is supported
    pub fn supports_long_mode(&self) -> bool {
        self.ext_features_edx & (1 << 29) != 0
    }

    /// Check if SSE2 is supported
    pub fn supports_sse2(&self) -> bool {
        self.features_edx & (1 << 26) != 0
    }

    /// Check if AVX is supported
    pub fn supports_avx(&self) -> bool {
        self.features_ecx & (1 << 28) != 0
    }
}

impl Default for X86_64CpuInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// x86_64 architecture-specific utilities
pub struct X86_64Utils;

impl X86_64Utils {
    /// Read from port
    pub unsafe fn inb(port: u16) -> u8 {
        let result: u8;
        core::arch::asm!(
            "in al, dx",
            out("al") result,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
        result
    }

    /// Write to port
    pub unsafe fn outb(port: u16, value: u8) {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }

    /// Read 32-bit from port
    pub unsafe fn inl(port: u16) -> u32 {
        let result: u32;
        core::arch::asm!(
            "in eax, dx",
            out("eax") result,
            in("dx") port,
            options(nomem, nostack, preserves_flags)
        );
        result
    }

    /// Write 32-bit to port
    pub unsafe fn outl(port: u16, value: u32) {
        core::arch::asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") value,
            options(nomem, nostack, preserves_flags)
        );
    }

    /// Read CR0
    pub fn read_cr0() -> u64 {
        let result: u64;
        unsafe {
            core::arch::asm!(
                "mov {}, cr0",
                out(reg) result,
                options(nomem, nostack, preserves_flags)
            );
        }
        result
    }

    /// Write CR0
    pub unsafe fn write_cr0(value: u64) {
        core::arch::asm!(
            "mov cr0, {}",
            in(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    /// Read CR4
    pub fn read_cr4() -> u64 {
        let result: u64;
        unsafe {
            core::arch::asm!(
                "mov {}, cr4",
                out(reg) result,
                options(nomem, nostack, preserves_flags)
            );
        }
        result
    }

    /// Write CR4
    pub unsafe fn write_cr4(value: u64) {
        core::arch::asm!(
            "mov cr4, {}",
            in(reg) value,
            options(nomem, nostack, preserves_flags)
        );
    }

    /// Read MSR
    pub unsafe fn read_msr(msr: u32) -> u64 {
        let low: u32;
        let high: u32;
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags)
        );
        ((high as u64) << 32) | (low as u64)
    }

    /// Write MSR
    pub unsafe fn write_msr(msr: u32, value: u64) {
        let low = value as u32;
        let high = (value >> 32) as u32;
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") low,
            in("edx") high,
            options(nomem, nostack, preserves_flags)
        );
    }

    /// Pause instruction
    pub fn pause() {
        unsafe {
            core::arch::asm!("pause", options(nomem, nostack));
        }
    }

    /// Memory barrier
    pub fn memory_barrier() {
        unsafe {
            core::arch::asm!("mfence", options(nomem, nostack));
        }
    }

    /// Halt the CPU
    pub fn halt() {
        unsafe {
            core::arch::asm!("hlt", options(nomem, nostack));
        }
    }

    /// Enable interrupts
    pub fn enable_interrupts() {
        unsafe {
            core::arch::asm!("sti", options(nomem, nostack));
        }
    }

    /// Disable interrupts
    pub fn disable_interrupts() {
        unsafe {
            core::arch::asm!("cli", options(nomem, nostack));
        }
    }

    /// Get current RFLAGS
    pub fn get_rflags() -> u64 {
        let result: u64;
        unsafe {
            core::arch::asm!(
                "pushf",
                "pop {}",
                out(reg) result,
                options(nomem, nostack, preserves_flags)
            );
        }
        result
    }

    /// Load GDT
    pub unsafe fn lgdt(gdt_ptr: &u16) {
        core::arch::asm!(
            "lgdt [{0}]",
            in(reg) gdt_ptr,
            options(nomem, nostack, preserves_flags)
        );
    }

    /// Load IDT
    pub unsafe fn lidt(idt_ptr: &u16) {
        core::arch::asm!(
            "lidt [{0}]",
            in(reg) idt_ptr,
            options(nomem, nostack, preserves_flags)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_info_creation() {
        let cpu_info = X86_64CpuInfo::new();
        assert_eq!(cpu_info.model, 0);
        assert_eq!(cpu_info.family, 0);
        assert_eq!(cpu_info.stepping, 0);
    }

    #[test]
    fn test_cpu_detection() {
        let mut cpu_info = X86_64CpuInfo::new();
        if cpu_info.cpuid_supported() {
            assert!(cpu_info.detect().is_ok());
            assert_ne!(cpu_info.vendor, [0; 12]);
        }
    }

    #[test]
    fn test_utils() {
        // Test read/write operations that are safe to test
        let rflags = X86_64Utils::get_rflags();
        println!("RFLAGS: {:#018X}", rflags);
    }
}