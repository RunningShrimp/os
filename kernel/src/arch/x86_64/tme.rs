//! # Intel Total Memory Encryption (TME) Support
//!
//! This module provides support for Intel TME and MKTME memory encryption.
//!
//! ## Features
//!
//! - **TME Detection**: Check CPUID and MSRs for TME support
//! - **MKTME Support**: Multi-Key TME configuration
//! - **Key Management**: Handle encryption keys
//! - **Memory Encryption Status**: Query encryption state
//!
//! ## Usage
//!
//! ```rust
//! use kernel::arch::x86_64::tme::{init_tme, TmeStatus};
//!
//! // Initialize TME
//! let tme_status = init_tme();
//! if tme_status.enabled {
//!     println!("TME is enabled");
//! }
//! ```

#![cfg(feature = "memory_encryption")]

extern crate alloc;

use alloc::string::String;
use core::sync::atomic::{AtomicBool, Ordering};

/// TME MSR addresses
const IA32_TME_CAPABILITY: u32 = 0x981;
const IA32_TME_ACTIVATE: u32 = 0x982;
const IA32_TME_EXCLUDE_MASK: u32 = 0x983;
const IA32_TME_EXCLUDE_BASE: u32 = 0x984;

/// TME capability bits
const TME_CAPABILITY_ENUM: u64 = 1 << 0;
const TME_CAPABILITY_AES_XTS: u64 = 1 << 1;
const TME_CAPABILITY_MKTME: u64 = 1 << 2;

/// TME activate bits
const TME_ACTIVATE_ENABLED: u64 = 1 << 0;
const TME_ACTIVATE_LOCKED: u64 = 1 << 1;

/// TME status
#[derive(Debug, Clone)]
pub struct TmeStatus {
    /// Whether TME is enabled
    pub enabled: bool,
    /// Whether TME is locked (cannot be disabled)
    pub locked: bool,
    /// Key ID in use
    pub key_id: u8,
    /// Available encryption keys
    pub encryption_keys: [u64; 8],
    /// Whether MKTME is supported
    pub mktme_supported: bool,
    /// Whether MKTME is enabled
    pub mktme_enabled: bool,
    /// Number of available keys
    pub num_keys: u8,
    /// Encryption algorithm
    pub algorithm: TmeAlgorithm,
}

/// TME encryption algorithm
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TmeAlgorithm {
    /// AES-XTS 128-bit
    AesXts128,
    /// AES-XTS 256-bit
    AesXts256,
    /// Unknown algorithm
    Unknown,
}

/// TME error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmeError {
    /// TME not supported
    NotSupported,
    /// TME not enabled
    NotEnabled,
    /// Invalid key ID
    InvalidKeyId,
    /// Initialization failed
    InitFailed,
    /// Hardware error
    HardwareError,
}

/// TME configuration
#[derive(Debug, Clone)]
pub struct TmeConfig {
    /// Whether to enable MKTME
    pub enable_mktme: bool,
    /// Number of keys to use (for MKTME)
    pub num_keys: u8,
}

impl Default for TmeConfig {
    fn default() -> Self {
        Self {
            enable_mktme: false,
            num_keys: 1,
        }
    }
}

/// Global TME status
static TME_STATUS: spin::Mutex<Option<TmeStatus>> = spin::Mutex::new(None);
static TME_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Check if TME is supported by the CPU
pub fn is_tme_supported() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        let mut cpuid_result = [0u32; 4];
        unsafe {
            core::arch::asm!(
                "cpuid",
                inout("eax") 0x7 => eax,
                inout("ecx") 0x0 => ecx,
                lateout("ebx") ebx,
                lateout("edx") edx,
            );
            cpuid_result[0] = eax;
            cpuid_result[1] = ebx;
            cpuid_result[2] = ecx;
            cpuid_result[3] = edx;
        }

        // Bit 13 of EBX indicates TME support
        (cpuid_result[1] & (1 << 13)) != 0
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// Check if MKTME is supported
pub fn is_mktme_supported() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        if !is_tme_supported() {
            return false;
        }

        // Read TME capability MSR
        let caps = read_tme_msr(IA32_TME_CAPABILITY);

        // Check MKTME capability
        (caps & TME_CAPABILITY_MKTME) != 0
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// Read TME MSR
#[cfg(target_arch = "x86_64")]
fn read_tme_msr(msr: u32) -> u64 {
    let (value_low, value_high): (u32, u32);
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") value_low,
            out("edx") value_high,
        );
    }
    ((value_high as u64) << 32) | (value_low as u64)
}

/// Write TME MSR
#[cfg(target_arch = "x86_64")]
fn write_tme_msr(msr: u32, value: u64) {
    let value_low = (value & 0xFFFFFFFF) as u32;
    let value_high = ((value >> 32) & 0xFFFFFFFF) as u32;

    unsafe {
        core::arch::asm!(
            "wrmsr",
            in("ecx") msr,
            in("eax") value_low,
            in("edx") value_high,
        );
    }
}

