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

#![allow(non_camel_case_types)]

extern crate alloc;

// Helper macro for printing
#[macro_export]
macro_rules! glib_println {
    ($($arg:tt)*) => {
        crate::println(&alloc::format!($($arg)*))
    };
}

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
pub use macros::*;

// GLib错误处理
pub mod error;
pub use error::GError;

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
/// 清理所有GLib资源，通常在程序退出时调用
pub fn cleanup() {
    async_io::cleanup();
    main_loop::cleanup();
    object::cleanup();
    memory::cleanup();

    glib_println!("[glib] GLib用户空间库清理完成");
}

/// GLib全局状态
pub struct GLibState {
    pub initialized: bool,
    pub debug_enabled: bool,
    pub memory_stats: memory::MemoryStats,
    pub object_stats: object::ObjectStats,
}

static mut G_LIB_STATE: Option<GLibState> = None;

/// 获取G_LIB_STATE的raw pointer
unsafe fn get_glib_state_ptr() -> *mut Option<GLibState> {
    core::ptr::addr_of_mut!(G_LIB_STATE)
}

/// 获取GLib全局状态
pub fn get_state() -> &'static GLibState {
    unsafe {
        let state_ptr = get_glib_state_ptr();
        (*state_ptr).as_ref().unwrap_or_else(|| {
            panic!("GLib未初始化！请先调用glib::init()");
        })
    }
}

/// 获取可变GLib全局状态
pub fn get_state_mut() -> &'static mut GLibState {
    unsafe {
        let state_ptr = get_glib_state_ptr();
        (*state_ptr).as_mut().unwrap_or_else(|| {
            panic!("GLib未初始化！请先调用glib::init()");
        })
    }
}

/// GLib宏定义
pub mod macros {
    /// g_new宏 - 分配并初始化数组
    #[macro_export]
    macro_rules! g_new {
        ($type:ty, $count:expr) => {
            {
                let size = core::mem::size_of::<$type>() * $count;
                let ptr = nos_api::memory::malloc(size);
                if ptr.is_null() {
                    core::ptr::null_mut()
                } else {
                    ptr as *mut $type
                }
            }
        };
    }
    
    /// g_free宏 - 释放内存
    #[macro_export]
    macro_rules! g_free {
        ($ptr:expr) => {
            if !$ptr.is_null() {
                nos_api::memory::free($ptr as *mut core::ffi::c_void);
            }
        };
    }
}

// 常量定义
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

    /// GLib优先级常量
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
    pub type Gboolean = c_int;

    /// GLib字符类型
    pub type gchar = c_char;
    pub type Gchar = c_char;

    /// GLib无符号字符类型
    pub type guchar = core::ffi::c_uchar;
    pub type Guchar = core::ffi::c_uchar;

    /// GLib整数类型
    pub type gint = c_int;
    pub type Gint = c_int;
    pub type guint = c_uint;
    pub type Guint = c_uint;

    /// GLib短整数类型
    pub type gshort = i16;
    pub type Gshort = i16;
    pub type gushort = u16;
    pub type Gushort = u16;
    
    /// GLib精确宽度的整数类型
    pub type gint8 = i8;
    pub type Gint8 = i8;
    pub type guint8 = u8;
    pub type Guint8 = u8;
    pub type gint16 = i16;
    pub type Gint16 = i16;
    pub type guint16 = u16;
    pub type Guint16 = u16;
    pub type gint32 = i32;
    pub type Gint32 = i32;
    pub type guint32 = u32;
    pub type Guint32 = u32;
    pub type gint64 = i64;
    pub type Gint64 = i64;
    pub type guint64 = u64;
    pub type Guint64 = u64;

    /// GLib长整数类型
    pub type glong = isize;
    pub type Glong = isize;
    pub type gulong = usize;
    pub type Gulong = usize;

    /// GLib指针类型
    pub type gpointer = *mut c_void;
    pub type Gpointer = *mut c_void;
    pub type gconstpointer = *const c_void;
    pub type Gconstpointer = *const c_void;

    /// GLib大小类型
    pub type gsize = usize;
    pub type Gsize = usize;
    pub type gssize = isize;
    pub type Gssize = isize;

    /// GLib浮点类型
    pub type gfloat = f32;
    pub type Gfloat = f32;
    pub type gdouble = f64;
    pub type Gdouble = f64;

    /// GLib回调函数类型
    pub type GFunc = unsafe extern "C" fn(gpointer, gpointer);
    pub type GCallback = unsafe extern "C" fn();
    pub type GCompareFunc = unsafe extern "C" fn(gconstpointer, gconstpointer) -> c_int;
    pub type GCompareDataFunc = unsafe extern "C" fn(gconstpointer, gconstpointer, gpointer) -> c_int;
    pub type GDestroyNotify = unsafe extern "C" fn(gpointer);
    pub type GCopyFunc = unsafe extern "C" fn(gconstpointer, gpointer) -> gpointer;
}

// 重新导出常用函数和类型
pub use types::*;
pub use constants::*;

/// GLib测试模块
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialization() {
        // 测试GLib初始化
        let result = init();
        assert!(result.is_ok());
        assert!(get_state().initialized);

        cleanup();
    }

    #[test]
    fn test_memory_allocation() {
        init().unwrap();

        // 测试基本内存分配
        let ptr = g_malloc(100);
        assert!(!ptr.is_null());

        // 测试重新分配
        let new_ptr = g_realloc(ptr, 200);
        assert!(!new_ptr.is_null());

        // 测试清零分配
        let zero_ptr = g_malloc0(50);
        assert!(!zero_ptr.is_null());

        // 清理
        g_free(new_ptr);
        g_free(zero_ptr);

        cleanup();
    }

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
        let error = GError::new(GQuark::from_string("test-domain"), 1, "Test error message");
        assert_eq!(error.domain, GQuark::from_string("test-domain"));
        assert_eq!(error.code, 1);
        assert_eq!(error.message, "Test error message");

        // 测试错误格式化
        let formatted_error = GError::new_literal(GQuark::from_string("test"), 2, "Formatted error");
        assert_eq!(formatted_error.message, "Formatted error");
    }
}