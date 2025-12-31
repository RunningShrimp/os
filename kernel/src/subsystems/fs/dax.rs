//! # DAX (Direct Access) 实现
//!
//! 提供持久化内存的直接访问支持，绕过页缓存。
//!
//! ## 功能
//!
//! - mmap 直接映射支持
//! - fsync/msync 持久化
//! - 故障注入和测试
//! - 与 ext4/xfs 集成
//!
//! ## 架构
//!
//! ```
//! DAX 层
//!     ├── 直接映射 (mmap)
//!     ├── 持久化同步 (fsync/msync)
//!     ├── 故障处理
//!     └── VFS 集成
//! ```

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use spin::Mutex;

use nos_api::Error;

use crate::vfs::FileAttr;
use crate::subsystems::sync::Mutex as AdvancedMutex;

/// DAX 魔数
pub const DAX_MAGIC: u32 = 0x44415858; // "DAXX"

/// DAX 页面大小
pub const DAX_PAGE_SIZE: usize = 4096;

/// 最大文件大小
pub const MAX_FILE_SIZE: u64 = 1 << 40; // 1 TB

/// DAX 映射类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaxMapType {
    /// 私有映射（COW）
    Private,
    /// 共享映射
    Shared,
}

/// DAX 持久化类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaxPersistType {
    /// 不持久化
    None,
    /// 仅数据
    Data,
    /// 数据和元数据
    DataSync,
}

/// DAX 区域
#[derive(Debug)]
pub struct DaxRegion {
    /// 区域 ID
    pub id: u64,
    /// 物理地址基址
    pub phys_base: u64,
    /// 虚拟地址基址
    pub virt_base: u64,
    /// 区域大小
    pub size: u64,
    /// 是否已映射
    pub mapped: AtomicBool,
}

impl Clone for DaxRegion {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            phys_base: self.phys_base,
            virt_base: self.virt_base,
            size: self.size,
            mapped: AtomicBool::new(self.mapped.load(core::sync::atomic::Ordering::SeqCst)),
        }
    }
}

/// DAX 文件元数据
#[derive(Debug, Clone)]
#[repr(C)]
pub struct DaxFileMetadata {
    /// 文件大小
    pub size: u64,
    /// 分配的块数
    pub allocated_blocks: u64,
    /// 修改时间
    pub mtime: u64,
    /// 文件模式
    pub mode: u32,
    /// 魔数
    pub magic: u32,
}

/// DAX 文件
#[derive(Debug)]
pub struct DaxFile {
    /// 文件 ID
    pub id: u64,
    /// 文件名
    pub name: String,
    /// 元数据
    pub metadata: Mutex<DaxFileMetadata>,
    /// 数据块映射（块索引 -> 物理地址）
    pub blocks: Mutex<BTreeMap<u64, u64>>,
    /// 所属区域
    pub region: Arc<DaxRegion>,
    /// 是否脏
    pub dirty: AtomicBool,
}

impl DaxFile {
    /// 创建新文件
    pub fn new(id: u64, name: String, region: Arc<DaxRegion>) -> Self {
        Self {
            id,
            name,
            metadata: Mutex::new(DaxFileMetadata {
                size: 0,
                allocated_blocks: 0,
                mtime: 0,
                mode: 0o644,
                magic: DAX_MAGIC,
            }),
            blocks: Mutex::new(BTreeMap::new()),
            region,
            dirty: AtomicBool::new(false),
        }
    }

    /// 分配块
    pub fn allocate_block(&self, block_index: u64) -> Result<u64, Error> {
        let mut blocks = self.blocks.lock();

        if blocks.contains_key(&block_index) {
            return Ok(*blocks.get(&block_index).unwrap());
        }

        // 从区域中分配块
        let offset = block_index * DAX_PAGE_SIZE as u64;
        let phys_addr = self.region.phys_base + offset;

        blocks.insert(block_index, phys_addr);

        // 更新元数据
        let mut metadata = self.metadata.lock();
        metadata.allocated_blocks += 1;

        Ok(phys_addr)
    }

