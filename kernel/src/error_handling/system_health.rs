//! System Health Module

extern crate alloc;
//
// 系统健康监控模块
// 提供系统健康状态监控、健康检查和健康评分功能

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;
use alloc::{format, vec};
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use super::*;
use crate::collections::HashMap;

/// 系统健康监控器
pub struct SystemHealthMonitor {
    /// 监控器ID
    pub id: u64,
    /// 健康指标
    health_metrics: BTreeMap<String, HealthMetric>,
    /// 健康检查规则
    health_checks: Vec<HealthCheck>,
    /// 系统组件状态
    component_status: BTreeMap<String, ComponentHealth>,
    /// 健康评分计算器
    health_scorer: HealthScorer,
    /// 监控配置
    config: HealthMonitoringConfig,
    /// 当前健康状态
    current_status: SystemHealthStatus,
    /// 健康历史
    health_history: Vec<HealthSnapshot>,
    /// 统计信息
    stats: HealthMonitoringStats,
    /// 检查计数器
    check_counter: AtomicU64,
}

/// 健康指标
#[derive(Debug, Clone)]
pub struct HealthMetric {
    /// 指标ID
    pub id: String,
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 指标单位
    pub unit: String,
    /// 当前值
    pub current_value: f64,
    /// 阈值配置
    pub thresholds: MetricThresholds,
    /// 指标状态
    pub status: MetricStatus,
    /// 最后更新时间
    pub last_updated: u64,
    /// 历史数据
    pub historical_data: Vec<f64>,
    /// 最大历史记录数
    pub max_history_size: usize,
}

/// 指标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    /// CPU使用率
    CPUUsage,
    /// 内存使用率
    MemoryUsage,
    /// 磁盘使用率
    DiskUsage,
    /// 网络吞吐量
    NetworkThroughput,
    /// 响应时间
    ResponseTime,
    /// 错误率
    ErrorRate,
    /// 吞吐量
    Throughput,
    /// 可用性
    Availability,
    /// 队列长度
    QueueLength,
    /// 连接数
    ConnectionCount,
    /// 自定义指标
    Custom,
}

/// 指标阈值
#[derive(Debug, Clone)]
pub struct MetricThresholds {
    /// 优秀阈值
    pub excellent: f64,
    /// 良好阈值
    pub good: f64,
    /// 警告阈值
    pub warning: f64,
    /// 严重阈值
    pub critical: f64,
    /// 最小值
    pub min_value: f64,
    /// 最大值
    pub max_value: f64,
}

/// 指标状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricStatus {
    /// 优秀
    Excellent,
    /// 良好
    Good,
    /// 警告
    Warning,
    /// 严重
    Critical,
    /// 未知
    Unknown,
}

/// 健康检查
#[derive(Debug, Clone)]
pub struct HealthCheck {
    /// 检查ID
    pub id: String,
    /// 检查名称
    pub name: String,
    /// 检查类型
    pub check_type: HealthCheckType,
    /// 检查目标
    pub target: String,
    /// 检查间隔（毫秒）
    pub interval_ms: u64,
    /// 超时时间（毫秒）
    pub timeout_ms: u64,
    /// 检查参数
    pub parameters: BTreeMap<String, String>,
    /// 成功条件
    pub success_criteria: Vec<SuccessCriterion>,
    /// 检查状态
    pub status: CheckStatus,
    /// 最后检查时间
    pub last_check: u64,
    /// 最后成功时间
    pub last_success: u64,
    /// 连续失败次数
    pub consecutive_failures: u32,
    /// 总检查次数
    pub total_checks: u64,
    /// 成功次数
    pub success_count: u64,
    /// 平均检查时间（毫秒）
    pub avg_check_time_ms: u64,
}

/// 健康检查类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthCheckType {
    /// HTTP检查
    HTTP,
    /// TCP检查
    TCP,
    /// ICMP检查
    ICMP,
    /// 程序检查
    Program,
    /// 数据库检查
    Database,
    /// 文件系统检查
    FileSystem,
    /// 进程检查
    Process,
    /// 系统调用检查
    SystemCall,
    /// 自定义检查
    Custom,
}

/// 成功条件
#[derive(Debug, Clone)]
pub struct SuccessCriterion {
    /// 条件类型
    pub criterion_type: CriterionType,
    /// 条件参数
    pub parameters: BTreeMap<String, String>,
    /// 期望值
    pub expected_value: String,
    /// 比较操作符
    pub operator: ComparisonOperator,
}

/// 条件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CriterionType {
    /// HTTP状态码
    HTTPStatusCode,
    /// 响应时间
    ResponseTime,
    /// 内容匹配
    ContentMatch,
    /// 正则表达式匹配
    RegexMatch,
    /// 文件存在
    FileExists,
    /// 进程运行
    ProcessRunning,
    /// 端口监听
    PortListening,
    /// 磁盘空间
    DiskSpace,
    /// 内存可用
    MemoryAvailable,
    /// 自定义条件
    CustomCriterion,
}

/// 比较操作符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
    /// 等于
    Equal,
    /// 不等于
    NotEqual,
    /// 大于
    GreaterThan,
    /// 大于等于
    GreaterThanOrEqual,
    /// 小于
    LessThan,
    /// 小于等于
    LessThanOrEqual,
    /// 包含
    Contains,
    /// 不包含
    NotContains,
    /// 正则匹配
    Regex,
}

