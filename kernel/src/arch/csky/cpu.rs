//! C-SKY CPU feature detection and initialization
//!
//! This module provides CPU feature detection for C-SKY processors,
//! including identification of processor type, cache configuration,
//! and extended instruction set support.

use core::sync::atomic::AtomicU64;

/// CPU feature flags
pub mod features {
    /// DSP extensions
    pub const CSKY_FEATURE_DSP: u64 = 1 << 0;
    /// DSP version 2
    pub const CSKY_FEATURE_DSP2: u64 = 1 << 1;
    /// Floating point unit
    pub const CSKY_FEATURE_FPU: u64 = 1 << 2;
    /// FPU version 2/3
    pub const CSKY_FEATURE_FPUV2: u64 = 1 << 3;
    /// Vector DSP
    pub const CSKY_FEATURE_VDSP: u64 = 1 << 4;
    /// VDSP version 2
    pub const CSKY_FEATURE_VDSP2: u64 = 1 << 5;
    /// VDSP version 3
    pub const CSKY_FEATURE_VDSP3: u64 = 1 << 6;
    /// Cache coherency
    pub const CSKY_FEATURE_CC: u64 = 1 << 7;
    /// High performance vectors
    pub const CSKY_FEATURE_HP: u64 = 1 << 8;
    /// Memory management unit
    pub const CSKY_FEATURE_MMU: u64 = 1 << 9;
    /// Write protection
    pub const CSKY_FEATURE_WP: u64 = 1 << 10;
    /// Optional atomic instructions
    pub const CSKY_FEATURE_ATOMICS: u64 = 1 << 11;
    /// Java extensions
    pub const CSKY_FEATURE_JAVA: u64 = 1 << 12;
    /// TrustZone
    pub const CSKY_FEATURE_TRUST: u64 = 1 << 13;
    /// Cache locking
    pub const CSKY_FEATURE_CACHELOCK: u64 = 1 << 14;
    /// Clock gating
    pub const CSKY_FEATURE_CLOCKGATING: u64 = 1 << 15;
    /// 16-bit instruction encoding
    pub const CSKY_FEATURE_E1: u64 = 1 << 16;
    /// TrustZone 2
    pub const CSKY_FEATURE_TRUST2: u64 = 1 << 17;
    /// 3D DSP extensions
    pub const CSKY_FEATURE_DSP3: u64 = 1 << 18;
    /// Loop buffer
    pub const CSKY_FEATURE_LOOPBUF: u64 = 1 << 19;
}

/// CPU information structure
#[derive(Debug, Clone)]
pub struct CpuInfo {
    /// Processor ID register value
    pub cpid: u32,
    /// Processor version
    pub version: u16,
    /// Processor revision
    pub revision: u8,
    /// Processor family
    pub family: u8,
    /// Processor name
    pub processor_name: &'static str,
    /// Feature flags
    pub features: u64,
    /// L1 instruction cache size (KB)
    pub l1i_size: u32,
    /// L1 data cache size (KB)
    pub l1d_size: u32,
    /// L2 cache size (KB)
    pub l2_size: u32,
    /// Cache line size (bytes)
    pub cache_line_size: u32,
    /// Number of CPUs
    pub cpu_count: u32,
    /// CPU frequency (MHz)
    pub cpu_freq_mhz: u32,
}

/// Global CPU information
static mut CPU_INFO: Option<CpuInfo> = None;

/// Initialize CPU subsystem and detect features
pub fn init_cpu() {
    let cpid = unsafe { read_cpid() };
    let version = ((cpid >> 16) & 0xFFFF) as u16;
    let revision = (cpid & 0xFF) as u8;
    let family = ((cpid >> 8) & 0xFF) as u8;

    let features = detect_features();
    let (l1i_size, l1d_size, l2_size, cache_line_size) = get_cache_info();
    let processor_name = get_processor_name(cpid);
    let cpu_freq_mhz = get_cpu_frequency();

    let info = CpuInfo {
        cpid,
        version,
        revision,
        family,
        processor_name,
        features,
        l1i_size,
        l1d_size,
        l2_size,
        cache_line_size,
        cpu_count: detect_cpu_count(),
        cpu_freq_mhz,
    };

    unsafe {
        CPU_INFO = Some(info.clone());
    }

    crate::println!("C-SKY: CPU initialized: {}", processor_name);
    crate::println!("C-SKY: Features: {:#x}", features);
}

/// Get CPU information
pub fn get_cpu_info() -> &'static CpuInfo {
    unsafe {
        CPU_INFO.as_ref().expect("CPU not initialized")
    }
}

