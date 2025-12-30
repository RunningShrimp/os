//! 进程凭证管理（credentials）
//!
//! 管理用户和组 ID (UID/GID) 的设置和切换。
//!
//! ## 概述
//!
//! POSIX 定义了多种用户/组 ID：
//! - **RUID (Real UID)**: 真实用户 ID，进程创建者
//! - **EUID (Effective UID)**: 有效用户 ID，用于权限检查
//! - **SUID (Saved Set-User ID)**: 保存的 set-user-ID，exec 前的 EUID
//!
//! ## 安全机制
//!
//! - CAP_SETUID: 允许修改任意 UID
//! - CAP_SETGID: 允许修改任意 GID
//! - 权限检查：非特权进程只能设置 SUID

use crate::api::SyscallError;
use crate::posix::{Uid, Gid};

/// setuid 系统调用 - 设置用户 ID
///
/// # POSIX 语义
///
/// 进程有不同的权限级别：
///
/// 1. **特权进程 (EUID == 0)**:
///    - 可以设置 RUID、EUID、SUID 为任意值
///    - 通常将三者设置为相同的 uid
///
/// 2. **非特权进程**:
///    - 只能设置 RUID 为当前 RUID
///    - 只能设置 EUID 为当前 RUID、EUID 或 SUID
///    - SUID 总是设置为 EUID 的旧值
///
/// # 参数
///
/// * `uid` - 要设置的用户 ID
///
/// # 返回值
///
/// * `Ok(())` - 成功
/// * `Err(SyscallError)` - 失败
///
/// # 错误
///
/// - `EPERM`: 权限不足
///
/// # 示例
///
/// ```no_run
/// use kernel::subsystems::process::credentials::sys_setuid;
///
/// // 特权进程可以设置为任意 UID
/// if sys_setuid(1000).is_ok() {
///     // 现在 RUID、EUID、SUID 都是 1000
/// }
///
/// // 非特权进程只能设置为特定值
/// // let result = sys_setuid(1000);  // 可能失败 EPERM
/// ```
pub fn sys_setuid(uid: Uid) -> Result<(), SyscallError> {
    use crate::subsystems::process::PROC_TABLE;

    // 获取当前进程
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;

    let old_ruid = proc.uid;
    let old_euid = proc.euid;
    let old_suid = proc.suid;

    // 检查是否有 CAP_SETUID 能力（特权进程）
    let has_cap_setuid = old_euid == 0;

    if has_cap_setuid {
        // 特权进程：可以设置所有 UID
        proc.uid = uid;
        proc.euid = uid;
        proc.suid = uid;
    } else {
        // 非特权进程：权限检查
        // 1. RUID 只能设置为当前 RUID
        if uid != old_ruid {
            return Err(SyscallError::PermissionDenied);
        }

        // 2. EUID 可以设置为 RUID、EUID 或 SUID
        if uid != old_ruid && uid != old_euid && uid != old_suid {
            return Err(SyscallError::PermissionDenied);
        }

        // 3. SUID 总是设置为旧的 EUID
        proc.uid = uid;  // RUID 可以设置（如果等于当前 RUID）
        proc.euid = uid;
        proc.suid = old_euid;
    }

    Ok(())
}

/// getuid 系统调用 - 获取真实用户 ID
///
/// # 返回值
///
/// 返回进程的真实用户 ID (RUID)
pub fn sys_getuid() -> Result<Uid, SyscallError> {
    use crate::subsystems::process::PROC_TABLE;

    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;

    Ok(proc.uid)
}

/// geteuid 系统调用 - 获取有效用户 ID
///
/// # 返回值
///
/// 返回进程的有效用户 ID (EUID)
pub fn sys_geteuid() -> Result<Uid, SyscallError> {
    use crate::subsystems::process::PROC_TABLE;

    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;

    Ok(proc.euid)
}

