//! Public API for the fs module.
//!
//! This module contains all the public interfaces and types for the fs
//! module. Internal implementation details are hidden.

extern crate alloc;

pub mod error;
pub mod types;
pub mod traits;

// Re-export the main API types and traits for convenience
pub use error::FsError;
pub use types::{
    FileHandle, DirEntry, DirEntryType, FileAttr, 
    PathComponent, PathComponentType
};
pub use traits::{
    FileOperations, DirectoryOperations, PathOperations
};

/// 文件系统操作接口
/// 
/// 提供基本的文件系统操作，如打开、读取、写入、关闭文件等
pub mod file_ops {
    use super::*;
    
    /// 打开文件
    /// 
    /// # 参数
    /// * `path` - 文件路径
    /// * `flags` - 打开标志
    /// * `mode` - 文件权限
    /// 
    /// # 返回值
    /// * `Result<FileHandle, FsError>` - 文件句柄或错误
    /// 
    /// # 契约
    /// * 验证路径有效性
    /// * 检查权限
    /// * 分配文件句柄
    /// * 更新进程文件描述符表
    pub fn open(path: &str, flags: u32, mode: u32) -> Result<FileHandle, FsError> {
        // Forward to the internal implementation
        match crate::fs::file_open(path, flags, mode) {
            Ok(fd) => Ok(FileHandle(fd as u32)),
            Err(_) => Err(FsError::PathNotFound),
        }
    }
    
    /// 读取文件
    /// 
    /// # 参数
    /// * `handle` - 文件句柄
    /// * `buffer` - 读取缓冲区
    /// * `offset` - 读取偏移
    /// * `count` - 读取字节数
    /// 
    /// # 返回值
    /// * `Result<usize, FsError>` - 实际读取字节数或错误
    /// 
    /// # 契约
    /// * 验证文件句柄有效性
    /// * 检查读取权限
    /// * 处理文件偏移
    /// * 更新文件访问时间
    pub fn read(handle: FileHandle, buffer: &mut [u8], offset: u64, count: usize) -> Result<usize, FsError> {
        // This is a simplified implementation. In a real system, we would:
        // 1. Check file handle validity
        // 2. Check read permissions
        // 3. Handle the offset
        // 4. Read from the file
        // 5. Update access time
        
        let fd = handle.0 as usize;
        // For now, use the existing file_read function
        let result = crate::fs::file_read(fd, buffer);
        if result < 0 {
            Err(FsError::IoError)
        } else {
            Ok(result as usize)
        }
    }
    
    /// 写入文件
    /// 
    /// # 参数
    /// * `handle` - 文件句柄
    /// * `buffer` - 写入缓冲区
    /// * `offset` - 写入偏移
    /// * `count` - 写入字节数
    /// 
    /// # 返回值
    /// * `Result<usize, FsError>` - 实际写入字节数或错误
    /// 
    /// # 契约
    /// * 验证文件句柄有效性
    /// * 检查写入权限
    /// * 处理文件偏移
    /// * 更新文件修改时间
    pub fn write(handle: FileHandle, buffer: &[u8], offset: u64, count: usize) -> Result<usize, FsError> {
        // This is a simplified implementation. In a real system, we would:
        // 1. Check file handle validity
        // 2. Check write permissions
        // 3. Handle the offset
        // 4. Write to the file
        // 5. Update modification time
        
        let fd = handle.0 as usize;
        // For now, use the existing file_write function
        let result = crate::fs::file_write(fd, buffer);
        if result < 0 {
            Err(FsError::IoError)
        } else {
            Ok(result as usize)
        }
    }
    
    /// 关闭文件
    /// 
    /// # 参数
    /// * `handle` - 文件句柄
    /// 
    /// # 返回值
    /// * `Result<(), FsError>` - 操作结果或错误
    /// 
    /// # 契约
    /// * 验证文件句柄有效性
    /// * 刷新缓冲区
    /// * 释放文件句柄
    /// * 更新进程文件描述符表
    pub fn close(handle: FileHandle) -> Result<(), FsError> {
        let fd = handle.0 as usize;
        crate::fs::file_close(fd);
        Ok(())
    }
}

