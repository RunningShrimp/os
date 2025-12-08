// Security Enhancement Module
//
// 安全机制强化模块
// 提供ASLR、SMAP/SMEP、访问控制列表等安全特性

pub mod aslr;
pub mod smap_smep;
pub mod acl;
pub mod capabilities;
pub mod seccomp;
pub mod selinux;
pub mod audit;
pub mod permission_check;

extern crate alloc;

use alloc::format;
use crate::reliability::errno::{EINVAL, ENOENT, ENOMEM, EIO, EPERM, EACCES};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use crate::sync::Mutex;

// Re-export commonly used types
pub use aslr::{
    AslrSubsystem, AslrConfig, AslrStats, MemoryRegionType,
    init_process_aslr, randomize_memory_region, is_aslr_enabled,
    get_aslr_config, update_aslr_config, get_aslr_statistics,
    validate_process_aslr, is_address_randomized, rerandomize_process
};

pub use smap_smep::{
    SmapSmepSubsystem, SmapSmepConfig, SmapSmepStats, SmapSmepViolation,
    ViolationType, ViolationSeverity, init_smap_smep, is_smap_enabled,
    is_smep_enabled, disable_smap_temporarily, enable_smap_temporarily,
    add_smap_allowed_region, get_smap_smep_statistics, update_smap_smep_config
};

pub use acl::{
    AclSubsystem, AclConfig, AclStats, AclEntry, AclPermissions, AclType,
    AccessControlList, AccessRequest, AccessDecision, ResourceType, InheritanceFlags,
    check_file_access, create_acl, get_acl_statistics, update_acl_config
};

pub use capabilities::{
    CapabilitySubsystem, CapabilitySets, ProcessCapabilities, Capability, CapType,
    init_process_capabilities, process_has_capability, get_process_capabilities,
    add_process_capability, remove_process_capability, fork_capabilities,
    exec_capabilities, cleanup_process_capabilities
};

pub use seccomp::{
    SeccompSubsystem, SeccompFilter, SeccompRule, SeccompAction, SeccompCmpOp,
    install_seccomp_filter, check_seccomp_syscall, remove_seccomp_filter,
    get_seccomp_statistics
};

pub use selinux::{
    SelinuxSubsystem, SelinuxContext, SelinuxRule, SelinuxPermission, SelinuxRuleType,
    init_process_selinux, get_process_selinux_context, set_process_selinux_context,
    check_selinux_access, get_file_selinux_context, set_file_selinux_context,
    is_selinux_enforcing, set_selinux_enforcing, get_selinux_statistics
};

pub use audit::{
    AuditSubsystem, AuditEvent, AuditEventType, AuditSeverity, AuditConfig, AuditFilter,
    AuditStats, create_audit_event, log_audit_event, log_security_violation,
    log_authentication_event, get_audit_events, get_audit_statistics,
    update_audit_config
};

pub use permission_check::{
    UnifiedPermissionChecker, PermissionRequest, PermissionResult, PermissionContext,
    Operation, init_unified_permission_checker, check_permission,
    get_unified_checker
};

// Global instances (initialized at runtime)
pub static ASLR: spin::Mutex<Option<AslrSubsystem>> = spin::Mutex::new(None);
pub static SMAP_SMEP: spin::Mutex<Option<SmapSmepSubsystem>> = spin::Mutex::new(None);
pub static ACL: spin::Mutex<Option<AclSubsystem>> = spin::Mutex::new(None);
pub static CAPABILITIES: spin::Mutex<Option<CapabilitySubsystem>> = spin::Mutex::new(None);
pub static SECCOMP: spin::Mutex<Option<SeccompSubsystem>> = spin::Mutex::new(None);
pub static SELINUX: spin::Mutex<Option<SelinuxSubsystem>> = spin::Mutex::new(None);
pub static AUDIT: spin::Mutex<Option<AuditSubsystem>> = spin::Mutex::new(None);

/// 安全子系统状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityStatus {
    /// 未初始化
    Uninitialized,
    /// 初始化中
    Initializing,
    /// 运行中
    Running,
    /// 停止中
    Stopping,
    /// 已停止
    Stopped,
    /// 错误状态
    Error,
}

