// OCI (Open Container Initiative) Runtime Specification Implementation
//
// OCI运行时规范实现模块
// 提供符合OCI 1.0标准的容器运行时实现，包括config.json解析、manifest支持和镜像格式

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic {AtomicU64,, Ordering};

use serde::{Deserialize, Serialize};

use crate::reliability::{EINVAL, EIO, ENOENT, ENOMEM};

/// OCI规范版本
pub const OCI_VERSION: &str = "1.0.0";

/// OCI配置文件名称
pub const OCI_CONFIG_FILENAME: &str = "config.json";

/// OCI根文件系统目录
pub const OCI_ROOTFS_DIR: &str = "rootfs";

/// OCI运行时状态文件
pub const OCI_RUNTIME_STATE_FILE: &str = "runtime-state.json";

/// OCI容器状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OciContainerState {
    /// 创建中
    Creating,
    /// 已创建
    Created,
    /// 运行中
    Running,
    /// 已暂停
    Paused,
    /// 已停止
    Stopped,
    /// 已退出
    Exited,
}

/// OCI规范 - 完整的容器配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciSpec {
    /// OCI规范版本
    pub ociVersion: String,
    /// 容器ID
    pub id: String,
    /// 进程配置
    pub process: OciProcess,
    /// 根文件系统配置
    pub root: OciRoot,
    /// 主机名
    pub hostname: String,
    /// 挂载点配置
    pub mounts: Vec<OciMount>,
    /// 钩子配置
    pub hooks: Option<OciHooks>,
    /// 注解
    pub annotations: Option<BTreeMap<String, String>>,
    /// Linux特定配置
    pub linux: Option<OciLinux>,
    /// Solaris特定配置
    pub solaris: Option<serde_json::Value>,
    /// Windows特定配置
    pub windows: Option<serde_json::Value>,
    /// VM特定配置
    pub vm: Option<serde_json::Value>,
}

impl Default for OciSpec {
    fn default() -> Self {
        Self {
            ociVersion: OCI_VERSION.to_string(),
            id: String::new(),
            process: OciProcess::default(),
            root: OciRoot::default(),
            hostname: "localhost".to_string(),
            mounts: Vec::new(),
            hooks: None,
            annotations: None,
            linux: None,
            solaris: None,
            windows: None,
            vm: None,
        }
    }
}

/// OCI进程配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciProcess {
    /// 是否使用终端
    pub terminal: bool,
    /// 用户配置
    pub user: OciUser,
    /// 环境变量
    pub env: Vec<String>,
    /// 命令行参数
    pub args: Vec<String>,
    /// 命令路径
    pub cwd: String,
    /// 可执行文件路径
    pub executable: Option<String>,
    /// 能力集
    pub capabilities: Option<OciCapabilities>,
    /// Rlimits资源限制
    pub rlimits: Vec<OciRlimit>,
    /// 是否设置新权限
    pub noNewPrivileges: bool,
    /// AppArmor配置文件
    pub apparmorProfile: String,
    /// Seccomp配置
    pub seccomp: Option<OciSeccomp>,
    /// SELinux上下文
    pub selinuxContext: String,
}

impl Default for OciProcess {
    fn default() -> Self {
        Self {
            terminal: false,
            user: OciUser::default(),
            env: Vec::new(),
            args: Vec::new(),
            cwd: "/".to_string(),
            executable: None,
            capabilities: None,
            rlimits: Vec::new(),
            noNewPrivileges: true,
            apparmorProfile: String::new(),
            seccomp: None,
            selinuxContext: String::new(),
        }
    }
}

/// OCI用户配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciUser {
    /// 用户ID
    pub uid: u32,
    /// 组ID
    pub gid: u32,
    /// 附加GID
    pub additionalGids: Vec<u32>,
    /// 用户名
    pub username: Option<String>,
}

impl Default for OciUser {
    fn default() -> Self {
        Self {
            uid: 0,
            gid: 0,
            additionalGids: Vec::new(),
            username: None,
        }
    }
}

/// OCI能力配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciCapabilities {
    /// 有效能力集
    pub effective: Vec<String>,
    /// 许可能力集
    pub permitted: Vec<String>,
    /// 可继承能力集
    pub inheritable: Vec<String>,
    /// 边界集
    pub bounding: Vec<String>,
    /// 环境能力集
    pub ambient: Vec<String>,
}

