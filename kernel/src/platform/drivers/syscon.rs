#![allow(dead_code)]

pub struct Syscon {
    base: usize,
}

impl Syscon {
    pub const fn new(base: usize) -> Self { Self { base } }

    #[inline]
    fn p32(&self, off: usize) -> *mut u32 { (self.base + off) as *mut u32 }

    #[inline]
    fn p64(&self, off: usize) -> *mut u64 { (self.base + off) as *mut u64 }

    pub fn read32(&self, off: usize) -> u32 { unsafe { crate::subsystems::mm::mmio_read32(self.p32(off) as *const u32) } }

    pub fn write32(&self, off: usize, val: u32) { unsafe { crate::subsystems::mm::mmio_write32(self.p32(off), val) } }

    pub fn read64(&self, off: usize) -> u64 { unsafe { crate::subsystems::mm::mmio_read64(self.p64(off) as *const u64) } }

    pub fn write64(&self, off: usize, val: u64) { unsafe { crate::subsystems::mm::mmio_write64(self.p64(off), val) } }
}
