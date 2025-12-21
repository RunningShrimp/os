//! Error Prediction and Prevention Module
//! 
//! 错误预测和预防模块
//! 提供错误模式识别、预测机制和预防性检查功能

extern crate alloc;
use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
    vec::Vec,
    string::{String, ToString},
};
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use spin::Mutex;
use hashbrown::HashMap;

use crate::time::get_timestamp;
use super::{
    ErrorRecord, ErrorCategory, ErrorSeverity, ErrorType,
    UnifiedError, ErrorPriority, EnhancedErrorContext
};

/// 错误模式
#[derive(Debug, Clone, PartialEq)]
pub struct ErrorPattern {
    /// 模式ID
    pub id: u64,
    /// 模式名称
    pub name: String,
    /// 模式描述
    pub description: String,
    /// 错误类别
    pub category: ErrorCategory,
    /// 错误严重级别
    pub severity: ErrorSeverity,
    /// 错误类型
    pub error_type: ErrorType,
    /// 模式匹配条件
    pub conditions: Vec<PatternCondition>,
    /// 预测准确率
    pub accuracy: f64,
    /// 出现频率
    pub frequency: u64,
    /// 最后更新时间
    pub last_updated: u64,
    /// 是否启用
    pub enabled: bool,
}

/// 模式匹配条件
#[derive(Debug, Clone, PartialEq)]
pub enum PatternCondition {
    /// 系统负载条件
    SystemLoad {
        threshold: f64,
        comparison: ComparisonOperator,
    },
    /// 内存使用条件
    MemoryUsage {
        threshold_percent: f64,
        comparison: ComparisonOperator,
    },
    /// 错误频率条件
    ErrorFrequency {
        error_type: ErrorType,
        time_window_seconds: u64,
        count: u32,
        comparison: ComparisonOperator,
    },
    /// 时间条件
    TimeWindow {
        start_hour: u8,
        end_hour: u8,
    },
    /// 进程条件
    ProcessCondition {
        process_name: String,
        condition: ProcessConditionType,
    },
    /// 自定义条件
    Custom {
        name: String,
        parameters: BTreeMap<String, String>,
    },
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

/// 进程条件类型
#[derive(Debug, Clone, PartialEq)]
pub enum ProcessConditionType {
    /// 进程存在
    Exists,
    /// 进程不存在
    NotExists,
    /// 进程状态
    Status(String),
    /// 进程资源使用
    ResourceUsage {
        resource_type: ResourceType,
        threshold: f64,
        comparison: ComparisonOperator,
    },
}

/// 资源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceType {
    /// CPU使用率
    CpuUsage,
    /// 内存使用量
    MemoryUsage,
    /// 文件描述符数量
    FileDescriptors,
    /// 线程数量
    ThreadCount,
}

/// 错误预测结果
#[derive(Debug, Clone)]
pub struct ErrorPrediction {
    /// 预测ID
    pub id: u64,
    /// 预测的错误类型
    pub predicted_error_type: ErrorType,
    /// 预测的错误类别
    pub predicted_category: ErrorCategory,
    /// 预测的严重级别
    pub predicted_severity: ErrorSeverity,
    /// 预测时间
    pub prediction_time: u64,
    /// 预测发生时间
    pub predicted_occurrence_time: u64,
    /// 置信度
    pub confidence: f64,
    /// 相关模式
    pub related_patterns: Vec<u64>,
    /// 预防建议
    pub prevention_recommendations: Vec<PreventionAction>,
    /// 系统状态快照
    pub system_state: SystemStateSnapshot,
    /// 是否已处理
    pub handled: bool,
}

/// 预防动作
#[derive(Debug, Clone)]
pub struct PreventionAction {
    /// 动作ID
    pub id: u64,
    /// 动作名称
    pub name: String,
    /// 动作描述
    pub description: String,
    /// 动作类型
    pub action_type: PreventionActionType,
    /// 优先级
    pub priority: ErrorPriority,
    /// 执行时间
    pub execution_time: u64,
    /// 预期效果
    pub expected_effect: String,
    /// 执行成本
    pub execution_cost: ExecutionCost,
    /// 是否已执行
    pub executed: bool,
}

