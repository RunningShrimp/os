//! Diagnosis session types

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::types::LogLevel;
use super::patterns::{FaultSeverity, RootCause, ImpactScope, RemediationRecommendation};

/// 诊断会话
#[derive(Debug, Clone)]
pub struct DiagnosisSession {
    /// 会话ID
    pub id: u64,
    /// 会话名称
    pub name: String,
    /// 开始时间
    pub start_time: u64,
    /// 结束时间
    pub end_time: Option<u64>,
    /// 会话状态
    pub status: SessionStatus,
    /// 输入数据
    pub input_data: DiagnosisInput,
    /// 诊断结果
    pub diagnosis_results: Vec<DiagnosisResult>,
    /// 使用的规则
    pub applied_rules: Vec<String>,
    /// 置信度
    pub confidence: f64,
    /// 会话日志
    pub logs: Vec<SessionLog>,
}

/// 会话状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    /// 初始化中
    Initializing,
    /// 运行中
    Running,
    /// 已完成
    Completed,
    /// 已取消
    Cancelled,
    /// 出错
    Error,
}

/// 诊断输入
#[derive(Debug, Clone)]
pub struct DiagnosisInput {
    /// 症状数据
    pub symptom_data: Vec<SymptomData>,
    /// 系统指标
    pub system_metrics: BTreeMap<String, f64>,
    /// 错误日志
    pub error_logs: Vec<ErrorLog>,
    /// 性能数据
    pub performance_data: Vec<PerformanceDataPoint>,
    /// 配置信息
    pub configuration_info: BTreeMap<String, String>,
    /// 上下文信息
    pub context_info: BTreeMap<String, String>,
}

/// 症状数据
#[derive(Debug, Clone)]
pub struct SymptomData {
    /// 症状名称
    pub symptom_name: String,
    /// 症状值
    pub value: f64,
    /// 时间戳
    pub timestamp: u64,
    /// 严重级别
    pub severity: f64,
}

/// 错误日志
#[derive(Debug, Clone)]
pub struct ErrorLog {
    /// 日志ID
    pub id: String,
    /// 错误消息
    pub message: String,
    /// 错误级别
    pub level: String,
    /// 组件
    pub component: String,
    /// 时间戳
    pub timestamp: u64,
    /// 堆栈跟踪
    pub stack_trace: Option<String>,
}

/// 性能数据点
#[derive(Debug, Clone)]
pub struct PerformanceDataPoint {
    /// 指标名称
    pub metric_name: String,
    /// 指标值
    pub value: f64,
    /// 时间戳
    pub timestamp: u64,
    /// 单位
    pub unit: String,
}

/// 诊断结果
#[derive(Debug, Clone)]
pub struct DiagnosisResult {
    /// 结果ID
    pub id: String,
    /// 故障模式ID
    pub fault_pattern_id: String,
    /// 诊断置信度
    pub confidence: f64,
    /// 故障严重级别
    pub severity: FaultSeverity,
    /// 根本原因分析
    pub root_cause_analysis: Vec<RootCause>,
    /// 影响评估
    pub impact_assessment: ImpactScope,
    /// 推荐行动
    pub recommended_actions: Vec<RemediationRecommendation>,
    /// 预测信息
    pub prediction_info: Option<PredictionInfo>,
    /// 生成时间
    pub generated_at: u64,
    /// 结果摘要
    pub summary: String,
}

/// 预测信息
#[derive(Debug, Clone)]
pub struct PredictionInfo {
    /// 预测的故障概率
    pub predicted_fault_probability: f64,
    /// 预测的故障时间
    pub predicted_fault_time: Option<u64>,
    /// 预测的影响范围
    pub predicted_impact_scope: String,
    /// 预测的置信度
    pub prediction_confidence: f64,
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

/// 诊断统计
#[derive(Debug, Clone, Default)]
pub struct DiagnosisStats {
    /// 总诊断次数
    pub total_diagnoses: u64,
    /// 成功诊断次数
    pub successful_diagnoses: u64,
    /// 平均诊断时间（毫秒）
    pub avg_diagnosis_time_ms: u64,
    /// 平均置信度
    pub avg_confidence: f64,
    /// 最常见的故障模式
    pub most_common_faults: Vec<String>,
    /// 按类别统计
    pub diagnoses_by_category: BTreeMap<String, u64>,
    /// 按严重级别统计
    pub diagnoses_by_severity: BTreeMap<FaultSeverity, u64>,
    /// 准确率
    pub accuracy: f64,
}

/// 诊断配置
#[derive(Debug, Clone)]
pub struct DiagnosisConfig {
    /// 启用自动诊断
    pub enable_auto_diagnosis: bool,
    /// 最大并发诊断数
    pub max_concurrent_diagnoses: u32,
    /// 诊断超时时间（秒）
    pub diagnosis_timeout_seconds: u64,
    /// 最小置信度阈值
    pub min_confidence_threshold: f64,
    /// 启用预测功能
    pub enable_prediction: bool,
    /// 预测窗口（小时）
    pub prediction_window_hours: u64,
    /// 启用机器学习
    pub enable_machine_learning: bool,
    /// 诊断历史保留数量
    pub diagnosis_history_size: usize,
    /// 启用实时监控
    pub enable_real_time_monitoring: bool,
}

impl Default for DiagnosisConfig {
    fn default() -> Self {
        Self {
            enable_auto_diagnosis: true,
            max_concurrent_diagnoses: 5,
            diagnosis_timeout_seconds: 300,
            min_confidence_threshold: 0.7,
            enable_prediction: true,
            prediction_window_hours: 24,
            enable_machine_learning: false,
            diagnosis_history_size: 1000,
            enable_real_time_monitoring: true,
        }
    }
}
