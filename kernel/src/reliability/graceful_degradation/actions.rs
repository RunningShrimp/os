//! Degradation and recovery actions

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// 降级行动
#[derive(Debug, Clone)]
pub struct DegradationAction {
    /// 动作ID
    pub id: String,
    /// 动作类型
    pub action_type: DegradationActionType,
    /// 动作名称
    pub name: String,
    /// 动作描述
    pub description: String,
    /// 动作参数
    pub parameters: BTreeMap<String, String>,
    /// 执行顺序
    pub execution_order: u32,
    /// 是否必须执行
    pub mandatory: bool,
    /// 回滚动作
    pub rollback_actions: Vec<RollbackAction>,
}

/// 降级行动类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegradationActionType {
    /// 禁用功能
    DisableFeature,
    /// 降低质量
    ReduceQuality,
    /// 限制并发
    LimitConcurrency,
    /// 增加超时
    IncreaseTimeout,
    /// 启用缓存
    EnableCache,
    /// 负载均衡
    LoadBalance,
    /// 限流
    RateLimit,
    /// 数据压缩
    CompressData,
    /// 异步处理
    AsyncProcessing,
    /// 简化计算
    SimplifyComputation,
    /// 降级模式
    DegradedMode,
    /// 自定义动作
    CustomAction,
}

/// 恢复条件
#[derive(Debug, Clone)]
pub struct RecoveryCondition {
    /// 条件ID
    pub id: String,
    /// 条件类型
    pub condition_type: RecoveryConditionType,
    /// 条件描述
    pub description: String,
    /// 恢复阈值
    pub recovery_threshold: f64,
    /// 稳定时间（秒）
    pub stability_duration: u64,
    /// 自动恢复
    pub auto_recovery: bool,
}

/// 恢复条件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryConditionType {
    /// 资源充足
    ResourceSufficient,
    /// 负载降低
    LoadReduced,
    /// 错误率降低
    ErrorRateReduced,
    /// 性能恢复
    PerformanceRestored,
    /// 时间窗口
    TimeWindow,
    /// 手动恢复
    ManualRecovery,
    /// 自定义恢复
    CustomRecovery,
}

/// 回滚动作
#[derive(Debug, Clone)]
pub struct RollbackAction {
    /// 动作描述
    pub description: String,
    /// 动作类型
    pub action_type: RollbackActionType,
    /// 动作参数
    pub parameters: BTreeMap<String, String>,
}

/// 回滚动作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackActionType {
    /// 启用功能
    EnableFeature,
    /// 恢复质量
    RestoreQuality,
    /// 移除限制
    RemoveLimit,
    /// 恢复超时
    RestoreTimeout,
    /// 禁用缓存
    DisableCache,
    /// 停止负载均衡
    StopLoadBalancing,
    /// 移除限流
    RemoveRateLimit,
    /// 同步处理
    SynchronousProcessing,
    /// 完整计算
    FullComputation,
    /// 正常模式
    NormalMode,
}
