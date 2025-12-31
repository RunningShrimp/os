//! 访问控制类型定义
//!
//! 定义访问控制相关的所有类型、枚举和结构体

use alloc::{
    collections::BTreeMap,
    string::String,
    vec::Vec,
};

/// 用户标识符
pub type UserId = u32;
/// 组标识符
pub type GroupId = u32;
/// 进程标识符
pub type ProcessId = u32;

/// 访问控制结果
#[derive(Debug, Clone, PartialEq)]
pub enum AccessResult {
    /// 访问被允许
    Allowed,
    /// 访问被拒绝 - 权限不足
    DeniedPermission(String),
    /// 访问被拒绝 - 资源不存在
    DeniedResourceNotFound(String),
    /// 访问被拒绝 - 资源不可访问
    DeniedResourceInaccessible(String),
    /// 访问被拒绝 - 操作不被支持
    DeniedOperationNotSupported(String),
    /// 访问检查失败 - 内部错误
    Failed(String),
}

/// 用户信息
#[derive(Debug, Clone)]
pub struct UserInfo {
    /// 用户ID
    pub uid: UserId,
    /// 用户名
    pub username: String,
    /// 主组ID
    pub gid: GroupId,
    /// 附加组ID列表
    pub supplementary_gids: Vec<GroupId>,
    /// 用户主目录
    pub home_directory: String,
    /// 用户shell
    pub shell: String,
    /// 用户全名
    pub full_name: String,
    /// 用户类型
    pub user_type: UserType,
    /// 账户状态
    pub account_status: AccountStatus,
}

/// 用户类型
#[derive(Debug, Clone, PartialEq)]
pub enum UserType {
    /// 系统用户
    System,
    /// 管理员用户
    Administrator,
    /// 普通用户
    Regular,
    /// 服务用户
    Service,
    /// 访客用户
    Guest,
}

/// 账户状态
#[derive(Debug, Clone, PartialEq)]
pub enum AccountStatus {
    /// 活跃
    Active,
    /// 已锁定
    Locked,
    /// 已过期
    Expired,
    /// 已禁用
    Disabled,
    /// 密码过期
    PasswordExpired,
}

/// 权限
#[derive(Debug, Clone, PartialEq)]
pub enum Permission {
    /// 读权限
    Read,
    /// 写权限
    Write,
    /// 执行权限
    Execute,
    /// 删除权限
    Delete,
    /// 创建权限
    Create,
    /// 管理权限
    Admin,
    /// 自定义权限
    Custom(String),
}

/// 资源类型
#[derive(Debug, Clone, PartialEq)]
pub enum ResourceType {
    /// 文件
    File,
    /// 目录
    Directory,
    /// 设备
    Device,
    /// 进程
    Process,
    /// 网络套接字
    NetworkSocket,
    /// IPC对象
    IpcObject,
    /// 内存区域
    MemoryRegion,
    /// 系统调用
    Syscall,
    /// 自定义资源
    Custom(String),
}

/// 访问控制条目
#[derive(Debug, Clone)]
pub struct AccessControlEntry {
    /// 资源类型
    pub resource_type: ResourceType,
    /// 资源标识符
    pub resource_id: String,
    /// 主体类型（用户或组）
    pub principal_type: PrincipalType,
    /// 主体标识符
    pub principal_id: u32,
    /// 权限集合
    pub permissions: Vec<Permission>,
    /// 访问规则（允许或拒绝）
    pub access_rule: AccessRule,
    /// 继承标志
    pub inheritable: bool,
}

/// 主体类型
#[derive(Debug, Clone, PartialEq)]
pub enum PrincipalType {
    /// 用户
    User,
    /// 组
    Group,
    /// 其他
    Other,
}

/// 访问规则
#[derive(Debug, Clone, PartialEq)]
pub enum AccessRule {
    /// 允许
    Allow,
    /// 拒绝
    Deny,
}

/// 能力
#[derive(Debug, Clone)]
pub struct Capability {
    /// 能力名称
    pub name: String,
    /// 能力描述
    pub description: String,
    /// 能力版本
    pub version: String,
    /// 能力类型
    pub capability_type: CapabilityType,
    /// 能力参数
    pub parameters: BTreeMap<String, String>,
}

/// 能力类型
#[derive(Debug, Clone, PartialEq)]
pub enum CapabilityType {
    /// 文件系统能力
    FileSystem,
    /// 网络能力
    Network,
    /// 进程管理能力
    ProcessManagement,
    /// 设备访问能力
    DeviceAccess,
    /// 系统管理能力
    SystemManagement,
    /// 安全能力
    Security,
    /// 自定义能力
    Custom(String),
}

/// 组信息
#[derive(Debug, Clone)]
pub struct GroupInfo {
    /// 组ID
    pub gid: GroupId,
    /// 组名
    pub groupname: String,
    /// 组成员列表
    pub members: Vec<UserId>,
    /// 组描述
    pub description: String,
}
