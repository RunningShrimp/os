extern crate alloc;

use core::sync::atomic::{AtomicBool, Ordering};

pub static RETPOLINE_ENABLED: AtomicBool = AtomicBool::new(true);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetpolineMode {
    Disabled,
    Auto,
    Force,
}

impl Default for RetpolineMode {
    fn default() -> Self {
        RetpolineMode::Auto
    }
}

pub struct RetpolineConfig {
    pub mode: RetpolineMode,
    pub ibpb_enabled: bool,
    pub ibrs_enabled: bool,
    pub ssbd_enabled: bool,
}

impl Default for RetpolineConfig {
    fn default() -> Self {
        Self {
            mode: RetpolineMode::Auto,
            ibpb_enabled: true,
            ibrs_enabled: true,
            ssbd_enabled: true,
        }
    }
}

pub fn detect_retpoline_needed() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            let mut eax: u32;
            let mut ebx: u32;
            let mut ecx: u32;
            let mut edx: u32;

            core::arch::asm!(
                "cpuid",
                inlateout("eax") 0 => eax,
                out("ebx") ebx,
                out("ecx") ecx,
                out("edx") edx,
                options(nostack)
            );

            let max_level = eax;

            if max_level < 7 {
                return false;
            }

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
                let has_ssb = (ebx & (1 << 4)) != 0;

                core::arch::asm!(
                    "cpuid",
                    inlateout("eax") 7, in("ecx") 0 => eax,
                    out("ebx") ebx,
                    out("ecx") ecx,
                    out("edx") edx,
                    options(nostack)
                );

                let has_ibpb = (edx & (1 << 26)) != 0;
                let has_stibp = (edx & (1 << 27)) != 0;

                let is_sandy_bridge = family == 6 && (full_model == 0x2A || full_model == 0x2D);
                let is_ivy_bridge = family == 6 && (full_model == 0x3A || full_model == 0x3E);
                let is_haswell = family == 6 && (full_model >= 0x3C && full_model <= 0x3F || full_model >= 0x45 && full_model <= 0x47);
                let is_broadwell = family == 6 && (full_model >= 0x3D && full_model <= 0x3F || full_model >= 0x46 && full_model <= 0x4F || full_model >= 0x56 && full_model <= 0x5F);
                let is_skylake = family == 6 && (full_model >= 0x4E && full_model <= 0x5E);
                let is_kabylake = family == 6 && (full_model == 0x8E || full_model == 0x9E);
                let is_coffee_lake = family == 6 && (full_model >= 0x9D && full_model <= 0x9F);

                let vulnerable = is_sandy_bridge || is_ivy_bridge || is_haswell || is_broadwell 
                    || is_skylake || is_kabylake || is_coffee_lake;

                return vulnerable && has_ibpb;
            }

            if is_amd {
                return true;
            }

            false
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

pub fn init_retpoline(config: RetpolineConfig) -> Result<(), &'static str> {
    let enabled = match config.mode {
        RetpolineMode::Disabled => false,
        RetpolineMode::Auto => detect_retpoline_needed(),
        RetpolineMode::Force => true,
    };

    RETPOLINE_ENABLED.store(enabled, Ordering::SeqCst);

    if !enabled {
        log::info!("Retpoline disabled by configuration");
        return Ok(());
    }

    log::info!("Retpoline enabled for Spectre v2 mitigation");

    #[cfg(target_arch = "x86_64")]
    {
        if config.ibpb_enabled {
            issue_ibpb();
            log::info!("IBPB enabled");
        }

        if config.ibrs_enabled {
            enable_ibrs();
            log::info!("IBRS enabled");
        }

        if config.ssbd_enabled {
            enable_ssbd();
            log::info!("SSBD enabled");
        }
    }

    Ok(())
}

#[cfg(target_arch = "x86_64")]
#[inline]
pub fn issue_ibpb() {
    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") 0x49,
            in("eax") 0,
            in("edx") 0,
            options(nostack)
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[inline]
pub fn enable_ibrs() {
    unsafe {
        let mut spec_ctrl: u64;
        core::arch::asm!(
            "rdmsr",
            inlateout("ecx") 0x48 => spec_ctrl,
            options(nostack)
        );

        spec_ctrl |= 1;

        core::arch::asm!(
            "wrmsr",
            inlateout("ecx") 0x48 => spec_ctrl,
            in("edx") 0,
            options(nostack)
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[inline]
pub fn disable_ibrs() {
    unsafe {
        let mut spec_ctrl: u64;
        core::arch::asm!(
            "rdmsr",
            inlateout("ecx") 0x48 => spec_ctrl,
            options(nostack)
        );

        spec_ctrl &= !1;

        core::arch::asm!(
            "wrmsr",
            inlateout("ecx") 0x48 => spec_ctrl,
            in("edx") 0,
            options(nostack)
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[inline]
pub fn enable_ssbd() {
    unsafe {
        let mut spec_ctrl: u64;
        core::arch::asm!(
            "rdmsr",
            inlateout("ecx") 0x48 => spec_ctrl,
            options(nostack)
        );

        spec_ctrl |= 1 << 2;

        core::arch::asm!(
            "wrmsr",
            inlateout("ecx") 0x48 => spec_ctrl,
            in("edx") 0,
            options(nostack)
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[inline]
pub fn disable_ssbd() {
    unsafe {
        let mut spec_ctrl: u64;
        core::arch::asm!(
            "rdmsr",
            inlateout("ecx") 0x48 => spec_ctrl,
            options(nostack)
        );

        spec_ctrl &= !(1 << 2);

        core::arch::asm!(
            "wrmsr",
            inlateout("ecx") 0x48 => spec_ctrl,
            in("edx") 0,
            options(nostack)
        );
    }
}

#[cfg(target_arch = "x86_64")]
#[inline]
pub unsafe fn retpoline_call(target: *const u8) {
    core::arch::asm!(
        "call {0}",
        "capture_spec:",
        "pause",
        "lfence",
        "jmp {1}",
        "2:",
        "mov [rsp + 8], {0}",
        "ret",
        in(reg) target,
        label capture_spec,
        options(nostack)
    );
}

#[cfg(target_arch = "x86_64")]
#[inline]
pub unsafe fn retpoline_jump(target: *const u8) {
    core::arch::asm!(
        "jmp {0}",
        "capture_spec:",
        "pause",
        "lfence",
        "jmp {1}",
        "2:",
        "mov [rsp + 8], {0}",
        "ret",
        in(reg) target,
        label capture_spec,
        options(nostack)
    );
}

#[cfg(target_arch = "x86_64")]
#[inline]
pub fn barrier_speculation() {
    unsafe {
        core::arch::asm!(
            "lfence",
            options(nostack)
        );
    }
}

#[inline]
pub fn entry_to_usermode_retpoline() {
    #[cfg(target_arch = "x86_64")]
    {
        barrier_speculation();
        unsafe {
            core::arch::asm!(
                "swapgs",
                options(nostack)
            );
        }
    }
}

#[inline]
pub fn entry_from_usermode_retpoline() {
    #[cfg(target_arch = "x86_64")]
    {
        barrier_speculation();
        unsafe {
            core::arch::asm!(
                "swapgs",
                options(nostack)
            );
        }
    }
}

pub fn is_retpoline_enabled() -> bool {
    RETPOLINE_ENABLED.load(Ordering::SeqCst)
}
