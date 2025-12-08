//! Error Classifier Module
//!
//! 错误分类器模块
//! 自动分类和分析错误

extern crate alloc;
extern crate hashbrown;
use crate::sync::{SpinLock, Mutex};
use hashbrown::HashMap;
use crate::compat::DefaultHasherBuilder;
use crate::time::SystemTime;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;
use alloc::{format, vec};
use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};

use super::*;

/// 错误分类器
pub struct ErrorClassifier {
    /// 分类器ID
    pub id: u64,
    /// 分类规则
    classification_rules: Vec<ClassificationRule>,
    /// 机器学习模型
    ml_model: Option<MLModel>,
    /// 错误模式
    error_patterns: HashMap<String, ErrorPattern, DefaultHasherBuilder>,
    /// 分类统计
    stats: ClassificationStats,
}

/// 分类规则
#[derive(Debug, Clone)]
pub struct ClassificationRule {
    /// 规则ID
    pub id: u64,
    /// 规则名称
    pub name: String,
    /// 规则描述
    pub description: String,
    /// 匹配条件
    pub conditions: Vec<ClassificationCondition>,
    /// 分类结果
    pub classification: ErrorClassification,
    /// 优先级
    pub priority: u8,
    /// 是否启用
    pub enabled: bool,
    /// 匹配次数
    pub match_count: u64,
    /// 创建时间
    pub created_at: u64,
}

/// 分类条件
#[derive(Debug, Clone)]
pub enum ClassificationCondition {
    /// 错误代码匹配
    ErrorCodeMatch(u32),
    /// 错误代码范围
    ErrorCodeRange(u32, u32),
    /// 错误类型匹配
    ErrorTypeMatch(ErrorType),
    /// 错误类别匹配
    CategoryMatch(ErrorCategory),
    /// 严重级别匹配
    SeverityMatch(ErrorSeverity),
    /// 错误消息包含关键词
    MessageContains(String),
    /// 错误源匹配
    SourceMatch(String),
    /// 错误上下文匹配
    ContextMatch(String),
    /// 错误频率阈值
    FrequencyThreshold(u32),
    /// 时间窗口匹配
    TimeWindowMatch(u64, u64),
    /// 组合条件
    And(Vec<ClassificationCondition>),
    /// 或条件
    Or(Vec<ClassificationCondition>),
    /// 非条件
    Not(Box<ClassificationCondition>),
}

/// 错误分类
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorClassification {
    /// 分类ID
    pub id: String,
    /// 分类名称
    pub name: String,
    /// 分类类型
    pub classification_type: ClassificationType,
    /// 严重级别
    pub severity: ErrorSeverity,
    /// 紧急程度
    pub urgency: Urgency,
    /// 影响范围
    pub impact_scope: ImpactScope,
    /// 恢复复杂度
    pub recovery_complexity: RecoveryComplexity,
    /// 是否需要立即处理
    pub requires_immediate_action: bool,
    /// 是否可自动恢复
    pub auto_recoverable: bool,
    /// 建议的响应时间
    pub recommended_response_time: u64, // 秒
    /// 相关标签
    pub tags: Vec<String>,
    /// 分类置信度
    pub confidence: f64,
}

/// 分类类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ClassificationType {
    /// 硬件故障
    HardwareFault,
    /// 软件缺陷
    SoftwareBug,
    /// 配置错误
    ConfigurationError,
    /// 网络问题
    NetworkIssue,
    /// 资源不足
    ResourceExhaustion,
    /// 用户错误
    UserError,
    /// 安全威胁
    SecurityThreat,
    /// 性能问题
    PerformanceIssue,
    /// 数据损坏
    DataCorruption,
    /// 系统过载
    SystemOverload,
    /// 未知问题
    UnknownIssue,
}

/// 紧急程度
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Urgency {
    /// 低
    Low = 1,
    /// 中
    Medium = 2,
    /// 高
    High = 3,
    /// 紧急
    Critical = 4,
}

/// 影响范围
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImpactScope {
    /// 本地
    Local,
    /// 系统级
    System,
    /// 网络级
    Network,
    /// 用户级
    User,
    /// 应用级
    Application,
    /// 数据级
    Data,
    /// 安全级
    Security,
}

/// 恢复复杂度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryComplexity {
    /// 简单（可自动恢复）
    Simple,
    /// 中等（需要人工干预）
    Medium,
    /// 复杂（需要专家处理）
    Complex,
    /// 极复杂（需要多部门协作）
    VeryComplex,
}