/// OCI资源限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciRlimit {
    /// 资源类型
    #[serde(rename = "type")]
    pub rlimit_type: String,
    /// 软限制
    pub soft: u64,
    /// 硬限制
    pub hard: u64,
}

/// OCI Seccomp配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciSeccomp {
    /// 默认动作
    pub defaultAction: String,
    /// 架构
    pub architectures: Vec<String>,
    /// 系统调用规则
    pub syscalls: Vec<OciSyscallRule>,
    /// 标志
    pub flags: Vec<String>,
}

/// OCI系统调用规则
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciSyscallRule {
    /// 系统调用名称
    pub names: Vec<String>,
    /// 动作
    pub action: String,
    /// 参数
    pub args: Option<Vec<OciSyscallArg>>,
}

/// OCI系统调用参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciSyscallArg {
    /// 索引
    pub index: u32,
    /// 值
    pub value: u64,
    /// 值2
    pub valueTwo: Option<u64>,
    /// 操作符
    pub op: String,
}

/// OCI根文件系统配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciRoot {
    /// 根文件系统路径
    pub path: String,
    /// 是否只读
    pub readonly: bool,
}

impl Default for OciRoot {
    fn default() -> Self {
        Self { path: String::new(), readonly: false }
    }
}

/// OCI挂载点配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciMount {
    /// 目标路径（容器内）
    pub destination: String,
    /// 源路径（主机）
    pub source: String,
    /// 挂载选项
    pub options: Vec<String>,
    /// 文件系统类型
    #[serde(rename = "type")]
    pub fs_type: String,
}

/// OCI钩子配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciHooks {
    /// 预启动钩子
    pub prestart: Vec<OciHook>,
    /// 启动后钩子
    pub poststart: Vec<OciHook>,
    /// 停止后钩子
    pub poststop: Vec<OciHook>,
}

/// OCI钩子
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciHook {
    /// 钩子路径
    pub path: String,
    /// 钩子参数
    pub args: Vec<String>,
    /// 钩子环境变量
    pub env: Vec<String>,
    /// 超时时间（秒）
    pub timeout: Option<u32>,
}

/// OCI Linux特定配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciLinux {
    /// UID映射
    pub uidMappings: Vec<OciIDMapping>,
    /// GID映射
    pub gidMappings: Vec<OciIDMapping>,
    /// 命名空间配置
    pub namespaces: Vec<OciLinuxNamespace>,
    /// 资源限制
    pub resources: Option<OciLinuxResources>,
    /// Cgroup路径
    pub cgroupsPath: Option<String>,
    /// 安全配置
    pub seccomp: Option<OciSeccomp>,
    /// 根文件系统传播
    pub rootfsPropagation: String,
    /// 屏蔽路径
    pub maskedPaths: Vec<String>,
    /// 只读路径
    pub readonlyPaths: Vec<String>,
    /// 挂载标签
    pub mountLabel: String,
    /// 智能PID检测
    pub intelRdt: Option<OciIntelRdt>,
}

/// OCI ID映射
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciIDMapping {
    /// 容器ID
    pub containerID: u32,
    /// 主机ID
    pub hostID: u32,
    /// 映射大小
    pub size: u32,
}

/// OCI Linux命名空间类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OciLinuxNamespaceType {
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

impl OciLinuxNamespaceType {
    /// 转换为克隆标志
    pub fn to_clone_flag(&self) -> u64 {
        match self {
            OciLinuxNamespaceType::Mount => 0x00020000,  // CLONE_NEWNS
            OciLinuxNamespaceType::UTS => 0x04000000,    // CLONE_NEWUTS
            OciLinuxNamespaceType::IPC => 0x08000000,    // CLONE_NEWIPC
            OciLinuxNamespaceType::Network => 0x40000000, // CLONE_NEWNET
            OciLinuxNamespaceType::PID => 0x20000000,    // CLONE_NEWPID
            OciLinuxNamespaceType::User => 0x10000000,   // CLONE_NEWUSER
            OciLinuxNamespaceType::Cgroup => 0x02000000, // CLONE_NEWCGROUP
        }
    }
}

