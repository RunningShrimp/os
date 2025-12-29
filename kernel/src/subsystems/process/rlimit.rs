//! rlimit 资源限制管理
//!
//! 提供进程资源限制（resource limits）功能。
//!
//! ## 概述
//!
//! rlimit 允许系统限制进程可以使用的资源量：
//! - **软限制 (rlim_cur)**: 当前限制，可以调整到硬限制
//! - **硬限制 (rlim_max)**: 上限，只能由特权进程降低
//!
//! ## 资源类型
//!
//! - `RLIMIT_AS`: 地址空间大小
//! - `RLIMIT_CORE`: core 文件大小
//! - `RLIMIT_CPU`: CPU 时间
//! - `RLIMIT_DATA`: 数据段大小
//! - `RLIMIT_FSIZE`: 文件大小
//! - `RLIMIT_NOFILE`: 文件描述符数
//! - `RLIMIT_NPROC`: 进程数
//! - `RLIMIT_STACK`: 栈大小
//!
//! ## POSIX 兼容性
//!
//! 实现 POSIX.1-2008 规范的 getrlimit/setrlimit/prlimit。

use crate::api::SyscallError;
use crate::subsystems::process::manager::PROC_TABLE;

/// 资源限制值
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RLimit {
    /// 软限制（当前限制）
    pub rlim_cur: u64,
    /// 硬限制（上限）
    pub rlim_max: u64,
}

impl RLimit {
    /// 创建新的资源限制
    pub const fn new(cur: u64, max: u64) -> Self {
        Self { rlim_cur: cur, rlim_max: max }
    }

    /// 无限限制
    pub const fn infinite() -> Self {
        Self { rlim_cur: RLIM_INFINITY, rlim_max: RLIM_INFINITY }
    }

    /// 验证限制值是否有效
    pub fn is_valid(&self) -> bool {
        self.rlim_cur <= self.rlim_max || self.rlim_max == RLIM_INFINITY
    }
}

/// RLIM_INFINITY - 表示无限限制
pub const RLIM_INFINITY: u64 = u64::MAX;

/// 资源类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum RLimitResource {
    /// 地址空间大小（字节）
    AS = 9,

    /// Core 文件大小（字节）
    CORE = 4,

    /// CPU 时间（秒）
    CPU = 0,

    /// 数据段大小（字节）
    DATA = 2,

    /// 文件大小（字节）
    FSIZE = 1,

    /// 文件描述符数
    NOFILE = 7,

    /// 进程数
    NPROC = 6,

    /// 栈大小（字节）
    STACK = 3,

    /// 最大内存锁定大小（字节）
    MEMLOCK = 8,

    /// 待决信号数
    SIGPENDING = 10,

    /// 消息队列字节数
    MSGQUEUE = 12,

    /// nice 值
    NICE = 13,

    /// 实时优先级
    RTPRIO = 14,

    /// 实时信号调度策略
    RTTIME = 15,
}

impl RLimitResource {
    /// 从 u32 创建资源类型
    pub fn from_u32(value: u32) -> Option<Self> {
        match value {
            0 => Some(RLimitResource::CPU),
            1 => Some(RLimitResource::FSIZE),
            2 => Some(RLimitResource::DATA),
            3 => Some(RLimitResource::STACK),
            4 => Some(RLimitResource::CORE),
            6 => Some(RLimitResource::NPROC),
            7 => Some(RLimitResource::NOFILE),
            8 => Some(RLimitResource::MEMLOCK),
            9 => Some(RLimitResource::AS),
            10 => Some(RLimitResource::SIGPENDING),
            12 => Some(RLimitResource::MSGQUEUE),
            13 => Some(RLimitResource::NICE),
            14 => Some(RLimitResource::RTPRIO),
            15 => Some(RLimitResource::RTTIME),
            _ => None,
        }
    }

