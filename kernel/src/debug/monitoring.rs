// 系统监控模块

extern crate alloc;
//
// 提供全面的系统监控功能，包括资源监控、性能监控、
// 事件监控和健康状态监控。
//
// 主要功能：
// - 实时系统监控
// - 资源使用率监控
// - 性能指标收集
// - 事件监控和告警
// - 健康状态检查
// - 监控数据存储和查询
// - 可视化支持

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::format;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::string::ToString;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;
use spin::Mutex;

use crate::time;

// Import println macro
#[allow(unused_imports)]
use crate::println;

/// 监控指标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    /// 计数器
    Counter,
    /// 计量器
    Gauge,
    /// 直方图
    Histogram,
    /// 摘要
    Summary,
    /// 原始值
    Raw,
}

/// 监控指标
#[derive(Debug, Clone)]
pub struct Metric {
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 指标值
    pub value: f64,
    /// 时间戳
    pub timestamp: u64,
    /// 标签
    pub labels: BTreeMap<String, String>,
    /// 单位
    pub unit: Option<String>,
    /// 描述
    pub description: Option<String>,
}

impl Metric {
    /// 创建新的监控指标
    pub fn new(name: String, metric_type: MetricType, value: f64) -> Self {
        Self {
            name,
            metric_type,
            value,
            timestamp: time::timestamp_millis(),
            labels: BTreeMap::new(),
            unit: None,
            description: None,
        }
    }

    /// 设置标签
    pub fn with_label(mut self, key: String, value: String) -> Self {
        self.labels.insert(key, value);
        self
    }

    /// 设置单位
    pub fn with_unit(mut self, unit: String) -> Self {
        self.unit = Some(unit);
        self
    }

    /// 设置描述
    pub fn with_description(mut self, description: String) -> Self {
        self.description = Some(description);
        self
    }
}

/// 监控事件
#[derive(Debug, Clone)]
pub struct MonitorEvent {
    /// 事件ID
    pub id: u64,
    /// 事件类型
    pub event_type: String,
    /// 事件严重级别
    pub severity: EventSeverity,
    /// 事件标题
    pub title: String,
    /// 事件描述
    pub description: String,
    /// 时间戳
    pub timestamp: u64,
    /// 源组件
    pub source: String,
    /// 标签
    pub labels: BTreeMap<String, String>,
    /// 事件数据
    pub data: BTreeMap<String, String>,
}

/// 事件严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventSeverity {
    /// 调试
    Debug = 0,
    /// 信息
    Info = 1,
    /// 警告
    Warning = 2,
    /// 错误
    Error = 3,
    /// 严重
    Critical = 4,
    /// 致命
    Fatal = 5,
}

/// 监控规则
#[derive(Debug, Clone)]
pub struct MonitorRule {
    /// 规则ID
    pub id: String,
    /// 规则名称
    pub name: String,
    /// 规则描述
    pub description: String,
    /// 监控指标
    pub metric_name: String,
    /// 条件表达式
    pub condition: String,
    /// 阈值
    pub threshold: f64,
    /// 比较操作符
    pub operator: ComparisonOperator,
    /// 事件严重级别
    pub severity: EventSeverity,
    /// 是否启用
    pub enabled: bool,
    /// 评估间隔（秒）
    pub interval: Duration,
    /// 持续时间阈值（秒）
    pub duration: Duration,
    /// 最后评估时间
    pub last_evaluation: u64,
    /// 违规开始时间
    pub violation_start: Option<u64>,
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

/// 系统资源使用情况
#[derive(Debug, Clone)]
pub struct SystemResourceUsage {
    /// CPU使用率（百分比）
    pub cpu_usage: f64,
    /// 内存使用率（百分比）
    pub memory_usage: f64,
    /// 内存使用量（字节）
    pub memory_used: u64,
    /// 内存总量（字节）
    pub memory_total: u64,
    /// 磁盘使用率（百分比）
    pub disk_usage: f64,
    /// 磁盘使用量（字节）
    pub disk_used: u64,
    /// 磁盘总量（字节）
    pub disk_total: u64,
    /// 网络IO（字节/秒）
    pub network_io: NetworkIO,
    /// 时间戳
    pub timestamp: u64,
}

/// 网络IO统计
#[derive(Debug, Clone, Copy)]
pub struct NetworkIO {
    /// 接收字节数
    pub rx_bytes: u64,
    /// 发送字节数
    pub tx_bytes: u64,
    /// 接收包数
    pub rx_packets: u64,
    /// 发送包数
    pub tx_packets: u64,
}

/// 监控配置
#[derive(Debug, Clone)]
pub struct MonitorConfig {
    /// 数据保留期（秒）
    pub retention_period: Duration,
    /// 采样间隔（毫秒）
    pub sample_interval: Duration,
    /// 最大指标数量
    pub max_metrics: usize,
    /// 最大事件数量
    pub max_events: usize,
    /// 是否启用自动清理
    pub auto_cleanup: bool,
    /// 批量操作大小
    pub batch_size: usize,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            retention_period: Duration::from_secs(24 * 60 * 60), // 24小时
            sample_interval: Duration::from_millis(1000),       // 1秒
            max_metrics: 100000,
            max_events: 10000,
            auto_cleanup: true,
            batch_size: 1000,
        }
    }
}

