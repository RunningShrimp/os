// Error Handling and Diagnostics Module

extern crate alloc;
//
// 错误处理和诊断模块
// 提供全面的错误处理、诊断分析和恢复机制

pub mod error_registry;
pub mod error_classifier;
pub mod error_recovery;
pub mod diagnostic_analyzer;
pub mod error_reporting;
pub mod unified;
pub mod error_tracing;
pub mod fault_tolerance;
pub mod system_health;
pub mod recovery_manager;
pub mod diagnostic_tools;
pub mod unified_error;
pub mod unified_engine;
pub mod error_prediction;
pub mod self_healing;
pub mod error_handling_traits;

// Re-export all public types and functions
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::String;
use alloc::string::ToString;
use core::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use spin::Mutex;
use spin::Once;

/// 错误严重级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorSeverity {
    /// 信息级别
    Info = 0,
    /// 低级别
    Low = 1,
    /// 警告级别
    Warning = 2,
    /// 中等级别
    Medium = 3,
    /// 高级别
    High = 4,
    /// 错误级别
    Error = 5,
    /// 严重错误
    Critical = 6,
    /// 致命错误
    Fatal = 7,
}

/// 错误类别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorCategory {
    /// 系统错误
    System,
    /// 内存错误
    Memory,
    /// 文件系统错误
    FileSystem,
    /// 网络错误
    Network,
    /// 设备错误
    Device,
    /// 进程错误
    Process,
    /// 安全错误
    Security,
    /// 应用错误
    Application,
    /// 硬件错误
    Hardware,
    /// 配置错误
    Configuration,
    /// 用户错误
    User,
    /// 资源错误
    Resource,
    /// 超时错误
    Timeout,
    /// 协议错误
    Protocol,
    /// 数据错误
    Data,
    /// 接口错误
    Interface,
}

/// 错误状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorStatus {
    /// 新错误
    New,
    /// 正在处理
    Processing,
    /// 活跃
    Active,
    /// 已恢复
    Recovered,
    /// 已处理
    Handled,
    /// 忽略
    Ignored,
    /// 升级中
    Escalated,
    /// 已关闭
    Closed,
}

/// 错误类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ErrorType {
    /// 运行时错误
    RuntimeError,
    /// 逻辑错误
    LogicError,
    /// 编译时错误
    CompileError,
    /// 配置错误
    ConfigurationError,
    /// 资源错误
    ResourceError,
    /// 权限错误
    PermissionError,
    /// 网络错误
    NetworkError,
    /// I/O错误
    IOError,
    /// 内存错误
    MemoryError,
    /// 系统调用错误
    SystemCallError,
    /// 验证错误
    ValidationError,
    /// 超时错误
    TimeoutError,
    /// 取消错误
    CancellationError,
    /// 系统错误（兼容旧代码）
    SystemError,
}

impl core::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ErrorType::RuntimeError => write!(f, "RuntimeError"),
            ErrorType::LogicError => write!(f, "LogicError"),
            ErrorType::CompileError => write!(f, "CompileError"),
            ErrorType::ConfigurationError => write!(f, "ConfigurationError"),
            ErrorType::ResourceError => write!(f, "ResourceError"),
            ErrorType::PermissionError => write!(f, "PermissionError"),
            ErrorType::NetworkError => write!(f, "NetworkError"),
            ErrorType::IOError => write!(f, "IOError"),
            ErrorType::MemoryError => write!(f, "MemoryError"),
            ErrorType::SyscallError => write!(f, "SyscallError"),
            ErrorType::SecurityError => write!(f, "SecurityError"),
            ErrorType::ValidationError => write!(f, "ValidationError"),
            ErrorType::TimeoutError => write!(f, "TimeoutError"),
            ErrorType::CancellationError => write!(f, "CancellationError"),
            ErrorType::SystemError => write!(f, "SystemError"),
        }
    }
}

/// 恢复策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RecoveryStrategy {
    /// 无恢复
    None,
    /// 自动重试
    Retry,
    /// 降级服务
    Degrade,
    /// 重启组件
    Restart,
    /// 释放资源（兼容旧代码）
    Release,
    /// 切换到备份
    Failover,
    /// 隔离故障
    Isolate,
    /// 用户干预
    Manual,
    /// 忽略错误
    Ignore,
}

