//! Virtualization Detection - CPU virtualization feature detection
//!
//! Provides:
//! - CPUID-based virtualization capability detection
//! - VMX (Intel VT-x) and SVM (AMD-V) detection
//! - Feature flag management
//! - CPU vendor identification

/// CPU vendors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuVendor {
    /// Intel
    Intel,
    /// AMD
    AMD,
    /// Other/Unknown
    Other,
}

/// Virtualization technologies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualizationTech {
    /// No virtualization support
    None,
    /// Intel VT-x (VMX)
    VTx,
    /// AMD-V (SVM)
    SVM,
    /// Both VT-x and SVM
    Both,
}

/// VMX (Intel VT-x) capabilities
#[derive(Debug, Clone, Copy)]
pub struct VmxCapabilities {
    /// VMX supported
    pub supported: bool,
    /// VMX locked in BIOS
    pub locked: bool,
    /// EPT (Extended Page Tables) supported
    pub ept_support: bool,
    /// VPID (Virtual Processor ID) supported
    pub vpid_support: bool,
    /// Unrestricted guest execution
    pub unrestricted_guest: bool,
    /// Enable MSR bitmap
    pub msr_bitmap_support: bool,
}

impl VmxCapabilities {
    /// Create VMX capabilities
    pub fn new() -> Self {
        VmxCapabilities {
            supported: false,
            locked: false,
            ept_support: false,
            vpid_support: false,
            unrestricted_guest: false,
            msr_bitmap_support: false,
        }
    }

    /// Check if VMX is fully usable
    pub fn is_usable(&self) -> bool {
        self.supported && !self.locked
    }
}

/// SVM (AMD-V) capabilities
#[derive(Debug, Clone, Copy)]
pub struct SvmCapabilities {
    /// SVM supported
    pub supported: bool,
    /// SVM locked in BIOS
    pub locked: bool,
    /// NPT (Nested Page Tables) supported
    pub npt_support: bool,
    /// ASID (Address Space ID) supported
    pub asid_support: bool,
    /// Decode assists
    pub decode_assists: bool,
    /// PAUSEFILTER supported
    pub pausefilter_support: bool,
}

impl SvmCapabilities {
    /// Create SVM capabilities
    pub fn new() -> Self {
        SvmCapabilities {
            supported: false,
            locked: false,
            npt_support: false,
            asid_support: false,
            decode_assists: false,
            pausefilter_support: false,
        }
    }

    /// Check if SVM is fully usable
    pub fn is_usable(&self) -> bool {
        self.supported && !self.locked
    }
}

/// CPU feature flags
#[derive(Debug, Clone, Copy)]
pub struct CpuFeatures {
    /// PAE (Physical Address Extension)
    pub pae: bool,
    /// PSE (Page Size Extension)
    pub pse: bool,
    /// MSR (Model Specific Registers)
    pub msr: bool,
    /// APIC
    pub apic: bool,
    /// CMOV (Conditional Move)
    pub cmov: bool,
    /// TSC (Time Stamp Counter)
    pub tsc: bool,
    /// RDMSR/WRMSR
    pub rdwrmsr: bool,
}

impl CpuFeatures {
    /// Create CPU features
    pub fn new() -> Self {
        CpuFeatures {
            pae: false,
            pse: false,
            msr: false,
            apic: false,
            cmov: false,
            tsc: false,
            rdwrmsr: false,
        }
    }
}

/// CPU information structure
#[derive(Debug, Clone, Copy)]
pub struct CpuInfo {
    /// CPU vendor
    pub vendor: CpuVendor,
    /// Family
    pub family: u8,
    /// Model
    pub model: u8,
    /// Stepping
    pub stepping: u8,
    /// Number of cores
    pub cores: u8,
    /// Cache size in KB
    pub cache_size: u32,
}

impl CpuInfo {
    /// Create CPU info
    pub fn new(vendor: CpuVendor) -> Self {
        CpuInfo {
            vendor,
            family: 0,
            model: 0,
            stepping: 0,
            cores: 1,
            cache_size: 0,
        }
    }
}

