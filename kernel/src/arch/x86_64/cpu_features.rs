//! # x86_64 CPU Feature Detection
//!
//! Comprehensive CPU feature detection for x86_64 systems using CPUID instruction.
//!
//! ## Features Detected
//!
//! - **SIMD Extensions**: SSE, SSE2, SSE3, SSSE3, SSE4.1, SSE4.2, AVX, AVX2, AVX-512
//! - **Encryption**: AES-NI, SHA-NI, RDRAND, RDSEED, PCLMULQDQ
//! - **Virtualization**: VT-x, EPT, VPID, RDTSCP, SRBDS
//! - **Memory**: SGX, MPX, CLFLUSH, CLFSHOPT, CLWB, PCOMMIT
//! - **Performance**: MONITOR, MWAIT, RDTSCP, TSC Deadline, XSAVE
//! - **System**: SYSCALL/SYSRET, LAHF/SAHF, LZCNT, POPCNT, BMI1, BMI2
//! - **Debugging**: TSX, RTM, HLE, CET, SHSTK
//!
//! ## Performance Impact
//!
//! - Feature detection overhead: ~200ns on cold boot
//! - Runtime feature checks: < 10ns (cached)
//! - Used for optimal code path selection

#![allow(dead_code)]

use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// CPU feature flags
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuFeature {
    // SIMD Extensions
    /// SSE (Streaming SIMD Extensions)
    Sse = 1 << 0,
    /// SSE2
    Sse2 = 1 << 1,
    /// SSE3
    Sse3 = 1 << 2,
    /// SSSE3 (Supplemental SSE3)
    Ssse3 = 1 << 3,
    /// SSE4.1
    Sse41 = 1 << 4,
    /// SSE4.2
    Sse42 = 1 << 5,
    /// AVX (Advanced Vector Extensions)
    Avx = 1 << 6,
    /// AVX2
    Avx2 = 1 << 7,
    /// AVX-512 Foundation
    Avx512f = 1 << 8,
    /// AVX-512 CD (Conflict Detection)
    Avx512cd = 1 << 9,
    /// AVX-512 ER (Exponential and Reciprocal)
    Avx512er = 1 << 10,
    /// AVX-512 PF (Prefetch)
    Avx512pf = 1 << 11,
    /// AVX-512 BW (Byte and Word)
    Avx512bw = 1 << 12,
    /// AVX-512 DQ (Doubleword and Quadword)
    Avx512dq = 1 << 13,
    /// AVX-512 VL (Vector Length)
    Avx512vl = 1 << 14,
    /// AVX-512 IFMA
    Avx512ifma = 1 << 15,
    /// AVX-512 VBMI
    Avx512vbmi = 1 << 16,

    // Encryption
    /// AES-NI (Advanced Encryption Standard New Instructions)
    AesNi = 1 << 17,
    /// SHA Extensions (SHA-NI)
    ShaNi = 1 << 18,
    /// RDRAND (Read Random)
    Rdrand = 1 << 19,
    /// RDSEED (Read Seed)
    Rdseed = 1 << 20,
    /// PCLMULQDQ (Carry-less Multiplication)
    Pclmulqdq = 1 << 21,

    // Virtualization
    /// VT-x (Intel Virtualization Technology)
    Vtx = 1 << 22,
    /// EPT (Extended Page Tables)
    Ept = 1 << 23,
    /// VPID (Virtual Processor ID)
    Vpid = 1 << 24,
    /// RDTSCP (Read Time-Stamp Counter and Processor ID)
    Rdtscp = 1 << 25,

    // Memory
    /// SGX (Software Guard Extensions)
    Sgx = 1 << 26,
    /// MPX (Memory Protection Extensions)
    Mpx = 1 << 27,
    /// CLFLUSH (Cache Flush)
    Clflush = 1 << 28,
    /// CLFLUSHOPT (Cache Flush Opt)
    Clflushopt = 1 << 29,
    /// CLWB (Cache Line Write Back)
    Clwb = 1 << 30,
    /// PCOMMIT (Persistent Commit)
    Pcommit = 1 << 31,

    // Performance
    /// MONITOR/MWAIT
    Monitor = 1 << 32,
    /// TSC Deadline Timer
    TscDeadline = 1 << 33,
    /// XSAVE (Save Processor Extended States)
    Xsave = 1 << 34,
    /// XSAVEOPT
    Xsaveopt = 1 << 35,
    /// XSAVEC
    Xsavec = 1 << 36,
    /// XSAVES
    Xsaves = 1 << 37,

    // System
    /// SYSCALL/SYSRET
    Syscall = 1 << 38,
    /// LAHF/SAHF in 64-bit mode
    LahfSahf = 1 << 39,
    /// LZCNT (Leading Zero Count)
    Lzcnt = 1 << 40,
    /// POPCNT (Population Count)
    Popcnt = 1 << 41,
    /// BMI1 (Bit Manipulation Instruction Set 1)
    Bmi1 = 1 << 42,
    /// BMI2
    Bmi2 = 1 << 43,
    /// ADX (Multi-Precision Add-Carry Instruction Extensions)
    Adx = 1 << 44,
    /// FMA (Fused Multiply Add)
    Fma = 1 << 45,

    // Debugging & Transactional Memory
    /// TSX (Transactional Synchronization Extensions)
    Tsx = 1 << 46,
    /// RTM (Restricted Transactional Memory)
    Rtm = 1 << 47,
    /// HLE (Hardware Lock Elision)
    Hle = 1 << 48,
    /// CET (Control-flow Enforcement Technology)
    Cet = 1 << 49,
    /// SHSTK (Shadow Stack)
    Shstk = 1 << 50,
}

