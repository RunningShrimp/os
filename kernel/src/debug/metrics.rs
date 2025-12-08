// 指标收集模块

extern crate alloc;
//
// 提供全面的指标收集功能，包括系统指标、应用指标、
// 自定义指标和指标聚合分析。
//
// 主要功能：
// - 系统指标收集
// - 应用指标收集
// - 自定义指标注册
// - 指标聚合和统计
// - 指标查询和导出
// - 性能监控集成
// - 实时指标分析

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec;
use alloc::vec::Vec;
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

/// 指标类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MetricType {
    /// 计数器（只能递增）
    Counter,
    /// 计量器（可增可减）
    Gauge,
    /// 直方图（统计分布）
    Histogram,
    /// 摘要（统计分位数）
    Summary,
}

/// 指标值
#[derive(Debug, Clone)]
pub enum MetricValue {
    /// 计数器值
    Counter(u64),
    /// 计量器值
    Gauge(f64),
    /// 直方图值
    Histogram(HistogramData),
    /// 摘要值
    Summary(SummaryData),
}

/// 直方图数据
#[derive(Debug, Clone)]
pub struct HistogramData {
    /// 样本总数
    pub sample_count: u64,
    /// 样本总和
    pub sample_sum: f64,
    /// 桶配置
    pub buckets: Vec<f64>,
    /// 桶计数
    pub bucket_counts: Vec<u64>,
}

/// 摘要数据
#[derive(Debug, Clone)]
pub struct SummaryData {
    /// 样本总数
    pub sample_count: u64,
    /// 样本总和
    pub sample_sum: f64,
    /// 分位数
    pub quantiles: Vec<(f64, f64)>,
}

/// 指标定义
#[derive(Debug, Clone)]
pub struct MetricDefinition {
    /// 指标名称
    pub name: String,
    /// 指标类型
    pub metric_type: MetricType,
    /// 指标描述
    pub description: String,
    /// 单位
    pub unit: Option<String>,
    /// 标签键
    pub label_keys: Vec<String>,
    /// 创建时间
    pub created_at: u64,
}

/// 指标实例
#[derive(Debug, Clone)]
pub struct MetricInstance {
    /// 指标定义
    pub definition: MetricDefinition,
    /// 标签值
    pub label_values: Vec<String>,
    /// 指标值
    pub value: MetricValue,
    /// 最后更新时间
    pub last_updated: u64,
}

/// 系统指标收集器接口
pub trait SystemMetricsCollector: Send + Sync {
    /// 收集系统指标
    fn collect_metrics(&self) -> Result<Vec<MetricInstance>, MetricsError>;
    /// 获取收集器名称
    fn name(&self) -> &str;
    /// 获取收集间隔
    fn interval(&self) -> Duration;
}

/// CPU指标收集器
pub struct CpuMetricsCollector {
    /// 上次收集时间
    last_collection: Arc<Mutex<u64>>,
    /// 上次CPU时间
    last_cpu_time: Arc<Mutex<CpuTime>>,
}

/// 内存指标收集器
pub struct MemoryMetricsCollector {
    /// 上次收集时间
    last_collection: Arc<Mutex<u64>>,
}

/// 网络指标收集器
pub struct NetworkMetricsCollector {
    /// 上次收集时间
    last_collection: Arc<Mutex<u64>>,
    /// 上次网络统计
    last_network_stats: Arc<Mutex<NetworkStats>>,
}

/// CPU时间统计
#[derive(Debug, Clone, Copy)]
pub struct CpuTime {
    /// 用户时间
    pub user_time: u64,
    /// 系统时间
    pub system_time: u64,
    /// 空闲时间
    pub idle_time: u64,
    /// 总时间
    pub total_time: u64,
}

/// 网络统计
#[derive(Debug, Clone, Copy)]
pub struct NetworkStats {
    /// 接收字节数
    pub rx_bytes: u64,
    /// 发送字节数
    pub tx_bytes: u64,
    /// 接收包数
    pub rx_packets: u64,
    /// 发送包数
    pub tx_packets: u64,
}

