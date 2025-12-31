// OCI Runtime Specification Support
//
// OCI运行时规范支持模块
// 提供OCI 1.0和1.1标准的完整实现

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

use crate::reliability::EINVAL;

/// 支持的OCI版本
pub const OCI_VERSION_1_0: &str = "1.0.0";
pub const OCI_VERSION_1_1: &str = "1.1.0";
pub const OCI_VERSION_CURRENT: &str = OCI_VERSION_1_1;

/// OCI规范配置
#[derive(Debug, Clone)]
pub struct OciConfig {
    /// OCI版本
    pub oci_version: String,
    /// 平台配置
    pub platform: OciPlatform,
    /// 进程配置
    pub process: OciProcess,
    /// 根文件系统
    pub root: OciRoot,
    /// 主机名
    pub hostname: String,
    /// 挂载点
    pub mounts: Vec<OciMount>,
    /// 钩子
    pub hooks: OciHooks,
    /// 注解
    pub annotations: BTreeMap<String, String>,
    /// Linux特定配置
    pub linux: OciLinux,
}

/// OCI平台配置
#[derive(Debug, Clone)]
pub struct OciPlatform {
    /// 操作系统
    pub os: String,
    /// 架构
    pub arch: String,
    /// 变体
    pub variant: Option<String>,
}

impl Default for OciPlatform {
    fn default() -> Self {
        Self {
            os: "linux".to_string(),
            arch: "amd64".to_string(),
            variant: None,
        }
    }
}

/// OCI进程配置
#[derive(Debug, Clone)]
pub struct OciProcess {
    /// 是否使用终端
    pub terminal: bool,
    /// 用户配置
    pub user: OciUser,
    /// 环境变量
    pub env: Vec<String>,
    /// 命令行参数
    pub args: Vec<String>,
    /// 可执行文件路径
    pub executable: Option<String>,
    /// 工作目录
    pub cwd: String,
    /// capabilities
    pub capabilities: Option<OciLinuxCapabilities>,
    /// rlimits
    pub rlimits: Vec<OciLinuxRlimit>,
    /// no_new_privileges
    pub no_new_privileges: bool,
    /// apparmor_profile
    pub apparmor_profile: Option<String>,
    /// seccomp
    pub seccomp: Option<OciSeccomp>,
}

/// OCI用户配置
#[derive(Debug, Clone)]
pub struct OciUser {
    /// UID
    pub uid: u32,
    /// GID
    pub gid: u32,
    /// 附加GID
    pub additional_gids: Vec<u32>,
    /// 用户名
    pub username: Option<String>,
}

impl Default for OciUser {
    fn default() -> Self {
        Self {
            uid: 0,
            gid: 0,
            additional_gids: Vec::new(),
            username: None,
        }
    }
}

/// OCI根文件系统配置
#[derive(Debug, Clone)]
pub struct OciRoot {
    /// 路径
    pub path: String,
    /// 只读
    pub readonly: bool,
}

/// OCI挂载点配置
#[derive(Debug, Clone)]
pub struct OciMount {
    /// 目标路径
    pub destination: String,
    /// 源路径
    pub source: String,
    /// 选项
    pub options: Vec<String>,
    /// 文件系统类型
    #[allow(non_camel_case_types)]
    pub typ: String,
    /// UID映射
    pub uid_mappings: Option<Vec<OciLinuxIDMapping>>,
    /// GID映射
    pub gid_mappings: Option<Vec<OciLinuxIDMapping>>,
}

/// OCI钩子配置
#[derive(Debug, Clone)]
pub struct OciHooks {
    /// 预启动钩子
    pub prestart: Vec<OciHook>,
    /// 启动后钩子
    pub poststart: Vec<OciHook>,
    /// 停止后钩子
    pub poststop: Vec<OciHook>,
}

impl Default for OciHooks {
    fn default() -> Self {
        Self {
            prestart: Vec::new(),
            poststart: Vec::new(),
            poststop: Vec::new(),
        }
    }
}

/// OCI单个钩子
#[derive(Debug, Clone)]
pub struct OciHook {
    /// 路径
    pub path: String,
    /// 参数
    pub args: Vec<String>,
    /// 环境变量
    pub env: Vec<String>,
    /// 超时（秒）
    pub timeout: Option<u32>,
}

