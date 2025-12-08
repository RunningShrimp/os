// 系统跟踪模块

extern crate alloc;
//
// 提供全面的系统跟踪功能，包括事件跟踪、执行流跟踪、
// 系统调用跟踪和分布式跟踪支持。
//
// 主要功能：
// - 系统事件跟踪
// - 执行流程跟踪
// - 系统调用跟踪
// - 分布式跟踪支持
// - 跟踪数据导出
// - 性能分析集成
// - 实时跟踪监控

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

/// 跟踪级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TraceLevel {
    /// 调试级别
    Debug = 0,
    /// 信息级别
    Info = 1,
    /// 警告级别
    Warning = 2,
    /// 错误级别
    Error = 3,
    /// 关键级别
    Critical = 4,
}

/// 跟踪事件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraceEventType {
    /// 函数开始
    FunctionStart,
    /// 函数结束
    FunctionEnd,
    /// 系统调用
    SystemCall,
    /// 中断
    Interrupt,
    /// 上下文切换
    ContextSwitch,
    /// 内存分配
    MemoryAllocation,
    /// 内存释放
    MemoryFree,
    /// IO操作
    IoOperation,
    /// 网络事件
    NetworkEvent,
    /// 自定义事件
    Custom,
}

/// 跟踪事件
#[derive(Debug, Clone)]
pub struct TraceEvent {
    /// 事件ID
    pub id: u64,
    /// 父事件ID
    pub parent_id: Option<u64>,
    /// 跟踪ID
    pub trace_id: String,
    /// 事件类型
    pub event_type: TraceEventType,
    /// 跟踪级别
    pub level: TraceLevel,
    /// 时间戳（纳秒）
    pub timestamp: u64,
    /// 持续时间（纳秒，可选）
    pub duration: Option<u64>,
    /// 线程ID
    pub thread_id: u64,
    /// 进程ID
    pub process_id: u64,
    /// 组件名称
    pub component: String,
    /// 事件名称
    pub name: String,
    /// 消息
    pub message: Option<String>,
    /// 标签
    pub tags: BTreeMap<String, String>,
    /// 数据
    pub data: BTreeMap<String, String>,
    /// 栈帧
    pub stack_frames: Vec<StackFrame>,
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

/// 跟踪段（Span）
#[derive(Debug, Clone)]
pub struct TraceSpan {
    /// 段ID
    pub id: String,
    /// 父段ID
    pub parent_id: Option<String>,
    /// 跟踪ID
    pub trace_id: String,
    /// 操作名称
    pub operation_name: String,
    /// 开始时间
    pub start_time: u64,
    /// 结束时间
    pub end_time: Option<u64>,
    /// 服务名称
    pub service_name: String,
    /// 资源名称
    pub resource_name: Option<String>,
    /// 标签
    pub tags: BTreeMap<String, String>,
    /// 日志
    pub logs: Vec<TraceLog>,
    /// 状态
    pub status: SpanStatus,
}

/// 段状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpanStatus {
    /// 正常
    Ok,
    /// 错误
    Error,
    /// 超时
    Timeout,
    /// 取消
    Canceled,
}

/// 跟踪日志
#[derive(Debug, Clone)]
pub struct TraceLog {
    /// 时间戳
    pub timestamp: u64,
    /// 级别
    pub level: TraceLevel,
    /// 消息
    pub message: String,
    /// 字段
    pub fields: BTreeMap<String, String>,
}

/// 跟踪配置
#[derive(Debug, Clone)]
pub struct TraceConfig {
    /// 是否启用跟踪
    pub enabled: bool,
    /// 最大事件数量
    pub max_events: usize,
    /// 最大段数量
    pub max_spans: usize,
    /// 跟踪级别过滤器
    pub level_filter: TraceLevel,
    /// 是否启用栈帧收集
    pub enable_stack_trace: bool,
    /// 最大栈深度
    pub max_stack_depth: usize,
    /// 事件保留期（秒）
    pub retention_period: Duration,
    /// 是否启用压缩
    pub enable_compression: bool,
    /// 批处理大小
    pub batch_size: usize,
    /// 导出间隔（秒）
    pub export_interval: Duration,
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_events: 1000000,
            max_spans: 100000,
            level_filter: TraceLevel::Info,
            enable_stack_trace: true,
            max_stack_depth: 16,
            retention_period: Duration::from_secs(24 * 60 * 60), // 24小时
            enable_compression: false,
            batch_size: 1000,
            export_interval: Duration::from_secs(60),
        }
    }
}