/// 指标注册表
pub struct MetricRegistry {
    /// 指标定义存储
    definitions: Arc<Mutex<BTreeMap<String, MetricDefinition>>>,
    /// 指标实例存储
    instances: Arc<Mutex<BTreeMap<String, Vec<MetricInstance>>>>,
    /// 系统收集器
    system_collectors: Arc<Mutex<Vec<Box<dyn SystemMetricsCollector>>>>,
    /// 自定义收集器
    custom_collectors: Arc<Mutex<BTreeMap<String, Box<dyn MetricsCollector>>>>,
    /// 统计信息
    statistics: MetricsStatistics,
}

/// 指标收集器接口
pub trait MetricsCollector: Send + Sync {
    /// 收集指标
    fn collect(&self) -> Result<Vec<MetricInstance>, MetricsError>;
}

/// 指标聚合器
pub struct MetricsAggregator {
    /// 聚合规则
    aggregation_rules: Arc<Mutex<Vec<AggregationRule>>>,
    /// 聚合结果
    aggregated_metrics: Arc<Mutex<BTreeMap<String, MetricInstance>>>,
}

/// 聚合规则
#[derive(Debug, Clone)]
pub struct AggregationRule {
    /// 规则ID
    pub id: String,
    /// 源指标名称模式
    pub source_metric_pattern: String,
    /// 聚合函数
    pub aggregation_function: AggregationFunction,
    /// 目标指标名称
    pub target_metric_name: String,
    /// 标签匹配规则
    pub label_matchers: Vec<LabelMatcher>,
    /// 聚合间隔
    pub interval: Duration,
}

/// 聚合函数
#[derive(Debug, Clone, Copy)]
pub enum AggregationFunction {
    /// 求和
    Sum,
    /// 平均值
    Average,
    /// 最大值
    Max,
    /// 最小值
    Min,
    /// 计数
    Count,
    /// 最新值
    Latest,
}

/// 标签匹配器
#[derive(Debug, Clone)]
pub struct LabelMatcher {
    /// 标签名
    pub name: String,
    /// 匹配模式
    pub pattern: String,
}

/// 指标导出器接口
pub trait MetricsExporter {
    /// 导出指标
    fn export(&self, metrics: &[MetricInstance]) -> Result<(), MetricsError>;
    /// 获取导出器名称
    fn name(&self) -> &str;
}

/// Prometheus格式导出器
pub struct PrometheusExporter {
    /// 输出格式
    format: PrometheusFormat,
}

/// Prometheus格式
#[derive(Debug, Clone, Copy)]
pub enum PrometheusFormat {
    /// 文本格式
    Text,
    /// 协议缓冲格式
    Protobuf,
}

/// 指标统计信息
#[derive(Debug, Default)]
pub struct MetricsStatistics {
    /// 注册的指标数量
    pub registered_metrics: AtomicUsize,
    /// 收集的样本总数
    pub total_samples_collected: AtomicU64,
    /// 收集错误数
    pub collection_errors: AtomicU64,
    /// 聚合的指标数量
    pub aggregated_metrics: AtomicUsize,
    /// 导出的指标数量
    pub exported_metrics: AtomicU64,
    /// 收集耗时（纳秒）
    pub collection_time_total: AtomicU64,
}

impl MetricRegistry {
    /// 创建新的指标注册表
    pub fn new() -> Self {
        Self {
            definitions: Arc::new(Mutex::new(BTreeMap::new())),
            instances: Arc::new(Mutex::new(BTreeMap::new())),
            system_collectors: Arc::new(Mutex::new(Vec::new())),
            custom_collectors: Arc::new(Mutex::new(BTreeMap::new())),
            statistics: MetricsStatistics::default(),
        }
    }

    /// 注册指标
    pub fn register_metric(&self, definition: MetricDefinition) -> Result<(), MetricsError> {
        let mut definitions = self.definitions.lock();

        if definitions.contains_key(&definition.name) {
            return Err(MetricsError::MetricAlreadyExists(definition.name));
        }

        definitions.insert(definition.name.clone(), definition.clone());
        self.statistics.registered_metrics.store(definitions.len(), Ordering::SeqCst);

        // 初始化实例存储
        let mut instances = self.instances.lock();
        let name = definition.name.clone();
        instances.insert(name.clone(), Vec::new());

        crate::println!("[metrics] 注册指标: {}", name);

        Ok(())
    }

