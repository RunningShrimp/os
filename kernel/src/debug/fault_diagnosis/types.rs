//! Basic type definitions for fault diagnosis

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// 条件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionType {
    /// 错误率阈值
    ErrorRateThreshold,
    /// 性能下降
    PerformanceDegradation,
    /// 资源耗尽
    ResourceExhaustion,
    /// 服务不可用
    ServiceUnavailable,
    /// 网络分区
    NetworkPartition,
    /// 数据不一致
    DataInconsistency,
    /// 自定义条件
    CustomCondition,
}

/// 触发条件
#[derive(Debug, Clone)]
pub struct TriggerCondition {
    /// 条件ID
    pub id: String,
    /// 条件类型
    pub condition_type: ConditionType,
    /// 条件参数
    pub parameters: BTreeMap<String, String>,
    /// 阈值
    pub threshold: f64,
    /// 时间窗口（秒）
    pub time_window_seconds: u64,
}

/// 日志级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// 调试
    Debug,
    /// 信息
    Info,
    /// 警告
    Warning,
    /// 错误
    Error,
    /// 致命
    Fatal,
}