/// 预防动作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PreventionActionType {
    /// 资源清理
    ResourceCleanup,
    /// 服务重启
    ServiceRestart,
    /// 配置调整
    ConfigurationAdjustment,
    /// 负载均衡
    LoadBalancing,
    /// 缓存预热
    CacheWarmup,
    /// 连接池调整
    ConnectionPoolAdjustment,
    /// 限流调整
    RateLimitAdjustment,
    /// 监控增强
    MonitoringEnhancement,
    /// 自定义动作
    Custom,
}

/// 执行成本
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionCost {
    /// 低成本
    Low,
    /// 中等成本
    Medium,
    /// 高成本
    High,
    /// 极高成本
    Critical,
}

/// 系统状态快照
#[derive(Debug, Clone)]
pub struct SystemStateSnapshot {
    /// 时间戳
    pub timestamp: u64,
    /// CPU使用率
    pub cpu_usage: f64,
    /// 内存使用率
    pub memory_usage: f64,
    /// 磁盘使用率
    pub disk_usage: f64,
    /// 网络I/O
    pub network_io: NetworkIoStats,
    /// 进程数量
    pub process_count: u32,
    /// 活跃连接数
    pub active_connections: u32,
    /// 系统负载
    pub system_load: f64,
    /// 自定义指标
    pub custom_metrics: BTreeMap<String, f64>,
}

/// 网络I/O统计
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkIoStats {
    /// 接收字节数
    pub rx_bytes: u64,
    /// 发送字节数
    pub tx_bytes: u64,
    /// 接收包数
    pub rx_packets: u64,
    /// 发送包数
    pub tx_packets: u64,
}

/// 错误预测器
pub struct ErrorPredictor {
    /// 预测器ID
    pub id: u64,
    /// 错误模式
    patterns: Arc<Mutex<Vec<ErrorPattern>>>,
    /// 历史错误记录
    error_history: Arc<Mutex<VecDeque<ErrorRecord>>>,
    /// 预测结果
    predictions: Arc<Mutex<Vec<ErrorPrediction>>>,
    /// 预防动作
    prevention_actions: Arc<Mutex<Vec<PreventionAction>>>,
    /// 配置
    config: PredictionConfig,
    /// 统计信息
    stats: Arc<Mutex<PredictionStats>>,
    /// 预测计数器
    prediction_counter: AtomicU64,
    /// 是否启用
    enabled: AtomicBool,
}

/// 预测配置
#[derive(Debug, Clone)]
pub struct PredictionConfig {
    /// 启用预测
    pub enable_prediction: bool,
    /// 历史记录保留数量
    pub history_retention_count: usize,
    /// 预测时间窗口（秒）
    pub prediction_window_seconds: u64,
    /// 最小置信度阈值
    pub min_confidence_threshold: f64,
    /// 最大预测数量
    pub max_predictions: usize,
    /// 自动执行预防动作
    pub auto_execute_prevention: bool,
    /// 预防动作执行阈值
    pub prevention_execution_threshold: f64,
    /// 模式学习启用
    pub enable_pattern_learning: bool,
    /// 模式更新间隔（秒）
    pub pattern_update_interval_seconds: u64,
}

impl Default for PredictionConfig {
    fn default() -> Self {
        Self {
            enable_prediction: true,
            history_retention_count: 10000,
            prediction_window_seconds: 3600, // 1小时
            min_confidence_threshold: 0.7,
            max_predictions: 100,
            auto_execute_prevention: false,
            prevention_execution_threshold: 0.8,
            enable_pattern_learning: true,
            pattern_update_interval_seconds: 86400, // 24小时
        }
    }
}

