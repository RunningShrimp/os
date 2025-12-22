//! Core - Bootloader initialization and architecture

pub mod allocator;
pub mod init;
pub mod boot_state;
pub mod boot_sequence;
pub mod version;
pub mod arch_x86_64;
pub mod arch_aarch64;
pub mod arch_riscv64;
pub mod allocator_integration;
pub mod deferred_init;