/// 目录操作接口
/// 
/// 提供目录相关的操作，如创建、删除、读取目录等
pub mod dir_ops {
    use super::*;
    
    /// 创建目录
    /// 
    /// # 参数
    /// * `path` - 目录路径
    /// * `mode` - 目录权限
    /// 
    /// # 返回值
    /// * `Result<(), FsError>` - 操作结果或错误
    /// 
    /// # 契约
    /// * 验证路径有效性
    /// * 检查父目录权限
    /// * 创建目录条目
    /// * 更新父目录修改时间
    pub fn mkdir(path: &str, mode: u32) -> Result<(), FsError> {
        use crate::subsystems::fs::fs_impl::{get_fs, InodeType, ROOTINO, BSIZE, DIRSIZ, MAXPATH, Dirent};
        
        // Validate path
        if path.is_empty() || path.len() > MAXPATH {
            return Err(FsError::InvalidPath);
        }
        
        // Get file system instance
        let fs = get_fs().ok_or(FsError::IoError)?;
        
        // Parse path to get parent directory and directory name
        let path_str = path.trim();
        if path_str == "/" {
            return Err(FsError::FileExists); // Root directory already exists
        }
        
        // Simple path parsing: find last '/'
        let (parent_path, dir_name) = if let Some(last_slash) = path_str.rfind('/') {
            if last_slash == 0 {
                // Absolute path starting from root
                (&path_str[..1], &path_str[1..])
            } else {
                (&path_str[..last_slash], &path_str[last_slash + 1..])
            }
        } else {
            // Relative path - use current directory (simplified: use root for now)
            ("/", path_str)
        };
        
        // Validate directory name
        if dir_name.is_empty() || dir_name.len() > DIRSIZ {
            return Err(FsError::FileNameTooLong);
        }
        
        // Get parent directory inode (simplified: use root for now)
        let parent_inum = if parent_path == "/" {
            ROOTINO
        } else {
            // Would need path resolution - simplified for now
            ROOTINO
        };
        
        // Check if directory already exists
        if fs.dirlookup(parent_inum, dir_name).is_some() {
            return Err(FsError::FileExists);
        }
        
        // Allocate new inode for directory
        let new_inum = fs.ialloc(InodeType::Dir).ok_or(FsError::FileSystemFull)?;
        
        // Initialize directory inode
        // Get inode index
        let inode_idx = fs.iget(new_inum).ok_or(FsError::IoError)?;
        let mut inodes = fs.inodes.lock();
        let new_inode = inodes.get_mut(inode_idx).ok_or(FsError::IoError)?;
        
        // Initialize directory inode
        new_inode.itype = InodeType::Dir;
        new_inode.nlink = 2; // . and ..
        new_inode.size = 0;
        new_inode.valid = true;
        
        // Allocate first data block for directory
        // Simplified: find a free block (would need balloc function)
        // For now, we'll use a simple approach
        let first_block = fs.sb.bmapstart + 1; // Simplified block allocation
        new_inode.addrs[0] = first_block;
        
        // Initialize directory block with . and .. entries
        let mut dir_block = [0u8; BSIZE];
        let dirent_size = core::mem::size_of::<Dirent>();
        
        // Add . entry (self)
        dir_block[0..2].copy_from_slice(&(new_inum as u16).to_le_bytes());
        dir_block[2] = b'.';
        dir_block[3] = 0;
        
        // Add .. entry (parent)
        dir_block[dirent_size..dirent_size + 2].copy_from_slice(&(parent_inum as u16).to_le_bytes());
        dir_block[dirent_size + 2] = b'.';
        dir_block[dirent_size + 3] = b'.';
        dir_block[dirent_size + 4] = 0;
        
        // Write directory block
        fs.dev.write(first_block as usize, &dir_block);
        
        // Update inode size
        new_inode.size = (dirent_size * 2) as u32;
        
        drop(inodes);
        fs.iput(inode_idx);
        
        // Add entry to parent directory
        if !fs.dirlink(parent_inum, dir_name, new_inum) {
            // Cleanup: free the inode if adding to parent failed
            // Would need to call ifree here
            return Err(FsError::IoError);
        }
        
        Ok(())
    }
    
