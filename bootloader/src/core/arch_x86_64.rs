/// x86_64 Architecture Optimization Module
/// 
/// Provides:
/// - CPU feature detection (CPUID)
/// - CPU optimization flags and tuning
/// - Real mode utilities and transitions
/// - BIOS interrupt support framework
/// - Advanced boot mode features
/// - Performance optimization settings

// Only enable inline assembly for x86_64
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{__cpuid, _mm_mfence};

#[cfg(not(target_arch = "x86_64"))]
mod dummy {
    // Dummy implementations for non-x86_64 architectures
    pub unsafe fn __cpuid(eax_in: u32) -> CpuidResult {
        // Log CPUID call attempt on non-x86_64 architecture
        log::trace!("CPUID instruction requested on non-x86_64 architecture, eax={:#x}", eax_in);
        // Return default values for unsupported architectures
        let result = CpuidResult { eax: 0, ebx: 0, ecx: 0, edx: 0 };
        // 使用eax字段以避免未使用警告
        let _ = result.eax;
        result
    }
    pub fn _mm_mfence() {}
    #[derive(Clone, Copy)]
    pub struct CpuidResult { pub eax: u32, pub ebx: u32, pub ecx: u32, pub edx: u32 }
}

#[cfg(not(target_arch = "x86_64"))]
use dummy::__cpuid;

/// CPU Features Detected via CPUID
#[derive(Debug, Clone, Copy)]
pub struct CpuFeatures {
    pub has_pae: bool,           // Physical Address Extension
    pub has_pse: bool,           // Page Size Extension (4MB)
    pub has_apic: bool,          // Advanced Programmable Interrupt Controller
    pub has_msr: bool,           // Model Specific Registers
    pub has_pat: bool,           // Page Attribute Table
    pub has_mtrr: bool,          // Memory Type Range Register
    pub has_cx8: bool,           // CMPXCHG8B (64-bit compare and exchange)
    pub has_nx: bool,            // No-Execute (NX) bit support
    pub has_rdtscp: bool,        // RDTSCP instruction
    pub has_smep: bool,          // Supervisor Mode Execution Protection
    pub has_smap: bool,          // Supervisor Mode Access Prevention
    pub has_umip: bool,          // User Mode Instruction Prevention
    pub has_pku: bool,           // Protection Keys for User pages
    pub has_avx: bool,           // Advanced Vector Extensions
    pub has_avx2: bool,          // Advanced Vector Extensions 2
    pub has_smx: bool,           // Secure Mode Extensions (TXT)
    pub has_vmx: bool,           // Virtual Machine Extensions
}

impl CpuFeatures {
    /// Detect all CPU features via CPUID
    pub fn detect() -> Self {
        let mut features = Self {
            has_pae: false,
            has_pse: false,
            has_apic: false,
            has_msr: false,
            has_pat: false,
            has_mtrr: false,
            has_cx8: false,
            has_nx: false,
            has_rdtscp: false,
            has_smep: false,
            has_smap: false,
            has_umip: false,
            has_pku: false,
            has_avx: false,
            has_avx2: false,
            has_smx: false,
            has_vmx: false,
        };

        // CPUID 0x00000001 - Feature Information
        unsafe {
            let cpuid_01 = __cpuid(0x00000001);
            
            // EDX features
            features.has_pse = (cpuid_01.edx & (1 << 3)) != 0;     // Page Size Extension
            features.has_apic = (cpuid_01.edx & (1 << 9)) != 0;    // APIC
            features.has_msr = (cpuid_01.edx & (1 << 5)) != 0;     // MSR
            features.has_mtrr = (cpuid_01.edx & (1 << 12)) != 0;   // MTRR
            features.has_pat = (cpuid_01.edx & (1 << 16)) != 0;    // PAT
            features.has_cx8 = (cpuid_01.edx & (1 << 8)) != 0;     // CMPXCHG8B
            
            // ECX features
            features.has_avx = (cpuid_01.ecx & (1 << 28)) != 0;    // AVX
            features.has_smx = (cpuid_01.ecx & (1 << 6)) != 0;     // SMX
            features.has_vmx = (cpuid_01.ecx & (1 << 5)) != 0;     // VMX
        }

        // CPUID 0x80000001 - Extended Feature Information (requires extended CPUID support)
        unsafe {
            let cpuid_ext = __cpuid(0x80000001);
            
            // EDX extended features
            features.has_nx = (cpuid_ext.edx & (1 << 20)) != 0;    // NX bit
            features.has_rdtscp = (cpuid_ext.edx & (1 << 27)) != 0; // RDTSCP
            
            // ECX extended features
            features.has_smap = (cpuid_ext.ecx & (1 << 20)) != 0;  // SMAP
            features.has_smep = (cpuid_ext.ecx & (1 << 25)) != 0;  // SMEP (note: different position in some docs)
            features.has_umip = (cpuid_ext.ecx & (1 << 2)) != 0;   // UMIP
            features.has_pku = (cpuid_ext.ecx & (1 << 3)) != 0;    // PKU
        }

        // CPUID 0x00000007 - Extended Features (Leaf 7)
        unsafe {
            let cpuid_07 = __cpuid(0x00000007);
            
            // EBX extended features
            features.has_smep = (cpuid_07.ebx & (1 << 7)) != 0;    // SMEP (correct position)
            features.has_smap = (cpuid_07.ebx & (1 << 20)) != 0;   // SMAP (correct position)
            features.has_avx2 = (cpuid_07.ebx & (1 << 5)) != 0;    // AVX2
            features.has_pku = (cpuid_07.ecx & (1 << 3)) != 0;     // PKU (ECX)
        }

        // PAE detection (typically always present on x86_64)
        features.has_pae = true;

        features
    }

