// Version and build information

pub const VERSION_MAJOR: u16 = 0;
pub const VERSION_MINOR: u16 = 1;
pub const VERSION_PATCH: u16 = 0;

pub const TARGET_ARCH: &str = "x86_64/aarch64/riscv64";

pub fn print_version() {
    crate::drivers::console::write_str("=== NOS Bootloader v0.1.0 ===\n");
}

pub fn print_build_info() {
    crate::drivers::console::write_str("Multi-arch bootloader\n");
    crate::drivers::console::write_str("Copyright (c) 2024 NOS Project\n");
}