/// 错误模式
#[derive(Debug, Clone)]
pub struct ErrorPattern {
    /// 模式ID
    pub id: String,
    /// 模式名称
    pub name: String,
    /// 模式类型
    pub pattern_type: PatternType,
    /// 模式表达式
    pub expression: String,
    /// 匹配权重
    pub weight: f64,
    /// 出现频率
    pub frequency: f64,
    /// 相关错误
    pub related_errors: Vec<u32>,
    /// 常见原因
    pub common_causes: Vec<String>,
    /// 检测方法
    pub detection_methods: Vec<String>,
    /// 预防措施
    pub prevention_measures: Vec<String>,
}

/// 模式类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternType {
    /// 时间模式
    Temporal,
    /// 频率模式
    Frequency,
    /// 序列模式
    Sequence,
    /// 相关性模式
    Correlation,
    /// 异常模式
    Anomaly,
    /// 趋势模式
    Trend,
}

/// 机器学习模型
#[derive(Debug, Clone)]
pub struct MLModel {
    /// 模型ID
    pub id: String,
    /// 模型类型
    pub model_type: MLModelType,
    /// 模型版本
    pub version: String,
    /// 特征提取器
    pub feature_extractor: FeatureExtractor,
    /// 分类算法
    pub algorithm: ClassificationAlgorithm,
    /// 模型精度
    pub accuracy: f64,
    /// 训练样本数
    pub training_samples: u64,
    /// 最后更新时间
    pub last_updated: u64,
}

/// 机器学习模型类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MLModelType {
    /// 决策树
    DecisionTree,
    /// 随机森林
    RandomForest,
    /// 支持向量机
    SVM,
    /// 神经网络
    NeuralNetwork,
    /// 朴素贝叶斯
    NaiveBayes,
    /// K近邻
    KNN,
    /// 逻辑回归
    LogisticRegression,
}

/// 特征提取器
#[derive(Debug, Clone)]
pub struct FeatureExtractor {
    /// 提取器ID
    pub id: String,
    /// 特征列表
    pub features: Vec<Feature>,
    /// 提取方法
    pub extraction_method: ExtractionMethod,
}

/// 特征
#[derive(Debug, Clone)]
pub struct Feature {
    /// 特征名称
    pub name: String,
    /// 特征类型
    pub feature_type: FeatureType,
    /// 特征值
    pub value: FeatureValue,
    /// 重要性权重
    pub importance: f64,
}

/// 特征类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureType {
    /// 数值型
    Numeric,
    /// 分类型
    Categorical,
    /// 布尔型
    Boolean,
    /// 文本型
    Text,
    /// 时间型
    Temporal,
}

/// 特征值
#[derive(Debug, Clone)]
pub enum FeatureValue {
    /// 整数值
    Integer(i64),
    /// 浮点值
    Float(f64),
    /// 布尔值
    Boolean(bool),
    /// 字符串
    String(String),
    /// 数组
    Array(Vec<FeatureValue>),
    /// 空值
    Null,
}

/// 提取方法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtractionMethod {
    /// 统计方法
    Statistical,
    /// 启发式方法
    Heuristic,
    /// 机器学习方法
    MachineLearning,
    /// 规则方法
    RuleBased,
}

/// 分类算法
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassificationAlgorithm {
    /// 基于规则
    RuleBased,
    /// 基于统计
    Statistical,
    /// 基于机器学习
    MachineLearning,
    /// 混合方法
    Hybrid,
}

/// 分类统计
#[derive(Debug)]
pub struct ClassificationStats {
    /// 总分类次数
    pub total_classifications: u64,
    /// 按类型统计
    pub classifications_by_type: HashMap<ClassificationType, u64, DefaultHasherBuilder>,
    /// 按严重级别统计
    pub classifications_by_severity: HashMap<ErrorSeverity, u64, DefaultHasherBuilder>,
    /// 平均分类时间（微秒）
    pub avg_classification_time_us: u64,
    /// 分类准确率
    pub classification_accuracy: f64,
    /// 误分类率
    pub misclassification_rate: f64,
    /// 模式匹配次数
    pub pattern_matches: u64,
    /// 规则匹配次数
    pub rule_matches: u64,
    /// 机器学习预测次数
    pub ml_predictions: u64,
}

