// x86_64 CPUID instruction wrapper and feature detection

#[derive(Clone, Copy)]
pub struct CpuidResult {
    pub eax: u32,
    pub ebx: u32,
    pub ecx: u32,
    pub edx: u32,
}

impl CpuidResult {
    pub fn new(eax: u32, ebx: u32, ecx: u32, edx: u32) -> Self {
        Self { eax, ebx, ecx, edx }
    }
}

pub fn cpuid(leaf: u32) -> CpuidResult {
    cpuid_subleaf(leaf, 0)
}

pub fn cpuid_subleaf(leaf: u32, subleaf: u32) -> CpuidResult {
    unsafe {
        let (eax, ebx, ecx, edx);
        core::arch::asm!(
            "cpuid",
            inout("eax") leaf => eax,
            out("ebx") ebx,
            inout("ecx") subleaf => ecx,
            out("edx") edx,
            options(nostack, preserves_flags)
        );
        CpuidResult::new(eax, ebx, ecx, edx)
    }
}

pub struct CpuFeatures {
    pub vendor_id: [u8; 12],
    pub has_msr: bool,
    pub has_apic: bool,
    pub has_pse: bool,
    pub has_pge: bool,
    pub has_pae: bool,
    pub has_cx8: bool,
    pub has_cmov: bool,
    pub has_clflush: bool,
    pub has_mmx: bool,
    pub has_sse: bool,
    pub has_sse2: bool,
    pub has_sse3: bool,
    pub has_ssse3: bool,
    pub has_sse41: bool,
    pub has_sse42: bool,
    pub has_avx: bool,
    pub has_avx2: bool,
    pub has_lm: bool,
    pub has_nx: bool,
    pub has_syscall: bool,
    pub max_cpuid: u32,
}

impl CpuFeatures {
    pub fn detect() -> Self {
        let mut features = Self {
            vendor_id: [0; 12],
            has_msr: false,
            has_apic: false,
            has_pse: false,
            has_pge: false,
            has_pae: false,
            has_cx8: false,
            has_cmov: false,
            has_clflush: false,
            has_mmx: false,
            has_sse: false,
            has_sse2: false,
            has_sse3: false,
            has_ssse3: false,
            has_sse41: false,
            has_sse42: false,
            has_avx: false,
            has_avx2: false,
            has_lm: false,
            has_nx: false,
            has_syscall: false,
            max_cpuid: 0,
        };

        // Get vendor ID
        let result = cpuid(0);
        features.max_cpuid = result.eax;
        features.vendor_id[0..4].copy_from_slice(&result.ebx.to_le_bytes());
        features.vendor_id[4..8]
            .copy_from_slice(&result.edx.to_le_bytes());
        features.vendor_id[8..12]
            .copy_from_slice(&result.ecx.to_le_bytes());

        // Get standard features
        if features.max_cpuid >= 1 {
            let result = cpuid(1);
            features.has_cx8 = (result.edx & (1 << 8)) != 0;
            features.has_cmov = (result.edx & (1 << 15)) != 0;
            features.has_msr = (result.edx & (1 << 5)) != 0;
            features.has_pae = (result.edx & (1 << 6)) != 0;
            features.has_apic = (result.edx & (1 << 9)) != 0;
            features.has_pse = (result.edx & (1 << 3)) != 0;
            features.has_pge = (result.edx & (1 << 13)) != 0;
            features.has_clflush = (result.edx & (1 << 19)) != 0;
            features.has_mmx = (result.edx & (1 << 23)) != 0;
            features.has_sse = (result.edx & (1 << 25)) != 0;
            features.has_sse2 = (result.edx & (1 << 26)) != 0;
            features.has_sse3 = (result.ecx & 0x1) != 0;
            features.has_ssse3 = (result.ecx & (1 << 9)) != 0;
            features.has_sse41 = (result.ecx & (1 << 19)) != 0;
            features.has_sse42 = (result.ecx & (1 << 20)) != 0;
            features.has_avx = (result.ecx & (1 << 28)) != 0;
        }

        // Get extended features
        let ext_result = cpuid(0x80000000);
        if ext_result.eax >= 0x80000001 {
            let result = cpuid(0x80000001);
            features.has_lm = (result.edx & (1 << 29)) != 0;
            features.has_nx = (result.edx & (1 << 20)) != 0;
            features.has_syscall = (result.edx & (1 << 11)) != 0;
        }

        // Check AVX2
        if features.has_avx && features.max_cpuid >= 7 {
            let result = cpuid_subleaf(7, 0);
            features.has_avx2 = (result.ebx & (1 << 5)) != 0;
        }

        features
    }

    pub fn print_summary(&self) {
        crate::console::write_str("CPU Features:\n");
        crate::console::write_str("  Long Mode: ");
        crate::console::write_str(if self.has_lm { "YES\n" } else { "NO\n" });
        crate::console::write_str("  SSE: ");
        crate::console::write_str(if self.has_sse2 {
            "SSE2+"
        } else if self.has_sse {
            "SSE"
        } else {
            "NO"
        });
        crate::console::write_str("\n");
        crate::console::write_str("  AVX: ");
        crate::console::write_str(if self.has_avx2 {
            "AVX2"
        } else if self.has_avx {
            "AVX"
        } else {
            "NO"
        });
        crate::console::write_str("\n");
    }
}