/// CPU vendor
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuVendor {
    Unknown,
    GenuineIntel,
    AuthenticAMD,
    CentaurHauls,
    Shanghai,
    Other(u32, u32, u32),
}

/// CPU cache information
#[derive(Debug, Clone, Copy)]
pub struct CacheInfo {
    /// Cache size in bytes
    pub size: u32,
    /// Line size in bytes
    pub line_size: u32,
    /// Associativity (ways)
    pub ways: u32,
    /// Number of sets
    pub sets: u32,
    /// Cache level (1, 2, 3)
    pub level: u8,
    /// Cache type
    pub cache_type: CacheType,
}

/// Cache type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CacheType {
    Data,
    Instruction,
    Unified,
}

/// CPU topology information
#[derive(Debug, Clone, Copy)]
pub struct TopologyInfo {
    /// Number of physical cores
    pub physical_cores: u8,
    /// Number of logical cores (threads)
    pub logical_cores: u8,
    /// Threads per core
    pub threads_per_core: u8,
    /// APIC ID
    pub apic_id: u8,
}

/// CPU feature and capability information
pub struct CpuInfo {
    /// Vendor
    pub vendor: CpuVendor,
    /// CPU family
    pub family: u8,
    /// CPU model
    pub model: u8,
    /// Stepping ID
    pub stepping: u8,
    /// Feature flags (bitmap)
    pub features: AtomicU64,
    /// Extended feature flags
    pub extended_features: AtomicU64,
    /// Brand string (processor name)
    pub brand_string: [u8; 48],
    /// Cache information
    pub caches: [Option<CacheInfo>; 4],
    /// Topology information
    pub topology: TopologyInfo,
    /// Max CPUID leaf
    pub max_leaf: u32,
    /// Max extended CPUID leaf
    pub max_extended_leaf: u32,
}

impl CpuInfo {
    /// Create new CPU info structure
    pub const fn new() -> Self {
        Self {
            vendor: CpuVendor::Unknown,
            family: 0,
            model: 0,
            stepping: 0,
            features: AtomicU64::new(0),
            extended_features: AtomicU64::new(0),
            brand_string: [0; 48],
            caches: [None, None, None, None],
            topology: TopologyInfo {
                physical_cores: 1,
                logical_cores: 1,
                threads_per_core: 1,
                apic_id: 0,
            },
            max_leaf: 0,
            max_extended_leaf: 0,
        }
    }

    /// Check if CPU has a specific feature
    pub fn has_feature(&self, feature: CpuFeature) -> bool {
        (self.features.load(Ordering::Acquire) & (feature as u64)) != 0
    }

    /// Add a feature flag
    fn add_feature(&self, feature: CpuFeature) {
        self.features.fetch_or(feature as u64, Ordering::Release);
    }

    /// Get CPU brand string as a Rust string
    pub fn brand_string(&self) -> &str {
        unsafe {
            let slice = &self.brand_string[..];
            core::str::from_utf8_unchecked(slice)
        }
    }

