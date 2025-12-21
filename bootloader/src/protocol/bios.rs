//! BIOS Boot Protocol Implementation
//!
//! This module provides BIOS-specific boot protocol implementation.

use crate::error::Result;
use crate::protocol::{BootProtocol, ProtocolType, MemoryMap, BootInfo, KernelImage};
use alloc::string::String;

pub struct BiosProtocol;

impl BiosProtocol {
    pub fn new() -> Self {
        Self
    }
}

impl BootProtocol for BiosProtocol {
    fn protocol_type(&self) -> ProtocolType {
        ProtocolType::BIOS
    }

    fn detect(&self) -> bool {
        // BIOS detection logic
        false
    }

    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    fn get_memory_map(&self) -> Result<MemoryMap> {
        Ok(MemoryMap::new())
    }

    fn get_boot_info(&self) -> Result<BootInfo> {
        Ok(BootInfo::new(ProtocolType::BIOS))
    }

    fn load_kernel(&mut self, _path: &str) -> Result<KernelImage> {
        unimplemented!("BIOS kernel loading")
    }

extern crate alloc;
    fn exit_boot_services(&mut self) -> Result<()> {
        Ok(())
    }
}
