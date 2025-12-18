//! Hardware Detection Domain Service Interface
//!
//! Defines abstract interface for hardware detection services.
//! This interface belongs to the domain layer and defines the contract
//! for hardware detection without exposing implementation details.
//!
//! # Architecture Notes
//! This interface follows the Dependency Inversion Principle:
//! - High-level modules (application layer) depend on this abstraction
//! - Low-level modules (infrastructure layer) implement this abstraction
//! - Neither depends on concrete implementations
//!
//! # Design Principles
//! - Interface Segregation: Only contains methods needed for hardware detection
//! - Single Responsibility: Focused solely on hardware detection operations
//! - Dependency Inversion: Depends on abstractions, not concretions

use super::boot_services::{HardwareInfo, GraphicsCapabilities};

/// Hardware Detection Service - Domain Interface
///
/// Defines the contract for hardware detection operations.
/// Implementations are provided by the infrastructure layer.
/// 
/// This interface enables the application layer to perform hardware detection
/// without depending on specific implementation details like BIOS calls,
/// UEFI services, or platform-specific code.
///
/// # Usage Pattern
/// ```rust,no_run
/// let mut hw_service: Box<dyn HardwareDetectionService> = // ... from DI container
/// let hw_info = hw_service.detect_hardware()?;
/// let cpu_info = hw_service.detect_cpu()?;
/// ```
///
/// # Implementation Guidelines
/// Implementations should:
/// - Cache results to avoid repeated expensive detection operations
/// - Handle platform-specific errors gracefully
/// - Provide meaningful error messages for debugging
/// - Follow the interface contract exactly
pub trait HardwareDetectionService: Send + Sync {
    /// Detect complete hardware information
    ///
    /// This is the primary method for hardware detection and should
    /// detect all major hardware components (CPU, memory, graphics).
    ///
    /// # Returns
    /// Complete hardware information including memory, graphics, and CPU capabilities
    ///
    /// # Errors
    /// Returns an error if hardware detection fails for any reason
    ///
    /// # Performance Notes
    /// This method may be expensive and should be cached by implementations
    fn detect_hardware(&mut self) -> Result<HardwareInfo, &'static str>;

    /// Detect graphics capabilities
    ///
    /// Detects the graphics capabilities of the system including
    /// supported modes, maximum resolution, and color depth.
    ///
    /// # Returns
    /// Graphics capabilities information
    ///
    /// # Errors
    /// Returns an error if graphics detection fails
    ///
    /// # Implementation Notes
    /// - BIOS implementations should use VBE functions
    /// - UEFI implementations should use GOP protocol
    /// - Should detect both framebuffer and mode-setting capabilities
    fn detect_graphics_capabilities(&self) -> Result<GraphicsCapabilities, &'static str>;

    /// Detect memory information
    ///
    /// Detects the total and available memory in the system.
    /// This includes both conventional and extended memory.
    ///
    /// # Returns
    /// Tuple of (total_memory, available_memory) in bytes
    ///
    /// # Errors
    /// Returns an error if memory detection fails
    ///
    /// # Implementation Notes
    /// - BIOS implementations should use INT 0x15/E820
    /// - UEFI implementations should use memory map services
    /// - Should account for reserved memory regions
    fn detect_memory(&self) -> Result<(u64, u64), &'static str>;

    /// Detect CPU information
    ///
    /// Detects detailed CPU information including vendor, family,
    /// model, stepping, and supported features.
    ///
    /// # Returns
    /// CPU information structure with vendor, family, model, and features
    ///
    /// # Errors
    /// Returns an error if CPU detection fails
    ///
    /// # Implementation Notes
    /// - Should use CPUID instruction where available
    /// - Must detect virtualization support (VT-x/AMD-V)
    /// - Should detect 64-bit support (Long Mode)
    /// - Should detect security features (NX bit, SMEP, SMAP)
    fn detect_cpu(&mut self) -> Result<CpuInfo, &'static str>;

    /// Check if hardware supports a specific graphics mode
    ///
    /// Quickly checks if a specific graphics mode is supported
    /// without performing full graphics detection.
    ///
    /// # Arguments
    /// * `width` - Screen width in pixels (320-4096)
    /// * `height` - Screen height in pixels (200-2160)
    /// * `bpp` - Bits per pixel (8, 16, 24, or 32)
    ///
    /// # Returns
    /// True if mode is supported, false otherwise
    ///
    /// # Performance Notes
    /// This method should be fast and not require expensive operations
    fn supports_graphics_mode(&self, width: u16, height: u16, bpp: u8) -> bool;

