//! GLib工具函数模块
//!
//! 提供与GLib兼容的杂项工具函数，包括：
//! - 数学函数
//! - 实用宏
//! - 随机数生成
//! - 时间和日期函数
//! - 环境变量处理
//! - 命令行执行
//! - 调试和日志工具

#![no_std]

extern crate alloc;

use crate::glib::{types::*, g_free, g_malloc, g_malloc0, g_strdup, error::GError, collections::GList};
use alloc::string::String;
use alloc::vec::Vec;
use core::ptr::{self, NonNull};
use core::ffi::{c_char, c_void, c_int};
use libm;


/// 最大值宏
pub const G_MAXINT8: i8 = i8::MAX;
pub const G_MAXINT16: i16 = i16::MAX;
pub const G_MAXINT32: i32 = i32::MAX;
pub const G_MAXINT64: i64 = i64::MAX;
pub const G_MAXUINT8: u8 = u8::MAX;
pub const G_MAXUINT16: u16 = u16::MAX;
pub const G_MAXUINT32: u32 = u32::MAX;
pub const G_MAXUINT64: u64 = u64::MAX;

/// 常用宏
#[macro_export]
macro_rules! g_new {
    ($t:ty, $n:expr) => {
        $crate::glib::g_malloc(core::mem::size_of::<$t>() * ($n as usize)) as *mut $t
    };
}

#[macro_export]
macro_rules! g_new0 {
    ($t:ty, $n:expr) => {
        $crate::glib::g_malloc0(core::mem::size_of::<$t>() * ($n as usize)) as *mut $t
    };
}

/// 简单数学函数
pub mod math {
    use super::*;

    /// 计算平方根
    pub fn g_sqrt(x: f64) -> f64 {
        if x < 0.0 {
            f64::NAN
        } else {
            libm::sqrt(x)
        }
    }

    /// 向上取整
    pub fn g_ceil(x: f64) -> f64 {
        libm::ceil(x)
    }

    /// 向下取整
    pub fn g_floor(x: f64) -> f64 {
        libm::floor(x)
    }

    /// 四舍五入
    pub fn g_round(x: f64) -> f64 {
        libm::round(x)
    }

    /// 指数函数
    pub fn g_exp(x: f64) -> f64 {
        libm::exp(x)
    }

    /// 自然对数
    pub fn g_ln(x: f64) -> f64 {
        if x <= 0.0 {
            f64::NAN
        } else {
            libm::log(x)
        }
    }

    /// 常用对数 (base 10)
    pub fn g_log10(x: f64) -> f64 {
        if x <= 0.0 {
            f64::NAN
        } else {
            libm::log10(x)
        }
    }

    /// 幂函数
    pub fn g_pow(x: f64, y: f64) -> f64 {
        if x < 0.0 && (y - libm::floor(y)) != 0.0 {
            f64::NAN
        } else {
            libm::pow(x, y)
        }
    }

    /// 正弦函数
    pub fn g_sin(x: f64) -> f64 {
        libm::sin(x)
    }

    /// 余弦函数
    pub fn g_cos(x: f64) -> f64 {
        libm::cos(x)
    }

    /// 正切函数
    pub fn g_tan(x: f64) -> f64 {
        libm::tan(x)
    }

    /// 反正弦函数
    pub fn g_asin(x: f64) -> f64 {
        if x < -1.0 || x > 1.0 {
            f64::NAN
        } else {
            libm::asin(x)
        }
    }

    /// 反余弦函数
    pub fn g_acos(x: f64) -> f64 {
        if x < -1.0 || x > 1.0 {
            f64::NAN
        } else {
            libm::acos(x)
        }
    }

    /// 反正切函数
    pub fn g_atan(x: f64) -> f64 {
        libm::atan(x)
    }
}

/// 随机数生成
pub mod random {
    use super::*;
    use core::sync::atomic::{AtomicU32, Ordering};

    static SEED: AtomicU32 = AtomicU32::new(12345);

    /// 设置随机数种子
    pub fn g_random_set_seed(seed: guint32) {
        SEED.store(seed, Ordering::SeqCst);
    }

