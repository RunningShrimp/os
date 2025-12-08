// Error Recovery Module

extern crate alloc;
//
// 错误恢复模块
// 提供错误恢复策略、恢复动作和恢复状态管理

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::string::ToString;
use alloc::format;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

use super::*;
use super::error_tracing::PerformanceMetrics;
use crate::error_handling::recovery_manager::RecoveryActionExecution;

/// 错误恢复管理器
pub struct ErrorRecoveryManager {
    /// 管理器ID
    pub id: u64,
    /// 恢复策略
    recovery_strategies: BTreeMap<String, RecoveryStrategy>,
    /// 恢复动作工厂
    action_factory: RecoveryActionFactory,
    /// 恢复历史
    recovery_history: Vec<RecoveryRecord>,
    /// 统计信息
    stats: RecoveryStats,
    /// 配置
    config: RecoveryConfig,
    /// 恢复计数器
    recovery_counter: AtomicU64,
}

/// 恢复策略
#[derive(Debug, Clone)]
pub struct RecoveryStrategy {
    /// 策略ID
    pub id: String,
    /// 策略名称
    pub name: String,
    /// 策略描述
    pub description: String,
    /// 适用错误类型
    pub applicable_error_types: Vec<ErrorType>,
    /// 适用错误类别
    pub applicable_error_categories: Vec<ErrorCategory>,
    /// 严重级别范围
    pub severity_range: (ErrorSeverity, ErrorSeverity),
    /// 恢复动作序列
    pub recovery_actions: Vec<RecoveryAction>,
    /// 重试配置
    pub retry_config: RetryConfig,
    /// 成功条件
    pub success_criteria: Vec<SuccessCriterion>,
    /// 策略优先级
    pub priority: u32,
    /// 启用状态
    pub enabled: bool,
    /// 策略统计
    pub stats: StrategyStats,
}

/// 恢复动作
#[derive(Debug, Clone)]
pub struct RecoveryAction {
    /// 动作ID
    pub id: String,
    /// 动作类型
    pub action_type: RecoveryActionType,
    /// 动作名称
    pub name: String,
    /// 动作描述
    pub description: String,
    /// 动作参数
    pub parameters: BTreeMap<String, String>,
    /// 执行超时（毫秒）
    pub timeout_ms: u64,
    /// 前置条件
    pub preconditions: Vec<Precondition>,
    /// 后置条件
    pub postconditions: Vec<Postcondition>,
    /// 回滚动作
    pub rollback_actions: Vec<RollbackAction>,
}

/// 恢复动作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RecoveryActionType {
    /// 重试操作
    Retry,
    /// 重启服务
    Restart,
    /// 重置组件
    Reset,
    /// 回滚操作
    Rollback,
    /// 故障隔离
    Isolate,
    /// 服务降级
    Degrade,
    /// 切换备份
    Failover,
    /// 释放资源
    Release,
    /// 分配资源
    Allocate,
    /// 重新配置
    Reconfigure,
    /// 清理状态
    Cleanup,
    /// 恢复状态
    Restore,
    /// 发送通知
    Notify,
    /// 记录日志
    Log,
    /// 自定义动作
    Custom,
}

impl core::str::FromStr for RecoveryActionType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "retry" => Ok(RecoveryActionType::Retry),
            "restart" => Ok(RecoveryActionType::Restart),
            "reset" => Ok(RecoveryActionType::Reset),
            "rollback" => Ok(RecoveryActionType::Rollback),
            "isolate" => Ok(RecoveryActionType::Isolate),
            "degrade" => Ok(RecoveryActionType::Degrade),
            "failover" => Ok(RecoveryActionType::Failover),
            "release" => Ok(RecoveryActionType::Release),
            "allocate" => Ok(RecoveryActionType::Allocate),
            "reconfigure" => Ok(RecoveryActionType::Reconfigure),
            "cleanup" => Ok(RecoveryActionType::Cleanup),
            "restore" => Ok(RecoveryActionType::Restore),
            "notify" => Ok(RecoveryActionType::Notify),
            "log" => Ok(RecoveryActionType::Log),
            "custom" => Ok(RecoveryActionType::Custom),
            _ => Err(()),
        }
    }
}

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔（毫秒）
    pub retry_interval_ms: u64,
    /// 指数退避
    pub exponential_backoff: bool,
    /// 退避倍数
    pub backoff_multiplier: f64,
    /// 最大退避时间（毫秒）
    pub max_backoff_ms: u64,
    /// 抖动
    pub jitter: bool,
}

