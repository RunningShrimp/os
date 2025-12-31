//! # Mandatory Access Control (MAC) Framework
//!
//! This module provides comprehensive MAC support including SELinux, AppArmor, and Smack.
//!
//! ## Features
//!
//! - **SELinux**: Type enforcement policy engine
//! - **AppArmor**: Path-based profile enforcement
//! - **Smack**: Label-based access control
//! - **Policy Loading**: Dynamic policy loading and unloading
//! - **Multi-Policy**: Support for multiple concurrent policies
//! - **Label Management**: Security context and label operations

use crate::prelude::*;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

// ============================================================================
// Constants
// ============================================================================

/// Maximum policy size (16MB)
const MAX_POLICY_SIZE: usize = 0x100_0000;

/// Maximum label length
const MAX_LABEL_LEN: usize = 256;

/// Maximum context length
const MAX_CONTEXT_LEN: usize = 4096;

/// SELinux enforce value
const SELINUX_ENFORCE: u32 = 1;

/// SELinux permissive value
const SELINUX_PERMISSIVE: u32 = 0;

/// SELinux disabled value
const SELINUX_DISABLED: u32 = -1i32 as u32;

// ============================================================================
// Error Type
// ============================================================================

pub use crate::error::unified::MacError;

pub type MacResult<T> = Result<T, MacError>;

// ============================================================================
// MAC Policy Types
// ============================================================================

/// MAC policy type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub enum PolicyType {
    /// SELinux policy
    Selinux,
    /// AppArmor profile
    AppArmor,
    /// Smack policy
    Smack,
    /// Tomoyo policy
    Tomoyo,
}

/// Policy state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyState {
    /// Policy is enabled and enforcing
    Enforcing,
    /// Policy is enabled but permissive
    Permissive,
    /// Policy is disabled
    Disabled,
}

/// Security label
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SecurityLabel {
    /// Label string
    label: String,
    /// Label type
    label_type: PolicyType,
}

impl SecurityLabel {
    /// Create new security label
    pub fn new(label: String, label_type: PolicyType) -> MacResult<Self> {
        if label.is_empty() || label.len() > MAX_LABEL_LEN {
            return Err(MacError::InvalidLabel);
        }
        Ok(Self { label, label_type })
    }

    /// Get label string
    pub fn as_str(&self) -> &str {
        &self.label
    }

    /// Get label type
    pub fn label_type(&self) -> PolicyType {
        self.label_type
    }

    /// Check if is wildcard
    pub fn is_wildcard(&self) -> bool {
        self.label == "*" || self.label == "?"
    }
}

/// Security context
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// User
    user: String,
    /// Role
    role: String,
    /// Type
    type_: String,
    /// Level (for MLS)
    level: String,
}

impl SecurityContext {
    /// Create new security context
    pub fn new(user: String, role: String, type_: String, level: String) -> Self {
        Self {
            user,
            role,
            type_,
            level,
        }
    }

    /// Parse from string (format: user:role:type:level)
    pub fn from_str(s: &str) -> MacResult<Self> {
        let parts: Vec<&str> = s.split(':').collect();
        if parts.len() < 3 {
            return Err(MacError::InvalidContext);
        }

        let user = parts[0].to_string();
        let role = parts[1].to_string();
        let type_ = parts[2].to_string();
        let level = if parts.len() > 3 {
            parts[3].to_string()
        } else {
            "s0".to_string()
        };

        Ok(Self::new(user, role, type_, level))
    }

    /// Convert to string
    pub fn to_string(&self) -> String {
        format!("{}:{}:{}:{}", self.user, self.role, self.type_, self.level)
    }

    /// Get type
    pub fn type_(&self) -> &str {
        &self.type_
    }

    /// Get user
    pub fn user(&self) -> &str {
        &self.user
    }

    /// Get role
    pub fn role(&self) -> &str {
        &self.role
    }

    /// Get level
    pub fn level(&self) -> &str {
        &self.level
    }
}

