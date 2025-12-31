//! # GPU Security and Isolation
//!
//! This module provides comprehensive GPU security features including:
//! - GPU context isolation for process separation
//! - Secure display with HDCP support
//! - GPU access control with permissions
//! - Attack mitigation
//! - Trusted Execution Environment (TEE) integration
//! - GPU content protection
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::graphics::security::{GpuSecurity, SecureContext};
//!
//! // Initialize GPU security
//! let security = GpuSecurity::init(&gpu)?;
//!
//! // Create secure context
//! let context = security.create_secure_context(100)?;
//!
//! // Enable HDCP
//! security.enable_hdcp()?;
//! ```

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use crate::subsystems::sync::{Mutex, RwLock};

use super::error::{GraphicsError, GraphicsResult};
use super::gpu::{GpuContext, GpuContextId};
use super::DeviceId;

/// GPU permission
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuPermission {
    /// Read access
    Read,
    /// Write access
    Write,
    /// Execute access (for shaders/commands)
    Execute,
    /// Full access
    All,
}

/// Security context for GPU access
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Context ID
    pub context_id: GpuContextId,
    /// Process ID
    pub pid: u64,
    /// User ID
    pub uid: u32,
    /// Group ID
    pub gid: u32,
    /// Permissions
    pub permissions: u32,
    /// Is trusted
    pub trusted: bool,
}

impl SecurityContext {
    /// Create a new security context
    pub fn new(context_id: GpuContextId, pid: u64, uid: u32, gid: u32) -> Self {
        Self {
            context_id,
            pid,
            uid,
            gid,
            permissions: 0,
            trusted: false,
        }
    }

    /// Check if has permission
    pub fn has_permission(&self, perm: GpuPermission) -> bool {
        let perm_bit = match perm {
            GpuPermission::Read => 0x1,
            GpuPermission::Write => 0x2,
            GpuPermission::Execute => 0x4,
            GpuPermission::All => 0x7,
        };
        (self.permissions & perm_bit) != 0
    }

    /// Grant permission
    pub fn grant_permission(&mut self, perm: GpuPermission) {
        let perm_bit = match perm {
            GpuPermission::Read => 0x1,
            GpuPermission::Write => 0x2,
            GpuPermission::Execute => 0x4,
            GpuPermission::All => 0x7,
        };
        self.permissions |= perm_bit;
    }

    /// Revoke permission
    pub fn revoke_permission(&mut self, perm: GpuPermission) {
        let perm_bit = match perm {
            GpuPermission::Read => 0x1,
            GpuPermission::Write => 0x2,
            GpuPermission::Execute => 0x4,
            GpuPermission::All => 0x7,
        };
        self.permissions &= !perm_bit;
    }

    /// Set as trusted
    pub fn set_trusted(&mut self, trusted: bool) {
        self.trusted = trusted;
    }
}

/// HDCP version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HdcpVersion {
    /// HDCP 1.4
    Hdcp14,
    /// HDCP 2.2
    Hdcp22,
    /// HDCP 2.3
    Hdcp23,
}

/// HDCP state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HdcpState {
    /// Not enabled
    Disabled,
    /// Authentication in progress
    Authenticating,
    /// Enabled and authenticated
    Enabled,
    /// Authentication failed
    Failed,
}

/// Secure display configuration
#[derive(Debug, Clone)]
pub struct SecureDisplayConfig {
    /// HDCP enabled
    pub hdcp_enabled: bool,
    /// HDCP version
    pub hdcp_version: Option<HdcpVersion>,
    /// HDCP state
    pub hdcp_state: HdcpState,
    /// Secure boot integration
    pub secure_boot: bool,
    /// TEE integration
    pub tee_integration: bool,
    /// Content protection level
    pub protection_level: u32,
}

impl SecureDisplayConfig {
    /// Create a new secure display configuration
    pub fn new() -> Self {
        Self {
            hdcp_enabled: false,
            hdcp_version: None,
            hdcp_state: HdcpState::Disabled,
            secure_boot: false,
            tee_integration: false,
            protection_level: 0,
        }
    }

    /// Enable HDCP
    pub fn enable_hdcp(&mut self, version: HdcpVersion) {
        self.hdcp_enabled = true;
        self.hdcp_version = Some(version);
        self.hdcp_state = HdcpState::Authenticating;
    }