/// 成功条件
#[derive(Debug, Clone)]
pub struct SuccessCriterion {
    /// 条件类型
    pub criterion_type: CriterionType,
    /// 条件参数
    pub parameters: BTreeMap<String, String>,
    /// 期望结果
    pub expected_result: String,
    /// 超时时间（毫秒）
    pub timeout_ms: u64,
}

/// 条件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CriterionType {
    /// 状态检查
    StatusCheck,
    /// 健康检查
    HealthCheck,
    /// 功能测试
    FunctionalTest,
    /// 性能测试
    PerformanceTest,
    /// 响应时间检查
    ResponseTimeCheck,
    /// 错误率检查
    ErrorRateCheck,
    /// 自定义条件
    CustomCriterion,
}

/// 前置条件
#[derive(Debug, Clone)]
pub struct Precondition {
    /// 条件描述
    pub description: String,
    /// 检查类型
    pub check_type: CheckType,
    /// 检查参数
    pub parameters: BTreeMap<String, String>,
    /// 是否必须满足
    pub required: bool,
}

/// 后置条件
#[derive(Debug, Clone)]
pub struct Postcondition {
    /// 条件描述
    pub description: String,
    /// 验证类型
    pub validation_type: ValidationType,
    /// 验证参数
    pub parameters: BTreeMap<String, String>,
    /// 验证超时（毫秒）
    pub timeout_ms: u64,
}

/// 检查类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckType {
    /// 系统状态检查
    SystemStateCheck,
    /// 资源可用性检查
    ResourceAvailabilityCheck,
    /// 权限检查
    PermissionCheck,
    /// 依赖检查
    DependencyCheck,
    /// 配置检查
    ConfigurationCheck,
}

/// 验证类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationType {
    /// 状态验证
    StateValidation,
    /// 功能验证
    FunctionalValidation,
    /// 性能验证
    PerformanceValidation,
    /// 安全验证
    SecurityValidation,
    /// 数据完整性验证
    DataIntegrityValidation,
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
    /// 执行顺序
    pub execution_order: u32,
}

/// 回滚动作类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RollbackActionType {
    /// 恢复配置
    RestoreConfig,
    /// 停止服务
    StopService,
    /// 清理资源
    CleanupResources,
    /// 恢复数据
    RestoreData,
    /// 回滚事务
    RollbackTransaction,
    /// 发送通知
    SendNotification,
}

/// 策略统计
#[derive(Debug, Clone, Default)]
pub struct StrategyStats {
    /// 执行次数
    pub execution_count: u64,
    /// 成功次数
    pub success_count: u64,
    /// 失败次数
    pub failure_count: u64,
    /// 平均执行时间（毫秒）
    pub avg_execution_time_ms: u64,
    /// 最大执行时间（毫秒）
    pub max_execution_time_ms: u64,
    /// 最小执行时间（毫秒）
    pub min_execution_time_ms: u64,
    /// 最后执行时间
    pub last_execution: u64,
    /// 成功率
    pub success_rate: f64,
}

/// 恢复记录
#[derive(Debug, Clone)]
pub struct RecoveryRecord {
    /// 记录ID
    pub id: u64,
    /// 策略ID
    pub strategy_id: String,
    /// 错误记录ID
    pub error_id: u64,
    /// 开始时间
    pub start_time: u64,
    /// 结束时间
    pub end_time: Option<u64>,
    /// 恢复状态
    pub status: RecoveryStatus,
    /// 执行的动作
    pub executed_actions: Vec<ExecutedAction>,
    /// 重试次数
    pub retry_count: u32,
    /// 最终结果
    pub final_result: Option<RecoveryResult>,
    /// 错误信息
    pub error_message: Option<String>,
}