    /// 获取随机整数
    pub fn g_random_int() -> guint32 {
        let mut x = SEED.load(Ordering::SeqCst);
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        SEED.store(x, Ordering::SeqCst);
        x
    }

    /// 获取指定范围内的随机整数 [begin, end)
    pub fn g_random_int_range(begin: gint32, end: gint32) -> gint32 {
        if begin >= end {
            return begin;
        }
        let range = (end - begin) as guint32;
        begin + (g_random_int() % range) as gint32
    }

    /// 获取随机浮点数 [0, 1)
    pub fn g_random_double() -> gdouble {
        (g_random_int() as f64) / (u32::MAX as f64 + 1.0)
    }
}

/// 时间和日期
pub mod time {
    use super::*;

    /// 获取系统当前时间（微秒）
    pub fn g_get_real_time() -> gint64 {
        (crate::time::get_timestamp() * 1_000_000) as i64
    }

    /// 获取单调时间（纳秒精度）
    pub fn g_get_monotonic_time() -> gint64 {
        (crate::time::get_timestamp() * 1_000_000_000) as i64 // 转换为纳秒
    }

    /// 休眠指定的微秒数
    pub fn g_usleep(microseconds: gulong) {
        crate::time::sleep(core::time::Duration::from_micros(microseconds as u64));
    }

    /// 休眠指定的秒数
    pub fn g_sleep(seconds: guint) {
        crate::time::sleep(core::time::Duration::from_secs(seconds as u64));
    }

    /// 日期结构
    #[repr(C)]
    pub struct GDate {
        pub julian_days: guint32,
        pub julian: guint,
        pub dmy: guint,
        pub day: guint8,
        pub month: guint8,
        pub year: guint16,
    }

    impl GDate {
        pub fn new() -> *mut GDate {
            g_malloc0(core::mem::size_of::<GDate>()) as *mut GDate
        }

        pub fn free(date: *mut GDate) {
            g_free(date as gpointer);
        }
    }

    pub fn g_date_set_dmy(date: *mut GDate, day: guint8, month: guint8, year: guint16) {
        if date.is_null() { return; }
        unsafe {
            (*date).day = day;
            (*date).month = month;
            (*date).year = year;
            (*date).julian = 1;
            (*date).julian_days = calculate_julian_days(day, month, year);
        }
    }

    pub fn g_date_get_day(date: *const GDate) -> guint8 {
        if date.is_null() { return 0; }
        unsafe { (*date).day }
    }

    pub fn g_date_get_month(date: *const GDate) -> guint8 {
        if date.is_null() { return 0; }
        unsafe { (*date).month }
    }

    pub fn g_date_get_year(date: *const GDate) -> guint16 {
        if date.is_null() { return 0; }
        unsafe { (*date).year }
    }

    pub fn g_date_add_days(date: *mut GDate, n_days: guint) {
        if date.is_null() { return; }
        unsafe {
            (*date).julian_days += n_days as u32;
            // 简单实现：不重新计算dmy
        }
    }

    pub fn g_date_subtract_days(date: *mut GDate, n_days: guint) {
        if date.is_null() { return; }
        unsafe {
            if (*date).julian_days >= n_days as u32 {
                (*date).julian_days -= n_days as u32;
            }
        }
    }

    fn calculate_julian_days(day: guint8, month: guint8, year: guint16) -> guint32 {
        // 简化的儒略日计算
        let a = (14 - month as i32) / 12;
        let y = year as i32 + 4800 - a;
        let m = month as i32 + 12 * a - 3;
        (day as i32 + (153 * m + 2) / 5 + 365 * y + y / 4 - y / 100 + y / 400 - 32045) as guint32
    }
}

/// 环境变量
pub mod environ {
    use super::*;

    /// 获取环境变量
    pub fn g_getenv(variable: *const gchar) -> *const gchar {
        if variable.is_null() { return ptr::null(); }
        // NOS内核目前不支持完整环境变量，返回null
        ptr::null()
    }