    /// Get hardware detection capabilities
    ///
    /// Returns information about what hardware detection features
    /// are available in the current implementation.
    ///
    /// # Returns
    /// Detection capabilities structure
    ///
    /// # Usage Notes
    /// The application layer can use this to determine what
    /// detection methods are available and adapt accordingly
    fn get_detection_capabilities(&self) -> DetectionCapabilities;
}

/// CPU Information - Value Object
///
/// Contains CPU identification and feature information.
/// This is a domain value object representing CPU characteristics.
/// It is immutable after creation and follows value object semantics.
///
/// # Design Notes
/// - All fields are public for easy access
/// - Constructor ensures valid data
/// - Helper methods provide convenient access to common queries
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuInfo {
    /// CPU vendor string (e.g., "GenuineIntel", "AuthenticAMD")
    /// 
    /// This is a 12-byte array containing the ASCII vendor string
    /// as returned by CPUID instruction with EAX=0
    pub vendor: [u8; 12],
    
    /// CPU family as defined by the manufacturer
    /// 
    /// Combined with model and stepping to identify a specific CPU
    pub family: u8,
    
    /// CPU model as defined by the manufacturer
    /// 
    /// Combined with family and stepping to identify a specific CPU
    pub model: u8,
    
    /// CPU stepping as defined by the manufacturer
    /// 
    /// Represents the revision of a specific CPU model
    pub stepping: u8,
    
    /// CPU feature flags indicating supported instructions and capabilities
    pub features: CpuFeatures,
    
    /// Maximum supported physical address bits
    /// 
    /// Indicates the maximum physical address width the CPU can handle
    /// Typical values: 32 (32-bit), 36 (PAE), 48 (x86-64)
    pub physical_address_bits: u8,
    
    /// Maximum supported linear (virtual) address bits
    /// 
    /// Indicates the maximum virtual address width the CPU can handle
    /// Typical values: 32 (32-bit), 48 (x86-64)
    pub linear_address_bits: u8,
}

impl CpuInfo {
    /// Create new CPU information with validation
    ///
    /// # Arguments
    /// * `vendor` - 12-byte CPU vendor string
    /// * `family` - CPU family
    /// * `model` - CPU model
    /// * `stepping` - CPU stepping
    /// * `features` - CPU feature flags
    /// * `physical_address_bits` - Physical address width
    /// * `linear_address_bits` - Linear address width
    ///
    /// # Returns
    /// New CpuInfo instance
    ///
    /// # Validation
    /// Ensures address bits are reasonable (>= 32, <= 52)
    pub fn new(
        vendor: [u8; 12],
        family: u8,
        model: u8,
        stepping: u8,
        features: CpuFeatures,
        physical_address_bits: u8,
        linear_address_bits: u8,
    ) -> Self {
        // Basic validation - in a real implementation, might be more strict
        let physical_bits = physical_address_bits.max(32).min(52);
        let linear_bits = linear_address_bits.max(32).min(52);
        
        Self {
            vendor,
            family,
            model,
            stepping,
            features,
            physical_address_bits: physical_bits,
            linear_address_bits: linear_bits,
        }
    }

    /// Get vendor string as a &str
    ///
    /// # Returns
    /// Vendor string or "Unknown" if invalid
    ///
    /// # Safety
    /// This is safe because vendor is always 12 bytes and we're
    /// creating a string slice that checks for UTF-8 validity
    pub fn vendor_string(&self) -> &str {
        // Find the null terminator if present
        let end = self.vendor.iter().position(|&b| b == 0).unwrap_or(12);
        core::str::from_utf8(&self.vendor[..end]).unwrap_or("Unknown")
    }

    /// Check if CPU supports 64-bit mode (Long Mode)
    ///
    /// # Returns
    /// True if CPU supports 64-bit, false otherwise
    pub fn supports_64bit(&self) -> bool {
        self.features.lm
    }

    /// Check if CPU supports virtualization
    ///
    /// # Returns
    /// True if CPU supports either Intel VT-x or AMD-V
    pub fn supports_virtualization(&self) -> bool {
        self.features.vmx || self.features.svm
    }