    /// Get human-readable feature list
    pub fn feature_list(&self) -> [(&'static str, bool); 17] {
        [
            ("PAE", self.has_pae),
            ("PSE", self.has_pse),
            ("APIC", self.has_apic),
            ("MSR", self.has_msr),
            ("PAT", self.has_pat),
            ("MTRR", self.has_mtrr),
            ("CX8", self.has_cx8),
            ("NX", self.has_nx),
            ("RDTSCP", self.has_rdtscp),
            ("SMEP", self.has_smep),
            ("SMAP", self.has_smap),
            ("UMIP", self.has_umip),
            ("PKU", self.has_pku),
            ("AVX", self.has_avx),
            ("AVX2", self.has_avx2),
            ("SMX", self.has_smx),
            ("VMX", self.has_vmx),
        ]
    }

    /// Check if critical features are present
    pub fn has_critical_features(&self) -> bool {
        self.has_pae && self.has_msr && self.has_pat && self.has_mtrr
    }

    /// Check if security features are present
    pub fn has_security_features(&self) -> bool {
        self.has_nx && self.has_smep && self.has_smap
    }

    /// Count total enabled features
    pub fn feature_count(&self) -> usize {
        let mut count = 0;
        for (_, enabled) in &self.feature_list() {
            if *enabled {
                count += 1;
            }
        }
        count
    }
}

/// CPU Optimization Settings
#[derive(Debug, Clone, Copy)]
pub struct CpuOptimization {
    pub enable_smep: bool,          // Kernel execution protection
    pub enable_smap: bool,          // Kernel memory access protection
    pub enable_pku: bool,           // Memory protection keys
    pub enable_nx: bool,            // Non-executable memory
    pub enable_pae: bool,           // 36-bit physical addressing
    pub enable_pat: bool,           // Page attribute table
    pub enable_mtrr: bool,          // Memory type range registers
    pub cache_mode: CacheMode,      // Cache strategy
    pub performance_boost: bool,    // CPU-specific performance optimization
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CacheMode {
    Disabled,                       // No cache optimization
    WriteBack,                      // Write-back (fastest)
    WriteThrough,                   // Write-through (safe)
    WriteProtected,                 // Write-protected (safe)
    Uncacheable,                    // Uncacheable (slowest, for MMIO)
}

impl CpuOptimization {
    /// Create default optimization settings
    pub fn new() -> Self {
        Self {
            enable_smep: true,
            enable_smap: true,
            enable_pku: false,      // Disabled by default (requires explicit setup)
            enable_nx: true,
            enable_pae: true,
            enable_pat: true,
            enable_mtrr: true,
            cache_mode: CacheMode::WriteBack,
            performance_boost: false,
        }
    }

    /// Create optimized settings for boot time
    pub fn for_boot_time() -> Self {
        Self {
            enable_smep: false,     // Disable during boot for compatibility
            enable_smap: false,     // Disable during boot
            enable_pku: false,
            enable_nx: true,        // Always enable NX
            enable_pae: true,
            enable_pat: true,
            enable_mtrr: false,     // Keep MTRR settings from firmware
            cache_mode: CacheMode::WriteBack,
            performance_boost: true,
        }
    }

