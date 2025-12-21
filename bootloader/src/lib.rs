//! NOS Bootloader - Modern UEFI and BIOS bootloader Library
//!
//! This library contains the core bootloader functionality.

#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![allow(unsafe_op_in_unsafe_fn)]

extern crate alloc;

// Core modules
pub mod arch;
pub mod error;
pub mod memory;
pub mod protocol;

// Architecture-specific modules
#[cfg(target_arch = "x86_64")]
pub use arch::x86_64;
#[cfg(target_arch = "aarch64")]
pub use arch::aarch64;
#[cfg(target_arch = "riscv64")]
pub use arch::riscv64;

// Protocol-specific modules
#[cfg(feature = "uefi_support")]
pub mod uefi;
#[cfg(feature = "bios_support")]
pub use protocol::bios;

// Feature-specific modules
#[cfg(feature = "graphics_support")]
pub mod graphics;
#[cfg(feature = "menu_support")]
pub mod boot_menu;
#[cfg(feature = "network_support")]
pub mod network;
#[cfg(feature = "recovery_support")]
pub mod recovery;

// Boot protocol modules
pub mod kernel;