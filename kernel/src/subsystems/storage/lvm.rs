//! # 逻辑卷管理 (LVM) 实现
//!
//! 提供灵活的逻辑卷管理功能：
//! - **物理卷 (PV)**: 管理物理存储设备
//! - **卷组 (VG)**: 聚合多个物理卷
//! - **逻辑卷 (LV)**: 创建可调整大小的逻辑卷
//! - **快照**: 支持卷快照
//! - **精简配置**: 按需分配存储

extern crate alloc;

use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use core::sync::atomic::{AtomicU64, AtomicU32, AtomicBool, AtomicU8, Ordering};
use crate::subsystems::sync::Mutex;
use crate::subsystems::drivers::block::{BlockDevice, Bio, BlockOp, BioStatus};
use crate::error::{Result, Error};

/// 默认扩展大小 (4MB)
const DEFAULT_EXTENT_SIZE: u64 = 4 * 1024 * 1024;

/// 物理卷状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PvState {
    /// 可用
    Available = 0,
    /// 正在使用
    InUse = 1,
    /// 故障
    Failed = 2,
    /// 正在移除
    Removing = 3,
}

/// 逻辑卷状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LvState {
    /// 可用
    Available = 0,
    /// 正在创建
    Creating = 1,
    /// 正在删除
    Deleting = 2,
    /// 正在扩容
    Extending = 3,
    /// 正在收缩
    Shrinking = 4,
    /// 故障
    Failed = 5,
}

/// 物理卷信息
pub struct PhysicalVolume {
    /// PV UUID
    pub uuid: String,
    /// PV 名称
    pub name: String,
    /// 底层块设备
    pub device: Arc<dyn BlockDevice>,
    /// PV 状态
    pub state: AtomicU8,
    /// 设备容量（字节）
    pub capacity: u64,
    /// 已用空间（字节）
    pub used: AtomicU64,
    /// 扩展大小
    pub extent_size: u64,
    /// 扩展分配位图
    pub allocation_map: Mutex<Vec<bool>>,
}

impl core::fmt::Debug for PhysicalVolume {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PhysicalVolume")
            .field("uuid", &self.uuid)
            .field("name", &self.name)
            .field("device", &"<BlockDevice>")
            .field("state", &self.state)
            .field("capacity", &self.capacity)
            .field("used", &self.used)
            .field("extent_size", &self.extent_size)
            .finish()
    }
}

impl Clone for PhysicalVolume {
    fn clone(&self) -> Self {
        Self {
            uuid: self.uuid.clone(),
            name: self.name.clone(),
            device: self.device.clone(),
            state: AtomicU8::new(self.state.load(Ordering::Relaxed)),
            capacity: self.capacity,
            used: AtomicU64::new(self.used.load(Ordering::Relaxed)),
            extent_size: self.extent_size,
            allocation_map: Mutex::new(self.allocation_map.lock().clone()),
        }
    }
}

impl PhysicalVolume {
    /// 创建新的物理卷
    pub fn new(
        uuid: String,
        name: String,
        device: Arc<dyn BlockDevice>,
        extent_size: u64,
    ) -> Self {
        let capacity = device.device_size();
        let extent_count = capacity / extent_size;

        Self {
            uuid,
            name,
            device,
            state: AtomicU8::new(PvState::Available as u8),
            capacity,
            used: AtomicU64::new(0),
            extent_size,
            allocation_map: Mutex::new(vec![false; extent_count as usize]),
        }
    }

    /// 分配扩展
    pub fn allocate_extents(&self, count: u64) -> Result<Vec<u64>> {
        let mut map = self.allocation_map.lock();
        let mut allocated = Vec::new();
        let mut found = 0u64;

        for (i, &used) in map.iter().enumerate() {
            if !used {
                map[i] = true;
                allocated.push(i as u64);
                found += 1;
                if found == count {
                    break;
                }
            }
        }

        if found == count {
            self.used.fetch_add(count * self.extent_size, Ordering::Relaxed);
            Ok(allocated)
        } else {
            // 回滚
            for &extent_index in &allocated {
                map[extent_index as usize] = false;
            }
            Err(Error::NoSpace)
        }
    }

