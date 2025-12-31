// Container Runtime and Orchestration Module
//
// 容器运行时和编排模块
// 提供OCI兼容的容器运行时、镜像管理、网络、存储、编排和安全管理
//
// This module implements comprehensive container orchestration including:
// - OCI-compliant container runtime (existing modules: oci, runtime, namespace, cgroup, network, rootfs)
// - Image management with layer support (image_management)
// - Advanced networking (advanced_network)
// - Storage drivers (storage)
// - Pod orchestration (orchestration)
// - Security policies (security)

extern crate alloc;

pub mod oci;
pub mod runtime;
pub mod namespace;
pub mod cgroup;
pub mod network;
pub mod rootfs;

// New advanced modules
pub mod image_management;
pub mod advanced_network;
pub mod storage;
pub mod orchestration;
pub mod security;

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

use crate::reliability::{EINVAL, EIO};

// 重新导出主要类型
pub use self::{
    cgroup::{Cgroup, CgroupManager, CgroupResources, CgroupVersion},
    namespace::{Namespace, NamespaceConfig, NamespaceManager, NamespaceType},
    network::{Network, NetworkConfig, NetworkManager, NetworkMode},
    oci::{
        OciCgroup, OciHooks, OciLinux, OciLinuxCpu, OciLinuxMemory, OciLinuxNamespace,
        OciLinuxNamespaceType, OciLinuxResources, OciMount, OciProcess, OciRoot, OciSpec,
        OciSpecParser, OciUser, OCI_VERSION,
    },
    rootfs::{Rootfs, RootfsConfig, RootfsInfo, RootfsManager},
    runtime::{
        Container, ContainerCreateOptions, ContainerRuntime, ContainerRuntimeError,
        ContainerRuntimeState, OciContainerState,
    },
    // New advanced modules
    advanced_network::{BridgePlugin, CniConfig, CniPlugin, VxlanNetwork},
    image_management::{ContainerImage, ImageManager, ImageReference},
    orchestration::{Pod, PodManager, Service},
    security::{AppArmorProfile, ImageScanner, SecurityContext, SecurityManager, SeccompProfile},
    storage::{OverlayfsDriver, StorageDriver, Volume, VolumeManager},
};

/// 容器配置
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    /// 容器名称
    pub name: String,
    /// 容器镜像
    pub image: Option<String>,
    /// OCI规范
    pub spec: Option<OciSpec>,
    /// Rootfs路径
    pub rootfs: Option<String>,
    /// 命名空间配置
    pub namespaces: Vec<NamespaceConfig>,
    /// Cgroup资源配置
    pub cgroup_resources: Option<CgroupResources>,
    /// 网络配置
    pub network: Option<NetworkConfig>,
    /// Rootfs配置
    pub rootfs_config: Option<RootfsConfig>,
    /// 是否立即启动
    pub start: bool,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            image: None,
            spec: None,
            rootfs: None,
            namespaces: Vec::new(),
            cgroup_resources: None,
            network: None,
            rootfs_config: None,
            start: false,
        }
    }
}

/// 容器统计信息
#[derive(Debug, Clone)]
pub struct ContainerStats {
    /// 容器ID
    pub container_id: u64,
    /// 容器名称
    pub name: String,
    /// 容器状态
    pub state: OciContainerState,
    /// 进程ID
    pub pid: Option<u32>,
    /// CPU使用率
    pub cpu_usage: f64,
    /// 内存使用量
    pub memory_usage: u64,
    /// 网络I/O统计
    pub network_io: NetworkIoStats,
    /// 磁盘I/O统计
    pub disk_io: DiskIoStats,
}

/// 网络I/O统计
#[derive(Debug, Clone)]
pub struct NetworkIoStats {
    /// 接收字节数
    pub rx_bytes: u64,
    /// 发送字节数
    pub tx_bytes: u64,
    /// 接收包数
    pub rx_packets: u64,
    /// 发送包数
    pub tx_packets: u64,
}

/// 磁盘I/O统计
#[derive(Debug, Clone)]
pub struct DiskIoStats {
    /// 读取字节数
    pub read_bytes: u64,
    /// 写入字节数
    pub write_bytes: u64,
    /// 读取操作数
    pub read_ops: u64,
    /// 写入操作数
    pub write_ops: u64,
}

/// 容器运行时系统
pub struct ContainerRuntimeSystem {
    /// 运行时
    pub runtime: ContainerRuntime,
    /// 命名空间管理器
    pub namespace_manager: NamespaceManager,
    /// Cgroup管理器
    pub cgroup_manager_v1: CgroupManager,
    pub cgroup_manager_v2: CgroupManager,
    /// 网络管理器
    pub network_manager: NetworkManager,
    /// Rootfs管理器
    pub rootfs_manager: RootfsManager,
}

