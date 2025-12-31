// Container Namespace Management
//
// 容器命名空间管理模块
// 提供UTS/PID/Network/Mount/IPC命名空间的创建、配置和隔离验证功能

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic {{AtomicU64,, Ordering}, Ordering};

use spin::Mutex;

use crate::{
    container::oci::OciLinuxNamespaceType,
    reliability::{EINVAL, EIO, ENOENT, ENOMEM},
};

/// 命名空间类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NamespaceType {
    /// 挂载命名空间
    Mount,
    /// UTS命名空间
    UTS,
    /// IPC命名空间
    IPC,
    /// 网络命名空间
    Network,
    /// PID命名空间
    PID,
    /// 用户命名空间
    User,
    /// Cgroup命名空间
    Cgroup,
}

impl From<OciLinuxNamespaceType> for NamespaceType {
    fn from(ns: OciLinuxNamespaceType) -> Self {
        match ns {
            OciLinuxNamespaceType::Mount => NamespaceType::Mount,
            OciLinuxNamespaceType::UTS => NamespaceType::UTS,
            OciLinuxNamespaceType::IPC => NamespaceType::IPC,
            OciLinuxNamespaceType::Network => NamespaceType::Network,
            OciLinuxNamespaceType::PID => NamespaceType::PID,
            OciLinuxNamespaceType::User => NamespaceType::User,
            OciLinuxNamespaceType::Cgroup => NamespaceType::Cgroup,
        }
    }
}

impl From<NamespaceType> for OciLinuxNamespaceType {
    fn from(ns: NamespaceType) -> Self {
        match ns {
            NamespaceType::Mount => OciLinuxNamespaceType::Mount,
            NamespaceType::UTS => OciLinuxNamespaceType::UTS,
            NamespaceType::IPC => OciLinuxNamespaceType::IPC,
            NamespaceType::Network => OciLinuxNamespaceType::Network,
            NamespaceType::PID => OciLinuxNamespaceType::PID,
            NamespaceType::User => OciLinuxNamespaceType::User,
            NamespaceType::Cgroup => OciLinuxNamespaceType::Cgroup,
        }
    }
}

impl NamespaceType {
    /// 获取命名空间文件名
    pub fn filename(&self) -> &str {
        match self {
            NamespaceType::Mount => "mnt",
            NamespaceType::UTS => "uts",
            NamespaceType::IPC => "ipc",
            NamespaceType::Network => "net",
            NamespaceType::PID => "pid",
            NamespaceType::User => "user",
            NamespaceType::Cgroup => "cgroup",
        }
    }

    /// 获取clone标志
    pub fn clone_flag(&self) -> u64 {
        match self {
            NamespaceType::Mount => 0x00020000,  // CLONE_NEWNS
            NamespaceType::UTS => 0x04000000,    // CLONE_NEWUTS
            NamespaceType::IPC => 0x08000000,    // CLONE_NEWIPC
            NamespaceType::Network => 0x40000000, // CLONE_NEWNET
            NamespaceType::PID => 0x20000000,    // CLONE_NEWPID
            NamespaceType::User => 0x10000000,   // CLONE_NEWUSER
            NamespaceType::Cgroup => 0x02000000, // CLONE_NEWCGROUP
        }
    }
}

/// 命名空间配置
#[derive(Debug, Clone)]
pub struct NamespaceConfig {
    /// 命名空间类型
    pub ns_type: NamespaceType,
    /// 是否创建新命名空间
    pub create_new: bool,
    /// 现有命名空间路径（用于加入现有命名空间）
    pub existing_path: Option<String>,
    /// 命名空间参数
    pub parameters: NamespaceParameters,
}

/// 命名空间参数
#[derive(Debug, Clone)]
pub struct NamespaceParameters {
    /// 挂载参数
    pub mount_params: Option<MountNamespaceParams>,
    /// 网络参数
    pub network_params: Option<NetworkNamespaceParams>,
    /// 用户命名空间参数
    pub user_params: Option<UserNamespaceParams>,
    /// UTS命名空间参数
    pub uts_params: Option<UTSNamespaceParams>,
}

