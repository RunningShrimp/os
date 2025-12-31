// Cgroup v2 Support
//
// Cgroup v2支持模块
// 提供统一的资源控制和限制功能

extern crate alloc;

use alloc::{
    collections::BTreeMap,
    format,
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::{AtomicU64, Ordering};

use spin::Mutex;

use crate::reliability::{EINVAL, EIO, ENOENT, ENOMEM};

/// Cgroup v2根目录
pub const CGROUP_V2_ROOT: &str = "/sys/fs/cgroup";

/// Cgroup v2控制器
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CgroupController {
    /// CPU控制器
    Cpu,
    /// 内存控制器
    Memory,
    /// IO控制器
    Io,
    /// PID控制器
    Pid,
    /// CPU集合控制器
    Cpuset,
    /// 冻结控制器
    Freeze,
    /// RDMA控制器
    Rdma,
    /// HugeTLB控制器
    Hugetlb,
}

impl CgroupController {
    /// 获取控制器名称
    pub fn name(&self) -> &str {
        match self {
            Self::Cpu => "cpu",
            Self::Memory => "memory",
            Self::Io => "io",
            Self::Pid => "pid",
            Self::Cpuset => "cpuset",
            Self::Freeze => "freeze",
            Self::Rdma => "rdma",
            Self::Hugetlb => "hugetlb",
        }
    }
}

/// Cgroup v2
#[derive(Debug, Clone)]
pub struct Cgroup {
    /// 名称
    pub name: String,
    /// 路径
    pub path: String,
    /// 父cgroup
    pub parent: Option<String>,
    /// 子cgroup
    pub children: Vec<String>,
    /// 控制器
    pub controllers: Vec<CgroupController>,
    /// 进程列表
    pub processes: Vec<u32>,
    /// 是否激活
    pub active: bool,
}

impl Cgroup {
    /// 创建新的cgroup
    pub fn new(name: String, path: String) -> Self {
        Self {
            name,
            path,
            parent: None,
            children: Vec::new(),
            controllers: Vec::new(),
            processes: Vec::new(),
            active: false,
        }
    }

    /// 激活cgroup
    pub fn activate(&mut self) -> Result<(), i32> {
        if self.active {
            return Err(EINVAL);
        }
        self.active = true;
        Ok(())
    }

    /// 添加控制器
    pub fn add_controller(&mut self, controller: CgroupController) {
        if !self.controllers.contains(&controller) {
            self.controllers.push(controller);
        }
    }

    /// 添加进程
    pub fn add_process(&mut self, pid: u32) {
        if !self.processes.contains(&pid) {
            self.processes.push(pid);
        }
    }

    /// 移除进程
    pub fn remove_process(&mut self, pid: u32) {
        self.processes.retain(|&p| p != pid);
    }

    /// 添加子cgroup
    pub fn add_child(&mut self, child_name: String) {
        if !self.children.contains(&child_name) {
            self.children.push(child_name);
        }
    }

    /// 获取进程数量
    pub fn process_count(&self) -> usize {
        self.processes.len()
    }
}

/// Cgroup v2管理器
pub struct CgroupV2Manager {
    /// cgroup树
    cgroups: BTreeMap<String, Cgroup>,
    /// 下一个cgroup ID
    next_id: AtomicU64,
}

impl CgroupV2Manager {
    /// 创建新的Cgroup v2管理器
    pub fn new() -> Self {
        let mut cgroups = BTreeMap::new();

        // 创建根cgroup
        let root_cgroup = Cgroup::new(
            "root".to_string(),
            CGROUP_V2_ROOT.to_string(),
        );
        cgroups.insert("/".to_string(), root_cgroup);

        Self {
            cgroups,
            next_id: AtomicU64::new(1),
        }
    }

    /// 创建cgroup
    pub fn create(&mut self, name: &str, parent: Option<&str>) -> Result<String, i32> {
        let parent_path = parent.unwrap_or("/");
        let path = if parent_path == "/" {
            format!("{}/{}", CGROUP_V2_ROOT, name)
        } else {
            format!("{}/{}", parent_path, name)
        };

        // 验证父cgroup存在
        if !self.cgroups.contains_key(parent_path) {
            return Err(ENOENT);
        }

        let mut cgroup = Cgroup::new(name.to_string(), path.clone());
        cgroup.parent = Some(parent_path.to_string());
        cgroup.activate()?;

        // 更新父cgroup的子列表
        if let Some(parent_cgroup) = self.cgroups.get_mut(parent_path) {
            parent_cgroup.add_child(name.to_string());
        }

        self.cgroups.insert(path.clone(), cgroup);

        crate::println!("[cgroup-v2] Created cgroup: {}", path);
        Ok(path)
    }

    /// 删除cgroup
    pub fn delete(&mut self, path: &str) -> Result<(), i32> {
        // 获取父路径以避免借用冲突
        let parent_path = {
            let cgroup = self.cgroups.get(path).ok_or(ENOENT)?;

            // 检查是否有子cgroup
            if !cgroup.children.is_empty() {
                return Err(EINVAL);
            }

            // 检查是否有进程
            if !cgroup.processes.is_empty() {
                return Err(EINVAL);
            }

            cgroup.parent.clone()
        };

        // 从父cgroup的子列表中移除
        if let Some(ref parent_path) = parent_path {
            if let Some(parent_cgroup) = self.cgroups.get_mut(parent_path) {
                parent_cgroup.children.retain(|c| c != path);
            }
        }

        self.cgroups.remove(path);

        crate::println!("[cgroup-v2] Deleted cgroup: {}", path);
        Ok(())
    }

    /// 添加进程到cgroup
    pub fn add_process(&mut self, path: &str, pid: u32) -> Result<(), i32> {
        let cgroup = self.cgroups.get_mut(path).ok_or(ENOENT)?;
        cgroup.add_process(pid);
        crate::println!("[cgroup-v2] Added process {} to cgroup {}", pid, path);
        Ok(())
    }

    /// 从cgroup移除进程
    pub fn remove_process(&mut self, path: &str, pid: u32) -> Result<(), i32> {
        let cgroup = self.cgroups.get_mut(path).ok_or(ENOENT)?;
        cgroup.remove_process(pid);
        crate::println!("[cgroup-v2] Removed process {} from cgroup {}", pid, path);
        Ok(())
    }

    /// 设置内存限制
    pub fn set_memory_limit(&mut self, path: &str, limit: u64) -> Result<(), i32> {
        let _cgroup = self.cgroups.get_mut(path).ok_or(ENOENT)?;

        // 写入memory.max文件
        // 在实际实现中，这里会写入/sys/fs/cgroup/{path}/memory.max
        crate::println!("[cgroup-v2] Set memory limit {} for cgroup {}", limit, path);
        Ok(())
    }

    /// 设置CPU份额（权重）
    pub fn set_cpu_weight(&mut self, path: &str, weight: u64) -> Result<(), i32> {
        let _cgroup = self.cgroups.get_mut(path).ok_or(ENOENT)?;

        // 写入cpu.weight文件
        // 在实际实现中，这里会写入/sys/fs/cgroup/{path}/cpu.weight
        crate::println!("[cgroup-v2] Set cpu weight {} for cgroup {}", weight, path);
        Ok(())
    }

    /// 设置CPU配额
    pub fn set_cpu_max(&mut self, path: &str, quota_us: u64, period_us: u64) -> Result<(), i32> {
        let _cgroup = self.cgroups.get_mut(path).ok_or(ENOENT)?;

        // 写入cpu.max文件：格式为"$quota $period"
        // 在实际实现中，这里会写入/sys/fs/cgroup/{path}/cpu.max
        crate::println!(
            "[cgroup-v2] Set cpu max {}/{} for cgroup {}",
            quota_us,
            period_us,
            path
        );
        Ok(())
    }

    /// 设置IO限制
    pub fn set_io_max(&mut self, path: &str, rbps: u64, wbps: u64) -> Result<(), i32> {
        let _cgroup = self.cgroups.get_mut(path).ok_or(ENOENT)?;

        // 写入io.max文件
        // 在实际实现中，这里会写入/sys/fs/cgroup/{path}/io.max
        crate::println!(
            "[cgroup-v2] Set io max read:{} write:{} for cgroup {}",
            rbps,
            wbps,
            path
        );
        Ok(())
    }

    /// 设置PID最大值
    pub fn set_pid_max(&mut self, path: &str, max: u64) -> Result<(), i32> {
        let _cgroup = self.cgroups.get_mut(path).ok_or(ENOENT)?;

        // 写入pids.max文件
        // 在实际实现中，这里会写入/sys/fs/cgroup/{path}/pids.max
        crate::println!("[cgroup-v2] Set pid max {} for cgroup {}", max, path);
        Ok(())
    }

    /// 冻结cgroup中的所有进程
    pub fn freeze(&mut self, path: &str) -> Result<(), i32> {
        let _cgroup = self.cgroups.get_mut(path).ok_or(ENOENT)?;

        // 写入1到cgroup.freeze文件
        // 在实际实现中，这里会写入/sys/fs/cgroup/{path}/cgroup.freeze
        crate::println!("[cgroup-v2] Froze cgroup {}", path);
        Ok(())
    }

    /// 解冻cgroup中的所有进程
    pub fn thaw(&mut self, path: &str) -> Result<(), i32> {
        let _cgroup = self.cgroups.get_mut(path).ok_or(ENOENT)?;

        // 写入0到cgroup.freeze文件
        // 在实际实现中，这里会写入/sys/fs/cgroup/{path}/cgroup.freeze
        crate::println!("[cgroup-v2] Thawed cgroup {}", path);
        Ok(())
    }

    /// 获取cgroup统计信息
    pub fn get_stats(&self, path: &str) -> Result<CgroupStats, i32> {
        let cgroup = self.cgroups.get(path).ok_or(ENOENT)?;

        // 在实际实现中，这里会读取cgroup.stat等文件
        Ok(CgroupStats {
            memory_usage: 0,
            memory_limit: 0,
            cpu_usage: 0,
            cpu_weight: 0,
            process_count: cgroup.process_count(),
            child_count: cgroup.children.len(),
        })
    }

    /// 获取cgroup
    pub fn get_cgroup(&self, path: &str) -> Option<&Cgroup> {
        self.cgroups.get(path)
    }

    /// 获取cgroup可变引用
    pub fn get_cgroup_mut(&mut self, path: &str) -> Option<&mut Cgroup> {
        self.cgroups.get_mut(path)
    }

    /// 列出所有cgroup路径
    pub fn list_cgroups(&self) -> Vec<String> {
        self.cgroups.keys().cloned().collect()
    }

    /// 获取子cgroup
    pub fn get_children(&self, path: &str) -> Vec<String> {
        if let Some(cgroup) = self.cgroups.get(path) {
            cgroup.children.clone()
        } else {
            Vec::new()
        }
    }
}

impl Default for CgroupV2Manager {
    fn default() -> Self {
        Self::new()
    }
}

/// Cgroup统计信息
#[derive(Debug, Clone)]
pub struct CgroupStats {
    /// 内存使用量（字节）
    pub memory_usage: u64,
    /// 内存限制（字节）
    pub memory_limit: u64,
    /// CPU使用量（纳秒）
    pub cpu_usage: u64,
    /// CPU权重
    pub cpu_weight: u64,
    /// 进程数量
    pub process_count: usize,
    /// 子cgroup数量
    pub child_count: usize,
}

/// 全局Cgroup v2管理器
static mut CGROUP_V2_MANAGER: Option<CgroupV2Manager> = None;
static mut CGROUP_V2_MANAGER_INITIALIZED: bool = false;

/// 初始化Cgroup v2管理器
pub fn init_cgroup_v2() -> Result<(), i32> {
    if unsafe { CGROUP_V2_MANAGER_INITIALIZED } {
        return Ok(());
    }

    let manager = CgroupV2Manager::new();

    unsafe {
        CGROUP_V2_MANAGER = Some(manager);
        CGROUP_V2_MANAGER_INITIALIZED = true;
    }

    crate::println!("[cgroup-v2] Initialized");
    Ok(())
}

/// 获取Cgroup v2管理器
pub fn get_cgroup_v2_manager() -> Option<&'static mut CgroupV2Manager> {
    unsafe { CGROUP_V2_MANAGER.as_mut() }
}

