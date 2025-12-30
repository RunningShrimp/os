//! Fault patterns and related types

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// 故障模式
#[derive(Debug, Clone)]
pub struct FaultPattern {
    /// 模式ID
    pub id: String,
    /// 模式名称
    pub name: String,
    /// 模式描述
    pub description: String,
    /// 模式类别
    pub category: FaultCategory,
    /// 严重级别
    pub severity: FaultSeverity,
    /// 故障特征
    pub characteristics: Vec<FaultCharacteristic>,
    /// 前兆症状
    pub premonition_symptoms: Vec<Symptom>,
    /// 根本原因
    pub root_causes: Vec<RootCause>,
    /// 影响范围
    pub impact_scope: ImpactScope,
    /// 检测方法
    pub detection_methods: Vec<DetectionMethod>,
    /// 修复建议
    pub remediation_recommendations: Vec<RemediationRecommendation>,
    /// 发生频率
    pub frequency: f64,
    /// 检测置信度
    pub detection_confidence: f64,
}

/// 故障类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultCategory {
    /// 硬件故障
    Hardware,
    /// 软件故障
    Software,
    /// 网络故障
    Network,
    /// 存储故障
    Storage,
    /// 配置故障
    Configuration,
    /// 安全故障
    Security,
    /// 性能故障
    Performance,
    /// 资源故障
    Resource,
    /// 依赖故障
    Dependency,
    /// 人为故障
    Human,
}

/// 故障严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FaultSeverity {
    /// 信息级
    Info = 1,
    /// 警告级
    Warning = 2,
    /// 错误级
    Error = 3,
    /// 严重级
    Critical = 4,
    /// 灾难级
    Catastrophic = 5,
}

/// 故障特征
#[derive(Debug, Clone)]
pub struct FaultCharacteristic {
    /// 特征名称
    pub name: String,
    /// 特征值
    pub value: String,
    /// 特征类型
    pub feature_type: FeatureType,
    /// 重要度
    pub importance: f64,
}

/// 特征类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureType {
    /// 数值特征
    Numerical,
    /// 分类特征
    Categorical,
    /// 时间序列特征
    TimeSeries,
    /// 文本特征
    Text,
    /// 图像特征
    Image,
}

/// 症状
#[derive(Debug, Clone)]
pub struct Symptom {
    /// 症状ID
    pub id: String,
    /// 症状名称
    pub name: String,
    /// 症状描述
    pub description: String,
    /// 症状类型
    pub symptom_type: SymptomType,
    /// 检测指标
    pub detection_metrics: Vec<String>,
    /// 出现概率
    pub occurrence_probability: f64,
    /// 持续时间（秒）
    pub duration_seconds: u64,
}

/// 症状类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymptomType {
    /// 性能下降
    PerformanceDegradation,
    /// 错误增加
    IncreasedErrors,
    /// 响应延迟
    IncreasedLatency,
    /// 资源耗尽
    ResourceExhaustion,
    /// 服务中断
    ServiceDisruption,
    /// 数据异常
    DataAnomaly,
    /// 行为异常
    BehaviorAnomaly,
}

/// 根本原因
#[derive(Debug, Clone)]
pub struct RootCause {
    /// 原因ID
    pub id: String,
    /// 原因描述
    pub description: String,
    /// 原因类别
    pub cause_category: CauseCategory,
    /// 可能性
    pub probability: f64,
    /// 证据链
    pub evidence_chain: Vec<Evidence>,
    /// 修复复杂度
    pub fix_complexity: FixComplexity,
    /// 预计修复时间（小时）
    pub estimated_fix_time_hours: u64,
}

/// 原因类别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CauseCategory {
    /// 设计缺陷
    DesignFlaw,
    /// 实现错误
    ImplementationError,
    /// 配置错误
    ConfigurationError,
    /// 环境因素
    EnvironmentalFactor,
    /// 资源限制
    ResourceLimitation,
    /// 外部依赖
    ExternalDependency,
    /// 人为错误
    HumanError,
    /// 未知原因
    Unknown,
}

/// 证据
#[derive(Debug, Clone)]
pub struct Evidence {
    /// 证据ID
    pub id: String,
    /// 证据类型
    pub evidence_type: EvidenceType,
    /// 证据内容
    pub content: String,
    /// 时间戳
    pub timestamp: u64,
    /// 可靠性评分
    pub reliability_score: f64,
    /// 权重
    pub weight: f64,
}

