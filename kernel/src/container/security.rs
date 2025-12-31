// Container Security Module
//
// 容器安全模块
// 提供镜像扫描、运行时安全、Seccomp/AppArmor配置和访问控制
//
// This module implements:
// - Image vulnerability scanning
// - Runtime security monitoring
// - Seccomp filter profiles
// - AppArmor profiles
// - Capability management
// - Rootless container support
// - Security context enforcement

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::reliability::{EIO, ENOENT, EPERM};

/// Vulnerability severity
///
/// 漏洞严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Critical
    Critical,
    /// High
    High,
    /// Medium
    Medium,
    /// Low
    Low,
    /// Unknown
    Unknown,
}

/// Vulnerability
///
/// 漏洞
#[derive(Debug, Clone)]
pub struct Vulnerability {
    /// CVE ID
    pub cve_id: String,
    /// Severity
    pub severity: Severity,
    /// Description
    pub description: String,
    /// Affected package
    pub package: String,
    /// Fixed version
    pub fixed_version: Option<String>,
    /// Link to advisory
    pub link: Option<String>,
}

/// Image scan result
///
/// 镜像扫描结果
#[derive(Debug, Clone)]
pub struct ImageScanResult {
    /// Image ID
    pub image_id: String,
    /// Scan timestamp
    pub timestamp: u64,
    /// Vulnerabilities found
    pub vulnerabilities: Vec<Vulnerability>,
    /// Passed
    pub passed: bool,
    /// Score (0-100)
    pub score: u32,
}

impl ImageScanResult {
    /// Calculate compliance score
    pub fn calculate_score(&mut self) {
        let mut score = 100u32;

        for vuln in &self.vulnerabilities {
            match vuln.severity {
                Severity::Critical => score = score.saturating_sub(20),
                Severity::High => score = score.saturating_sub(10),
                Severity::Medium => score = score.saturating_sub(5),
                Severity::Low => score = score.saturating_sub(1),
                Severity::Unknown => {}
            }
        }

        self.score = score;
    }

    /// Check if scan passed
    pub fn check_passed(&mut self, policy: &SecurityPolicy) {
        self.passed = policy.verify_scan_result(self);
    }
}

/// Security policy
///
/// 安全策略
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// Policy name
    pub name: String,
    /// Require signed images
    pub require_signed: bool,
    /// Allowed registries
    pub allowed_registries: Vec<String>,
    /// Maximum severity level
    pub max_severity: Severity,
    /// Forbidden capabilities
    pub forbidden_capabilities: Vec<String>,
    /// Required capabilities
    pub required_capabilities: Vec<String>,
    /// Allow privilege escalation
    pub allow_privilege_escalation: bool,
    /// Require non-root
    pub require_non_root: bool,
    /// Read-only root filesystem
    pub read_only_root_filesystem: bool,
}

impl SecurityPolicy {
    /// Verify scan result against policy
    pub fn verify_scan_result(&self, result: &ImageScanResult) -> bool {
        for vuln in &result.vulnerabilities {
            if vuln.severity <= self.max_severity {
                return false;
            }
        }
        true
    }

    /// Validate security context
    pub fn validate_security_context(&self, ctx: &SecurityContext) -> bool {
        // Check if running as root when not allowed
        if self.require_non_root && ctx.run_as_user == 0 {
            return false;
        }

        // Check privilege escalation
        if !self.allow_privilege_escalation && ctx.allow_privilege_escalation {
            return false;
        }

        // Check forbidden capabilities
        for cap in &self.forbidden_capabilities {
            if ctx.capabilities.contains(cap) {
                return false;
            }
        }

        // Check required capabilities
        for cap in &self.required_capabilities {
            if !ctx.capabilities.contains(cap) {
                return false;
            }
        }

        true
    }
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            require_signed: false,
            allowed_registries: vec!["docker.io".to_string(), "gcr.io".to_string()],
            max_severity: Severity::High,
            forbidden_capabilities: vec!["CAP_SYS_ADMIN".to_string(), "CAP_SYS_MODULE".to_string()],
            required_capabilities: Vec::new(),
            allow_privilege_escalation: false,
            require_non_root: true,
            read_only_root_filesystem: false,
        }
    }
}

