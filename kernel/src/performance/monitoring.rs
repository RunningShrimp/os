//! 性能监控框架
//!
//! 这个模块实现了一个全面的性能监控系统，包含以下功能：
//! - 实时性能指标收集
//! - 性能数据分析和报告
//! - 性能阈值告警
//! - 性能趋势分析
//! - 自动性能调优

extern crate alloc;

use alloc::{string::String, vec::Vec, collections::BTreeMap};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use crate::subsystems::sync::{Mutex, SpinLock};
use crate::subsystems::mm::optimized_memory_manager::MemoryStats;
use crate::vfs::optimized_filesystem::FileSystemStatsSnapshot;
use crate::io::optimized_io_manager::IoPerformanceStatsSnapshot;

/// 性能指标类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MetricType {
    /// CPU使用率
    CpuUsage,
    /// 内存使用率
    MemoryUsage,
    /// 内存分配率
    MemoryAllocationRate,
    /// 内存碎片率
    MemoryFragmentation,
    /// I/O吞吐量
    IoThroughput,
    /// I/O延迟
    IoLatency,
    /// I/O队列深度
    IoQueueDepth,
    /// 文件系统缓存命中率
    FsCacheHitRate,
    /// 上下文切换率
    ContextSwitchRate,
    /// 系统调用率
    SyscallRate,
    /// 网络吞吐量
    NetworkThroughput,
    /// 网络延迟
    NetworkLatency,
}

/// 性能指标
#[derive(Debug, Clone)]
pub struct PerformanceMetric {
    /// 指标类型
    pub metric_type: MetricType,
    /// 指标名称
    pub name: String,
    /// 当前值
    pub current_value: f64,
    /// 平均值
    pub avg_value: f64,
    /// 最小值
    pub min_value: f64,
    /// 最大值
    pub max_value: f64,
    /// 单位
    pub unit: String,
    /// 采样时间
    pub sample_time: u64,
    /// 采样计数
    pub sample_count: u64,
}

/// 性能阈值
#[derive(Debug, Clone)]
pub struct PerformanceThreshold {
    /// 指标类型
    pub metric_type: MetricType,
    /// 警告阈值
    pub warning_threshold: f64,
    /// 严重阈值
    pub critical_threshold: f64,
    /// 比较操作符
    pub comparator: ThresholdComparator,
    /// 是否启用
    pub enabled: bool,
}

/// 阈值比较操作符
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThresholdComparator {
    /// 大于
    GreaterThan,
    /// 小于
    LessThan,
    /// 等于
    Equal,
    /// 大于等于
    GreaterThanOrEqual,
    /// 小于等于
    LessThanOrEqual,
}

/// 性能告警
#[derive(Debug, Clone)]
pub struct PerformanceAlert {
    /// 告警ID
    pub alert_id: usize,
    /// 告警类型
    pub alert_type: AlertType,
    /// 指标类型
    pub metric_type: MetricType,
    /// 当前值
    pub current_value: f64,
    /// 阈值
    pub threshold: f64,
    /// 告警时间
    pub alert_time: u64,
    /// 告警消息
    pub message: String,
}

/// 告警类型
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AlertType {
    /// 警告
    Warning,
    /// 严重
    Critical,
    /// 信息
    Info,
}

/// 性能趋势
#[derive(Debug, Clone)]
pub struct PerformanceTrend {
    /// 指标类型
    pub metric_type: MetricType,
    /// 趋势方向
    pub direction: TrendDirection,
    /// 趋势强度（0-1）
    pub strength: f64,
    /// 预测值
    pub predicted_value: f64,
    /// 预测时间
    pub predicted_time: u64,
}

/// 趋势方向
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TrendDirection {
    /// 上升
    Increasing,
    /// 下降
    Decreasing,
    /// 稳定
    Stable,
}

/// 性能监控器
pub struct PerformanceMonitor {
    /// 性能指标
    metrics: SpinLock<BTreeMap<MetricType, PerformanceMetric>>,
    /// 性能阈值
    thresholds: Mutex<Vec<PerformanceThreshold>>,
    /// 活动告警
    active_alerts: Mutex<Vec<PerformanceAlert>>,
    /// 性能趋势
    trends: Mutex<Vec<PerformanceTrend>>,
    /// 历史数据
    history: Mutex<BTreeMap<MetricType, Vec<f64>>>,
    /// 历史数据最大长度
    max_history_length: usize,
    /// 采样间隔（毫秒）
    sample_interval: u64,
    /// 下一次采样时间
    next_sample_time: AtomicU64,
    /// 告警ID计数器
    next_alert_id: AtomicUsize,
    /// 是否启用
    enabled: AtomicUsize,
}

