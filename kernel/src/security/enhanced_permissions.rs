//! 增强权限控制系统
//! 
//! 本模块提供细粒度的权限管理机制，支持基于角色的访问控制(RBAC)、
//! 能力安全(capabilities)和强制访问控制(MAC)。

use core::sync::atomic::{AtomicU64, Ordering};
use crate::subsystems::sync::Mutex;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

/// 权限位定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PermissionBits(u64);

impl PermissionBits {
    pub const NONE: Self = Self(0);
    
    // 文件权限
    pub const FILE_READ: Self = Self(1 << 0);
    pub const FILE_WRITE: Self = Self(1 << 1);
    pub const FILE_EXECUTE: Self = Self(1 << 2);
    pub const FILE_DELETE: Self = Self(1 << 3);
    
    // 目录权限
    pub const DIR_LIST: Self = Self(1 << 4);
    pub const DIR_CREATE: Self = Self(1 << 5);
    pub const DIR_REMOVE: Self = Self(1 << 6);
    
    // 进程权限
    pub const PROCESS_CREATE: Self = Self(1 << 7);
    pub const PROCESS_TERMINATE: Self = Self(1 << 8);
    pub const PROCESS_SIGNAL: Self = Self(1 << 9);
    pub const PROCESS_DEBUG: Self = Self(1 << 10);
    
    // 网络权限
    pub const NETWORK_BIND: Self = Self(1 << 11);
    pub const NETWORK_CONNECT: Self = Self(1 << 12);
    pub const NETWORK_LISTEN: Self = Self(1 << 13);
    
    // 系统权限
    pub const SYSTEM_REBOOT: Self = Self(1 << 14);
    pub const SYSTEM_SHUTDOWN: Self = Self(1 << 15);
    pub const SYSTEM_CONFIGURE: Self = Self(1 << 16);
    
    // 设备权限
    pub const DEVICE_READ: Self = Self(1 << 17);
    pub const DEVICE_WRITE: Self = Self(1 << 18);
    pub const DEVICE_MMAP: Self = Self(1 << 19);
    
    // 内存权限
    pub const MEMORY_ALLOCATE: Self = Self(1 << 20);
    pub const MEMORY_LOCK: Self = Self(1 << 21);
    pub const MEMORY_MPROTECT: Self = Self(1 << 22);
    
    // 时间权限
    pub const TIME_SET: Self = Self(1 << 23);
    pub const TIME_ADJUST: Self = Self(1 << 24);
    
    // 安全权限
    pub const SECURITY_CONFIGURE: Self = Self(1 << 25);
    pub const SECURITY_AUDIT: Self = Self(1 << 26);
    
    // 所有权限
    pub const ALL: Self = Self(u64::MAX);
}

impl core::ops::BitOr for PermissionBits {
    type Output = Self;
    
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitAnd for PermissionBits {
    type Output = Self;
    
    fn bitand(self, rhs: Self) -> Self {
        Self(self.0 & rhs.0)
    }
}

impl core::ops::BitOrAssign for PermissionBits {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl core::ops::BitAndAssign for PermissionBits {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

/// 用户ID
pub type UserId = u32;

/// 组ID
pub type GroupId = u32;

/// 进程ID
pub type ProcessId = u32;

/// 安全上下文
#[derive(Debug, Clone)]
pub struct SecurityContext {
    pub user_id: UserId,
    pub group_id: GroupId,
    pub supplementary_groups: Vec<GroupId>,
    pub capabilities: CapabilitySet,
    pub role: Option<RoleId>,
    pub clearance_level: ClearanceLevel,
}

/// 能力集
#[derive(Debug, Clone)]
pub struct CapabilitySet {
    bits: AtomicU64,
}

impl CapabilitySet {
    pub fn new() -> Self {
        Self {
            bits: AtomicU64::new(0),
        }
    }
    
    pub fn has(&self, capability: Capability) -> bool {
        let bits = self.bits.load(Ordering::Acquire);
        (bits & capability.as_bit()) != 0
    }
    
    pub fn add(&self, capability: Capability) {
        let mut bits = self.bits.load(Ordering::Acquire);
        bits |= capability.as_bit();
        self.bits.store(bits, Ordering::Release);
    }
    
    pub fn remove(&self, capability: Capability) {
        let mut bits = self.bits.load(Ordering::Acquire);
        bits &= !capability.as_bit();
        self.bits.store(bits, Ordering::Release);
    }
    