/// Security context
///
/// 安全上下文
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Run as user
    pub run_as_user: u32,
    /// Run as group
    pub run_as_group: u32,
    /// Run as non-root
    pub run_as_non_root: bool,
    /// Capabilities
    pub capabilities: Vec<String>,
    /// Privileged
    pub privileged: bool,
    /// Allow privilege escalation
    pub allow_privilege_escalation: bool,
    /// Read-only root filesystem
    pub read_only_root_filesystem: bool,
    /// Seccomp profile
    pub seccomp_profile: Option<String>,
    /// AppArmor profile
    pub apparmor_profile: Option<String>,
    /// SELinux options
    pub selinux_options: Option<SelinuxOptions>,
}

/// SELinux options
#[derive(Debug, Clone)]
pub struct SelinuxOptions {
    /// User
    pub user: String,
    /// Role
    pub role: String,
    /// Type
    pub typ: String,
    /// Level
    pub level: String,
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self {
            run_as_user: 0,
            run_as_group: 0,
            run_as_non_root: false,
            capabilities: Vec::new(),
            privileged: false,
            allow_privilege_escalation: false,
            read_only_root_filesystem: false,
            seccomp_profile: None,
            apparmor_profile: None,
            selinux_options: None,
        }
    }
}

/// Seccomp action
///
/// Seccomp动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompAction {
    /// Allow
    Allow,
    /// Kill
    Kill,
    /// Trap
    Trap,
    /// Errno
    Errno,
    /// Trace
    Trace,
}

/// Seccomp rule
///
/// Seccomp规则
#[derive(Debug, Clone)]
pub struct SeccompRule {
    /// Syscall names
    pub names: Vec<String>,
    /// Action
    pub action: SeccompAction,
    /// Args
    pub args: Vec<SeccompArg>,
}

/// Seccomp argument
#[derive(Debug, Clone)]
pub struct SeccompArg {
    /// Index
    pub index: u32,
    /// Value
    pub value: u64,
    /// Value two
    pub value_two: Option<u64>,
    /// Operator
    pub operator: SeccompOperator,
}

/// Seccomp operator
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompOperator {
    /// Not equal
    NotEqual,
    /// Less than
    LessThan,
    /// Less than or equal
    LessThanOrEqual,
    /// Equal to
    EqualTo,
    /// Greater than or equal
    GreaterThanOrEqual,
    /// Greater than
    GreaterThan,
    /// Masked equal
    MaskedEqual,
}

/// Seccomp profile
///
/// Seccomp配置文件
#[derive(Debug, Clone)]
pub struct SeccompProfile {
    /// Profile name
    pub name: String,
    /// Default action
    pub default_action: SeccompAction,
    /// Architectures
    pub architectures: Vec<String>,
    /// Syscalls
    pub syscalls: Vec<SeccompRule>,
    /// Flags
    pub flags: Vec<String>,
}

impl SeccompProfile {
    /// Create default profile
    pub fn default_profile() -> Self {
        Self {
            name: "default".to_string(),
            default_action: SeccompAction::Errno,
            architectures: vec!["SCMP_ARCH_X86_64".to_string(), "SCMP_ARCH_X86".to_string()],
            syscalls: vec![
                SeccompRule {
                    names: vec![
                        "read".to_string(),
                        "write".to_string(),
                        "open".to_string(),
                        "close".to_string(),
                        "stat".to_string(),
                        "fstat".to_string(),
                        "lstat".to_string(),
                        "poll".to_string(),
                        "lseek".to_string(),
                        "mmap".to_string(),
                        "mprotect".to_string(),
                        "munmap".to_string(),
                        "brk".to_string(),
                        "rt_sigaction".to_string(),
                        "rt_sigprocmask".to_string(),
                        "rt_sigreturn".to_string(),
                        "ioctl".to_string(),
                        "pread64".to_string(),
                        "pwrite64".to_string(),
                        "readv".to_string(),
                        "writev".to_string(),
                        "access".to_string(),
                        "pipe".to_string(),
                        "select".to_string(),
                        "sched_yield".to_string(),
                        "mremap".to_string(),
                        "msync".to_string(),
                        "mincore".to_string(),
                        "madvise".to_string(),
                        "dup".to_string(),
                        "dup2".to_string(),
                        "pause".to_string(),
                        "nanosleep".to_string(),
                        "getitimer".to_string(),
                        "alarm".to_string(),
                        "setitimer".to_string(),
                        "getpid".to_string(),
                        "sendfile".to_string(),
                        "socket".to_string(),
                        "connect".to_string(),
                        "accept".to_string(),
                        "sendto".to_string(),
                        "recvfrom".to_string(),
                        "sendmsg".to_string(),
                        "recvmsg".to_string(),
                        "shutdown".to_string(),
                        "bind".to_string(),
                        "listen".to_string(),
                        "getsockname".to_string(),
                        "getpeername".to_string(),
                        "socketpair".to_string(),
                        "setsockopt".to_string(),
                        "getsockopt".to_string(),
                        "clone".to_string(),
                        "fork".to_string(),
                        "vfork".to_string(),
                        "execve".to_string(),
                        "exit".to_string(),
                        "wait4".to_string(),
                        "kill".to_string(),
                        "uname".to_string(),
                    ],
                    action: SeccompAction::Allow,
                    args: Vec::new(),
                },
            ],
            flags: Vec::new(),
        }
    }