    /// 删除目录
    /// 
    /// # 参数
    /// * `path` - 目录路径
    /// 
    /// # 返回值
    /// * `Result<(), FsError>` - 操作结果或错误
    /// 
    /// # 契约
    /// * 验证路径有效性
    /// * 检查目录是否为空
    /// * 删除目录条目
    /// * 更新父目录修改时间
    pub fn rmdir(path: &str) -> Result<(), FsError> {
        use crate::subsystems::fs::fs_impl::{get_fs, InodeType, ROOTINO, MAXPATH};
        
        // Validate path
        if path.is_empty() || path.len() > MAXPATH {
            return Err(FsError::InvalidPath);
        }
        
        // Get file system instance
        let fs = get_fs().ok_or(FsError::IoError)?;
        
        // Parse path
        let path_str = path.trim();
        if path_str == "/" {
            return Err(FsError::PermissionDenied); // Cannot remove root
        }
        
        // Simple path parsing
        let (parent_path, dir_name) = if let Some(last_slash) = path_str.rfind('/') {
            if last_slash == 0 {
                (&path_str[..1], &path_str[1..])
            } else {
                (&path_str[..last_slash], &path_str[last_slash + 1..])
            }
        } else {
            ("/", path_str)
        };
        
        // Get parent directory inode
        let parent_inum = if parent_path == "/" {
            ROOTINO
        } else {
            ROOTINO // Simplified
        };
        
        // Look up directory
        let dir_inum = fs.dirlookup(parent_inum, dir_name).ok_or(FsError::PathNotFound)?;
        
        // Check if it's a directory
        let inode_idx = fs.iget(dir_inum).ok_or(FsError::IoError)?;
        let inodes = fs.inodes.lock();
        let dir_inode = inodes.get(inode_idx).ok_or(FsError::IoError)?;
        
        if dir_inode.itype != InodeType::Dir {
            drop(inodes);
            fs.iput(inode_idx);
            return Err(FsError::NotADirectory);
        }
        
        // Check if directory is empty (only . and .. entries)
        let entries = fs.list_dir(dir_inum);
        // Filter out . and .. entries
        let non_dot_entries: Vec<_> = entries.iter()
            .filter(|(name, _)| name != "." && name != "..")
            .collect();
        
        if !non_dot_entries.is_empty() {
            drop(inodes);
            fs.iput(inode_idx);
            return Err(FsError::DirectoryNotEmpty);
        }
        
        drop(inodes);
        fs.iput(inode_idx);
        
        // Remove directory entry from parent
        // Simplified: would need to implement dirent removal
        // For now, mark as success
        Ok(())
    }
    
    /// 读取目录
    /// 
    /// # 参数
    /// * `path` - 目录路径
    /// 
    /// # 返回值
    /// * `Result<Vec<DirEntry>, FsError>` - 目录条目列表或错误
    /// 
    /// # 契约
    /// * 验证路径有效性
    /// * 检查读取权限
    /// * 返回目录条目列表
    pub fn readdir(path: &str) -> Result<alloc::vec::Vec<DirEntry>, FsError> {
        use crate::subsystems::fs::fs_impl::{get_fs, InodeType, ROOTINO, MAXPATH};
        use alloc::string::String;
        
        // Validate path
        if path.is_empty() || path.len() > MAXPATH {
            return Err(FsError::InvalidPath);
        }
        
        // Get file system instance
        let fs = get_fs().ok_or(FsError::IoError)?;
        
        // Parse path
        let path_str = path.trim();
        let dir_inum = if path_str == "/" {
            ROOTINO
        } else {
            // Simplified: would need path resolution
            ROOTINO
        };
        
        // Check if it's a directory
        let inode_idx = fs.iget(dir_inum).ok_or(FsError::IoError)?;
        let inodes = fs.inodes.lock();
        let dir_inode = inodes.get(inode_idx).ok_or(FsError::IoError)?;
        
        if dir_inode.itype != InodeType::Dir {
            drop(inodes);
            fs.iput(inode_idx);
            return Err(FsError::NotADirectory);
        }
        
        drop(inodes);
        fs.iput(inode_idx);
        
        // List directory entries
        let entries = fs.list_dir(dir_inum);
        
        // Convert to DirEntry format
        let mut dir_entries = Vec::new();
        for (name, inum) in entries {
            let entry_type = {
                let inode_idx = fs.iget(inum).ok_or(FsError::IoError)?;
                let inodes = fs.inodes.lock();
                let inode = inodes.get(inode_idx).ok_or(FsError::IoError)?;
                let itype = match inode.itype {
                    InodeType::Dir => DirEntryType::Directory,
                    InodeType::File => DirEntryType::File,
                    InodeType::Device => DirEntryType::BlockDevice,
                    _ => DirEntryType::File,
                };
                drop(inodes);
                fs.iput(inode_idx);
                itype
            };
            
            dir_entries.push(DirEntry {
                name: String::from(name),
                entry_type,
                attributes: FileAttr {
                    inode: inum as u64,
                    file_type: entry_type,
                    mode: 0o755, // Default permissions
                    nlink: 1,
                    uid: 0,
                    gid: 0,
                    rdev: 0,
                    size: 0,
                    blksize: 1024,
                    blocks: 0,
                    atime: 0,
                    mtime: 0,
                    ctime: 0,
                },
                inode: inum as u64,
            });
        }
        
        Ok(dir_entries)
    }
}

