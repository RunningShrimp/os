//! Degradation session types

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::vec::Vec;

use super::types::{ExecutionStatus, LogLevel};
use super::actions::DegradationActionType;

/// 降级会话
#[derive(Debug, Clone)]
pub struct DegradationSession {
    /// 会话ID
    pub id: String,
    /// 策略ID
    pub strategy_id: String,
    /// 服务名称
    pub service_name: String,
    /// 开始时间
    pub start_time: u64,
    /// 结束时间
    pub end_time: Option<u64>,
    /// 会话状态
    pub status: DegradationStatus,
    /// 触发原因
    pub trigger_reason: String,
    /// 执行的动作
    pub executed_actions: Vec<ExecutedDegradationAction>,
    /// 降级效果
    pub degradation_effect: DegradationEffect,
    /// 恢复状态
    pub recovery_status: RecoveryStatus,
    /// 会话日志
    pub logs: Vec<SessionLog>,
}

/// 降级状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegradationStatus {
    /// 初始化中
    Initializing,
    /// 执行中
    Executing,
    /// 已降级
    Degraded,
    /// 恢复中
    Recovering,
    /// 已恢复
    Recovered,
    /// 失败
    Failed,
    /// 已取消
    Cancelled,
}

/// 执行的降级行动
#[derive(Debug, Clone)]
pub struct ExecutedDegradationAction {
    /// 动作ID
    pub action_id: String,
    /// 动作类型
    pub action_type: DegradationActionType,
    /// 开始时间
    pub start_time: u64,
    /// 结束时间
    pub end_time: Option<u64>,
    /// 执行状态
    pub status: ExecutionStatus,
    /// 执行结果
    pub result: Option<String>,
    /// 错误信息
    pub error_message: Option<String>,
}

/// 降级效果
#[derive(Debug, Clone)]
pub struct DegradationEffect {
    /// 性能改善
    pub performance_improvement: f64,
    /// 资源节省
    pub resource_savings: f64,
    /// 服务质量变化
    pub quality_change: QualityChange,
    /// 用户体验影响
    pub user_experience_impact: UserExperienceImpact,
    /// 业务影响
    pub business_impact: BusinessImpact,
}

/// 质量变化
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QualityChange {
    /// 提升
    Improved,
    /// 保持
    Maintained,
    /// 轻微下降
    SlightlyDecreased,
    /// 明显下降
    SignificantlyDecreased,
    /// 严重下降
    SeverelyDecreased,
}

/// 用户体验影响
#[derive(Debug, Clone)]
pub struct UserExperienceImpact {
    /// 响应时间变化（百分比）
    pub response_time_change_percent: f64,
    /// 功能完整性变化（百分比）
    pub functionality_completeness_percent: f64,
    /// 用户满意度影响（评分）
    pub satisfaction_impact: f64,
    /// 支持的用户数变化
    pub supported_users_change: i32,
}

/// 业务影响
#[derive(Debug, Clone)]
pub struct BusinessImpact {
    /// 收入影响（百分比）
    pub revenue_impact_percent: f64,
    /// 成本节省（美元）
    pub cost_savings: f64,
    /// SLA合规性影响
    pub sla_compliance_impact: SLAComplianceImpact,
    /// 客户流失风险
    pub customer_churn_risk: f64,
}

/// SLA合规性影响
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SLAComplianceImpact {
    /// 无影响
    None,
    /// 轻微影响
    Minor,
    /// 中等影响
    Moderate,
    /// 重大影响
    Major,
    /// 严重违约
    Violation,
}

/// 恢复状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStatus {
    /// 未开始
    NotStarted,
    /// 进行中
    InProgress,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 跳过
    Skipped,
}

/// 会话日志
#[derive(Debug, Clone)]
pub struct SessionLog {
    /// 日志ID
    pub id: String,
    /// 时间戳
    pub timestamp: u64,
    /// 日志级别
    pub level: LogLevel,
    /// 消息
    pub message: String,
    /// 详细信息
    pub details: Option<String>,
}

/// 降级统计
#[derive(Debug, Clone, Default)]
pub struct DegradationStats {
    /// 总降级次数
    pub total_degradations: u64,
    /// 自动降级次数
    pub auto_degradations: u64,
    /// 手动降级次数
    pub manual_degradations: u64,
    /// 平均降级时间（秒）
    pub avg_degradation_duration: u64,
    /// 成功恢复次数
    pub successful_recoveries: u64,
    /// 失败恢复次数
    pub failed_recoveries: u64,
    /// 按策略类型统计
    pub degradations_by_type: BTreeMap<String, u64>,
    /// 按服务统计
    pub degradations_by_service: BTreeMap<String, u64>,
    /// 用户影响统计
    pub user_impact_summary: UserImpactSummary,
}

/// 用户影响摘要
#[derive(Debug, Clone, Default)]
pub struct UserImpactSummary {
    /// 影响的用户总数
    pub total_affected_users: u64,
    /// 平均影响持续时间（分钟）
    pub avg_impact_duration: u64,
    /// 影响严重度分布
    pub severity_distribution: BTreeMap<String, u64>,
    /// 用户满意度变化
    pub satisfaction_change: f64,
}

/// 降级配置
#[derive(Debug, Clone)]
pub struct DegradationConfig {
    /// 启用自动降级
    pub enable_auto_degradation: bool,
    /// 默认降级策略
    pub default_strategies: Vec<String>,
    /// 最大并发降级数
    pub max_concurrent_degradations: u32,
    /// 降级历史保留数量
    pub degradation_history_size: usize,
    /// 启用预测性降级
    pub enable_predictive_degradation: bool,
    /// 降级前检查时间（秒）
    pub pre_degradation_check_time: u64,
    /// 最小降级持续时间（秒）
    pub min_degradation_duration: u64,
    /// 启用渐进式降级
    pub enable_gradual_degradation: bool,
    /// 用户影响阈值
    pub user_impact_threshold: f64,
}

impl Default for DegradationConfig {
    fn default() -> Self {
        Self {
            enable_auto_degradation: true,
            default_strategies: vec![
                "performance_degradation".to_string(),
                "feature_degradation".to_string(),
            ],
            max_concurrent_degradations: 5,
            degradation_history_size: 1000,
            enable_predictive_degradation: false,
            pre_degradation_check_time: 30,
            min_degradation_duration: 60,
            enable_gradual_degradation: true,
            user_impact_threshold: 0.1,
        }
    }
}