/// OCI Linux命名空间
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciLinuxNamespace {
    /// 命名空间类型
    #[serde(rename = "type")]
    pub ns_type: OciLinuxNamespaceType,
    /// 命名空间路径
    pub path: Option<String>,
}

/// OCI Linux资源限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciLinuxResources {
    /// 内存限制
    pub memory: Option<OciLinuxMemory>,
    /// CPU限制
    pub cpu: Option<OciLinuxCpu>,
    /// 设备限制
    pub devices: Vec<OciLinuxDeviceCgroup>,
    /// 网络限制
    pub network: Option<OciLinuxNetwork>,
    /// 块I/O限制
    pub blockIO: Option<OciLinuxBlockIO>,
    /// 巨页限制
    pub hugepageLimits: Vec<OciLinuxHugepageLimit>,
    /// 进程数限制
    pub pids: Option<OciLinuxPids>,
}

/// OCI Linux内存限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciLinuxMemory {
    /// 内存限制（字节）
    pub limit: Option<u64>,
    /// 保留内存（字节）
    pub reservation: Option<u64>,
    /// 交换空间限制（字节）
    pub swap: Option<u64>,
    /// 内核内存限制（字节）
    pub kernel: Option<u64>,
    /// 内核TCP内存限制（字节）
    pub kernelTCP: Option<u64>,
    /// 内存软限制（字节）
    pub swappiness: Option<u64>,
    /// OOM控制
    pub disableOOMKiller: Option<bool>,
}

/// OCI Linux CPU限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciLinuxCpu {
    /// CPU配额（微秒）
    pub quota: Option<i64>,
    /// CPU周期（微秒）
    pub period: Option<u64>,
    /// CPU实时运行时间（微秒）
    pub realtimeRuntime: Option<u64>,
    /// CPU实时周期（微秒）
    pub realtimePeriod: Option<u64>,
    /// CPU份额
    pub shares: Option<u64>,
    /// CPU亲和性
    pub cpus: Option<String>,
    /// 内存节点亲和性
    pub mems: Option<String>,
}

/// OCI Linux设备控制组
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciLinuxDeviceCgroup {
    /// 是否允许
    pub allow: bool,
    /// 设备类型
    #[serde(rename = "type")]
    pub device_type: Option<String>,
    /// 主设备号
    pub major: Option<i64>,
    /// 次设备号
    pub minor: Option<i64>,
    /// 访问权限
    pub access: Option<String>,
    /// 是否允许写
    pub allow_write: Option<bool>,
}

/// OCI Linux网络限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciLinuxNetwork {
    /// 类ID
    pub classID: Option<u32>,
    /// 优先级
    pub priorities: Vec<OciLinuxNetworkPriority>,
}

/// OCI Linux网络优先级
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciLinuxNetworkPriority {
    /// 接口名称
    pub name: String,
    /// 优先级
    pub priority: u32,
}

/// OCI Linux块I/O限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciLinuxBlockIO {
    /// 块I/O权重
    pub weight: Option<u16>,
    /// 叶块I/O权重
    pub leafWeight: Option<u16>,
    /// 块I/O权重设备
    pub weightDevice: Vec<OciLinuxWeightDevice>,
    /// 块I/O节流
    pub throttle: Vec<OciLinuxThrottle>,
}

/// OCI Linux权重设备
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciLinuxWeightDevice {
    /// 主设备号
    pub major: u64,
    /// 次设备号
    pub minor: u64,
    /// 权重
    pub weight: Option<u16>,
    /// 叶权重
    pub leafWeight: Option<u16>,
}

/// OCI Linux节流配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciLinuxThrottle {
    /// 主设备号
    pub major: u64,
    /// 次设备号
    pub minor: u64,
    /// 读取速率（字节/秒）
    pub rate: Option<u64>,
}

/// OCI Linux巨页限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciLinuxHugepageLimit {
    /// 页面大小
    pub pageSize: String,
    /// 限制（字节）
    pub limit: u64,
}

/// OCI Linux进程数限制
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciLinuxPids {
    /// 最大进程数
    pub limit: i64,
}