/// 跟踪统计信息
#[derive(Debug, Default)]
pub struct TraceStatistics {
    /// 总事件数
    pub total_events: AtomicU64,
    /// 总段数
    pub total_spans: AtomicU64,
    /// 丢弃的事件数
    pub dropped_events: AtomicU64,
    /// 当前活动段数
    pub active_spans: AtomicUsize,
    /// 存储大小（字节）
    pub storage_size: AtomicU64,
    /// 导出的事件数
    pub exported_events: AtomicU64,
    /// 平均延迟（纳秒）
    pub average_latency: AtomicU64,
}

/// 跟踪器
pub struct Tracer {
    /// 跟踪ID
    pub trace_id: String,
    /// 配置
    config: TraceConfig,
    /// 事件存储
    events: Arc<Mutex<Vec<TraceEvent>>>,
    /// 段存储
    spans: Arc<Mutex<BTreeMap<String, TraceSpan>>>,
    /// 活动段栈
    active_span_stack: Arc<Mutex<Vec<String>>>,
    /// 统计信息
    statistics: TraceStatistics,
    /// 下一个事件ID
    next_event_id: AtomicU64,
    /// 是否已启用
    enabled: core::sync::atomic::AtomicBool,
}

/// 跟踪引擎
pub struct TraceEngine {
    /// 配置
    config: TraceConfig,
    /// 跟踪器实例
    tracers: Arc<Mutex<BTreeMap<String, Arc<Tracer>>>>,
    /// 全局事件存储
    global_events: Arc<Mutex<Vec<TraceEvent>>>,
    /// 事件导出器
    exporters: Arc<Mutex<Vec<Box<dyn TraceExporter>>>>,
    /// 统计信息
    statistics: TraceStatistics,
}

/// 跟踪数据导出器接口
pub trait TraceExporter: Send + Sync {
    /// 导出事件
    fn export_events(&self, events: &[TraceEvent]) -> Result<(), TraceError>;
    /// 导出段
    fn export_spans(&self, spans: &[TraceSpan]) -> Result<(), TraceError>;
    /// 获取导出器名称
    fn name(&self) -> &str;
}

/// 控制台导出器
pub struct ConsoleExporter {
    /// 是否启用颜色输出
    enable_colors: bool,
}

/// 文件导出器
pub struct FileExporter {
    /// 输出文件路径
    file_path: String,
    /// 文件格式
    format: ExportFormat,
}

/// 导出格式
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    /// JSON格式
    Json,
    /// CSV格式
    Csv,
    /// 二进制格式
    Binary,
}

impl Tracer {
    /// 创建新的跟踪器
    pub fn new(trace_id: String, config: TraceConfig) -> Self {
        let enabled_flag = core::sync::atomic::AtomicBool::new(config.enabled.clone());

        Self {
            trace_id,
            config,
            events: Arc::new(Mutex::new(Vec::new())),
            spans: Arc::new(Mutex::new(BTreeMap::new())),
            active_span_stack: Arc::new(Mutex::new(Vec::new())),
            statistics: TraceStatistics::default(),
            next_event_id: AtomicU64::new(1),
            enabled: enabled_flag,
        }
    }

    /// 启用跟踪
    pub fn enable(&self) {
        self.enabled.store(true, Ordering::SeqCst);
    }

    /// 禁用跟踪
    pub fn disable(&self) {
        self.enabled.store(false, Ordering::SeqCst);
    }

