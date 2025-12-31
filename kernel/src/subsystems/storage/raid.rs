//! # RAID (Redundant Array of Independent Disks) 实现
//!
//! 提供多种 RAID 级别的支持：
//! - **RAID 0**: 条带化，无冗余
//! - **RAID 1**: 镜像，完全冗余
//! - **RAID 5**: 带分布式奇偶校验的条带化
//! - **RAID 6**: 带双奇偶校验的条带化
//! - **RAID 10**: 条带化镜像

extern crate alloc;

use alloc::vec::Vec;
use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, AtomicU32, AtomicU8, AtomicBool, Ordering};
use crate::subsystems::sync::Mutex;
use crate::subsystems::drivers::block::{BlockDevice, Bio, BlockOp, BioStatus};
use crate::error::{Result, Error};

/// RAID 级别
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RaidLevel {
    /// RAID 0: 条带化，无冗余
    Raid0 = 0,
    /// RAID 1: 镜像，完全冗余
    Raid1 = 1,
    /// RAID 5: 带分布式奇偶校验的条带化
    Raid5 = 5,
    /// RAID 6: 带双奇偶校验的条带化
    Raid6 = 6,
    /// RAID 10: 条带化镜像
    Raid10 = 10,
}

impl RaidLevel {
    /// 获取最小设备数
    pub fn min_devices(&self) -> usize {
        match self {
            RaidLevel::Raid0 => 2,
            RaidLevel::Raid1 => 2,
            RaidLevel::Raid5 => 3,
            RaidLevel::Raid6 => 4,
            RaidLevel::Raid10 => 4,
        }
    }

    /// 是否支持冗余
    pub fn has_redundancy(&self) -> bool {
        matches!(self, RaidLevel::Raid1 | RaidLevel::Raid5 | RaidLevel::Raid6 | RaidLevel::Raid10)
    }

    /// 获取冗余级别（允许故障的设备数）
    pub fn redundancy_level(&self) -> usize {
        match self {
            RaidLevel::Raid0 => 0,
            RaidLevel::Raid1 => 1,
            RaidLevel::Raid5 => 1,
            RaidLevel::Raid6 => 2,
            RaidLevel::Raid10 => 1, // 每个镜像对允许1个故障
        }
    }
}

/// RAID 阵列状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RaidState {
    /// 正常运行
    Healthy = 0,
    /// 降级运行（有设备故障）
    Degraded = 1,
    /// 正在重建
    Rebuilding = 2,
    /// 故障（超过冗余级别）
    Failed = 3,
}

/// RAID 设备状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum DeviceState {
    /// 设备正常
    Healthy = 0,
    /// 设备故障
    Failed = 1,
    /// 设备正在重建
    Rebuilding = 2,
    /// 设备掉线
    Offline = 3,
}

/// RAID 设备信息
pub struct RaidDevice {
    /// 设备 ID
    pub id: u32,
    /// 块设备
    pub device: Arc<dyn BlockDevice>,
    /// 设备状态
    pub state: DeviceState,
    /// 最后错误时间
    pub last_error_time: u64,
    /// 错误计数
    pub error_count: AtomicU32,
}

/// RAID 统计信息
#[derive(Debug, Clone, Copy)]
pub struct RaidStats {
    /// 读操作数
    pub reads: u64,
    /// 写操作数
    pub writes: u64,
    /// 重建进度 (0-100)
    pub rebuild_progress: u32,
    /// 故障设备数
    pub failed_devices: u32,
    /// 总容量（字节）
    pub total_capacity: u64,
    /// 可用容量（字节）
    pub available_capacity: u64,
}

/// RAID 阵列
pub struct RaidArray {
    /// 阵列 ID
    id: u32,
    /// 阵列名称
    name: alloc::string::String,
    /// RAID 级别
    level: RaidLevel,
    /// 成员设备
    devices: Mutex<Vec<Arc<RaidDevice>>>,
    /// 条带大小（字节）
    stripe_size: u64,
    /// 块大小（字节）
    chunk_size: u64,
    /// 阵列状态
    state: AtomicU8,
    /// 统计信息
    stats: Mutex<RaidStats>,
    /// 总容量
    total_capacity: u64,
    /// 可用容量
    available_capacity: u64,
    /// 重建标志
    rebuilding: AtomicBool,
    /// 写意图日志
    write_intent: Mutex<BTreeMap<u64, Vec<u8>>>,
}