/// OCI Intel RDT配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciIntelRdt {
    /// CLOS ID
    pub closID: String,
    /// 内存带宽禁用
    pub l3CacheSchema: String,
    /// 内存带宽禁用
    pub memBwSchema: String,
}

/// OCI manifest（镜像格式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciManifest {
    /// OCI规范版本
    pub schemaVersion: u64,
    /// 媒体类型
    pub mediaType: String,
    /// 配置描述
    pub config: OciDescriptor,
    /// 层列表
    pub layers: Vec<OciDescriptor>,
    /// 注解
    pub annotations: Option<BTreeMap<String, String>>,
}

/// OCI描述符
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciDescriptor {
    /// 媒体类型
    pub mediaType: String,
    /// 摘要
    pub digest: String,
    /// 大小
    pub size: u64,
    /// URL列表
    pub urls: Option<Vec<String>>,
    /// 平台
    pub platform: Option<OciPlatform>,
    /// 注解
    pub annotations: Option<BTreeMap<String, String>>,
}

/// OCI平台
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciPlatform {
    /// 架构
    pub architecture: String,
    /// 操作系统
    pub os: String,
    /// 变体
    #[serde(rename = "variant")]
    pub os_variant: Option<String>,
}

/// OCI配置（镜像配置）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciImageConfig {
    /// OCI规范版本
    pub schemaVersion: u64,
    /// 媒体类型
    pub mediaType: String,
    /// 配置时间
    pub created: Option<String>,
    /// 作者
    pub author: Option<String>,
    /// 架构
    pub architecture: String,
    /// 操作系统
    pub os: String,
    /// 配置
    pub config: Option<OciImageConfigConfig>,
    /// 根文件系统
    pub rootfs: OciImageRootfs,
    /// 历史
    pub history: Option<Vec<OciHistory>>,
}

/// OCI镜像配置配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciImageConfigConfig {
    /// 镜像用户
    pub User: Option<String>,
    /// 暴露端口
    pub ExposedPorts: Option<BTreeMap<String, String>>,
    /// 环境变量
    pub Env: Option<Vec<String>>,
    /// 入口点
    pub Entrypoint: Option<Vec<String>>,
    /// 命令
    pub Cmd: Option<Vec<String>>,
    /// 工作目录
    pub WorkingDir: Option<String>,
    /// 标签
    pub Labels: Option<BTreeMap<String, String>>,
}

/// OCI镜像根文件系统
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciImageRootfs {
    /// 类型
    #[serde(rename = "type")]
    pub fs_type: String,
    /// 层的摘要
    pub diff_ids: Vec<String>,
}

/// OCI历史记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OciHistory {
    /// 创建时间
    pub created: Option<String>,
    /// 创建者
    pub created_by: Option<String>,
    /// 作者
    pub author: Option<String>,
    /// 注释
    pub comment: Option<String>,
    /// 空层
    pub empty_layer: Option<bool>,
}

/// OCI规范解析器
pub struct OciSpecParser;

impl OciSpecParser {
    /// 从JSON字符串解析OCI规范
    pub fn from_json(json_str: &str) -> Result<OciSpec, i32> {
        serde_json::from_str(json_str).map_err(|_| EINVAL)
    }

    /// 将OCI规范序列化为JSON字符串
    pub fn to_json(spec: &OciSpec) -> Result<String, i32> {
        serde_json::to_string_pretty(spec).map_err(|_| EIO)
    }

    /// 从文件加载OCI规范
    pub fn from_file(path: &str) -> Result<OciSpec, i32> {
        // 在实际实现中，这里会从文件系统读取文件
        crate::println!("[oci-parser] Loading OCI spec from: {}", path);
        Err(EIO) // 简化实现
    }

    /// 将OCI规范保存到文件
    pub fn to_file(spec: &OciSpec, path: &str) -> Result<(), i32> {
        let json_str = Self::to_json(spec)?;
        // 在实际实现中，这里会写入文件系统
        crate::println!("[oci-parser] Saving OCI spec to: {}", path);
        crate::println!("[oci-parser] Spec JSON:\n{}", json_str);
        Ok(())
    }

