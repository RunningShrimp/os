// 性能分析模块

extern crate alloc;
//
// 提供全面的性能分析功能，包括CPU性能分析、内存性能分析、
// 函数调用分析、热点分析和性能优化建议。
//
// 主要功能：
// - CPU性能分析
// - 内存使用分析
// - 函数调用分析
// - 热点检测和分析
// - 性能瓶颈识别
// - 优化建议生成
// - 性能数据可视化

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{format, vec};
use alloc::string::String;
use alloc::string::ToString;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;
use spin::Mutex;

use crate::time;

// Import println macro
#[allow(unused_imports)]
use crate::println;

/// 性能分析器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProfilerType {
    /// CPU分析器
    Cpu,
    /// 内存分析器
    Memory,
    /// 函数调用分析器
    Function,
    /// 系统调用分析器
    SystemCall,
    /// IO分析器
    Io,
    /// 网络分析器
    Network,
}

/// 性能样本
#[derive(Debug, Clone)]
pub struct PerformanceSample {
    /// 样本ID
    pub id: u64,
    /// 时间戳
    pub timestamp: u64,
    /// 采样线程ID
    pub thread_id: u64,
    /// 栈帧信息
    pub stack_frames: Vec<StackFrame>,
    /// CPU使用率
    pub cpu_usage: f64,
    /// 内存使用情况
    pub memory_info: MemoryInfo,
    /// 扩展数据
    pub extended_data: BTreeMap<String, String>,
}

/// 栈帧信息
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// 函数地址
    pub function_address: usize,
    /// 函数名称
    pub function_name: Option<String>,
    /// 模块名称
    pub module_name: Option<String>,
    /// 文件名
    pub file_name: Option<String>,
    /// 行号
    pub line_number: Option<u32>,
    /// 偏移量
    pub offset: u32,
}

/// 内存信息
#[derive(Debug, Clone, Copy)]
pub struct MemoryInfo {
    /// 已分配内存（字节）
    pub allocated: u64,
    /// 已释放内存（字节）
    pub freed: u64,
    /// 当前使用内存（字节）
    pub current_usage: u64,
    /// 峰值内存使用（字节）
    pub peak_usage: u64,
    /// 分配次数
    pub allocation_count: u64,
    /// 释放次数
    pub free_count: u64,
}

/// 函数性能统计
#[derive(Debug, Clone)]
pub struct FunctionStatistics {
    /// 函数名称
    pub function_name: String,
    /// 调用次数
    pub call_count: u64,
    /// 总执行时间（纳秒）
    pub total_time: u64,
    /// 最小执行时间（纳秒）
    pub min_time: u64,
    /// 最大执行时间（纳秒）
    pub max_time: u64,
    /// 平均执行时间（纳秒）
    pub avg_time: f64,
    /// CPU时间（纳秒）
    pub cpu_time: u64,
    /// 内存分配（字节）
    pub memory_allocated: u64,
    /// 内存释放（字节）
    pub memory_freed: u64,
    /// 最后调用时间
    pub last_call_time: u64,
    /// 调用栈信息
    pub call_stacks: Vec<Vec<String>>,
}

/// 热点函数
#[derive(Debug, Clone)]
pub struct HotspotFunction {
    /// 函数名称
    pub function_name: String,
    /// 热点分数（0-100）
    pub hotspot_score: f64,
    /// 调用次数
    pub call_count: u64,
    /// 总执行时间（纳秒）
    pub total_time: u64,
    /// 占总时间的百分比
    pub time_percentage: f64,
    /// 优化潜力
    pub optimization_potential: OptimizationPotential,
    /// 优化建议
    pub optimization_suggestions: Vec<String>,
}

/// 优化潜力
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationPotential {
    /// 低
    Low,
    /// 中等
    Medium,
    /// 高
    High,
    /// 很高
    VeryHigh,
}

/// 性能分析结果
#[derive(Debug, Clone)]
pub struct ProfilingResult {
    /// 分析ID
    pub analysis_id: String,
    /// 分析器类型
    pub profiler_type: ProfilerType,
    /// 开始时间
    pub start_time: u64,
    /// 结束时间
    pub end_time: u64,
    /// 总样本数
    pub total_samples: u64,
    /// 函数统计
    pub function_stats: BTreeMap<String, FunctionStatistics>,
    /// 热点函数
    pub hotspots: Vec<HotspotFunction>,
    /// 性能指标
    pub performance_metrics: PerformanceMetrics,
    /// 优化建议
    pub optimization_suggestions: Vec<OptimizationSuggestion>,
}

