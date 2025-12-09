//! C标准库环境变量支持
//!
//! 提供完整的stdlib.h环境变量函数支持，包括：
//! - 环境变量操作：getenv, setenv, unsetenv, clearenv
//! - 环境变量遍历
//! - 安全的环境变量管理
//! - 进程环境空间管理
//! - 环境变量权限控制

extern crate alloc;

use alloc::format;
use core::ffi::{c_char, c_int};
use core::str::FromStr;
use crate::libc::error::set_errno;
use crate::libc::error::errno::{EINVAL, ENOMEM};
use crate::reliability::errno::{EPERM, EAGAIN};
use crate::sync::Mutex;

/// 环境变量条目
#[derive(Debug, Clone)]
pub struct EnvEntry {
    /// 变量名
    pub name: heapless::String<256>,
    /// 变量值
    pub value: heapless::String<1024>,
    /// 是否被修改过
    pub modified: bool,
    /// 原始值（用于reset）
    pub original_value: Option<heapless::String<1024>>,
}

/// 环境变量管理器配置
#[derive(Debug, Clone)]
pub struct EnvConfig {
    /// 最大环境变量数量
    pub max_entries: usize,
    /// 最大变量名长度
    pub max_name_length: usize,
    /// 最大变量值长度
    pub max_value_length: usize,
    /// 是否允许修改系统环境变量
    pub allow_sys_modification: bool,
    /// 是否启用安全检查
    pub enable_security_checks: bool,
}

impl Default for EnvConfig {
    fn default() -> Self {
        Self {
            max_entries: 128,
            max_name_length: 256,
            max_value_length: 1024,
            allow_sys_modification: false,
            enable_security_checks: true,
        }
    }
}

/// 环境变量统计信息
#[derive(Debug, Clone, Default)]
pub struct EnvStats {
    /// 当前环境变量数量
    pub current_count: usize,
    /// 历史添加次数
    pub total_additions: usize,
    /// 历史删除次数
    pub total_deletions: usize,
    /// 历史修改次数
    pub total_modifications: usize,
    /// 查询命中次数
    pub query_hits: usize,
    /// 查询未命中次数
    pub query_misses: usize,
}

/// 增强的环境变量管理器
pub struct EnhancedEnvManager {
    /// 配置
    config: EnvConfig,
    /// 统计信息
    stats: Mutex<EnvStats>,
    /// 环境变量表
    env_table: crate::sync::Mutex<heapless::Vec<EnvEntry, 128>>,
    /// 系统环境变量前缀
    sys_prefixes: &'static [&'static str; 5],
}

/// 系统环境变量前缀
static SYSTEM_ENV_PREFIXES: [&str; 5] = [
    "PATH",
    "HOME",
    "USER",
    "SHELL",
    "TERM",
];

impl EnhancedEnvManager {
    /// 创建新的环境变量管理器
    pub fn new(config: EnvConfig) -> Self {
        Self {
            config,
            stats: Mutex::new(EnvStats::default()),
            env_table: crate::sync::Mutex::new(heapless::Vec::new()),
            sys_prefixes: &SYSTEM_ENV_PREFIXES,
        }
    }

    /// 初始化环境变量管理器
    pub fn initialize(&self) -> Result<(), c_int> {
        crate::println!("[env_lib] 初始化环境变量管理器");

        // 加载初始环境变量
        if let Err(e) = self.load_initial_env() {
            crate::println!("[env_lib] 警告：加载初始环境变量失败: {:?}", e);
        }

        crate::println!("[env_lib] 环境变量管理器初始化完成");
        Ok(())
    }

    /// 获取环境变量值
    pub fn getenv(&self, name: *const c_char) -> *const c_char {
        if name.is_null() {
            set_errno(EINVAL);
            return core::ptr::null();
        }

        let name_str = unsafe {
            match core::ffi::CStr::from_ptr(name).to_str() {
                Ok(s) => s,
                Err(_) => {
                    self.stats.lock().query_misses += 1;
                    return core::ptr::null();
                }
            }
        };

        if let Some(mut table) = self.env_table.try_lock() {
            for entry in table.iter() {
                if entry.name == name_str {
                    self.stats.lock().query_hits += 1;
                    return entry.value.as_ptr() as *const c_char;
                }
            }
        }

        self.stats.lock().query_misses += 1;
        core::ptr::null()
    }

