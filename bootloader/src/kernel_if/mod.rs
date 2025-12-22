//! Kernel Interface - Loading, handoff, bootstrap, protocol (P0, P4)

pub mod kernel_handoff;
pub mod kernel_loader;
pub mod kernel_loader_impl;
pub mod kernel_bootstrap;
pub mod kernel_entry;
pub mod elf_loader;
pub mod elf_loader_v2;
pub mod elf_loader_hardened;
pub mod elf64;
pub mod boot_info_builder;
pub mod protocol_manager;
