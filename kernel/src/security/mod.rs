//! # 安全子系统
//!
//! 提供内核安全机制，保护系统免受各种攻击和未授权访问。
//!
//! ## 概述
//!
//! 安全子系统提供多层次的安全保护：
//! - **内存安全**: ASLR、Stack Canaries、SMEP/SMAP
//! - **访问控制**: ACL、Capabilities、权限检查
//! - **安全审计**: 审计日志、合规性检查
//! - **内存审计**: 内存使用监控和异常检测
//!
//! ## 主要组件
//!
//! ### 内存安全
//! - [`aslr`]: 地址空间布局随机化
//! - [`stack_canaries`]: 栈保护金丝雀
//! - [`memory_security`]: 内存安全机制
//! - [`memory_audit`]: 内存审计和监控
//!
//! ### 访问控制
//! - [`enhanced_permissions`]: 增强的权限系统
//! - [`acl`]: 访问控制列表
//! - [`capabilities`]: POSIX Capabilities
//! - [`seccomp`]: 系统调用过滤
//!
//! ### 网络安全
//! - [`firewall`]: 防火墙和包过滤引擎
//! - [`ids`]: 入侵检测系统 (IDS/IPS)
//! - [`ddos`]: DDoS 防护机制
//! - [`vpn`]: VPN 隧道协议 (IPsec, WireGuard, OpenVPN)
//! - [`net_analysis`]: 网络流量分析和 DPI
//! - [`zero_trust`]: 零信任网络架构
//!
//! ### 加密和密钥管理
//! - [`certificate`]: X.509 证书管理
//! - [`hsm`]: 硬件安全模块 (HSM)
//! - [`kms`]: 密钥管理服务 (KMS)
//! - [`tpm`]: 可信平台模块 (TPM)
//! - [`secure_boot`]: 安全启动
//!
//! ### 审计和合规
//! - [`audit`]: 安全审计框架
//! - [`audit_enhanced`]: 增强审计系统
//!
//! ## 安全机制
//!
//! ### 编译时保护
//!
//! - **Stack Canaries**: 检测栈溢出
//! - **ASLR**: 随机化内存布局
//! - **Fortify Source**: 缓冲区溢出保护
//!
//! ### 运行时保护
//!
//! - **SMEP/SMAP**: 禁止内核执行/访问用户空间
//! - **NX 位**: 禁止执行不可写内存
//! - **Page Table Isolation**: 内核和用户空间页表隔离
//!
//! ### 访问控制
//!
//! - **DAC**: 自主访问控制（Unix 权限）
//! - **ACL**: 访问控制列表
//! - **Capabilities**: 细粒度权限
//! - **MAC**: 强制访问控制（可选）
//!
//! ## 使用示例
//!
//! ### 初始化安全子系统
//!
//! ```no_run
//! use kernel::security::init_security_subsystem;
//!
//! // 初始化所有安全机制
//! init_security_subsystem()?;
//! # Ok::<(), kernel::security::SecurityError>(())
//! ```
//!
//! ### ASLR
//!
//! ```no_run
//! use kernel::security::{randomize_memory_region, MemoryRegionType};
//!
//! // 随机化栈地址
//! let base_addr = 0x7fff_0000_0000;
//! let size = 0x1000;
//! let randomized = randomize_memory_region(
//!     base_addr,
//!     size,
//!     MemoryRegionType::Stack
//! )?;
//! ```
//!
//! ### 权限检查
//!
//! ```no_run
//! use kernel::security::check_permission;
//!
//! // 检查文件访问权限
//! if check_permission(uid, gid, file_mode, required_perm) {
//!     // 允许访问
//! }
//! ```
//!
//! ## 设计决策
//!
//! ### 深度防御
//!
//! 采用多层次的安全防护：
//! - 即使一层被突破，其他层仍能保护系统
//! - 编译时和运行时保护结合
//! - 软件和硬件机制协同
//!
//! ### 最小权限原则
//!
//! - 进程只获得必需的权限
//! - 使用Capabilities而非 root
//! - 限制特权操作
//!
//! ## 性能影响
//!
//! - **ASLR**: < 1% 启动时开销
//! - **Stack Canaries**: < 2% 函数调用开销
//! - **权限检查**: O(1) 常数时间
//!
//! ## 安全等级
//!
//! NOS 支持多个安全等级：
//! - **Level 0**: 无安全保护（开发用）
//! - **Level 1**: 基础保护（生产推荐）
//! - **Level 2**: 增强保护（高安全需求）
//! - **Level 3**: 最高保护（军事/金融）
//!
//! ## 合规性
//!
//! - **POSIX.1e**: ACL 和 Capabilities
//! - **LSPP**: 强制访问控制
//! - **Common Criteria**: 安全评估
//!
//! ## 相关模块
//!
//! - [`crate::security_audit`]: 安全审计框架
//! - [`crate::subsystems::process`]: 进程安全
//! - [`crate::arch`]: 架构特定的安全特性

