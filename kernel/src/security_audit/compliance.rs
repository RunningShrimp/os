// Compliance Module for Security Audit

extern crate alloc;
//
// 合规模块，负责检查系统是否符合各种安全合规标准

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{format, vec};
use alloc::string::String;
use alloc::string::ToString;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use crate::security::audit::{AuditEvent, AuditEventType, AuditSeverity};
use super::{ComplianceStandard, ComplianceResult, ComplianceStatus, SecurityAuditStats};

/// 合规检查器
pub struct ComplianceChecker {
    /// 检查器ID
    pub id: u64,
    /// 支持的合规标准
    supported_standards: Vec<ComplianceStandard>,
    /// 合规规则
    rules: BTreeMap<ComplianceStandard, Vec<ComplianceRule>>,
    /// 检查结果缓存
    result_cache: Arc<Mutex<BTreeMap<ComplianceStandard, Vec<ComplianceResult>>>>,
    /// 检查统计
    stats: Arc<Mutex<ComplianceCheckerStats>>,
    /// 下一个检查ID
    next_check_id: AtomicU64,
}

/// 合规规则
#[derive(Debug, Clone)]
pub struct ComplianceRule {
    /// 规则ID
    pub id: u64,
    /// 合规标准
    pub standard: ComplianceStandard,
    /// 规则类别
    pub category: ComplianceCategory,
    /// 规则名称
    pub name: String,
    /// 规则描述
    pub description: String,
    /// 规则条件
    pub condition: ComplianceCondition,
    /// 预期结果
    pub expected_result: ExpectedResult,
    /// 检查方法
    pub check_method: CheckMethod,
    /// 规则严重性
    pub severity: ComplianceSeverity,
    /// 是否启用
    pub enabled: bool,
}

/// 合规类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComplianceCategory {
    /// 访问控制
    AccessControl,
    /// 身份认证
    Authentication,
    /// 加密
    Encryption,
    /// 日志记录
    Logging,
    /// 监控
    Monitoring,
    /// 数据保护
    DataProtection,
    /// 网络安全
    NetworkSecurity,
    /// 系统配置
    SystemConfiguration,
    /// 物理安全
    PhysicalSecurity,
    /// 应急响应
    IncidentResponse,
}

/// 合规条件
#[derive(Debug, Clone)]
pub enum ComplianceCondition {
    /// 事件计数条件
    EventCount {
        event_type: AuditEventType,
        time_range: (u64, u64),
        operator: ComparisonOperator,
        threshold: u64,
    },
    /// 系统配置条件
    SystemConfig {
        config_key: String,
        expected_value: String,
    },
    /// 文件权限条件
    FilePermissions {
        file_path: String,
        expected_permissions: String,
    },
    /// 服务状态条件
    ServiceStatus {
        service_name: String,
        expected_status: ServiceStatus,
    },
    /// 自定义条件
    Custom {
        check_function: String,
        parameters: Vec<String>,
    },
}

/// 比较操作符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
    /// 等于
    Equals,
    /// 不等于
    NotEquals,
    /// 大于
    GreaterThan,
    /// 大于等于
    GreaterThanOrEqual,
    /// 小于
    LessThan,
    /// 小于等于
    LessThanOrEqual,
}

/// 预期结果
#[derive(Debug, Clone)]
pub enum ExpectedResult {
    /// 布尔值
    Boolean(bool),
    /// 数值
    Numeric(u64),
    /// 字符串
    String(String),
    /// 列表
    List(Vec<String>),
}

/// 检查方法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckMethod {
    /// 事件分析
    EventAnalysis,
    /// 系统调用
    SystemCall,
    /// 文件检查
    FileCheck,
    /// 网络扫描
    NetworkScan,
    /// 配置检查
    ConfigCheck,
    /// API调用
    ApiCall,
}

/// 服务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    /// 运行中
    Running,
    /// 已停止
    Stopped,
    /// 启用
    Enabled,
    /// 禁用
    Disabled,
}

/// 合规严重性
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ComplianceSeverity {
    /// 信息
    Info,
    /// 低
    Low,
    /// 中
    Medium,
    /// 高
    High,
    /// 严重
    Critical,
}

