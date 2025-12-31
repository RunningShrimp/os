// Enhanced Namespace Support
//
// 增强命名空间支持
// 提供完整的命名空间隔离和管理功能

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    format,
    string::String,
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;

use crate::reliability::{EINVAL, EIO, ENOENT};
use crate::subsystems::cloud_native::oci::spec::OciLinuxNamespaceType;

/// 命名空间类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NamespaceType {
    /// 挂载命名空间
    Mount,
    /// UTS命名空间
    Uts,
    /// IPC命名空间
    Ipc,
    /// 网络命名空间
    Network,
    /// PID命名空间
    Pid,
    /// 用户命名空间
    User,
    /// Cgroup命名空间
    Cgroup,
}

impl From<NamespaceType> for OciLinuxNamespaceType {
    fn from(ns_type: NamespaceType) -> Self {
        match ns_type {
            NamespaceType::Mount => OciLinuxNamespaceType::Mount,
            NamespaceType::Uts => OciLinuxNamespaceType::UTS,
            NamespaceType::Ipc => OciLinuxNamespaceType::IPC,
            NamespaceType::Network => OciLinuxNamespaceType::Network,
            NamespaceType::Pid => OciLinuxNamespaceType::PID,
            NamespaceType::User => OciLinuxNamespaceType::User,
            NamespaceType::Cgroup => OciLinuxNamespaceType::Cgroup,
        }
    }
}

impl From<OciLinuxNamespaceType> for NamespaceType {
    fn from(ns: OciLinuxNamespaceType) -> Self {
        match ns {
            OciLinuxNamespaceType::Mount => NamespaceType::Mount,
            OciLinuxNamespaceType::UTS => NamespaceType::Uts,
            OciLinuxNamespaceType::IPC => NamespaceType::Ipc,
            OciLinuxNamespaceType::Network => NamespaceType::Network,
            OciLinuxNamespaceType::PID => NamespaceType::Pid,
            OciLinuxNamespaceType::User => NamespaceType::User,
            OciLinuxNamespaceType::Cgroup => NamespaceType::Cgroup,
        }
    }
}

/// 命名空间
#[derive(Debug, Clone)]
pub struct Namespace {
    /// 命名空间ID
    pub id: u64,
    /// 命名空间类型
    pub ns_type: NamespaceType,
    /// 命名空间路径
    pub path: String,
    /// 是否激活
    pub active: bool,
    /// 进程列表
    pub processes: Arc<Mutex<Vec<u32>>>,
}