    /// 验证OCI规范
    pub fn validate(spec: &OciSpec) -> Result<(), i32> {
        // 检查OCI版本
        if spec.ociVersion.is_empty() {
            return Err(EINVAL);
        }

        // 检查容器ID
        if spec.id.is_empty() {
            return Err(EINVAL);
        }

        // 检查进程配置
        if spec.process.args.is_empty() {
            return Err(EINVAL);
        }

        // 检查根文件系统
        if spec.root.path.is_empty() {
            return Err(EINVAL);
        }

        // 检查工作目录
        if spec.process.cwd.is_empty() {
            return Err(EINVAL);
        }

        // 验证Linux配置
        if let Some(ref linux) = spec.linux {
            Self::validate_linux_config(linux)?;
        }

        Ok(())
    }

    /// 验证Linux配置
    fn validate_linux_config(linux: &OciLinux) -> Result<(), i32> {
        // 验证命名空间
        for ns in &linux.namespaces {
            if ns.path.is_none() && ns.ns_type != OciLinuxNamespaceType::User {
                // 只有用户命名空间可以没有路径
                continue;
            }
        }

        // 验证ID映射
        for uid_map in &linux.uidMappings {
            if uid_map.size == 0 {
                return Err(EINVAL);
            }
        }

        for gid_map in &linux.gidMappings {
            if gid_map.size == 0 {
                return Err(EINVAL);
            }
        }

        Ok(())
    }

    /// 创建默认OCI规范
    pub fn create_default(id: &str) -> OciSpec {
        OciSpec {
            ociVersion: OCI_VERSION.to_string(),
            id: id.to_string(),
            process: OciProcess {
                terminal: false,
                user: OciUser {
                    uid: 0,
                    gid: 0,
                    additionalGids: Vec::new(),
                    username: Some("root".to_string()),
                },
                env: vec![
                    "PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string(),
                    "TERM=xterm".to_string(),
                ],
                args: vec!["/bin/sh".to_string()],
                cwd: "/".to_string(),
                executable: Some("/bin/sh".to_string()),
                capabilities: None,
                rlimits: Vec::new(),
                noNewPrivileges: true,
                apparmorProfile: String::new(),
                seccomp: None,
                selinuxContext: String::new(),
            },
            root: OciRoot {
                path: format!("/var/lib/containers/{}/rootfs", id),
                readonly: false,
            },
            hostname: "container".to_string(),
            mounts: Self::default_mounts(),
            hooks: None,
            annotations: None,
            linux: Some(Self::default_linux_config()),
            solaris: None,
            windows: None,
            vm: None,
        }
    }

    /// 默认挂载点
    fn default_mounts() -> Vec<OciMount> {
        vec![
            OciMount {
                destination: "/proc".to_string(),
                source: "proc".to_string(),
                options: vec![],
                fs_type: "proc".to_string(),
            },
            OciMount {
                destination: "/dev".to_string(),
                source: "tmpfs".to_string(),
                options: vec!["nosuid".to_string(), "strictatime".to_string()],
                fs_type: "tmpfs".to_string(),
            },
            OciMount {
                destination: "/dev/pts".to_string(),
                source: "devpts".to_string(),
                options: vec!["nosuid".to_string(), "noexec".to_string(), "newinstance".to_string()],
                fs_type: "devpts".to_string(),
            },
            OciMount {
                destination: "/dev/shm".to_string(),
                source: "shm".to_string(),
                options: vec!["nosuid".to_string(), "noexec".to_string(), "nodev".to_string()],
                fs_type: "tmpfs".to_string(),
            },
            OciMount {
                destination: "/dev/mqueue".to_string(),
                source: "mqueue".to_string(),
                options: vec!["nosuid".to_string(), "noexec".to_string(), "nodev".to_string()],
                fs_type: "mqueue".to_string(),
            },
            OciMount {
                destination: "/sys".to_string(),
                source: "sysfs".to_string(),
                options: vec!["nosuid".to_string(), "noexec".to_string(), "nodev".to_string()],
                fs_type: "sysfs".to_string(),
            },
        ]
    }