    /// Check if CPU supports NX (No-Execute) bit
    ///
    /// # Returns
    /// True if CPU supports NX bit, false otherwise
    pub fn supports_nx(&self) -> bool {
        self.features.nx
    }

    /// Check if CPU supports PAE (Physical Address Extension)
    ///
    /// # Returns
    /// True if CPU supports PAE, false otherwise
    pub fn supports_pae(&self) -> bool {
        self.features.pae
    }

    /// Get CPU identification string
    ///
    /// # Returns
    /// Formatted string with vendor, family, model, and stepping
    pub fn identification_string(&self) -> alloc::string::String {
        alloc::format!(
            "{} Family {} Model {} Stepping {}",
            self.vendor_string(),
            self.family,
            self.model,
            self.stepping
        )
    }
}

/// CPU Feature Flags - Value Object
///
/// Represents CPU feature flags and capabilities.
/// This is a domain value object containing CPU features.
/// It is immutable after creation and follows value object semantics.
///
/// # Design Notes
/// - All features are boolean flags
/// - Default implementation provides conservative defaults
/// - Features are based on x86 CPUID instruction
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CpuFeatures {
    /// FPU (Floating Point Unit) - x87 FPU present
    pub fpu: bool,
    
    /// VME (Virtual Mode Extensions) - Virtual 8086 mode enhancements
    pub vme: bool,
    
    /// DE (Debugging Extensions) - I/O breakpoints
    pub de: bool,
    
    /// PSE (Page Size Extension) - 4MB pages support
    pub pse: bool,
    
    /// TSC (Time Stamp Counter) - RDTSC instruction support
    pub tsc: bool,
    
    /// MSR (Model Specific Registers) - RDMSR/WRMSR instructions
    pub msr: bool,
    
    /// PAE (Physical Address Extension) - 36-bit physical addressing
    pub pae: bool,
    
    /// MCE (Machine Check Exception) - Machine check exception support
    pub mce: bool,
    
    /// CX8 (CMPXCHG8B instruction) - 8-byte compare and swap
    pub cx8: bool,
    
    /// APIC (Advanced Programmable Interrupt Controller) - On-chip APIC
    pub apic: bool,
    
    /// SEP (SYSENTER/SYSEXIT instructions) - Fast system calls
    pub sep: bool,
    
    /// MTRR (Memory Type Range Registers) - Memory type control
    pub mtrr: bool,
    
    /// PGE (Page Global Enable) - Global page flag support
    pub pge: bool,
    
    /// MCA (Machine Check Architecture) - Machine check architecture
    pub mca: bool,
    
    /// CMOV (Conditional Move Instruction) - CMOV instruction support
    pub cmov: bool,
    
    /// PAT (Page Attribute Table) - Page attribute table
    pub pat: bool,
    
    /// PSE36 (36-bit Page Size Extension) - 4GB+ addressing with PSE
    pub pse36: bool,
    
    /// PSN (Processor Serial Number) - CPU serial number (usually disabled)
    pub psn: bool,
    
    /// CLFLUSH (Cache Line Flush) - CLFLUSH instruction support
    pub clflush: bool,
    
    /// DS (Debug Store) - Debug store area
    pub ds: bool,
    
    /// TM (Thermal Monitor) - Thermal monitoring and control
    pub tm: bool,
    
    /// HTT (Hyper-Threading Technology) - Multiple logical processors
    pub htt: bool,
    
    /// TM2 (Thermal Monitor 2) - Enhanced thermal monitoring
    pub tm2: bool,
    
    /// IA-64 Architecture - Itanium architecture support
    pub ia64: bool,
    
    /// PBE (Pending Break Enable) - Pending break enable
    pub pbe: bool,
    
    /// SSE (Streaming SIMD Extensions) - SSE instruction support
    pub sse: bool,
    
    /// SSE2 (Streaming SIMD Extensions 2) - SSE2 instruction support
    pub sse2: bool,
    
    /// SS (Self-Snoop) - Cache snoop control
    pub ss: bool,
    
    /// LM (Long Mode) - x86-64 architecture support
    pub lm: bool,
    
    /// 3DNow! instructions - AMD 3DNow! instruction support
    pub now: bool,
    
    /// 3DNow! extensions - AMD 3DNow! extensions
    pub nowext: bool,
    
    /// VMX (Intel Virtualization Technology) - Intel VT-x support
    pub vmx: bool,
    
    /// SVM (AMD Virtualization Technology) - AMD-V support
    pub svm: bool,
    
    /// NX (No-execute bit) - Execute disable support
    pub nx: bool,
}