    /// 记录指标值
    pub fn record_metric(&self, metric_name: &str, label_values: Vec<String>, value: MetricValue) -> Result<(), MetricsError> {
        let definitions = self.definitions.lock();
        let definition = definitions.get(metric_name)
            .ok_or(MetricsError::MetricNotFound(metric_name.to_string()))?;

        if label_values.len() != definition.label_keys.len() {
            return Err(MetricsError::LabelMismatch(
                definition.label_keys.len(),
                label_values.len()
            ));
        }

        // 创建或更新指标实例
        let instance = MetricInstance {
            definition: definition.clone(),
            label_values: label_values.clone(),
            value,
            last_updated: time::timestamp_millis(),
        };

        let mut instances = self.instances.lock();
        let metric_instances = instances.get_mut(metric_name).unwrap();

        // 查找是否已存在相同标签的实例
        if let Some(existing) = metric_instances.iter_mut().find(|i| i.label_values == label_values) {
            existing.value = instance.value;
            existing.last_updated = instance.last_updated;
        } else {
            metric_instances.push(instance);
        }

        Ok(())
    }

    /// 递增计数器
    pub fn increment_counter(&self, metric_name: &str, label_values: Vec<String>, value: u64) -> Result<(), MetricsError> {
        let definitions = self.definitions.lock();
        let definition = definitions.get(metric_name)
            .ok_or(MetricsError::MetricNotFound(metric_name.to_string()))?;

        if definition.metric_type != MetricType::Counter {
            return Err(MetricsError::InvalidMetricType("Counter".to_string()));
        }

        let mut instances = self.instances.lock();
        let metric_instances = instances.get_mut(metric_name).unwrap();

        // 查找或创建实例
        if let Some(existing) = metric_instances.iter_mut().find(|i| i.label_values == label_values) {
            if let MetricValue::Counter(current) = &mut existing.value {
                *current += value;
                existing.last_updated = time::timestamp_millis();
            }
        } else {
            metric_instances.push(MetricInstance {
                definition: definition.clone(),
                label_values,
                value: MetricValue::Counter(value),
                last_updated: time::timestamp_millis(),
            });
        }

        Ok(())
    }

    /// 设置计量器值
    pub fn set_gauge(&self, metric_name: &str, label_values: Vec<String>, value: f64) -> Result<(), MetricsError> {
        let definitions = self.definitions.lock();
        let definition = definitions.get(metric_name)
            .ok_or(MetricsError::MetricNotFound(metric_name.to_string()))?;

        if definition.metric_type != MetricType::Gauge {
            return Err(MetricsError::InvalidMetricType("Gauge".to_string()));
        }

        let mut instances = self.instances.lock();
        let metric_instances = instances.get_mut(metric_name).unwrap();

        // 查找或创建实例
        if let Some(existing) = metric_instances.iter_mut().find(|i| i.label_values == label_values) {
            if let MetricValue::Gauge(_) = &mut existing.value {
                existing.value = MetricValue::Gauge(value);
                existing.last_updated = time::timestamp_millis();
            }
        } else {
            metric_instances.push(MetricInstance {
                definition: definition.clone(),
                label_values,
                value: MetricValue::Gauge(value),
                last_updated: time::timestamp_millis(),
            });
        }

        Ok(())
    }

    /// 观察直方图值
    pub fn observe_histogram(&self, metric_name: &str, label_values: Vec<String>, value: f64) -> Result<(), MetricsError> {
        let definitions = self.definitions.lock();
        let definition = definitions.get(metric_name)
            .ok_or(MetricsError::MetricNotFound(metric_name.to_string()))?;

        if definition.metric_type != MetricType::Histogram {
            return Err(MetricsError::InvalidMetricType("Histogram".to_string()));
        }

        let mut instances = self.instances.lock();
        let metric_instances = instances.get_mut(metric_name).unwrap();

        // 查找或创建实例
        if let Some(existing) = metric_instances.iter_mut().find(|i| i.label_values == label_values) {
            if let MetricValue::Histogram(hist) = &mut existing.value {
                hist.sample_count += 1;
                hist.sample_sum += value;

                // 更新桶计数
                for (i, bucket) in hist.buckets.iter().enumerate() {
                    if value <= *bucket {
                        hist.bucket_counts[i] += 1;
                    }
                }
                existing.last_updated = time::timestamp_millis();
            }
        } else {
            // 创建新的直方图
            let default_buckets = vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0];
            let bucket_counts = vec![0; default_buckets.len()];

            metric_instances.push(MetricInstance {
                definition: definition.clone(),
                label_values,
                value: MetricValue::Histogram(HistogramData {
                    sample_count: 1,
                    sample_sum: value,
                    buckets: default_buckets,
                    bucket_counts,
                }),
                last_updated: time::timestamp_millis(),
            });
        }

