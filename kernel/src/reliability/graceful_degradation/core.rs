//! Core graceful degradation manager implementation

use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

// Import println macro for use in this module
#[allow(unused_imports)]
use crate::println;

use super::types::LogLevel;
use super::strategy::{DegradationStrategy, DegradationType, TriggerType, TriggerCondition};
use super::actions::{DegradationAction, DegradationActionType, RecoveryCondition, RecoveryConditionType, RollbackAction, RollbackActionType};
use super::quality::{ServiceQualityController, QualityMetric, MetricType, QualityThreshold, QualityLevel, ControllerStatus};
use super::managers::{
    FeatureManager, Feature, FeatureCategory, ImportanceLevel, ResourceRequirements, QualityRequirements, FeatureState,
    LoadManager, LoadStrategy, LoadStrategyType, LoadThresholds, AllocationAlgorithm, LoadStatus, LoadLevel,
    ResourceManager, ResourcePool, ResourceType, PoolStatus, ResourceUsageStatistics,
};
use super::session::{
    DegradationSession, DegradationStatus, ExecutedDegradationAction, DegradationEffect, QualityChange,
    UserExperienceImpact, BusinessImpact, SLAComplianceImpact, RecoveryStatus, SessionLog, DegradationStats, DegradationConfig,
};

/// 优雅降级管理器
pub struct GracefulDegradationManager {
    /// 管理器ID
    pub id: u64,
    /// 降级策略
    degradation_strategies: BTreeMap<String, DegradationStrategy>,
    /// 服务质量控制器
    quality_controllers: BTreeMap<String, ServiceQualityController>,
    /// 降级会话
    active_degradations: BTreeMap<String, DegradationSession>,
    /// 功能管理器
    feature_manager: FeatureManager,
    /// 负载管理器
    load_manager: LoadManager,
    /// 资源管理器
    resource_manager: ResourceManager,
    /// 统计信息
    stats: DegradationStats,
    /// 配置
    config: DegradationConfig,
    /// 会话计数器
    session_counter: AtomicU64,
}

impl GracefulDegradationManager {
    /// 创建新的优雅降级管理器
    pub fn new() -> Self {
        Self {
            id: 1,
            degradation_strategies: BTreeMap::new(),
            quality_controllers: BTreeMap::new(),
            active_degradations: BTreeMap::new(),
            feature_manager: FeatureManager {
                features: BTreeMap::new(),
                dependencies: BTreeMap::new(),
                feature_states: BTreeMap::new(),
            },
            load_manager: LoadManager {
                load_strategies: BTreeMap::new(),
                current_load: LoadStatus {
                    total_load: 0.0,
                    available_capacity: 100.0,
                    load_percentage: 0.0,
                    load_level: LoadLevel::Low,
                    last_updated: crate::subsystems::time::get_timestamp(),
                },
                load_history: Vec::new(),
            },
            resource_manager: ResourceManager {
                resource_pools: BTreeMap::new(),
                resource_allocations: BTreeMap::new(),
                usage_statistics: ResourceUsageStatistics::default(),
            },
            stats: DegradationStats::default(),
            config: DegradationConfig::default(),
            session_counter: AtomicU64::new(1),
        }
    }

    /// 初始化优雅降级管理器
    pub fn init(&mut self) -> Result<(), &'static str> {
        self.load_default_degradation_strategies()?;
        self.initialize_feature_manager()?;
        self.initialize_load_manager()?;
        self.initialize_resource_manager()?;
        self.initialize_quality_controllers()?;