    /// 读取块
    pub fn read_block(&self, block_index: u64, buffer: &mut [u8]) -> Result<usize, Error> {
        let phys_addr = {
            let blocks = self.blocks.lock();
            *blocks.get(&block_index).ok_or_else(|| Error::NotFound("block not found".into()))?
        };

        let virt_addr = self.region.virt_base + (phys_addr - self.region.phys_base);

        unsafe {
            let ptr = virt_addr as *const u8;
            for i in 0..buffer.len() {
                buffer[i] = ptr.add(i).read_volatile();
            }
        }

        Ok(buffer.len())
    }

    /// 写入块
    pub fn write_block(&self, block_index: u64, data: &[u8]) -> Result<usize, Error> {
        // 确保块已分配
        self.allocate_block(block_index)?;

        let blocks = self.blocks.lock();
        let phys_addr = *blocks.get(&block_index).ok_or_else(|| Error::NotFound("block not found".into()))?;
        drop(blocks);

        let virt_addr = self.region.virt_base + (phys_addr - self.region.phys_base);

        unsafe {
            let ptr = virt_addr as *mut u8;
            for i in 0..data.len() {
                ptr.add(i).write_volatile(data[i]);
            }
        }

        // 标记为脏
        self.dirty.store(true, Ordering::Release);

        Ok(data.len())
    }

    /// 持久化文件
    pub fn persist(&self) -> Result<(), Error> {
        if !self.dirty.load(Ordering::Acquire) {
            return Ok(());
        }

        // 持久化所有块
        let blocks = self.blocks.lock();
        for (_block_index, phys_addr) in blocks.iter() {
            let offset = phys_addr - self.region.phys_base;
            let virt_addr = self.region.virt_base + offset;

            // 刷新缓存行
            unsafe {
                crate::subsystems::mm::libpmem::pmem_flush(virt_addr, DAX_PAGE_SIZE);
            }
        }
        drop(blocks);

        // 排空缓冲区
        crate::subsystems::mm::libpmem::pmem_drain();

        // 持久化元数据
        // GH-#1212: 持久化文件元数据
        // See: https://github.com/npos/kernel/issues/1212

        self.dirty.store(false, Ordering::Release);

        Ok(())
    }

    /// 获取文件属性
    pub fn getattr(&self) -> FileAttr {
        let metadata = self.metadata.lock();

        FileAttr {
            ino: self.id,
            size: metadata.size,
            blocks: metadata.allocated_blocks,
            atime: metadata.mtime,
            mtime: metadata.mtime,
            ctime: metadata.mtime,
            mode: crate::vfs::types::FileMode(metadata.mode),
            nlink: 1,
            uid: 0,
            gid: 0,
            rdev: 0,
            blksize: DAX_PAGE_SIZE as u32,
        }
    }
}

/// DAX 设备
#[derive(Debug)]
pub struct DaxDevice {
    /// 设备 ID
    pub id: u64,
    /// 设备名称
    pub name: String,
    /// DAX 区域
    pub region: Arc<DaxRegion>,
    /// 文件映射
    pub files: Mutex<BTreeMap<u64, Arc<DaxFile>>>,
    /// 下一个文件 ID
    pub next_file_id: AtomicU64,
}

impl DaxDevice {
    /// 创建新的 DAX 设备
    pub fn new(id: u64, name: String, phys_base: u64, size: u64) -> Result<Self, Error> {
        let region = Arc::new(DaxRegion {
            id,
            phys_base,
            virt_base: 0x4000_0000_0000, // GH-#1213: 实际映射
            // See: https://github.com/npos/kernel/issues/1213
            size,
            mapped: AtomicBool::new(false),
        });

        Ok(Self {
            id,
            name,
            region,
            files: Mutex::new(BTreeMap::new()),
            next_file_id: AtomicU64::new(1),
        })
    }