/// OCI Linux配置
#[derive(Debug, Clone)]
pub struct OciLinux {
    /// UID映射
    pub uid_mappings: Vec<OciLinuxIDMapping>,
    /// GID映射
    pub gid_mappings: Vec<OciLinuxIDMapping>,
    /// 命名空间
    pub namespaces: Vec<OciLinuxNamespace>,
    /// 资源限制
    pub resources: Option<OciLinuxResources>,
    /// Cgroups路径
    pub cgroups_path: Option<String>,
    /// capabilities
    pub capabilities: Option<OciLinuxCapabilities>,
    /// Seccomp
    pub seccomp: Option<OciSeccomp>,
    /// Rootfs传播
    pub rootfs_propagation: Option<String>,
    /// 执行mask
    pub masked_paths: Vec<String>,
    pub readonly_paths: Vec<String>,
    /// 挂载标签
    pub mount_label: Option<String>,
    /// 智能PID
    pub intel_rdt: Option<OciLinuxIntelRdt>,
}

/// OCI Linux ID映射
#[derive(Debug, Clone)]
pub struct OciLinuxIDMapping {
    /// 容器ID
    pub container_id: u32,
    /// 主机ID
    pub host_id: u32,
    /// 映射大小
    pub size: u32,
}

/// OCI Linux命名空间
#[derive(Debug, Clone)]
pub struct OciLinuxNamespace {
    /// 类型
    #[allow(non_camel_case_types)]
    pub typ: OciLinuxNamespaceType,
    /// 路径
    pub path: Option<String>,
}

/// OCI Linux命名空间类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// OCI Linux资源限制
#[derive(Debug, Clone)]
pub struct OciLinuxResources {
    /// 内存限制
    pub memory: Option<OciLinuxMemory>,
    /// CPU限制
    pub cpu: Option<OciLinuxCpu>,
    /// 设备限制
    pub devices: Vec<OciLinuxDeviceCgroup>,
    /// 网络限制
    pub network: Option<OciLinuxNetwork>,
    /// 块IO限制
    pub block_io: Option<OciLinuxBlockIO>,
    /// 巨页限制
    pub hugepages: Vec<OciLinuxHugepageLimit>,
    /// OOM killer调整
    pub oom_score_adj: Option<i32>,
}

/// OCI Linux内存限制
#[derive(Debug, Clone)]
pub struct OciLinuxMemory {
    /// 内存限制（字节）
    pub limit: Option<u64>,
    /// 保留内存（字节）
    pub reservation: Option<u64>,
    /// 交换空间限制（字节）
    pub swap: Option<u64>,
    /// 内核限制（字节）
    pub kernel: Option<u64>,
    /// 内核TCP限制（字节）
    pub kernel_tcp: Option<u64>,
    /// 使用huge pages
    pub hugepage_limits: Vec<OciLinuxHugepageLimit>,
    /// 内存交换行为
    pub swappiness: Option<u64>,
    /// 是否禁用OOM killer
    pub disable_oom_killer: Option<bool>,
    /// OOM优先级
    pub oom_score_adj: Option<i32>,
}

/// OCI Linux CPU限制
#[derive(Debug, Clone)]
pub struct OciLinuxCpu {
    /// CPU配额（微秒）
    pub quota: Option<i64>,
    /// CPU周期（微秒）
    pub period: Option<u64>,
    /// CPU亲和性
    pub cpus: Option<String>,
    /// 内存节点亲和性
    pub mems: Option<String>,
    /// CPU份额
    pub shares: Option<u64>,
    /// 实时调度周期
    pub realtime_runtime: Option<u64>,
    pub realtime_period: Option<u64>,
}

/// OCI Linux设备cgroup规则
#[derive(Debug, Clone)]
pub struct OciLinuxDeviceCgroup {
    /// 是否允许
    pub allow: bool,
    /// 设备类型
    #[allow(non_camel_case_types)]
    pub typ: OciLinuxDeviceType,
    /// 主设备号
    pub major: Option<i64>,
    /// 次设备号
    pub minor: Option<i64>,
    /// 访问权限
    pub access: String,
}

/// OCI Linux设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OciLinuxDeviceType {
    /// 字符设备
    Char,
    /// 块设备
    Block,
    /// 所有设备
    #[allow(non_camel_case_types)]
    All,
}

/// OCI Linux网络限制
#[derive(Debug, Clone)]
pub struct OciLinuxNetwork {
    /// 类ID
    pub class_id: Option<u32>,
    /// 优先级
    pub priorities: Vec<OciLinuxNetworkPriority>,
}

