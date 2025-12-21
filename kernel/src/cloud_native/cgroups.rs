// Control Groups (cgroups) Support Module
//
// 控制组支持模块
// 提供进程资源限制和隔离功能

extern crate alloc;

use alloc::format;
use crate::reliability::errno::{EINVAL, ENOENT, ENOMEM, EIO, EACCES};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::string::ToString;
use alloc::sync::Arc;
use spin::Mutex;
use alloc::vec;
use alloc::vec::Vec;

/// cgroup版本
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupVersion {
    V1,
    V2,
}

/// cgroup子系统类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CgroupSubsystem {
    /// CPU子系统
    Cpu,
    /// 内存子系统
    Memory,
    /// I/O子系统
    Io,
    /// 块设备子系统
    Blkio,
    /// CPU集合子系统
    Cpuset,
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
}

/// cgroup资源配置
#[derive(Debug, Clone)]
pub struct CgroupResourceConfig {
    /// CPU配置
    pub cpu: Option<CpuConfig>,
    /// 内存配置
    pub memory: Option<MemoryConfig>,
    /// I/O配置
    pub io: Option<IoConfig>,
    /// 块设备配置
    pub blkio: Option<BlkioConfig>,
    /// 进程ID配置
    pub pids: Option<PidsConfig>,
}

/// CPU配置
#[derive(Debug, Clone)]
pub struct CpuConfig {
    /// CPU配额（微秒）
    pub quota: Option<i64>,
    /// CPU周期（微秒）
    pub period: Option<u64>,
    /// CPU份额
    pub shares: Option<u64>,
    /// CPU亲和性
    pub cpus: Option<String>,
    /// 内存节点亲和性
    pub mems: Option<String>,
    /// 实时运行时间（微秒）
    pub rt_runtime: Option<u64>,
    /// 实时周期（微秒）
    pub rt_period: Option<u64>,
}

/// 内存配置
#[derive(Debug, Clone)]
pub struct MemoryConfig {
    /// 内存限制（字节）
    pub limit: Option<u64>,
    /// 软限制（字节）
    pub soft_limit: Option<u64>,
    /// 交换空间限制（字节）
    pub swap_limit: Option<u64>,
    /// 内核限制（字节）
    pub kernel_limit: Option<u64>,
    /// 内核TCP限制（字节）
    pub kernel_tcp_limit: Option<u64>,
    /// 最大使用量限制（字节）
    pub max_usage: Option<u64>,
    /// OOM控制
    pub oom_control: OomControl,
}

/// OOM控制
#[derive(Debug, Clone)]
pub struct OomControl {
    /// 是否禁用OOM killer
    pub oom_kill_disable: bool,
    /// OOM控制优先级
    pub oom_kill_adj: Option<i32>,
    /// OOM kill优先级
    pub oom_kill_prio: Option<u16>,
}

impl Default for OomControl {
    fn default() -> Self {
        Self {
            oom_kill_disable: false,
            oom_kill_adj: None,
            oom_kill_prio: None,
        }
    }
}

/// I/O配置
#[derive(Debug, Clone)]
pub struct IoConfig {
    /// I/O权重
    pub weight: Option<u16>,
    /// I/O最大带宽限制
    pub max: Option<Vec<IoMax>>,
}

/// I/O最大限制
#[derive(Debug, Clone)]
pub struct IoMax {
    /// 设备类型
    pub major: u32,
    /// 次设备号
    pub minor: u32,
    /// 读带宽（字节/秒）
    pub read_bps: Option<u64>,
    /// 写带宽（字节/秒）
    pub write_bps: Option<u64>,
    /// 读IOPS限制
    pub read_iops: Option<u64>,
    /// 写IOPS限制
    pub write_iops: Option<u64>,
}

/// 块设备配置
#[derive(Debug, Clone)]
pub struct BlkioConfig {
    /// 块设备权重
    pub weight: Option<u16>,
    /// 块设备权重设备
    pub weight_device: Option<Vec<BlkioWeightDevice>>,
    /// 块设备节流配置
    pub throttle: Option<Vec<BlkioThrottle>>,
}