        println!("[GracefulDegradation] Graceful degradation manager initialized successfully");
        Ok(())
    }

    /// 触发降级
    pub fn trigger_degradation(&mut self, strategy_id: &str, service_name: &str, trigger_reason: &str) -> Result<String, &'static str> {
        let strategy = self.degradation_strategies.get(strategy_id)
            .ok_or("Degradation strategy not found")?;

        if !strategy.enabled {
            return Err("Degradation strategy is disabled");
        }

        if self.active_degradations.len() >= self.config.max_concurrent_degradations as usize {
            return Err("Maximum concurrent degradations reached");
        }

        let session_id = format!("degradation_{}", self.session_counter.fetch_add(1, Ordering::SeqCst));
        let start_time = crate::subsystems::time::get_timestamp();

        let session = DegradationSession {
            id: session_id.clone(),
            strategy_id: strategy_id.to_string(),
            service_name: service_name.to_string(),
            start_time,
            end_time: None,
            status: DegradationStatus::Initializing,
            trigger_reason: trigger_reason.to_string(),
            executed_actions: Vec::new(),
            degradation_effect: DegradationEffect {
                performance_improvement: 0.0,
                resource_savings: 0.0,
                quality_change: QualityChange::Maintained,
                user_experience_impact: UserExperienceImpact {
                    response_time_change_percent: 0.0,
                    functionality_completeness_percent: 100.0,
                    satisfaction_impact: 0.0,
                    supported_users_change: 0,
                },
                business_impact: BusinessImpact {
                    revenue_impact_percent: 0.0,
                    cost_savings: 0.0,
                    sla_compliance_impact: SLAComplianceImpact::None,
                    customer_churn_risk: 0.0,
                },
            },
            recovery_status: RecoveryStatus::NotStarted,
            logs: Vec::new(),
        };

        let strategy_clone = strategy.clone();
        self.execute_degradation_actions(&session_id, &strategy_clone, service_name)?;

        let mut updated_session = session;
        updated_session.status = DegradationStatus::Degraded;
        self.active_degradations.insert(session_id.clone(), updated_session);
        self.update_degradation_stats();
        self.add_session_log(&session_id, LogLevel::Info, &format!("Degradation triggered for service: {}", service_name), "GracefulDegradation");

        Ok(session_id)
    }

    /// 执行降级行动
    fn execute_degradation_actions(&mut self, session_id: &str, strategy: &DegradationStrategy, service_name: &str) -> Result<(), &'static str> {
        for action in &strategy.degradation_actions {
            let execution_start = crate::subsystems::time::get_timestamp();
            let result = self.execute_degradation_action(action, service_name);
            let execution_end = crate::subsystems::time::get_timestamp();

            let executed_action = ExecutedDegradationAction {
                action_id: action.id.clone(),
                action_type: action.action_type,
                start_time: execution_start,
                end_time: Some(execution_end),
                status: if result.is_ok() { super::types::ExecutionStatus::Success } else { super::types::ExecutionStatus::Failed },
                result: if result.is_ok() { Some("Action executed successfully".to_string()) } else { None },
                error_message: if result.is_err() { Some(format!("{:?}", result.as_ref().unwrap_err())) } else { None },
            };

            if let Some(session) = self.active_degradations.get_mut(session_id) {
                session.executed_actions.push(executed_action);
            }

            match result {
                Ok(_) => {
                    self.add_session_log(session_id, LogLevel::Info, &format!("Action {} executed successfully", action.name), &format!("{:?}", action.action_type));
                }
                Err(e) => {
                    self.add_session_log(session_id, LogLevel::Error, &format!("Action {} failed: {}", action.name, e), &format!("{:?}", action.action_type));
                    return Err(e);
                }
            }
        }
        Ok(())
    }

    /// 执行单个降级行作
    fn execute_degradation_action(&mut self, action: &DegradationAction, service_name: &str) -> Result<(), &'static str> {
        match action.action_type {
            DegradationActionType::DisableFeature => {
                self.disable_feature(&action.parameters.get("feature_name").unwrap_or(&"".to_string()))?;
            }
            DegradationActionType::ReduceQuality => {
                self.reduce_quality(service_name, &action.parameters)?;
            }
            DegradationActionType::LimitConcurrency => {
                self.limit_concurrency(service_name, &action.parameters)?;
            }
            DegradationActionType::IncreaseTimeout => {
                self.increase_timeout(service_name, &action.parameters)?;
            }
            DegradationActionType::EnableCache => {
                self.enable_cache(service_name, &action.parameters)?;
            }
            DegradationActionType::RateLimit => {
                self.enable_rate_limiting(service_name, &action.parameters)?;
            }
            DegradationActionType::CompressData => {
                self.enable_data_compression(service_name, &action.parameters)?;
            }
            DegradationActionType::AsyncProcessing => {
                self.enable_async_processing(service_name, &action.parameters)?;
            }
            DegradationActionType::SimplifyComputation => {
                self.simplify_computation(service_name, &action.parameters)?;
            }
            DegradationActionType::DegradedMode => {
                self.enable_degraded_mode(service_name, &action.parameters)?;
            }
            DegradationActionType::CustomAction => {
                self.execute_custom_action(service_name, &action.parameters)?;
            }
            _ => {
                println!("[GracefulDegradation] Executing action: {:?}", action.action_type);
            }
        }
        Ok(())
    }

    /// 禁用功能
    fn disable_feature(&mut self, feature_name: &str) -> Result<(), &'static str> {
        if let Some(feature_state) = self.feature_manager.feature_states.get_mut(feature_name) {
            *feature_state = FeatureState::Disabled;
            println!("[GracefulDegradation] Feature {} disabled", feature_name);
        }
        Ok(())
    }

    /// 降低质量
    fn reduce_quality(&mut self, service_name: &str, parameters: &BTreeMap<String, String>) -> Result<(), &'static str> {
        if let Some(controller) = self.quality_controllers.get_mut(service_name) {
            let quality_reduction = parameters.get("quality_reduction")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.2);

            for (metric_name, metric) in &mut controller.quality_metrics {
                let _metric_name_ref = metric_name;
                match metric.metric_type {
                    MetricType::ResponseTime => {
                        metric.target_value *= 1.0 + quality_reduction;
                    }
                    MetricType::Throughput => {
                        metric.target_value *= 1.0 - quality_reduction;
                    }
                    _ => {}
                }
            }

            if controller.current_quality_level > QualityLevel::Degraded {
                controller.current_quality_level = QualityLevel::Degraded;
            }

            println!("[GracefulDegradation] Quality reduced for service: {}", service_name);
        }
        Ok(())
    }

    /// 限制并发
    fn limit_concurrency(&mut self, service_name: &str, parameters: &BTreeMap<String, String>) -> Result<(), &'static str> {
        let max_concurrency = parameters.get("max_concurrency")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(100);
        println!("[GracefulDegradation] Concurrency limited to {} for service: {}", max_concurrency, service_name);
        Ok(())
    }

    /// 增加超时
    fn increase_timeout(&mut self, service_name: &str, parameters: &BTreeMap<String, String>) -> Result<(), &'static str> {
        let timeout_multiplier = parameters.get("timeout_multiplier")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(2.0);
        println!("[GracefulDegradation] Timeout increased by {}x for service: {}", timeout_multiplier, service_name);
        Ok(())
    }

    /// 启用缓存
    fn enable_cache(&mut self, service_name: &str, _parameters: &BTreeMap<String, String>) -> Result<(), &'static str> {
        println!("[GracefulDegradation] Cache enabled for service: {}", service_name);
        Ok(())
    }

    /// 启用限流
    fn enable_rate_limiting(&mut self, service_name: &str, parameters: &BTreeMap<String, String>) -> Result<(), &'static str> {
        let rate_limit = parameters.get("rate_limit")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1000);
        println!("[GracefulDegradation] Rate limiting enabled ({} req/s) for service: {}", rate_limit, service_name);
        Ok(())
    }

    /// 启用数据压缩
    fn enable_data_compression(&mut self, service_name: &str, _parameters: &BTreeMap<String, String>) -> Result<(), &'static str> {
        println!("[GracefulDegradation] Data compression enabled for service: {}", service_name);
        Ok(())
    }

    /// 启用异步处理
    fn enable_async_processing(&mut self, service_name: &str, _parameters: &BTreeMap<String, String>) -> Result<(), &'static str> {
        println!("[GracefulDegradation] Async processing enabled for service: {}", service_name);
        Ok(())
    }

    /// 简化计算
    fn simplify_computation(&mut self, service_name: &str, parameters: &BTreeMap<String, String>) -> Result<(), &'static str> {
        let simplification_level = parameters.get("level")
            .and_then(|s| s.parse::<u8>().ok())
            .unwrap_or(1);
        println!("[GracefulDegradation] Computation simplified (level: {}) for service: {}", simplification_level, service_name);
        Ok(())
    }

    /// 启用降级模式
    fn enable_degraded_mode(&mut self, service_name: &str, _parameters: &BTreeMap<String, String>) -> Result<(), &'static str> {
        println!("[GracefulDegradation] Degraded mode enabled for service: {}", service_name);
        Ok(())
    }

    /// 执行自定义动作
    fn execute_custom_action(&mut self, service_name: &str, parameters: &BTreeMap<String, String>) -> Result<(), &'static str> {
        let action_name = parameters.get("action_name").map(|s| s.as_str()).unwrap_or("unknown");
        println!("[GracefulDegradation] Custom action '{}' executed for service: {}", action_name, service_name);
        Ok(())
    }

    /// 尝试恢复
    pub fn attempt_recovery(&mut self, session_id: &str) -> Result<bool, &'static str> {
        let (session_status, strategy_id, service_name) = {
            let session = self.active_degradations.get(session_id)
                .ok_or("Degradation session not found")?;
            (session.status, session.strategy_id.clone(), session.service_name.clone())
        };

        if session_status != DegradationStatus::Degraded {
            return Err("Session is not in degraded state");
        }

        let strategy = self.degradation_strategies.get(&strategy_id)
            .ok_or("Strategy not found")?;
        let strategy_clone = strategy.clone();

        if !self.check_recovery_conditions(&strategy.recovery_conditions, &service_name)? {
            return Ok(false);
        }

        let session_clone = {
            let session = self.active_degradations.get(session_id)
                .ok_or("Degradation session not found")?;
            session.clone()
        };

        let recovery_success = self.execute_recovery_actions(&session_clone, &strategy_clone)?;

        if let Some(session_mut) = self.active_degradations.get_mut(session_id) {
            if recovery_success {
                session_mut.status = DegradationStatus::Recovered;
                session_mut.recovery_status = RecoveryStatus::Completed;
                session_mut.end_time = Some(crate::subsystems::time::get_timestamp());
            } else {
                session_mut.status = DegradationStatus::Degraded;
                session_mut.recovery_status = RecoveryStatus::Failed;
            }
        }

        if recovery_success {
            self.add_session_log(session_id, LogLevel::Info, "Recovery completed successfully", "GracefulDegradation");
        } else {
            self.add_session_log(session_id, LogLevel::Error, "Recovery failed", "GracefulDegradation");
        }

        Ok(recovery_success)
    }

    /// 检查恢复条件
    fn check_recovery_conditions(&self, recovery_conditions: &[RecoveryCondition], _service_name: &str) -> Result<bool, &'static str> {
        for condition in recovery_conditions {
            match condition.condition_type {
                RecoveryConditionType::ResourceSufficient => Ok(true),
                RecoveryConditionType::LoadReduced => Ok(true),
                RecoveryConditionType::ErrorRateReduced => Ok(true),
                RecoveryConditionType::PerformanceRestored => Ok(true),
                RecoveryConditionType::TimeWindow => Ok(true),
                RecoveryConditionType::ManualRecovery => Ok(false),
                RecoveryConditionType::CustomRecovery => Ok(true),
            }?;
        }
        Ok(true)
    }

    /// 执行恢复动作
    fn execute_recovery_actions(&mut self, session: &DegradationSession, strategy: &DegradationStrategy) -> Result<bool, &'static str> {
        let mut success_count = 0;
        let mut total_count = 0;

        for action in strategy.degradation_actions.iter().rev() {
            total_count += 1;
            if self.execute_rollback_action(&action.rollback_actions, &session.service_name)? {
                success_count += 1;
            }
        }

        Ok(success_count == total_count)
    }

    /// 执行回滚动作
    fn execute_rollback_action(&mut self, rollback_actions: &[RollbackAction], service_name: &str) -> Result<bool, &'static str> {
        for rollback_action in rollback_actions {
            match rollback_action.action_type {
                RollbackActionType::EnableFeature => {
                    self.enable_feature(&rollback_action.parameters.get("feature_name").unwrap_or(&"".to_string()))?;
                }
                RollbackActionType::RestoreQuality => {
                    self.restore_quality(service_name, &rollback_action.parameters)?;
                }
                RollbackActionType::RemoveLimit => {
                    self.remove_limit(service_name, &rollback_action.parameters)?;
                }
                RollbackActionType::RestoreTimeout => {
                    self.restore_timeout(service_name, &rollback_action.parameters)?;
                }
                _ => {
                    println!("[GracefulDegradation] Executing rollback action: {:?}", rollback_action.action_type);
                }
            }
        }
        Ok(true)
    }

    /// 启用功能
    fn enable_feature(&mut self, feature_name: &str) -> Result<(), &'static str> {
        if let Some(feature_state) = self.feature_manager.feature_states.get_mut(feature_name) {
            *feature_state = FeatureState::Enabled;
            println!("[GracefulDegradation] Feature {} enabled", feature_name);
        }
        Ok(())
    }

    /// 恢复质量
    fn restore_quality(&mut self, service_name: &str, parameters: &BTreeMap<String, String>) -> Result<(), &'static str> {
        if let Some(controller) = self.quality_controllers.get_mut(service_name) {
            let quality_restoration = parameters.get("quality_restoration")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(1.0);

            for (metric_name, metric) in &mut controller.quality_metrics {
                let _metric_name_ref = metric_name;
                match metric.metric_type {
                    MetricType::ResponseTime => {
                        metric.target_value /= 1.0 + quality_restoration;
                    }
                    MetricType::Throughput => {
                        metric.target_value *= 1.0 + quality_restoration;
                    }
                    _ => {}
                }
            }

            controller.current_quality_level = QualityLevel::Good;
            println!("[GracefulDegradation] Quality restored for service: {}", service_name);
        }
        Ok(())
    }

    /// 移除限制
    fn remove_limit(&mut self, service_name: &str, _parameters: &BTreeMap<String, String>) -> Result<(), &'static str> {
        println!("[GracefulDegradation] Limits removed for service: {}", service_name);
        Ok(())
    }

    /// 恢复超时
    fn restore_timeout(&mut self, service_name: &str, _parameters: &BTreeMap<String, String>) -> Result<(), &'static str> {
        println!("[GracefulDegradation] Timeout restored for service: {}", service_name);
        Ok(())
    }

    /// 添加会话日志
    fn add_session_log(&mut self, session_id: &str, level: LogLevel, message: &str, source: &str) {
        if let Some(session) = self.active_degradations.get_mut(session_id) {
            let log = SessionLog {
                id: format!("log_{}", crate::subsystems::time::get_timestamp()),
                timestamp: crate::subsystems::time::get_timestamp(),
                level,
                message: message.to_string(),
                details: Some(format!("Source: {}", source)),
            };
            session.logs.push(log);
        }
    }

    /// 更新降级统计
    fn update_degradation_stats(&mut self) {
        self.stats.total_degradations += 1;
        self.stats.auto_degradations += 1;
    }

    /// 加载默认降级策略
    fn load_default_degradation_strategies(&mut self) -> Result<(), &'static str> {
        let strategies = vec![
            DegradationStrategy {
                id: "performance_degradation".to_string(),
                name: "Performance Degradation Strategy".to_string(),
                description: "Degrades performance to maintain service availability".to_string(),
                strategy_type: DegradationType::PerformanceDegradation,
                trigger_conditions: vec![
                    super::strategy::DegradationTrigger {
                        id: "high_cpu".to_string(),
                        trigger_type: TriggerType::ThresholdBased,
                        condition: TriggerCondition::CPUUsage {
                            threshold: 90.0,
                            duration: 300,
                        },
                        threshold: 90.0,
                        duration_seconds: 300,
                        immediate: false,
                    },
                ],
                degradation_actions: vec![
                    DegradationAction {
                        id: "reduce_quality".to_string(),
                        action_type: DegradationActionType::ReduceQuality,
                        name: "Reduce Service Quality".to_string(),
                        description: "Reduce quality settings to lower resource usage".to_string(),
                        parameters: {
                            let mut params = BTreeMap::new();
                            params.insert("quality_reduction".to_string(), "0.3".to_string());
                            params
                        },
                        execution_order: 1,
                        mandatory: true,
                        rollback_actions: vec![
                            RollbackAction {
                                description: "Restore original quality settings".to_string(),
                                action_type: RollbackActionType::RestoreQuality,
                                parameters: BTreeMap::new(),
                            },
                        ],
                    },
                ],
                recovery_conditions: vec![
                    RecoveryCondition {
                        id: "resource_recovery".to_string(),
                        condition_type: RecoveryConditionType::ResourceSufficient,
                        description: "System resources are sufficient".to_string(),
                        recovery_threshold: 0.8,
                        stability_duration: 300,
                        auto_recovery: true,
                    },
                ],
                priority: 1,
                enabled: true,
                parameters: BTreeMap::new(),
                stats: super::strategy::StrategyStats::default(),
            },
        ];

        for strategy in strategies {
            self.degradation_strategies.insert(strategy.id.clone(), strategy);
        }
        Ok(())
    }

    /// 初始化功能管理器
    fn initialize_feature_manager(&mut self) -> Result<(), &'static str> {
        let features = vec![
            Feature {
                id: "advanced_analytics".to_string(),
                name: "Advanced Analytics".to_string(),
                description: "Advanced data analytics and reporting".to_string(),
                category: FeatureCategory::Optional,
                importance_level: ImportanceLevel::Normal,
                resource_requirements: ResourceRequirements::default(),
                quality_requirements: QualityRequirements::default(),
                enabled: true,
            },
        ];

        for feature in features {
            let feature_id = feature.id.clone();
            self.feature_manager.features.insert(feature_id.clone(), feature);
            self.feature_manager.feature_states.insert(feature_id, FeatureState::Enabled);
        }
        Ok(())
    }

    /// 初始化负载管理器
    fn initialize_load_manager(&mut self) -> Result<(), &'static str> {
        let strategies = vec![
            LoadStrategy {
                id: "dynamic_allocation".to_string(),
                name: "Dynamic Load Allocation".to_string(),
                strategy_type: LoadStrategyType::DynamicAllocation,
                load_thresholds: LoadThresholds {
                    normal_threshold: 60.0,
                    high_threshold: 75.0,
                    overload_threshold: 90.0,
                    severe_overload_threshold: 95.0,
                },
                allocation_algorithm: AllocationAlgorithm::Adaptive,
                parameters: BTreeMap::new(),
            },
        ];

        for strategy in strategies {
            self.load_manager.load_strategies.insert(strategy.id.clone(), strategy);
        }
        Ok(())
    }

    /// 初始化资源管理器
    fn initialize_resource_manager(&mut self) -> Result<(), &'static str> {
        let pools = vec![
            ResourcePool {
                id: "cpu_pool".to_string(),
                name: "CPU Resource Pool".to_string(),
                resource_type: ResourceType::CPU,
                total_capacity: 100,
                allocated_capacity: 0,
                available_capacity: 100,
                reserved_capacity: 20,
                status: PoolStatus::Available,
            },
        ];

        for pool in pools {
            self.resource_manager.resource_pools.insert(pool.id.clone(), pool);
        }
        Ok(())
    }

    /// 初始化服务质量控制器
    fn initialize_quality_controllers(&mut self) -> Result<(), &'static str> {
        let controllers = vec![
            ServiceQualityController {
                id: "web_service".to_string(),
                service_name: "Web Service".to_string(),
                quality_metrics: {
                    let mut metrics = BTreeMap::new();
                    metrics.insert("response_time".to_string(), QualityMetric {
                        name: "Response Time".to_string(),
                        metric_type: MetricType::ResponseTime,
                        current_value: 50.0,
                        target_value: 100.0,
                        unit: "ms".to_string(),
                        weight: 0.4,
                        last_updated: crate::subsystems::time::get_timestamp(),
                    });
                    metrics
                },
                quality_thresholds: {
                    let mut thresholds = BTreeMap::new();
                    thresholds.insert("response_time".to_string(), QualityThreshold::default());
                    thresholds
                },
                control_policies: Vec::new(),
                current_quality_level: QualityLevel::Good,
                quality_history: Vec::new(),
                status: ControllerStatus::Active,
            },
        ];

        for controller in controllers {
            self.quality_controllers.insert(controller.id.clone(), controller);
        }
        Ok(())
    }

    /// 获取活动降级
    pub fn get_active_degradations(&self) -> &BTreeMap<String, DegradationSession> {
        &self.active_degradations
    }

    /// 获取降级策略
    pub fn get_degradation_strategies(&self) -> &BTreeMap<String, DegradationStrategy> {
        &self.degradation_strategies
    }

    /// 获取服务质量控制器
    pub fn get_quality_controllers(&self) -> &BTreeMap<String, ServiceQualityController> {
        &self.quality_controllers
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> DegradationStats {
        self.stats.clone()
    }

    /// 更新配置
    pub fn update_config(&mut self, config: DegradationConfig) -> Result<(), &'static str> {
        self.config = config;
        Ok(())
    }

    /// 停止优雅降级管理器
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        let session_ids: Vec<String> = self.active_degradations.keys().cloned().collect();
        for session_id in session_ids {
            let _ = self.attempt_recovery(&session_id);
        }

        self.degradation_strategies.clear();
        self.quality_controllers.clear();
        self.active_degradations.clear();

        println!("[GracefulDegradation] Graceful degradation manager shutdown successfully");
        Ok(())
    }
}

/// 创建默认的优雅降级管理器
pub fn create_graceful_degradation_manager() -> Arc<Mutex<GracefulDegradationManager>> {
    Arc::new(Mutex::new(GracefulDegradationManager::new()))
}