        Ok(())
    }

    /// 获取指标
    pub fn get_metrics(&self, metric_name: &str) -> Result<Vec<MetricInstance>, MetricsError> {
        let instances = self.instances.lock();
        Ok(instances.get(metric_name).cloned().unwrap_or_default())
    }

    /// 获取所有指标
    pub fn get_all_metrics(&self) -> Result<Vec<MetricInstance>, MetricsError> {
        let instances = self.instances.lock();
        let mut all_metrics = Vec::new();

        for metric_instances in instances.values() {
            all_metrics.extend(metric_instances.clone());
        }

        Ok(all_metrics)
    }

    /// 添加系统收集器
    pub fn add_system_collector(&self, collector: Box<dyn SystemMetricsCollector>) -> Result<(), MetricsError> {
        let mut collectors = self.system_collectors.lock();
        collectors.push(collector);
        crate::println!("[metrics] 添加系统收集器: {}", collectors.last().unwrap().name());
        Ok(())
    }

    /// 添加自定义收集器
    pub fn add_custom_collector(&self, name: String, collector: Box<dyn MetricsCollector>) -> Result<(), MetricsError> {
        let mut collectors = self.custom_collectors.lock();
        collectors.insert(name.clone(), collector);
        crate::println!("[metrics] 添加自定义收集器: {}", name);
        Ok(())
    }

    /// 收集所有系统指标
    pub fn collect_system_metrics(&self) -> Result<Vec<MetricInstance>, MetricsError> {
        let mut all_metrics = Vec::new();
        let collectors = self.system_collectors.lock();

        for collector in collectors.iter() {
            let start_time = time::timestamp_nanos();

            match collector.collect_metrics() {
                Ok(mut metrics) => {
                    all_metrics.append(&mut metrics);
                    self.statistics.total_samples_collected.fetch_add(
                        metrics.len() as u64,
                        Ordering::SeqCst
                    );
                }
                Err(e) => {
                    self.statistics.collection_errors.fetch_add(1, Ordering::SeqCst);
                    crate::println!("[metrics] 系统指标收集失败 {}: {:?}", collector.name(), e);
                }
            }

            let collection_time = time::timestamp_nanos() - start_time;
            self.statistics.collection_time_total.fetch_add(collection_time, Ordering::SeqCst);
        }

        Ok(all_metrics)
    }

    /// 收集所有自定义指标
    pub fn collect_custom_metrics(&self) -> Result<Vec<MetricInstance>, MetricsError> {
        let mut all_metrics = Vec::new();
        let collectors = self.custom_collectors.lock();

        for (name, collector) in collectors.iter() {
            match collector.collect() {
                Ok(mut metrics) => {
                    all_metrics.append(&mut metrics);
                    self.statistics.total_samples_collected.fetch_add(
                        metrics.len() as u64,
                        Ordering::SeqCst
                    );
                }
                Err(e) => {
                    self.statistics.collection_errors.fetch_add(1, Ordering::SeqCst);
                    crate::println!("[metrics] 自定义指标收集失败 {}: {:?}", name, e);
                }
            }
        }

        Ok(all_metrics)
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> MetricsStatistics {
        MetricsStatistics {
            registered_metrics: AtomicUsize::new(
                self.statistics.registered_metrics.load(Ordering::SeqCst)
            ),
            total_samples_collected: AtomicU64::new(
                self.statistics.total_samples_collected.load(Ordering::SeqCst)
            ),
            collection_errors: AtomicU64::new(
                self.statistics.collection_errors.load(Ordering::SeqCst)
            ),
            aggregated_metrics: AtomicUsize::new(
                self.statistics.aggregated_metrics.load(Ordering::SeqCst)
            ),
            exported_metrics: AtomicU64::new(
                self.statistics.exported_metrics.load(Ordering::SeqCst)
            ),
            collection_time_total: AtomicU64::new(
                self.statistics.collection_time_total.load(Ordering::SeqCst)
            ),
        }
    }
}

