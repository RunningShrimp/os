#![allow(unused)]

extern crate alloc;

use alloc::string::String;
use bitflags::bitflags;

pub type ServiceVersionTuple = (u16, u16);

pub trait Versioned {
    fn service_name(&self) -> &'static str;
    fn version(&self) -> ServiceVersionTuple;
}

pub trait ProvidesCapabilities {
    fn capabilities(&self) -> ServiceCapabilities;
}

bitflags! {
    pub struct ServiceCapabilities: u32 {
        const MEMORY   = 0b0000_0001;
        const PROCESS  = 0b0000_0010;
        const FS       = 0b0000_0100;
        const IPC      = 0b0000_1000;
        const NETWORK  = 0b0001_0000;
        const DRIVER   = 0b0010_0000;
        const SYSCALL  = 0b0100_0000;
        const METRICS  = 0b1000_0000;
    }
}

pub struct ServiceDescriptor {
    pub name: &'static str,
    pub version: ServiceVersionTuple,
    pub capabilities: ServiceCapabilities,
    pub endpoint: usize,
}

impl ServiceDescriptor {
    pub fn id(&self) -> String {
        let (maj, min) = self.version;
        String::from(self.name) + ":" + &maj.to_string() + "." + &min.to_string()
    }
}

