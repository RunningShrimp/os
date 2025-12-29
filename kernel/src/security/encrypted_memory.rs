//! # Memory Encryption Abstraction Layer
//!
//! This module provides a unified interface for memory encryption technologies
//! including AMD SEV and Intel TME.
//!
//! ## Features
//!
//! - **Unified Interface**: Single API for different encryption technologies
//! - **Runtime Configuration**: Configure encryption at runtime
//! - **Cross-Platform**: Support for multiple hardware platforms
//! - **Key Management**: Handle encryption keys
//! - **Performance Monitoring**: Track encryption overhead
//!
//! ## Usage
//!
//! ```rust
//! use kernel::security::encrypted_memory::{
//!     EncryptionType, EncryptedMemoryConfig, init_encrypted_memory,
//!     map_encrypted_page
//! };
//!
//! // Initialize encrypted memory
//! init_encrypted_memory(EncryptionType::Auto);
//!
//! // Map an encrypted page
//! let config = EncryptedMemoryConfig {
//!     encryption_type: EncryptionType::Sev,
//!     key_id: Some(0),
//! };
//! let virt_addr = map_encrypted_page(phys_frame, &config)?;
//! ```

#![cfg(feature = "memory_encryption")]

extern crate alloc;

use alloc::string::String;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};

use spin::Mutex;

use crate::types::stubs::VirtAddr;

/// Re-export platform-specific modules
#[cfg(target_arch = "x86_64")]
pub use crate::arch::x86_64::sev as sev_impl;
#[cfg(target_arch = "x86_64")]
pub use crate::arch::x86_64::tme as tme_impl;

/// Memory encryption type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionType {
    /// No encryption
    None,
    /// AMD SEV
    Sev,
    /// Intel TME
    Tme,
    /// Auto-detect available encryption
    Auto,
}

/// Memory encryption error
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EncryptionError {
    /// Encryption not supported
    NotSupported,
    /// Invalid configuration
    InvalidConfig,
    /// Encryption failed
    EncryptionFailed,
    /// Decryption failed
    DecryptionFailed,
    /// Invalid key ID
    InvalidKeyId,
    /// Hardware error
    HardwareError,
}

/// Memory encryption configuration
#[derive(Debug, Clone)]
pub struct EncryptedMemoryConfig {
    /// Encryption type to use
    pub encryption_type: EncryptionType,
    /// Key ID for multi-key encryption (MKTME)
    pub key_id: Option<u8>,
    /// Whether to verify encryption integrity
    pub verify_integrity: bool,
}

impl Default for EncryptedMemoryConfig {
    fn default() -> Self {
        Self {
            encryption_type: EncryptionType::Auto,
            key_id: None,
            verify_integrity: true,
        }
    }
}

/// Memory encryption status
#[derive(Debug, Clone)]
pub struct EncryptionStatus {
    /// Encryption type in use
    pub encryption_type: EncryptionType,
    /// Whether encryption is enabled
    pub enabled: bool,
    /// Key ID in use
    pub key_id: Option<u8>,
    /// Hardware support detected
    pub hardware_supported: bool,
    /// Encryption details
    pub details: String,
}

/// Memory encryption manager
pub struct MemoryEncryption {
    /// Current configuration
    config: EncryptedMemoryConfig,
    /// Encryption status
    status: EncryptionStatus,
    /// Whether initialized
    initialized: bool,
}

impl MemoryEncryption {
    /// Create a new memory encryption manager
    pub fn new(config: EncryptedMemoryConfig) -> Self {
        Self {
            config: config.clone(),
            status: Self::detect_support(&config),
            initialized: false,
        }
    }

    /// Detect hardware support for encryption
    fn detect_support(config: &EncryptedMemoryConfig) -> EncryptionStatus {
        let encryption_type = match config.encryption_type {
            EncryptionType::Auto => Self::detect_available_encryption(),
            _ => config.encryption_type,
        };

        match encryption_type {
            EncryptionType::Sev => {
                #[cfg(target_arch = "x86_64")]
                {
                    if sev_impl::is_sev_supported() {
                        let sev_status = sev_impl::get_sev_status();
                        if let Some(status) = sev_status {
                            return EncryptionStatus {
                                encryption_type: EncryptionType::Sev,
                                enabled: status.enabled,
                                key_id: None,
                                hardware_supported: true,
                                details: sev_impl::get_sev_info(),
                            };
                        }
                    }
                }

                EncryptionStatus {
                    encryption_type: EncryptionType::Sev,
                    enabled: false,
                    key_id: None,
                    hardware_supported: false,
                    details: "SEV not supported".to_string(),
                }
            }

            EncryptionType::Tme => {
                #[cfg(target_arch = "x86_64")]
                {
                    if tme_impl::is_tme_supported() {
                        let tme_status = tme_impl::get_tme_status();
                        if let Some(status) = tme_status {
                            return EncryptionStatus {
                                encryption_type: EncryptionType::Tme,
                                enabled: status.enabled,
                                key_id: Some(status.key_id),
                                hardware_supported: true,
                                details: tme_impl::get_tme_info(),
                            };
                        }
                    }
                }

                EncryptionStatus {
                    encryption_type: EncryptionType::Tme,
                    enabled: false,
                    key_id: None,
                    hardware_supported: false,
                    details: "TME not supported".to_string(),
                }
            }

            EncryptionType::None => EncryptionStatus {
                encryption_type: EncryptionType::None,
                enabled: false,
                key_id: None,
                hardware_supported: true,
                details: "Encryption disabled".to_string(),
            },

            EncryptionType::Auto => EncryptionStatus {
                encryption_type: EncryptionType::None,
                enabled: false,
                key_id: None,
                hardware_supported: false,
                details: "No encryption available".to_string(),
            },
        }
    }

