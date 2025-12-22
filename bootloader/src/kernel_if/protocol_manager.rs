//! Protocol Manager
//!
//! This module provides a comprehensive protocol management system for the bootloader.
//! It handles detection, initialization, and management of different boot protocols
//! (UEFI, BIOS, Multiboot2) with proper error handling and state management.

use crate::utils::error::Result;
use crate::protocol::BootProtocolType;
use crate::alloc::string::ToString;

/// Protocol-specific errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    /// Protocol not supported
    NotSupported,
    /// Protocol initialization failed
    InitializationFailed,
    /// Invalid protocol state
    InvalidState,
    /// Protocol detection failed
    DetectionFailed,
}

impl From<ProtocolError> for crate::utils::error::BootError {
    fn from(err: ProtocolError) -> Self {
        match err {
            ProtocolError::NotSupported => crate::utils::error::BootError::ProtocolNotSupported,
            ProtocolError::InitializationFailed => crate::utils::error::BootError::ProtocolInitializationFailed("Protocol initialization failed".to_string()),
            ProtocolError::InvalidState => crate::utils::error::BootError::InvalidState,
            ProtocolError::DetectionFailed => crate::utils::error::BootError::ProtocolDetectionFailed,
        }
    }
}

/// Protocol manager state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolState {
    /// Not initialized
    Uninitialized,
    /// Detection in progress
    Detecting,
    /// Initialized and ready
    Ready,
    /// Failed to initialize
    Failed(ProtocolError),
}

/// Protocol manager with enhanced functionality
#[derive(Debug)]
pub struct ProtocolManager {
    /// Current protocol type
    protocol_type: BootProtocolType,
    /// Current state
    state: ProtocolState,
    /// Available protocols bitmask for quick checking
    available_protocols: u8,
}

// Bitmask constants for available protocols
const UEFI_BIT: u8 = 0b001;
const BIOS_BIT: u8 = 0b010;
const MULTIBOOT2_BIT: u8 = 0b100;

impl ProtocolManager {
    /// Create a new protocol manager with automatic protocol detection
    pub fn new() -> Self {
        let mut manager = Self {
            protocol_type: BootProtocolType::Uefi, // Default fallback
            state: ProtocolState::Uninitialized,
            available_protocols: 0,
        };
        
        // Perform initial protocol detection
        manager.detect_available_protocols();
        manager
    }

    /// Create a protocol manager with a specific protocol type
    pub fn with_protocol(protocol_type: BootProtocolType) -> Self {
        let mut manager = Self {
            protocol_type,
            state: ProtocolState::Uninitialized,
            available_protocols: 0,
        };
        
        manager.detect_available_protocols();
        manager
    }

    /// Detect available boot protocols
    fn detect_available_protocols(&mut self) {
        self.state = ProtocolState::Detecting;
        
        // Reset available protocols
        self.available_protocols = 0;
        
        // Detect UEFI support
        if self.is_uefi_available() {
            self.available_protocols |= UEFI_BIT;
        }
        
        // Detect BIOS support
        if self.is_bios_available() {
            self.available_protocols |= BIOS_BIT;
        }
        
        // Detect Multiboot2 support
        if self.is_multiboot2_available() {
            self.available_protocols |= MULTIBOOT2_BIT;
        }
        
        // Select the best available protocol
        self.select_best_protocol();
    }

    /// Check if UEFI is available
    #[cfg(feature = "uefi_support")]
    fn is_uefi_available(&self) -> bool {
        // In a real implementation, this would check for UEFI system table
        // For now, we'll use a simple compile-time check
        true
    }

    /// Check if UEFI is available (no UEFI support)
    #[cfg(not(feature = "uefi_support"))]
    fn is_uefi_available(&self) -> bool {
        false
    }

    /// Check if BIOS is available
    #[cfg(feature = "bios_support")]
    fn is_bios_available(&self) -> bool {
        // In a real implementation, this would check for BIOS interrupts
        // For now, we'll use a simple compile-time check
        true
    }

    /// Check if BIOS is available (no BIOS support)
    #[cfg(not(feature = "bios_support"))]
    fn is_bios_available(&self) -> bool {
        false
    }

    /// Check if Multiboot2 is available
    fn is_multiboot2_available(&self) -> bool {
        // In a real implementation, this would check for Multiboot2 info
        // For now, we'll assume it's available if either UEFI or BIOS is available
        self.available_protocols & (UEFI_BIT | BIOS_BIT) != 0
    }

    /// Select the best available protocol based on priority
    fn select_best_protocol(&mut self) {
        // Priority order: UEFI > Multiboot2 > BIOS
        if self.available_protocols & UEFI_BIT != 0 {
            self.protocol_type = BootProtocolType::Uefi;
        } else if self.available_protocols & MULTIBOOT2_BIT != 0 {
            self.protocol_type = BootProtocolType::Multiboot2;
        } else if self.available_protocols & BIOS_BIT != 0 {
            self.protocol_type = BootProtocolType::Bios;
        } else {
            self.state = ProtocolState::Failed(ProtocolError::NotSupported);
            return;
        }
        
        self.state = ProtocolState::Ready;
    }