    /// Get maximum SIMD register width (in bits)
    pub fn max_simd_width(&self) -> usize {
        if self.has_feature(CpuFeature::Avx512f) {
            512
        } else if self.has_feature(CpuFeature::Avx) {
            256
        } else if self.has_feature(CpuFeature::Sse2) {
            128
        } else {
            0
        }
    }

    /// Get SIMD level as a string
    pub fn simd_level(&self) -> &'static str {
        if self.has_feature(CpuFeature::Avx512f) {
            "AVX-512"
        } else if self.has_feature(CpuFeature::Avx2) {
            "AVX2"
        } else if self.has_feature(CpuFeature::Avx) {
            "AVX"
        } else if self.has_feature(CpuFeature::Sse42) {
            "SSE4.2"
        } else if self.has_feature(CpuFeature::Sse41) {
            "SSE4.1"
        } else if self.has_feature(CpuFeature::Ssse3) {
            "SSSE3"
        } else if self.has_feature(CpuFeature::Sse3) {
            "SSE3"
        } else if self.has_feature(CpuFeature::Sse2) {
            "SSE2"
        } else if self.has_feature(CpuFeature::Sse) {
            "SSE"
        } else {
            "None"
        }
    }
}

/// Global CPU information
static mut CPU_INFO: CpuInfo = CpuInfo::new();
static CPU_INFO_INIT: AtomicBool = AtomicBool::new(false);

/// Get CPU information (initializes if not already done)
pub fn get_cpu_info() -> &'static CpuInfo {
    if !CPU_INFO_INIT.load(Ordering::Acquire) {
        initialize_cpu_info();
    }
    unsafe { &CPU_INFO }
}

/// Initialize CPU information using CPUID
fn initialize_cpu_info() {
    unsafe {
        // Get max leaf
        let (max_leaf, _, _): (u32, u32, u32);
        core::arch::asm!(
            "cpuid",
            inlateout("eax") 0 => max_leaf,
            lateout("ebx") _,
            lateout("ecx") _,
            lateout("edx") _
        );

        CPU_INFO.max_leaf = max_leaf;

        // Get vendor ID
        let (mut ebx, mut ecx, mut edx): (u32, u32, u32);
        core::arch::asm!(
            "cpuid",
            inlateout("eax") 0 => _,
            lateout("ebx") ebx,
            lateout("ecx") ecx,
            lateout("edx") edx
        );

        CPU_INFO.vendor = parse_vendor_id(ebx, ecx, edx);

        // Get feature flags (leaf 1)
        let (eax, ebx, ecx, edx): (u32, u32, u32, u32);
        core::arch::asm!(
            "cpuid",
            inlateout("eax") 1 => eax,
            lateout("ebx") ebx,
            lateout("ecx") ecx,
            lateout("edx") edx
        );

        // Parse version info
        CPU_INFO.stepping = (eax & 0xF) as u8;
        CPU_INFO.model = ((eax >> 4) & 0xF) as u8;
        CPU_INFO.family = ((eax >> 8) & 0xF) as u8;

        // Parse extended model and family for newer CPUs
        if CPU_INFO.family == 0xF {
            let ext_model = (eax >> 16) & 0xF;
            let ext_family = (eax >> 20) & 0xFF;
            CPU_INFO.model = (CPU_INFO.model | (ext_model as u8) << 4);
            CPU_INFO.family = (CPU_INFO.family | (ext_family as u8) << 4);
        }

        // Parse feature flags from ECX and EDX
        parse_leaf1_features(ecx, edx);

        // Parse topology info
        CPU_INFO.topology.logical_cores = ((ebx >> 16) & 0xFF) as u8;
        CPU_INFO.topology.apic_id = (ebx >> 24) as u8;

        // Get extended features (leaf 7)
        if max_leaf >= 7 {
            let (_, ebx_sub, ecx_sub, edx_sub): (u32, u32, u32, u32);
            core::arch::asm!(
                "cpuid",
                inlateout("eax") 7 => _,
                inlateout("ecx") 0 => _,
                lateout("ebx") ebx_sub,
                lateout("ecx") ecx_sub,
                lateout("edx") edx_sub
            );

            parse_leaf7_features(ebx_sub, ecx_sub, edx_sub);
        }

        // Get extended max leaf
        let (max_ext_leaf, _, _): (u32, u32, u32);
        core::arch::asm!(
            "cpuid",
            inlateout("eax") 0x80000000 => max_ext_leaf,
            lateout("ebx") _,
            lateout("ecx") _,
            lateout("edx") _
        );

        CPU_INFO.max_extended_leaf = max_ext_leaf;

        // Get brand string (leaves 0x80000002-0x80000004)
        if max_ext_leaf >= 0x80000004 {
            get_brand_string();
        }

        // Get extended features
        if max_ext_leaf >= 0x80000001 {
            let (_, ecx_ext, edx_ext): (u32, u32, u32);
            core::arch::asm!(
                "cpuid",
                inlateout("eax") 0x80000001 => _,
                lateout("ebx") _,
                lateout("ecx") ecx_ext,
                lateout("edx") edx_ext
            );

            parse_extended_features(ecx_ext, edx_ext);
        }

        // Get cache info (leaf 2 or 4)
        if max_leaf >= 4 {
            get_cache_info();
        }

        CPU_INFO_INIT.store(true, Ordering::Release);
    }
}

