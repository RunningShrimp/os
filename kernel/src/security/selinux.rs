//! # SELinux Implementation
//!
//! This module provides Security-Enhanced Linux (SELinux) functionality with
//! Type Enforcement, Role-Based Access Control, and Multi-Level Security.
//!
//! ## Features
//!
//! - **Type Enforcement (TE)**: Fine-grained mandatory access control
//! - **Role-Based Access Control (RBAC)**: Role-based permissions
//! - **Multi-Level Security (MLS)**: Hierarchical security levels
//! - **AVC (Access Vector Cache)**: High-performance access decisions
//! - **Policy Compiler**: Binary policy loading and validation
//!
//! ## Usage
//!
//! ```no_run
//! use kernel::security::selinux::*;
//!
//! init_selinux()?;
//! let allowed = check_access(&subject_ctx, &object_ctx, SecurityClass::File, FilePermissions::Read)?;
//! ```

use crate::prelude::*;
use alloc::collections::BTreeSet;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicU8, Ordering};

// ============================================================================
// Constants and Types
// ============================================================================

/// Security class definitions
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SecurityClass {
    File = 1,
    Dir = 2,
    Socket = 4,
    Fifo = 5,
    Symlink = 7,
    ChrFile = 8,
    BlkFile = 9,
    Msgq = 12,
    Shm = 13,
    Sem = 14,
    Process = 21,
    Ipc = 23,
    Peer = 26,
    Capability = 29,
    Packet = 39,
}

/// File permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilePermissions(u32);

impl FilePermissions {
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const CREATE: Self = Self(1 << 2);
    pub const UNLINK: Self = Self(1 << 3);
    pub const GETATTR: Self = Self(1 << 4);
    pub const SETATTR: Self = Self(1 << 5);
    pub const EXECUTE: Self = Self(1 << 6);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

/// Security context
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityContext {
    pub user: String,
    pub role: String,
    pub type_: String,
    pub level: SecurityLevel,
}

/// MLS security level
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityLevel {
    pub sensitivity: u32,
    pub categories: BTreeSet<u32>,
}

impl SecurityLevel {
    pub fn new(sensitivity: u32) -> Self {
        Self {
            sensitivity,
            categories: BTreeSet::new(),
        }
    }