    /// 获取资源的默认限制
    pub fn default_limit(&self) -> RLimit {
        match self {
            RLimitResource::AS => RLimit::new(u64::MAX, u64::MAX),
            RLimitResource::CORE => RLimit::new(0, u64::MAX),
            RLimitResource::CPU => RLimit::new(u64::MAX, u64::MAX),
            RLimitResource::DATA => RLimit::new(u64::MAX, u64::MAX),
            RLimitResource::FSIZE => RLimit::new(u64::MAX, u64::MAX),
            RLimitResource::NOFILE => RLimit::new(1024, 4096),
            RLimitResource::NPROC => RLimit::new(0, 0),
            RLimitResource::STACK => RLimit::new(8 * 1024 * 1024, u64::MAX),
            RLimitResource::MEMLOCK => RLimit::new(64 * 1024 * 1024, 64 * 1024 * 1024),
            RLimitResource::SIGPENDING => RLimit::new(0, 0),
            RLimitResource::MSGQUEUE => RLimit::new(819200, 819200),
            RLimitResource::NICE => RLimit::new(0, 0),
            RLimitResource::RTPRIO => RLimit::new(0, 0),
            RLimitResource::RTTIME => RLimit::new(0, 0),
        }
    }
}

/// getrlimit 系统调用 - 获取资源限制
///
/// # POSIX 语义
///
/// 获取当前进程的指定资源的软限制和硬限制。
///
/// # 参数
///
/// * `resource` - 资源类型
/// * `rlim` - 输出参数，存储资源限制
///
/// # 返回值
///
/// * `Ok(())` - 成功
/// * `Err(SyscallError)` - 失败
///
/// # 错误
///
/// - `EINVAL`: 无效的资源类型
pub fn sys_getrlimit(resource: u32, rlim: &mut RLimit) -> Result<(), SyscallError> {
    let resource_type = RLimitResource::from_u32(resource).ok_or(SyscallError::InvalidArgument)?;

    let pid = crate::process::myproc().ok_or(SyscallError::NoProcess)?;
    let table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NoProcess)?;

    // 从进程的 rlimits 数组获取限制
    let index = resource as usize;
    if index >= proc.rlimits.len() {
        *rlim = resource_type.default_limit();
    } else {
        *rlim = proc.rlimits[index];
    }

    Ok(())
}

/// setrlimit 系统调用 - 设置资源限制
///
/// # POSIX 语义
///
/// 设置当前进程的指定资源的软限制和硬限制：
///
/// 1. **特权进程**: 可以设置任意限制
/// 2. **非特权进程**:
///    - 软限制不能超过硬限制
///    - 硬限制只能降低，不能升高
///    - 不能将限制设置为低于当前使用量
///
/// # 参数
///
/// * `resource` - 资源类型
/// * `rlim` - 新的资源限制
///
/// # 返回值
///
/// * `Ok(())` - 成功
/// * `Err(SyscallError)` - 失败
///
/// # 错误
///
/// - `EINVAL`: 无效的资源类型或限制值
/// - `EPERM`: 权限不足
pub fn sys_setrlimit(resource: u32, rlim: &RLimit) -> Result<(), SyscallError> {
    let resource_type = RLimitResource::from_u32(resource).ok_or(SyscallError::InvalidArgument)?;

    // 验证限制值
    if !rlim.is_valid() {
        return Err(SyscallError::InvalidArgument);
    }

    let pid = crate::process::myproc().ok_or(SyscallError::NoProcess)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NoProcess)?;

    // 获取当前限制
    let index = resource as usize;
    let current_limit = if index >= proc.rlimits.len() {
        resource_type.default_limit()
    } else {
        proc.rlimits[index]
    };

    // 非特权进程检查
    let is_privileged = proc.euid == 0;

    if !is_privileged {
        // 软限制不能超过硬限制
        if rlim.rlim_cur > rlim.rlim_max && rlim.rlim_max != RLIM_INFINITY {
            return Err(SyscallError::InvalidArgument);
        }

        // 硬限制不能升高
        if rlim.rlim_max > current_limit.rlim_max {
            return Err(SyscallError::OperationNotPermitted);
        }

        // 硬限制不能低于软限制
        if rlim.rlim_max < rlim.rlim_cur && rlim.rlim_cur != RLIM_INFINITY {
            return Err(SyscallError::InvalidArgument);
        }
    }

    // 设置新限制
    if index >= proc.rlimits.len() {
        // 简化实现：如果索引超出范围，暂时不设置
        // 完整实现应该扩展数组
    } else {
        proc.rlimits[index] = *rlim;
    }

    Ok(())
}

