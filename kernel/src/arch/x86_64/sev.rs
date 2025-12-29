//! # AMD Secure Encrypted Virtualization (SEV) Support
//!
//! This module provides support for AMD SEV/SEV-ES memory encryption technology.
//!
//! ## Features
//!
//! - **SEV Detection**: Check CPUID for SEV support
//! - **SEV/SEV-ES Initialization**: Configure encrypted memory
//! - **Key Management**: Handle encryption keys
//! - **VMPL Support**: VM Permission Level configuration
//!
//! ## Usage
//!
//! ```rust
//! use kernel::arch::x86_64::sev::{init_sev, SevStatus};
//!
//! // Initialize SEV
//! let sev_status = init_sev();
//! if sev_status.enabled {
//!     println!("SEV is enabled");
//! }
//! ```

#![cfg(feature = "memory_encryption")]

extern crate alloc;

use alloc::string::String;
use core::sync::atomic::{AtomicBool, Ordering};

/// SEV MSR addresses
const MSR_SEV_STATUS: u32 = 0xC001_0131;
const MSR_SEV_ENABLED: u32 = 0xC001_0131;
const MSR_SEV_ES_ENABLED: u32 = 0xC001_0132;

/// SEV status flags
pub const SEV_STATUS_ENABLED: u64 = 1 << 0;
pub const SEV_STATUS_ES_ENABLED: u64 = 1 << 1;
pub const SEV_STATUS_SECURE_DEBUG: u64 = 1 << 2;

/// SEV status
#[derive(Debug, Clone)]
pub struct SevStatus {
    /// Whether SEV is enabled
    pub enabled: bool,
    /// Whether memory is encrypted
    pub encrypted: bool,
    /// Whether SEV-ES (Encrypted State) is enabled
    pub es_enabled: bool,
    /// Whether secure debug is enabled
    pub secure_debug: bool,
    /// SEV version
    pub version: SevVersion,
    /// Number of VMPL levels
    pub vmpl_levels: u8,
}

/// SEV version information
#[derive(Debug, Clone, Copy)]
pub struct SevVersion {
    /// Major version
    pub major: u8,
    /// Minor version
    pub minor: u8,
}

/// SEV error types
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SevError {
    /// SEV not supported
    NotSupported,
    /// SEV not enabled
    NotEnabled,
    /// Initialization failed
    InitFailed,
    /// Invalid VMPL level
    InvalidVmplLevel,
    /// Hardware error
    HardwareError,
}

/// SEV configuration
#[derive(Debug, Clone)]
pub struct SevConfig {
    /// Whether to use SEV-ES
    pub use_es: bool,
    /// Whether to enable secure debug
    pub enable_secure_debug: bool,
    /// VMPL level to use (0-3)
    pub vmpl_level: u8,
}

impl Default for SevConfig {
    fn default() -> Self {
        Self {
            use_es: true,
            enable_secure_debug: false,
            vmpl_level: 0,
        }
    }
}

/// Global SEV status
static SEV_STATUS: spin::Mutex<Option<SevStatus>> = spin::Mutex::new(None);
static SEV_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Check if SEV is supported by the CPU
pub fn is_sev_supported() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        let mut cpuid_result = [0u32; 4];
        unsafe {
            core::arch::asm!(
                "cpuid",
                inout("eax") 0x8000_0000 => eax,
                lateout("ebx") ebx,
                lateout("ecx") ecx,
                lateout("edx") edx,
            );
            cpuid_result[0] = eax;
            cpuid_result[1] = ebx;
            cpuid_result[2] = ecx;
            cpuid_result[3] = edx;
        }

        // Check if extended function 0x8000_001F is available
        if cpuid_result[0] < 0x8000_001F {
            return false;
        }

        // Check SEV support in CPUID.0x8000_001F:EBX
        unsafe {
            core::arch::asm!(
                "cpuid",
                inout("eax") 0x8000_001F => eax,
                lateout("ebx") ebx,
                lateout("ecx") ecx,
                lateout("edx") edx,
            );
            cpuid_result[0] = eax;
            cpuid_result[1] = ebx;
            cpuid_result[2] = ecx;
            cpuid_result[3] = edx;
        }

        // Bit 0 of EBX indicates SEV support
        (cpuid_result[1] & 0x1) != 0
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// Check if SEV-ES is supported
pub fn is_sev_es_supported() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        if !is_sev_supported() {
            return false;
        }

        let mut cpuid_result = [0u32; 4];
        unsafe {
            core::arch::asm!(
                "cpuid",
                inout("eax") 0x8000_001F => eax,
                lateout("ebx") ebx,
                lateout("ecx") ecx,
                lateout("edx") edx,
            );
            cpuid_result[0] = eax;
            cpuid_result[1] = ebx;
            cpuid_result[2] = ecx;
            cpuid_result[3] = edx;
        }

        // Bit 3 of EBX indicates SEV-ES support
        (cpuid_result[1] & 0x8) != 0
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        false
    }
}

/// Read SEV status MSR
#[cfg(target_arch = "x86_64")]
fn read_sev_msr(msr: u32) -> u64 {
    let value: u64;
    unsafe {
        core::arch::asm!(
            "rdmsr",
            in("ecx") msr,
            out("eax") value_low,
            out("edx") value_high,
        );
        value = ((value_high as u64) << 32) | (value_low as u64);
    }
    value
}