/// 安全配置
#[derive(Debug, Clone)]
pub struct SecurityConfig {
    /// 是否启用ASLR
    pub enable_aslr: bool,
    /// 是否启用SMAP
    pub enable_smap: bool,
    /// 是否启用SMEP
    pub enable_smep: bool,
    /// 是否启用访问控制列表
    pub enable_acl: bool,
    /// 是否启用Capabilities
    pub enable_capabilities: bool,
    /// 是否启用Seccomp
    pub enable_seccomp: bool,
    /// 是否启用SELinux
    pub enable_selinux: bool,
    /// 是否启用安全审计
    pub enable_audit: bool,
    /// 安全级别
    pub security_level: SecurityLevel,
}

/// 安全级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityLevel {
    /// 基础安全
    Basic,
    /// 标准安全
    Standard,
    /// 高级安全
    High,
    /// 军用级安全
    Military,
    /// 零信任
    ZeroTrust,
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_aslr: true,
            enable_smap: true,
            enable_smep: true,
            enable_acl: true,
            enable_capabilities: true,
            enable_seccomp: true,
            enable_selinux: false, // 默认关闭，可配置
            enable_audit: true,
            security_level: SecurityLevel::Standard,
        }
    }
}

/// 安全统计信息
#[derive(Debug, Clone)]
pub struct SecurityStats {
    /// ASLR统计
    pub aslr_stats: aslr::AslrStats,
    /// SMAP/SMEP统计
    pub smap_smep_stats: smap_smep::SmapSmepStats,
    /// ACL统计
    pub acl_stats: acl::AclStats,
    /// Capabilities统计
    pub capabilities_stats: capabilities::CapabilitySets,
    /// Seccomp统计
    pub seccomp_stats: seccomp::SeccompStats,
    /// SELinux统计
    pub selinux_stats: selinux::SelinuxStats,
    /// 审计统计
    pub audit_stats: audit::AuditStats,
}

impl Default for SecurityStats {
    fn default() -> Self {
        Self {
            aslr_stats: aslr::AslrStats::default(),
            smap_smep_stats: smap_smep::SmapSmepStats::default(),
            acl_stats: acl::AclStats::default(),
            capabilities_stats: capabilities::CapabilitySets::default(),
            seccomp_stats: seccomp::SeccompStats::default(),
            selinux_stats: selinux::SelinuxStats::default(),
            audit_stats: audit::AuditStats::default(),
        }
    }
}

/// 安全事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SecurityEvent {
    /// ASLR事件
    Aslr,
    /// SMAP事件
    Smap,
    /// SMEP事件
    Smep,
    /// ACL事件
    Acl,
    /// Capabilities事件
    Capabilities,
    /// Seccomp事件
    Seccomp,
    /// SELinux事件
    Selinux,
    /// 审计事件
    Audit,
    /// 认证事件
    Authentication,
    /// 授权事件
    Authorization,
}

/// 安全事件
#[derive(Debug, Clone)]
pub struct SecurityEventInfo {
    /// 事件ID
    pub event_id: u64,
    /// 事件类型
    pub event_type: SecurityEvent,
    /// 事件时间戳
    pub timestamp: u64,
    /// 进程ID
    pub pid: u32,
    /// 用户ID
    pub uid: u32,
    /// 组ID
    pub gid: u32,
    /// 事件消息
    pub message: String,
    /// 事件严重级别
    pub severity: EventSeverity,
    /// 事件数据
    pub data: BTreeMap<String, String>,
}

/// 事件严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventSeverity {
    /// 信息
    Info,
    /// 警告
    Warning,
    /// 错误
    Error,
    /// 严重
    Critical,
    /// 致命
    Fatal,
}

/// 安全威胁类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityThreat {
    /// 缓冲区溢出
    BufferOverflow,
    /// 整数溢出
    IntegerOverflow,
    /// 格式化字符串
    FormatString,
    /// 竞争条件
    RaceCondition,
    /// 权限提升
    PrivilegeEscalation,
    /// 代码注入
    CodeInjection,
    /// 拒绝服务攻击
    DenialOfService,
    /// 恶意软件
    Malware,
    /// 网络攻击
    NetworkAttack,
}

/// 安全策略
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    /// 策略ID
    pub policy_id: u64,
    /// 策略名称
    pub name: String,
    /// 策略描述
    pub description: String,
    /// 策略规则
    pub rules: Vec<SecurityRule>,
    /// 是否启用
    pub enabled: bool,
    /// 优先级
    pub priority: u32,
    /// 策略类型
    pub policy_type: PolicyType,
}

