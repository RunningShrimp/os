// Memory and Performance Analyzer Module
//
// 内存和性能分析器模块
// 提供内存分析、性能分析、热点分析等功能

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use crate::debug::breakpoint::SourceLocation;
use crate::debug::session::{ProcessInfo, DebugLevel};

/// 内存分析器
#[derive(Debug, Clone)]
pub struct MemoryAnalyzer {
    /// 分析器ID
    pub id: u64,
    /// 内存快照
    pub memory_snapshots: Vec<MemorySnapshot>,
    /// 内存泄漏检测器
    leak_detector: LeakDetector,
    /// 内存使用统计
    pub usage_statistics: MemoryUsageStatistics,
    /// 内存映射信息
    memory_mappings: Vec<MemoryMapping>,
    /// 堆分析器
    stack_analyzer: StackAnalyzer,
}

impl MemoryAnalyzer {
    /// 创建新的内存分析器
    pub fn new(id: u64) -> Self {
        Self {
            id,
            memory_snapshots: Vec::new(),
            leak_detector: LeakDetector {
                id: 1,
                allocation_records: BTreeMap::new(),
                deallocation_records: BTreeMap::new(),
                leak_threshold: 1000,
                detection_stats: LeakDetectionStats::default(),
            },
            usage_statistics: MemoryUsageStatistics::default(),
            memory_mappings: Vec::new(),
            stack_analyzer: StackAnalyzer {
                id: 1,
                current_call_stack: Vec::new(),
                stack_trace_history: Vec::new(),
                overflow_detector: StackOverflowDetector {
                    id: 1,
                    stack_limit: 8 * 1024 * 1024, // 8MB
                    current_depth: 0,
                    detection_threshold: 0.8,
                    detected_overflows: 0,
                },
            },
        }
    }

    /// 初始化内存分析器
    pub fn init(&mut self) -> Result<(), &'static str> {
        // 清空内存快照
        self.memory_snapshots.clear();
        // 初始化泄漏检测器
        self.leak_detector.allocation_records.clear();
        self.leak_detector.deallocation_records.clear();
        self.leak_detector.detection_stats = LeakDetectionStats::default();
        // 重置内存使用统计
        self.usage_statistics = MemoryUsageStatistics::default();
        // 清空内存映射
        self.memory_mappings.clear();
        // 重置堆分析器
        self.stack_analyzer.current_call_stack.clear();
        self.stack_analyzer.stack_trace_history.clear();
        Ok(())
    }
}

/// 内存快照
#[derive(Debug, Clone)]
pub struct MemorySnapshot {
    /// 快照ID
    pub id: u64,
    /// 快照时间
    pub timestamp: u64,
    /// 进程ID
    pub process_id: u32,
    /// 线程ID
    pub thread_id: u32,
    /// 内存区域
    pub memory_regions: Vec<MemoryRegion>,
    /// 堆信息
    pub stack_info: Vec<StackFrame>,
    /// 堆指针
    pub stack_pointer: u64,
    /// 堆大小
    pub stack_size: u64,
    /// 堆使用量
    pub stack_usage: u64,
}

/// 内存区域
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// 区域ID
    pub id: String,
    /// 区域类型
    pub region_type: MemoryRegionType,
    /// 起始地址
    pub start_address: u64,
    /// 结束地址
    pub end_address: u64,
    /// 区域大小
    pub size: u64,
    /// 权限
    pub permissions: MemoryPermissions,
    /// 区域名称
    pub name: Option<String>,
    /// 映射文件
    pub mapped_file: Option<String>,
}

/// 内存区域类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// 代码段
    Code,
    /// 数据段
    Data,
    /// 堆段
    Stack,
    /// 堆段
    Heap,
    /// 共享内存
    Shared,
    /// 内存映射文件
    MappedFile,
    /// 设备内存
    DeviceMemory,
    /// 内核内存
    Kernel,
    /// 保留内存
    Reserved,
    /// 未知
    Unknown,
}

/// 内存权限
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryPermissions {
    /// 可读
    pub readable: bool,
    /// 可写
    pub writable: bool,
    /// 可执行
    pub executable: bool,
    /// 共享
    pub shared: bool,
    /// 私有
    pub private: bool,
}