/// 检查状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// 健康
    Healthy,
    /// 不健康
    Unhealthy,
    /// 未知
    Unknown,
    /// 禁用
    Disabled,
}

/// 组件健康状态
#[derive(Debug, Clone)]
pub struct ComponentHealth {
    /// 组件ID
    pub id: String,
    /// 组件名称
    pub name: String,
    /// 组件类型
    pub component_type: ComponentType,
    /// 健康状态
    pub health_status: ComponentHealthStatus,
    /// 关键性级别
    pub criticality: CriticalityLevel,
    /// 相关指标
    pub related_metrics: Vec<String>,
    /// 相关健康检查
    pub related_health_checks: Vec<String>,
    /// 依赖组件
    pub dependencies: Vec<String>,
    /// 最后更新时间
    pub last_updated: u64,
    /// 健康评分
    pub health_score: f64,
    /// 状态消息
    pub status_message: String,
}

/// 组件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentType {
    /// 服务
    Service,
    /// 数据库
    Database,
    /// 消息队列
    MessageQueue,
    /// 缓存
    Cache,
    /// 负载均衡器
    LoadBalancer,
    /// Web服务器
    WebServer,
    /// 应用服务器
    ApplicationServer,
    /// 系统服务
    SystemService,
    /// 硬件组件
    Hardware,
    /// 网络组件
    Network,
}

/// 组件健康状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentHealthStatus {
    /// 健康
    Healthy,
    /// 降级
    Degraded,
    /// 不健康
    Unhealthy,
    /// 维护中
    Maintenance,
    /// 未知
    Unknown,
}

/// 关键性级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CriticalityLevel {
    /// 关键
    Critical,
    /// 重要
    Important,
    /// 一般
    Normal,
    /// 可选
    Optional,
}

/// 健康评分计算器
#[derive(Debug, Clone)]
pub struct HealthScorer {
    /// 评分算法
    pub scoring_algorithm: ScoringAlgorithm,
    /// 权重配置
    pub weights: BTreeMap<String, f64>,
    /// 评分阈值
    pub score_thresholds: ScoreThresholds,
}

/// 评分算法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScoringAlgorithm {
    /// 加权平均
    WeightedAverage,
    /// 指数衰减
    ExponentialDecay,
    /// 移动平均
    MovingAverage,
    /// 机器学习模型
    MachineLearning,
    /// 自定义算法
    Custom,
}

/// 评分阈值
#[derive(Debug, Clone)]
pub struct ScoreThresholds {
    /// 优秀阈值
    pub excellent: f64,
    /// 良好阈值
    pub good: f64,
    /// 警告阈值
    pub warning: f64,
    /// 严重阈值
    pub critical: f64,
}

/// 系统健康状态
#[derive(Debug, Clone)]
pub struct SystemHealthStatus {
    /// 整体健康评分
    pub overall_score: f64,
    /// 健康等级
    pub health_level: HealthLevel,
    /// 健康组件数
    pub healthy_components: u32,
    /// 降级组件数
    pub degraded_components: u32,
    /// 不健康组件数
    pub unhealthy_components: u32,
    /// 未知组件数
    pub unknown_components: u32,
    /// 总组件数
    pub total_components: u32,
    /// 健康检查通过率
    pub health_check_pass_rate: f64,
    /// 关键问题
    pub critical_issues: Vec<CriticalIssue>,
    /// 生成时间
    pub timestamp: u64,
    /// 状态摘要
    pub summary: String,
}

/// 健康等级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthLevel {
    /// 优秀
    Excellent,
    /// 良好
    Good,
    /// 警告
    Warning,
    /// 严重
    Critical,
    /// 未知
    Unknown,
}

/// 关键问题
#[derive(Debug, Clone)]
pub struct CriticalIssue {
    /// 问题ID
    pub id: String,
    /// 问题类型
    pub issue_type: IssueType,
    /// 严重级别
    pub severity: ErrorSeverity,
    /// 影响组件
    pub affected_components: Vec<String>,
    /// 问题描述
    pub description: String,
    /// 建议解决方案
    pub recommended_actions: Vec<String>,
    /// 发现时间
    pub discovered_at: u64,
    /// 是否已解决
    pub resolved: bool,
    /// 解决时间
    pub resolved_at: Option<u64>,
}

/// 问题类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IssueType {
    /// 性能问题
    Performance,
    /// 可用性问题
    Availability,
    /// 资源不足
    ResourceShortage,
    /// 错误率过高
    HighErrorRate,
    /// 容量问题
    Capacity,
    /// 安全问题
    Security,
    /// 配置问题
    Configuration,
    /// 网络问题
    Network,
    /// 依赖问题
    Dependency,
    /// 其他问题
    Other,
}

/// 健康快照
#[derive(Debug, Clone)]
pub struct HealthSnapshot {
    /// 快照ID
    pub id: String,
    /// 时间戳
    pub timestamp: u64,
    /// 系统健康状态
    pub system_status: SystemHealthStatus,
    /// 组件健康状态
    pub component_status: BTreeMap<String, ComponentHealth>,
    /// 健康指标值
    pub metric_values: BTreeMap<String, f64>,
    /// 健康检查结果
    pub health_check_results: BTreeMap<String, HealthCheckResult>,
}