impl Namespace {
    /// 创建新命名空间
    pub fn new(id: u64, ns_type: NamespaceType, path: String) -> Self {
        Self {
            id,
            ns_type,
            path,
            active: false,
            processes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// 激活命名空间
    pub fn activate(&mut self) -> Result<(), i32> {
        if self.active {
            return Err(EINVAL);
        }
        self.active = true;
        Ok(())
    }

    /// 添加进程
    pub fn add_process(&self, pid: u32) {
        let mut processes = self.processes.lock();
        if !processes.contains(&pid) {
            processes.push(pid);
        }
    }

    /// 移除进程
    pub fn remove_process(&self, pid: u32) {
        let mut processes = self.processes.lock();
        processes.retain(|&p| p != pid);
    }

    /// 获取进程数量
    pub fn process_count(&self) -> usize {
        self.processes.lock().len()
    }
}

/// 命名空间集合
#[derive(Debug, Clone)]
pub struct NamespaceSet {
    /// 挂载命名空间
    pub mount: Option<Namespace>,
    /// UTS命名空间
    pub uts: Option<Namespace>,
    /// IPC命名空间
    pub ipc: Option<Namespace>,
    /// 网络命名空间
    pub network: Option<Namespace>,
    /// PID命名空间
    pub pid: Option<Namespace>,
    /// 用户命名空间
    pub user: Option<Namespace>,
    /// Cgroup命名空间
    pub cgroup: Option<Namespace>,
}

impl NamespaceSet {
    /// 创建新的命名空间集合
    pub fn new() -> Self {
        Self {
            mount: None,
            uts: None,
            ipc: None,
            network: None,
            pid: None,
            user: None,
            cgroup: None,
        }
    }

    /// 检查是否包含指定的命名空间类型
    pub fn has_namespace(&self, ns_type: NamespaceType) -> bool {
        match ns_type {
            NamespaceType::Mount => self.mount.is_some(),
            NamespaceType::Uts => self.uts.is_some(),
            NamespaceType::Ipc => self.ipc.is_some(),
            NamespaceType::Network => self.network.is_some(),
            NamespaceType::Pid => self.pid.is_some(),
            NamespaceType::User => self.user.is_some(),
            NamespaceType::Cgroup => self.cgroup.is_some(),
        }
    }

    /// 获取指定类型的命名空间
    pub fn get_namespace(&self, ns_type: NamespaceType) -> Option<&Namespace> {
        match ns_type {
            NamespaceType::Mount => self.mount.as_ref(),
            NamespaceType::Uts => self.uts.as_ref(),
            NamespaceType::Ipc => self.ipc.as_ref(),
            NamespaceType::Network => self.network.as_ref(),
            NamespaceType::Pid => self.pid.as_ref(),
            NamespaceType::User => self.user.as_ref(),
            NamespaceType::Cgroup => self.cgroup.as_ref(),
        }
    }
}

impl Default for NamespaceSet {
    fn default() -> Self {
        Self::new()
    }
}

/// 命名空间管理器
pub struct NamespaceManager {
    /// 所有命名空间
    namespaces: BTreeMap<u64, Arc<Mutex<Namespace>>>,
    /// 进程到命名空间的映射
    process_namespaces: BTreeMap<u32, NamespaceSet>,
    /// 类型索引
    type_index: BTreeMap<NamespaceType, Vec<u64>>,
    /// 下一个命名空间ID
    next_id: AtomicU64,
}

impl NamespaceManager {
    /// 创建新的命名空间管理器
    pub fn new() -> Self {
        Self {
            namespaces: BTreeMap::new(),
            process_namespaces: BTreeMap::new(),
            type_index: BTreeMap::new(),
            next_id: AtomicU64::new(1),
        }
    }

    /// 创建命名空间集合
    pub fn create_namespaces(&mut self, types: &[NamespaceType]) -> Result<NamespaceSet, i32> {
        let mut ns_set = NamespaceSet::new();

        for &ns_type in types {
            let ns_id = self.next_id.fetch_add(1, Ordering::SeqCst);
            let path = format!("/var/run/ns/{:?}-{}", ns_type, ns_id);

            let mut namespace = Namespace::new(ns_id, ns_type, path);
            namespace.activate()?;

            let ns_arc = Arc::new(Mutex::new(namespace));
            self.namespaces.insert(ns_id, ns_arc.clone());

            // 更新类型索引
            let entry = self.type_index.entry(ns_type).or_insert_with(Vec::new);
            entry.push(ns_id);

            // 更新集合
            match ns_type {
                NamespaceType::Mount => ns_set.mount = Some((*ns_arc.lock()).clone()),
                NamespaceType::Uts => ns_set.uts = Some((*ns_arc.lock()).clone()),
                NamespaceType::Ipc => ns_set.ipc = Some((*ns_arc.lock()).clone()),
                NamespaceType::Network => ns_set.network = Some((*ns_arc.lock()).clone()),
                NamespaceType::Pid => ns_set.pid = Some((*ns_arc.lock()).clone()),
                NamespaceType::User => ns_set.user = Some((*ns_arc.lock()).clone()),
                NamespaceType::Cgroup => ns_set.cgroup = Some((*ns_arc.lock()).clone()),
            }
        }

        crate::println!("[namespace-manager] Created namespace set with {} namespaces", types.len());
        Ok(ns_set)
    }

    /// 为进程分配命名空间集合
    pub fn assign_namespaces(&mut self, pid: u32, ns_set: &NamespaceSet) -> Result<(), i32> {
        // 将进程添加到所有命名空间
        if let Some(ref ns) = ns_set.mount {
            ns.add_process(pid);
        }
        if let Some(ref ns) = ns_set.uts {
            ns.add_process(pid);
        }
        if let Some(ref ns) = ns_set.ipc {
            ns.add_process(pid);
        }
        if let Some(ref ns) = ns_set.network {
            ns.add_process(pid);
        }
        if let Some(ref ns) = ns_set.pid {
            ns.add_process(pid);
        }
        if let Some(ref ns) = ns_set.user {
            ns.add_process(pid);
        }
        if let Some(ref ns) = ns_set.cgroup {
            ns.add_process(pid);
        }

        // 保存进程的命名空间集合
        self.process_namespaces.insert(pid, ns_set.clone());

        crate::println!("[namespace-manager] Assigned namespaces to process {}", pid);
        Ok(())
    }

    /// 克隆命名空间（fork时使用）
    pub fn clone_namespaces(&mut self, parent_pid: u32, child_pid: u32) -> Result<(), i32> {
        let parent_set = self.process_namespaces.get(&parent_pid).ok_or(ENOENT)?;
        let parent_set = parent_set.clone();

        // 子进程继承父进程的命名空间
        self.assign_namespaces(child_pid, &parent_set)?;

        crate::println!(
            "[namespace-manager] Cloned namespaces from {} to {}",
            parent_pid,
            child_pid
        );
        Ok(())
    }

    /// 获取进程的命名空间集合
    pub fn get_process_namespaces(&self, pid: u32) -> Option<NamespaceSet> {
        self.process_namespaces.get(&pid).cloned()
    }

    /// 获取指定类型的所有命名空间
    pub fn get_namespaces_by_type(&self, ns_type: NamespaceType) -> Vec<Arc<Mutex<Namespace>>> {
        if let Some(ns_ids) = self.type_index.get(&ns_type) {
            ns_ids
                .iter()
                .filter_map(|&id| self.namespaces.get(&id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 获取命名空间
    pub fn get_namespace(&self, id: u64) -> Option<Arc<Mutex<Namespace>>> {
        self.namespaces.get(&id).cloned()
    }

    /// 清理进程的命名空间
    pub fn cleanup_process(&mut self, pid: u32) -> Result<(), i32> {
        if let Some(ns_set) = self.process_namespaces.remove(&pid) {
            // 从所有命名空间中移除进程
            if let Some(ref ns) = ns_set.mount {
                ns.remove_process(pid);
            }
            if let Some(ref ns) = ns_set.uts {
                ns.remove_process(pid);
            }
            if let Some(ref ns) = ns_set.ipc {
                ns.remove_process(pid);
            }
            if let Some(ref ns) = ns_set.network {
                ns.remove_process(pid);
            }
            if let Some(ref ns) = ns_set.pid {
                ns.remove_process(pid);
            }
            if let Some(ref ns) = ns_set.user {
                ns.remove_process(pid);
            }
            if let Some(ref ns) = ns_set.cgroup {
                ns.remove_process(pid);
            }

            crate::println!("[namespace-manager] Cleaned up namespaces for process {}", pid);
            Ok(())
        } else {
            Err(ENOENT)
        }
    }

    /// 删除命名空间
    pub fn delete_namespace(&mut self, id: u64) -> Result<(), i32> {
        if let Some(ns_arc) = self.namespaces.remove(&id) {
            let ns_type = {
                let ns = ns_arc.lock();
                ns.ns_type
            };

            // 从类型索引中移除
            if let Some(ref mut ns_ids) = self.type_index.get_mut(&ns_type) {
                ns_ids.retain(|&ns_id| ns_id != id);
                if ns_ids.is_empty() {
                    self.type_index.remove(&ns_type);
                }
            }

            crate::println!("[namespace-manager] Deleted namespace {}", id);
            Ok(())
        } else {
            Err(ENOENT)
        }
    }

    /// 获取命名空间数量
    pub fn namespace_count(&self) -> usize {
        self.namespaces.len()
    }

    /// 获取进程数量
    pub fn process_count(&self) -> usize {
        self.process_namespaces.len()
    }
}

impl Default for NamespaceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局命名空间管理器
static mut NAMESPACE_MANAGER: Option<NamespaceManager> = None;
static mut NAMESPACE_MANAGER_INITIALIZED: bool = false;

/// 初始化命名空间管理器
pub fn init_namespace_manager() -> Result<(), i32> {
    if unsafe { NAMESPACE_MANAGER_INITIALIZED } {
        return Ok(());
    }

    let manager = NamespaceManager::new();

    unsafe {
        NAMESPACE_MANAGER = Some(manager);
        NAMESPACE_MANAGER_INITIALIZED = true;
    }

    crate::println!("[namespace-manager] Initialized");
    Ok(())
}

/// 获取命名空间管理器
pub fn get_namespace_manager() -> Option<&'static mut NamespaceManager> {
    unsafe { NAMESPACE_MANAGER.as_mut() }
}

/// 创建命名空间集合
pub fn create_namespace_set(types: &[NamespaceType]) -> Result<NamespaceSet, i32> {
    let manager = get_namespace_manager().ok_or(EIO)?;
    manager.create_namespaces(types)
}

/// 为进程分配命名空间
pub fn assign_namespaces_to_process(pid: u32, ns_set: &NamespaceSet) -> Result<(), i32> {
    let manager = get_namespace_manager().ok_or(EIO)?;
    manager.assign_namespaces(pid, ns_set)
}

/// 克隆命名空间
pub fn clone_process_namespaces(parent_pid: u32, child_pid: u32) -> Result<(), i32> {
    let manager = get_namespace_manager().ok_or(EIO)?;
    manager.clone_namespaces(parent_pid, child_pid)
}