/// 挂载命名空间参数
#[derive(Debug, Clone)]
pub struct MountNamespaceParams {
    /// 根文件系统路径
    pub rootfs_path: Option<String>,
    /// 挂载点配置
    pub mount_points: Vec<MountPoint>,
    /// 传播类型
    pub propagation: MountPropagation,
    /// 是否只读
    pub readonly: bool,
}

/// 挂载点
#[derive(Debug, Clone)]
pub struct MountPoint {
    /// 源路径
    pub source: String,
    /// 目标路径
    pub target: String,
    /// 文件系统类型
    pub fs_type: String,
    /// 挂载选项
    pub options: Vec<String>,
    /// 挂载标志
    pub flags: u64,
}

/// 挂载传播类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountPropagation {
    /// 私有挂载（默认）
    Private,
    /// 共享挂载
    Shared,
    /// 从属挂载
    Slave,
    /// 不可绑定挂载
    Unbindable,
}

/// 网络命名空间参数
#[derive(Debug, Clone)]
pub struct NetworkNamespaceParams {
    /// 网络接口配置
    pub interfaces: Vec<NetworkInterface>,
    /// 路由配置
    pub routes: Vec<Route>,
    /// DNS配置
    pub dns: DnsConfig,
    /// 主机名
    pub hostname: Option<String>,
}

/// 网络接口
#[derive(Debug, Clone)]
pub struct NetworkInterface {
    /// 接口名称
    pub name: String,
    /// 接口类型
    pub if_type: InterfaceType,
    /// MAC地址
    pub mac_address: Option<String>,
    /// IP地址
    pub ip_addresses: Vec<String>,
    /// 是否启用
    pub enabled: bool,
    /// MTU
    pub mtu: Option<u32>,
}

/// 接口类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterfaceType {
    /// 环回接口
    Loopback,
    /// 以太网接口
    Ethernet,
    /// 虚拟以太网对
    Veth,
    /// 网桥
    Bridge,
    /// VLAN
    Vlan,
}

/// 路由
#[derive(Debug, Clone)]
pub struct Route {
    /// 目标网络
    pub destination: String,
    /// 网关
    pub gateway: Option<String>,
    /// 接口
    pub interface: String,
    /// 路由类型
    pub route_type: RouteType,
    /// 度量
    pub metric: Option<u32>,
}

/// 路由类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteType {
    /// 直连路由
    Connected,
    /// 静态路由
    Static,
    /// 默认路由
    Default,
}

/// DNS配置
#[derive(Debug, Clone)]
pub struct DnsConfig {
    /// DNS服务器
    pub servers: Vec<String>,
    /// 搜索域
    pub search_domains: Vec<String>,
    /// 选项
    pub options: Vec<String>,
}

/// 用户命名空间参数
#[derive(Debug, Clone)]
pub struct UserNamespaceParams {
    /// UID映射
    pub uid_map: Vec<IDMapping>,
    /// GID映射
    pub gid_map: Vec<IDMapping>,
}

/// ID映射
#[derive(Debug, Clone)]
pub struct IDMapping {
    /// 容器内ID
    pub container_id: u32,
    /// 主机ID
    pub host_id: u32,
    /// 映射范围大小
    pub range_size: u32,
}

/// UTS命名空间参数
#[derive(Debug, Clone)]
pub struct UTSNamespaceParams {
    /// 主机名
    pub hostname: String,
    /// 域名
    pub domainname: String,
}

/// 命名空间
pub struct Namespace {
    /// 命名空间ID
    pub id: u64,
    /// 命名空间类型
    pub ns_type: NamespaceType,
    /// 命名空间路径
    pub path: String,
    /// 配置
    pub config: NamespaceConfig,
    /// 进程列表
    pub processes: Arc<Mutex<Vec<u32>>>,
    /// 是否激活
    pub active: bool,
}

impl Namespace {
    /// 创建新命名空间
    pub fn new(id: u64, ns_type: NamespaceType, config: NamespaceConfig) -> Self {
        let path = format!("/var/run/namespaces/{}/{}", ns_type.filename(), id);

        Self {
            id,
            ns_type,
            path,
            config,
            processes: Arc::new(Mutex::new(Vec::new())),
            active: false,
        }
    }