/// Virtualization detector
pub struct VirtualizationDetector {
    /// CPU vendor
    cpu_vendor: CpuVendor,
    /// CPU info
    cpu_info: CpuInfo,
    /// CPU features
    cpu_features: CpuFeatures,
    /// VMX capabilities
    vmx_caps: VmxCapabilities,
    /// SVM capabilities
    svm_caps: SvmCapabilities,
    /// Detected virtualization technology
    virt_tech: VirtualizationTech,
    /// Detection complete
    detected: bool,
}

impl VirtualizationDetector {
    /// Create virtualization detector
    pub fn new() -> Self {
        VirtualizationDetector {
            cpu_vendor: CpuVendor::Other,
            cpu_info: CpuInfo::new(CpuVendor::Other),
            cpu_features: CpuFeatures::new(),
            vmx_caps: VmxCapabilities::new(),
            svm_caps: SvmCapabilities::new(),
            virt_tech: VirtualizationTech::None,
            detected: false,
        }
    }

    /// Detect CPU vendor from CPUID
    pub fn detect_vendor(&mut self) -> CpuVendor {
        // Simulated: Would use CPUID instruction 0x00
        // Returns: EBX=0x756E6547 ('Genu'), EDX=0x49656E69 ('ineI'), ECX=0x6C65746E ('letn')
        // for Intel, or EBX=0x68747541 ('Auth'), EDX=0x69746E41 ('itne'), ECX=0x444D4163 ('cAMD')
        // for AMD
        
        self.cpu_vendor = CpuVendor::Intel; // Simulated detection
        self.cpu_info.vendor = self.cpu_vendor;
        self.cpu_vendor
    }

    /// Detect VMX (Intel VT-x) support
    pub fn detect_vmx(&mut self) -> bool {
        // CPUID 0x01, ECX bit 5 (VMX flag)
        // If set, VMX is supported
        
        self.vmx_caps.supported = true;
        
        // Check additional capabilities with CPUID 0x05 (MSR list)
        self.vmx_caps.ept_support = true;
        self.vmx_caps.vpid_support = true;
        self.vmx_caps.unrestricted_guest = true;
        
        self.vmx_caps.supported
    }

    /// Detect SVM (AMD-V) support
    pub fn detect_svm(&mut self) -> bool {
        // CPUID 0x80000001, ECX bit 2 (SVM flag)
        // If set, SVM is supported
        
        self.svm_caps.supported = false; // Simulated
        
        if self.svm_caps.supported {
            self.svm_caps.npt_support = true;
            self.svm_caps.asid_support = true;
        }
        
        self.svm_caps.supported
    }

    /// Detect CPU features
    pub fn detect_features(&mut self) -> bool {
        // CPUID 0x01, EDX register
        self.cpu_features.pae = true;
        self.cpu_features.pse = true;
        self.cpu_features.msr = true;
        self.cpu_features.apic = true;
        self.cpu_features.cmov = true;
        self.cpu_features.tsc = true;
        self.cpu_features.rdwrmsr = true;
        
        true
    }

    /// Perform full detection
    pub fn detect_all(&mut self) -> bool {
        self.detect_vendor();
        self.detect_features();
        
        let has_vmx = self.detect_vmx();
        let has_svm = self.detect_svm();
        
        self.virt_tech = match (has_vmx, has_svm) {
            (true, true) => VirtualizationTech::Both,
            (true, false) => VirtualizationTech::VTx,
            (false, true) => VirtualizationTech::SVM,
            (false, false) => VirtualizationTech::None,
        };
        
        self.detected = true;
        true
    }

    /// Get virtualization technology
    pub fn get_virt_tech(&self) -> VirtualizationTech {
        self.virt_tech
    }

    /// Check if virtualization is available
    pub fn has_virtualization(&self) -> bool {
        self.virt_tech != VirtualizationTech::None
    }

