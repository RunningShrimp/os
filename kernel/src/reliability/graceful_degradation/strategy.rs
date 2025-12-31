//! Degradation strategy types and structures

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::actions::{DegradationAction, RecoveryCondition};

/// 降级策略
#[derive(Debug, Clone)]
pub struct DegradationStrategy {
    /// 策略ID
    pub id: String,
    /// 策略名称
    pub name: String,
    /// 策略描述
    pub description: String,
    /// 策略类型
    pub strategy_type: DegradationType,
    /// 触发条件
    pub trigger_conditions: Vec<DegradationTrigger>,
    /// 降级行动
    pub degradation_actions: Vec<DegradationAction>,
    /// 恢复条件
    pub recovery_conditions: Vec<RecoveryCondition>,
    /// 优先级
    pub priority: u32,
    /// 启用状态
    pub enabled: bool,
    /// 策略参数
    pub parameters: BTreeMap<String, String>,
    /// 策略统计
    pub stats: StrategyStats,
}

/// 降级类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegradationType {
    /// 功能降级
    FeatureDegradation,
    /// 性能降级
    PerformanceDegradation,
    /// 容量降级
    CapacityDegradation,
    /// 可用性降级
    AvailabilityDegradation,
    /// 混合降级
    HybridDegradation,
    /// 自适应降级
    AdaptiveDegradation,
}

/// 降级触发器
#[derive(Debug, Clone)]
pub struct DegradationTrigger {
    /// 触发器ID
    pub id: String,
    /// 触发器类型
    pub trigger_type: TriggerType,
    /// 触发条件
    pub condition: TriggerCondition,
    /// 触发阈值
    pub threshold: f64,
    /// 持续时间（秒）
    pub duration_seconds: u64,
    /// 是否立即触发
    pub immediate: bool,
}

/// 触发器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TriggerType {
    /// 基于阈值
    ThresholdBased,
    /// 基于趋势
    TrendBased,
    /// 基于预测
    PredictionBased,
    /// 基于事件
    EventBased,
    /// 基于时间
    TimeBased,
    /// 基于负载
    LoadBased,
    /// 基于错误率
    ErrorRateBased,
}

/// 触发条件
#[derive(Debug, Clone)]
pub enum TriggerCondition {
    /// CPU使用率
    CPUUsage {
        threshold: f64,
        duration: u64,
    },
    /// 内存使用率
    MemoryUsage {
        threshold: f64,
        duration: u64,
    },
    /// 响应时间
    ResponseTime {
        threshold: f64,
        percentile: u8,
    },
    /// 错误率
    ErrorRate {
        threshold: f64,
        window: u64,
    },
    /// 队列长度
    QueueLength {
        threshold: u32,
    },
    /// 并发用户数
    ConcurrentUsers {
        threshold: u32,
    },
    /// 自定义条件
    Custom {
        condition: String,
        parameters: BTreeMap<String, String>,
    },
}

/// 策略统计
#[derive(Debug, Clone, Default)]
pub struct StrategyStats {
    /// 触发次数
    pub trigger_count: u64,
    /// 成功恢复次数
    pub successful_recoveries: u64,
    /// 平均降级时间（秒）
    pub avg_degradation_duration: u64,
    /// 最大降级时间（秒）
    pub max_degradation_duration: u64,
    /// 最小降级时间（秒）
    pub min_degradation_duration: u64,
    /// 最后触发时间
    pub last_triggered: u64,
    /// 降级效果评分
    pub effectiveness_score: f64,
}