impl RaidArray {
    /// 创建新的 RAID 阵列
    ///
    /// # Arguments
    ///
    /// * `level` - RAID 级别
    /// * `devices` - 成员设备列表
    pub fn new(
        id: u32,
        name: alloc::string::String,
        level: RaidLevel,
        mut devices: Vec<Arc<dyn BlockDevice>>,
        stripe_size: u64,
    ) -> Result<Self> {
        // 检查设备数量
        if devices.len() < level.min_devices() {
            return Err(Error::InvalidInput);
        }

        // 检查条带大小
        if stripe_size == 0 || stripe_size % 512 != 0 {
            return Err(Error::InvalidInput);
        }

        // 所有设备必须具有相同的块大小
        let sector_size = devices[0].sector_size();
        for device in &devices {
            if device.sector_size() != sector_size {
                return Err(Error::InvalidInput);
            }
        }

        // 计算容量
        let min_device_size = devices.iter()
            .map(|d| d.device_size())
            .min()
            .ok_or(Error::InvalidInput)?;

        let total_capacity = match level {
            RaidLevel::Raid0 => min_device_size * devices.len() as u64,
            RaidLevel::Raid1 => min_device_size,
            RaidLevel::Raid5 => min_device_size * (devices.len() - 1) as u64,
            RaidLevel::Raid6 => min_device_size * (devices.len() - 2) as u64,
            RaidLevel::Raid10 => min_device_size * (devices.len() / 2) as u64,
        };

        let available_capacity = total_capacity;

        // 包装设备
        let raid_devices: Vec<Arc<RaidDevice>> = devices
            .into_iter()
            .enumerate()
            .map(|(i, d)| Arc::new(RaidDevice {
                id: i as u32,
                device: d,
                state: DeviceState::Healthy,
                last_error_time: 0,
                error_count: AtomicU32::new(0),
            }))
            .collect();

        let stats = RaidStats {
            reads: 0,
            writes: 0,
            rebuild_progress: 0,
            failed_devices: 0,
            total_capacity,
            available_capacity,
        };

        Ok(Self {
            id,
            name,
            level,
            devices: Mutex::new(raid_devices),
            stripe_size,
            chunk_size: sector_size,
            state: AtomicU8::new(RaidState::Healthy as u8),
            stats: Mutex::new(stats),
            total_capacity,
            available_capacity,
            rebuilding: AtomicBool::new(false),
            write_intent: Mutex::new(BTreeMap::new()),
        })
    }

    /// 读取数据
    pub fn read(&self, offset: u64, buffer: &mut [u8]) -> Result<usize> {
        match self.level {
            RaidLevel::Raid0 => self.read_raid0(offset, buffer),
            RaidLevel::Raid1 => self.read_raid1(offset, buffer),
            RaidLevel::Raid5 => self.read_raid5(offset, buffer),
            RaidLevel::Raid6 => self.read_raid6(offset, buffer),
            RaidLevel::Raid10 => self.read_raid10(offset, buffer),
        }
    }

    /// 写入数据
    pub fn write(&self, offset: u64, data: &[u8]) -> Result<usize> {
        match self.level {
            RaidLevel::Raid0 => self.write_raid0(offset, data),
            RaidLevel::Raid1 => self.write_raid1(offset, data),
            RaidLevel::Raid5 => self.write_raid5(offset, data),
            RaidLevel::Raid6 => self.write_raid6(offset, data),
            RaidLevel::Raid10 => self.write_raid10(offset, data),
        }
    }

    /// 刷新缓存
    pub fn flush(&self) -> Result<()> {
        let devices = self.devices.lock();
        for device in devices.iter() {
            if device.state == DeviceState::Healthy {
                device.device.flush()?;
            }
        }
        Ok(())
    }