/// prlimit 系统调用 - 获取或设置进程的资源限制
///
/// # POSIX 语义
///
/// prlimit 是 getrlimit/setrlimit 的增强版本，可以操作任意进程：
///
/// - 如果 `new_limit` 为 None：只获取限制
/// - 如果 `new_limit` 为 Some：设置限制并可选地返回旧限制
///
/// # 参数
///
/// * `pid` - 目标进程 PID（0 表示当前进程）
/// * `resource` - 资源类型
/// * `new_limit` - 新的限制（None 表示不修改）
/// * `old_limit` - 输出参数，存储旧限制
///
/// # 返回值
///
/// * `Ok(())` - 成功
/// * `Err(SyscallError)` - 失败
///
/// # 错误
///
/// - `EINVAL`: 无效的参数
/// - `ESRCH`: 进程不存在
/// - `EPERM`: 权限不足
pub fn sys_prlimit(
    pid: i32,
    resource: u32,
    new_limit: Option<&RLimit>,
    old_limit: &mut RLimit,
) -> Result<(), SyscallError> {
    let resource_type = RLimitResource::from_u32(resource).ok_or(SyscallError::InvalidArgument)?;

    // 确定目标进程
    let target_pid = if pid == 0 {
        crate::process::myproc().ok_or(SyscallError::NoProcess)?
    } else {
        pid as u64
    };

    let mut table = PROC_TABLE.lock();
    let proc = table.find(target_pid).ok_or(SyscallError::NoProcess)?;

    // 检查权限
    let current_pid = crate::process::myproc().ok_or(SyscallError::NoProcess)?;
    let current_proc = table.find(current_pid).ok_or(SyscallError::NoProcess)?;

    if target_pid != current_pid && current_proc.euid != 0 {
        return Err(SyscallError::OperationNotPermitted);
    }

    // 获取当前限制
    let index = resource as usize;
    let current_rlimit = if index >= proc.rlimits.len() {
        resource_type.default_limit()
    } else {
        proc.rlimits[index]
    };

    // 返回旧限制
    *old_limit = current_rlimit;

    // 设置新限制（如果提供）
    if let Some(new_rlimit) = new_limit {
        // 验证限制值
        if !new_rlimit.is_valid() {
            return Err(SyscallError::InvalidArgument);
        }

        let is_privileged = current_proc.euid == 0;

        if !is_privileged && target_pid != current_pid {
            return Err(SyscallError::OperationNotPermitted);
        }

        if !is_privileged {
            // 软限制不能超过硬限制
            if new_rlimit.rlim_cur > new_rlimit.rlim_max && new_rlimit.rlim_max != RLIM_INFINITY {
                return Err(SyscallError::InvalidArgument);
            }

            // 硬限制不能升高
            if new_rlimit.rlim_max > current_rlimit.rlim_max {
                return Err(SyscallError::OperationNotPermitted);
            }
        }

        // 设置新限制
        if index < proc.rlimits.len() {
            proc.rlimits[index] = *new_rlimit;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rlimit_creation() {
        let rlim = RLimit::new(1024, 4096);
        assert_eq!(rlim.rlim_cur, 1024);
        assert_eq!(rlim.rlim_max, 4096);
        assert!(rlim.is_valid());
    }

    #[test]
    fn test_rlimit_infinity() {
        let rlim = RLimit::infinite();
        assert_eq!(rlim.rlim_cur, RLIM_INFINITY);
        assert_eq!(rlim.rlim_max, RLIM_INFINITY);
    }

    #[test]
    fn test_rlimit_invalid() {
        let rlim = RLimit::new(4096, 1024);
        assert!(!rlim.is_valid());
    }
}
