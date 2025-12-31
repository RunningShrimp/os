//! LoongArch CPU feature detection and initialization
//!
//! This module provides CPU feature detection for LoongArch processors,
//! including identification of processor type, cache configuration,
//! and extended instruction set support.

use core::sync::atomic::{AtomicU64, Ordering};

/// CPU feature flags
pub mod features {
    /// LSX 128-bit SIMD support
    pub const LA_FEATURE_LSX: u64 = 1 << 0;
    /// LASX 256-bit SIMD support
    pub const LA_FEATURE_LASX: u64 = 1 << 1;
    /// LSX/LASX context in SW
    pub const LA_FEATURE_LAM_BH: u64 = 1 << 2;
    /// Complex instructions
    pub const LA_FEATURE_COMPLEX: u64 = 1 << 3;
    /// Crypto instructions
    pub const LA_FEATURE_CRYPTO: u64 = 1 << 4;
    /// LVZ 128-bit vector instructions
    pub const LA_FEATURE_LVZ: u64 = 1 << 5;
    /// LBT x86 binary translation
    pub const LA_FEATURE_LBT_X86: u64 = 1 << 6;
    /// LBT ARM binary translation
    pub const LA_FEATURE_LBT_ARM: u64 = 1 << 7;
    /// LBT MIPS binary translation
    pub const LA_FEATURE_LBT_MIPS: u64 = 1 << 8;
    /// Page table walk
    pub const LA_FEATURE_PTW: u64 = 1 << 9;
    /// USCA (User Space Cache Access)
    pub const LA_FEATURE_USCA: u64 = 1 << 10;
    /// AVE (Audio and Video Engine)
    pub const LA_FEATURE_AVE: u64 = 1 << 11;
    /// Base ISA extensions (FP, SP, etc.)
    pub const LA_FEATURE_FP: u64 = 1 << 12;
    /// Single precision float
    pub const LA_FEATURE_SP: u64 = 1 << 13;
    /// Double precision float
    pub const LA_FEATURE_DP: u64 = 1 << 14;
    /// Memory consistency model
    pub const LA_FEATURE_WEAK_ORDERING: u64 = 1 << 15;
    /// SMP (Simultaneous Multithreading) support
    pub const LA_FEATURE_SMP: u64 = 1 << 16;
    /// Hardware virtualization
    pub const LA_FEATURE_VIRT: u64 = 1 << 17;
    /// Per-address space ASIDs
    pub const LA_FEATURE_ASID: u64 = 1 << 18;
    /// Page table walk (hardware page table walker)
    pub const LA_FEATURE_HPTW: u64 = 1 << 19;
    /// Standard L1 cache line size
    pub const LA_FEATURE_LL_SC: u64 = 1 << 20;
    /// 32-bit address space support
    pub const LA_FEATURE_A32: u64 = 1 << 21;
    /// 64-bit address space support
    pub const LA_FEATURE_A64: u64 = 1 << 22;
    /// Privileged instruction set
    pub const LA_FEATURE_PRIV: u64 = 1 << 23;
    /// Unaligned access support
    pub const LA_FEATURE_UAL: u64 = 1 << 24;
}

/// CPU information structure
#[derive(Debug, Clone)]
pub struct CpuInfo {
    /// Processor ID register value
    pub prid: u32,
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
    /// L3 cache size (KB)
    pub l3_size: u32,
    /// Cache line size (bytes)
    pub cache_line_size: u32,
    /// Number of CPUs
    pub cpu_count: u32,
    /// CPU frequency (MHz)
    pub cpu_freq_mhz: u32,
}

/// Global CPU information (initialized once)
static mut CPU_INFO: Option<CpuInfo> = None;

/// Initialize CPU subsystem and detect features
pub fn init_cpu() {
    let prid = unsafe { read_prid() };
    let version = ((prid >> 16) & 0xFFFF) as u16;
    let revision = (prid & 0xFF) as u8;
    let family = ((prid >> 8) & 0xFF) as u8;

    // Detect CPU features
    let features = detect_features();

    // Get cache information
    let (l1i_size, l1d_size, l2_size, l3_size, cache_line_size) = get_cache_info();

    // Determine processor name
    let processor_name = get_processor_name(prid);

    let cpu_freq_mhz = get_cpu_frequency();

    let info = CpuInfo {
        prid,
        version,
        revision,
        family,
        processor_name,
        features,
        l1i_size,
        l1d_size,
        l2_size,
        l3_size,
        cache_line_size,
        cpu_count: detect_cpu_count(),
        cpu_freq_mhz,
    };

    // Store CPU info
    unsafe {
        CPU_INFO = Some(info.clone());
    }

    crate::println!("LoongArch64: CPU initialized: {}", processor_name);
    crate::println!("LoongArch64: Features: {:#x}", features);
}

/// Get CPU information
pub fn get_cpu_info() -> &'static CpuInfo {
    unsafe {
        CPU_INFO.as_ref().expect("CPU not initialized")
    }
}

/// Read Processor ID register
///
/// # Returns
///
/// The value of the PRID register
#[inline]
unsafe fn read_prid() -> u32 {
    let prid: u32;
    unsafe {
        core::arch::asm!(
            "csrrd {0}, 0x0",  // PRID register
            out(reg) prid,
            options(nostack, nomem)
        );
    }
    prid
}