/// 性能指标
#[derive(Debug, Clone, Copy)]
pub struct PerformanceMetrics {
    /// 总执行时间（纳秒）
    pub total_execution_time: u64,
    /// CPU使用率（百分比）
    pub cpu_usage_percentage: f64,
    /// 内存峰值（字节）
    pub memory_peak: u64,
    /// 内存平均值（字节）
    pub memory_average: u64,
    /// 上下文切换次数
    pub context_switches: u64,
    /// 页面错误次数
    pub page_faults: u64,
    /// 缓存未命中次数
    pub cache_misses: u64,
    /// 分支预测错误次数
    pub branch_mispredictions: u64,
}

/// 优化建议
#[derive(Debug, Clone)]
pub struct OptimizationSuggestion {
    /// 建议ID
    pub id: String,
    /// 建议类型
    pub suggestion_type: OptimizationType,
    /// 优先级
    pub priority: OptimizationPriority,
    /// 描述
    pub description: String,
    /// 目标函数
    pub target_function: Option<String>,
    /// 预期改进
    pub expected_improvement: String,
    /// 实现复杂度
    pub implementation_complexity: ImplementationComplexity,
    /// 详细的实现步骤
    pub implementation_steps: Vec<String>,
}

/// 优化类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationType {
    /// 算法优化
    AlgorithmOptimization,
    /// 数据结构优化
    DataStructureOptimization,
    /// 缓存优化
    CacheOptimization,
    /// 并行化
    Parallelization,
    /// 内存优化
    MemoryOptimization,
    /// IO优化
    IoOptimization,
    /// 编译器优化
    CompilerOptimization,
}

/// 优化优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptimizationPriority {
    /// 低
    Low = 1,
    /// 中等
    Medium = 2,
    /// 高
    High = 3,
    /// 紧急
    Critical = 4,
}

/// 实现复杂度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImplementationComplexity {
    /// 简单
    Simple,
    /// 中等
    Medium,
    /// 复杂
    Complex,
    /// 非常复杂
    VeryComplex,
}

/// 性能分析配置
#[derive(Debug, Clone)]
pub struct ProfilingConfig {
    /// 采样间隔（纳秒）
    pub sampling_interval: Duration,
    /// 最大样本数
    pub max_samples: usize,
    /// 最大函数深度
    pub max_function_depth: usize,
    /// 是否启用内存分析
    pub enable_memory_profiling: bool,
    /// 是否启用调用栈收集
    pub enable_stack_trace: bool,
    /// 是否启用实时分析
    pub enable_realtime_analysis: bool,
    /// 热点阈值（百分比）
    pub hotspot_threshold: f64,
}

impl Default for ProfilingConfig {
    fn default() -> Self {
        Self {
            sampling_interval: Duration::from_micros(100), // 100微秒
            max_samples: 1000000,
            max_function_depth: 32,
            enable_memory_profiling: true,
            enable_stack_trace: true,
            enable_realtime_analysis: false,
            hotspot_threshold: 1.0, // 1%
        }
    }
}

/// 性能分析统计
#[derive(Debug, Default)]
pub struct ProfilingStatistics {
    /// 总分析次数
    pub total_profiling_sessions: AtomicU64,
    /// 总采样次数
    pub total_samples_collected: AtomicU64,
    /// 总函数调用次数
    pub total_function_calls: AtomicU64,
    /// 当前活动分析器数量
    pub active_profilers: AtomicUsize,
    /// 收集的数据大小（字节）
    pub data_collected_size: AtomicU64,
    /// 分析耗时（纳秒）
    pub analysis_time_total: AtomicU64,
}

/// 性能分析器
pub struct Profiler {
    /// 分析器ID
    pub id: String,
    /// 分析器类型
    pub profiler_type: ProfilerType,
    /// 配置
    config: ProfilingConfig,
    /// 性能样本存储
    samples: Arc<Mutex<Vec<PerformanceSample>>>,
    /// 函数统计
    function_stats: Arc<Mutex<BTreeMap<String, FunctionStatistics>>>,
    /// 当前调用栈
    call_stack: Arc<Mutex<Vec<StackFrame>>>,
    /// 是否正在分析
    is_profiling: core::sync::atomic::AtomicBool,
    /// 开始时间
    start_time: Arc<Mutex<Option<u64>>>,
    /// 统计信息
    statistics: ProfilingStatistics,
}