/// 安全规则
#[derive(Debug, Clone)]
pub struct SecurityRule {
    /// 规则ID
    pub rule_id: u64,
    /// 规则名称
    pub name: String,
    /// 规则条件
    pub condition: RuleCondition,
    /// 规则动作
    pub action: RuleAction,
    /// 规则描述
    pub description: String,
}

/// 规则条件
#[derive(Debug, Clone)]
pub enum RuleCondition {
    /// 文件路径匹配
    FilePath(String),
    /// 系统调用匹配
    Syscall(u32),
    /// 用户ID匹配
    Uid(u32),
    /// 组ID匹配
    Gid(u32),
    /// 进程名匹配
    ProcessName(String),
    /// 网络地址匹配
    NetworkAddress(String),
    /// 任意条件
    Any,
    /// 组合条件
    And(Vec<RuleCondition>),
    /// 或条件
    Or(Vec<RuleCondition>),
    /// 非条件
    Not(Box<RuleCondition>),
}

/// 规则动作
#[derive(Debug, Clone)]
pub enum RuleAction {
    /// 允许
    Allow,
    /// 拒绝
    Deny,
    /// 记录日志
    Log,
    /// 发送告警
    Alert,
    /// 终止进程
    Terminate,
    /// 隔离进程
    Isolate,
    /// 调用回调
    Callback(String),
}

/// 策略类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyType {
    /// 访问控制
    AccessControl,
    /// 入侵检测
    IntrusionDetection,
    /// 防病毒
    Antivirus,
    /// 防火墙
    Firewall,
    /// 审计
    Audit,
    /// 完整性检查
    Integrity,
}

/// 安全子系统
pub struct SecuritySubsystem {
    /// 安全状态
    status: SecurityStatus,
    /// 安全配置
    config: SecurityConfig,
    /// 安全统计
    stats: Arc<Mutex<SecurityStats>>,
    /// 安全策略
    policies: Arc<Mutex<Vec<SecurityPolicy>>>,
    /// 事件处理器
    event_handlers: BTreeMap<SecurityEvent, Vec<EventHandler>>,
    /// 威胁检测器
    threat_detectors: Vec<Box<dyn ThreatDetector>>,
    /// 安全日志
    security_log: Arc<Mutex<Vec<SecurityEventInfo>>>,
}

