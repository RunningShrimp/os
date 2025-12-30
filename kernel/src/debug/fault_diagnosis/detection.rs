//! Detection methods and diagnosis rules

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use super::types::{ConditionType, TriggerCondition};

/// 诊断规则
#[derive(Debug, Clone)]
pub struct DiagnosisRule {
    /// 规则ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 规则描述
    pub description: String,
    /// 规则类型
    pub rule_type: DiagnosisRuleType,
    /// 触发条件
    pub trigger_conditions: Vec<TriggerCondition>,
    /// 诊断逻辑
    pub diagnosis_logic: DiagnosisLogic,
    /// 置信度权重
    pub confidence_weight: f64,
    /// 优先级
    pub priority: u32,
    /// 启用状态
    pub enabled: bool,
    /// 规则统计
    pub stats: RuleStats,
}

/// 诊断规则类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosisRuleType {
    /// 规则引擎
    RuleEngine,
    /// 机器学习模型
    MachineLearning,
    /// 专家系统
    ExpertSystem,
    /// 统计分析
    Statistical,
    /// 异常检测
    AnomalyDetection,
    /// 因果分析
    CausalAnalysis,
}

/// 诊断逻辑
#[derive(Debug, Clone)]
pub enum DiagnosisLogic {
    /// 简单匹配
    SimpleMatch { patterns: Vec<String> },
    /// 复杂规则
    ComplexRule { conditions: Vec<LogicCondition>, operator: LogicOperator },
    /// 决策树
    DecisionTree { tree: DecisionTreeNode },
    /// 贝叶斯网络
    BayesianNetwork { nodes: Vec<BayesianNode>, edges: Vec<BayesianEdge> },
}

/// 逻辑条件
#[derive(Debug, Clone)]
pub struct LogicCondition {
    /// 条件字段
    pub field: String,
    /// 操作符
    pub operator: LogicOperator,
    /// 值
    pub value: String,
    /// 权重
    pub weight: f64,
}

/// 逻辑操作符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicOperator {
    /// 等于
    Equals,
    /// 不等于
    NotEquals,
    /// 大于
    GreaterThan,
    /// 小于
    LessThan,
    /// 大于等于
    GreaterThanOrEqual,
    /// 小于等于
    LessThanOrEqual,
    /// 包含
    Contains,
    /// 正则匹配
    Regex,
    /// 逻辑与
    And,
    /// 逻辑或
    Or,
    /// 逻辑非
    Not,
}

/// 决策树节点
#[derive(Debug, Clone)]
pub struct DecisionTreeNode {
    /// 节点ID
    pub id: String,
    /// 节点类型
    pub node_type: NodeType,
    /// 条件特征
    pub feature: Option<String>,
    /// 分裂值
    pub split_value: Option<f64>,
    /// 左子节点
    pub left_child: Option<Box<DecisionTreeNode>>,
    /// 右子节点
    pub right_child: Option<Box<DecisionTreeNode>>,
    /// 预测结果
    pub prediction: Option<DiagnosisResult>,
    /// 置信度
    pub confidence: f64,
}

/// 节点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    /// 根节点
    Root,
    /// 内部节点
    Internal,
    /// 叶子节点
    Leaf,
}

/// 贝叶斯节点
#[derive(Debug, Clone)]
pub struct BayesianNode {
    /// 节点ID
    pub id: String,
    /// 节点名称
    pub name: String,
    /// 可能状态
    pub states: Vec<String>,
    /// 条件概率表
    pub cpt: BTreeMap<String, f64>,
    /// 父节点ID
    pub parents: Vec<String>,
}

/// 贝叶斯边
#[derive(Debug, Clone)]
pub struct BayesianEdge {
    /// 源节点
    pub from: String,
    /// 目标节点
    pub to: String,
    /// 因果强度
    pub strength: f64,
}

/// 规则统计
#[derive(Debug, Clone, Default)]
pub struct RuleStats {
    /// 触发次数
    pub trigger_count: u64,
    /// 成功诊断次数
    pub successful_diagnoses: u64,
    /// 准确率
    pub accuracy: f64,
    /// 平均置信度
    pub avg_confidence: f64,
    /// 最后触发时间
    pub last_triggered: u64,
}

/// 诊断结果（placeholder for forward reference）
#[derive(Debug, Clone)]
pub struct DiagnosisResult {
    /// 结果ID
    pub id: String,
    /// 置信度
    pub confidence: f64,
    /// 结果摘要
    pub summary: String,
}