    /// 记录事件
    pub fn record_event(&self, mut event: TraceEvent) -> Result<(), TraceError> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Ok(());
        }

        // 检查级别过滤器
        if event.level < self.config.level_filter {
            return Ok(());
        }

        // 设置事件ID和跟踪ID
        event.id = self.next_event_id.fetch_add(1, Ordering::SeqCst);
        event.trace_id = self.trace_id.clone();

        // 收集栈帧（如果启用）
        if self.config.enable_stack_trace {
            event.stack_frames = self.collect_stack_frames();
        }

        // 存储事件
        let mut events = self.events.lock();
        events.push(event.clone());

        // 限制事件数量
        if events.len() > self.config.max_events {
            events.remove(0);
            self.statistics.dropped_events.fetch_add(1, Ordering::SeqCst);
        }

        // 更新统计
        self.statistics.total_events.fetch_add(1, Ordering::SeqCst);

        Ok(())
    }

    /// 开始段
    pub fn start_span(&self, operation_name: String, parent_id: Option<String>) -> Result<String, TraceError> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Ok(String::new());
        }

        let span_id = self.generate_span_id();
        let current_time = time::timestamp_nanos();

        // 获取父段ID
        let parent_id = if parent_id.is_some() {
            parent_id
        } else {
            let stack = self.active_span_stack.lock();
            stack.last().cloned()
        };

        // 创建段
        let span = TraceSpan {
            id: span_id.clone(),
            parent_id,
            trace_id: self.trace_id.clone(),
            operation_name: operation_name.clone(),
            start_time: current_time,
            end_time: None,
            service_name: "kernel".to_string(),
            resource_name: None,
            tags: BTreeMap::new(),
            logs: Vec::new(),
            status: SpanStatus::Ok,
        };

        // 存储段
        let mut spans = self.spans.lock();
        spans.insert(span_id.clone(), span.clone());
        self.statistics.total_spans.fetch_add(1, Ordering::SeqCst);
        self.statistics.active_spans.store(spans.len(), Ordering::SeqCst);

        // 添加到活动段栈
        let mut stack = self.active_span_stack.lock();
        stack.push(span_id.clone());

        // 记录段开始事件
        self.record_event(TraceEvent {
            id: 0, // 将在record_event中设置
            parent_id: None,
            trace_id: String::new(), // 将在record_event中设置
            event_type: TraceEventType::FunctionStart,
            level: TraceLevel::Info,
            timestamp: current_time,
            duration: None,
            thread_id: self.get_current_thread_id(),
            process_id: self.get_current_process_id(),
            component: "tracer".to_string(),
            name: operation_name,
            message: Some(format!("开始段: {}", span_id)),
            tags: BTreeMap::new(),
            data: {
                let mut data = BTreeMap::new();
                data.insert("span_id".to_string(), span_id.clone());
                data
            },
            stack_frames: Vec::new(),
        })?;

        Ok(span_id)
    }

    /// 结束段
    pub fn end_span(&self, span_id: &str) -> Result<(), TraceError> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Ok(());
        }

        let current_time = time::timestamp_nanos();

        // 更新段
        let mut spans = self.spans.lock();
        if let Some(span) = spans.get_mut(span_id) {
            span.end_time = Some(current_time);
            span.status = SpanStatus::Ok;
        }

        // 从活动段栈移除
        let mut stack = self.active_span_stack.lock();
        if let Some(index) = stack.iter().position(|id| id == span_id) {
            stack.remove(index);
        }

        self.statistics.active_spans.store(spans.len(), Ordering::SeqCst);

        // 记录段结束事件
        self.record_event(TraceEvent {
            id: 0,
            parent_id: None,
            trace_id: String::new(),
            event_type: TraceEventType::FunctionEnd,
            level: TraceLevel::Info,
            timestamp: current_time,
            duration: None,
            thread_id: self.get_current_thread_id(),
            process_id: self.get_current_process_id(),
            component: "tracer".to_string(),
            name: "段结束".to_string(),
            message: Some(format!("结束段: {}", span_id)),
            tags: BTreeMap::new(),
            data: {
                let mut data = BTreeMap::new();
                data.insert("span_id".to_string(), span_id.to_string());
                data
            },
            stack_frames: Vec::new(),
        })?;

        Ok(())
    }

    /// 记录日志
    pub fn log(&self, span_id: Option<&str>, level: TraceLevel, message: String) -> Result<(), TraceError> {
        if !self.enabled.load(Ordering::SeqCst) {
            return Ok(());
        }

        let current_time = time::timestamp_nanos();

        // 记录到段（如果提供了span_id）
        if let Some(span_id) = span_id {
            let mut spans = self.spans.lock();
            if let Some(span) = spans.get_mut(span_id) {
                span.logs.push(TraceLog {
                    timestamp: current_time,
                    level,
                    message: message.clone(),
                    fields: BTreeMap::new(),
                });
            }
        }

        // 记录为事件
        self.record_event(TraceEvent {
            id: 0,
            parent_id: None,
            trace_id: String::new(),
            event_type: TraceEventType::Custom,
            level,
            timestamp: current_time,
            duration: None,
            thread_id: self.get_current_thread_id(),
            process_id: self.get_current_process_id(),
            component: "tracer".to_string(),
            name: "日志".to_string(),
            message: Some(message),
            tags: BTreeMap::new(),
            data: BTreeMap::new(),
            stack_frames: Vec::new(),
        })?;

        Ok(())
    }

    /// 获取事件
    pub fn get_events(&self, start_time: Option<u64>, end_time: Option<u64>) -> Result<Vec<TraceEvent>, TraceError> {
        let events = self.events.lock();

        let filtered_events: Vec<TraceEvent> = events
            .iter()
            .filter(|e| {
                let time_match = match (start_time, end_time) {
                    (Some(start), Some(end)) => e.timestamp >= start && e.timestamp <= end,
                    (Some(start), None) => e.timestamp >= start,
                    (None, Some(end)) => e.timestamp <= end,
                    (None, None) => true,
                };
                time_match
            })
            .cloned()
            .collect();

        Ok(filtered_events)
    }

    /// 获取段
    pub fn get_spans(&self) -> Result<Vec<TraceSpan>, TraceError> {
        let spans = self.spans.lock();
        Ok(spans.values().cloned().collect())
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> TraceStatistics {
        TraceStatistics {
            total_events: AtomicU64::new(self.statistics.total_events.load(Ordering::SeqCst)),
            total_spans: AtomicU64::new(self.statistics.total_spans.load(Ordering::SeqCst)),
            dropped_events: AtomicU64::new(self.statistics.dropped_events.load(Ordering::SeqCst)),
            active_spans: AtomicUsize::new(self.statistics.active_spans.load(Ordering::SeqCst)),
            storage_size: AtomicU64::new(self.statistics.storage_size.load(Ordering::SeqCst)),
            exported_events: AtomicU64::new(self.statistics.exported_events.load(Ordering::SeqCst)),
            average_latency: AtomicU64::new(self.statistics.average_latency.load(Ordering::SeqCst)),
        }
    }

    /// 清理过期数据
    pub fn cleanup_expired_data(&self) -> Result<(), TraceError> {
        let cutoff_time = time::timestamp_nanos() - self.config.retention_period.as_nanos() as u64;

        // 清理过期事件
        let mut events = self.events.lock();
        events.retain(|e| e.timestamp >= cutoff_time);

        // 清理过期段
        let mut spans = self.spans.lock();
        spans.retain(|_, s| s.start_time >= cutoff_time);

        // 更新统计
        self.statistics.active_spans.store(spans.len(), Ordering::SeqCst);

        Ok(())
    }

    /// 辅助方法
    fn collect_stack_frames(&self) -> Vec<StackFrame> {
        // 简单实现，实际应该从调用栈获取
        Vec::new()
    }

    fn get_current_thread_id(&self) -> u64 {
        // 简单实现，实际应该从线程管理器获取
        1
    }

    fn get_current_process_id(&self) -> u64 {
        // 简单实现，实际应该从进程管理器获取
        1
    }

    fn generate_span_id(&self) -> String {
        static NEXT_SPAN_ID: AtomicU64 = AtomicU64::new(1);
        let id = NEXT_SPAN_ID.fetch_add(1, Ordering::SeqCst);
        format!("span_{}", id)
    }
}

