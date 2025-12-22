//! 文件系统模块类型定义
//! 
//! 本模块定义了文件系统相关的类型、枚举和结构体，包括：
//! - 文件类型和权限
//! - 文件描述符和状态
//! - 目录条目和路径
//! - 文件系统操作参数

use alloc::string::String;
use alloc::vec::Vec;

/// 文件系统操作类型枚举
///
/// 定义文件系统服务支持的操作类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemOperation {
    /// 获取文件状态 (stat)
    Stat,
    /// 获取符号链接状态 (lstat)
    Lstat,
    /// 检查文件访问权限 (access)
    Access,
    /// 改变当前目录 (chdir)
    ChangeDirectory,
    /// 获取当前目录 (getcwd)
    GetCurrentDirectory,
    /// 创建目录 (mkdir)
    MakeDirectory,
    /// 删除目录 (rmdir)
    RemoveDirectory,
    /// 删除文件 (unlink)
    Unlink,
    /// 重命名文件 (rename)
    Rename,
    /// 创建硬链接 (link)
    Link,
    /// 创建符号链接 (symlink)
    Readlink,
    /// 读取符号链接 (readlink)
    ChangeMode,
    /// 改变文件权限 (chmod)
    ChangeOwner,
    /// 改变文件所有者 (chown)
    SetUmask,
    /// 设置umask
    Mount,
    /// 创建符号链接
    Symlink,
}

/// 文件类型枚举
///
/// 定义文件的类型，用于文件系统操作。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    /// 普通文件
    RegularFile,
    /// 目录
    Directory,
    /// 字符设备
    CharacterDevice,
    /// 块设备
    BlockDevice,
    /// 符号链接
    SymbolicLink,
    /// 命名管道
    NamedPipe,
    /// 套接字
    Socket,
}

/// 文件权限位
/// 
/// 定义文件的访问权限。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FilePermissions {
    /// 所有者读权限
    pub owner_read: bool,
    /// 所有者写权限
    pub owner_write: bool,
    /// 所有者执行权限
    pub owner_execute: bool,
    /// 组读权限
    pub group_read: bool,
    /// 组写权限
    pub group_write: bool,
    /// 组执行权限
    pub group_execute: bool,
    /// 其他读权限
    pub other_read: bool,
    /// 其他写权限
    pub other_write: bool,
    /// 其他执行权限
    pub other_execute: bool,
}

impl FilePermissions {
    /// 创建新的文件权限
    pub fn new() -> Self {
        Self {
            owner_read: true,
            owner_write: true,
            owner_execute: false,
            group_read: true,
            group_write: false,
            group_execute: false,
            other_read: true,
            other_write: false,
            other_execute: false,
        }
    }

    /// 转换为八进制权限表示
    pub fn to_octal(&self) -> u16 {
        let mut mode = 0u16;
        
        if self.owner_read { mode |= 0o400; }
        if self.owner_write { mode |= 0o200; }
        if self.owner_execute { mode |= 0o100; }
        if self.group_read { mode |= 0o040; }
        if self.group_write { mode |= 0o020; }
        if self.group_execute { mode |= 0o010; }
        if self.other_read { mode |= 0o004; }
        if self.other_write { mode |= 0o002; }
        if self.other_execute { mode |= 0o001; }
        
        mode
    }

    /// 从八进制权限创建
    pub fn from_octal(mode: u16) -> Self {
        Self {
            owner_read: (mode & 0o400) != 0,
            owner_write: (mode & 0o200) != 0,
            owner_execute: (mode & 0o100) != 0,
            group_read: (mode & 0o040) != 0,
            group_write: (mode & 0o020) != 0,
            group_execute: (mode & 0o010) != 0,
            other_read: (mode & 0o004) != 0,
            other_write: (mode & 0o002) != 0,
            other_execute: (mode & 0o001) != 0,
        }
    }
}

impl Default for FilePermissions {
    fn default() -> Self {
        Self::new()
    }
}

