// Container Cgroup Management
//
// 容器Cgroup管理模块
// 提供CPU/内存/IO层级cgroup资源限制和统计信息功能

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic {AtomicU64,, Ordering};

use spin::Mutex;

use crate::reliability::{EINVAL, EIO, ENOENT, ENOMEM};

/// Cgroup版本
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupVersion {
    /// Cgroup v1
    V1,
    /// Cgroup v2（统一层次结构）
    V2,
}

/// Cgroup子系统类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CgroupSubsystem {
    /// CPU子系统
    Cpu,
    /// CPU集合子系统
    Cpuset,
    /// CPU调度器（实时）
    Cpuacct,
    /// 内存子系统
    Memory,
    /// 块设备I/O子系统
    Blkio,
    /// I/O子系统（v2）
    Io,
    /// 设备子系统
    Devices,
    /// 冻结子系统
    Freezer,
    /// 网络类别子系统
    NetCls,
    /// 网络优先级子系统
    NetPrio,
    /// 进程ID子系统
    Pids,
    /// 性能事件子系统
    PerfEvent,
}

impl CgroupSubsystem {
    /// 获取子系统名称（v1）
    pub fn v1_name(&self) -> &str {
        match self {
            CgroupSubsystem::Cpu => "cpu",
            CgroupSubsystem::Cpuset => "cpuset",
            CgroupSubsystem::Cpuacct => "cpuacct",
            CgroupSubsystem::Memory => "memory",
            CgroupSubsystem::Blkio => "blkio",
            CgroupSubsystem::Io => "blkio", // v1中使用blkio
            CgroupSubsystem::Devices => "devices",
            CgroupSubsystem::Freezer => "freezer",
            CgroupSubsystem::NetCls => "net_cls",
            CgroupSubsystem::NetPrio => "net_prio",
            CgroupSubsystem::Pids => "pids",
            CgroupSubsystem::PerfEvent => "perf_event",
        }
    }

    /// 获取子系统名称（v2）
    pub fn v2_name(&self) -> &str {
        match self {
            CgroupSubsystem::Cpu => "cpu",
            CgroupSubsystem::Memory => "memory",
            CgroupSubsystem::Io => "io",
            CgroupSubsystem::Pids => "pids",
            _ => "",
        }
    }

    /// 检查是否在v2中支持
    pub fn supported_in_v2(&self) -> bool {
        matches!(
            self,
            CgroupSubsystem::Cpu
                | CgroupSubsystem::Memory
                | CgroupSubsystem::Io
                | CgroupSubsystem::Pids
        )
    }
}

/// Cgroup资源配置
#[derive(Debug, Clone)]
pub struct CgroupResources {
    /// CPU配置
    pub cpu: Option<CpuResources>,
    /// 内存配置
    pub memory: Option<MemoryResources>,
    /// I/O配置
    pub io: Option<IoResources>,
    /// 块I/O配置（v1）
    pub blkio: Option<BlkioResources>,
    /// 进程数配置
    pub pids: Option<PidsResources>,
    /// 设备配置
    pub devices: Option<DeviceResources>,
}

/// CPU资源配置
#[derive(Debug, Clone)]
pub struct CpuResources {
    /// CPU配额（微秒，-1表示无限制）
    pub quota: i64,
    /// CPU周期（微秒）
    pub period: u64,
    /// CPU份额（1024为基准）
    pub shares: u64,
    /// CPU亲和性（CPU列表）
    pub cpus: Option<String>,
    /// 内存节点亲和性
    pub mems: Option<String>,
    /// 实时运行时间（微秒）
    pub rt_runtime: Option<u64>,
    /// 实时周期（微秒）
    pub rt_period: Option<u64>,
}

impl Default for CpuResources {
    fn default() -> Self {
        Self {
            quota: -1,      // 无限制
            period: 100000, // 100ms
            shares: 1024,
            cpus: None,
            mems: None,
            rt_runtime: None,
            rt_period: None,
        }
    }
}

/// 内存资源配置
#[derive(Debug, Clone)]
pub struct MemoryResources {
    /// 内存限制（字节）
    pub limit: u64,
    /// 软限制（字节）
    pub soft_limit: Option<u64>,
    /// 交换空间限制（字节）
    pub swap_limit: Option<u64>,
    /// 内核内存限制（字节）
    pub kernel_limit: Option<u64>,
    /// 内核TCP内存限制（字节）
    pub tcp_limit: Option<u64>,
    /// OOM控制
    pub oom_control: OomControl,
    /// 内存保留（字节）
    pub reservation: Option<u64>,
}