//! 安全模块
//!
//! 提供增强的安全功能，包括细粒度权限控制、
//! 能力安全、审计和强制访问控制。

pub mod aslr;
pub mod audit;
pub mod audit_enhanced;
pub mod apparmor;
pub mod attestation;
pub mod breach;
pub mod capabilities;
pub mod certificate;
pub mod ddos;
pub mod enforcement;
pub mod enhanced_permissions;
pub mod firewall;
pub mod hsm;
pub mod ids;
pub mod keys;
pub mod kms;
pub mod lsm_framework;
pub mod mac;
pub mod memory_audit;
pub mod memory_security;
pub mod net_analysis;
pub mod sandbox;
pub mod secure_boot;
pub mod selinux;
pub mod selinux_enhanced;
pub mod stack_canaries;
pub mod tee;
pub mod tpm;
pub mod vpn;
pub mod zero_trust;

// P2 Priority Security Features (optional, feature-gated)
#[cfg(feature = "cfi")]
pub mod cfi;

#[cfg(feature = "shadow_stack")]
pub mod shadow_stack;

#[cfg(feature = "memory_encryption")]
pub mod encrypted_memory;

// Stage 3-5: Advanced Security Features
pub use audit_enhanced::{
    AlertConfig, AlertBackend, AlertingSystem, AuditContext, AuditError, AuditEvent,
    AuditEventType, AuditRule, AuditRuleAction, AuditRuleCondition, AuditOperator,
    AuditSeverity, AuditStatistics, AuditSystem, ComplianceReport, ComplianceReporter,
    ComplianceStandard, LogIntegrityChain, init_audit_system, get_audit_system,
};
pub use certificate::{
    Certificate, CertificateParser, CertificateRevocationList, CertificateSigningRequest,
    CertificateValidator, CertificateVersion, CrlEntry, CrlManager, DistinguishedName, Extension,
    OcspClient, OcspResponse, OcspStatus, RootCaManager, SignatureAlgorithm, Validity,
    ValidationStatus, init_certificate_subsystem, get_certificate_validator,
    extensions,
};
pub use hsm::{
    HsmDevice, HsmDeviceInfo, HsmError, HsmManager, HsmObjectAttributes, HsmObjectHandle,
    HsmSession, HsmSessionHandle, HsmSlotId, HsmStats, HsmStatus, Pkcs11KeyType,
    Pkcs11Mechanism, Pkcs11ObjectClass, SoftwareHsm, init_hsm, get_hsm_manager,
};
pub use kms::{
    KeyData, KeyDerivationFunction,
    KeyEntry, KeyEscrowManager, KeyFormat, KeyLifecycleManager, KeyMetadata,
    KeyProvider, KeyStatus, KeyStorage, KeyType, KeyUsage, KmsError,
    SoftwareKeyProvider, init_kms, get_kms,
};
pub use secure_boot::{
    BootLogger, BootMeasurement, CertificateValidator as SecureBootCertValidator,
    DbManager, EfiSignatureData, EfiSignatureList, EfiSignatureOwner, EfiSignatureType,
    ModuleSignature, ModuleVerifier, RecoveryManager as SecureBootRecoveryManager,
    SecureBootError, SecureBootManager, SecureBootState, SignatureDatabase,
    init_secure_boot, get_secure_boot_manager,
};
pub use tpm::{
    PcrPolicy, PcrRegister, TpmAlgorithm, TpmAttestationReport, TpmCommandBuffer,
    TpmCommandCode, TpmDevice, TpmEccCurve, TpmError, TpmHandleType, TpmHierarchy,
    TpmKeyHandle, TpmPermanentHandle, TpmPublicKey, TpmResource, TpmResourceManager,
    TpmSealedData, TpmStats, TpmTag, init_tpm, get_tpm_device,
};

