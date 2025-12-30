//! # VFS Inode Operations
//!
//! 定义虚拟文件系统的核心 inode 操作接口。
//!
//! ## 功能
//!
//! - **InodeOps trait**: 定义所有文件系统必须实现的核心操作
//! - **默认实现**: 为某些操作提供合理的默认行为
//! - **扩展支持**: 支持文件锁、扩展属性等高级功能

extern crate alloc;

use crate::prelude::*;
use alloc::sync::Arc;

// Import from vfs_interface to avoid type conflicts
use crate::vfs_interface::{FileAttr, FileMode, VfsError, DirEntry};

// Import VfsResult type alias from vfs module
use crate::vfs::VfsResult;

/// Inode 操作 trait
///
/// 所有文件系统实现必须实现此 trait 以提供文件和目录操作。
/// 此 trait 定义了 VFS 层与具体文件系统之间的接口。
pub trait InodeOps: Send + Sync {
    /// 获取文件属性
    ///
    /// 返回文件的元数据，包括权限、大小、时间戳等。
    fn getattr(&self) -> VfsResult<FileAttr>;

    /// 设置文件属性
    ///
    /// 更新文件的元数据。
    fn setattr(&self, attr: &FileAttr) -> VfsResult<()> {
        let _ = attr;
        Err(VfsError::NotSupported)
    }

    /// 在目录中查找条目
    ///
    /// 在当前目录中查找给定的名称，返回对应的 inode。
    fn lookup(&self, name: &str) -> VfsResult<Arc<dyn InodeOps>> {
        let _ = name;
        Err(VfsError::NotADirectory)
    }

    /// 创建常规文件
    ///
    /// 在当前目录中创建一个新文件。
    fn create(&self, name: &str, mode: FileMode) -> VfsResult<Arc<dyn InodeOps>> {
        let _ = (name, mode);
        Err(VfsError::NotSupported)
    }

    /// 创建目录
    ///
    /// 在当前目录中创建一个新目录。
    fn mkdir(&self, name: &str, mode: FileMode) -> VfsResult<Arc<dyn InodeOps>> {
        let _ = (name, mode);
        Err(VfsError::NotSupported)
    }

    /// 删除文件
    ///
    /// 从当前目录中删除一个文件。
    fn unlink(&self, name: &str) -> VfsResult<()> {
        let _ = name;
        Err(VfsError::NotSupported)
    }

    /// 删除目录
    ///
    /// 从当前目录中删除一个目录。目录必须为空。
    fn rmdir(&self, name: &str) -> VfsResult<()> {
        let _ = name;
        Err(VfsError::NotSupported)
    }

    /// 检查目录是否为空
    fn is_empty(&self) -> VfsResult<bool> {
        Ok(true)
    }

    /// 创建硬链接
    ///
    /// 在当前目录中为现有 inode 创建一个硬链接。
    fn link(&self, name: &str, inode: Arc<dyn InodeOps>) -> VfsResult<()> {
        let _ = (name, inode);
        Err(VfsError::NotSupported)
    }

    /// 创建符号链接
    ///
    /// 在当前目录中创建一个指向 target 的符号链接。
    fn symlink(&self, name: &str, target: &str) -> VfsResult<Arc<dyn InodeOps>> {
        let _ = (name, target);
        Err(VfsError::NotSupported)
    }

    /// 读取符号链接目标
    ///
    /// 返回符号链接指向的目标路径。
    fn readlink(&self) -> VfsResult<String> {
        Err(VfsError::NotASymlink)
    }

    /// 读取目录内容
    ///
    /// 返回目录中的条目列表。offset 用于分页。
    fn readdir(&self, offset: usize) -> VfsResult<Vec<DirEntry>> {
        let _ = offset;
        Err(VfsError::NotDirectory)
    }

    /// 读取文件内容
    ///
    /// 从文件中读取数据到缓冲区。offset 是读取起始位置。
    /// 返回实际读取的字节数。
    fn read(&self, offset: u64, buf: &mut [u8]) -> VfsResult<usize> {
        let _ = (offset, buf);
        Err(VfsError::IsADirectory)
    }

    /// 写入文件内容
    ///
    /// 将数据从缓冲区写入文件。offset 是写入起始位置。
    /// 返回实际写入的字节数。
    fn write(&self, offset: u64, buf: &[u8]) -> VfsResult<usize> {
        let _ = (offset, buf);
        Err(VfsError::IsADirectory)
    }

    /// 截断文件
    ///
    /// 将文件大小设置为指定的大小。
    fn truncate(&self, size: u64) -> VfsResult<()> {
        let _ = size;
        Err(VfsError::NotSupported)
    }

    /// 获取文件锁
    ///
    /// 尝试获取文件锁。返回锁 ID 或错误。
    fn get_file_lock(&self, cmd: u32, lock: &FileLock) -> VfsResult<u64> {
        let _ = (cmd, lock);
        Err(VfsError::NotSupported)
    }

    /// 释放文件锁
    ///
    /// 释放之前获取的文件锁。
    fn release_file_lock(&self, lock: &FileLock) -> VfsResult<()> {
        let _ = lock;
        Err(VfsError::NotSupported)
    }

    /// 设置扩展属性
    ///
    /// 为文件设置扩展属性。
    fn set_xattr(&self, name: &str, value: &[u8], flags: u32) -> VfsResult<()> {
        let _ = (name, value, flags);
        Err(VfsError::NotSupported)
    }

    /// 获取扩展属性
    ///
    /// 获取文件的扩展属性值。
    fn get_xattr(&self, name: &str, value: &mut [u8]) -> VfsResult<usize> {
        let _ = (name, value);
        Err(VfsError::NotSupported)
    }

    /// 删除扩展属性
    ///
    /// 删除文件的扩展属性。
    fn remove_xattr(&self, name: &str) -> VfsResult<()> {
        let _ = name;
        Err(VfsError::NotSupported)
    }

    /// 列出扩展属性
    ///
    /// 列出文件的所有扩展属性名称。
    fn list_xattr(&self, list: &mut [u8]) -> VfsResult<usize> {
        let _ = list;
        Err(VfsError::NotSupported)
    }
}

/// 文件锁结构
///
/// 用于文件锁操作的结构体。
#[derive(Debug, Clone)]
pub struct FileLock {
    /// 锁类型 (读锁/写锁)
    pub lock_type: u16,
    /// 锁的起始位置
    pub start: u64,
    /// 锁的结束位置
    pub len: u64,
    /// 进程 ID
    pub pid: u32,
}

impl FileLock {
    /// 创建新的文件锁
    pub fn new(lock_type: u16, start: u64, len: u64, pid: u32) -> Self {
        Self {
            lock_type,
            start,
            len,
            pid,
        }
    }

    /// 创建读锁
    pub fn shared(start: u64, len: u64, pid: u32) -> Self {
        Self {
            lock_type: 0, // F_RDLCK
            start,
            len,
            pid,
        }
    }

    /// 创建写锁
    pub fn exclusive(start: u64, len: u64, pid: u32) -> Self {
        Self {
            lock_type: 1, // F_WRLCK
            start,
            len,
            pid,
        }
    }

    /// 创建解锁
    pub fn unlock(start: u64, len: u64, pid: u32) -> Self {
        Self {
            lock_type: 2, // F_UNLCK
            start,
            len,
            pid,
        }
    }
}