/// 块设备权重设备
#[derive(Debug, Clone)]
pub struct BlkioWeightDevice {
    /// 主设备号
    pub major: u32,
    /// 次设备号
    pub minor: u32,
    /// 权重
    pub weight: u16,
}

/// 块设备节流配置
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
    /// 读IOPS限制
    pub read_iops: Option<u64>,
    /// 写IOPS限制
    pub write_iops: Option<u64>,
}

/// 进程ID配置
#[derive(Debug, Clone)]
pub struct PidsConfig {
    /// 最大进程数
    pub max: Option<i64>,
    /// 当前进程数
    pub current: Option<i64>,
}

/// cgroup统计信息
#[derive(Debug, Clone)]
pub struct CgroupStats {
    /// cgroup路径
    pub path: String,
    /// 子系统统计
    pub subsystem_stats: BTreeMap<CgroupSubsystem, CgroupSubsystemStats>,
}

/// 子系统统计信息
#[derive(Debug, Clone)]
pub struct CgroupSubsystemStats {
    /// 子系统名称
    pub name: String,
    /// 是否启用
    pub enabled: bool,
    /// 当前值
    pub current_value: Option<u64>,
    /// 最大值
    pub max_value: Option<u64>,
    /// 使用率
    pub usage_percent: Option<f64>,
}

/// cgroup
pub struct Cgroup {
    /// cgroup路径
    pub path: String,
    /// cgroup版本
    pub version: CgroupVersion,
    /// 子系统配置
    pub config: CgroupResourceConfig,
    /// 进程列表
    pub processes: Arc<Mutex<Vec<u32>>>,
    /// 统计信息
    pub stats: Arc<Mutex<CgroupStats>>,
    /// 是否激活
    pub active: bool,
}