impl TraceEngine {
    /// 创建新的跟踪引擎
    pub fn new(config: TraceConfig) -> Self {
        Self {
            config,
            tracers: Arc::new(Mutex::new(BTreeMap::new())),
            global_events: Arc::new(Mutex::new(Vec::new())),
            exporters: Arc::new(Mutex::new(Vec::new())),
            statistics: TraceStatistics::default(),
        }
    }

    /// 创建跟踪器
    pub fn create_tracer(&self, trace_id: String) -> Result<Arc<Tracer>, TraceError> {
        let tracer = Arc::new(Tracer::new(trace_id.clone(), self.config.clone()));

        let mut tracers = self.tracers.lock();
        tracers.insert(trace_id.clone(), tracer.clone());

        Ok(tracer)
    }

    /// 获取跟踪器
    pub fn get_tracer(&self, trace_id: &str) -> Result<Arc<Tracer>, TraceError> {
        let tracers = self.tracers.lock();
        tracers.get(trace_id)
            .cloned()
            .ok_or(TraceError::TracerNotFound(trace_id.to_string()))
    }

    /// 添加导出器
    pub fn add_exporter(&self, exporter: Box<dyn TraceExporter>) -> Result<(), TraceError> {
        let mut exporters = self.exporters.lock();
        exporters.push(exporter);
        crate::println!("[trace] 添加导出器: {}", exporters.last().unwrap().name());
        Ok(())
    }

