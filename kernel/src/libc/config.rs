//! C标准库配置管理
//!
//! 提供灵活的配置系统，支持不同的C库实现类型和参数调整。
//! 允许在编译时和运行时配置C库行为。

use core::ffi::c_int;
use crate::libc::interface::{ImplementationType, CLibStats};
use core::cell::Cell;

/// C库配置结构
#[derive(Debug, Clone)]
pub struct LibcConfig {
    /// C库实现类型
    pub implementation: ImplementationType,
    /// 是否启用调试模式
    pub enable_debug: bool,
    /// 内存池大小（字节）
    pub memory_pool_size: usize,
    /// I/O缓冲区大小（字节）
    pub buffer_size: usize,
    /// 是否启用内存统计
    pub enable_memory_stats: bool,
    /// 是否启用函数调用统计
    pub enable_call_stats: bool,
    /// 是否启用错误检查
    pub enable_error_checking: bool,
    /// 最大文件描述符数量
    pub max_file_descriptors: usize,
    /// 是否启用线程安全
    pub enable_thread_safety: bool,
    /// 格式化输出精度
    pub printf_precision: u32,
    /// 是否启用严格模式
    pub strict_mode: bool,
}

impl Default for LibcConfig {
    fn default() -> Self {
        Self {
            implementation: ImplementationType::Simple,
            enable_debug: cfg!(debug_assertions),
            memory_pool_size: 1024 * 1024, // 1MB
            buffer_size: 4096,               // 4KB
            enable_memory_stats: true,
            enable_call_stats: true,
            enable_error_checking: true,
            max_file_descriptors: 256,
            enable_thread_safety: true,
            printf_precision: 6,
            strict_mode: false,
        }
    }
}

impl LibcConfig {
    /// 创建最小化配置
    pub fn minimal() -> Self {
        Self {
            implementation: ImplementationType::Minimal,
            enable_debug: false,
            memory_pool_size: 256 * 1024,  // 256KB
            buffer_size: 1024,              // 1KB
            enable_memory_stats: false,
            enable_call_stats: false,
            enable_error_checking: true,
            max_file_descriptors: 64,
            enable_thread_safety: false,
            printf_precision: 2,
            strict_mode: true,
        }
    }

    /// 创建简化配置
    pub fn simple() -> Self {
        Self {
            implementation: ImplementationType::Simple,
            enable_debug: cfg!(debug_assertions),
            memory_pool_size: 512 * 1024,  // 512KB
            buffer_size: 2048,              // 2KB
            enable_memory_stats: true,
            enable_call_stats: true,
            enable_error_checking: true,
            max_file_descriptors: 128,
            enable_thread_safety: true,
            printf_precision: 4,
            strict_mode: false,
        }
    }

    /// 创建完整配置
    pub fn full() -> Self {
        Self {
            implementation: ImplementationType::Unified,
            enable_debug: cfg!(debug_assertions),
            memory_pool_size: 2 * 1024 * 1024,  // 2MB
            buffer_size: 8192,                  // 8KB
            enable_memory_stats: true,
            enable_call_stats: true,
            enable_error_checking: true,
            max_file_descriptors: 1024,
            enable_thread_safety: true,
            printf_precision: 10,
            strict_mode: false,
        }
    }

    /// 创建统一配置
    pub fn unified() -> Self {
        Self {
            implementation: ImplementationType::Unified,
            enable_debug: cfg!(debug_assertions),
            memory_pool_size: 512 * 1024,    // 512KB
            buffer_size: 32 * 1024,          // 32KB
            enable_memory_stats: true,
            enable_call_stats: true,
            enable_error_checking: true,
            max_file_descriptors: 128,
            enable_thread_safety: true,
            printf_precision: 6,
            strict_mode: false,
        }
    }