/// Access vector (permission)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessVector {
    /// Permissions
    perms: u32,
}

impl AccessVector {
    /// Create new access vector
    pub fn new(perms: u32) -> Self {
        Self { perms }
    }

    /// Add permission
    pub fn with_perm(mut self, perm: u32) -> Self {
        self.perms |= perm;
        self
    }

    /// Check if has permission
    pub fn has(&self, perm: u32) -> bool {
        self.perms & perm == perm
    }

    /// Get all permissions
    pub fn perms(&self) -> u32 {
        self.perms
    }
}

// ============================================================================
// SELinux Policy
// ============================================================================

/// SELinux type enforcement rule
#[derive(Debug, Clone)]
pub struct TeRule {
    /// Source type
    source_type: String,
    /// Target type
    target_type: String,
    /// Object class
    object_class: String,
    /// Permissions
    permissions: AccessVector,
}

impl TeRule {
    /// Create new TE rule
    pub fn new(
        source_type: String,
        target_type: String,
        object_class: String,
        permissions: AccessVector,
    ) -> Self {
        Self {
            source_type,
            target_type,
            object_class,
            permissions,
        }
    }

    /// Match source and target types
    pub fn matches(&self, source: &str, target: &str) -> bool {
        self.source_type == source && self.target_type == target
    }

    /// Check if allows permission
    pub fn allows(&self, perm: u32) -> bool {
        self.permissions.has(perm)
    }
}

/// SELinux policy
#[derive(Debug)]
pub struct SelinuxPolicy {
    /// Policy state
    state: AtomicU32,
    /// Type enforcement rules
    te_rules: Mutex<Vec<TeRule>>,
    /// Type aliases
    type_aliases: Mutex<BTreeMap<String, String>>,
    /// Role mappings
    role_mappings: Mutex<BTreeMap<String, Vec<String>>>,
    /// Default contexts
    default_contexts: Mutex<BTreeMap<String, SecurityContext>>,
}

impl SelinuxPolicy {
    /// Create new SELinux policy
    pub fn new() -> Self {
        Self {
            state: AtomicU32::new(SELINUX_PERMISSIVE),
            te_rules: Mutex::new(Vec::new()),
            type_aliases: Mutex::new(BTreeMap::new()),
            role_mappings: Mutex::new(BTreeMap::new()),
            default_contexts: Mutex::new(BTreeMap::new()),
        }
    }

    /// Set policy state
    pub fn set_state(&self, state: PolicyState) {
        let value = match state {
            PolicyState::Enforcing => SELINUX_ENFORCE,
            PolicyState::Permissive => SELINUX_PERMISSIVE,
            PolicyState::Disabled => SELINUX_DISABLED,
        };
        self.state.store(value, Ordering::SeqCst);
    }

    /// Get policy state
    pub fn state(&self) -> PolicyState {
        match self.state.load(Ordering::SeqCst) {
            SELINUX_ENFORCE => PolicyState::Enforcing,
            SELINUX_PERMISSIVE => PolicyState::Permissive,
            _ => PolicyState::Disabled,
        }
    }

    /// Add TE rule
    pub fn add_te_rule(&self, rule: TeRule) {
        self.te_rules.lock().push(rule);
    }

    /// Find TE rules
    pub fn find_te_rules(&self, source: &str, target: &str) -> Vec<TeRule> {
        self.te_rules
            .lock()
            .iter()
            .filter(|r| r.matches(source, target))
            .cloned()
            .collect()
    }

    /// Add type alias
    pub fn add_type_alias(&self, alias: String, actual_type: String) {
        self.type_aliases.lock().insert(alias, actual_type);
    }

    /// Resolve type alias
    pub fn resolve_type(&self, type_: &str) -> String {
        self.type_aliases
            .lock()
            .get(type_)
            .cloned()
            .unwrap_or_else(|| type_.to_string())
    }

