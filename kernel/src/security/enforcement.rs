//! # Security Enforcement Module
//!
//! This module provides comprehensive security enforcement including LSM hooks,
//! security module registry, and per-process security context management.
//!
//! ## Features
//!
//! - **LSM Hooks**: Linux Security Modules hook points
//! - **Security Module Registry**: Dynamic security module registration
//! - **Security Context**: Per-process security state
//! - **Security-Aware Scheduling**: Security priority in scheduler
//! - **Policy Negotiation**: Multi-module policy composition
//! - **Security Statistics**: Real-time security metrics

use crate::prelude::*;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// ============================================================================
// Constants
// ============================================================================

/// Maximum number of security modules
const MAX_SECURITY_MODULES: usize = 16;

/// Maximum security context size
const MAX_CONTEXT_SIZE: usize = 4096;

/// LSM hook count
const LSM_HOOK_COUNT: usize = 100;

/// Security task priority boost
const SECURITY_PRIORITY_BOOST: i32 = 5;

// ============================================================================
// LSM Hooks
// ============================================================================

/// LSM hook types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsmHook {
    /// File permission check
    FilePermission,
    /// File open
    FileOpen,
    /// File operation (read, write, etc.)
    FileOperation,
    /// Socket creation
    SocketCreate,
    /// Socket connection
    SocketConnect,
    /// Socket accept
    SocketAccept,
    /// Socket send
    SocketSend,
    /// Socket receive
    SocketReceive,
    /// Process execution
    BprmSetSecurity,
    /// Process fork
    TaskFork,
    /// Process exit
    TaskFree,
    /// Capability check
    Capability,
    /// Unix stream connection
    UnixStreamConnect,
    /// mmap check
    MmapFile,
    /// File lock
    FileLock,
    /// File permission
    FilePermissionSb,
    /// Kernel module load
    KernelModuleRequest,
    /// Kernel read file
    KernelReadFile,
    /// Task prctl
    TaskPrctl,
    /// Setuid
    Setuid,
    /// Setgid
    Setgid,
    /// Setgroups
    Setgroups,
    /// Syslog access
    Syslog,
    /// Settime
    Settime,
    /// Quota control
    QuotaOn,
    /// MSG queue permission
    MsgQueuePermission,
    /// Shared memory permission
    ShmPermission,
    /// Semaphore permission
    SemPermission,
    /// Key permission
    KeyPermission,
    /// IPC permission
    IpcPermission,
}

/// LSM hook callback result
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LsmResult {
    /// Allow operation
    Allow,
    /// Deny operation
    Deny,
    /// Return error
    Error(i32),
}

impl LsmResult {
    /// Check if allowed
    pub fn is_allowed(&self) -> bool {
        matches!(self, LsmResult::Allow)
    }

    /// Convert to error code
    pub fn to_errno(&self) -> i32 {
        match self {
            LsmResult::Allow => 0,
            LsmResult::Deny => crate::reliability::errno::EACCES,
            LsmResult::Error(err) => *err,
        }
    }
}

/// LSM hook callback
pub type LsmCallback = fn(
    subject: &SecurityContext,
    object: &SecurityObject,
    operation: u32,
) -> LsmResult;

// ============================================================================
// Security Object
// ============================================================================

/// Security object type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityObjectType {
    /// File
    File,
    /// Directory
    Directory,
    /// Socket
    Socket,
    /// Process
    Process,
    /// IPC object
    Ipc,
    /// Key
    Key,
    /// Network device
    NetworkDevice,
    /// Unknown
    Unknown,
}

/// Security object
#[derive(Debug, Clone)]
pub struct SecurityObject {
    /// Object type
    object_type: SecurityObjectType,
    /// Object ID (inode, socket FD, etc.)
    id: u64,
    /// Security label
    label: Option<String>,
    /// Security context
    context: Option<SecurityContext>,
}

impl SecurityObject {
    /// Create new security object
    pub fn new(object_type: SecurityObjectType, id: u64) -> Self {
        Self {
            object_type,
            id,
            label: None,
            context: None,
        }
    }

    /// Get object type
    pub fn object_type(&self) -> SecurityObjectType {
        self.object_type
    }