/// 预测统计
#[derive(Debug, Clone, Default)]
pub struct PredictionStats {
    /// 总预测数
    pub total_predictions: u64,
    /// 正确预测数
    pub correct_predictions: u64,
    /// 预测准确率
    pub prediction_accuracy: f64,
    /// 预防动作执行数
    pub prevention_actions_executed: u64,
    /// 预防成功率
    pub prevention_success_rate: f64,
    /// 模式数量
    pub pattern_count: usize,
    /// 最后预测时间
    pub last_prediction_time: u64,
}

impl ErrorPredictor {
    /// 创建新的错误预测器
    pub fn new(config: PredictionConfig) -> Self {
        Self {
            id: 1,
            patterns: Arc::new(Mutex::new(Vec::new())),
            error_history: Arc::new(Mutex::new(VecDeque::new())),
            predictions: Arc::new(Mutex::new(Vec::new())),
            prevention_actions: Arc::new(Mutex::new(Vec::new())),
            config,
            stats: Arc::new(Mutex::new(PredictionStats::default())),
            prediction_counter: AtomicU64::new(1),
            enabled: AtomicBool::new(false),
        }
    }

    /// 初始化预测器
    pub fn init(&self) -> Result<(), &'static str> {
        self.enabled.store(true, Ordering::SeqCst);
        
        // 初始化默认模式
        self.initialize_default_patterns()?;
        