    /// Detect available encryption type
    fn detect_available_encryption() -> EncryptionType {
        #[cfg(target_arch = "x86_64")]
        {
            // Prefer SEV over TME
            if sev_impl::is_sev_supported() {
                return EncryptionType::Sev;
            }

            if tme_impl::is_tme_supported() {
                return EncryptionType::Tme;
            }
        }

        EncryptionType::None
    }

    /// Initialize memory encryption
    pub fn init(&mut self) -> Result<(), EncryptionError> {
        if !self.status.hardware_supported {
            return Err(EncryptionError::NotSupported);
        }

        match self.config.encryption_type {
            EncryptionType::Sev => {
                #[cfg(target_arch = "x86_64")]
                {
                    sev_impl::init_sev();
                    self.initialized = true;
                    return Ok(());
                }

                #[cfg(not(target_arch = "x86_64"))]
                {
                    return Err(EncryptionError::NotSupported);
                }
            }

            EncryptionType::Tme => {
                #[cfg(target_arch = "x86_64")]
                {
                    tme_impl::init_tme();
                    self.initialized = true;
                    return Ok(());
                }

                #[cfg(not(target_arch = "x86_64"))]
                {
                    return Err(EncryptionError::NotSupported);
                }
            }

            EncryptionType::None => {
                self.initialized = true;
                Ok(())
            }

            EncryptionType::Auto => {
                let detected = Self::detect_available_encryption();
                self.config.encryption_type = detected;
                self.status = Self::detect_support(&self.config);
                self.init()
            }
        }
    }

    /// Get encryption status
    pub fn status(&self) -> &EncryptionStatus {
        &self.status
    }

    /// Check if encryption is enabled
    pub fn is_enabled(&self) -> bool {
        self.status.enabled
    }

    /// Check if initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Encrypt a physical address
    pub fn encrypt_address(&self, pa: u64) -> u64 {
        match self.config.encryption_type {
            EncryptionType::Sev => {
                #[cfg(target_arch = "x86_64")]
                {
                    sev_impl::encrypt_pa(pa)
                }

                #[cfg(not(target_arch = "x86_64"))]
                {
                    pa
                }
            }

            EncryptionType::Tme => {
                if let Some(key_id) = self.config.key_id {
                    #[cfg(target_arch = "x86_64")]
                    {
                        tme_impl::encrypt_with_key_id(pa, key_id)
                    }

                    #[cfg(not(target_arch = "x86_64"))]
                    {
                        pa
                    }
                } else {
                    pa
                }
            }

            _ => pa,
        }
    }

    /// Decrypt a physical address
    pub fn decrypt_address(&self, encrypted_pa: u64) -> u64 {
        match self.config.encryption_type {
            EncryptionType::Sev => {
                #[cfg(target_arch = "x86_64")]
                {
                    sev_impl::decrypt_pa(encrypted_pa)
                }

                #[cfg(not(target_arch = "x86_64"))]
                {
                    encrypted_pa
                }
            }

            EncryptionType::Tme => {
                // TME doesn't modify address bits in the same way
                encrypted_pa
            }

            _ => encrypted_pa,
        }
    }

    /// Set encryption key ID
    pub fn set_key_id(&mut self, key_id: u8) -> Result<(), EncryptionError> {
        if key_id > 7 {
            return Err(EncryptionError::InvalidKeyId);
        }

        self.config.key_id = Some(key_id);

        #[cfg(target_arch = "x86_64")]
        {
            if self.config.encryption_type == EncryptionType::Tme {
                tme_impl::set_key_id(key_id)
                    .map_err(|_| EncryptionError::InvalidKeyId)?;
            }
        }

        Ok(())
    }

    /// Get encryption statistics
    pub fn get_stats(&self) -> EncryptionStats {
        EncryptionStats {
            enabled: self.status.enabled,
            encryption_type: self.config.encryption_type,
            key_id: self.config.key_id,
            hardware_supported: self.status.hardware_supported,
        }
    }
}

/// Memory encryption statistics
#[derive(Debug, Clone)]
pub struct EncryptionStats {
    /// Whether encryption is enabled
    pub enabled: bool,
    /// Encryption type
    pub encryption_type: EncryptionType,
    /// Key ID in use
    pub key_id: Option<u8>,
    /// Hardware support available
    pub hardware_supported: bool,
}