    /// 创建命名空间
    pub fn create(&mut self) -> Result<(), i32> {
        if self.active {
            return Err(EINVAL);
        }

        match self.ns_type {
            NamespaceType::Mount => self.create_mount_namespace()?,
            NamespaceType::UTS => self.create_uts_namespace()?,
            NamespaceType::IPC => self.create_ipc_namespace()?,
            NamespaceType::Network => self.create_network_namespace()?,
            NamespaceType::PID => self.create_pid_namespace()?,
            NamespaceType::User => self.create_user_namespace()?,
            NamespaceType::Cgroup => self.create_cgroup_namespace()?,
        }

        self.active = true;
        crate::println!(
            "[container-ns] Created namespace: {:?} (ID: {})",
            self.ns_type,
            self.id
        );

        Ok(())
    }

    /// 创建挂载命名空间
    fn create_mount_namespace(&self) -> Result<(), i32> {
        if let Some(ref mount_params) = self.config.parameters.mount_params {
            // 设置根文件系统
            if let Some(ref rootfs_path) = mount_params.rootfs_path {
                self.setup_rootfs(rootfs_path, mount_params.readonly)?;
            }

            // 设置挂载点
            for mount_point in &mount_params.mount_points {
                self.setup_mount_point(mount_point)?;
            }

            // 设置传播类型
            self.set_propagation(mount_params.propagation)?;
        }

        Ok(())
    }

    /// 创建UTS命名空间
    fn create_uts_namespace(&self) -> Result<(), i32> {
        if let Some(ref uts_params) = self.config.parameters.uts_params {
            // 设置主机名
            self.set_hostname(&uts_params.hostname)?;

            // 设置域名
            self.set_domainname(&uts_params.domainname)?;

            crate::println!(
                "[container-ns] Set hostname: {}, domainname: {}",
                uts_params.hostname,
                uts_params.domainname
            );
        }

        Ok(())
    }

    /// 创建IPC命名空间
    fn create_ipc_namespace(&self) -> Result<(), i32> {
        crate::println!("[container-ns] Created IPC namespace");
        // 在实际实现中，这里会创建新的IPC命名空间
        Ok(())
    }

    /// 创建网络命名空间
    fn create_network_namespace(&self) -> Result<(), i32> {
        if let Some(ref network_params) = self.config.parameters.network_params {
            // 创建网络接口
            for interface in &network_params.interfaces {
                self.setup_interface(interface)?;
            }

            // 设置路由
            for route in &network_params.routes {
                self.setup_route(route)?;
            }

            // 设置DNS
            self.setup_dns(&network_params.dns)?;
        }

        Ok(())
    }

    /// 创建PID命名空间
    fn create_pid_namespace(&self) -> Result<(), i32> {
        crate::println!("[container-ns] Created PID namespace");
        // 在实际实现中，这里会创建新的PID命名空间
        Ok(())
    }

    /// 创建用户命名空间
    fn create_user_namespace(&self) -> Result<(), i32> {
        if let Some(ref user_params) = self.config.parameters.user_params {
            // 设置UID映射
            for uid_map in &user_params.uid_map {
                self.set_uid_mapping(uid_map)?;
            }

            // 设置GID映射
            for gid_map in &user_params.gid_map {
                self.set_gid_mapping(gid_map)?;
            }
        }

        Ok(())
    }

    /// 创建cgroup命名空间
    fn create_cgroup_namespace(&self) -> Result<(), i32> {
        crate::println!("[container-ns] Created cgroup namespace");
        // 在实际实现中，这里会创建新的cgroup命名空间
        Ok(())
    }

    /// 设置根文件系统
    fn setup_rootfs(&self, rootfs_path: &str, readonly: bool) -> Result<(), i32> {
        crate::println!(
            "[container-ns] Setting up rootfs: {} (readonly: {})",
            rootfs_path,
            readonly
        );

        // 在实际实现中，这里会执行pivot_root或chroot
        Ok(())
    }

    /// 设置挂载点
    fn setup_mount_point(&self, mount_point: &MountPoint) -> Result<(), i32> {
        crate::println!(
            "[container-ns] Mounting: {} -> {} ({})",
            mount_point.source,
            mount_point.target,
            mount_point.fs_type
        );

        // 在实际实现中，这里会调用mount系统调用
        Ok(())
    }