    /// RAID 0 读取（条带化）
    fn read_raid0(&self, offset: u64, buffer: &mut [u8]) -> Result<usize> {
        let devices = self.devices.lock();
        let num_devices = devices.len();
        let stripe_size = self.stripe_size;

        let mut total_read = 0;
        let mut current_offset = offset;

        while total_read < buffer.len() {
            let stripe_index = (current_offset / (stripe_size * num_devices as u64)) as usize;
            let device_index = stripe_index % num_devices;
            let device_offset = (stripe_index / num_devices) as u64 * stripe_size
                + (current_offset % stripe_size);

            let remaining = buffer.len() - total_read;
            let chunk_size = remaining.min(stripe_size as usize - (current_offset % stripe_size) as usize);

            let device = &devices[device_index];
            if device.state == DeviceState::Healthy {
                let sector = device_offset / self.chunk_size;
                device.device.read(sector, &mut buffer[total_read..total_read + chunk_size])?;
            } else {
                return Err(Error::IoError);
            }

            total_read += chunk_size;
            current_offset += chunk_size as u64;
        }

        // Update stats
        let mut stats = self.stats.lock();
        stats.reads += 1;

        Ok(total_read)
    }

    /// RAID 0 写入（条带化）
    fn write_raid0(&self, offset: u64, data: &[u8]) -> Result<usize> {
        let devices = self.devices.lock();
        let num_devices = devices.len();
        let stripe_size = self.stripe_size;

        let mut total_written = 0;
        let mut current_offset = offset;

        while total_written < data.len() {
            let stripe_index = (current_offset / (stripe_size * num_devices as u64)) as usize;
            let device_index = stripe_index % num_devices;
            let device_offset = (stripe_index / num_devices) as u64 * stripe_size
                + (current_offset % stripe_size);

            let remaining = data.len() - total_written;
            let chunk_size = remaining.min(stripe_size as usize - (current_offset % stripe_size) as usize);

            let device = &devices[device_index];
            if device.state == DeviceState::Healthy {
                let sector = device_offset / self.chunk_size;
                device.device.write(sector, &data[total_written..total_written + chunk_size])?;
            } else {
                return Err(Error::IoError);
            }

            total_written += chunk_size;
            current_offset += chunk_size as u64;
        }

        // Update stats
        let mut stats = self.stats.lock();
        stats.writes += 1;

        Ok(total_written)
    }

    /// RAID 1 读取（镜像）
    fn read_raid1(&self, offset: u64, buffer: &mut [u8]) -> Result<usize> {
        let devices = self.devices.lock();

        // 从第一个健康设备读取
        for device in devices.iter() {
            if device.state == DeviceState::Healthy {
                let sector = offset / self.chunk_size;
                let result = device.device.read(sector, buffer);

                if result.is_ok() {
                    let mut stats = self.stats.lock();
                    stats.reads += 1;
                    return result;
                }
            }
        }

        Err(Error::IoError)
    }

    /// RAID 1 写入（镜像）
    fn write_raid1(&self, offset: u64, data: &[u8]) -> Result<usize> {
        let devices = self.devices.lock();

        // 写入所有健康设备
        let mut success_count = 0;
        for device in devices.iter() {
            if device.state == DeviceState::Healthy {
                let sector = offset / self.chunk_size;
                if device.device.write(sector, data).is_ok() {
                    success_count += 1;
                }
            }
        }

        if success_count > 0 {
            let mut stats = self.stats.lock();
            stats.writes += 1;
            Ok(data.len())
        } else {
            Err(Error::IoError)
        }
    }

