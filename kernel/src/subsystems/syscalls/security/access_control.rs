//! 访问控制和权限管理模块
//! 
//! 本模块提供系统的访问控制和权限管理功能，包括：
//! - 用户身份验证
//! - 权限检查
//! - 资源访问控制
//! - 能力管理
//! - 访问控制列表(ACL)

use nos_nos_error_handling::unified::KernelError;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;

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

/// 访问控制管理器
pub struct AccessControlManager {
    /// 用户信息映射
    users: Arc<Mutex<BTreeMap<UserId, UserInfo>>>,
    /// 组信息映射
    groups: Arc<Mutex<BTreeMap<GroupId, GroupInfo>>>,
    /// 访问控制列表
    acl: Arc<Mutex<Vec<AccessControlEntry>>>,
    /// 能力映射
    capabilities: Arc<Mutex<BTreeMap<String, Capability>>>,
    /// 用户能力映射
    user_capabilities: Arc<Mutex<BTreeMap<UserId, Vec<String>>>>,
    /// 管理器配置
    config: AccessControlConfig,
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

/// 访问控制配置
#[derive(Debug, Clone)]
pub struct AccessControlConfig {
    /// 是否启用严格模式
    pub strict_mode: bool,
    /// 默认访问规则
    pub default_access_rule: AccessRule,
    /// 是否启用能力系统
    pub enable_capabilities: bool,
    /// 是否启用审计日志
    pub enable_audit_log: bool,
    /// 最大ACL条目数
    pub max_acl_entries: usize,
}

impl Default for AccessControlConfig {
    fn default() -> Self {
        Self {
            strict_mode: false,
            default_access_rule: AccessRule::Deny,
            enable_capabilities: true,
            enable_audit_log: true,
            max_acl_entries: 10000,
        }
    }
}

impl AccessControlManager {
    /// 创建新的访问控制管理器
    pub fn new(config: AccessControlConfig) -> Self {
        let mut manager = Self {
            users: Arc::new(Mutex::new(BTreeMap::new())),
            groups: Arc::new(Mutex::new(BTreeMap::new())),
            acl: Arc::new(Mutex::new(Vec::new())),
            capabilities: Arc::new(Mutex::new(BTreeMap::new())),
            user_capabilities: Arc::new(Mutex::new(BTreeMap::new())),
            config,
        };
        
        // 初始化默认用户和组
        manager.init_default_users_and_groups();
        
        // 初始化默认能力
        manager.init_default_capabilities();
        
        manager
    }
    
    /// 使用默认配置创建访问控制管理器
    pub fn with_default_config() -> Self {
        Self::new(AccessControlConfig::default())
    }
    
    /// 初始化默认用户和组
    fn init_default_users_and_groups(&mut self) {
        // 创建root用户
        let root_user = UserInfo {
            uid: 0,
            username: "root".to_string(),
            gid: 0,
            supplementary_gids: vec![],
            home_directory: "/root".to_string(),
            shell: "/bin/sh".to_string(),
            full_name: "System Administrator".to_string(),
            user_type: UserType::Administrator,
            account_status: AccountStatus::Active,
        };
        
        // 创建root组
        let root_group = GroupInfo {
            gid: 0,
            groupname: "root".to_string(),
            members: vec![0],
            description: "System Administrators".to_string(),
        };
        
        // 创建nobody用户
        let nobody_user = UserInfo {
            uid: 65534,
            username: "nobody".to_string(),
            gid: 65534,
            supplementary_gids: vec![],
            home_directory: "/nonexistent".to_string(),
            shell: "/bin/false".to_string(),
            full_name: "Unprivileged User".to_string(),
            user_type: UserType::Guest,
            account_status: AccountStatus::Active,
        };
        
        // 创建nobody组
        let nobody_group = GroupInfo {
            gid: 65534,
            groupname: "nogroup".to_string(),
            members: vec![65534],
            description: "Unprivileged Users".to_string(),
        };
        
        // 添加到映射
        self.users.lock().insert(0, root_user);
        self.users.lock().insert(65534, nobody_user);
        self.groups.lock().insert(0, root_group);
        self.groups.lock().insert(65534, nobody_group);
    }
    
