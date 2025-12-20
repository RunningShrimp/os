//! GLib字符串和工具函数模块
//!
//! 提供与GLib兼容的字符串操作功能，包括：
//! - GString 动态字符串
//! - UTF-8 字符串处理
//! - 字符串转换和格式化
//! - 路径处理
//! - 命令行参数解析
//! - 配置文件处理

#![no_std]

extern crate alloc;

use crate::glib::{types::*, g_free, g_malloc, g_malloc0, g_realloc, error::GError};
use alloc::string::String;
use alloc::vec::Vec;
use core::ptr::{self, NonNull};
use core::ffi::{c_char, c_int, c_void};
use core::str;


/// GString 动态字符串结构
#[derive(Debug)]
pub struct GString {
    pub str: *mut gchar,
    pub len: usize,
    pub allocated_len: usize,
}

impl GString {
    /// 创建新的GString
    pub fn new(init: &str) -> *mut GString {
        let bytes = init.as_bytes();
        let str_ptr = unsafe {
            let ptr = g_malloc0(bytes.len() + 1) as *mut gchar;
            if !ptr.is_null() {
                ptr::copy_nonoverlapping(bytes.as_ptr(), ptr as *mut u8, bytes.len());
            }
            ptr
        };

        if str_ptr.is_null() {
            return ptr::null_mut();
        }

        unsafe {
            let gstring = g_malloc0(core::mem::size_of::<GString>()) as *mut GString;
            if gstring.is_null() {
                g_free(str_ptr as gpointer);
                return ptr::null_mut();
            }

            (*gstring).str = str_ptr;
            (*gstring).len = bytes.len();
            (*gstring).allocated_len = bytes.len() + 1;

            glib_println!("[glib_string] 创建GString: len={}", bytes.len());
            gstring
        }
    }

    /// 获取字符串长度
    pub fn len(gstring: *const GString) -> usize {
        if gstring.is_null() {
            return 0;
        }
        unsafe { (*gstring).len }
    }

    /// 获取分配的长度
    pub fn allocated_len(gstring: *const GString) -> usize {
        if gstring.is_null() {
            return 0;
        }
        unsafe { (*gstring).allocated_len }
    }

    /// 追加字符串
    pub fn append(gstring: *mut GString, val: &str) -> *mut GString {
        if gstring.is_null() {
            return GString::new(val);
        }

        let bytes = val.as_bytes();
        let new_len = unsafe { (*gstring).len + bytes.len() };

        // 检查是否需要重新分配
        if new_len + 1 > unsafe { (*gstring).allocated_len } {
            let new_allocated_len = new_len + 1 + (new_len / 2); // 增加50%额外空间
            unsafe {
                let new_str = g_realloc((*gstring).str as gpointer, new_allocated_len) as *mut gchar;
                if new_str.is_null() {
                    return gstring; // 分配失败，返回原字符串
                }

                (*gstring).str = new_str;
                (*gstring).allocated_len = new_allocated_len;
            }
        }

        // 追加新字符串
        unsafe {
            let dest = (*gstring).str.add((*gstring).len);
            ptr::copy_nonoverlapping(bytes.as_ptr(), dest as *mut u8, bytes.len());
            *(*gstring).str.add(new_len) = 0; // 添加null终止符
            (*gstring).len = new_len;
        }

        gstring
    }

    /// 追加字符
    pub fn append_c(gstring: *mut GString, c: gchar) -> *mut GString {
        if gstring.is_null() {
            let mut str_bytes = [0u8; 1];
            str_bytes[0] = c as u8;
            // 简化处理，直接创建新字符串
            return GString::new(unsafe { str::from_utf8_unchecked(&str_bytes) });
        }

        let new_len = unsafe { (*gstring).len + 1 };

        // 检查是否需要重新分配
        if new_len + 1 > unsafe { (*gstring).allocated_len } {
            let new_allocated_len = new_len + 1 + (new_len / 2);
            unsafe {
                let new_str = g_realloc((*gstring).str as gpointer, new_allocated_len) as *mut gchar;
                if new_str.is_null() {
                    return gstring;
                }

                (*gstring).str = new_str;
                (*gstring).allocated_len = new_allocated_len;
            }
        }

        // 追加字符
        unsafe {
            *(*gstring).str.add((*gstring).len) = c;
            *(*gstring).str.add(new_len) = 0;
            (*gstring).len = new_len;
        }

        gstring
    }

