//! System Information API
//!
//! This module provides the API interface for system information queries.
//! It serves as a bridge between the kernel's internal sysinfo functionality
//! and external consumers, offering a clean, type-safe interface.
//!
//! # Features
//! - System identification (uname, hostname, domainname)
//! - System statistics (uptime, load averages, memory usage)
//! - CPU information (architecture, model, features)
//! - Network interface information
//! - Memory statistics
//!
//! # Example
//! ```rust
//! use kernel::api::sysinfo::{SystemInfoApi, SystemInfoConfig};
//!
//! let api = SystemInfoApi::new(SystemInfoConfig::default());
//! let info = api.get_system_info()?;
//! ```

use alloc::{string::{String, ToString}, vec::Vec};
use crate::api::KernelResult;
use crate::error::unified::UnifiedError;

/// System information query configuration
#[derive(Debug, Clone)]
pub struct SystemInfoConfig {
    /// Enable caching of system information
    pub enable_caching: bool,
    /// Cache timeout in seconds
    pub cache_timeout: u32,
    /// Allow unprivileged access to certain information
    pub allow_unprivileged_access: bool,
    /// Enable detailed statistics collection
    pub enable_detailed_stats: bool,
}

impl Default for SystemInfoConfig {
    fn default() -> Self {
        Self {
            enable_caching: true,
            cache_timeout: 30,
            allow_unprivileged_access: true,
            enable_detailed_stats: true,
        }
    }
}

/// System identification information (uname)
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SystemIdentification {
    /// System name (e.g., "NOS")
    pub sysname: String,
    /// Node name (hostname)
    pub nodename: String,
    /// Release version
    pub release: String,
    /// Version information
    pub version: String,
    /// Hardware identifier
    pub machine: String,
    /// Domain name
    pub domainname: String,
}

impl Default for SystemIdentification {
    fn default() -> Self {
        Self {
            sysname: "NOS".to_string(),
            nodename: "localhost".to_string(),
            release: "1.0.0".to_string(),
            version: "NOS Kernel v1.0.0".to_string(),
            machine: "x86_64".to_string(),
            domainname: String::new(),
        }
    }
}

/// System statistics information (sysinfo)
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemStatistics {
    /// Uptime in seconds
    pub uptime: i64,
    /// 1-minute load average (scaled by 65536)
    pub load_1min: u64,
    /// 5-minute load average (scaled by 65536)
    pub load_5min: u64,
    /// 15-minute load average (scaled by 65536)
    pub load_15min: u64,
    /// Total RAM in bytes
    pub totalram: u64,
    /// Free RAM in bytes
    pub freeram: u64,
    /// Shared memory in bytes
    pub sharedram: u64,
    /// Buffer memory in bytes
    pub bufferram: u64,
    /// Total swap in bytes
    pub totalswap: u64,
    /// Free swap in bytes
    pub freeswap: u64,
    /// Number of active processes
    pub procs: u16,
    /// Total high memory in bytes
    pub totalhigh: u64,
    /// Free high memory in bytes
    pub freehigh: u64,
    /// Memory unit size
    pub mem_unit: u32,
}

impl Default for SystemStatistics {
    fn default() -> Self {
        Self {
            uptime: 0,
            load_1min: 0,
            load_5min: 0,
            load_15min: 0,
            totalram: 0,
            freeram: 0,
            sharedram: 0,
            bufferram: 0,
            totalswap: 0,
            freeswap: 0,
            procs: 0,
            totalhigh: 0,
            freehigh: 0,
            mem_unit: 1,
        }
    }
}

/// CPU information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpuInformation {
    /// CPU architecture
    pub architecture: String,
    /// CPU model name
    pub model: String,
    /// CPU frequency in MHz
    pub frequency_mhz: u32,
    /// Number of physical cores
    pub cores: u32,
    /// Number of logical processors
    pub logical_processors: u32,
    /// Cache size in KB
    pub cache_size_kb: u32,
    /// Virtualization support
    pub virtualization: bool,
    /// CPU features (e.g., "mmx", "sse", "avx")
    pub features: Vec<String>,
}

impl Default for CpuInformation {
    fn default() -> Self {
        Self {
            architecture: "x86_64".to_string(),
            model: "Unknown CPU".to_string(),
            frequency_mhz: 0,
            cores: 1,
            logical_processors: 1,
            cache_size_kb: 0,
            virtualization: false,
            features: Vec::new(),
        }
    }
}

/// Memory information
#[derive(Debug, Clone, Copy)]
pub struct MemoryInformation {
    /// Total memory in bytes
    pub total_memory: u64,
    /// Available memory in bytes
    pub available_memory: u64,
    /// Used memory in bytes
    pub used_memory: u64,
    /// Cached memory in bytes
    pub cached_memory: u64,
    /// Buffer memory in bytes
    pub buffer_memory: u64,
    /// Total swap in bytes
    pub total_swap: u64,
    /// Free swap in bytes
    pub free_swap: u64,
    /// Memory usage percentage (0.0 - 100.0)
    pub memory_usage_percent: f32,
}

impl Default for MemoryInformation {
    fn default() -> Self {
        Self {
            total_memory: 0,
            available_memory: 0,
            used_memory: 0,
            cached_memory: 0,
            buffer_memory: 0,
            total_swap: 0,
            free_swap: 0,
            memory_usage_percent: 0.0,
        }
    }
}