impl CpuMetricsCollector {
    /// 创建新的CPU指标收集器
    pub fn new() -> Self {
        Self {
            last_collection: Arc::new(Mutex::new(0)),
            last_cpu_time: Arc::new(Mutex::new(CpuTime {
                user_time: 0,
                system_time: 0,
                idle_time: 0,
                total_time: 0,
            })),
        }
    }
}

impl SystemMetricsCollector for CpuMetricsCollector {
    fn collect_metrics(&self) -> Result<Vec<MetricInstance>, MetricsError> {
        let current_time = time::timestamp_millis();
        let mut last_collection = self.last_collection.lock();
        let mut last_cpu_time = self.last_cpu_time.lock();

        // 简单实现，实际应该从系统获取真实数据
        let current_cpu_time = CpuTime {
            user_time: 1000000,
            system_time: 500000,
            idle_time: 1500000,
            total_time: 3000000,
        };

        let mut metrics = Vec::new();

        // CPU使用率指标
        if *last_collection > 0 {
            let time_diff = current_time - *last_collection;
            let user_diff = current_cpu_time.user_time - last_cpu_time.user_time;
            let system_diff = current_cpu_time.system_time - last_cpu_time.system_time;
            let idle_diff = current_cpu_time.idle_time - last_cpu_time.idle_time;
            let total_diff = current_cpu_time.total_time - last_cpu_time.total_time;

            if total_diff > 0 {
                let user_usage = (user_diff as f64 / total_diff as f64) * 100.0;
                let system_usage = (system_diff as f64 / total_diff as f64) * 100.0;
                let idle_usage = (idle_diff as f64 / total_diff as f64) * 100.0;

                // 用户CPU使用率
                metrics.push(MetricInstance {
                    definition: MetricDefinition {
                        name: "system_cpu_user_seconds_total".to_string(),
                        metric_type: MetricType::Counter,
                        description: "用户CPU使用总时间（秒）".to_string(),
                        unit: Some("seconds".to_string()),
                        label_keys: vec!["cpu".to_string()],
                        created_at: current_time,
                    },
                    label_values: vec!["0".to_string()],
                    value: MetricValue::Counter(current_cpu_time.user_time / 1000000),
                    last_updated: current_time,
                });

                // 系统CPU使用率
                metrics.push(MetricInstance {
                    definition: MetricDefinition {
                        name: "system_cpu_system_seconds_total".to_string(),
                        metric_type: MetricType::Counter,
                        description: "系统CPU使用总时间（秒）".to_string(),
                        unit: Some("seconds".to_string()),
                        label_keys: vec!["cpu".to_string()],
                        created_at: current_time,
                    },
                    label_values: vec!["0".to_string()],
                    value: MetricValue::Counter(current_cpu_time.system_time / 1000000),
                    last_updated: current_time,
                });

                // CPU使用率计量器
                metrics.push(MetricInstance {
                    definition: MetricDefinition {
                        name: "system_cpu_usage_percentage".to_string(),
                        metric_type: MetricType::Gauge,
                        description: "CPU使用率（百分比）".to_string(),
                        unit: Some("percent".to_string()),
                        label_keys: vec!["mode".to_string()],
                        created_at: current_time,
                    },
                    label_values: vec!["user".to_string()],
                    value: MetricValue::Gauge(user_usage),
                    last_updated: current_time,
                });
            }
        }

        *last_collection = current_time;
        *last_cpu_time = current_cpu_time;

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "cpu"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(5)
    }
}

impl MemoryMetricsCollector {
    /// 创建新的内存指标收集器
    pub fn new() -> Self {
        Self {
            last_collection: Arc::new(Mutex::new(0)),
        }
    }
}

