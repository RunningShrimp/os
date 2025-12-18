//! 性能监控模块
//!
//! 本模块提供性能监控功能，合并自nos-perf。

use nos_api::Result;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::Mutex;

/// 性能监控器
pub struct PerformanceMonitor {
    metrics: Mutex<BTreeMap<String, PerformanceMetric>>,
    collectors: Mutex<Vec<Arc<dyn PerformanceCollector>>>,
}

impl PerformanceMonitor {
    /// 创建新的性能监控器
    pub fn new() -> Self {
        Self {
            metrics: Mutex::new(BTreeMap::new()),
            collectors: Mutex::new(Vec::new()),
        }
    }
    
    /// 添加性能收集器
    pub fn add_collector(&self, collector: Arc<dyn PerformanceCollector>) {
        let mut collectors = self.collectors.lock();
        collectors.push(collector);
    }
    
    /// 移除性能收集器
    pub fn remove_collector(&self, collector_name: &str) {
        let mut collectors = self.collectors.lock();
        collectors.retain(|c| c.name() != collector_name);
    }
    
    /// 收集性能指标
    pub fn collect_metrics(&self) -> Result<BTreeMap<String, PerformanceMetric>> {
        let collectors = self.collectors.lock();
        let mut all_metrics = BTreeMap::new();
        
        for collector in collectors.iter() {
            let metrics = collector.collect()?;
            for (name, metric) in metrics {
                all_metrics.insert(name, metric);
            }
        }
        
        // 更新内部指标
        {
            let mut internal_metrics = self.metrics.lock();
            for (name, metric) in &all_metrics {
                internal_metrics.insert(name.clone(), metric.clone());
            }
        }
        
        Ok(all_metrics)
    }
    
    /// 获取特定指标
    pub fn get_metric(&self, name: &str) -> Option<PerformanceMetric> {
        let metrics = self.metrics.lock();
        metrics.get(name).cloned()
    }
    
    /// 获取所有指标
    pub fn get_all_metrics(&self) -> BTreeMap<String, PerformanceMetric> {
        let metrics = self.metrics.lock();
        metrics.clone()
    }
    
    /// 清除所有指标
    pub fn clear_metrics(&self) {
        let mut metrics = self.metrics.lock();
        metrics.clear();
    }
}

/// 性能指标
#[derive(Debug, Clone)]
pub struct PerformanceMetric {
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 指标值
    pub value: MetricValue,
    /// 指标单位
    pub unit: String,
    /// 时间戳
    pub timestamp: u64,
    /// 标签
    pub tags: BTreeMap<String, String>,
}

/// 指标类型
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
}

/// 指标值
#[derive(Debug, Clone)]
pub enum MetricValue {
    /// 整数值
    Integer(i64),
    /// 浮点数值
    Float(f64),
    /// 布尔值
    Boolean(bool),
    /// 字符串值
    String(String),
    /// 数组值
    Array(Vec<MetricValue>),
}

/// 性能收集器
pub trait PerformanceCollector: Send + Sync {
    /// 收集性能指标
    fn collect(&self) -> Result<BTreeMap<String, PerformanceMetric>>;
    
    /// 获取收集器名称
    fn name(&self) -> &str;
    
    /// 获取收集器描述
    fn description(&self) -> &str {
        "Performance collector"
    }
}

/// CPU性能收集器
pub struct CpuPerformanceCollector {
    name: String,
}

impl CpuPerformanceCollector {
    /// 创建新的CPU性能收集器
    pub fn new() -> Self {
        Self {
            name: "cpu".to_string(),
        }
    }
}

impl PerformanceCollector for CpuPerformanceCollector {
    fn collect(&self) -> Result<BTreeMap<String, PerformanceMetric>> {
        let mut metrics = BTreeMap::new();
        
        // 占位符实现：收集CPU使用率
        metrics.insert("cpu_usage".to_string(), PerformanceMetric {
            name: "cpu_usage".to_string(),
            metric_type: MetricType::Gauge,
            value: MetricValue::Float(0.0),
            unit: "percent".to_string(),
            timestamp: nos_api::event::get_time_ns(),
            tags: BTreeMap::new(),
        });
        
        // 占位符实现：收集CPU温度
        metrics.insert("cpu_temperature".to_string(), PerformanceMetric {
            name: "cpu_temperature".to_string(),
            metric_type: MetricType::Gauge,
            value: MetricValue::Float(0.0),
            unit: "celsius".to_string(),
            timestamp: nos_api::event::get_time_ns(),
            tags: BTreeMap::new(),
        });
        
        Ok(metrics)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        "CPU performance collector"
    }
}