/// OOM控制
#[derive(Debug, Clone, Copy)]
pub struct OomControl {
    /// 是否禁用OOM killer
    pub disable_oom_killer: bool,
    /// OOM优先级
    pub oom_score_adj: Option<i32>,
}

impl Default for OomControl {
    fn default() -> Self {
        Self { disable_oom_killer: false, oom_score_adj: None }
    }
}

impl Default for MemoryResources {
    fn default() -> Self {
        Self {
            limit: u64::MAX,
            soft_limit: None,
            swap_limit: None,
            kernel_limit: None,
            tcp_limit: None,
            oom_control: OomControl::default(),
            reservation: None,
        }
    }
}

/// I/O资源配置（v2）
#[derive(Debug, Clone)]
pub struct IoResources {
    /// I/O权重（1-10000）
    pub weight: Option<u16>,
    /// 叶权重
    pub leaf_weight: Option<u16>,
    /// I/O限制
    pub max: Option<Vec<IoLimit>>,
}

/// I/O限制
#[derive(Debug, Clone)]
pub struct IoLimit {
    /// 主设备号
    pub major: u32,
    /// 次设备号
    pub minor: u32,
    /// 读带宽（字节/秒）
    pub read_bps: Option<u64>,
    /// 写带宽（字节/秒）
    pub write_bps: Option<u64>,
    /// 读IOPS
    pub read_iops: Option<u64>,
    /// 写IOPS
    pub write_iops: Option<u64>,
}

/// 块I/O资源配置（v1）
#[derive(Debug, Clone)]
pub struct BlkioResources {
    /// 块I/O权重
    pub weight: Option<u16>,
    /// 叶权重
    pub leaf_weight: Option<u16>,
    /// 权重设备
    pub weight_device: Option<Vec<BlkioWeightDevice>>,
    /// 节流配置
    pub throttle: Option<Vec<BlkioThrottle>>,
}

/// 块I/O权重设备
#[derive(Debug, Clone)]
pub struct BlkioWeightDevice {
    /// 主设备号
    pub major: u32,
    /// 次设备号
    pub minor: u32,
    /// 权重
    pub weight: u16,
    /// 叶权重
    pub leaf_weight: Option<u16>,
}

/// 块I/O节流配置
#[derive(Debug, Clone)]
pub struct BlkioThrottle {
    /// 主设备号
    pub major: u32,
    /// 次设备号
    pub minor: u32,
    /// 读带宽（字节/秒）
    pub read_bps: Option<u64>,
    /// 写带宽（字节/秒）
    pub write_bps: Option<u64>,
    /// 读IOPS
    pub read_iops: Option<u64>,
    /// 写IOPS
    pub write_iops: Option<u64>,
}

/// 进程数资源配置
#[derive(Debug, Clone)]
pub struct PidsResources {
    /// 最大进程数（-1表示无限制）
    pub max: i64,
}

impl Default for PidsResources {
    fn default() -> Self {
        Self { max: -1 }
    }
}

/// 设备资源配置
#[derive(Debug, Clone)]
pub struct DeviceResources {
    /// 设备规则
    pub rules: Vec<DeviceRule>,
}

/// 设备规则
#[derive(Debug, Clone)]
pub struct DeviceRule {
    /// 是否允许
    pub allow: bool,
    /// 设备类型
    pub dev_type: DeviceType,
    /// 主设备号
    pub major: Option<i64>,
    /// 次设备号
    pub minor: Option<i64>,
    /// 访问权限
    pub access: String,
}

/// 设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    /// 字符设备
    Char,
    /// 块设备
    Block,
    /// 所有设备
    All,
}

/// Cgroup统计信息
#[derive(Debug, Clone)]
pub struct CgroupStats {
    /// CPU统计
    pub cpu: Option<CpuStats>,
    /// 内存统计
    pub memory: Option<MemoryStats>,
    /// I/O统计
    pub io: Option<IoStats>,
    /// 进程数统计
    pub pids: Option<PidsStats>,
    /// 摘要统计
    pub summary: SummaryStats,
}