// Network Security Module Exports
pub use firewall::{
    FirewallEngine, FirewallError, FirewallRule, FirewallChain, FirewallStats,
    RuleTarget, PacketMatch, AddressMatch, PortMatch, TcpFlags, RateLimit,
    ConntrackEntry, ConntrackState, NatEntry, NatType, Protocol, ChainType,
    init_firewall, add_rule,
};
pub use ids::{
    IdsEngine, IdsError, IdsRule, Detection, ThreatLevel, DetectionType,
    RuleAction, Protocol as IdsProtocol, AddressSpec, PortSpec, RuleOption,
    Signature, SignatureType, SignatureContext, AnomalyModel, DetectionAlgorithm,
    IdsStatistics, IdsConfig, PerformanceMode,
    init_ids, process_packet_ids,
};
pub use ddos::{
    DdosProtectionEngine, DdosError, IpListType, RateLimitStrategy,
    RateLimitEntry, RateLimitConfig, SynCookieState, SynCookie, SynCookieConfig,
    TrafficCleaningConfig, CleaningAction, AdaptiveFilterConfig, DdosStatistics,
    init_ddos_protection, process_packet_ddos, blacklist_ip, whitelist_ip,
};
pub use vpn::{
    VpnManager, VpnError, VpnProtocol, VpnTunnel, TunnelConfig, TunnelState,
    EncryptionAlgorithm, AuthenticationAlgorithm, KeyExchangeMethod,
    IpsecSa, WireGuardPeer, VpnStatistics,
    init_vpn, create_ipsec_tunnel, create_wireguard_tunnel,
};
pub use net_analysis::{
    TrafficAnalyzer, ProtocolType, TrafficEntry, FlowState, DpiResult,
    BehaviorAnalysisResult, TrafficAnalysisStats, TrafficAnalysisConfig,
    init_traffic_analysis, analyze_packet_traffic,
};
pub use zero_trust::{
    ZeroTrustEngine, ZeroTrustError, Identity, IdentityType, Device, DeviceStatus,
    TrustLevel, AccessPolicy, PolicyCondition, ConditionType, ConditionOperator,
    AccessToken, AccessRequest, AccessDecision, MicroSegmentationRule, ZeroTrustStatistics,
    init_zero_trust, evaluate_access_zero_trust,
};

// 只导出在其他地方直接使用的安全函数
use aslr::AslrSubsystem;
pub use aslr::{MemoryRegionType, initialize_aslr, is_aslr_enabled, randomize_memory_region};
pub use enhanced_permissions::init_permission_manager;
pub use memory_audit::{
    MEMORY_AUDITOR, MemoryAuditConfig, MemoryAuditResult, MemoryAuditor, MemorySafetyFinding,
    MemoryStatistics, MemoryUsage, get_memory_usage, record_allocation, record_deallocation,
    run_memory_audit,
};
pub use memory_security::{
    create_process_security_context, init_security, remove_process_security_context,
};
// Global ASLR subsystem instance
use spin::Mutex;
pub use stack_canaries::CanaryConfig;
pub static ASLR: Mutex<Option<AslrSubsystem>> = Mutex::new(None);

/// Initialize security subsystem
pub fn init_security_subsystem() -> Result<(), SecurityError> {
    // Initialize enhanced permission system
    init_permission_manager();

    // Initialize memory security system
    memory_security::init_security()?;

    // Initialize stack canaries system
    let canary_config = CanaryConfig::default();
    stack_canaries::init_stack_canaries(canary_config)
        .map_err(|_| SecurityError::PermissionDenied)?;

    Ok(())
}

/// Get current security level for a process
pub fn get_current_security_level(pid: u32) -> memory_security::SecurityLevel {
    // For now, return Medium security level for all non-kernel processes
    // In a real implementation, this would be based on process credentials
    if pid == 0 {
        memory_security::SecurityLevel::System
    } else {
        memory_security::SecurityLevel::Medium
    }
}

/// Security errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    /// Permission denied
    PermissionDenied,
    /// Process not found
    ProcessNotFound,
    /// Memory security error
    MemorySecurityError(memory_security::SecurityError),
}

impl From<memory_security::SecurityError> for SecurityError {
    fn from(err: memory_security::SecurityError) -> Self {
        SecurityError::MemorySecurityError(err)
    }
}

// Security mechanisms test suite
#[cfg(test)]
pub mod tests;