/// 监控统计信息
#[derive(Debug, Default)]
pub struct MonitorStatistics {
    /// 收集的指标总数
    pub total_metrics_collected: AtomicU64,
    /// 生成的事件总数
    pub total_events_generated: AtomicU64,
    /// 规则评估总数
    pub total_rule_evaluations: AtomicU64,
    /// 告警触发总数
    pub total_alerts_triggered: AtomicU64,
    /// 当前活跃指标数
    pub active_metrics: AtomicUsize,
    /// 当前活跃规则数
    pub active_rules: AtomicUsize,
    /// 当前事件队列大小
    pub event_queue_size: AtomicUsize,
    /// 监控数据大小（字节）
    pub monitor_data_size: AtomicU64,
}

/// 监控引擎
pub struct MonitorEngine {
    /// 配置
    config: MonitorConfig,
    /// 监控指标存储
    metrics: Arc<Mutex<BTreeMap<String, Vec<Metric>>>>,
    /// 监控事件存储
    events: Arc<Mutex<Vec<MonitorEvent>>>,
    /// 监控规则
    rules: Arc<Mutex<BTreeMap<String, MonitorRule>>>,
    /// 系统资源监控
    resource_monitor: Arc<Mutex<SystemResourceMonitor>>,
    /// 事件监听器
    event_listeners: Arc<Mutex<Vec<EventListener>>>,
    /// 统计信息
    statistics: MonitorStatistics,
    /// 是否已启动
    running: core::sync::atomic::AtomicBool,
}

/// 系统资源监控器
pub struct SystemResourceMonitor {
    /// 历史数据
    history: Vec<SystemResourceUsage>,
    /// 最大历史记录数
    max_history: usize,
    /// 上次更新时间
    last_update: u64,
}

/// 事件监听器
pub struct EventListener {
    /// 监听器ID
    pub id: String,
    /// 监听器名称
    pub name: String,
    /// 事件过滤器
    pub filter: EventFilter,
    /// 回调函数
    pub callback: Box<dyn Fn(&MonitorEvent) + Send>,
}

/// 事件过滤器
#[derive(Debug, Clone)]
pub struct EventFilter {
    /// 事件类型过滤器
    pub event_types: Vec<String>,
    /// 严重级别过滤器
    pub severities: Vec<EventSeverity>,
    /// 源过滤器
    pub sources: Vec<String>,
    /// 标签过滤器
    pub label_filters: BTreeMap<String, String>,
}

impl MonitorEngine {
    /// 创建新的监控引擎
    pub fn new(config: MonitorConfig) -> Self {
        Self {
            config,
            metrics: Arc::new(Mutex::new(BTreeMap::new())),
            events: Arc::new(Mutex::new(Vec::new())),
            rules: Arc::new(Mutex::new(BTreeMap::new())),
            resource_monitor: Arc::new(Mutex::new(SystemResourceMonitor::new(1000))),
            event_listeners: Arc::new(Mutex::new(Vec::new())),
            statistics: MonitorStatistics::default(),
            running: core::sync::atomic::AtomicBool::new(false),
        }
    }