/// 健康检查结果
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// 检查ID
    pub check_id: String,
    /// 检查时间
    pub timestamp: u64,
    /// 检查结果
    pub result: CheckResult,
    /// 响应时间（毫秒）
    pub response_time_ms: u64,
    /// 错误消息
    pub error_message: Option<String>,
    /// 详细信息
    pub details: BTreeMap<String, String>,
}

/// 检查结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckResult {
    /// 通过
    Pass,
    /// 失败
    Fail,
    /// 超时
    Timeout,
    /// 跳过
    Skip,
}

/// 健康监控统计
#[derive(Debug, Clone, Default)]
pub struct HealthMonitoringStats {
    /// 总健康检查次数
    pub total_health_checks: u64,
    /// 成功检查次数
    pub successful_checks: u64,
    /// 失败检查次数
    pub failed_checks: u64,
    /// 平均检查时间（毫秒）
    pub avg_check_time_ms: u64,
    /// 关键问题数
    pub critical_issues: u64,
    /// 已解决问题数
    pub resolved_issues: u64,
    /// 健康评分历史
    pub health_score_history: Vec<f64>,
    /// 按组件统计
    pub checks_by_component: BTreeMap<String, u64>,
}

/// 健康监控配置
#[derive(Debug, Clone)]
pub struct HealthMonitoringConfig {
    /// 启用自动健康检查
    pub enable_auto_health_check: bool,
    /// 检查间隔（毫秒）
    pub check_interval_ms: u64,
    /// 健康历史保留数量
    pub health_history_size: usize,
    /// 指标历史保留数量
    pub metric_history_size: usize,
    /// 启用健康评分计算
    pub enable_health_scoring: bool,
    /// 评分更新间隔（毫秒）
    pub scoring_interval_ms: u64,
    /// 关键问题阈值
    pub critical_issue_threshold: u32,
    /// 启用自动修复
    pub enable_auto_remediation: bool,
    /// 通知阈值
    pub notification_threshold: f64,
}

impl Default for HealthMonitoringConfig {
    fn default() -> Self {
        Self {
            enable_auto_health_check: true,
            check_interval_ms: 30000, // 30秒
            health_history_size: 1000,
            metric_history_size: 100,
            enable_health_scoring: true,
            scoring_interval_ms: 60000, // 1分钟
            critical_issue_threshold: 1,
            enable_auto_remediation: false,
            notification_threshold: 0.7,
        }
    }
}

impl Default for ScoreThresholds {
    fn default() -> Self {
        Self {
            excellent: 0.9,
            good: 0.7,
            warning: 0.5,
            critical: 0.3,
        }
    }
}

impl SystemHealthMonitor {
    /// 创建新的系统健康监控器
    pub fn new() -> Self {
        Self {
            id: 1,
            health_metrics: BTreeMap::new(),
            health_checks: Vec::new(),
            component_status: BTreeMap::new(),
            health_scorer: HealthScorer {
                scoring_algorithm: ScoringAlgorithm::WeightedAverage,
                weights: BTreeMap::new(),
                score_thresholds: ScoreThresholds::default(),
            },
            config: HealthMonitoringConfig::default(),
            current_status: SystemHealthStatus {
                overall_score: 1.0,
                health_level: HealthLevel::Excellent,
                healthy_components: 0,
                degraded_components: 0,
                unhealthy_components: 0,
                unknown_components: 0,
                total_components: 0,
                health_check_pass_rate: 1.0,
                critical_issues: Vec::new(),
                timestamp: crate::time::get_timestamp(),
                summary: "System is healthy".to_string(),
            },
            health_history: Vec::new(),
            stats: HealthMonitoringStats::default(),
            check_counter: AtomicU64::new(1),
        }
    }

    /// 初始化系统健康监控器
    pub fn init(&mut self) -> Result<(), &'static str> {
        // 初始化默认健康指标
        self.initialize_default_metrics()?;

        // 初始化默认健康检查
        self.initialize_default_health_checks()?;

        // 初始化默认组件状态
        self.initialize_default_components()?;

        // 初始化评分权重
        self.initialize_scoring_weights()?;

