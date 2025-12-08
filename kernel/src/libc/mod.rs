//! C标准库统一支持模块
//!
//! 提供完整的newlib C标准库集成，支持标准C程序在NOS上运行。
//! 这个模块实现了统一的接口架构，支持多种实现版本。
//!
//! 主要功能：
//! - 统一的C标准库接口定义
//! - 多种实现版本（最小化、简化、完整）
//! - 配置管理系统
//! - 统一的错误处理机制
//! - 内存管理和I/O操作
//! - 字符串处理函数
//! - 完整的系统调用接口

extern crate alloc;

use alloc::{format, string::String};
use core::ffi::{c_void, c_int, c_char, c_uint};

// 核心接口和 errors handling
pub mod interface;
use interface::size_t;
pub mod error;
pub mod config;
pub mod memory_adapter;
pub mod io_manager;
pub mod formatter;

// 标准库扩展模块
pub mod math_lib;
pub mod string_lib;
pub mod time_lib;
pub mod random_lib;
pub mod env_lib;
pub mod sysinfo_lib;

// 具体实现版本
pub mod implementations;
// minimal和simple实现已整合到implementations.rs中

#[cfg(test)]
pub mod memory_tests;

#[cfg(test)]
pub mod io_tests;

pub mod validation;

/// 标准库全面测试套件
#[cfg(test)]
pub mod standard_tests;

// 重新导出核心组件
pub use interface::*;
pub use error::*;
pub use config::*;

// 重导出实现
// Note: minimal and simple are now part of newlib.rs
pub use implementations::*;

/// 全局C库实例
static mut GLOBAL_CLIB: Option<&'static dyn CLibInterface> = None;
static mut CLIB_INITIALIZED: bool = false;

/// 初始化C标准库系统
///
/// 这个函数负责：
/// 1. 初始化错误处理系统
/// 2. 创建和配置C库实例
/// 3. 设置全局接口
///
/// # 返回值
/// * `Ok(())` - 初始化成功
/// * `Err(String)` - 初始化失败，包含错误描述
pub fn init() -> Result<(), String> {
    if unsafe { CLIB_INITIALIZED } {
        crate::println!("[libc] C库已经初始化，跳过重复初始化");
        return Ok(());
    }

    crate::println!("[libc] 开始初始化C标准库支持模块");

    // 1. 初始化错误处理系统
    error::init_error_handling();

    // 2. 获取配置（这里使用默认配置，后续可以从环境变量读取）
    // 注意：dispatch 返回 isize，不能直接匹配 Result
    // 暂时使用默认配置，后续可以通过其他方式获取环境变量
    let config = config::get_default_config();

    // 3. 创建对应的C库实现
    let libc_impl: &'static dyn CLibInterface = match config.implementation {
        interface::ImplementationType::Minimal => {
            crate::println!("[libc] 使用统一C库实现（最小配置）");
            crate::libc::implementations::create_unified_c_lib()
        }
        interface::ImplementationType::Simple => {
            crate::println!("[libc] 使用统一C库实现（简化配置）");
            crate::libc::implementations::create_unified_c_lib()
        }
        interface::ImplementationType::Full => {
            crate::println!("[libc] 使用统一C库实现（完整配置）");
            crate::libc::implementations::create_unified_c_lib()
        }
        interface::ImplementationType::Unified => {
            crate::println!("[libc] 使用统一C库实现");
            crate::libc::implementations::create_unified_c_lib()
        }
    };

    // 4. 初始化C库实例
    if let Err(e) = libc_impl.initialize() {
        return Err(format!("C库实例初始化失败: {:?}", e));
    }

    // 5. 设置全局接口
    unsafe {
        GLOBAL_CLIB = Some(libc_impl);
        CLIB_INITIALIZED = true;
    }

    // 6. 初始化全局配置
    unsafe {
        if let Err(e) = config::initialize_config(config) {
            crate::println!("[libc] 警告：配置初始化失败: {:?}", e);
        }
    }

    // 7. 初始化全局接口
    unsafe {
        interface::initialize_c_lib(libc_impl);
    }

    // 8. 打印配置信息
    let summary = unsafe { config::get_config().summary() };
    crate::println!("[libc] C库配置: 实现类型={:?}, 内存池={}MB, 缓冲区={}KB, 最大FD={}, 功能数={}",
        summary.implementation,
        summary.memory_pool_mb,
        summary.buffer_kb,
        summary.max_fds,
        summary.features_enabled);

    crate::println!("[libc] C标准库支持模块初始化完成");
    Ok(())
}

/// 获取全局C库接口
///
/// # 返回值
/// * 当前活跃的C库实现引用
///
/// # 安全性
/// 必须在C库初始化后调用
pub fn get_c_lib() -> &'static dyn CLibInterface {
    unsafe {
        if let Some(lib) = GLOBAL_CLIB {
            lib
        } else {
            panic!("C库未初始化！请确保在系统启动时调用init()");
        }
    }
}

/// 检查C库是否已初始化
pub fn is_initialized() -> bool {
    unsafe { CLIB_INITIALIZED }
}

/// 获取C库统计信息
///
/// # 返回值
/// * 当前C库的统计信息
pub fn get_stats() -> CLibStats {
    if is_initialized() {
        get_c_lib().get_stats()
    } else {
        CLibStats::default()
    }
}