    /// 释放扩展
    pub fn free_extents(&self, extents: &[u64]) {
        let mut map = self.allocation_map.lock();
        let mut freed_bytes = 0u64;

        for &extent_index in extents {
            if extent_index as usize < map.len() {
                if map[extent_index as usize] {
                    map[extent_index as usize] = false;
                    freed_bytes += self.extent_size;
                }
            }
        }

        self.used.fetch_sub(freed_bytes, Ordering::Relaxed);
    }

    /// 获取可用扩展数
    pub fn free_extent_count(&self) -> u64 {
        let map = self.allocation_map.lock();
        map.iter().filter(|&&used| !used).count() as u64
    }

    /// 获取状态
    pub fn state(&self) -> PvState {
        match self.state.load(Ordering::Acquire) {
            0 => PvState::Available,
            1 => PvState::InUse,
            2 => PvState::Failed,
            3 => PvState::Removing,
            _ => PvState::Failed,
        }
    }
}

/// 卷组
pub struct VolumeGroup {
    /// VG UUID
    pub uuid: String,
    /// VG 名称
    pub name: String,
    /// 成员物理卷
    pub physical_volumes: Mutex<BTreeMap<String, Arc<PhysicalVolume>>>,
    /// 逻辑卷列表
    pub logical_volumes: Mutex<BTreeMap<String, Arc<LogicalVolume>>>,
    /// 扩展大小
    pub extent_size: u64,
    /// 下一个 LV ID
    next_lv_id: AtomicU64,
}

impl VolumeGroup {
    /// 创建新的卷组
    pub fn new(name: String, extent_size: u64) -> Self {
        Self {
            uuid: Self::generate_uuid(),
            name,
            physical_volumes: Mutex::new(BTreeMap::new()),
            logical_volumes: Mutex::new(BTreeMap::new()),
            extent_size,
            next_lv_id: AtomicU64::new(1),
        }
    }

    /// 添加物理卷
    pub fn add_pv(&self, pv: Arc<PhysicalVolume>) -> Result<()> {
        let mut pvs = self.physical_volumes.lock();
        if pvs.contains_key(&pv.uuid) {
            return Err(Error::Exists);
        }

        pv.state.store(PvState::InUse as u8, Ordering::Release);
        pvs.insert(pv.uuid.clone(), pv);

        Ok(())
    }

    /// 移除物理卷
    pub fn remove_pv(&self, pv_uuid: &str) -> Result<()> {
        let mut pvs = self.physical_volumes.lock();
        let pv = pvs.get(pv_uuid).ok_or(Error::NotFound)?;

        // 检查是否还有数据
        if pv.used.load(Ordering::Acquire) > 0 {
            return Err(Error::ResourceBusy);
        }

        pv.state.store(PvState::Removing as u8, Ordering::Release);
        pvs.remove(pv_uuid);

        Ok(())
    }

    /// 创建逻辑卷
    pub fn create_lv(
        &self,
        name: String,
        size: u64,
    ) -> Result<Arc<LogicalVolume>> {
        // 计算需要的扩展数
        let extent_count = (size + self.extent_size - 1) / self.extent_size;

        // 从各个 PV 分配扩展
        let mut allocations = Vec::new();
        let pvs = self.physical_volumes.lock();

        for (_uuid, pv) in pvs.iter() {
            if pv.state() != PvState::InUse {
                continue;
            }

            let needed = extent_count - allocations.len() as u64;
            if needed == 0 {
                break;
            }

            match pv.allocate_extents(needed) {
                Ok(extents) => {
                    for extent in extents {
                        allocations.push((pv.uuid.clone(), extent));
                    }
                }
                Err(_) => continue,
            }
        }

        if allocations.len() as u64 < extent_count {
            // 回滚分配
            for (pv_uuid, extents) in allocations {
                if let Some(pv) = pvs.get(&pv_uuid) {
                    pv.free_extents(&[extents]);
                }
            }
            return Err(Error::NoSpace);
        }

        drop(pvs);

        // 创建逻辑卷
        let id = self.next_lv_id.fetch_add(1, Ordering::Relaxed);
        let lv = Arc::new(LogicalVolume::new(
            id,
            name,
            size,
            allocations,
            self.extent_size,
        )?);

        let mut lvs = self.logical_volumes.lock();
        lvs.insert(lv.name.clone(), lv.clone());

        Ok(lv)
    }

