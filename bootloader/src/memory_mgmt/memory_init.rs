// Early memory initialization for bootloader

/// Initialize memory regions accessible during boot
pub fn init_memory() {
    #[cfg(target_arch = "x86_64")]
    init_memory_x86_64();
    
    #[cfg(target_arch = "aarch64")]
    init_memory_aarch64();
    
    #[cfg(target_arch = "riscv64")]
    init_memory_riscv64();
}

#[cfg(target_arch = "x86_64")]
fn init_memory_x86_64() {
    // Identity map lower 4GB using 2MB pages
    // This is done by hardware or bootloader already
}

#[cfg(target_arch = "aarch64")]
fn init_memory_aarch64() {
    // ARM64 starts with identity mapping provided by firmware
    // No additional setup needed for P0
}

#[cfg(target_arch = "riscv64")]
fn init_memory_riscv64() {
    // RISC-V identity mapping provided by SBI
    // Virtual address = physical address until paging setup
}

/// Validate bootloader can access memory
pub fn verify_memory_access() -> bool {
    // Test write to heap region
    let test_addr = 0x10000 as *mut u32;
    unsafe {
        test_addr.write_volatile(0xdeadbeef);
        let val = test_addr.read_volatile();
        val == 0xdeadbeef
    }
}