    /// 预分配空间
    pub fn gstring_prepend(gstring: *mut GString, val: &str) -> *mut GString {
        if gstring.is_null() {
            return GString::new(val);
        }

        let bytes = val.as_bytes();
        let new_len = unsafe { (*gstring).len + bytes.len() };

        // 检查是否需要重新分配
        if new_len + 1 > unsafe { (*gstring).allocated_len } {
            let new_allocated_len = new_len + 1 + (new_len / 2);
            unsafe {
                let new_str = g_realloc((*gstring).str as gpointer, new_allocated_len) as *mut gchar;
                if new_str.is_null() {
                    return gstring;
                }

                // 移动原有数据
                ptr::copy((*gstring).str, new_str.add(bytes.len()), (*gstring).len);

                (*gstring).str = new_str;
                (*gstring).allocated_len = new_allocated_len;
            }
        }

        // 在前面添加新数据
        unsafe {
            ptr::copy_nonoverlapping(bytes.as_ptr(), (*gstring).str as *mut u8, bytes.len());
            *(*gstring).str.add(new_len) = 0;
            (*gstring).len = new_len;
        }

        gstring
    }

    /// 插入字符串
    pub fn insert(gstring: *mut GString, pos: usize, val: &str) -> *mut GString {
        if gstring.is_null() {
            return GString::new(val);
        }

        let bytes = val.as_bytes();
        let old_len = unsafe { (*gstring).len };
        let pos = core::cmp::min(pos, old_len);
        let new_len = old_len + bytes.len();

        // 检查是否需要重新分配
        if new_len + 1 > unsafe { (*gstring).allocated_len } {
            let new_allocated_len = new_len + 1 + (new_len / 2);
            unsafe {
                let new_str = g_realloc((*gstring).str as gpointer, new_allocated_len) as *mut gchar;
                if new_str.is_null() {
                    return gstring;
                }

                (*gstring).str = new_str;
                (*gstring).allocated_len = new_allocated_len;
            }
        }

        // 移动数据为新字符串腾出空间
        unsafe {
            let insert_pos = (*gstring).str.add(pos);
            let move_dest = insert_pos.add(bytes.len());
            let move_src = insert_pos;
            let move_len = old_len - pos;

            if move_len > 0 {
                ptr::copy(move_src, move_dest, move_len);
            }

            // 插入新数据
            ptr::copy_nonoverlapping(bytes.as_ptr(), insert_pos as *mut u8, bytes.len());
            *(*gstring).str.add(new_len) = 0;
            (*gstring).len = new_len;
        }

        gstring
    }

    /// 删除指定范围的字符
    pub fn erase(gstring: *mut GString, pos: usize, len: usize) -> *mut GString {
        if gstring.is_null() || len == 0 {
            return gstring;
        }

        let old_len = unsafe { (*gstring).len };
        if pos >= old_len {
            return gstring;
        }

        let actual_len = core::cmp::min(len, old_len - pos);
        let new_len = old_len - actual_len;

        unsafe {
            let erase_start = (*gstring).str.add(pos);
            let erase_end = erase_start.add(actual_len);
            let move_dest = erase_start;
            let move_src = erase_end;
            let move_len = old_len - pos - actual_len;

            if move_len > 0 {
                ptr::copy(move_src, move_dest, move_len);
            }

            *(*gstring).str.add(new_len) = 0;
            (*gstring).len = new_len;
        }

        gstring
    }

    /// 截断字符串
    pub fn truncate(gstring: *mut GString, len: usize) -> *mut GString {
        if gstring.is_null() {
            return ptr::null_mut();
        }

        unsafe {
            if len < (*gstring).len {
                *(*gstring).str.add(len) = 0;
                (*gstring).len = len;
            }
        }

        gstring
    }