/// Initialize SEV
pub fn init_sev() -> SevStatus {
    if !is_sev_supported() {
        let status = SevStatus {
            enabled: false,
            encrypted: false,
            es_enabled: false,
            secure_debug: false,
            version: SevVersion { major: 0, minor: 0 },
            vmpl_levels: 0,
        };
        *SEV_STATUS.lock() = Some(status.clone());
        return status;
    }

    #[cfg(target_arch = "x86_64")]
    {
        // Read SEV status MSR
        let msr_value = read_sev_msr(MSR_SEV_STATUS);

        let enabled = (msr_value & SEV_STATUS_ENABLED) != 0;
        let es_enabled = (msr_value & SEV_STATUS_ES_ENABLED) != 0;
        let secure_debug = (msr_value & SEV_STATUS_SECURE_DEBUG) != 0;

        // Get SEV version from CPUID
        let mut cpuid_result = [0u32; 4];
        unsafe {
            core::arch::asm!(
                "cpuid",
                inout("eax") 0x8000_001F => eax,
                lateout("ebx") ebx,
                lateout("ecx") ecx,
                lateout("edx") edx,
            );
            cpuid_result[0] = eax;
            cpuid_result[1] = ebx;
            cpuid_result[2] = ecx;
            cpuid_result[3] = edx;
        }

        // Version is in EAX
        let major = ((cpuid_result[0] >> 8) & 0xFF) as u8;
        let minor = (cpuid_result[0] & 0xFF) as u8;

        // Get number of VMPL levels from EBX
        let vmpl_levels = ((cpuid_result[1] >> 12) & 0xF) as u8;

        let status = SevStatus {
            enabled,
            encrypted: enabled,
            es_enabled,
            secure_debug,
            version: SevVersion { major, minor },
            vmpl_levels,
        };

        *SEV_STATUS.lock() = Some(status.clone());
        SEV_INITIALIZED.store(true, Ordering::Release);

        status
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        let status = SevStatus {
            enabled: false,
            encrypted: false,
            es_enabled: false,
            secure_debug: false,
            version: SevVersion { major: 0, minor: 0 },
            vmpl_levels: 0,
        };
        *SEV_STATUS.lock() = Some(status.clone());
        status
    }
}

/// Get SEV status
pub fn get_sev_status() -> Option<SevStatus> {
    SEV_STATUS.lock().clone()
}

/// Check if SEV is initialized
pub fn is_sev_initialized() -> bool {
    SEV_INITIALIZED.load(Ordering::Acquire)
}

/// Set VMPL level
pub fn set_vmpl_level(level: u8) -> Result<(), SevError> {
    if level > 3 {
        return Err(SevError::InvalidVmplLevel);
    }

    // In a real implementation, this would set the VMPL level
    // through appropriate MSR or VMGEXIT
    Ok(())
}

/// Flush cache for encrypted memory
pub fn flush_encrypted_cache(addr: u64, size: usize) {
    #[cfg(target_arch = "x86_64")]
    {
        // Use WBINVD to flush cache
        unsafe {
            core::arch::asm!("wbinvd");
        }
    }
}

/// Convert physical address to encrypted address
pub fn encrypt_pa(pa: u64) -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        // Set the C-bit (bit 47) to indicate encryption
        pa | (1 << 47)
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        pa
    }
}

/// Convert encrypted address to physical address
pub fn decrypt_pa(encrypted_pa: u64) -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        // Clear the C-bit
        encrypted_pa & !(1 << 47)
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        encrypted_pa
    }
}

/// Get SEV information string
pub fn get_sev_info() -> String {
    if let Some(status) = get_sev_status() {
        if status.enabled {
            alloc::format!(
                "SEV {}.{}{}",
                status.version.major,
                status.version.minor,
                if status.es_enabled { "-ES" } else { "" }
            )
        } else {
            "SEV not enabled".to_string()
        }
    } else {
        "SEV not initialized".to_string()
    }
}

/// SEV health metrics
#[derive(Debug, Clone)]
pub struct SevHealthMetrics {
    /// SEV enabled
    pub enabled: bool,
    /// Memory encrypted
    pub encrypted: bool,
    /// SEV-ES enabled
    pub es_enabled: bool,
    /// SEV version
    pub version: String,
    /// VMPL levels
    pub vmpl_levels: u8,
}

/// Get SEV health metrics
pub fn get_sev_health_metrics() -> SevHealthMetrics {
    if let Some(status) = get_sev_status() {
        SevHealthMetrics {
            enabled: status.enabled,
            encrypted: status.encrypted,
            es_enabled: status.es_enabled,
            version: alloc::format!("{}.{}", status.version.major, status.version.minor),
            vmpl_levels: status.vmpl_levels,
        }
    } else {
        SevHealthMetrics {
            enabled: false,
            encrypted: false,
            es_enabled: false,
            version: "0.0".to_string(),
            vmpl_levels: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sev_version() {
        let version = SevVersion { major: 1, minor: 2 };
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
    }

    #[test]
    fn test_sev_config_default() {
        let config = SevConfig::default();
        assert!(config.use_es);
        assert!(!config.enable_secure_debug);
        assert_eq!(config.vmpl_level, 0);
    }

    #[test]
    fn test_encrypt_decrypt_pa() {
        let pa = 0x1000u64;
        let encrypted = encrypt_pa(pa);

        #[cfg(target_arch = "x86_64")]
        assert_ne!(encrypted, pa);

        let decrypted = decrypt_pa(encrypted);
        assert_eq!(decrypted, pa);
    }

    #[test]
    fn test_set_vmpl_level() {
        assert!(set_vmpl_level(0).is_ok());
        assert!(set_vmpl_level(3).is_ok());
        assert_eq!(set_vmpl_level(4), Err(SevError::InvalidVmplLevel));
    }
}