    /// 导出所有跟踪数据
    pub fn export_all(&self) -> Result<(), TraceError> {
        let exporters = self.exporters.lock();
        let tracers = self.tracers.lock();

        for exporter in exporters.iter() {
            // 导出每个跟踪器的事件和段
            for tracer in tracers.values() {
                let events = tracer.get_events(None, None)?;
                let spans = tracer.get_spans()?;

                if !events.is_empty() {
                    if let Err(e) = exporter.export_events(&events) {
                        crate::println!("[trace] 导出事件失败: {:?}", e);
                    }
                }

                if !spans.is_empty() {
                    if let Err(e) = exporter.export_spans(&spans) {
                        crate::println!("[trace] 导出段失败: {:?}", e);
                    }
                }
            }
        }

        Ok(())
    }

    /// 获取全局统计信息
    pub fn get_global_statistics(&self) -> TraceStatistics {
        TraceStatistics {
            total_events: AtomicU64::new(self.statistics.total_events.load(Ordering::SeqCst)),
            total_spans: AtomicU64::new(self.statistics.total_spans.load(Ordering::SeqCst)),
            dropped_events: AtomicU64::new(self.statistics.dropped_events.load(Ordering::SeqCst)),
            active_spans: AtomicUsize::new(self.statistics.active_spans.load(Ordering::SeqCst)),
            storage_size: AtomicU64::new(self.statistics.storage_size.load(Ordering::SeqCst)),
            exported_events: AtomicU64::new(self.statistics.exported_events.load(Ordering::SeqCst)),
            average_latency: AtomicU64::new(self.statistics.average_latency.load(Ordering::SeqCst)),
        }
    }

    /// 清理所有跟踪器中的过期数据
    pub fn cleanup_all_expired_data(&self) -> Result<(), TraceError> {
        let tracers = self.tracers.lock();

        for tracer in tracers.values() {
            tracer.cleanup_expired_data()?;
        }

        // 清理全局事件
        let cutoff_time = time::timestamp_nanos() - self.config.retention_period.as_nanos() as u64;
        let mut global_events = self.global_events.lock();
        global_events.retain(|e| e.timestamp >= cutoff_time);

        Ok(())
    }
}

impl ConsoleExporter {
    /// 创建新的控制台导出器
    pub fn new(enable_colors: bool) -> Self {
        Self { enable_colors }
    }
}

impl TraceExporter for ConsoleExporter {
    fn export_events(&self, events: &[TraceEvent]) -> Result<(), TraceError> {
        for event in events {
            let level_str = match event.level {
                TraceLevel::Debug => "DEBUG",
                TraceLevel::Info => "INFO",
                TraceLevel::Warning => "WARN",
                TraceLevel::Error => "ERROR",
                TraceLevel::Critical => "CRITICAL",
            };

            let message = event.message.as_deref().unwrap_or("");

            crate::println!("[{}] {} [{}:{}] {}",
                level_str,
                event.timestamp,
                event.component,
                event.name,
                message
            );
        }

        Ok(())
    }

    fn export_spans(&self, spans: &[TraceSpan]) -> Result<(), TraceError> {
        for span in spans {
            let duration = if let Some(end_time) = span.end_time {
                end_time - span.start_time
            } else {
                0
            };

            crate::println!("Span: {} -> {} ({}ns)",
                span.operation_name,
                span.service_name,
                duration
            );
        }

        Ok(())
    }

