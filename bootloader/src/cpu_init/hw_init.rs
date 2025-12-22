/// Hardware Initialization and Feature Detection
///
/// Detects CPU features and initializes hardware components required for
/// kernel execution (paging, protection, etc).

/// CPU vendor identification
#[derive(Debug, Clone, Copy)]
pub enum CPUVendor {
    Intel,
    AMD,
    Cyrix,
    VIA,
    Centaur,
    Unknown,
}

impl CPUVendor {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Intel => "Intel",
            Self::AMD => "AMD",
            Self::Cyrix => "Cyrix",
            Self::VIA => "VIA",
            Self::Centaur => "Centaur",
            Self::Unknown => "Unknown",
        }
    }
}

/// CPUID information
#[derive(Debug, Clone, Copy)]
pub struct CPUIDInfo {
    pub vendor: CPUVendor,
    pub stepping: u32,
    pub model: u32,
    pub family: u32,
    pub processor_type: u32,
}

impl CPUIDInfo {
    pub fn new() -> Self {
        Self {
            vendor: CPUVendor::Unknown,
            stepping: 0,
            model: 0,
            family: 0,
            processor_type: 0,
        }
    }

    /// Parse CPUID from EAX register
    pub fn from_eax(eax: u32) -> Self {
        let stepping = eax & 0xF;
        let model = (eax >> 4) & 0xF;
        let family = (eax >> 8) & 0xF;
        let processor_type = (eax >> 12) & 0x3;

        Self {
            vendor: CPUVendor::Unknown,
            stepping,
            model,
            family,
            processor_type,
        }
    }
}

/// CPU feature flags
#[derive(Debug, Clone, Copy)]
pub struct CPUFeatures {
    pub fpu: bool,          // x87 FPU
    pub vme: bool,          // Virtual mode extension
    pub de: bool,           // Debug extension
    pub pse: bool,          // Page size extension
    pub tsc: bool,          // Time stamp counter
    pub msr: bool,          // Model specific registers
    pub pae: bool,          // Physical address extension
    pub mce: bool,          // Machine check exception
    pub cx8: bool,          // CMPXCHG8B instruction
    pub apic: bool,         // APIC on chip
    pub sep: bool,          // SYSENTER/SYSEXIT
    pub mtrr: bool,         // Memory type range registers
    pub pge: bool,          // PTE global bit
    pub mca: bool,          // Machine check architecture
    pub cmov: bool,         // Conditional move
    pub pat: bool,          // Page attribute table
    pub pse36: bool,        // 36-bit PSE
    pub psn: bool,          // Processor serial number
    pub clfsh: bool,        // CLFLUSH instruction
    pub ds: bool,           // Debug store
    pub acpi: bool,         // ACPI support
    pub mmx: bool,          // MMX
    pub fxsr: bool,         // FXSAVE/FXRSTOR
    pub sse: bool,          // SSE
    pub sse2: bool,         // SSE2
    pub ss: bool,           // Self snoop
    pub htt: bool,          // Hyper-threading
    pub tm: bool,           // Thermal monitor
    pub ia64: bool,         // IA64
}

impl CPUFeatures {
    pub fn new() -> Self {
        Self {
            fpu: false,
            vme: false,
            de: false,
            pse: false,
            tsc: false,
            msr: false,
            pae: false,
            mce: false,
            cx8: false,
            apic: false,
            sep: false,
            mtrr: false,
            pge: false,
            mca: false,
            cmov: false,
            pat: false,
            pse36: false,
            psn: false,
            clfsh: false,
            ds: false,
            acpi: false,
            mmx: false,
            fxsr: false,
            sse: false,
            sse2: false,
            ss: false,
            htt: false,
            tm: false,
            ia64: false,
        }
    }

    /// Parse features from EDX register
    pub fn from_edx(edx: u32) -> Self {
        Self {
            fpu: (edx & (1 << 0)) != 0,
            vme: (edx & (1 << 1)) != 0,
            de: (edx & (1 << 2)) != 0,
            pse: (edx & (1 << 3)) != 0,
            tsc: (edx & (1 << 4)) != 0,
            msr: (edx & (1 << 5)) != 0,
            pae: (edx & (1 << 6)) != 0,
            mce: (edx & (1 << 7)) != 0,
            cx8: (edx & (1 << 8)) != 0,
            apic: (edx & (1 << 9)) != 0,
            sep: (edx & (1 << 11)) != 0,
            mtrr: (edx & (1 << 12)) != 0,
            pge: (edx & (1 << 13)) != 0,
            mca: (edx & (1 << 14)) != 0,
            cmov: (edx & (1 << 15)) != 0,
            pat: (edx & (1 << 16)) != 0,
            pse36: (edx & (1 << 17)) != 0,
            psn: (edx & (1 << 18)) != 0,
            clfsh: (edx & (1 << 19)) != 0,
            ds: (edx & (1 << 21)) != 0,
            acpi: (edx & (1 << 22)) != 0,
            mmx: (edx & (1 << 23)) != 0,
            fxsr: (edx & (1 << 24)) != 0,
            sse: (edx & (1 << 25)) != 0,
            sse2: (edx & (1 << 26)) != 0,
            ss: (edx & (1 << 27)) != 0,
            htt: (edx & (1 << 28)) != 0,
            tm: (edx & (1 << 29)) != 0,
            ia64: (edx & (1 << 30)) != 0,
        }
    }

