//! # 扩展属性 (Extended Attributes) 框架
//!
//! 提供文件扩展属性的支持，允许将额外的元数据与文件关联。
//!
//! ## 功能
//!
//! - **命名空间**: 支持 user、trusted、security 命名空间
//! - **大小限制**: 遵循 POSIX 扩展属性限制
//! - **安全属性**: 支持 SELinux、SMACK 等安全框架
//! - **批量操作**: 支持列表、获取、设置、删除操作

extern crate alloc;
use alloc::{collections::BTreeMap, string::String, vec::Vec};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::subsystems::sync::Mutex;
use crate::subsystems::fs::api::error::FsError;

/// 扩展属性名称最大长度
pub const XATTR_NAME_MAX: usize = 255;

/// 扩展属性值最大大小
pub const XATTR_SIZE_MAX: usize = 65536;

/// 扩展属性列表最大大小
pub const XATTR_LIST_MAX: usize = 65536;

/// 扩展属性命名空间
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum XattrNamespace {
    /// user.* - 用户属性（可由任何用户修改）
    User,
    /// trusted.* - 受信任属性（仅 root 可修改）
    Trusted,
    /// security.* - 安全属性（用于 LSM 框架）
    Security,
    /// system.* - 系统属性（内核使用）
    System,
}

impl XattrNamespace {
    /// 从名称解析命名空间
    pub fn from_name(name: &str) -> Result<Self, FsError> {
        if let Some(ns_str) = name.split('.').next() {
            match ns_str {
                "user" => Ok(XattrNamespace::User),
                "trusted" => Ok(XattrNamespace::Trusted),
                "security" => Ok(XattrNamespace::Security),
                "system" => Ok(XattrNamespace::System),
                _ => Err(FsError::InvalidInput),
            }
        } else {
            Err(FsError::InvalidInput)
        }
    }

    /// 获取命名空间前缀
    pub fn prefix(&self) -> &str {
        match self {
            XattrNamespace::User => "user",
            XattrNamespace::Trusted => "trusted",
            XattrNamespace::Security => "security",
            XattrNamespace::System => "system",
        }
    }
}

/// 扩展属性条目
#[derive(Debug, Clone)]
pub struct XattrEntry {
    /// 完整属性名称（包含命名空间）
    pub name: String,
    /// 属性值
    pub value: Vec<u8>,
    /// 创建时间
    pub created_at: u64,
    /// 修改时间
    pub modified_at: u64,
}

impl XattrEntry {
    /// 创建新的扩展属性条目
    pub fn new(name: String, value: Vec<u8>) -> Self {
        let now = get_timestamp();
        Self {
            name,
            value,
            created_at: now,
            modified_at: now,
        }
    }

    /// 获取属性值的大小
    pub fn size(&self) -> usize {
        self.value.len()
    }
}

/// 文件的扩展属性集合
#[derive(Debug)]
pub struct XattrSet {
    /// 属性映射: name -> entry
    attrs: Mutex<BTreeMap<String, XattrEntry>>,
    /// inode 编号（用于标识文件）
    inode: u32,
    /// 总大小限制
    max_size: usize,
}

impl XattrSet {
    /// 创建新的扩展属性集合
    pub fn new(inode: u32) -> Self {
        Self {
            attrs: Mutex::new(BTreeMap::new()),
            inode,
            max_size: XATTR_SIZE_MAX,
        }
    }

    /// 设置扩展属性
    ///
    /// # 参数
    ///
    /// * `name`: 属性名称（必须包含命名空间前缀）
    /// * `value`: 属性值
    /// * `flags`: 操作标志 (XATTR_CREATE, XATTR_REPLACE)
    ///
    /// # 错误
    ///
    /// - `FsError::Exists`: 属性已存在且设置了 XATTR_CREATE
    /// - `FsError::NotFound`: 属性不存在且设置了 XATTR_REPLACE
    /// - `FsError::NoSpace`: 超过大小限制
    pub fn set(&self, name: &str, value: &[u8], flags: u32) -> Result<(), FsError> {
        // 验证属性名称
        XattrNamespace::from_name(name)?;

        // 验证大小限制
        if name.len() > XATTR_NAME_MAX || value.len() > XATTR_SIZE_MAX {
            return Err(FsError::InvalidInput);
        }

        let mut attrs = self.attrs.lock();

        // 检查大小限制
        let current_size = attrs.values().map(|v| v.size()).sum::<usize>();
        if let Some(existing) = attrs.get(name) {
            if current_size - existing.size() + value.len() > self.max_size {
                return Err(FsError::NoSpace);
            }
        } else {
            if current_size + value.len() > self.max_size {
                return Err(FsError::NoSpace);
            }
        }

        // 检查标志
        if flags & XATTR_CREATE != 0 && attrs.contains_key(name) {
            return Err(FsError::Exists);
        }
        if flags & XATTR_REPLACE != 0 && !attrs.contains_key(name) {
            return Err(FsError::NotFound);
        }

        // 创建或更新属性
        let entry = XattrEntry::new(name.to_string(), value.to_vec());
        attrs.insert(name.to_string(), entry);

        Ok(())
    }