impl PerformanceMonitor {
    /// 创建新的性能监控器
    pub fn new() -> Self {
        Self {
            metrics: SpinLock::new(BTreeMap::new()),
            thresholds: Mutex::new(Vec::new()),
            active_alerts: Mutex::new(Vec::new()),
            trends: Mutex::new(Vec::new()),
            history: Mutex::new(BTreeMap::new()),
            max_history_length: 100, // 保留最近100个采样点
            sample_interval: 1000, // 1秒采样间隔
            next_sample_time: AtomicU64::new(0),
            next_alert_id: AtomicUsize::new(1),
            enabled: AtomicUsize::new(1),
        }
    }

    /// 初始化性能监控器
    pub fn init(&self) {
        // 初始化默认指标
        self.init_default_metrics();
        
        // 初始化默认阈值
        self.init_default_thresholds();
        
        crate::println!("[perf_monitor] Performance monitor initialized");
    }

    /// 初始化默认指标
    fn init_default_metrics(&self) {
        let mut metrics = self.metrics.lock();
        
        // CPU使用率
        metrics.insert(MetricType::CpuUsage, PerformanceMetric {
            metric_type: MetricType::CpuUsage,
            name: "CPU Usage".to_string(),
            current_value: 0.0,
            avg_value: 0.0,
            min_value: 0.0,
            max_value: 0.0,
            unit: "%".to_string(),
            sample_time: 0,
            sample_count: 0,
        });
        
        // 内存使用率
        metrics.insert(MetricType::MemoryUsage, PerformanceMetric {
            metric_type: MetricType::MemoryUsage,
            name: "Memory Usage".to_string(),
            current_value: 0.0,
            avg_value: 0.0,
            min_value: 0.0,
            max_value: 0.0,
            unit: "%".to_string(),
            sample_time: 0,
            sample_count: 0,
        });
        
        // I/O延迟
        metrics.insert(MetricType::IoLatency, PerformanceMetric {
            metric_type: MetricType::IoLatency,
            name: "I/O Latency".to_string(),
            current_value: 0.0,
            avg_value: 0.0,
            min_value: 0.0,
            max_value: 0.0,
            unit: "ns".to_string(),
            sample_time: 0,
            sample_count: 0,
        });
        
        // 文件系统缓存命中率
        metrics.insert(MetricType::FsCacheHitRate, PerformanceMetric {
            metric_type: MetricType::FsCacheHitRate,
            name: "FS Cache Hit Rate".to_string(),
            current_value: 0.0,
            avg_value: 0.0,
            min_value: 0.0,
            max_value: 0.0,
            unit: "%".to_string(),
            sample_time: 0,
            sample_count: 0,
        });
    }

    /// 初始化默认阈值
    fn init_default_thresholds(&self) {
        let mut thresholds = self.thresholds.lock();
        
        // CPU使用率阈值
        thresholds.push(PerformanceThreshold {
            metric_type: MetricType::CpuUsage,
            warning_threshold: 70.0,
            critical_threshold: 90.0,
            comparator: ThresholdComparator::GreaterThanOrEqual,
            enabled: true,
        });
        
        // 内存使用率阈值
        thresholds.push(PerformanceThreshold {
            metric_type: MetricType::MemoryUsage,
            warning_threshold: 80.0,
            critical_threshold: 95.0,
            comparator: ThresholdComparator::GreaterThanOrEqual,
            enabled: true,
        });
        
        // I/O延迟阈值
        thresholds.push(PerformanceThreshold {
            metric_type: MetricType::IoLatency,
            warning_threshold: 10_000_000.0, // 10ms
            critical_threshold: 50_000_000.0, // 50ms
            comparator: ThresholdComparator::GreaterThanOrEqual,
            enabled: true,
        });
        
        // 文件系统缓存命中率阈值
        thresholds.push(PerformanceThreshold {
            metric_type: MetricType::FsCacheHitRate,
            warning_threshold: 80.0,
            critical_threshold: 60.0,
            comparator: ThresholdComparator::LessThanOrEqual,
            enabled: true,
        });
    }