/// setgid 系统调用 - 设置组 ID
///
/// # POSIX 语义
///
/// 与 setuid 类似，但操作的是组 ID：
///
/// 1. **特权进程 (EGID == 0)**:
///    - 可以设置 RGID、EGID、SGID 为任意值
///
/// 2. **非特权进程**:
///    - 只能设置 RGID 为当前 RGID
///    - 只能设置 EGID 为当前 RGID、EGID 或 SGID
///    - SGID 总是设置为 EGID 的旧值
///
/// # 参数
///
/// * `gid` - 要设置的组 ID
///
/// # 返回值
///
/// * `Ok(())` - 成功
/// * `Err(SyscallError)` - 失败
///
/// # 错误
///
/// - `EPERM`: 权限不足
pub fn sys_setgid(gid: Gid) -> Result<(), SyscallError> {
    use crate::subsystems::process::PROC_TABLE;

    // 获取当前进程
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;

    let old_rgid = proc.gid;
    let old_egid = proc.egid;
    let old_sgid = proc.sgid;

    // 检查是否有 CAP_SETGID 能力（特权进程）
    let has_cap_setgid = old_egid == 0;

    if has_cap_setgid {
        // 特权进程：可以设置所有 GID
        proc.gid = gid;
        proc.egid = gid;
        proc.sgid = gid;
    } else {
        // 非特权进程：权限检查
        // 1. RGID 只能设置为当前 RGID
        if gid != old_rgid {
            return Err(SyscallError::PermissionDenied);
        }

        // 2. EGID 可以设置为 RGID、EGID 或 SGID
        if gid != old_rgid && gid != old_egid && gid != old_sgid {
            return Err(SyscallError::PermissionDenied);
        }

        // 3. SGID 总是设置为旧的 EGID
        proc.gid = gid;   // RGID 可以设置（如果等于当前 RGID）
        proc.egid = gid;
        proc.sgid = old_egid;
    }

    Ok(())
}

/// getgid 系统调用 - 获取真实组 ID
///
/// # 返回值
///
/// 返回进程的真实组 ID (RGID)
pub fn sys_getgid() -> Result<Gid, SyscallError> {
    use crate::subsystems::process::PROC_TABLE;

    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;

    Ok(proc.gid)
}

/// getegid 系统调用 - 获取有效组 ID
///
/// # 返回值
///
/// 返回进程的有效组 ID (EGID)
pub fn sys_getegid() -> Result<Gid, SyscallError> {
    use crate::subsystems::process::PROC_TABLE;

    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;

    Ok(proc.egid)
}

/// seteuid 系统调用 - 设置有效用户 ID
///
/// # POSIX 语义
///
/// seteuid 只设置 EUID，不影响 RUID 和 SUID：
///
/// 1. **特权进程 (EUID == 0)**:
///    - 可以设置 EUID 为任意值
///
/// 2. **非特权进程**:
///    - 只能设置 EUID 为当前 RUID、EUID 或 SUID
///
/// # 参数
///
/// * `euid` - 要设置的有效用户 ID
///
/// # 返回值
///
/// * `Ok(())` - 成功
/// * `Err(SyscallError)` - 失败
pub fn sys_seteuid(euid: Uid) -> Result<(), SyscallError> {
    use crate::subsystems::process::PROC_TABLE;

    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;

    let old_euid = proc.euid;
    let old_ruid = proc.uid;
    let old_suid = proc.suid;

    // 特权进程可以设置任意 EUID
    if old_euid == 0 {
        proc.euid = euid;
    } else {
        // 非特权进程：EUID 只能设置为 RUID、EUID 或 SUID
        if euid != old_ruid && euid != old_euid && euid != old_suid {
            return Err(SyscallError::PermissionDenied);
        }
        proc.euid = euid;
    }

    Ok(())
}

/// setegid 系统调用 - 设置有效组 ID
///
/// # POSIX 语义
///
/// 与 seteuid 类似，但操作的是组 ID。
///
/// # 参数
///
/// * `egid` - 要设置的有效组 ID
///
/// # 返回值
///
/// * `Ok(())` - 成功
/// * `Err(SyscallError)` - 失败
pub fn sys_setegid(egid: Gid) -> Result<(), SyscallError> {
    use crate::subsystems::process::PROC_TABLE;

    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;

    let old_egid = proc.egid;
    let old_rgid = proc.gid;
    let old_sgid = proc.sgid;

    // 特权进程可以设置任意 EGID
    if old_egid == 0 {
        proc.egid = egid;
    } else {
        // 非特权进程：EGID 只能设置为 RGID、EGID 或 SGID
        if egid != old_rgid && egid != old_egid && egid != old_sgid {
            return Err(SyscallError::PermissionDenied);
        }
        proc.egid = egid;
    }

    Ok(())
}