    /// 获取扩展属性
    ///
    /// # 参数
    ///
    /// * `name`: 属性名称
    /// * `value`: 输出缓冲区
    ///
    /// # 返回
    ///
    /// 返回实际属性值的大小。如果缓冲区太小，返回 ERANGE 错误。
    pub fn get(&self, name: &str, value: &mut [u8]) -> Result<usize, FsError> {
        let attrs = self.attrs.lock();

        let entry = attrs.get(name).ok_or(FsError::NotFound)?;

        if value.len() < entry.value.len() {
            return Err(FsError::InvalidInput); // ERANGE
        }

        value[..entry.value.len()].copy_from_slice(&entry.value);
        Ok(entry.value.len())
    }

    /// 删除扩展属性
    pub fn remove(&self, name: &str) -> Result<(), FsError> {
        let mut attrs = self.attrs.lock();

        attrs.remove(name).ok_or(FsError::NotFound)?;
        Ok(())
    }

    /// 列出所有扩展属性名称
    ///
    /// # 参数
    ///
    /// * `list`: 输出缓冲区（存储以 null 结尾的名称列表）
    ///
    /// # 返回
    ///
    /// 返回实际列表的大小。
    pub fn list(&self, list: &mut [u8]) -> Result<usize, FsError> {
        let attrs = self.attrs.lock();

        let mut offset = 0;
        for name in attrs.keys() {
            let name_bytes = name.as_bytes();
            let name_len = name_bytes.len() + 1; // 包括 null 终止符

            if offset + name_len > list.len() {
                return Err(FsError::InvalidInput); // ERANGE
            }

            list[offset..offset + name_bytes.len()].copy_from_slice(name_bytes);
            list[offset + name_bytes.len()] = 0; // null 终止符
            offset += name_len;
        }

        Ok(offset)
    }

    /// 获取所有属性名称
    pub fn get_names(&self) -> Vec<String> {
        let attrs = self.attrs.lock();
        attrs.keys().cloned().collect()
    }

    /// 获取属性数量
    pub fn count(&self) -> usize {
        self.attrs.lock().len()
    }
}

/// 扩展属性操作标志
pub const XATTR_CREATE: u32 = 0x1;  // 仅创建新属性
pub const XATTR_REPLACE: u32 = 0x2;  // 仅替换现有属性

/// 全局扩展属性管理器
pub struct XattrManager {
    /// 文件扩展属性映射: inode -> XattrSet
    xattrs: Mutex<BTreeMap<u32, Arc<XattrSet>>>,
    /// 统计信息
    stats: Mutex<XattrStats>,
}

/// 扩展属性统计信息
#[derive(Debug, Default)]
pub struct XattrStats {
    /// 总设置操作数
    pub total_sets: u64,
    /// 总获取操作数
    pub total_gets: u64,
    /// 总删除操作数
    pub total_removes: u64,
    /// 总列表操作数
    pub total_lists: u64,
}

impl XattrManager {
    /// 创建新的扩展属性管理器
    pub fn new() -> Self {
        Self {
            xattrs: Mutex::new(BTreeMap::new()),
            stats: Mutex::new(XattrStats::default()),
        }
    }

    /// 获取或创建文件的扩展属性集合
    fn get_xattr_set(&self, inode: u32) -> Arc<XattrSet> {
        let mut xattrs = self.xattrs.lock();

        if let Some(xattr_set) = xattrs.get(&inode) {
            xattr_set.clone()
        } else {
            let xattr_set = Arc::new(XattrSet::new(inode));
            xattrs.insert(inode, xattr_set.clone());
            xattr_set
        }
    }