/// 泄漏检测器
#[derive(Debug, Clone)]
pub struct LeakDetector {
    /// 检测器ID
    pub id: u64,
    /// 检测的分配记录
    pub allocation_records: BTreeMap<u64, AllocationRecord>,
    /// 检测的释放记录
    pub deallocation_records: BTreeMap<u64, DeallocationRecord>,
    /// 疑似泄漏阈值
    pub leak_threshold: u64,
    /// 检测统计
    pub detection_stats: LeakDetectionStats,
}

/// 分配记录
#[derive(Debug, Clone)]
pub struct AllocationRecord {
    /// 分配ID
    pub id: u64,
    /// 分配地址
    pub address: u64,
    /// 分配大小
    pub size: u64,
    /// 分配时间
    pub timestamp: u64,
    /// 分配类型
    pub allocation_type: AllocationType,
    /// 调用栈
    pub call_stack: Vec<CallFrame>,
    /// 线程ID
    pub thread_id: u32,
    /// 进程ID
    pub process_id: u32,
    /// 分配标记
    pub tags: Vec<String>,
}

/// 释放记录
#[derive(Debug, Clone)]
pub struct DeallocationRecord {
    /// 释放ID
    pub id: u64,
    /// 释放地址
    pub address: u64,
    /// 释放大小
    pub size: u64,
    /// 释放时间
    pub timestamp: u64,
    /// 对应的分配ID
    pub allocation_id: u64,
    /// 释放类型
    pub deallocation_type: DeallocationType,
    /// 线程ID
    pub thread_id: u32,
    /// 进程ID
    pub process_id: u32,
}

/// 分配类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AllocationType {
    /// 堆分配
    Stack,
    /// 堆分配
    Heap,
    /// 静态分配
    Static,
    /// 共享内存
    Shared,
    /// 内存映射
    Mapped,
    /// 设备分配
    Device,
    /// 自定义分配
    Custom,
}

/// 释放类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeallocationType {
    /// 正常释放
    Normal,
    /// 自动释放
    Automatic,
    /// 强制释放
    Forced,
    /// 异常释放
    Exception,
}

/// 泄漏检测统计
#[derive(Debug, Clone, Default)]
pub struct LeakDetectionStats {
    /// 总分配次数
    pub total_allocations: u64,
    /// 总释放次数
    pub total_deallocations: u64,
    /// 检测到的泄漏
    pub detected_leaks: u64,
    /// 总泄漏大小
    pub total_leaked_bytes: u64,
    /// 平均泄漏大小
    pub avg_leak_size: f64,
    /// 最大泄漏大小
    pub max_leak_size: u64,
    /// 检测效率
    pub detection_efficiency: f64,
}

/// 内存使用统计
#[derive(Debug, Clone, Default)]
pub struct MemoryUsageStatistics {
    /// 总分配内存
    pub total_allocated: u64,
    /// 总释放内存
    pub total_freed: u64,
    /// 当前使用内存
    pub current_usage: u64,
    /// 峰值使用量
    pub peak_usage: u64,
    /// 分配次数
    pub allocation_count: u64,
    /// 释放次数
    pub deallocation_count: u64,
    /// 内存碎片化程度
    pub fragmentation_ratio: f64,
    /// 平均分配大小
    pub avg_allocation_size: f64,
}

/// 内存映射信息
#[derive(Debug, Clone)]
pub struct MemoryMapping {
    /// 映射ID
    pub id: String,
    /// 映射地址
    pub address: u64,
    /// 映射大小
    pub size: u64,
    /// 映射文件路径
    pub file_path: String,
    /// 映射权限
    pub permissions: MemoryPermissions,
    /// 映射类型
    pub mapping_type: MappingType,
    /// 映射状态
    pub status: MappingStatus,
    /// 创建时间
    pub created_at: u64,
}

/// 映射类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingType {
    /// 私有映射
    Private,
    /// 共享映射
    Shared,
    /// 匿名映射
    Anonymous,
    /// 文件映射
    File,
    /// 设备映射
    Device,
}

/// 映射状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MappingStatus {
    /// 活动
    Active,
    /// 非活动
    Inactive,
    /// 错误
    Error,
}

/// 堆分析器
#[derive(Debug, Clone)]
pub struct StackAnalyzer {
    /// 分析器ID
    pub id: u64,
    /// 当前线程的调用栈
    pub current_call_stack: Vec<StackFrame>,
    /// 堆跟踪历史
    pub stack_trace_history: Vec<StackTrace>,
    /// 堆溢出检测
    pub overflow_detector: StackOverflowDetector,
}

