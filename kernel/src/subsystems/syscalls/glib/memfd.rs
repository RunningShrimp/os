//! Memfd file descriptor API

use alloc::vec::Vec;
use crate::api::KernelError;

pub struct MemFdInstance {
    data: Vec<u8>,
    size: usize,
}

impl MemFdInstance {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            size: 0,
        }
    }

    pub fn read(&self, buf: &mut [u8]) -> Result<usize, KernelError> {
        let bytes_to_read = core::cmp::min(buf.len(), self.data.len());
        buf[..bytes_to_read].copy_from_slice(&self.data[..bytes_to_read]);
        Ok(bytes_to_read)
    }

    pub fn write(&mut self, buf: &[u8]) -> Result<usize, KernelError> {
        let bytes_to_write = buf.len();
        self.data.extend_from_slice(buf);
        self.size = self.data.len();
        Ok(bytes_to_write)
    }
}

pub fn init() -> Result<(), KernelError> {
    Ok(())
}

/// Get a memfd instance by index
pub fn get_memfd_instance(_instance_idx: usize) -> Option<MemFdInstance> {
    // GH-#1339: Implement actual instance management
    // See: https://github.com/npos/kernel/issues/1339
    Some(MemFdInstance::new())
}