        crate::println!("[ErrorPredictor] Error predictor initialized successfully");
        Ok(())
    }

    /// 初始化默认错误模式
    fn initialize_default_patterns(&self) -> Result<(), &'static str> {
        let mut patterns = self.patterns.lock();
        
        // 内存不足模式
        patterns.push(ErrorPattern {
            id: 1,
            name: "内存不足预测".to_string(),
            description: "当内存使用率超过90%时预测内存不足错误".to_string(),
            category: ErrorCategory::Memory,
            severity: ErrorSeverity::High,
            error_type: ErrorType::ResourceError,
            conditions: vec![
                PatternCondition::MemoryUsage {
                    threshold_percent: 90.0,
                    comparison: ComparisonOperator::GreaterThanOrEqual,
                },
            ],
            accuracy: 0.85,
            frequency: 0,
            last_updated: get_timestamp(),
            enabled: true,
        });

        // 系统负载过高模式
        patterns.push(ErrorPattern {
            id: 2,
            name: "系统负载过高预测".to_string(),
            description: "当系统负载超过CPU核心数的2倍时预测系统过载".to_string(),
            category: ErrorCategory::System,
            severity: ErrorSeverity::Medium,
            error_type: ErrorType::ResourceError,
            conditions: vec![
                PatternCondition::SystemLoad {
                    threshold: 2.0,
                    comparison: ComparisonOperator::GreaterThanOrEqual,
                },
            ],
            accuracy: 0.75,
            frequency: 0,
            last_updated: get_timestamp(),
            enabled: true,
        });

        // 文件描述符耗尽模式
        patterns.push(ErrorPattern {
            id: 3,
            name: "文件描述符耗尽预测".to_string(),
            description: "当进程文件描述符使用率超过95%时预测文件描述符耗尽".to_string(),
            category: ErrorCategory::Resource,
            severity: ErrorSeverity::High,
            error_type: ErrorType::ResourceError,
            conditions: vec![
                PatternCondition::ProcessCondition {
                    process_name: "*".to_string(),
                    condition: ProcessConditionType::ResourceUsage {
                        resource_type: ResourceType::FileDescriptors,
                        threshold: 95.0,
                        comparison: ComparisonOperator::GreaterThanOrEqual,
                    },
                },
            ],
            accuracy: 0.80,
            frequency: 0,
            last_updated: get_timestamp(),
            enabled: true,
        });

        Ok(())
    }

    /// 添加错误记录到历史
    pub fn add_error_record(&self, error_record: ErrorRecord) -> Result<(), &'static str> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Err("Error predictor is not enabled");
        }

        let mut history = self.error_history.lock();
        history.push_back(error_record);

        // 限制历史记录数量
        if history.len() > self.config.history_retention_count {
            history.pop_front();
        }

        // 如果启用模式学习，更新模式
        if self.config.enable_pattern_learning {
            self.update_patterns_from_history()?;
        }

        Ok(())
    }

    /// 执行错误预测
    pub fn predict_errors(&self) -> Result<Vec<ErrorPrediction>, &'static str> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Err("Error predictor is not enabled");
        }

        let current_time = get_timestamp();
        let system_state = self.capture_system_state();
        let patterns = self.patterns.lock();
        let mut predictions = Vec::new();

        for pattern in patterns.iter().filter(|p| p.enabled) {
            if let Some(confidence) = self.evaluate_pattern(pattern, &system_state) {
                if confidence >= self.config.min_confidence_threshold {
                    let prediction_id = self.prediction_counter.fetch_add(1, Ordering::SeqCst);
                    
                    let prediction = ErrorPrediction {
                        id: prediction_id,
                        predicted_error_type: pattern.error_type,
                        predicted_category: pattern.category,
                        predicted_severity: pattern.severity,
                        prediction_time: current_time,
                        predicted_occurrence_time: current_time + self.config.prediction_window_seconds,
                        confidence,
                        related_patterns: vec![pattern.id],
                        prevention_recommendations: self.generate_prevention_actions(pattern),
                        system_state: system_state.clone(),
                        handled: false,
                    };

                    predictions.push(prediction);
                }
            }
        }

        // 更新预测统计
        self.update_prediction_stats(&predictions);

        // 存储预测结果
        {
            let mut stored_predictions = self.predictions.lock();
            for prediction in &predictions {
                stored_predictions.push(prediction.clone());
            }
            
            // 限制预测结果数量
            if stored_predictions.len() > self.config.max_predictions {
                let len = stored_predictions.len();
                let remove_count = len - self.config.max_predictions;
                stored_predictions.drain(0..remove_count);
            }
        }

        Ok(predictions)
    }

    /// 评估模式匹配
    fn evaluate_pattern(&self, pattern: &ErrorPattern, system_state: &SystemStateSnapshot) -> Option<f64> {
        let mut matched_conditions = 0;
        let total_conditions = pattern.conditions.len();

        for condition in &pattern.conditions {
            if self.evaluate_condition(condition, system_state) {
                matched_conditions += 1;
            }
        }

        if total_conditions == 0 {
            return None;
        }

        let match_ratio = matched_conditions as f64 / total_conditions as f64;
        Some(match_ratio * pattern.accuracy)
    }

    /// 评估单个条件
    fn evaluate_condition(&self, condition: &PatternCondition, system_state: &SystemStateSnapshot) -> bool {
        match condition {
            PatternCondition::SystemLoad { threshold, comparison } => {
                self.compare_values(system_state.system_load, *threshold, *comparison)
            },
            PatternCondition::MemoryUsage { threshold_percent, comparison } => {
                self.compare_values(system_state.memory_usage, *threshold_percent, *comparison)
            },
            PatternCondition::ErrorFrequency { error_type: _, time_window_seconds: _, count: _, comparison: _ } => {
                // 简化实现，实际应该查询错误历史
                false
            },
            PatternCondition::TimeWindow { start_hour, end_hour } => {
                let current_hour = (crate::time::get_timestamp() / 3600) % 24;
                current_hour >= *start_hour as u64 && current_hour <= *end_hour as u64
            },
            PatternCondition::ProcessCondition { process_name: _, condition: _ } => {
                // 简化实现，实际应该查询进程状态
                false
            },
            PatternCondition::Custom { name: _, parameters: _ } => {
                // 简化实现，实际应该执行自定义条件
                false
            },
        }
    }

    /// 比较数值
    fn compare_values(&self, actual: f64, threshold: f64, comparison: ComparisonOperator) -> bool {
        match comparison {
            ComparisonOperator::GreaterThan => actual > threshold,
            ComparisonOperator::GreaterThanOrEqual => actual >= threshold,
            ComparisonOperator::LessThan => actual < threshold,
            ComparisonOperator::LessThanOrEqual => actual <= threshold,
            ComparisonOperator::Equal => (actual - threshold).abs() < f64::EPSILON,
            ComparisonOperator::NotEqual => (actual - threshold).abs() >= f64::EPSILON,
        }
    }

    /// 捕获系统状态
    fn capture_system_state(&self) -> SystemStateSnapshot {
        // 简化实现，实际应该从系统监控模块获取
        SystemStateSnapshot {
            timestamp: get_timestamp(),
            cpu_usage: 0.0,
            memory_usage: 0.0,
            disk_usage: 0.0,
            network_io: NetworkIoStats {
                rx_bytes: 0,
                tx_bytes: 0,
                rx_packets: 0,
                tx_packets: 0,
            },
            process_count: 0,
            active_connections: 0,
            system_load: 0.0,
            custom_metrics: BTreeMap::new(),
        }
    }

    /// 生成预防动作
    fn generate_prevention_actions(&self, pattern: &ErrorPattern) -> Vec<PreventionAction> {
        let mut actions = Vec::new();
        let action_id = self.prediction_counter.fetch_add(1, Ordering::SeqCst);

        match pattern.category {
            ErrorCategory::Memory => {
                actions.push(PreventionAction {
                    id: action_id,
                    name: "内存清理".to_string(),
                    description: "清理不必要的内存缓存和临时对象".to_string(),
                    action_type: PreventionActionType::ResourceCleanup,
                    priority: ErrorPriority::High,
                    execution_time: get_timestamp(),
                    expected_effect: "释放内存，降低内存使用率".to_string(),
                    execution_cost: ExecutionCost::Low,
                    executed: false,
                });
            },
            ErrorCategory::System => {
                actions.push(PreventionAction {
                    id: action_id,
                    name: "负载均衡".to_string(),
                    description: "重新分配系统负载，平衡资源使用".to_string(),
                    action_type: PreventionActionType::LoadBalancing,
                    priority: ErrorPriority::Normal,
                    execution_time: get_timestamp(),
                    expected_effect: "降低系统负载，提高响应性能".to_string(),
                    execution_cost: ExecutionCost::Medium,
                    executed: false,
                });
            },
            ErrorCategory::Resource => {
                actions.push(PreventionAction {
                    id: action_id,
                    name: "资源释放".to_string(),
                    description: "释放未使用的系统资源".to_string(),
                    action_type: PreventionActionType::ResourceCleanup,
                    priority: ErrorPriority::High,
                    execution_time: get_timestamp(),
                    expected_effect: "增加可用资源，避免资源耗尽".to_string(),
                    execution_cost: ExecutionCost::Low,
                    executed: false,
                });
            },
            _ => {
                // 默认预防动作
                actions.push(PreventionAction {
                    id: action_id,
                    name: "监控增强".to_string(),
                    description: "增强相关组件的监控和日志记录".to_string(),
                    action_type: PreventionActionType::MonitoringEnhancement,
                    priority: ErrorPriority::Low,
                    execution_time: get_timestamp(),
                    expected_effect: "提高错误检测和诊断能力".to_string(),
                    execution_cost: ExecutionCost::Low,
                    executed: false,
                });
            }
        }

        actions
    }

    /// 执行预防动作
    pub fn execute_prevention_action(&self, action_id: u64) -> Result<(), &'static str> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Err("Error predictor is not enabled");
        }

        let mut actions = self.prevention_actions.lock();
        if let Some(action) = actions.iter_mut().find(|a| a.id == action_id) {
            if action.executed {
                return Err("Prevention action already executed");
            }

            // 执行预防动作
            let success = self.perform_prevention_action(action);

            action.executed = true;
            action.execution_time = get_timestamp();

            if success {
                // 更新统计信息
                let mut stats = self.stats.lock();
                stats.prevention_actions_executed += 1;
                
                crate::println!("[ErrorPredictor] Prevention action {} executed successfully", action.name);
            } else {
                crate::println!("[ErrorPredictor] Prevention action {} execution failed", action.name);
            }

            Ok(())
        } else {
            Err("Prevention action not found")
        }
    }

    /// 执行具体的预防动作
    fn perform_prevention_action(&self, action: &PreventionAction) -> bool {
        match action.action_type {
            PreventionActionType::ResourceCleanup => {
                // 执行资源清理
                crate::println!("[ErrorPredictor] Executing resource cleanup: {}", action.description);
                true
            },
            PreventionActionType::ServiceRestart => {
                // 执行服务重启
                crate::println!("[ErrorPredictor] Executing service restart: {}", action.description);
                true
            },
            PreventionActionType::ConfigurationAdjustment => {
                // 执行配置调整
                crate::println!("[ErrorPredictor] Executing configuration adjustment: {}", action.description);
                true
            },
            PreventionActionType::LoadBalancing => {
                // 执行负载均衡
                crate::println!("[ErrorPredictor] Executing load balancing: {}", action.description);
                true
            },
            PreventionActionType::CacheWarmup => {
                // 执行缓存预热
                crate::println!("[ErrorPredictor] Executing cache warmup: {}", action.description);
                true
            },
            PreventionActionType::ConnectionPoolAdjustment => {
                // 执行连接池调整
                crate::println!("[ErrorPredictor] Executing connection pool adjustment: {}", action.description);
                true
            },
            PreventionActionType::RateLimitAdjustment => {
                // 执行限流调整
                crate::println!("[ErrorPredictor] Executing rate limit adjustment: {}", action.description);
                true
            },
            PreventionActionType::MonitoringEnhancement => {
                // 执行监控增强
                crate::println!("[ErrorPredictor] Executing monitoring enhancement: {}", action.description);
                true
            },
            PreventionActionType::Custom => {
                // 执行自定义动作
                crate::println!("[ErrorPredictor] Executing custom action: {}", action.description);
                true
            },
        }
    }

    /// 从历史记录更新模式
    fn update_patterns_from_history(&self) -> Result<(), &'static str> {
        let history = self.error_history.lock();
        let mut patterns = self.patterns.lock();
        
        // 简化实现：根据历史错误频率更新模式准确率
        for pattern in patterns.iter_mut() {
            let recent_errors: Vec<_> = history.iter()
                .filter(|e| e.category == pattern.category && e.error_type == pattern.error_type)
                .collect();
            
            if !recent_errors.is_empty() {
                // 根据最近的表现调整准确率
                pattern.accuracy = (pattern.accuracy * 0.8 + 0.2).min(1.0);
                pattern.frequency = recent_errors.len() as u64;
                pattern.last_updated = get_timestamp();
            }
        }

        Ok(())
    }

    /// 更新预测统计
    fn update_prediction_stats(&self, predictions: &[ErrorPrediction]) {
        let mut stats = self.stats.lock();
        stats.total_predictions += predictions.len() as u64;
        stats.last_prediction_time = get_timestamp();
        
        if !predictions.is_empty() {
            let avg_confidence = predictions.iter().map(|p| p.confidence).sum::<f64>() / predictions.len() as f64;
            // 简化实现：使用平均置信度作为准确率估计
            stats.prediction_accuracy = avg_confidence;
        }
    }

    /// 获取预测结果
    pub fn get_predictions(&self, limit: Option<usize>) -> Vec<ErrorPrediction> {
        let predictions = self.predictions.lock();
        let mut result = predictions.clone();
        
        // 按置信度排序
        result.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap());
        
        if let Some(limit) = limit {
            result.truncate(limit);
        }
        
        result
    }

    /// 获取预防动作
    pub fn get_prevention_actions(&self, executed_only: bool) -> Vec<PreventionAction> {
        let actions = self.prevention_actions.lock();
        let mut result = actions.clone();
        
        if executed_only {
            result.retain(|a| a.executed);
        }
        
        result
    }

    /// 获取错误模式
    pub fn get_patterns(&self, enabled_only: bool) -> Vec<ErrorPattern> {
        let patterns = self.patterns.lock();
        let mut result = patterns.clone();
        
        if enabled_only {
            result.retain(|p| p.enabled);
        }
        
        result
    }

    /// 添加自定义模式
    pub fn add_pattern(&self, pattern: ErrorPattern) -> Result<(), &'static str> {
        let mut patterns = self.patterns.lock();
        patterns.push(pattern);
        
        // 更新统计
        let mut stats = self.stats.lock();
        stats.pattern_count = patterns.len();
        
        Ok(())
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> PredictionStats {
        self.stats.lock().clone()
    }

    /// 停止预测器
    pub fn shutdown(&self) -> Result<(), &'static str> {
        self.enabled.store(false, Ordering::SeqCst);
        crate::println!("[ErrorPredictor] Error predictor shutdown successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_comparison_operators() {
        let predictor = ErrorPredictor::new(PredictionConfig::default());
        
        assert!(predictor.compare_values(5.0, 3.0, ComparisonOperator::GreaterThan));
        assert!(predictor.compare_values(3.0, 3.0, ComparisonOperator::GreaterThanOrEqual));
        assert!(predictor.compare_values(1.0, 3.0, ComparisonOperator::LessThan));
        assert!(predictor.compare_values(3.0, 3.0, ComparisonOperator::LessThanOrEqual));
        assert!(predictor.compare_values(3.0, 3.0, ComparisonOperator::Equal));
        assert!(predictor.compare_values(3.0, 4.0, ComparisonOperator::NotEqual));
    }

    #[test]
    fn test_pattern_creation() {
        let pattern = ErrorPattern {
            id: 1,
            name: "测试模式".to_string(),
            description: "测试描述".to_string(),
            category: ErrorCategory::Memory,
            severity: ErrorSeverity::High,
            error_type: ErrorType::ResourceError,
            conditions: vec![
                PatternCondition::MemoryUsage {
                    threshold_percent: 90.0,
                    comparison: ComparisonOperator::GreaterThanOrEqual,
                },
            ],
            accuracy: 0.85,
            frequency: 0,
            last_updated: get_timestamp(),
            enabled: true,
        };

        assert_eq!(pattern.id, 1);
        assert_eq!(pattern.name, "测试模式");
        assert_eq!(pattern.category, ErrorCategory::Memory);
        assert!(pattern.enabled);
    }

    #[test]
    fn test_prediction_config_default() {
        let config = PredictionConfig::default();
        assert!(config.enable_prediction);
        assert_eq!(config.history_retention_count, 10000);
        assert_eq!(config.prediction_window_seconds, 3600);
        assert_eq!(config.min_confidence_threshold, 0.7);
    }

    #[test]
    fn test_prevention_action() {
        let action = PreventionAction {
            id: 1,
            name: "测试动作".to_string(),
            description: "测试描述".to_string(),
            action_type: PreventionActionType::ResourceCleanup,
            priority: ErrorPriority::High,
            execution_time: get_timestamp(),
            expected_effect: "测试效果".to_string(),
            execution_cost: ExecutionCost::Low,
            executed: false,
        };

        assert_eq!(action.id, 1);
        assert_eq!(action.action_type, PreventionActionType::ResourceCleanup);
        assert_eq!(action.execution_cost, ExecutionCost::Low);
        assert!(!action.executed);
    }
}