    /// 启动监控
    pub fn start(&self) -> Result<(), MonitorError> {
        if self.running.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.running.store(true, Ordering::SeqCst);
        crate::println!("[monitor] 监控引擎已启动");

        Ok(())
    }

    /// 停止监控
    pub fn stop(&self) -> Result<(), MonitorError> {
        self.running.store(false, Ordering::SeqCst);
        crate::println!("[monitor] 监控引擎已停止");

        Ok(())
    }

    /// 记录监控指标
    pub fn record_metric(&self, metric: Metric) -> Result<(), MonitorError> {
        let mut metrics = self.metrics.lock();

        // 添加指标到存储
        let metric_list = metrics.entry(metric.name.clone()).or_insert_with(Vec::new);
        metric_list.push(metric.clone());

        // 限制指标数量
        if metric_list.len() > self.config.max_metrics {
            metric_list.remove(0);
        }

        // 更新统计
        self.statistics.total_metrics_collected.fetch_add(1, Ordering::SeqCst);
        self.statistics.active_metrics.store(
            metrics.values().map(|v| v.len()).sum(),
            Ordering::SeqCst
        );

        // 评估相关规则
        self.evaluate_rules_for_metric(&metric)?;

        Ok(())
    }

    /// 生成监控事件
    pub fn emit_event(&self, event: MonitorEvent) -> Result<(), MonitorError> {
        let mut events = self.events.lock();

        // 添加事件到存储
        events.push(event.clone());

        // 限制事件数量
        if events.len() > self.config.max_events {
            events.remove(0);
        }

        // 更新统计
        self.statistics.total_events_generated.fetch_add(1, Ordering::SeqCst);
        self.statistics.event_queue_size.store(events.len(), Ordering::SeqCst);

        // 通知事件监听器
        self.notify_event_listeners(&event)?;

        Ok(())
    }

    /// 添加监控规则
    pub fn add_rule(&self, rule: MonitorRule) -> Result<(), MonitorError> {
        let mut rules = self.rules.lock();
        rules.insert(rule.id.clone(), rule.clone());

        self.statistics.active_rules.store(rules.len(), Ordering::SeqCst);

        crate::println!("[monitor] 添加监控规则: {}", rule.name);

        Ok(())
    }

    /// 移除监控规则
    pub fn remove_rule(&self, rule_id: &str) -> Result<(), MonitorError> {
        let mut rules = self.rules.lock();
        if rules.remove(rule_id).is_some() {
            self.statistics.active_rules.store(rules.len(), Ordering::SeqCst);
            crate::println!("[monitor] 移除监控规则: {}", rule_id);
        }

        Ok(())
    }

    /// 添加事件监听器
    pub fn add_event_listener(&self, listener: EventListener) -> Result<(), MonitorError> {
        let mut listeners = self.event_listeners.lock();
        listeners.push(listener);

        Ok(())
    }

    /// 获取系统资源使用情况
    pub fn get_resource_usage(&self) -> Result<SystemResourceUsage, MonitorError> {
        let mut monitor = self.resource_monitor.lock();
        monitor.update_usage();
        Ok(monitor.current_usage())
    }

    /// 查询监控指标
    pub fn query_metrics(&self, metric_name: &str, start_time: u64, end_time: u64) -> Result<Vec<Metric>, MonitorError> {
        let metrics = self.metrics.lock();

        if let Some(metric_list) = metrics.get(metric_name) {
            let filtered_metrics: Vec<Metric> = metric_list
                .iter()
                .filter(|m| m.timestamp >= start_time && m.timestamp <= end_time)
                .cloned()
                .collect();

            Ok(filtered_metrics)
        } else {
            Ok(Vec::new())
        }
    }

    /// 查询监控事件
    pub fn query_events(&self, start_time: u64, end_time: u64) -> Result<Vec<MonitorEvent>, MonitorError> {
        let events = self.events.lock();

        let filtered_events: Vec<MonitorEvent> = events
            .iter()
            .filter(|e| e.timestamp >= start_time && e.timestamp <= end_time)
            .cloned()
            .collect();

        Ok(filtered_events)
    }

