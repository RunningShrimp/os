//! 系统调用安全模块
//!
//! 本模块提供系统调用的安全功能，包括：
//! - 系统调用安全验证
//! - 权限管理
//! - 安全策略
//! - 审计日志
//! - 访问控制

// Access control modules (flattened from access_control/ subdirectory)
pub mod access_types;
pub mod access_config;
pub mod access_manager;
pub mod syscall_validator;

// Re-exports from access control modules (excluding conflicting ResourceType)
pub use access_types::{
    UserId, GroupId, ProcessId, AccessResult, UserInfo, UserType, AccountStatus,
    Permission, ResourceType as AccessResourceType, AccessControlEntry,
    PrincipalType, AccessRule, Capability, CapabilityType, GroupInfo,
};
pub use access_config::AccessControlConfig;
pub use access_manager::AccessControlManager;

// Re-exports from syscall_validator (excluding conflicting ResourceType)
pub use syscall_validator::{
    SecurityValidationResult, SecurityContext, SecurityLevel, ResourceAccess,
    SyscallSecurityPolicy, ArgumentValidationRule, ArgumentValidationType,
    ResourceRequirement, ResourceType as ValidatorResourceType, AuditLogEntry,
    SyscallSecurityValidator, ValidatorConfig,
};