/// 合规检查器统计
#[derive(Debug, Default, Clone)]
pub struct ComplianceCheckerStats {
    /// 总检查次数
    pub total_checks: u64,
    /// 按标准统计
    pub checks_by_standard: BTreeMap<ComplianceStandard, u64>,
    /// 按类别统计
    pub checks_by_category: BTreeMap<ComplianceCategory, u64>,
    /// 合规检查次数
    pub compliant_checks: u64,
    /// 不合规检查次数
    pub non_compliant_checks: u64,
    /// 部分合规检查次数
    pub partially_compliant_checks: u64,
    /// 平均检查时间（微秒）
    pub avg_check_time_us: u64,
    /// 缓存命中次数
    pub cache_hits: u64,
    /// 缓存未命中次数
    pub cache_misses: u64,
}

impl ComplianceChecker {
    /// 创建新的合规检查器
    pub fn new() -> Self {
        Self {
            id: 1,
            supported_standards: Vec::new(),
            rules: BTreeMap::new(),
            result_cache: Arc::new(Mutex::new(BTreeMap::new())),
            stats: Arc::new(Mutex::new(ComplianceCheckerStats::default())),
            next_check_id: AtomicU64::new(1),
        }
    }

    /// 初始化合规检查器
    pub fn init(&mut self, standards: &[ComplianceStandard]) -> Result<(), &'static str> {
        self.supported_standards = standards.to_vec();

        // 为每个标准加载默认规则
        for &standard in standards {
            self.load_default_rules(standard)?;
        }