/// 恢复状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStatus {
    /// 等待中
    Pending,
    /// 执行中
    InProgress,
    /// 已完成
    Completed,
    /// 失败
    Failed,
    /// 超时
    Timeout,
    /// 已取消
    Cancelled,
    /// 已回滚
    RolledBack,
}

/// 执行的动作
#[derive(Debug, Clone)]
pub struct ExecutedAction {
    /// 动作ID
    pub action_id: String,
    /// 动作类型
    pub action_type: RecoveryActionType,
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

/// 执行状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionStatus {
    /// 成功
    Success,
    /// 失败
    Failed,
    /// 超时
    Timeout,
    /// 跳过
    Skipped,
}

/// 恢复结果
#[derive(Debug, Clone)]
pub struct RecoveryResult {
    /// 结果类型
    pub result_type: RecoveryResultType,
    /// 结果描述
    pub description: String,
    /// 恢复的数据
    pub recovered_data: Option<String>,
    /// 性能指标
    pub performance_metrics: Option<PerformanceMetrics>,
    /// 建议后续动作
    pub recommended_followup: Vec<String>,
}

/// 恢复结果类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryResultType {
    /// 完全恢复
    FullRecovery,
    /// 部分恢复
    PartialRecovery,
    /// 降级恢复
    DegradedRecovery,
    /// 临时恢复
    TemporaryRecovery,
    /// 恢复失败
    RecoveryFailed,
}

/// 恢复统计
#[derive(Debug, Clone, Default)]
pub struct RecoveryStats {
    /// 总恢复尝试次数
    pub total_recovery_attempts: u64,
    /// 成功恢复次数
    pub successful_recoveries: u64,
    /// 失败恢复次数
    pub failed_recoveries: u64,
    /// 平均恢复时间（毫秒）
    pub avg_recovery_time_ms: u64,
    /// 恢复成功率
    pub recovery_success_rate: f64,
    /// 按策略类型统计
    pub recoveries_by_strategy: BTreeMap<String, u64>,
    /// 按错误类型统计
    pub recoveries_by_error_type: BTreeMap<ErrorType, u64>,
    /// 按严重级别统计
    pub recoveries_by_severity: BTreeMap<ErrorSeverity, u64>,
}

/// 恢复配置
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    /// 启用自动恢复
    pub enable_auto_recovery: bool,
    /// 最大并发恢复数
    pub max_concurrent_recoveries: u32,
    /// 默认重试配置
    pub default_retry_config: RetryConfig,
    /// 恢复历史保留数量
    pub recovery_history_size: usize,
    /// 启用恢复监控
    pub enable_recovery_monitoring: bool,
    /// 恢复超时时间（毫秒）
    pub recovery_timeout_ms: u64,
    /// 启用恢复通知
    pub enable_recovery_notifications: bool,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            enable_auto_recovery: true,
            max_concurrent_recoveries: 5,
            default_retry_config: RetryConfig {
                max_retries: 3,
                retry_interval_ms: 1000,
                exponential_backoff: true,
                backoff_multiplier: 2.0,
                max_backoff_ms: 30000,
                jitter: true,
            },
            recovery_history_size: 1000,
            enable_recovery_monitoring: true,
            recovery_timeout_ms: 300000, // 5分钟
            enable_recovery_notifications: true,
        }
    }
}

impl Default for RetryConfig {
    fn default() -> Self {
        RecoveryConfig::default().default_retry_config
    }
}

// 已通过 #[derive(Default)] 提供默认实现

/// 恢复动作工厂
pub struct RecoveryActionFactory {
    /// 动作创建器
    action_creators: BTreeMap<RecoveryActionType, Box<dyn RecoveryActionCreator>>,
}

/// 恢复动作创建器接口
pub trait RecoveryActionCreator: Send {
    /// 创建恢复动作
    fn create_action(&self, parameters: &BTreeMap<String, String>) -> Result<Box<dyn RecoveryActionExecutor>, &'static str>;
}