impl Default for ClassificationStats {
    fn default() -> Self {
        Self {
            total_classifications: 0,
            classifications_by_type: HashMap::with_hasher(DefaultHasherBuilder),
            classifications_by_severity: HashMap::with_hasher(DefaultHasherBuilder),
            avg_classification_time_us: 0,
            classification_accuracy: 0.0,
            misclassification_rate: 0.0,
            pattern_matches: 0,
            rule_matches: 0,
            ml_predictions: 0,
        }
    }
}

impl Clone for ClassificationStats {
    fn clone(&self) -> Self {
        Self {
            total_classifications: self.total_classifications,
            classifications_by_type: self.classifications_by_type.clone(),
            classifications_by_severity: self.classifications_by_severity.clone(),
            avg_classification_time_us: self.avg_classification_time_us,
            classification_accuracy: self.classification_accuracy,
            misclassification_rate: self.misclassification_rate,
            pattern_matches: self.pattern_matches,
            rule_matches: self.rule_matches,
            ml_predictions: self.ml_predictions,
        }
    }
}

/// 分类预测结果
#[derive(Debug, Clone)]
pub struct ClassificationPrediction {
    pub classification: ErrorClassification,
    pub confidence: f64,
    pub prediction_time: u64,
}

impl ErrorClassifier {
    /// 创建新的错误分类器
    pub fn new() -> Self {
        Self {
            id: 1,
            classification_rules: Vec::new(),
            ml_model: None,
            error_patterns: HashMap::with_hasher(DefaultHasherBuilder),
            stats: ClassificationStats::default(),
        }
    }

    /// 初始化错误分类器
    pub fn init(&mut self) -> Result<(), &'static str> {
        // 初始化分类规则
        self.init_classification_rules()?;

        // 初始化错误模式
        self.init_error_patterns()?;

        // 初始化机器学习模型（可选）
        // self.init_ml_model()?;