    /// 默认Linux配置
    fn default_linux_config() -> OciLinux {
        OciLinux {
            uidMappings: Vec::new(),
            gidMappings: Vec::new(),
            namespaces: vec![
                OciLinuxNamespace {
                    ns_type: OciLinuxNamespaceType::PID,
                    path: None,
                },
                OciLinuxNamespace {
                    ns_type: OciLinuxNamespaceType::Network,
                    path: None,
                },
                OciLinuxNamespace {
                    ns_type: OciLinuxNamespaceType::IPC,
                    path: None,
                },
                OciLinuxNamespace {
                    ns_type: OciLinuxNamespaceType::UTS,
                    path: None,
                },
                OciLinuxNamespace {
                    ns_type: OciLinuxNamespaceType::Mount,
                    path: None,
                },
            ],
            resources: None,
            cgroupsPath: Some("/nos".to_string()),
            seccomp: None,
            rootfsPropagation: "rprivate".to_string(),
            maskedPaths: vec![
                "/proc/kcore".to_string(),
                "/proc/latency_stats".to_string(),
                "/proc/timer_list".to_string(),
                "/proc/timer_stats".to_string(),
                "/proc/sched_debug".to_string(),
                "/sys/firmware".to_string(),
            ],
            readonlyPaths: vec![
                "/proc/asound".to_string(),
                "/proc/bus".to_string(),
                "/proc/fs".to_string(),
                "/proc/irq".to_string(),
                "/proc/sys".to_string(),
                "/proc/sysrq-trigger".to_string(),
            ],
            mountLabel: String::new(),
            intelRdt: None,
        }
    }
}

/// OCI manifest解析器
pub struct OciManifestParser;

impl OciManifestParser {
    /// 从JSON解析manifest
    pub fn from_json(json_str: &str) -> Result<OciManifest, i32> {
        serde_json::from_str(json_str).map_err(|_| EINVAL)
    }

    /// 将manifest序列化为JSON
    pub fn to_json(manifest: &OciManifest) -> Result<String, i32> {
        serde_json::to_string_pretty(manifest).map_err(|_| EIO)
    }

    /// 验证manifest
    pub fn validate(manifest: &OciManifest) -> Result<(), i32> {
        if manifest.schemaVersion != 2 {
            return Err(EINVAL);
        }

        if manifest.config.mediaType.is_empty() {
            return Err(EINVAL);
        }

        if manifest.config.digest.is_empty() {
            return Err(EINVAL);
        }

        Ok(())
    }
}

/// OCI镜像配置解析器
pub struct OciImageConfigParser;

impl OciImageConfigParser {
    /// 从JSON解析镜像配置
    pub fn from_json(json_str: &str) -> Result<OciImageConfig, i32> {
        serde_json::from_str(json_str).map_err(|_| EINVAL)
    }

    /// 将镜像配置序列化为JSON
    pub fn to_json(config: &OciImageConfig) -> Result<String, i32> {
        serde_json::to_string_pretty(config).map_err(|_| EIO)
    }

    /// 从镜像配置生成OCI规范
    pub fn to_oci_spec(config: &OciImageConfig, container_id: &str) -> OciSpec {
        let mut spec = OciSpecParser::create_default(container_id);

        // 应用镜像配置
        if let Some(ref image_config) = config.config {
            // 设置用户
            if let Some(ref user) = image_config.User {
                spec.process.user.username = Some(user.clone());
                // 解析uid:gid格式
                if let Some(pos) = user.find(':') {
                    let uid_str = &user[..pos];
                    let gid_str = &user[pos + 1..];
                    spec.process.user.uid = uid_str.parse().unwrap_or(0);
                    spec.process.user.gid = gid_str.parse().unwrap_or(0);
                }
            }

            // 设置环境变量
            if let Some(ref env) = image_config.Env {
                spec.process.env = env.clone();
            }

            // 设置入口点和命令
            if let Some(ref entrypoint) = image_config.Entrypoint {
                if let Some(ref cmd) = image_config.Cmd {
                    spec.process.args = {
                        let mut args = entrypoint.clone();
                        args.extend(cmd.clone());
                        args
                    };
                } else {
                    spec.process.args = entrypoint.clone();
                }
                spec.process.executable = Some(entrypoint[0].clone());
            } else if let Some(ref cmd) = image_config.Cmd {
                spec.process.args = cmd.clone();
                spec.process.executable = Some(cmd[0].clone());
            }

            // 设置工作目录
            if let Some(ref workdir) = image_config.WorkingDir {
                spec.process.cwd = workdir.clone();
            }

            // 设置标签
            if let Some(ref labels) = image_config.Labels {
                spec.annotations = Some(labels.clone());
            }
        }

        spec
    }
}

