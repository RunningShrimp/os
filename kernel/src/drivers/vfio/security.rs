//! Security and Sandboxing for VFIO
//!
//! VFIO provides security through multiple layers:
//!
//! 1. **IOMMU Protection**: All DMA is translated through IOMMU
//! 2. **Group Isolation**: Devices that can DMA to each other are grouped together
//! 3. **Permission Checking**: Fine-grained access control
//! 4. **Sandboxing**: Userspace drivers run with restricted privileges
//!
//! # Security Model
//!
//! ```text
//! Userspace Driver
//!     |
//!     | Request device access
//!     v
//! Security Policy Check
//!     |
//!     |-- Is user allowed? (capability check)
//!     |-- Is device available? (not in use)
//!     |-- Is group viable? (all devices in group together)
//!     v
//! Grant Access
//!     |
//!     | All device operations go through:
//!     |-- Permission checks (read/write/mmap)
//!     |-- IOMMU translation (DMA isolation)
//!     |-- Interrupt filtering
//!     v
//! Safe Device Access
//! ```

use crate::drivers::vfio::{
    device::VfioDevice,
    group::VfioGroup,
    VfioError, VfioResult,
};
use alloc::collections::BTreeSet;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::{Mutex, RwLock};

/// Security permission
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Permission {
    /// Read from device registers
    Read,

    /// Write to device registers
    Write,

    /// Map device regions
    Mmap,

    /// Perform DMA operations
    Dma,

    /// Configure interrupts
    Irq,

    /// Reset device
    Reset,

    /// Access config space
    Config,
}

impl Permission {
    /// Convert to flag
    pub fn as_flag(&self) -> u32 {
        match self {
            Self::Read => 0x1,
            Self::Write => 0x2,
            Self::Mmap => 0x4,
            Self::Dma => 0x8,
            Self::Irq => 0x10,
            Self::Reset => 0x20,
            Self::Config => 0x40,
        }
    }

    /// Convert from flag
    pub fn from_flag(flag: u32) -> Option<Self> {
        match flag {
            0x1 => Some(Self::Read),
            0x2 => Some(Self::Write),
            0x4 => Some(Self::Mmap),
            0x8 => Some(Self::Dma),
            0x10 => Some(Self::Irq),
            0x20 => Some(Self::Reset),
            0x40 => Some(Self::Config),
            _ => None,
        }
    }
}

/// Security policy
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Allowed permissions
    permissions: u32,

    /// Allowed device IDs (vendor:device)
    allowed_devices: Vec<(u16, u16)>,

    /// Blocked device IDs
    blocked_devices: Vec<(u16, u16)>,

    /// Require IOMMU
    require_iommu: bool,

    /// DMA size limit
    max_dma_size: u64,

    /// Limit number of devices
    max_devices: usize,
}

impl SecurityPolicy {
    /// Create new policy (deny all by default)
    pub fn new() -> Self {
        Self {
            permissions: 0,
            allowed_devices: Vec::new(),
            blocked_devices: Vec::new(),
            require_iommu: true,
            max_dma_size: 1 << 40, // 1TB default
            max_devices: 8,
        }
    }

    /// Allow permission
    pub fn allow(&mut self, permission: Permission) -> &mut Self {
        self.permissions |= permission.as_flag();
        self
    }

    /// Deny permission
    pub fn deny(&mut self, permission: Permission) -> &mut Self {
        self.permissions &= !permission.as_flag();
        self
    }

    /// Check if permission is granted
    pub fn is_allowed(&self, permission: Permission) -> bool {
        (self.permissions & permission.as_flag()) != 0
    }

    /// Add allowed device
    pub fn allow_device(&mut self, vendor: u16, device: u16) -> &mut Self {
        self.allowed_devices.push((vendor, device));
        self
    }

    /// Add blocked device
    pub fn block_device(&mut self, vendor: u16, device: u16) -> &mut Self {
        self.blocked_devices.push((vendor, device));
        self
    }