    /// 启动性能监控
    pub fn start(&self) {
        self.enabled.store(1, Ordering::Relaxed);
        self.next_sample_time.store(crate::subsystems::time::get_time_ms(), Ordering::Relaxed);
        
        crate::println!("[perf_monitor] Performance monitoring started");
    }

    /// 停止性能监控
    pub fn stop(&self) {
        self.enabled.store(0, Ordering::Relaxed);
        
        crate::println!("[perf_monitor] Performance monitoring stopped");
    }

    /// 更新性能指标
    pub fn update_metric(&self, metric_type: MetricType, value: f64) {
        if self.enabled.load(Ordering::Relaxed) == 0 {
            return;
        }
        
        let current_time = crate::subsystems::time::get_time_ms();
        
        // 更新指标
        {
            let mut metrics = self.metrics.lock();
            if let Some(metric) = metrics.get_mut(&metric_type) {
                metric.current_value = value;
                metric.sample_time = current_time;
                metric.sample_count += 1;
                
                // 更新统计值
                if metric.sample_count == 1 {
                    metric.min_value = value;
                    metric.max_value = value;
                    metric.avg_value = value;
                } else {
                    metric.min_value = metric.min_value.min(value);
                    metric.max_value = metric.max_value.max(value);
                    
                    // 计算移动平均值
                    let alpha = 0.1; // 平滑因子
                    metric.avg_value = alpha * value + (1.0 - alpha) * metric.avg_value;
                }
            }
        }
        
        // 添加到历史数据
        {
            let mut history = self.history.lock();
            let values = history.entry(metric_type).or_insert_with(Vec::new);
            values.push(value);
            
            // 限制历史数据长度
            if values.len() > self.max_history_length {
                values.remove(0);
            }
        }
        
        // 检查阈值
        self.check_thresholds(metric_type, value);
        
        // 更新趋势
        self.update_trends(metric_type);
    }

    /// 检查阈值
    fn check_thresholds(&self, metric_type: MetricType, value: f64) {
        let thresholds = self.thresholds.lock();
        
        for threshold in thresholds.iter().filter(|t| t.enabled && t.metric_type == metric_type) {
            let triggered = match threshold.comparator {
                ThresholdComparator::GreaterThan => value > threshold.warning_threshold,
                ThresholdComparator::LessThan => value < threshold.warning_threshold,
                ThresholdComparator::Equal => (value - threshold.warning_threshold).abs() < f64::EPSILON,
                ThresholdComparator::GreaterThanOrEqual => value >= threshold.warning_threshold,
                ThresholdComparator::LessThanOrEqual => value <= threshold.warning_threshold,
            };
            
            if triggered {
                // 确定告警级别
                let alert_type = if match threshold.comparator {
                    ThresholdComparator::GreaterThan | ThresholdComparator::GreaterThanOrEqual => {
                        value >= threshold.critical_threshold
                    }
                    ThresholdComparator::LessThan | ThresholdComparator::LessThanOrEqual => {
                        value <= threshold.critical_threshold
                    }
                    ThresholdComparator::Equal => false,
                } {
                    AlertType::Critical
                } else {
                    AlertType::Warning
                };
                
                // 创建告警
                self.create_alert(metric_type, alert_type, value, threshold.warning_threshold);
            }
        }
    }