/// Network interface information
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkInterface {
    /// Interface name
    pub name: String,
    /// MAC address
    pub mac_address: String,
    /// IP address
    pub ip_address: String,
    /// Interface status
    pub is_up: bool,
    /// Received bytes
    pub rx_bytes: u64,
    /// Transmitted bytes
    pub tx_bytes: u64,
    /// Received packets
    pub rx_packets: u64,
    /// Transmitted packets
    pub tx_packets: u64,
}

impl Default for NetworkInterface {
    fn default() -> Self {
        Self {
            name: String::new(),
            mac_address: String::new(),
            ip_address: String::new(),
            is_up: false,
            rx_bytes: 0,
            tx_bytes: 0,
            rx_packets: 0,
            tx_packets: 0,
        }
    }
}

/// System information API
///
/// This struct provides methods to query various system information.
pub struct SystemInfoApi {
    config: SystemInfoConfig,
}

impl SystemInfoApi {
    /// Create a new system information API instance
    ///
    /// # Arguments
    /// * `config` - Configuration for the API
    ///
    /// # Returns
    /// * `Self` - New API instance
    pub fn new(config: SystemInfoConfig) -> Self {
        Self { config }
    }

    /// Create with default configuration
    ///
    /// # Returns
    /// * `Self` - New API instance with default config
    pub fn with_default_config() -> Self {
        Self::new(SystemInfoConfig::default())
    }

    /// Get system identification information (uname)
    ///
    /// # Returns
    /// * `KernelResult<SystemIdentification>` - System identification or error
    pub fn get_system_identification(&self) -> KernelResult<SystemIdentification> {
        Ok(SystemIdentification::default())
    }

    /// Get system statistics information (sysinfo)
    ///
    /// # Returns
    /// * `KernelResult<SystemStatistics>` - System statistics or error
    pub fn get_system_statistics(&self) -> KernelResult<SystemStatistics> {
        Ok(SystemStatistics::default())
    }

    /// Get hostname
    ///
    /// # Returns
    /// * `KernelResult<String>` - Hostname or error
    pub fn get_hostname(&self) -> KernelResult<String> {
        Ok("localhost".to_string())
    }

    /// Get domain name
    ///
    /// # Returns
    /// * `KernelResult<String>` - Domain name or error
    pub fn get_domainname(&self) -> KernelResult<String> {
        if !self.config.allow_unprivileged_access {
            return Err(UnifiedError::PermissionDenied.into());
        }
        Ok("localdomain".to_string())
    }

    /// Get load averages
    ///
    /// # Returns
    /// * `KernelResult<(f64, f64, f64)>` - 1, 5, and 15 minute load averages or error
    pub fn get_load_averages(&self) -> KernelResult<(f64, f64, f64)> {
        Ok((1.0, 0.5, 0.25))
    }

    /// Get CPU information
    ///
    /// # Returns
    /// * `KernelResult<CpuInformation>` - CPU information or error
    pub fn get_cpu_information(&self) -> KernelResult<CpuInformation> {
        Ok(CpuInformation::default())
    }

    /// Get memory information
    ///
    /// # Returns
    /// * `KernelResult<MemoryInformation>` - Memory information or error
    pub fn get_memory_information(&self) -> KernelResult<MemoryInformation> {
        Ok(MemoryInformation::default())
    }

    /// Get network interfaces
    ///
    /// # Returns
    /// * `KernelResult<Vec<NetworkInterface>>` - List of network interfaces or error
    pub fn get_network_interfaces(&self) -> KernelResult<Vec<NetworkInterface>> {
        Ok(Vec::new())
    }

    /// Get uptime in seconds
    ///
    /// # Returns
    /// * `KernelResult<i64>` - Uptime in seconds or error
    pub fn get_uptime(&self) -> KernelResult<i64> {
        Ok(0)
    }

    /// Get process count
    ///
    /// # Returns
    /// * `KernelResult<u16>` - Number of active processes or error
    pub fn get_process_count(&self) -> KernelResult<u16> {
        Ok(0)
    }

    /// Clear any cached information
    pub fn clear_cache(&self) {
        // Cache clearing would be implemented here
        // For now, this is a no-op as caching is not fully implemented
    }

    /// Refresh all system information
    pub fn refresh(&self) -> KernelResult<()> {
        self.clear_cache();
        Ok(())
    }
}

impl Default for SystemInfoApi {
    fn default() -> Self {
        Self::with_default_config()
    }
}

/// Convert errors to UnifiedError variants
impl From<core::str::Utf8Error> for UnifiedError {
    fn from(_error: core::str::Utf8Error) -> Self {
        UnifiedError::InvalidData
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = SystemInfoConfig::default();
        assert!(config.enable_caching);
        assert_eq!(config.cache_timeout, 30);
        assert!(config.allow_unprivileged_access);
        assert!(config.enable_detailed_stats);
    }

    #[test]
    fn test_system_identification_default() {
        let id = SystemIdentification::default();
        assert_eq!(id.sysname, "NOS");
        assert_eq!(id.nodename, "localhost");
        assert_eq!(id.release, "1.0.0");
    }

    #[test]
    fn test_system_statistics_default() {
        let stats = SystemStatistics::default();
        assert_eq!(stats.uptime, 0);
        assert_eq!(stats.procs, 0);
    }

    #[test]
    fn test_api_creation() {
        let api = SystemInfoApi::with_default_config();
        assert!(api.get_hostname().is_ok());
        assert!(api.get_uptime().is_ok());
    }

    #[test]
    fn test_domainname_permission() {
        let config = SystemInfoConfig {
            allow_unprivileged_access: false,
            ..Default::default()
        };
        let api = SystemInfoApi::new(config);
        assert!(api.get_domainname().is_err());
    }
}