    /// Create secure optimized settings
    pub fn for_security() -> Self {
        Self {
            enable_smep: true,
            enable_smap: true,
            enable_pku: true,
            enable_nx: true,
            enable_pae: true,
            enable_pat: true,
            enable_mtrr: true,
            cache_mode: CacheMode::WriteBack,
            performance_boost: false,
        }
    }

    /// Apply optimization (requires previous feature detection)
    pub fn apply(&self, features: &CpuFeatures) -> Result<(), &'static str> {
        // Validate features exist before enabling
        if self.enable_smep && !features.has_smep {
            return Err("SMEP not available");
        }
        if self.enable_smap && !features.has_smap {
            return Err("SMAP not available");
        }
        if self.enable_pku && !features.has_pku {
            return Err("PKU not available");
        }

        // Write CR4 register with security flags
        // This would require inline assembly for actual CPU register modification
        // In bootloader context, this is typically done during boot sequence initialization
        #[cfg(target_arch = "x86_64")]
        unsafe {
            let mut cr4: u64;
            core::arch::asm!(
                "mov {}, cr4",
                out(reg) cr4,
                options(nostack, preserves_flags)
            );

            // Set SMEP (bit 20)
            if self.enable_smep {
                cr4 |= 1u64 << 20;
            } else {
                cr4 &= !(1u64 << 20);
            }

            // Set SMAP (bit 21)
            if self.enable_smap {
                cr4 |= 1u64 << 21;
            } else {
                cr4 &= !(1u64 << 21);
            }

            // Set PKE (bit 22) - enable PKU
            if self.enable_pku {
                cr4 |= 1u64 << 22;
            } else {
                cr4 &= !(1u64 << 22);
            }

            core::arch::asm!(
                "mov cr4, {}",
                in(reg) cr4,
                options(nostack, preserves_flags)
            );

            // Memory fence to ensure changes take effect
            _mm_mfence();
        }

        #[cfg(not(target_arch = "x86_64"))]
        {
            // Non-x86_64 platforms: just return success
            // (actual optimization would happen on real hardware)
        }

        Ok(())
    }

    /// Get optimization flags for CR0/CR4 registers
    pub fn get_cr_flags(&self) -> (u64, u64) {
        let mut cr0 = 0u64;
        let mut cr4 = 0u64;

        // CR0 - System Control Register
        if self.enable_nx {
            cr0 |= 1u64 << 31;  // WP (Write Protect)
        }

        // CR4 - Extended Control Register
        if self.enable_pae {
            cr4 |= 1u64 << 5;   // PAE (Physical Address Extension)
        }
        if self.enable_smep {
            cr4 |= 1u64 << 20;  // SMEP
        }
        if self.enable_smap {
            cr4 |= 1u64 << 21;  // SMAP
        }
        if self.enable_pku {
            cr4 |= 1u64 << 22;  // PKE
        }

        (cr0, cr4)
    }
}

/// Real Mode Utilities (for BIOS transitions if needed)
pub struct RealModeUtils;

impl RealModeUtils {
    /// Calculate real mode address from segment:offset
    pub fn segment_to_physical(segment: u16, offset: u16) -> u32 {
        ((segment as u32) << 4) + (offset as u32)
    }

    /// Calculate segment from physical address
    pub fn physical_to_segment(addr: u32) -> (u16, u16) {
        let segment = (addr >> 4) as u16;
        let offset = (addr & 0xF) as u16;
        (segment, offset)
    }

    /// Create real mode far pointer
    pub fn far_pointer(segment: u16, offset: u16) -> u32 {
        ((segment as u32) << 16) | (offset as u32)
    }
}

/// BIOS Interrupt Support Framework
pub struct BiosInterruptSupport {
    /// Interrupt vectors mapped
    pub vectors_mapped: usize,
    /// Last interrupt called
    pub last_interrupt: u8,
    /// Interrupt status
    pub status: InterruptStatus,
}

#[derive(Debug, Clone, Copy)]
pub enum InterruptStatus {
    Ready,
    Executing,
    Completed,
    Failed,
}

impl BiosInterruptSupport {
    /// Create new BIOS interrupt support
    pub fn new() -> Self {
        Self {
            vectors_mapped: 0,
            last_interrupt: 0,
            status: InterruptStatus::Ready,
        }
    }