/// 文件状态信息
/// 
/// 包含文件的详细属性信息。
#[derive(Debug, Clone)]
pub struct FileStatus {
    /// 文件类型
    pub file_type: FileType,
    /// 文件权限
    pub permissions: FilePermissions,
    /// 文件所有者ID
    pub uid: u32,
    /// 文件组ID
    pub gid: u32,
    /// 文件大小（字节）
    pub size: u64,
    /// 文件块数
    pub blocks: u64,
    /// 最后访问时间
    pub atime: u64,
    /// 最后修改时间
    pub mtime: u64,
    /// 最后状态改变时间
    pub ctime: u64,
    /// 硬链接数
    pub nlink: u32,
    /// 设备ID
    pub device: u32,
    /// inode号
    pub inode: u64,
}

/// 文件描述符信息
/// 
/// 包含文件描述符的状态和属性。
#[derive(Debug, Clone)]
pub struct FileDescriptor {
    /// 描述符ID
    pub fd: i32,
    /// 文件路径
    pub path: String,
    /// 文件类型
    pub file_type: FileType,
    /// 打开模式
    pub open_mode: OpenMode,
    /// 当前位置偏移
    pub offset: u64,
    /// 文件权限
    pub permissions: FilePermissions,
    /// 是否为非阻塞模式
    pub non_blocking: bool,
    /// 是否为追加模式
    pub append_mode: bool,
}

/// 文件打开模式
/// 
/// 定义文件的打开模式。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenMode {
    /// 只读
    ReadOnly,
    /// 只写
    WriteOnly,
    /// 读写
    ReadWrite,
    /// 创建（如果不存在）
    Create,
    /// 追加
    Append,
    /// 截断
    Truncate,
    /// 非阻塞
    NonBlocking,
}

/// 目录条目
/// 
/// 表示目录中的一个条目。
#[derive(Debug, Clone)]
pub struct DirEntry {
    /// inode号
    pub inode: u64,
    /// 条目类型
    pub entry_type: FileType,
    /// 条目名称
    pub name: String,
}

/// 文件系统操作标志
/// 
/// 定义文件系统操作的标志位。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsFlags {
    /// 不跟随符号链接
    NoFollow,
    /// 创建目录
    CreateDirectory,
    /// 递归操作
    Recursive,
    /// 强制操作
    Force,
    /// 详细输出
    Verbose,
}

/// 文件系统统计信息
/// 
/// 包含文件系统的使用统计。
#[derive(Debug, Clone)]
pub struct FsStats {
    /// 文件系统类型
    pub fs_type: String,
    /// 总块数
    pub total_blocks: u64,
    /// 可用块数
    pub free_blocks: u64,
    /// 已用块数
    pub used_blocks: u64,
    /// 总inode数
    pub total_inodes: u64,
    /// 可用inode数
    pub free_inodes: u64,
    /// 块大小
    pub block_size: u32,
    /// inode大小
    pub inode_size: u32,
}

/// 文件操作参数
/// 
/// 包含文件操作所需的参数。
#[derive(Debug, Clone)]
pub struct FileOperationParams {
    /// 文件路径
    pub path: String,
    /// 操作模式
    pub mode: OpenMode,
    /// 权限设置
    pub permissions: Option<FilePermissions>,
    /// 操作标志
    pub flags: Vec<FsFlags>,
    /// 缓冲区大小
    pub buffer_size: Option<usize>,
}

impl Default for FileOperationParams {
    fn default() -> Self {
        Self {
            path: String::new(),
            mode: OpenMode::ReadOnly,
            permissions: None,
            flags: Vec::new(),
            buffer_size: None,
        }
    }
}

/// 文件系统错误类型
/// 
/// 定义文件系统模块特有的错误类型。
#[derive(Debug, Clone)]
pub enum FsError {
    /// 文件不存在
    FileNotFound,
    /// 权限不足
    PermissionDenied,
    /// 文件已存在
    FileExists,
    /// 无效路径
    InvalidPath,
    /// 目录非空
    DirectoryNotEmpty,
    /// 不是目录
    NotDirectory,
    /// 不是文件
    NotFile,
    /// 磁盘空间不足
    DiskFull,
    /// 文件系统只读
    ReadOnlyFileSystem,
    /// 文件描述符无效
    InvalidFd,
    /// 文件描述符已用完
    FdTableFull,
    /// 文件名过长
    NameTooLong,
    /// 符号链接循环
    SymbolicLinkLoop,
    /// 系统调用不支持
    UnsupportedSyscall,
}