/// 错误记录
#[derive(Debug, Clone)]
pub struct ErrorRecord {
    /// 错误ID
    pub id: u64,
    /// 错误代码
    pub code: u32,
    /// 错误类型
    pub error_type: ErrorType,
    /// 错误类别
    pub category: ErrorCategory,
    /// 严重级别
    pub severity: ErrorSeverity,
    /// 错误状态
    pub status: ErrorStatus,
    /// 错误消息
    pub message: String,
    /// 详细描述
    pub description: String,
    /// 错误源
    pub source: ErrorSource,
    /// 发生时间
    pub timestamp: u64,
    /// 错误上下文
    pub context: ErrorContext,
    /// 堆栈跟踪
    pub stack_trace: Vec<StackFrame>,
    /// 相关系统状态
    pub system_state: SystemStateSnapshot,
    /// 恢复动作
    pub recovery_actions: Vec<RecoveryAction>,
    /// 重复次数
    pub occurrence_count: u32,
    /// 上次发生时间
    pub last_occurrence: u64,
    /// 是否已解决
    pub resolved: bool,
    /// 解决时间
    pub resolution_time: Option<u64>,
    /// 解决方法
    pub resolution_method: Option<String>,
    /// 元数据
    pub metadata: BTreeMap<String, String>,
}

/// 错误源
#[derive(Debug, Clone)]
pub struct ErrorSource {
    /// 源模块
    pub module: String,
    /// 源函数
    pub function: String,
    /// 源文件
    pub file: String,
    /// 行号
    pub line: u32,
    /// 列号
    pub column: u32,
    /// 进程ID
    pub process_id: u32,
    /// 线程ID
    pub thread_id: u32,
    /// CPU ID
    pub cpu_id: u32,
}

/// 错误上下文
#[derive(Debug, Clone)]
pub struct ErrorContext {
    /// 环境变量
    pub environment_variables: BTreeMap<String, String>,
    /// 系统配置
    pub system_config: BTreeMap<String, String>,
    /// 用户输入
    pub user_input: Option<String>,
    /// 相关数据
    pub related_data: Vec<u8>,
    /// 操作序列
    pub operation_sequence: Vec<String>,
    /// 错误前置条件
    pub preconditions: Vec<String>,
    /// 错误后置条件
    pub postconditions: Vec<String>,
}

/// 堆栈帧
#[derive(Debug, Clone)]
pub struct StackFrame {
    /// 函数名
    pub function: String,
    /// 模块名
    pub module: String,
    /// 文件名
    pub file: String,
    /// 行号
    pub line: u32,
    /// 函数地址
    pub address: u64,
    /// 偏移量
    pub offset: u64,
}

/// 系统状态快照
#[derive(Debug, Clone)]
pub struct SystemStateSnapshot {
    /// 内存使用
    pub memory_usage: MemoryUsage,
    /// CPU使用率
    pub cpu_usage: CpuUsage,
    /// 进程状态
    pub process_states: Vec<ProcessState>,
    /// 网络状态
    pub network_state: NetworkState,
    /// 文件系统状态
    pub filesystem_state: FileSystemState,
    /// 设备状态
    pub device_states: Vec<DeviceState>,
    /// 系统负载
    pub system_load: SystemLoad,
    /// 采集时间
    pub timestamp: u64,
}

impl Default for SystemStateSnapshot {
    fn default() -> Self {
        Self {
            memory_usage: MemoryUsage {
                total_memory: 0,
                used_memory: 0,
                available_memory: 0,
                cached_memory: 0,
                swap_used: 0,
                kernel_memory: 0,
            },
            cpu_usage: CpuUsage {
                usage_percent: 0.0,
                user_percent: 0.0,
                system_percent: 0.0,
                idle_percent: 0.0,
                wait_percent: 0.0,
                interrupt_percent: 0.0,
            },
            process_states: Vec::new(),
            network_state: NetworkState {
                active_connections: 0,
                listening_ports: 0,
                interfaces: Vec::new(),
                packet_stats: PacketStats {
                    total_rx: 0,
                    total_tx: 0,
                    dropped: 0,
                    errors: 0,
                },
            },
            filesystem_state: FileSystemState {
                mount_points: Vec::new(),
                disk_usage: Vec::new(),
                io_stats: IoStats {
                    read_operations: 0,
                    write_operations: 0,
                    read_bytes: 0,
                    write_bytes: 0,
                    io_wait_time: 0,
                },
            },
            device_states: Vec::new(),
            system_load: SystemLoad {
                load_1min: 0.0,
                load_5min: 0.0,
                load_15min: 0.0,
                run_queue_length: 0,
                blocked_processes: 0,
            },
            timestamp: 0,
        }
    }
}