    /// Get object ID
    pub fn id(&self) -> u64 {
        self.id
    }

    /// Get security label
    pub fn label(&self) -> Option<&String> {
        self.label.as_ref()
    }

    /// Set security label
    pub fn set_label(&mut self, label: String) {
        self.label = Some(label);
    }

    /// Get security context
    pub fn context(&self) -> Option<&SecurityContext> {
        self.context.as_ref()
    }

    /// Set security context
    pub fn set_context(&mut self, context: SecurityContext) {
        self.context = Some(context);
    }
}

// ============================================================================
// Security Context
// ============================================================================

use super::mac::SecurityContext as MacSecurityContext;

/// Security context (per-process)
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Process ID
    pid: u32,
    /// User ID
    uid: u32,
    /// Group ID
    gid: u32,
    /// Session ID
    session_id: u32,
    /// Security label
    label: String,
    /// MAC security context
    mac_context: Option<MacSecurityContext>,
    /// Capabilities
    capabilities: u64,
    /// Securebits
    securebits: u32,
    /// SELinux exec context
    selinux_exec: Option<String>,
    /// Created at
    created_at: u64,
}

impl SecurityContext {
    /// Create new security context
    pub fn new(pid: u32, uid: u32, gid: u32, label: String) -> Self {
        Self {
            pid,
            uid,
            gid,
            session_id: 0,
            label,
            mac_context: None,
            capabilities: 0,
            securebits: 0,
            selinux_exec: None,
            created_at: 0,
        }
    }

    /// Get PID
    pub fn pid(&self) -> u32 {
        self.pid
    }

    /// Get UID
    pub fn uid(&self) -> u32 {
        self.uid
    }

    /// Get GID
    pub fn gid(&self) -> u32 {
        self.gid
    }

    /// Get session ID
    pub fn session_id(&self) -> u32 {
        self.session_id
    }

    /// Set session ID
    pub fn set_session_id(&mut self, session_id: u32) {
        self.session_id = session_id;
    }

    /// Get security label
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Set security label
    pub fn set_label(&mut self, label: String) {
        self.label = label;
    }

    /// Get MAC context
    pub fn mac_context(&self) -> Option<&MacSecurityContext> {
        self.mac_context.as_ref()
    }

    /// Set MAC context
    pub fn set_mac_context(&mut self, context: MacSecurityContext) {
        self.mac_context = Some(context);
    }

    /// Get capabilities
    pub fn capabilities(&self) -> u64 {
        self.capabilities
    }

    /// Set capabilities
    pub fn set_capabilities(&mut self, capabilities: u64) {
        self.capabilities = capabilities;
    }

    /// Has capability
    pub fn has_capability(&self, cap: u64) -> bool {
        (self.capabilities & cap) == cap
    }

    /// Get securebits
    pub fn securebits(&self) -> u32 {
        self.securebits
    }

    /// Set securebits
    pub fn set_securebits(&mut self, securebits: u32) {
        self.securebits = securebits;
    }

    /// Get SELinux exec context
    pub fn selinux_exec(&self) -> Option<&String> {
        self.selinux_exec.as_ref()
    }

    /// Set SELinux exec context
    pub fn set_selinux_exec(&mut self, exec: String) {
        self.selinux_exec = Some(exec);
    }
}

// ============================================================================
// Security Module
// ============================================================================

/// Security module trait
pub trait SecurityModule: Send + Sync {
    /// Get module name
    fn name(&self) -> &str;