/// CPU统计
#[derive(Debug, Clone)]
pub struct CpuStats {
    /// 使用时间（纳秒）
    pub usage: u64,
    /// 用户时间（纳秒）
    pub user: u64,
    /// 系统时间（纳秒）
    pub system: u64,
    /// 使用的CPU核数
    pub nr_periods: u64,
    /// 被节流的周期数
    pub nr_throttled: u64,
    /// 被节流的时间（纳秒）
    pub throttled_time: u64,
}

/// 内存统计
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// 当前使用量（字节）
    pub usage: u64,
    /// 最大使用量（字节）
    pub max_usage: u64,
    /// 交换空间使用量（字节）
    pub swap_usage: u64,
    /// 内核内存使用量（字节）
    pub kernel_usage: u64,
    /// 缓存（字节）
    pub cache: u64,
    /// RSS（字节）
    pub rss: u64,
    /// 页面错误数
    pub pgfault: u64,
    /// 主要页面错误数
    pub pgmajfault: u64,
}

/// I/O统计
#[derive(Debug, Clone)]
pub struct IoStats {
    /// 读字节数
    pub read_bytes: u64,
    /// 写字节数
    pub write_bytes: u64,
    /// 读操作数
    pub read_ios: u64,
    /// 写操作数
    pub write_ios: u64,
    /// 同步读字节数
    pub sync_read_bytes: u64,
    /// 同步写字节数
    pub sync_write_bytes: u64,
    /// 异步读字节数
    pub async_read_bytes: u64,
    /// 异步写字节数
    pub async_write_bytes: u64,
}

/// 进程数统计
#[derive(Debug, Clone)]
pub struct PidsStats {
    /// 当前进程数
    pub current: u64,
    /// 最大进程数
    pub max: i64,
}

/// 摘要统计
#[derive(Debug, Clone)]
pub struct SummaryStats {
    /// 进程数
    pub nr_processes: usize,
    /// 线程数
    pub nr_threads: usize,
}

/// Cgroup实例
pub struct Cgroup {
    /// Cgroup路径
    pub path: String,
    /// Cgroup版本
    pub version: CgroupVersion,
    /// 启用的子系统
    pub subsystems: Vec<CgroupSubsystem>,
    /// 资源配置
    pub resources: CgroupResources,
    /// 进程列表
    pub processes: Arc<Mutex<Vec<u32>>>,
    /// 统计信息
    pub stats: Arc<Mutex<CgroupStats>>,
    /// 是否激活
    pub active: bool,
}

