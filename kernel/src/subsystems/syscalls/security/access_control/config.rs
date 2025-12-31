//! 访问控制配置
//!
//! 定义访问控制系统的配置选项

use super::types::AccessRule;

/// 访问控制配置
#[derive(Debug, Clone)]
pub struct AccessControlConfig {
    /// 是否启用严格模式
    pub strict_mode: bool,
    /// 默认访问规则
    pub default_access_rule: AccessRule,
    /// 是否启用能力系统
    pub enable_capabilities: bool,
    /// 是否启用审计日志
    pub enable_audit_log: bool,
    /// 最大ACL条目数
    pub max_acl_entries: usize,
}

impl Default for AccessControlConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            default_access_rule: AccessRule::Deny,
            enable_capabilities: true,
            enable_audit_log: true,
            max_acl_entries: 10000,
        }
    }
}
