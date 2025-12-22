//! Advanced Boot Protocol Support
//!
//! Provides support for advanced boot protocols including:
//! - Multiboot3 protocol framework
//! - UEFI boot service integration
//! - Hybrid boot mode detection
//! - Protocol version negotiation

use core::fmt;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;


/// Boot protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootProtocolType {
    Multiboot,          // Original Multiboot
    Multiboot2,         // Multiboot2 (current standard)
    Multiboot3,         // Multiboot3 (future)
    UEFI,               // UEFI boot services
    EFI,                // EFI (legacy)
    DirectBoot,         // Direct boot (no bootloader protocol)
    Unknown,
}

impl fmt::Display for BootProtocolType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootProtocolType::Multiboot => write!(f, "Multiboot"),
            BootProtocolType::Multiboot2 => write!(f, "Multiboot2"),
            BootProtocolType::Multiboot3 => write!(f, "Multiboot3"),
            BootProtocolType::UEFI => write!(f, "UEFI"),
            BootProtocolType::EFI => write!(f, "EFI"),
            BootProtocolType::DirectBoot => write!(f, "Direct Boot"),
            BootProtocolType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Boot mode type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootMode {
    Legacy,             // Legacy BIOS boot
    UEFI,               // UEFI Firmware boot
    Hybrid,             // Supports both Legacy and UEFI
    Secure,             // Secure Boot enabled
    Unknown,
}

impl fmt::Display for BootMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BootMode::Legacy => write!(f, "Legacy BIOS"),
            BootMode::UEFI => write!(f, "UEFI"),
            BootMode::Hybrid => write!(f, "Hybrid"),
            BootMode::Secure => write!(f, "Secure Boot"),
            BootMode::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Protocol feature
#[derive(Debug, Clone)]
pub struct ProtocolFeature {
    pub name: String,
    pub version: u32,
    pub is_supported: bool,
    pub is_required: bool,
}

impl ProtocolFeature {
    /// Create new protocol feature
    pub fn new(name: &str, version: u32) -> Self {
        ProtocolFeature {
            name: String::from(name),
            version,
            is_supported: false,
            is_required: false,
        }
    }

    /// Mark as required
    pub fn required(mut self) -> Self {
        self.is_required = true;
        self
    }

    /// Check if feature meets requirements
    pub fn meets_requirement(&self) -> bool {
        if self.is_required {
            self.is_supported
        } else {
            true
        }
    }
}

impl fmt::Display for ProtocolFeature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = if self.is_supported { "✓" } else { "✗" };
        write!(
            f,
            "{} {} v{} {}",
            status,
            self.name,
            self.version,
            if self.is_required { "(required)" } else { "" }
        )
    }
}

/// Protocol information
#[derive(Debug, Clone)]
pub struct ProtocolInfo {
    pub protocol_type: BootProtocolType,
    pub boot_mode: BootMode,
    pub version: u32,
    pub features: Vec<ProtocolFeature>,
    pub is_available: bool,
}

impl ProtocolInfo {
    /// Create new protocol info
    pub fn new(protocol_type: BootProtocolType, boot_mode: BootMode) -> Self {
        ProtocolInfo {
            protocol_type,
            boot_mode,
            version: 0,
            features: Vec::new(),
            is_available: false,
        }
    }

    /// Add feature
    pub fn add_feature(&mut self, feature: ProtocolFeature) {
        self.features.push(feature);
    }

    /// Check if protocol is valid
    pub fn is_valid(&self) -> bool {
        self.version > 0
            && self.is_available
            && self.features.iter().all(|f| f.meets_requirement())
    }

    /// Get unsupported required features
    pub fn unsupported_required_features(&self) -> Vec<&ProtocolFeature> {
        self.features
            .iter()
            .filter(|f| f.is_required && !f.is_supported)
            .collect()
    }
}

impl fmt::Display for ProtocolInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} v{} ({}) - {}",
            self.protocol_type,
            self.version,
            self.boot_mode,
            if self.is_available { "Available" } else { "Unavailable" }
        )
    }
}

/// Advanced Boot Protocol Manager
pub struct AdvancedBootProtocol {
    available_protocols: Vec<ProtocolInfo>,
    current_protocol: Option<BootProtocolType>,
    boot_mode: BootMode,
    protocol_negotiation_count: u32,
    protocol_fallback_count: u32,
}

impl AdvancedBootProtocol {
    /// Create new advanced boot protocol manager
    pub fn new() -> Self {
        AdvancedBootProtocol {
            available_protocols: Vec::new(),
            current_protocol: None,
            boot_mode: BootMode::Unknown,
            protocol_negotiation_count: 0,
            protocol_fallback_count: 0,
        }
    }

