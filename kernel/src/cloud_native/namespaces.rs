// Linux Namespaces Support Module
//
// Linux命名空间支持模块
// 提供进程隔离和虚拟化功能，支持容器运行时的命名空间管理

extern crate alloc;

use alloc::format;
use crate::reliability::errno::{EINVAL, ENOENT, ENOMEM, EIO, EPERM, EACCES};
use crate::cloud_native::oci::OciLinuxNamespaceType;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use spin::Mutex;
use alloc::vec;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

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
    /// 时间命名空间
    Time,
}

impl From<NamespaceType> for OciLinuxNamespaceType {
    fn from(ns_type: NamespaceType) -> Self {
        match ns_type {
            NamespaceType::Mount => OciLinuxNamespaceType::Mount,
            NamespaceType::UTS => OciLinuxNamespaceType::UTS,
            NamespaceType::IPC => OciLinuxNamespaceType::IPC,
            NamespaceType::Network => OciLinuxNamespaceType::Network,
            NamespaceType::PID => OciLinuxNamespaceType::PID,
            NamespaceType::User => OciLinuxNamespaceType::User,
            NamespaceType::Cgroup => OciLinuxNamespaceType::Cgroup,
            NamespaceType::Time => panic!("Time namespace not supported in OCI"),
        }
    }
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

/// 命名空间配置
#[derive(Debug, Clone)]
pub struct NamespaceConfig {
    /// 命名空间类型
    pub ns_type: NamespaceType,
    /// 是否为新命名空间
    pub new_namespace: bool,
    /// 现有命名空间路径（用于加入现有命名空间）
    pub existing_path: Option<String>,
    /// 命名空间参数
    pub parameters: NamespaceParameters,
}

/// 命名空间参数
#[derive(Debug, Clone)]
pub struct NamespaceParameters {
    /// 挂载参数
    pub mount_params: Option<MountParameters>,
    /// 网络参数
    pub network_params: Option<NetworkParameters>,
    /// 用户命名空间参数
    pub user_params: Option<UserNamespaceParameters>,
    /// UTS命名空间参数
    pub uts_params: Option<UTSParameters>,
}

/// 挂载参数
#[derive(Debug, Clone)]
pub struct MountParameters {
    /// 根文件系统路径
    pub rootfs_path: Option<String>,
    /// 挂载点配置
    pub mount_points: Vec<MountPoint>,
    /// 传播类型
    pub propagation: MountPropagation,
    /// 是否为只读
    pub read_only: bool,
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
    /// 私有挂载
    Private,
    /// 共享挂载
    Shared,
    /// 从属挂载
    Slave,
    /// 不可绑定挂载
    Unbindable,
}

/// 网络参数
#[derive(Debug, Clone)]
pub struct NetworkParameters {
    /// 网络接口配置
    pub interfaces: Vec<NetworkInterface>,
    /// 路由配置
    pub routes: Vec<Route>,
    /// DNS配置
    pub dns: DNSConfig,
    /// 主机名
    pub hostname: Option<String>,
    /// 域名
    pub domainname: Option<String>,
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
    /// 虚拟以太网接口
    Veth,
    /// 网桥接口
    Bridge,
    /// VLAN接口
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
pub struct DNSConfig {
    /// DNS服务器
    pub servers: Vec<String>,
    /// 搜索域
    pub search_domains: Vec<String>,
    /// 选项
    pub options: Vec<String>,
}

/// 用户命名空间参数
#[derive(Debug, Clone)]
pub struct UserNamespaceParameters {
    /// UID映射
    pub uid_map: Vec<IdMapping>,
    /// GID映射
    pub gid_map: Vec<IdMapping>,
}

/// ID映射
#[derive(Debug, Clone)]
pub struct IdMapping {
    /// 容器内ID
    pub container_id: u32,
    /// 主机ID
    pub host_id: u32,
    /// 映射范围大小
    pub range_size: u32,
}

/// UTS命名空间参数
#[derive(Debug, Clone)]
pub struct UTSParameters {
    /// 主机名
    pub hostname: String,
    /// 域名
    pub domainname: String,
}

/// 命名空间
pub struct Namespace {
    /// 命名空间ID
    pub ns_id: u64,
    /// 命名空间类型
    pub ns_type: NamespaceType,
    /// 命名空间路径
    pub path: String,
    /// 命名空间配置
    pub config: NamespaceConfig,
    /// 进程列表
    pub processes: Arc<Mutex<Vec<u32>>>,
    /// 是否激活
    pub active: bool,
}

impl Namespace {
    /// 创建新的命名空间
    pub fn new(ns_id: u64, ns_type: NamespaceType, config: NamespaceConfig) -> Self {
        let path = format!("/proc/{}/ns/{:?}", get_current_pid(), ns_type);

        Self {
            ns_id,
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

        // 创建命名空间
        match self.ns_type {
            NamespaceType::Mount => self.create_mount_namespace()?,
            NamespaceType::UTS => self.create_uts_namespace()?,
            NamespaceType::IPC => self.create_ipc_namespace()?,
            NamespaceType::Network => self.create_network_namespace()?,
            NamespaceType::PID => self.create_pid_namespace()?,
            NamespaceType::User => self.create_user_namespace()?,
            NamespaceType::Cgroup => self.create_cgroup_namespace()?,
            NamespaceType::Time => self.create_time_namespace()?,
        }

        self.active = true;
        crate::println!("[namespaces] Created namespace: {:?} (ID: {})", self.ns_type, self.ns_id);
        Ok(())
    }

    /// 创建挂载命名空间
    fn create_mount_namespace(&self) -> Result<(), i32> {
        if let Some(ref mount_params) = self.config.parameters.mount_params {
            // 设置根文件系统
            if let Some(ref rootfs_path) = mount_params.rootfs_path {
                self.setup_rootfs(rootfs_path, mount_params.read_only)?;
            }

            // 设置挂载点
            for mount_point in &mount_params.mount_points {
                self.setup_mount_point(mount_point)?;
            }

            // 设置传播类型
            self.set_mount_propagation(mount_params.propagation)?;
        }

        Ok(())
    }

    /// 创建UTS命名空间
    fn create_uts_namespace(&self) -> Result<(), i32> {
        if let Some(ref uts_params) = self.config.parameters.uts_params {
            // 设置主机名
            crate::syscalls::process::set_hostname(&uts_params.hostname)?;

            // 设置域名
            crate::syscalls::process::set_domainname(&uts_params.domainname)?;

            crate::println!("[namespaces] Set hostname: {}, domainname: {}",
                uts_params.hostname, uts_params.domainname);
        }

        Ok(())
    }

    /// 创建IPC命名空间
    fn create_ipc_namespace(&self) -> Result<(), i32> {
        crate::println!("[namespaces] Created IPC namespace");
        // 在实际实现中，这里会设置新的IPC命名空间
        Ok(())
    }

    /// 创建网络命名空间
    fn create_network_namespace(&self) -> Result<(), i32> {
        if let Some(ref network_params) = self.config.parameters.network_params {
            // 创建网络接口
            for interface in &network_params.interfaces {
                self.setup_network_interface(interface)?;
            }

            // 设置路由
            for route in &network_params.routes {
                self.setup_route(route)?;
            }

            // 设置DNS
            self.setup_dns(&network_params.dns)?;

            // 设置主机名和域名
            if let Some(ref hostname) = network_params.hostname {
                crate::syscalls::process::set_hostname(hostname)?;
            }
            if let Some(ref domainname) = network_params.domainname {
                crate::syscalls::process::set_domainname(domainname)?;
            }
        }

        Ok(())
    }

    /// 创建PID命名空间
    fn create_pid_namespace(&self) -> Result<(), i32> {
        crate::println!("[namespaces] Created PID namespace");
        // 在实际实现中，这里会设置新的PID命名空间
        Ok(())
    }

    /// 创建用户命名空间
    fn create_user_namespace(&self) -> Result<(), i32> {
        if let Some(ref user_params) = self.config.parameters.user_params {
            // 设置UID映射
            for uid_map in &user_params.uid_map {
                self.setup_uid_mapping(uid_map)?;
            }

            // 设置GID映射
            for gid_map in &user_params.gid_map {
                self.setup_gid_mapping(gid_map)?;
            }
        }

        Ok(())
    }

    /// 创建cgroup命名空间
    fn create_cgroup_namespace(&self) -> Result<(), i32> {
        crate::println!("[namespaces] Created cgroup namespace");
        // 在实际实现中，这里会设置新的cgroup命名空间
        Ok(())
    }

    /// 创建时间命名空间
    fn create_time_namespace(&self) -> Result<(), i32> {
        crate::println!("[namespaces] Created time namespace");
        // 在实际实现中，这里会设置新的时间命名空间
        Ok(())
    }

    /// 设置根文件系统
    fn setup_rootfs(&self, rootfs_path: &str, read_only: bool) -> Result<(), i32> {
        crate::println!("[namespaces] Setting up rootfs: {} (readonly: {})", rootfs_path, read_only);

        // 挂载根文件系统
        let flags = if read_only { 0x1 } else { 0x0 }; // MS_RDONLY
        crate::syscalls::fs::mount("rootfs", "/", Some(rootfs_path), flags).map_err(|_| -1)?;

        Ok(())
    }

    /// 设置挂载点
    fn setup_mount_point(&self, mount_point: &MountPoint) -> Result<(), i32> {
        crate::println!("[namespaces] Setting up mount point: {} -> {} ({})",
            mount_point.source, mount_point.target, mount_point.fs_type);

        // 挂载文件系统
        let _options_str = mount_point.options.join(",");
        crate::syscalls::fs::mount(
            &mount_point.fs_type,
            &mount_point.target,
            Some(&mount_point.source[..]),
            mount_point.flags as u32,
        ).map_err(|_| -1)?;

        Ok(())
    }

    /// 设置挂载传播
    fn set_mount_propagation(&self, propagation: MountPropagation) -> Result<(), i32> {
        let mount_flag = match propagation {
            MountPropagation::Private => 0x40000, // MS_PRIVATE
            MountPropagation::Shared => 0x100000,  // MS_SHARED
            MountPropagation::Slave => 0x80000,    // MS_SLAVE
            MountPropagation::Unbindable => 0x200000, // MS_UNBINDABLE
        };

        crate::println!("[namespaces] Setting mount propagation: {:?}", propagation);
        crate::syscalls::fs::mount("none", "/", None, mount_flag).map_err(|_| -1)?;

        Ok(())
    }

    /// 设置网络接口
    fn setup_network_interface(&self, interface: &NetworkInterface) -> Result<(), i32> {
        crate::println!("[namespaces] Setting up network interface: {} ({:?})", interface.name, interface.if_type);

        // 根据接口类型进行配置
        match interface.if_type {
            InterfaceType::Loopback => {
                self.setup_loopback_interface(interface)?;
            }
            InterfaceType::Veth => {
                self.setup_veth_interface(interface)?;
            }
            InterfaceType::Bridge => {
                self.setup_bridge_interface(interface)?;
            }
            _ => {
                crate::println!("[namespaces] Interface type {:?} not implemented", interface.if_type);
            }
        }

        Ok(())
    }

    /// 设置环回接口
    fn setup_loopback_interface(&self, interface: &NetworkInterface) -> Result<(), i32> {
        // 启用环回接口
        crate::syscalls::network::interface_up(&interface.name)?;

        // 设置IP地址
        for ip in &interface.ip_addresses {
            crate::syscalls::network::add_interface_address(&interface.name, ip, "127.0.0.1")?;
        }

        Ok(())
    }

    /// 设置veth接口
    fn setup_veth_interface(&self, interface: &NetworkInterface) -> Result<(), i32> {
        // 创建veth对
        let peer_name = format!("{}-peer", interface.name);
        crate::syscalls::network::create_veth_pair(&interface.name, &peer_name)?;

        // 启用接口
        crate::syscalls::network::interface_up(&interface.name)?;

        // 设置IP地址
        for ip in &interface.ip_addresses {
            crate::syscalls::network::add_interface_address(&interface.name, ip, "")?;
        }

        // 设置MTU
        if let Some(mtu) = interface.mtu {
            crate::syscalls::network::set_interface_mtu(&interface.name, mtu)?;
        }

        Ok(())
    }

    /// 设置网桥接口
    fn setup_bridge_interface(&self, interface: &NetworkInterface) -> Result<(), i32> {
        // 创建网桥
        crate::syscalls::network::create_bridge(&interface.name)?;

        // 启用网桥
        crate::syscalls::network::interface_up(&interface.name)?;

        // 设置IP地址
        for ip in &interface.ip_addresses {
            crate::syscalls::network::add_interface_address(&interface.name, ip, "")?;
        }

        // 设置MTU
        if let Some(mtu) = interface.mtu {
            crate::syscalls::network::set_interface_mtu(&interface.name, mtu)?;
        }

        Ok(())
    }

    /// 设置路由
    fn setup_route(&self, route: &Route) -> Result<(), i32> {
        crate::println!("[namespaces] Setting up route: {} via {} (dev: {})",
            route.destination,
            route.gateway.as_deref().unwrap_or("direct"),
            route.interface);

        match route.route_type {
            RouteType::Default => {
                if let Some(ref gateway) = route.gateway {
                    crate::syscalls::network::add_route("0.0.0.0/0", gateway, &route.interface)?;
                }
            }
            RouteType::Static => {
                if let Some(ref gateway) = route.gateway {
                    crate::syscalls::network::add_route(&route.destination, gateway, &route.interface)?;
                } else {
                    crate::syscalls::network::add_route(&route.destination, "", &route.interface)?;
                }
            }
            RouteType::Connected => {
                crate::syscalls::network::add_route(&route.destination, "", &route.interface)?;
            }
        }

        Ok(())
    }

    /// 设置DNS
    fn setup_dns(&self, dns_config: &DNSConfig) -> Result<(), i32> {
        // 写入resolv.conf
        let resolv_conf_path = "/etc/resolv.conf";
        let mut content = String::new();

        for server in &dns_config.servers {
            content.push_str(&format!("nameserver {}\n", server));
        }

        if !dns_config.search_domains.is_empty() {
            content.push_str(&format!("search {}\n", dns_config.search_domains.join(" ")));
        }

        for option in &dns_config.options {
            content.push_str(&format!("options {}\n", option));
        }

        // 在实际实现中，这里会写入文件系统
        crate::println!("[namespaces] DNS configuration:\n{}", content);

        Ok(())
    }

    /// 设置UID映射
    fn setup_uid_mapping(&self, uid_map: &IdMapping) -> Result<(), i32> {
        crate::println!("[namespaces] Setting up UID mapping: {} -> {} (range: {})",
            uid_map.container_id, uid_map.host_id, uid_map.range_size);

        // 在实际实现中，这里会写入/proc/[pid]/uid_map
        let map_str = format!("{} {} {}", uid_map.container_id, uid_map.host_id, uid_map.range_size);
        crate::println!("[namespaces] UID map: {}", map_str);

        Ok(())
    }

    /// 设置GID映射
    fn setup_gid_mapping(&self, gid_map: &IdMapping) -> Result<(), i32> {
        crate::println!("[namespaces] Setting up GID mapping: {} -> {} (range: {})",
            gid_map.container_id, gid_map.host_id, gid_map.range_size);

        // 在实际实现中，这里会写入/proc/[pid]/gid_map
        let map_str = format!("{} {} {}", gid_map.container_id, gid_map.host_id, gid_map.range_size);
        crate::println!("[namespaces] GID map: {}", map_str);

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

        crate::println!("[namespaces] Added process {} to namespace {:?} (ID: {})",
            pid, self.ns_type, self.ns_id);
        Ok(())
    }

    /// 从命名空间移除进程
    pub fn remove_process(&self, pid: u32) -> Result<(), i32> {
        // 将进程移出命名空间
        self.leave_namespace(pid)?;

        // 更新进程列表
        {
            let mut processes = self.processes.lock();
            processes.retain(|&p| p != pid);
        }

        crate::println!("[namespaces] Removed process {} from namespace {:?} (ID: {})",
            pid, self.ns_type, self.ns_id);
        Ok(())
    }

    /// 加入命名空间
    fn join_namespace(&self, pid: u32) -> Result<(), i32> {
        // 在实际实现中，这里会使用setns()系统调用
        crate::println!("[namespaces] Joining namespace {:?} for process {}", self.ns_type, pid);
        Ok(())
    }

    /// 离开命名空间
    fn leave_namespace(&self, pid: u32) -> Result<(), i32> {
        // 在实际实现中，这里会将进程移回父命名空间
        crate::println!("[namespaces] Leaving namespace {:?} for process {}", self.ns_type, pid);
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
        crate::println!("[namespaces] Destroyed namespace {:?} (ID: {})", self.ns_type, self.ns_id);
        Ok(())
    }

    /// 清理命名空间
    fn cleanup_namespace(&self) -> Result<(), i32> {
        match self.ns_type {
            NamespaceType::Mount => self.cleanup_mount_namespace()?,
            NamespaceType::Network => self.cleanup_network_namespace()?,
            _ => {
                // 其他命名空间的清理
                crate::println!("[namespaces] Cleaning up namespace {:?}", self.ns_type);
            }
        }
        Ok(())
    }

    /// 清理挂载命名空间
    fn cleanup_mount_namespace(&self) -> Result<(), i32> {
        crate::println!("[namespaces] Cleaning up mount namespace");
        // 在实际实现中，这里会卸载所有挂载点
        Ok(())
    }

    /// 清理网络命名空间
    fn cleanup_network_namespace(&self) -> Result<(), i32> {
        crate::println!("[namespaces] Cleaning up network namespace");
        // 在实际实现中，这里会删除所有网络接口和路由
        Ok(())
    }

    /// 获取进程数量
    pub fn get_process_count(&self) -> usize {
        self.processes.lock().len()
    }

    /// 获取进程列表
    pub fn get_processes(&self) -> Vec<u32> {
        self.processes.lock().clone()
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
    /// 命名空间数量
    namespace_count: AtomicUsize,
}

impl NamespaceManager {
    /// 创建新的命名空间管理器
    pub fn new() -> Self {
        Self {
            namespaces: BTreeMap::new(),
            type_index: BTreeMap::new(),
            next_namespace_id: AtomicU64::new(1),
            namespace_count: AtomicUsize::new(0),
        }
    }

    /// 创建命名空间
    pub fn create_namespace(&mut self, ns_type: NamespaceType, config: NamespaceConfig) -> Result<u64, i32> {
        let ns_id = self.next_namespace_id.fetch_add(1, Ordering::SeqCst);
        let mut namespace = Namespace::new(ns_id, ns_type, config);

        namespace.create()?;

        let namespace_arc = Arc::new(Mutex::new(namespace));
        self.namespaces.insert(ns_id, namespace_arc.clone());

        // 更新类型索引
        let entry = self.type_index.entry(ns_type).or_insert_with(Vec::new);
        entry.push(ns_id);

        self.namespace_count.fetch_add(1, Ordering::SeqCst);

        crate::println!("[namespaces] Created namespace: {:?} (ID: {})", ns_type, ns_id);
        Ok(ns_id)
    }

    /// 获取命名空间
    pub fn get_namespace(&self, ns_id: u64) -> Option<Arc<Mutex<Namespace>>> {
        self.namespaces.get(&ns_id).cloned()
    }

    /// 按类型获取命名空间
    pub fn get_namespaces_by_type(&self, ns_type: NamespaceType) -> Vec<Arc<Mutex<Namespace>>> {
        if let Some(ns_ids) = self.type_index.get(&ns_type) {
            ns_ids.iter()
                .filter_map(|&ns_id| self.namespaces.get(&ns_id).cloned())
                .collect()
        } else {
            Vec::new()
        }
    }

    /// 删除命名空间
    pub fn delete_namespace(&mut self, ns_id: u64) -> Result<(), i32> {
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

            self.namespace_count.fetch_sub(1, Ordering::SeqCst);
            crate::println!("[namespaces] Deleted namespace ID: {}", ns_id);
            Ok(())
        } else {
            Err(ENOENT)
        }
    }

    /// 获取所有命名空间ID
    pub fn get_namespace_ids(&self) -> Vec<u64> {
        self.namespaces.keys().copied().collect()
    }

    /// 获取命名空间数量
    pub fn get_namespace_count(&self) -> usize {
        self.namespace_count.load(Ordering::SeqCst)
    }

    /// 列出所有命名空间
    pub fn list_namespaces(&self) -> Vec<(u64, NamespaceType, bool)> {
        self.namespaces.values()
            .map(|namespace| {
                let ns = namespace.lock();
                (ns.ns_id, ns.ns_type, ns.active)
            })
            .collect()
    }

    /// 清理所有命名空间
    pub fn cleanup_all_namespaces(&mut self) -> Result<(), i32> {
        let namespace_ids: Vec<u64> = self.namespaces.keys().copied().collect();
        for ns_id in namespace_ids {
            if let Err(e) = self.delete_namespace(ns_id) {
                crate::println!("[namespaces] Warning: Failed to delete namespace {}: {}", ns_id, e);
            }
        }

        crate::println!("[namespaces] Cleaned up all namespaces");
        Ok(())
    }
}

/// 全局命名空间管理器实例
static mut NAMESPACE_MANAGER: Option<NamespaceManager> = None;
static mut NAMESPACE_MANAGER_INITIALIZED: bool = false;

/// 初始化命名空间
pub fn initialize_namespaces() -> Result<(), i32> {
    if unsafe { NAMESPACE_MANAGER_INITIALIZED } {
        return Ok(());
    }

    let manager = NamespaceManager::new();

    unsafe {
        NAMESPACE_MANAGER = Some(manager);
        NAMESPACE_MANAGER_INITIALIZED = true;
    }

    crate::println!("[namespaces] Namespace manager initialized");
    Ok(())
}

/// 获取命名空间管理器引用
pub fn get_namespace_manager() -> Option<&'static NamespaceManager> {
    unsafe {
        NAMESPACE_MANAGER.as_ref()
    }
}

/// 获取命名空间管理器可变引用
pub fn get_namespace_manager_mut() -> Option<&'static mut NamespaceManager> {
    unsafe {
        NAMESPACE_MANAGER.as_mut()
    }
}

/// 创建命名空间（便捷函数）
pub fn create_namespace(ns_type: crate::cloud_native::oci::OciLinuxNamespaceType, path: Option<String>) -> Result<(), i32> {
    let manager = get_namespace_manager_mut().ok_or(EIO)?;

    let config = NamespaceConfig {
        ns_type: ns_type.into(),
        new_namespace: path.is_none(),
        existing_path: path,
        parameters: NamespaceParameters {
            mount_params: None,
            network_params: None,
            user_params: None,
            uts_params: None,
        },
    };

    let _ns_id = manager.create_namespace(config.ns_type, config)?;
    Ok(())
}

/// 挂载根文件系统
pub fn mount_rootfs(rootfs_path: &str, read_only: bool) -> Result<(), i32> {
    let manager = get_namespace_manager_mut().ok_or(EIO)?;

    let config = NamespaceConfig {
        ns_type: NamespaceType::Mount,
        new_namespace: true,
        existing_path: None,
        parameters: NamespaceParameters {
            mount_params: Some(MountParameters {
                rootfs_path: Some(rootfs_path.to_string()),
                mount_points: Vec::new(),
                propagation: MountPropagation::Private,
                read_only,
            }),
            network_params: None,
            user_params: None,
            uts_params: None,
        },
    };

    let _ns_id = manager.create_namespace(NamespaceType::Mount, config)?;
    Ok(())
}

/// 挂载设备
pub fn mount_device(source: &str, target: &str, fs_type: &str, options: &[String]) -> Result<(), i32> {
    let manager = get_namespace_manager_mut().ok_or(EIO)?;

    let mount_point = MountPoint {
        source: source.to_string(),
        target: target.to_string(),
        fs_type: fs_type.to_string(),
        options: options.to_vec(),
        flags: 0, // 简化实现
    };

    let config = NamespaceConfig {
        ns_type: NamespaceType::Mount,
        new_namespace: false, // 使用现有的挂载命名空间
        existing_path: None,
        parameters: NamespaceParameters {
            mount_params: Some(MountParameters {
                rootfs_path: None,
                mount_points: vec![mount_point],
                propagation: MountPropagation::Private,
                read_only: false,
            }),
            network_params: None,
            user_params: None,
            uts_params: None,
        },
    };

    let _ns_id = manager.create_namespace(NamespaceType::Mount, config)?;
    Ok(())
}

/// 清理挂载点
pub fn cleanup_mounts(container_name: &str) -> Result<(), i32> {
    crate::println!("[namespaces] Cleaning up mounts for container: {}", container_name);
    // 在实际实现中，这里会卸载容器的所有挂载点
    Ok(())
}

/// 清理命名空间
pub fn cleanup_namespaces(container_name: &str) -> Result<(), i32> {
    let manager = get_namespace_manager_mut().ok_or(EIO)?;

    // 查找与容器相关的命名空间
    let all_namespaces = manager.list_namespaces();
    for (ns_id, _ns_type, _active) in all_namespaces {
        // 简化实现：删除所有命名空间
        let _ = manager.delete_namespace(ns_id);
    }

    crate::println!("[namespaces] Cleaned up namespaces for container: {}", container_name);
    Ok(())
}

/// 获取当前进程ID
fn get_current_pid() -> u32 {
    crate::process::getpid() as u32
}