impl Cgroup {
    /// 创建新的cgroup
    pub fn new(
        path: String,
        version: CgroupVersion,
        subsystems: Vec<CgroupSubsystem>,
        resources: CgroupResources,
    ) -> Self {
        let stats = CgroupStats {
            cpu: None,
            memory: None,
            io: None,
            pids: None,
            summary: SummaryStats { nr_processes: 0, nr_threads: 0 },
        };

        Self {
            path,
            version,
            subsystems,
            resources,
            processes: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(stats)),
            active: false,
        }
    }

    /// 创建cgroup
    pub fn create(&mut self) -> Result<(), i32> {
        // 创建cgroup目录
        self.create_cgroup_directory()?;

        // 配置子系统
        self.configure_subsystems()?;

        self.active = true;

        crate::println!("[container-cgroup] Created cgroup: {}", self.path);

        Ok(())
    }

    /// 创建cgroup目录
    fn create_cgroup_directory(&self) -> Result<(), i32> {
        match self.version {
            CgroupVersion::V1 => self.create_v1_directory(),
            CgroupVersion::V2 => self.create_v2_directory(),
        }
    }

    /// 创建v1目录
    fn create_v1_directory(&self) -> Result<(), i32> {
        // 为每个子系统创建目录
        for subsystem in &self.subsystems {
            let subsystem_path = format!("/sys/fs/cgroup/{}/{}", subsystem.v1_name(), self.path);
            crate::println!(
                "[container-cgroup] Creating v1 directory: {}",
                subsystem_path
            );

            // 在实际实现中，这里会创建文件系统目录
        }

        Ok(())
    }

    /// 创建v2目录
    fn create_v2_directory(&self) -> Result<(), i32> {
        let unified_path = format!("/sys/fs/cgroup/unified/{}", self.path);
        crate::println!("[container-cgroup] Creating v2 directory: {}", unified_path);

        // 在实际实现中，这里会创建文件系统目录
        Ok(())
    }

    /// 配置子系统
    fn configure_subsystems(&self) -> Result<(), i32> {
        // 配置CPU
        if let Some(ref cpu) = self.resources.cpu {
            self.configure_cpu(cpu)?;
        }

        // 配置内存
        if let Some(ref memory) = self.resources.memory {
            self.configure_memory(memory)?;
        }

        // 配置I/O
        if self.version == CgroupVersion::V2 {
            if let Some(ref io) = self.resources.io {
                self.configure_io_v2(io)?;
            }
        } else {
            if let Some(ref blkio) = self.resources.blkio {
                self.configure_blkio(blkio)?;
            }
        }

        // 配置进程数
        if let Some(ref pids) = self.resources.pids {
            self.configure_pids(pids)?;
        }

        Ok(())
    }

    /// 配置CPU
    fn configure_cpu(&self, cpu: &CpuResources) -> Result<(), i32> {
        match self.version {
            CgroupVersion::V1 => {
                self.write_v1_file("cpu", "cpu.cfs_quota_us", &cpu.quota.to_string())?;
                self.write_v1_file("cpu", "cpu.cfs_period_us", &cpu.period.to_string())?;
                self.write_v1_file("cpu", "cpu.shares", &cpu.shares.to_string())?;

                if let Some(ref cpus) = cpu.cpus {
                    self.write_v1_file("cpuset", "cpuset.cpus", cpus)?;
                }
                if let Some(ref mems) = cpu.mems {
                    self.write_v1_file("cpuset", "cpuset.mems", mems)?;
                }
            },
            CgroupVersion::V2 => {
                self.write_v2_file(
                    "cpu.max",
                    &format!("{} {}", cpu.quota, cpu.period),
                )?;
                self.write_v2_file("cpu.weight", &cpu.shares.to_string())?;
            },
        }

        Ok(())
    }

    /// 配置内存
    fn configure_memory(&self, memory: &MemoryResources) -> Result<(), i32> {
        match self.version {
            CgroupVersion::V1 => {
                self.write_v1_file("memory", "memory.limit_in_bytes", &memory.limit.to_string())?;

                if let Some(soft_limit) = memory.soft_limit {
                    self.write_v1_file("memory", "memory.soft_limit_in_bytes", &soft_limit.to_string())?;
                }

                if let Some(swap_limit) = memory.swap_limit {
                    self.write_v1_file(
                        "memory",
                        "memory.memsw.limit_in_bytes",
                        &swap_limit.to_string(),
                    )?;
                }

                if memory.oom_control.disable_oom_killer {
                    self.write_v1_file("memory", "memory.oom_control", "1")?;
                }
            },
            CgroupVersion::V2 => {
                self.write_v2_file("memory.max", &memory.limit.to_string())?;

                if memory.oom_control.disable_oom_killer {
                    self.write_v2_file("memory.oom.group", "1")?;
                }
            },
        }

        Ok(())
    }

    /// 配置I/O（v2）
    fn configure_io_v2(&self, io: &IoResources) -> Result<(), i32> {
        if let Some(weight) = io.weight {
            self.write_v2_file("io.weight", &format!("default {}", weight))?;
        }

        if let Some(ref limits) = io.max {
            for limit in limits {
                let config = format!(
                    "{}:{} rbps={} wbps={} riops={} wiops={}",
                    limit.major,
                    limit.minor,
                    limit.read_bps.unwrap_or(0),
                    limit.write_bps.unwrap_or(0),
                    limit.read_iops.unwrap_or(0),
                    limit.write_iops.unwrap_or(0)
                );
                self.write_v2_file("io.max", &config)?;
            }
        }

        Ok(())
    }

    /// 配置块I/O（v1）
    fn configure_blkio(&self, blkio: &BlkioResources) -> Result<(), i32> {
        if let Some(weight) = blkio.weight {
            self.write_v1_file("blkio", "blkio.weight", &weight.to_string())?;
        }

        if let Some(ref weight_devices) = blkio.weight_device {
            for wd in weight_devices {
                let config = format!("{}:{} {}", wd.major, wd.minor, wd.weight);
                self.write_v1_file("blkio", "blkio.weight_device", &config)?;
            }
        }

        if let Some(ref throttles) = blkio.throttle {
            for throttle in throttles {
                if let Some(read_bps) = throttle.read_bps {
                    let config = format!("{}:{} {}", throttle.major, throttle.minor, read_bps);
                    self.write_v1_file("blkio", "blkio.throttle.read_bps_device", &config)?;
                }

                if let Some(write_bps) = throttle.write_bps {
                    let config = format!("{}:{} {}", throttle.major, throttle.minor, write_bps);
                    self.write_v1_file("blkio", "blkio.throttle.write_bps_device", &config)?;
                }
            }
        }

        Ok(())
    }

    /// 配置进程数
    fn configure_pids(&self, pids: &PidsResources) -> Result<(), i32> {
        self.write_v1_file("pids", "pids.max", &pids.max.to_string())?;
        Ok(())
    }

    /// 写入v1文件
    fn write_v1_file(&self, subsystem: &str, filename: &str, value: &str) -> Result<(), i32> {
        let path = format!("/sys/fs/cgroup/{}/{}/{}", subsystem, self.path, filename);
        crate::println!("[container-cgroup] Writing '{}' to {}", value, path);

        // 在实际实现中，这里会写入文件
        Ok(())
    }

    /// 写入v2文件
    fn write_v2_file(&self, filename: &str, value: &str) -> Result<(), i32> {
        let path = format!("/sys/fs/cgroup/unified/{}/{}", self.path, filename);
        crate::println!("[container-cgroup] Writing '{}' to {}", value, path);

        // 在实际实现中，这里会写入文件
        Ok(())
    }

    /// 添加进程
    pub fn add_process(&self, pid: u32) -> Result<(), i32> {
        if !self.active {
            return Err(EINVAL);
        }

        match self.version {
            CgroupVersion::V1 => {
                for subsystem in &self.subsystems {
                    self.write_v1_file(subsystem.v1_name(), "tasks", &pid.to_string())?;
                }
            },
            CgroupVersion::V2 => {
                self.write_v2_file("cgroup.procs", &pid.to_string())?;
            },
        }

        {
            let mut processes = self.processes.lock();
            if !processes.contains(&pid) {
                processes.push(pid);
            }
        }

        crate::println!("[container-cgroup] Added process {} to cgroup {}", pid, self.path);

        Ok(())
    }

    /// 移除进程
    pub fn remove_process(&self, pid: u32) -> Result<(), i32> {
        if !self.active {
            return Err(EINVAL);
        }

        // 将进程移到父cgroup
        // 在实际实现中，这里会将进程ID写入父cgroup的tasks文件

        {
            let mut processes = self.processes.lock();
            processes.retain(|&p| p != pid);
        }

        crate::println!("[container-cgroup] Removed process {} from cgroup {}", pid, self.path);

        Ok(())
    }

    /// 删除cgroup
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

        // 删除cgroup目录
        self.remove_cgroup_directory()?;

        self.active = false;

        crate::println!("[container-cgroup] Destroyed cgroup: {}", self.path);

        Ok(())
    }

    /// 删除cgroup目录
    fn remove_cgroup_directory(&self) -> Result<(), i32> {
        match self.version {
            CgroupVersion::V1 => {
                for subsystem in &self.subsystems {
                    let subsystem_path =
                        format!("/sys/fs/cgroup/{}/{}", subsystem.v1_name(), self.path);
                    crate::println!(
                        "[container-cgroup] Removing v1 directory: {}",
                        subsystem_path
                    );
                }
            },
            CgroupVersion::V2 => {
                let unified_path = format!("/sys/fs/cgroup/unified/{}", self.path);
                crate::println!("[container-cgroup] Removing v2 directory: {}", unified_path);
            },
        }

        Ok(())
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> CgroupStats {
        let mut stats = self.stats.lock();

        // 更新统计信息
        stats.summary.nr_processes = self.processes.lock().len();

        if let Some(ref cpu_stats) = stats.cpu {
            // 更新CPU统计
            let _ = cpu_stats;
        }

        if let Some(ref memory_stats) = stats.memory {
            // 更新内存统计
            let _ = memory_stats;
        }

        stats.clone()
    }
}