/// 性能分析引擎
pub struct PerformanceAnalysisEngine {
    /// 配置
    config: ProfilingConfig,
    /// 活动的分析器
    active_profilers: Arc<Mutex<BTreeMap<String, Arc<Profiler>>>>,
    /// 分析结果存储
    analysis_results: Arc<Mutex<BTreeMap<String, ProfilingResult>>>,
    /// 统计信息
    statistics: ProfilingStatistics,
    /// 下一次分析器ID
    next_profiler_id: AtomicU64,
}

impl Profiler {
    /// 创建新的性能分析器
    pub fn new(id: String, profiler_type: ProfilerType, config: ProfilingConfig) -> Self {
        Self {
            id,
            profiler_type,
            config,
            samples: Arc::new(Mutex::new(Vec::new())),
            function_stats: Arc::new(Mutex::new(BTreeMap::new())),
            call_stack: Arc::new(Mutex::new(Vec::new())),
            is_profiling: core::sync::atomic::AtomicBool::new(false),
            start_time: Arc::new(Mutex::new(None)),
            statistics: ProfilingStatistics::default(),
        }
    }

    /// 开始性能分析
    pub fn start(&self) -> Result<(), ProfilingError> {
        if self.is_profiling.load(Ordering::SeqCst) {
            return Ok(());
        }

        // 清空之前的数据
        self.samples.lock().clear();
        self.function_stats.lock().clear();
        self.call_stack.lock().clear();

        // 设置开始时间
        *self.start_time.lock() = Some(time::timestamp_nanos());

        // 开始分析
        self.is_profiling.store(true, Ordering::SeqCst);

        crate::println!("[profiling] {} 分析器已启动", self.profiler_type_as_string());

        Ok(())
    }

    /// 停止性能分析
    pub fn stop(&self) -> Result<(), ProfilingError> {
        if !self.is_profiling.load(Ordering::SeqCst) {
            return Ok(());
        }

        self.is_profiling.store(false, Ordering::SeqCst);

        crate::println!("[profiling] {} 分析器已停止", self.profiler_type_as_string());

        Ok(())
    }

    /// 记录函数调用开始
    pub fn function_enter(&self, function_name: String, module_name: Option<String>) -> Result<(), ProfilingError> {
        if !self.is_profiling.load(Ordering::SeqCst) {
            return Ok(());
        }

        let current_time = time::timestamp_nanos();
        let frame = StackFrame {
            function_address: function_name.as_ptr() as usize,
            function_name: Some(function_name.clone()),
            module_name,
            file_name: None,
            line_number: None,
            offset: 0,
        };

        self.call_stack.lock().push(frame);

        // 更新函数统计
        self.update_function_statistics(&function_name, current_time, true)?;

        Ok(())
    }

    /// 记录函数调用结束
    pub fn function_exit(&self, function_name: String) -> Result<(), ProfilingError> {
        if !self.is_profiling.load(Ordering::SeqCst) {
            return Ok(());
        }

        let current_time = time::timestamp_nanos();

        // 移除栈帧
        let mut call_stack = self.call_stack.lock();
        if let Some(index) = call_stack.iter().position(|f| {
            f.function_name.as_ref().map_or(false, |name| name == &function_name)
        }) {
            call_stack.remove(index);
        }

        // 更新函数统计
        self.update_function_statistics(&function_name, current_time, false)?;

        Ok(())
    }