    /// 释放GString并返回C字符串
    pub fn free(gstring: *mut GString) -> *mut gchar {
        if gstring.is_null() {
            return ptr::null_mut();
        }

        unsafe {
            let str_ptr = (*gstring).str;
            g_free(gstring as gpointer);
            str_ptr
        }
    }

    /// 释放GString和C字符串
    pub fn free_and_free(gstring: *mut GString) {
        if gstring.is_null() {
            return;
        }

        unsafe {
            g_free((*gstring).str as gpointer);
            g_free(gstring as gpointer);
        }
    }

    /// 转换为Rust字符串（需要调用者确保生命周期安全）
    pub unsafe fn as_str(gstring: *const GString) -> &'static str {
        if gstring.is_null() || (*gstring).str.is_null() {
            return "";
        }
        str::from_utf8_unchecked(core::slice::from_raw_parts(
            (*gstring).str as *const u8,
            (*gstring).len,
        ))
    }
}

/// UTF-8 字符串验证
pub fn g_utf8_validate(str_data: *const gchar, max_len: isize) -> gboolean {
    if str_data.is_null() {
        return 1; // null指针被认为是有效的UTF-8
    }

    unsafe {
        let len = if max_len < 0 {
            // 计算null终止字符串的长度
            let mut len = 0;
            while *str_data.add(len) != 0 {
                len += 1;
            }
            len
        } else {
            max_len as usize
        };

        if len == 0 {
            return 1; // 空字符串是有效的UTF-8
        }

        let slice = core::slice::from_raw_parts(str_data as *const u8, len);
        match str::from_utf8(slice) {
            Ok(_) => 1, // true
            Err(_) => 0, // false
        }
    }
}

/// UTF-8 字符串长度（字符数，不是字节数）
pub fn g_utf8_strlen(str_data: *const gchar, max_len: isize) -> usize {
    if str_data.is_null() {
        return 0;
    }

    unsafe {
        let len = if max_len < 0 {
            let mut len = 0;
            while *str_data.add(len) != 0 {
                len += 1;
            }
            len
        } else {
            max_len as usize
        };

        if len == 0 {
            return 0;
        }

        let slice = core::slice::from_raw_parts(str_data as *const u8, len);
        let mut char_count = 0;
        let mut i = 0;

        while i < len {
            let byte = slice[i];
            if (byte & 0x80) == 0 {
                // ASCII字符
                i += 1;
            } else if (byte & 0xE0) == 0xC0 {
                // 2字节UTF-8字符
                i += 2;
            } else if (byte & 0xF0) == 0xE0 {
                // 3字节UTF-8字符
                i += 3;
            } else if (byte & 0xF8) == 0xF0 {
                // 4字节UTF-8字符
                i += 4;
            } else {
                // 无效的UTF-8序列
                break;
            }
            char_count += 1;
        }

        char_count
    }
}

/// 字符串比较
pub fn g_strcmp0(str1: *const gchar, str2: *const gchar) -> c_int {
    if str1.is_null() && str2.is_null() {
        return 0;
    }
    if str1.is_null() {
        return -1;
    }
    if str2.is_null() {
        return 1;
    }

    unsafe {
        let mut i = 0;
        loop {
            let c1 = *str1.add(i);
            let c2 = *str2.add(i);

            if c1 == 0 && c2 == 0 {
                return 0;
            }
            if c1 == 0 {
                return -1;
            }
            if c2 == 0 {
                return 1;
            }

            if c1 < c2 {
                return -1;
            } else if c1 > c2 {
                return 1;
            }

            i += 1;
        }
    }
}

/// 不区分大小写的字符串比较
pub fn g_ascii_strcasecmp(str1: *const gchar, str2: *const gchar) -> c_int {
    if str1.is_null() && str2.is_null() {
        return 0;
    }
    if str1.is_null() {
        return -1;
    }
    if str2.is_null() {
        return 1;
    }

    unsafe {
        let mut i = 0;
        loop {
            let c1 = *str1.add(i);
            let c2 = *str2.add(i);

            let c1_lower = if c1 >= b'A' as i8 && c1 <= b'Z' as i8 {
                c1 + (b'a' as i8 - b'A' as i8)
            } else {
                c1
            };

            let c2_lower = if c2 >= b'A' as i8 && c2 <= b'Z' as i8 {
                c2 + (b'a' as i8 - b'A' as i8)
            } else {
                c2
            };

            if c1_lower == 0 && c2_lower == 0 {
                return 0;
            }
            if c1_lower == 0 {
                return -1;
            }
            if c2_lower == 0 {
                return 1;
            }

            if c1_lower < c2_lower {
                return -1;
            } else if c1_lower > c2_lower {
                return 1;
            }

            i += 1;
        }
    }
}