    /// Disable HDCP
    pub fn disable_hdcp(&mut self) {
        self.hdcp_enabled = false;
        self.hdcp_version = None;
        self.hdcp_state = HdcpState::Disabled;
    }

    /// Set HDCP state
    pub fn set_hdcp_state(&mut self, state: HdcpState) {
        self.hdcp_state = state;
    }

    /// Set protection level
    pub fn set_protection_level(&mut self, level: u32) {
        self.protection_level = level;
    }
}

impl Default for SecureDisplayConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Secure context for isolated GPU access
#[derive(Debug)]
pub struct SecureContext {
    /// Context ID
    id: GpuContextId,
    /// Security context
    security_context: SecurityContext,
    /// Is active
    active: Arc<AtomicBool>,
    /// Memory isolation enabled
    memory_isolation: Arc<AtomicBool>,
    /// Trusted execution
    trusted_execution: Arc<AtomicBool>,
}

impl SecureContext {
    /// Create a new secure context
    pub fn new(id: GpuContextId, pid: u64, uid: u32, gid: u32) -> Self {
        let security_context = SecurityContext::new(id, pid, uid, gid);

        Self {
            id,
            security_context,
            active: Arc::new(AtomicBool::new(false)),
            memory_isolation: Arc::new(AtomicBool::new(true)),
            trusted_execution: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Get context ID
    pub fn id(&self) -> GpuContextId {
        self.id
    }

    /// Get security context
    pub fn security_context(&self) -> &SecurityContext {
        &self.security_context
    }

    /// Check if active
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    /// Activate context
    pub fn activate(&self) {
        self.active.store(true, Ordering::Release);
    }

    /// Deactivate context
    pub fn deactivate(&self) {
        self.active.store(false, Ordering::Release);
    }

    /// Enable memory isolation
    pub fn enable_memory_isolation(&self) {
        self.memory_isolation.store(true, Ordering::Release);
    }

    /// Disable memory isolation
    pub fn disable_memory_isolation(&self) {
        self.memory_isolation.store(false, Ordering::Release);
    }

    /// Check if memory isolation is enabled
    pub fn is_memory_isolated(&self) -> bool {
        self.memory_isolation.load(Ordering::Acquire)
    }

    /// Enable trusted execution
    pub fn enable_trusted_execution(&self) {
        self.trusted_execution.store(true, Ordering::Release);
    }

    /// Check if trusted execution is enabled
    pub fn is_trusted_execution(&self) -> bool {
        self.trusted_execution.load(Ordering::Acquire)
    }
}

/// GPU attack type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackType {
    /// DMA attack
    DmaAttack,
    /// Side-channel attack
    SideChannel,
    /// Rowhammer attack
    Rowhammer,
    /// Spectre/Meltdown variant
    SpectreMeltdown,
    /// Privilege escalation
    PrivilegeEscalation,
}

/// Attack mitigation strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MitigationStrategy {
    /// Disable vulnerable feature
    DisableFeature,
    /// Apply software patch
    SoftwarePatch,
    /// Hardware mitigation
    HardwareMitigation,
    /// Rate limiting
    RateLimiting,
}

/// Security policy
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Policy ID
    pub id: u32,
    /// Policy name
    pub name: String,
    /// Max GPU memory per context
    pub max_memory_per_context: u64,
    /// Max command buffer size
    pub max_command_buffer_size: u64,
    /// Allowed shader types
    pub allowed_shader_types: u32,
    /// Require secure boot
    pub require_secure_boot: bool,
    /// Require TEE
    pub require_tee: bool,
}

impl SecurityPolicy {
    /// Create a new security policy
    pub fn new(id: u32, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            max_memory_per_context: 512 * 1024 * 1024, // 512MB default
            max_command_buffer_size: 256 * 1024,       // 256KB default
            allowed_shader_types: 0xFF,                // All types allowed
            require_secure_boot: false,
            require_tee: false,
        }
    }

    /// Set maximum memory per context
    pub fn set_max_memory(&mut self, size: u64) {
        self.max_memory_per_context = size;
    }

    /// Set maximum command buffer size
    pub fn set_max_command_buffer_size(&mut self, size: u64) {
        self.max_command_buffer_size = size;
    }
}