    /// Create runtime profile
    pub fn runtime_profile() -> Self {
        Self {
            name: "runtime".to_string(),
            default_action: SeccompAction::Errno,
            architectures: vec!["SCMP_ARCH_X86_64".to_string()],
            syscalls: vec![
                SeccompRule {
                    names: vec![
                        "clone".to_string(),
                        "execve".to_string(),
                        "execveat".to_string(),
                    ],
                    action: SeccompAction::Allow,
                    args: vec![],
                },
            ],
            flags: vec!["SECCOMP_FILTER_FLAG_TSYNC".to_string()],
        }
    }
}

/// AppArmor profile
///
/// AppArmor配置文件
#[derive(Debug, Clone)]
pub struct AppArmorProfile {
    /// Profile name
    pub name: String,
    /// Profile mode
    pub mode: AppArmorMode,
    /// Path rules
    pub path_rules: Vec<AppArmorPathRule>,
    /// Capability rules
    pub capability_rules: Vec<String>,
}

/// AppArmor mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppArmorMode {
    /// Enforce
    Enforce,
    /// Complain
    Complain,
    /// Unconfined
    Unconfined,
}

/// AppArmor path rule
#[derive(Debug, Clone)]
pub struct AppArmorPathRule {
    /// Path
    pub path: String,
    /// Permissions
    pub permissions: String,
}

impl AppArmorProfile {
    /// Create default profile
    pub fn default_profile() -> Self {
        Self {
            name: "docker-default".to_string(),
            mode: AppArmorMode::Enforce,
            path_rules: vec![
                AppArmorPathRule {
                    path: "/".to_string(),
                    permissions: "r".to_string(),
                },
                AppArmorPathRule {
                    path: "/proc/**".to_string(),
                    permissions: "r".to_string(),
                },
                AppArmorPathRule {
                    path: "/sys/**".to_string(),
                    permissions: "r".to_string(),
                },
                AppArmorPathRule {
                    path: "/dev/null".to_string(),
                    permissions: "rw".to_string(),
                },
                AppArmorPathRule {
                    path: "/dev/zero".to_string(),
                    permissions: "rw".to_string(),
                },
                AppArmorPathRule {
                    path: "/dev/full".to_string(),
                    permissions: "rw".to_string(),
                },
                AppArmorPathRule {
                    path: "/dev/tty".to_string(),
                    permissions: "rw".to_string(),
                },
                AppArmorPathRule {
                    path: "/dev/urandom".to_string(),
                    permissions: "r".to_string(),
                },
                AppArmorPathRule {
                    path: "/dev/random".to_string(),
                    permissions: "r".to_string(),
                },
            ],
            capability_rules: vec![
                "chown".to_string(),
                "dac_override".to_string(),
                "fowner".to_string(),
                "fsetid".to_string(),
                "kill".to_string(),
                "setgid".to_string(),
                "setuid".to_string(),
                "setpcap".to_string(),
                "net_bind_service".to_string(),
                "net_raw".to_string(),
                "sys_chroot".to_string(),
                "mknod".to_string(),
                "setfcap".to_string(),
            ],
        }
    }
}

/// Runtime security event
///
/// 运行时安全事件
#[derive(Debug, Clone)]
pub struct SecurityEvent {
    /// Event ID
    pub id: String,
    /// Timestamp
    pub timestamp: u64,
    /// Container ID
    pub container_id: String,
    /// Event type
    pub event_type: SecurityEventType,
    /// Severity
    pub severity: Severity,
    /// Description
    pub description: String,
    /// Details
    pub details: BTreeMap<String, String>,
}