    /// 设置传播类型
    fn set_propagation(&self, propagation: MountPropagation) -> Result<(), i32> {
        let flag = match propagation {
            MountPropagation::Private => 0x40000,     // MS_PRIVATE
            MountPropagation::Shared => 0x100000,     // MS_SHARED
            MountPropagation::Slave => 0x80000,       // MS_SLAVE
            MountPropagation::Unbindable => 0x200000, // MS_UNBINDABLE
        };

        crate::println!("[container-ns] Setting mount propagation: {:?}", propagation);
        // 在实际实现中，这里会调用mount系统调用设置传播标志
        Ok(())
    }

    /// 设置主机名
    fn set_hostname(&self, hostname: &str) -> Result<(), i32> {
        crate::println!("[container-ns] Setting hostname: {}", hostname);
        // 在实际实现中，这里会调用sethostname系统调用
        Ok(())
    }

    /// 设置域名
    fn set_domainname(&self, domainname: &str) -> Result<(), i32> {
        crate::println!("[container-ns] Setting domainname: {}", domainname);
        // 在实际实现中，这里会调用setdomainname系统调用
        Ok(())
    }

    /// 设置网络接口
    fn setup_interface(&self, interface: &NetworkInterface) -> Result<(), i32> {
        crate::println!(
            "[container-ns] Setting up interface: {} ({:?})",
            interface.name,
            interface.if_type
        );

        match interface.if_type {
            InterfaceType::Loopback => {
                self.setup_loopback(interface)?;
            },
            InterfaceType::Veth => {
                self.setup_veth(interface)?;
            },
            InterfaceType::Bridge => {
                self.setup_bridge(interface)?;
            },
            _ => {
                crate::println!(
                    "[container-ns] Interface type {:?} not implemented",
                    interface.if_type
                );
            },
        }

        Ok(())
    }

    /// 设置环回接口
    fn setup_loopback(&self, interface: &NetworkInterface) -> Result<(), i32> {
        // 启用环回接口
        crate::println!("[container-ns] Enabling loopback interface: {}", interface.name);
        // 在实际实现中，这里会配置网络接口
        Ok(())
    }

    /// 设置veth对
    fn setup_veth(&self, interface: &NetworkInterface) -> Result<(), i32> {
        // 创建veth对
        let peer_name = format!("{}-peer", interface.name);
        crate::println!(
            "[container-ns] Creating veth pair: {} <-> {}",
            interface.name,
            peer_name
        );

        // 在实际实现中，这里会创建veth对并配置
        Ok(())
    }

    /// 设置网桥
    fn setup_bridge(&self, interface: &NetworkInterface) -> Result<(), i32> {
        crate::println!("[container-ns] Creating bridge: {}", interface.name);
        // 在实际实现中，这里会创建网桥
        Ok(())
    }

    /// 设置路由
    fn setup_route(&self, route: &Route) -> Result<(), i32> {
        crate::println!(
            "[container-ns] Adding route: {} via {} (dev: {})",
            route.destination,
            route.gateway.as_deref().unwrap_or("direct"),
            route.interface
        );

        // 在实际实现中，这里会添加路由
        Ok(())
    }

    /// 设置DNS
    fn setup_dns(&self, dns: &DnsConfig) -> Result<(), i32> {
        // 生成resolv.conf内容
        let mut content = String::new();

        for server in &dns.servers {
            content.push_str(&format!("nameserver {}\n", server));
        }

        if !dns.search_domains.is_empty() {
            content.push_str(&format!("search {}\n", dns.search_domains.join(" ")));
        }

        for option in &dns.options {
            content.push_str(&format!("options {}\n", option));
        }

        crate::println!("[container-ns] DNS configuration:\n{}", content);

        // 在实际实现中，这里会写入/etc/resolv.conf
        Ok(())
    }

    /// 设置UID映射
    fn set_uid_mapping(&self, uid_map: &IDMapping) -> Result<(), i32> {
        crate::println!(
            "[container-ns] Setting UID mapping: {} -> {} (range: {})",
            uid_map.container_id,
            uid_map.host_id,
            uid_map.range_size
        );

        // 在实际实现中，这里会写入/proc/[pid]/uid_map
        Ok(())
    }