    /// Initialize module
    fn init(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Hook point for LSM
    fn hook(&self, _hook_type: LsmHook, _subject: &SecurityContext, _object: &SecurityObject, _operation: u32) -> LsmResult {
        LsmResult::Allow // Default allow
    }

    /// Set security context
    fn set_context(&self, _context: &mut SecurityContext) -> Result<(), String> {
        Ok(())
    }

    /// Get security context
    fn get_context(&self, context: &SecurityContext) -> Result<SecurityContext, String> {
        Ok(context.clone())
    }

    /// Clone context
    fn clone_context(&self, context: &SecurityContext) -> Result<SecurityContext, String> {
        Ok(context.clone())
    }

    /// Transfer context
    fn transfer_context(&self, src: &SecurityContext, dst: &mut SecurityContext) -> Result<(), String> {
        dst.label = src.label.clone();
        dst.mac_context = src.mac_context.clone();
        Ok(())
    }
}

/// Simple security module for testing
#[derive(Debug)]
pub struct DummySecurityModule {
    name: String,
}

impl DummySecurityModule {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl SecurityModule for DummySecurityModule {
    fn name(&self) -> &str {
        &self.name
    }
}

// ============================================================================
// Security Module Registry
// ============================================================================

/// Security module registry
pub struct SecurityModuleRegistry {
    /// Registered modules
    modules: Mutex<Vec<Box<dyn SecurityModule>>>,
    /// Module index by name
    module_index: Mutex<BTreeMap<String, usize>>,
    /// Enabled modules
    enabled: Mutex<BTreeSet<String>>,
    /// Hook callbacks
    hooks: Mutex<BTreeMap<LsmHook, Vec<usize>>>,
}

impl SecurityModuleRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            modules: Mutex::new(Vec::new()),
            module_index: Mutex::new(BTreeMap::new()),
            enabled: Mutex::new(BTreeSet::new()),
            hooks: Mutex::new(BTreeMap::new()),
        }
    }

    /// Register security module
    pub fn register(&self, module: Box<dyn SecurityModule>) -> Result<(), String> {
        let name = module.name().to_string();

        let mut modules = self.modules.lock();
        let mut index = self.module_index.lock();

        if index.contains_key(&name) {
            return Err(format!("Module {} already registered", name));
        }

        if modules.len() >= MAX_SECURITY_MODULES {
            return Err("Too many security modules".to_string());
        }

        let idx = modules.len();
        modules.push(module);
        index.insert(name, idx);

        Ok(())
    }

    /// Unregister security module
    pub fn unregister(&self, name: &str) -> Result<(), String> {
        let mut index = self.module_index.lock();
        let idx = index.remove(name).ok_or_else(|| format!("Module {} not found", name))?;

        let mut modules = self.modules.lock();
        if idx < modules.len() {
            modules.remove(idx);
        }

        // Update index
        for (_n, i) in index.iter_mut() {
            if *i > idx {
                *i -= 1;
            }
        }

        let mut enabled = self.enabled.lock();
        enabled.remove(name);

        Ok(())
    }

    /// Enable module
    pub fn enable(&self, name: &str) -> Result<(), String> {
        let index = self.module_index.lock();
        index.get(name).ok_or_else(|| format!("Module {} not found", name))?;

        let mut enabled = self.enabled.lock();
        enabled.insert(name.to_string());

        Ok(())
    }

    /// Disable module
    pub fn disable(&self, name: &str) -> Result<(), String> {
        let index = self.module_index.lock();
        index.get(name).ok_or_else(|| format!("Module {} not found", name))?;

        let mut enabled = self.enabled.lock();
        enabled.remove(name);

        Ok(())
    }

    /// Check if module is enabled
    pub fn is_enabled(&self, name: &str) -> bool {
        let enabled = self.enabled.lock();
        enabled.contains(name)
    }

    /// Get module by name
    pub fn get_module(&self, name: &str) -> Option<usize> {
        let index = self.module_index.lock();
        index.get(name).copied()
    }

    /// Call all enabled modules for hook
    pub fn call_hooks(
        &self,
        hook_type: LsmHook,
        subject: &SecurityContext,
        object: &SecurityObject,
        operation: u32,
    ) -> LsmResult {
        let enabled = self.enabled.lock();
        let modules = self.modules.lock();

        for name in enabled.iter() {
            if let Some(idx) = self.module_index.lock().get(name) {
                if let Some(module) = modules.get(*idx) {
                    let result = module.hook(hook_type, subject, object, operation);
                    if !result.is_allowed() {
                        return result;
                    }
                }
            }
        }

        LsmResult::Allow
    }

    /// Get statistics
    pub fn get_stats(&self) -> SecurityModuleStats {
        let modules = self.modules.lock();
        let enabled = self.enabled.lock();

        SecurityModuleStats {
            total_modules: modules.len(),
            enabled_modules: enabled.len(),
            registered_modules: modules.iter().map(|m| m.name().to_string()).collect(),
        }
    }
}