/// Initialize TME
pub fn init_tme() -> TmeStatus {
    if !is_tme_supported() {
        let status = TmeStatus {
            enabled: false,
            locked: false,
            key_id: 0,
            encryption_keys: [0; 8],
            mktme_supported: false,
            mktme_enabled: false,
            num_keys: 0,
            algorithm: TmeAlgorithm::Unknown,
        };
        *TME_STATUS.lock() = Some(status.clone());
        return status;
    }

    #[cfg(target_arch = "x86_64")]
    {
        // Read TME capability MSR
        let caps = read_tme_msr(IA32_TME_CAPABILITY);

        // Read TME activate MSR
        let activate = read_tme_msr(IA32_TME_ACTIVATE);

        let enabled = (activate & TME_ACTIVATE_ENABLED) != 0;
        let locked = (activate & TME_ACTIVATE_LOCKED) != 0;

        // Determine encryption algorithm
        let algorithm = if (caps & TME_CAPABILITY_AES_XTS) != 0 {
            TmeAlgorithm::AesXts128 // Simplified - real detection would check for 256-bit
        } else {
            TmeAlgorithm::Unknown
        };

        // Check MKTME support
        let mktme_supported = (caps & TME_CAPABILITY_MKTME) != 0;

        // Determine number of keys (MKTME)
        let num_keys = if mktme_supported {
            // Read number of keys from capability MSR (bits 32-47)
            ((caps >> 32) & 0xFFFF) as u8
        } else {
            1
        };

        // For now, use key ID 0 (default)
        let key_id = 0;

        // Initialize encryption keys array
        let mut encryption_keys = [0u64; 8];
        for i in 0..8 {
            encryption_keys[i] = i as u64;
        }

        let status = TmeStatus {
            enabled,
            locked,
            key_id,
            encryption_keys,
            mktme_supported,
            mktme_enabled: mktme_supported && enabled,
            num_keys,
            algorithm,
        };

        *TME_STATUS.lock() = Some(status.clone());
        TME_INITIALIZED.store(true, Ordering::Release);

        status
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        let status = TmeStatus {
            enabled: false,
            locked: false,
            key_id: 0,
            encryption_keys: [0; 8],
            mktme_supported: false,
            mktme_enabled: false,
            num_keys: 0,
            algorithm: TmeAlgorithm::Unknown,
        };
        *TME_STATUS.lock() = Some(status.clone());
        status
    }
}

/// Get TME status
pub fn get_tme_status() -> Option<TmeStatus> {
    TME_STATUS.lock().clone()
}

/// Check if TME is initialized
pub fn is_tme_initialized() -> bool {
    TME_INITIALIZED.load(Ordering::Acquire)
}

/// Set encryption key ID (for MKTME)
pub fn set_key_id(key_id: u8) -> Result<(), TmeError> {
    if key_id > 7 {
        return Err(TmeError::InvalidKeyId);
    }

    // In a real implementation, this would set the key ID
    // through appropriate MSR or memory management
    Ok(())
}

/// Convert physical address to encrypted address with key ID
pub fn encrypt_with_key_id(pa: u64, key_id: u8) -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        // Encode key ID in bits 52:57 of the address
        // (This is platform-specific - real implementation would check CPUID)
        if key_id == 0 {
            pa
        } else {
            pa | ((key_id as u64) << 52)
        }
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        pa
    }
}

/// Get key ID from encrypted address
pub fn get_key_id_from_address(encrypted_pa: u64) -> u8 {
    #[cfg(target_arch = "x86_64")]
    {
        // Extract key ID from bits 52:57
        ((encrypted_pa >> 52) & 0x3F) as u8
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        0
    }
}

/// Get TME information string
pub fn get_tme_info() -> String {
    if let Some(status) = get_tme_status() {
        if status.enabled {
            let mktme_info = if status.mktme_enabled {
                alloc::format!("/MKTME ({} keys)", status.num_keys)
            } else {
                "".to_string()
            };
            alloc::format!(
                "TME {}{}",
                match status.algorithm {
                    TmeAlgorithm::AesXts128 => "AES-XTS-128",
                    TmeAlgorithm::AesXts256 => "AES-XTS-256",
                    TmeAlgorithm::Unknown => "Unknown",
                },
                mktme_info
            )
        } else {
            "TME not enabled".to_string()
        }
    } else {
        "TME not initialized".to_string()
    }
}

/// TME health metrics
#[derive(Debug, Clone)]
pub struct TmeHealthMetrics {
    /// TME enabled
    pub enabled: bool,
    /// TME locked
    pub locked: bool,
    /// Active key ID
    pub key_id: u8,
    /// MKTME supported
    pub mktme_supported: bool,
    /// MKTME enabled
    pub mktme_enabled: bool,
    /// Number of keys
    pub num_keys: u8,
    /// Encryption algorithm
    pub algorithm: String,
}

/// Get TME health metrics
pub fn get_tme_health_metrics() -> TmeHealthMetrics {
    if let Some(status) = get_tme_status() {
        TmeHealthMetrics {
            enabled: status.enabled,
            locked: status.locked,
            key_id: status.key_id,
            mktme_supported: status.mktme_supported,
            mktme_enabled: status.mktme_enabled,
            num_keys: status.num_keys,
            algorithm: format!("{:?}", status.algorithm),
        }
    } else {
        TmeHealthMetrics {
            enabled: false,
            locked: false,
            key_id: 0,
            mktme_supported: false,
            mktme_enabled: false,
            num_keys: 0,
            algorithm: "Unknown".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tme_algorithm() {
        assert_eq!(TmeAlgorithm::AesXts128, TmeAlgorithm::AesXts128);
        assert_ne!(TmeAlgorithm::AesXts128, TmeAlgorithm::AesXts256);
    }

    #[test]
    fn test_tme_config_default() {
        let config = TmeConfig::default();
        assert!(!config.enable_mktme);
        assert_eq!(config.num_keys, 1);
    }

    #[test]
    fn test_set_key_id() {
        assert!(set_key_id(0).is_ok());
        assert!(set_key_id(7).is_ok());
        assert_eq!(set_key_id(8), Err(TmeError::InvalidKeyId));
    }

    #[test]
    fn test_encrypt_with_key_id() {
        let pa = 0x1000u64;
        let encrypted = encrypt_with_key_id(pa, 0);
        assert_eq!(encrypted, pa);

        let encrypted = encrypt_with_key_id(pa, 1);
        #[cfg(target_arch = "x86_64")]
        assert_ne!(encrypted, pa);
    }
}