/// 堆跟踪
#[derive(Debug, Clone)]
pub struct StackTrace {
    /// 跟踪ID
    pub id: u64,
    /// 跟踪时间
    pub timestamp: u64,
    /// 线程ID
    pub thread_id: u32,
    /// 调用栈
    pub call_stack: Vec<StackFrame>,
    /// 栈深度
    pub stack_depth: u32,
    /// 跟踪类型
    pub trace_type: StackTraceType,
}

/// 堆跟踪类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackTraceType {
    /// 函数调用跟踪
    FunctionCall,
    /// 异常跟踪
    Exception,
    /// 中断跟踪
    Interrupt,
    /// 系统调用跟踪
    SystemCall,
}

/// 堆溢出检测器
#[derive(Debug, Clone)]
pub struct StackOverflowDetector {
    /// 检测器ID
    pub id: u64,
    /// 栈限制
    pub stack_limit: u64,
    /// 当前栈深度
    pub current_depth: u64,
    /// 检测阈值
    pub detection_threshold: f64,
    /// 检测到的溢出次数
    pub detected_overflows: u64,
}

/// 栈帧（用于兼容性，与CallFrame结构相同）
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// 帧地址
    pub return_address: u64,
    /// 帧指针
    pub frame_pointer: u64,
    /// 栈指针
    pub stack_pointer: u64,
    /// 函数地址
    pub function_address: u64,
    /// 函数名
    pub function_name: Option<String>,
    /// 模块名
    pub module_name: Option<String>,
    /// 源文件位置
    pub source_location: Option<SourceLocation>,
    /// 局部变量
    pub local_variables: Vec<VariableInfo>,
    /// 参数
    pub parameters: Vec<VariableInfo>,
    /// 帧大小
    pub frame_size: u64,
}

/// 调用帧
#[derive(Debug, Clone)]
pub struct CallFrame {
    /// 帧地址
    pub return_address: u64,
    /// 帧指针
    pub frame_pointer: u64,
    /// 栈指针
    pub stack_pointer: u64,
    /// 函数地址
    pub function_address: u64,
    /// 函数名
    pub function_name: Option<String>,
    /// 模块名
    pub module_name: Option<String>,
    /// 源文件位置
    pub source_location: Option<SourceLocation>,
    /// 局部变量
    pub local_variables: Vec<VariableInfo>,
    /// 参数
    pub parameters: Vec<VariableInfo>,
    /// 帧大小
    pub frame_size: u64,
}

/// 变量信息
#[derive(Debug, Clone)]
pub struct VariableInfo {
    /// 变量名
    pub name: String,
    /// 变量类型
    pub var_type: String,
    /// 变量地址
    pub address: u64,
    /// 变量大小
    pub size: u64,
    /// 变量值
    pub value: Option<String>,
}

/// 性能分析器
#[derive(Debug, Clone)]
pub struct PerformanceAnalyzer {
    /// 分析器ID
    pub id: u64,
    /// 性能计数器
    pub performance_counters: BTreeMap<String, PerformanceCounter>,
    /// 采样数据
    pub sampling_data: Vec<PerformanceSample>,
    /// 热点分析
    pub hotspot_analysis: HotspotAnalysis,
    /// 性能报告
    pub performance_reports: Vec<PerformanceReport>,
    /// 分析配置
    pub analysis_config: PerformanceAnalysisConfig,
}

impl PerformanceAnalyzer {
    /// 初始化性能分析器
    pub fn init(&mut self) -> Result<(), &'static str> {
        // 清空性能计数器
        self.performance_counters.clear();
        // 清空采样数据
        self.sampling_data.clear();
        // 重置热点分析
        self.hotspot_analysis.hotspot_functions.clear();
        self.hotspot_analysis.hotspot_lines.clear();
        self.hotspot_analysis.hotspot_modules.clear();
        self.hotspot_analysis.analysis_time = 0;
        // 清空性能报告
        self.performance_reports.clear();
        // 重置分析配置为默认值
        self.analysis_config = PerformanceAnalysisConfig::default();
        Ok(())
    }
}