        crate::println!("[SystemHealth] System health monitor initialized successfully");
        Ok(())
    }

    /// 执行健康检查
    pub fn perform_health_check(&mut self) -> Result<Vec<HealthCheckResult>, &'static str> {
        let mut results = Vec::new();
        let start_time = crate::time::get_timestamp();

        // 执行所有健康检查
        // Execute health checks by index to avoid borrowing conflict
        let indices_to_run: Vec<_> = self.health_checks.iter()
            .enumerate()
            .filter(|(_, hc)| hc.status != CheckStatus::Disabled)
            .map(|(i, _)| i)
            .collect();

        for index in indices_to_run {
            match self.execute_health_check_by_index(index) {
                Ok(result) => {
                    results.push(result);
                }
                Err(e) => {
                    crate::println!("[SystemHealth] Failed to execute health check at index {}: {}", index, e);
                    // 创建一个失败的结果
                    let failed_result = HealthCheckResult {
                        check_id: format!("check_{}", index),
                        timestamp: crate::time::get_timestamp(),
                        result: CheckResult::Fail,
                        response_time_ms: 0,
                        error_message: Some(e.to_string()),
                        details: BTreeMap::new(),
                    };
                    results.push(failed_result);
                }
            }
        }

        // 更新系统健康状态
        self.update_system_health_status(&results)?;

        // 生成健康快照
        self.create_health_snapshot()?;

        // 更新统计信息
        let check_time = crate::time::get_timestamp() - start_time;
        self.update_health_monitoring_stats(&results, check_time);

        Ok(results)
    }

    /// 执行单个健康检查
    fn execute_health_check(&mut self, health_check: &mut HealthCheck) -> Result<HealthCheckResult, &'static str> {
        let start_time = crate::time::get_timestamp();
        health_check.last_check = start_time;
        health_check.total_checks += 1;

        let check_start = crate::time::get_timestamp();
        let (result, error_message) = match health_check.check_type {
            HealthCheckType::HTTP => self.check_http_endpoint(health_check),
            HealthCheckType::TCP => self.check_tcp_connection(health_check),
            HealthCheckType::Process => self.check_process_status(health_check),
            HealthCheckType::FileSystem => self.check_filesystem(health_check),
            HealthCheckType::Custom => self.check_custom(health_check),
            _ => (CheckResult::Pass, None),
        };

        let response_time = (crate::time::get_timestamp() - check_start) * 1000; // 转换为毫秒

        // 更新检查状态
        if result == CheckResult::Pass {
            health_check.last_success = start_time;
            health_check.consecutive_failures = 0;
            health_check.success_count += 1;
            health_check.status = CheckStatus::Healthy;
        } else {
            health_check.consecutive_failures += 1;
            if health_check.consecutive_failures >= 3 {
                health_check.status = CheckStatus::Unhealthy;
            }
        }

        // 更新平均检查时间
        let total_time = health_check.avg_check_time_ms * (health_check.total_checks - 1) + response_time;
        health_check.avg_check_time_ms = total_time / health_check.total_checks;

        let check_result = HealthCheckResult {
            check_id: health_check.id.clone(),
            timestamp: start_time,
            result,
            response_time_ms: response_time,
            error_message,
            details: BTreeMap::new(),
        };

        Ok(check_result)
    }

    /// 按索引执行健康检查
    fn execute_health_check_by_index(&mut self, index: usize) -> Result<HealthCheckResult, &'static str> {
        if index >= self.health_checks.len() {
            return Err("Health check index out of bounds");
        }

        // 克隆健康检查以避免借用冲突
        let health_check_clone = self.health_checks[index].clone();
        let mut health_check = health_check_clone;

        // 执行健康检查
        let result = self.execute_health_check(&mut health_check)?;

        // 更新原健康检查的状态
        if let Some(original_check) = self.health_checks.get_mut(index) {
            original_check.last_check = health_check.last_check;
            original_check.total_checks = health_check.total_checks;
            original_check.consecutive_failures = health_check.consecutive_failures;
            original_check.last_success = health_check.last_success;
            original_check.avg_check_time_ms = health_check.avg_check_time_ms;
        }

        Ok(result)
    }

    /// HTTP端点检查
    fn check_http_endpoint(&self, health_check: &HealthCheck) -> (CheckResult, Option<String>) {
        // 模拟HTTP检查
        // 在实际实现中，这里会发送HTTP请求
        if health_check.target.contains("healthy") {
            (CheckResult::Pass, None)
        } else {
            (CheckResult::Fail, Some("HTTP endpoint returned error".to_string()))
        }
    }

    /// TCP连接检查
    fn check_tcp_connection(&self, health_check: &HealthCheck) -> (CheckResult, Option<String>) {
        // 模拟TCP连接检查
        // 在实际实现中，这里会建立TCP连接
        if health_check.target.contains("localhost") {
            (CheckResult::Pass, None)
        } else {
            (CheckResult::Fail, Some("TCP connection failed".to_string()))
        }
    }

    /// 进程状态检查
    fn check_process_status(&self, health_check: &HealthCheck) -> (CheckResult, Option<String>) {
        // 模拟进程状态检查
        // 在实际实现中，这里会检查进程是否运行
        (CheckResult::Pass, None)
    }

    /// 文件系统检查
    fn check_filesystem(&self, health_check: &HealthCheck) -> (CheckResult, Option<String>) {
        // 模拟文件系统检查
        // 在实际实现中，这里会检查文件系统状态
        (CheckResult::Pass, None)
    }

    /// 自定义检查
    fn check_custom(&self, health_check: &HealthCheck) -> (CheckResult, Option<String>) {
        // 模拟自定义检查
        (CheckResult::Pass, None)
    }

    /// 更新系统健康状态
    fn update_system_health_status(&mut self, results: &[HealthCheckResult]) -> Result<(), &'static str> {
        // 统计健康检查结果
        let total_checks = results.len() as u64;
        let successful_checks = results.iter().filter(|r| r.result == CheckResult::Pass).count() as u64;
        let pass_rate = if total_checks > 0 {
            successful_checks as f64 / total_checks as f64
        } else {
            1.0
        };

        // 统计组件健康状态
        let mut healthy_count = 0;
        let mut degraded_count = 0;
        let mut unhealthy_count = 0;
        let mut unknown_count = 0;

        for component in self.component_status.values() {
            match component.health_status {
                ComponentHealthStatus::Healthy => healthy_count += 1,
                ComponentHealthStatus::Degraded => degraded_count += 1,
                ComponentHealthStatus::Unhealthy => unhealthy_count += 1,
                ComponentHealthStatus::Maintenance | ComponentHealthStatus::Unknown => unknown_count += 1,
            }
        }

        let total_components = (healthy_count + degraded_count + unhealthy_count + unknown_count) as u32;

        // 计算健康评分
        let health_score = self.calculate_health_score()?;

        // 确定健康等级
        let health_level = self.determine_health_level(health_score);

        // 识别关键问题
        let critical_issues = self.identify_critical_issues(results);

        // 生成状态摘要
        let summary = self.generate_status_summary(health_level, healthy_count, total_components);

        self.current_status = SystemHealthStatus {
            overall_score: health_score,
            health_level,
            healthy_components: healthy_count,
            degraded_components: degraded_count,
            unhealthy_components: unhealthy_count,
            unknown_components: unknown_count,
            total_components,
            health_check_pass_rate: pass_rate,
            critical_issues,
            timestamp: crate::time::get_timestamp(),
            summary,
        };

        Ok(())
    }

    /// 计算健康评分
    fn calculate_health_score(&self) -> Result<f64, &'static str> {
        match self.health_scorer.scoring_algorithm {
            ScoringAlgorithm::WeightedAverage => {
                let mut weighted_sum = 0.0;
                let mut total_weight = 0.0;

                // 基于组件健康评分计算
                for component in self.component_status.values() {
                    let weight = self.health_scorer.weights
                        .get(&component.id)
                        .copied()
                        .unwrap_or(1.0);
                    weighted_sum += component.health_score * weight;
                    total_weight += weight;
                }

                if total_weight > 0.0 {
                    Ok(weighted_sum / total_weight)
                } else {
                    Ok(1.0)
                }
            }
            ScoringAlgorithm::ExponentialDecay => {
                // 实现指数衰减算法
                Ok(0.8)
            }
            ScoringAlgorithm::MovingAverage => {
                // 实现移动平均算法
                Ok(0.85)
            }
            ScoringAlgorithm::MachineLearning => {
                // 实现机器学习评分
                Ok(0.9)
            }
            ScoringAlgorithm::Custom => {
                // 实现自定义评分算法
                Ok(0.75)
            }
        }
    }

    /// 确定健康等级
    fn determine_health_level(&self, score: f64) -> HealthLevel {
        let thresholds = &self.health_scorer.score_thresholds;

        if score >= thresholds.excellent {
            HealthLevel::Excellent
        } else if score >= thresholds.good {
            HealthLevel::Good
        } else if score >= thresholds.warning {
            HealthLevel::Warning
        } else if score >= thresholds.critical {
            HealthLevel::Critical
        } else {
            HealthLevel::Unknown
        }
    }

    /// 识别关键问题
    fn identify_critical_issues(&self, results: &[HealthCheckResult]) -> Vec<CriticalIssue> {
        let mut issues = Vec::new();

        // 基于健康检查结果识别问题
        for result in results {
            if result.result == CheckResult::Fail {
                let issue = CriticalIssue {
                    id: format!("issue_{}", result.check_id),
                    issue_type: IssueType::Availability,
                    severity: ErrorSeverity::Error,
                    affected_components: vec![result.check_id.clone()],
                    description: format!("Health check failed: {}", result.check_id),
                    recommended_actions: vec![
                        "Investigate component status".to_string(),
                        "Check system logs".to_string(),
                    ],
                    discovered_at: result.timestamp,
                    resolved: false,
                    resolved_at: None,
                };
                issues.push(issue);
            }
        }

        // 基于组件状态识别问题
        for (component_id, component) in &self.component_status {
            if component.health_status == ComponentHealthStatus::Unhealthy {
                let issue = CriticalIssue {
                    id: format!("component_issue_{}", component_id),
                    issue_type: IssueType::Availability,
                    severity: ErrorSeverity::Critical,
                    affected_components: vec![component_id.clone()],
                    description: format!("Component {} is unhealthy", component.name),
                    recommended_actions: vec![
                        "Restart component".to_string(),
                        "Check component dependencies".to_string(),
                    ],
                    discovered_at: component.last_updated,
                    resolved: false,
                    resolved_at: None,
                };
                issues.push(issue);
            }
        }

        issues
    }

    /// 生成状态摘要
    fn generate_status_summary(&self, health_level: HealthLevel, healthy_count: u32, total_components: u32) -> String {
        let health_percentage = if total_components > 0 {
            (healthy_count as f64 / total_components as f64) * 100.0
        } else {
            100.0
        };

        match health_level {
            HealthLevel::Excellent => format!("System is excellent ({}% components healthy)", health_percentage),
            HealthLevel::Good => format!("System is good ({}% components healthy)", health_percentage),
            HealthLevel::Warning => format!("System has warnings ({}% components healthy)", health_percentage),
            HealthLevel::Critical => format!("System is critical ({}% components healthy)", health_percentage),
            HealthLevel::Unknown => "System health is unknown".to_string(),
        }
    }

    /// 创建健康快照
    fn create_health_snapshot(&mut self) -> Result<(), &'static str> {
        let snapshot_id = format!("snapshot_{}", self.check_counter.fetch_add(1, Ordering::SeqCst));

        let metric_values: BTreeMap<_, _> = self.health_metrics
            .iter()
            .map(|(id, metric)| (id.clone(), metric.current_value))
            .collect();

        let health_check_results: BTreeMap<_, _> = self.health_checks
            .iter()
            .map(|check| {
                let result = HealthCheckResult {
                    check_id: check.id.clone(),
                    timestamp: check.last_check,
                    result: if check.status == CheckStatus::Healthy { CheckResult::Pass } else { CheckResult::Fail },
                    response_time_ms: check.avg_check_time_ms,
                    error_message: None,
                    details: BTreeMap::new(),
                };
                (check.id.clone(), result)
            })
            .collect();

        let snapshot = HealthSnapshot {
            id: snapshot_id,
            timestamp: crate::time::get_timestamp(),
            system_status: self.current_status.clone(),
            component_status: self.component_status.clone(),
            metric_values,
            health_check_results,
        };

        self.health_history.push(snapshot);

        // 限制历史记录数量
        if self.health_history.len() > self.config.health_history_size {
            self.health_history.remove(0);
        }

        Ok(())
    }

    /// 更新健康监控统计
    fn update_health_monitoring_stats(&mut self, results: &[HealthCheckResult], total_check_time: u64) {
        self.stats.total_health_checks += results.len() as u64;
        self.stats.successful_checks += results.iter().filter(|r| r.result == CheckResult::Pass).count() as u64;
        self.stats.failed_checks += results.iter().filter(|r| r.result != CheckResult::Pass).count() as u64;

        // 更新平均检查时间
        if !results.is_empty() {
            let total_time = self.stats.avg_check_time_ms * (self.stats.total_health_checks - results.len() as u64) + total_check_time;
            self.stats.avg_check_time_ms = total_time / self.stats.total_health_checks;
        }

        // 更新健康评分历史
        self.stats.health_score_history.push(self.current_status.overall_score);
        if self.stats.health_score_history.len() > 100 {
            self.stats.health_score_history.remove(0);
        }

        // 更新关键问题数
        self.stats.critical_issues = self.current_status.critical_issues.len() as u64;
    }

    /// 更新健康指标
    pub fn update_metric(&mut self, metric_id: &str, value: f64) -> Result<(), &'static str> {
        let metric = self.health_metrics.get_mut(metric_id)
            .ok_or("Metric not found")?;

        // 添加到历史数据
        metric.historical_data.push(value);
        if metric.historical_data.len() > metric.max_history_size {
            metric.historical_data.remove(0);
        }

        // 更新当前值和状态
        let thresholds = metric.thresholds.clone();
        metric.current_value = value;
        metric.status = Self::determine_metric_status_static(value, &thresholds);
        metric.last_updated = crate::time::get_timestamp();

        // 更新相关组件健康状态
        self.update_related_components_health(metric_id)?;

        Ok(())
    }

    /// 确定指标状态
    fn determine_metric_status(&self, value: f64, thresholds: &MetricThresholds) -> MetricStatus {
        Self::determine_metric_status_static(value, thresholds)
    }

    /// 静态方法确定指标状态
    fn determine_metric_status_static(value: f64, thresholds: &MetricThresholds) -> MetricStatus {
        if value >= thresholds.excellent {
            MetricStatus::Excellent
        } else if value >= thresholds.good {
            MetricStatus::Good
        } else if value >= thresholds.warning {
            MetricStatus::Warning
        } else if value >= thresholds.critical {
            MetricStatus::Critical
        } else {
            MetricStatus::Unknown
        }
    }

    /// 更新相关组件健康状态
    fn update_related_components_health(&mut self, metric_id: &str) -> Result<(), &'static str> {
        // 提取health_metrics避免借用冲突
        let health_metrics = &self.health_metrics;

        for (component_id, component) in &mut self.component_status {
            if component.related_metrics.contains(&metric_id.to_string()) {
                // 提取组件相关指标以避免借用冲突
                let related_metrics: Vec<String> = component.related_metrics.clone();

                // 基于相关指标重新计算组件健康评分
                component.health_score = Self::calculate_component_health_score_static(component, &related_metrics, health_metrics).unwrap_or(0.0);
                component.last_updated = crate::time::get_timestamp();

                // 更新组件健康状态
                component.health_status = Self::determine_component_health_status_static(component.health_score);
            }
        }

        Ok(())
    }

    /// 计算组件健康评分
    fn calculate_component_health_score(&self, component: &ComponentHealth) -> Result<f64, &'static str> {
        let mut total_score = 0.0;
        let mut count = 0;

        // 基于相关指标计算评分
        for metric_id in &component.related_metrics {
            if let Some(metric) = self.health_metrics.get(metric_id) {
                let score = self.metric_to_score(metric.current_value, &metric.thresholds);
                total_score += score;
                count += 1;
            }
        }

        if count > 0 {
            Ok(total_score / count as f64)
        } else {
            Ok(1.0) // 默认健康评分
        }
    }

    /// 指标值转换为评分
    fn metric_to_score(&self, value: f64, thresholds: &MetricThresholds) -> f64 {
        if value >= thresholds.excellent {
            1.0
        } else if value >= thresholds.good {
            0.8
        } else if value >= thresholds.warning {
            0.6
        } else if value >= thresholds.critical {
            0.4
        } else {
            0.2
        }
    }

    /// 确定组件健康状态
    fn determine_component_health_status(&self, health_score: f64) -> ComponentHealthStatus {
        Self::determine_component_health_status_static(health_score)
    }

    /// 静态方法计算组件健康评分
    fn calculate_component_health_score_static(
        component: &ComponentHealth,
        related_metrics: &[String],
        health_metrics: &BTreeMap<String, HealthMetric>,
    ) -> Result<f64, &'static str> {
        let mut total_score = 0.0;
        let mut count = 0;

        // 基于相关指标计算评分
        for metric_id in related_metrics {
            if let Some(metric) = health_metrics.get(metric_id) {
                let score = Self::metric_to_score_static(metric.current_value, &metric.thresholds);
                total_score += score;
                count += 1;
            }
        }

        if count > 0 {
            Ok(total_score / count as f64)
        } else {
            Ok(1.0) // 默认健康评分
        }
    }

    /// 静态方法指标值转换为评分
    fn metric_to_score_static(value: f64, thresholds: &MetricThresholds) -> f64 {
        if value >= thresholds.excellent {
            1.0
        } else if value >= thresholds.good {
            0.8
        } else if value >= thresholds.warning {
            0.6
        } else if value >= thresholds.critical {
            0.4
        } else {
            0.2
        }
    }

    /// 静态方法确定组件健康状态
    fn determine_component_health_status_static(health_score: f64) -> ComponentHealthStatus {
        if health_score >= 0.8 {
            ComponentHealthStatus::Healthy
        } else if health_score >= 0.6 {
            ComponentHealthStatus::Degraded
        } else if health_score >= 0.4 {
            ComponentHealthStatus::Unhealthy
        } else {
            ComponentHealthStatus::Unknown
        }
    }

    /// 添加健康指标
    pub fn add_metric(&mut self, metric: HealthMetric) -> Result<(), &'static str> {
        self.health_metrics.insert(metric.id.clone(), metric);
        Ok(())
    }

    /// 添加健康检查
    pub fn add_health_check(&mut self, health_check: HealthCheck) -> Result<(), &'static str> {
        self.health_checks.push(health_check);
        Ok(())
    }

    /// 添加组件
    pub fn add_component(&mut self, component: ComponentHealth) -> Result<(), &'static str> {
        self.component_status.insert(component.id.clone(), component);
        self.current_status.total_components += 1;
        Ok(())
    }

    /// 获取当前健康状态
    pub fn get_current_status(&self) -> SystemHealthStatus {
        self.current_status.clone()
    }

    /// 获取健康历史
    pub fn get_health_history(&self, limit: Option<usize>) -> Vec<&HealthSnapshot> {
        let mut history = self.health_history.iter().collect::<Vec<_>>();
        history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        if let Some(limit) = limit {
            history.truncate(limit);
        }

        history
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> HealthMonitoringStats {
        self.stats.clone()
    }

    /// 初始化默认健康指标
    fn initialize_default_metrics(&mut self) -> Result<(), &'static str> {
        let metrics = vec![
            HealthMetric {
                id: "cpu_usage".to_string(),
                name: "CPU Usage".to_string(),
                metric_type: MetricType::CPUUsage,
                unit: "%".to_string(),
                current_value: 0.0,
                thresholds: MetricThresholds {
                    excellent: 80.0,
                    good: 70.0,
                    warning: 85.0,
                    critical: 95.0,
                    min_value: 0.0,
                    max_value: 100.0,
                },
                status: MetricStatus::Unknown,
                last_updated: crate::time::get_timestamp(),
                historical_data: Vec::new(),
                max_history_size: 100,
            },
            HealthMetric {
                id: "memory_usage".to_string(),
                name: "Memory Usage".to_string(),
                metric_type: MetricType::MemoryUsage,
                unit: "%".to_string(),
                current_value: 0.0,
                thresholds: MetricThresholds {
                    excellent: 70.0,
                    good: 80.0,
                    warning: 85.0,
                    critical: 95.0,
                    min_value: 0.0,
                    max_value: 100.0,
                },
                status: MetricStatus::Unknown,
                last_updated: crate::time::get_timestamp(),
                historical_data: Vec::new(),
                max_history_size: 100,
            },
            HealthMetric {
                id: "error_rate".to_string(),
                name: "Error Rate".to_string(),
                metric_type: MetricType::ErrorRate,
                unit: "%".to_string(),
                current_value: 0.0,
                thresholds: MetricThresholds {
                    excellent: 1.0,
                    good: 2.0,
                    warning: 5.0,
                    critical: 10.0,
                    min_value: 0.0,
                    max_value: 100.0,
                },
                status: MetricStatus::Unknown,
                last_updated: crate::time::get_timestamp(),
                historical_data: Vec::new(),
                max_history_size: 100,
            },
        ];

        for metric in metrics {
            self.health_metrics.insert(metric.id.clone(), metric);
        }

        Ok(())
    }

    /// 初始化默认健康检查
    fn initialize_default_health_checks(&mut self) -> Result<(), &'static str> {
        let health_checks = vec![
            HealthCheck {
                id: "system_process_check".to_string(),
                name: "System Process Check".to_string(),
                check_type: HealthCheckType::Process,
                target: "kernel".to_string(),
                interval_ms: 30000,
                timeout_ms: 5000,
                parameters: BTreeMap::new(),
                success_criteria: vec![
                    SuccessCriterion {
                        criterion_type: CriterionType::ProcessRunning,
                        parameters: BTreeMap::new(),
                        expected_value: "running".to_string(),
                        operator: ComparisonOperator::Equal,
                    },
                ],
                status: CheckStatus::Healthy,
                last_check: crate::time::get_timestamp(),
                last_success: crate::time::get_timestamp(),
                consecutive_failures: 0,
                total_checks: 0,
                success_count: 0,
                avg_check_time_ms: 0,
            },
            HealthCheck {
                id: "filesystem_check".to_string(),
                name: "Filesystem Check".to_string(),
                check_type: HealthCheckType::FileSystem,
                target: "/".to_string(),
                interval_ms: 60000,
                timeout_ms: 10000,
                parameters: BTreeMap::new(),
                success_criteria: vec![
                    SuccessCriterion {
                        criterion_type: CriterionType::DiskSpace,
                        parameters: BTreeMap::new(),
                        expected_value: "90".to_string(),
                        operator: ComparisonOperator::LessThan,
                    },
                ],
                status: CheckStatus::Healthy,
                last_check: crate::time::get_timestamp(),
                last_success: crate::time::get_timestamp(),
                consecutive_failures: 0,
                total_checks: 0,
                success_count: 0,
                avg_check_time_ms: 0,
            },
        ];

        self.health_checks = health_checks;
        Ok(())
    }

    /// 初始化默认组件
    fn initialize_default_components(&mut self) -> Result<(), &'static str> {
        let components = vec![
            ComponentHealth {
                id: "kernel".to_string(),
                name: "Kernel".to_string(),
                component_type: ComponentType::SystemService,
                health_status: ComponentHealthStatus::Healthy,
                criticality: CriticalityLevel::Critical,
                related_metrics: vec!["cpu_usage".to_string(), "memory_usage".to_string()],
                related_health_checks: vec!["system_process_check".to_string()],
                dependencies: Vec::new(),
                last_updated: crate::time::get_timestamp(),
                health_score: 1.0,
                status_message: "Kernel is running normally".to_string(),
            },
            ComponentHealth {
                id: "filesystem".to_string(),
                name: "Filesystem".to_string(),
                component_type: ComponentType::SystemService,
                health_status: ComponentHealthStatus::Healthy,
                criticality: CriticalityLevel::Critical,
                related_metrics: vec!["error_rate".to_string()],
                related_health_checks: vec!["filesystem_check".to_string()],
                dependencies: vec!["kernel".to_string()],
                last_updated: crate::time::get_timestamp(),
                health_score: 1.0,
                status_message: "Filesystem is healthy".to_string(),
            },
        ];

        for component in components {
            self.component_status.insert(component.id.clone(), component);
            self.current_status.total_components += 1;
        }

        Ok(())
    }

    /// 初始化评分权重
    fn initialize_scoring_weights(&mut self) -> Result<(), &'static str> {
        self.health_scorer.weights.insert("kernel".to_string(), 0.4);
        self.health_scorer.weights.insert("filesystem".to_string(), 0.3);
        self.health_scorer.weights.insert("network".to_string(), 0.2);
        self.health_scorer.weights.insert("memory".to_string(), 0.1);
        Ok(())
    }

    /// 停止系统健康监控器
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        self.health_metrics.clear();
        self.health_checks.clear();
        self.component_status.clear();
        self.health_history.clear();

        crate::println!("[SystemHealth] System health monitor shutdown successfully");
        Ok(())
    }
}

/// 创建默认的系统健康监控器
pub fn create_system_health_monitor() -> Arc<Mutex<SystemHealthMonitor>> {
    Arc::new(Mutex::new(SystemHealthMonitor::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_health_monitor_creation() {
        let monitor = SystemHealthMonitor::new();
        assert_eq!(monitor.id, 1);
        assert!(monitor.health_metrics.is_empty());
        assert!(monitor.health_checks.is_empty());
    }

    #[test]
    fn test_health_level_determination() {
        let monitor = SystemHealthMonitor::new();

        assert_eq!(monitor.determine_health_level(0.95), HealthLevel::Excellent);
        assert_eq!(monitor.determine_health_level(0.75), HealthLevel::Good);
        assert_eq!(monitor.determine_health_level(0.55), HealthLevel::Warning);
        assert_eq!(monitor.determine_health_level(0.35), HealthLevel::Critical);
        assert_eq!(monitor.determine_health_level(0.15), HealthLevel::Unknown);
    }

    #[test]
    fn test_health_monitoring_config_default() {
        let config = HealthMonitoringConfig::default();
        assert!(config.enable_auto_health_check);
        assert_eq!(config.check_interval_ms, 30000);
        assert!(config.enable_health_scoring);
    }
}