/// OCI镜像层
#[derive(Debug, Clone)]
pub struct OciLayer {
    /// 层摘要
    pub digest: String,
    /// 媒体类型
    pub media_type: String,
    /// 大小
    pub size: u64,
    /// 压缩数据
    pub data: Vec<u8>,
}

/// OCI镜像
#[derive(Debug, Clone)]
pub struct OciImage {
    /// 镜像manifest
    pub manifest: OciManifest,
    /// 镜像配置
    pub config: OciImageConfig,
    /// 层列表
    pub layers: Vec<OciLayer>,
}

impl OciImage {
    /// 创建新镜像
    pub fn new(manifest: OciManifest, config: OciImageConfig, layers: Vec<OciLayer>) -> Self {
        Self { manifest, config, layers }
    }

    /// 解包镜像到指定目录
    pub fn unpack(&self, dest_dir: &str) -> Result<(), i32> {
        crate::println!("[oci-image] Unpacking image to: {}", dest_dir);

        // 在实际实现中，这里会解压层并应用白线格式
        for (i, layer) in self.layers.iter().enumerate() {
            crate::println!(
                "[oci-image] Unpacking layer {}: {} ({} bytes)",
                i,
                layer.digest,
                layer.size
            );
        }

        Ok(())
    }

    /// 验证镜像完整性
    pub fn verify(&self) -> Result<(), i32> {
        // 验证manifest
        OciManifestParser::validate(&self.manifest)?;

        // 验证层摘要
        // 在实际实现中，这里会计算层的SHA256摘要并与manifest中的摘要比较

        Ok(())
    }
}

/// OCI镜像仓库
#[derive(Debug, Clone)]
pub struct OciImageRegistry {
    /// 仓库URL
    pub url: String,
    /// 是否使用HTTPS
    pub secure: bool,
}

impl OciImageRegistry {
    /// 创建新仓库配置
    pub fn new(url: String) -> Self {
        Self { url, secure: url.starts_with("https://") }
    }

    /// 拉取镜像
    pub fn pull(&self, image_name: &str) -> Result<OciImage, i32> {
        crate::println!("[oci-registry] Pulling image: {} from {}", image_name, self.url);

        // 在实际实现中，这里会：
        // 1. 连接到容器仓库
        // 2. 获取manifest
        // 3. 获取config
        // 4. 下载所有层
        // 5. 验证完整性

        Err(EIO) // 简化实现
    }

    /// 推送镜像
    pub fn push(&self, image: &OciImage, image_name: &str) -> Result<(), i32> {
        crate::println!("[oci-registry] Pushing image: {} to {}", image_name, self.url);

        // 在实际实现中，这里会：
        // 1. 上传所有层
        // 2. 上传config
        // 3. 上传manifest
        // 4. 更新标签

        Err(EIO) // 简化实现
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_oci_spec_validation() {
        let spec = OciSpecParser::create_default("test-container");
        assert!(OciSpecParser::validate(&spec).is_ok());
    }

    #[test]
    fn test_oci_spec_serialization() {
        let spec = OciSpecParser::create_default("test-container");
        let json = OciSpecParser::to_json(&spec);
        assert!(json.is_ok());
    }

    #[test]
    fn test_namespace_clone_flags() {
        assert_eq!(OciLinuxNamespaceType::Mount.to_clone_flag(), 0x00020000);
        assert_eq!(OciLinuxNamespaceType::UTS.to_clone_flag(), 0x04000000);
        assert_eq!(OciLinuxNamespaceType::IPC.to_clone_flag(), 0x08000000);
        assert_eq!(OciLinuxNamespaceType::Network.to_clone_flag(), 0x40000000);
        assert_eq!(OciLinuxNamespaceType::PID.to_clone_flag(), 0x20000000);
        assert_eq!(OciLinuxNamespaceType::User.to_clone_flag(), 0x10000000);
    }
}