impl SystemMetricsCollector for MemoryMetricsCollector {
    fn collect_metrics(&self) -> Result<Vec<MetricInstance>, MetricsError> {
        let current_time = time::timestamp_millis();
        let mut last_collection = self.last_collection.lock();

        // 简单实现，实际应该从系统获取真实数据
        let total_memory = 1024 * 1024 * 1024; // 1GB
        let free_memory = 512 * 1024 * 1024;   // 512MB
        let used_memory = total_memory - free_memory;
        let available_memory = free_memory;

        let mut metrics = Vec::new();

        // 内存总量
        metrics.push(MetricInstance {
            definition: MetricDefinition {
                name: "system_memory_total_bytes".to_string(),
                metric_type: MetricType::Gauge,
                description: "系统总内存（字节）".to_string(),
                unit: Some("bytes".to_string()),
                label_keys: vec![],
                created_at: current_time,
            },
            label_values: vec![],
            value: MetricValue::Gauge(total_memory as f64),
            last_updated: current_time,
        });

        // 已使用内存
        metrics.push(MetricInstance {
            definition: MetricDefinition {
                name: "system_memory_used_bytes".to_string(),
                metric_type: MetricType::Gauge,
                description: "已使用内存（字节）".to_string(),
                unit: Some("bytes".to_string()),
                label_keys: vec![],
                created_at: current_time,
            },
            label_values: vec![],
            value: MetricValue::Gauge(used_memory as f64),
            last_updated: current_time,
        });

        // 可用内存
        metrics.push(MetricInstance {
            definition: MetricDefinition {
                name: "system_memory_available_bytes".to_string(),
                metric_type: MetricType::Gauge,
                description: "可用内存（字节）".to_string(),
                unit: Some("bytes".to_string()),
                label_keys: vec![],
                created_at: current_time,
            },
            label_values: vec![],
            value: MetricValue::Gauge(available_memory as f64),
            last_updated: current_time,
        });

        *last_collection = current_time;

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "memory"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(10)
    }
}

impl NetworkMetricsCollector {
    /// 创建新的网络指标收集器
    pub fn new() -> Self {
        Self {
            last_collection: Arc::new(Mutex::new(0)),
            last_network_stats: Arc::new(Mutex::new(NetworkStats {
                rx_bytes: 0,
                tx_bytes: 0,
                rx_packets: 0,
                tx_packets: 0,
            })),
        }
    }
}

impl SystemMetricsCollector for NetworkMetricsCollector {
    fn collect_metrics(&self) -> Result<Vec<MetricInstance>, MetricsError> {
        let current_time = time::timestamp_millis();
        let mut last_collection = self.last_collection.lock();
        let mut last_stats = self.last_network_stats.lock();

        // 简单实现，实际应该从系统获取真实数据
        let current_stats = NetworkStats {
            rx_bytes: 1024 * 1024,    // 1MB
            tx_bytes: 512 * 1024,     // 512KB
            rx_packets: 1000,
            tx_packets: 500,
        };

        let mut metrics = Vec::new();

        // 计算速率
        if *last_collection > 0 {
            let time_diff = current_time - *last_collection;
            if time_diff > 0 {
                let rx_rate = (current_stats.rx_bytes - last_stats.rx_bytes) as f64 / (time_diff as f64 / 1000.0);
                let tx_rate = (current_stats.tx_bytes - last_stats.tx_bytes) as f64 / (time_diff as f64 / 1000.0);

                // 接收速率
                metrics.push(MetricInstance {
                    definition: MetricDefinition {
                        name: "system_network_receive_bytes_per_second".to_string(),
                        metric_type: MetricType::Gauge,
                        description: "网络接收速率（字节/秒）".to_string(),
                        unit: Some("bytes/sec".to_string()),
                        label_keys: vec!["interface".to_string()],
                        created_at: current_time,
                    },
                    label_values: vec!["eth0".to_string()],
                    value: MetricValue::Gauge(rx_rate),
                    last_updated: current_time,
                });

                // 发送速率
                metrics.push(MetricInstance {
                    definition: MetricDefinition {
                        name: "system_network_transmit_bytes_per_second".to_string(),
                        metric_type: MetricType::Gauge,
                        description: "网络发送速率（字节/秒）".to_string(),
                        unit: Some("bytes/sec".to_string()),
                        label_keys: vec!["interface".to_string()],
                        created_at: current_time,
                    },
                    label_values: vec!["eth0".to_string()],
                    value: MetricValue::Gauge(tx_rate),
                    last_updated: current_time,
                });
            }
        }

        // 累计计数器
        metrics.push(MetricInstance {
            definition: MetricDefinition {
                name: "system_network_receive_bytes_total".to_string(),
                metric_type: MetricType::Counter,
                description: "网络接收总字节数".to_string(),
                unit: Some("bytes".to_string()),
                label_keys: vec!["interface".to_string()],
                created_at: current_time,
            },
            label_values: vec!["eth0".to_string()],
            value: MetricValue::Counter(current_stats.rx_bytes),
            last_updated: current_time,
        });

        metrics.push(MetricInstance {
            definition: MetricDefinition {
                name: "system_network_transmit_bytes_total".to_string(),
                metric_type: MetricType::Counter,
                description: "网络发送总字节数".to_string(),
                unit: Some("bytes".to_string()),
                label_keys: vec!["interface".to_string()],
                created_at: current_time,
            },
            label_values: vec!["eth0".to_string()],
            value: MetricValue::Counter(current_stats.tx_bytes),
            last_updated: current_time,
        });

        *last_collection = current_time;
        *last_stats = current_stats;

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "network"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(5)
    }
}