    /// Register protocol
    pub fn register_protocol(&mut self, protocol: ProtocolInfo) -> bool {
        if !protocol.protocol_type.eq(&BootProtocolType::Unknown) {
            self.available_protocols.push(protocol);
            true
        } else {
            false
        }
    }

    /// Detect boot mode
    pub fn detect_boot_mode(&mut self) -> BootMode {
        // Framework for detecting actual boot mode from firmware
        self.boot_mode = BootMode::Hybrid;
        self.boot_mode
    }

    /// Negotiate protocol
    pub fn negotiate_protocol(&mut self) -> Option<BootProtocolType> {
        self.protocol_negotiation_count += 1;

        // Priority: Multiboot3 > UEFI > Multiboot2 > Legacy
        let protocol_priority = [
            BootProtocolType::Multiboot3,
            BootProtocolType::UEFI,
            BootProtocolType::Multiboot2,
            BootProtocolType::Multiboot,
        ];

        for protocol_type in &protocol_priority {
            if let Some(proto) = self.available_protocols.iter().find(|p| p.protocol_type == *protocol_type) {
                if proto.is_valid() {
                    self.current_protocol = Some(*protocol_type);
                    return Some(*protocol_type);
                }
            }
        }

        None
    }

    /// Select fallback protocol
    pub fn select_fallback_protocol(&mut self) -> Option<BootProtocolType> {
        self.protocol_fallback_count += 1;

        // Find first available valid protocol
        for proto in &self.available_protocols {
            if proto.is_available {
                self.current_protocol = Some(proto.protocol_type);
                return Some(proto.protocol_type);
            }
        }

        None
    }

    /// Get current protocol
    pub fn get_current_protocol(&self) -> Option<BootProtocolType> {
        self.current_protocol
    }

    /// Check if protocol is available
    pub fn has_protocol(&self, protocol_type: BootProtocolType) -> bool {
        self.available_protocols.iter().any(|p| p.protocol_type == protocol_type && p.is_available)
    }

    /// Get protocol count
    pub fn protocol_count(&self) -> usize {
        self.available_protocols.len()
    }

    /// Get available protocols
    pub fn get_available_protocols(&self) -> Vec<&ProtocolInfo> {
        self.available_protocols
            .iter()
            .filter(|p| p.is_available)
            .collect()
    }

    /// Get protocol details
    pub fn get_protocol_details(&self, protocol_type: BootProtocolType) -> Option<&ProtocolInfo> {
        self.available_protocols.iter().find(|p| p.protocol_type == protocol_type)
    }

    /// Check boot mode
    pub fn get_boot_mode(&self) -> BootMode {
        self.boot_mode
    }

    /// Get protocol statistics
    pub fn get_stats(&self) -> (u32, u32, usize) {
        (self.protocol_negotiation_count, self.protocol_fallback_count, self.protocol_count())
    }

    /// Get detailed protocol report
    pub fn protocol_report(&self) -> String {
        let mut report = String::from("=== Boot Protocol Report ===\n");
        
        report.push_str(&format!("Boot Mode: {}\n", self.boot_mode));
        
        if let Some(current) = self.current_protocol {
            report.push_str(&format!("Current Protocol: {}\n", current));
        }
        
        report.push_str(&format!("\nAvailable Protocols: {}\n", self.protocol_count()));
        for proto in &self.available_protocols {
            report.push_str(&format!("  {}\n", proto));
            for feature in &proto.features {
                report.push_str(&format!("    {}\n", feature));
            }
        }
        
        report.push_str(&format!(
            "\nNegotiations: {}, Fallbacks: {}\n",
            self.protocol_negotiation_count, self.protocol_fallback_count
        ));
        
        report
    }

    /// Check if system is ready for boot
    pub fn is_ready_for_boot(&self) -> bool {
        self.current_protocol.is_some()
            && self.available_protocols
                .iter()
                .any(|p| p.protocol_type == self.current_protocol.unwrap() && p.is_valid())
    }
}

