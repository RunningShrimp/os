//! GLib错误处理系统
//!
//! 提供与GLib兼容的错误处理机制，包括：
//! - GError结构体
//! - 错误域(GQuark)管理
//! - 错误码定义
//! - 错误消息格式化

#![no_std]

extern crate alloc;

use alloc::string::String;
use core::ffi::c_int;
use crate::glib::types::*;
use alloc::boxed::Box;

/// 从字符串创建错误域
pub fn g_quark_from_string(string: &str) -> GQuark {
    // 简单的哈希实现
    let mut hash: u32 = 5381;
    for byte in string.bytes() {
        hash = ((hash << 5).wrapping_add(hash)).wrapping_add(byte as u32);
    }
    hash
}

/// 将错误域转换为字符串（简化实现）
pub fn g_quark_to_string(quark: GQuark) -> String {
    alloc::format!("domain-{}", quark)
}

/// GLib错误结构体
#[derive(Debug, Clone)]
pub struct GError {
    /// 错误域
    pub domain: GQuark,
    /// 错误码
    pub code: c_int,
    /// 错误消息
    pub message: String,
}

impl GError {
    /// 创建新的错误
    pub fn new(domain: GQuark, code: c_int, message: &str) -> Self {
        Self {
            domain,
            code,
            message: String::from(message),
        }
    }

    /// 创建字面错误
    pub fn new_literal(domain: GQuark, code: c_int, message: &str) -> Self {
        Self {
            domain,
            code,
            message: String::from(message),
        }
    }

    /// 格式化错误消息
    pub fn new_printf(domain: GQuark, code: c_int, format: &str, args: core::fmt::Arguments) -> Self {
        Self {
            domain,
            code,
            message: alloc::format!("{}", format),
        }
    }

    /// 复制错误
    pub fn copy(&self) -> Self {
        self.clone()
    }

    /// 匹配错误域和码
    pub fn matches(&self, domain: GQuark, code: c_int) -> bool {
        self.domain == domain && self.code == code
    }

    /// 检查是否匹配域
    pub fn matches_domain(&self, domain: GQuark) -> bool {
        self.domain == domain
    }
}

/// 预定义的错误域
pub mod domains {
    use super::*;

    /// GLib一般错误
    pub const G_FILE_ERROR: GQuark = 1; // 简化常量

    /// GLib转换错误
    pub const G_SPAWN_ERROR: GQuark = 2;

    /// GLib标记错误
    pub const G_MARKUP_ERROR: GQuark = 3;

    /// GLib线程错误
    pub const G_THREAD_ERROR: GQuark = 4;

    /// GLib正则表达式错误
    pub const G_REGEX_ERROR: GQuark = 5;

    /// 自定义错误域
    pub fn g_quark_from_static_string(string: &str) -> GQuark {
        g_quark_from_string(string)
    }
}

/// 文件错误码
pub mod file_errors {
    use crate::glib::types::*;
    use core::ffi::c_int;

    pub const G_FILE_ERROR_EXIST: c_int = 2;
    pub const G_FILE_ERROR_ISDIR: c_int = 3;
    pub const G_FILE_ERROR_ACCES: c_int = 4;
    pub const G_FILE_ERROR_NAMETOOLONG: c_int = 5;
    pub const G_FILE_ERROR_NOENT: c_int = 6;
    pub const G_FILE_ERROR_NOSPC: c_int = 7;
    pub const G_FILE_ERROR_NOTDIR: c_int = 8;
    pub const G_FILE_ERROR_ROFS: c_int = 9;
    pub const G_FILE_ERROR_IO: c_int = 10;
    pub const G_FILE_ERROR_PERM: c_int = 11;
    pub const G_FILE_ERROR_FAILED: c_int = 12;
}

/// 错误工具函数
pub mod error_utils {
    use super::*;

    /// 设置错误指针
    pub unsafe fn set_error(error: *mut *mut GError, domain: GQuark, code: c_int, message: &str) {
        if !error.is_null() && (*error).is_null() {
            let gerror = Box::new(GError::new(domain, code, message));
            *error = Box::into_raw(gerror);
        }
    }