/// 证据类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceType {
    /// 系统日志
    SystemLog,
    /// 性能指标
    PerformanceMetric,
    /// 错误消息
    ErrorMessage,
    /// 用户报告
    UserReport,
    /// 监控告警
    MonitoringAlert,
    /// 配置变更
    ConfigurationChange,
}

/// 修复复杂度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixComplexity {
    /// 简单
    Simple,
    /// 中等
    Medium,
    /// 复杂
    Complex,
    /// 非常复杂
    VeryComplex,
}

/// 影响范围
#[derive(Debug, Clone)]
pub struct ImpactScope {
    /// 影响的组件
    pub affected_components: Vec<String>,
    /// 影响的用户数
    pub affected_users: u64,
    /// 业务影响等级
    pub business_impact: BusinessImpact,
    /// 影响持续时间（分钟）
    pub impact_duration_minutes: u64,
    /// 财务影响
    pub financial_impact: FinancialImpact,
}

/// 业务影响
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BusinessImpact {
    /// 无影响
    None,
    /// 轻微影响
    Minor,
    /// 中等影响
    Moderate,
    /// 重大影响
    Major,
    /// 严重影响
    Severe,
}

/// 财务影响
#[derive(Debug, Clone)]
pub struct FinancialImpact {
    /// 直接损失（美元）
    pub direct_loss: f64,
    /// 间接损失（美元）
    pub indirect_loss: f64,
    /// 恢复成本（美元）
    pub recovery_cost: f64,
    /// 声誉影响评分
    pub reputation_impact_score: f64,
}

/// 检测方法
#[derive(Debug, Clone)]
pub struct DetectionMethod {
    /// 方法ID
    pub id: String,
    /// 方法名称
    pub name: String,
    /// 方法类型
    pub method_type: DetectionMethodType,
    /// 检测指标
    pub detection_metrics: Vec<String>,
    /// 检测阈值
    pub detection_thresholds: BTreeMap<String, f64>,
    /// 检测频率（秒）
    pub detection_frequency_seconds: u64,
    /// 准确率
    pub accuracy: f64,
}

/// 检测方法类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DetectionMethodType {
    /// 基于阈值
    ThresholdBased,
    /// 基于模式
    PatternBased,
    /// 基于异常检测
    AnomalyDetection,
    /// 基于机器学习
    MachineLearning,
    /// 基于规则
    RuleBased,
    /// 基于统计
    Statistical,
}

/// 修复建议
#[derive(Debug, Clone)]
pub struct RemediationRecommendation {
    /// 建议ID
    pub id: String,
    /// 建议描述
    pub description: String,
    /// 建议类型
    pub recommendation_type: RecommendationType,
    /// 优先级
    pub priority: RecommendationPriority,
    /// 实施步骤
    pub implementation_steps: Vec<String>,
    /// 预期效果
    pub expected_outcome: String,
    /// 风险评估
    pub risk_assessment: RiskAssessment,
    /// 所需资源
    pub required_resources: Vec<String>,
    /// 预计实施时间（小时）
    pub estimated_implementation_time_hours: u64,
    /// 成功率
    pub success_rate: f64,
}

/// 建议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecommendationType {
    /// 立即修复
    ImmediateFix,
    /// 临时解决方案
    TemporarySolution,
    /// 永久修复
    PermanentFix,
    /// 预防措施
    PreventiveMeasure,
    /// 系统升级
    SystemUpgrade,
    /// 配置变更
    ConfigurationChange,
    /// 流程改进
    ProcessImprovement,
}

/// 建议优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecommendationPriority {
    /// 紧急
    Urgent,
    /// 高
    High,
    /// 中
    Medium,
    /// 低
    Low,
}

/// 风险评估
#[derive(Debug, Clone)]
pub struct RiskAssessment {
    /// 技术风险
    pub technical_risk: f64,
    /// 业务风险
    pub business_risk: f64,
    /// 安全风险
    pub security_risk: f64,
    /// 财务风险
    pub financial_risk: f64,
    /// 总体风险评级
    pub overall_risk_rating: RiskRating,
    /// 风险缓解措施
    pub mitigation_measures: Vec<String>,
}

/// 风险评级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RiskRating {
    /// 低风险
    Low,
    /// 中等风险
    Medium,
    /// 高风险
    High,
    /// 极高风险
    Critical,
}
