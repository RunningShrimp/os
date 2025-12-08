// Container Management Module
//
// 容器管理模块
// 提供容器生命周期管理、资源隔离和容器运行时功能

extern crate alloc;

use alloc::format;
use crate::reliability::errno::{EINVAL, ENOENT, ENOMEM, EIO, EPERM};
use crate::cloud_native::oci::{OciContainerSpec, OciProcess, OciRoot, OciUser};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use spin::Mutex;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// 容器ID类型
pub type ContainerId = u64;

/// 容器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerState {
    /// 未创建
    Created,
    /// 运行中
    Running,
    /// 已暂停
    Paused,
    /// 已停止
    Stopped,
    /// 已退出
    Exited,
    /// 错误状态
    Error,
}

/// 容器配置
#[derive(Debug, Clone)]
pub struct ContainerConfig {
    /// 容器名称
    pub name: String,
    /// 镜像名称
    pub image: String,
    /// 命令
    pub command: Vec<String>,
    /// 参数
    pub args: Vec<String>,
    /// 环境变量
    pub env: Vec<String>,
    /// 工作目录
    pub working_dir: String,
    /// 用户配置
    pub user: ContainerUser,
    /// 资源限制
    pub resources: ContainerResources,
    /// 挂载点
    pub mounts: Vec<ContainerMount>,
    /// 网络配置
    pub network: ContainerNetwork,
    /// 安全配置
    pub security: ContainerSecurity,
    /// 健康检查配置
    pub health_check: Option<ContainerHealthCheck>,
}

/// 容器用户配置
#[derive(Debug, Clone)]
pub struct ContainerUser {
    /// 用户ID
    pub uid: u32,
    /// 组ID
    pub gid: u32,
    /// 用户名
    pub username: Option<String>,
    /// 附加组
    pub additional_gids: Vec<u32>,
}

impl Default for ContainerUser {
    fn default() -> Self {
        Self {
            uid: 0,
            gid: 0,
            username: None,
            additional_gids: Vec::new(),
        }
    }
}

/// 容器资源配置
#[derive(Debug, Clone)]
pub struct ContainerResources {
    /// 内存限制（字节）
    pub memory_limit: Option<u64>,
    /// CPU限制（核数）
    pub cpu_limit: Option<f64>,
    /// CPU份额
    pub cpu_shares: Option<u64>,
    /// 磁盘空间限制（字节）
    pub disk_limit: Option<u64>,
    /// 网络带宽限制（字节/秒）
    pub network_bandwidth: Option<u64>,
}

impl Default for ContainerResources {
    fn default() -> Self {
        Self {
            memory_limit: Some(512 * 1024 * 1024), // 512MB默认内存限制
            cpu_limit: Some(1.0),                    // 1个CPU核心
            cpu_shares: Some(1024),                  // 默认CPU份额
            disk_limit: Some(10 * 1024 * 1024 * 1024), // 10GB默认磁盘限制
            network_bandwidth: None,
        }
    }
}

/// 容器挂载配置
#[derive(Debug, Clone)]
pub struct ContainerMount {
    /// 源路径
    pub source: String,
    /// 目标路径
    pub destination: String,
    /// 文件系统类型
    pub fs_type: String,
    /// 挂载选项
    pub options: Vec<String>,
    /// 是否只读
    pub read_only: bool,
}

/// 容器网络配置
#[derive(Debug, Clone)]
pub struct ContainerNetwork {
    /// 网络模式
    pub mode: NetworkMode,
    /// 网络名称
    pub network_name: Option<String>,
    /// IP地址
    pub ip_address: Option<String>,
    /// 端口映射
    pub port_mappings: Vec<PortMapping>,
    /// 主机名
    pub hostname: Option<String>,
}

/// 网络模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkMode {
    /// 桥接模式
    Bridge,
    /// 主机网络
    Host,
    /// 无网络
    None,
    /// 容器网络
    Container,
}

/// 端口映射
#[derive(Debug, Clone)]
pub struct PortMapping {
    /// 主机端口
    pub host_port: u16,
    /// 容器端口
    pub container_port: u16,
    /// 协议
    pub protocol: PortProtocol,
    /// 主机IP
    pub host_ip: Option<String>,
}