    /// 采集性能样本
    pub fn collect_sample(&self) -> Result<(), ProfilingError> {
        if !self.is_profiling.load(Ordering::SeqCst) {
            return Ok(());
        }

        let current_time = time::timestamp_nanos();
        let thread_id = self.get_current_thread_id();

        // 收集栈帧
        let stack_frames = {
            let call_stack = self.call_stack.lock();
            call_stack.clone()
        };

        // 收集内存信息
        let memory_info = self.collect_memory_info()?;

        // 创建样本
        let sample = PerformanceSample {
            id: self.generate_sample_id(),
            timestamp: current_time,
            thread_id,
            stack_frames,
            cpu_usage: self.get_cpu_usage(),
            memory_info,
            extended_data: BTreeMap::new(),
        };

        // 存储样本
        let mut samples = self.samples.lock();
        samples.push(sample);

        // 限制样本数量
        if samples.len() > self.config.max_samples {
            samples.remove(0);
        }

        // 更新统计
        self.statistics.total_samples_collected.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    /// 获取分析结果
    pub fn get_analysis_result(&self) -> Result<ProfilingResult, ProfilingError> {
        let samples = self.samples.lock();
        let function_stats = self.function_stats.lock();
        let start_time = *self.start_time.lock();

        if start_time.is_none() {
            return Err(ProfilingError::NotStarted);
        }

        let start_time = start_time.unwrap();
        let end_time = time::timestamp_nanos();

        // 计算热点函数
        let hotspots = self.identify_hotspots(&function_stats)?;

        // 计算性能指标
        let performance_metrics = self.calculate_performance_metrics(&samples)?;

        // 生成优化建议
        let optimization_suggestions = self.generate_optimization_suggestions(&hotspots);

        Ok(ProfilingResult {
            analysis_id: format!("{}_{}", self.id, start_time),
            profiler_type: self.profiler_type,
            start_time,
            end_time,
            total_samples: samples.len() as u64,
            function_stats: function_stats.clone(),
            hotspots,
            performance_metrics,
            optimization_suggestions,
        })
    }

    /// 更新函数统计
    fn update_function_statistics(&self, function_name: &str, timestamp: u64, is_entry: bool) -> Result<(), ProfilingError> {
        let mut stats = self.function_stats.lock();

        let stat = stats.entry(function_name.to_string()).or_insert_with(|| FunctionStatistics {
            function_name: function_name.to_string(),
            call_count: 0,
            total_time: 0,
            min_time: u64::MAX,
            max_time: 0,
            avg_time: 0.0,
            cpu_time: 0,
            memory_allocated: 0,
            memory_freed: 0,
            last_call_time: 0,
            call_stacks: Vec::new(),
        });

        if is_entry {
            // 函数入口
            stat.call_count += 1;
            stat.last_call_time = timestamp;
        } else {
            // 函数出口 - 计算执行时间
            if let Some(entry_time) = self.find_function_entry_time(function_name) {
                let execution_time = timestamp - entry_time;
                stat.total_time += execution_time;
                stat.min_time = stat.min_time.min(execution_time);
                stat.max_time = stat.max_time.max(execution_time);
                stat.avg_time = stat.total_time as f64 / stat.call_count as f64;
            }
        }

        Ok(())
    }

    /// 识别热点函数
    fn identify_hotspots(&self, function_stats: &BTreeMap<String, FunctionStatistics>) -> Result<Vec<HotspotFunction>, ProfilingError> {
        let mut hotspots: Vec<HotspotFunction> = Vec::new();
        let total_time: u64 = function_stats.values().map(|s| s.total_time).sum();

        for (function_name, stat) in function_stats {
            if total_time == 0 {
                continue;
            }

            let time_percentage = (stat.total_time as f64 / total_time as f64) * 100.0;

            if time_percentage >= self.config.hotspot_threshold {
                let hotspot_score = self.calculate_hotspot_score(stat, time_percentage);
                let optimization_potential = self.assess_optimization_potential(stat);
                let optimization_suggestions = self.generate_function_optimization_suggestions(function_name, stat);

                hotspots.push(HotspotFunction {
                    function_name: function_name.clone(),
                    hotspot_score,
                    call_count: stat.call_count,
                    total_time: stat.total_time,
                    time_percentage,
                    optimization_potential,
                    optimization_suggestions,
                });
            }
        }

        // 按热点分数排序
        hotspots.sort_by(|a, b| b.hotspot_score.partial_cmp(&a.hotspot_score).unwrap());

        Ok(hotspots)
    }

    /// 计算热点分数
    fn calculate_hotspot_score(&self, stat: &FunctionStatistics, time_percentage: f64) -> f64 {
        // 综合考虑执行时间占比、调用次数和平均执行时间
        let time_score = time_percentage;
        let call_score = (stat.call_count as f64 / 1000.0).min(100.0);
        let duration_score = (stat.avg_time / 1000000.0).min(100.0); // 转换为毫秒

        (time_score + call_score + duration_score) / 3.0
    }

    /// 评估优化潜力
    fn assess_optimization_potential(&self, stat: &FunctionStatistics) -> OptimizationPotential {
        let avg_time_ms = stat.avg_time / 1000000.0;
        let call_frequency = stat.call_count as f64 / (stat.total_time as f64 / stat.avg_time);

        if avg_time_ms > 100.0 || call_frequency > 1000.0 {
            OptimizationPotential::VeryHigh
        } else if avg_time_ms > 10.0 || call_frequency > 100.0 {
            OptimizationPotential::High
        } else if avg_time_ms > 1.0 || call_frequency > 10.0 {
            OptimizationPotential::Medium
        } else {
            OptimizationPotential::Low
        }
    }

    /// 生成函数优化建议
    fn generate_function_optimization_suggestions(&self, function_name: &str, stat: &FunctionStatistics) -> Vec<String> {
        let mut suggestions = Vec::new();

        let avg_time_ms = stat.avg_time / 1000000.0;
        let call_count = stat.call_count;

        if avg_time_ms > 50.0 {
            suggestions.push("考虑算法优化，减少时间复杂度".to_string());
        }

        if call_count > 1000 {
            suggestions.push("考虑缓存结果，避免重复计算".to_string());
        }

        if stat.memory_allocated > 1024 * 1024 {
            suggestions.push("考虑内存池优化，减少动态分配".to_string());
        }

        if stat.call_stacks.len() > 10 {
            suggestions.push("考虑内联优化，减少函数调用开销".to_string());
        }

        suggestions
    }

    /// 计算性能指标
    fn calculate_performance_metrics(&self, samples: &[PerformanceSample]) -> Result<PerformanceMetrics, ProfilingError> {
        if samples.is_empty() {
            return Ok(PerformanceMetrics {
                total_execution_time: 0,
                cpu_usage_percentage: 0.0,
                memory_peak: 0,
                memory_average: 0,
                context_switches: 0,
                page_faults: 0,
                cache_misses: 0,
                branch_mispredictions: 0,
            });
        }

        let start_time = samples.first().unwrap().timestamp;
        let end_time = samples.last().unwrap().timestamp;
        let total_execution_time = end_time - start_time;

        let total_cpu_usage: f64 = samples.iter().map(|s| s.cpu_usage).sum();
        let avg_cpu_usage = total_cpu_usage / samples.len() as f64;

        let memory_usage_values: Vec<u64> = samples.iter().map(|s| s.memory_info.current_usage).collect();
        let memory_peak = memory_usage_values.iter().max().copied().unwrap_or(0);
        let memory_average = memory_usage_values.iter().sum::<u64>() / memory_usage_values.len() as u64;

        Ok(PerformanceMetrics {
            total_execution_time,
            cpu_usage_percentage: avg_cpu_usage,
            memory_peak,
            memory_average,
            context_switches: 0, // 需要从系统获取
            page_faults: 0,      // 需要从系统获取
            cache_misses: 0,     // 需要从硬件计数器获取
            branch_mispredictions: 0, // 需要从硬件计数器获取
        })
    }

    /// 生成优化建议
    fn generate_optimization_suggestions(&self, hotspots: &[HotspotFunction]) -> Vec<OptimizationSuggestion> {
        let mut suggestions = Vec::new();

        for (i, hotspot) in hotspots.iter().take(10).enumerate() {
            if hotspot.hotspot_score > 50.0 {
                suggestions.push(OptimizationSuggestion {
                    id: format!("opt_{}", i),
                    suggestion_type: OptimizationType::AlgorithmOptimization,
                    priority: if hotspot.hotspot_score > 80.0 {
                        OptimizationPriority::Critical
                    } else if hotspot.hotspot_score > 60.0 {
                        OptimizationPriority::High
                    } else {
                        OptimizationPriority::Medium
                    },
                    description: format!(
                        "优化热点函数 {}，当前占用 {}% 的执行时间",
                        hotspot.function_name, hotspot.time_percentage
                    ),
                    target_function: Some(hotspot.function_name.clone()),
                    expected_improvement: format!("预计可减少 {}% 的执行时间", hotspot.time_percentage * 0.5),
                    implementation_complexity: ImplementationComplexity::Medium,
                    implementation_steps: vec![
                        "分析函数的算法复杂度".to_string(),
                        "寻找更高效的替代算法".to_string(),
                        "实现并测试新算法".to_string(),
                    ],
                });
            }
        }

        // 添加通用的优化建议
        if !hotspots.is_empty() {
            suggestions.push(OptimizationSuggestion {
                id: "general_memory_opt".to_string(),
                suggestion_type: OptimizationType::MemoryOptimization,
                priority: OptimizationPriority::Medium,
                description: "启用内存池优化，减少内存分配开销".to_string(),
                target_function: None,
                expected_improvement: "预计可减少 10-20% 的内存分配时间".to_string(),
                implementation_complexity: ImplementationComplexity::Complex,
                implementation_steps: vec![
                    "实现内存池分配器".to_string(),
                    "替换动态内存分配调用".to_string(),
                    "测试和验证内存使用".to_string(),
                ],
            });
        }

        suggestions
    }

    /// 辅助方法
    fn profiler_type_as_string(&self) -> &'static str {
        match self.profiler_type {
            ProfilerType::Cpu => "CPU",
            ProfilerType::Memory => "内存",
            ProfilerType::Function => "函数",
            ProfilerType::SystemCall => "系统调用",
            ProfilerType::Io => "IO",
            ProfilerType::Network => "网络",
        }
    }