    /// Check if CPU has required features for x86_64 operation
    pub fn is_capable_x86_64(&self) -> bool {
        self.pae && self.pse && self.msr
    }

    /// Count enabled features
    pub fn count(&self) -> u32 {
        let mut count = 0;
        if self.fpu { count += 1; }
        if self.vme { count += 1; }
        if self.de { count += 1; }
        if self.pse { count += 1; }
        if self.tsc { count += 1; }
        if self.msr { count += 1; }
        if self.pae { count += 1; }
        if self.mce { count += 1; }
        if self.cx8 { count += 1; }
        if self.apic { count += 1; }
        if self.sep { count += 1; }
        if self.mtrr { count += 1; }
        if self.pge { count += 1; }
        if self.mca { count += 1; }
        if self.cmov { count += 1; }
        if self.pat { count += 1; }
        if self.pse36 { count += 1; }
        if self.mmx { count += 1; }
        if self.sse { count += 1; }
        if self.sse2 { count += 1; }
        if self.htt { count += 1; }
        count
    }
}

/// Hardware detection and initialization
pub struct HardwareInitializer {
    cpu_info: CPUIDInfo,
    cpu_features: CPUFeatures,
    initialized: bool,
}

impl HardwareInitializer {
    pub fn new() -> Self {
        Self {
            cpu_info: CPUIDInfo::new(),
            cpu_features: CPUFeatures::new(),
            initialized: false,
        }
    }

    /// Detect CPU features (framework)
    pub fn detect_cpu(&mut self) -> Result<(), &'static str> {
        // Framework: would use CPUID instruction here
        // For now, set some conservative defaults
        self.cpu_features = CPUFeatures::new();
        self.cpu_features.fpu = true;
        self.cpu_features.pse = true;
        self.cpu_features.pae = true;
        self.cpu_features.msr = true;
        self.cpu_features.tsc = true;
        
        Ok(())
    }

    /// Initialize paging (framework)
    pub fn init_paging(&mut self) -> Result<(), &'static str> {
        if !self.cpu_features.pae {
            return Err("PAE not supported");
        }
        Ok(())
    }

    /// Initialize APIC (if present)
    pub fn init_apic(&mut self) -> Result<(), &'static str> {
        if !self.cpu_features.apic {
            return Err("APIC not supported");
        }
        Ok(())
    }

    /// Complete hardware initialization
    pub fn init_all(&mut self) -> Result<(), &'static str> {
        self.detect_cpu()?;

        if !self.cpu_features.is_capable_x86_64() {
            return Err("CPU does not support required x86_64 features");
        }

        self.init_paging()?;
        self.initialized = true;
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    pub fn cpu_info(&self) -> &CPUIDInfo {
        &self.cpu_info
    }

    pub fn cpu_features(&self) -> &CPUFeatures {
        &self.cpu_features
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_vendor_strings() {
        assert_eq!(CPUVendor::Intel.as_str(), "Intel");
        assert_eq!(CPUVendor::AMD.as_str(), "AMD");
    }

    #[test]
    fn test_cpu_id_info() {
        let info = CPUIDInfo::new();
        assert_eq!(info.stepping, 0);
    }

    #[test]
    fn test_cpu_features_creation() {
        let features = CPUFeatures::new();
        assert!(!features.fpu);
    }

    #[test]
    fn test_cpu_features_from_edx() {
        let edx = (1 << 0) | (1 << 6) | (1 << 5);  // FPU, PAE, MSR
        let features = CPUFeatures::from_edx(edx);
        assert!(features.fpu);
        assert!(features.pae);
        assert!(features.msr);
        assert!(!features.tsc);
    }

    #[test]
    fn test_x86_64_capability() {
        let mut features = CPUFeatures::new();
        assert!(!features.is_capable_x86_64());

        features.pae = true;
        features.pse = true;
        features.msr = true;
        assert!(features.is_capable_x86_64());
    }

    #[test]
    fn test_hardware_initializer() {
        let mut hw = HardwareInitializer::new();
        assert!(!hw.is_initialized());

        assert!(hw.init_all().is_ok());
        assert!(hw.is_initialized());
    }

    #[test]
    fn test_feature_count() {
        let mut features = CPUFeatures::new();
        assert_eq!(features.count(), 0);

        features.fpu = true;
        features.tsc = true;
        assert_eq!(features.count(), 2);
    }
}