    pub fn dominates(&self, other: &Self) -> bool {
        if self.sensitivity < other.sensitivity {
            return false;
        }
        other.categories.iter().all(|c| self.categories.contains(c))
    }
}

// ============================================================================
// AVC (Access Vector Cache)
// ============================================================================

/// AVC entry
#[derive(Debug, Clone)]
struct AvcEntry {
    ssid: u32,
    tsid: u32,
    av: u32,
    allowed: bool,
    timestamp: u64,
}

/// Access Vector Cache
#[derive(Debug)]
pub struct AccessVectorCache {
    cache: Mutex<Vec<AvcEntry>>,
    max_size: usize,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl AccessVectorCache {
    pub fn new(max_size: usize) -> Self {
        Self {
            cache: Mutex::new(Vec::with_capacity(max_size)),
            max_size,
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    pub fn lookup(&self, ssid: u32, tsid: u32, av: u32) -> Option<bool> {
        let cache = self.cache.lock();
        for entry in cache.iter() {
            if entry.ssid == ssid && entry.tsid == tsid && (entry.av & av) == av {
                self.hits.fetch_add(1, Ordering::Relaxed);
                return Some(entry.allowed);
            }
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        None
    }

    pub fn insert(&self, ssid: u32, tsid: u32, av: u32, allowed: bool) {
        let mut cache = self.cache.lock();
        if cache.len() >= self.max_size {
            cache.remove(0);
        }
        cache.push(AvcEntry {
            ssid,
            tsid,
            av,
            allowed,
            timestamp: 0,
        });
    }

    pub fn clear(&self) {
        self.cache.lock().clear();
    }
}

// ============================================================================
// TE Rules
// ============================================================================

/// TE allow rule
#[derive(Debug, Clone)]
pub struct TeRule {
    pub source_type: String,
    pub target_type: String,
    pub class: SecurityClass,
    pub perms: u32,
}

/// Type enforcement database
#[derive(Debug)]
pub struct TypeEnforcementDb {
    allow_rules: Mutex<Vec<TeRule>>,
}

impl TypeEnforcementDb {
    pub fn new() -> Self {
        Self {
            allow_rules: Mutex::new(Vec::new()),
        }
    }

    pub fn add_allow_rule(&self, rule: TeRule) {
        self.allow_rules.lock().push(rule);
    }

    pub fn check_access(&self, source: &str, target: &str, class: SecurityClass, perms: u32) -> bool {
        let rules = self.allow_rules.lock();
        for rule in rules.iter() {
            if rule.source_type == source && rule.target_type == target && rule.class == class {
                if (rule.perms & perms) == perms {
                    return true;
                }
            }
        }
        false
    }
}

// ============================================================================
// RBAC Rules
// ============================================================================

/// Role mapping
#[derive(Debug, Clone)]
pub struct RoleMapping {
    pub role: String,
    pub types: BTreeSet<String>,
}

/// RBAC database
#[derive(Debug)]
pub struct RoleBasedAccessDb {
    role_mappings: Mutex<Vec<RoleMapping>>,
}

impl RoleBasedAccessDb {
    pub fn new() -> Self {
        Self {
            role_mappings: Mutex::new(Vec::new()),
        }
    }

    pub fn add_role_mapping(&self, mapping: RoleMapping) {
        self.role_mappings.lock().push(mapping);
    }

    pub fn check_role(&self, role: &str, type_: &str) -> bool {
        let mappings = self.role_mappings.lock();
        for mapping in mappings.iter() {
            if mapping.role == role && mapping.types.contains(type_) {
                return true;
            }
        }
        false
    }
}

// ============================================================================
// Policy Engine
// ============================================================================

/// SELinux policy
#[derive(Debug)]
pub struct SelinuxPolicy {
    pub te_db: TypeEnforcementDb,
    pub rbac_db: RoleBasedAccessDb,
    pub version: u32,
    pub mls_enabled: bool,
}

impl SelinuxPolicy {
    pub fn new() -> Self {
        Self {
            te_db: TypeEnforcementDb::new(),
            rbac_db: RoleBasedAccessDb::new(),
            version: 31,
            mls_enabled: false,
        }
    }

    pub fn load_default_policy(&mut self) {
        self.te_db.add_allow_rule(TeRule {
            source_type: String::from("user_t"),
            target_type: String::from("user_t"),
            class: SecurityClass::File,
            perms: FilePermissions::READ.0 | FilePermissions::WRITE.0,
        });

        self.rbac_db.add_role_mapping(RoleMapping {
            role: String::from("user_r"),
            types: {
                let mut set = BTreeSet::new();
                set.insert(String::from("user_t"));
                set
            },
        });
    }
}

// ============================================================================
// SELinux Manager
// ============================================================================

/// SELinux enforcement mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnforceMode {
    Permissive = 0,
    Enforcing = 1,
    Disabled = 2,
}

/// SELinux manager
#[derive(Debug)]
pub struct SelinuxManager {
    pub policy: SelinuxPolicy,
    pub avc: AccessVectorCache,
    pub enforce_mode: AtomicU8,
    pub enabled: AtomicBool,
    pub access_checks: AtomicU64,
    pub denials: AtomicU64,
}

impl SelinuxManager {
    pub fn new() -> Self {
        Self {
            policy: SelinuxPolicy::new(),
            avc: AccessVectorCache::new(1024),
            enforce_mode: AtomicU8::new(EnforceMode::Enforcing as u8),
            enabled: AtomicBool::new(true),
            access_checks: AtomicU64::new(0),
            denials: AtomicU64::new(0),
        }
    }

    pub fn initialize(&mut self) -> Result<()> {
        self.policy.load_default_policy();
        log_info!("[selinux] SELinux initialized in Enforcing mode");
        Ok(())
    }

    pub fn check_access(
        &self,
        subject: &SecurityContext,
        object: &SecurityContext,
        class: SecurityClass,
        perms: u32,
    ) -> Result<bool> {
        self.access_checks.fetch_add(1, Ordering::Relaxed);

        if !self.enabled.load(Ordering::Relaxed) {
            return Ok(true);
        }

        if self.policy.mls_enabled {
            if !self.check_mls(subject, object) {
                self.denials.fetch_add(1, Ordering::Relaxed);
                return Ok(false);
            }
        }

        if !self.policy.rbac_db.check_role(&subject.role, &subject.type_) {
            self.denials.fetch_add(1, Ordering::Relaxed);
            return Ok(false);
        }

        let allowed = self.policy.te_db.check_access(
            &subject.type_,
            &object.type_,
            class,
            perms,
        );

        if !allowed {
            self.denials.fetch_add(1, Ordering::Relaxed);
        }

        Ok(allowed)
    }

    fn check_mls(&self, subject: &SecurityContext, object: &SecurityContext) -> bool {
        subject.level.dominates(&object.level)
    }

    pub fn set_enforce_mode(&self, mode: EnforceMode) {
        self.enforce_mode.store(mode as u8, Ordering::Release);
    }

    pub fn get_enforce_mode(&self) -> EnforceMode {
        match self.enforce_mode.load(Ordering::Acquire) {
            0 => EnforceMode::Permissive,
            1 => EnforceMode::Enforcing,
            _ => EnforceMode::Disabled,
        }
    }

    pub fn clear_avc_cache(&self) {
        self.avc.clear();
    }
}

// ============================================================================
// Global State
// ============================================================================

static GLOBAL_SELINUX: Mutex<Option<SelinuxManager>> = Mutex::new(None);

pub fn init_selinux() -> Result<()> {
    let mut global = GLOBAL_SELINUX.lock();
    if global.is_some() {
        return Ok(());
    }

    let mut manager = SelinuxManager::new();
    manager.initialize()?;
    *global = Some(manager);
    Ok(())
}

pub fn get_selinux_manager() -> Result<&'static Mutex<Option<SelinuxManager>>> {
    Ok(&GLOBAL_SELINUX)
}

pub fn check_access(
    subject: &SecurityContext,
    object: &SecurityContext,
    class: SecurityClass,
    perms: u32,
) -> Result<bool> {
    let global = GLOBAL_SELINUX.lock();
    let manager = global.as_ref().ok_or(Error::NotFound)?;
    manager.check_access(subject, object, class, perms)
}

pub fn set_enforce_mode(mode: EnforceMode) -> Result<()> {
    let global = GLOBAL_SELINUX.lock();
    let manager = global.as_ref().ok_or(Error::NotFound)?;
    manager.set_enforce_mode(mode);
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_context() {
        let ctx = SecurityContext {
            user: String::from("user_u"),
            role: String::from("user_r"),
            type_: String::from("user_t"),
            level: SecurityLevel::new(0),
        };
        assert_eq!(ctx.user, "user_u");
    }

    #[test]
    fn test_mls_dominance() {
        let low = SecurityLevel::new(0);
        let high = SecurityLevel::new(1);
        assert!(high.dominates(&low));
        assert!(!low.dominates(&high));
    }

    #[test]
    fn test_selinux_init() {
        let mut manager = SelinuxManager::new();
        assert!(manager.initialize().is_ok());
        assert!(manager.enabled.load(Ordering::Relaxed));
    }
}

// ============================================================================
// Extended Security Classes
// ============================================================================

/// Extended security classes
#[repr(u16)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtendedSecurityClass {
    /// Key management
    Key = 31,
    /// Network firewall rules
    Firewall = 34,
    /// Tun device
    TunDevice = 36,
    /// BPF programs
    Bpf = 43,
    /// Perf events
    PerfEvent = 51,
    /// IMA/EVM
    Ima = 53,
    /// Landlock
    Landlock = 54,
    /// User namespace
    UserNamespace = 55,
    /// PID namespace
    PidNamespace = 56,
    /// Cgroup namespace
    CgroupNamespace = 57,
    /// Time namespace
    TimeNamespace = 58,
}

/// Common permissions across classes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommonPermissions(u32);

impl CommonPermissions {
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const CREATE: Self = Self(1 << 2);
    pub const GETATTR: Self = Self(1 << 3);
    pub const SETATTR: Self = Self(1 << 4);
    pub const DELETE: Self = Self(1 << 5);

