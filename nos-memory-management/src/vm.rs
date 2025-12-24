//! Virtual Memory module
//! Provides virtual memory management abstractions

extern crate alloc;

use alloc::vec::Vec;

/// Page structure
#[derive(Debug, Clone, Copy)]
pub struct Page {
    pub addr: usize,
    pub flags: u64,
}

impl Page {
    pub fn new(addr: usize) -> Self {
        Self {
            addr,
            flags: 0,
        }
    }

    pub fn with_flags(addr: usize, flags: u64) -> Self {
        Self {
            addr,
            flags,
        }
    }

    pub fn addr(&self) -> usize {
        self.addr
    }

    pub fn is_valid(&self) -> bool {
        self.addr != 0
    }
}

impl Default for Page {
    fn default() -> Self {
        Self {
            addr: 0,
            flags: 0,
        }
    }
}

/// Memory mapping flags
pub mod flags {
    pub const READ: u64 = 0x1;
    pub const WRITE: u64 = 0x2;
    pub const EXEC: u64 = 0x4;
    pub const USER: u64 = 0x8;
    pub const GLOBAL: u64 = 0x10;
    pub const NOCACHE: u64 = 0x20;
}