    /// 初始化默认能力
    fn init_default_capabilities(&mut self) {
        // 文件系统能力
        let fs_read = Capability {
            name: "fs.read".to_string(),
            description: "Read files and directories".to_string(),
            version: "1.0".to_string(),
            capability_type: CapabilityType::FileSystem,
            parameters: BTreeMap::new(),
        };
        
        let fs_write = Capability {
            name: "fs.write".to_string(),
            description: "Write files and directories".to_string(),
            version: "1.0".to_string(),
            capability_type: CapabilityType::FileSystem,
            parameters: BTreeMap::new(),
        };
        
        // 网络能力
        let net_create = Capability {
            name: "net.create".to_string(),
            description: "Create network sockets".to_string(),
            version: "1.0".to_string(),
            capability_type: CapabilityType::Network,
            parameters: BTreeMap::new(),
        };
        
        let net_connect = Capability {
            name: "net.connect".to_string(),
            description: "Connect to network endpoints".to_string(),
            version: "1.0".to_string(),
            capability_type: CapabilityType::Network,
            parameters: BTreeMap::new(),
        };
        
        // 进程管理能力
        let proc_fork = Capability {
            name: "proc.fork".to_string(),
            description: "Create new processes".to_string(),
            version: "1.0".to_string(),
            capability_type: CapabilityType::ProcessManagement,
            parameters: BTreeMap::new(),
        };
        
        let proc_kill = Capability {
            name: "proc.kill".to_string(),
            description: "Terminate processes".to_string(),
            version: "1.0".to_string(),
            capability_type: CapabilityType::ProcessManagement,
            parameters: BTreeMap::new(),
        };
        
        // 添加到能力映射
        self.capabilities.lock().insert("fs.read".to_string(), fs_read);
        self.capabilities.lock().insert("fs.write".to_string(), fs_write);
        self.capabilities.lock().insert("net.create".to_string(), net_create);
        self.capabilities.lock().insert("net.connect".to_string(), net_connect);
        self.capabilities.lock().insert("proc.fork".to_string(), proc_fork);
        self.capabilities.lock().insert("proc.kill".to_string(), proc_kill);
        
        // 为root用户分配所有能力
        let root_caps = vec![
            "fs.read".to_string(),
            "fs.write".to_string(),
            "net.create".to_string(),
            "net.connect".to_string(),
            "proc.fork".to_string(),
            "proc.kill".to_string(),
        ];
        self.user_capabilities.lock().insert(0, root_caps);
    }
    
    /// 检查用户访问权限
    pub fn check_access(
        &self,
        uid: UserId,
        resource_type: ResourceType,
        resource_id: &str,
        permission: Permission,
    ) -> AccessResult {
        // 获取用户信息
        let user = match self.users.lock().get(&uid) {
            Some(user) => user.clone(),
            None => return AccessResult::DeniedResourceNotFound(format!("User {} not found", uid)),
        };
        
        // 检查账户状态
        if user.account_status != AccountStatus::Active {
            return AccessResult::DeniedResourceInaccessible(format!("User account is not active"));
        }
        
        // 检查ACL
        let acl = self.acl.lock();
        let mut explicit_allow = false;
        let mut explicit_deny = false;
        
        // 检查用户特定的ACL条目
        for entry in acl.iter() {
            if entry.resource_type == resource_type && 
               (entry.resource_id == "*" || entry.resource_id == resource_id) &&
               entry.principal_type == PrincipalType::User &&
               entry.principal_id == uid {
                
                if entry.permissions.contains(&permission) {
                    match entry.access_rule {
                        AccessRule::Allow => explicit_allow = true,
                        AccessRule::Deny => explicit_deny = true,
                    }
                }
            }
        }
        
        // 检查组特定的ACL条目
        if !explicit_deny && !explicit_allow {
            let groups = self.groups.lock();
            
            // 检查主组
            if let Some(group) = groups.get(&user.gid) {
                for entry in acl.iter() {
                    if entry.resource_type == resource_type && 
                       (entry.resource_id == "*" || entry.resource_id == resource_id) &&
                       entry.principal_type == PrincipalType::Group &&
                       entry.principal_id == user.gid {
                        
                        if entry.permissions.contains(&permission) {
                            match entry.access_rule {
                                AccessRule::Allow => explicit_allow = true,
                                AccessRule::Deny => explicit_deny = true,
                            }
                        }
                    }
                }
            }
            
            // 检查附加组
            for &gid in &user.supplementary_gids {
                if let Some(group) = groups.get(&gid) {
                    for entry in acl.iter() {
                        if entry.resource_type == resource_type && 
                           (entry.resource_id == "*" || entry.resource_id == resource_id) &&
                           entry.principal_type == PrincipalType::Group &&
                           entry.principal_id == gid {
                            
                            if entry.permissions.contains(&permission) {
                                match entry.access_rule {
                                    AccessRule::Allow => explicit_allow = true,
                                    AccessRule::Deny => explicit_deny = true,
                                }
                            }
                        }
                    }
                }
            }
        }
        
        // 检查"其他"ACL条目
        if !explicit_deny && !explicit_allow {
            for entry in acl.iter() {
                if entry.resource_type == resource_type && 
                   (entry.resource_id == "*" || entry.resource_id == resource_id) &&
                   entry.principal_type == PrincipalType::Other {
                    
                    if entry.permissions.contains(&permission) {
                        match entry.access_rule {
                            AccessRule::Allow => explicit_allow = true,
                            AccessRule::Deny => explicit_deny = true,
                        }
                    }
                }
            }
        }
        
        // 应用访问规则
        if explicit_deny {
            return AccessResult::DeniedPermission("Explicitly denied by ACL".to_string());
        }
        
        if explicit_allow {
            return AccessResult::Allowed;
        }
        
        // 如果没有明确的规则，应用默认规则
        match self.config.default_access_rule {
            AccessRule::Allow => AccessResult::Allowed,
            AccessRule::Deny => AccessResult::DeniedPermission("Denied by default rule".to_string()),
        }
    }
    