        crate::println!("[ErrorClassifier] Error classifier initialized successfully");
        Ok(())
    }

    /// 分类错误
    pub fn classify_error(&mut self, error_record: &mut ErrorRecord) -> Result<(), &'static str> {
        let start_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();

        // 应用分类规则
        let mut best_classification = None;
        let mut best_confidence = 0.0;
        let mut best_priority = 0;

        for rule in &self.classification_rules {
            if !rule.enabled {
                continue;
            }

            if self.matches_rule(error_record, rule) {
                let confidence = self.calculate_rule_confidence(error_record, rule);

                if confidence > best_confidence || (confidence == best_confidence && rule.priority > best_priority) {
                    best_classification = Some(rule.classification.clone());
                    best_confidence = confidence;
                    best_priority = rule.priority;
                }
            }
        }

        // 应用模式匹配
        if let Some(pattern_classification) = self.apply_pattern_matching(error_record)? {
            let pattern_confidence = 0.8; // 固定的模式匹配置信度

            if pattern_confidence > best_confidence {
                best_classification = Some(pattern_classification);
                best_confidence = pattern_confidence;
            }
        }

        // 应用机器学习模型（如果可用）
        if let Some(ref ml_model) = self.ml_model {
            if let Some(ml_classification) = self.apply_ml_model(error_record, ml_model)? {
                let ml_confidence = ml_model.accuracy;

                if ml_confidence > best_confidence {
                    best_classification = Some(ml_classification);
                    best_confidence = ml_confidence;
                }
            }
        }

        // 更新错误记录
        if let Some(classification) = best_classification {
            error_record.metadata.insert("classification_id".to_string(), classification.id.clone());
            error_record.metadata.insert("classification_name".to_string(), classification.name.clone());
            error_record.metadata.insert("classification_type".to_string(), format!("{:?}", classification.classification_type));
            error_record.metadata.insert("classification_confidence".to_string(), format!("{:.2}", classification.confidence));
            error_record.metadata.insert("requires_immediate_action".to_string(), classification.requires_immediate_action.to_string());
            error_record.metadata.insert("auto_recoverable".to_string(), classification.auto_recoverable.to_string());

            // 更新统计信息
            self.update_classification_stats(&classification);
        }

        let elapsed = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default();
        let classification_time = elapsed.as_micros() as u64;

        // 更新平均分类时间
        let stats = &mut self.stats;
        stats.total_classifications += 1;
        stats.avg_classification_time_us = (stats.avg_classification_time_us + classification_time) / 2;

        Ok(())
    }

    /// 检查规则是否匹配
    fn matches_rule(&self, error_record: &ErrorRecord, rule: &ClassificationRule) -> bool {
        for condition in &rule.conditions {
            if !self.matches_condition(error_record, condition) {
                return false;
            }
        }
        true
    }

    /// 检查条件是否匹配
    fn matches_condition(&self, error_record: &ErrorRecord, condition: &ClassificationCondition) -> bool {
        match condition {
            ClassificationCondition::ErrorCodeMatch(code) => error_record.code == *code,
            ClassificationCondition::ErrorCodeRange(start, end) => error_record.code >= *start && error_record.code <= *end,
            ClassificationCondition::ErrorTypeMatch(error_type) => error_record.error_type == *error_type,
            ClassificationCondition::CategoryMatch(category) => error_record.category == *category,
            ClassificationCondition::SeverityMatch(severity) => error_record.severity == *severity,
            ClassificationCondition::MessageContains(keyword) => error_record.message.contains(keyword),
            ClassificationCondition::SourceMatch(source) => error_record.source.module.contains(source),
            ClassificationCondition::ContextMatch(context) => {
                // 简化的上下文匹配
                false // 在实际实现中会更复杂
            }
            ClassificationCondition::FrequencyThreshold(threshold) => error_record.occurrence_count >= *threshold,
            ClassificationCondition::TimeWindowMatch(start, end) => {
                error_record.timestamp >= *start && error_record.timestamp <= *end
            }
            ClassificationCondition::And(conditions) => {
                conditions.iter().all(|cond| self.matches_condition(error_record, cond))
            }
            ClassificationCondition::Or(conditions) => {
                conditions.iter().any(|cond| self.matches_condition(error_record, cond))
            }
            ClassificationCondition::Not(condition) => !self.matches_condition(error_record, condition),
        }
    }

    /// 计算规则置信度
    fn calculate_rule_confidence(&self, error_record: &ErrorRecord, rule: &ClassificationRule) -> f64 {
        let mut confidence = 1.0;
        let condition_count = rule.conditions.len();

        if condition_count == 0 {
            return 0.0;
        }

        // 基于匹配条件数量计算置信度
        let mut matched_conditions = 0;
        for condition in &rule.conditions {
            if self.matches_condition(error_record, condition) {
                matched_conditions += 1;
            }
        }

        confidence = matched_conditions as f64 / condition_count as f64;

        // 基于规则使用频率调整置信度
        let usage_factor = (rule.match_count as f64 + 1.0) / 100.0;
        confidence = confidence * (1.0 - usage_factor.min(0.5));

        confidence.max(0.0).min(1.0)
    }

    /// 应用模式匹配
    fn apply_pattern_matching(&self, error_record: &ErrorRecord) -> Result<Option<ErrorClassification>, &'static str> {
        for pattern in self.error_patterns.values() {
            if self.matches_pattern(error_record, pattern)? {
                let classification = ErrorClassification {
                    id: pattern.id.clone(),
                    name: pattern.name.clone(),
                    classification_type: self.pattern_to_classification_type(pattern.pattern_type),
                    severity: ErrorSeverity::Medium, // 默认严重级别
                    urgency: Urgency::Medium,
                    impact_scope: ImpactScope::Local,
                    recovery_complexity: RecoveryComplexity::Medium,
                    requires_immediate_action: pattern.weight > 0.8,
                    auto_recoverable: pattern.frequency < 0.1,
                    recommended_response_time: 300, // 5分钟
                    tags: vec!["pattern_matched".to_string()],
                    confidence: pattern.weight,
                };
                return Ok(Some(classification));
            }
        }

        Ok(None)
    }

    /// 检查模式是否匹配
    fn matches_pattern(&self, error_record: &ErrorRecord, pattern: &ErrorPattern) -> Result<bool, &'static str> {
        // 简化的模式匹配实现
        let message = &error_record.message;
        let pattern_expr = &pattern.expression;

        // 检查消息是否包含模式表达式
        let matches = message.contains(pattern_expr) ||
                      error_record.description.contains(pattern_expr) ||
                      error_record.source.function.contains(pattern_expr);

        Ok(matches)
    }

    /// 将模式类型转换为分类类型
    fn pattern_to_classification_type(&self, pattern_type: PatternType) -> ClassificationType {
        match pattern_type {
            PatternType::Temporal => ClassificationType::SystemOverload,
            PatternType::Frequency => ClassificationType::ResourceExhaustion,
            PatternType::Sequence => ClassificationType::SoftwareBug,
            PatternType::Correlation => ClassificationType::NetworkIssue,
            PatternType::Anomaly => ClassificationType::UnknownIssue,
            PatternType::Trend => ClassificationType::PerformanceIssue,
        }
    }

    /// 应用机器学习模型
    fn apply_ml_model(&self, error_record: &ErrorRecord, ml_model: &MLModel) -> Result<Option<ErrorClassification>, &'static str> {
        // 提取特征
        let features = self.extract_features(error_record)?;

        // 使用模型进行预测
        let prediction = self.predict_with_model(&features, ml_model)?;

        if prediction.confidence > 0.7 {
            Ok(Some(prediction.classification))
        } else {
            Ok(None)
        }
    }

    /// 提取特征
    fn extract_features(&self, error_record: &ErrorRecord) -> Result<Vec<Feature>, &'static str> {
        let mut features = Vec::new();

        // 错误代码特征
        features.push(Feature {
            name: "error_code".to_string(),
            feature_type: FeatureType::Numeric,
            value: FeatureValue::Integer(error_record.code as i64),
            importance: 1.0,
        });

        // 错误严重级别特征
        let severity_value = match error_record.severity {
            ErrorSeverity::Info => 1,
            ErrorSeverity::Low => 2,
            ErrorSeverity::Warning => 3,
            ErrorSeverity::Medium => 4,
            ErrorSeverity::High => 5,
            ErrorSeverity::Error => 6,
            ErrorSeverity::Critical => 7,
            ErrorSeverity::Fatal => 8,
        };
        features.push(Feature {
            name: "severity".to_string(),
            feature_type: FeatureType::Numeric,
            value: FeatureValue::Integer(severity_value),
            importance: 1.0,
        });

        // 重复次数特征
        features.push(Feature {
            name: "occurrence_count".to_string(),
            feature_type: FeatureType::Numeric,
            value: FeatureValue::Integer(error_record.occurrence_count as i64),
            importance: 0.8,
        });

        // 错误类型特征
        let error_type_value = match error_record.error_type {
            ErrorType::RuntimeError => 1,
            ErrorType::LogicError => 2,
            ErrorType::CompileError => 3,
            ErrorType::ConfigurationError => 4,
            ErrorType::ResourceError => 5,
            ErrorType::PermissionError => 6,
            ErrorType::NetworkError => 7,
            ErrorType::IOError => 8,
            ErrorType::MemoryError => 9,
            ErrorType::SystemCallError => 10,
            ErrorType::ValidationError => 11,
            ErrorType::TimeoutError => 12,
            ErrorType::CancellationError => 13,
            ErrorType::SystemError => 14, // Added for completeness
        };
        features.push(Feature {
            name: "error_type".to_string(),
            feature_type: FeatureType::Numeric,
            value: FeatureValue::Integer(error_type_value),
            importance: 0.9,
        });

        // 时间特征
        features.push(Feature {
            name: "timestamp".to_string(),
            feature_type: FeatureType::Temporal,
            value: FeatureValue::Integer(error_record.timestamp as i64),
            importance: 0.6,
        });

        Ok(features)
    }

    /// 使用模型进行预测
    fn predict_with_model(&self, features: &[Feature], ml_model: &MLModel) -> Result<ClassificationPrediction, &'static str> {
        // 简化的机器学习预测实现
        // 在实际实现中会使用真正的机器学习算法

        let classification_type = if features.iter().any(|f| f.name == "error_code") {
            ClassificationType::SystemOverload
        } else {
            ClassificationType::UnknownIssue
        };

        let classification = ErrorClassification {
            id: "ml_prediction".to_string(),
            name: "ML Predicted Classification".to_string(),
            classification_type,
            severity: ErrorSeverity::Medium,
            urgency: Urgency::Medium,
            impact_scope: ImpactScope::Local,
            recovery_complexity: RecoveryComplexity::Medium,
            requires_immediate_action: false,
            auto_recoverable: true,
            recommended_response_time: 600,
            tags: vec!["ml_predicted".to_string()],
            confidence: ml_model.accuracy,
        };

        Ok(ClassificationPrediction {
            classification,
            confidence: ml_model.accuracy,
            prediction_time: 1000, // 模拟1ms预测时间
        })
    }


    /// 更新分类统计
    fn update_classification_stats(&mut self, classification: &ErrorClassification) {
        self.stats.total_classifications += 1;
        *self.stats.classifications_by_type.entry(classification.classification_type).or_insert(0) += 1;
        *self.stats.classifications_by_severity.entry(classification.severity).or_insert(0) += 1;
    }

    /// 初始化分类规则
    fn init_classification_rules(&mut self) -> Result<(), &'static str> {
        // 系统严重错误分类规则
        self.classification_rules.push(ClassificationRule {
            id: 1,
            name: "Critical System Error".to_string(),
            description: "Classifies critical system errors".to_string(),
            conditions: vec![
                ClassificationCondition::CategoryMatch(ErrorCategory::System),
                ClassificationCondition::SeverityMatch(ErrorSeverity::Critical),
            ],
            classification: ErrorClassification {
                id: "critical_system".to_string(),
                name: "Critical System Error".to_string(),
                classification_type: ClassificationType::HardwareFault,
                severity: ErrorSeverity::Critical,
                urgency: Urgency::Critical,
                impact_scope: ImpactScope::System,
                recovery_complexity: RecoveryComplexity::Complex,
                requires_immediate_action: true,
                auto_recoverable: false,
                recommended_response_time: 30, // 30秒内响应
                tags: vec!["critical".to_string(), "system".to_string()],
                confidence: 1.0,
            },
            priority: 100,
            enabled: true,
            match_count: 0,
            created_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs(),
        });

        // 内存错误分类规则
        self.classification_rules.push(ClassificationRule {
            id: 2,
            name: "Memory Error".to_string(),
            description: "Classifies memory-related errors".to_string(),
            conditions: vec![
                ClassificationCondition::CategoryMatch(ErrorCategory::Memory),
                ClassificationCondition::Or(vec![
                    ClassificationCondition::ErrorCodeRange(2000, 2099),
                    ClassificationCondition::MessageContains("memory".to_string()),
                ]),
            ],
            classification: ErrorClassification {
                id: "memory_error".to_string(),
                name: "Memory Error".to_string(),
                classification_type: ClassificationType::ResourceExhaustion,
                severity: ErrorSeverity::Error,
                urgency: Urgency::High,
                impact_scope: ImpactScope::System,
                recovery_complexity: RecoveryComplexity::Medium,
                requires_immediate_action: false,
                auto_recoverable: true,
                recommended_response_time: 300, // 5分钟内响应
                tags: vec!["memory".to_string(), "resource".to_string()],
                confidence: 0.9,
            },
            priority: 80,
            enabled: true,
            match_count: 0,
            created_at: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs(),
        });

        Ok(())
    }

    /// 初始化错误模式
    fn init_error_patterns(&mut self) -> Result<(), &'static str> {
        // 频繁错误模式
        self.error_patterns.insert("frequent_errors".to_string(), ErrorPattern {
            id: "frequent_errors".to_string(),
            name: "Frequent Error Pattern".to_string(),
            pattern_type: PatternType::Frequency,
            expression: "timeout".to_string(),
            weight: 0.7,
            frequency: 0.8,
            related_errors: vec![4001, 1004], // timeout, connection failed
            common_causes: vec![
                "Network congestion".to_string(),
                "Server overload".to_string(),
                "Resource exhaustion".to_string(),
            ],
            detection_methods: vec![
                "Frequency analysis".to_string(),
                "Time series analysis".to_string(),
            ],
            prevention_measures: vec![
                "Implement connection pooling".to_string(),
                "Add timeout handling".to_string(),
                "Monitor resource usage".to_string(),
            ],
        });

        // 级联错误模式
        self.error_patterns.insert("cascade_errors".to_string(), ErrorPattern {
            id: "cascade_errors".to_string(),
            name: "Cascade Error Pattern".to_string(),
            pattern_type: PatternType::Sequence,
            expression: "cascade".to_string(),
            weight: 0.9,
            frequency: 0.3,
            related_errors: vec![1001, 1002], // system init failed, memory allocation failed
            common_causes: vec![
                "Component dependency failure".to_string(),
                "Error propagation".to_string(),
                "Lack of error isolation".to_string(),
            ],
            detection_methods: vec![
                "Error chain analysis".to_string(),
                "Dependency graph analysis".to_string(),
            ],
            prevention_measures: vec![
                "Implement error boundaries".to_string(),
                "Add circuit breakers".to_string(),
                "Improve error isolation".to_string(),
            ],
        });

        Ok(())
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> ClassificationStats {
        self.stats.clone()
    }

    /// 重置统计信息
    pub fn reset_statistics(&mut self) {
        self.stats = ClassificationStats::default();
    }

    /// 停止错误分类器
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        // 清理所有数据
        self.classification_rules.clear();
        self.ml_model = None;
        self.error_patterns.clear();
        self.stats = ClassificationStats::default();

        crate::println!("[ErrorClassifier] Error classifier shutdown successfully");
        Ok(())
    }
}