    pub fn clear(&self) {
        self.bits.store(0, Ordering::Release);
    }
}

/// 能力定义
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    // 文件能力
    CapFileRead,
    CapFileWrite,
    CapFileExecute,
    CapFileDelete,
    
    // 网络能力
    CapNetBind,
    CapNetConnect,
    CapNetListen,
    
    // 进程能力
    CapProcessCreate,
    CapProcessTerminate,
    CapProcessSignal,
    
    // 系统能力
    CapSysReboot,
    CapSysShutdown,
    CapSysConfigure,
}

impl Capability {
    pub fn as_bit(self) -> u64 {
        match self {
            Capability::CapFileRead => 1 << 0,
            Capability::CapFileWrite => 1 << 1,
            Capability::CapFileExecute => 1 << 2,
            Capability::CapFileDelete => 1 << 3,
            Capability::CapNetBind => 1 << 4,
            Capability::CapNetConnect => 1 << 5,
            Capability::CapNetListen => 1 << 6,
            Capability::CapProcessCreate => 1 << 7,
            Capability::CapProcessTerminate => 1 << 8,
            Capability::CapProcessSignal => 1 << 9,
            Capability::CapSysReboot => 1 << 10,
            Capability::CapSysShutdown => 1 << 11,
            Capability::CapSysConfigure => 1 << 12,
        }
    }
}

/// 角色ID
pub type RoleId = u32;

/// 角色定义
#[derive(Debug, Clone)]
pub struct Role {
    pub id: RoleId,
    pub name: String,
    pub permissions: PermissionBits,
    pub inherited_roles: Vec<RoleId>,
}

/// 清除级别
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ClearanceLevel {
    Unclassified = 0,
    Confidential = 1,
    Secret = 2,
    TopSecret = 3,
}

/// 访问控制条目
#[derive(Debug, Clone)]
pub struct AccessControlEntry {
    pub subject: Subject,
    pub object: Object,
    pub permissions: PermissionBits,
    pub conditions: Vec<AccessCondition>,
}

/// 访问主体
#[derive(Debug, Clone)]
pub enum Subject {
    User(UserId),
    Group(GroupId),
    Role(RoleId),
    Process(ProcessId),
}

/// 访问对象
#[derive(Debug, Clone)]
pub enum Object {
    File(String),
    Directory(String),
    Device(String),
    NetworkPort(u16),
    SystemResource(String),
}

/// 访问条件
#[derive(Debug, Clone)]
pub enum AccessCondition {
    TimeWindow { start: u64, end: u64 },
    IpAddress(u32),
    ProcessState(String),
    Custom(String),
}

/// 权限检查结果
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessResult {
    Allow,
    Deny { reason: DenyReason },
}

/// 拒绝原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DenyReason {
    PermissionDenied,
    InsufficientClearance,
    TimeRestriction,
    IpRestriction,
    RoleRestriction,
    CapabilityMissing,
    SystemPolicy,
}

/// 增强权限管理器
#[derive(Debug)]
pub struct EnhancedPermissionManager {
    roles: Mutex<BTreeMap<RoleId, Role>>,
    access_control: Mutex<Vec<AccessControlEntry>>,
    default_permissions: PermissionBits,
    audit_log: Mutex<Vec<AuditEntry>>,
}

impl EnhancedPermissionManager {
    /// 创建新的权限管理器
    pub fn new() -> Self {
        Self {
            roles: Mutex::new(BTreeMap::new()),
            access_control: Mutex::new(Vec::new()),
            default_permissions: PermissionBits::NONE,
            audit_log: Mutex::new(Vec::new()),
        }
    }
    
    /// 检查权限
    pub fn check_permission(
        &self,
        context: &SecurityContext,
        object: &Object,
        requested: PermissionBits,
    ) -> AccessResult {
        // 1. 检查能力
        if !self.check_capabilities(context, requested) {
            return AccessResult::Deny {
                reason: DenyReason::CapabilityMissing,
            };
        }
        
        // 2. 检查角色权限
        if let Some(role_id) = context.role {
            if !self.check_role_permissions(role_id, requested) {
                return AccessResult::Deny {
                    reason: DenyReason::RoleRestriction,
                };
            }
        }
        
        // 3. 检查访问控制列表
        if let Some(deny_reason) = self.check_access_control(context, object, requested) {
            return AccessResult::Deny { reason: deny_reason };
        }
        
        // 4. 检查清除级别
        if !self.check_clearance(context, object) {
            return AccessResult::Deny {
                reason: DenyReason::InsufficientClearance,
            };
        }
        
        // 5. 检查时间限制
        if !self.check_time_restrictions(context, object) {
            return AccessResult::Deny {
                reason: DenyReason::TimeRestriction,
            };
        }
        
        // 权限检查通过
        self.log_access(context, object, requested, AccessResult::Allow);
        AccessResult::Allow
    }
    