/// 内存使用情况
#[derive(Debug, Clone)]
pub struct MemoryUsage {
    /// 总内存
    pub total_memory: u64,
    /// 已用内存
    pub used_memory: u64,
    /// 可用内存
    pub available_memory: u64,
    /// 缓存内存
    pub cached_memory: u64,
    /// 交换使用
    pub swap_used: u64,
    /// 内核内存
    pub kernel_memory: u64,
}

/// CPU使用情况
#[derive(Debug, Clone)]
pub struct CpuUsage {
    /// CPU使用率（百分比）
    pub usage_percent: f64,
    /// 用户态使用率
    pub user_percent: f64,
    /// 系统态使用率
    pub system_percent: f64,
    /// 空闲率
    pub idle_percent: f64,
    /// 等待率
    pub wait_percent: f64,
    /// 中断率
    pub interrupt_percent: f64,
}

/// 进程状态快照
#[derive(Debug, Clone)]
pub struct ProcessState {
    /// 进程ID
    pub process_id: u32,
    /// 进程名
    pub name: String,
    /// 进程状态
    pub status: String,
    /// CPU使用率
    pub cpu_usage: f64,
    /// 内存使用
    pub memory_usage: u64,
    /// 打开的文件数
    pub open_files: u32,
    /// 线程数
    pub thread_count: u32,
    /// 运行时间
    pub runtime: u64,
}

/// 网络状态快照
#[derive(Debug, Clone)]
pub struct NetworkState {
    /// 活动连接数
    pub active_connections: u32,
    /// 监听端口数
    pub listening_ports: u32,
    /// 网络接口状态
    pub interfaces: Vec<NetworkInterface>,
    /// 数据包统计
    pub packet_stats: PacketStats,
}

/// 网络接口状态
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    /// 接口名称
    pub name: String,
    /// 接口状态
    pub status: String,
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
}

/// 数据包统计
#[derive(Debug, Clone)]
pub struct PacketStats {
    /// 总接收包
    pub total_rx: u64,
    /// 总发送包
    pub total_tx: u64,
    /// 丢弃包
    pub dropped: u64,
    /// 错误包
    pub errors: u64,
}

/// 文件系统状态快照
#[derive(Debug, Clone)]
pub struct FileSystemState {
    /// 挂载点状态
    pub mount_points: Vec<MountPoint>,
    /// 磁盘使用情况
    pub disk_usage: Vec<DiskUsage>,
    /// I/O统计
    pub io_stats: IoStats,
}

/// 挂载点状态
#[derive(Debug, Clone)]
pub struct MountPoint {
    /// 挂载点路径
    pub mount_point: String,
    /// 设备名
    pub device: String,
    /// 文件系统类型
    pub filesystem_type: String,
    /// 挂载选项
    pub options: String,
    /// 状态
    pub status: String,
}

/// 磁盘使用情况
#[derive(Debug, Clone)]
pub struct DiskUsage {
    /// 磁盘设备
    pub device: String,
    /// 总大小
    pub total_size: u64,
    /// 已用大小
    pub used_size: u64,
    /// 可用大小
    pub available_size: u64,
    /// 使用百分比
    pub usage_percent: f64,
}

/// I/O统计
#[derive(Debug, Clone)]
pub struct IoStats {
    /// 读操作数
    pub read_operations: u64,
    /// 写操作数
    pub write_operations: u64,
    /// 读字节数
    pub read_bytes: u64,
    /// 写字节数
    pub write_bytes: u64,
    /// I/O等待时间
    pub io_wait_time: u64,
}

/// 设备状态
#[derive(Debug, Clone)]
pub struct DeviceState {
    /// 设备名称
    pub name: String,
    /// 设备类型
    pub device_type: String,
    /// 设备状态
    pub status: String,
    /// 驱动程序
    pub driver: String,
    /// 设备参数
    pub parameters: BTreeMap<String, String>,
}

/// 系统负载
#[derive(Debug, Clone)]
pub struct SystemLoad {
    /// 1分钟平均负载
    pub load_1min: f64,
    /// 5分钟平均负载
    pub load_5min: f64,
    /// 15分钟平均负载
    pub load_15min: f64,
    /// 运行队列长度
    pub run_queue_length: u32,
    /// 阻塞进程数
    pub blocked_processes: u32,
}

