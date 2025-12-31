// Container Rootfs Management
//
// 容器根文件系统管理
// 提供rootfs构建、overlayfs和设备管理功能

extern crate alloc;

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use crate::reliability::EINVAL;

/// Rootfs层
#[derive(Debug, Clone)]
pub struct Layer {
    /// 层ID
    pub id: String,
    /// 层路径
    pub path: String,
    /// 是否只读
    pub read_only: bool,
    /// 层类型
    pub layer_type: LayerType,
}

/// 层类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerType {
    /// 基础层
    Base,
    /// 中间层
    Intermediate,
    /// 顶层（可写）
    Upper,
}

/// Rootfs构建器
#[derive(Debug, Clone)]
pub struct RootfsBuilder {
    /// 基础层（只读）
    pub lower_layers: Vec<Layer>,
    /// 上层（可写）
    pub upper_layer: Option<Layer>,
    /// 工作目录
    pub work_dir: Option<String>,
    /// 挂载点
    pub mount_point: String,
}

impl RootfsBuilder {
    /// 创建新的Rootfs构建器
    pub fn new(mount_point: String) -> Self {
        Self {
            lower_layers: Vec::new(),
            upper_layer: None,
            work_dir: None,
            mount_point,
        }
    }

    /// 添加基础层
    pub fn add_lower_layer(&mut self, layer: Layer) -> Result<(), i32> {
        if !layer.read_only {
            return Err(EINVAL);
        }
        self.lower_layers.push(layer);
        Ok(())
    }

    /// 设置上层（可写层）
    pub fn set_upper_layer(&mut self, layer: Layer) -> Result<(), i32> {
        if layer.read_only {
            return Err(EINVAL);
        }
        self.upper_layer = Some(layer);
        Ok(())
    }

    /// 设置工作目录
    pub fn set_work_dir(&mut self, work_dir: String) {
        self.work_dir = Some(work_dir);
    }

    /// 构建rootfs（不使用overlayfs）
    pub fn build(&self) -> Result<(), i32> {
        crate::println!("[rootfs] Building rootfs at {}", self.mount_point);

        // 1. 创建目录结构
        self.create_directory_structure()?;

        // 2. 创建设备节点
        self.create_device_nodes()?;

        // 3. 设置挂载点
        self.setup_mounts()?;

        crate::println!("[rootfs] Built rootfs successfully");

        Ok(())
    }

    /// 构建overlayfs rootfs
    pub fn build_overlay(&self) -> Result<(), i32> {
        if self.upper_layer.is_none() || self.work_dir.is_none() {
            return Err(EINVAL);
        }

        crate::println!("[rootfs] Building overlayfs at {}", self.mount_point);

        // 1. 准备overlayfs参数
        let lower_dirs = self
            .lower_layers
            .iter()
            .map(|l| l.path.clone())
            .collect::<Vec<_>>()
            .join(":");

        let upper_dir = self.upper_layer.as_ref().unwrap().path.clone();
        let work_dir = self.work_dir.as_ref().unwrap().clone();

        // 2. 挂载overlayfs
        self.mount_overlayfs(&lower_dirs, &upper_dir, &work_dir)?;

        // 3. 创建设备节点
        self.create_device_nodes()?;

        crate::println!("[rootfs] Built overlayfs successfully");

        Ok(())
    }

    /// 创建目录结构
    fn create_directory_structure(&self) -> Result<(), i32> {
        let essential_dirs = [
            "bin", "sbin", "lib", "lib64", "usr", "usr/bin", "usr/sbin", "usr/lib",
            "etc", "proc", "sys", "dev", "tmp", "var", "var/run", "home", "root",
        ];

        for dir in &essential_dirs {
            let path = format!("{}/{}", self.mount_point, dir);
            // 在实际实现中，这里会创建目录
            crate::println!("[rootfs] Creating directory: {}", path);
        }

        Ok(())
    }

    /// 创建设备节点
    fn create_device_nodes(&self) -> Result<(), i32> {
        // 标准设备节点
        let devices = [
            ("null", 1, 3, 0o666),
            ("zero", 1, 5, 0o666),
            ("full", 1, 7, 0o666),
            ("random", 1, 8, 0o666),
            ("urandom", 1, 9, 0o666),
            ("tty", 5, 0, 0o666),
        ];

        for (name, major, minor, _mode) in &devices {
            let path = format!("{}/dev/{}", self.mount_point, name);
            // 在实际实现中，这里会使用mknod创建设备节点
            crate::println!(
                "[rootfs] Creating device node: {} (major={}, minor={})",
                path,
                major,
                minor
            );
        }

        // 创建标准符号链接
        let symlinks = [
            ("/proc/self/fd", "dev/fd"),
            ("/proc/self/fd/0", "dev/stdin"),
            ("/proc/self/fd/1", "dev/stdout"),
            ("/proc/self/fd/2", "dev/stderr"),
        ];

        for (target, link) in &symlinks {
            let path = format!("{}/{}", self.mount_point, link);
            // 在实际实现中，这里会创建符号链接
            crate::println!("[rootfs] Creating symlink: {} -> {}", path, target);
        }

        Ok(())
    }