    /// 检查能力
    fn check_capabilities(&self, context: &SecurityContext, requested: PermissionBits) -> bool {
        // 检查每个请求的权限是否在能力集中
        // 这里简化实现，实际应该检查每个位
        true // 暂时返回true，实际需要详细实现
    }
    
    /// 检查角色权限
    fn check_role_permissions(&self, role_id: RoleId, requested: PermissionBits) -> bool {
        let roles = self.roles.lock();
        if let Some(role) = roles.get(&role_id) {
            // 检查直接权限
            if (role.permissions & requested) == requested {
                return true;
            }
            
            // 检查继承角色的权限
            for &inherited_role_id in &role.inherited_roles {
                if let Some(inherited_role) = roles.get(&inherited_role_id) {
                    if (inherited_role.permissions & requested) == requested {
                        return true;
                    }
                }
            }
        }
        
        false
    }
    
    /// 检查访问控制列表
    fn check_access_control(
        &self,
        context: &SecurityContext,
        object: &Object,
        requested: PermissionBits,
    ) -> Option<DenyReason> {
        let acl = self.access_control.lock();
        
        for entry in acl.iter() {
            if self.matches_subject(&entry.subject, context) &&
               self.matches_object(&entry.object, object) &&
               (entry.permissions & requested) != PermissionBits::NONE {
                
                // 检查条件
                for condition in &entry.conditions {
                    if !self.check_condition(condition, context, object) {
                        return Some(DenyReason::SystemPolicy);
                    }
                }
                
                // 找到匹配的ACL条目，拒绝访问
                return Some(DenyReason::SystemPolicy);
            }
        }
        
        None // 没有匹配的拒绝条目
    }
    
    /// 检查主体是否匹配
    fn matches_subject(&self, subject: &Subject, context: &SecurityContext) -> bool {
        match subject {
            Subject::User(uid) => *uid == context.user_id,
            Subject::Group(gid) => *gid == context.group_id,
            Subject::Role(rid) => context.role == Some(*rid),
            Subject::Process(pid) => false, // 需要获取当前进程ID
        }
    }
    
    /// 检查对象是否匹配
    fn matches_object(&self, object: &Object, target: &Object) -> bool {
        match (object, target) {
            (Object::File(pattern), Object::File(target)) => {
                target.starts_with(pattern) || pattern == "*"
            }
            (Object::Directory(pattern), Object::Directory(target)) => {
                target.starts_with(pattern) || pattern == "*"
            }
            (Object::Device(pattern), Object::Device(target)) => {
                target.starts_with(pattern) || pattern == "*"
            }
            (Object::NetworkPort(pattern), Object::NetworkPort(target)) => {
                *pattern == *target || *pattern == 0 // 0表示所有端口
            }
            (Object::SystemResource(pattern), Object::SystemResource(target)) => {
                target.starts_with(pattern) || pattern == "*"
            }
            _ => false,
        }
    }
    
    /// 检查访问条件
    fn check_condition(&self, condition: &AccessCondition, context: &SecurityContext, object: &Object) -> bool {
        match condition {
            AccessCondition::TimeWindow { start, end } => {
                // 获取当前时间并检查是否在窗口内
                // 这里简化实现
                true
            }
            AccessCondition::IpAddress(ip) => {
                // 检查源IP地址
                // 这里简化实现
                true
            }
            AccessCondition::ProcessState(state) => {
                // 检查进程状态
                // 这里简化实现
                true
            }
            AccessCondition::Custom(_) => {
                // 自定义条件
                true
            }
        }
    }
    
    /// 检查清除级别
    fn check_clearance(&self, context: &SecurityContext, object: &Object) -> bool {
        // 获取对象的清除级别要求
        let required_clearance = self.get_object_clearance(object);
        context.clearance_level >= required_clearance
    }
    