    /// 验证配置的有效性
    pub fn validate(&self) -> Result<(), ConfigError> {
        // 验证内存池大小
        if self.memory_pool_size < 1024 {
            return Err(ConfigError::InvalidMemoryPoolSize);
        }
        if self.memory_pool_size > 100 * 1024 * 1024 {
            return Err(ConfigError::MemoryPoolTooLarge);
        }

        // 验证缓冲区大小
        if self.buffer_size < 64 {
            return Err(ConfigError::InvalidBufferSize);
        }
        if self.buffer_size > self.memory_pool_size / 4 {
            return Err(ConfigError::BufferTooLarge);
        }

        // 验证文件描述符数量
        if self.max_file_descriptors < 8 {
            return Err(ConfigError::InvalidFileDescriptorLimit);
        }
        if self.max_file_descriptors > 65536 {
            return Err(ConfigError::FileDescriptorLimitTooHigh);
        }

        // 验证printf精度
        if self.printf_precision > 20 {
            return Err(ConfigError::InvalidPrintfPrecision);
        }

        Ok(())
    }

    /// 根据系统资源自动调整配置
    pub fn auto_adjust(mut self) -> Self {
        // 根据可用内存调整内存池大小
        // 这里可以调用系统函数获取可用内存信息
        // 暂时使用固定逻辑

        if self.memory_pool_size > 1024 * 1024 {
            // 在大内存系统中，我们可以使用更大的内存池
            self.buffer_size = self.buffer_size.max(8192);
        }

        // 根据实现类型调整其他参数
        match self.implementation {
            ImplementationType::Minimal => {
                self.enable_memory_stats = false;
                self.enable_call_stats = false;
                self.enable_thread_safety = false;
            }
            ImplementationType::Simple => {
                self.enable_memory_stats = true;
                self.enable_call_stats = true;
            }
            ImplementationType::Full => {
                self.enable_memory_stats = true;
                self.enable_call_stats = true;
                self.enable_thread_safety = true;
            }
            ImplementationType::Unified => {
                self.enable_memory_stats = true;
                self.enable_call_stats = true;
                self.enable_thread_safety = true;
            }
        }

        self
    }

    /// 获取配置摘要信息
    pub fn summary(&self) -> ConfigSummary {
        ConfigSummary {
            implementation: self.implementation,
            memory_pool_mb: self.memory_pool_size / (1024 * 1024),
            buffer_kb: self.buffer_size / 1024,
            max_fds: self.max_file_descriptors,
            features_enabled: self.count_enabled_features(),
        }
    }

    /// 计算启用的功能数量
    fn count_enabled_features(&self) -> usize {
        let mut count = 0;
        if self.enable_debug { count += 1; }
        if self.enable_memory_stats { count += 1; }
        if self.enable_call_stats { count += 1; }
        if self.enable_error_checking { count += 1; }
        if self.enable_thread_safety { count += 1; }
        count
    }
}

/// 配置摘要信息
#[derive(Debug, Clone)]
pub struct ConfigSummary {
    /// 实现类型
    pub implementation: ImplementationType,
    /// 内存池大小（MB）
    pub memory_pool_mb: usize,
    /// 缓冲区大小（KB）
    pub buffer_kb: usize,
    /// 最大文件描述符数
    pub max_fds: usize,
    /// 启用的功能数量
    pub features_enabled: usize,
}

/// 配置错误类型
#[derive(Debug, Clone, PartialEq)]
pub enum ConfigError {
    /// 无效的内存池大小
    InvalidMemoryPoolSize,
    /// 内存池过大
    MemoryPoolTooLarge,
    /// 无效的缓冲区大小
    InvalidBufferSize,
    /// 缓冲区过大
    BufferTooLarge,
    /// 无效的文件描述符限制
    InvalidFileDescriptorLimit,
    /// 文件描述符限制过高
    FileDescriptorLimitTooHigh,
    /// 无效的printf精度
    InvalidPrintfPrecision,
    /// 其他配置错误
    Other(&'static str),
}

impl core::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ConfigError::InvalidMemoryPoolSize => write!(f, "内存池大小过小（最小1KB）"),
            ConfigError::MemoryPoolTooLarge => write!(f, "内存池大小过大（最大100MB）"),
            ConfigError::InvalidBufferSize => write!(f, "缓冲区大小过小（最小64字节）"),
            ConfigError::BufferTooLarge => write!(f, "缓冲区大小过大（超过内存池的1/4）"),
            ConfigError::InvalidFileDescriptorLimit => write!(f, "文件描述符限制过小（最小8）"),
            ConfigError::FileDescriptorLimitTooHigh => write!(f, "文件描述符限制过高（最大65536）"),
            ConfigError::InvalidPrintfPrecision => write!(f, "printf精度过高（最大20）"),
            ConfigError::Other(msg) => write!(f, "配置错误: {}", msg),
        }
    }
}