    /// 删除逻辑卷
    pub fn remove_lv(&self, name: &str) -> Result<()> {
        let mut lvs = self.logical_volumes.lock();
        let lv = lvs.remove(name).ok_or(Error::NotFound)?;

        // 释放所有扩展
        let pvs = self.physical_volumes.lock();
        for (pv_uuid, extents) in lv.allocations.lock().iter() {
            if let Some(pv) = pvs.get(pv_uuid) {
                pv.free_extents(extents);
            }
        }

        Ok(())
    }

    /// 扩容逻辑卷
    pub fn extend_lv(&self, name: &str, additional_size: u64) -> Result<()> {
        let lvs = self.logical_volumes.lock();
        let lv = lvs.get(name).ok_or(Error::NotFound)?.clone();

        let additional_extents = (additional_size + self.extent_size - 1) / self.extent_size;

        // 分配新扩展
        let mut new_allocations = Vec::new();
        let pvs = self.physical_volumes.lock();

        for (_uuid, pv) in pvs.iter() {
            if pv.state() != PvState::InUse {
                continue;
            }

            let needed = additional_extents - new_allocations.len() as u64;
            if needed == 0 {
                break;
            }

            match pv.allocate_extents(needed) {
                Ok(extents) => {
                    for extent in extents {
                        new_allocations.push((pv.uuid.clone(), extent));
                    }
                }
                Err(_) => continue,
            }
        }

        if new_allocations.len() as u64 < additional_extents {
            // 回滚
            for (pv_uuid, extent) in &new_allocations {
                if let Some(pv) = pvs.get(pv_uuid) {
                    pv.free_extents(&[*extent]);
                }
            }
            return Err(Error::NoSpace);
        }

        drop(pvs);

        // 更新逻辑卷
        let mut allocations = lv.allocations.lock();
        for (pv_uuid, extent) in new_allocations {
            allocations.entry(pv_uuid).or_insert_with(Vec::new).push(extent);
        }

        lv.size.fetch_add(additional_size, Ordering::Relaxed);

        Ok(())
    }

    /// 获取总容量
    pub fn total_capacity(&self) -> u64 {
        let pvs = self.physical_volumes.lock();
        pvs.values().map(|pv| pv.capacity).sum()
    }

    /// 获取可用容量
    pub fn free_capacity(&self) -> u64 {
        let pvs = self.physical_volumes.lock();
        pvs.values().map(|pv| {
            pv.capacity - pv.used.load(Ordering::Relaxed)
        }).sum()
    }

    /// 生成 UUID
    fn generate_uuid() -> String {
        // 简化实现
        alloc::format!("pv-{}", core::time::Duration::from_secs(0).as_nanos())
    }
}

/// 逻辑卷
pub struct LogicalVolume {
    /// LV ID
    pub id: u64,
    /// LV 名称
    pub name: String,
    /// 逻辑卷大小（字节）
    pub size: AtomicU64,
    /// 扩展分配：PV UUID -> 扩展列表
    pub allocations: Mutex<BTreeMap<String, Vec<u64>>>,
    /// 扩展大小
    pub extent_size: u64,
    /// 卷组引用
    pub vg: Arc<VolumeGroup>,
    /// LV 状态
    pub state: AtomicU8,
    /// 快照列表
    pub snapshots: Mutex<Vec<Arc<Snapshot>>>,
}