/// 创建cgroup
pub fn create_cgroup(name: &str, parent: Option<&str>) -> Result<String, i32> {
    let manager = get_cgroup_v2_manager().ok_or(EIO)?;
    manager.create(name, parent)
}

/// 删除cgroup
pub fn delete_cgroup(path: &str) -> Result<(), i32> {
    let manager = get_cgroup_v2_manager().ok_or(EIO)?;
    manager.delete(path)
}

/// 添加进程到cgroup
pub fn add_process_to_cgroup(path: &str, pid: u32) -> Result<(), i32> {
    let manager = get_cgroup_v2_manager().ok_or(EIO)?;
    manager.add_process(path, pid)
}

/// 设置内存限制
pub fn set_memory_limit(path: &str, limit: u64) -> Result<(), i32> {
    let manager = get_cgroup_v2_manager().ok_or(EIO)?;
    manager.set_memory_limit(path, limit)
}

/// 设置CPU权重
pub fn set_cpu_weight(path: &str, weight: u64) -> Result<(), i32> {
    let manager = get_cgroup_v2_manager().ok_or(EIO)?;
    manager.set_cpu_weight(path, weight)
}

/// 设置CPU配额
pub fn set_cpu_max(path: &str, quota_us: u64, period_us: u64) -> Result<(), i32> {
    let manager = get_cgroup_v2_manager().ok_or(EIO)?;
    manager.set_cpu_max(path, quota_us, period_us)
}

/// 冻结cgroup
pub fn freeze_cgroup(path: &str) -> Result<(), i32> {
    let manager = get_cgroup_v2_manager().ok_or(EIO)?;
    manager.freeze(path)
}

/// 解冻cgroup
pub fn thaw_cgroup(path: &str) -> Result<(), i32> {
    let manager = get_cgroup_v2_manager().ok_or(EIO)?;
    manager.thaw(path)
}

/// 获取cgroup统计信息
pub fn get_cgroup_stats(path: &str) -> Result<CgroupStats, i32> {
    let manager = get_cgroup_v2_manager().ok_or(EIO)?;
    manager.get_stats(path)
}