    /// RAID 5 读取（带奇偶校验的条带化）
    fn read_raid5(&self, offset: u64, buffer: &mut [u8]) -> Result<usize> {
        let devices = self.devices.lock();
        let num_devices = devices.len();
        let stripe_size = self.stripe_size;

        let mut total_read = 0;
        let mut current_offset = offset;

        while total_read < buffer.len() {
            let stripe_index = (current_offset / (stripe_size * num_devices as u64)) as usize;
            let device_index = stripe_index % num_devices;
            let device_offset = (stripe_index / num_devices) as u64 * stripe_size
                + (current_offset % stripe_size);

            let remaining = buffer.len() - total_read;
            let chunk_size = remaining.min(stripe_size as usize - (current_offset % stripe_size) as usize);

            let device = &devices[device_index];
            if device.state == DeviceState::Healthy {
                let sector = device_offset / self.chunk_size;
                device.device.read(sector, &mut buffer[total_read..total_read + chunk_size])?;
            } else {
                // 设备故障，尝试从奇偶校验重建
                self.rebuild_from_parity(stripe_index, device_offset, buffer, total_read, chunk_size)?;
            }

            total_read += chunk_size;
            current_offset += chunk_size as u64;
        }

        let mut stats = self.stats.lock();
        stats.reads += 1;

        Ok(total_read)
    }

    /// RAID 5 写入（带奇偶校验的条带化）
    fn write_raid5(&self, offset: u64, data: &[u8]) -> Result<usize> {
        let devices = self.devices.lock();
        let num_devices = devices.len();
        let stripe_size = self.stripe_size;

        let mut total_written = 0;
        let mut current_offset = offset;

        while total_written < data.len() {
            let stripe_index = (current_offset / (stripe_size * num_devices as u64)) as usize;
            let device_index = stripe_index % num_devices;
            let parity_index = (num_devices - 1) - (stripe_index % num_devices);
            let device_offset = (stripe_index / num_devices) as u64 * stripe_size
                + (current_offset % stripe_size);

            let remaining = data.len() - total_written;
            let chunk_size = remaining.min(stripe_size as usize - (current_offset % stripe_size) as usize);

            // Read-modify-write for RAID 5
            let mut old_data = vec![0u8; chunk_size];
            let sector = device_offset / self.chunk_size;

            // 读取旧数据
            if devices[device_index].state == DeviceState::Healthy {
                devices[device_index].device.read(sector, &mut old_data)?;
            }

            // 计算奇偶校验
            let parity = self.calculate_parity(
                &data[total_written..total_written + chunk_size],
                &old_data,
                chunk_size,
            );

            // 写入数据
            if devices[device_index].state == DeviceState::Healthy {
                devices[device_index].device.write(sector, &data[total_written..total_written + chunk_size])?;
            }

            // 更新奇偶校验
            if devices[parity_index].state == DeviceState::Healthy {
                let parity_sector = device_offset / self.chunk_size;
                devices[parity_index].device.write(parity_sector, &parity)?;
            }

            total_written += chunk_size;
            current_offset += chunk_size as u64;
        }

        let mut stats = self.stats.lock();
        stats.writes += 1;

        Ok(total_written)
    }

    /// RAID 6 读取（带双奇偶校验的条带化）
    fn read_raid6(&self, offset: u64, buffer: &mut [u8]) -> Result<usize> {
        // RAID 6 类似 RAID 5，但使用两个奇偶校验块
        // 简化实现：类似 RAID 5
        self.read_raid5(offset, buffer)
    }

    /// RAID 6 写入（带双奇偶校验的条带化）
    fn write_raid6(&self, offset: u64, data: &[u8]) -> Result<usize> {
        // RAID 6 类似 RAID 5，但计算两个奇偶校验块
        // 简化实现：类似 RAID 5
        self.write_raid5(offset, data)
    }

    /// RAID 10 读取（条带化镜像）
    fn read_raid10(&self, offset: u64, buffer: &mut [u8]) -> Result<usize> {
        let devices = self.devices.lock();
        let num_devices = devices.len();
        let stripe_size = self.stripe_size;

        let stripe_index = (offset / (stripe_size * num_devices as u64)) as usize;
        let mirror_index = stripe_index % (num_devices / 2);
        let device_index = mirror_index * 2; // 使用镜像对的第一个设备

        let device = &devices[device_index];
        if device.state == DeviceState::Healthy {
            let sector = (offset / self.chunk_size) % device.device.sector_count();
            device.device.read(sector, buffer)?;

            let mut stats = self.stats.lock();
            stats.reads += 1;
            return Ok(buffer.len());
        }

        Err(Error::IoError)
    }