impl Default for CpuFeatures {
    fn default() -> Self {
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
            clflush: false,
            ds: false,
            tm: false,
            pbe: false,
            sse: false,
            sse2: false,
            ss: false,
            htt: false,
            tm2: false,
            ia64: false,
            lm: false,
            now: false,
            nowext: false,
            vmx: false,
            svm: false,
            nx: false,
        }
    }
}

/// Hardware Detection Capabilities - Value Object
///
/// Represents what hardware detection features are available.
/// This is a domain value object describing detection capabilities.
/// It is immutable after creation and follows value object semantics.
///
/// # Design Notes
/// - Indicates what detection methods are available
/// - Helps application layer adapt to available capabilities
/// - Supports progressive enhancement based on available features
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetectionCapabilities {
    /// CPU detection is available
    /// 
    /// Indicates if CPUID-based CPU detection is supported
    pub cpu_detection: bool,
    
    /// Memory detection is available
    /// 
    /// Indicates if memory map detection is supported
    pub memory_detection: bool,
    
    /// Graphics detection is available
    /// 
    /// Indicates if graphics capabilities detection is supported
    pub graphics_detection: bool,
    
    /// ACPI detection is available
    /// 
    /// Indicates if ACPI table parsing is supported
    pub acpi_detection: bool,
    
    /// PCI device enumeration is available
    /// 
    /// Indicates if PCI bus enumeration is supported
    pub pci_enumeration: bool,
    
    /// USB device enumeration is available
    /// 
    /// Indicates if USB device enumeration is supported
    pub usb_enumeration: bool,
}

impl DetectionCapabilities {
    /// Create new detection capabilities
    ///
    /// # Arguments
    /// * `cpu_detection` - CPU detection availability
    /// * `memory_detection` - Memory detection availability
    /// * `graphics_detection` - Graphics detection availability
    /// * `acpi_detection` - ACPI detection availability
    /// * `pci_enumeration` - PCI enumeration availability
    /// * `usb_enumeration` - USB enumeration availability
    ///
    /// # Returns
    /// New DetectionCapabilities instance
    pub fn new(
        cpu_detection: bool,
        memory_detection: bool,
        graphics_detection: bool,
        acpi_detection: bool,
        pci_enumeration: bool,
        usb_enumeration: bool,
    ) -> Self {
        Self {
            cpu_detection,
            memory_detection,
            graphics_detection,
            acpi_detection,
            pci_enumeration,
            usb_enumeration,
        }
    }

    /// Check if all basic detection capabilities are available
    ///
    /// Basic detection includes CPU, memory, and graphics detection.
    /// This is the minimum required for most bootloader operations.
    ///
    /// # Returns
    /// True if basic detection is available, false otherwise
    pub fn has_basic_detection(&self) -> bool {
        self.cpu_detection && self.memory_detection && self.graphics_detection
    }

    /// Check if advanced detection capabilities are available
    ///
    /// Advanced detection includes basic detection plus ACPI and PCI.
    /// This enables more sophisticated bootloader features.
    ///
    /// # Returns
    /// True if advanced detection is available, false otherwise
    pub fn has_advanced_detection(&self) -> bool {
        self.has_basic_detection() && self.acpi_detection && self.pci_enumeration
    }

    /// Check if full detection capabilities are available
    ///
    /// Full detection includes all supported detection methods.
    /// This provides the most comprehensive hardware information.
    ///
    /// # Returns
    /// True if full detection is available, false otherwise
    pub fn has_full_detection(&self) -> bool {
        self.has_advanced_detection() && self.usb_enumeration
    }

    /// Get detection capability level
    ///
    /// # Returns
    /// String describing the detection capability level
    pub fn capability_level(&self) -> &'static str {
        if self.has_full_detection() {
            "full"
        } else if self.has_advanced_detection() {
            "advanced"
        } else if self.has_basic_detection() {
            "basic"
        } else {
            "minimal"
        }
    }
}