    /// 创建告警
    fn create_alert(&self, metric_type: MetricType, alert_type: AlertType, 
                   current_value: f64, threshold: f64) {
        let alert_id = self.next_alert_id.fetch_add(1, Ordering::SeqCst);
        let current_time = crate::subsystems::time::get_time_ms();
        
        let metric_name = match metric_type {
            MetricType::CpuUsage => "CPU Usage",
            MetricType::MemoryUsage => "Memory Usage",
            MetricType::MemoryAllocationRate => "Memory Allocation Rate",
            MetricType::MemoryFragmentation => "Memory Fragmentation",
            MetricType::IoThroughput => "I/O Throughput",
            MetricType::IoLatency => "I/O Latency",
            MetricType::IoQueueDepth => "I/O Queue Depth",
            MetricType::FsCacheHitRate => "FS Cache Hit Rate",
            MetricType::ContextSwitchRate => "Context Switch Rate",
            MetricType::SyscallRate => "Syscall Rate",
            MetricType::NetworkThroughput => "Network Throughput",
            MetricType::NetworkLatency => "Network Latency",
        };
        
        let alert_level = match alert_type {
            AlertType::Warning => "WARNING",
            AlertType::Critical => "CRITICAL",
            AlertType::Info => "INFO",
        };
        
        let message = format!("{}: {} {:.2} {} threshold {:.2}", 
                            alert_level, metric_name, current_value, 
                            if current_value >= threshold { "exceeds" } else { "below" }, 
                            threshold);
        
        let alert = PerformanceAlert {
            alert_id,
            alert_type,
            metric_type,
            current_value,
            threshold,
            alert_time: current_time,
            message,
        };
        
        // 添加到活动告警
        {
            let mut active_alerts = self.active_alerts.lock();
            active_alerts.push(alert);
            
            // 限制告警数量
            if active_alerts.len() > 100 {
                active_alerts.remove(0);
            }
        }
        
        // 输出告警
        crate::println!("[perf_alert] {}", message);
    }

    /// 更新趋势
    fn update_trends(&self, metric_type: MetricType) {
        let history = self.history.lock();
        
        if let Some(values) = history.get(&metric_type) {
            if values.len() < 10 {
                return; // 需要足够的数据点
            }
            
            // 简单线性回归计算趋势
            let n = values.len() as f64;
            let sum_x = (0..values.len()).map(|i| i as f64).sum::<f64>();
            let sum_y = values.iter().sum::<f64>();
            let sum_xy = values.iter().enumerate()
                .map(|(i, &y)| (i as f64) * y).sum::<f64>();
            let sum_x2 = (0..values.len()).map(|i| (i as f64).powi(2)).sum::<f64>();
            
            let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x.powi(2));
            
            // 计算趋势强度
            let y_mean = sum_y / n;
            let variance = values.iter().map(|y| (y - y_mean).powi(2)).sum::<f64>() / n;
            let strength = if variance > 0.0 {
                (slope.powi(2) / variance).sqrt() / (slope.abs() + 1.0)
            } else {
                0.0
            };
            
            // 确定趋势方向
            let direction = if slope.abs() < 0.01 {
                TrendDirection::Stable
            } else if slope > 0.0 {
                TrendDirection::Increasing
            } else {
                TrendDirection::Decreasing
            };
            
            // 预测未来值
            let future_steps = 10;
            let predicted_value = if values.len() > 0 {
                values[values.len() - 1] + slope * future_steps as f64
            } else {
                0.0
            };
            
            let predicted_time = crate::subsystems::time::get_time_ms() + (future_steps * self.sample_interval as usize) as u64;
            
            let trend = PerformanceTrend {
                metric_type,
                direction,
                strength,
                predicted_value,
                predicted_time,
            };
            
            // 更新趋势
            {
                let mut trends = self.trends.lock();
                
                // 移除旧的趋势
                trends.retain(|t| t.metric_type != metric_type);
                
                // 添加新趋势
                trends.push(trend);
            }
        }
    }

    /// 采样性能指标
    pub fn sample_metrics(&self) {
        if self.enabled.load(Ordering::Relaxed) == 0 {
            return;
        }
        
        let current_time = crate::subsystems::time::get_time_ms();
        let next_sample_time = self.next_sample_time.load(Ordering::Relaxed);
        
        if current_time < next_sample_time {
            return;
        }
        
        // 设置下一次采样时间
        self.next_sample_time.store(current_time + self.sample_interval, Ordering::Relaxed);
        
        // 采样CPU使用率
        self.sample_cpu_usage();
        
        // 采样内存使用率
        self.sample_memory_usage();
        
        // 采样I/O性能
        self.sample_io_performance();
        
        // 采样文件系统性能
        self.sample_filesystem_performance();
    }

    /// 采样CPU使用率
    fn sample_cpu_usage(&self) {
        // 简化实现：模拟CPU使用率
        let cpu_usage = 50.0 + (current_time / 1000) % 50 as f64;
        self.update_metric(MetricType::CpuUsage, cpu_usage);
    }

    /// 采样内存使用率
    fn sample_memory_usage(&self) {
        // 获取内存统计
        if let Some(memory_stats) = crate::subsystems::mm::optimized_memory_manager::get_optimized_memory_stats() {
            // 计算内存压力
            let memory_usage = memory_stats.memory_pressure as f64;
            self.update_metric(MetricType::MemoryUsage, memory_usage);
            
            // 计算内存碎片率
            let fragmentation = if memory_stats.total_allocations > 0 {
                (memory_stats.defrag_count as f64) / (memory_stats.total_allocations as f64) * 100.0
            } else {
                0.0
            };
            self.update_metric(MetricType::MemoryFragmentation, fragmentation);
        }
    }

    /// 采样I/O性能
    fn sample_io_performance(&self) {
        // 获取I/O统计
        if let Some(io_stats) = crate::io::optimized_io_manager::get_optimized_io_stats() {
            // 更新I/O延迟
            let io_latency = io_stats.avg_io_latency as f64;
            self.update_metric(MetricType::IoLatency, io_latency);
            
            // 更新I/O队列深度
            let queue_depth = io_stats.queue_depth as f64;
            self.update_metric(MetricType::IoQueueDepth, queue_depth);
        }
    }

    /// 采样文件系统性能
    fn sample_filesystem_performance(&self) {
        // 获取文件系统统计
        if let Some(fs_stats) = crate::vfs::optimized_filesystem::get_optimized_fs_stats() {
            // 更新文件系统缓存命中率
            let cache_hit_rate = fs_stats.cache_hit_rate * 100.0;
            self.update_metric(MetricType::FsCacheHitRate, cache_hit_rate);
        }
    }

    /// 获取性能报告
    pub fn get_performance_report(&self) -> PerformanceReport {
        let metrics = self.metrics.lock();
        let alerts = self.active_alerts.lock();
        let trends = self.trends.lock();
        
        PerformanceReport {
            timestamp: crate::subsystems::time::get_time_ms(),
            metrics: metrics.values().cloned().collect(),
            alerts: alerts.clone(),
            trends: trends.clone(),
        }
    }

    /// 清理旧数据
    pub fn cleanup(&self) {
        // 清理旧的告警
        {
            let mut alerts = self.active_alerts.lock();
            let current_time = crate::subsystems::time::get_time_ms();
            
            // 移除超过1小时的告警
            alerts.retain(|alert| current_time - alert.alert_time < 3_600_000);
        }
        
        // 清理旧的趋势
        {
            let mut trends = self.trends.lock();
            let current_time = crate::subsystems::time::get_time_ms();
            
            // 移除过期的趋势
            trends.retain(|trend| trend.predicted_time > current_time);
        }
    }
}