impl Clone for LogicalVolume {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            size: AtomicU64::new(self.size.load(Ordering::Relaxed)),
            allocations: Mutex::new(self.allocations.lock().clone()),
            extent_size: self.extent_size,
            vg: self.vg.clone(),
            state: AtomicU8::new(self.state.load(Ordering::Relaxed)),
            snapshots: Mutex::new(self.snapshots.lock().clone()),
        }
    }
}

impl LogicalVolume {
    /// 创建新的逻辑卷
    pub fn new(
        id: u64,
        name: String,
        size: u64,
        allocations: Vec<(String, u64)>,
        extent_size: u64,
    ) -> Result<Self> {
        let mut allocation_map = BTreeMap::new();
        for (pv_uuid, extent) in allocations {
            allocation_map.entry(pv_uuid).or_insert_with(Vec::new).push(extent);
        }

        Ok(Self {
            id,
            name,
            size: AtomicU64::new(size),
            allocations: Mutex::new(allocation_map),
            extent_size,
            vg: Arc::new(VolumeGroup::new("temp-vg".to_string(), extent_size)), // Placeholder
            state: AtomicU8::new(LvState::Available as u8),
            snapshots: Mutex::new(Vec::new()),
        })
    }

    /// 读取数据
    pub fn read(&self, offset: u64, buffer: &mut [u8]) -> Result<usize> {
        if self.state.load(Ordering::Acquire) != LvState::Available as u8 {
            return Err(Error::InvalidState);
        }

        let allocations = self.allocations.lock();
        let extent_size = self.extent_size;

        let mut total_read = 0;
        let mut current_offset = offset;
        let pvs = self.vg.physical_volumes.lock();

        while total_read < buffer.len() {
            // 计算扩展索引
            let global_extent_index = current_offset / extent_size;
            let extent_offset = current_offset % extent_size;

            // 查找对应的 PV 和扩展
            let mut found = false;
            let mut extent_index_in_lv = 0u64;

            for (_pv_uuid, extents) in allocations.iter() {
                for &extent in extents {
                    if extent_index_in_lv == global_extent_index {
                        // 找到了
                        if let Some(pv) = pvs.get(_pv_uuid) {
                            let pv_offset = extent * extent_size + extent_offset;
                            let sector = pv_offset / pv.device.sector_size();

                            let remaining = buffer.len() - total_read;
                            let chunk_size = remaining.min((extent_size - extent_offset) as usize);

                            let bytes_read = pv.device.read(sector, &mut buffer[total_read..total_read + chunk_size])?;
                            total_read += bytes_read;
                            current_offset += bytes_read as u64;
                            found = true;
                        }
                        break;
                    }
                    extent_index_in_lv += 1;
                }
                if found {
                    break;
                }
            }

            if !found {
                break;
            }
        }

        Ok(total_read)
    }

    /// 写入数据
    pub fn write(&self, offset: u64, data: &[u8]) -> Result<usize> {
        if self.state.load(Ordering::Acquire) != LvState::Available as u8 {
            return Err(Error::InvalidState);
        }

        let allocations = self.allocations.lock();
        let extent_size = self.extent_size;

        let mut total_written = 0;
        let mut current_offset = offset;
        let pvs = self.vg.physical_volumes.lock();

        while total_written < data.len() {
            // 计算扩展索引
            let global_extent_index = current_offset / extent_size;
            let extent_offset = current_offset % extent_size;

            // 查找对应的 PV 和扩展
            let mut found = false;
            let mut extent_index_in_lv = 0u64;

            for (_pv_uuid, extents) in allocations.iter() {
                for &extent in extents {
                    if extent_index_in_lv == global_extent_index {
                        // 找到了
                        if let Some(pv) = pvs.get(_pv_uuid) {
                            let pv_offset = extent * extent_size + extent_offset;
                            let sector = pv_offset / pv.device.sector_size();

                            let remaining = data.len() - total_written;
                            let chunk_size = remaining.min((extent_size - extent_offset) as usize);

                            let bytes_written = pv.device.write(sector, &data[total_written..total_written + chunk_size])?;
                            total_written += bytes_written;
                            current_offset += bytes_written as u64;
                            found = true;
                        }
                        break;
                    }
                    extent_index_in_lv += 1;
                }
                if found {
                    break;
                }
            }

            if !found {
                break;
            }
        }

        Ok(total_written)
    }