impl ContainerRuntimeSystem {
    /// 创建新的容器运行时系统
    pub fn new() -> Result<Self, i32> {
        Ok(Self {
            runtime: ContainerRuntime::new(),
            namespace_manager: NamespaceManager::new(),
            cgroup_manager_v1: CgroupManager::new(CgroupVersion::V1),
            cgroup_manager_v2: CgroupManager::new(CgroupVersion::V2),
            network_manager: NetworkManager::new(),
            rootfs_manager: RootfsManager::new(),
        })
    }

    /// 初始化容器运行时系统
    pub fn initialize(&mut self) -> Result<(), i32> {
        crate::println!("[container-system] Initializing container runtime system");

        // 初始化子管理器
        runtime::init_container_runtime()
            .map_err(|_| EIO)?;
        namespace::init_namespace_manager()
            .map_err(|_| EIO)?;
        cgroup::init_cgroup_manager()
            .map_err(|_| EIO)?;
        network::init_network_manager()
            .map_err(|_| EIO)?;

        crate::println!("[container-system] Container runtime system initialized");

        Ok(())
    }

    /// 创建并启动容器
    pub fn create_and_start(&mut self, config: ContainerConfig) -> Result<u64, i32> {
        crate::println!("[container-system] Creating container: {}", config.name);

        // 1. 准备rootfs
        if let Some(rootfs_config) = &config.rootfs_config {
            let _rootfs = self
                .rootfs_manager
                .prepare_rootfs(rootfs_config.clone())
                .map_err(|e| {
                    crate::println!("[container-system] Failed to prepare rootfs: {}", e);
                    e
                })?;
        }

        // 2. 创建命名空间
        let mut namespace_ids = Vec::new();
        for ns_config in &config.namespaces {
            let ns_id = self
                .namespace_manager
                .create(ns_config.clone())
                .map_err(|e| {
                    crate::println!("[container-system] Failed to create namespace: {}", e);
                    e
                })?;
            namespace_ids.push(ns_id);
        }

        // 3. 创建cgroup
        if let Some(resources) = &config.cgroup_resources {
            let cgroup_name = format!("container-{}", config.name);
            let _cgroup = self
                .cgroup_manager_v1
                .create(&cgroup_name, resources.clone())
                .map_err(|e| {
                    crate::println!("[container-system] Failed to create cgroup: {}", e);
                    e
                })?;
        }

        // 4. 创建网络
        if let Some(network_config) = &config.network {
            let _network_id = self
                .network_manager
                .create(network_config.clone())
                .map_err(|e| {
                    crate::println!("[container-system] Failed to create network: {}", e);
                    e
                })?;
        }

        // 5. 创建容器
        let create_options = ContainerCreateOptions {
            id: Some(format!("container-{}", config.name)),
            name: Some(config.name.clone()),
            spec: config.spec.clone(),
            rootfs: config.rootfs.clone(),
            start: false,
        };

        let container_id = self
            .runtime
            .create(create_options)
            .map_err(|_| EIO)?;

        // 6. 启动容器（如果需要）
        if config.start {
            self.runtime.start(container_id).map_err(|e| {
                crate::println!("[container-system] Failed to start container: {}", e);
                EIO
            })?;
        }

        crate::println!("[container-system] Container {} created with ID: {}", config.name, container_id);

        Ok(container_id)
    }

    /// 停止并删除容器
    pub fn stop_and_remove(&mut self, container_id: u64) -> Result<(), i32> {
        crate::println!("[container-system] Stopping container {}", container_id);

        // 停止容器
        self.runtime.stop(container_id, 10)?;

        // 删除容器
        self.runtime.delete(container_id)?;

        crate::println!("[container-system] Container {} removed", container_id);

        Ok(())
    }

    /// 获取容器统计信息
    pub fn get_container_stats(&self, container_id: u64) -> Result<ContainerStats, i32> {
        let state = self.runtime.get_state(container_id).map_err(|_| EIO)?;

        let stats = ContainerStats {
            container_id: state.id,
            name: state.name,
            state: state.state,
            pid: state.pid,
            cpu_usage: 0.0,
            memory_usage: 0,
            network_io: NetworkIoStats {
                rx_bytes: 0,
                tx_bytes: 0,
                rx_packets: 0,
                tx_packets: 0,
            },
            disk_io: DiskIoStats {
                read_bytes: 0,
                write_bytes: 0,
                read_ops: 0,
                write_ops: 0,
            },
        };

        Ok(stats)
    }