    fn get_current_thread_id(&self) -> u64 {
        // 简单实现，实际应该从线程管理器获取
        1
    }

    fn collect_memory_info(&self) -> Result<MemoryInfo, ProfilingError> {
        // 简单实现，实际应该从内存管理器获取
        Ok(MemoryInfo {
            allocated: 0,
            freed: 0,
            current_usage: 0,
            peak_usage: 0,
            allocation_count: 0,
            free_count: 0,
        })
    }

    fn get_cpu_usage(&self) -> f64 {
        // 简单实现，实际应该从CPU监控获取
        0.1
    }

    fn generate_sample_id(&self) -> u64 {
        static NEXT_SAMPLE_ID: AtomicU64 = AtomicU64::new(1);
        NEXT_SAMPLE_ID.fetch_add(1, Ordering::SeqCst)
    }

    fn find_function_entry_time(&self, _function_name: &str) -> Option<u64> {
        // 简单实现，实际应该维护函数调用时间映射
        Some(time::timestamp_nanos() - 1000)
    }
}

impl PerformanceAnalysisEngine {
    /// 创建新的性能分析引擎
    pub fn new(config: ProfilingConfig) -> Self {
        Self {
            config,
            active_profilers: Arc::new(Mutex::new(BTreeMap::new())),
            analysis_results: Arc::new(Mutex::new(BTreeMap::new())),
            statistics: ProfilingStatistics::default(),
            next_profiler_id: AtomicU64::new(1),
        }
    }