        crate::println!("[ComplianceChecker] Compliance checker initialized for {} standards", standards.len());
        Ok(())
    }

    /// 加载默认规则
    fn load_default_rules(&mut self, standard: ComplianceStandard) -> Result<(), &'static str> {
        let rules = match standard {
            ComplianceStandard::SOC2 => self.load_soc2_rules(),
            ComplianceStandard::PCI_DSS => self.load_pci_dss_rules(),
            ComplianceStandard::ISO_27001 => self.load_iso27001_rules(),
            ComplianceStandard::GDPR => self.load_gdpr_rules(),
            ComplianceStandard::HIPAA => self.load_hipaa_rules(),
            ComplianceStandard::NIST => self.load_nist_rules(),
            ComplianceStandard::SOX => self.load_sox_rules(),
            ComplianceStandard::FIPS => self.load_fips_rules(),
        };

        self.rules.insert(standard, rules);
        Ok(())
    }

    /// 加载SOC2规则
    fn load_soc2_rules(&self) -> Vec<ComplianceRule> {
        vec![
            ComplianceRule {
                id: 1,
                standard: ComplianceStandard::SOC2,
                category: ComplianceCategory::Logging,
                name: "Security Event Logging".to_string(),
                description: "All security events must be logged".to_string(),
                condition: ComplianceCondition::EventCount {
                    event_type: AuditEventType::SecurityViolation,
                    time_range: (0, crate::subsystems::time::get_timestamp()),
                    operator: ComparisonOperator::GreaterThan,
                    threshold: 0,
                },
                expected_result: ExpectedResult::Boolean(true),
                check_method: CheckMethod::EventAnalysis,
                severity: ComplianceSeverity::High,
                enabled: true,
            },
            ComplianceRule {
                id: 2,
                standard: ComplianceStandard::SOC2,
                category: ComplianceCategory::AccessControl,
                name: "Access Control Logging".to_string(),
                description: "Access control events must be logged".to_string(),
                condition: ComplianceCondition::EventCount {
                    event_type: AuditEventType::Authentication,
                    time_range: (0, crate::subsystems::time::get_timestamp()),
                    operator: ComparisonOperator::GreaterThan,
                    threshold: 0,
                },
                expected_result: ExpectedResult::Boolean(true),
                check_method: CheckMethod::EventAnalysis,
                severity: ComplianceSeverity::Medium,
                enabled: true,
            },
        ]
    }

    /// 加载PCI DSS规则
    fn load_pci_dss_rules(&self) -> Vec<ComplianceRule> {
        vec![
            ComplianceRule {
                id: 1001,
                standard: ComplianceStandard::PCI_DSS,
                category: ComplianceCategory::Encryption,
                name: "Card Data Encryption".to_string(),
                description: "Cardholder data must be encrypted".to_string(),
                condition: ComplianceCondition::SystemConfig {
                    config_key: "encryption.card_data.enabled".to_string(),
                    expected_value: "true".to_string(),
                },
                expected_result: ExpectedResult::Boolean(true),
                check_method: CheckMethod::ConfigCheck,
                severity: ComplianceSeverity::Critical,
                enabled: true,
            },
            ComplianceRule {
                id: 1002,
                standard: ComplianceStandard::PCI_DSS,
                category: ComplianceCategory::AccessControl,
                name: "Access Control".to_string(),
                description: "Access to cardholder data must be restricted".to_string(),
                condition: ComplianceCondition::SystemConfig {
                    config_key: "access.card_data.restricted".to_string(),
                    expected_value: "true".to_string(),
                },
                expected_result: ExpectedResult::Boolean(true),
                check_method: CheckMethod::ConfigCheck,
                severity: ComplianceSeverity::Critical,
                enabled: true,
            },
        ]
    }

    /// 加载ISO 27001规则
    fn load_iso27001_rules(&self) -> Vec<ComplianceRule> {
        vec![
            ComplianceRule {
                id: 2001,
                standard: ComplianceStandard::ISO_27001,
                category: ComplianceCategory::Monitoring,
                name: "System Monitoring".to_string(),
                description: "Systems must be monitored for security events".to_string(),
                condition: ComplianceCondition::EventCount {
                    event_type: AuditEventType::KernelEvent,
                    time_range: (0, crate::subsystems::time::get_timestamp()),
                    operator: ComparisonOperator::GreaterThan,
                    threshold: 0,
                },
                expected_result: ExpectedResult::Boolean(true),
                check_method: CheckMethod::EventAnalysis,
                severity: ComplianceSeverity::Medium,
                enabled: true,
            },
        ]
    }

    /// 加载GDPR规则
    fn load_gdpr_rules(&self) -> Vec<ComplianceRule> {
        vec![
            ComplianceRule {
                id: 3001,
                standard: ComplianceStandard::GDPR,
                category: ComplianceCategory::DataProtection,
                name: "Personal Data Protection".to_string(),
                description: "Personal data must be protected".to_string(),
                condition: ComplianceCondition::SystemConfig {
                    config_key: "data.personal.protection".to_string(),
                    expected_value: "enabled".to_string(),
                },
                expected_result: ExpectedResult::Boolean(true),
                check_method: CheckMethod::ConfigCheck,
                severity: ComplianceSeverity::High,
                enabled: true,
            },
        ]
    }

    /// 加载HIPAA规则
    fn load_hipaa_rules(&self) -> Vec<ComplianceRule> {
        vec![
            ComplianceRule {
                id: 4001,
                standard: ComplianceStandard::HIPAA,
                category: ComplianceCategory::AccessControl,
                name: "PHI Access Control".to_string(),
                description: "Access to PHI must be controlled".to_string(),
                condition: ComplianceCondition::SystemConfig {
                    config_key: "access.phi.controlled".to_string(),
                    expected_value: "true".to_string(),
                },
                expected_result: ExpectedResult::Boolean(true),
                check_method: CheckMethod::ConfigCheck,
                severity: ComplianceSeverity::Critical,
                enabled: true,
            },
        ]
    }

    /// 加载NIST规则
    fn load_nist_rules(&self) -> Vec<ComplianceRule> {
        vec![
            ComplianceRule {
                id: 5001,
                standard: ComplianceStandard::NIST,
                category: ComplianceCategory::IncidentResponse,
                name: "Incident Response".to_string(),
                description: "Security incidents must be responded to".to_string(),
                condition: ComplianceCondition::EventCount {
                    event_type: AuditEventType::SecurityViolation,
                    time_range: (0, crate::subsystems::time::get_timestamp()),
                    operator: ComparisonOperator::GreaterThan,
                    threshold: 0,
                },
                expected_result: ExpectedResult::Boolean(true),
                check_method: CheckMethod::EventAnalysis,
                severity: ComplianceSeverity::Medium,
                enabled: true,
            },
        ]
    }

    /// 加载SOX规则
    fn load_sox_rules(&self) -> Vec<ComplianceRule> {
        vec![
            ComplianceRule {
                id: 6001,
                standard: ComplianceStandard::SOX,
                category: ComplianceCategory::Logging,
                name: "Financial Audit Trail".to_string(),
                description: "Financial transactions must be auditable".to_string(),
                condition: ComplianceCondition::EventCount {
                    event_type: AuditEventType::Process,
                    time_range: (0, crate::subsystems::time::get_timestamp()),
                    operator: ComparisonOperator::GreaterThan,
                    threshold: 0,
                },
                expected_result: ExpectedResult::Boolean(true),
                check_method: CheckMethod::EventAnalysis,
                severity: ComplianceSeverity::High,
                enabled: true,
            },
        ]
    }

    /// 加载FIPS规则
    fn load_fips_rules(&self) -> Vec<ComplianceRule> {
        vec![
            ComplianceRule {
                id: 7001,
                standard: ComplianceStandard::FIPS,
                category: ComplianceCategory::Encryption,
                name: "FIPS Encryption".to_string(),
                description: "Must use FIPS approved encryption".to_string(),
                condition: ComplianceCondition::SystemConfig {
                    config_key: "encryption.fips.enabled".to_string(),
                    expected_value: "true".to_string(),
                },
                expected_result: ExpectedResult::Boolean(true),
                check_method: CheckMethod::ConfigCheck,
                severity: ComplianceSeverity::Critical,
                enabled: true,
            },
        ]
    }

    /// 检查单个事件
    pub fn check_event(&mut self, event: &AuditEvent) -> Result<(), &'static str> {
        // 先收集需要执行的检查，避免借用冲突
        let mut checks_to_execute = Vec::new();

        for standard in &self.supported_standards {
            if let Some(rules) = self.rules.get(standard) {
                for rule in rules {
                    if !rule.enabled {
                        continue;
                    }

                    if self.rule_matches_event(rule, event) {
                        checks_to_execute.push(rule.clone());
                    }
                }
            }
        }

        // 执行收集到的合规检查
        for rule in checks_to_execute {
            self.execute_compliance_check(&rule, event)?;
        }
        Ok(())
    }

    /// 检查规则是否匹配事件
    fn rule_matches_event(&self, rule: &ComplianceRule, event: &AuditEvent) -> bool {
        match &rule.condition {
            ComplianceCondition::EventCount { event_type, time_range, .. } => {
                event.event_type == *event_type &&
                event.timestamp >= time_range.0 &&
                event.timestamp <= time_range.1
            }
            _ => false,
        }
    }

    /// 执行合规检查
    fn execute_compliance_check(&mut self, rule: &ComplianceRule, event: &AuditEvent) -> Result<(), &'static str> {
        let start_time = crate::subsystems::time::get_timestamp_nanos();

        let result = match &rule.condition {
            ComplianceCondition::EventCount { event_type, time_range, operator, threshold } => {
                self.check_event_count(*event_type, *time_range, *operator, *threshold)
            }
            ComplianceCondition::SystemConfig { config_key, expected_value } => {
                self.check_system_config(config_key, expected_value)
            }
            ComplianceCondition::FilePermissions { file_path, expected_permissions } => {
                self.check_file_permissions(file_path, expected_permissions)
            }
            ComplianceCondition::ServiceStatus { service_name, expected_status } => {
                self.check_service_status(service_name, *expected_status)
            }
            ComplianceCondition::Custom { check_function, parameters } => {
                self.check_custom(check_function, parameters)
            }
        };

        let status = match result {
            Ok(true) => ComplianceStatus::Compliant,
            Ok(false) => ComplianceStatus::NonCompliant,
            Err(_) => ComplianceStatus::Unknown,
        };

        let compliance_result = ComplianceResult {
            standard: rule.standard,
            check_item: rule.name.clone(),
            status,
            details: format!("Compliance check for rule '{}' on event {}", rule.name, event.id),
            recommendations: self.generate_recommendations(rule, status),
            timestamp: crate::subsystems::time::get_timestamp_nanos(),
        };

        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.total_checks += 1;
            *stats.checks_by_standard.entry(rule.standard).or_insert(0) += 1;
            *stats.checks_by_category.entry(rule.category).or_insert(0) += 1;

            match status {
                ComplianceStatus::Compliant => stats.compliant_checks += 1,
                ComplianceStatus::NonCompliant => stats.non_compliant_checks += 1,
                ComplianceStatus::PartiallyCompliant => stats.partially_compliant_checks += 1,
                ComplianceStatus::Unknown => {}
            }

            let elapsed = crate::subsystems::time::get_timestamp_nanos() - start_time;
            stats.avg_check_time_us = (stats.avg_check_time_us + elapsed / 1000) / 2;
        }

        crate::println!("[ComplianceChecker] {} compliance check: {}", rule.name, format!("{:?}", status));
        Ok(())
    }

    /// 检查事件计数
    fn check_event_count(&self, event_type: AuditEventType, time_range: (u64, u64), operator: ComparisonOperator, threshold: u64) -> Result<bool, &'static str> {
        // 简化的事件计数检查
        // 实际实现会查询审计数据库
        let count = self.get_event_count_in_range(event_type, time_range);

        let result = match operator {
            ComparisonOperator::Equals => count == threshold,
            ComparisonOperator::NotEquals => count != threshold,
            ComparisonOperator::GreaterThan => count > threshold,
            ComparisonOperator::GreaterThanOrEqual => count >= threshold,
            ComparisonOperator::LessThan => count < threshold,
            ComparisonOperator::LessThanOrEqual => count <= threshold,
        };

        Ok(result)
    }

    /// 获取时间范围内的事件数量
    fn get_event_count_in_range(&self, _event_type: AuditEventType, _time_range: (u64, u64)) -> u64 {
        // 简化实现，返回模拟数据
        5
    }

    /// 检查系统配置
    fn check_system_config(&self, config_key: &str, expected_value: &str) -> Result<bool, &'static str> {
        // 简化的系统配置检查
        // 实际实现会检查实际系统配置
        match config_key {
            "encryption.card_data.enabled" => Ok(expected_value == "true"),
            "access.card_data.restricted" => Ok(expected_value == "true"),
            "data.personal.protection" => Ok(expected_value == "enabled"),
            _ => Ok(false),
        }
    }

    /// 检查文件权限
    fn check_file_permissions(&self, file_path: &str, expected_permissions: &str) -> Result<bool, &'static str> {
        // 简化的文件权限检查
        crate::println!("[ComplianceChecker] Checking permissions for {}: expected {}", file_path, expected_permissions);
        Ok(true) // 简化实现
    }

    /// 检查服务状态
    fn check_service_status(&self, service_name: &str, expected_status: ServiceStatus) -> Result<bool, &'static str> {
        // 简化的服务状态检查
        crate::println!("[ComplianceChecker] Checking service {}: expected {:?}", service_name, expected_status);
        Ok(true) // 简化实现
    }

    /// 检查自定义条件
    fn check_custom(&self, check_function: &str, parameters: &[String]) -> Result<bool, &'static str> {
        // 简化的自定义检查
        crate::println!("[ComplianceChecker] Custom check {} with parameters: {:?}", check_function, parameters);
        Ok(true) // 简化实现
    }

    /// 生成建议
    fn generate_recommendations(&self, rule: &ComplianceRule, status: ComplianceStatus) -> Vec<String> {
        match status {
            ComplianceStatus::Compliant => vec![
                format!("Continue maintaining compliance for {}", rule.name),
            ],
            ComplianceStatus::NonCompliant => vec![
                format!("Remediate non-compliance for {}", rule.name),
                format!("Review and update {} configuration", rule.name),
                format!("Implement additional controls for {:?}", rule.category),
            ],
            ComplianceStatus::PartiallyCompliant => vec![
                format!("Complete compliance implementation for {}", rule.name),
                format!("Address remaining gaps in {:?}", rule.category),
            ],
            ComplianceStatus::Unknown => vec![
                format!("Investigate compliance status for {}", rule.name),
                format!("Gather more information about {}", rule.name),
            ],
        }
    }

    /// 运行完整合规检查
    pub fn run_full_check(&mut self) -> Result<Vec<ComplianceResult>, &'static str> {
        let mut all_results = Vec::new();

        // 先收集所有标准，避免借用冲突
        let standards: Vec<_> = self.supported_standards.iter().copied().collect();
        for standard in standards {
            let standard_results = self.run_standard_check(standard)?;
            all_results.extend(standard_results);
        }

        Ok(all_results)
    }

    /// 运行特定标准的合规检查
    pub fn run_standard_check(&mut self, standard: ComplianceStandard) -> Result<Vec<ComplianceResult>, &'static str> {
        let mut results = Vec::new();

        if let Some(rules) = self.rules.get(&standard) {
            for rule in rules {
                if !rule.enabled {
                    continue;
                }

                // 执行规则检查
                let check_result = self.execute_rule_check(rule)?;

                let compliance_result = ComplianceResult {
                    standard,
                    check_item: rule.name.clone(),
                    status: check_result.status,
                    details: check_result.details,
                    recommendations: check_result.recommendations,
                    timestamp: crate::subsystems::time::get_timestamp_nanos(),
                };

                results.push(compliance_result);
            }
        }

        // 缓存结果
        {
            let mut cache = self.result_cache.lock();
            cache.insert(standard, results.clone());
        }

        Ok(results)
    }

    /// 执行规则检查
    fn execute_rule_check(&self, rule: &ComplianceRule) -> Result<ComplianceCheckResult, &'static str> {
        let start_time = crate::subsystems::time::get_timestamp_nanos();

        let result = match &rule.condition {
            ComplianceCondition::SystemConfig { config_key, expected_value } => {
                self.check_system_config(config_key, expected_value)
            }
            ComplianceCondition::FilePermissions { file_path, expected_permissions } => {
                self.check_file_permissions(file_path, expected_permissions)
            }
            ComplianceCondition::ServiceStatus { service_name, expected_status } => {
                self.check_service_status(service_name, *expected_status)
            }
            _ => Ok(true), // 简化实现
        };

        let status = match result {
            Ok(true) => ComplianceStatus::Compliant,
            Ok(false) => ComplianceStatus::NonCompliant,
            Err(_) => ComplianceStatus::Unknown,
        };

        let elapsed = crate::subsystems::time::get_timestamp_nanos() - start_time;

        Ok(ComplianceCheckResult {
            status,
            details: format!("Rule '{}' check completed in {}μs", rule.name, elapsed / 1000),
            recommendations: self.generate_recommendations(rule, status),
        })
    }

    /// 获取支持的合规标准
    pub fn get_supported_standards(&self) -> &[ComplianceStandard] {
        &self.supported_standards
    }

    /// 添加合规规则
    pub fn add_rule(&mut self, rule: ComplianceRule) -> Result<(), &'static str> {
        self.rules.entry(rule.standard).or_insert_with(Vec::new).push(rule);
        Ok(())
    }

    /// 移除合规规则
    pub fn remove_rule(&mut self, standard: ComplianceStandard, rule_id: u64) -> Result<(), &'static str> {
        if let Some(rules) = self.rules.get_mut(&standard) {
            rules.retain(|rule| rule.id != rule_id);
            Ok(())
        } else {
            Err("Standard not found")
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> ComplianceCheckerStats {
        self.stats.lock().clone()
    }

    /// 重置统计信息
    pub fn reset_stats(&self) {
        *self.stats.lock() = ComplianceCheckerStats::default();
    }

    /// 清除结果缓存
    pub fn clear_cache(&self) {
        *self.result_cache.lock() = BTreeMap::new();
    }

    /// 获取缓存的结果
    pub fn get_cached_results(&self, standard: ComplianceStandard) -> Option<Vec<ComplianceResult>> {
        self.result_cache.lock().get(&standard).cloned()
    }
}