/// 全局配置实例
static mut GLOBAL_CONFIG: Option<LibcConfig> = None;
static mut CONFIG_INITIALIZED: bool = false;

/// 初始化全局C库配置
///
/// # 参数
/// * `config` - 要使用的配置
///
/// # 安全性
/// 只能在系统初始化时调用一次
pub unsafe fn initialize_config(config: LibcConfig) -> Result<(), ConfigError> {
    if CONFIG_INITIALIZED {
        crate::println!("[libc] 警告：配置已经初始化，跳过重复初始化");
        return Ok(());
    }

    // 验证配置
    config.validate()?;

    GLOBAL_CONFIG = Some(config);
    CONFIG_INITIALIZED = true;

    crate::println!("[libc] C库配置初始化完成");
    Ok(())
}

/// 获取全局配置
///
/// # 返回值
/// * 返回当前配置的引用
///
/// # 安全性
/// 必须在配置初始化后调用
pub unsafe fn get_config() -> &'static LibcConfig {
    if let Some(ref config) = GLOBAL_CONFIG {
        config
    } else {
        panic!("C库配置未初始化！请确保在系统启动时调用initialize_config()");
    }
}

/// 获取配置的可变引用
///
/// # 返回值
/// * 返回当前配置的可变引用
///
/// # 安全性
/// 必须在配置初始化后调用，并且需要确保独占访问
pub unsafe fn get_config_mut() -> &'static mut LibcConfig {
    if let Some(config) = GLOBAL_CONFIG.as_mut() {
        config
    } else {
        panic!("C库配置未初始化！请确保在系统启动时调用initialize_config()");
    }
}

/// 检查配置是否已初始化
pub fn is_config_initialized() -> bool {
    unsafe { CONFIG_INITIALIZED }
}

/// 从环境变量创建配置
///
/// # 参数
/// * `libc_type` - C库类型环境变量的值
///
/// # 返回值
/// * 对应的配置实例
pub fn config_from_env(libc_type: &str) -> LibcConfig {
    match libc_type {
        "minimal" => LibcConfig::minimal(),
        "simple" => LibcConfig::simple(),
        "full" => LibcConfig::full(),
        "unified" => LibcConfig::unified(),
        _ => LibcConfig::unified(),
    }.auto_adjust()
}

/// 获取默认配置
pub fn get_default_config() -> LibcConfig {
    LibcConfig::default()
}

/// 运行时配置管理器
pub struct ConfigManager {
    config: LibcConfig,
    stats: ConfigStats,
}

#[derive(Debug, Default)]
pub struct ConfigStats {
    /// 配置更新次数
    pub config_updates: Cell<u64>,
    /// 配置验证失败次数
    pub validation_failures: Cell<u64>,
    /// 配置查询次数
    pub config_queries: Cell<u64>,
}

impl ConfigManager {
    /// 创建新的配置管理器
    pub fn new(config: LibcConfig) -> Result<Self, ConfigError> {
        config.validate()?;
        Ok(Self {
            config,
            stats: ConfigStats::default(),
        })
    }