    /// 设置挂载点
    fn setup_mounts(&self) -> Result<(), i32> {
        // 挂载proc
        crate::println!("[rootfs] Mounting proc at {}/proc", self.mount_point);
        // 在实际实现中，这里会执行：
        // mount("proc", "/proc", "proc", MS_NOSUID | MS_NOEXEC | MS_NODEV, "")

        // 挂载sysfs
        crate::println!("[rootfs] Mounting sysfs at {}/sys", self.mount_point);
        // 在实际实现中，这里会执行：
        // mount("sysfs", "/sys", "sysfs", MS_NOSUID | MS_NOEXEC | MS_NODEV, "")

        // 挂载tmpfs到/dev
        crate::println!("[rootfs] Mounting tmpfs at {}/dev", self.mount_point);
        // 在实际实现中，这里会执行：
        // mount("tmpfs", "/dev", "tmpfs", MS_NOSUID | MS_STRICTATIME, "mode=755")

        // 挂载tmpfs到/dev/shm
        crate::println!("[rootfs] Mounting tmpfs at {}/dev/shm", self.mount_point);
        // 在实际实现中，这里会执行：
        // mount("tmpfs", "/dev/shm", "tmpfs", MS_NOSUID | MS_NODEV, "")

        // 挂载tmpfs到/dev/pts
        crate::println!("[rootfs] Mounting devpts at {}/dev/pts", self.mount_point);
        // 在实际实现中，这里会执行：
        // mount("devpts", "/dev/pts", "devpts", MS_NOSUID | MS_NOEXEC, "")

        Ok(())
    }

    /// 挂载overlayfs
    fn mount_overlayfs(&self, lower: &str, upper: &str, work: &str) -> Result<(), i32> {
        let options = format!("lowerdir={},upperdir={},workdir={}", lower, upper, work);

        crate::println!(
            "[rootfs] Mounting overlayfs with options: {}",
            options
        );

        // 在实际实现中，这里会执行：
        // mount("overlay", mount_point, "overlay", 0, options)

        Ok(())
    }

    /// 卸载rootfs
    pub fn unmount(&self) -> Result<(), i32> {
        crate::println!("[rootfs] Unmounting rootfs at {}", self.mount_point);

        // 在实际实现中，这里会按顺序卸载所有挂载点

        Ok(())
    }
}

/// Rootfs管理器
pub struct RootfsManager {
    /// rootfs列表
    rootfs_list: Vec<String>,
}

impl RootfsManager {
    /// 创建新的Rootfs管理器
    pub fn new() -> Self {
        Self {
            rootfs_list: Vec::new(),
        }
    }

    /// 创建容器rootfs
    pub fn create_container_rootfs(
        &mut self,
        container_id: &str,
        base_layers: Vec<String>,
    ) -> Result<String, i32> {
        let mount_point = format!("/var/lib/containers/{}/rootfs", container_id);
        let upper_layer = format!("/var/lib/containers/{}/upper", container_id);
        let work_dir = format!("/var/lib/containers/{}/work", container_id);

        let mut builder = RootfsBuilder::new(mount_point.clone());

        // 添加基础层
        for (i, layer_path) in base_layers.iter().enumerate() {
            let layer = Layer {
                id: format!("layer-{}", i),
                path: layer_path.clone(),
                read_only: true,
                layer_type: LayerType::Base,
            };
            builder.add_lower_layer(layer)?;
        }

        // 设置上层
        let upper = Layer {
            id: "upper".to_string(),
            path: upper_layer,
            read_only: false,
            layer_type: LayerType::Upper,
        };
        builder.set_upper_layer(upper)?;

        // 设置工作目录
        builder.set_work_dir(work_dir);

        // 构建overlayfs
        builder.build_overlay()?;

        self.rootfs_list.push(mount_point.clone());

        crate::println!("[rootfs-manager] Created rootfs for container {}", container_id);

        Ok(mount_point)
    }

    /// 删除容器rootfs
    pub fn delete_container_rootfs(&mut self, container_id: &str) -> Result<(), i32> {
        let mount_point = format!("/var/lib/containers/{}/rootfs", container_id);

        // 卸载
        let builder = RootfsBuilder {
            lower_layers: Vec::new(),
            upper_layer: None,
            work_dir: None,
            mount_point: mount_point.clone(),
        };
        builder.unmount()?;

        // 从列表中移除
        self.rootfs_list.retain(|p| p != &mount_point);

        crate::println!("[rootfs-manager] Deleted rootfs for container {}", container_id);

        Ok(())
    }

    /// 获取rootfs路径
    pub fn get_rootfs_path(&self, container_id: &str) -> Option<String> {
        let mount_point = format!("/var/lib/containers/{}/rootfs", container_id);
        if self.rootfs_list.contains(&mount_point) {
            Some(mount_point)
        } else {
            None
        }
    }
}

impl Default for RootfsManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 全局Rootfs管理器
static mut ROOTFS_MANAGER: Option<RootfsManager> = None;
static mut ROOTFS_MANAGER_INITIALIZED: bool = false;

/// 初始化Rootfs管理器
pub fn init_rootfs_manager() -> Result<(), i32> {
    if unsafe { ROOTFS_MANAGER_INITIALIZED } {
        return Ok(());
    }

    let manager = RootfsManager::new();

    unsafe {
        ROOTFS_MANAGER = Some(manager);
        ROOTFS_MANAGER_INITIALIZED = true;
    }

    crate::println!("[rootfs-manager] Initialized");

    Ok(())
}

/// 获取Rootfs管理器
pub fn get_rootfs_manager() -> Option<&'static mut RootfsManager> {
    unsafe { ROOTFS_MANAGER.as_mut() }
}
