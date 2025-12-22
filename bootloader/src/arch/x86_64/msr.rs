// x86_64 Model-Specific Register (MSR) operations

// MSR addresses
pub const IA32_EFER: u32 = 0xC0000080; // Extended Feature Enable
pub const IA32_STAR: u32 = 0xC0000081; // SYSRET/SYSCALL RIP
pub const IA32_LSTAR: u32 = 0xC0000082; // SYSCALL RIP 64-bit
pub const IA32_CSTAR: u32 = 0xC0000083; // Compat SYSCALL RIP
pub const IA32_SFMASK: u32 = 0xC0000084; // SYSCALL flags mask
pub const IA32_FS_BASE: u32 = 0xC0000100; // FS base
pub const IA32_GS_BASE: u32 = 0xC0000101; // GS base
pub const IA32_KERNEL_GS_BASE: u32 = 0xC0000102; // Kernel GS base

// EFER bits
pub const EFER_SCE: u64 = 0x001; // SYSCALL/SYSRET enable
pub const EFER_LME: u64 = 0x100; // Long mode enable
pub const EFER_LMA: u64 = 0x400; // Long mode active
pub const EFER_NXE: u64 = 0x800; // No-execute enable

pub fn read_msr(msr: u32) -> u64 {
    unsafe {
        let (low, high): (u32, u32);
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") low,
            out("edx") high,
            options(nostack, preserves_flags)
        );
        ((high as u64) << 32) | (low as u64)
    }
}

pub fn write_msr(msr: u32, value: u64) {
    unsafe {
        let low = value as u32;
        let high = (value >> 32) as u32;
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") low,
            in("edx") high,
            options(nostack, preserves_flags)
        );
    }
}

pub fn read_efer() -> u64 {
    read_msr(IA32_EFER)
}

pub fn write_efer(value: u64) {
    write_msr(IA32_EFER, value);
}

pub fn set_efer_flag(flag: u64) {
    let efer = read_efer();
    write_efer(efer | flag);
}

pub fn clear_efer_flag(flag: u64) {
    let efer = read_efer();
    write_efer(efer & !flag);
}

pub fn set_fs_base(addr: u64) {
    write_msr(IA32_FS_BASE, addr);
}

pub fn set_gs_base(addr: u64) {
    write_msr(IA32_GS_BASE, addr);
}

pub fn set_kernel_gs_base(addr: u64) {
    write_msr(IA32_KERNEL_GS_BASE, addr);
}

pub fn setup_syscall_handlers(
    star: u64,
    lstar: u64,
    sfmask: u64,
) {
    write_msr(IA32_STAR, star);
    write_msr(IA32_LSTAR, lstar);
    write_msr(IA32_SFMASK, sfmask);
}

pub fn enable_syscall() {
    set_efer_flag(EFER_SCE);
}

pub fn enable_nx_bit() {
    set_efer_flag(EFER_NXE);
}

pub fn is_long_mode_active() -> bool {
    (read_efer() & EFER_LMA) != 0
}