/// Read Processor ID register
#[inline]
unsafe fn read_cpid() -> u32 {
    let cpid: u32;
    unsafe {
        core::arch::asm!(
            "mfcr {0}, cr13",
            out(reg) cpid,
            options(nostack, nomem)
        );
    }
    cpid
}

/// Detect CPU features
fn detect_features() -> u64 {
    let mut features = 0u64;

    // Read C-CPUCFG registers for feature detection
    unsafe {
        // Read CPUCFG register 1 (processor features)
        let cfg1: u32;
        core::arch::asm!(
            "mfcr {0}, cr4<1, 0>", // C_CPUCFG0
            out(reg) cfg1,
            options(nostack, nomem)
        );

        if cfg1 & 0x01 != 0 {
            features |= features::CSKY_FEATURE_MMU;
        }
        if cfg1 & 0x02 != 0 {
            features |= features::CSKY_FEATURE_FPU;
        }
        if cfg1 & 0x04 != 0 {
            features |= features::CSKY_FEATURE_DSP;
        }
        if cfg1 & 0x08 != 0 {
            features |= features::CSKY_FEATURE_VDSP;
        }
        if cfg1 & 0x10 != 0 {
            features |= features::CSKY_FEATURE_CC;
        }
        if cfg1 & 0x20 != 0 {
            features |= features::CSKY_FEATURE_E1;
        }

        // Always present in C-SKY v2
        features |= features::CSKY_FEATURE_ATOMICS;
    }

    features
}

/// Get cache information
fn get_cache_info() -> (u32, u32, u32, u32) {
    unsafe {
        // Read cache configuration registers
        let ccr0: u32;
        let ccr1: u32;

        core::arch::asm!(
            "mfcr {0}, cr0<17, 15>", // CPUCFG_CCR0
            out(reg) ccr0,
            options(nostack, nomem)
        );
        core::arch::asm!(
            "mfcr {0}, cr1<17, 15>", // CPUCFG_CCR1
            out(reg) ccr1,
            options(nostack, nomem)
        );

        // Parse cache sizes from registers
        let l1i_size = ((ccr0 >> 20) & 0x3F) * 16; // KB
        let l1d_size = ((ccr0 >> 14) & 0x3F) * 16; // KB
        let l2_size = if ccr1 != 0 {
            ((ccr1 >> 20) & 0x3F) * 64 // KB
        } else {
            0
        };

        // Cache line size is typically 32 bytes for C-SKY
        let cache_line_size = 32;

        (l1i_size, l1d_size, l2_size, cache_line_size)
    }
}

/// Get processor name from CPID
fn get_processor_name(cpid: u32) -> &'static str {
    let core_id = (cpid >> 16) & 0xFFFF;

    match core_id {
        0x5820..=0x582F => "CK801",
        0x5830..=0x583F => "CK802",
        0x5840..=0x584F => "CK803",
        0x5850..=0x585F => "CK805",
        0x5860..=0x586F => "CK807",
        0x5870..=0x587F => "CK810",
        0x5880..=0x588F => "CK810V",
        0x5890..=0x589F => "CK860",
        0x58A0..=0x58AF => "CK862",
        0x58B0..=0x58BF => "CK870",
        0x58C0..=0x58CF => "CK872",
        _ => "Unknown C-SKY Processor",
    }
}

/// Detect CPU count
fn detect_cpu_count() -> u32 {
    // TODO: Implement proper CPU count detection
    1
}

/// Get CPU frequency
fn get_cpu_frequency() -> u32 {
    // TODO: Implement proper CPU frequency detection
    800 // 800 MHz default
}

/// Check if CPU supports a specific feature
pub fn has_feature(feature: u64) -> bool {
    let info = get_cpu_info();
    (info.features & feature) != 0
}

/// Check if CPU has DSP
pub fn has_dsp() -> bool {
    has_feature(features::CSKY_FEATURE_DSP)
}

/// Check if CPU has FPU
pub fn has_fpu() -> bool {
    has_feature(features::CSKY_FEATURE_FPU)
}

/// Check if CPU has MMU
pub fn has_mmu() -> bool {
    has_feature(features::CSKY_FEATURE_MMU)
}

/// Enable CPU features
pub fn enable_features() {
    unsafe {
        // Enable FPU if present
        if has_fpu() {
            // Enable floating point unit
            core::arch::asm!(
                "mtcr {0}, cr0<1, 0>",
                in(reg) 0x00, // Enable FPU
                options(nostack, nomem)
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_info() {
        let info = get_cpu_info();
        assert!(info.features != 0);
    }

    #[test]
    fn test_processor_name() {
        let info = get_cpu_info();
        assert!(!info.processor_name.is_empty());
    }
}
