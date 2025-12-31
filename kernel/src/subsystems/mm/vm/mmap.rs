//! 内存映射功能
//!
//! 提供mmap/munmap等内存映射相关的系统调用实现

use alloc::vec::Vec;

use crate::subsystems::mm::vm::{VmRegion, VmRegionType, VmError, PAGE_SIZE};
use crate::subsystems::mm::PhysAddr;
use crate::subsystems::syscalls::common::SyscallResult;
use crate::types::MapFlags;
use crate::error::SyscallError;

/// Physical frame type - alias for physical address
pub type PhysFrame = PhysAddr;

/// Convert VmError to SyscallError
impl From<VmError> for SyscallError {
    fn from(err: VmError) -> Self {
        match err {
            VmError::InvalidAddress => SyscallError::BadAddress,
            VmError::OutOfMemory => SyscallError::OutOfMemory,
            VmError::PermissionDenied => SyscallError::PermissionDenied,
            VmError::Overlap => SyscallError::InvalidArgument,
            VmError::Alignment => SyscallError::InvalidArgument,
            VmError::Other => SyscallError::IoError,
        }
    }
}

/// 创建内存映射
///
/// # 参数
/// - `addr`: 建议的起始地址（可以为NULL）
/// - `length`: 映射长度
/// - `flags`: 映射标志
/// - `fd`: 文件描述符（-1表示匿名映射）
/// - `offset`: 文件偏移量
///
/// # 返回
/// 成功时返回映射的起始地址，失败时返回错误
pub fn sys_mmap(
    addr: Option<usize>,
    length: usize,
    flags: u32,
    fd: isize,
    _offset: usize,
) -> SyscallResult<i64> {
    // 获取当前地址空间
    let vm_space = crate::subsystems::mm::vm::vm_manager().lock()
        .current_space()
        .map_err(|_| SyscallError::InvalidArgument)?;

    // 对齐长度到页边界
    let aligned_length = (length + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;

    // 确定映射地址
    let map_addr = if let Some(addr) = addr {
        // 对齐到页边界
        (addr + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE
    } else {
        // 自动分配地址
        vm_space.allocate(aligned_length).map_err(|_| SyscallError::OutOfMemory)?
    };

    // 转换flags为MapFlags
    let map_flags = MapFlags::from_bits(flags);

    // 创建内存区域
    let region = if fd == -1 {
        // 匿名映射
        VmRegion {
            start: map_addr,
            end: map_addr + aligned_length,
            frames: allocate_physical_frames(aligned_length / PAGE_SIZE)?,
            flags: map_flags,
            offset: 0,
            region_type: VmRegionType::Anonymous,
        }
    } else {
        // 文件映射
        // GH-#1094: 实现文件映射
        // See: https://github.com/npos/kernel/issues/1094
        return Err(SyscallError::NotSupported);
    };

    // 添加到地址空间
    vm_space.add_region(region.clone()).map_err(|_| SyscallError::OutOfMemory)?;

    // 映射物理页到虚拟地址
    for (i, &frame) in region.frames.iter().enumerate() {
        let virt = region.start + i * PAGE_SIZE;
        vm_space.map(virt, frame, map_flags).map_err(|_| SyscallError::OutOfMemory)?;
    }

    Ok(map_addr as i64)
}

/// 取消内存映射
///
/// # 参数
/// - `addr`: 映射的起始地址
/// - `length`: 取消映射的长度
///
/// # 返回
/// 成功时返回0，失败时返回错误
pub fn sys_munmap(addr: usize, length: usize) -> SyscallResult<i64> {
    // 获取当前地址空间
    let vm_space = crate::subsystems::mm::vm::vm_manager().lock()
        .current_space()
        .map_err(|_| SyscallError::InvalidArgument)?;

    // 对齐长度到页边界
    let aligned_length = (length + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;

    // 查找内存区域
    let region = vm_space.find_region(addr).ok_or(SyscallError::InvalidArgument)?;

    // 检查长度是否匹配
    if region.end - region.start != aligned_length {
        return Err(SyscallError::InvalidArgument);
    }

    // 取消映射所有页
    for i in 0..(aligned_length / PAGE_SIZE) {
        let virt = region.start + i * PAGE_SIZE;
        vm_space.unmap(virt).map_err(|_| SyscallError::InvalidArgument)?;
    }

    // 移除内存区域
    vm_space.remove_region(region.start).map_err(|_| SyscallError::InvalidArgument)?;

    // 释放物理页
    free_physical_frames(&region.frames);

    Ok(0)
}

/// 重新映射内存
///
/// # 参数
/// - `old_addr`: 旧地址
/// - `old_size`: 旧大小
/// - `new_size`: 新大小
/// - `flags`: 标志（可选）
///
/// # 返回
/// 成功时返回新的地址，失败时返回错误
pub fn sys_mremap(
    old_addr: usize,
    old_size: usize,
    new_size: usize,
    _flags: Option<u32>,
) -> SyscallResult<i64> {
    // 获取当前地址空间
    let vm_space = crate::subsystems::mm::vm::vm_manager().lock()
        .current_space()
        .map_err(|_| SyscallError::InvalidArgument)?;

    // 对齐大小到页边界
    let aligned_old_size = (old_size + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;
    let aligned_new_size = (new_size + PAGE_SIZE - 1) / PAGE_SIZE * PAGE_SIZE;

    // 查找旧区域
    let old_region = vm_space.find_region(old_addr).ok_or(SyscallError::InvalidArgument)?;

    // 如果大小相同，直接返回
    if aligned_old_size == aligned_new_size {
        return Ok(old_addr as i64);
    }

    // 如果新大小更小，截断区域
    if aligned_new_size < aligned_old_size {
        // 取消映射多余的页
        for i in (aligned_new_size / PAGE_SIZE)..(aligned_old_size / PAGE_SIZE) {
            let virt = old_region.start + i * PAGE_SIZE;
            vm_space.unmap(virt).map_err(|_| SyscallError::InvalidArgument)?;
        }

        // 更新区域
        let mut new_region = old_region.clone();
        new_region.end = new_region.start + aligned_new_size;
        new_region.frames.truncate(aligned_new_size / PAGE_SIZE);

        vm_space.remove_region(old_region.start).map_err(|_| SyscallError::InvalidArgument)?;
        vm_space.add_region(new_region).map_err(|_| SyscallError::InvalidArgument)?;

        return Ok(old_addr as i64);
    }

    // 新大小更大，需要扩展
    // 检查是否有足够空间
    let new_end = old_region.start + aligned_new_size;
    if new_end > vm_space.end_addr {
        return Err(SyscallError::OutOfMemory);
    }

    // 分配新的物理页
    let additional_frames = allocate_physical_frames(
        (aligned_new_size - aligned_old_size) / PAGE_SIZE
    )?;

    // 创建新区域
    let mut new_region = old_region.clone();
    new_region.end = new_end;
    new_region.frames.extend(additional_frames.iter());

    // 映射新的页
    for (i, &frame) in additional_frames.iter().enumerate() {
        let virt = old_region.start + aligned_old_size + i * PAGE_SIZE;
        let flags = old_region.flags;
        vm_space.map(virt, frame, flags).map_err(|_| SyscallError::OutOfMemory)?;
    }

    // 更新区域
    vm_space.remove_region(old_region.start).map_err(|_| SyscallError::InvalidArgument)?;
    vm_space.add_region(new_region).map_err(|_| SyscallError::InvalidArgument)?;

    Ok(old_addr as i64)
}

/// 同步内存映射到文件
///
/// # 参数
/// - `addr`: 起始地址
/// - `length`: 长度
/// - `flags`: 同步标志
///
/// # 返回
/// 成功时返回0，失败时返回错误
pub fn sys_msync(addr: usize, _length: usize, _flags: u32) -> SyscallResult<i64> {
    // 获取当前地址空间
    let vm_space = crate::subsystems::mm::vm::vm_manager().lock()
        .current_space()
        .map_err(|_| SyscallError::InvalidArgument)?;

    // 查找内存区域
    let region = vm_space.find_region(addr).ok_or(SyscallError::InvalidArgument)?;

    // 如果是文件映射，同步到文件
    if region.region_type == VmRegionType::File {
        // GH-#1095: 实现文件同步
        // See: https://github.com/npos/kernel/issues/1095
        return Err(SyscallError::NotSupported);
    }

    // 匿名映射不需要同步
    Ok(0)
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 分配物理页帧
fn allocate_physical_frames(count: usize) -> Result<Vec<PhysFrame>, VmError> {
    let mut frames = Vec::with_capacity(count);

    for _ in 0..count {
        // GH-#1096: 实现真正的物理页分配
        // See: https://github.com/npos/kernel/issues/1096
        // 这里使用占位符
        frames.push(0);
    }

    Ok(frames)
}

/// 释放物理页帧
fn free_physical_frames(_frames: &[PhysFrame]) {
    // GH-#1097: 实现真正的物理页释放
    // See: https://github.com/npos/kernel/issues/1097
}

// ============================================================================
// 单元测试
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mmap_anonymous() {
        let result = sys_mmap(None, 4096, MapFlags::readable(), -1, 0);
        assert!(result.is_ok());
    }

    #[test]
    fn test_munmap() {
        // 首先创建映射
        let addr = sys_mmap(None, 4096, MapFlags::readable(), -1, 0).unwrap();

        // 然后取消映射
        let result = sys_munmap(addr as usize, 4096);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mremap_expand() {
        // 创建初始映射
        let addr = sys_mmap(None, 4096, MapFlags::readable(), -1, 0).unwrap();

        // 扩展映射
        let result = sys_mremap(addr as usize, 4096, 8192, None);
        assert!(result.is_ok());
    }

    #[test]
    fn test_mremap_shrink() {
        // 创建初始映射
        let addr = sys_mmap(None, 8192, MapFlags::readable(), -1, 0).unwrap();

        // 缩小映射
        let result = sys_mremap(addr as usize, 8192, 4096, None);
        assert!(result.is_ok());
    }
}