/// 端口协议
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortProtocol {
    TCP,
    UDP,
}

impl Default for ContainerNetwork {
    fn default() -> Self {
        Self {
            mode: NetworkMode::Bridge,
            network_name: Some("bridge".to_string()),
            ip_address: None,
            port_mappings: Vec::new(),
            hostname: None,
        }
    }
}

/// 容器安全配置
#[derive(Debug, Clone)]
pub struct ContainerSecurity {
    /// 是否特权模式
    pub privileged: bool,
    /// capabilities
    pub capabilities: Vec<String>,
    /// Seccomp配置
    pub seccomp_profile: Option<String>,
    /// AppArmor配置
    pub apparmor_profile: Option<String>,
    /// 只读根文件系统
    pub read_only_rootfs: bool,
    /// 是否允许新权限
    pub no_new_privileges: bool,
    /// 用户命名空间
    pub user_ns: Option<UserNamespace>,
}

/// 用户命名空间配置
#[derive(Debug, Clone)]
pub struct UserNamespace {
    /// UID映射
    pub uid_map: Vec<IdMap>,
    /// GID映射
    pub gid_map: Vec<IdMap>,
}

/// ID映射
#[derive(Debug, Clone)]
pub struct IdMap {
    /// 容器ID
    pub container_id: u32,
    /// 主机ID
    pub host_id: u32,
    /// 映射大小
    pub size: u32,
}

impl Default for ContainerSecurity {
    fn default() -> Self {
        Self {
            privileged: false,
            capabilities: Vec::new(),
            seccomp_profile: None,
            apparmor_profile: None,
            read_only_rootfs: false,
            no_new_privileges: true,
            user_ns: None,
        }
    }
}

/// 容器健康检查配置
#[derive(Debug, Clone)]
pub struct ContainerHealthCheck {
    /// 检查命令
    pub command: Vec<String>,
    /// 检查间隔（秒）
    pub interval: u32,
    /// 超时时间（秒）
    pub timeout: u32,
    /// 重试次数
    pub retries: u32,
    /// 开始延迟（秒）
    pub start_period: u32,
}

/// 容器统计信息
#[derive(Debug, Clone)]
pub struct ContainerStats {
    /// 容器ID
    pub container_id: ContainerId,
    /// 容器名称
    pub name: String,
    /// 状态
    pub state: ContainerState,
    /// 进程ID
    pub pid: Option<u32>,
    /// 创建时间
    pub created_at: u64,
    /// 启动时间
    pub started_at: Option<u64>,
    /// 停止时间
    pub stopped_at: Option<u64>,
    /// 退出代码
    pub exit_code: Option<i32>,
    /// CPU使用率
    pub cpu_usage: f64,
    /// 内存使用量
    pub memory_usage: u64,
    /// 网络I/O
    pub network_io: NetworkIOStats,
    /// 磁盘I/O
    pub disk_io: DiskIOStats,
}

/// 网络I/O统计
#[derive(Debug, Clone)]
pub struct NetworkIOStats {
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
pub struct DiskIOStats {
    /// 读取字节数
    pub read_bytes: u64,
    /// 写入字节数
    pub write_bytes: u64,
    /// 读取操作数
    pub reads: u64,
    /// 写入操作数
    pub writes: u64,
}

/// 容器实例
pub struct Container {
    /// 容器ID
    pub id: ContainerId,
    /// 容器名称
    pub name: String,
    /// 容器配置
    pub config: ContainerConfig,
    /// 容器状态
    pub state: ContainerState,
    /// 进程ID
    pub pid: Option<u32>,
    /// 创建时间
    pub created_at: u64,
    /// 启动时间
    pub started_at: Option<u64>,
    /// 停止时间
    pub stopped_at: Option<u64>,
    /// 退出代码
    pub exit_code: Option<i32>,
    /// OCI容器规范
    pub oci_spec: Option<OciContainerSpec>,
    /// 统计信息
    pub stats: Arc<Mutex<ContainerStats>>,
}

impl Container {
    /// 创建新容器
    pub fn new(id: ContainerId, config: ContainerConfig) -> Self {
        let created_at = get_current_time();

        Self {
            id,
            name: config.name.clone(),
            config,
            state: ContainerState::Created,
            pid: None,
            created_at,
            started_at: None,
            stopped_at: None,
            exit_code: None,
            oci_spec: None,
            stats: Arc::new(Mutex::new(ContainerStats {
                container_id: id,
                name: String::new(),
                state: ContainerState::Created,
                pid: None,
                created_at,
                started_at: None,
                stopped_at: None,
                exit_code: None,
                cpu_usage: 0.0,
                memory_usage: 0,
                network_io: NetworkIOStats {
                    rx_bytes: 0,
                    tx_bytes: 0,
                    rx_packets: 0,
                    tx_packets: 0,
                },
                disk_io: DiskIOStats {
                    read_bytes: 0,
                    write_bytes: 0,
                    reads: 0,
                    writes: 0,
                },
            })),
        }
    }