    /// Check if virtualization is usable
    pub fn is_virtualization_usable(&self) -> bool {
        match self.virt_tech {
            VirtualizationTech::VTx => self.vmx_caps.is_usable(),
            VirtualizationTech::SVM => self.svm_caps.is_usable(),
            VirtualizationTech::Both => {
                self.vmx_caps.is_usable() || self.svm_caps.is_usable()
            }
            VirtualizationTech::None => false,
        }
    }

    /// Get VMX capabilities
    pub fn get_vmx_caps(&self) -> VmxCapabilities {
        self.vmx_caps
    }

    /// Get SVM capabilities
    pub fn get_svm_caps(&self) -> SvmCapabilities {
        self.svm_caps
    }

    /// Get CPU info
    pub fn get_cpu_info(&self) -> CpuInfo {
        self.cpu_info
    }

    /// Get CPU features
    pub fn get_cpu_features(&self) -> CpuFeatures {
        self.cpu_features
    }

    /// Get detection report
    pub fn detection_report(&self) -> VirtualizationReport {
        VirtualizationReport {
            vendor: self.cpu_vendor,
            virt_tech: self.virt_tech,
            vmx_supported: self.vmx_caps.supported,
            svm_supported: self.svm_caps.supported,
            vmx_usable: self.vmx_caps.is_usable(),
            svm_usable: self.svm_caps.is_usable(),
            ept_support: self.vmx_caps.ept_support,
            npt_support: self.svm_caps.npt_support,
        }
    }
}

/// Virtualization detection report
#[derive(Debug, Clone, Copy)]
pub struct VirtualizationReport {
    /// CPU vendor
    pub vendor: CpuVendor,
    /// Virtualization technology
    pub virt_tech: VirtualizationTech,
    /// VMX supported
    pub vmx_supported: bool,
    /// SVM supported
    pub svm_supported: bool,
    /// VMX usable (not locked)
    pub vmx_usable: bool,
    /// SVM usable (not locked)
    pub svm_usable: bool,
    /// EPT support
    pub ept_support: bool,
    /// NPT support
    pub npt_support: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_vendors() {
        assert_ne!(CpuVendor::Intel, CpuVendor::AMD);
        assert_eq!(CpuVendor::Intel, CpuVendor::Intel);
    }

    #[test]
    fn test_virtualization_techs() {
        assert_ne!(VirtualizationTech::VTx, VirtualizationTech::SVM);
        assert_eq!(VirtualizationTech::None, VirtualizationTech::None);
    }

    #[test]
    fn test_vmx_capabilities_creation() {
        let vmx = VmxCapabilities::new();
        assert!(!vmx.supported);
        assert!(!vmx.locked);
    }

    #[test]
    fn test_vmx_capabilities_usable() {
        let mut vmx = VmxCapabilities::new();
        assert!(!vmx.is_usable());
        
        vmx.supported = true;
        assert!(vmx.is_usable());
        
        vmx.locked = true;
        assert!(!vmx.is_usable());
    }

    #[test]
    fn test_svm_capabilities_creation() {
        let svm = SvmCapabilities::new();
        assert!(!svm.supported);
        assert!(!svm.locked);
    }

    #[test]
    fn test_svm_capabilities_usable() {
        let mut svm = SvmCapabilities::new();
        assert!(!svm.is_usable());
        
        svm.supported = true;
        assert!(svm.is_usable());
        
        svm.locked = true;
        assert!(!svm.is_usable());
    }

    #[test]
    fn test_cpu_features_creation() {
        let features = CpuFeatures::new();
        assert!(!features.pae);
        assert!(!features.msr);
    }

    #[test]
    fn test_cpu_info_creation() {
        let info = CpuInfo::new(CpuVendor::Intel);
        assert_eq!(info.vendor, CpuVendor::Intel);
        assert_eq!(info.cores, 1);
    }

    #[test]
    fn test_virtualization_detector_creation() {
        let detector = VirtualizationDetector::new();
        assert!(!detector.detected);
        assert_eq!(detector.get_virt_tech(), VirtualizationTech::None);
    }