    /// 列出所有容器
    pub fn list_containers(&self) -> Vec<ContainerStats> {
        let states = self.runtime.list();

        states
            .into_iter()
            .map(|state| ContainerStats {
                container_id: state.id,
                name: state.name,
                state: state.state,
                pid: state.pid,
                cpu_usage: 0.0,
                memory_usage: 0,
                network_io: NetworkIoStats {
                    rx_bytes: 0,
                    tx_bytes: 0,
                    rx_packets: 0,
                    tx_packets: 0,
                },
                disk_io: DiskIoStats {
                    read_bytes: 0,
                    write_bytes: 0,
                    read_ops: 0,
                    write_ops: 0,
                },
            })
            .collect()
    }

    /// 更新所有容器状态
    pub fn update_all(&self) {
        self.runtime.update_all();
        self.network_manager.update_all_stats();
    }

    /// 清理所有资源
    pub fn cleanup_all(&mut self) -> Result<(), i32> {
        crate::println!("[container-system] Cleaning up all container resources");

        // 停止并删除所有容器
        let container_ids: Vec<u64> = self.runtime.list().iter().map(|s| s.id).collect();
        for id in container_ids {
            let _ = self.stop_and_remove(id);
        }

        // 清理命名空间
        self.namespace_manager.cleanup_all()?;

        // 清理cgroups
        self.cgroup_manager_v1.cleanup_all()?;
        self.cgroup_manager_v2.cleanup_all()?;

        // 清理网络
        self.network_manager.cleanup_all()?;

        crate::println!("[container-system] All container resources cleaned up");

        Ok(())
    }

    /// 获取系统信息
    pub fn get_system_info(&self) -> ContainerSystemInfo {
        ContainerSystemInfo {
            container_count: self.runtime.list().len(),
            namespace_count: self.namespace_manager.count(),
            cgroup_count_v1: self.cgroup_manager_v1.cgroups.len(),
            cgroup_count_v2: self.cgroup_manager_v2.cgroups.len(),
            network_count: self.network_manager.list().len(),
        }
    }
}

impl Default for ContainerRuntimeSystem {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| {
            // 如果创建失败，返回一个基本的实例
            Self {
                runtime: ContainerRuntime::new(),
                namespace_manager: NamespaceManager::new(),
                cgroup_manager_v1: CgroupManager::new(CgroupVersion::V1),
                cgroup_manager_v2: CgroupManager::new(CgroupVersion::V2),
                network_manager: NetworkManager::new(),
                rootfs_manager: RootfsManager::new(),
            }
        })
    }
}

/// 容器系统信息
#[derive(Debug, Clone)]
pub struct ContainerSystemInfo {
    /// 容器数量
    pub container_count: usize,
    /// 命名空间数量
    pub namespace_count: usize,
    /// Cgroup v1数量
    pub cgroup_count_v1: usize,
    /// Cgroup v2数量
    pub cgroup_count_v2: usize,
    /// 网络数量
    pub network_count: usize,
}

/// 全局容器运行时系统实例
static mut CONTAINER_SYSTEM: Option<ContainerRuntimeSystem> = None;
static mut CONTAINER_SYSTEM_INITIALIZED: bool = false;

/// 初始化容器运行时系统
pub fn init_container_system() -> Result<(), i32> {
    if unsafe { CONTAINER_SYSTEM_INITIALIZED } {
        return Ok(());
    }

    let mut system = ContainerRuntimeSystem::new()?;
    system.initialize()?;

    unsafe {
        CONTAINER_SYSTEM = Some(system);
        CONTAINER_SYSTEM_INITIALIZED = true;
    }

    crate::println!("[container-system] Container runtime system initialized");
    Ok(())
}

/// 获取容器运行时系统引用
pub fn get_container_system() -> Option<&'static ContainerRuntimeSystem> {
    unsafe { CONTAINER_SYSTEM.as_ref() }
}

/// 获取容器运行时系统可变引用
pub fn get_container_system_mut() -> Option<&'static mut ContainerRuntimeSystem> {
    unsafe { CONTAINER_SYSTEM.as_mut() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_config_default() {
        let config = ContainerConfig::default();
        assert_eq!(config.name, "");
        assert!(config.namespaces.is_empty());
    }

    #[test]
    fn test_container_system_creation() {
        let system = ContainerRuntimeSystem::new();
        assert!(system.is_ok());
    }

    #[test]
    fn test_container_system_info() {
        let system = ContainerRuntimeSystem::new().unwrap();
        let info = system.get_system_info();
        assert_eq!(info.container_count, 0);
        assert_eq!(info.namespace_count, 0);
    }

    #[test]
    fn test_oci_version() {
        assert_eq!(OCI_VERSION, "1.0.0");
    }
}
