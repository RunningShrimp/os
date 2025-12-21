//! GLib用户空间实现
//!
//! 提供完整的GLib功能支持，包括内存管理、数据结构、对象系统、
//! 事件循环、异步I/O等核心功能。这个实现与NOS内核的GLib扩展
//! 系统调用深度集成，提供高性能的GLib支持。
//!
//! 主要模块：
//! - memory: 内存管理 (g_malloc, g_free, g_slice等)
//! - collections: 数据结构 (GList, GSList, GHashTable等)
//! - object: 对象系统 (GObject基础实现)
//! - main_loop: 主循环和事件系统 (GMainLoop, GMainContext)
//! - async_io: 异步I/O系统 (GIO异步操作)
//! - string: 字符串操作 (GString, 字符串工具函数)
//! - utils: 工具函数和宏
//!

#![no_std]

extern crate alloc;

// Helper macro for printing
#[macro_export]
macro_rules! glib_println {
    ($($arg:tt)*) => {
        crate::println(&alloc::format!($($arg)*))
    };
}

/// GLib常用常量
pub mod constants {
    /// GLib版本信息
    pub const GLIB_MAJOR_VERSION: u32 = 2;
    pub const GLIB_MINOR_VERSION: u32 = 76;
    pub const GLIB_MICRO_VERSION: u32 = 0;

    /// 内存对齐常量
    pub const G_MEM_ALIGN: usize = 8;
    pub const G_MEM_ALIGN_POW2: usize = 3; // 2^3 = 8

    /// 默认内存池大小
    pub const DEFAULT_MEMORY_POOL_SIZE: usize = 1024 * 1024; // 1MB

    /// 默认最大内存池数量
    pub const DEFAULT_MAX_MEMORY_POOLS: usize = 16;

    /// 默认事件循环优先级
    pub const G_PRIORITY_DEFAULT: i32 = 0;
    pub const G_PRIORITY_DEFAULT_IDLE: i32 = 200;
    pub const G_PRIORITY_HIGH: i32 = -100;
    pub const G_PRIORITY_HIGH_IDLE: i32 = 100;
    pub const G_PRIORITY_LOW: i32 = 300;

    /// 默认异步操作超时
    pub const G_ASYNC_DEFAULT_TIMEOUT: u32 = 5000; // 5秒

    /// 最大对象类型数量
    pub const G_MAX_OBJECT_TYPES: usize = 1024;

    /// 最大信号处理器数量
    pub const G_MAX_SIGNAL_HANDLERS: usize = 64;
}

/// GLib类型别名
pub mod types {
    use core::ffi::{c_void, c_char, c_int, c_uint};

    /// GLib布尔类型
    pub type gboolean = c_int;

    /// GLib字符类型
    pub type gchar = c_char;

    /// GLib无符号字符类型
    pub type guchar = core::ffi::c_uchar;

    /// GLib整数类型
    pub type gint = c_int;
    pub type guint = c_uint;
    pub type gshort = i16;
    pub type gushort = u16;
    pub type gint8 = i8;
    pub type gint16 = i16;
    pub type gint32 = i32;
    pub type guint8 = u8;
    pub type guint16 = u16;
    pub type guint32 = u32;

    /// GLib长整数类型
    pub type glong = isize;
    pub type gulong = usize;

    /// GLib指针类型
    pub type gpointer = *mut c_void;
    pub type gconstpointer = *const c_void;

    /// GLib大小类型
    pub type gsize = usize;
    pub type gssize = isize;

    /// GLib时间戳类型
    pub type gint64 = i64;
    pub type guint64 = u64;

    /// GLib浮点类型
    pub type gfloat = f32;
    pub type gdouble = f64;

    /// GLib回调函数类型
    pub type GFunc = unsafe extern "C" fn(gpointer, gpointer);
    pub type GCallback = unsafe extern "C" fn();
    pub type GCompareFunc = unsafe extern "C" fn(gconstpointer, gconstpointer) -> c_int;
    pub type GCompareDataFunc = unsafe extern "C" fn(gconstpointer, gconstpointer, gpointer) -> c_int;
    pub type GDestroyNotify = unsafe extern "C" fn(gpointer);
    pub type GCopyFunc = unsafe extern "C" fn(gconstpointer, gpointer) -> gpointer;

    /// GLib错误域类型
    pub type GQuark = u32;
}

pub use constants::*;
pub use types::*;

// GLib错误处理
pub mod error;
pub use error::GError;

// 核心模块
pub mod memory;
pub mod collections;
pub mod object;
pub mod main_loop;
pub mod async_io;
pub mod string;
pub mod utils;

// 重新导出公共接口
pub use memory::*;
pub use collections::*;
pub use object::*;
pub use main_loop::*;
pub use async_io::*;
pub use string::*;
pub use utils::*;

/// GLib初始化函数
///
/// 初始化GLib用户空间库，包括：
/// - 初始化内存管理器
/// - 设置系统调用接口
/// - 初始化对象系统
/// - 配置默认参数
pub fn init() -> Result<(), GError> {
    // 初始化各个子系统
    memory::init()?;
    object::init()?;
    main_loop::init()?;
    async_io::init()?;

    glib_println!("[glib] GLib用户空间库初始化完成");
    Ok(())
}

/// GLib清理函数
///
/// 释放GLib使用的资源
pub fn cleanup() {
    async_io::cleanup();
    main_loop::cleanup();
    object::cleanup();
    memory::cleanup();
    glib_println!("[glib] GLib用户空间库清理完成");
}

/// 获取全局状态
pub(crate) fn get_state_mut() -> *mut GLibState {
    static mut STATE: GLibState = GLibState {
        initialized: false,
    };
    unsafe { &raw mut STATE as *mut GLibState }
}

/// GLib全局状态
pub struct GLibState {
    pub initialized: bool,
}

/// GQuark助手函数
pub fn g_quark_from_string(string: &str) -> types::GQuark {
    error::g_quark_from_string(string)
}

pub fn g_quark_to_string(quark: types::GQuark) -> alloc::string::String {
    error::g_quark_to_string(quark)
}

/// GLib测试模块
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_macros() {
        // 测试g_new宏
        let int_array = g_new!(i32, 10);
        assert!(!int_array.is_null());

        // 测试g_new0宏
        let zero_array = g_new0!(u8, 20);
        assert!(!zero_array.is_null());

        // 清理
        g_free(int_array as gpointer);
        g_free(zero_array as gpointer);
    }

    #[test]
    fn test_error_handling() {
        use error::*;

        // 测试错误创建
        let error = GError::new(g_quark_from_string("test-domain"), 1, "Test error message");
        assert_eq!(error.domain, g_quark_from_string("test-domain"));
        assert_eq!(error.code, 1);
        assert_eq!(error.message, "Test error message");

        // 测试错误格式化
        let formatted_error = GError::new_literal(g_quark_from_string("test"), 2, "Formatted error");
        assert_eq!(formatted_error.message, "Formatted error");
    }
}