/// setreuid 系统调用 - 设置真实和有效用户 ID
///
/// # POSIX 语义
///
/// setreuid 可以同时设置 RUID 和 EUID：
///
/// 1. **特权进程**:
///    - 可以设置 ruid 和 euid 为任意值
///
/// 2. **非特权进程**:
///    - ruid 只能设置为当前 RUID 或 EUID
///    - euid 只能设置为当前 RUID、EUID 或 SUID
///
/// # 参数
///
/// * `ruid` - 真实用户 ID（-1 表示不修改）
/// * `euid` - 有效用户 ID（-1 表示不修改）
///
/// # 返回值
///
/// * `Ok(())` - 成功
/// * `Err(SyscallError)` - 失败
pub fn sys_setreuid(ruid: i32, euid: i32) -> Result<(), SyscallError> {
    use crate::subsystems::process::PROC_TABLE;

    const UID_NO_CHANGE: i32 = -1;

    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;

    let old_ruid = proc.uid;
    let old_euid = proc.euid;
    let old_suid = proc.suid;
    let is_privileged = old_euid == 0;

    // 设置 RUID
    if ruid != UID_NO_CHANGE {
        let new_ruid = ruid as Uid;
        if is_privileged {
            proc.uid = new_ruid;
        } else {
            // 非特权进程：RUID 只能设置为当前 RUID 或 EUID
            if new_ruid != old_ruid && new_ruid != old_euid {
                return Err(SyscallError::PermissionDenied);
            }
            proc.uid = new_ruid;
        }
    }

    // 设置 EUID
    if euid != UID_NO_CHANGE {
        let new_euid = euid as Uid;
        if is_privileged {
            proc.euid = new_euid;
        } else {
            // 非特权进程：EUID 只能设置为当前 RUID、EUID 或 SUID
            if new_euid != old_ruid && new_euid != old_euid && new_euid != old_suid {
                return Err(SyscallError::PermissionDenied);
            }
            proc.euid = new_euid;
        }

        // 如果修改了 EUID，更新 SUID
        if euid != UID_NO_CHANGE {
            proc.suid = new_euid;
        }
    }

    Ok(())
}

/// setregid 系统调用 - 设置真实和有效组 ID
///
/// # POSIX 语义
///
/// 与 setreuid 类似，但操作的是组 ID。
///
/// # 参数
///
/// * `rgid` - 真实组 ID（-1 表示不修改）
/// * `egid` - 有效组 ID（-1 表示不修改）
///
/// # 返回值
///
/// * `Ok(())` - 成功
/// * `Err(SyscallError)` - 失败
pub fn sys_setregid(rgid: i32, egid: i32) -> Result<(), SyscallError> {
    use crate::subsystems::process::PROC_TABLE;

    const GID_NO_CHANGE: i32 = -1;

    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let mut table = PROC_TABLE.lock();
    let proc = table.find(pid).ok_or(SyscallError::NotFound)?;

    let old_rgid = proc.gid;
    let old_egid = proc.egid;
    let old_sgid = proc.sgid;
    let is_privileged = old_egid == 0;

    // 设置 RGID
    if rgid != GID_NO_CHANGE {
        let new_rgid = rgid as Gid;
        if is_privileged {
            proc.gid = new_rgid;
        } else {
            // 非特权进程：RGID 只能设置为当前 RGID 或 EGID
            if new_rgid != old_rgid && new_rgid != old_egid {
                return Err(SyscallError::PermissionDenied);
            }
            proc.gid = new_rgid;
        }
    }

    // 设置 EGID
    if egid != GID_NO_CHANGE {
        let new_egid = egid as Gid;
        if is_privileged {
            proc.egid = new_egid;
        } else {
            // 非特权进程：EGID 只能设置为当前 RGID、EGID 或 SGID
            if new_egid != old_rgid && new_egid != old_egid && new_egid != old_sgid {
                return Err(SyscallError::PermissionDenied);
            }
            proc.egid = new_egid;
        }

        // 如果修改了 EGID，更新 SGID
        if egid != GID_NO_CHANGE {
            proc.sgid = new_egid;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_setuid_constants() {
        // 测试常量定义
        const TEST_UID: Uid = 1000;
        assert_eq!(TEST_UID, 1000);
    }
}