/// Cgroup管理器
pub struct CgroupManager {
    /// Cgroup列表
    pub cgroups: BTreeMap<String, Arc<Mutex<Cgroup>>>,
    /// Cgroup版本
    pub version: CgroupVersion,
    /// 根路径
    pub root_path: String,
    /// 下一个cgroup ID
    next_cgroup_id: AtomicU64,
}

impl CgroupManager {
    /// 创建新的cgroup管理器
    pub fn new(version: CgroupVersion) -> Self {
        let root_path = match version {
            CgroupVersion::V1 => "/sys/fs/cgroup".to_string(),
            CgroupVersion::V2 => "/sys/fs/cgroup/unified".to_string(),
        };

        Self {
            cgroups: BTreeMap::new(),
            version,
            root_path,
            next_cgroup_id: AtomicU64::new(1),
        }
    }

    /// 创建cgroup
    pub fn create(&mut self, name: &str, resources: CgroupResources) -> Result<Arc<Mutex<Cgroup>>, i32> {
        // 确定要启用的子系统
        let subsystems = self.determine_subsystems(&resources);

        let path = format!("{}/{}", self.root_path, name);

        let mut cgroup = Cgroup::new(path, self.version, subsystems, resources);
        cgroup.create()?;

        let cgroup_arc = Arc::new(Mutex::new(cgroup));
        self.cgroups.insert(name.to_string(), cgroup_arc.clone());

        Ok(cgroup_arc)
    }

