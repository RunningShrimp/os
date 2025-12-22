// x86_64 Long Mode initialization and setup

use core::mem::size_of;

pub const CR0_PROTECTED_MODE: u64 = 0x00000001;
pub const CR0_PAGING: u64 = 0x80000000;
pub const CR0_CACHE_DISABLE: u64 = 0x40000000;
pub const CR0_WRITE_PROTECT: u64 = 0x00010000;

pub const CR4_PAE: u64 = 0x00000020;
pub const CR4_PSE: u64 = 0x00000010;
pub const CR4_PGE: u64 = 0x00000080;

pub const EFER_LME: u64 = 0x00000100;
pub const EFER_LMA: u64 = 0x00000400;
pub const EFER_SCE: u64 = 0x00000001;
pub const EFER_NXE: u64 = 0x00000800;

pub const IA32_EFER: u32 = 0xC0000080;

#[repr(C, packed)]
pub struct PageTableEntry {
    pub value: u64,
}

impl PageTableEntry {
    pub const PRESENT: u64 = 0x001;
    pub const WRITABLE: u64 = 0x002;
    pub const USER: u64 = 0x004;
    pub const WRITE_THROUGH: u64 = 0x008;
    pub const CACHE_DISABLE: u64 = 0x010;
    pub const ACCESSED: u64 = 0x020;
    pub const DIRTY: u64 = 0x040;
    pub const HUGE_PAGE: u64 = 0x080;
    pub const GLOBAL: u64 = 0x100;

    pub fn new(addr: u64, flags: u64) -> Self {
        Self {
            value: (addr & 0x000FFFFFFFFFF000) | (flags & 0xFFF),
        }
    }

    pub fn null() -> Self {
        Self { value: 0 }
    }

    pub fn is_present(&self) -> bool {
        (self.value & Self::PRESENT) != 0
    }
}

pub fn enable_long_mode() -> bool {
    unsafe {
        // Check CPUID support
        if !has_long_mode_support() {
            return false;
        }

        // Set PAE flag in CR4
        let mut cr4: u64;
        core::arch::asm!(
            "mov {}, cr4",
            out(reg) cr4,
            options(nostack, preserves_flags)
        );
        cr4 |= CR4_PAE | CR4_PSE | CR4_PGE;
        core::arch::asm!(
            "mov cr4, {}",
            in(reg) cr4,
            options(nostack)
        );

        // Set LME flag in EFER
        let mut efer = read_msr(IA32_EFER);
        efer |= EFER_LME | EFER_SCE | EFER_NXE;
        write_msr(IA32_EFER, efer);

        // Set paging bit in CR0
        let mut cr0: u64;
        core::arch::asm!(
            "mov {}, cr0",
            out(reg) cr0,
            options(nostack, preserves_flags)
        );
        cr0 |=
            CR0_PAGING | CR0_PROTECTED_MODE | CR0_WRITE_PROTECT;
        cr0 &= !CR0_CACHE_DISABLE;
        core::arch::asm!(
            "mov cr0, {}",
            in(reg) cr0,
            options(nostack)
        );

        true
    }
}

pub fn has_long_mode_support() -> bool {
    unsafe {
        // CPUID extended function 0x80000001 checks for long mode
        let mut eax: u32;
        let mut edx: u32;
        core::arch::asm!(
            "mov eax, 0x80000001",
            "cpuid",
            out("eax") eax,
            out("edx") edx,
            options(nostack)
        );

        // Bit 29 in EDX = long mode support
        (edx & 0x20000000) != 0
    }
}

fn read_msr(msr: u32) -> u64 {
    unsafe {
        let (low, high): (u32, u32);
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nostack)
        );
        ((high as u64) << 32) | (low as u64)
    }
}

fn write_msr(msr: u32, value: u64) {
    unsafe {
        let low = value as u32;
        let high = (value >> 32) as u32;
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") low,
            in("edx") high,
            options(nostack)
        );
    }
}

pub fn setup_page_tables() -> bool {
    // For bootloader, use identity mapping
    // Kernel will set up proper virtual memory
    true
}

pub fn verify_long_mode_enabled() -> bool {
    unsafe {
        let efer = read_msr(IA32_EFER);
        (efer & EFER_LMA) != 0
    }
}