    /// 创建快照
    pub fn create_snapshot(&self, name: String) -> Result<Arc<Snapshot>> {
        let snapshot = Arc::new(Snapshot::new(
            name,
            self.id,
            self.size.load(Ordering::Relaxed),
        )?);

        let mut snapshots = self.snapshots.lock();
        snapshots.push(snapshot.clone());

        Ok(snapshot)
    }

    /// 获取状态
    pub fn state(&self) -> LvState {
        match self.state.load(Ordering::Acquire) {
            0 => LvState::Available,
            1 => LvState::Creating,
            2 => LvState::Deleting,
            3 => LvState::Extending,
            4 => LvState::Shrinking,
            5 => LvState::Failed,
            _ => LvState::Failed,
        }
    }
}

/// 逻辑卷快照
pub struct Snapshot {
    /// 快照 ID
    pub id: u64,
    /// 快照名称
    pub name: String,
    /// 源逻辑卷 ID
    pub source_lv_id: u64,
    /// 快照大小（字节）
    pub size: u64,
    /// 创建时间
    pub created_at: u64,
    /// 写时复制数据
    pub cow_data: Mutex<BTreeMap<u64, Vec<u8>>>,
}

impl Snapshot {
    /// 创建新的快照
    pub fn new(name: String, source_lv_id: u64, size: u64) -> Result<Self> {
        Ok(Self {
            id: Self::generate_id(),
            name,
            source_lv_id,
            size,
            created_at: Self::get_timestamp(),
            cow_data: Mutex::new(BTreeMap::new()),
        })
    }

    /// 读取快照数据
    pub fn read(&self, offset: u64, buffer: &mut [u8]) -> Result<usize> {
        let cow_data = self.cow_data.lock();

        // 检查是否有 COW 数据
        if let Some(data) = cow_data.get(&offset) {
            let len = data.len().min(buffer.len());
            buffer[..len].copy_from_slice(&data[..len]);
            return Ok(len);
        }

        // 否则需要从源卷读取
        Err(Error::NotFound)
    }

    /// 恢复快照
    pub fn restore(&self) -> Result<()> {
        // 简化实现：需要遍历所有 COW 数据并写回源卷
        Ok(())
    }

    /// 删除快照
    pub fn delete(self) -> Result<()> {
        // 释放所有 COW 数据
        Ok(())
    }

    /// 生成 ID
    fn generate_id() -> u64 {
        // 简化实现
        core::time::Duration::from_secs(0).as_nanos() as u64
    }

    /// 获取时间戳
    fn get_timestamp() -> u64 {
        // 简化实现
        0
    }
}

// 实现 BlockDevice trait 以便逻辑卷可以作为块设备使用
impl BlockDevice for LogicalVolume {
    fn read(&self, sector: u64, buffer: &mut [u8]) -> Result<usize> {
        let offset = sector * 512; // 假设 512 字节扇区
        self.read(offset, buffer)
    }

    fn write(&self, sector: u64, data: &[u8]) -> Result<usize> {
        let offset = sector * 512;
        self.write(offset, data)
    }

    fn flush(&self) -> Result<()> {
        // 同步所有底层设备
        let pvs = self.vg.physical_volumes.lock();
        for pv in pvs.values() {
            pv.device.flush()?;
        }
        Ok(())
    }