    /// 设置格式化错误指针
    pub unsafe fn set_error_printf(
        error: *mut *mut GError,
        domain: GQuark,
        code: c_int,
        format: &str,
        args: core::fmt::Arguments,
    ) {
        if !error.is_null() && (*error).is_null() {
            let gerror = Box::new(GError::new_printf(domain, code, format, args));
            *error = Box::into_raw(gerror);
        }
    }

    /// 传播错误
    pub unsafe fn propagate_error(dest: *mut *mut GError, src: *mut GError) {
        if !dest.is_null() && (*dest).is_null() && !src.is_null() {
            *dest = src;
        }
    }

    /// 释放错误
    pub unsafe fn clear_error(error: *mut *mut GError) {
        if !error.is_null() && !(*error).is_null() {
            let _ = Box::from_raw(*error);
            *error = core::ptr::null_mut();
        }
    }

    /// 复制错误
    pub unsafe fn copy_error(src: *const GError) -> *mut GError {
        if src.is_null() {
            return core::ptr::null_mut();
        }
        let gerror = (*src).copy();
        Box::into_raw(Box::new(gerror))
    }
}

/// 便捷宏
#[macro_export]
macro_rules! g_set_error {
    ($error:expr, $domain:expr, $code:expr, $fmt:expr $(,$($arg:tt)*)?) => {
        unsafe {
            $crate::glib::error::error_utils::set_error(
                $error,
                $domain,
                $code,
                alloc::format!($fmt $(,$($arg)*)?).as_str(),
            )
        }
    };
}

#[macro_export]
macro_rules! g_propagate_error {
    ($dest:expr, $src:expr) => {
        unsafe { $crate::glib::error::error_utils::propagate_error($dest, $src) }
    };
}

#[macro_export]
macro_rules! g_clear_error {
    ($error:expr) => {
        unsafe { $crate::glib::error::error_utils::clear_error($error) }
    };
}

#[macro_export]
macro_rules! g_error_matches {
    ($err:expr, $domain:expr, $code:expr) => {
        unsafe { !$err.is_null() && (*$err).matches($domain, $code) }
    };
}

/// 错误处理测试
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quark_creation() {
        let quark1 = g_quark_from_string("test-domain");
        let quark2 = g_quark_from_string("test-domain");
        assert_eq!(quark1, quark2); // 相同字符串应该产生相同的哈希值
    }

    #[test]
    fn test_gerror_creation() {
        let domain = g_quark_from_string("test");
        let error = GError::new(domain, 1, "Test message");
        assert_eq!(error.domain, domain);
        assert_eq!(error.code, 1);
        assert_eq!(error.message, "Test message");
    }

    #[test]
    fn test_error_matching() {
        let domain = g_quark_from_string("test");
        let error = GError::new(domain, 1, "Test message");

        assert!(error.matches(domain, 1));
        assert!(!error.matches(domain, 2));
        assert!(!error.matches(g_quark_from_string("other"), 1));
        assert!(error.matches_domain(domain));
        assert!(!error.matches_domain(g_quark_from_string("other")));
    }

    #[test]
    fn test_error_copy() {
        let domain = g_quark_from_string("test");
        let error1 = GError::new(domain, 1, "Test message");
        let error2 = error1.copy();

        assert_eq!(error1.domain, error2.domain);
        assert_eq!(error1.code, error2.code);
        assert_eq!(error1.message, error2.message);
    }

    #[test]
    fn test_error_pointer_functions() {
        unsafe {
            let mut error: *mut GError = core::ptr::null_mut();

            // 测试设置错误
            let domain = g_quark_from_string("test");
            error_utils::set_error(&mut error, domain, 1, "Test error");
            assert!(!error.is_null());
            assert_eq!((*error).domain, domain);
            assert_eq!((*error).code, 1);

            // 测试复制错误
            let error_copy = error_utils::copy_error(error);
            assert!(!error_copy.is_null());
            assert_eq!((*error_copy).message, "Test error");

            // 清理错误
            error_utils::clear_error(&mut error);
            error_utils::clear_error(&mut error_copy);
        }
    }
}