    /// 更新配置
    pub fn update_config(&mut self, new_config: LibcConfig) -> Result<(), ConfigError> {
        new_config.validate()?;
        self.config = new_config;
        self.stats.config_updates.set(self.stats.config_updates.get() + 1);
        Ok(())
    }

    /// 获取当前配置
    pub fn config(&self) -> &LibcConfig {
        self.stats.config_queries.set(self.stats.config_queries.get() + 1);
        &self.config
    }

    /// 获取可变配置引用
    pub fn config_mut(&mut self) -> &mut LibcConfig {
        self.stats.config_queries.set(self.stats.config_queries.get() + 1);
        &mut self.config
    }

    /// 获取统计信息
    pub fn stats(&self) -> &ConfigStats {
        &self.stats
    }

    /// 重置统计信息
    pub fn reset_stats(&mut self) {
        self.stats = ConfigStats::default();
    }
}

/// 编译时配置宏
///
/// 这些宏允许在编译时配置C库行为
#[macro_export]
macro_rules! libc_config {
    (minimal) => {
        $crate::libc::config::LibcConfig::minimal()
    };
    (simple) => {
        $crate::libc::config::LibcConfig::simple()
    };
    (full) => {
        $crate::libc::config::LibcConfig::full()
    };
    (unified) => {
        $crate::libc::config::LibcConfig::unified()
    };
    (custom {
        $($field:ident = $value:expr),* $(,)?
    }) => {
        {
            let mut config = $crate::libc::config::LibcConfig::default();
            $(config.$field = $value;)*
            config.auto_adjust()
        }
    };
}

/// 功能检查宏
#[macro_export]
macro_rules! libc_feature_enabled {
    (debug) => {
        unsafe { $crate::libc::config::get_config().enable_debug }
    };
    (memory_stats) => {
        unsafe { $crate::libc::config::get_config().enable_memory_stats }
    };
    (call_stats) => {
        unsafe { $crate::libc::config::get_config().enable_call_stats }
    };
    (error_checking) => {
        unsafe { $crate::libc::config::get_config().enable_error_checking }
    };
    (thread_safety) => {
        unsafe { $crate::libc::config::get_config().enable_thread_safety }
    };
    ($feature:ident) => {
        compile_error!(concat!("未知的C库功能: ", stringify!($feature)))
    };
}

/// 条件编译宏
#[macro_export]
macro_rules! libc_if {
    ($condition:expr, $then:block $(, $else:block)?) => {
        if $condition {
            $then
        } $(else $else)?
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LibcConfig::default();
        assert_eq!(config.implementation, ImplementationType::Simple);
        assert_eq!(config.memory_pool_size, 1024 * 1024);
        assert_eq!(config.buffer_size, 4096);
    }

    #[test]
    fn test_minimal_config() {
        let config = LibcConfig::minimal();
        assert_eq!(config.implementation, ImplementationType::Minimal);
        assert_eq!(config.memory_pool_size, 256 * 1024);
        assert_eq!(config.buffer_size, 1024);
        assert!(!config.enable_memory_stats);
        assert!(!config.enable_call_stats);
    }

    #[test]
    fn test_config_validation() {
        let mut config = LibcConfig::default();

        // 有效配置应该通过验证
        assert!(config.validate().is_ok());

        // 无效的内存池大小应该失败
        config.memory_pool_size = 512; // 小于最小值1024
        assert!(config.validate().is_err());

        // 修复后应该通过
        config.memory_pool_size = 1024;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_auto_adjust() {
        let config = LibcConfig::minimal().auto_adjust();
        assert_eq!(config.implementation, ImplementationType::Minimal);
        assert!(!config.enable_memory_stats);
        assert!(!config.enable_call_stats);
    }

    #[test]
    fn test_config_summary() {
        let config = LibcConfig::simple();
        let summary = config.summary();
        assert_eq!(summary.implementation, ImplementationType::Simple);
        assert_eq!(summary.memory_pool_mb, 512);
        assert_eq!(summary.buffer_kb, 2);
        assert_eq!(summary.max_fds, 128);
    }
}