    /// 获取监控统计信息
    pub fn get_statistics(&self) -> MonitorStatistics {
        MonitorStatistics {
            total_metrics_collected: AtomicU64::new(
                self.statistics.total_metrics_collected.load(Ordering::SeqCst)
            ),
            total_events_generated: AtomicU64::new(
                self.statistics.total_events_generated.load(Ordering::SeqCst)
            ),
            total_rule_evaluations: AtomicU64::new(
                self.statistics.total_rule_evaluations.load(Ordering::SeqCst)
            ),
            total_alerts_triggered: AtomicU64::new(
                self.statistics.total_alerts_triggered.load(Ordering::SeqCst)
            ),
            active_metrics: AtomicUsize::new(
                self.statistics.active_metrics.load(Ordering::SeqCst)
            ),
            active_rules: AtomicUsize::new(
                self.statistics.active_rules.load(Ordering::SeqCst)
            ),
            event_queue_size: AtomicUsize::new(
                self.statistics.event_queue_size.load(Ordering::SeqCst)
            ),
            monitor_data_size: AtomicU64::new(
                self.statistics.monitor_data_size.load(Ordering::SeqCst)
            ),
        }
    }

    /// 评估监控规则
    fn evaluate_rules_for_metric(&self, metric: &Metric) -> Result<(), MonitorError> {
        let rules = self.rules.lock();

        for rule in rules.values() {
            if rule.enabled && rule.metric_name == metric.name {
                if let Err(e) = self.evaluate_rule(rule, metric) {
                    crate::println!("[monitor] 规则评估失败 {}: {:?}", rule.name, e);
                }
            }
        }

        Ok(())
    }