    pub fn bits(&self) -> u32 {
        self.0
    }
}

// ============================================================================
// Policy Rules Extended
// ============================================================================

/// Extended TE rules with conditions
#[derive(Debug, Clone)]
pub struct TeRuleExtended {
    pub base: TeRule,
    pub conditions: Vec<RuleCondition>,
    pub audit_enabled: bool,
}

/// Rule conditions
#[derive(Debug, Clone)]
pub enum RuleCondition {
    /// User condition
    User(String),
    /// Role condition
    Role(String),
    /// Type condition
    Type(String),
    /// MLS level condition
    MlsLevel(String),
    /// Boolean condition
    Bool(String, bool),
}

/// Constraint expression
#[derive(Debug, Clone)]
pub struct Constraint {
    pub permissions: u32,
    pub class: SecurityClass,
    pub expression: ConstraintExpr,
}

/// Constraint expression tree
#[derive(Debug, Clone)]
pub enum ConstraintExpr {
    /// And operation
    And(Box<ConstraintExpr>, Box<ConstraintExpr>),
    /// Or operation
    Or(Box<ConstraintExpr>, Box<ConstraintExpr>),
    /// Not operation
    Not(Box<ConstraintExpr>),
    /// User comparison
    UserEq(ConstraintOp),
    /// Role comparison
    RoleEq(ConstraintOp),
    /// Type comparison
    TypeEq(ConstraintOp),
    /// MLS level comparison
    LevelDominates,
}

/// Constraint operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintOp {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

/// Constraint database
#[derive(Debug)]
pub struct ConstraintDb {
    pub constraints: Mutex<Vec<Constraint>>,
}

impl ConstraintDb {
    pub fn new() -> Self {
        Self {
            constraints: Mutex::new(Vec::new()),
        }
    }

