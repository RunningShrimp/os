//! 系统调用安全模块
//!
//! 本模块提供系统调用的安全功能，包括：
//! - 系统调用安全验证
//! - 权限管理
//! - 安全策略
//! - 审计日志
//! - 访问控制

pub mod access_control;
pub mod syscall_validator;

// Re-exports from access_control (excluding conflicting ResourceType)
pub use access_control::{
    UserId, GroupId, ProcessId, AccessResult, UserInfo, UserType, AccountStatus,
    Permission, ResourceType as AccessResourceType, AccessControlEntry,
    PrincipalType, AccessRule, Capability, CapabilityType,
    AccessControlManager, GroupInfo, AccessControlConfig,
};

// Re-exports from syscall_validator (excluding conflicting ResourceType)
pub use syscall_validator::{
    SecurityValidationResult, SecurityContext, SecurityLevel, ResourceAccess,
    SyscallSecurityPolicy, ArgumentValidationRule, ArgumentValidationType,
    ResourceRequirement, ResourceType as ValidatorResourceType, AuditLogEntry,
    SyscallSecurityValidator, ValidatorConfig,
};