    /// 创建新的分析器
    pub fn create_profiler(&self, profiler_type: ProfilerType) -> Result<String, ProfilingError> {
        let id = self.generate_profiler_id();
        let profiler = Arc::new(Profiler::new(id.clone(), profiler_type, self.config.clone()));

        let mut profilers = self.active_profilers.lock();
        profilers.insert(id.clone(), profiler);

        self.statistics.active_profilers.store(profilers.len(), Ordering::SeqCst);

        crate::println!("[profiling] 创建 {} 分析器: {}", profiler_type_as_string(profiler_type), id);

        Ok(id)
    }

    /// 获取分析器
    pub fn get_profiler(&self, profiler_id: &str) -> Result<Arc<Profiler>, ProfilingError> {
        let profilers = self.active_profilers.lock();
        profilers.get(profiler_id)
            .cloned()
            .ok_or(ProfilingError::ProfilerNotFound(profiler_id.to_string()))
    }

    /// 启动分析器
    pub fn start_profiler(&self, profiler_id: &str) -> Result<(), ProfilingError> {
        let profilers = self.active_profilers.lock();
        if let Some(profiler) = profilers.get(profiler_id) {
            profiler.start()
        } else {
            Err(ProfilingError::ProfilerNotFound(profiler_id.to_string()))
        }
    }

    /// 停止分析器
    pub fn stop_profiler(&self, profiler_id: &str) -> Result<(), ProfilingError> {
        let profilers = self.active_profilers.lock();
        if let Some(profiler) = profilers.get(profiler_id) {
            profiler.stop()
        } else {
            Err(ProfilingError::ProfilerNotFound(profiler_id.to_string()))
        }
    }

