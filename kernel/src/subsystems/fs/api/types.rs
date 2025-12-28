//! Public type definitions for the fs module API.
//!
//! This module contains all public types that are used by the fs module API.

extern crate alloc;

///文件句柄类型
///
///用于标识打开的文件
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileHandle(pub u32);

///目录条目类型
///
///表示目录中的一个条目
#[derive(Debug, Clone)]
pub struct DirEntry {
    ///条目名称
    pub name: alloc::string::String,
    ///条目类型
    pub entry_type: DirEntryType,
    ///条目属性
    pub attributes: FileAttr,
    ///条目inode号
    pub inode: u64,
}

///目录条目类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirEntryType {
    ///普通文件
    File,
    ///目录
    Directory,
    ///符号链接
    SymbolicLink,
    ///块设备
    BlockDevice,
    ///字符设备
    CharacterDevice,
    ///FIFO管道
    FIFO,
    ///套接字
    Socket,
}

///文件属性类型
///
///包含文件的元数据信息
#[derive(Debug, Clone)]
pub struct FileAttr {
    ///inode号
    pub inode: u64,
    ///文件类型
    pub file_type: DirEntryType,
    ///文件权限
    pub mode: u32,
    ///硬链接数
    pub nlink: u32,
    ///用户ID
    pub uid: u32,
    ///组ID
    pub gid: u32,
    ///设备ID
    pub rdev: u64,
    ///文件大小
    pub size: u64,
    ///块大小
    pub blksize: u64,
    ///块数
    pub blocks: u64,
    ///最后访问时间
    pub atime: u64,
    ///最后修改时间
    pub mtime: u64,
    ///创建时间
    pub ctime: u64,
}

///路径组件类型
///
///表示路径中的一个组件
#[derive(Debug, Clone)]
pub struct PathComponent {
    ///组件名称
    pub name: alloc::string::String,
    ///组件类型
    pub component_type: PathComponentType,
}

///路径组件类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathComponentType {
    ///目录名
    Directory,
    ///文件名
    File,
    ///当前目录
    CurrentDir,
    ///父目录
    ParentDir,
    ///根目录
    RootDir,
}