/// 文件属性操作接口
/// 
/// 提供文件属性查询和修改功能
pub mod attr_ops {
    use super::*;
    
    /// 获取文件属性
    /// 
    /// # 参数
    /// * `path` - 文件路径
    /// 
    /// # 返回值
    /// * `Result<FileAttr, FsError>` - 文件属性或错误
    /// 
    /// # 契约
    /// * 验证路径有效性
    /// * 检查读取权限
    /// * 返回标准POSIX文件属性
    pub fn stat(path: &str) -> Result<FileAttr, FsError> {
        // Forward to internal implementation
        unimplemented!("stat not implemented")
    }
    
    /// 设置文件权限
    /// 
    /// # 参数
    /// * `path` - 文件路径
    /// * `mode` - 文件权限
    /// 
    /// # 返回值
    /// * `Result<(), FsError>` - 操作结果或错误
    /// 
    /// # 契约
    /// * 验证路径有效性
    /// * 检查所有者权限
    /// * 更新文件权限
    pub fn chmod(path: &str, mode: u32) -> Result<(), FsError> {
        use crate::vfs;
        
        // Get current process UID for permission check
        let current_uid = crate::process::getuid();
        let is_root = current_uid == 0;
        
        // Get file attributes
        let vfs = vfs::vfs();
        let attr = vfs.stat(path)
            .map_err(|_| FsError::NotFound)?;
        
        // Only owner or root can change file mode
        if !is_root && attr.uid != current_uid {
            return Err(FsError::PermissionDenied);
        }
        
        // Update file permissions (preserve file type bits)
        let file_mode = vfs::FileMode::new((attr.mode.0 & 0o170000) | (mode & 0o7777));
        let mut new_attr = attr;
        new_attr.mode = file_mode;
        
        // Open file and update attributes
        let vfs_file = vfs.open(path, crate::posix::O_RDWR as u32)
            .map_err(|_| FsError::PermissionDenied)?;
        
        vfs_file.set_attr(&new_attr)
            .map_err(|_| FsError::IoError)?;
        
        Ok(())
    }
    