/// Security event type
///
/// 安全事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityEventType {
    /// Suspicious process
    SuspiciousProcess,
    /// File system access
    FilesystemAccess,
    /// Network activity
    NetworkActivity,
    /// System call violation
    SyscallViolation,
    /// Privilege escalation
    PrivilegeEscalation,
    /// Container escape attempt
    ContainerEscape,
}

/// Image scanner
///
/// 镜像扫描器
pub struct ImageScanner {
    /// Scan results cache
    scan_cache: BTreeMap<String, ImageScanResult>,
    /// Vulnerability database
    vuln_db: BTreeMap<String, Vec<Vulnerability>>,
    /// Next scan ID
    next_scan_id: AtomicU64,
}

impl ImageScanner {
    /// Create new image scanner
    pub fn new() -> Self {
        Self {
            scan_cache: BTreeMap::new(),
            vuln_db: BTreeMap::new(),
            next_scan_id: AtomicU64::new(1),
        }
    }

    /// Scan image
    pub fn scan_image(&mut self, image_id: &str) -> Result<ImageScanResult, SecurityError> {
        crate::println!("[security] Scanning image {}", image_id);

        // In real implementation, scan image layers and package databases
        let vulnerabilities = self.check_vulnerabilities(image_id)?;

        let mut result = ImageScanResult {
            image_id: image_id.to_string(),
            timestamp: self.current_time(),
            vulnerabilities,
            passed: false,
            score: 0,
        };

        result.calculate_score();

        self.scan_cache.insert(image_id.to_string(), result.clone());

        Ok(result)
    }

    /// Get cached scan result
    pub fn get_scan_result(&self, image_id: &str) -> Option<&ImageScanResult> {
        self.scan_cache.get(image_id)
    }

    /// Check vulnerabilities
    fn check_vulnerabilities(&self, _image_id: &str) -> Result<Vec<Vulnerability>, SecurityError> {
        // In real implementation, query vulnerability database
        Ok(Vec::new())
    }

    /// Current time
    fn current_time(&self) -> u64 {
        crate::subsystems::time::rdtsc() as u64
    }
}

/// Security manager
///
/// 安全管理器
pub struct SecurityManager {
    /// Image scanner
    scanner: ImageScanner,
    /// Security policies
    policies: BTreeMap<String, SecurityPolicy>,
    /// Seccomp profiles
    seccomp_profiles: BTreeMap<String, SeccompProfile>,
    /// AppArmor profiles
    apparmor_profiles: BTreeMap<String, AppArmorProfile>,
    /// Security events
    events: Vec<SecurityEvent>,
    /// Next event ID
    next_event_id: AtomicU64,
}

impl SecurityManager {
    /// Create new security manager
    pub fn new() -> Self {
        let mut seccomp_profiles = BTreeMap::new();
        seccomp_profiles.insert("default".to_string(), SeccompProfile::default_profile());
        seccomp_profiles.insert("runtime".to_string(), SeccompProfile::runtime_profile());

        let mut apparmor_profiles = BTreeMap::new();
        apparmor_profiles.insert("default".to_string(), AppArmorProfile::default_profile());

        Self {
            scanner: ImageScanner::new(),
            policies: BTreeMap::new(),
            seccomp_profiles,
            apparmor_profiles,
            events: Vec::new(),
            next_event_id: AtomicU64::new(1),
        }
    }

    /// Scan image
    pub fn scan_image(&mut self, image_id: &str) -> Result<ImageScanResult, SecurityError> {
        self.scanner.scan_image(image_id)
    }

    /// Add security policy
    pub fn add_policy(&mut self, policy: SecurityPolicy) {
        self.policies.insert(policy.name.clone(), policy);
    }

    /// Validate image against policy
    pub fn validate_image(&mut self, image_id: &str, policy_name: &str) -> Result<bool, SecurityError> {
        let policy = self
            .policies
            .get(policy_name)
            .ok_or(SecurityError::PolicyNotFound)?;

        let scan_result = self.scan_image(image_id)?;

        Ok(policy.verify_scan_result(&scan_result))
    }

    /// Get seccomp profile
    pub fn get_seccomp_profile(&self, name: &str) -> Option<&SeccompProfile> {
        self.seccomp_profiles.get(name)
    }

    /// Get apparmor profile
    pub fn get_apparmor_profile(&self, name: &str) -> Option<&AppArmorProfile> {
        self.apparmor_profiles.get(name)
    }