/// 性能报告
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    /// 报告时间戳
    pub timestamp: u64,
    /// 性能指标
    pub metrics: Vec<PerformanceMetric>,
    /// 活动告警
    pub alerts: Vec<PerformanceAlert>,
    /// 性能趋势
    pub trends: Vec<PerformanceTrend>,
}

/// 全局性能监控器
static GLOBAL_PERF_MONITOR: SpinLock<Option<PerformanceMonitor>> = SpinLock::new(None);

/// 初始化全局性能监控器
pub fn init_global_perf_monitor() {
    let mut global_monitor = GLOBAL_PERF_MONITOR.lock();
    if global_monitor.is_none() {
        let monitor = PerformanceMonitor::new();
        monitor.init();
        monitor.start();
        *global_monitor = Some(monitor);
        crate::println!("[perf_monitor] Global performance monitor initialized");
    }
}

/// 获取全局性能监控器
pub fn get_global_perf_monitor() -> Option<PerformanceMonitor> {
    GLOBAL_PERF_MONITOR.lock().clone()
}

/// 更新性能指标（全局接口）
pub fn update_perf_metric(metric_type: MetricType, value: f64) {
    if let Some(ref monitor) = get_global_perf_monitor() {
        monitor.update_metric(metric_type, value);
    }
}

/// 获取性能报告（全局接口）
pub fn get_performance_report() -> Option<PerformanceReport> {
    get_global_perf_monitor().map(|monitor| monitor.get_performance_report())
}

/// 采样性能指标（全局接口）
pub fn sample_perf_metrics() {
    if let Some(ref monitor) = get_global_perf_monitor() {
        monitor.sample_metrics();
    }
}

/// 清理性能数据（全局接口）
pub fn cleanup_perf_data() {
    if let Some(ref monitor) = get_global_perf_monitor() {
        monitor.cleanup();
    }
}