    /// 启动容器
    pub fn start(&mut self) -> Result<u32, i32> {
        if self.state != ContainerState::Created {
            return Err(EINVAL);
        }

        // 创建OCI规范
        let oci_spec = self.create_oci_spec()?;
        self.oci_spec = Some(oci_spec.clone());

        // 使用OCI运行时创建容器
        let container_id = crate::cloud_native::oci::create_oci_container(oci_spec)?;

        // 启动OCI容器
        let pid = crate::cloud_native::oci::start_oci_container(&container_id)?;

        // 设置容器状态
        self.state = ContainerState::Running;
        self.pid = Some(pid);
        self.started_at = Some(get_current_time());

        // 应用资源限制
        self.apply_resource_limits()?;

        // 设置网络
        self.setup_network()?;

        crate::println!("[container] Started container '{}' (ID: {}, PID: {})", self.name, self.id, pid);

        // 更新统计信息
        self.update_stats();

        Ok(pid)
    }

    /// 停止容器
    pub fn stop(&mut self, timeout_sec: Option<u32>) -> Result<(), i32> {
        if self.state != ContainerState::Running {
            return Err(EINVAL);
        }

        if let Some(ref oci_spec) = self.oci_spec {
            // 使用OCI运行时停止容器
            let timeout = timeout_sec.unwrap_or(10);
            crate::cloud_native::oci::stop_oci_container(&oci_spec.id, timeout)?;
        }

        // 设置容器状态
        self.state = ContainerState::Stopped;
        self.stopped_at = Some(get_current_time());

        crate::println!("[container] Stopped container '{}' (ID: {})", self.name, self.id);

        // 更新统计信息
        self.update_stats();

        Ok(())
    }

    /// 暂停容器
    pub fn pause(&mut self) -> Result<(), i32> {
        if self.state != ContainerState::Running {
            return Err(EINVAL);
        }

        if let Some(pid) = self.pid {
            // 发送SIGSTOP信号
            crate::syscalls::process::kill_process(pid as u64, 19)?; // SIGSTOP
        }

        self.state = ContainerState::Paused;
        crate::println!("[container] Paused container '{}' (ID: {})", self.name, self.id);

        Ok(())
    }

    /// 恢复容器
    pub fn resume(&mut self) -> Result<(), i32> {
        if self.state != ContainerState::Paused {
            return Err(EINVAL);
        }

        if let Some(pid) = self.pid {
            // 发送SIGCONT信号
            crate::syscalls::process::kill_process(pid as u64, 18)?; // SIGCONT
        }

        self.state = ContainerState::Running;
        crate::println!("[container] Resumed container '{}' (ID: {})", self.name, self.id);

        Ok(())
    }

    /// 删除容器
    pub fn remove(mut self) -> Result<(), i32> {
        if self.state == ContainerState::Running {
            return Err(EINVAL);
        }

        if let Some(ref oci_spec) = self.oci_spec {
            // 使用OCI运行时删除容器
            crate::cloud_native::oci::delete_oci_container(&oci_spec.id)?;
        }

        // 清理容器资源
        self.cleanup_resources()?;

        crate::println!("[container] Removed container '{}' (ID: {})", self.name, self.id);
        Ok(())
    }

