// Debug Types Module
//
// 调试类型模块
// 提供符号管理、调试配置等通用类型

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::string::String;
use crate::debug::session::DebugLevel;

/// 调试符号管理器
#[derive(Debug, Clone)]
pub struct SymbolManager {
    /// 管理器ID
    pub id: u64,
    /// 符号表
    pub symbol_tables: BTreeMap<String, SymbolTable>,
    /// 调试信息
    pub debug_info: DebugInfo,
    /// 符号缓存
    pub symbol_cache: BTreeMap<u64, Symbol>,
    /// 源文件映射
    pub source_mappings: BTreeMap<String, SourceMapping>,
}

impl SymbolManager {
    /// 初始化符号管理器
    pub fn init(&mut self) -> Result<(), &'static str> {
        // 清空符号表
        self.symbol_tables.clear();
        // 重置调试信息
        self.debug_info.debug_format = DebugFormat::ELF;
        self.debug_info.debug_level = DebugLevel::Info;
        self.debug_info.enabled_features.clear();
        self.debug_info.output_buffer.clear();
        // 清空符号缓存
        self.symbol_cache.clear();
        // 清空源文件映射
        self.source_mappings.clear();
        Ok(())
    }
}

/// 符号表
#[derive(Debug, Clone)]
pub struct SymbolTable {
    /// 表ID
    pub id: String,
    /// 表名称
    pub name: String,
    /// 表类型
    pub table_type: SymbolTableType,
    /// 符号列表
    pub symbols: BTreeMap<String, Symbol>,
    /// 符号统计
    pub symbol_stats: SymbolTableStats,
}

/// 符号表类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolTableType {
    /// 动态符号
    Dynamic,
    /// 静态符号
    Static,
    /// 调试符号
    Debug,
    /// 导出符号
    Exported,
    /// 内部符号
    Internal,
}

/// 符号
#[derive(Debug, Clone)]
pub struct Symbol {
    /// 符号名
    pub name: String,
    /// 符号地址
    pub address: u64,
    /// 符号大小
    pub size: u64,
    /// 符号类型
    pub symbol_type: SymbolType,
    /// 符号作用域
    pub scope: SymbolScope,
    /// 源文件
    pub source_file: Option<String>,
    /// 源行号
    pub source_line: Option<u32>,
    /// 符号描述
    pub description: Option<String>,
}

/// 符号类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolType {
    /// 函数
    Function,
    /// 变量
    Variable,
    /// 类型
    Type,
    /// 常量
    Constant,
    /// 标签
    Label,
    /// 宏
    Macro,
}

/// 符号作用域
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolScope {
    /// 全局
    Global,
    /// 局部
    Local,
    /// 静态
    Static,
    /// 外部
    External,
}

/// 符号表统计
#[derive(Debug, Clone, Default)]
pub struct SymbolTableStats {
    /// 符号数量
    pub symbol_count: u64,
    /// 函数符号数量
    pub function_symbols: u64,
    /// 变量符号数量
    pub variable_symbols: u64,
    /// 类型符号数量
    pub type_symbols: u64,
    /// 总大小
    pub total_size: u64,
    /// 平均符号大小
    pub avg_symbol_size: f64,
    /// 唯一符号名称长度
    pub avg_symbol_name_length: f64,
}

/// 调试信息
#[derive(Debug, Clone)]
pub struct DebugInfo {
    /// 调试信息格式
    pub debug_format: DebugFormat,
    /// 调试信息级别
    pub debug_level: DebugLevel,
    /// 启用的调试特性
    pub enabled_features: Vec<DebugFeature>,
    /// 调试输出缓冲区
    pub output_buffer: Vec<String>,
}

/// 调试格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugFormat {
    /// DWARF
    DWARF,
    /// STABS
    STABS,
    /// PDB
    PDB,
    /// PE
    PE,
    /// ELF
    ELF,
    /// 自定义格式
    Custom,
}