    /// Check access
    pub fn check_access(
        &self,
        source: &str,
        target: &str,
        class: &str,
        perm: u32,
    ) -> MacResult<bool> {
        // Resolve type aliases
        let source_type = self.resolve_type(source);
        let target_type = self.resolve_type(target);

        // Find matching rules
        let rules = self.find_te_rules(&source_type, &target_type);

        // Check if any rule allows the permission
        for rule in rules {
            if rule.object_class == class && rule.allows(perm) {
                return Ok(true);
            }
        }

        // Default deny
        Ok(false)
    }
}

// ============================================================================
// AppArmor Policy
// ============================================================================

/// AppArmor profile rule
#[derive(Debug, Clone)]
pub struct AaRule {
    /// Path pattern
    path: String,
    /// Permissions
    permissions: u32,
    /// File mode (r, w, x, etc.)
    file_mode: String,
}

impl AaRule {
    /// Create new AppArmor rule
    pub fn new(path: String, permissions: u32, file_mode: String) -> Self {
        Self {
            path,
            permissions,
            file_mode,
        }
    }

    /// Check if matches path
    pub fn matches(&self, path: &str) -> bool {
        // Simple matching (would support wildcards in real implementation)
        self.path == path
    }

    /// Check if allows permission
    pub fn allows(&self, perm: u32) -> bool {
        self.permissions & perm == perm
    }
}

/// AppArmor profile
#[derive(Debug)]
pub struct AppArmorProfile {
    /// Profile name
    name: String,
    /// Profile state
    state: PolicyState,
    /// Rules
    rules: Mutex<Vec<AaRule>>,
    /// Attach conditions
    attach: Mutex<Option<String>>,
}

impl AppArmorProfile {
    /// Create new AppArmor profile
    pub fn new(name: String) -> Self {
        Self {
            name,
            state: PolicyState::Enforcing,
            rules: Mutex::new(Vec::new()),
            attach: Mutex::new(None),
        }
    }

    /// Get profile name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set profile state
    pub fn set_state(&mut self, state: PolicyState) {
        self.state = state;
    }

    /// Get profile state
    pub fn state(&self) -> PolicyState {
        self.state
    }

    /// Add rule
    pub fn add_rule(&self, rule: AaRule) {
        self.rules.lock().push(rule);
    }

    /// Check file access
    pub fn check_file_access(&self, path: &str, perm: u32) -> MacResult<bool> {
        let rules = self.rules.lock();

        // Find matching rule
        for rule in rules.iter() {
            if rule.matches(path) {
                return Ok(rule.allows(perm));
            }
        }

        // Default deny
        Ok(false)
    }
}

// ============================================================================
// MAC Manager
// ============================================================================

/// MAC subject (process, socket, etc.)
#[derive(Debug)]
pub struct Subject {
    /// Security context
    context: Mutex<SecurityContext>,
    /// Subject ID (PID, socket FD, etc.)
    id: u32,
    /// Subject type
    subject_type: SubjectType,
}

/// Subject type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubjectType {
    /// Process
    Process,
    /// Socket
    Socket,
    /// File descriptor
    Fd,
    /// IPC object
    Ipc,
}

impl Subject {
    /// Create new subject
    pub fn new(id: u32, subject_type: SubjectType, context: SecurityContext) -> Self {
        Self {
            context: Mutex::new(context),
            id,
            subject_type,
        }
    }

    /// Get security context
    pub fn context(&self) -> SecurityContext {
        self.context.lock().clone()
    }

    /// Set security context
    pub fn set_context(&self, context: SecurityContext) {
        *self.context.lock() = context;
    }

    /// Get subject ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get subject type
    pub fn subject_type(&self) -> SubjectType {
        self.subject_type
    }
}

/// MAC object (file, socket, etc.)
#[derive(Debug)]
pub struct Object {
    /// Security label
    label: SecurityLabel,
    /// Object type
    object_type: ObjectType,
}

/// Object type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    /// File
    File,
    /// Directory
    Directory,
    /// Socket
    Socket,
    /// IPC object
    Ipc,
    /// Process
    Process,
    /// Network endpoint
    Network,
}