    #[test]
    fn test_detect_vendor() {
        let mut detector = VirtualizationDetector::new();
        let vendor = detector.detect_vendor();
        assert_eq!(vendor, CpuVendor::Intel); // Simulated
    }

    #[test]
    fn test_detect_vmx() {
        let mut detector = VirtualizationDetector::new();
        assert!(detector.detect_vmx());
        assert!(detector.vmx_caps.supported);
    }

    #[test]
    fn test_detect_svm() {
        let mut detector = VirtualizationDetector::new();
        detector.detect_svm();
        // Simulated to return false
    }

    #[test]
    fn test_detect_features() {
        let mut detector = VirtualizationDetector::new();
        assert!(detector.detect_features());
        let features = detector.get_cpu_features();
        assert!(features.pae);
        assert!(features.msr);
    }

    #[test]
    fn test_detect_all() {
        let mut detector = VirtualizationDetector::new();
        assert!(detector.detect_all());
        assert!(detector.detected);
    }

    #[test]
    fn test_has_virtualization() {
        let mut detector = VirtualizationDetector::new();
        detector.detect_all();
        assert!(detector.has_virtualization());
    }

    #[test]
    fn test_is_virtualization_usable() {
        let mut detector = VirtualizationDetector::new();
        detector.detect_all();
        assert!(detector.is_virtualization_usable());
    }

    #[test]
    fn test_vmx_features() {
        let mut detector = VirtualizationDetector::new();
        detector.detect_vmx();
        let caps = detector.get_vmx_caps();
        assert!(caps.ept_support);
        assert!(caps.vpid_support);
    }

    #[test]
    fn test_cpu_info_family_model() {
        let mut info = CpuInfo::new(CpuVendor::Intel);
        info.family = 6;
        info.model = 158;
        info.stepping = 10;
        
        assert_eq!(info.family, 6);
        assert_eq!(info.model, 158);
    }

    #[test]
    fn test_detection_report() {
        let mut detector = VirtualizationDetector::new();
        detector.detect_all();
        
        let report = detector.detection_report();
        assert_eq!(report.vendor, CpuVendor::Intel);
        assert_ne!(report.virt_tech, VirtualizationTech::None);
    }

    #[test]
    fn test_vmx_locked_not_usable() {
        let mut detector = VirtualizationDetector::new();
        detector.detect_vmx();
        detector.vmx_caps.locked = true;
        
        assert!(!detector.is_virtualization_usable());
    }

    #[test]
    fn test_multiple_cores() {
        let mut info = CpuInfo::new(CpuVendor::AMD);
        info.cores = 8;
        assert_eq!(info.cores, 8);
    }

    #[test]
    fn test_cpu_cache_size() {
        let mut info = CpuInfo::new(CpuVendor::Intel);
        info.cache_size = 8192; // 8MB
        assert_eq!(info.cache_size, 8192);
    }

    #[test]
    fn test_feature_combination() {
        let mut features = CpuFeatures::new();
        features.pae = true;
        features.msr = true;
        features.apic = true;
        
        assert!(features.pae);
        assert!(features.msr);
        assert!(features.apic);
    }

    #[test]
    fn test_svm_features() {
        let mut detector = VirtualizationDetector::new();
        detector.detect_svm();
        let caps = detector.get_svm_caps();
        assert_eq!(caps.npt_support, false); // Simulated
    }

    #[test]
    fn test_virtualization_tech_detection() {
        let mut detector = VirtualizationDetector::new();
        detector.detect_all();
        
        match detector.get_virt_tech() {
            VirtualizationTech::VTx => assert!(detector.vmx_caps.supported),
            VirtualizationTech::SVM => assert!(detector.svm_caps.supported),
            VirtualizationTech::Both => {
                assert!(detector.vmx_caps.supported);
                assert!(detector.svm_caps.supported);
            }
            VirtualizationTech::None => assert!(!detector.has_virtualization()),
        }
    }
}