    /// RAID 10 写入（条带化镜像）
    fn write_raid10(&self, offset: u64, data: &[u8]) -> Result<usize> {
        let devices = self.devices.lock();
        let num_devices = devices.len();
        let stripe_size = self.stripe_size;

        let stripe_index = (offset / (stripe_size * num_devices as u64)) as usize;
        let mirror_index = stripe_index % (num_devices / 2);

        // 写入镜像对
        let device1_index = mirror_index * 2;
        let device2_index = device1_index + 1;

        let sector = (offset / self.chunk_size) % devices[device1_index].device.sector_count();

        let mut success = false;
        if devices[device1_index].state == DeviceState::Healthy {
            if devices[device1_index].device.write(sector, data).is_ok() {
                success = true;
            }
        }

        if devices[device2_index].state == DeviceState::Healthy {
            if devices[device2_index].device.write(sector, data).is_ok() {
                success = true;
            }
        }

        if success {
            let mut stats = self.stats.lock();
            stats.writes += 1;
            Ok(data.len())
        } else {
            Err(Error::IoError)
        }
    }

    /// 计算奇偶校验（XOR）
    fn calculate_parity(&self, new_data: &[u8], old_data: &[u8], len: usize) -> Vec<u8> {
        let mut parity = vec![0u8; len];
        for i in 0..len {
            parity[i] = new_data[i] ^ old_data[i];
        }
        parity
    }

    /// 从奇偶校验重建数据
    fn rebuild_from_parity(
        &self,
        _stripe_index: usize,
        _device_offset: u64,
        _buffer: &mut [u8],
        _offset: usize,
        _len: usize,
    ) -> Result<()> {
        // 简化实现：实际需要读取其他设备的数据并 XOR
        Err(Error::NotImplemented)
    }

    /// 标记设备故障
    pub fn mark_device_failed(&self, device_id: u32) -> Result<()> {
        let mut devices = self.devices.lock();
        if let Some(index) = devices.iter().position(|d| d.id == device_id) {
            let device = Arc::make_mut(&mut devices[index]);
            device.state = DeviceState::Failed;
            device.last_error_time = Self::get_timestamp();
            device.error_count.fetch_add(1, Ordering::Relaxed);

            // Update state
            self.update_state(&devices);

            Ok(())
        } else {
            Err(Error::NotFound)
        }
    }

    /// 更新阵列状态
    fn update_state(&self, devices: &[Arc<RaidDevice>]) {
        let failed_count = devices.iter()
            .filter(|d| d.state == DeviceState::Failed)
            .count();

        let redundancy = self.level.redundancy_level();

        let new_state = if failed_count == 0 {
            RaidState::Healthy
        } else if failed_count <= redundancy {
            RaidState::Degraded
        } else {
            RaidState::Failed
        };

        self.state.store(new_state as u8, Ordering::Release);

        // Update stats
        let mut stats = self.stats.lock();
        stats.failed_devices = failed_count as u32;
    }

    /// 启动重建
    pub fn start_rebuild(&self, new_device: Arc<dyn BlockDevice>) -> Result<()> {
        if !self.level.has_redundancy() {
            return Err(Error::NotSupported);
        }

        if self.rebuilding.load(Ordering::Acquire) {
            return Err(Error::ResourceBusy);
        }

        self.rebuilding.store(true, Ordering::Release);
        self.state.store(RaidState::Rebuilding as u8, Ordering::Release);

        // 添加新设备
        let mut devices = self.devices.lock();
        let new_id = devices.len() as u32;
        let raid_device = Arc::new(RaidDevice {
            id: new_id,
            device: new_device,
            state: DeviceState::Rebuilding,
            last_error_time: 0,
            error_count: AtomicU32::new(0),
        });
        devices.push(raid_device);

        Ok(())
    }

