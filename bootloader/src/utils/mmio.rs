// Memory-mapped I/O utilities

use core::marker::PhantomData;
use core::ptr::{read_volatile, write_volatile};

/// Generic MMIO register wrapper
pub struct MmioReg<T> {
    addr: *mut T,
    _phantom: PhantomData<T>,
}

impl<T> MmioReg<T> {
    /// Create MMIO register from physical address
    ///
    /// # Safety
    /// - `addr` must be a valid physical memory address for the MMIO register
    /// - `addr` must be properly aligned for type `T`
    /// - The memory at `addr` must be accessible for reading and writing
    pub unsafe fn new(addr: usize) -> Self {
        Self {
            addr: addr as *mut T,
            _phantom: PhantomData,
        }
    }

    /// Read volatile value
    pub fn read(&self) -> T
    where
        T: Copy,
    {
        unsafe { read_volatile(self.addr) }
    }

    /// Write volatile value
    pub fn write(&mut self, val: T) {
        unsafe { write_volatile(self.addr, val) }
    }
}

/// Safe MMIO register with type safety
pub struct Mmio8(MmioReg<u8>);
pub struct Mmio32(MmioReg<u32>);
pub struct Mmio64(MmioReg<u64>);

impl Mmio8 {
    /// Create 8-bit MMIO register from physical address
    ///
    /// # Safety
    /// - `addr` must be a valid physical memory address for an 8-bit MMIO register
    /// - `addr` must be properly aligned for u8 type
    /// - The memory at `addr` must be accessible for reading and writing
    pub unsafe fn new(addr: usize) -> Self {
        Self(MmioReg::new(addr))
    }

    pub fn read(&self) -> u8 {
        self.0.read()
    }

    pub fn write(&mut self, val: u8) {
        self.0.write(val)
    }
}

impl Mmio32 {
    /// Create 32-bit MMIO register from physical address
    ///
    /// # Safety
    /// - `addr` must be a valid physical memory address for a 32-bit MMIO register
    /// - `addr` must be properly aligned for u32 type (4-byte aligned)
    /// - The memory at `addr` must be accessible for reading and writing
    pub unsafe fn new(addr: usize) -> Self {
        Self(MmioReg::new(addr))
    }

    pub fn read(&self) -> u32 {
        self.0.read()
    }

    pub fn write(&mut self, val: u32) {
        self.0.write(val)
    }
}

impl Mmio64 {
    /// Create 64-bit MMIO register from physical address
    ///
    /// # Safety
    /// - `addr` must be a valid physical memory address for a 64-bit MMIO register
    /// - `addr` must be properly aligned for u64 type (8-byte aligned)
    /// - The memory at `addr` must be accessible for reading and writing
    pub unsafe fn new(addr: usize) -> Self {
        Self(MmioReg::new(addr))
    }

    pub fn read(&self) -> u64 {
        self.0.read()
    }

    pub fn write(&mut self, val: u64) {
        self.0.write(val)
    }
}