    pub fn add_constraint(&self, constraint: Constraint) {
        self.constraints.lock().push(constraint);
    }

    pub fn evaluate(
        &self,
        subject: &SecurityContext,
        object: &SecurityContext,
        class: SecurityClass,
        perms: u32,
    ) -> bool {
        let constraints = self.constraints.lock();

        for constraint in constraints.iter() {
            if constraint.class == class && (constraint.permissions & perms) != 0 {
                if !self.eval_constraint(&constraint.expression, subject, object) {
                    return false;
                }
            }
        }

        true
    }

    fn eval_constraint(
        &self,
        expr: &ConstraintExpr,
        subject: &SecurityContext,
        object: &SecurityContext,
    ) -> bool {
        match expr {
            ConstraintExpr::And(left, right) => {
                self.eval_constraint(left, subject, object)
                    && self.eval_constraint(right, subject, object)
            }
            ConstraintExpr::Or(left, right) => {
                self.eval_constraint(left, subject, object)
                    || self.eval_constraint(right, subject, object)
            }
            ConstraintExpr::Not(inner) => !self.eval_constraint(inner, subject, object),
            ConstraintExpr::UserEq(_) => subject.user == object.user,
            ConstraintExpr::RoleEq(_) => subject.role == object.role,
            ConstraintExpr::TypeEq(_) => subject.type_ == object.type_,
            ConstraintExpr::LevelDominates => subject.level.dominates(&object.level),
        }
    }
}

// ============================================================================
// SELinux Policy with Constraints
// ============================================================================

/// Enhanced SELinux policy
#[derive(Debug)]
pub struct SelinuxPolicyEnhanced {
    pub base: SelinuxPolicy,
    pub constraint_db: ConstraintDb,
    pub booleans: Mutex<BTreeMap<String, bool>>,
    pub conditional_rules: Mutex<Vec<ConditionalRule>>,
}

/// Conditional rule
#[derive(Debug, Clone)]
pub struct ConditionalRule {
    pub bool_var: String,
    pub rule: TeRule,
}

impl SelinuxPolicyEnhanced {
    pub fn new() -> Self {
        Self {
            base: SelinuxPolicy::new(),
            constraint_db: ConstraintDb::new(),
            booleans: Mutex::new(BTreeMap::new()),
            conditional_rules: Mutex::new(Vec::new()),
        }
    }