    /// 设置或添加环境变量
    pub fn setenv(&self, name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int {
        if name.is_null() || value.is_null() {
            set_errno(EINVAL);
            return -1;
        }

        let name_str = unsafe {
            match core::ffi::CStr::from_ptr(name).to_str() {
                Ok(s) => s,
                Err(_) => {
                    set_errno(EINVAL);
                    return -1;
                }
            }
        };

        let value_str = unsafe {
            match core::ffi::CStr::from_ptr(value).to_str() {
                Ok(s) => s,
                Err(_) => {
                    set_errno(EINVAL);
                    return -1;
                }
            }
        };

        // 长度检查
        if name_str.len() > self.config.max_name_length ||
           value_str.len() > self.config.max_value_length {
            set_errno(ENOMEM);
            return -1;
        }

        // 安全检查
        if self.config.enable_security_checks && self.is_name_dangerous(name_str) {
            set_errno(EINVAL);
            return -1;
        }

        // 检查系统变量修改权限
        if !self.config.allow_sys_modification && self.is_system_variable(name_str) {
            crate::println!("[env_lib] 警告：尝试修改系统环境变量: {}", name_str);
            set_errno(EPERM);
            return -1;
        }

        if let Some(mut table) = self.env_table.try_lock() {
            // 查找现有条目
            for entry in table.iter_mut() {
                if entry.name == name_str {
                    if overwrite != 0 {
                        // 更新现有条目
                        if !entry.modified {
                            entry.original_value = Some(entry.value.clone());
                            entry.modified = true;
                        }
                        entry.value.clear();
                        entry.value.push_str(value_str).ok();
                        self.stats.lock().total_modifications += 1;
                        return 0;
                    } else {
                        // 不允许覆盖
                        return -1;
                    }
                }
            }

            // 添加新条目
            if table.len() < self.config.max_entries {
                let mut entry = EnvEntry {
                    name: heapless::String::from_str(name_str).unwrap_or_else(|_| heapless::String::new()),
                    value: heapless::String::from_str(value_str).unwrap_or_else(|_| heapless::String::new()),
                    modified: true,
                    original_value: None,
                };

                match table.push(entry) {
                    Ok(_) => {}
                    Err(_) => {
                        set_errno(ENOMEM);
                        return -1;
                    }
                }

                self.stats.lock().current_count = table.len();
                self.stats.lock().total_additions += 1;
                return 0;
            } else {
                set_errno(ENOMEM);
                return -1;
            }
        }

        set_errno(EAGAIN);
        -1
    }

    /// 删除环境变量
    pub fn unsetenv(&self, name: *const c_char) -> c_int {
        if name.is_null() {
            set_errno(EINVAL);
            return -1;
        }

        let name_str = unsafe {
            match core::ffi::CStr::from_ptr(name).to_str() {
                Ok(s) => s,
                Err(_) => {
                    set_errno(EINVAL);
                    return -1;
                }
            }
        };

        // 检查系统变量删除权限
        if !self.config.allow_sys_modification && self.is_system_variable(name_str) {
            crate::println!("[env_lib] 警告：尝试删除系统环境变量: {}", name_str);
            set_errno(EPERM);
            return -1;
        }

        if let Some(mut table) = self.env_table.try_lock() {
            for (i, entry) in table.iter().enumerate() {
                if entry.name == name_str {
                    table.remove(i);
                    self.stats.lock().current_count = table.len();
                    self.stats.lock().total_deletions += 1;
                    return 0;
                }
            }
        }

        // 变量不存在，这是正常情况
        0
    }

    /// 清空所有环境变量
    pub fn clearenv(&self) -> c_int {
        if let Some(mut table) = self.env_table.try_lock() {
            let count = table.len();
            table.clear();
            self.stats.lock().current_count = 0;
            self.stats.lock().total_deletions += count;
            crate::println!("[env_lib] 清空了{}个环境变量", count);
            0
        } else {
            set_errno(EAGAIN);
            -1
        }
    }

    /// 获取环境变量统计信息
    pub fn get_stats(&self) -> EnvStats {
        self.stats.lock().clone()
    }

    /// 打印环境变量报告
    pub fn print_env_report(&self) {
        crate::println!("\n=== 环境变量管理器统计报告 ===");

        let stats = self.stats.lock();
        crate::println!("当前变量数量: {}", stats.current_count);
        crate::println!("历史添加次数: {}", stats.total_additions);
        crate::println!("历史删除次数: {}", stats.total_deletions);
        crate::println!("历史修改次数: {}", stats.total_modifications);
        crate::println!("查询命中次数: {}", stats.query_hits);
        crate::println!("查询未命中次数: {}", stats.query_misses);

        if stats.query_hits + stats.query_misses > 0 {
            let hit_rate = (stats.query_hits as f64 /
                           (stats.query_hits + stats.query_misses) as f64) * 100.0;
            crate::println!("查询命中率: {:.2}%", hit_rate);
        }

        // 列出当前环境变量
        if let Some(table) = self.env_table.try_lock() {
            if !table.is_empty() {
                crate::println!("\n当前环境变量:");
                for entry in table.iter() {
                    let status = if entry.modified {
                        if entry.original_value.is_some() {
                            "修改"
                        } else {
                            "新增"
                        }
                    } else {
                        "原始"
                    };
                    crate::println!("  {}={} [{}]", entry.name, entry.value, status);
                }
            }
        }

        crate::println!("============================");
    }

    /// 列出所有环境变量
    pub fn list_variables(&self) -> heapless::Vec<(heapless::String<256>, heapless::String<256>), 128> {
        let mut result = heapless::Vec::new();

        if let Some(table) = self.env_table.try_lock() {
            for entry in table.iter() {
                let name = heapless::String::<256>::from_str(&entry.name).unwrap_or_default();
                let value = heapless::String::<256>::from_str(&entry.value).unwrap_or_default();
                if result.push((name, value)).is_err() {
                    break;
                }
            }
        }

        result
    }

    /// 重置所有修改的环境变量
    pub fn reset_modified(&self) -> c_int {
        let mut reset_count = 0;

        if let Some(mut table) = self.env_table.try_lock() {
            for entry in table.iter_mut() {
                if entry.modified {
                    if let Some(original) = &entry.original_value {
                        entry.value = original.clone();
                        entry.modified = false;
                        reset_count += 1;
                    }
                }
            }
        }

        crate::println!("[env_lib] 重置了{}个环境变量", reset_count);
        0
    }

    /// 导出环境变量为字符串数组
    pub fn export_environ(&self) -> Result<*mut *mut c_char, c_int> {
        if let Some(table) = self.env_table.try_lock() {
            let count = table.len();
            if count == 0 {
                return Ok(core::ptr::null_mut());
            }

            // 分配environ数组
            let array_layout = unsafe {
                core::alloc::Layout::from_size_align(
                    (count + 1) * core::mem::size_of::<*mut c_char>(),
                    core::mem::align_of::<*mut c_char>()
                ).unwrap()
            };
            let array_ptr = unsafe { alloc::alloc::alloc(array_layout) as *mut *mut c_char };

            if array_ptr.is_null() {
                set_errno(ENOMEM);
                return Err(-1);
            }

            // 分配每个环境变量字符串
            for (i, entry) in table.iter().enumerate() {
                let env_string = format!("{}={}", entry.name, entry.value);
                let string_layout = unsafe {
                    core::alloc::Layout::from_size_align(
                        env_string.len() + 1,
                        1
                    ).unwrap()
                };
                let string_ptr = unsafe { alloc::alloc::alloc(string_layout) as *mut c_char };

                if !string_ptr.is_null() {
                    unsafe {
                        core::ptr::copy_nonoverlapping(
                            env_string.as_ptr(),
                            string_ptr as *mut u8,
                            env_string.len()
                        );
                        *string_ptr.add(env_string.len()) = 0;
                    }

                    unsafe {
                        *array_ptr.add(i) = string_ptr;
                    }
                } else {
                    // 清理已分配的内存
                    for j in 0..i {
                        if let ptr = unsafe { *array_ptr.add(j) } {
                            unsafe {
                                let layout = core::alloc::Layout::from_size_align(
                                    self.strlen(ptr) + 1,
                                    1
                                ).unwrap();
                                alloc::alloc::dealloc(ptr as *mut u8, layout);
                            }
                        }
                    }
                    unsafe {
                        alloc::alloc::dealloc(array_ptr as *mut u8, array_layout);
                    }
                    return Err(-1);
                }
            }

            // 添加终止符
            unsafe {
                *array_ptr.add(count) = core::ptr::null_mut();
            }

            Ok(array_ptr)
        } else {
            set_errno(EAGAIN);
            Err(-1)
        }
    }

    /// 检查是否为危险的环境变量名
    fn is_name_dangerous(&self, name: &str) -> bool {
        let dangerous_names = [
            "LD_PRELOAD",
            "LD_LIBRARY_PATH",
            "DYLD_LIBRARY_PATH",
            "DYLD_INSERT_LIBRARIES",
            "IFS",
            "PATH",
            "HOME",
            "PWD",
            "SHELL",
            "USER",
            "DISPLAY",
            "DBUS_SESSION_BUS_ADDRESS",
            "XAUTHORITY",
            "SSH_AGENT_PID",
            "SSH_AUTH_SOCK",
        ];

        dangerous_names.iter().any(|&dangerous| name == dangerous)
    }

    /// 检查是否为系统环境变量
    fn is_system_variable(&self, name: &str) -> bool {
        self.sys_prefixes.iter().any(|&prefix| name.starts_with(prefix))
    }

    /// 加载初始环境变量
    fn load_initial_env(&self) -> Result<(), c_int> {
        // 这里应该从系统加载初始环境变量
        // 暂时设置一些基本的变量

        // 设置PATH变量
        let path_value = "/usr/local/bin:/usr/bin:/bin";
        self.setenv(
            b"PATH\0".as_ptr() as *const c_char,
            path_value.as_ptr() as *const c_char,
            0
        );

        // 设置USER变量
        let user_value = "root";
        self.setenv(
            b"USER\0".as_ptr() as *const c_char,
            user_value.as_ptr() as *const c_char,
            0
        );

        // 设置SHELL变量
        let shell_value = "/bin/sh";
        self.setenv(
            b"SHELL\0".as_ptr() as *const c_char,
            shell_value.as_ptr() as *const c_char,
            0
        );

        Ok(())
    }

    /// 获取字符串长度
    fn strlen(&self, s: *const c_char) -> usize {
        if s.is_null() {
            0
        } else {
            unsafe {
                let mut len = 0;
                let mut ptr = s;
                while *ptr != 0 {
                    len += 1;
                    ptr = ptr.add(1);
                }
                len
            }
        }
    }
}

impl Default for EnhancedEnvManager {
    fn default() -> Self {
        Self::new(EnvConfig::default())
    }
}

// 导出全局环境变量管理器实例
pub static mut ENV_MANAGER: Option<EnhancedEnvManager> = None;

/// 初始化全局环境变量管理器
pub fn init_env_manager() {
    unsafe {
        if ENV_MANAGER.is_none() {
            ENV_MANAGER = Some(EnhancedEnvManager::new(EnvConfig::default()));
        }
    }
}

/// 获取全局环境变量管理器
pub fn get_env_manager() -> &'static mut EnhancedEnvManager {
    unsafe {
        if ENV_MANAGER.is_none() {
            init_env_manager();
        }
        ENV_MANAGER.as_mut().unwrap()
    }
}

// 便捷的环境变量函数包装器
#[inline]
pub fn getenv(name: *const c_char) -> *const c_char {
    unsafe { get_env_manager().getenv(name) }
}

#[inline]
pub fn setenv(name: *const c_char, value: *const c_char, overwrite: c_int) -> c_int {
    unsafe { get_env_manager().setenv(name, value, overwrite) }
}

#[inline]
pub fn unsetenv(name: *const c_char) -> c_int {
    unsafe { get_env_manager().unsetenv(name) }
}

#[inline]
pub fn clearenv() -> c_int {
    unsafe { get_env_manager().clearenv() }
}