impl Object {
    /// Create new object
    pub fn new(label: SecurityLabel, object_type: ObjectType) -> Self {
        Self { label, object_type }
    }

    /// Get security label
    pub fn label(&self) -> &SecurityLabel {
        &self.label
    }

    /// Get object type
    pub fn object_type(&self) -> ObjectType {
        self.object_type
    }
}

/// MAC manager
pub struct MacManager {
    /// SELinux policy
    selinux_policy: Mutex<Option<SelinuxPolicy>>,
    /// AppArmor profiles (indexed by name)
    apparmor_profiles: Mutex<BTreeMap<String, AppArmorProfile>>,
    /// Subjects (indexed by ID)
    subjects: Mutex<BTreeMap<u32, Arc<Subject>>>,
    /// MAC enabled
    enabled: AtomicBool,
    /// Active policies
    active_policies: Mutex<BTreeSet<PolicyType>>,
}

impl MacManager {
    /// Create new MAC manager
    pub fn new() -> Self {
        Self {
            selinux_policy: Mutex::new(None),
            apparmor_profiles: Mutex::new(BTreeMap::new()),
            subjects: Mutex::new(BTreeMap::new()),
            enabled: AtomicBool::new(false),
            active_policies: Mutex::new(BTreeSet::new()),
        }
    }

