//! Service quality control types

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::types::ConditionType;

/// 服务质量控制器
#[derive(Debug, Clone)]
pub struct ServiceQualityController {
    /// 控制器ID
    pub id: String,
    /// 服务名称
    pub service_name: String,
    /// 质量指标
    pub quality_metrics: BTreeMap<String, QualityMetric>,
    /// 质量阈值
    pub quality_thresholds: BTreeMap<String, QualityThreshold>,
    /// 控制策略
    pub control_policies: Vec<QualityControlPolicy>,
    /// 当前质量等级
    pub current_quality_level: QualityLevel,
    /// 质量历史
    pub quality_history: Vec<QualitySnapshot>,
    /// 控制器状态
    pub status: ControllerStatus,
}

/// 质量指标
#[derive(Debug, Clone)]
pub struct QualityMetric {
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 当前值
    pub current_value: f64,
    /// 目标值
    pub target_value: f64,
    /// 单位
    pub unit: String,
    /// 权重
    pub weight: f64,
    /// 更新时间
    pub last_updated: u64,
}

/// 指标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    /// 响应时间
    ResponseTime,
    /// 吞吐量
    Throughput,
    /// 错误率
    ErrorRate,
    /// 可用性
    Availability,
    /// 资源使用率
    ResourceUsage,
    /// 队列长度
    QueueLength,
    /// 并发数
    Concurrency,
}

/// 质量阈值
#[derive(Debug, Clone)]
pub struct QualityThreshold {
    /// 最优阈值
    pub optimal: f64,
    /// 可接受阈值
    pub acceptable: f64,
    /// 警告阈值
    pub warning: f64,
    /// 降级阈值
    pub degradation: f64,
    /// 严重降级阈值
    pub severe_degradation: f64,
}

impl Default for QualityThreshold {
    fn default() -> Self {
        Self {
            optimal: 50.0,
            acceptable: 75.0,
            warning: 85.0,
            degradation: 90.0,
            severe_degradation: 95.0,
        }
    }
}

/// 质量控制策略
#[derive(Debug, Clone)]
pub struct QualityControlPolicy {
    /// 策略ID
    pub id: String,
    /// 策略名称
    pub name: String,
    /// 控制条件
    pub control_conditions: Vec<ControlCondition>,
    /// 控制动作
    pub control_actions: Vec<ControlAction>,
    /// 策略优先级
    pub priority: u32,
    /// 启用状态
    pub enabled: bool,
}

/// 控制条件
#[derive(Debug, Clone)]
pub struct ControlCondition {
    /// 条件类型
    pub condition_type: ConditionType,
    /// 指标名称
    pub metric_name: String,
    /// 操作符
    pub operator: ComparisonOperator,
    /// 阈值
    pub threshold: f64,
    /// 持续时间（秒）
    pub duration: u64,
}

/// 比较操作符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
    /// 大于
    GreaterThan,
    /// 大于等于
    GreaterThanOrEqual,
    /// 小于
    LessThan,
    /// 小于等于
    LessThanOrEqual,
    /// 等于
    Equal,
    /// 不等于
    NotEqual,
}

/// 控制动作
#[derive(Debug, Clone)]
pub struct ControlAction {
    /// 动作类型
    pub action_type: ControlActionType,
    /// 动作参数
    pub parameters: BTreeMap<String, String>,
    /// 延迟执行（秒）
    pub delay_seconds: u64,
}

/// 控制动作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlActionType {
    /// 调整参数
    AdjustParameter,
    /// 启用缓存
    EnableCache,
    /// 增加超时
    IncreaseTimeout,
    /// 限制并发
    LimitConcurrency,
    /// 启用限流
    EnableRateLimit,
    /// 切换算法
    SwitchAlgorithm,
    /// 启用降级模式
    EnableDegradedMode,
}

/// 质量等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum QualityLevel {
    /// 优秀
    Excellent = 5,
    /// 良好
    Good = 4,
    /// 可接受
    Acceptable = 3,
    /// 降级
    Degraded = 2,
    /// 严重降级
    SeverelyDegraded = 1,
    /// 不可用
    Unavailable = 0,
}

/// 质量快照
#[derive(Debug, Clone)]
pub struct QualitySnapshot {
    /// 时间戳
    pub timestamp: u64,
    /// 质量等级
    pub quality_level: QualityLevel,
    /// 指标值
    pub metric_values: BTreeMap<String, f64>,
    /// 服务状态
    pub service_status: ServiceStatus,
    /// 用户影响
    pub user_impact: UserImpact,
}

/// 控制器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControllerStatus {
    /// 活动
    Active,
    /// 降级中
    Degraded,
    /// 维护中
    Maintenance,
    /// 停用
    Disabled,
}

/// 服务状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServiceStatus {
    /// 正常
    Normal,
    /// 警告
    Warning,
    /// 降级
    Degraded,
    /// 不可用
    Unavailable,
}

/// 用户影响
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UserImpact {
    /// 无影响
    None,
    /// 轻微影响
    Minor,
    /// 中等影响
    Moderate,
    /// 严重影响
    Severe,
}