/// 调试特性
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DebugFeature {
    /// 变量跟踪
    VariableTracking,
    /// 函数跟踪
    FunctionTracing,
    /// 内存跟踪
    MemoryTracking,
    /// 系统调用跟踪
    SystemCallTracing,
    /// 异常跟踪
    ExceptionTracing,
    /// 性能监控
    PerformanceMonitoring,
    /// 网络监控
    NetworkMonitoring,
    /// 自定义跟踪
    CustomTracing,
}

/// 源文件映射
#[derive(Debug, Clone)]
pub struct SourceMapping {
    /// 映射ID
    pub id: String,
    /// 源文件路径
    pub source_file: String,
    /// 目标地址
    pub target_address: u64,
    /// 映射大小
    pub mapping_size: u64,
    /// 行号映射表
    pub line_mappings: Vec<LineMapping>,
}

/// 行号映射
#[derive(Debug, Clone)]
pub struct LineMapping {
    /// 源行号
    pub source_line: u32,
    /// 目标行号
    pub target_line: u32,
    /// 地址偏移
    pub address_offset: u64,
    /// 列偏移
    pub column_offset: u32,
}

/// 调试配置
#[derive(Debug, Clone)]
pub struct DebugConfig {
    /// 启用自动调试
    pub enable_auto_debug: bool,
    /// 最大并发调试会话数
    pub max_concurrent_sessions: u32,
    /// 默认调试级别
    pub default_debug_level: DebugLevel,
    /// 调试历史保留数量
    pub debug_history_size: usize,
    /// 启用性能分析
    pub enable_performance_analysis: bool,
    /// 启用内存分析
    pub enable_memory_analysis: bool,
    /// 启用符号解析
    pub enable_symbol_resolution: bool,
    /// 调试输出缓冲区大小
    pub debug_buffer_size: usize,
    /// 调试输出文件路径
    pub debug_output_file: Option<String>,
    /// 启用实时监控
    pub enable_real_time_monitoring: bool,
    /// 监控间隔（毫秒）
    pub monitoring_interval_ms: u64,
}

impl Default for DebugConfig {
    fn default() -> Self {
        Self {
            enable_auto_debug: false,
            max_concurrent_sessions: 5,
            default_debug_level: DebugLevel::Info,
            debug_history_size: 1000,
            enable_performance_analysis: true,
            enable_memory_analysis: true,
            enable_symbol_resolution: true,
            debug_buffer_size: 1024 * 1024, // 1MB
            debug_output_file: None,
            enable_real_time_monitoring: false,
            monitoring_interval_ms: 1000,
        }
    }
}

/// 调试统计信息
#[derive(Debug, Clone)]
pub struct DebugStats {
    /// 总会话数
    pub total_sessions: u64,
    /// 成功会话数
    pub successful_sessions: u64,
    /// 失败会话数
    pub failed_sessions: u64,
    /// 平均会话持续时间
    pub avg_session_duration: u64,
    /// 设置的断点数
    pub breakpoints_set: u64,
    /// 命中的断点数
    pub breakpoints_hit: u64,
    /// 内存快照数量
    pub memory_snapshots_taken: u64,
    /// 性能报告生成数量
    pub performance_reports_generated: u64,
    /// 检测到的泄漏数
    pub leaks_detected: u64,
    /// 解析的符号数
    pub symbols_resolved: u64,
    /// 分析的字节数
    pub bytes_analyzed: u64,
    /// 活动插件数
    pub active_plugins: u64,
    /// 最常用的特性
    pub most_used_features: Vec<String>,
    /// 按类型分组的会话
    pub sessions_by_type: BTreeMap<String, u64>,
    /// 按严重程度分组的错误
    pub errors_by_severity: BTreeMap<String, u64>,
}

impl Default for DebugStats {
    fn default() -> Self {
        Self {
            total_sessions: 0,
            successful_sessions: 0,
            failed_sessions: 0,
            avg_session_duration: 0,
            breakpoints_set: 0,
            breakpoints_hit: 0,
            memory_snapshots_taken: 0,
            performance_reports_generated: 0,
            leaks_detected: 0,
            symbols_resolved: 0,
            bytes_analyzed: 0,
            active_plugins: 0,
            most_used_features: Vec::new(),
            sessions_by_type: BTreeMap::new(),
            errors_by_severity: BTreeMap::new(),
        }
    }
}