/// 恢复动作
#[derive(Debug, Clone)]
pub struct RecoveryAction {
    /// 动作ID
    pub id: u64,
    /// 动作类型
    pub action_type: RecoveryActionType,
    /// 动作名称
    pub name: String,
    /// 动作描述
    pub description: String,
    /// 执行时间
    pub execution_time: u64,
    /// 成功标志
    pub success: bool,
    /// 结果消息
    pub result_message: String,
    /// 动作参数
    pub parameters: BTreeMap<String, String>,
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
    /// 释放资源
    Release,
    /// 分配资源
    Allocate,
    /// 隔离组件
    Isolate,
    /// 升级处理
    Escalate,
    /// 记录日志
    Log,
    /// 发送通知
    Notify,
    /// 回滚操作
    Rollback,
    /// 切换模式
    SwitchMode,
}

/// 错误处理配置
#[derive(Debug, Clone)]
pub struct ErrorHandlingConfig {
    /// 启用错误恢复
    pub enable_recovery: bool,
    /// 最大重试次数
    pub max_retries: u32,
    /// 重试间隔（毫秒）
    pub retry_interval_ms: u64,
    /// 错误升级阈值
    pub escalation_threshold: u32,
    /// 自动恢复策略
    pub auto_recovery_strategies: Vec<RecoveryStrategy>,
    /// 错误记录保留时间（秒）
    pub retention_period_seconds: u64,
    /// 最大错误记录数
    pub max_error_records: usize,
    /// 启用错误聚合
    pub enable_error_aggregation: bool,
    /// 聚合时间窗口（秒）
    pub aggregation_window_seconds: u64,
    /// 启用错误预测
    pub enable_error_prediction: bool,
    /// 启用健康检查
    pub enable_health_checks: bool,
    /// 健康检查间隔（秒）
    pub health_check_interval_seconds: u64,
}

impl Default for ErrorHandlingConfig {
    fn default() -> Self {
        Self {
            enable_recovery: true,
            max_retries: 3,
            retry_interval_ms: 1000,
            escalation_threshold: 5,
            auto_recovery_strategies: vec![
                RecoveryStrategy::Retry,
                RecoveryStrategy::Degrade,
            ],
            retention_period_seconds: 86400 * 7, // 7天
            max_error_records: 10000,
            enable_error_aggregation: true,
            aggregation_window_seconds: 300, // 5分钟
            enable_error_prediction: false,
            enable_health_checks: true,
            health_check_interval_seconds: 60,
        }
    }
}

/// 错误处理统计
#[derive(Debug, Clone, Default)]
pub struct ErrorHandlingStats {
    /// 总错误数
    pub total_errors: u64,
    /// 按类别统计
    pub errors_by_category: BTreeMap<ErrorCategory, u64>,
    /// 按严重级别统计
    pub errors_by_severity: BTreeMap<ErrorSeverity, u64>,
    /// 已恢复错误数
    pub recovered_errors: u64,
    /// 恢复成功率
    pub recovery_success_rate: f64,
    /// 平均恢复时间（毫秒）
    pub avg_recovery_time_ms: u64,
    /// 错误预测准确率
    pub prediction_accuracy: f64,
    /// 系统健康评分
    pub health_score: f64,
}

/// 全局错误处理引擎
pub struct ErrorHandlingEngine {
    /// 引擎ID
    pub id: u64,
    /// 引擎配置
    config: ErrorHandlingConfig,
    /// 错误记录
    error_records: Vec<ErrorRecord>,
    /// 错误注册表
    error_registry: Arc<Mutex<error_registry::ErrorRegistry>>,
    /// 错误分类器
    error_classifier: Arc<Mutex<error_classifier::ErrorClassifier>>,
    /// 恢复管理器
    recovery_manager: Arc<Mutex<recovery_manager::RecoveryManager>>,
    /// 诊断分析器
    diagnostic_analyzer: Arc<Mutex<diagnostic_analyzer::DiagnosticAnalyzer>>,
    /// 错误报告器
    error_reporter: Arc<Mutex<error_reporting::ErrorReporter>>,
    /// 错误追踪器
    error_tracer: Arc<Mutex<error_tracing::ErrorTracer>>,
    /// 系统健康监控
    system_health: Arc<Mutex<system_health::SystemHealthMonitor>>,
    /// 容错管理器
    fault_tolerance: Arc<Mutex<fault_tolerance::FaultToleranceManager>>,
    /// 诊断工具
    diagnostic_tools: Arc<Mutex<diagnostic_tools::DiagnosticTools>>,
    /// 统计信息
    stats: Arc<Mutex<ErrorHandlingStats>>,
    /// 错误计数器
    error_counter: AtomicU64,
    /// 是否正在运行
    running: AtomicBool,
}

