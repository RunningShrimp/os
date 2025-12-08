//! x86_64 architecture-specific implementation
//!
//! This module provides x86_64-specific functionality for the bootloader,
//! including CPU detection, interrupt handling, and kernel transition.

use crate::arch::{CpuFeatures, MemoryLayout};
use crate::error::{BootError, Result};
use core::arch::asm;

/// Early x86_64 initialization
pub fn early_init() {
    // Set up basic x86_64 environment
    // In a real implementation, this would include:
    // - CPUID detection
    // - Feature detection
    // - Basic setup

    // For now, just ensure we're in 64-bit mode
    let mut cpuid_result = [0u32; 4];
    unsafe {
        asm!(
            "mov eax, 0x80000001",
            "cpuid",
            inout("eax") cpuid_result[0] => _,
            out("ebx") cpuid_result[1],
            out("ecx") cpuid_result[2],
            out("edx") cpuid_result[3],
            options(nomem, nostack, preserves_flags),
        );
    }

    // Check if we're actually in 64-bit mode
    if cpuid_result[3] & (1 << 29) == 0 {
        panic!("x86_64 bootloader requires 64-bit CPU");
    }
}

/// Get x86_64 CPU features
pub fn get_cpu_features() -> CpuFeatures {
    let mut features = CpuFeatures {
        has_virtualization: false,
        is_64bit: true,
        cache_line_size: 64, // Default x86_64 cache line size
        cpu_count: 1,       // Will be updated with proper detection
        arch_flags: 0,
    };

    // CPUID detection for features
    let mut cpuid_eax = 0u32;
    let mut cpuid_ebx = 0u32;
    let mut cpuid_ecx = 0u32;
    let mut cpuid_edx = 0u32;

    unsafe {
        // Basic CPUID information
        asm!(
            "mov eax, 1",
            "cpuid",
            out("eax") cpuid_eax,
            out("ebx") cpuid_ebx,
            out("ecx") cpuid_ecx,
            out("edx") cpuid_edx,
            options(nomem, nostack, preserves_flags),
        );
    }

    // Check for virtualization support (Intel VT-x)
    if cpuid_ecx & (1 << 5) != 0 {
        features.has_virtualization = true;
    }

    // Store feature flags
    features.arch_flags = ((cpuid_edx as u64) << 32) | (cpuid_ecx as u64);

    // CPU count detection (simplified - in real implementation would be more complex)
    let logical_processor_count = (cpuid_ebx & 0x00FF0000) >> 16;
    if logical_processor_count > 0 {
        features.cpu_count = logical_processor_count as usize;
    }

    features
}

/// Disable interrupts on x86_64
pub fn interrupt_disable() {
    unsafe {
        asm!("cli", options(nomem, nostack, preserves_flags));
    }
}