    /// 检查用户能力
    pub fn check_capability(&self, uid: UserId, capability_name: &str) -> AccessResult {
        // 获取用户信息
        let user = match self.users.lock().get(&uid) {
            Some(user) => user.clone(),
            None => return AccessResult::DeniedResourceNotFound(format!("User {} not found", uid)),
        };
        
        // 检查账户状态
        if user.account_status != AccountStatus::Active {
            return AccessResult::DeniedResourceInaccessible(format!("User account is not active"));
        }
        
        // 检查能力是否存在
        if !self.capabilities.lock().contains_key(capability_name) {
            return AccessResult::DeniedOperationNotSupported(format!("Capability {} not found", capability_name));
        }
        
        // 检查用户是否具有该能力
        let user_caps = self.user_capabilities.lock();
        if let Some(caps) = user_caps.get(&uid) {
            if caps.contains(&capability_name.to_string()) {
                return AccessResult::Allowed;
            }
        }
        
        // root用户拥有所有能力
        if uid == 0 {
            return AccessResult::Allowed;
        }
        
        AccessResult::DeniedPermission(format!("User {} does not have capability {}", uid, capability_name))
    }
    
    /// 添加用户
    pub fn add_user(&self, user: UserInfo) -> Result<(), KernelError> {
        let mut users = self.users.lock();
        
        // 检查用户是否已存在
        if users.contains_key(&user.uid) {
            return Err(KernelError::AlreadyInProgress);
        }
        
        // 添加用户
        users.insert(user.uid, user);
        
        Ok(())
    }
    
    /// 添加组
    pub fn add_group(&self, group: GroupInfo) -> Result<(), KernelError> {
        let mut groups = self.groups.lock();
        
        // 检查组是否已存在
        if groups.contains_key(&group.gid) {
            return Err(KernelError::AlreadyInProgress);
        }
        
        // 添加组
        groups.insert(group.gid, group);
        
        Ok(())
    }
    
    /// 添加ACL条目
    pub fn add_acl_entry(&self, entry: AccessControlEntry) -> Result<(), KernelError> {
        let mut acl = self.acl.lock();
        
        // 检查ACL条目数是否超过限制
        if acl.len() >= self.config.max_acl_entries {
            return Err(KernelError::OutOfSpace);
        }
        
        // 添加ACL条目
        acl.push(entry);
        
        Ok(())
    }
    
    /// 移除ACL条目
    pub fn remove_acl_entry(&self, resource_type: ResourceType, resource_id: &str, 
                           principal_type: PrincipalType, principal_id: u32) -> Result<(), KernelError> {
        let mut acl = self.acl.lock();
        
        // 查找并移除匹配的ACL条目
        acl.retain(|entry| {
            !(entry.resource_type == resource_type && 
              entry.resource_id == resource_id &&
              entry.principal_type == principal_type &&
              entry.principal_id == principal_id)
        });
        
        Ok(())
    }
    
    /// 为用户分配能力
    pub fn grant_capability(&self, uid: UserId, capability_name: &str) -> Result<(), KernelError> {
        // 检查能力是否存在
        if !self.capabilities.lock().contains_key(capability_name) {
            return Err(KernelError::NotFound);
        }
        
        // 添加能力到用户
        let mut user_caps = self.user_capabilities.lock();
        let caps = user_caps.entry(uid).or_insert_with(Vec::new);
        
        if !caps.contains(&capability_name.to_string()) {
            caps.push(capability_name.to_string());
        }
        
        Ok(())
    }
    
    /// 撤销用户能力
    pub fn revoke_capability(&self, uid: UserId, capability_name: &str) -> Result<(), KernelError> {
        let mut user_caps = self.user_capabilities.lock();
        
        if let Some(caps) = user_caps.get_mut(&uid) {
            caps.retain(|cap| cap != capability_name);
        }
        
        Ok(())
    }
    
    /// 获取用户信息
    pub fn get_user(&self, uid: UserId) -> Option<UserInfo> {
        self.users.lock().get(&uid).cloned()
    }
    
    /// 获取组信息
    pub fn get_group(&self, gid: GroupId) -> Option<GroupInfo> {
        self.groups.lock().get(&gid).cloned()
    }
    
    /// 获取用户能力列表
    pub fn get_user_capabilities(&self, uid: UserId) -> Vec<String> {
        self.user_capabilities.lock()
            .get(&uid)
            .map(|caps| caps.clone())
            .unwrap_or_default()
    }
    
    /// 获取所有能力
    pub fn get_all_capabilities(&self) -> BTreeMap<String, Capability> {
        self.capabilities.lock().clone()
    }
    
    /// 获取ACL条目
    pub fn get_acl_entries(&self) -> Vec<AccessControlEntry> {
        self.acl.lock().clone()
    }
}