    /// 设置扩展属性
    pub fn setxattr(
        &self,
        inode: u32,
        name: &str,
        value: &[u8],
        flags: u32,
    ) -> Result<(), FsError> {
        let xattr_set = self.get_xattr_set(inode);
        xattr_set.set(name, value, flags)?;

        // 更新统计
        let mut stats = self.stats.lock();
        stats.total_sets += 1;

        Ok(())
    }

    /// 获取扩展属性
    pub fn getxattr(&self, inode: u32, name: &str, value: &mut [u8]) -> Result<usize, FsError> {
        let xattr_set = self.get_xattr_set(inode);
        let result = xattr_set.get(name, value)?;

        // 更新统计
        let mut stats = self.stats.lock();
        stats.total_gets += 1;

        Ok(result)
    }

    /// 删除扩展属性
    pub fn removexattr(&self, inode: u32, name: &str) -> Result<(), FsError> {
        let xattr_set = self.get_xattr_set(inode);
        xattr_set.remove(name)?;

        // 更新统计
        let mut stats = self.stats.lock();
        stats.total_removes += 1;

        Ok(())
    }

    /// 列出扩展属性
    pub fn listxattr(&self, inode: u32, list: &mut [u8]) -> Result<usize, FsError> {
        let xattr_set = self.get_xattr_set(inode);
        let result = xattr_set.list(list)?;

        // 更新统计
        let mut stats = self.stats.lock();
        stats.total_lists += 1;

        Ok(result)
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> XattrStats {
        self.stats.lock().clone()
    }

    /// 删除文件的所有扩展属性
    pub fn remove_all(&self, inode: u32) {
        let mut xattrs = self.xattrs.lock();
        xattrs.remove(&inode);
    }
}

/// 获取当前时间戳
fn get_timestamp() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// 全局扩展属性管理器实例
static mut XATTR_MANAGER: Option<XattrManager> = None;

/// 初始化扩展属性管理器
pub fn init() {
    unsafe {
        XATTR_MANAGER = Some(XattrManager::new());
    }
    crate::println!("xattr: initialized extended attributes manager");
}

/// 获取扩展属性管理器
pub fn get_manager() -> Option<&'static XattrManager> {
    unsafe { XATTR_MANAGER.as_ref() }
}

/// setxattr 系统调用
///
/// # 参数
///
/// * `path`: 文件路径
/// * `name`: 属性名称
/// * `value`: 属性值
/// * `size`: 属性值大小
/// * `flags`: 操作标志
///
/// # 返回
///
/// 成功返回 0，失败返回错误码
pub fn sys_setxattr(
    path: &str,
    name: &str,
    value: &[u8],
    size: usize,
    flags: u32,
) -> isize {
    // 验证参数
    if name.is_empty() || name.len() > XATTR_NAME_MAX {
        return -22; // EINVAL
    }

    if size > XATTR_SIZE_MAX {
        return -34; // E2BIG
    }

    // 获取文件 inode
    let inode = match lookup_path_inode(path) {
        Ok(inode) => inode,
        Err(e) => return error_to_errno(e),
    };

    // 获取扩展属性管理器
    let manager = match get_manager() {
        Some(m) => m,
        None => return -38, // ENOSYS
    };

    // 设置扩展属性
    match manager.setxattr(inode, name, &value[..size], flags) {
        Ok(()) => 0,
        Err(e) => error_to_errno(e),
    }
}

/// getxattr 系统调用
///
/// # 参数
///
/// * `path`: 文件路径
/// * `name`: 属性名称
/// * `value`: 输出缓冲区
/// * `size`: 缓冲区大小
///
/// # 返回
///
/// 成功返回属性值大小，失败返回错误码
pub fn sys_getxattr(path: &str, name: &str, value: &mut [u8], size: usize) -> isize {
    // 验证参数
    if name.is_empty() || name.len() > XATTR_NAME_MAX {
        return -22; // EINVAL
    }

    if size > XATTR_LIST_MAX {
        return -34; // E2BIG
    }

    // 获取文件 inode
    let inode = match lookup_path_inode(path) {
        Ok(inode) => inode,
        Err(e) => return error_to_errno(e),
    };

    // 获取扩展属性管理器
    let manager = match get_manager() {
        Some(m) => m,
        None => return -38, // ENOSYS
    };

    // 获取扩展属性
    match manager.getxattr(inode, name, value) {
        Ok(size) => size as isize,
        Err(e) => error_to_errno(e),
    }
}