    /// 评估单个规则
    fn evaluate_rule(&self, rule: &MonitorRule, metric: &Metric) -> Result<(), MonitorError> {
        // 检查条件
        let condition_met = match rule.operator {
            ComparisonOperator::GreaterThan => metric.value > rule.threshold,
            ComparisonOperator::GreaterThanOrEqual => metric.value >= rule.threshold,
            ComparisonOperator::LessThan => metric.value < rule.threshold,
            ComparisonOperator::LessThanOrEqual => metric.value <= rule.threshold,
            ComparisonOperator::Equal => (metric.value - rule.threshold).abs() < f64::EPSILON,
            ComparisonOperator::NotEqual => (metric.value - rule.threshold).abs() >= f64::EPSILON,
        };

        let current_time = time::timestamp_millis();

        if condition_met {
            // 条件满足，检查持续时间
            let violation_start = rule.violation_start.unwrap_or(current_time);

            if current_time - violation_start >= rule.duration.as_millis() as u64 {
                // 触发告警
                self.trigger_alert(rule, metric)?;
            }
        } else {
            // 条件不满足，重置违规开始时间
            // 这里需要更新规则，但由于是不可变引用，实际实现需要用其他方式
        }

        self.statistics.total_rule_evaluations.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    /// 触发告警
    fn trigger_alert(&self, rule: &MonitorRule, metric: &Metric) -> Result<(), MonitorError> {
        let event = MonitorEvent {
            id: self.generate_event_id(),
            event_type: "alert".to_string(),
            severity: rule.severity,
            title: format!("监控告警: {}", rule.name),
            description: format!(
                "指标 {} 值 {} {} 阈值 {}",
                metric.name, metric.value,
                self.operator_to_string(rule.operator),
                rule.threshold
            ),
            timestamp: time::timestamp_millis(),
            source: "monitor".to_string(),
            labels: metric.labels.clone(),
            data: {
                let mut data = BTreeMap::new();
                data.insert("rule_id".to_string(), rule.id.clone());
                data.insert("metric_name".to_string(), metric.name.clone());
                data.insert("metric_value".to_string(), metric.value.to_string());
                data.insert("threshold".to_string(), rule.threshold.to_string());
                data
            },
        };

        self.emit_event(event)?;
        self.statistics.total_alerts_triggered.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    /// 通知事件监听器
    fn notify_event_listeners(&self, event: &MonitorEvent) -> Result<(), MonitorError> {
        let listeners = self.event_listeners.lock();

        for listener in listeners.iter() {
            if self.event_matches_filter(event, &listener.filter) {
                (listener.callback)(event);
            }
        }

        Ok(())
    }

    /// 检查事件是否匹配过滤器
    fn event_matches_filter(&self, event: &MonitorEvent, filter: &EventFilter) -> bool {
        // 检查事件类型
        if !filter.event_types.is_empty() && !filter.event_types.contains(&event.event_type) {
            return false;
        }

        // 检查严重级别
        if !filter.severities.is_empty() && !filter.severities.contains(&event.severity) {
            return false;
        }

        // 检查源
        if !filter.sources.is_empty() && !filter.sources.contains(&event.source) {
            return false;
        }

        // 检查标签
        for (key, value) in &filter.label_filters {
            if let Some(event_value) = event.labels.get(key) {
                if event_value != value {
                    return false;
                }
            } else {
                return false;
            }
        }

        true
    }

    /// 生成事件ID
    fn generate_event_id(&self) -> u64 {
        static NEXT_EVENT_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_EVENT_ID.fetch_add(1, Ordering::SeqCst)
    }

    /// 将操作符转换为字符串
    fn operator_to_string(&self, operator: ComparisonOperator) -> &'static str {
        match operator {
            ComparisonOperator::GreaterThan => ">",
            ComparisonOperator::GreaterThanOrEqual => ">=",
            ComparisonOperator::LessThan => "<",
            ComparisonOperator::LessThanOrEqual => "<=",
            ComparisonOperator::Equal => "==",
            ComparisonOperator::NotEqual => "!=",
        }
    }

    /// 清理过期数据
    pub fn cleanup_expired_data(&self) -> Result<(), MonitorError> {
        let cutoff_time = time::timestamp_millis() - self.config.retention_period.as_millis() as u64;

        // 清理过期指标
        let mut metrics = self.metrics.lock();
        for metric_list in metrics.values_mut() {
            metric_list.retain(|m| m.timestamp >= cutoff_time);
        }

        // 清理过期事件
        let mut events = self.events.lock();
        events.retain(|e| e.timestamp >= cutoff_time);

        // 更新统计
        self.statistics.active_metrics.store(
            metrics.values().map(|v| v.len()).sum(),
            Ordering::SeqCst
        );
        self.statistics.event_queue_size.store(events.len(), Ordering::SeqCst);

        Ok(())
    }
}

impl SystemResourceMonitor {
    /// 创建新的系统资源监控器
    pub fn new(max_history: usize) -> Self {
        Self {
            history: Vec::new(),
            max_history,
            last_update: 0,
        }
    }

    /// 更新资源使用情况
    pub fn update_usage(&mut self) {
        let current_time = time::timestamp_millis();

        // 简单的实现，实际应该从系统获取真实数据
        let usage = SystemResourceUsage {
            cpu_usage: 0.1, // 10%
            memory_usage: 0.3, // 30%
            memory_used: 300 * 1024 * 1024, // 300MB
            memory_total: 1024 * 1024 * 1024, // 1GB
            disk_usage: 0.2, // 20%
            disk_used: 2 * 1024 * 1024 * 1024, // 2GB
            disk_total: 10 * 1024 * 1024 * 1024, // 10GB
            network_io: NetworkIO {
                rx_bytes: 1024 * 1024,
                tx_bytes: 512 * 1024,
                rx_packets: 1000,
                tx_packets: 500,
            },
            timestamp: current_time,
        };

        self.history.push(usage);

        // 限制历史记录数量
        if self.history.len() > self.max_history {
            self.history.remove(0);
        }

        self.last_update = current_time;
    }

    /// 获取当前资源使用情况
    pub fn current_usage(&self) -> SystemResourceUsage {
        self.history.last().cloned().unwrap_or_else(|| SystemResourceUsage {
            cpu_usage: 0.0,
            memory_usage: 0.0,
            memory_used: 0,
            memory_total: 0,
            disk_usage: 0.0,
            disk_used: 0,
            disk_total: 0,
            network_io: NetworkIO {
                rx_bytes: 0,
                tx_bytes: 0,
                rx_packets: 0,
                tx_packets: 0,
            },
            timestamp: time::timestamp_millis(),
        })
    }