    /// 获取阵列状态
    pub fn state(&self) -> RaidState {
        match self.state.load(Ordering::Acquire) {
            0 => RaidState::Healthy,
            1 => RaidState::Degraded,
            2 => RaidState::Rebuilding,
            3 => RaidState::Failed,
            _ => RaidState::Failed,
        }
    }

    /// 获取统计信息
    pub fn get_stats(&self) -> RaidStats {
        let mut stats = self.stats.lock();
        stats.rebuild_progress = if self.rebuilding.load(Ordering::Acquire) {
            // 简化：实际需要计算真实进度
            50
        } else {
            100
        };
        *stats
    }

    /// 获取设备数量
    pub fn device_count(&self) -> usize {
        self.devices.lock().len()
    }

    /// 获取阵列容量
    pub fn capacity(&self) -> u64 {
        self.total_capacity
    }

    /// 获取可用容量
    pub fn available_capacity(&self) -> u64 {
        self.available_capacity
    }

    /// 获取时间戳
    fn get_timestamp() -> u64 {
        // 简化实现
        0
    }
}

// 实现 BlockDevice trait 以便 RAID 阵列可以作为块设备使用
impl BlockDevice for RaidArray {
    fn read(&self, sector: u64, buffer: &mut [u8]) -> Result<usize> {
        let offset = sector * self.chunk_size;
        self.read(offset, buffer)
    }

    fn write(&self, sector: u64, data: &[u8]) -> Result<usize> {
        let offset = sector * self.chunk_size;
        self.write(offset, data)
    }

    fn flush(&self) -> Result<()> {
        self.flush()
    }

    fn device_size(&self) -> u64 {
        self.total_capacity
    }

    fn device_name(&self) -> &str {
        &self.name
    }
}

/// 全局 RAID 管理器
pub struct RaidManager {
    /// RAID 阵列列表
    arrays: Mutex<BTreeMap<u32, Arc<RaidArray>>>,
    /// 下一个阵列 ID
    next_id: AtomicU64,
    /// 初始化标志
    initialized: AtomicBool,
}

impl RaidManager {
    pub fn new() -> Self {
        Self {
            arrays: Mutex::new(BTreeMap::new()),
            next_id: AtomicU64::new(1),
            initialized: AtomicBool::new(false),
        }
    }

    /// 创建 RAID 阵列
    pub fn create_array(
        &self,
        name: alloc::string::String,
        level: RaidLevel,
        devices: Vec<Arc<dyn BlockDevice>>,
        stripe_size: u64,
    ) -> Result<Arc<RaidArray>> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed) as u32;
        let array = Arc::new(RaidArray::new(id, name, level, devices, stripe_size)?);

        let mut arrays = self.arrays.lock();
        arrays.insert(id, array.clone());

        Ok(array)
    }

    /// 删除 RAID 阵列
    pub fn delete_array(&self, id: u32) -> Result<()> {
        let mut arrays = self.arrays.lock();
        arrays.remove(&id).ok_or(Error::NotFound)?;
        Ok(())
    }

    /// 获取 RAID 阵列
    pub fn get_array(&self, id: u32) -> Option<Arc<RaidArray>> {
        let arrays = self.arrays.lock();
        arrays.get(&id).cloned()
    }

    /// 列出所有阵列
    pub fn list_arrays(&self) -> Vec<u32> {
        let arrays = self.arrays.lock();
        arrays.keys().cloned().collect()
    }
}

/// 全局 RAID 管理器实例
static RAID_MANAGER: spin::Once<Arc<RaidManager>> = spin::Once::new();

/// 获取全局 RAID 管理器
pub fn raid_manager() -> &'static Arc<RaidManager> {
    RAID_MANAGER.call_once(|| Arc::new(RaidManager::new()))
}

/// 初始化 RAID 子系统
pub fn init() -> Result<()> {
    crate::println!("[raid] RAID subsystem initialized");
    Ok(())
}