/// 性能计数器
#[derive(Debug, Clone)]
pub struct PerformanceCounter {
    /// 计数器ID
    pub id: String,
    /// 计数器名称
    pub name: String,
    /// 计数器类型
    pub counter_type: CounterType,
    /// 当前值
    pub current_value: u64,
    /// 总计值
    pub total_value: u64,
    /// 平均值
    pub average_value: f64,
    /// 最大值
    pub max_value: u64,
    /// 最小值
    pub min_value: u64,
    /// 重置次数
    pub reset_count: u64,
    /// 更新时间
    pub last_updated: u64,
    /// 计数器单位
    pub unit: String,
}

/// 计数器类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterType {
    /// 计数器
    Counter,
    /// 计时器
    Timer,
    /// 仪表器
    Gauge,
    /// 直方图
    Histogram,
    /// 摘要统计
    Summary,
    /// 累计器
    Accumulator,
}

/// 性能采样
#[derive(Debug, Clone)]
pub struct PerformanceSample {
    /// 采样ID
    pub id: String,
    /// 采样时间
    pub timestamp: u64,
    /// 采样数据
    pub sample_data: BTreeMap<String, f64>,
    /// 系统状态
    pub system_state: SystemState,
    /// 线程信息
    pub process_info: ProcessInfo,
    /// CPU信息
    pub cpu_info: CPUInfo,
    /// 内存信息
    pub memory_info: MemoryInfo,
    /// 网络信息
    pub network_info: NetworkInfo,
}

/// 热点分析配置
#[derive(Debug, Clone)]
pub struct HotspotAnalysisConfig {
    /// 最小执行次数
    pub min_execution_count: u64,
    /// 最小执行时间（纳秒）
    pub min_execution_time: u64,
    /// 热点阈值
    pub hotspot_threshold: f64,
    /// 包含系统函数
    pub include_system_functions: bool,
    /// 最大热点函数数
    pub max_hotspot_functions: usize,
    /// 最大热点行数
    pub max_hotspot_lines: usize,
    /// 最大热点模块数
    pub max_hotspot_modules: usize,
}

impl Default for HotspotAnalysisConfig {
    fn default() -> Self {
        Self {
            min_execution_count: 10,
            min_execution_time: 1000,
            hotspot_threshold: 0.05,
            include_system_functions: false,
            max_hotspot_functions: 50,
            max_hotspot_lines: 100,
            max_hotspot_modules: 20,
        }
    }
}

/// 热点分析
#[derive(Debug, Clone)]
pub struct HotspotAnalysis {
    /// 分析ID
    pub id: u64,
    /// 分析时间
    pub analysis_time: u64,
    /// 热点函数
    pub hotspot_functions: Vec<HotspotFunction>,
    /// 热点代码行
    pub hotspot_lines: Vec<HotspotLine>,
    /// 热点模块
    pub hotspot_modules: Vec<HotspotModule>,
    /// 分析配置
    pub analysis_config: HotspotAnalysisConfig,
}

/// 热点函数
#[derive(Debug, Clone)]
pub struct HotspotFunction {
    /// 函数名
    pub function_name: String,
    /// 模块名
    pub module_name: String,
    /// 执行次数
    pub execution_count: u64,
    /// 总执行时间
    pub total_execution_time: u64,
    /// 平均执行时间
    pub avg_execution_time: f64,
    /// 最大执行时间
    pub max_execution_time: u64,
    /// 热点分数
    pub hotspot_score: f64,
    /// 源文件位置
    pub source_location: Option<SourceLocation>,
}

/// 热点代码行
#[derive(Debug, Clone)]
pub struct HotspotLine {
    /// 行号
    pub line_number: u32,
    /// 文件名
    pub file_name: String,
    /// 模块名
    pub module_name: String,
    /// 函数名
    pub function_name: Option<String>,
    /// 执行次数
    pub execution_count: u64,
    /// 执行时间
    pub execution_time: u64,
    /// 热点分数
    pub hotspot_score: f64,
    /// 代码内容
    pub code_content: String,
}

/// 热点模块
#[derive(Debug, Clone)]
pub struct HotspotModule {
    /// 模块名
    pub module_name: String,
    /// 文件路径
    pub file_path: String,
    /// 总执行时间
    pub total_execution_time: u64,
    /// 函数数量
    pub function_count: u32,
    /// 热点分数
    pub hotspot_score: f64,
    /// 代码行数
    pub line_count: u32,
    /// 复杂度评分
    pub complexity_score: f64,
}