impl ErrorHandlingEngine {
    /// 创建新的错误处理引擎
    pub fn new(config: ErrorHandlingConfig) -> Self {
        Self {
            id: 1,
            config,
            error_records: Vec::new(),
            error_registry: Arc::new(Mutex::new(error_registry::ErrorRegistry::new())),
            error_classifier: Arc::new(Mutex::new(error_classifier::ErrorClassifier::new())),
            recovery_manager: Arc::new(Mutex::new(recovery_manager::RecoveryManager::new())),
            diagnostic_analyzer: Arc::new(Mutex::new(diagnostic_analyzer::DiagnosticAnalyzer::new())),
            error_reporter: Arc::new(Mutex::new(error_reporting::ErrorReporter::new())),
            error_tracer: Arc::new(Mutex::new(error_tracing::ErrorTracer::new())),
            system_health: Arc::new(Mutex::new(system_health::SystemHealthMonitor::new())),
            fault_tolerance: Arc::new(Mutex::new(fault_tolerance::FaultToleranceManager::new())),
            diagnostic_tools: Arc::new(Mutex::new(diagnostic_tools::DiagnosticTools::new())),
            stats: Arc::new(Mutex::new(ErrorHandlingStats::default())),
            error_counter: AtomicU64::new(1),
            running: AtomicBool::new(false),
        }
    }

    /// 初始化错误处理引擎
    pub fn init(&mut self) -> Result<(), &'static str> {
        self.running.store(true, Ordering::SeqCst);

        // 初始化各个组件
        self.error_registry.lock().init()?;
        self.error_classifier.lock().init()?;
        self.recovery_manager.lock().init()?;
        self.diagnostic_analyzer.lock().init()?;
        self.error_reporter.lock().init()?;
        self.error_tracer.lock().init()?;
        self.system_health.lock().init()?;
        self.fault_tolerance.lock().init()?;
        self.diagnostic_tools.lock().init()?;