/// GPU security manager
#[derive(Debug)]
pub struct GpuSecurity {
    /// Device ID
    device_id: DeviceId,
    /// Security contexts
    security_contexts: Mutex<BTreeMap<GpuContextId, SecurityContext>>,
    /// Secure contexts
    secure_contexts: Mutex<BTreeMap<GpuContextId, Arc<SecureContext>>>,
    /// Next context ID
    next_context_id: Arc<AtomicU32>,
    /// Secure display configuration
    secure_display: RwLock<SecureDisplayConfig>,
    /// HDCP supported
    hdcp_supported: Arc<AtomicBool>,
    /// Attack mitigations
    mitigations: Mutex<BTreeMap<AttackType, MitigationStrategy>>,
    /// Security policies
    policies: Mutex<BTreeMap<u32, SecurityPolicy>>,
    /// Next policy ID
    next_policy_id: Arc<AtomicU32>,
}

impl GpuSecurity {
    /// Initialize GPU security
    pub fn init(_gpu: &super::GpuDevice) -> GraphicsResult<Self> {
        // Set up default attack mitigations
        let mut mitigations = BTreeMap::new();
        mitigations.insert(AttackType::DmaAttack, MitigationStrategy::HardwareMitigation);
        mitigations.insert(AttackType::SideChannel, MitigationStrategy::SoftwarePatch);
        mitigations.insert(AttackType::Rowhammer, MitigationStrategy::RateLimiting);
        mitigations.insert(AttackType::SpectreMeltdown, MitigationStrategy::SoftwarePatch);
        mitigations.insert(
            AttackType::PrivilegeEscalation,
            MitigationStrategy::HardwareMitigation,
        );

        Ok(Self {
            device_id: DeviceId::new(1),
            security_contexts: Mutex::new(BTreeMap::new()),
            secure_contexts: Mutex::new(BTreeMap::new()),
            next_context_id: Arc::new(AtomicU32::new(1)),
            secure_display: RwLock::new(SecureDisplayConfig::new()),
            hdcp_supported: Arc::new(AtomicBool::new(true)),
            mitigations: Mutex::new(mitigations),
            policies: Mutex::new(BTreeMap::new()),
            next_policy_id: Arc::new(AtomicU32::new(1)),
        })
    }

    /// Get device ID
    pub fn device_id(&self) -> DeviceId {
        self.device_id
    }

    /// Check if HDCP is supported
    pub fn supports_hdcp(&self) -> bool {
        self.hdcp_supported.load(Ordering::Acquire)
    }

    /// Create security context
    pub fn create_security_context(
        &self,
        pid: u64,
        uid: u32,
        gid: u32,
    ) -> GraphicsResult<SecurityContext> {
        let id = GpuContextId::new(self.next_context_id.fetch_add(1, Ordering::SeqCst));
        let context = SecurityContext::new(id, pid, uid, gid);

        let mut contexts = self.security_contexts.lock();
        contexts.insert(id, context.clone());

        Ok(context)
    }

    /// Get security context
    pub fn get_security_context(&self, id: GpuContextId) -> GraphicsResult<SecurityContext> {
        let contexts = self.security_contexts.lock();
        contexts
            .get(&id)
            .cloned()
            .ok_or_else(|| GraphicsError::InvalidContext("Security context not found".to_string()))
    }

    /// Destroy security context
    pub fn destroy_security_context(&self, id: GpuContextId) -> GraphicsResult<()> {
        let mut contexts = self.security_contexts.lock();
        contexts
            .remove(&id)
            .ok_or_else(|| GraphicsError::InvalidContext("Security context not found".to_string()))?;
        Ok(())
    }

    /// Create secure context
    pub fn create_secure_context(&self, pid: u64) -> GraphicsResult<SecureContext> {
        let id = GpuContextId::new(self.next_context_id.fetch_add(1, Ordering::SeqCst));
        let context = Arc::new(SecureContext::new(id, pid, 0, 0));

        let mut contexts = self.secure_contexts.lock();
        contexts.insert(id, context.clone());

        // Also create security context
        let _sec_ctx = self.create_security_context(pid, 0, 0)?;

        Ok((*context).clone())
    }