/// 性能报告
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    /// 报告ID
    pub id: String,
    /// 报告名称
    pub name: String,
    /// 报告类型
    pub report_type: ReportType,
    /// 生成时间
    pub generated_at: u64,
    /// 报告期间
    pub time_range: TimeRange,
    /// 性能摘要
    pub performance_summary: PerformanceSummary,
    /// 详细分析
    pub detailed_analysis: Vec<AnalysisResult>,
    /// 建议
    pub recommendations: Vec<Recommendation>,
}

/// 报告类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportType {
    /// 综合报告
    Comprehensive,
    /// 内存报告
    Memory,
    /// CPU报告
    CPU,
    /// I/O报告
    IO,
    /// 网络报告
    Network,
    /// 函数分析报告
    FunctionAnalysis,
    /// 热点分析报告
    HotspotAnalysis,
}

/// 时间范围
#[derive(Debug, Clone)]
pub struct TimeRange {
    /// 开始时间
    pub start_time: u64,
    /// 结束时间
    pub end_time: u64,
    /// 持续时间
    pub duration: u64,
}

/// 性能摘要
#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    /// 总执行时间
    pub total_execution_time: u64,
    /// 平均执行时间
    pub avg_execution_time: f64,
    /// 最大执行时间
    pub max_execution_time: u64,
    /// 总调用次数
    pub total_calls: u64,
    /// 错误率
    pub error_rate: f64,
    /// CPU使用率
    pub cpu_usage: f64,
    /// 内存使用率
    pub memory_usage: f64,
    /// I/O等待时间
    pub io_wait_time: u64,
}

/// 分析结果
#[derive(Debug, Clone)]
pub struct AnalysisResult {
    /// 结果类型
    pub result_type: AnalysisResultType,
    /// 结果描述
    pub description: String,
    /// 结果数据
    pub result_data: BTreeMap<String, String>,
    /// 置信度
    pub confidence: f64,
    /// 重要性
    pub importance: AnalysisImportance,
}

/// 分析结果类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisResultType {
    /// 性能问题
    PerformanceIssue,
    /// 内存问题
    MemoryIssue,
    /// 错误模式
    ErrorPattern,
    /// 优化建议
    OptimizationOpportunity,
    /// 瓶颈识别
    Bottleneck,
    /// 代码质量问题
    CodeQualityIssue,
}

/// 分析重要性
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalysisImportance {
    /// 低
    Low,
    /// 中
    Medium,
    /// 高
    High,
    /// 关键
    Critical,
}

/// 建议
#[derive(Debug, Clone)]
pub struct Recommendation {
    /// 建议ID
    pub id: String,
    /// 建议类型
    pub recommendation_type: RecommendationType,
    /// 建议描述
    pub description: String,
    /// 建议优先级
    pub priority: RecommendationPriority,
    /// 预期效果
    pub expected_impact: String,
    /// 实施难度
    pub implementation_difficulty: ImplementationDifficulty,
    /// 实施时间估计
    pub estimated_time: u64,
    /// 参考资源
    pub references: Vec<String>,
}

/// 建议类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecommendationType {
    /// 算单优化
    SimpleOptimization,
    /// 算法优化
    AlgorithmOptimization,
    /// 数据结构优化
    DataStructureOptimization,
    /// 内存优化
    MemoryOptimization,
    /// I/O优化
    IOOptimization,
    /// 并发优化
    ConcurrencyOptimization,
    /// 系统架构优化
    ArchitectureOptimization,
    /// 代码重构
    CodeRefactoring,
}

/// 建议优先级
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecommendationPriority {
    /// 低
    Low,
    /// 中
    Medium,
    /// 高
    High,
    /// 紧急
    Urgent,
}