        crate::println!("[ErrorHandling] Error handling engine initialized successfully");
        Ok(())
    }

    /// 记录错误
    pub fn record_error(&mut self, error_record: ErrorRecord) -> Result<u64, &'static str> {
        if !self.running.load(Ordering::SeqCst) {
            return Err("Error handling engine is not running");
        }

        let error_id = self.error_counter.fetch_add(1, Ordering::SeqCst);

        // 分类错误
        let mut error_record = error_record;
        error_record.id = error_id;

        {
            let mut classifier = self.error_classifier.lock();
            classifier.classify_error(&mut error_record)?;
        }

        // 分析错误
        {
            let mut analyzer = self.diagnostic_analyzer.lock();
            analyzer.analyze_error(&error_record)?;
        }

        // 添加到记录列表
        self.error_records.push(error_record.clone());

        // 限制记录数量
        if self.error_records.len() > self.config.max_error_records {
            self.error_records.remove(0);
        }

        // 执行恢复动作
        if self.config.enable_recovery {
            self.execute_recovery_actions(&error_record)?;
        }

        // 报告错误
        {
            let mut reporter = self.error_reporter.lock();
            reporter.report_error(&error_record)?;
        }

        // 更新统计信息
        self.update_statistics(&error_record);

        Ok(error_id)
    }

    /// 执行恢复动作
    fn execute_recovery_actions(&mut self, error_record: &ErrorRecord) -> Result<(), &'static str> {
        let mut recovery_manager = self.recovery_manager.lock();

        for action in &error_record.recovery_actions {
            // Convert RecoveryAction to RecoveryActionExecution
            use recovery_manager::{RecoveryActionExecution, RecoveryActionStatus, RecoveryResult};
            use hashbrown::HashMap;
            let mut parameters = HashMap::with_hasher(crate::compat::DefaultHasherBuilder);
            for (k, v) in &action.parameters {
                parameters.insert(k.clone(), v.clone());
            }
            let result = if action.success {
                Some(RecoveryResult {
                    id: action.id,
                    action_id: action.id,
                    status: recovery_manager::RecoveryStatus::Success,
                    execution_time_ms: action.execution_time,
                    message: action.result_message.clone(),
                    error: None,
                    side_effects: Vec::new(),
                    affected_resources: Vec::new(),
                    performance_impact: recovery_manager::PerformanceImpact::None,
                })
            } else {
                None
            };
            let execution = RecoveryActionExecution {
                id: action.id,
                action_type: action.action_type,
                parameters,
                start_time: crate::time::get_timestamp(),
                end_time: Some(action.execution_time),
                status: if action.success { RecoveryActionStatus::Completed } else { RecoveryActionStatus::Failed },
                result,
            };
            recovery_manager.execute_recovery_action(&execution)?;
        }

        // 自动恢复策略
        if !self.config.auto_recovery_strategies.is_empty() {
            for strategy in &self.config.auto_recovery_strategies {
                recovery_manager.apply_recovery_strategy(strategy, error_record)?;
            }
        }

        Ok(())
    }

    /// 获取错误记录
    pub fn get_error_records(&self, limit: Option<usize>, category: Option<ErrorCategory>, severity: Option<ErrorSeverity>) -> Vec<ErrorRecord> {
        let mut records = self.error_records.clone();

        // 按类别过滤
        if let Some(cat) = category {
            records.retain(|r| r.category == cat);
        }

        // 按严重级别过滤
        if let Some(sev) = severity {
            records.retain(|r| r.severity == sev);
        }

        // 按时间排序（最新的在前）
        records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

        // 限制数量
        if let Some(limit) = limit {
            records.truncate(limit);
        }

        records
    }

    /// 获取错误统计信息
    pub fn get_statistics(&self) -> ErrorHandlingStats {
        self.stats.lock().clone()
    }

    /// 获取系统健康状态
    pub fn get_system_health(&self) -> system_health::SystemHealthStatus {
        let health_monitor = self.system_health.lock();
        health_monitor.get_current_status()
    }

    /// 更新配置
    pub fn update_config(&mut self, config: ErrorHandlingConfig) -> Result<(), &'static str> {
        self.config = config;
        Ok(())
    }

    /// 执行健康检查
    pub fn perform_health_check(&self) -> Result<Vec<system_health::HealthCheckResult>, &'static str> {
        let mut health_monitor = self.system_health.lock();
        health_monitor.perform_health_check()
    }

    /// 生成错误报告
    pub fn generate_error_report(&self, time_range: Option<(u64, u64)>) -> Result<String, &'static str> {
        let mut reporter = self.error_reporter.lock();
        reporter.generate_report(&self.error_records, time_range)
    }

    /// 更新统计信息
    fn update_statistics(&self, error_record: &ErrorRecord) {
        let mut stats = self.stats.lock();

        stats.total_errors += 1;

        *stats.errors_by_category.entry(error_record.category).or_insert(0) += 1;
        *stats.errors_by_severity.entry(error_record.severity).or_insert(0) += 1;

        if error_record.resolved {
            stats.recovered_errors += 1;
        }
    }

    /// 停止错误处理引擎
    pub fn shutdown(&mut self) -> Result<(), &'static str> {
        self.running.store(false, Ordering::SeqCst);

        // 停止各个组件
        self.error_registry.lock().shutdown()?;
        self.error_classifier.lock().shutdown()?;
        self.recovery_manager.lock().shutdown()?;
        self.diagnostic_analyzer.lock().shutdown()?;
        self.error_reporter.lock().shutdown()?;
        self.error_tracer.lock().shutdown()?;
        self.system_health.lock().shutdown()?;
        self.fault_tolerance.lock().shutdown()?;
        self.diagnostic_tools.lock().shutdown()?;

        crate::println!("[ErrorHandling] Error handling engine shutdown successfully");
        Ok(())
    }
}

/// 全局错误处理引擎实例
pub static ERROR_HANDLING_ENGINE: Once<spin::Mutex<ErrorHandlingEngine>> = Once::new();

/// Initialize the error handling engine
pub fn init_global_error_handling() {
    ERROR_HANDLING_ENGINE.call_once(||
        spin::Mutex::new(ErrorHandlingEngine::new(ErrorHandlingConfig::default()))
    );
}

/// Get the global error handling engine instance
pub fn get_error_handling_engine() -> &'static spin::Mutex<ErrorHandlingEngine> {
    ERROR_HANDLING_ENGINE.call_once(||
        spin::Mutex::new(ErrorHandlingEngine::new(ErrorHandlingConfig::default()))
    );
    ERROR_HANDLING_ENGINE.get().unwrap()
}

