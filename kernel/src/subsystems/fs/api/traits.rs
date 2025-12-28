//! Interface contracts for the fs module.
//!
//! This module defines the traits that all file system implementations must
//! satisfy.

use super::{DirEntry, PathComponent, FsError};

///文件操作特征
///
///所有文件操作实现必须满足此特征
pub trait FileOperations {
    ///读取文件数据
    ///
    ///#契约
    ///* 必须处理文件偏移超出文件大小的情况
    ///* 必须处理部分读取
    ///* 必须更新访问时间
    ///* 必须保证线程安全
    fn read(&self, offset: u64, buffer: &mut [u8]) -> Result<usize, FsError>;
    
    ///写入文件数据
    ///
    ///#契约
    ///* 必须处理文件空间不足的情况
    ///* 必须处理部分写入
    ///* 必须更新修改时间
    ///* 必须保证线程安全
    fn write(&self, offset: u64, buffer: &[u8]) -> Result<usize, FsError>;
    
    ///获取文件大小
    ///
    ///#契约
    ///* 必须返回当前文件大小
    ///* 必须考虑文件锁的影响
    fn size(&self) -> u64;
    
    ///截断文件
    ///
    ///#契约
    ///* 必须处理截断位置超出文件大小的情况
    ///* 必须更新修改时间
    ///* 必须保证数据一致性
    fn truncate(&self, size: u64) -> Result<(), FsError>;
    
    ///同步文件数据
    ///
    ///#契约
    ///* 必须确保数据写入存储设备
    ///* 必须处理同步错误
    fn sync(&self) -> Result<(), FsError>;
}

///目录操作特征
///
///所有目录操作实现必须满足此特征
pub trait DirectoryOperations {
    ///创建子目录
    ///
    ///#契约
    ///* 必须验证父目录存在性
    ///* 必须检查创建权限
    ///* 必须初始化目录结构
    ///* 必须更新父目录修改时间
    fn create_subdir(&self, name: &str, mode: u32) -> Result<(), FsError>;
    
    ///删除子目录
    ///
    ///#契约
    ///* 必须验证目录为空
    ///* 必须检查删除权限
    ///* 必须递归删除所有内容
    ///* 必须更新父目录修改时间
    fn remove_subdir(&self, name: &str) -> Result<(), FsError>;
    
    ///列出目录条目
    ///
    ///#契约
    ///* 必须包含所有文件和子目录
    ///* 必须按排序顺序返回
    ///* 必须处理符号链接
    ///* 必须过滤隐藏文件（根据标志）
    fn list_entries(&self) -> Result<alloc::vec::Vec<DirEntry>, FsError>;
    
    ///查找目录条目
    ///
    ///#契约
    ///* 支持精确匹配和通配符匹配
    ///* 必须处理大小写敏感性
    ///* 必须返回完整条目信息
    fn find_entry(&self, name: &str) -> Option<DirEntry>;
}

///路径处理特征
///
///所有路径处理实现必须满足此特征
pub trait PathOperations {
    ///解析路径
    ///
    ///#契约
    ///* 必须正确处理绝对路径和相对路径
    ///* 必须解析符号链接（可选）
    ///* 必须处理路径分隔符标准化
    ///* 必须验证路径长度限制
    fn parse(&self, follow_symlinks: bool) -> Result<alloc::vec::Vec<PathComponent>, FsError>;
    
    ///规范化路径
    ///
    ///#契约
    ///* 必须消除冗余路径分隔符
    ///* 必须处理"."和".."组件
    ///* 必须解析符号链接
    ///* 必须返回绝对路径
    fn normalize(&self) -> Result<alloc::string::String, FsError>;
    
    ///验证路径
    ///
    ///#契约
    ///* 必须检查无效字符
    ///* 必须检查路径长度
    ///* 必须检查路径格式
    ///* 必须检查安全限制
    fn validate(&self) -> Result<(), FsError>;
    
    ///连接路径组件
    ///
    ///#契约
    ///* 必须正确处理路径分隔符
    ///* 必须处理空组件
    ///* 必须保持路径有效性
    fn join(&self, component: &str) -> alloc::string::String;
}