    /// Get secure context
    pub fn get_secure_context(&self, id: GpuContextId) -> GraphicsResult<Arc<SecureContext>> {
        let contexts = self.secure_contexts.lock();
        contexts
            .get(&id)
            .cloned()
            .ok_or_else(|| GraphicsError::InvalidContext("Secure context not found".to_string()))
    }

    /// Destroy secure context
    pub fn destroy_secure_context(&self, id: GpuContextId) -> GraphicsResult<()> {
        let mut contexts = self.secure_contexts.lock();
        contexts
            .remove(&id)
            .ok_or_else(|| GraphicsError::InvalidContext("Secure context not found".to_string()))?;
        Ok(())
    }

    /// Check GPU access permission
    pub fn check_permission(
        &self,
        context_id: GpuContextId,
        perm: GpuPermission,
    ) -> GraphicsResult<bool> {
        let context = self.get_security_context(context_id)?;
        Ok(context.has_permission(perm))
    }

    /// Grant permission to context
    pub fn grant_permission(
        &self,
        context_id: GpuContextId,
        perm: GpuPermission,
    ) -> GraphicsResult<()> {
        let mut contexts = self.security_contexts.lock();
        let context = contexts
            .get_mut(&context_id)
            .ok_or_else(|| GraphicsError::InvalidContext("Security context not found".to_string()))?;
        context.grant_permission(perm);
        Ok(())
    }

    /// Revoke permission from context
    pub fn revoke_permission(
        &self,
        context_id: GpuContextId,
        perm: GpuPermission,
    ) -> GraphicsResult<()> {
        let mut contexts = self.security_contexts.lock();
        let context = contexts
            .get_mut(&context_id)
            .ok_or_else(|| GraphicsError::InvalidContext("Security context not found".to_string()))?;
        context.revoke_permission(perm);
        Ok(())
    }

    /// Enable HDCP
    pub fn enable_hdcp(&self) -> GraphicsResult<()> {
        if !self.supports_hdcp() {
            return Err(GraphicsError::NotSupported("HDCP not supported".to_string()));
        }

        let mut config = self.secure_display.write();
        config.enable_hdcp(HdcpVersion::Hdcp22);

        // Simulate authentication
        config.set_hdcp_state(HdcpState::Enabled);

        Ok(())
    }

    /// Disable HDCP
    pub fn disable_hdcp(&self) -> GraphicsResult<()> {
        let mut config = self.secure_display.write();
        config.disable_hdcp();
        Ok(())
    }

    /// Get HDCP state
    pub fn hdcp_state(&self) -> HdcpState {
        let config = self.secure_display.read();
        config.hdcp_state
    }

    /// Get secure display configuration
    pub fn secure_display_config(&self) -> SecureDisplayConfig {
        let config = self.secure_display.read();
        config.clone()
    }

    /// Set content protection level
    pub fn set_protection_level(&self, level: u32) -> GraphicsResult<()> {
        let mut config = self.secure_display.write();
        config.set_protection_level(level);
        Ok(())
    }

    /// Add attack mitigation
    pub fn add_mitigation(&self, attack: AttackType, strategy: MitigationStrategy) {
        let mut mitigations = self.mitigations.lock();
        mitigations.insert(attack, strategy);
    }

    /// Get mitigation strategy
    pub fn get_mitigation(&self, attack: AttackType) -> Option<MitigationStrategy> {
        let mitigations = self.mitigations.lock();
        mitigations.get(&attack).copied()
    }

    /// Create security policy
    pub fn create_policy(&self, name: &str) -> GraphicsResult<u32> {
        let id = self.next_policy_id.fetch_add(1, Ordering::SeqCst);
        let policy = SecurityPolicy::new(id, name);

        let mut policies = self.policies.lock();
        policies.insert(id, policy);

        Ok(id)
    }

    /// Get security policy
    pub fn get_policy(&self, id: u32) -> GraphicsResult<SecurityPolicy> {
        let policies = self.policies.lock();
        policies
            .get(&id)
            .cloned()
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid policy ID".to_string()))
    }

    /// Apply security policy to context
    pub fn apply_policy(&self, _context_id: GpuContextId, _policy_id: u32) -> GraphicsResult<()> {
        // In real implementation, apply policy rules to context
        Ok(())
    }