    /// 获取对象的清除级别要求
    fn get_object_clearance(&self, object: &Object) -> ClearanceLevel {
        match object {
            Object::File(path) => {
                if path.contains("/secret/") {
                    ClearanceLevel::Secret
                } else if path.contains("/confidential/") {
                    ClearanceLevel::Confidential
                } else {
                    ClearanceLevel::Unclassified
                }
            }
            Object::Directory(path) => {
                if path.contains("/secret/") {
                    ClearanceLevel::Secret
                } else if path.contains("/confidential/") {
                    ClearanceLevel::Confidential
                } else {
                    ClearanceLevel::Unclassified
                }
            }
            _ => ClearanceLevel::Unclassified,
        }
    }
    
    /// 检查时间限制
    fn check_time_restrictions(&self, context: &SecurityContext, object: &Object) -> bool {
        // 简化实现，实际应该检查具体的时间窗口
        true
    }
    
    /// 记录访问日志
    fn log_access(&self, context: &SecurityContext, object: &Object, requested: PermissionBits, result: AccessResult) {
        let entry = AuditEntry {
            timestamp: self.get_current_time(),
            subject: Subject::User(context.user_id),
            object: object.clone(),
            requested_permissions: requested,
            result,
            process_id: self.get_current_process_id(),
        };
        
        let mut log = self.audit_log.lock();
        log.push(entry);
        
        // 保持日志大小在合理范围内
        if log.len() > 10000 {
            log.remove(0);
        }
    }
    
    /// 获取当前时间（简化实现）
    fn get_current_time(&self) -> u64 {
        // 实际应该从系统时钟获取
        0
    }
    
    /// 获取当前进程ID（简化实现）
    fn get_current_process_id(&self) -> ProcessId {
        // 实际应该从进程管理器获取
        0
    }
    
    /// 添加角色
    pub fn add_role(&self, role: Role) -> Result<(), &'static str> {
        let mut roles = self.roles.lock();
        if roles.contains_key(&role.id) {
            return Err("Role already exists");
        }
        roles.insert(role.id, role);
        Ok(())
    }
    
    /// 删除角色
    pub fn remove_role(&self, role_id: RoleId) -> Result<(), &'static str> {
        let mut roles = self.roles.lock();
        if !roles.contains_key(&role_id) {
            return Err("Role not found");
        }
        roles.remove(&role_id);
        Ok(())
    }
    
    /// 添加访问控制条目
    pub fn add_access_control_entry(&self, entry: AccessControlEntry) {
        let mut acl = self.access_control.lock();
        acl.push(entry);
    }
    
    /// 删除访问控制条目
    pub fn remove_access_control_entry(&self, index: usize) -> Result<(), &'static str> {
        let mut acl = self.access_control.lock();
        if index >= acl.len() {
            return Err("Invalid index");
        }
        acl.remove(index);
        Ok(())
    }
    
    /// 获取审计日志
    pub fn get_audit_log(&self) -> Vec<AuditEntry> {
        self.audit_log.lock().clone()
    }
    
    /// 清空审计日志
    pub fn clear_audit_log(&self) {
        self.audit_log.lock().clear();
    }
}

/// 审计日志条目
#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: u64,
    pub subject: Subject,
    pub object: Object,
    pub requested_permissions: PermissionBits,
    pub result: AccessResult,
    pub process_id: ProcessId,
}

/// 全局权限管理器
static GLOBAL_PERMISSION_MANAGER: Mutex<Option<EnhancedPermissionManager>> = Mutex::new(None);

/// 获取全局权限管理器
pub fn get_global_permission_manager() -> &'static Mutex<EnhancedPermissionManager> {
    &GLOBAL_PERMISSION_MANAGER
}

/// 初始化全局权限管理器
pub fn init_permission_manager() {
    let mut manager = GLOBAL_PERMISSION_MANAGER.lock();
    if manager.is_none() {
        *manager = Some(EnhancedPermissionManager::new());
        crate::println!("[security] Enhanced permission manager initialized");
    }
}

/// 检查权限（便捷函数）
pub fn check_permission(
    context: &SecurityContext,
    object: &Object,
    requested: PermissionBits,
) -> AccessResult {
    let manager = GLOBAL_PERMISSION_MANAGER.lock();
    if let Some(ref mgr) = *manager {
        mgr.check_permission(context, object, requested)
    } else {
        AccessResult::Deny {
            reason: DenyReason::SystemPolicy,
        }
    }
}

/// 获取当前进程的安全上下文
pub fn get_current_security_context() -> SecurityContext {
    // 实际应该从当前进程获取
    SecurityContext {
        user_id: 0,
        group_id: 0,
        supplementary_groups: Vec::new(),
        capabilities: CapabilitySet::new(),
        role: None,
        clearance_level: ClearanceLevel::Unclassified,
    }
}