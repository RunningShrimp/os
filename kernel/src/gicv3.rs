#![allow(dead_code)]

#[cfg(target_arch = "aarch64")]
pub struct GicV3 {
    dist_base: usize,
    redist_base: usize,
}

#[cfg(target_arch = "aarch64")]
impl GicV3 {
    pub const fn new(dist_base: usize, redist_base: usize) -> Self {
        Self { dist_base, redist_base }
    }

    #[inline]
    fn d32(&self, off: usize) -> *mut u32 { (self.dist_base + off) as *mut u32 }
    #[inline]
    fn r32(&self, off: usize) -> *mut u32 { (self.redist_base + off) as *mut u32 }

    pub fn enable(&self) {
        // Enable system register interface and CPU group 1
        unsafe {
            core::arch::asm!("msr icc_sre_el1, {}", in(reg) 1u64);
            core::arch::asm!("isb");
            core::arch::asm!("msr icc_pmr_el1, {}", in(reg) 0xFFu64);
            core::arch::asm!("msr icc_igrpen1_el1, {}", in(reg) 1u64);
        }

        // Wake redistributor (clear Sleep bit) and wait ChildrenAsleep==0
        let mut waker = crate::mm::mmio_read32(self.r32(0x0014) as *const u32);
        crate::mm::mmio_write32(self.r32(0x0014), waker & !(1 << 1));
        loop {
            waker = crate::mm::mmio_read32(self.r32(0x0014) as *const u32);
            if (waker & (1 << 2)) == 0 { break; }
            core::hint::spin_loop();
        }

        // Enable distributor for Group1NS
        crate::mm::mmio_write32(self.d32(0x000), 0x2);
    }

    pub fn disable(&self) {
        unsafe {
            core::arch::asm!("msr icc_igrpen1_el1, {}", in(reg) 0u64);
        }
        crate::mm::mmio_write32(self.d32(0x000), 0x0);
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
        unsafe {
            core::arch::asm!("msr icc_sre_el1, {}", in(reg) 1u64);
            core::arch::asm!("isb");
            core::arch::asm!("msr icc_pmr_el1, {}", in(reg) 0xFFu64);
            core::arch::asm!("msr icc_igrpen1_el1, {}", in(reg) 1u64);
        }
    }
}