/// 内存性能收集器
pub struct MemoryPerformanceCollector {
    name: String,
}

impl MemoryPerformanceCollector {
    /// 创建新的内存性能收集器
    pub fn new() -> Self {
        Self {
            name: "memory".to_string(),
        }
    }
}

impl PerformanceCollector for MemoryPerformanceCollector {
    fn collect(&self) -> Result<BTreeMap<String, PerformanceMetric>> {
        let mut metrics = BTreeMap::new();
        
        // 占位符实现：收集内存使用量
        metrics.insert("memory_usage".to_string(), PerformanceMetric {
            name: "memory_usage".to_string(),
            metric_type: MetricType::Gauge,
            value: MetricValue::Integer(0),
            unit: "bytes".to_string(),
            timestamp: nos_api::event::get_time_ns(),
            tags: BTreeMap::new(),
        });
        
        // 占位符实现：收集内存使用率
        metrics.insert("memory_usage_percent".to_string(), PerformanceMetric {
            name: "memory_usage_percent".to_string(),
            metric_type: MetricType::Gauge,
            value: MetricValue::Float(0.0),
            unit: "percent".to_string(),
            timestamp: nos_api::event::get_time_ns(),
            tags: BTreeMap::new(),
        });
        
        Ok(metrics)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        "Memory performance collector"
    }
}

/// 系统调用性能收集器
pub struct SyscallPerformanceCollector {
    name: String,
}

impl SyscallPerformanceCollector {
    /// 创建新的系统调用性能收集器
    pub fn new() -> Self {
        Self {
            name: "syscall".to_string(),
        }
    }
}

impl PerformanceCollector for SyscallPerformanceCollector {
    fn collect(&self) -> Result<BTreeMap<String, PerformanceMetric>> {
        let mut metrics = BTreeMap::new();
        
        // 占位符实现：收集系统调用总数
        metrics.insert("syscall_total".to_string(), PerformanceMetric {
            name: "syscall_total".to_string(),
            metric_type: MetricType::Counter,
            value: MetricValue::Integer(0),
            unit: "count".to_string(),
            timestamp: nos_api::event::get_time_ns(),
            tags: BTreeMap::new(),
        });
        
        // 占位符实现：收集系统调用错误数
        metrics.insert("syscall_errors".to_string(), PerformanceMetric {
            name: "syscall_errors".to_string(),
            metric_type: MetricType::Counter,
            value: MetricValue::Integer(0),
            unit: "count".to_string(),
            timestamp: nos_api::event::get_time_ns(),
            tags: BTreeMap::new(),
        });
        
        Ok(metrics)
    }
    
    fn name(&self) -> &str {
        &self.name
    }
    
    fn description(&self) -> &str {
        "System call performance collector"
    }
}

/// 全局性能监控器
static mut GLOBAL_PERFORMANCE_MONITOR: Option<Arc<PerformanceMonitor>> = None;
static PERFORMANCE_MONITOR_INIT: Mutex<bool> = Mutex::new(false);

/// 初始化全局性能监控器
pub fn init_performance_monitor() -> Result<()> {
    let mut is_init = PERFORMANCE_MONITOR_INIT.lock();
    if *is_init {
        return Ok(());
    }
    
    let monitor = Arc::new(PerformanceMonitor::new());
    
    // 添加默认收集器
    monitor.add_collector(Arc::new(CpuPerformanceCollector::new()));
    monitor.add_collector(Arc::new(MemoryPerformanceCollector::new()));
    monitor.add_collector(Arc::new(SyscallPerformanceCollector::new()));
    
    unsafe {
        GLOBAL_PERFORMANCE_MONITOR = Some(monitor);
    }
    *is_init = true;
    Ok(())
}

/// 获取全局性能监控器
pub fn get_performance_monitor() -> Arc<PerformanceMonitor> {
    unsafe {
        GLOBAL_PERFORMANCE_MONITOR
            .as_ref()
            .expect("Performance monitor not initialized")
            .clone()
    }
}

/// 收集性能指标
pub fn collect_performance_metrics() -> Result<BTreeMap<String, PerformanceMetric>> {
    get_performance_monitor().collect_metrics()
}

/// 获取特定性能指标
pub fn get_performance_metric(name: &str) -> Option<PerformanceMetric> {
    get_performance_monitor().get_metric(name)
}

/// 获取所有性能指标
pub fn get_all_performance_metrics() -> BTreeMap<String, PerformanceMetric> {
    get_performance_monitor().get_all_metrics()
}

/// 清除所有性能指标
pub fn clear_performance_metrics() {
    get_performance_monitor().clear_metrics()
}