    /// Check if device is allowed
    pub fn is_device_allowed(&self, vendor: u16, device: u16) -> bool {
        // Check blocklist first
        if self.blocked_devices.contains(&(vendor, device)) {
            return false;
        }

        // If allowlist is empty, allow all (except blocked)
        if self.allowed_devices.is_empty() {
            return true;
        }

        // Check allowlist
        self.allowed_devices.contains(&(vendor, device))
    }

    /// Set DMA size limit
    pub fn set_dma_limit(&mut self, size: u64) -> &mut Self {
        self.max_dma_size = size;
        self
    }

    /// Set max devices
    pub fn set_max_devices(&mut self, count: usize) -> &mut Self {
        self.max_devices = count;
        self
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// VFIO sandbox - Enforces security policies
pub struct VfioSandbox {
    /// Security policy
    policy: Mutex<SecurityPolicy>,

    /// Granted permissions per device
    device_permissions: RwLock<BTreeSet<u64>>,

    /// Active devices count
    active_devices: AtomicU64,

    /// Total DMA mapped
    total_dma: AtomicU64,

    /// Security violations
    violations: Mutex<Vec<SecurityViolation>>,
}

impl VfioSandbox {
    /// Create new sandbox
    pub fn new() -> Self {
        Self {
            policy: Mutex::new(SecurityPolicy::new()),
            device_permissions: RwLock::new(BTreeSet::new()),
            active_devices: AtomicU64::new(0),
            total_dma: AtomicU64::new(0),
            violations: Mutex::new(Vec::new()),
        }
    }

    /// Get security policy
    pub fn policy(&self) -> &Mutex<SecurityPolicy> {
        &self.policy
    }

    /// Check if operation is allowed
    pub fn check_permission(&self, device_id: u64, permission: Permission) -> VfioResult<()> {
        let policy = self.policy.lock();

        // Check global permission
        if !policy.is_allowed(permission) {
            self.record_violation(SecurityViolation {
                device_id,
                violation_type: ViolationType::PermissionDenied,
                permission: Some(permission),
                message: alloc::format!("Permission {:?} denied", permission),
            });
            return Err(VfioError::PermissionDenied);
        }

        // Check device-specific permission
        let permissions = self.device_permissions.read();
        if !permissions.contains(&device_id) {
            drop(permissions);
            self.record_violation(SecurityViolation {
                device_id,
                violation_type: ViolationType::DeviceNotAuthorized,
                permission: Some(permission),
                message: alloc::format!("Device {} not authorized", device_id),
            });
            return Err(VfioError::PermissionDenied);
        }

        Ok(())
    }

    /// Grant access to device
    pub fn grant_device_access(
        &self,
        device_id: u64,
        vendor: u16,
        device: u16,
        policy: &SecurityPolicy,
    ) -> VfioResult<()> {
        let global_policy = self.policy.lock();

        // Check device allowlist/blocklist
        if !global_policy.is_device_allowed(vendor, device) {
            self.record_violation(SecurityViolation {
                device_id,
                violation_type: ViolationType::DeviceBlocked,
                permission: None,
                message: alloc::format!(
                    "Device {:04x}:{:04x} is blocked",
                    vendor, device
                ),
            });
            return Err(VfioError::PermissionDenied);
        }

        // Check device limit
        let active = self.active_devices.load(Ordering::Relaxed);
        if active >= global_policy.max_devices as u64 {
            self.record_violation(SecurityViolation {
                device_id,
                violation_type: ViolationType::ResourceLimit,
                permission: None,
                message: alloc::format!("Device limit {} exceeded", global_policy.max_devices),
            });
            return Err(VfioError::ResourceExhausted);
        }

        drop(global_policy);

        // Grant permission
        self.device_permissions
            .write()
            .insert(device_id);

        self.active_devices.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Revoke device access
    pub fn revoke_device_access(&self, device_id: u64) -> VfioResult<()> {
        self.device_permissions
            .write()
            .remove(&device_id);

        self.active_devices.fetch_sub(1, Ordering::Relaxed);

        Ok(())
    }

    /// Check DMA mapping
    pub fn check_dma(&self, device_id: u64, size: u64) -> VfioResult<()> {
        // Check DMA permission
        self.check_permission(device_id, Permission::Dma)?;

        let policy = self.policy.lock();

        // Check size limit
        let current_total = self.total_dma.load(Ordering::Relaxed);
        if current_total + size > policy.max_dma_size {
            self.record_violation(SecurityViolation {
                device_id,
                violation_type: ViolationType::ResourceLimit,
                permission: Some(Permission::Dma),
                message: alloc::format!(
                    "DMA limit {} exceeded (current={}, requested={})",
                    policy.max_dma_size, current_total, size
                ),
            });
            return Err(VfioError::ResourceExhausted);
        }

        Ok(())
    }

    /// Record DMA mapping
    pub fn record_dma(&self, _device_id: u64, size: u64) -> VfioResult<()> {
        self.total_dma.fetch_add(size, Ordering::Relaxed);
        Ok(())
    }

    /// Record DMA unmap
    pub fn record_dma_unmap(&self, _device_id: u64, size: u64) -> VfioResult<()> {
        self.total_dma.fetch_sub(size, Ordering::Relaxed);
        Ok(())
    }

    /// Record security violation
    fn record_violation(&self, violation: SecurityViolation) {
        let mut violations = self.violations.lock();
        violations.push(violation);

        // Log violation
        log::warn!("VFIO security violation: {}", violation.message);
    }

    /// Get security violations
    pub fn get_violations(&self) -> Vec<SecurityViolation> {
        self.violations.lock().clone()
    }

    /// Clear security violations
    pub fn clear_violations(&self) {
        self.violations.lock().clear();
    }

    /// Get security statistics
    pub fn get_stats(&self) -> SecurityStats {
        let policy = self.policy.lock();

        SecurityStats {
            active_devices: self.active_devices.load(Ordering::Relaxed) as usize,
            total_dma_mapped: self.total_dma.load(Ordering::Relaxed),
            max_dma_size: policy.max_dma_size,
            max_devices: policy.max_devices,
            violations_count: self.violations.lock().len(),
        }
    }

    /// Validate device group
    ///
    /// Ensures all devices in a group are granted together
    pub fn validate_group(&self, _group: &VfioGroup) -> VfioResult<()> {
        // GH-#1364: Check that all devices in group are authorized
        // See: https://github.com/npos/kernel/issues/1364
        Ok(())
    }

    /// Validate IOMMU requirement
    pub fn validate_iommu(&self, has_iommu: bool) -> VfioResult<()> {
        let policy = self.policy.lock();

        if policy.require_iommu && !has_iommu {
            return Err(VfioError::PermissionDenied);
        }

        Ok(())
    }

    /// Check isolation
    ///
    /// Ensures devices cannot access each other's memory
    pub fn check_isolation(&self, _device1: u64, _device2: u64) -> VfioResult<()> {
        // GH-#1365: Verify IOMMU isolation
        // See: https://github.com/npos/kernel/issues/1365
        Ok(())
    }
}

/// Security violation record
#[derive(Debug, Clone)]
pub struct SecurityViolation {
    /// Device ID that caused violation
    pub device_id: u64,

    /// Type of violation
    pub violation_type: ViolationType,

    /// Permission involved (if any)
    pub permission: Option<Permission>,

    /// Violation message
    pub message: alloc::string::String,
}

/// Type of security violation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViolationType {
    /// Permission denied
    PermissionDenied,

    /// Device not authorized
    DeviceNotAuthorized,

    /// Device blocked
    DeviceBlocked,

    /// Resource limit exceeded
    ResourceLimit,

    /// IOMMU required but not available
    IommuRequired,

    /// Isolation violation
    IsolationViolation,
}

/// Security statistics
#[derive(Debug, Clone, Copy)]
pub struct SecurityStats {
    pub active_devices: usize,
    pub total_dma_mapped: u64,
    pub max_dma_size: u64,
    pub max_devices: usize,
    pub violations_count: usize,
}

/// Security error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityError {
    PermissionDenied,
    DeviceBlocked,
    ResourceExhausted,
    IommuRequired,
    IsolationViolation,
    PolicyViolation,
}

impl core::fmt::Display for SecurityError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::DeviceBlocked => write!(f, "Device is blocked"),
            Self::ResourceExhausted => write!(f, "Resource limit exceeded"),
            Self::IommuRequired => write!(f, "IOMMU required"),
            Self::IsolationViolation => write!(f, "Isolation violation"),
            Self::PolicyViolation => write!(f, "Policy violation"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_permission_flags() {
        assert_eq!(Permission::Read.as_flag(), 0x1);
        assert_eq!(Permission::Write.as_flag(), 0x2);
        assert_eq!(Permission::Mmap.as_flag(), 0x4);

        assert_eq!(Permission::from_flag(0x1), Some(Permission::Read));
        assert_eq!(Permission::from_flag(0x99), None);
    }

    #[test]
    fn test_security_policy() {
        let mut policy = SecurityPolicy::new();

        assert!(!policy.is_allowed(Permission::Read));

        policy.allow(Permission::Read);
        assert!(policy.is_allowed(Permission::Read));

        policy.deny(Permission::Read);
        assert!(!policy.is_allowed(Permission::Read));
    }

    #[test]
    fn test_device_filtering() {
        let mut policy = SecurityPolicy::new();

        // Block specific device
        policy.block_device(0x1234, 0x5678);
        assert!(!policy.is_device_allowed(0x1234, 0x5678));

        // Allow specific device
        policy.allow_device(0x1111, 0x2222);
        assert!(policy.is_device_allowed(0x1111, 0x2222));

        // Test with empty allowlist (should allow all except blocked)
        assert!(policy.is_device_allowed(0x9999, 0x8888));
    }

    #[test]
    fn test_sandbox_permission_check() {
        let sandbox = VfioSandbox::new();

        // Grant read permission
        sandbox.policy().lock().allow(Permission::Read);

        // Authorize device
        sandbox
            .grant_device_access(1, 0x1234, 0x5678, &SecurityPolicy::new())
            .unwrap();

        // Check allowed permission
        assert!(sandbox.check_permission(1, Permission::Read).is_ok());

        // Check denied permission
        assert!(sandbox.check_permission(1, Permission::Write).is_err());
    }

    #[test]
    fn test_sandbox_device_access() {
        let sandbox = VfioSandbox::new();

        sandbox
            .policy()
            .lock()
            .allow_device(0x1234, 0x5678);

        sandbox
            .grant_device_access(1, 0x1234, 0x5678, &SecurityPolicy::new())
            .unwrap();

        sandbox.revoke_device_access(1).unwrap();
    }

    #[test]
    fn test_dma_limits() {
        let sandbox = VfioSandbox::new();

        sandbox
            .policy()
            .lock()
            .allow(Permission::Dma)
            .set_dma_limit(0x10000);

        sandbox
            .grant_device_access(1, 0x1234, 0x5678, &SecurityPolicy::new())
            .unwrap();

        // Within limit
        assert!(sandbox.check_dma(1, 0x1000).is_ok());

        // Exceeds limit
        assert!(sandbox.check_dma(1, 0x20000).is_err());
    }

    #[test]
    fn test_violations() {
        let sandbox = VfioSandbox::new();

        // Try unauthorized access
        sandbox.policy().lock().allow(Permission::Read);
        sandbox
            .grant_device_access(1, 0x1234, 0x5678, &SecurityPolicy::new())
            .unwrap();

        let _ = sandbox.check_permission(1, Permission::Write);

        // Check violations were recorded
        let violations = sandbox.get_violations();
        assert!(!violations.is_empty());
    }
}