/// 恢复动作执行器接口
pub trait RecoveryActionExecutor {
    /// 执行动作
    fn execute(&mut self, context: &RecoveryContext) -> Result<RecoveryResult, &'static str>;
    /// 回滚动作
    fn rollback(&mut self, context: &RecoveryContext) -> Result<(), &'static str>;
    /// 获取动作类型
    fn get_action_type(&self) -> RecoveryActionType;
}

/// 恢复上下文
#[derive(Debug, Clone)]
pub struct RecoveryContext {
    /// 错误记录
    pub error_record: ErrorRecord,
    /// 恢复记录ID
    pub recovery_id: u64,
    /// 执行时间戳
    pub timestamp: u64,
    /// 上下文数据
    pub context_data: BTreeMap<String, String>,
}

/// 基础恢复动作执行器
pub struct BaseRecoveryActionExecutor {
    action_type: RecoveryActionType,
    parameters: BTreeMap<String, String>,
}

impl BaseRecoveryActionExecutor {
    /// 创建基础恢复动作执行器
    pub fn new(action_type: RecoveryActionType, parameters: BTreeMap<String, String>) -> Self {
        Self {
            action_type,
            parameters,
        }
    }
}

impl RecoveryActionExecutor for BaseRecoveryActionExecutor {
    fn execute(&mut self, _context: &RecoveryContext) -> Result<RecoveryResult, &'static str> {
        // 基础实现 - 实际实现中会根据动作类型执行具体逻辑
        Ok(RecoveryResult {
            result_type: RecoveryResultType::FullRecovery,
            description: format!("Base action {:?} executed", self.action_type),
            recovered_data: None,
            performance_metrics: None,
            recommended_followup: Vec::new(),
        })
    }

    fn rollback(&mut self, _context: &RecoveryContext) -> Result<(), &'static str> {
        // 基础回滚实现
        Ok(())
    }

    fn get_action_type(&self) -> RecoveryActionType {
        self.action_type
    }
}

impl RecoveryActionFactory {
    /// 创建新的恢复动作工厂
    pub fn new() -> Self {
        let mut factory = Self {
            action_creators: BTreeMap::new(),
        };

        // 注册默认的动作创建器
        factory.register_default_creators();

        factory
    }

    /// 注册默认的动作创建器
    fn register_default_creators(&mut self) {
        self.action_creators.insert(RecoveryActionType::Retry, Box::new(DefaultActionCreator));
        self.action_creators.insert(RecoveryActionType::Restart, Box::new(DefaultActionCreator));
        self.action_creators.insert(RecoveryActionType::Reset, Box::new(DefaultActionCreator));
        self.action_creators.insert(RecoveryActionType::Isolate, Box::new(DefaultActionCreator));
    }

    /// 创建恢复动作执行器
    pub fn create_executor(&self, action_type: RecoveryActionType, parameters: &BTreeMap<String, String>) -> Result<Box<dyn RecoveryActionExecutor>, &'static str> {
        let creator = self.action_creators.get(&action_type)
            .ok_or("No creator registered for action type")?;

        creator.create_action(parameters)
    }
}

/// 默认动作创建器
struct DefaultActionCreator;

impl RecoveryActionCreator for DefaultActionCreator {
    fn create_action(&self, parameters: &BTreeMap<String, String>) -> Result<Box<dyn RecoveryActionExecutor>, &'static str> {
        // 创建基础执行器
        let action_type = parameters.get("action_type")
            .and_then(|s| s.parse().ok())
            .unwrap_or(RecoveryActionType::Retry);

        Ok(Box::new(BaseRecoveryActionExecutor::new(action_type, parameters.clone())))
    }
}

impl ErrorRecoveryManager {
    /// 创建新的错误恢复管理器
    pub fn new() -> Self {
        Self {
            id: 1,
            recovery_strategies: BTreeMap::new(),
            action_factory: RecoveryActionFactory::new(),
            recovery_history: Vec::new(),
            stats: RecoveryStats::default(),
            config: RecoveryConfig::default(),
            recovery_counter: AtomicU64::new(1),
        }
    }

    /// 初始化错误恢复管理器
    pub fn init(&mut self) -> Result<(), &'static str> {
        // 加载默认恢复策略
        self.load_default_recovery_strategies()?;