/// 初始化错误处理系统
pub fn init_error_handling() -> Result<(), &'static str> {
    let config = ErrorHandlingConfig::default();
    let mut engine = get_error_handling_engine().lock();
    engine.update_config(config)?;
    engine.init()
}

/// 记录错误
pub fn record_error(error_code: u32, error_type: ErrorType, category: ErrorCategory, severity: ErrorSeverity, message: &str, source: &ErrorSource) -> Result<u64, &'static str> {
    let error_record = ErrorRecord {
        id: 0, // 将在record_error中分配
        code: error_code,
        error_type,
        category,
        severity,
        status: ErrorStatus::New,
        message: message.to_string(),
        description: String::new(),
        source: source.clone(),
        timestamp: crate::time::get_timestamp(),
        context: ErrorContext {
            environment_variables: BTreeMap::new(),
            system_config: BTreeMap::new(),
            user_input: None,
            related_data: Vec::new(),
            operation_sequence: Vec::new(),
            preconditions: Vec::new(),
            postconditions: Vec::new(),
        },
        stack_trace: Vec::new(),
        system_state: SystemStateSnapshot {
            memory_usage: MemoryUsage {
                total_memory: 0,
                used_memory: 0,
                available_memory: 0,
                cached_memory: 0,
                swap_used: 0,
                kernel_memory: 0,
            },
            cpu_usage: CpuUsage {
                usage_percent: 0.0,
                user_percent: 0.0,
                system_percent: 0.0,
                idle_percent: 0.0,
                wait_percent: 0.0,
                interrupt_percent: 0.0,
            },
            process_states: Vec::new(),
            network_state: NetworkState {
                active_connections: 0,
                listening_ports: 0,
                interfaces: Vec::new(),
                packet_stats: PacketStats {
                    total_rx: 0,
                    total_tx: 0,
                    dropped: 0,
                    errors: 0,
                },
            },
            filesystem_state: FileSystemState {
                mount_points: Vec::new(),
                disk_usage: Vec::new(),
                io_stats: IoStats {
                    read_operations: 0,
                    write_operations: 0,
                    read_bytes: 0,
                    write_bytes: 0,
                    io_wait_time: 0,
                },
            },
            device_states: Vec::new(),
            system_load: SystemLoad {
                load_1min: 0.0,
                load_5min: 0.0,
                load_15min: 0.0,
                run_queue_length: 0,
                blocked_processes: 0,
            },
            timestamp: crate::time::get_timestamp(),
        },
        recovery_actions: Vec::new(),
        occurrence_count: 1,
        last_occurrence: crate::time::get_timestamp(),
        resolved: false,
        resolution_time: None,
        resolution_method: None,
        metadata: BTreeMap::new(),
    };

    get_error_handling_engine().lock().record_error(error_record)
}

/// 获取错误统计信息
pub fn get_error_statistics() -> ErrorHandlingStats {
    get_error_handling_engine().lock().get_statistics()
}

/// 获取系统健康状态
pub fn get_system_health() -> system_health::SystemHealthStatus {
    get_error_handling_engine().lock().get_system_health()
}

/// 执行健康检查
pub fn perform_health_check() -> Result<Vec<system_health::HealthCheckResult>, &'static str> {
    get_error_handling_engine().lock().perform_health_check()
}

/// 生成错误报告
pub fn generate_error_report(time_range: Option<(u64, u64)>) -> Result<String, &'static str> {
    get_error_handling_engine().lock().generate_error_report(time_range)
}

/// 停止错误处理系统
pub fn shutdown_error_handling() -> Result<(), &'static str> {
    get_error_handling_engine().lock().shutdown()
}

// Re-export unified error handling types and functions
pub use unified_error::{
    UnifiedError, ErrorPriority,
    ErrorMetadata, ErrorSignature, EnhancedErrorContext
};

pub use unified_engine::{
    UnifiedErrorHandlingEngine, ErrorAggregator, ErrorMonitor,
    PerformanceMetrics
};

pub use error_prediction::{
    ErrorPredictor, ErrorPattern, ErrorPrediction, PreventionAction,
    PatternCondition, ComparisonOperator, ProcessConditionType,
    ResourceType, PreventionActionType, ExecutionCost,
    SystemStateSnapshot as PredictionSystemStateSnapshot, NetworkIoStats, PredictionConfig, PredictionStats
};