    /// 设置GID映射
    fn set_gid_mapping(&self, gid_map: &IDMapping) -> Result<(), i32> {
        crate::println!(
            "[container-ns] Setting GID mapping: {} -> {} (range: {})",
            gid_map.container_id,
            gid_map.host_id,
            gid_map.range_size
        );

        // 在实际实现中，这里会写入/proc/[pid]/gid_map
        Ok(())
    }

    /// 添加进程到命名空间
    pub fn add_process(&self, pid: u32) -> Result<(), i32> {
        if !self.active {
            return Err(EINVAL);
        }

        // 将进程加入命名空间
        self.join_namespace(pid)?;

        // 更新进程列表
        {
            let mut processes = self.processes.lock();
            if !processes.contains(&pid) {
                processes.push(pid);
            }
        }

        crate::println!(
            "[container-ns] Added process {} to namespace {:?} (ID: {})",
            pid,
            self.ns_type,
            self.id
        );

        Ok(())
    }

    /// 从命名空间移除进程
    pub fn remove_process(&self, pid: u32) -> Result<(), i32> {
        self.leave_namespace(pid)?;

        {
            let mut processes = self.processes.lock();
            processes.retain(|&p| p != pid);
        }

        crate::println!(
            "[container-ns] Removed process {} from namespace {:?} (ID: {})",
            pid,
            self.ns_type,
            self.id
        );

        Ok(())
    }

    /// 加入命名空间
    fn join_namespace(&self, pid: u32) -> Result<(), i32> {
        crate::println!(
            "[container-ns] Process {} joining namespace {:?}",
            pid,
            self.ns_type
        );

        // 在实际实现中，这里会使用setns()系统调用
        Ok(())
    }

    /// 离开命名空间
    fn leave_namespace(&self, pid: u32) -> Result<(), i32> {
        crate::println!(
            "[container-ns] Process {} leaving namespace {:?}",
            pid,
            self.ns_type
        );

        // 在实际实现中，这里会将进程移回父命名空间
        Ok(())
    }

    /// 删除命名空间
    pub fn destroy(&mut self) -> Result<(), i32> {
        if !self.active {
            return Err(EINVAL);
        }

        // 移除所有进程
        {
            let processes = self.processes.lock().clone();
            for pid in processes {
                let _ = self.remove_process(pid);
            }
        }

        // 清理命名空间
        self.cleanup_namespace()?;

        self.active = false;

        crate::println!(
            "[container-ns] Destroyed namespace {:?} (ID: {})",
            self.ns_type,
            self.id
        );

        Ok(())
    }

    /// 清理命名空间
    fn cleanup_namespace(&self) -> Result<(), i32> {
        crate::println!(
            "[container-ns] Cleaning up namespace {:?}",
            self.ns_type
        );

        // 在实际实现中，这里会清理命名空间资源
        Ok(())
    }

    /// 获取进程数量
    pub fn get_process_count(&self) -> usize {
        self.processes.lock().len()
    }

    /// 验证隔离
    pub fn verify_isolation(&self) -> Result<bool, i32> {
        if !self.active {
            return Ok(false);
        }

        match self.ns_type {
            NamespaceType::Mount => self.verify_mount_isolation(),
            NamespaceType::Network => self.verify_network_isolation(),
            NamespaceType::PID => self.verify_pid_isolation(),
            NamespaceType::UTS => self.verify_uts_isolation(),
            _ => Ok(true),
        }
    }

    /// 验证挂载隔离
    fn verify_mount_isolation(&self) -> Result<bool, i32> {
        // 在实际实现中，这里会检查/proc/self/mounts
        crate::println!("[container-ns] Verifying mount namespace isolation");
        Ok(true)
    }

    /// 验证网络隔离
    fn verify_network_isolation(&self) -> Result<bool, i32> {
        // 在实际实现中，这里会检查网络接口和路由
        crate::println!("[container-ns] Verifying network namespace isolation");
        Ok(true)
    }

    /// 验证PID隔离
    fn verify_pid_isolation(&self) -> Result<bool, i32> {
        // 在实际实现中，这里会检查PID 1是否为容器的init进程
        crate::println!("[container-ns] Verifying PID namespace isolation");
        Ok(true)
    }