/// Detect CPU features from CPUCFG registers
fn detect_features() -> u64 {
    let mut features = 0u64;

    unsafe {
        // Check CPUCFG registers for features
        // CPUCFG register 0-1: Basic ISA features
        let cfg0 = super::read_cpucfg(0);
        let cfg1 = super::read_cpucfg(1);

        if cfg0 & 0x01 != 0 {
            features |= features::LA_FEATURE_FP;
        }
        if cfg0 & 0x02 != 0 {
            features |= features::LA_FEATURE_SP;
        }
        if cfg0 & 0x04 != 0 {
            features |= features::LA_FEATURE_DP;
        }

        // Check for LSX/LASX
        if cfg0 & 0x10 != 0 {
            features |= features::LA_FEATURE_LSX;
        }
        if cfg0 & 0x20 != 0 {
            features |= features::LA_FEATURE_LASX;
        }

        // Check for crypto extensions
        if cfg0 & 0x100 != 0 {
            features |= features::LA_FEATURE_CRYPTO;
        }

        // Check for complex instructions
        if cfg1 & 0x01 != 0 {
            features |= features::LA_FEATURE_COMPLEX;
        }

        // Check for virtualization
        if cfg1 & 0x04 != 0 {
            features |= features::LA_FEATURE_VIRT;
        }

        // Check for SMP
        if cfg1 & 0x08 != 0 {
            features |= features::LA_FEATURE_SMP;
        }

        // Always present in LoongArch64
        features |= features::LA_FEATURE_A64;
        features |= features::LA_FEATURE_LL_SC;
        features |= features::LA_FEATURE_PRIV;
    }

    features
}

/// Get cache information from CPUCFG
fn get_cache_info() -> (u32, u32, u32, u32, u32) {
    unsafe {
        // CPUCFG register 16-19: Cache information
        let cfg16 = super::read_cpucfg(16); // L1 I-cache
        let cfg17 = super::read_cpucfg(17); // L1 D-cache
        let cfg18 = super::read_cpucfg(18); // L2 cache
        let cfg19 = super::read_cpucfg(19); // L3 cache

        let l1i_size = ((cfg16 >> 16) & 0xFFFF) * 4; // Convert to KB
        let l1d_size = ((cfg17 >> 16) & 0xFFFF) * 4;
        let l2_size = if cfg18 != 0 {
            ((cfg18 >> 16) & 0xFFFF) * 4
        } else {
            0
        };
        let l3_size = if cfg19 != 0 {
            ((cfg19 >> 16) & 0xFFFF) * 4
        } else {
            0
        };

        // Cache line size (typically 32 or 64 bytes)
        let cache_line_size = 32; // Default for LoongArch

        (l1i_size, l1d_size, l2_size, l3_size, cache_line_size)
    }
}

/// Get processor name from PRID
fn get_processor_name(prid: u32) -> &'static str {
    let core_id = (prid >> 16) & 0xFFFF;

    match core_id {
        0x14C0..=0x14CF => "Loongson 3A5000 (LA464)",
        0x14D0..=0x14DF => "Loongson 3A6000 (LA664)",
        0x14A0..=0x14AF => "Loongson 3C5000 (LA464)",
        0x14B0..=0x14BF => "Loongson 3C6000 (LA664)",
        0x14E0..=0x14EF => "Loongson 2K2000",
        0x14F0..=0x14FF => "Loongson 2K3000",
        _ => "Unknown Loongson Processor",
    }
}

/// Detect number of CPUs in the system
fn detect_cpu_count() -> u32 {
    // Try to get CPU count from firmware/device tree
    // For now, assume single CPU
    1
}

/// Get CPU frequency
fn get_cpu_frequency() -> u32 {
    // Try to get CPU frequency from CPUCFG or firmware
    // For now, return a reasonable default
    2000 // 2 GHz default
}

/// Check if CPU supports a specific feature
///
/// # Arguments
///
/// * `feature` - Feature flag to check
///
/// # Returns
///
/// `true` if the feature is supported, `false` otherwise
pub fn has_feature(feature: u64) -> bool {
    let info = get_cpu_info();
    (info.features & feature) != 0
}

/// Check if CPU supports LSX (128-bit SIMD)
pub fn has_lsx() -> bool {
    has_feature(features::LA_FEATURE_LSX)
}

/// Check if CPU supports LASX (256-bit SIMD)
pub fn has_lasx() -> bool {
    has_feature(features::LA_FEATURE_LASX)
}

/// Check if CPU supports crypto extensions
pub fn has_crypto() -> bool {
    has_feature(features::LA_FEATURE_CRYPTO)
}

/// Check if CPU supports virtualization
pub fn has_virtualization() -> bool {
    has_feature(features::LA_FEATURE_VIRT)
}

/// Enable CPU features (e.g., enable floating point)
pub fn enable_features() {
    unsafe {
        // Enable floating point
        core::arch::asm!(
            "movfc {0}, $fcsr0",
            out(reg) _,
            "movfc $fcsr0, {0}",
            in(reg) 0x1f, // Enable all exceptions
            options(nostack, nomem)
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_feature_detection() {
        // Basic smoke test for feature detection
        let info = get_cpu_info();
        assert!(info.features != 0, "Should have some features");
        assert!(info.features & features::LA_FEATURE_A64 != 0, "Should be 64-bit");
    }

    #[test]
    fn test_cache_info() {
        let info = get_cpu_info();
        assert!(info.l1i_size > 0, "Should have L1 instruction cache");
        assert!(info.l1d_size > 0, "Should have L1 data cache");
        assert!(info.cache_line_size > 0, "Should have cache line size");
    }

    #[test]
    fn test_processor_name() {
        let info = get_cpu_info();
        assert!(!info.processor_name.is_empty(), "Should have processor name");
    }
}
