//! Firmware Interface - Boot protocol support (BIOS, UEFI, Multiboot2)

pub mod bios_loader;
pub mod disk_io;
pub mod disk_reader;
pub mod gpt_handler;
pub mod mbr_handler;
pub mod multiboot2_executor;
pub mod multiboot_loader;
pub mod uefi_boot_services;
pub mod uefi_loader;