    /// Check if interrupt is available
    pub fn is_interrupt_available(&self, interrupt_number: u8) -> bool {
        // In real implementation, would check interrupt descriptor table
        // For now, support common BIOS interrupts
        matches!(
            interrupt_number,
            0x10 |  // Video services
            0x13 |  // Disk services
            0x15 |  // Miscellaneous services
            0x16 |  // Keyboard services
            0x17 |  // Printer services
            0x19    // Bootstrap loader
        )
    }

    /// Mark interrupt as mapped
    pub fn mark_mapped(&mut self, count: usize) {
        self.vectors_mapped = count;
    }

    /// Execute BIOS interrupt (framework - actual execution would require real mode)
    pub fn execute_interrupt(&mut self, _int_num: u8) -> Result<(), &'static str> {
        // In real implementation, would:
        // 1. Switch to real mode
        // 2. Set up registers
        // 3. Call interrupt
        // 4. Switch back to protected/long mode
        // For now, just framework
        self.status = InterruptStatus::Completed;
        Ok(())
    }
}

/// x86_64 Boot Configuration
pub struct X86_64BootConfig {
    pub features: CpuFeatures,
    pub optimization: CpuOptimization,
    pub bios_support: BiosInterruptSupport,
    pub boot_mode: BootMode,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BootMode {
    Legacy,         // Legacy BIOS
    Uefi,           // UEFI
    Multiboot2,     // Multiboot2
    DirectBoot,     // Direct kernel boot
}

impl X86_64BootConfig {
    /// Initialize x86_64 boot configuration
    pub fn initialize() -> Self {
        let features = CpuFeatures::detect();
        let optimization = CpuOptimization::for_boot_time();
        
        Self {
            features,
            optimization,
            bios_support: BiosInterruptSupport::new(),
            boot_mode: BootMode::Legacy,
        }
    }

    /// Setup optimized x86_64 boot
    pub fn setup(&mut self, mode: BootMode) -> Result<(), &'static str> {
        self.boot_mode = mode;
        
        // Apply optimizations if features available
        if self.features.has_critical_features() {
            self.optimization.apply(&self.features)?;
        }

        Ok(())
    }

    /// Get boot summary
    pub fn get_summary(&self) -> X86_64BootSummary {
        X86_64BootSummary {
            cpu_features_count: self.features.feature_count(),
            has_security_features: self.features.has_security_features(),
            has_critical_features: self.features.has_critical_features(),
            optimization_applied: self.optimization.enable_smep || self.optimization.enable_smap,
            boot_mode: self.boot_mode,
        }
    }
}

/// Boot Summary for diagnostic output
#[derive(Debug, Clone, Copy)]
pub struct X86_64BootSummary {
    pub cpu_features_count: usize,
    pub has_security_features: bool,
    pub has_critical_features: bool,
    pub optimization_applied: bool,
    pub boot_mode: BootMode,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_feature_detection() {
        let features = CpuFeatures::detect();
        assert!(features.has_pae);  // Always present on x86_64
        assert!(features.has_nx);   // Always present on x86_64
    }

    #[test]
    fn test_cpu_optimization_creation() {
        let opt = CpuOptimization::new();
        assert!(opt.enable_smep);
        assert!(opt.enable_smap);
    }

    #[test]
    fn test_optimization_for_security() {
        let opt = CpuOptimization::for_security();
        assert!(opt.enable_smep);
        assert!(opt.enable_smap);
        assert!(opt.enable_pku);
    }

    #[test]
    fn test_optimization_for_boot_time() {
        let opt = CpuOptimization::for_boot_time();
        assert!(!opt.enable_smep);  // Disabled for boot compatibility
        assert!(!opt.enable_smap);  // Disabled for boot compatibility
        assert!(opt.enable_nx);
    }

    #[test]
    fn test_real_mode_address_calculation() {
        let segment = 0x1000u16;
        let offset = 0x0100u16;
        let physical = RealModeUtils::segment_to_physical(segment, offset);
        assert_eq!(physical, 0x10100);
    }

    #[test]
    fn test_bios_interrupt_detection() {
        let bios = BiosInterruptSupport::new();
        assert!(bios.is_interrupt_available(0x10));  // Video
        assert!(bios.is_interrupt_available(0x13));  // Disk
        assert!(!bios.is_interrupt_available(0x20)); // Not BIOS
    }

    #[test]
    fn test_x86_64_boot_config() {
        let config = X86_64BootConfig::initialize();
        assert!(config.features.has_pae);
        assert_eq!(config.boot_mode, BootMode::Legacy);
    }
}
