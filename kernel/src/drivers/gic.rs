#![allow(dead_code)]

#[cfg(target_arch = "aarch64")]
pub struct GicV2 {
    dist_base: usize,
    cpu_base: usize,
}

#[cfg(target_arch = "aarch64")]
impl GicV2 {
    pub const fn new(dist_base: usize, cpu_base: usize) -> Self {
        Self { dist_base, cpu_base }
    }

    #[inline]
    fn d32(&self, off: usize) -> *mut u32 {
        (self.dist_base + off) as *mut u32
    }

    #[inline]
    fn c32(&self, off: usize) -> *mut u32 {
        (self.cpu_base + off) as *mut u32
    }

    pub fn enable(&self) {
        crate::mm::mmio_write32(self.d32(0x000), 1);
        crate::mm::mmio_write32(self.c32(0x000), 1);
        crate::mm::mmio_write32(self.c32(0x004), 0xFF);
    }

    pub fn disable(&self) {
        crate::mm::mmio_write32(self.c32(0x000), 0);
        crate::mm::mmio_write32(self.d32(0x000), 0);
    }

    pub fn set_enable(&self, irq: usize) {
        let reg = 0x100 + ((irq / 32) * 4);
        let bit = 1u32 << (irq % 32);
        let v = crate::mm::mmio_read32(self.d32(reg) as *const u32);
        crate::mm::mmio_write32(self.d32(reg), v | bit);
    }

    pub fn clear_enable(&self, irq: usize) {
        let reg = 0x180 + ((irq / 32) * 4);
        let bit = 1u32 << (irq % 32);
        crate::mm::mmio_write32(self.d32(reg), bit);
    }

    pub fn cpu_enable(&self) {
        crate::mm::mmio_write32(self.c32(0x000), 1);
        crate::mm::mmio_write32(self.c32(0x004), 0xFF);
    }
}