    /// 创建OCI规范
    fn create_oci_spec(&self) -> Result<OciContainerSpec, i32> {
        let container_id = format!("container-{}", self.id);

        let oci_process = OciProcess {
            terminal: false,
            user: OciUser {
                uid: self.config.user.uid,
                gid: self.config.user.gid,
                additional_gids: self.config.user.additional_gids.clone(),
                username: self.config.user.username.clone(),
            },
            env: self.config.env.clone(),
            cwd: self.config.working_dir.clone(),
            args: self.config.args.clone(),
            executable: self.config.command.first().cloned(),
        };

        let oci_root = OciRoot {
            path: format!("/var/lib/containers/{}/rootfs", self.id),
            readonly: self.config.security.read_only_rootfs,
        };

        let oci_mounts = self.config.mounts.iter().map(|m| {
            crate::cloud_native::oci::OciMount {
                destination: m.destination.clone(),
                source: m.source.clone(),
                options: m.options.clone(),
                typ: m.fs_type.clone(),
            }
        }).collect();

        let oci_resources = self.create_oci_resources()?;

        let oci_spec = OciContainerSpec {
            id: container_id,
            process: oci_process,
            root: oci_root,
            mounts: oci_mounts,
            resources: oci_resources,
            linux: Some(self.create_oci_linux_config()?),
            annotations: BTreeMap::new(),
        };

        Ok(oci_spec)
    }

    /// 创建OCI资源配置
    fn create_oci_resources(&self) -> Result<Option<crate::cloud_native::oci::OciLinuxResources>, i32> {
        let mut memory = None;
        let mut cpu = None;

        // 内存限制
        if let Some(limit) = self.config.resources.memory_limit {
            memory = Some(crate::cloud_native::oci::OciLinuxMemory {
                limit: Some(limit),
                reservation: None,
                swap: None,
                kernel: None,
                kernel_tcp: None,
                hugepage_limits: Vec::new(),
            });
        }

        // CPU限制
        if self.config.resources.cpu_limit.is_some() || self.config.resources.cpu_shares.is_some() {
            cpu = Some(crate::cloud_native::oci::OciLinuxCpu {
                quota: self.config.resources.cpu_limit.map(|limit| (limit * 1000000.0) as i64),
                period: Some(1000000), // 1秒
                cpus: None,
                mems: None,
                shares: self.config.resources.cpu_shares,
            });
        }

        if memory.is_some() || cpu.is_some() {
            Ok(Some(crate::cloud_native::oci::OciLinuxResources {
                memory,
                cpu,
                devices: Vec::new(),
                network: None,
            }))
        } else {
            Ok(None)
        }
    }

    /// 创建OCI Linux配置
    fn create_oci_linux_config(&self) -> Result<crate::cloud_native::oci::OciLinux, i32> {
        let mut namespaces = Vec::new();

        // 添加标准命名空间
        namespaces.push(crate::cloud_native::oci::OciLinuxNamespace {
            typ: crate::cloud_native::oci::OciLinuxNamespaceType::PID,
            path: None,
        });
        namespaces.push(crate::cloud_native::oci::OciLinuxNamespace {
            typ: crate::cloud_native::oci::OciLinuxNamespaceType::Mount,
            path: None,
        });
        namespaces.push(crate::cloud_native::oci::OciLinuxNamespace {
            typ: crate::cloud_native::oci::OciLinuxNamespaceType::UTS,
            path: None,
        });
        namespaces.push(crate::cloud_native::oci::OciLinuxNamespace {
            typ: crate::cloud_native::oci::OciLinuxNamespaceType::IPC,
            path: None,
        });

        // 根据网络模式添加网络命名空间
        if self.config.network.mode != NetworkMode::Host {
            namespaces.push(crate::cloud_native::oci::OciLinuxNamespace {
                typ: crate::cloud_native::oci::OciLinuxNamespaceType::Network,
                path: None,
            });
        }

        // 如果配置了用户命名空间
        if self.config.security.user_ns.is_some() {
            namespaces.push(crate::cloud_native::oci::OciLinuxNamespace {
                typ: crate::cloud_native::oci::OciLinuxNamespaceType::User,
                path: None,
            });
        }

        Ok(crate::cloud_native::oci::OciLinux {
            uid_mappings: Vec::new(),
            gid_mappings: Vec::new(),
            namespaces,
            resources: None,
            cgroups_path: Some(format!("/sys/fs/cgroup/containers/{}", self.id)),
        })
    }