    /// Apply security context
    pub fn apply_security_context(&self, ctx: &SecurityContext) -> Result<(), SecurityError> {
        // Apply seccomp profile
        if let Some(profile_name) = &ctx.seccomp_profile {
            let profile = self
                .seccomp_profiles
                .get(profile_name)
                .ok_or(SecurityError::ProfileNotFound)?;

            self.apply_seccomp_profile(profile)?;
        }

        // Apply apparmor profile
        if let Some(profile_name) = &ctx.apparmor_profile {
            let profile = self
                .apparmor_profiles
                .get(profile_name)
                .ok_or(SecurityError::ProfileNotFound)?;

            self.apply_apparmor_profile(profile)?;
        }

        Ok(())
    }

    /// Apply seccomp profile
    fn apply_seccomp_profile(&self, _profile: &SeccompProfile) -> Result<(), SecurityError> {
        // In real implementation, use seccomp system call
        Ok(())
    }

    /// Apply apparmor profile
    fn apply_apparmor_profile(&self, _profile: &AppArmorProfile) -> Result<(), SecurityError> {
        // In real implementation, use apparmor API
        Ok(())
    }

    /// Record security event
    pub fn record_event(&mut self, event: SecurityEvent) {
        crate::println!("[security] Recording security event: {}", event.description);
        self.events.push(event);
    }

    /// Get security events
    pub fn get_events(&self, container_id: Option<&str>) -> Vec<&SecurityEvent> {
        if let Some(id) = container_id {
            self.events
                .iter()
                .filter(|e| e.container_id == id)
                .collect()
        } else {
            self.events.iter().collect()
        }
    }
}

/// Security errors
///
/// 安全错误
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityError {
    /// Scan failed
    ScanFailed,
    /// Policy not found
    PolicyNotFound,
    /// Profile not found
    ProfileNotFound,
    /// Validation failed
    ValidationFailed,
    /// Apply failed
    ApplyFailed,
}

/// Global security manager instance
static mut SECURITY_MANAGER: Option<SecurityManager> = None;
static mut SECURITY_MANAGER_INITIALIZED: bool = false;

/// Initialize security module
pub fn initialize_security_module() -> Result<(), SecurityError> {
    if unsafe { SECURITY_MANAGER_INITIALIZED } {
        return Ok(());
    }

    let manager = SecurityManager::new();

    // Add default policy
    manager.add_policy(SecurityPolicy::default());

    unsafe {
        SECURITY_MANAGER = Some(manager);
        SECURITY_MANAGER_INITIALIZED = true;
    }

    crate::println!("[security] Security module initialized");
    Ok(())
}

/// Get security manager
pub fn get_security_manager() -> Option<&'static mut SecurityManager> {
    unsafe { SECURITY_MANAGER.as_mut() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_security_policy_default() {
        let policy = SecurityPolicy::default();
        assert_eq!(policy.name, "default");
        assert!(policy.require_non_root);
        assert!(!policy.allow_privilege_escalation);
    }

    #[test]
    fn test_security_policy_validation() {
        let policy = SecurityPolicy::default();

        let ctx = SecurityContext {
            run_as_user: 0,
            run_as_group: 0,
            run_as_non_root: false,
            capabilities: Vec::new(),
            privileged: false,
            allow_privilege_escalation: false,
            read_only_root_filesystem: false,
            seccomp_profile: None,
            apparmor_profile: None,
            selinux_options: None,
        };

        // Should fail because running as root
        assert!(!policy.validate_security_context(&ctx));
    }

    #[test]
    fn test_seccomp_profile() {
        let profile = SeccompProfile::default_profile();
        assert_eq!(profile.name, "default");
        assert_eq!(profile.default_action, SeccompAction::Errno);
    }

    #[test]
    fn test_apparmor_profile() {
        let profile = AppArmorProfile::default_profile();
        assert_eq!(profile.name, "docker-default");
        assert_eq!(profile.mode, AppArmorMode::Enforce);
    }

    #[test]
    fn test_image_scanner() {
        let mut scanner = ImageScanner::new();
        let result = scanner.scan_image("test-image");
        assert!(result.is_ok());
    }

    #[test]
    fn test_security_manager() {
        let manager = SecurityManager::new();

        let seccomp_profile = manager.get_seccomp_profile("default");
        assert!(seccomp_profile.is_some());

        let apparmor_profile = manager.get_apparmor_profile("default");
        assert!(apparmor_profile.is_some());
    }
}