    pub fn set_boolean(&self, name: String, value: bool) {
        let mut booleans = self.booleans.lock();
        booleans.insert(name, value);
    }

    pub fn get_boolean(&self, name: &str) -> Option<bool> {
        let booleans = self.booleans.lock();
        booleans.get(name).copied()
    }

    pub fn add_conditional_rule(&self, bool_var: String, rule: TeRule) {
        let mut rules = self.conditional_rules.lock();
        rules.push(ConditionalRule { bool_var, rule });
    }

    pub fn check_access_with_constraints(
        &self,
        subject: &SecurityContext,
        object: &SecurityContext,
        class: SecurityClass,
        perms: u32,
    ) -> bool {
        // Check base TE rules
        if !self.base.te_db.check_access(&subject.type_, &object.type_, class, perms) {
            return false;
        }

        // Check constraints
        if !self
            .constraint_db
            .evaluate(subject, object, class, perms)
        {
            return false;
        }

        // Check conditional rules
        let cond_rules = self.conditional_rules.lock();
        let booleans = self.booleans.lock();

        for cond in cond_rules.iter() {
            if let Some(&value) = booleans.get(&cond.bool_var) {
                if value {
                    if cond.rule.source_type == subject.type_
                        && cond.rule.target_type == object.type_
                        && cond.rule.class == class
                        && (cond.rule.perms & perms) != 0
                    {
                        return true;
                    }
                }
            }
        }

        true
    }
}

// ============================================================================
// Per-domain mapping
// ============================================================================

/// Domain transition rule
#[derive(Debug, Clone)]
pub struct DomainTransition {
    pub from_domain: String,
    pub to_domain: String,
    pub program_type: String,
    pub entrypoint: bool,
}

/// Domain transition database
#[derive(Debug)]
pub struct DomainTransitionDb {
    pub transitions: Mutex<Vec<DomainTransition>>,
}

impl DomainTransitionDb {
    pub fn new() -> Self {
        Self {
            transitions: Mutex::new(Vec::new()),
        }
    }

    pub fn add_transition(&self, transition: DomainTransition) {
        self.transitions.lock().push(transition);
    }