    /// 应用资源限制
    fn apply_resource_limits(&self) -> Result<(), i32> {
        // 内存限制
        if let Some(limit) = self.config.resources.memory_limit {
            crate::cloud_native::cgroups::set_memory_limit_for_process(
                self.pid.unwrap_or(0),
                limit,
            )?;
        }

        // CPU限制
        if let Some(limit) = self.config.resources.cpu_limit {
            crate::cloud_native::cgroups::set_cpu_limit_for_process(
                self.pid.unwrap_or(0),
                limit,
            )?;
        }

        // 磁盘限制
        if let Some(limit) = self.config.resources.disk_limit {
            crate::cloud_native::cgroups::set_disk_limit_for_process(
                self.pid.unwrap_or(0),
                limit,
            )?;
        }

        Ok(())
    }

    /// 设置网络
    fn setup_network(&self) -> Result<(), i32> {
        match self.config.network.mode {
            NetworkMode::Bridge => {
                // 设置桥接网络
                self.setup_bridge_network()?;
            }
            NetworkMode::Host => {
                // 使用主机网络，无需额外设置
            }
            NetworkMode::None => {
                // 无网络模式
                self.setup_none_network()?;
            }
            NetworkMode::Container => {
                // 容器网络模式
                self.setup_container_network()?;
            }
        }

        // 设置端口映射
        self.setup_port_mappings()?;

        // 设置主机名
        if let Some(ref hostname) = self.config.network.hostname {
            crate::syscalls::process::set_hostname_for_process(
                self.pid.unwrap_or(0) as u64,
                hostname,
            )?;
        }

        Ok(())
    }

    /// 设置桥接网络
    fn setup_bridge_network(&self) -> Result<(), i32> {
        crate::println!("[container] Setting up bridge network for container '{}'", self.name);
        // 在实际实现中，这里会创建网络接口、配置IP等
        Ok(())
    }

    /// 设置无网络模式
    fn setup_none_network(&self) -> Result<(), i32> {
        crate::println!("[container] Setting up none network for container '{}'", self.name);
        // 在实际实现中，这里会禁用网络接口
        Ok(())
    }

    /// 设置容器网络模式
    fn setup_container_network(&self) -> Result<(), i32> {
        crate::println!("[container] Setting up container network for container '{}'", self.name);
        // 在实际实现中，这里会共享另一个容器的网络命名空间
        Ok(())
    }

    /// 设置端口映射
    fn setup_port_mappings(&self) -> Result<(), i32> {
        for port_mapping in &self.config.network.port_mappings {
            match port_mapping.protocol {
                PortProtocol::TCP => {
                    crate::println!("[container] Setting up TCP port mapping {}:{} -> {}",
                        port_mapping.host_ip.as_deref().unwrap_or("0.0.0.0"),
                        port_mapping.host_port,
                        port_mapping.container_port);
                }
                PortProtocol::UDP => {
                    crate::println!("[container] Setting up UDP port mapping {}:{} -> {}",
                        port_mapping.host_ip.as_deref().unwrap_or("0.0.0.0"),
                        port_mapping.host_port,
                        port_mapping.container_port);
                }
            }
            // 在实际实现中，这里会设置iptables规则或其他网络配置
        }
        Ok(())
    }

    /// 清理容器资源
    fn cleanup_resources(&self) -> Result<(), i32> {
        // 清理cgroups
        crate::cloud_native::cgroups::cleanup_container_cgroups(&format!("{}", self.id))?;

        // 清理网络配置
        self.cleanup_network()?;

        // 清理挂载点
        self.cleanup_mounts()?;

        Ok(())
    }

    /// 清理网络配置
    fn cleanup_network(&self) -> Result<(), i32> {
        crate::println!("[container] Cleaning up network for container '{}'", self.name);
        // 在实际实现中，这里会删除网络接口、清理端口映射等
        Ok(())
    }