/// Security module statistics
#[derive(Debug, Clone)]
pub struct SecurityModuleStats {
    /// Total number of modules
    pub total_modules: usize,
    /// Number of enabled modules
    pub enabled_modules: usize,
    /// List of registered modules
    pub registered_modules: Vec<String>,
}

// ============================================================================
// Security-Aware Scheduling
// ============================================================================

/// Security priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SecurityPriority(i32);

impl SecurityPriority {
    /// Create new priority
    pub fn new(base: i32, security_factor: u32) -> Self {
        let boost = if security_factor > 0 {
            SECURITY_PRIORITY_BOOST
        } else {
            0
        };
        Self(base + boost)
    }

    /// Get raw value
    pub fn value(&self) -> i32 {
        self.0
    }
}

/// Security scheduler integration
pub struct SecurityScheduler {
    /// Process security scores
    security_scores: Mutex<BTreeMap<u32, u32>>,
}

impl SecurityScheduler {
    /// Create new security scheduler
    pub fn new() -> Self {
        Self {
            security_scores: Mutex::new(BTreeMap::new()),
        }
    }

    /// Set process security score
    pub fn set_security_score(&self, pid: u32, score: u32) {
        self.security_scores.lock().insert(pid, score);
    }

    /// Get process security score
    pub fn get_security_score(&self, pid: u32) -> Option<u32> {
        self.security_scores.lock().get(&pid).copied()
    }

    /// Calculate security priority
    pub fn calculate_priority(&self, pid: u32, base_priority: i32) -> SecurityPriority {
        let score = self.get_security_score(pid).unwrap_or(0);
        SecurityPriority::new(base_priority, score)
    }

    /// Remove process
    pub fn remove_process(&self, pid: u32) {
        self.security_scores.lock().remove(&pid);
    }
}

// ============================================================================
// Security Statistics
// ============================================================================

/// Security event
#[derive(Debug, Clone)]
pub struct SecurityEvent {
    /// Event type
    pub event_type: String,
    /// Subject PID
    pub subject_pid: u32,
    /// Object ID
    pub object_id: u64,
    /// Operation
    pub operation: u32,
    /// Result
    pub result: LsmResult,
    /// Timestamp
    pub timestamp: u64,
}

/// Security statistics
pub struct SecurityStatistics {
    /// Total access checks
    total_checks: AtomicU64,
    /// Allowed accesses
    allowed: AtomicU64,
    /// Denied accesses
    denied: AtomicU64,
    /// Security events (circular buffer)
    events: Mutex<Vec<SecurityEvent>>,
    /// Maximum events to store
    max_events: usize,
}

impl SecurityStatistics {
    /// Create new statistics
    pub fn new(max_events: usize) -> Self {
        Self {
            total_checks: AtomicU64::new(0),
            allowed: AtomicU64::new(0),
            denied: AtomicU64::new(0),
            events: Mutex::new(Vec::with_capacity(max_events)),
            max_events,
        }
    }

    /// Record access check
    pub fn record_check(&self, result: LsmResult) {
        self.total_checks.fetch_add(1, Ordering::SeqCst);
        if result.is_allowed() {
            self.allowed.fetch_add(1, Ordering::SeqCst);
        } else {
            self.denied.fetch_add(1, Ordering::SeqCst);
        }
    }

    /// Log security event
    pub fn log_event(&self, event: SecurityEvent) {
        let mut events = self.events.lock();
        if events.len() >= self.max_events {
            events.remove(0);
        }
        events.push(event);
    }

    /// Get statistics
    pub fn get_stats(&self) -> SecurityStats {
        SecurityStats {
            total_checks: self.total_checks.load(Ordering::SeqCst),
            allowed: self.allowed.load(Ordering::SeqCst),
            denied: self.denied.load(Ordering::SeqCst),
            recent_events: self.events.lock().clone(),
        }
    }
}