    /// 确定要启用的子系统
    fn determine_subsystems(&self, resources: &CgroupResources) -> Vec<CgroupSubsystem> {
        let mut subsystems = Vec::new();

        if resources.cpu.is_some() {
            subsystems.push(CgroupSubsystem::Cpu);
        }

        if resources.memory.is_some() {
            subsystems.push(CgroupSubsystem::Memory);
        }

        if self.version == CgroupVersion::V2 {
            if resources.io.is_some() {
                subsystems.push(CgroupSubsystem::Io);
            }
        } else {
            if resources.blkio.is_some() {
                subsystems.push(CgroupSubsystem::Blkio);
            }
        }

        if resources.pids.is_some() {
            subsystems.push(CgroupSubsystem::Pids);
        }

        subsystems
    }

    /// 获取cgroup
    pub fn get(&self, name: &str) -> Option<Arc<Mutex<Cgroup>>> {
        self.cgroups.get(name).cloned()
    }

    /// 删除cgroup
    pub fn delete(&mut self, name: &str) -> Result<(), i32> {
        if let Some(cgroup) = self.cgroups.remove(name) {
            let mut cg = cgroup.lock();
            cg.destroy()?;
            Ok(())
        } else {
            Err(ENOENT)
        }
    }

    /// 列出所有cgroup
    pub fn list(&self) -> Vec<String> {
        self.cgroups.keys().cloned().collect()
    }

    /// 清理所有cgroup
    pub fn cleanup_all(&mut self) -> Result<(), i32> {
        let names: Vec<String> = self.cgroups.keys().cloned().collect();

        for name in names {
            if let Err(e) = self.delete(&name) {
                crate::println!(
                    "[container-cgroup] Warning: Failed to delete cgroup {}: {}",
                    name,
                    e
                );
            }
        }

        Ok(())
    }
}

/// 全局Cgroup管理器实例
static mut CGROUP_MANAGER_V1: Option<CgroupManager> = None;
static mut CGROUP_MANAGER_V2: Option<CgroupManager> = None;
static mut CGROUP_MANAGER_INITIALIZED: bool = false;

/// 初始化Cgroup管理器
pub fn init_cgroup_manager() -> Result<(), i32> {
    if unsafe { CGROUP_MANAGER_INITIALIZED } {
        return Ok(());
    }

    let v1_manager = CgroupManager::new(CgroupVersion::V1);
    let v2_manager = CgroupManager::new(CgroupVersion::V2);

    unsafe {
        CGROUP_MANAGER_V1 = Some(v1_manager);
        CGROUP_MANAGER_V2 = Some(v2_manager);
        CGROUP_MANAGER_INITIALIZED = true;
    }

    crate::println!("[container-cgroup] Cgroup manager initialized");
    Ok(())
}

/// 获取v1管理器
pub fn get_v1_manager() -> Option<&'static CgroupManager> {
    unsafe { CGROUP_MANAGER_V1.as_ref() }
}

/// 获取v1管理器（可变）
pub fn get_v1_manager_mut() -> Option<&'static mut CgroupManager> {
    unsafe { CGROUP_MANAGER_V1.as_mut() }
}

/// 获取v2管理器
pub fn get_v2_manager() -> Option<&'static CgroupManager> {
    unsafe { CGROUP_MANAGER_V2.as_ref() }
}

/// 获取v2管理器（可变）
pub fn get_v2_manager_mut() -> Option<&'static mut CgroupManager> {
    unsafe { CGROUP_MANAGER_V2.as_mut() }
}