        crate::println!("[ErrorRecovery] Error recovery manager initialized successfully");
        Ok(())
    }

    /// 执行恢复策略
    pub fn execute_recovery_strategy(&mut self, strategy_id: &str, error_record: &ErrorRecord) -> Result<RecoveryResult, &'static str> {
        let strategy = self.recovery_strategies.get_mut(strategy_id)
            .ok_or("Recovery strategy not found")?;

        if !strategy.enabled {
            return Err("Recovery strategy is disabled");
        }

        let recovery_id = self.recovery_counter.fetch_add(1, Ordering::SeqCst);
        let start_time = crate::time::get_timestamp();

        let recovery_record = RecoveryRecord {
            id: recovery_id,
            strategy_id: strategy_id.to_string(),
            error_id: error_record.id,
            start_time,
            end_time: None,
            status: RecoveryStatus::InProgress,
            executed_actions: Vec::new(),
            retry_count: 0,
            final_result: None,
            error_message: None,
        };

        // 释放strategy借用再执行恢复动作
        drop(strategy);
        let result = self.execute_recovery_actions_by_id(strategy_id, error_record, &recovery_record)?;

        let end_time = crate::time::get_timestamp();
        let execution_time = end_time - start_time;

        // 更新恢复记录
        let mut updated_record = recovery_record;
        updated_record.end_time = Some(end_time);
        updated_record.status = RecoveryStatus::Completed;
        updated_record.final_result = Some(result.clone());

        // 保存恢复记录
        self.save_recovery_record(updated_record);

        // 更新恢复统计
        self.update_recovery_stats(&result, execution_time);

        Ok(result)
    }

    /// 执行恢复动作
    fn execute_recovery_actions(&mut self, strategy: &RecoveryStrategy, error_record: &ErrorRecord, recovery_record: &RecoveryRecord) -> Result<RecoveryResult, &'static str> {
        let context = RecoveryContext {
            error_record: error_record.clone(),
            recovery_id: recovery_record.id,
            timestamp: crate::time::get_timestamp(),
            context_data: BTreeMap::new(),
        };

        for action in &strategy.recovery_actions {
            // 检查前置条件
            if !self.check_preconditions(&action.preconditions, &context) {
                continue;
            }

            // 创建动作执行器
            let mut executor = self.action_factory.create_executor(action.action_type, &action.parameters)?;

            // 执行动作
            let result = executor.execute(&context);

            match result {
                Ok(action_result) => {
                    // 检查后置条件
                    if !self.check_postconditions(&action.postconditions, &context) {
                        // 回滚动作
                        let _ = executor.rollback(&context);
                        return Err("Postcondition check failed");
                    }

                    // 如果成功，检查是否满足成功条件
                    if self.check_success_criteria(&strategy.success_criteria, &action_result) {
                        return Ok(action_result);
                    }
                }
                Err(e) => {
                    // 动作失败，根据重试配置决定是否重试
                    if strategy.retry_config.max_retries > 0 {
                        return self.retry_action(strategy, error_record, &context);
                    }
                    return Err(e);
                }
            }
        }

        // 所有动作执行完成，但没有完全恢复
        Ok(RecoveryResult {
            result_type: RecoveryResultType::PartialRecovery,
            description: "Recovery actions completed with partial success".to_string(),
            recovered_data: None,
            performance_metrics: None,
            recommended_followup: vec!["Monitor system stability".to_string()],
        })
    }

    /// 重试动作
    fn retry_action(&mut self, strategy: &RecoveryStrategy, error_record: &ErrorRecord, context: &RecoveryContext) -> Result<RecoveryResult, &'static str> {
        let mut retry_count = 0;
        let mut retry_interval = strategy.retry_config.retry_interval_ms;

        while retry_count < strategy.retry_config.max_retries {
            retry_count += 1;

            // 等待重试间隔
            if retry_interval > 0 {
                // 实现等待逻辑
                crate::time::sleep(retry_interval);
            }

            // 重新执行恢复动作
            let result = self.execute_recovery_actions(strategy, error_record, &RecoveryRecord {
                id: context.recovery_id,
                strategy_id: strategy.id.clone(),
                error_id: error_record.id,
                start_time: crate::time::get_timestamp(),
                end_time: None,
                status: RecoveryStatus::InProgress,
                executed_actions: Vec::new(),
                retry_count,
                final_result: None,
                error_message: None,
            });

            match result {
                Ok(recovery_result) => {
                    return Ok(recovery_result);
                }
                Err(_) => {
                    // 更新重试间隔
                    if strategy.retry_config.exponential_backoff {
                        retry_interval = (retry_interval as f64 * strategy.retry_config.backoff_multiplier) as u64;
                        retry_interval = retry_interval.min(strategy.retry_config.max_backoff_ms);
                    }
                }
            }
        }

        Err("All retry attempts failed")
    }

    /// 检查前置条件
    fn check_preconditions(&self, preconditions: &[Precondition], _context: &RecoveryContext) -> bool {
        for precondition in preconditions {
            if precondition.required {
                // 实现前置条件检查
                if !self.evaluate_precondition(precondition) {
                    return false;
                }
            }
        }
        true
    }

    /// 评估前置条件
    fn evaluate_precondition(&self, _precondition: &Precondition) -> bool {
        // 实现具体的前置条件评估逻辑
        true
    }

    /// 检查后置条件
    fn check_postconditions(&self, postconditions: &[Postcondition], _context: &RecoveryContext) -> bool {
        for postcondition in postconditions {
            if !self.evaluate_postcondition(postcondition) {
                return false;
            }
        }
        true
    }

    /// 评估后置条件
    fn evaluate_postcondition(&self, _postcondition: &Postcondition) -> bool {
        // 实现具体的后置条件评估逻辑
        true
    }

    /// 检查成功条件
    fn check_success_criteria(&self, success_criteria: &[SuccessCriterion], result: &RecoveryResult) -> bool {
        for criterion in success_criteria {
            if !self.evaluate_success_criterion(criterion, result) {
                return false;
            }
        }
        true
    }

    /// 评估成功条件
    fn evaluate_success_criterion(&self, _criterion: &SuccessCriterion, _result: &RecoveryResult) -> bool {
        // 实现具体的成功条件评估逻辑
        true
    }

    /// 更新策略统计
    fn update_strategy_stats(&mut self, strategy: &mut RecoveryStrategy, start_time: u64) {
        strategy.stats.execution_count += 1;
        strategy.stats.last_execution = start_time;
    }

    /// 保存恢复记录
    fn save_recovery_record(&mut self, record: RecoveryRecord) {
        self.recovery_history.push(record);

        // 限制历史记录数量
        if self.recovery_history.len() > self.config.recovery_history_size {
            self.recovery_history.remove(0);
        }
    }

    /// 更新恢复统计
    fn update_recovery_stats(&mut self, result: &RecoveryResult, execution_time: u64) {
        self.stats.total_recovery_attempts += 1;

        match result.result_type {
            RecoveryResultType::FullRecovery | RecoveryResultType::PartialRecovery => {
                self.stats.successful_recoveries += 1;
            }
            _ => {
                self.stats.failed_recoveries += 1;
            }
        }

        // 更新平均恢复时间
        let total_time = self.stats.avg_recovery_time_ms * (self.stats.total_recovery_attempts - 1) + execution_time;
        self.stats.avg_recovery_time_ms = total_time / self.stats.total_recovery_attempts;

        // 计算恢复成功率
        self.stats.recovery_success_rate = self.stats.successful_recoveries as f64 / self.stats.total_recovery_attempts as f64;
    }

    /// 应用恢复策略
    pub fn apply_recovery_strategy(&mut self, strategy: &RecoveryStrategy, error_record: &ErrorRecord) -> Result<RecoveryResult, &'static str> {
        self.execute_recovery_strategy(&strategy.id, error_record)
    }

    /// 添加恢复策略
    pub fn add_recovery_strategy(&mut self, strategy: RecoveryStrategy) -> Result<(), &'static str> {
        self.recovery_strategies.insert(strategy.id.clone(), strategy);
        Ok(())
    }

    /// 获取适用的恢复策略
    pub fn get_applicable_strategies(&self, error_record: &ErrorRecord) -> Vec<&RecoveryStrategy> {
        self.recovery_strategies
            .values()
            .filter(|strategy| {
                strategy.enabled
                && (strategy.applicable_error_types.is_empty() || strategy.applicable_error_types.contains(&error_record.error_type))
                && (strategy.applicable_error_categories.is_empty() || strategy.applicable_error_categories.contains(&error_record.category))
                && error_record.severity >= strategy.severity_range.0
                && error_record.severity <= strategy.severity_range.1
            })
            .collect()
    }

    /// 获取恢复策略
    pub fn get_recovery_strategies(&self) -> &BTreeMap<String, RecoveryStrategy> {
        &self.recovery_strategies
    }

    /// 获取恢复历史
    pub fn get_recovery_history(&self, limit: Option<usize>) -> Vec<&RecoveryRecord> {
        let mut history = self.recovery_history.iter().collect::<Vec<_>>();
        history.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        if let Some(limit) = limit {
            history.truncate(limit);
        }

        history
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> RecoveryStats {
        self.stats.clone()
    }

    /// 更新配置
    pub fn update_config(&mut self, config: RecoveryConfig) -> Result<(), &'static str> {
        self.config = config;
        Ok(())
    }

    /// 加载默认恢复策略
    fn load_default_recovery_strategies(&mut self) -> Result<(), &'static str> {
        let strategies = vec![
            RecoveryStrategy {
                id: "basic_retry".to_string(),
                name: "Basic Retry Strategy".to_string(),
                description: "Simple retry strategy for transient errors".to_string(),
                applicable_error_types: vec![
                    ErrorType::NetworkError,
                    ErrorType::TimeoutError,
                    ErrorType::RuntimeError,
                ],
                applicable_error_categories: vec![
                    ErrorCategory::Network,
                    ErrorCategory::Timeout,
                ],
                severity_range: (ErrorSeverity::Info, ErrorSeverity::Error),
                recovery_actions: vec![
                    RecoveryAction {
                        id: "retry_action".to_string(),
                        action_type: RecoveryActionType::Retry,
                        name: "Retry Operation".to_string(),
                        description: "Retry the failed operation".to_string(),
                        parameters: {
                            let mut params = BTreeMap::new();
                            params.insert("action_type".to_string(), "Retry".to_string());
                            params
                        },
                        timeout_ms: 30000,
                        preconditions: Vec::new(),
                        postconditions: Vec::new(),
                        rollback_actions: Vec::new(),
                    },
                ],
                retry_config: RetryConfig {
                    max_retries: 3,
                    retry_interval_ms: 1000,
                    exponential_backoff: true,
                    backoff_multiplier: 2.0,
                    max_backoff_ms: 10000,
                    jitter: true,
                },
                success_criteria: vec![
                    SuccessCriterion {
                        criterion_type: CriterionType::StatusCheck,
                        parameters: BTreeMap::new(),
                        expected_result: "success".to_string(),
                        timeout_ms: 5000,
                    },
                ],
                priority: 1,
                enabled: true,
                stats: StrategyStats::default(),
            },
            RecoveryStrategy {
                id: "service_restart".to_string(),
                name: "Service Restart Strategy".to_string(),
                description: "Restart service to recover from errors".to_string(),
                applicable_error_types: vec![
                    ErrorType::SystemError,
                    ErrorType::RuntimeError,
                ],
                applicable_error_categories: vec![
                    ErrorCategory::System,
                    ErrorCategory::Process,
                ],
                severity_range: (ErrorSeverity::Warning, ErrorSeverity::Critical),
                recovery_actions: vec![
                    RecoveryAction {
                        id: "restart_action".to_string(),
                        action_type: RecoveryActionType::Restart,
                        name: "Restart Service".to_string(),
                        description: "Restart the affected service".to_string(),
                        parameters: {
                            let mut params = BTreeMap::new();
                            params.insert("action_type".to_string(), "Restart".to_string());
                            params.insert("graceful".to_string(), "true".to_string());
                            params
                        },
                        timeout_ms: 60000,
                        preconditions: vec![
                            Precondition {
                                description: "Service must be installed".to_string(),
                                check_type: CheckType::SystemStateCheck,
                                parameters: BTreeMap::new(),
                                required: true,
                            },
                        ],
                        postconditions: vec![
                            Postcondition {
                                description: "Service must be running".to_string(),
                                validation_type: ValidationType::StateValidation,
                                parameters: BTreeMap::new(),
                                timeout_ms: 30000,
                            },
                        ],
                        rollback_actions: vec![
                            RollbackAction {
                                description: "Stop service if restart fails".to_string(),
                                action_type: RollbackActionType::StopService,
                                parameters: BTreeMap::new(),
                                execution_order: 1,
                            },
                        ],
                    },
                ],
                retry_config: RetryConfig {
                    max_retries: 1,
                    retry_interval_ms: 5000,
                    exponential_backoff: false,
                    backoff_multiplier: 1.0,
                    max_backoff_ms: 5000,
                    jitter: false,
                },
                success_criteria: vec![
                    SuccessCriterion {
                        criterion_type: CriterionType::HealthCheck,
                        parameters: BTreeMap::new(),
                        expected_result: "healthy".to_string(),
                        timeout_ms: 10000,
                    },
                ],
                priority: 2,
                enabled: true,
                stats: StrategyStats::default(),
            },
        ];

        for strategy in strategies {
            self.recovery_strategies.insert(strategy.id.clone(), strategy);
        }

        Ok(())
    }

    /// 停止错误恢复管理器
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        self.recovery_strategies.clear();
        self.recovery_history.clear();

        crate::println!("[ErrorRecovery] Error recovery manager shutdown successfully");
        Ok(())
    }
}