/// 实施难度
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImplementationDifficulty {
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
pub struct PerformanceAnalysisConfig {
    /// 采样间隔（毫秒）
    pub sampling_interval_ms: u64,
    /// 采样窗口大小
    pub sampling_window_size: usize,
    /// 热点分析阈值
    pub hotspot_threshold: f64,
    /// 启用调用图分析
    pub enable_call_graph_analysis: bool,
    /// 启用内存泄漏检测
    pub enable_leak_detection: bool,
    /// 启用性能预测
    pub enable_performance_prediction: bool,
    /// 报告生成间隔（秒）
    pub report_generation_interval_seconds: u64,
}

impl Default for PerformanceAnalysisConfig {
    fn default() -> Self {
        Self {
            sampling_interval_ms: 100,
            sampling_window_size: 1000,
            hotspot_threshold: 0.05,
            enable_call_graph_analysis: true,
            enable_leak_detection: true,
            enable_performance_prediction: false,
            report_generation_interval_seconds: 300, // 5分钟
        }
    }
}

/// 系统状态
#[derive(Debug, Clone)]
pub struct SystemState {
    /// 系统负载
    pub system_load: f64,
    /// 上下文切换次数
    pub context_switches: u64,
    /// 中断次数
    pub interrupts: u64,
    /// 系统调用次数
    pub system_calls: u64,
    /// 页面错误次数
    pub page_faults: u64,
    /// 缓存未命中次数
    pub cache_misses: u64,
}

/// CPU信息
#[derive(Debug, Clone)]
pub struct CPUInfo {
    /// CPU频率（MHz）
    pub cpu_frequency: f64,
    /// CPU使用率
    pub cpu_usage: f64,
    /// 用户态使用率
    pub user_usage: f64,
    /// 系统态使用率
    pub system_usage: f64,
    /// 空闲率
    pub idle_usage: f64,
    /// 等待率
    pub wait_usage: f64,
    /// 中断率
    pub interrupt_rate: f64,
    /// 温度（如果支持）
    pub temperature: Option<f64>,
}

/// 内存信息
#[derive(Debug, Clone)]
pub struct MemoryInfo {
    /// 总内存
    pub total_memory: u64,
    /// 可用内存
    pub available_memory: u64,
    /// 已用内存
    pub used_memory: u64,
    /// 缓存内存
    pub cached_memory: u64,
    /// 交换内存
    pub swap_memory: u64,
    /// 共享内存
    pub shared_memory: u64,
    /// 内存使用率
    pub memory_usage: f64,
    /// 内存碎片化
    pub fragmentation_ratio: f64,
}

/// 网络信息
#[derive(Debug, Clone)]
pub struct NetworkInfo {
    /// 网络接口状态
    pub interface_status: Vec<InterfaceStatus>,
    /// 活动连接数
    pub active_connections: u32,
    /// 总接收字节数
    pub total_rx_bytes: u64,
    /// 总发送字节数
    pub total_tx_bytes: u64,
    /// 接收包数
    pub total_rx_packets: u64,
    /// 发送包数
    pub total_tx_packets: u64,
    /// 网络延迟
    pub network_latency: f64,
    /// 带宽利用率
    pub bandwidth_utilization: f64,
}

/// 接口状态
#[derive(Debug, Clone)]
pub struct InterfaceStatus {
    /// 接口名称
    pub interface_name: String,
    /// 接口状态
    pub status: InterfaceState,
    /// 接口速度
    pub speed: u64,
    /// 接收字节数
    pub rx_bytes: u64,
    /// 发送字节数
    pub tx_bytes: u64,
    /// 接收包数
    pub rx_packets: u64,
    /// 发送包数
    pub tx_packets: u64,
    /// 错误包数
    pub error_packets: u64,
    /// 接口类型
    pub interface_type: InterfaceType,
}

/// 接口状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceState {
    /// 活动
    Up,
    /// 非活动
    Down,
    /// 管理中
    AdminDown,
    /// 测试模式
    Testing,
    /// 禁用
    Disabled,
}

/// 接口类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    /// 以太网
    Ethernet,
    /// 无线网络
    Wireless,
    /// 回环接口
    Loopback,
    /// 虚拟接口
    Virtual,
    /// 点对点
    PointToPoint,
    /// 广播
    Broadcast,
}

/// 性能快照
#[derive(Debug, Clone)]
pub struct PerformanceSnapshot {
    /// 快照ID
    pub id: String,
    /// 快照时间
    pub timestamp: u64,
    /// 进程ID
    pub process_id: u32,
    /// 线程ID
    pub thread_id: u32,
    /// CPU信息
    pub cpu_info: CPUInfo,
    /// 内存信息
    pub memory_info: MemoryInfo,
    /// 性能计数器
    pub performance_counters: BTreeMap<String, u64>,
}