/// Enable interrupts on x86_64
pub fn interrupt_enable() {
    unsafe {
        asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

/// Check if interrupts are enabled on x86_64
pub fn interrupt_enabled() -> bool {
    let flags: u64;
    unsafe {
        asm!("pushfq; pop {}", out(reg) flags, options(nomem, nostack, preserves_flags));
    }
    (flags & 0x200) != 0
}

/// Wait for interrupt (halt instruction)
pub fn wait_for_interrupt() {
    unsafe {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

/// Memory barrier for x86_64
pub fn memory_barrier() {
    unsafe {
        asm!("mfence", options(nomem, nostack, preserves_flags));
    }
}

/// Get current CPU ID (simplified for x86_64)
pub fn cpu_id() -> usize {
    // In a real implementation, this would use CPUID to get the APIC ID
    // For now, return 0 (boot CPU)
    0
}

/// Jump to kernel entry point on x86_64
///
/// # Safety
/// This function performs an unsafe transition to the kernel
pub unsafe fn jump_to_kernel(entry_point: usize, boot_params: &crate::arch::BootParameters) -> ! {
    // Set up kernel stack
    let stack_top = crate::arch::Architecture::X86_64.default_stack_size() - 8;

    // Load boot parameters into appropriate registers
    // According to System V AMD64 ABI:
    // - RDI: first parameter (boot_params pointer)
    // - RSI: second parameter (would be magic number if needed)
    // - RSP: stack pointer

    asm!(
        "mov rsp, {stack_top}",
        "mov rdi, {boot_params}",
        "jmp {entry_point}",
        stack_top = in(reg) stack_top,
        boot_params = in(reg) boot_params as *const _,
        entry_point = in(reg) entry_point,
        options(nomem, nostack),
    );

    // Should never reach here
    unreachable!();
}

/// Reboot the system on x86_64
pub fn reboot() -> ! {
    // Use the keyboard controller to reset the system
    let mut temp: u8;

    unsafe {
        // Disable interrupts
        asm!("cli", options(nomem, nostack, preserves_flags));

        // Try keyboard controller reset
        for _ in 0..10 {
            asm!(
                "mov al, 0xFE",
                "out 0x64, al",
                out("al") temp,
                options(nomem, nostack, preserves_flags),
            );
        }

        // Fallback: triple fault
        asm!(
            "lidt [{idt_ptr}]",
            "int 3", // This should cause a triple fault
            idt_ptr = in(reg) 0usize, // Null IDT pointer
            options(nomem, nostack, preserves_flags),
        );
    }

    // If we get here, try to halt
    loop {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

/// Shutdown the system on x86_64
pub fn shutdown() -> ! {
    // x86_64 doesn't have a standard shutdown mechanism
    // We'll try a few methods and then halt

    unsafe {
        // Try ACPI shutdown if available
        // (This would require ACPI tables to be parsed first)

        // Try to halt using port 60h (keyboard controller)
        let mut temp: u8;
        asm!(
            "mov al, 0xFE",
            "out 0x64, al",
            out("al") temp,
            options(nomem, nostack, preserves_flags),
        );
    }

    // Fallback: halt
    loop {
        wait_for_interrupt();
    }
}

/// Read from I/O port
pub unsafe fn inb(port: u16) -> u8 {
    let value: u8;
    asm!(
        "in al, dx",
        in("dx") port,
        out("al") value,
        options(nomem, nostack, preserves_flags),
    );
    value
}

/// Write to I/O port
pub unsafe fn outb(port: u16, value: u8) {
    asm!(
        "out dx, al",
        in("dx") port,
        in("al") value,
        options(nomem, nostack, preserves_flags),
    );
}

/// Read from I/O port (word)
pub unsafe fn inw(port: u16) -> u16 {
    let value: u16;
    asm!(
        "in ax, dx",
        in("dx") port,
        out("ax") value,
        options(nomem, nostack, preserves_flags),
    );
    value
}

/// Write to I/O port (word)
pub unsafe fn outw(port: u16, value: u16) {
    asm!(
        "out dx, ax",
        in("dx") port,
        in("ax") value,
        options(nomem, nostack, preserves_flags),
    );
}

/// Read from I/O port (double word)
pub unsafe fn inl(port: u16) -> u32 {
    let value: u32;
    asm!(
        "in eax, dx",
        in("dx") port,
        out("eax") value,
        options(nomem, nostack, preserves_flags),
    );
    value
}

/// Write to I/O port (double word)
pub unsafe fn outl(port: u16, value: u32) {
    asm!(
        "out dx, eax",
        in("dx") port,
        in("eax") value,
        options(nomem, nostack, preserves_flags),
    );
}

/// CPUID instruction wrapper
pub unsafe fn cpuid(eax: u32) -> (u32, u32, u32, u32) {
    let (mut eax_out, mut ebx_out, mut ecx_out, mut edx_out) = (0u32, 0u32, 0u32, 0u32);

    asm!(
        "cpuid",
        inlateout("eax") eax => eax_out,
        out("ebx") ebx_out,
        out("ecx") ecx_out,
        out("edx") edx_out,
        options(nomem, nostack, preserves_flags),
    );

    (eax_out, ebx_out, ecx_out, edx_out)
}

/// Read Model Specific Register (MSR)
pub unsafe fn rdmsr(msr: u32) -> u64 {
    let (low, high): (u32, u32);

    asm!(
        "rdmsr",
        in("ecx") msr,
        out("eax") low,
        out("edx") high,
        options(nomem, nostack, preserves_flags),
    );

    ((high as u64) << 32) | (low as u64)
}

/// Write Model Specific Register (MSR)
pub unsafe fn wrmsr(msr: u32, value: u64) {
    let low = value as u32;
    let high = (value >> 32) as u32;

    asm!(
        "wrmsr",
        in("ecx") msr,
        in("eax") low,
        in("edx") high,
        options(nomem, nostack, preserves_flags),
    );
}

/// Get current control register (CR0)
pub unsafe fn get_cr0() -> u64 {
    let value: u64;
    asm!("mov {}, cr0", out(reg) value, options(nomem, nostack, preserves_flags));
    value
}

/// Get current control register (CR3) - page table base
pub unsafe fn get_cr3() -> u64 {
    let value: u64;
    asm!("mov {}, cr3", out(reg) value, options(nomem, nostack, preserves_flags));
    value
}

/// Set control register (CR3) - page table base
pub unsafe fn set_cr3(value: u64) {
    asm!("mov cr3, {}", in(reg) value, options(nomem, nostack, preserves_flags));
}

/// Invalidate TLB entry
pub unsafe fn invlpg(addr: usize) {
    asm!("invlpg [{}]", in(reg) addr, options(nomem, nostack, preserves_flags));
}

/// Get current TSC (Time Stamp Counter)
pub unsafe fn get_tsc() -> u64 {
    let low: u32;
    let high: u32;

    asm!(
        "rdtsc",
        out("eax") low,
        out("edx") high,
        options(nomem, nostack, preserves_flags),
    );

    ((high as u64) << 32) | (low as u64)
}

/// Get memory layout for x86_64
pub fn get_memory_layout(kernel_size: usize) -> Result<MemoryLayout> {
    let layout = MemoryLayout::for_architecture(crate::arch::Architecture::X86_64, kernel_size);
    layout.validate()?;
    Ok(layout)
}

/// Check if PAE (Physical Address Extension) is supported
pub fn supports_pae() -> bool {
    unsafe {
        let (_, _, _, edx) = cpuid(1);
        edx & (1 << 6) != 0
    }
}

/// Check if NX (No Execute) bit is supported
pub fn supports_nx() -> bool {
    unsafe {
        let (_, _, edx, _) = cpuid(0x80000001);
        edx & (1 << 20) != 0
    }
}

/// Check if we're running under a hypervisor
pub fn is_virtualized() -> bool {
    unsafe {
        let (_, _, ecx, _) = cpuid(1);
        ecx & (1 << 31) != 0
    }
}