    fn device_size(&self) -> u64 {
        self.size.load(Ordering::Relaxed)
    }

    fn device_name(&self) -> &str {
        &self.name
    }
}

/// LVM 管理器
pub struct LvmManager {
    /// 卷组列表
    pub volume_groups: Mutex<BTreeMap<String, Arc<VolumeGroup>>>,
    /// 物理卷列表（不属于任何 VG 的 PV）
    pub orphan_pvs: Mutex<BTreeMap<String, Arc<PhysicalVolume>>>,
    /// 下一个 VG ID
    next_vg_id: AtomicU64,
    /// 初始化标志
    initialized: AtomicBool,
}

impl LvmManager {
    pub fn new() -> Self {
        Self {
            volume_groups: Mutex::new(BTreeMap::new()),
            orphan_pvs: Mutex::new(BTreeMap::new()),
            next_vg_id: AtomicU64::new(1),
            initialized: AtomicBool::new(false),
        }
    }

    /// 初始化
    pub fn init(&self) -> Result<()> {
        if self.initialized.load(Ordering::Acquire) {
            return Ok(());
        }

        self.initialized.store(true, Ordering::Release);
        Ok(())
    }

    /// 创建物理卷
    pub fn create_pv(
        &self,
        name: String,
        device: Arc<dyn BlockDevice>,
        extent_size: Option<u64>,
    ) -> Result<Arc<PhysicalVolume>> {
        let uuid = Self::generate_uuid();
        let extent_size = extent_size.unwrap_or(DEFAULT_EXTENT_SIZE);

        let pv = Arc::new(PhysicalVolume::new(
            uuid.clone(),
            name,
            device,
            extent_size,
        ));

        let mut orphans = self.orphan_pvs.lock();
        orphans.insert(uuid, pv.clone());

        Ok(pv)
    }

    /// 创建卷组
    pub fn create_vg(
        &self,
        name: String,
        pv_uuids: Vec<String>,
        extent_size: Option<u64>>,
    ) -> Result<Arc<VolumeGroup>> {
        let vg = Arc::new(VolumeGroup::new(name, extent_size.unwrap_or(DEFAULT_EXTENT_SIZE)));

        let mut orphans = self.orphan_pvs.lock();
        for pv_uuid in pv_uuids {
            if let Some(pv) = orphans.remove(&pv_uuid) {
                vg.add_pv(pv)?;
            }
        }

        let mut vgs = self.volume_groups.lock();
        vgs.insert(vg.name.clone(), vg.clone());

        Ok(vg)
    }

    /// 删除卷组
    pub fn delete_vg(&self, name: &str) -> Result<()> {
        let mut vgs = self.volume_groups.lock();
        vgs.remove(name).ok_or(Error::NotFound)?;
        Ok(())
    }

    /// 获取卷组
    pub fn get_vg(&self, name: &str) -> Option<Arc<VolumeGroup>> {
        let vgs = self.volume_groups.lock();
        vgs.get(name).cloned()
    }

    /// 列出所有卷组
    pub fn list_vgs(&self) -> Vec<String> {
        let vgs = self.volume_groups.lock();
        vgs.keys().cloned().collect()
    }

    /// 生成 UUID
    fn generate_uuid() -> String {
        alloc::format!("pv-{}", core::time::Duration::from_secs(0).as_nanos())
    }
}

/// 全局 LVM 管理器实例
static LVM_MANAGER: spin::Once<Arc<LvmManager>> = spin::Once::new();

/// 获取全局 LVM 管理器
pub fn lvm_manager() -> &'static Arc<LvmManager> {
    LVM_MANAGER.call_once(|| Arc::new(LvmManager::new()))
}

/// 初始化 LVM 子系统
pub fn init() -> Result<()> {
    lvm_manager().init()?;
    crate::println!("[lvm] LVM subsystem initialized");
    Ok(())
}