/// Security statistics snapshot
#[derive(Debug, Clone)]
pub struct SecurityStats {
    /// Total access checks
    pub total_checks: u64,
    /// Allowed accesses
    pub allowed: u64,
    /// Denied accesses
    pub denied: u64,
    /// Recent security events
    pub recent_events: Vec<SecurityEvent>,
}

// ============================================================================
// Enforcement Manager
// ============================================================================

/// Security enforcement manager
pub struct SecurityEnforcementManager {
    /// Security module registry
    registry: SecurityModuleRegistry,
    /// Security contexts (indexed by PID)
    contexts: Mutex<BTreeMap<u32, SecurityContext>>,
    /// Security scheduler
    scheduler: SecurityScheduler,
    /// Security statistics
    statistics: SecurityStatistics,
    /// Enforcement enabled
    enabled: AtomicBool,
}

impl SecurityEnforcementManager {
    /// Create new enforcement manager
    pub fn new() -> Self {
        Self {
            registry: SecurityModuleRegistry::new(),
            contexts: Mutex::new(BTreeMap::new()),
            scheduler: SecurityScheduler::new(),
            statistics: SecurityStatistics::new(1000),
            enabled: AtomicBool::new(false),
        }
    }

    /// Initialize enforcement
    pub fn init(&self) -> Result<(), String> {
        self.enabled.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Check if enforcement is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled.load(Ordering::SeqCst)
    }

    /// Get module registry
    pub fn registry(&self) -> &SecurityModuleRegistry {
        &self.registry
    }

    /// Get scheduler
    pub fn scheduler(&self) -> &SecurityScheduler {
        &self.scheduler
    }

    /// Get statistics
    pub fn statistics(&self) -> &SecurityStatistics {
        &self.statistics
    }

    /// Register security context
    pub fn register_context(&self, context: SecurityContext) {
        let pid = context.pid();
        self.contexts.lock().insert(pid, context);
    }

    /// Unregister security context
    pub fn unregister_context(&self, pid: u32) {
        self.contexts.lock().remove(&pid);
        self.scheduler.remove_process(pid);
    }

    /// Get security context
    pub fn get_context(&self, pid: u32) -> Option<SecurityContext> {
        self.contexts.lock().get(&pid).cloned()
    }

    /// Update security context
    pub fn update_context(&self, pid: u32, f: impl FnOnce(&mut SecurityContext)) -> Result<(), String> {
        let mut contexts = self.contexts.lock();
        let context = contexts.get_mut(&pid).ok_or_else(|| format!("Context for PID {} not found", pid))?;
        f(context);
        Ok(())
    }

    /// Perform access check
    pub fn check_access(
        &self,
        hook_type: LsmHook,
        subject_pid: u32,
        object: &SecurityObject,
        operation: u32,
    ) -> LsmResult {
        if !self.is_enabled() {
            return LsmResult::Allow;
        }

        // Get subject context
        let subject = match self.get_context(subject_pid) {
            Some(ctx) => ctx,
            None => return LsmResult::Allow, // Default allow if no context
        };

        // Call all security modules
        let result = self.registry.call_hooks(hook_type, &subject, object, operation);

        // Record statistics
        self.statistics.record_check(result);

        // Log event
        self.statistics.log_event(SecurityEvent {
            event_type: format!("{:?}", hook_type),
            subject_pid,
            object_id: object.id(),
            operation,
            result,
            timestamp: 0, // Stub: would use actual timestamp
        });

        result
    }

    /// Calculate scheduling priority
    pub fn calculate_priority(&self, pid: u32, base_priority: i32) -> SecurityPriority {
        self.scheduler.calculate_priority(pid, base_priority)
    }

    /// Get comprehensive statistics
    pub fn get_enforcement_stats(&self) -> EnforcementStats {
        let module_stats = self.registry.get_stats();
        let security_stats = self.statistics.get_stats();
        let contexts = self.contexts.lock();

        EnforcementStats {
            enabled: self.is_enabled(),
            registered_modules: module_stats.total_modules,
            enabled_modules: module_stats.enabled_modules,
            tracked_contexts: contexts.len(),
            total_checks: security_stats.total_checks,
            allowed_accesses: security_stats.allowed,
            denied_accesses: security_stats.denied,
            allow_rate: if security_stats.total_checks > 0 {
                (security_stats.allowed as f64 / security_stats.total_checks as f64) * 100.0
            } else {
                0.0
            },
        }
    }
}