impl Default for DetectionCapabilities {
    fn default() -> Self {
        Self {
            cpu_detection: true,
            memory_detection: true,
            graphics_detection: true,
            acpi_detection: false,
            pci_enumeration: false,
            usb_enumeration: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_info_creation() {
        let vendor = b"GenuineIntel";
        let mut vendor_array = [0u8; 12];
        vendor_array[..vendor.len()].copy_from_slice(vendor);
        
        let features = CpuFeatures {
            fpu: true,
            pae: true,
            lm: true,
            nx: true,
            vmx: true,
            ..Default::default()
        };
        
        let cpu_info = CpuInfo::new(
            vendor_array,
            6,
            15,
            1,
            features,
            48,
            48,
        );

        assert_eq!(cpu_info.family, 6);
        assert_eq!(cpu_info.model, 15);
        assert_eq!(cpu_info.stepping, 1);
        assert_eq!(cpu_info.vendor_string(), "GenuineIntel");
        assert!(cpu_info.supports_64bit());
        assert!(cpu_info.supports_nx());
        assert!(cpu_info.supports_virtualization());
        assert!(cpu_info.supports_pae());
    }

    #[test]
    fn test_cpu_info_validation() {
        let vendor = b"AuthenticAMD";
        let mut vendor_array = [0u8; 12];
        vendor_array[..vendor.len()].copy_from_slice(vendor);
        
        let features = CpuFeatures::default();
        
        // Test with invalid address bits (too small)
        let cpu_info = CpuInfo::new(
            vendor_array,
            23,
            1,
            0,
            features,
            16, // Too small, should be clamped to 32
            48,
        );
        assert_eq!(cpu_info.physical_address_bits, 32);
        
        // Test with invalid address bits (too large)
        let cpu_info = CpuInfo::new(
            vendor_array,
            23,
            1,
            0,
            features,
            64, // Too large, should be clamped to 52
            48,
        );
        assert_eq!(cpu_info.physical_address_bits, 52);
    }

    #[test]
    fn test_detection_capabilities() {
        let basic = DetectionCapabilities::new(true, true, true, false, false, false);
        assert!(basic.has_basic_detection());
        assert!(!basic.has_advanced_detection());
        assert!(!basic.has_full_detection());
        assert_eq!(basic.capability_level(), "basic");

        let advanced = DetectionCapabilities::new(true, true, true, true, true, false);
        assert!(advanced.has_basic_detection());
        assert!(advanced.has_advanced_detection());
        assert!(!advanced.has_full_detection());
        assert_eq!(advanced.capability_level(), "advanced");

        let full = DetectionCapabilities::new(true, true, true, true, true, true);
        assert!(full.has_basic_detection());
        assert!(full.has_advanced_detection());
        assert!(full.has_full_detection());
        assert_eq!(full.capability_level(), "full");
    }

    #[test]
    fn test_cpu_features() {
        let mut features = CpuFeatures::default();
        features.lm = true;
        features.nx = true;
        features.vmx = true;

        let vendor = b"GenuineIntel";
        let mut vendor_array = [0u8; 12];
        vendor_array[..vendor.len()].copy_from_slice(vendor);
        
        let cpu_info = CpuInfo::new(
            vendor_array,
            6,
            15,
            1,
            features,
            48,
            48,
        );

        assert!(cpu_info.supports_64bit());
        assert!(cpu_info.supports_nx());
        assert!(cpu_info.supports_virtualization());
        assert!(!cpu_info.supports_pae()); // Not set in this test
    }

    #[test]
    fn test_cpu_identification_string() {
        let vendor = b"AuthenticAMD";
        let mut vendor_array = [0u8; 12];
        vendor_array[..vendor.len()].copy_from_slice(vendor);
        
        let features = CpuFeatures::default();
        let cpu_info = CpuInfo::new(vendor_array, 23, 1, 0, features, 48, 48);
        
        let id_string = cpu_info.identification_string();
        assert!(id_string.contains("AuthenticAMD"));
        assert!(id_string.contains("Family 23"));
        assert!(id_string.contains("Model 1"));
        assert!(id_string.contains("Stepping 0"));
    }

    #[test]
    fn test_detection_capabilities_default() {
        let caps = DetectionCapabilities::default();
        assert!(caps.cpu_detection);
        assert!(caps.memory_detection);
        assert!(caps.graphics_detection);
        assert!(!caps.acpi_detection);
        assert!(!caps.pci_enumeration);
        assert!(!caps.usb_enumeration);
        assert_eq!(caps.capability_level(), "basic");
    }
}