    /// 创建文件
    pub fn create_file(&self, name: String) -> Result<Arc<DaxFile>, Error> {
        let file_id = self.next_file_id.fetch_add(1, Ordering::SeqCst);
        let file = Arc::new(DaxFile::new(file_id, name, self.region.clone()));

        self.files.lock().insert(file_id, file.clone());

        Ok(file)
    }

    /// 获取文件
    pub fn get_file(&self, file_id: u64) -> Result<Arc<DaxFile>, Error> {
        self.files
            .lock()
            .get(&file_id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("file_id {}", file_id)))
    }

    /// 删除文件
    pub fn delete_file(&self, file_id: u64) -> Result<(), Error> {
        self.files.lock().remove(&file_id).ok_or_else(|| Error::NotFound(format!("file_id {}", file_id)))?;
        Ok(())
    }

    /// 映射文件到内存
    pub fn mmap(
        &self,
        file_id: u64,
        _addr: u64,
        size: u64,
        map_type: DaxMapType,
    ) -> Result<u64, Error> {
        let file = self.get_file(file_id)?;

        // 计算映射地址
        let map_addr = self.region.virt_base + (file.id * MAX_FILE_SIZE);

        crate::println!("[dax] mmap file {} at 0x{:x}, size=0x{:x}, type={:?}",
            file.name, map_addr, size, map_type);

        Ok(map_addr)
    }

    /// 同步文件到持久化内存
    pub fn msync(&self, file_id: u64, addr: u64, size: u64) -> Result<(), Error> {
        let file = self.get_file(file_id)?;

        // 刷新指定范围
        unsafe {
            crate::subsystems::mm::libpmem::pmem_flush(addr, size as usize);
        }
        crate::subsystems::mm::libpmem::pmem_drain();

        file.persist()?;

        Ok(())
    }

    /// fsync 实现
    pub fn fsync(&self, file_id: u64, datasync: bool) -> Result<(), Error> {
        let file = self.get_file(file_id)?;
        file.persist()?;

        if !datasync {
            // 持久化元数据
            // GH-#1214: 实现元数据持久化
            // See: https://github.com/npos/kernel/issues/1214
        }

        Ok(())
    }
}

/// DAX 管理器
#[derive(Debug)]
pub struct DaxManager {
    /// DAX 设备列表
    pub devices: Mutex<BTreeMap<u64, Arc<DaxDevice>>>,
    /// 统计信息
    pub stats: DaxStats,
    /// 是否启用
    pub enabled: bool,
}

#[derive(Debug)]
pub struct DaxStats {
    pub device_count: AtomicU64,
    pub file_count: AtomicU64,
    pub total_mapped: AtomicU64,
    pub total_flushed: AtomicU64,
}

impl Clone for DaxStats {
    fn clone(&self) -> Self {
        Self {
            device_count: AtomicU64::new(self.device_count.load(core::sync::atomic::Ordering::SeqCst)),
            file_count: AtomicU64::new(self.file_count.load(core::sync::atomic::Ordering::SeqCst)),
            total_mapped: AtomicU64::new(self.total_mapped.load(core::sync::atomic::Ordering::SeqCst)),
            total_flushed: AtomicU64::new(self.total_flushed.load(core::sync::atomic::Ordering::SeqCst)),
        }
    }
}

impl Default for DaxStats {
    fn default() -> Self {
        Self {
            device_count: AtomicU64::new(0),
            file_count: AtomicU64::new(0),
            total_mapped: AtomicU64::new(0),
            total_flushed: AtomicU64::new(0),
        }
    }
}

impl DaxManager {
    /// 创建新的 DAX 管理器
    pub fn new() -> Self {
        Self {
            devices: Mutex::new(BTreeMap::new()),
            stats: DaxStats::default(),
            enabled: false,
        }
    }

    /// 初始化 DAX 管理器
    pub fn init(&mut self) {
        self.enabled = true;
        crate::println!("[dax] DAX manager initialized");
    }