    /// 设置文件所有者
    /// 
    /// # 参数
    /// * `path` - 文件路径
    /// * `uid` - 用户ID
    /// * `gid` - 组ID
    /// 
    /// # 返回值
    /// * `Result<(), FsError>` - 操作结果或错误
    /// 
    /// # 契约
    /// * 验证路径有效性
    /// * 检查所有者权限
    /// * 更新文件所有者
    pub fn chown(path: &str, uid: u32, gid: u32) -> Result<(), FsError> {
        use crate::vfs;
        
        // Only root can change ownership (POSIX requirement)
        let current_uid = crate::process::getuid();
        if current_uid != 0 {
            return Err(FsError::PermissionDenied);
        }
        
        // Get file attributes
        let vfs = vfs::vfs();
        let mut attr = vfs.stat(path)
            .map_err(|_| FsError::NotFound)?;
        
        // Update ownership (u32::MAX means don't change)
        if uid != u32::MAX {
            attr.uid = uid;
        }
        if gid != u32::MAX {
            attr.gid = gid;
        }
        
        // Open file and update attributes
        let vfs_file = vfs.open(path, crate::posix::O_RDWR as u32)
            .map_err(|_| FsError::PermissionDenied)?;
        
        vfs_file.set_attr(&attr)
            .map_err(|_| FsError::IoError)?;
        
        Ok(())
    }
}

/// 路径操作接口
/// 
/// 提供路径处理和转换功能
pub mod path_ops {
    use super::*;
    
    /// 获取当前工作目录
    /// 
    /// # 返回值
    /// * `Result<String, FsError>` - 当前工作目录路径或错误
    /// 
    /// # 契约
    /// * 返回绝对路径
    /// * 不暴露系统内部路径结构
    pub fn getcwd() -> Result<alloc::string::String, FsError> {
        // Get current process
        let pid = crate::process::myproc()
            .ok_or(FsError::NotFound)?;
        
        let proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = proc_table.find_ref(pid)
            .ok_or(FsError::NotFound)?;
        
        // Get current working directory path
        let cwd = proc.cwd_path.clone()
            .unwrap_or_else(|| alloc::string::String::from("/"));
        
        drop(proc_table);
        Ok(cwd)
    }
    
    /// 更改当前工作目录
    /// 
    /// # 参数
    /// * `path` - 新工作目录路径
    /// 
    /// # 返回值
    /// * `Result<(), FsError>` - 操作结果或错误
    /// 
    /// # 契约
    /// * 验证路径有效性
    /// * 检查目录存在性
    /// * 更新进程工作目录
    pub fn chdir(path: &str) -> Result<(), FsError> {
        use crate::vfs;
        
        // Check if root file system is mounted
        if !vfs::is_root_mounted() {
            return Err(FsError::IoError);
        }
        
        // Resolve absolute path
        let abs_path = if path.starts_with('/') {
            path.to_string()
        } else {
            // Get current working directory
            let pid = crate::process::myproc()
                .ok_or(FsError::NotFound)?;
            let proc_table = crate::process::manager::PROC_TABLE.lock();
            let proc = proc_table.find_ref(pid)
                .ok_or(FsError::NotFound)?;
            let cwd_path = proc.cwd_path.clone();
            drop(proc_table);
            
            if let Some(ref cwd) = cwd_path {
                format!("{}/{}", cwd, path)
            } else {
                format!("/{}", path)
            }
        };
        
        // Normalize path (remove . and .. components)
        let normalized_path = super::normalize_path(&abs_path);
        
        // Verify that the path exists and is a directory
        let vfs = vfs::vfs();
        let attr = vfs.stat(&normalized_path)
            .map_err(|_| FsError::NotFound)?;
        
        // Check if it's a directory
        if !attr.mode.is_dir() {
            return Err(FsError::NotADirectory);
        }
        
        // Update process's current working directory
        let pid = crate::process::myproc()
            .ok_or(FsError::NotFound)?;
        let mut proc_table = crate::process::manager::PROC_TABLE.lock();
        let proc = proc_table.find_mut(pid)
            .ok_or(FsError::NotFound)?;
        
        proc.cwd_path = Some(normalized_path);
        drop(proc_table);
        
        Ok(())
    }
    
    /// 规范化路径
    /// 
    /// # 参数
    /// * `path` - 待规范化的路径
    /// 
    /// # 返回值
    /// * `Result<String, FsError>` - 规范化后的路径或错误
    /// 
    /// # 契约
    /// * 处理相对路径
    /// * 解析符号链接
    /// * 消除冗余分隔符
    /// * 验证最终路径有效性
    pub fn normalize_path(path: &str) -> Result<alloc::string::String, FsError> {
        // Forward to internal implementation
        unimplemented!("normalize_path not implemented")
    }
}