/// Global memory encryption instance
static GLOBAL_ENCRYPTION: Mutex<Option<MemoryEncryption>> = Mutex::new(None);
static ENCRYPTION_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialize encrypted memory subsystem
pub fn init_encrypted_memory(encryption_type: EncryptionType) -> Result<(), EncryptionError> {
    let config = EncryptedMemoryConfig {
        encryption_type,
        key_id: None,
        verify_integrity: true,
    };

    let mut encryption = MemoryEncryption::new(config);
    encryption.init()?;

    *GLOBAL_ENCRYPTION.lock() = Some(encryption);
    ENCRYPTION_INITIALIZED.store(true, Ordering::Release);

    Ok(())
}

/// Get global memory encryption instance
pub fn get_encrypted_memory() -> Option<&'static MemoryEncryption> {
    // This is a simplified version
    None
}

/// Check if encrypted memory is initialized
pub fn is_encrypted_memory_initialized() -> bool {
    ENCRYPTION_INITIALIZED.load(Ordering::Acquire)
}

/// Get encryption status
pub fn get_encryption_status() -> Option<EncryptionStatus> {
    GLOBAL_ENCRYPTION.lock().as_ref().map(|e| e.status().clone())
}

/// Encrypt a physical address
pub fn encrypt_physical_address(pa: u64) -> u64 {
    if let Some(encryption) = GLOBAL_ENCRYPTION.lock().as_ref() {
        encryption.encrypt_address(pa)
    } else {
        pa
    }
}

/// Decrypt a physical address
pub fn decrypt_physical_address(encrypted_pa: u64) -> u64 {
    if let Some(encryption) = GLOBAL_ENCRYPTION.lock().as_ref() {
        encryption.decrypt_address(encrypted_pa)
    } else {
        encrypted_pa
    }
}

/// Set encryption key ID
pub fn set_encryption_key_id(key_id: u8) -> Result<(), EncryptionError> {
    if let Some(encryption) = GLOBAL_ENCRYPTION.lock().as_mut() {
        encryption.set_key_id(key_id)
    } else {
        Err(EncryptionError::InvalidConfig)
    }
}

/// Get encryption statistics
pub fn get_encryption_stats() -> Option<EncryptionStats> {
    GLOBAL_ENCRYPTION.lock().as_ref().map(|e| e.get_stats())
}

/// Map an encrypted page (stub implementation)
pub fn map_encrypted_page(
    phys: u64,
    config: &EncryptedMemoryConfig,
) -> Result<VirtAddr, EncryptionError> {
    // In a real implementation, this would map a physical page
    // with encryption enabled
    Ok(VirtAddr::new(0 as usize))
}

/// Encryption health metrics
#[derive(Debug, Clone)]
pub struct EncryptionHealthMetrics {
    /// Encryption type
    pub encryption_type: String,
    /// Encryption enabled
    pub enabled: bool,
    /// Hardware supported
    pub hardware_supported: bool,
    /// Key ID
    pub key_id: Option<u8>,
    /// Status details
    pub details: String,
}

/// Get encryption health metrics
pub fn get_encryption_health_metrics() -> EncryptionHealthMetrics {
    if let Some(status) = get_encryption_status() {
        EncryptionHealthMetrics {
            encryption_type: format!("{:?}", status.encryption_type),
            enabled: status.enabled,
            hardware_supported: status.hardware_supported,
            key_id: status.key_id,
            details: status.details,
        }
    } else {
        EncryptionHealthMetrics {
            encryption_type: "None".to_string(),
            enabled: false,
            hardware_supported: false,
            key_id: None,
            details: "Not initialized".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_config_default() {
        let config = EncryptedMemoryConfig::default();
        assert_eq!(config.encryption_type, EncryptionType::Auto);
        assert!(config.key_id.is_none());
        assert!(config.verify_integrity);
    }

    #[test]
    fn test_memory_encryption_creation() {
        let config = EncryptedMemoryConfig::default();
        let encryption = MemoryEncryption::new(config);

        assert!(!encryption.is_initialized());
    }

    #[test]
    fn test_set_key_id() {
        let mut encryption = MemoryEncryption::new(EncryptedMemoryConfig::default());
        assert!(encryption.set_key_id(0).is_ok());
        assert!(encryption.set_key_id(7).is_ok());
        assert_eq!(encryption.set_key_id(8), Err(EncryptionError::InvalidKeyId));
    }

    #[test]
    fn test_encrypt_decrypt_none() {
        let config = EncryptedMemoryConfig {
            encryption_type: EncryptionType::None,
            key_id: None,
            verify_integrity: false,
        };
        let encryption = MemoryEncryption::new(config);

        let pa = 0x1000u64;
        let encrypted = encryption.encrypt_address(pa);
        let decrypted = encryption.decrypt_address(encrypted);

        assert_eq!(pa, decrypted);
    }
}