impl PrometheusExporter {
    /// 创建新的Prometheus导出器
    pub fn new(format: PrometheusFormat) -> Self {
        Self { format }
    }
}

impl MetricsExporter for PrometheusExporter {
    fn export(&self, metrics: &[MetricInstance]) -> Result<(), MetricsError> {
        match self.format {
            PrometheusFormat::Text => {
                // 简化实现，实际应该格式化为Prometheus文本格式
                crate::println!("# Prometheus metrics export");
                for metric in metrics {
                    let metric_name = &metric.definition.name;
                    match &metric.value {
                        MetricValue::Counter(value) => {
                            crate::println!("{} {}", metric_name, value);
                        }
                        MetricValue::Gauge(value) => {
                            crate::println!("{} {}", metric_name, value);
                        }
                        MetricValue::Histogram(hist) => {
                            crate::println!("{}_count {}", metric_name, hist.sample_count);
                            crate::println!("{}_sum {}", metric_name, hist.sample_sum);
                        }
                        MetricValue::Summary(summary) => {
                            crate::println!("{}_count {}", metric_name, summary.sample_count);
                            crate::println!("{}_sum {}", metric_name, summary.sample_sum);
                        }
                    }
                }
            }
            PrometheusFormat::Protobuf => {
                // 简化实现，实际应该使用Protocol Buffers格式
                crate::println!("Protobuf format export not implemented");
            }
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "prometheus"
    }
}

/// 指标错误类型
#[derive(Debug, Clone)]
pub enum MetricsError {
    /// 指标已存在
    MetricAlreadyExists(String),
    /// 指标不存在
    MetricNotFound(String),
    /// 标签不匹配
    LabelMismatch(usize, usize),
    /// 无效的指标类型
    InvalidMetricType(String),
    /// 收集错误
    CollectionError(String),
    /// 导出错误
    ExportError(String),
}

impl core::fmt::Display for MetricsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MetricsError::MetricAlreadyExists(name) => write!(f, "指标已存在: {}", name),
            MetricsError::MetricNotFound(name) => write!(f, "指标不存在: {}", name),
            MetricsError::LabelMismatch(expected, actual) => {
                write!(f, "标签数量不匹配，期望: {}，实际: {}", expected, actual)
            }
            MetricsError::InvalidMetricType(t) => write!(f, "无效的指标类型: {}", t),
            MetricsError::CollectionError(msg) => write!(f, "收集错误: {}", msg),
            MetricsError::ExportError(msg) => write!(f, "导出错误: {}", msg),
        }
    }
}

/// 全局指标注册表实例
static METRIC_REGISTRY: spin::Mutex<Option<Arc<MetricRegistry>>> = spin::Mutex::new(None);

/// 初始化指标子系统
pub fn init() -> Result<(), MetricsError> {
    let registry = MetricRegistry::new();

    // 添加默认系统收集器
    registry.add_system_collector(Box::new(CpuMetricsCollector::new()))?;
    registry.add_system_collector(Box::new(MemoryMetricsCollector::new()))?;
    registry.add_system_collector(Box::new(NetworkMetricsCollector::new()))?;

    let registry = Arc::new(registry);
    let mut global_registry = METRIC_REGISTRY.lock();
    *global_registry = Some(registry);

    crate::println!("[metrics] 指标子系统初始化完成");
    Ok(())
}

/// 获取全局指标注册表
pub fn get_metric_registry() -> Result<Arc<MetricRegistry>, MetricsError> {
    let registry = METRIC_REGISTRY.lock();
    registry.as_ref()
        .cloned()
        .ok_or(MetricsError::CollectionError("指标注册表未初始化".to_string()))
}