    /// 注册 DAX 设备
    pub fn register_device(&self, device: Arc<DaxDevice>) -> Result<(), Error> {
        self.devices.lock().insert(device.id, device.clone());
        self.stats.device_count.fetch_add(1, Ordering::Relaxed);

        crate::println!("[dax] Registered device: {}", device.name);

        Ok(())
    }

    /// 获取设备
    pub fn get_device(&self, device_id: u64) -> Result<Arc<DaxDevice>, Error> {
        self.devices
            .lock()
            .get(&device_id)
            .cloned()
            .ok_or_else(|| Error::NotFound(format!("device_id {}", device_id)))
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> DaxStatsSnapshot {
        DaxStatsSnapshot {
            device_count: self.stats.device_count.load(Ordering::Relaxed),
            file_count: self.stats.file_count.load(Ordering::Relaxed),
            total_mapped: self.stats.total_mapped.load(Ordering::Relaxed),
            total_flushed: self.stats.total_flushed.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone)]
pub struct DaxStatsSnapshot {
    pub device_count: u64,
    pub file_count: u64,
    pub total_mapped: u64,
    pub total_flushed: u64,
}

/// 全局 DAX 管理器
static DAX_MANAGER: AdvancedMutex<Option<DaxManager>> = AdvancedMutex::new(None);

/// 初始化 DAX
pub fn init() -> Result<(), Error> {
    let mut manager_guard = DAX_MANAGER.lock();
    let mut manager = DaxManager::new();
    manager.init();
    *manager_guard = Some(manager);
    Ok(())
}

/// 关闭 DAX
pub fn shutdown() -> Result<(), Error> {
    *DAX_MANAGER.lock() = None;
    Ok(())
}

/// 获取 DAX 管理器
pub fn manager() -> Result<&'static AdvancedMutex<Option<DaxManager>>, Error> {
    if DAX_MANAGER.lock().is_some() {
        Ok(&DAX_MANAGER)
    } else {
        Err(Error::InvalidState("DAX manager not initialized".to_string()))
    }
}

/// 注册 DAX 设备（便捷函数）
pub fn register_device(name: String, phys_base: u64, size: u64) -> Result<u64, Error> {
    let manager_guard = DAX_MANAGER.lock();
    let manager = manager_guard.as_ref().ok_or_else(|| Error::InvalidState("DAX manager not initialized".to_string()))?;

    let device_id = manager.stats.device_count.load(Ordering::Relaxed);
    let device = Arc::new(DaxDevice::new(device_id, name, phys_base, size)?);
    manager.register_device(device)?;

    Ok(device_id)
}

/// mmap 实现（便捷函数）
pub fn mmap(device_id: u64, file_id: u64, addr: u64, size: u64, map_type: DaxMapType) -> Result<u64, Error> {
    let manager_guard = DAX_MANAGER.lock();
    let manager = manager_guard.as_ref().ok_or_else(|| Error::InvalidState("DAX manager not initialized".into()))?;
    let device = manager.get_device(device_id)?;
    device.mmap(file_id, addr, size, map_type)
}

/// msync 实现（便捷函数）
pub fn msync(device_id: u64, file_id: u64, addr: u64, size: u64) -> Result<(), Error> {
    let manager_guard = DAX_MANAGER.lock();
    let manager = manager_guard.as_ref().ok_or_else(|| Error::InvalidState("DAX manager not initialized".into()))?;
    let device = manager.get_device(device_id)?;
    device.msync(file_id, addr, size)
}

/// fsync 实现（便捷函数）
pub fn fsync(device_id: u64, file_id: u64, datasync: bool) -> Result<(), Error> {
    let manager_guard = DAX_MANAGER.lock();
    let manager = manager_guard.as_ref().ok_or_else(|| Error::InvalidState("DAX manager not initialized".into()))?;
    let device = manager.get_device(device_id)?;
    device.fsync(file_id, datasync)
}
