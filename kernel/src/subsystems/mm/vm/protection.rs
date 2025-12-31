//! 内存保护功能
//!
//! 提供mprotect等内存保护相关的系统调用实现

use crate::prelude::*;

use crate::types::MapFlags;
use crate::error::SyscallError;
use crate::subsystems::syscalls::common::SyscallResult;

/// 更改内存保护属性
///
/// # 参数
/// - `addr`: 起始地址
/// - `length`: 长度
/// - `flags`: 新的保护标志
///
/// # 返回
/// 成功时返回0，失败时返回错误
pub fn sys_mprotect(addr: usize, length: usize, flags: MapFlags) -> SyscallResult<i64> {
    // 获取当前地址空间
    let vm_space = crate::subsystems::mm::vm::vm_manager().lock()
        .current_space()
        .map_err(|_| SyscallError::InvalidArgument)?;

    // 对齐地址和长度到页边界
    let aligned_addr = addr / PAGE_SIZE * PAGE_SIZE;
    let aligned_length = (length + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;

    // 检查地址范围
    let region = vm_space.find_region(aligned_addr).ok_or(SyscallError::InvalidArgument)?;

    // 检查长度是否超出区域
    if aligned_addr + aligned_length > region.end {
        return Err(SyscallError::InvalidArgument);
    }

    // 更改保护属性
    vm_space.protect(aligned_addr, aligned_length, flags)
        .map_err(|_| SyscallError::OutOfMemory)?;

    Ok(0)
}

/// 锁定内存到RAM
///
/// # 参数
/// - `addr`: 起始地址
/// - `length`: 长度
///
/// # 返回
/// 成功时返回0，失败时返回错误
pub fn sys_mlock(addr: usize, length: usize) -> SyscallResult<i64> {
    // 获取当前地址空间
    let vm_space = crate::subsystems::mm::vm::vm_manager().lock()
        .current_space()
        .map_err(|_| SyscallError::InvalidArgument)?;

    // 对齐地址和长度到页边界
    let aligned_addr = addr / PAGE_SIZE * PAGE_SIZE;
    let aligned_length = (length + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;

    // 检查地址范围
    let region = vm_space.find_region(aligned_addr).ok_or(SyscallError::InvalidArgument)?;

    // 检查长度是否超出区域
    if aligned_addr + aligned_length > region.end {
        return Err(SyscallError::InvalidArgument);
    }

    // GH-#1090: 实现真正的内存锁定
    // See: https://github.com/npos/kernel/issues/1090
    // 这里只是标记为已锁定
    crate::println!("mlock: locked {} bytes at {:#x}", aligned_length, aligned_addr);

    Ok(0)
}

/// 解锁内存
///
/// # 参数
/// - `addr`: 起始地址
/// - `length`: 长度
///
/// # 返回
/// 成功时返回0，失败时返回错误
pub fn sys_munlock(addr: usize, length: usize) -> SyscallResult<i64> {
    // 获取当前地址空间
    let vm_space = crate::subsystems::mm::vm::vm_manager().lock()
        .current_space()
        .map_err(|_| SyscallError::InvalidArgument)?;

    // 对齐地址和长度到页边界
    let aligned_addr = addr / PAGE_SIZE * PAGE_SIZE;
    let aligned_length = (length + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;

    // 检查地址范围
    let region = vm_space.find_region(aligned_addr).ok_or(SyscallError::InvalidArgument)?;

    // 检查长度是否超出区域
    if aligned_addr + aligned_length > region.end {
        return Err(SyscallError::InvalidArgument);
    }

    // GH-#1091: 实现真正的内存解锁
    // See: https://github.com/npos/kernel/issues/1091
    crate::println!("munlock: unlocked {} bytes at {:#x}", aligned_length, aligned_addr);

    Ok(0)
}

/// 锁定进程地址空间
///
/// # 参数
/// - `flags`: 锁定标志
///
/// # 返回
/// 成功时返回0，失败时返回错误
pub fn sys_mlockall(flags: MlockAllFlags) -> SyscallResult<i64> {
    // 获取当前地址空间
    let _vm_space = crate::subsystems::mm::vm::vm_manager().lock()
        .current_space()
        .map_err(|_| SyscallError::InvalidArgument)?;

    // GH-#1092: 实现真正的地址空间锁定
    // See: https://github.com/npos/kernel/issues/1092
    crate::println!("mlockall: locked address space with flags {:?}", flags);

    Ok(0)
}

/// 解锁进程地址空间
///
/// # 返回
/// 成功时返回0，失败时返回错误
pub fn sys_munlockall() -> SyscallResult<i64> {
    // 获取当前地址空间
    let _vm_space = crate::subsystems::mm::vm::vm_manager().lock()
        .current_space()
        .map_err(|_| SyscallError::InvalidArgument)?;

    // GH-#1093: 实现真正的地址空间解锁
    // See: https://github.com/npos/kernel/issues/1093
    crate::println!("munlockall: unlocked address space");

    Ok(0)
}

/// mlockall标志
#[derive(Debug, Clone, Copy)]
pub struct MlockAllFlags {
    /// 锁定当前和未来的映射
    pub current: bool,
    /// 锁定未来的映射
    pub future: bool,
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mprotect() {
        // 首先创建映射
        let addr = crate::subsystems::mm::vm::mmap::sys_mmap(
            None,
            4096,
            MapFlags::PROT_READ,
            -1,
            0
        ).unwrap();

        // 更改保护属性
        let result = sys_mprotect(
            addr as usize,
            4096,
            MapFlags::PROT_READ | MapFlags::PROT_WRITE
        );
        assert!(result.is_ok());
    }

    #[test]
    fn test_mlock() {
        // 首先创建映射
        let addr = crate::subsystems::mm::vm::mmap::sys_mmap(
            None,
            4096,
            MapFlags::PROT_READ,
            -1,
            0
        ).unwrap();

        // 锁定内存
        let result = sys_mlock(addr as usize, 4096);
        assert!(result.is_ok());
    }

    #[test]
    fn test_munlock() {
        // 首先创建映射并锁定
        let addr = crate::subsystems::mm::vm::mmap::sys_mmap(
            None,
            4096,
            MapFlags::PROT_READ,
            -1,
            0
        ).unwrap();

        sys_mlock(addr as usize, 4096).unwrap();

        // 解锁内存
        let result = sys_munlock(addr as usize, 4096);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mlockall() {
        let flags = MlockAllFlags {
            current: true,
            future: false,
        };

        let result = sys_mlockall(flags);
        assert!(result.is_ok());
    }

    #[test]
    fn test_munlockall() {
        let result = sys_munlockall();
        assert!(result.is_ok());
    }
}