impl SecuritySubsystem {
    /// 创建新的安全子系统
    pub fn new(config: SecurityConfig) -> Self {
        let mut event_handlers = BTreeMap::new();

        // 注册默认事件处理器
        event_handlers.insert(SecurityEvent::Aslr, vec![
            EventHandler::Log,
            EventHandler::Alert,
        ]);
        event_handlers.insert(SecurityEvent::Acl, vec![
            EventHandler::Log,
            EventHandler::Alert,
        ]);
        event_handlers.insert(SecurityEvent::Audit, vec![
            EventHandler::Log,
        ]);

        Self {
            status: SecurityStatus::Uninitialized,
            config,
            stats: Arc::new(Mutex::new(SecurityStats::default())),
            policies: Arc::new(Mutex::new(Vec::new())),
            event_handlers,
            threat_detectors: Vec::new(),
            security_log: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 初始化安全子系统
    pub fn initialize(&mut self) -> Result<(), i32> {
        self.status = SecurityStatus::Initializing;

        crate::println!("[security] Initializing security subsystem...");

        // 根据配置初始化各个安全组件
        if self.config.enable_aslr {
            aslr::initialize_aslr()?;
            crate::println!("[security] ASLR initialized");
        }

        if self.config.enable_smap {
            smap_smep::initialize_smap_smep(smap_smep::SmapSmepConfig {
                smap_enabled: true,
                smep_enabled: true,
                strict_mode: false,
                log_violations: true,
                kill_violations: false,
                allow_override: false,
                violation_threshold: 10,
            }).map_err(|_| EIO)?;
            crate::println!("[security] SMAP initialized");
        }

        if self.config.enable_smep {
            smap_smep::initialize_smap_smep(smap_smep::SmapSmepConfig {
                smap_enabled: false,
                smep_enabled: true,
                strict_mode: false,
                log_violations: true,
                kill_violations: false,
                allow_override: false,
                violation_threshold: 10,
            }).map_err(|_| EIO)?;
            crate::println!("[security] SMEP initialized");
        }

        if self.config.enable_acl {
            acl::initialize_acl()?;
            crate::println!("[security] ACL initialized");
        }

        if self.config.enable_capabilities {
            capabilities::initialize_capabilities()?;
            crate::println!("[security] Capabilities initialized");
        }

        if self.config.enable_seccomp {
            seccomp::initialize_seccomp()?;
            crate::println!("[security] Seccomp initialized");
        }

        if self.config.enable_selinux {
            selinux::initialize_selinux()?;
            crate::println!("[security] SELinux initialized");
        }

        if self.config.enable_audit {
            audit::initialize_audit()?;
            crate::println!("[security] Audit subsystem initialized");
        }

        // 初始化默认安全策略
        self.initialize_default_policies()?;

        // 注册默认威胁检测器
        self.register_default_threat_detectors();

        self.status = SecurityStatus::Running;
        crate::println!("[security] Security subsystem initialized successfully");

        Ok(())
    }

    /// 处理安全事件
    pub fn handle_security_event(&self, event: SecurityEventInfo) -> Result<(), i32> {
        if self.status != SecurityStatus::Running {
            return Err(EIO);
        }

        // 记录事件
        {
            let mut log = self.security_log.lock();
            log.push(event.clone());
            // 保持日志大小在合理范围内
            if log.len() > 10000 {
                log.drain(0..1000);
            }
        }

        // 处理事件
        if let Some(handlers) = self.event_handlers.get(&event.event_type) {
            for handler in handlers {
                self.execute_event_handler(handler, &event)?;
            }
        }

        // 更新统计信息
        self.update_event_stats(&event)?;

        // 检测威胁
        self.detect_threats(&event)?;

        Ok(())
    }

    /// 执行事件处理器
    fn execute_event_handler(&self, handler: &EventHandler, event: &SecurityEventInfo) -> Result<(), i32> {
        match handler {
            EventHandler::Log => {
                crate::println!("[security] [{:?}] {}",
                    event.severity,
                    event.message
                );
                Ok(())
            }
            EventHandler::Alert => {
                // 发送告警
                self.send_alert(event)?;
                Ok(())
            }
            EventHandler::Terminate => {
                // 终止进程
                crate::syscalls::process::kill_process(event.pid as u64, 9)?; // SIGKILL
                Ok(())
            }
            EventHandler::Isolate => {
                // 隔离进程
                self.isolate_process(event.pid)?;
                Ok(())
            }
            EventHandler::Callback => {
                // 调用回调函数
                Ok(())
            }
        }
    }

    /// 发送告警
    fn send_alert(&self, event: &SecurityEventInfo) -> Result<(), i32> {
        crate::println!("[security] ALERT: {} [PID: {} UID: {}]",
            event.message, event.pid, event.uid);
        // 在实际实现中，这里会发送到告警系统
        Ok(())
    }

    /// 隔离进程
    fn isolate_process(&self, pid: u32) -> Result<(), i32> {
        crate::println!("[security] Isolating process: {}", pid);
        // 在实际实现中，这里会隔离进程到沙箱环境
        Ok(())
    }

    /// 更新事件统计
    fn update_event_stats(&self, event: &SecurityEventInfo) -> Result<(), i32> {
        let mut stats = self.stats.lock();
        match event.event_type {
            SecurityEvent::Aslr => {
                stats.aslr_stats.events_processed += 1;
            }
            SecurityEvent::Smap => {
                stats.smap_smep_stats.events_processed += 1;
            }
            SecurityEvent::Smep => {
                stats.smap_smep_stats.events_processed += 1;
            }
            SecurityEvent::Acl => {
                stats.acl_stats.events_processed += 1;
            }
            SecurityEvent::Capabilities => {
                // CapabilitySets does not track events_processed; skip
            }
            SecurityEvent::Seccomp => {
                stats.seccomp_stats.events_processed += 1;
            }
            SecurityEvent::Selinux => {
                stats.selinux_stats.events_processed += 1;
            }
            SecurityEvent::Audit => {
                stats.audit_stats.events_processed += 1;
            }
            _ => {}
        }
        Ok(())
    }

    /// 检测威胁
    fn detect_threats(&self, event: &SecurityEventInfo) -> Result<(), i32> {
        for detector in &self.threat_detectors {
            if let Some(threat) = detector.detect_threat(event)? {
                // 处理检测到的威胁
                self.handle_threat(threat)?;
            }
        }
        Ok(())
    }

    /// 处理威胁
    fn handle_threat(&self, threat: SecurityThreatInfo) -> Result<(), i32> {
        crate::println!("[security] THREAT DETECTED: {:?} - {}",
            threat.threat_type, threat.description);

        // 记录威胁事件
        let threat_event = SecurityEventInfo {
            event_id: {
                static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(1);
                NEXT_EVENT_ID.fetch_add(1, Ordering::SeqCst)
            },
            event_type: SecurityEvent::Audit,
            timestamp: self.get_current_time(),
            pid: threat.pid,
            uid: threat.uid,
            gid: threat.gid,
            message: format!("Threat detected: {}", threat.description),
            severity: match threat.severity {
                ThreatSeverity::Low => EventSeverity::Info,
                ThreatSeverity::Medium => EventSeverity::Warning,
                ThreatSeverity::High => EventSeverity::Error,
                ThreatSeverity::Critical => EventSeverity::Critical,
            },
            data: {
                let mut data = BTreeMap::new();
                data.insert("threat_type".to_string(), format!("{:?}", threat.threat_type));
                data.insert("confidence".to_string(), threat.confidence.to_string());
                data.insert("mitigation".to_string(), format!("{:?}", threat.mitigation));
                data
            },
        };

        self.handle_security_event(threat_event)?;
        Ok(())
    }

    /// 获取当前时间
    fn get_current_time(&self) -> u64 {
        crate::time::rdtsc() as u64
    }

    /// 添加安全策略
    pub fn add_security_policy(&self, policy: SecurityPolicy) -> Result<(), i32> {
        if self.status != SecurityStatus::Running {
            return Err(EIO);
        }

        {
            let mut policies = self.policies.lock();
            policies.push(policy.clone());
        }

        crate::println!("[security] Added security policy: {}", policy.name);
        Ok(())
    }

    /// 移除安全策略
    pub fn remove_security_policy(&self, policy_id: u64) -> Result<(), i32> {
        if self.status != SecurityStatus::Running {
            return Err(EIO);
        }

        {
            let mut policies = self.policies.lock();
            policies.retain(|p| p.policy_id != policy_id);
        }

        crate::println!("[security] Removed security policy: {}", policy_id);
        Ok(())
    }

    /// 检查安全策略
    pub fn check_security_policy(&self, context: &SecurityContext) -> Result<PolicyDecision, i32> {
        if self.status != SecurityStatus::Running {
            return Err(EIO);
        }

        let policies = self.policies.lock();

        // 按优先级排序策略，高优先级的先检查
        let mut sorted_policies = policies.clone();
        sorted_policies.sort_by(|a, b| b.priority.cmp(&a.priority));

        for policy in &sorted_policies {
            if !policy.enabled {
                continue;
            }

            for rule in &policy.rules {
                if self.evaluate_rule_condition(&rule.condition, context)? {
                    return Ok(PolicyDecision {
                        policy_id: policy.policy_id,
                        policy_name: policy.name.clone(),
                        action: rule.action.clone(),
                        rule_id: rule.rule_id,
                        rule_name: rule.name.clone(),
                    });
                }
            }
        }

        // 默认允许
        Ok(PolicyDecision {
            policy_id: 0,
            policy_name: "default".to_string(),
            action: RuleAction::Allow,
            rule_id: 0,
            rule_name: "default".to_string(),
        })
    }

    /// 评估规则条件
    fn evaluate_rule_condition(&self, condition: &RuleCondition, context: &SecurityContext) -> Result<bool, i32> {
        match condition {
            RuleCondition::FilePath(path) => Ok(context.file_path.as_ref().map_or(false, |p| p == path)),
            RuleCondition::Syscall(syscall) => Ok(context.syscall == Some(*syscall)),
            RuleCondition::Uid(uid) => Ok(context.uid == *uid),
            RuleCondition::Gid(gid) => Ok(context.gid == *gid),
            RuleCondition::ProcessName(name) => Ok(context.process_name.as_ref().map_or(false, |p| p == name)),
            RuleCondition::NetworkAddress(addr) => Ok(context.network_address.as_ref().map_or(false, |a| a == addr)),
            RuleCondition::Any => Ok(true),
            RuleCondition::And(conditions) => {
                for cond in conditions {
                    if !self.evaluate_rule_condition(cond, context)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            RuleCondition::Or(conditions) => {
                for cond in conditions {
                    if self.evaluate_rule_condition(cond, context)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            RuleCondition::Not(cond) => Ok(!self.evaluate_rule_condition(cond, context)?),
        }
    }

    /// 初始化默认安全策略
    fn initialize_default_policies(&self) -> Result<(), i32> {
        // 添加默认的访问控制策略
        let access_control_policy = SecurityPolicy {
            policy_id: 1,
            name: "Default Access Control".to_string(),
            description: "Default access control policy for process and file access".to_string(),
            rules: vec![
                SecurityRule {
                    rule_id: 1,
                    name: "Allow standard directories".to_string(),
                    condition: RuleCondition::Or(vec![
                        RuleCondition::FilePath("/bin".to_string()),
                        RuleCondition::FilePath("/usr/bin".to_string()),
                        RuleCondition::FilePath("/lib".to_string()),
                        RuleCondition::FilePath("/usr/lib".to_string()),
                    ]),
                    action: RuleAction::Allow,
                    description: "Allow access to standard system directories".to_string(),
                },
                SecurityRule {
                    rule_id: 2,
                    name: "Log sensitive access".to_string(),
                    condition: RuleCondition::Or(vec![
                        RuleCondition::FilePath("/etc".to_string()),
                        RuleCondition::FilePath("/root".to_string()),
                        RuleCondition::FilePath("/var/log".to_string()),
                    ]),
                    action: RuleAction::Log,
                    description: "Log access to sensitive directories".to_string(),
                },
            ],
            enabled: true,
            priority: 100,
            policy_type: PolicyType::AccessControl,
        };

        self.add_security_policy(access_control_policy)?;

        crate::println!("[security] Initialized default security policies");
        Ok(())
    }

    /// 注册默认威胁检测器
    fn register_default_threat_detectors(&mut self) {
        // 注册缓冲区溢出检测器
        self.threat_detectors.push(Box::new(BufferOverflowDetector::new()));
        // 注册权限提升检测器
        self.threat_detectors.push(Box::new(PrivilegeEscalationDetector::new()));
        // 注册恶意软件检测器
        self.threat_detectors.push(Box::new(MalwareDetector::new()));

        crate::println!("[security] Registered default threat detectors");
    }

    /// 获取安全统计信息
    pub fn get_stats(&self) -> SecurityStats {
        self.stats.lock().clone()
    }

    /// 获取安全日志
    pub fn get_security_log(&self, limit: Option<usize>) -> Vec<SecurityEventInfo> {
        let log = self.security_log.lock();
        if let Some(limit) = limit {
            log.iter().rev().take(limit).cloned().collect()
        } else {
            log.iter().rev().cloned().collect()
        }
    }

    /// 停止安全子系统
    pub fn shutdown(&mut self) -> Result<(), i32> {
        if self.status != SecurityStatus::Running {
            return Err(EINVAL);
        }

        self.status = SecurityStatus::Stopping;

        // 停止各个安全组件
        if self.config.enable_audit {
            let _ = audit::cleanup_audit();
        }
        if self.config.enable_selinux {
            let _ = selinux::cleanup_selinux();
        }
        if self.config.enable_seccomp {
            let _ = seccomp::cleanup_seccomp();
        }
        if self.config.enable_capabilities {
            let _ = capabilities::cleanup_capabilities();
        }
        if self.config.enable_acl {
            let _ = acl::cleanup_acl();
        }
        if self.config.enable_smap {
            let _ = smap_smep::cleanup_smap_smep();
        }
        if self.config.enable_smep {
            let _ = smap_smep::cleanup_smap_smep();
        }
        if self.config.enable_aslr {
            aslr::cleanup_aslr();
        }

        self.status = SecurityStatus::Stopped;
        crate::println!("[security] Security subsystem shutdown successfully");
        Ok(())
    }

    /// 获取安全状态
    pub fn get_status(&self) -> SecurityStatus {
        self.status
    }
}

/// 安全策略决策
#[derive(Debug, Clone)]
pub struct PolicyDecision {
    pub policy_id: u64,
    pub policy_name: String,
    pub action: RuleAction,
    pub rule_id: u64,
    pub rule_name: String,
}

/// 安全上下文
#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// 进程ID
    pub pid: u32,
    /// 用户ID
    pub uid: u32,
    /// 组ID
    pub gid: u32,
    /// 进程名
    pub process_name: Option<String>,
    /// 文件路径
    pub file_path: Option<String>,
    /// 系统调用号
    pub syscall: Option<u32>,
    /// 网络地址
    pub network_address: Option<String>,
    /// 进程参数
    pub args: Vec<String>,
    /// 环境变量
    pub env: Vec<String>,
    /// 工作目录
    pub cwd: Option<String>,
}

impl SecurityContext {
    /// 从当前进程创建上下文
    pub fn from_current_process() -> Self {
        Self {
            pid: crate::process::getpid() as u32,
            uid: crate::process::getuid(),
            gid: crate::process::getgid(),
            process_name: Some("kernel".to_string()), // 简化实现
            file_path: None,
            syscall: None,
            network_address: None,
            args: Vec::new(),
            env: Vec::new(),
            cwd: None,
        }
    }

    /// 从进程信息创建上下文
    pub fn from_process_info(pid: u32, uid: u32, gid: u32) -> Self {
        Self {
            pid,
            uid,
            gid,
            process_name: None,
            file_path: None,
            syscall: None,
            network_address: None,
            args: Vec::new(),
            env: Vec::new(),
            cwd: None,
        }
    }
}

/// 事件处理器
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EventHandler {
    Log,
    Alert,
    Terminate,
    Isolate,
    Callback,
}

/// 威胁检测器特征
trait ThreatDetector {
    /// 检测威胁
    fn detect_threat(&self, event: &SecurityEventInfo) -> Result<Option<SecurityThreatInfo>, i32>;
    /// 获取检测器名称
    fn name(&self) -> &str;
}

/// 威胁信息
#[derive(Debug, Clone)]
pub struct SecurityThreatInfo {
    /// 威胁类型
    pub threat_type: SecurityThreat,
    /// 威胁描述
    pub description: String,
    /// 威胁严重程度
    pub severity: ThreatSeverity,
    /// 检测置信度
    pub confidence: u8,
    /// 进程ID
    pub pid: u32,
    /// 用户ID
    pub uid: u32,
    /// 组ID
    pub gid: u32,
    /// 建议的缓解措施
    pub mitigation: ThreatMitigation,
    /// 威胁数据
    pub data: BTreeMap<String, String>,
}

/// 威胁严重程度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThreatSeverity {
    /// 低威胁
    Low,
    /// 中等威胁
    Medium,
    /// 高威胁
    High,
    /// 严重威胁
    Critical,
}

/// 威胁缓解措施
#[derive(Debug, Clone)]
pub enum ThreatMitigation {
    /// 无措施
    None,
    /// 监控
    Monitor,
    /// 隔离
    Isolate,
    /// 终止
    Terminate,
    /// 阻止
    Block,
    /// 修复
    Fix,
}

/// 缓冲区溢出检测器
struct BufferOverflowDetector {
    name: String,
}

impl BufferOverflowDetector {
    fn new() -> Self {
        Self {
            name: "BufferOverflowDetector".to_string(),
        }
    }
}

impl ThreatDetector for BufferOverflowDetector {
    fn detect_threat(&self, event: &SecurityEventInfo) -> Result<Option<SecurityThreatInfo>, i32> {
        // 简化的缓冲区溢出检测逻辑
        if event.message.contains("buffer overflow") ||
           event.message.contains("stack overflow") ||
           event.message.contains("heap overflow") {
            Ok(Some(SecurityThreatInfo {
                threat_type: SecurityThreat::BufferOverflow,
                description: "Potential buffer overflow detected".to_string(),
                severity: ThreatSeverity::High,
                confidence: 80,
                pid: event.pid,
                uid: event.uid,
                gid: event.gid,
                mitigation: ThreatMitigation::Terminate,
                data: BTreeMap::new(),
            }))
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// 权限提升检测器
struct PrivilegeEscalationDetector {
    name: String,
}

impl PrivilegeEscalationDetector {
    fn new() -> Self {
        Self {
            name: "PrivilegeEscalationDetector".to_string(),
        }
    }
}

impl ThreatDetector for PrivilegeEscalationDetector {
    fn detect_threat(&self, event: &SecurityEventInfo) -> Result<Option<SecurityThreatInfo>, i32> {
        // 简化的权限提升检测逻辑
        if event.message.contains("privilege escalation") ||
           event.message.contains("sudo") ||
           event.message.contains("setuid") {
            Ok(Some(SecurityThreatInfo {
                threat_type: SecurityThreat::PrivilegeEscalation,
                description: "Potential privilege escalation attempt".to_string(),
                severity: ThreatSeverity::High,
                confidence: 90,
                pid: event.pid,
                uid: event.uid,
                gid: event.gid,
                mitigation: ThreatMitigation::Monitor,
                data: BTreeMap::new(),
            }))
        } else {
            Ok(None)
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// 恶意软件检测器
struct MalwareDetector {
    name: String,
}

impl MalwareDetector {
    fn new() -> Self {
        Self {
            name: "MalwareDetector".to_string(),
        }
    }
}

impl ThreatDetector for MalwareDetector {
    fn detect_threat(&self, event: &SecurityEventInfo) -> Result<Option<SecurityThreatInfo>, i32> {
        // 简化的恶意软件检测逻辑
        let suspicious_patterns = vec![
            "malware",
            "virus",
            "trojan",
            "rootkit",
            "backdoor",
            "keylogger",
        ];

        for pattern in &suspicious_patterns {
            if event.message.to_lowercase().contains(pattern) {
                return Ok(Some(SecurityThreatInfo {
                    threat_type: SecurityThreat::Malware,
                    description: format!("Suspicious activity detected: {}", pattern),
                    severity: ThreatSeverity::Critical,
                    confidence: 70,
                    pid: event.pid,
                    uid: event.uid,
                    gid: event.gid,
                    mitigation: ThreatMitigation::Isolate,
                    data: {
                        let mut data = BTreeMap::new();
                        data.insert("pattern".to_string(), pattern.clone().to_string());
                        data
                    },
                }));
            }
        }

        Ok(None)
    }

    fn name(&self) -> &str {
        &self.name
    }
}

/// 全局安全子系统实例
static mut SECURITY_SUBSYSTEM: Option<SecuritySubsystem> = None;
static mut SECURITY_SUBSYSTEM_INITIALIZED: bool = false;

/// 初始化安全子系统
pub fn initialize_security(config: SecurityConfig) -> Result<(), i32> {
    if unsafe { SECURITY_SUBSYSTEM_INITIALIZED } {
        return Ok(());
    }

    let mut subsystem = SecuritySubsystem::new(config);
    subsystem.initialize()?;

    unsafe {
        SECURITY_SUBSYSTEM = Some(subsystem);
        SECURITY_SUBSYSTEM_INITIALIZED = true;
    }

    crate::println!("[security] Security subsystem initialized");
    Ok(())
}

/// 获取安全子系统引用
pub fn get_security_subsystem() -> Option<&'static SecuritySubsystem> {
    unsafe {
        SECURITY_SUBSYSTEM.as_ref()
    }
}

/// 获取安全统计信息
pub fn get_security_stats() -> Option<SecurityStats> {
    get_security_subsystem().map(|ss| ss.get_stats())
}

/// 处理安全事件
pub fn handle_security_event(event_type: SecurityEvent, message: &str) -> Result<(), i32> {
    let subsystem = get_security_subsystem().ok_or(EIO)?;
    let event = SecurityEventInfo {
        event_id: {
            static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(1);
            NEXT_EVENT_ID.fetch_add(1, Ordering::SeqCst)
        },
        event_type,
        timestamp: crate::time::rdtsc() as u64,
        pid: crate::process::getpid() as u32,
        uid: crate::posix::thread::getuid(),
        gid: crate::posix::thread::getgid(),
        message: message.to_string(),
        severity: EventSeverity::Info,
        data: BTreeMap::new(),
    };

    subsystem.handle_security_event(event)
}

/// 检查安全策略
pub fn check_security_policy(context: &SecurityContext) -> Result<PolicyDecision, i32> {
    let subsystem = get_security_subsystem().ok_or(EIO)?;
    subsystem.check_security_policy(context)
}

/// 初始化安全子系统
pub fn init_security_subsystem() -> Result<(), &'static str> {
    // 初始化各个安全子系统
    smap_smep::init_smap_smep(smap_smep::SmapSmepConfig::default())?;

    crate::println!("[Security] Security subsystem initialized successfully");
    Ok(())
}
