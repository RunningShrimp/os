extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, Ordering};

use crate::subsystems::mm::PAGE_SIZE;
use crate::types::stubs::VirtAddr;

const KPTI_ENTRY_SIZE: usize = 8;

pub static KPTI_ENABLED: AtomicBool = AtomicBool::new(true);

pub static USER_KASLR_BASE: VirtAddr = VirtAddr(0x555500000000);

#[derive(Debug, Clone, Copy)]
pub enum KptiMode {
    Disabled,
    Enabled,
    Auto,
}

impl Default for KptiMode {
    fn default() -> Self {
        KptiMode::Auto
    }
}

pub struct KptiConfig {
    pub mode: KptiMode,
    pub percpu_pgtables: bool,
    pub pcid_enabled: bool,
}

impl Default for KptiConfig {
    fn default() -> Self {
        Self {
            mode: KptiMode::Auto,
            percpu_pgtables: true,
            pcid_enabled: true,
        }
    }
}

pub struct KptiState {
    pub enabled: bool,
    pub config: KptiConfig,
    pub kernel_cr3: u64,
    pub user_cr3: u64,
    pub pgd: *mut u64,
}

unsafe impl Send for KptiState {}
unsafe impl Sync for KptiState {}

impl KptiState {
    pub fn new(config: KptiConfig) -> Self {
        let enabled = match config.mode {
            KptiMode::Disabled => false,
            KptiMode::Enabled => true,
            KptiMode::Auto => detect_kpti_needed(),
        };

        KPTI_ENABLED.store(enabled, Ordering::SeqCst);

        Self {
            enabled,
            config,
            kernel_cr3: 0,
            user_cr3: 0,
            pgd: core::ptr::null_mut(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn switch_to_user_cr3(&self) -> u64 {
        if !self.enabled {
            return 0;
        }

        #[cfg(target_arch = "x86_64")]
        {
            unsafe {
                let new_cr3 = self.user_cr3;
                if self.config.pcid_enabled {
                    core::arch::asm!(
                        "mov {0}, cr3",
                        in(reg) new_cr3,
                        options(nostack)
                    );
                } else {
                    core::arch::asm!(
                        "mov {0}, cr3",
                        in(reg) new_cr3,
                        options(nostack)
                    );
                }
                new_cr3
            }
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            0
        }
    }

    pub fn switch_to_kernel_cr3(&self) -> u64 {
        if !self.enabled {
            return 0;
        }

        #[cfg(target_arch = "x86_64")]
        {
            unsafe {
                let new_cr3 = self.kernel_cr3;
                core::arch::asm!(
                    "mov {0}, cr3",
                    in(reg) new_cr3,
                    options(nostack)
                );
                new_cr3
            }
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            0
        }
    }
}

pub fn detect_kpti_needed() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            let mut eax: u32;
            let mut ebx: u32;
            let mut ecx: u32;
            let mut edx: u32;

            core::arch::asm!(
                "cpuid",
                inlateout("eax") 1 => eax,
                out("ebx") ebx,
                out("ecx") ecx,
                out("edx") edx,
                options(nostack)
            );

            let family = (eax >> 8) & 0xF;
            let model = (eax >> 4) & 0xF;
            let extended_model = (eax >> 16) & 0xF;
            let full_model = (extended_model << 4) | model;

            let is_intel = ebx == 0x756E6547;
            let is_amd = ebx == 0x68747541;

            if is_intel {
                let is_sandy_bridge = family == 6 && (full_model == 0x2A || full_model == 0x2D);
                let is_ivy_bridge = family == 6 && (full_model == 0x3A || full_model == 0x3E);
                let is_haswell = family == 6 && (full_model >= 0x3C && full_model <= 0x3F || full_model >= 0x45 && full_model <= 0x47);
                let is_broadwell = family == 6 && (full_model >= 0x3D && full_model <= 0x3F || full_model >= 0x46 && full_model <= 0x4F || full_model >= 0x56 && full_model <= 0x5F);
                let is_skylake = family == 6 && (full_model >= 0x4E && full_model <= 0x5E);
                let is_kabylake = family == 6 && (full_model == 0x8E || full_model == 0x9E);
                let is_coffee_lake = family == 6 && (full_model >= 0x9D && full_model <= 0x9F);

                let vulnerable = is_sandy_bridge || is_ivy_bridge || is_haswell || is_broadwell 
                    || is_skylake || is_kabylake || is_coffee_lake;

                return vulnerable;
            }

            if is_amd {
                return false;
            }

            false
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

pub fn init_kpti(config: KptiConfig) -> Result<(), &'static str> {
    let state = KptiState::new(config);

    if !state.enabled {
        log::info!("KPTI disabled by configuration");
        return Ok(());
    }

    log::info!("KPTI enabled for Meltdown mitigation");

    #[cfg(target_arch = "x86_64")]
    {
        if config.pcid_enabled {
            unsafe {
                let mut cr4: u64;
                core::arch::asm!(
                    "mov {0}, cr4",
                    out(reg) cr4,
                    options(nostack)
                );

                cr4 |= 1 << 17;

                core::arch::asm!(
                    "mov cr4, {0}",
                    in(reg) cr4,
                    options(nostack)
                );

                log::info!("PCID enabled for KPTI");
            }
        }
    }

    Ok(())
}

pub fn create_user_pgd(kernel_pgd: *mut u64) -> Result<*mut u64, &'static str> {
    extern crate alloc;

    let user_pgd = unsafe {
        alloc::alloc::alloc_zeroed(
            alloc::alloc::Layout::from_size_align(PAGE_SIZE, PAGE_SIZE)
                .map_err(|_| "Invalid layout")?
        ) as *mut u64
    };

    if user_pgd.is_null() {
        return Err("Failed to allocate user page directory");
    }

    unsafe {
        let kernel_pgd_slice = core::slice::from_raw_parts(kernel_pgd, 512);
        let user_pgd_slice = core::slice::from_raw_parts_mut(user_pgd, 512);

        for i in 0..512 {
            user_pgd_slice[i] = kernel_pgd_slice[i];
        }

        let entry_index = (USER_KASLR_BASE.0 >> 39) & 0x1FF;

        if entry_index < 512 {
            user_pgd_slice[entry_index] = 0;
        }
    }

    Ok(user_pgd)
}

pub fn get_cr3() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            let cr3: u64;
            core::arch::asm!(
                "mov {0}, cr3",
                out(reg) cr3,
                options(nostack)
            );
            cr3
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        0
    }
}

pub fn set_cr3(value: u64) {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            core::arch::asm!(
                "mov cr3, {0}",
                in(reg) value,
                options(nostack)
            );
        }
    }
}

#[inline]
pub fn entry_to_usermode() {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            core::arch::asm!(
                "swapgs",
                options(nostack)
            );
        }
    }
}

#[inline]
pub fn entry_from_usermode() {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            core::arch::asm!(
                "swapgs",
                options(nostack)
            );
        }
    }
}

pub fn flush_tlb_addr(addr: VirtAddr) {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            core::arch::asm!(
                "invlpg [{0}]",
                in(reg) addr.0,
                options(nostack, nostack, memory)
            );
        }
    }
}

pub fn flush_tlb_all() {
    #[cfg(target_arch = "x86_64")]
    {
        let cr3 = get_cr3();
        set_cr3(cr3);
    }
}