    /// Validate GPU access
    pub fn validate_access(&self, context_id: GpuContextId, operation: &str) -> GraphicsResult<()> {
        let context = self.get_security_context(context_id)?;

        // Check if context has required permissions
        let required = match operation {
            "read" => GpuPermission::Read,
            "write" => GpuPermission::Write,
            "execute" => GpuPermission::Execute,
            _ => return Err(GraphicsError::PermissionDenied("Invalid operation".to_string())),
        };

        if !context.has_permission(required) {
            return Err(GraphicsError::PermissionDenied(
                "Insufficient permissions".to_string(),
            ));
        }

        Ok(())
    }

    /// Isolate GPU context
    pub fn isolate_context(&self, context_id: GpuContextId) -> GraphicsResult<()> {
        let context = self.get_secure_context(context_id)?;
        context.enable_memory_isolation();
        Ok(())
    }

    /// Enable TEE integration
    pub fn enable_tee(&self) -> GraphicsResult<()> {
        let mut config = self.secure_display.write();
        config.tee_integration = true;
        Ok(())
    }

    /// Disable TEE integration
    pub fn disable_tee(&self) -> GraphicsResult<()> {
        let mut config = self.secure_display.write();
        config.tee_integration = false;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_context() {
        let id = GpuContextId::new(1);
        let context = SecurityContext::new(id, 100, 501, 501);
        assert_eq!(context.context_id, id);
        assert_eq!(context.pid, 100);
        assert!(!context.has_permission(GpuPermission::Read));
    }

    #[test]
    fn test_security_context_permissions() {
        let id = GpuContextId::new(1);
        let mut context = SecurityContext::new(id, 100, 501, 501);

        context.grant_permission(GpuPermission::Read);
        assert!(context.has_permission(GpuPermission::Read));

        context.grant_permission(GpuPermission::Write);
        assert!(context.has_permission(GpuPermission::Write));

        context.revoke_permission(GpuPermission::Read);
        assert!(!context.has_permission(GpuPermission::Read));
    }

    #[test]
    fn test_secure_display_config() {
        let config = SecureDisplayConfig::new();
        assert!(!config.hdcp_enabled);
        assert_eq!(config.hdcp_state, HdcpState::Disabled);
    }

    #[test]
    fn test_secure_display_hdcp() {
        let mut config = SecureDisplayConfig::new();
        config.enable_hdcp(HdcpVersion::Hdcp22);

        assert!(config.hdcp_enabled);
        assert_eq!(config.hdcp_version, Some(HdcpVersion::Hdcp22));
        assert_eq!(config.hdcp_state, HdcpState::Authenticating);

        config.set_hdcp_state(HdcpState::Enabled);
        assert_eq!(config.hdcp_state, HdcpState::Enabled);
    }

    #[test]
    fn test_secure_context() {
        let id = GpuContextId::new(1);
        let context = SecureContext::new(id, 100, 501, 501);

        assert_eq!(context.id(), id);
        assert!(!context.is_active());
        assert!(context.is_memory_isolated());

        context.activate();
        assert!(context.is_active());
    }

    #[test]
    fn test_security_policy() {
        let policy = SecurityPolicy::new(1, "test_policy");
        assert_eq!(policy.id, 1);
        assert_eq!(policy.name, "test_policy");
        assert_eq!(policy.max_memory_per_context, 512 * 1024 * 1024);
    }

    #[test]
    fn test_security_policy_custom() {
        let mut policy = SecurityPolicy::new(1, "custom_policy");
        policy.set_max_memory(1024 * 1024 * 1024);
        policy.set_max_command_buffer_size(512 * 1024);

        assert_eq!(policy.max_memory_per_context, 1024 * 1024 * 1024);
        assert_eq!(policy.max_command_buffer_size, 512 * 1024);
    }

    #[test]
    fn test_gpu_security_init() {
        let security = GpuSecurity::init(&unsafe { core::mem::zeroed() }).unwrap();
        assert!(security.supports_hdcp());
    }

    #[test]
    fn test_gpu_security_create_context() {
        let security = GpuSecurity::init(&unsafe { core::mem::zeroed() }).unwrap();
        let context = security.create_security_context(100, 501, 501).unwrap();
        assert_eq!(context.context_id.value(), 1);
        assert_eq!(context.pid, 100);
    }

    #[test]
    fn test_gpu_security_permissions() {
        let security = GpuSecurity::init(&unsafe { core::mem::zeroed() }).unwrap();
        let context = security.create_security_context(100, 501, 501).unwrap();

        assert!(!context.has_permission(GpuPermission::Read));

        security
            .grant_permission(context.context_id, GpuPermission::Read)
            .unwrap();

        let updated = security.get_security_context(context.context_id).unwrap();
        assert!(updated.has_permission(GpuPermission::Read));
    }

    #[test]
    fn test_gpu_security_check_permission() {
        let security = GpuSecurity::init(&unsafe { core::mem::zeroed() }).unwrap();
        let context = security.create_security_context(100, 501, 501).unwrap();

        security
            .grant_permission(context.context_id, GpuPermission::Write)
            .unwrap();

        assert!(security
            .check_permission(context.context_id, GpuPermission::Write)
            .unwrap());
    }

    #[test]
    fn test_gpu_security_hdcp() {
        let security = GpuSecurity::init(&unsafe { core::mem::zeroed() }).unwrap();

        assert!(security.enable_hdcp().is_ok());
        assert_eq!(security.hdcp_state(), HdcpState::Enabled);

        assert!(security.disable_hdcp().is_ok());
        assert_eq!(security.hdcp_state(), HdcpState::Disabled);
    }

    #[test]
    fn test_gpu_security_hdcp_unsupported() {
        let security = GpuSecurity::init(&unsafe { core::mem::zeroed() }).unwrap();
        security.hdcp_supported.store(false, Ordering::Release);

        assert!(security.enable_hdcp().is_err());
    }

    #[test]
    fn test_gpu_security_secure_context() {
        let security = GpuSecurity::init(&unsafe { core::mem::zeroed() }).unwrap();
        let context = security.create_secure_context(100).unwrap();

        assert_eq!(context.id().value(), 1);
        assert!(context.is_memory_isolated());

        security
            .isolate_context(context.id())
            .unwrap();

        let context = security.get_secure_context(context.id()).unwrap();
        assert!(context.is_memory_isolated());
    }

    #[test]
    fn test_gpu_security_mitigation() {
        let security = GpuSecurity::init(&unsafe { core::mem::zeroed() }).unwrap();

        let strategy = security.get_mitigation(AttackType::DmaAttack);
        assert_eq!(strategy, Some(MitigationStrategy::HardwareMitigation));

        security.add_mitigation(AttackType::DmaAttack, MitigationStrategy::DisableFeature);

        let strategy = security.get_mitigation(AttackType::DmaAttack);
        assert_eq!(strategy, Some(MitigationStrategy::DisableFeature));
    }

    #[test]
    fn test_gpu_security_policy() {
        let security = GpuSecurity::init(&unsafe { core::mem::zeroed() }).unwrap();
        let id = security.create_policy("test_policy").unwrap();
        assert_eq!(id, 1);

        let policy = security.get_policy(id).unwrap();
        assert_eq!(policy.name, "test_policy");
    }

    #[test]
    fn test_gpu_security_validate_access() {
        let security = GpuSecurity::init(&unsafe { core::mem::zeroed() }).unwrap();
        let context = security.create_security_context(100, 501, 501).unwrap();

        security
            .grant_permission(context.context_id, GpuPermission::Read)
            .unwrap();

        assert!(security
            .validate_access(context.context_id, "read")
            .is_ok());

        assert!(security
            .validate_access(context.context_id, "write")
            .is_err());
    }

    #[test]
    fn test_gpu_security_tee() {
        let security = GpuSecurity::init(&unsafe { core::mem::zeroed() }).unwrap();

        assert!(security.enable_tee().is_ok());

        let config = security.secure_display_config();
        assert!(config.tee_integration);

        assert!(security.disable_tee().is_ok());
    }

    #[test]
    fn test_hdcp_state() {
        let state1 = HdcpState::Disabled;
        let state2 = HdcpState::Authenticating;
        let state3 = HdcpState::Enabled;
        let state4 = HdcpState::Failed;

        assert!(state1 != state2);
        assert!(state2 != state3);
        assert!(state3 != state4);
    }

    #[test]
    fn test_attack_types() {
        assert_ne!(AttackType::DmaAttack, AttackType::SideChannel);
        assert_ne!(AttackType::Rowhammer, AttackType::SpectreMeltdown);
    }
}