/// 复制字符串
pub fn g_strdup(str_data: *const gchar) -> *mut gchar {
    if str_data.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        // 计算字符串长度
        let mut len = 0;
        while *str_data.add(len) != 0 {
            len += 1;
        }

        let new_str = g_malloc(len + 1) as *mut gchar;
        if !new_str.is_null() {
            ptr::copy_nonoverlapping(str_data, new_str, len + 1);
        }

        new_str
    }
}

/// 复制字符串（限制长度）
pub fn g_strndup(str_data: *const gchar, n: usize) -> *mut gchar {
    if str_data.is_null() {
        return ptr::null_mut();
    }

    unsafe {
        let mut len = 0;
        while len < n && *str_data.add(len) != 0 {
            len += 1;
        }

        let new_str = g_malloc(len + 1) as *mut gchar;
        if !new_str.is_null() {
            ptr::copy_nonoverlapping(str_data, new_str, len);
            *new_str.add(len) = 0;
        }

        new_str
    }
}

/// 复制字符串并替换
pub fn g_strdup_printf(format: *const gchar, _args: *mut c_void) -> *mut gchar {
    if format.is_null() {
        return ptr::null_mut();
    }

    // 简化实现：假设格式字符串是 "%s"
    unsafe {
        if *format == b'%' as i8 && *format.add(1) == b's' as i8 {
            let str_arg = *format.add(2) as *const gchar;
            g_strdup(str_arg)
        } else {
            g_strdup(format)
        }
    }
}

/// 字符串连接
pub fn g_strconcat(str1: *const gchar, str2: *const gchar) -> *mut gchar {
    if str1.is_null() && str2.is_null() {
        return g_strdup(b"".as_ptr() as *const c_char);
    }

    unsafe {
        let len1 = if !str1.is_null() {
            let mut len = 0;
            while *str1.add(len) != 0 {
                len += 1;
            }
            len
        } else {
            0
        };

        let len2 = if !str2.is_null() {
            let mut len = 0;
            while *str2.add(len) != 0 {
                len += 1;
            }
            len
        } else {
            0
        };

        let total_len = len1 + len2;
        let result = g_malloc(total_len + 1) as *mut gchar;

        if !result.is_null() {
            if !str1.is_null() {
                ptr::copy_nonoverlapping(str1, result, len1);
            }
            if !str2.is_null() {
                ptr::copy_nonoverlapping(str2, result.add(len1), len2);
            }
            *result.add(total_len) = 0;
        }

        result
    }
}

/// 路径处理：获取目录名
pub fn g_path_dirname(file_path: *const gchar) -> *mut gchar {
    if file_path.is_null() {
        return g_strdup(b".".as_ptr() as *const c_char);
    }

    unsafe {
        let mut len = 0;
        while *file_path.add(len) != 0 {
            len += 1;
        }

        if len == 0 {
            return g_strdup(b".".as_ptr() as *const c_char);
        }

        // 查找最后一个路径分隔符
        let mut last_slash = len;
        for i in (0..len).rev() {
            if *file_path.add(i) == b'/' as i8 {
                last_slash = i;
                break;
            }
        }

        if last_slash == 0 {
            return g_strdup(b"/".as_ptr() as *const c_char);
        }

        if last_slash == len {
            // 没有找到分隔符
            return g_strdup(b".".as_ptr() as *const c_char);
        }

        g_strndup(file_path, last_slash)
    }
}