pub use self_healing::{
    SelfHealingSystem, SelfHealingStrategy, SelfHealingAction,
    HealingTriggerCondition, MetricComparison, HealingPriority,
    HealingCost, HealingExecution, HealingResult, HealingExecutionStrategy,
    HealingConfig, HealingStats, SelfHealingActionType
};

pub use error_handling_traits::{
    ErrorHandler, ErrorRecoverer, ErrorDiagnoser, ErrorPredictorTrait, ErrorListener,
    ErrorHandlingResult, HandlerStatistics, RecoveryResult as TraitsRecoveryResult, RecoveryStrategy as TraitsRecoveryStrategy,
    RecoveryPriority, RecoveryAction as TraitsRecoveryAction, RecoveryActionType as TraitsRecoveryActionType,
    DiagnosisResult, ImpactScope, DiagnosisDepth,
    UnifiedErrorHandlingManager, ManagerStatistics, HealthCheckResult as TraitsHealthCheckResult
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_severity_ordering() {
        assert!(ErrorSeverity::Info < ErrorSeverity::Warning);
        assert!(ErrorSeverity::Warning < ErrorSeverity::Error);
        assert!(ErrorSeverity::Error < ErrorSeverity::Critical);
        assert!(ErrorSeverity::Critical < ErrorSeverity::Fatal);
    }

    #[test]
    fn test_error_category() {
        assert_ne!(ErrorCategory::System, ErrorCategory::Memory);
        assert_eq!(ErrorCategory::Network, ErrorCategory::Network);
    }

    #[test]
    fn test_error_config_default() {
        let config = ErrorHandlingConfig::default();
        assert!(config.enable_recovery);
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.retry_interval_ms, 1000);
    }

    #[test]
    fn test_error_record_creation() {
        let source = ErrorSource {
            module: "test".to_string(),
            function: "test_func".to_string(),
            file: "test.rs".to_string(),
            line: 10,
            column: 5,
            process_id: 1,
            thread_id: 1,
            cpu_id: 0,
        };

        let record = ErrorRecord {
            id: 1,
            code: 1001,
            error_type: ErrorType::RuntimeError,
            category: ErrorCategory::System,
            severity: ErrorSeverity::Error,
            status: ErrorStatus::New,
            message: "Test error".to_string(),
            description: "Test error description".to_string(),
            source: source.clone(),
            timestamp: 0,
            context: ErrorContext {
                environment_variables: BTreeMap::new(),
                system_config: BTreeMap::new(),
                user_input: None,
                related_data: Vec::new(),
                operation_sequence: Vec::new(),
                preconditions: Vec::new(),
                postconditions: Vec::new(),
            },
            stack_trace: Vec::new(),
            system_state: SystemStateSnapshot {
                memory_usage: MemoryUsage {
                    total_memory: 0,
                    used_memory: 0,
                    available_memory: 0,
                    cached_memory: 0,
                    swap_used: 0,
                    kernel_memory: 0,
                },
                cpu_usage: CpuUsage {
                    usage_percent: 0.0,
                    user_percent: 0.0,
                    system_percent: 0.0,
                    idle_percent: 0.0,
                    wait_percent: 0.0,
                    interrupt_percent: 0.0,
                },
                process_states: Vec::new(),
                network_state: NetworkState {
                    active_connections: 0,
                    listening_ports: 0,
                    interfaces: Vec::new(),
                    packet_stats: PacketStats {
                        total_rx: 0,
                        total_tx: 0,
                        dropped: 0,
                        errors: 0,
                    },
                },
                filesystem_state: FileSystemState {
                    mount_points: Vec::new(),
                    disk_usage: Vec::new(),
                    io_stats: IoStats {
                        read_operations: 0,
                        write_operations: 0,
                        read_bytes: 0,
                        write_bytes: 0,
                        io_wait_time: 0,
                    },
                },
                device_states: Vec::new(),
                system_load: SystemLoad {
                    load_1min: 0.0,
                    load_5min: 0.0,
                    load_15min: 0.0,
                    run_queue_length: 0,
                    blocked_processes: 0,
                },
                timestamp: 0,
            },
            recovery_actions: Vec::new(),
            occurrence_count: 1,
            last_occurrence: 0,
            resolved: false,
            resolution_time: None,
            resolution_method: None,
            metadata: BTreeMap::new(),
        };

        assert_eq!(record.id, 1);
        assert_eq!(record.severity, ErrorSeverity::Error);
        assert_eq!(record.source.module, "test");
    }
}