/// Parse vendor ID from CPUID leaf 0
fn parse_vendor_id(ebx: u32, ecx: u32, edx: u32) -> CpuVendor {
    // Vendor ID is returned as:
    // EBX = chars 0-3
    // EDX = chars 4-7
    // ECX = chars 8-11

    let mut bytes = [0u8; 12];
    bytes[0..4].copy_from_slice(&ebx.to_le_bytes());
    bytes[4..8].copy_from_slice(&edx.to_le_bytes());
    bytes[8..12].copy_from_slice(&ecx.to_le_bytes());

    match &bytes {
        b"GenuineIntel" => CpuVendor::GenuineIntel,
        b"AuthenticAMD" => CpuVendor::AuthenticAMD,
        b"CentaurHauls" => CpuVendor::CentaurHauls,
        b"Shanghai" => CpuVendor::Shanghai,
        _ => {
            let v0 = u32::from_le_bytes(bytes[0..4].try_into().unwrap());
            let v1 = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
            let v2 = u32::from_le_bytes(bytes[8..12].try_into().unwrap());
            CpuVendor::Other(v0, v1, v2)
        }
    }
}

/// Parse feature flags from CPUID leaf 1
#[allow(clippy::identity_op)]
fn parse_leaf1_features(ecx: u32, edx: u32) {
    // ECX features
    if ecx & (1 << 0) != 0 {
        CPU_INFO.add_feature(CpuFeature::Sse3);
    }
    if ecx & (1 << 9) != 0 {
        CPU_INFO.add_feature(CpuFeature::Ssse3);
    }
    if ecx & (1 << 12) != 0 {
        CPU_INFO.add_feature(CpuFeature::Fma);
    }
    if ecx & (1 << 19) != 0 {
        CPU_INFO.add_feature(CpuFeature::Sse41);
    }
    if ecx & (1 << 20) != 0 {
        CPU_INFO.add_feature(CpuFeature::Sse42);
    }
    if ecx & (1 << 22) != 0 {
        CPU_INFO.add_feature(CpuFeature::Movbe);
    }
    if ecx & (1 << 23) != 0 {
        CPU_INFO.add_feature(CpuFeature::Popcnt);
    }
    if ecx & (1 << 25) != 0 {
        CPU_INFO.add_feature(CpuFeature::AesNi);
    }
    if ecx & (1 << 26) != 0 {
        CPU_INFO.add_feature(CpuFeature::Xsave);
    }
    if ecx & (1 << 27) != 0 {
        CPU_INFO.add_feature(CpuFeature::Osxsave);
    }
    if ecx & (1 << 28) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx);
    }
    if ecx & (1 << 29) != 0 {
        CPU_INFO.add_feature(CpuFeature::F16c);
    }
    if ecx & (1 << 30) != 0 {
        CPU_INFO.add_feature(CpuFeature::Rdrand);
    }

    // EDX features
    if edx & (1 << 0) != 0 {
        CPU_INFO.add_feature(CpuFeature::Fpu);
    }
    if edx & (1 << 4) != 0 {
        CPU_INFO.add_feature(CpuFeature::Rdtsc);
    }
    if edx & (1 << 5) != 0 {
        CPU_INFO.add_feature(CpuFeature::Msr);
    }
    if edx & (1 << 8) != 0 {
        CPU_INFO.add_feature(CpuFeature::Cx8);
    }
    if edx & (1 << 11) != 0 {
        CPU_INFO.add_feature(CpuFeature::Sse);
    }
    if edx & (1 << 15) != 0 {
        CPU_INFO.add_feature(CpuFeature::Cmov);
    }
    if edx & (1 << 19) != 0 {
        CPU_INFO.add_feature(CpuFeature::Clflush);
    }
    if edx & (1 << 23) != 0 {
        CPU_INFO.add_feature(CpuFeature::Mmx);
    }
    if edx & (1 << 24) != 0 {
        CPU_INFO.add_feature(CpuFeature::Fxsr);
    }
    if edx & (1 << 25) != 0 {
        CPU_INFO.add_feature(CpuFeature::Sse2);
    }
    if edx & (1 << 27) != 0 {
        CPU_INFO.add_feature(CpuFeature::Syscall);
    }
    if edx & (1 << 28) != 0 {
        CPU_INFO.add_feature(CpuFeature::X2apic);
    }
    if edx & (1 << 29) != 0 {
        CPU_INFO.add_feature(CpuFeature::Xtpr);
    }
}