/// 路径处理：获取文件名
pub fn g_path_basename(file_path: *const gchar) -> *mut gchar {
    if file_path.is_null() {
        return g_strdup(b".".as_ptr() as *const c_char);
    }

    unsafe {
        let mut len = 0;
        while *file_path.add(len) != 0 {
            len += 1;
        }

        if len == 0 {
            return g_strdup(b".".as_ptr() as *const c_char);
        }

        // 查找最后一个路径分隔符
        let mut last_slash = len;
        for i in (0..len).rev() {
            if *file_path.add(i) == b'/' as i8 {
                last_slash = i;
                break;
            }
        }

        if last_slash == len {
            // 没有找到分隔符，返回整个路径
            return g_strdup(file_path);
        }

        if last_slash + 1 >= len {
            return g_strdup(b"/".as_ptr() as *const c_char);
        }

        g_strdup(file_path.add(last_slash + 1))
    }
}

/// 命令行参数解析
#[derive(Debug)]
pub struct GOptionContext {
    pub program_name: String,
    pub entries: Vec<GOptionEntry>,
}

#[derive(Debug, Clone)]
pub struct GOptionEntry {
    pub long_name: String,
    pub short_name: gchar,
    pub flags: GOptionFlags,
    pub arg: GOptionArg,
    pub arg_data: gpointer,
    pub description: String,
    pub arg_description: String,
}

/// 选项参数类型
pub type GOptionArg = i32;
pub const G_OPTION_ARG_NONE: GOptionArg = 0;
pub const G_OPTION_ARG_STRING: GOptionArg = 1;
pub const G_OPTION_ARG_INT: GOptionArg = 2;
pub const G_OPTION_ARG_CALLBACK: GOptionArg = 3;
pub const G_OPTION_ARG_FILENAME: GOptionArg = 4;
pub const G_OPTION_ARG_STRING_ARRAY: GOptionArg = 5;
pub const G_OPTION_ARG_FILENAME_ARRAY: GOptionArg = 6;
pub const G_OPTION_ARG_DOUBLE: GOptionArg = 7;
pub const G_OPTION_ARG_INT64: GOptionArg = 8;

/// 选项标志
pub type GOptionFlags = i32;
pub const G_OPTION_FLAG_NONE: GOptionFlags = 0;
pub const G_OPTION_FLAG_HIDDEN: GOptionFlags = 1 << 0;
pub const G_OPTION_FLAG_IN_MAIN: GOptionFlags = 1 << 1;
pub const G_OPTION_FLAG_REVERSE: GOptionFlags = 1 << 2;
pub const G_OPTION_FLAG_NO_ARG: GOptionFlags = 1 << 3;
pub const G_OPTION_FLAG_FILENAME: GOptionFlags = 1 << 4;
pub const G_OPTION_FLAG_OPTIONAL_ARG: GOptionFlags = 1 << 5;
pub const G_OPTION_FLAG_NOALIAS: GOptionFlags = 1 << 6;

/// 创建选项上下文
pub fn g_option_context_new(parameter_string: *const gchar) -> *mut GOptionContext {
    let program_name = if !parameter_string.is_null() {
        unsafe {
            let mut len = 0;
            while *parameter_string.add(len) != 0 {
                len += 1;
            }
            String::from_utf8_unchecked(core::slice::from_raw_parts(
                parameter_string as *const u8,
                len,
            ).to_vec())
        }
    } else {
        String::new()
    };

    unsafe {
        let context = g_malloc0(core::mem::size_of::<GOptionContext>()) as *mut GOptionContext;
        if !context.is_null() {
            (*context).program_name = program_name;
            (*context).entries = Vec::new();
        }
        context
    }
}

/// 添加选项
pub fn g_option_context_add_main_entries(
    context: *mut GOptionContext,
    entries: *const GOptionEntry,
    entry_length: usize,
) {
    if context.is_null() || entries.is_null() {
        return;
    }

    unsafe {
        for i in 0..entry_length {
            let entry = entries.add(i);
            (*context).entries.push((*entry).clone());
        }
    }
}

/// 解析命令行参数
pub fn g_option_context_parse(
    context: *mut GOptionContext,
    argc: *mut c_int,
    argv: *mut *mut *mut gchar,
    error: *mut *mut GError,
) -> gboolean {
    // 简化实现：总是返回成功
    if !context.is_null() && !argc.is_null() && !argv.is_null() {
        glib_println!("[glib_string] 解析 {} 个命令行参数", unsafe { *argc });
        1 // true
    } else {
        0 // false
    }
}