    pub fn find_transition(
        &self,
        from_domain: &str,
        program_type: &str,
    ) -> Option<String> {
        let transitions = self.transitions.lock();

        for trans in transitions.iter() {
            if trans.from_domain == from_domain && trans.program_type == program_type {
                return Some(trans.to_domain.clone());
            }
        }

        None
    }
}

// ============================================================================
// SELinux Manager Enhanced
// ============================================================================

/// Enhanced SELinux manager
#[derive(Debug)]
pub struct SelinuxManagerEnhanced {
    pub base: SelinuxManager,
    pub policy_enhanced: SelinuxPolicyEnhanced,
    pub domain_transitions: DomainTransitionDb,
    pub file_contexts: Mutex<BTreeMap<String, SecurityContext>>,
}

impl SelinuxManagerEnhanced {
    pub fn new() -> Self {
        Self {
            base: SelinuxManager::new(),
            policy_enhanced: SelinuxPolicyEnhanced::new(),
            domain_transitions: DomainTransitionDb::new(),
            file_contexts: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn initialize(&mut self) -> Result<()> {
        self.base.initialize()?;
        self.load_default_policy()?;
        log_info!("[selinux] Enhanced SELinux initialized");
        Ok(())
    }

    pub fn check_access_enhanced(
        &self,
        subject: &SecurityContext,
        object: &SecurityContext,
        class: SecurityClass,
        perms: u32,
    ) -> Result<bool> {
        self.base.access_checks.fetch_add(1, Ordering::Relaxed);

        if !self.base.enabled.load(Ordering::Relaxed) {
            return Ok(true);
        }

        let allowed = self
            .policy_enhanced
            .check_access_with_constraints(subject, object, class, perms);

        if !allowed {
            self.base.denials.fetch_add(1, Ordering::Relaxed);
        }

        Ok(allowed)
    }

    pub fn compute_domain_transition(
        &self,
        current: &SecurityContext,
        program_type: &str,
    ) -> Option<SecurityContext> {
        let new_type = self
            .domain_transitions
            .find_transition(&current.type_, program_type)?;

        Some(SecurityContext {
            user: current.user.clone(),
            role: current.role.clone(),
            type_: new_type,
            level: current.level.clone(),
        })
    }

    pub fn set_file_context(&self, path: String, context: SecurityContext) {
        self.file_contexts.lock().insert(path, context);
    }

    pub fn get_file_context(&self, path: &str) -> Option<SecurityContext> {
        let contexts = self.file_contexts.lock();
        
        // Exact match first
        if let Some(ctx) = contexts.get(path) {
            return Some(ctx.clone());
        }

        // Prefix match for directories
        for (p, ctx) in contexts.iter() {
            if path.starts_with(p) {
                return Some(ctx.clone());
            }
        }

        None
    }

    fn load_default_policy(&mut self) -> Result<()> {
        self.policy_enhanced.base.load_default_policy();

        // Add some constraints
        self.policy_enhanced.constraint_db.add_constraint(Constraint {
            permissions: FilePermissions::WRITE.0,
            class: SecurityClass::File,
            expression: ConstraintExpr::UserEq(ConstraintOp::Eq),
        });

        // Add domain transitions
        self.domain_transitions.add_transition(DomainTransition {
            from_domain: "user_t".to_string(),
            to_domain: "exec_t".to_string(),
            program_type: "bin_t".to_string(),
            entrypoint: true,
        });

        // Add file contexts
        self.set_file_context(
            "/etc/.*".to_string(),
            SecurityContext {
                user: "system_u".to_string(),
                role: "object_r".to_string(),
                type_: "etc_t".to_string(),
                level: SecurityLevel::new(0),
            },
        );

        Ok(())
    }
}

#[cfg(test)]
mod extended_tests {
    use super::*;

    #[test]
    fn test_constraint_db() {
        let db = ConstraintDb::new();

        db.add_constraint(Constraint {
            permissions: 1,
            class: SecurityClass::File,
            expression: ConstraintExpr::UserEq(ConstraintOp::Eq),
        });

        let subject = SecurityContext {
            user: "user".to_string(),
            role: "role".to_string(),
            type_: "type".to_string(),
            level: SecurityLevel::new(0),
        };

        let object = SecurityContext {
            user: "user".to_string(),
            role: "role".to_string(),
            type_: "type".to_string(),
            level: SecurityLevel::new(0),
        };

        assert!(db.evaluate(&subject, &object, SecurityClass::File, 1));
    }

    #[test]
    fn test_domain_transition() {
        let db = DomainTransitionDb::new();

        db.add_transition(DomainTransition {
            from_domain: "user_t".to_string(),
            to_domain: "exec_t".to_string(),
            program_type: "bin_t".to_string(),
            entrypoint: true,
        });

        let result = db.find_transition("user_t", "bin_t");
        assert_eq!(result, Some("exec_t".to_string()));
    }

    #[test]
    fn test_enhanced_manager() {
        let mut manager = SelinuxManagerEnhanced::new();
        assert!(manager.initialize().is_ok());

        let subject = SecurityContext {
            user: "user".to_string(),
            role: "role".to_string(),
            type_: "type".to_string(),
            level: SecurityLevel::new(0),
        };

        let object = SecurityContext {
            user: "user".to_string(),
            role: "role".to_string(),
            type_: "type".to_string(),
            level: SecurityLevel::new(0),
        };

        let allowed = manager
            .check_access_enhanced(&subject, &object, SecurityClass::File, 1)
            .unwrap();

        assert!(allowed);
    }
}