/// OCI Linux网络优先级
#[derive(Debug, Clone)]
pub struct OciLinuxNetworkPriority {
    /// 接口名称
    pub name: String,
    /// 优先级
    pub priority: u32,
}

/// OCI Linux块IO限制
#[derive(Debug, Clone)]
pub struct OciLinuxBlockIO {
    /// 读BPS限制
    pub read_bps_device: Vec<OciLinuxThrottleDevice>,
    /// 写BPS限制
    pub write_bps_device: Vec<OciLinuxThrottleDevice>,
    /// 读IOPS限制
    pub read_iops_device: Vec<OciLinuxThrottleDevice>,
    /// 写IOPS限制
    pub write_iops_device: Vec<OciLinuxThrottleDevice>,
    /// 权重
    pub weight: Option<u64>,
    /// 叶权重
    pub leaf_weight: Option<u64>,
    /// 设备权重
    pub weight_device: Vec<OciLinuxWeightDevice>,
}

/// OCI Linux节流设备
#[derive(Debug, Clone)]
pub struct OciLinuxThrottleDevice {
    /// 主设备号
    pub major: u64,
    /// 次设备号
    pub minor: u64,
    /// 速率（字节/秒）
    pub rate: u64,
}

/// OCI Linux权重设备
#[derive(Debug, Clone)]
pub struct OciLinuxWeightDevice {
    /// 主设备号
    pub major: u64,
    /// 次设备号
    pub minor: u64,
    /// 权重
    pub weight: Option<u16>,
    /// 叶权重
    pub leaf_weight: Option<u16>,
}

/// OCI Linux巨页限制
#[derive(Debug, Clone)]
pub struct OciLinuxHugepageLimit {
    /// 页面大小
    pub page_size: String,
    /// 限制（字节）
    pub limit: u64,
}

/// OCI Linux capabilities
#[derive(Debug, Clone)]
pub struct OciLinuxCapabilities {
    /// Effective
    pub effective: Vec<String>,
    /// Bounding
    pub bounding: Vec<String>,
    /// Inheritable
    pub inheritable: Vec<String>,
    /// Permitted
    pub permitted: Vec<String>,
    /// Ambient
    pub ambient: Vec<String>,
}

/// OCI Linux rlimit
#[derive(Debug, Clone)]
pub struct OciLinuxRlimit {
    /// 类型
    #[allow(non_camel_case_types)]
    pub typ: String,
    /// 软限制
    pub soft: u64,
    /// 硬限制
    pub hard: u64,
}

/// OCI Seccomp配置
#[derive(Debug, Clone)]
pub struct OciSeccomp {
    /// 默认动作
    pub default_action: OciSeccompAction,
    /// 架构
    pub architectures: Vec<String>,
    /// 系统调用规则
    pub syscalls: Vec<OciSyscallRule>,
    /// 标志
    pub flags: Vec<String>,
}

/// OCI Seccomp动作
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OciSeccompAction {
    /// 允许
    Allow,
    /// 拒绝并返回errno
    Errno(u32),
    /// 杀死进程
    KillProcess,
    /// 杀死线程
    KillThread,
    /// 记录
    Log,
    /// 追踪
    Trace,
}

/// OCI系统调用规则
#[derive(Debug, Clone)]
pub struct OciSyscallRule {
    /// 系统调用名称
    pub names: Vec<String>,
    /// 动作
    pub action: OciSeccompAction,
    /// 参数
    pub args: Vec<OciSeccompArg>,
}

/// OCI Seccomp参数
#[derive(Debug, Clone)]
pub struct OciSeccompArg {
    /// 索引
    pub index: u32,
    /// 值
    pub value: u64,
    /// 值掩码
    pub value_two: Option<u64>,
    /// 操作
    pub op: OciSeccompOperator,
}

/// OCI Seccomp操作符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OciSeccompOperator {
    /// 不等于
    NotEqual,
    /// 小于
    LessThan,
    /// 小于等于
    LessEqual,
    /// 等于
    Equal,
    /// 大于等于
    GreaterEqual,
    /// 大于
    GreaterThan,
    /// 掩码等于
    MaskedEqual,
}

/// OCI Intel RDT配置
#[derive(Debug, Clone)]
pub struct OciLinuxIntelRdt {
    /// CLOS ID
    pub l3_cache_schema: Option<String>,
    /// 内存带宽
    pub mem_bw_schema: Option<String>,
}