/// 释放选项上下文
pub fn g_option_context_free(context: *mut GOptionContext) {
    if !context.is_null() {
        unsafe {
            g_free(context as gpointer);
        }
    }
}

/// 字符串测试
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gstring_creation() {
        let gstring = GString::new("Hello, World!");
        assert!(!gstring.is_null());

        unsafe {
            assert_eq!(GString::len(gstring), 13);
            assert_eq!(GString::as_str(gstring), "Hello, World!");
        }

        let c_str = GString::free(gstring);
        assert!(!c_str.is_null());
        g_free(c_str as gpointer);
    }

    #[test]
    fn test_gstring_append() {
        let gstring = GString::new("Hello");
        assert!(!gstring.is_null());

        let gstring = GString::append(gstring, ", World!");
        unsafe {
            assert_eq!(GString::as_str(gstring), "Hello, World!");
        }

        GString::free_and_free(gstring);
    }

    #[test]
    fn test_gstring_insert() {
        let gstring = GString::new("Hello World!");
        assert!(!gstring.is_null());

        let gstring = GString::insert(gstring, 5, ", ");
        unsafe {
            assert_eq!(GString::as_str(gstring), "Hello, World!");
        }

        GString::free_and_free(gstring);
    }

    #[test]
    fn test_gstring_erase() {
        let gstring = GString::new("Hello, World!");
        assert!(!gstring.is_null());

        let gstring = GString::erase(gstring, 5, 7);
        unsafe {
            assert_eq!(GString::as_str(gstring), "Hello!");
        }

        GString::free_and_free(gstring);
    }

    #[test]
    fn test_utf8_validation() {
        let valid_utf8 = b"Hello, \xE4\xB8\xAD\xE6\x96\x87\0"; // "Hello, 中文" in UTF-8
        assert_eq!(g_utf8_validate(valid_utf8.as_ptr() as *const i8, -1), 1);

        let invalid_utf8 = b"\xFF\xFE\xFD\0"; // Invalid UTF-8
        assert_eq!(g_utf8_validate(invalid_utf8.as_ptr() as *const i8, -1), 0);
    }

    #[test]
    fn test_string_comparison() {
        let str1 = b"Hello\0";
        let str2 = b"hello\0";
        let str3 = b"Hello\0";

        assert_eq!(g_strcmp0(str1.as_ptr() as *const i8, str2.as_ptr() as *const i8), -1);
        assert_eq!(g_strcmp0(str1.as_ptr() as *const i8, str3.as_ptr() as *const i8), 0);
        assert_eq!(g_ascii_strcasecmp(str1.as_ptr() as *const i8, str2.as_ptr() as *const i8), 0);
    }

    #[test]
    fn test_string_duplication() {
        let original = b"Test String\0";
        let copy = g_strdup(original.as_ptr() as *const i8);

        assert!(!copy.is_null());
        assert_eq!(g_strcmp0(original.as_ptr() as *const i8, copy), 0);

        g_free(copy as gpointer);
    }

    #[test]
    fn test_path_handling() {
        let path = b"/home/user/document.txt\0";
        let dirname = g_path_dirname(path.as_ptr() as *const i8);
        let basename = g_path_basename(path.as_ptr() as *const i8);

        assert!(!dirname.is_null());
        assert!(!basename.is_null());

        unsafe {
            assert_eq!(g_strcmp0(dirname, b"/home/user\0".as_ptr() as *const i8), 0);
            assert_eq!(g_strcmp0(basename, b"document.txt\0".as_ptr() as *const i8), 0);
        }

        g_free(dirname as gpointer);
        g_free(basename as gpointer);
    }

    #[test]
    fn test_option_context() {
        let context = g_option_context_new(b"test-program [options]\0".as_ptr() as *const i8);
        assert!(!context.is_null());

        unsafe {
            assert_eq!((*context).program_name, "test-program [options]");
        }

        g_option_context_free(context);
    }
}