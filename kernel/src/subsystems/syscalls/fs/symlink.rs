//! # 符号链接系统调用实现
//!
//! 提供 symlink() 和 readlink() 系统调用的实现。

extern crate alloc;
use alloc::string::ToString;

use crate::subsystems::fs::vfs::{symlink, Path, error::VfsError};

/// symlink 系统调用
///
/// 创建一个名为 newpath 的符号链接，该链接包含 oldpath 字符串。
///
/// # 参数
///
/// * `oldpath`: 目标路径（符号链接指向的内容）
/// * `newpath`: 新符号链接的路径
///
/// # 返回
///
/// 成功返回 0，失败返回负的错误码
///
/// # 错误
///
/// - `EEXIST`: newpath 已存在
/// - `ENOENT`: newpath 的父目录不存在
/// - `EACCES`: 父目录没有写权限
/// - `ENOSPC`: 文件系统空间不足
/// - `ENAMETOOLONG`: 路径名过长
pub fn sys_symlink(oldpath: &str, newpath: &str) -> isize {
    // 验证路径非空
    if oldpath.is_empty() || newpath.is_empty() {
        crate::println!("symlink: empty path");
        return -14; // EFAULT
    }

    // 验证路径长度
    const PATH_MAX: usize = 4096;
    if oldpath.len() > PATH_MAX || newpath.len() > PATH_MAX {
        crate::println!("symlink: path too long");
        return -36; // ENAMETOOLONG
    }

    // 创建路径对象
    let old = Path::new(oldpath);
    let new = Path::new(newpath);

    // 执行符号链接创建
    match symlink::symlink(&old, &new) {
        Ok(()) => {
            crate::println!("symlink: created '{}' -> '{}'", newpath, oldpath);
            0
        }
        Err(e) => {
            crate::println!("symlink: failed to create '{}': {:?}", newpath, e);
            error_to_errno(e)
        }
    }
}

/// readlink 系统调用
///
/// 读取符号链接的值（即它指向的内容）。
///
/// # 参数
///
/// * `path`: 符号链接路径
/// * `buf`: 输出缓冲区
/// * `bufsize`: 缓冲区大小
///
/// # 返回
///
/// 成功返回放入 buf 的字节数，失败返回负的错误码
///
/// # 错误
///
/// - `ENOENT`: path 不存在
/// - `EINVAL`: path 不是符号链接
/// - `EACCES`: 没有读权限
/// - `ENAMETOOLONG`: 路径名过长
pub fn sys_readlink(path: &str, buf: &mut [u8]) -> isize {
    // 验证路径非空
    if path.is_empty() {
        crate::println!("readlink: empty path");
        return -14; // EFAULT
    }

    // 验证路径长度
    const PATH_MAX: usize = 4096;
    if path.len() > PATH_MAX {
        crate::println!("readlink: path too long");
        return -36; // ENAMETOOLONG
    }

    // 创建路径对象
    let link_path = Path::new(path);

    // 读取符号链接目标
    match symlink::readlink(&link_path) {
        Ok(target) => {
            let target_str = target.to_string();
            let target_bytes = target_str.as_bytes();

            // 检查缓冲区大小
            if buf.len() < target_bytes.len() {
                crate::println!("readlink: buffer too small (need {})", target_bytes.len());
                return -36; // ENAMETOOLONG 或 ERANGE
            }

            // 复制目标到缓冲区
            buf[..target_bytes.len()].copy_from_slice(target_bytes);

            crate::println!(
                "readlink: '{}' -> '{}' ({} bytes)",
                path,
                target_str,
                target_bytes.len()
            );

            target_bytes.len() as isize
        }
        Err(e) => {
            crate::println!("readlink: failed to read '{}': {:?}", path, e);
            error_to_errno(e)
        }
    }
}

/// 将 VfsError 转换为 errno
fn error_to_errno(err: crate::vfs::error::VfsError) -> isize {
    use crate::subsystems::fs::vfs::error::VfsError;

    match err {
        VfsError::NotFound => -2,           // ENOENT
        VfsError::PermissionDenied => -13,  // EACCES
        VfsError::Exists => -17,            // EEXIST
        VfsError::NoSpace => -28,           // ENOSPC
        VfsError::InvalidInput => -22,      // EINVAL
        VfsError::InvalidPath => -36,       // ENAMETOOLONG
        VfsError::NotASymlink => -22,       // EINVAL
        VfsError::Loop => -40,              // ELOOP
        VfsError::TooManyLinks => -40,      // ELOOP
        VfsError::NotSupported => -95,      // EOPNOTSUPP
        _ => {
            crate::println!("symlink: unhandled error: {:?}", err);
            -5 // EIO
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_symlink_empty_path() {
        let result = sys_symlink("", "/tmp/link");
        assert_eq!(result, -14); // EFAULT
    }

    #[test]
    fn test_readlink_empty_path() {
        let mut buf = [0u8; 256];
        let result = sys_readlink("", &mut buf);
        assert_eq!(result, -14); // EFAULT
    }
}