    /// 获取历史资源使用情况
    pub fn get_history(&self) -> &[SystemResourceUsage] {
        &self.history
    }
}

/// 监控错误类型
#[derive(Debug, Clone)]
pub enum MonitorError {
    /// 监控引擎未启动
    NotStarted,
    /// 指标不存在
    MetricNotFound(String),
    /// 规则不存在
    RuleNotFound(String),
    /// 事件监听器错误
    EventListenerError(String),
    /// 存储错误
    StorageError(String),
    /// 配置错误
    ConfigurationError(String),
}

impl core::fmt::Display for MonitorError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MonitorError::NotStarted => write!(f, "监控引擎未启动"),
            MonitorError::MetricNotFound(name) => write!(f, "指标不存在: {}", name),
            MonitorError::RuleNotFound(id) => write!(f, "规则不存在: {}", id),
            MonitorError::EventListenerError(msg) => write!(f, "事件监听器错误: {}", msg),
            MonitorError::StorageError(msg) => write!(f, "存储错误: {}", msg),
            MonitorError::ConfigurationError(msg) => write!(f, "配置错误: {}", msg),
        }
    }
}

/// 全局监控引擎实例
static MONITOR_ENGINE: spin::Mutex<Option<MonitorEngine>> = spin::Mutex::new(None);

/// 初始化监控子系统
pub fn init() -> Result<(), MonitorError> {
    let config = MonitorConfig::default();
    let engine = MonitorEngine::new(config);

    // 添加默认监控规则
    setup_default_monitoring_rules(&engine)?;

    // 启动监控引擎
    engine.start()?;

    let mut global_engine = MONITOR_ENGINE.lock();
    *global_engine = Some(engine);

    crate::println!("[monitor] 监控子系统初始化完成");
    Ok(())
}

/// 设置默认监控规则
fn setup_default_monitoring_rules(engine: &MonitorEngine) -> Result<(), MonitorError> {
    // CPU使用率监控
    let cpu_rule = MonitorRule {
        id: "cpu_usage_high".to_string(),
        name: "CPU使用率过高".to_string(),
        description: "当CPU使用率超过80%时触发告警".to_string(),
        metric_name: "system.cpu.usage".to_string(),
        condition: "cpu_usage > 0.8".to_string(),
        threshold: 80.0,
        operator: ComparisonOperator::GreaterThan,
        severity: EventSeverity::Warning,
        enabled: true,
        interval: Duration::from_secs(5),
        duration: Duration::from_secs(30),
        last_evaluation: 0,
        violation_start: None,
    };

    engine.add_rule(cpu_rule)?;

    // 内存使用率监控
    let memory_rule = MonitorRule {
        id: "memory_usage_high".to_string(),
        name: "内存使用率过高".to_string(),
        description: "当内存使用率超过90%时触发告警".to_string(),
        metric_name: "system.memory.usage".to_string(),
        condition: "memory_usage > 0.9".to_string(),
        threshold: 90.0,
        operator: ComparisonOperator::GreaterThan,
        severity: EventSeverity::Warning,
        enabled: true,
        interval: Duration::from_secs(5),
        duration: Duration::from_secs(30),
        last_evaluation: 0,
        violation_start: None,
    };

    engine.add_rule(memory_rule)?;

    // 磁盘使用率监控
    let disk_rule = MonitorRule {
        id: "disk_usage_high".to_string(),
        name: "磁盘使用率过高".to_string(),
        description: "当磁盘使用率超过85%时触发告警".to_string(),
        metric_name: "system.disk.usage".to_string(),
        condition: "disk_usage > 0.85".to_string(),
        threshold: 85.0,
        operator: ComparisonOperator::GreaterThan,
        severity: EventSeverity::Warning,
        enabled: true,
        interval: Duration::from_secs(60),
        duration: Duration::from_secs(60),
        last_evaluation: 0,
        violation_start: None,
    };

    engine.add_rule(disk_rule)?;

    Ok(())
}

/// 获取全局监控引擎
pub fn get_monitor_engine() -> spin::MutexGuard<'static, Option<MonitorEngine>> {
    MONITOR_ENGINE.lock()
}