impl Cgroup {
    /// 创建新的cgroup
    pub fn new(path: String, version: CgroupVersion, config: CgroupResourceConfig) -> Self {
        let mut subsystem_stats = BTreeMap::new();

        // 初始化子系统统计
        if config.cpu.is_some() {
            subsystem_stats.insert(CgroupSubsystem::Cpu, CgroupSubsystemStats {
                name: "cpu".to_string(),
                enabled: true,
                current_value: None,
                max_value: None,
                usage_percent: None,
            });
        }

        if config.memory.is_some() {
            subsystem_stats.insert(CgroupSubsystem::Memory, CgroupSubsystemStats {
                name: "memory".to_string(),
                enabled: true,
                current_value: None,
                max_value: None,
                usage_percent: None,
            });
        }

        if config.io.is_some() {
            subsystem_stats.insert(CgroupSubsystem::Io, CgroupSubsystemStats {
                name: "io".to_string(),
                enabled: true,
                current_value: None,
                max_value: None,
                usage_percent: None,
            });
        }

        if config.blkio.is_some() {
            subsystem_stats.insert(CgroupSubsystem::Blkio, CgroupSubsystemStats {
                name: "blkio".to_string(),
                enabled: true,
                current_value: None,
                max_value: None,
                usage_percent: None,
            });
        }

        if config.pids.is_some() {
            subsystem_stats.insert(CgroupSubsystem::Pids, CgroupSubsystemStats {
                name: "pids".to_string(),
                enabled: true,
                current_value: None,
                max_value: None,
                usage_percent: None,
            });
        }

        Self {
            path,
            version,
            config,
            processes: Arc::new(Mutex::new(Vec::new())),
            stats: Arc::new(Mutex::new(CgroupStats {
                path: String::new(),
                subsystem_stats,
            })),
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

        // 更新统计信息
        let mut stats = self.stats.lock();
        stats.path = self.path.clone();

        crate::println!("[cgroups] Created cgroup: {}", self.path);
        Ok(())
    }

    /// 添加进程到cgroup
    pub fn add_process(&self, pid: u32) -> Result<(), i32> {
        if !self.active {
            return Err(EINVAL);
        }

        // 将进程添加到所有启用的子系统
        self.add_process_to_subsystems(pid)?;

        // 更新进程列表
        {
            let mut processes = self.processes.lock();
            if !processes.contains(&pid) {
                processes.push(pid);
            }
        }

        crate::println!("[cgroups] Added process {} to cgroup: {}", pid, self.path);
        Ok(())
    }

    /// 从cgroup移除进程
    pub fn remove_process(&self, pid: u32) -> Result<(), i32> {
        if !self.active {
            return Err(EINVAL);
        }

        // 从子系统移除进程
        self.remove_process_from_subsystems(pid)?;

        // 更新进程列表
        {
            let mut processes = self.processes.lock();
            processes.retain(|&p| p != pid);
        }

        crate::println!("[cgroups] Removed process {} from cgroup: {}", pid, self.path);
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

        // 清理子系统
        self.cleanup_subsystems()?;

        // 删除cgroup目录
        self.remove_cgroup_directory()?;

        self.active = false;

        crate::println!("[cgroups] Destroyed cgroup: {}", self.path);
        Ok(())
    }

    /// 创建cgroup目录
    fn create_cgroup_directory(&self) -> Result<(), i32> {
        match self.version {
            CgroupVersion::V1 => {
                // 创建v1 cgroup目录
                self.create_v1_cgroup_directory()
            }
            CgroupVersion::V2 => {
                // 创建v2 cgroup目录
                self.create_v2_cgroup_directory()
            }
        }
    }

    /// 创建v1 cgroup目录
    fn create_v1_cgroup_directory(&self) -> Result<(), i32> {
        // 为每个启用的子系统创建目录
        if self.config.cpu.is_some() {
            self.create_subsystem_directory("cpu")?;
        }
        if self.config.memory.is_some() {
            self.create_subsystem_directory("memory")?;
        }
        if self.config.io.is_some() {
            self.create_subsystem_directory("blkio")?; // v1中使用blkio
        }
        if self.config.blkio.is_some() {
            self.create_subsystem_directory("blkio")?;
        }
        if self.config.pids.is_some() {
            self.create_subsystem_directory("pids")?;
        }

        Ok(())
    }

    /// 创建v2 cgroup目录
    fn create_v2_cgroup_directory(&self) -> Result<(), i32> {
        // 创建统一的cgroup目录
        crate::println!("[cgroups] Creating unified cgroup directory: {}", self.path);
        // 在实际实现中，这里会创建文件系统目录
        Ok(())
    }

    /// 创建子系统目录
    fn create_subsystem_directory(&self, subsystem: &str) -> Result<(), i32> {
        let path = format!("/sys/fs/cgroup/{}/{}", subsystem, self.path);
        crate::println!("[cgroups] Creating subsystem directory: {}", path);
        // 在实际实现中，这里会创建文件系统目录
        Ok(())
    }

    /// 配置子系统
    fn configure_subsystems(&self) -> Result<(), i32> {
        // 配置CPU子系统
        if let Some(ref cpu_config) = self.config.cpu {
            self.configure_cpu_subsystem(cpu_config)?;
        }

        // 配置内存子系统
        if let Some(ref memory_config) = self.config.memory {
            self.configure_memory_subsystem(memory_config)?;
        }

        // 配置I/O子系统
        if let Some(ref io_config) = self.config.io {
            self.configure_io_subsystem(io_config)?;
        }

        // 配置块设备子系统
        if let Some(ref blkio_config) = self.config.blkio {
            self.configure_blkio_subsystem(blkio_config)?;
        }

        // 配置进程ID子系统
        if let Some(ref pids_config) = self.config.pids {
            self.configure_pids_subsystem(pids_config)?;
        }

        Ok(())
    }

    /// 配置CPU子系统
    fn configure_cpu_subsystem(&self, config: &CpuConfig) -> Result<(), i32> {
        match self.version {
            CgroupVersion::V1 => {
                // 配置v1 CPU子系统
                if let Some(quota) = config.quota {
                    self.write_cgroup_file("cpu", "cpu.cfs_quota_us", &quota.to_string())?;
                }
                if let Some(period) = config.period {
                    self.write_cgroup_file("cpu", "cpu.cfs_period_us", &period.to_string())?;
                }
                if let Some(shares) = config.shares {
                    self.write_cgroup_file("cpu", "cpu.shares", &shares.to_string())?;
                }
                if let Some(ref cpus) = config.cpus {
                    self.write_cgroup_file("cpuset", "cpuset.cpus", cpus)?;
                }
            }
            CgroupVersion::V2 => {
                // 配置v2 CPU子系统
                if let Some(quota) = config.quota {
                    self.write_cgroup_file("", "cpu.max", &format!("{} {}", quota, config.period.unwrap_or(1000000)))?;
                }
                if let Some(shares) = config.shares {
                    self.write_cgroup_file("", "cpu.weight", &shares.to_string())?;
                }
            }
        }
        Ok(())
    }

    /// 配置内存子系统
    fn configure_memory_subsystem(&self, config: &MemoryConfig) -> Result<(), i32> {
        match self.version {
            CgroupVersion::V1 => {
                if let Some(limit) = config.limit {
                    self.write_cgroup_file("memory", "memory.limit_in_bytes", &limit.to_string())?;
                }
                if let Some(swap_limit) = config.swap_limit {
                    self.write_cgroup_file("memory", "memory.memsw.limit_in_bytes", &swap_limit.to_string())?;
                }
                if config.oom_control.oom_kill_disable {
                    self.write_cgroup_file("memory", "memory.oom_control", "1")?;
                }
            }
            CgroupVersion::V2 => {
                if let Some(limit) = config.limit {
                    self.write_cgroup_file("", "memory.max", &limit.to_string())?;
                }
                if config.oom_control.oom_kill_disable {
                    self.write_cgroup_file("", "memory.oom.group", "1")?;
                }
            }
        }
        Ok(())
    }

    /// 配置I/O子系统
    fn configure_io_subsystem(&self, config: &IoConfig) -> Result<(), i32> {
        match self.version {
            CgroupVersion::V1 => {
                // v1中使用blkio子系统
                if let Some(weight) = config.weight {
                    self.write_cgroup_file("blkio", "blkio.weight", &weight.to_string())?;
                }
                if let Some(ref max_config) = config.max {
                    for io_max in max_config {
                        let config_str = format!("{}:{} {} {} {} {}",
                            io_max.major, io_max.minor,
                            io_max.read_bps.unwrap_or(0),
                            io_max.write_bps.unwrap_or(0),
                            io_max.read_iops.unwrap_or(0),
                            io_max.write_iops.unwrap_or(0));
                        self.write_cgroup_file("blkio", "blkio.throttle.read_bps_device", &config_str)?;
                    }
                }
            }
            CgroupVersion::V2 => {
                if let Some(weight) = config.weight {
                    self.write_cgroup_file("", "io.weight", &format!("default {}", weight))?;
                }
                if let Some(ref max_config) = config.max {
                    for io_max in max_config {
                        let config_str = format!("{}:{} rbps={} wbps={} riops={} wiops={}",
                            io_max.major, io_max.minor,
                            io_max.read_bps.unwrap_or(0),
                            io_max.write_bps.unwrap_or(0),
                            io_max.read_iops.unwrap_or(0),
                            io_max.write_iops.unwrap_or(0));
                        self.write_cgroup_file("", "io.max", &config_str)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// 配置块设备子系统
    fn configure_blkio_subsystem(&self, config: &BlkioConfig) -> Result<(), i32> {
        if let Some(weight) = config.weight {
            self.write_cgroup_file("blkio", "blkio.weight", &weight.to_string())?;
        }
        if let Some(ref weight_devices) = config.weight_device {
            for weight_device in weight_devices {
                let config_str = format!("{}:{} {}", weight_device.major, weight_device.minor, weight_device.weight);
                self.write_cgroup_file("blkio", "blkio.weight_device", &config_str)?;
            }
        }
        if let Some(ref throttle) = config.throttle {
            for throttle_config in throttle {
                let config_str = format!("{}:{} {} {}",
                    throttle_config.major, throttle_config.minor,
                    throttle_config.read_bps.unwrap_or(0),
                    throttle_config.write_bps.unwrap_or(0));
                self.write_cgroup_file("blkio", "blkio.throttle.read_bps_device", &config_str)?;
            }
        }
        Ok(())
    }

    /// 配置进程ID子系统
    fn configure_pids_subsystem(&self, config: &PidsConfig) -> Result<(), i32> {
        if let Some(max) = config.max {
            self.write_cgroup_file("pids", "pids.max", &max.to_string())?;
        }
        Ok(())
    }

    /// 写入cgroup文件
    fn write_cgroup_file(&self, subsystem: &str, filename: &str, value: &str) -> Result<(), i32> {
        let path = if subsystem.is_empty() {
            // v2统一层次结构
            format!("/sys/fs/cgroup/{}/{}", self.path, filename)
        } else {
            // v1子系统层次结构
            format!("/sys/fs/cgroup/{}/{}/{}", subsystem, self.path, filename)
        };

        crate::println!("[cgroups] Writing '{}' to {}", value, path);
        // 在实际实现中，这里会写入文件系统
        Ok(())
    }

    /// 将进程添加到子系统
    fn add_process_to_subsystems(&self, pid: u32) -> Result<(), i32> {
        match self.version {
            CgroupVersion::V1 => {
                // v1：为每个启用的子系统添加进程
                if self.config.cpu.is_some() {
                    self.write_cgroup_file("cpu", "tasks", &pid.to_string())?;
                }
                if self.config.memory.is_some() {
                    self.write_cgroup_file("memory", "tasks", &pid.to_string())?;
                }
                if self.config.io.is_some() || self.config.blkio.is_some() {
                    self.write_cgroup_file("blkio", "tasks", &pid.to_string())?;
                }
                if self.config.pids.is_some() {
                    self.write_cgroup_file("pids", "tasks", &pid.to_string())?;
                }
            }
            CgroupVersion::V2 => {
                // v2：统一进程文件
                self.write_cgroup_file("", "cgroup.procs", &pid.to_string())?;
            }
        }
        Ok(())
    }

    /// 从子系统移除进程
    fn remove_process_from_subsystems(&self, pid: u32) -> Result<(), i32> {
        // 将进程移到父cgroup
        match self.version {
            CgroupVersion::V1 => {
                // v1：移除进程到根cgroup
                self.write_cgroup_file("cpu", "tasks", &pid.to_string())?;
                self.write_cgroup_file("memory", "tasks", &pid.to_string())?;
                self.write_cgroup_file("blkio", "tasks", &pid.to_string())?;
                self.write_cgroup_file("pids", "tasks", &pid.to_string())?;
            }
            CgroupVersion::V2 => {
                // v2：移除进程到父cgroup
                self.write_cgroup_file("", "cgroup.procs", &pid.to_string())?;
            }
        }
        Ok(())
    }

    /// 清理子系统
    fn cleanup_subsystems(&self) -> Result<(), i32> {
        crate::println!("[cgroups] Cleaning up subsystems for cgroup: {}", self.path);
        // 在实际实现中，这里会清理配置文件
        Ok(())
    }

    /// 删除cgroup目录
    fn remove_cgroup_directory(&self) -> Result<(), i32> {
        match self.version {
            CgroupVersion::V1 => {
                // 删除v1子系统目录
                if self.config.cpu.is_some() {
                    self.remove_subsystem_directory("cpu")?;
                }
                if self.config.memory.is_some() {
                    self.remove_subsystem_directory("memory")?;
                }
                if self.config.io.is_some() || self.config.blkio.is_some() {
                    self.remove_subsystem_directory("blkio")?;
                }
                if self.config.pids.is_some() {
                    self.remove_subsystem_directory("pids")?;
                }
            }
            CgroupVersion::V2 => {
                // 删除v2统一目录
                crate::println!("[cgroups] Removing unified cgroup directory: {}", self.path);
            }
        }
        Ok(())
    }

    /// 删除子系统目录
    fn remove_subsystem_directory(&self, subsystem: &str) -> Result<(), i32> {
        let path = format!("/sys/fs/cgroup/{}/{}", subsystem, self.path);
        crate::println!("[cgroups] Removing subsystem directory: {}", path);
        // 在实际实现中，这里会删除文件系统目录
        Ok(())
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> CgroupStats {
        let mut stats = self.stats.lock();
        self.update_subsystem_stats(&mut stats.subsystem_stats);
        stats.clone()
    }

    /// 更新子系统统计信息
    fn update_subsystem_stats(&self, subsystem_stats: &mut BTreeMap<CgroupSubsystem, CgroupSubsystemStats>) {
        // 更新CPU统计
        if let Some(stats) = subsystem_stats.get_mut(&CgroupSubsystem::Cpu) {
            stats.current_value = Some(self.get_cpu_usage());
            stats.max_value = self.config.cpu.as_ref().and_then(|c| c.quota.map(|q| q as u64));
            if let (Some(current), Some(max)) = (stats.current_value, stats.max_value) {
                stats.usage_percent = Some((current as f64 / max as f64) * 100.0);
            }
        }

        // 更新内存统计
        if let Some(stats) = subsystem_stats.get_mut(&CgroupSubsystem::Memory) {
            stats.current_value = Some(self.get_memory_usage());
            stats.max_value = self.config.memory.as_ref().and_then(|c| c.limit);
            if let (Some(current), Some(max)) = (stats.current_value, stats.max_value) {
                stats.usage_percent = Some((current as f64 / max as f64) * 100.0);
            }
        }

        // 更新进程数统计
        if let Some(stats) = subsystem_stats.get_mut(&CgroupSubsystem::Pids) {
            let current_process_count = self.processes.lock().len() as u64;
            stats.current_value = Some(current_process_count);
            stats.max_value = self.config.pids.as_ref().and_then(|c| c.max.map(|m| m as u64));
            if let (Some(current), Some(max)) = (stats.current_value, stats.max_value) {
                stats.usage_percent = Some((current as f64 / max as f64) * 100.0);
            }
        }
    }

    /// 获取CPU使用量
    fn get_cpu_usage(&self) -> u64 {
        // 在实际实现中，这里会读取cgroup的CPU统计文件
        self.processes.lock().len() as u64 * 1000000 // 简化实现
    }

    /// 获取内存使用量
    fn get_memory_usage(&self) -> u64 {
        // 在实际实现中，这里会读取cgroup的内存统计文件
        self.processes.lock().len() as u64 * 1024 * 1024 // 简化实现
    }
}

/// cgroup管理器
pub struct CgroupManager {
    /// cgroup列表
    cgroups: BTreeMap<String, Arc<Mutex<Cgroup>>>,
    /// cgroup版本
    version: CgroupVersion,
    /// 根cgroup路径
    root_path: String,
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
        }
    }

    /// 创建cgroup
    pub fn create_cgroup(&mut self, name: &str, config: CgroupResourceConfig) -> Result<Arc<Mutex<Cgroup>>, i32> {
        let path = format!("{}/{}", self.root_path, name);
        let mut cgroup = Cgroup::new(path.clone(), self.version, config);

        cgroup.create()?;

        let cgroup_arc = Arc::new(Mutex::new(cgroup));
        self.cgroups.insert(name.to_string(), cgroup_arc.clone());

        crate::println!("[cgroups] Created cgroup: {}", name);
        Ok(cgroup_arc)
    }

    /// 获取cgroup
    pub fn get_cgroup(&self, name: &str) -> Option<Arc<Mutex<Cgroup>>> {
        self.cgroups.get(name).cloned()
    }

    /// 删除cgroup
    pub fn delete_cgroup(&mut self, name: &str) -> Result<(), i32> {
        if let Some(cgroup) = self.cgroups.remove(name) {
            let mut cg = cgroup.lock();
            cg.destroy()?;
            crate::println!("[cgroups] Deleted cgroup: {}", name);
            Ok(())
        } else {
            Err(ENOENT)
        }
    }

    /// 获取所有cgroup名称
    pub fn get_cgroup_names(&self) -> Vec<String> {
        self.cgroups.keys().cloned().collect()
    }

    /// 获取所有cgroup统计信息
    pub fn get_all_stats(&self) -> Vec<CgroupStats> {
        self.cgroups.values()
            .map(|cgroup| {
                let cg = cgroup.lock();
                cg.get_stats()
            })
            .collect()
    }
}

/// 全局cgroup管理器实例
static mut CGROUP_MANAGER_V1: Option<CgroupManager> = None;
static mut CGROUP_MANAGER_V2: Option<CgroupManager> = None;
static mut CGROUP_INITIALIZED: bool = false;

/// 初始化cgroups
pub fn initialize_cgroups() -> Result<(), i32> {
    if unsafe { CGROUP_INITIALIZED } {
        return Ok(());
    }

    // 创建v1和v2管理器
    let v1_manager = CgroupManager::new(CgroupVersion::V1);
    let v2_manager = CgroupManager::new(CgroupVersion::V2);

    unsafe {
        CGROUP_MANAGER_V1 = Some(v1_manager);
        CGROUP_MANAGER_V2 = Some(v2_manager);
        CGROUP_INITIALIZED = true;
    }

    crate::println!("[cgroups] Control groups initialized");
    Ok(())
}

/// 获取v1 cgroup管理器
pub fn get_v1_cgroup_manager() -> Option<&'static CgroupManager> {
    unsafe {
        CGROUP_MANAGER_V1.as_ref()
    }
}

/// 获取v1 cgroup管理器（可变）
pub fn get_v1_cgroup_manager_mut() -> Option<&'static mut CgroupManager> {
    unsafe {
        CGROUP_MANAGER_V1.as_mut()
    }
}

/// 获取v2 cgroup管理器
pub fn get_v2_cgroup_manager() -> Option<&'static CgroupManager> {
    unsafe {
        CGROUP_MANAGER_V2.as_ref()
    }
}

/// 获取v2 cgroup管理器（可变）
pub fn get_v2_cgroup_manager_mut() -> Option<&'static mut CgroupManager> {
    unsafe {
        CGROUP_MANAGER_V2.as_mut()
    }
}

/// 设置内存限制
pub fn set_memory_limit(limit: u64) -> Result<(), i32> {
    let manager = get_v1_cgroup_manager_mut().ok_or(EIO)?;
    let config = CgroupResourceConfig {
        cpu: None,
        memory: Some(MemoryConfig {
            limit: Some(limit),
            soft_limit: None,
            swap_limit: None,
            kernel_limit: None,
            kernel_tcp_limit: None,
            max_usage: None,
            oom_control: OomControl::default(),
        }),
        io: None,
        blkio: None,
        pids: None,
    };

    let _cgroup = manager.create_cgroup("memory-limit", config)?;
    Ok(())
}

/// 设置CPU配额
pub fn set_cpu_quota(quota: i64) -> Result<(), i32> {
    let manager = get_v1_cgroup_manager_mut().ok_or(EIO)?;
    let config = CgroupResourceConfig {
        cpu: Some(CpuConfig {
            quota: Some(quota),
            period: Some(1000000),
            shares: None,
            cpus: None,
            mems: None,
            rt_runtime: None,
            rt_period: None,
        }),
        memory: None,
        io: None,
        blkio: None,
        pids: None,
    };

    let _cgroup = manager.create_cgroup("cpu-quota", config)?;
    Ok(())
}

/// 设置CPU份额
pub fn set_cpu_shares(shares: u64) -> Result<(), i32> {
    let manager = get_v1_cgroup_manager_mut().ok_or(EIO)?;
    let config = CgroupResourceConfig {
        cpu: Some(CpuConfig {
            quota: None,
            period: None,
            shares: Some(shares),
            cpus: None,
            mems: None,
            rt_runtime: None,
            rt_period: None,
        }),
        memory: None,
        io: None,
        blkio: None,
        pids: None,
    };

    let _cgroup = manager.create_cgroup("cpu-shares", config)?;
    Ok(())
}

/// 为特定进程设置内存限制
pub fn set_memory_limit_for_process(pid: u32, limit: u64) -> Result<(), i32> {
    let manager = get_v1_cgroup_manager_mut().ok_or(EIO)?;
    let cgroup_name = format!("process-{}", pid);

    let config = CgroupResourceConfig {
        cpu: None,
        memory: Some(MemoryConfig {
            limit: Some(limit),
            soft_limit: None,
            swap_limit: None,
            kernel_limit: None,
            kernel_tcp_limit: None,
            max_usage: None,
            oom_control: OomControl::default(),
        }),
        io: None,
        blkio: None,
        pids: None,
    };

    let cgroup = manager.create_cgroup(&cgroup_name, config)?;
    {
        let cg = cgroup.lock();
        cg.add_process(pid)?;
    }

    Ok(())
}

/// 为特定进程设置CPU限制
pub fn set_cpu_limit_for_process(pid: u32, limit: f64) -> Result<(), i32> {
    let manager = get_v1_cgroup_manager_mut().ok_or(EIO)?;
    let cgroup_name = format!("process-{}", pid);

    let config = CgroupResourceConfig {
        cpu: Some(CpuConfig {
            quota: Some((limit * 1000000.0) as i64),
            period: Some(1000000),
            shares: None,
            cpus: None,
            mems: None,
            rt_runtime: None,
            rt_period: None,
        }),
        memory: None,
        io: None,
        blkio: None,
        pids: None,
    };

    let cgroup = manager.create_cgroup(&cgroup_name, config)?;
    {
        let cg = cgroup.lock();
        cg.add_process(pid)?;
    }

    Ok(())
}

/// 为特定进程设置磁盘限制
pub fn set_disk_limit_for_process(pid: u32, limit: u64) -> Result<(), i32> {
    let manager = get_v1_cgroup_manager_mut().ok_or(EIO)?;
    let cgroup_name = format!("process-{}", pid);

    let config = CgroupResourceConfig {
        cpu: None,
        memory: None,
        io: Some(IoConfig {
            weight: None,
            max: Some(vec![
                IoMax {
                    major: 8, // 假设是sda设备
                    minor: 0,
                    read_bps: Some(limit),
                    write_bps: Some(limit),
                    read_iops: None,
                    write_iops: None,
                }
            ]),
        }),
        blkio: None,
        pids: None,
    };

    let cgroup = manager.create_cgroup(&cgroup_name, config)?;
    {
        let cg = cgroup.lock();
        cg.add_process(pid)?;
    }

    Ok(())
}

/// 清理容器cgroups
pub fn cleanup_container_cgroups(container_name: &str) -> Result<(), i32> {
    let v1_manager = get_v1_cgroup_manager_mut().ok_or(EIO)?;

    // 清理可能的cgroup名称
    let possible_names = vec![
        format!("container-{}", container_name),
        format!("process-container-{}", container_name),
    ];

    for name in possible_names {
        if v1_manager.get_cgroup(&name).is_some() {
            v1_manager.delete_cgroup(&name)?;
        }
    }

    Ok(())
}

/// 清理所有cgroups
pub fn cleanup_cgroups() -> Result<(), i32> {
    let v1_manager = get_v1_cgroup_manager_mut().ok_or(EIO)?;
    let v2_manager = get_v2_cgroup_manager_mut().ok_or(EIO)?;

    // 获取所有cgroup名称
    let v1_names: Vec<String> = v1_manager.get_cgroup_names();
    let v2_names: Vec<String> = v2_manager.get_cgroup_names();

    // 删除所有v1 cgroups
    for name in v1_names {
        let _ = v1_manager.delete_cgroup(&name);
    }

    // 删除所有v2 cgroups
    for name in v2_names {
        let _ = v2_manager.delete_cgroup(&name);
    }

    crate::println!("[cgroups] Cleaned up all control groups");
    Ok(())
}