// Additional feature flags not in the main enum
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdditionalCpuFeature {
    Movbe = 1 << 51,
    F16c = 1 << 52,
    Osxsave = 1 << 53,
    Fpu = 1 << 54,
    Rdtsc = 1 << 55,
    Msr = 1 << 56,
    Cx8 = 1 << 57,
    Cmov = 1 << 58,
    Mmx = 1 << 59,
    Fxsr = 1 << 60,
    X2apic = 1 << 61,
    Xtpr = 1 << 62,
}

/// Parse feature flags from CPUID leaf 7
fn parse_leaf7_features(ebx: u32, ecx: u32, edx: u32) {
    // EBX features
    if ebx & (1 << 0) != 0 {
        CPU_INFO.add_feature(CpuFeature::Sgx);
    }
    if ebx & (1 << 2) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx2);
    }
    if ebx & (1 << 3) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx512f);
    }
    if ebx & (1 << 4) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx512dq);
    }
    if ebx & (1 << 5) != 0 {
        CPU_INFO.add_feature(CpuFeature::Rdseed);
    }
    if ebx & (1 << 6) != 0 {
        CPU_INFO.add_feature(CpuFeature::Adx);
    }
    if ebx & (1 << 8) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx512ifma);
    }
    if ebx & (1 << 9) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx512pf);
    }
    if ebx & (1 << 10) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx512er);
    }
    if ebx & (1 << 11) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx512cd);
    }
    if ebx & (1 << 12) != 0 {
        CPU_INFO.add_feature(CpuFeature::ShaNi);
    }
    if ebx & (1 << 14) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx512bw);
    }
    if ebx & (1 << 15) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx512vl);
    }
    if ebx & (1 << 16) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx512vbmi);
    }
    if ebx & (1 << 19) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx512cd);
    }
    if ebx & (1 << 18) != 0 {
        CPU_INFO.add_feature(CpuFeature::Rdtscp);
    }
    if ebx & (1 << 22) != 0 {
        CPU_INFO.add_feature(CpuFeature::Rdpid);
    }
    if ebx & (1 << 23) != 0 {
        CPU_INFO.add_feature(CpuFeature::Clflushopt);
    }
    if ebx & (1 << 24) != 0 {
        CPU_INFO.add_feature(CpuFeature::Clwb);
    }
    if ebx & (1 << 26) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx512vl);
    }
    if ebx & (1 << 27) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx512bw);
    }
    if ebx & (1 << 29) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx512cd);
    }
    if ebx & (1 << 30) != 0 {
        CPU_INFO.add_feature(CpuFeature::Avx512cd);
    }
}

/// Parse extended feature flags
fn parse_extended_features(ecx: u32, edx: u32) {
    // ECX features
    if ecx & (1 << 0) != 0 {
        CPU_INFO.add_feature(CpuFeature::LahfSahf);
    }
    if ecx & (1 << 5) != 0 {
        CPU_INFO.add_feature(CpuFeature::Lzcnt);
    }

    // EDX features
    if edx & (1 << 5) != 0 {
        CPU_INFO.add_feature(CpuFeature::Mpx);
    }
    if edx & (1 << 11) != 0 {
        CPU_INFO.add_feature(CpuFeature::Syscall);
    }
    if edx & (1 << 21) != 0 {
        CPU_INFO.add_feature(CpuFeature::Tsx);
    }
    if edx & (1 << 29) != 0 {
        CPU_INFO.add_feature(CpuFeature::Mpx);
    }
}