/// C标准库常量定义
pub mod constants {
    use crate::libc::interface::macros::*;
    // 统一从interface模块导入所有宏定义
}

// 便捷函数，直接使用全局C库接口
pub mod convenience {
    use super::*;

    /// 内存分配函数
    #[inline]
    pub unsafe fn malloc(size: size_t) -> *mut c_void {
        get_c_lib().malloc(size)
    }

    /// 内存释放函数
    #[inline]
    pub unsafe fn free(ptr: *mut c_void) {
        get_c_lib().free(ptr);
    }

    /// 内存重新分配函数
    #[inline]
    pub unsafe fn realloc(ptr: *mut c_void, size: size_t) -> *mut c_void {
        get_c_lib().realloc(ptr, size)
    }

    /// 清零内存分配函数
    #[inline]
    pub unsafe fn calloc(nmemb: size_t, size: size_t) -> *mut c_void {
        get_c_lib().calloc(nmemb, size)
    }

    /// 字符串长度函数
    #[inline]
    pub unsafe fn strlen(s: *const c_char) -> size_t {
        get_c_lib().strlen(s)
    }

    /// 字符串复制函数
    #[inline]
    pub unsafe fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char {
        get_c_lib().strcpy(dest, src)
    }

    /// 字符串比较函数
    #[inline]
    pub unsafe fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int {
        get_c_lib().strcmp(s1, s2)
    }

    /// 内存设置函数
    #[inline]
    pub unsafe fn memset(s: *mut c_void, c: c_int, n: size_t) -> *mut c_void {
        get_c_lib().memset(s, c, n)
    }

    /// 内存复制函数
    #[inline]
    pub unsafe fn memcpy(dest: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void {
        get_c_lib().memcpy(dest, src, n)
    }

    /// 格式化输出函数 (简化版，不支持复杂格式化)
    #[inline]
    pub unsafe fn printf(format: *const c_char) -> c_int {
        if format.is_null() {
            return -1;
        }
        let fmt_str = core::ffi::CStr::from_ptr(format).to_str().unwrap_or("");
        crate::print!("{}", fmt_str);
        fmt_str.len() as c_int
    }

    /// 字符串输出函数
    #[inline]
    pub unsafe fn puts(s: *const c_char) -> c_int {
        get_c_lib().puts(s)
    }

    /// 字符输出函数
    #[inline]
    pub unsafe fn putchar(c: c_int) -> c_int {
        get_c_lib().putchar(c)
    }

    /// 字符输入函数
    #[inline]
    pub unsafe fn getchar() -> c_int {
        get_c_lib().getchar()
    }

    /// 退出函数
    #[inline]
    pub unsafe fn exit(status: c_int) -> ! {
        get_c_lib().exit(status);
    }

    /// 获取进程ID函数
    #[inline]
    pub unsafe fn getpid() -> c_int {
        get_c_lib().getpid()
    }

    /// 睡眠函数
    #[inline]
    pub unsafe fn sleep(seconds: c_uint) -> c_uint {
        get_c_lib().sleep(seconds)
    }

    /// 时间函数
    #[inline]
    pub unsafe fn time(timer: *mut c_void) -> c_int {
        get_c_lib().time(timer)
    }
}

// 重导出便利函数，供外部使用
pub use convenience::*;

/// C库状态报告
pub fn print_status_report() {
    crate::println!("\n=== C标准库状态报告 ===");
    crate::println!("初始化状态: {}", if is_initialized() { "已初始化" } else { "未初始化" });

    if is_initialized() {
        let stats = get_stats();
        crate::println!("内存统计: {:?}", stats);

        let config = unsafe { config::get_config() };
        crate::println!("当前配置: {:?}", config);

        let errno = error::get_errno();
        if errno != 0 {
            crate::println!("当前错误: {} ({})", errno, error::strerror(errno));
        } else {
            crate::println!("当前错误: 无");
        }
    }
    crate::println!("==================");
}

// 单元测试
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        // 测试初始化流程
        let result = init();
        assert!(result.is_ok());
        assert!(is_initialized());
    }

    #[test]
    fn test_convenience_functions() {
        // 这个测试需要在已初始化的环境中运行
        unsafe {
            let ptr = malloc(100);
            assert!(!ptr.is_null());

            let len = strlen(b"test".as_ptr() as *const c_char);
            assert_eq!(len, 4);

            let result = printf(b"test %s %d\n".as_ptr(), "hello", 42);
            assert!(result > 0);
        }
    }

    #[test]
    fn test_error_handling() {
        error::clear_errno();
        assert_eq!(error::get_errno(), 0);

        error::set_errno(errno::ENOENT);
        assert_eq!(error::get_errno(), errno::ENOENT);

        assert_eq!(error::strerror(errno::ENOENT), "No such file or directory");
    }

    #[test]
    fn test_config_validation() {
        let mut config = config::LibcConfig::default();

        // 有效配置应该通过验证
        assert!(config.validate().is_ok());

        // 无效的内存池大小应该失败
        config.memory_pool_size = 512; // 小于最小值
        assert!(config.validate().is_err());

        // 修复后应该通过
        config.memory_pool_size = 1024;
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_status_report() {
        // 确保状态报告不会崩溃
        print_status_report();
    }
}
