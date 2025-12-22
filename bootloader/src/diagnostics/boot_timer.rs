// Boot timing measurements for performance analysis

use core::sync::atomic::{AtomicU64, Ordering};

pub struct BootTimer {
    start_time: AtomicU64,
    checkpoints: [AtomicU64; 16],
}

impl BootTimer {
    pub const fn new() -> Self {
        Self {
            start_time: AtomicU64::new(0),
            checkpoints: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
        }
    }

    pub fn start(&self) {
        self.start_time.store(get_ticks(), Ordering::Release);
    }

    pub fn checkpoint(&self, index: usize) {
        if index < 16 {
            self.checkpoints[index].store(get_ticks(), Ordering::Release);
        }
    }

    pub fn elapsed(&self) -> u64 {
        let now = get_ticks();
        let start = self.start_time.load(Ordering::Acquire);
        if start > 0 {
            now.wrapping_sub(start)
        } else {
            0
        }
    }

    pub fn checkpoint_elapsed(&self, index: usize) -> u64 {
        if index < 16 {
            let checkpoint = self.checkpoints[index].load(Ordering::Acquire);
            let start = self.start_time.load(Ordering::Acquire);
            if start > 0 && checkpoint > 0 {
                checkpoint.wrapping_sub(start)
            } else {
                0
            }
        } else {
            0
        }
    }

    pub fn print_stats(&self) {
        let total = self.elapsed();
        crate::drivers::console::write_str("Boot timing: ");
        crate::drivers::console::write_str(if total > 0 { "~" } else { "0" });
        crate::drivers::console::write_str("ms\n");
    }
}

pub fn get_ticks() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        let tsc: u64;
        unsafe {
            core::arch::asm!(
                "rdtsc",
                out("rax") tsc,
                options(nostack, preserves_flags)
            );
        }
        tsc
    }

    #[cfg(target_arch = "aarch64")]
    {
        let cntvct: u64;
        unsafe {
            core::arch::asm!(
                "mrs {}, cntvct_el0",
                out(reg) cntvct,
                options(nostack, preserves_flags)
            );
        }
        cntvct
    }

    #[cfg(target_arch = "riscv64")]
    {
        let time: u64;
        unsafe {
            core::arch::asm!(
                "rdtime {}",
                out(reg) time,
                options(nostack, preserves_flags)
            );
        }
        time
    }
}

pub static BOOT_TIMER: BootTimer = BootTimer::new();