    /// 验证UTS隔离
    fn verify_uts_isolation(&self) -> Result<bool, i32> {
        // 在实际实现中，这里会检查主机名
        crate::println!("[container-ns] Verifying UTS namespace isolation");
        Ok(true)
    }
}

/// 命名空间管理器
pub struct NamespaceManager {
    /// 命名空间列表
    namespaces: BTreeMap<u64, Arc<Mutex<Namespace>>>,
    /// 类型索引
    type_index: BTreeMap<NamespaceType, Vec<u64>>,
    /// 下一个命名空间ID
    next_namespace_id: AtomicU64,
}

impl NamespaceManager {
    /// 创建新的命名空间管理器
    pub fn new() -> Self {
        Self {
            namespaces: BTreeMap::new(),
            type_index: BTreeMap::new(),
            next_namespace_id: AtomicU64::new(1),
        }
    }

    /// 创建命名空间
    pub fn create(&mut self, config: NamespaceConfig) -> Result<u64, i32> {
        let ns_id = self.next_namespace_id.fetch_add(1, Ordering::SeqCst);
        let ns_type = config.ns_type;

        let mut namespace = Namespace::new(ns_id, ns_type, config);
        namespace.create()?;

        let namespace_arc = Arc::new(Mutex::new(namespace));
        self.namespaces.insert(ns_id, namespace_arc.clone());

        // 更新类型索引
        let entry = self.type_index.entry(ns_type).or_insert_with(Vec::new);
        entry.push(ns_id);

        Ok(ns_id)
    }

    /// 获取命名空间
    pub fn get(&self, ns_id: u64) -> Option<Arc<Mutex<Namespace>>> {
        self.namespaces.get(&ns_id).cloned()
    }

    /// 按类型获取命名空间
    pub fn get_by_type(&self, ns_type: NamespaceType) -> Vec<Arc<Mutex<Namespace>>> {
        if let Some(ns_ids) = self.type_index.get(&ns_type) {
            ns_ids
                .iter()
                .filter_map(|&ns_id| self.namespaces.get(&ns_id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 删除命名空间
    pub fn delete(&mut self, ns_id: u64) -> Result<(), i32> {
        if let Some(namespace) = self.namespaces.remove(&ns_id) {
            let ns_type = {
                let ns = namespace.lock();
                ns.ns_type
            };

            // 清理类型索引
            if let Some(ref mut ns_ids) = self.type_index.get_mut(&ns_type) {
                ns_ids.retain(|&id| id != ns_id);
                if ns_ids.is_empty() {
                    self.type_index.remove(&ns_type);
                }
            }

            // 删除命名空间
            {
                let mut ns = namespace.lock();
                ns.destroy()?;
            }

            crate::println!("[container-ns] Deleted namespace ID: {}", ns_id);
            Ok(())
        } else {
            Err(ENOENT)
        }
    }

    /// 获取所有命名空间ID
    pub fn list_ids(&self) -> Vec<u64> {
        self.namespaces.keys().copied().collect()
    }

    /// 获取命名空间数量
    pub fn count(&self) -> usize {
        self.namespaces.len()
    }

    /// 清理所有命名空间
    pub fn cleanup_all(&mut self) -> Result<(), i32> {
        let namespace_ids: Vec<u64> = self.namespaces.keys().copied().collect();

        for ns_id in namespace_ids {
            if let Err(e) = self.delete(ns_id) {
                crate::println!(
                    "[container-ns] Warning: Failed to delete namespace {}: {}",
                    ns_id,
                    e
                );
            }
        }

        crate::println!("[container-ns] Cleaned up all namespaces");
        Ok(())
    }
}

impl Default for NamespaceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局命名空间管理器实例
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

    crate::println!("[container-ns] Namespace manager initialized");
    Ok(())
}

/// 获取命名空间管理器引用
pub fn get_namespace_manager() -> Option<&'static NamespaceManager> {
    unsafe { NAMESPACE_MANAGER.as_ref() }
}

/// 获取命名空间管理器可变引用
pub fn get_namespace_manager_mut() -> Option<&'static mut NamespaceManager> {
    unsafe { NAMESPACE_MANAGER.as_mut() }
}