/// 创建默认的错误恢复管理器
pub fn create_error_recovery_manager() -> Arc<Mutex<ErrorRecoveryManager>> {
    Arc::new(Mutex::new(ErrorRecoveryManager::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_recovery_manager_creation() {
        let manager = ErrorRecoveryManager::new();
        assert_eq!(manager.id, 1);
        assert!(manager.recovery_strategies.is_empty());
        assert!(manager.recovery_history.is_empty());
    }

    #[test]
    fn test_recovery_strategy_creation() {
        let strategy = RecoveryStrategy {
            id: "test_strategy".to_string(),
            name: "Test Strategy".to_string(),
            description: "Test recovery strategy".to_string(),
            applicable_error_types: vec![ErrorType::RuntimeError],
            applicable_error_categories: vec![ErrorCategory::System],
            severity_range: (ErrorSeverity::Error, ErrorSeverity::Critical),
            recovery_actions: vec![
                RecoveryAction {
                    id: "test_action".to_string(),
                    action_type: RecoveryActionType::Retry,
                    name: "Test Action".to_string(),
                    description: "Test recovery action".to_string(),
                    parameters: BTreeMap::new(),
                    timeout_ms: 5000,
                    preconditions: Vec::new(),
                    postconditions: Vec::new(),
                    rollback_actions: Vec::new(),
                },
            ],
            retry_config: RetryConfig::default(),
            success_criteria: Vec::new(),
            priority: 1,
            enabled: true,
            stats: StrategyStats::default(),
        };

        assert_eq!(strategy.id, "test_strategy");
        assert_eq!(strategy.priority, 1);
        assert!(strategy.enabled);
    }

    #[test]
    fn test_recovery_config_default() {
        let config = RecoveryConfig::default();
        assert!(config.enable_auto_recovery);
        assert_eq!(config.max_concurrent_recoveries, 5);
        assert_eq!(config.default_retry_config.max_retries, 3);
    }
}

impl ErrorRecoveryManager {
    /// 通过ID执行恢复动作
    fn execute_recovery_actions_by_id(
        &mut self,
        strategy_id: &str,
        error_record: &ErrorRecord,
        recovery_record: &RecoveryRecord,
    ) -> Result<RecoveryResult, &'static str> {
        let strategy_clone = self.recovery_strategies.get(strategy_id).cloned()
            .ok_or("Recovery strategy not found")?;
        self.execute_recovery_actions(&strategy_clone, error_record, recovery_record)
    }
}