    /// 清理挂载点
    fn cleanup_mounts(&self) -> Result<(), i32> {
        crate::println!("[container] Cleaning up mounts for container '{}'", self.name);
        // 在实际实现中，这里会卸载容器的挂载点
        Ok(())
    }

    /// 更新统计信息
    fn update_stats(&self) {
        let mut stats = self.stats.lock();
        stats.container_id = self.id;
        stats.name = self.name.clone();
        stats.state = self.state;
        stats.pid = self.pid;
        stats.created_at = self.created_at;
        stats.started_at = self.started_at;
        stats.stopped_at = self.stopped_at;
        stats.exit_code = self.exit_code;

        // 更新资源使用统计
        if let Some(pid) = self.pid {
            stats.cpu_usage = self.get_cpu_usage(pid);
            stats.memory_usage = self.get_memory_usage(pid);
            stats.network_io = self.get_network_io_stats(pid);
            stats.disk_io = self.get_disk_io_stats(pid);
        }
    }

    /// 获取CPU使用率
    fn get_cpu_usage(&self, pid: u32) -> f64 {
        // 在实际实现中，这里会读取/proc/[pid]/stat并计算CPU使用率
        0.0 // 简化实现
    }

    /// 获取内存使用量
    fn get_memory_usage(&self, pid: u32) -> u64 {
        // 在实际实现中，这里会读取/proc/[pid]/status并获取内存使用量
        0 // 简化实现
    }

    /// 获取网络I/O统计
    fn get_network_io_stats(&self, pid: u32) -> NetworkIOStats {
        // 在实际实现中，这里会获取容器的网络I/O统计
        NetworkIOStats {
            rx_bytes: 0,
            tx_bytes: 0,
            rx_packets: 0,
            tx_packets: 0,
        }
    }