    fn name(&self) -> &str {
        "console"
    }
}

impl FileExporter {
    /// 创建新的文件导出器
    pub fn new(file_path: String, format: ExportFormat) -> Self {
        Self { file_path, format }
    }
}

impl TraceExporter for FileExporter {
    fn export_events(&self, events: &[TraceEvent]) -> Result<(), TraceError> {
        // 简化实现，实际应该写入文件
        crate::println!("[file_exporter] 导出 {} 个事件到 {}", events.len(), self.file_path);
        Ok(())
    }

    fn export_spans(&self, spans: &[TraceSpan]) -> Result<(), TraceError> {
        // 简化实现，实际应该写入文件
        crate::println!("[file_exporter] 导出 {} 个段到 {}", spans.len(), self.file_path);
        Ok(())
    }

    fn name(&self) -> &str {
        "file"
    }
}

/// 跟踪错误类型
#[derive(Debug, Clone)]
pub enum TraceError {
    /// 跟踪器不存在
    TracerNotFound(String),
    /// 配置错误
    ConfigurationError(String),
    /// 导出错误
    ExportError(String),
    /// 存储错误
    StorageError(String),
    /// 系统错误
    SystemError(String),
}

impl core::fmt::Display for TraceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            TraceError::TracerNotFound(id) => write!(f, "跟踪器不存在: {}", id),
            TraceError::ConfigurationError(msg) => write!(f, "配置错误: {}", msg),
            TraceError::ExportError(msg) => write!(f, "导出错误: {}", msg),
            TraceError::StorageError(msg) => write!(f, "存储错误: {}", msg),
            TraceError::SystemError(msg) => write!(f, "系统错误: {}", msg),
        }
    }
}

/// 全局跟踪引擎实例
static TRACE_ENGINE: spin::Mutex<Option<Arc<TraceEngine>>> = spin::Mutex::new(None);

/// 初始化跟踪子系统
pub fn init() -> Result<(), TraceError> {
    let config = TraceConfig::default();
    let engine = Arc::new(TraceEngine::new(config));

    // 添加默认导出器
    engine.add_exporter(Box::new(ConsoleExporter::new(true)))?;

    let mut global_engine = TRACE_ENGINE.lock();
    *global_engine = Some(engine);

    crate::println!("[trace] 跟踪子系统初始化完成");
    Ok(())
}

/// 获取全局跟踪引擎
pub fn get_trace_engine() -> Result<Arc<TraceEngine>, TraceError> {
    let engine = TRACE_ENGINE.lock();
    engine.as_ref()
        .cloned()
        .ok_or(TraceError::SystemError("跟踪引擎未初始化".to_string()))
}

/// 便捷宏
#[macro_export]
macro_rules! trace_event {
    ($level:expr, $name:expr, $message:expr) => {
        if let Ok(engine) = $crate::debug::tracing::get_trace_engine() {
            if let Ok(tracer) = engine.get_tracer("default") {
                let _ = tracer.record_event($crate::debug::tracing::TraceEvent {
                    id: 0,
                    parent_id: None,
                    trace_id: String::new(),
                    event_type: $crate::debug::tracing::TraceEventType::Custom,
                    level: $level,
                    timestamp: $crate::time::timestamp_nanos(),
                    duration: None,
                    thread_id: 1,
                    process_id: 1,
                    component: module_path!().to_string(),
                    name: $name.to_string(),
                    message: Some($message.to_string()),
                    tags: std::collections::BTreeMap::new(),
                    data: std::collections::BTreeMap::new(),
                    stack_frames: Vec::new(),
                });
            }
        }
    };
}

#[macro_export]
macro_rules! trace_info {
    ($name:expr, $message:expr) => {
        $crate::trace_event!($crate::debug::tracing::TraceLevel::Info, $name, $message);
    };
}

#[macro_export]
macro_rules! trace_warn {
    ($name:expr, $message:expr) => {
        $crate::trace_event!($crate::debug::tracing::TraceLevel::Warning, $name, $message);
    };
}

#[macro_export]
macro_rules! trace_error {
    ($name:expr, $message:expr) => {
        $crate::trace_event!($crate::debug::tracing::TraceLevel::Error, $name, $message);
    };
}