    /// 移除分析器
    pub fn remove_profiler(&self, profiler_id: &str) -> Result<ProfilingResult, ProfilingError> {
        let mut profilers = self.active_profilers.lock();
        if let Some(profiler) = profilers.remove(profiler_id) {
            // 停止分析器
            profiler.stop()?;

            // 获取分析结果
            let result = profiler.get_analysis_result()?;

            // 存储分析结果
            let mut results = self.analysis_results.lock();
            results.insert(result.analysis_id.clone(), result.clone());

            // 更新统计
            self.statistics.active_profilers.store(profilers.len(), Ordering::SeqCst);
            self.statistics.total_profiling_sessions.fetch_add(1, Ordering::SeqCst);

            Ok(result)
        } else {
            Err(ProfilingError::ProfilerNotFound(profiler_id.to_string()))
        }
    }

    /// 获取分析结果
    pub fn get_analysis_result(&self, analysis_id: &str) -> Result<ProfilingResult, ProfilingError> {
        let results = self.analysis_results.lock();
        results.get(analysis_id).cloned().ok_or(ProfilingError::AnalysisNotFound(analysis_id.to_string()))
    }

    /// 获取所有分析结果
    pub fn get_all_analysis_results(&self) -> Result<Vec<ProfilingResult>, ProfilingError> {
        let results = self.analysis_results.lock();
        Ok(results.values().cloned().collect())
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> ProfilingStatistics {
        ProfilingStatistics {
            total_profiling_sessions: AtomicU64::new(
                self.statistics.total_profiling_sessions.load(Ordering::SeqCst)
            ),
            total_samples_collected: AtomicU64::new(
                self.statistics.total_samples_collected.load(Ordering::SeqCst)
            ),
            total_function_calls: AtomicU64::new(
                self.statistics.total_function_calls.load(Ordering::SeqCst)
            ),
            active_profilers: AtomicUsize::new(
                self.statistics.active_profilers.load(Ordering::SeqCst)
            ),
            data_collected_size: AtomicU64::new(
                self.statistics.data_collected_size.load(Ordering::SeqCst)
            ),
            analysis_time_total: AtomicU64::new(
                self.statistics.analysis_time_total.load(Ordering::SeqCst)
            ),
        }
    }

    /// 生成分析器ID
    fn generate_profiler_id(&self) -> String {
        let id = self.next_profiler_id.fetch_add(1, Ordering::SeqCst);
        format!("profiler_{}", id)
    }
}

/// 性能分析错误类型
#[derive(Debug, Clone)]
pub enum ProfilingError {
    /// 分析器未启动
    NotStarted,
    /// 分析器不存在
    ProfilerNotFound(String),
    /// 分析结果不存在
    AnalysisNotFound(String),
    /// 配置错误
    ConfigurationError(String),
    /// 内存不足
    OutOfMemory,
    /// 系统错误
    SystemError(String),
}

impl core::fmt::Display for ProfilingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProfilingError::NotStarted => write!(f, "性能分析器未启动"),
            ProfilingError::ProfilerNotFound(id) => write!(f, "分析器不存在: {}", id),
            ProfilingError::AnalysisNotFound(id) => write!(f, "分析结果不存在: {}", id),
            ProfilingError::ConfigurationError(msg) => write!(f, "配置错误: {}", msg),
            ProfilingError::OutOfMemory => write!(f, "内存不足"),
            ProfilingError::SystemError(msg) => write!(f, "系统错误: {}", msg),
        }
    }
}

/// 辅助函数
fn profiler_type_as_string(profiler_type: ProfilerType) -> &'static str {
    match profiler_type {
        ProfilerType::Cpu => "CPU",
        ProfilerType::Memory => "内存",
        ProfilerType::Function => "函数",
        ProfilerType::SystemCall => "系统调用",
        ProfilerType::Io => "IO",
        ProfilerType::Network => "网络",
    }
}

/// 全局性能分析引擎实例
static PERFORMANCE_ENGINE: spin::Mutex<Option<PerformanceAnalysisEngine>> = spin::Mutex::new(None);

/// 初始化性能分析子系统
pub fn init() -> Result<(), ProfilingError> {
    let config = ProfilingConfig::default();
    let engine = PerformanceAnalysisEngine::new(config);

    let mut global_engine = PERFORMANCE_ENGINE.lock();
    *global_engine = Some(engine);

    crate::println!("[profiling] 性能分析子系统初始化完成");
    Ok(())
}

/// 获取全局性能分析引擎
pub fn get_performance_engine() -> spin::MutexGuard<'static, Option<PerformanceAnalysisEngine>> {
    PERFORMANCE_ENGINE.lock()
}