/// Comprehensive enforcement statistics
#[derive(Debug, Clone)]
pub struct EnforcementStats {
    /// Enforcement enabled
    pub enabled: bool,
    /// Number of registered modules
    pub registered_modules: usize,
    /// Number of enabled modules
    pub enabled_modules: usize,
    /// Number of tracked contexts
    pub tracked_contexts: usize,
    /// Total access checks
    pub total_checks: u64,
    /// Allowed accesses
    pub allowed_accesses: u64,
    /// Denied accesses
    pub denied_accesses: u64,
    /// Allow rate percentage
    pub allow_rate: f64,
}

// ============================================================================
// Global Enforcement Manager
// ============================================================================

use crate::subsystems::sync::lazy::Lazy;

static ENFORCEMENT_MANAGER: Lazy<spin::Mutex<SecurityEnforcementManager>> =
    Lazy::new(|| spin::Mutex::new(SecurityEnforcementManager::new()));

/// Get enforcement manager
pub fn enforcement_manager() -> &'static Lazy<spin::Mutex<SecurityEnforcementManager>> {
    &ENFORCEMENT_MANAGER
}

/// Initialize security enforcement
pub fn init_enforcement() -> Result<(), String> {
    ENFORCEMENT_MANAGER.lock().init()
}

/// Perform security check
pub fn security_check(
    hook_type: LsmHook,
    subject_pid: u32,
    object: &SecurityObject,
    operation: u32,
) -> LsmResult {
    ENFORCEMENT_MANAGER.lock().check_access(hook_type, subject_pid, object, operation)
}

/// Register security module
pub fn register_security_module(module: Box<dyn SecurityModule>) -> Result<(), String> {
    ENFORCEMENT_MANAGER.lock().registry().register(module)
}

/// Get enforcement statistics
pub fn get_enforcement_stats() -> EnforcementStats {
    ENFORCEMENT_MANAGER.lock().get_enforcement_stats()
}

/// Register security context
pub fn register_security_context(context: SecurityContext) {
    ENFORCEMENT_MANAGER.lock().register_context(context)
}

/// Unregister security context
pub fn unregister_security_context(pid: u32) {
    ENFORCEMENT_MANAGER.lock().unregister_context(pid)
}

/// Get security context
pub fn get_security_context(pid: u32) -> Option<SecurityContext> {
    ENFORCEMENT_MANAGER.lock().get_context(pid)
}

/// Calculate security-aware priority
pub fn calculate_security_priority(pid: u32, base_priority: i32) -> SecurityPriority {
    ENFORCEMENT_MANAGER.lock().calculate_priority(pid, base_priority)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lsm_result() {
        assert!(LsmResult::Allow.is_allowed());
        assert!(!LsmResult::Deny.is_allowed());
        assert_eq!(LsmResult::Allow.to_errno(), 0);
    }

    #[test]
    fn test_security_context() {
        let ctx = SecurityContext::new(1, 0, 0, "system_u:system_r:system_t".to_string());
        assert_eq!(ctx.pid(), 1);
        assert_eq!(ctx.uid(), 0);
        assert!(!ctx.has_capability(0x01));
    }

    #[test]
    fn test_security_object() {
        let mut obj = SecurityObject::new(SecurityObjectType::File, 12345);
        obj.set_label("system_u:object_r:file_t".to_string());
        assert_eq!(obj.object_type(), SecurityObjectType::File);
        assert_eq!(obj.id(), 12345);
    }

    #[test]
    fn test_security_priority() {
        let prio = SecurityPriority::new(100, 10);
        assert_eq!(prio.value(), 105); // base + boost
    }

    #[test]
    fn test_module_registry() {
        let registry = SecurityModuleRegistry::new();
        let module = Box::new(DummySecurityModule::new("test".to_string()));
        assert!(registry.register(module).is_ok());
        assert!(registry.is_enabled("test") == false);
        assert!(registry.enable("test").is_ok());
        assert!(registry.is_enabled("test"));
    }
}