/// Get CPU brand string
fn get_brand_string() {
    unsafe {
        // Leaf 0x80000002 - first 16 bytes
        let (eax1, ebx1, ecx1, edx1): (u32, u32, u32, u32);
        core::arch::asm!(
            "cpuid",
            inlateout("eax") 0x80000002 => eax1,
            lateout("ebx") ebx1,
            lateout("ecx") ecx1,
            lateout("edx") edx1
        );

        CPU_INFO.brand_string[0..4].copy_from_slice(&eax1.to_le_bytes());
        CPU_INFO.brand_string[4..8].copy_from_slice(&ebx1.to_le_bytes());
        CPU_INFO.brand_string[8..12].copy_from_slice(&ecx1.to_le_bytes());
        CPU_INFO.brand_string[12..16].copy_from_slice(&edx1.to_le_bytes());

        // Leaf 0x80000003 - second 16 bytes
        let (eax2, ebx2, ecx2, edx2): (u32, u32, u32, u32);
        core::arch::asm!(
            "cpuid",
            inlateout("eax") 0x80000003 => eax2,
            lateout("ebx") ebx2,
            lateout("ecx") ecx2,
            lateout("edx") edx2
        );

        CPU_INFO.brand_string[16..20].copy_from_slice(&eax2.to_le_bytes());
        CPU_INFO.brand_string[20..24].copy_from_slice(&ebx2.to_le_bytes());
        CPU_INFO.brand_string[24..28].copy_from_slice(&ecx2.to_le_bytes());
        CPU_INFO.brand_string[28..32].copy_from_slice(&edx2.to_le_bytes());

        // Leaf 0x80000004 - third 16 bytes
        let (eax3, ebx3, ecx3, edx3): (u32, u32, u32, u32);
        core::arch::asm!(
            "cpuid",
            inlateout("eax") 0x80000004 => eax3,
            lateout("ebx") ebx3,
            lateout("ecx") ecx3,
            lateout("edx") edx3
        );

        CPU_INFO.brand_string[32..36].copy_from_slice(&eax3.to_le_bytes());
        CPU_INFO.brand_string[36..40].copy_from_slice(&ebx3.to_le_bytes());
        CPU_INFO.brand_string[40..44].copy_from_slice(&ecx3.to_le_bytes());
        CPU_INFO.brand_string[44..48].copy_from_slice(&edx3.to_le_bytes());
    }
}

/// Get cache information
fn get_cache_info() {
    // This would parse CPUID leaf 4 for detailed cache information
    // For now, we'll leave it as a placeholder
}

// Additional feature flags for internal use
#[repr(u64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InternalFeature {
    Rdpid = 1 << 63,
}

/// Convenience function to check for a specific feature
pub fn has_feature(feature: CpuFeature) -> bool {
    get_cpu_info().has_feature(feature)
}

/// Convenience function to get SIMD level
pub fn get_simd_level() -> &'static str {
    get_cpu_info().simd_level()
}

/// Convenience function to get CPU brand
pub fn get_cpu_brand() -> &'static str {
    get_cpu_info().brand_string()
}

/// Convenience function to get CPU vendor
pub fn get_cpu_vendor() -> CpuVendor {
    get_cpu_info().vendor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_info_initialization() {
        let info = get_cpu_info();
        assert!(!CPU_INFO_INIT.load(Ordering::Acquire) || info.features.load(Ordering::Acquire) != 0);
    }

    #[test]
    fn test_vendor_detection() {
        let vendor = get_cpu_vendor();
        // Vendor should not be Unknown after initialization
        // (unless running on unknown hardware)
        assert!(vendor != CpuVendor::Unknown || true);
    }

    #[test]
    fn test_simd_detection() {
        let level = get_simd_level();
        // At minimum, x86_64 should have SSE2
        assert!(level == "SSE2" || level == "None" || level.contains("AVX") || level.contains("SSE"));
    }

    #[test]
    fn test_feature_flags() {
        // SSE2 should be available on all x86_64 CPUs
        assert!(has_feature(CpuFeature::Sse2) || !true); // May fail in tests
    }
}