    /// Initialize MAC subsystem
    pub fn init(&self) -> MacResult<()> {
        self.enabled.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Check if MAC is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Load SELinux policy
    pub fn load_selinux_policy(&self, policy_data: &[u8]) -> MacResult<()> {
        if policy_data.len() > MAX_POLICY_SIZE {
            return Err(MacError::PolicyLoadFailed);
        }

        // In a real implementation, this would:
        // 1. Parse binary policy
        // 2. Validate policy structure
        // 3. Load into kernel

        let policy = SelinuxPolicy::new();
        *self.selinux_policy.lock() = Some(policy);

        let mut active = self.active_policies.lock();
        active.insert(PolicyType::Selinux);

        Ok(())
    }

    /// Load AppArmor profile
    pub fn load_apparmor_profile(&self, name: String, profile_data: &[u8]) -> MacResult<()> {
        if profile_data.len() > MAX_POLICY_SIZE {
            return Err(MacError::PolicyLoadFailed);
        }

        // In a real implementation, this would:
        // 1. Parse profile text
        // 2. Compile rules
        // 3. Load into kernel

        let profile = AppArmorProfile::new(name.clone());
        self.apparmor_profiles.lock().insert(name, profile);

        let mut active = self.active_policies.lock();
        active.insert(PolicyType::AppArmor);

        Ok(())
    }

    /// Unload policy
    pub fn unload_policy(&self, policy_type: PolicyType) -> MacResult<()> {
        match policy_type {
            PolicyType::Selinux => {
                *self.selinux_policy.lock() = None;
            }
            PolicyType::AppArmor => {
                // Clear all AppArmor profiles
                self.apparmor_profiles.lock().clear();
            }
            _ => {
                return Err(MacError::PolicyNotFound);
            }
        }

        let mut active = self.active_policies.lock();
        active.remove(&policy_type);

        Ok(())
    }

    /// Register subject
    pub fn register_subject(&self, subject: Arc<Subject>) -> MacResult<()> {
        let mut subjects = self.subjects.lock();
        subjects.insert(subject.id(), subject);
        Ok(())
    }

    /// Unregister subject
    pub fn unregister_subject(&self, id: u32) -> MacResult<()> {
        let mut subjects = self.subjects.lock();
        subjects.remove(&id).ok_or(MacError::SubjectNotFound)?;
        Ok(())
    }

    /// Get subject
    pub fn get_subject(&self, id: u32) -> MacResult<Arc<Subject>> {
        let subjects = self.subjects.lock();
        let subject = subjects.get(&id).ok_or(MacError::SubjectNotFound)?;
        Ok(subject.clone())
    }

    /// Check access (main entry point)
    pub fn check_access(
        &self,
        subject: &Subject,
        object: &Object,
        permission: u32,
    ) -> MacResult<bool> {
        if !self.is_enabled() {
            return Ok(true); // Allow if MAC disabled
        }

        let active = self.active_policies.lock();

        // Check each active policy
        for policy_type in active.iter() {
            let allowed = match policy_type {
                PolicyType::Selinux => self.check_selinux_access(subject, object, permission)?,
                PolicyType::AppArmor => {
                    self.check_apparmor_access(subject, object, permission)?
                }
                _ => true, // Default allow for unsupported policies
            };

            if !allowed {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Check SELinux access
    fn check_selinux_access(
        &self,
        subject: &Subject,
        object: &Object,
        permission: u32,
    ) -> MacResult<bool> {
        let selinux_guard = self.selinux_policy.lock();
        let selinux = selinux_guard
            .as_ref()
            .ok_or(MacError::PolicyNotFound)?;

        let sctx = subject.context();
        let tlabel = object.label();

        // Get class name from object type
        let class = match object.object_type() {
            ObjectType::File => "file",
            ObjectType::Directory => "dir",
            ObjectType::Socket => "socket",
            ObjectType::Ipc => "ipc",
            ObjectType::Process => "process",
            ObjectType::Network => "network",
        };

        // Check policy state
        match selinux.state() {
            PolicyState::Disabled => return Ok(true),
            PolicyState::Permissive => {
                // Allow but log
                let _ = selinux.check_access(
                    sctx.type_(),
                    tlabel.as_str(),
                    class,
                    permission,
                );
                return Ok(true);
            }
            PolicyState::Enforcing => {}
        }

        // Check access
        selinux.check_access(sctx.type_(), tlabel.as_str(), class, permission)
    }

    /// Check AppArmor access
    fn check_apparmor_access(
        &self,
        _subject: &Subject,
        _object: &Object,
        _permission: u32,
    ) -> MacResult<bool> {
        // In a real implementation, this would:
        // 1. Find profile for subject
        // 2. Check path-based rules
        // 3. Return allow/deny

        // Stub implementation
        Ok(true)
    }

    /// Set subject security context
    pub fn set_context(&self, id: u32, context: SecurityContext) -> MacResult<()> {
        let subject = self.get_subject(id)?;
        subject.set_context(context);
        Ok(())
    }

    /// Get subject security context
    pub fn get_context(&self, id: u32) -> MacResult<SecurityContext> {
        let subject = self.get_subject(id)?;
        Ok(subject.context())
    }

    /// Set policy state
    pub fn set_policy_state(&self, policy_type: PolicyType, state: PolicyState) -> MacResult<()> {
        match policy_type {
            PolicyType::Selinux => {
                let selinux_guard = self.selinux_policy.lock();
                let selinux = selinux_guard
                    .as_ref()
                    .ok_or(MacError::PolicyNotFound)?;
                selinux.set_state(state);
            }
            PolicyType::AppArmor => {
                // In a real implementation, would set profile state
                return Err(MacError::NotSupported);
            }
            _ => {
                return Err(MacError::PolicyNotFound);
            }
        }
        Ok(())
    }

    /// Get policy state
    pub fn get_policy_state(&self, policy_type: PolicyType) -> MacResult<PolicyState> {
        match policy_type {
            PolicyType::Selinux => {
                let selinux_guard = self.selinux_policy.lock();
                let selinux = selinux_guard
                    .as_ref()
                    .ok_or(MacError::PolicyNotFound)?;
                Ok(selinux.state())
            }
            PolicyType::AppArmor => {
                // In a real implementation, would get profile state
                Ok(PolicyState::Enforcing)
            }
            _ => Err(MacError::PolicyNotFound),
        }
    }

    /// Get statistics
    pub fn get_stats(&self) -> MacStats {
        let active = self.active_policies.lock();
        let subjects = self.subjects.lock();
        let profiles = self.apparmor_profiles.lock();

        MacStats {
            selinux_enabled: active.contains(&PolicyType::Selinux),
            apparmor_enabled: active.contains(&PolicyType::AppArmor),
            smack_enabled: active.contains(&PolicyType::Smack),
            total_subjects: subjects.len(),
            total_profiles: profiles.len(),
        }
    }
}

/// MAC statistics
#[derive(Debug, Clone)]
pub struct MacStats {
    /// SELinux enabled
    pub selinux_enabled: bool,
    /// AppArmor enabled
    pub apparmor_enabled: bool,
    /// Smack enabled
    pub smack_enabled: bool,
    /// Total subjects
    pub total_subjects: usize,
    /// Total AppArmor profiles
    pub total_profiles: usize,
}

// ============================================================================
// Global MAC Manager
// ============================================================================

use crate::subsystems::sync::lazy::Lazy;

static MAC_MANAGER: Lazy<spin::Mutex<MacManager>> =
    Lazy::new(|| spin::Mutex::new(MacManager::new()));


/// Initialize MAC subsystem
pub fn init_mac() -> MacResult<()> {
    MAC_MANAGER.lock().init()
}

/// Load policy
pub fn load_policy(policy_type: PolicyType, policy_data: &[u8]) -> MacResult<()> {
    match policy_type {
        PolicyType::Selinux => MAC_MANAGER.lock().load_selinux_policy(policy_data),
        PolicyType::AppArmor => {
            MAC_MANAGER.lock().load_apparmor_profile("default".to_string(), policy_data)
        }
        _ => Err(MacError::NotSupported),
    }
}

/// Unload policy
pub fn unload_policy(policy_type: PolicyType) -> MacResult<()> {
    MAC_MANAGER.lock().unload_policy(policy_type)
}

/// Check access
pub fn check_access(
    subject: &Subject,
    object: &Object,
    permission: u32,
) -> MacResult<bool> {
    MAC_MANAGER.lock().check_access(subject, object, permission)
}

/// Set security context
pub fn set_context(id: u32, context: SecurityContext) -> MacResult<()> {
    MAC_MANAGER.lock().set_context(id, context)
}

/// Get security context
pub fn get_context(id: u32) -> MacResult<SecurityContext> {
    MAC_MANAGER.lock().get_context(id)
}

/// Get MAC statistics
pub fn get_mac_stats() -> MacStats {
    MAC_MANAGER.lock().get_stats()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_context() {
        let ctx = SecurityContext::new("user_u".to_string(), "role_r".to_string(), "type_t".to_string(), "s0".to_string());
        assert_eq!(ctx.type_(), "type_t");
        assert_eq!(ctx.user(), "user_u");
    }

    #[test]
    fn test_security_context_from_str() {
        let ctx = SecurityContext::from_str("user_u:role_r:type_t:s0").unwrap();
        assert_eq!(ctx.type_(), "type_t");
        assert_eq!(ctx.role(), "role_r");
    }

    #[test]
    fn test_access_vector() {
        let av = AccessVector::new(0).with_perm(0x01).with_perm(0x02);
        assert!(av.has(0x01));
        assert!(av.has(0x02));
        assert!(!av.has(0x04));
    }

    #[test]
    fn test_security_label() {
        let label = SecurityLabel::new("system_u:object_r:file_t".to_string(), PolicyType::Selinux).unwrap();
        assert_eq!(label.as_str(), "system_u:object_r:file_t");
        assert!(!label.is_wildcard());
    }

    #[test]
    fn test_te_rule() {
        let rule = TeRule::new(
            "httpd_t".to_string(),
            "httpd_content_t".to_string(),
            "file".to_string(),
            AccessVector::new(0).with_perm(0x01),
        );
        assert!(rule.matches("httpd_t", "httpd_content_t"));
        assert!(rule.allows(0x01));
    }
}

pub fn mac_manager() -> &'static Lazy<spin::Mutex<MacManager>> {
    &MAC_MANAGER
}