impl fmt::Display for AdvancedBootProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AdvancedBootProtocol {{ mode: {}, protocols: {}, current: {:?} }}",
            self.boot_mode, self.protocol_count(), self.current_protocol
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_protocol_type_display() {
        assert_eq!(BootProtocolType::Multiboot2.to_string(), "Multiboot2");
        assert_eq!(BootProtocolType::UEFI.to_string(), "UEFI");
    }

    #[test]
    fn test_boot_mode_display() {
        assert_eq!(BootMode::Legacy.to_string(), "Legacy BIOS");
        assert_eq!(BootMode::Secure.to_string(), "Secure Boot");
    }

    #[test]
    fn test_protocol_feature_creation() {
        let feature = ProtocolFeature::new("Paging", 1);
        assert_eq!(feature.name, "Paging");
        assert!(!feature.is_supported);
    }

    #[test]
    fn test_protocol_feature_required() {
        let feature = ProtocolFeature::new("Memory Map", 1).required();
        assert!(feature.is_required);
    }

    #[test]
    fn test_protocol_feature_requirement() {
        let mut feature = ProtocolFeature::new("Test", 1).required();
        assert!(!feature.meets_requirement());
        
        feature.is_supported = true;
        assert!(feature.meets_requirement());
    }

    #[test]
    fn test_protocol_info_creation() {
        let proto = ProtocolInfo::new(BootProtocolType::Multiboot2, BootMode::Legacy);
        assert_eq!(proto.protocol_type, BootProtocolType::Multiboot2);
        assert_eq!(proto.boot_mode, BootMode::Legacy);
    }

    #[test]
    fn test_protocol_info_add_feature() {
        let mut proto = ProtocolInfo::new(BootProtocolType::UEFI, BootMode::UEFI);
        let feature = ProtocolFeature::new("Boot Services", 2);
        proto.add_feature(feature);
        
        assert_eq!(proto.features.len(), 1);
    }

    #[test]
    fn test_protocol_info_validity() {
        let mut proto = ProtocolInfo::new(BootProtocolType::Multiboot2, BootMode::Legacy);
        assert!(!proto.is_valid()); // No version, not available
        
        proto.version = 2;
        proto.is_available = true;
        assert!(proto.is_valid());
    }

    #[test]
    fn test_advanced_boot_protocol_creation() {
        let manager = AdvancedBootProtocol::new();
        assert_eq!(manager.protocol_count(), 0);
        assert!(manager.get_current_protocol().is_none());
    }

    #[test]
    fn test_advanced_boot_protocol_register() {
        let mut manager = AdvancedBootProtocol::new();
        let proto = ProtocolInfo::new(BootProtocolType::Multiboot2, BootMode::Legacy);
        
        assert!(manager.register_protocol(proto));
        assert_eq!(manager.protocol_count(), 1);
    }

    #[test]
    fn test_advanced_boot_protocol_detect_mode() {
        let mut manager = AdvancedBootProtocol::new();
        let mode = manager.detect_boot_mode();
        assert_eq!(mode, BootMode::Hybrid);
    }

    #[test]
    fn test_advanced_boot_protocol_negotiate() {
        let mut manager = AdvancedBootProtocol::new();
        
        let mut proto = ProtocolInfo::new(BootProtocolType::Multiboot2, BootMode::Legacy);
        proto.version = 2;
        proto.is_available = true;
        
        manager.register_protocol(proto);
        
        let result = manager.negotiate_protocol();
        assert!(result.is_some());
    }

    #[test]
    fn test_advanced_boot_protocol_has_protocol() {
        let mut manager = AdvancedBootProtocol::new();
        
        let mut proto = ProtocolInfo::new(BootProtocolType::UEFI, BootMode::UEFI);
        proto.is_available = true;
        
        manager.register_protocol(proto);
        assert!(manager.has_protocol(BootProtocolType::UEFI));
        assert!(!manager.has_protocol(BootProtocolType::Multiboot3));
    }

    #[test]
    fn test_advanced_boot_protocol_statistics() {
        let mut manager = AdvancedBootProtocol::new();
        
        let proto = ProtocolInfo::new(BootProtocolType::Multiboot2, BootMode::Legacy);
        manager.register_protocol(proto);
        
        let (neg, fallback, count) = manager.get_stats();
        assert_eq!(count, 1);
        assert_eq!(neg, 0); // No negotiations yet
        assert_eq!(fallback, 0); // No fallbacks yet
    }

    #[test]
    fn test_advanced_boot_protocol_ready_for_boot() {
        let mut manager = AdvancedBootProtocol::new();
        
        let mut proto = ProtocolInfo::new(BootProtocolType::Multiboot2, BootMode::Legacy);
        proto.version = 2;
        proto.is_available = true;
        
        manager.register_protocol(proto);
        manager.negotiate_protocol();
        
        assert!(manager.is_ready_for_boot());
    }

    #[test]
    fn test_advanced_boot_protocol_report() {
        let mut manager = AdvancedBootProtocol::new();
        
        let proto = ProtocolInfo::new(BootProtocolType::UEFI, BootMode::UEFI);
        manager.register_protocol(proto);
        
        let report = manager.protocol_report();
        assert!(report.contains("Boot Protocol Report"));
        assert!(report.contains("Available Protocols"));
    }
}