/// listxattr 系统调用
///
/// # 参数
///
/// * `path`: 文件路径
/// * `list`: 输出缓冲区
/// * `size`: 缓冲区大小
///
/// # 返回
///
/// 成功返回列表大小，失败返回错误码
pub fn sys_listxattr(path: &str, list: &mut [u8], size: usize) -> isize {
    // 验证参数
    if size > XATTR_LIST_MAX {
        return -34; // E2BIG
    }

    // 获取文件 inode
    let inode = match lookup_path_inode(path) {
        Ok(inode) => inode,
        Err(e) => return error_to_errno(e),
    };

    // 获取扩展属性管理器
    let manager = match get_manager() {
        Some(m) => m,
        None => return -38, // ENOSYS
    };

    // 列出扩展属性
    match manager.listxattr(inode, list) {
        Ok(size) => size as isize,
        Err(e) => error_to_errno(e),
    }
}

/// removexattr 系统调用
///
/// # 参数
///
/// * `path`: 文件路径
/// * `name`: 属性名称
///
/// # 返回
///
/// 成功返回 0，失败返回错误码
pub fn sys_removexattr(path: &str, name: &str) -> isize {
    // 验证参数
    if name.is_empty() || name.len() > XATTR_NAME_MAX {
        return -22; // EINVAL
    }

    // 获取文件 inode
    let inode = match lookup_path_inode(path) {
        Ok(inode) => inode,
        Err(e) => return error_to_errno(e),
    };

    // 获取扩展属性管理器
    let manager = match get_manager() {
        Some(m) => m,
        None => return -38, // ENOSYS
    };

    // 删除扩展属性
    match manager.removexattr(inode, name) {
        Ok(()) => 0,
        Err(e) => error_to_errno(e),
    }
}

/// 查找路径对应的 inode（占位符实现）
fn lookup_path_inode(_path: &str) -> Result<u32, FsError> {
    // TODO: 实现 VFS 路径查找
    // 暂时返回一个假的 inode
    Ok(1)
}

/// 将 FsError 转换为 errno
fn error_to_errno(err: FsError) -> isize {
    match err {
        FsError::NotFound => -2,      // ENOENT
        FsError::PermissionDenied => -13, // EACCES
        FsError::NoSpace => -28,      // ENOSPC
        FsError::InvalidInput => -22, // EINVAL
        FsError::Exists => -17,       // EEXIST
        FsError::NotSupported => -95, // EOPNOTSUPP
        _ => -5,                       // EIO
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xattr_namespace() {
        assert_eq!(
            XattrNamespace::from_name("user.test").unwrap(),
            XattrNamespace::User
        );
        assert_eq!(
            XattrNamespace::from_name("security.selinux").unwrap(),
            XattrNamespace::Security
        );
        assert!(XattrNamespace::from_name("invalid").is_err());
    }

    #[test]
    fn test_xattr_set() {
        let set = XattrSet::new(1);

        // 设置属性
        set.set("user.test", b"hello", 0).unwrap();

        // 获取属性
        let mut buf = [0u8; 32];
        let size = set.get("user.test", &mut buf).unwrap();
        assert_eq!(size, 5);
        assert_eq!(&buf[..5], b"hello");
    }

    #[test]
    fn test_xattr_replace() {
        let set = XattrSet::new(1);

        // 创建模式：属性不存在时成功
        set.set("user.test", b"hello", XATTR_CREATE).unwrap();

        // 创建模式：属性已存在时失败
        assert!(matches!(
            set.set("user.test", b"world", XATTR_CREATE),
            Err(FsError::Exists)
        ));

        // 替换模式：属性存在时成功
        set.set("user.test", b"world", XATTR_REPLACE).unwrap();
        let mut buf = [0u8; 32];
        let size = set.get("user.test", &mut buf).unwrap();
        assert_eq!(&buf[..size], b"world");
    }

    #[test]
    fn test_xattr_list() {
        let set = XattrSet::new(1);

        set.set("user.test1", b"value1", 0).unwrap();
        set.set("user.test2", b"value2", 0).unwrap();

        let names = set.get_names();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"user.test1".to_string()));
        assert!(names.contains(&"user.test2".to_string()));
    }
}