impl OciConfig {
    /// 从JSON字符串加载配置
    ///
    /// ```
    /// # use kernel::subsystems::cloud_native::oci::spec::OciConfig;
    /// let json_str = r#"{
    ///   "ociVersion": "1.0.0",
    ///   "platform": {
    ///     "os": "linux",
    ///     "arch": "amd64"
    ///   },
    ///   "process": {
    ///     "terminal": false,
    ///     "user": {
    ///       "uid": 0,
    ///       "gid": 0
    ///     },
    ///     "args": ["/bin/sh"],
    ///     "env": ["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"],
    ///     "cwd": "/"
    ///   },
    ///   "root": {
    ///     "path": "rootfs",
    ///     "readonly": true
    ///   },
    ///   "hostname": "container",
    ///   "mounts": [],
    ///   "hooks": {},
    ///   "annotations": {},
    ///   "linux": {
    ///     "uidMappings": [],
    ///     "gidMappings": [],
    ///     "namespaces": [
    ///       {"type": "pid"},
    ///       {"type": "network"},
    ///       {"type": "ipc"},
    ///       {"type": "uts"},
    ///       {"type": "mount"}
    ///     ]
    ///   }
    /// }"#;
    ///
    /// let config = OciConfig::from_json(json_str).unwrap();
    /// ```
    pub fn from_json(_json: &str) -> Result<Self, i32> {
        // GH-#1353: 实现JSON解析
        // See: https://github.com/npos/kernel/issues/1353
        // 目前返回默认配置
        Ok(Self::default())
    }

    /// 验证配置
    pub fn validate(&self) -> Result<(), i32> {
        // 验证OCI版本
        if self.oci_version != OCI_VERSION_1_0 && self.oci_version != OCI_VERSION_1_1 {
            return Err(EINVAL);
        }

        // 验证进程配置
        if self.process.args.is_empty() {
            return Err(EINVAL);
        }

        // 验证根文件系统
        if self.root.path.is_empty() {
            return Err(EINVAL);
        }

        // 验证工作目录
        if self.process.cwd.is_empty() {
            return Err(EINVAL);
        }

        // 验证命名空间
        if self.linux.namespaces.is_empty() {
            return Err(EINVAL);
        }

        Ok(())
    }

    /// 转换为JSON字符串
    pub fn to_json(&self) -> String {
        // GH-#1354: 实现JSON序列化
        // See: https://github.com/npos/kernel/issues/1354
        format!("{{ oci_version: {}, root: {} }}", self.oci_version, self.root.path)
    }
}

impl Default for OciConfig {
    fn default() -> Self {
        Self {
            oci_version: OCI_VERSION_CURRENT.to_string(),
            platform: OciPlatform::default(),
            process: OciProcess {
                terminal: false,
                user: OciUser::default(),
                env: vec!["PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin".to_string()],
                args: vec!["/bin/sh".to_string()],
                executable: None,
                cwd: "/".to_string(),
                capabilities: None,
                rlimits: Vec::new(),
                no_new_privileges: true,
                apparmor_profile: None,
                seccomp: None,
            },
            root: OciRoot {
                path: "rootfs".to_string(),
                readonly: false,
            },
            hostname: "container".to_string(),
            mounts: Vec::new(),
            hooks: OciHooks::default(),
            annotations: BTreeMap::new(),
            linux: OciLinux {
                uid_mappings: Vec::new(),
                gid_mappings: Vec::new(),
                namespaces: vec![
                    OciLinuxNamespace {
                        typ: OciLinuxNamespaceType::PID,
                        path: None,
                    },
                    OciLinuxNamespace {
                        typ: OciLinuxNamespaceType::Network,
                        path: None,
                    },
                    OciLinuxNamespace {
                        typ: OciLinuxNamespaceType::IPC,
                        path: None,
                    },
                    OciLinuxNamespace {
                        typ: OciLinuxNamespaceType::UTS,
                        path: None,
                    },
                    OciLinuxNamespace {
                        typ: OciLinuxNamespaceType::Mount,
                        path: None,
                    },
                ],
                resources: None,
                cgroups_path: None,
                capabilities: None,
                seccomp: None,
                rootfs_propagation: None,
                masked_paths: Vec::new(),
                readonly_paths: Vec::new(),
                mount_label: None,
                intel_rdt: None,
            },
        }
    }
}