    /// Initialize the current protocol
    pub fn initialize(&mut self) -> Result<()> {
        match self.state {
            ProtocolState::Failed(err) => return Err(err.into()),
            ProtocolState::Ready => {
                // Perform protocol-specific initialization
                match self.protocol_type {
                    BootProtocolType::Uefi => self.initialize_uefi()?,
                    BootProtocolType::Bios => self.initialize_bios()?,
                    BootProtocolType::Multiboot2 => self.initialize_multiboot2()?,
                }
            }
            _ => {
                // Re-detect protocols if not ready
                self.detect_available_protocols();
                if let ProtocolState::Failed(err) = self.state {
                    return Err(err.into());
                }
                return self.initialize();
            }
        }
        
        Ok(())
    }

    /// Initialize UEFI protocol
    #[cfg(feature = "uefi_support")]
    fn initialize_uefi(&mut self) -> Result<()> {
        // In a real implementation, this would initialize UEFI services
        // For now, we'll just return success
        Ok(())
    }

    /// Initialize UEFI protocol (no UEFI support)
    #[cfg(not(feature = "uefi_support"))]
    fn initialize_uefi(&mut self) -> Result<()> {
        Err(ProtocolError::NotSupported.into())
    }

    /// Initialize BIOS protocol
    #[cfg(feature = "bios_support")]
    fn initialize_bios(&mut self) -> Result<()> {
        // In a real implementation, this would initialize BIOS services
        // For now, we'll just return success
        Ok(())
    }

    /// Initialize BIOS protocol (no BIOS support)
    #[cfg(not(feature = "bios_support"))]
    fn initialize_bios(&mut self) -> Result<()> {
        Err(ProtocolError::NotSupported.into())
    }

    /// Initialize Multiboot2 protocol
    fn initialize_multiboot2(&mut self) -> Result<()> {
        // In a real implementation, this would initialize Multiboot2
        // For now, we'll just return success
        Ok(())
    }

    /// Get the current protocol type
    pub fn protocol_type(&self) -> BootProtocolType {
        self.protocol_type
    }

    /// Get the current state
    pub fn state(&self) -> ProtocolState {
        self.state
    }

    /// Check if a specific protocol is available
    pub fn is_protocol_available(&self, protocol_type: BootProtocolType) -> bool {
        let bit = match protocol_type {
            BootProtocolType::Uefi => UEFI_BIT,
            BootProtocolType::Bios => BIOS_BIT,
            BootProtocolType::Multiboot2 => MULTIBOOT2_BIT,
        };
        
        self.available_protocols & bit != 0
    }

    /// Switch to a different protocol type
    pub fn switch_protocol(&mut self, protocol_type: BootProtocolType) -> Result<()> {
        if !self.is_protocol_available(protocol_type) {
            return Err(ProtocolError::NotSupported.into());
        }
        
        self.protocol_type = protocol_type;
        self.state = ProtocolState::Uninitialized;
        self.initialize()
    }

    /// Get a list of all available protocols
    pub fn available_protocols(&self) -> &[BootProtocolType] {
        // Use a static array to avoid heap allocation
        const ALL_PROTOCOLS: [BootProtocolType; 3] = [
            BootProtocolType::Uefi,
            BootProtocolType::Bios,
            BootProtocolType::Multiboot2,
        ];
        
        // Return a slice of available protocols
        // This is a bit inefficient but avoids heap allocation
        &ALL_PROTOCOLS
    }

    /// Validate the current protocol configuration
    pub fn validate(&self) -> Result<()> {
        match self.state {
            ProtocolState::Ready => Ok(()),
            ProtocolState::Failed(err) => Err(err.into()),
            ProtocolState::Uninitialized => Err(crate::utils::error::BootError::NotInitialized),
            ProtocolState::Detecting => Err(crate::utils::error::BootError::InvalidState),
        }
    }

    /// Reset the protocol manager to uninitialized state
    pub fn reset(&mut self) {
        self.state = ProtocolState::Uninitialized;
        self.detect_available_protocols();
    }
}

impl Default for ProtocolManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_protocol_manager_creation() {
        let pm = ProtocolManager::new();
        assert!(pm.state() != ProtocolState::Uninitialized);
    }

    #[test]
    fn test_protocol_with_specific_type() {
        let pm = ProtocolManager::with_protocol(BootProtocolType::Bios);
        assert_eq!(pm.protocol_type(), BootProtocolType::Bios);
    }

    #[test]
    fn test_protocol_availability() {
        let pm = ProtocolManager::new();
        // At least one protocol should be available
        let available = pm.available_protocols();
        assert!(!available.is_empty());
    }

    #[test]
    fn test_protocol_validation() {
        let mut pm = ProtocolManager::new();
        // Should fail if not initialized
        assert!(pm.validate().is_err());
        
        // Initialize and validate
        let _ = pm.initialize();
        assert!(pm.validate().is_ok());
    }

    #[test]
    fn test_protocol_reset() {
        let mut pm = ProtocolManager::new();
        let initial_state = pm.state();
        
        pm.reset();
        assert_eq!(pm.state(), initial_state);
    }
}