    /// 设置环境变量
    pub fn g_setenv(variable: *const gchar, value: *const gchar, overwrite: gboolean) -> gboolean {
        if variable.is_null() || value.is_null() { return 0; }
        // 暂不实现
        1
    }

    /// 取消设置环境变量
    pub fn g_unsetenv(variable: *const gchar) {
        // 暂不实现
    }

    /// 获取所有环境变量
    pub fn g_listenv() -> *mut *mut gchar {
        // 返回空列表
        g_malloc0(core::mem::size_of::<*mut gchar>()) as *mut *mut gchar
    }

    /// 获取当前用户名称
    pub fn g_get_user_name() -> *const gchar {
        b"nos_user\0".as_ptr() as *const gchar
    }

    /// 获取用户真实名称
    pub fn g_get_real_name() -> *const gchar {
        b"NOS System User\0".as_ptr() as *const gchar
    }

    /// 获取用户主目录
    pub fn g_get_home_dir() -> *const gchar {
        b"/\0".as_ptr() as *const gchar
    }

    /// 获取临时目录
    pub fn g_get_tmp_dir() -> *const gchar {
        b"/tmp\0".as_ptr() as *const gchar
    }

    /// 获取当前工作目录
    pub fn g_get_current_dir() -> *mut gchar {
        g_strdup(b"/\0".as_ptr() as *const gchar)
    }
}

/// 辅助函数：从C字符串创建Rust String
pub(crate) unsafe fn from_cstr(ptr: *const gchar) -> String {
    if ptr.is_null() {
        return String::from("");
    }
    let mut len = 0;
    while *ptr.add(len) != 0 {
        len += 1;
    }
    let slice = core::slice::from_raw_parts(ptr as *const u8, len);
    String::from_utf8_lossy(slice).into_owned()
}

/// 位操作工具
pub mod bit_utils {
    use super::*;

    /// 查找最低有效位
    pub fn g_bit_nth_lsf(mask: gulong, nth_bit: gint) -> gint {
        let mut i = (nth_bit + 1) as usize;
        while i < core::mem::size_of::<gulong>() * 8 {
            if (mask & (1 << i)) != 0 {
                return i as gint;
            }
            i += 1;
        }
        -1
    }

    /// 查找最高有效位
    pub fn g_bit_nth_msf(mask: gulong, nth_bit: gint) -> gint {
        let mut i = if nth_bit < 0 {
            core::mem::size_of::<gulong>() * 8 - 1
        } else {
            (nth_bit - 1) as usize
        };
        
        while i > 0 {
            if (mask & (1 << i)) != 0 {
                return i as gint;
            }
            i -= 1;
        }
        if (mask & 1) != 0 && i == 0 { return 0; }
        -1
    }

    /// 计算存储位
    pub fn g_bit_storage(number: gulong) -> guint {
        if number == 0 { return 0; }
        (core::mem::size_of::<gulong>() * 8 - (number.leading_zeros() as usize)) as guint
    }
}

/// 字节序转换
pub mod byte_order {
    use super::*;

    pub fn g_htonl(val: guint32) -> guint32 { val.to_be() }
    pub fn g_ntohl(val: guint32) -> guint32 { guint32::from_be(val) }
    pub fn g_htons(val: guint16) -> guint16 { val.to_be() }
    pub fn g_ntohs(val: guint16) -> guint16 { guint16::from_be(val) }
}

/// 断言和调试
pub mod debug {
    use super::*;

    pub fn g_return_if_fail(condition: bool) {
        if !condition {
            glib_println!("[glib] CRITICAL: assertion failed");
        }
    }

    pub fn g_warn_if_fail(condition: bool) {
        if !condition {
            glib_println!("[glib] WARNING: assertion failed");
        }
    }

    pub fn g_on_error_query(prg_name: *const gchar) {
        glib_println!("[glib] ERROR in program: {:?}", prg_name);
    }

    pub fn g_on_error_stack_trace(prg_name: *const gchar) {
        glib_println!("[glib] Stack trace for: {:?}", prg_name);
    }
}