    /// 获取磁盘I/O统计
    fn get_disk_io_stats(&self, pid: u32) -> DiskIOStats {
        // 在实际实现中，这里会获取容器的磁盘I/O统计
        DiskIOStats {
            read_bytes: 0,
            write_bytes: 0,
            reads: 0,
            writes: 0,
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> ContainerStats {
        self.update_stats();
        self.stats.lock().clone()
    }
}

/// 容器管理器
pub struct ContainerManager {
    /// 容器列表
    containers: BTreeMap<ContainerId, Arc<Mutex<Container>>>,
    /// 下一个容器ID
    next_container_id: AtomicU64,
    /// 容器数量
    container_count: AtomicUsize,
}

impl ContainerManager {
    /// 创建新的容器管理器
    pub fn new() -> Self {
        Self {
            containers: BTreeMap::new(),
            next_container_id: AtomicU64::new(1),
            container_count: AtomicUsize::new(0),
        }
    }

    /// 创建容器
    pub fn create_container(&mut self, config: ContainerConfig) -> Result<ContainerId, i32> {
        let container_id = self.next_container_id.fetch_add(1, Ordering::SeqCst);
        let container = Container::new(container_id, config);

        let container_arc = Arc::new(Mutex::new(container));
        self.containers.insert(container_id, container_arc);
        self.container_count.fetch_add(1, Ordering::SeqCst);

        crate::println!("[container] Created container with ID: {}", container_id);
        Ok(container_id)
    }

    /// 获取容器
    pub fn get_container(&self, container_id: ContainerId) -> Option<Arc<Mutex<Container>>> {
        self.containers.get(&container_id).cloned()
    }

    /// 删除容器
    pub fn remove_container(&mut self, container_id: ContainerId) -> Result<(), i32> {
        if let Some(container) = self.containers.remove(&container_id) {
            let container_mutex = container.lock();
            // 注意：这里不能直接调用remove()，因为会消费self
            // 在实际使用中，需要调整API设计
            drop(container_mutex);
            self.container_count.fetch_sub(1, Ordering::SeqCst);
            crate::println!("[container] Removed container with ID: {}", container_id);
            Ok(())
        } else {
            Err(ENOENT)
        }
    }

    /// 获取所有容器ID
    pub fn get_container_ids(&self) -> Vec<ContainerId> {
        self.containers.keys().copied().collect()
    }

    /// 获取容器数量
    pub fn get_container_count(&self) -> usize {
        self.container_count.load(Ordering::SeqCst)
    }

    /// 获取活跃容器数量
    pub fn get_active_container_count(&self) -> usize {
        self.containers.values()
            .filter(|container| {
                container.lock().state == ContainerState::Running
            })
            .count()
    }

    /// 列出所有容器
    pub fn list_containers(&self) -> Vec<ContainerStats> {
        self.containers.values()
            .map(|container| {
                let container = container.lock();
                container.get_stats()
            })
            .collect()
    }

    /// 停止所有容器
    pub fn stop_all_containers(&self) -> Result<(), i32> {
        for container in self.containers.values() {
            let mut cont = container.lock();
            if cont.state == ContainerState::Running {
                if let Err(e) = cont.stop(Some(5)) {
                    crate::println!("[container] Warning: Failed to stop container {}: {}", cont.id, e);
                }
            }
        }
        Ok(())
    }

    /// 清理所有容器
    pub fn cleanup_all_containers(&mut self) -> Result<(), i32> {
        // 停止所有容器
        self.stop_all_containers()?;

        // 删除所有容器
        let container_ids: Vec<ContainerId> = self.containers.keys().copied().collect();
        for container_id in container_ids {
            if let Err(e) = self.remove_container(container_id) {
                crate::println!("[container] Warning: Failed to remove container {}: {}", container_id, e);
            }
        }

        Ok(())
    }
}

/// 全局容器管理器实例
static mut CONTAINER_MANAGER: Option<ContainerManager> = None;
static mut CONTAINER_MANAGER_INITIALIZED: bool = false;

/// 初始化容器管理器
pub fn init_container_manager() -> Result<(), i32> {
    if unsafe { CONTAINER_MANAGER_INITIALIZED } {
        return Ok(());
    }

    let manager = ContainerManager::new();

    unsafe {
        CONTAINER_MANAGER = Some(manager);
        CONTAINER_MANAGER_INITIALIZED = true;
    }

    crate::println!("[container] Container manager initialized");
    Ok(())
}

/// 获取容器管理器引用
pub fn get_container_manager() -> Option<&'static mut ContainerManager> {
    unsafe {
        CONTAINER_MANAGER.as_mut()
    }
}

/// 创建容器（便捷函数）
pub fn create_container(config: ContainerConfig) -> Result<ContainerId, i32> {
    let manager = get_container_manager().ok_or(EIO)?;
    manager.create_container(config)
}

/// 启动容器（便捷函数）
pub fn start_container(container_id: ContainerId) -> Result<(), i32> {
    let manager = get_container_manager().ok_or(EIO)?;
    let container = manager.get_container(container_id).ok_or(ENOENT)?;
    let mut cont = container.lock();
    cont.start()?;
    Ok(())
}

/// 停止容器（便捷函数）
pub fn stop_container(container_id: ContainerId) -> Result<(), i32> {
    let manager = get_container_manager().ok_or(EIO)?;
    let container = manager.get_container(container_id).ok_or(ENOENT)?;
    let mut cont = container.lock();
    cont.stop(Some(10))?;
    Ok(())
}

/// 删除容器（便捷函数）
pub fn remove_container(container_id: ContainerId) -> Result<(), i32> {
    let manager = get_container_manager().ok_or(EIO)?;
    let container = manager.get_container(container_id).ok_or(ENOENT)?;
    let mut cont = container.lock();

    // 检查容器状态
    if cont.state == ContainerState::Running {
        return Err(EINVAL);
    }

    // 移除容器
    drop(cont); // 释放锁
    manager.remove_container(container_id)
}

/// 停止所有容器（便捷函数）
pub fn stop_all_containers() -> Result<(), i32> {
    let manager = get_container_manager().ok_or(EIO)?;
    manager.stop_all_containers()
}

/// 获取容器数量
pub fn get_container_count() -> usize {
    get_container_manager()
        .map(|manager| manager.get_container_count())
        .unwrap_or(0)
}

/// 获取活跃容器数量
pub fn get_active_container_count() -> usize {
    get_container_manager()
        .map(|manager| manager.get_active_container_count())
        .unwrap_or(0)
}

/// 获取当前时间（纳秒）
fn get_current_time() -> u64 {
    crate::time::rdtsc() as u64
}