/// 创建默认的错误分类器
pub fn create_error_classifier() -> Arc<Mutex<ErrorClassifier>> {
    Arc::new(Mutex::new(ErrorClassifier::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_classifier_creation() {
        let classifier = ErrorClassifier::new();
        assert_eq!(classifier.id, 1);
        assert!(classifier.classification_rules.is_empty());
        assert!(classifier.error_patterns.is_empty());
    }

    #[test]
    fn test_classification_condition() {
        let error_record = ErrorRecord {
            id: 1,
            code: 1001,
            error_type: ErrorType::SystemError,
            category: ErrorCategory::System,
            severity: ErrorSeverity::Critical,
            status: ErrorStatus::New,
            message: "Test error message".to_string(),
            description: String::new(),
            source: ErrorSource {
                module: "test".to_string(),
                function: "test_func".to_string(),
                file: "test.rs".to_string(),
                line: 10,
                column: 5,
                process_id: 1,
                thread_id: 1,
                cpu_id: 0,
            },
            timestamp: 0,
            context: ErrorContext {
                environment_variables: super::BTreeMap::new(),
                system_config: super::BTreeMap::new(),
                user_input: None,
                related_data: Vec::new(),
                operation_sequence: Vec::new(),
                preconditions: Vec::new(),
                postconditions: Vec::new(),
            },
            stack_trace: Vec::new(),
            system_state: super::SystemStateSnapshot {
                memory_usage: super::MemoryUsage {
                    total_memory: 0,
                    used_memory: 0,
                    available_memory: 0,
                    cached_memory: 0,
                    swap_used: 0,
                    kernel_memory: 0,
                },
                cpu_usage: super::CpuUsage {
                    usage_percent: 0.0,
                    user_percent: 0.0,
                    system_percent: 0.0,
                    idle_percent: 0.0,
                    wait_percent: 0.0,
                    interrupt_percent: 0.0,
                },
                process_states: Vec::new(),
                network_state: super::NetworkState {
                    active_connections: 0,
                    listening_ports: 0,
                    interfaces: Vec::new(),
                    packet_stats: super::PacketStats {
                        total_rx: 0,
                        total_tx: 0,
                        dropped: 0,
                        errors: 0,
                    },
                },
                filesystem_state: super::FileSystemState {
                    mount_points: Vec::new(),
                    disk_usage: Vec::new(),
                    io_stats: super::IoStats {
                        read_operations: 0,
                        write_operations: 0,
                        read_bytes: 0,
                        write_bytes: 0,
                        io_wait_time: 0,
                    },
                },
                device_states: Vec::new(),
                system_load: super::SystemLoad {
                    load_1min: 0.0,
                    load_5min: 0.0,
                    load_15min: 0.0,
                    run_queue_length: 0,
                    blocked_processes: 0,
                },
                timestamp: 0,
            },
            recovery_actions: Vec::new(),
            occurrence_count: 1,
            last_occurrence: 0,
            resolved: false,
            resolution_time: None,
            resolution_method: None,
            metadata: super::BTreeMap::new(),
        };

        let classifier = ErrorClassifier::new();

        // 测试错误代码匹配
        let condition = ClassificationCondition::ErrorCodeMatch(1001);
        assert!(classifier.matches_condition(&error_record, &condition));

        // 测试错误类别匹配
        let condition = ClassificationCondition::CategoryMatch(ErrorCategory::System);
        assert!(classifier.matches_condition(&error_record, &condition));

        // 测试严重级别匹配
        let condition = ClassificationCondition::SeverityMatch(ErrorSeverity::Critical);
        assert!(classifier.matches_condition(&error_record, &condition));
    }
}