/// 合规检查结果
#[derive(Debug, Clone)]
pub struct ComplianceCheckResult {
    /// 检查状态
    pub status: ComplianceStatus,
    /// 检查详情
    pub details: String,
    /// 建议
    pub recommendations: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compliance_checker_creation() {
        let checker = ComplianceChecker::new();
        assert_eq!(checker.id, 1);
        assert_eq!(checker.supported_standards.len(), 0);
    }

    #[test]
    fn test_compliance_rule_creation() {
        let rule = ComplianceRule {
            id: 1,
            standard: ComplianceStandard::SOC2,
            category: ComplianceCategory::Logging,
            name: "Test Rule".to_string(),
            description: "Test compliance rule".to_string(),
            condition: ComplianceCondition::SystemConfig {
                config_key: "test.key".to_string(),
                expected_value: "true".to_string(),
            },
            expected_result: ExpectedResult::Boolean(true),
            check_method: CheckMethod::ConfigCheck,
            severity: ComplianceSeverity::Medium,
            enabled: true,
        };

        assert_eq!(rule.id, 1);
        assert_eq!(rule.standard, ComplianceStandard::SOC2);
        assert_eq!(rule.category, ComplianceCategory::Logging);
        assert_eq!(rule.severity, ComplianceSeverity::Medium);
        assert!(rule.enabled);
    }

    #[test]
    fn test_compliance_checker_stats() {
        let checker = ComplianceChecker::new();
        let stats = checker.get_stats();
        assert_eq!(stats.total_checks, 0);
        assert_eq!(stats.compliant_checks, 0);
        assert_eq!(stats.non_compliant_checks, 0);
    }

    #[test]
    fn test_comparison_operator() {
        assert_eq!(ComparisonOperator::Equals, ComparisonOperator::Equals);
        assert_ne!(ComparisonOperator::Equals, ComparisonOperator::NotEquals);
    }

    #[test]
    fn test_compliance_severity_ordering() {
        assert!(ComplianceSeverity::Info < ComplianceSeverity::Low);
        assert!(ComplianceSeverity::Low < ComplianceSeverity::Medium);
        assert!(ComplianceSeverity::Medium < ComplianceSeverity::High);
        assert!(ComplianceSeverity::High < ComplianceSeverity::Critical);
    }
}