impl FsError {
    /// 获取错误码
    pub fn error_code(&self) -> i32 {
        match self {
            FsError::FileNotFound => -2,
            FsError::PermissionDenied => -13,
            FsError::FileExists => -17,
            FsError::InvalidPath => -22,
            FsError::DirectoryNotEmpty => -39,
            FsError::NotDirectory => -20,
            FsError::NotFile => -21,
            FsError::DiskFull => -28,
            FsError::ReadOnlyFileSystem => -30,
            FsError::InvalidFd => -9,
            FsError::FdTableFull => -24,
            FsError::NameTooLong => -36,
            FsError::SymbolicLinkLoop => -40,
            FsError::UnsupportedSyscall => -38,
        }
    }

    /// 获取错误描述
    pub fn error_message(&self) -> &str {
        match self {
            FsError::FileNotFound => "File not found",
            FsError::PermissionDenied => "Permission denied",
            FsError::FileExists => "File already exists",
            FsError::InvalidPath => "Invalid path",
            FsError::DirectoryNotEmpty => "Directory not empty",
            FsError::NotDirectory => "Not a directory",
            FsError::NotFile => "Not a file",
            FsError::DiskFull => "No space left on device",
            FsError::ReadOnlyFileSystem => "Read-only file system",
            FsError::InvalidFd => "Invalid file descriptor",
            FsError::FdTableFull => "File descriptor table full",
            FsError::NameTooLong => "File name too long",
            FsError::SymbolicLinkLoop => "Too many levels of symbolic links",
            FsError::UnsupportedSyscall => "Unsupported syscall",
        }
    }
}

/// 虚拟文件系统接口特征
/// 
/// 定义虚拟文件系统的基本操作接口。
pub trait VfsOperations: Send + Sync {
    /// 打开文件
    fn open(&self, path: &str, mode: OpenMode) -> Result<i32, FsError>;
    
    /// 关闭文件
    fn close(&self, fd: i32) -> Result<(), FsError>;
    
    /// 读取文件
    fn read(&self, fd: i32, buffer: &mut [u8]) -> Result<usize, FsError>;
    
    /// 写入文件
    fn write(&self, fd: i32, buffer: &[u8]) -> Result<usize, FsError>;
    
    /// 查询文件状态
    fn stat(&self, path: &str) -> Result<FileStatus, FsError>;
    
    /// 创建目录
    fn mkdir(&self, path: &str, permissions: FilePermissions) -> Result<(), FsError>;
    
    /// 删除目录
    fn rmdir(&self, path: &str) -> Result<(), FsError>;
    
    /// 列出目录内容
    fn readdir(&self, path: &str) -> Result<Vec<DirEntry>, FsError>;
    
    /// 创建符号链接
    fn symlink(&self, target: &str, link_path: &str) -> Result<(), FsError>;
    
    /// 读取符号链接
    fn readlink(&self, path: &str) -> Result<String, FsError>;
    
    /// 重命名文件
    fn rename(&self, old_path: &str, new_path: &str) -> Result<(), FsError>;
    
    /// 删除文件
    fn unlink(&self, path: &str) -> Result<(), FsError>;
    
    /// 获取文件系统统计
    fn statfs(&self, path: &str) -> Result<FsStats, FsError>;
}

/// 文件系统错误类型枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilesystemError {
    /// 无效参数
    InvalidArgument,
    /// 权限被拒绝
    PermissionDenied,
    /// 文件不存在
    FileNotFound,
    /// 文件已存在
    FileExists,
    /// 路径太长
    PathTooLong,
    /// 磁盘空间不足
    NoSpace,
    /// 只读文件系统
    ReadOnlyFilesystem,
    /// I/O 错误
    IoError,
    /// 不支持的操作
    UnsupportedOperation,
    /// 文件名太长
    FileNameTooLong,
}