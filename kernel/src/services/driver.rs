// Device Driver Service
//
// 设备驱动管理服务
// 提供设备驱动注册、管理和设备抽象功能

extern crate alloc;
use crate::microkernel::ipc::MessageQueue;
use core::sync::atomic::AtomicU32;

use crate::types::stubs::{Message, MessageType, send_message, receive_message, BlockDevice, get_service_registry};
use crate::microkernel::service_registry::{ServiceId, ServiceInfo, InterfaceVersion, ServiceCategory};
// TODO: Implement device driver types
// use crate::drivers::{BlockDevice, CharDevice, NetworkDevice, Device};
use crate::reliability::errno::{EINVAL, ENOENT, EEXIST, ENOMEM, EIO, ENODEV};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
use alloc::string::String;
use alloc::string::ToString;
use spin::Mutex;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

/// 设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeviceType {
    Block,      // 块设备
    Char,       // 字符设备
    Network,    // 网络设备
    Graphics,   // 图形设备
    Audio,      // 音频设备
    Input,      // 输入设备
    Memory,     // 内存设备
    Bus,        // 总线设备
    Misc,       // 其他设备
}

/// 设备状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    Unknown,    // 未知状态
    Detached,   // 已分离
    Attaching,  // 正在连接
    Attached,   // 已连接
    Powered,    // 已供电
    Configured, // 已配置
    Suspended,  // 已暂停
    Error,      // 错误状态
}

/// 设备驱动信息
#[derive(Debug, Clone)]
pub struct DriverInfo {
    /// 驱动名称
    pub name: String,
    /// 驱动版本
    pub version: String,
    /// 驱动厂商
    pub vendor: String,
    /// 支持的设备类型
    pub device_types: Vec<DeviceType>,
    /// 驱动状态
    pub status: DriverStatus,
    /// 支持的设备ID列表
    pub supported_device_ids: Vec<[u32; 2]>, // [vendor_id, device_id]
    /// 驱动函数表
    pub function_table: DriverFunctionTable,
}

/// 驱动状态
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverStatus {
    Unloaded,   // 未加载
    Loading,    // 加载中
    Loaded,     // 已加载
    Active,     // 活跃
    Error,      // 错误
}

/// 驱动函数表
#[derive(Debug, Clone)]
pub struct DriverFunctionTable {
    /// 初始化函数
    pub init_fn: Option<usize>,
    /// 清理函数
    pub cleanup_fn: Option<usize>,
    /// 打开设备函数
    pub open_fn: Option<usize>,
    /// 关闭设备函数
    pub close_fn: Option<usize>,
    /// 读设备函数
    pub read_fn: Option<usize>,
    /// 写设备函数
    pub write_fn: Option<usize>,
    /// 控制设备函数
    pub ioctl_fn: Option<usize>,
}

/// 设备信息
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// 设备ID
    pub device_id: u32,
    /// 设备名称
    pub name: String,
    /// 设备类型
    pub device_type: DeviceType,
    /// 设备状态
    pub status: DeviceStatus,
    /// 设备主设备号
    pub major: u32,
    /// 设备次设备号
    pub minor: u32,
    /// 设备厂商ID
    pub vendor_id: Option<u32>,
    /// 设备ID
    pub product_id: Option<u32>,
    /// 设备类
    pub device_class: String,
    /// 设备子类
    pub device_subclass: String,
    /// 驱动名称
    pub driver_name: Option<String>,
    /// 设备资源
    pub resources: DeviceResources,
    /// 设备统计信息
    pub stats: DeviceStats,
}

/// 设备资源
#[derive(Debug, Clone)]
pub struct DeviceResources {
    /// 内存资源
    pub memory_regions: Vec<MemoryRegion>,
    /// I/O端口资源
    pub io_ports: Vec<IOPortRange>,
    /// IRQ资源
    pub irqs: Vec<u32>,
    /// DMA通道
    pub dma_channels: Vec<u32>,
    /// 时钟资源
    pub clocks: Vec<String>,
}

/// 内存区域
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// 起始地址
    pub start: usize,
    /// 大小
    pub size: usize,
    /// 类型
    pub region_type: MemoryRegionType,
    /// 权限
    pub permissions: u32,
}

/// 内存区域类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    MMIO,       // 内存映射I/O
    SystemRAM,  // 系统内存
    DeviceRAM,  // 设备内存
    Reserved,   // 保留区域
}

/// I/O端口范围
#[derive(Debug, Clone)]
pub struct IOPortRange {
    /// 起始端口
    pub start: u16,
    /// 端口数量
    pub count: u16,
}

/// 设备统计信息
#[derive(Debug)]
pub struct DeviceStats {
    /// 读取操作次数
    pub read_operations: AtomicU64,
    /// 写入操作次数
    pub write_operations: AtomicU64,
    /// 读取字节数
    pub bytes_read: AtomicU64,
    /// 写入字节数
    pub bytes_written: AtomicU64,
    /// 错误次数
    pub errors: AtomicU64,
    /// 中断次数
    pub interrupts: AtomicU64,
}

impl Clone for DeviceStats {
    fn clone(&self) -> Self {
        Self {
            read_operations: AtomicU64::new(self.read_operations.load(Ordering::Relaxed)),
            write_operations: AtomicU64::new(self.write_operations.load(Ordering::Relaxed)),
            bytes_read: AtomicU64::new(self.bytes_read.load(Ordering::Relaxed)),
            bytes_written: AtomicU64::new(self.bytes_written.load(Ordering::Relaxed)),
            errors: AtomicU64::new(self.errors.load(Ordering::Relaxed)),
            interrupts: AtomicU64::new(self.interrupts.load(Ordering::Relaxed)),
        }
    }
}

impl DeviceStats {
    /// 创建新的设备统计信息
    pub const fn new() -> Self {
        Self {
            read_operations: AtomicU64::new(0),
            write_operations: AtomicU64::new(0),
            bytes_read: AtomicU64::new(0),
            bytes_written: AtomicU64::new(0),
            errors: AtomicU64::new(0),
            interrupts: AtomicU64::new(0),
        }
    }

    /// 更新读操作统计
    pub fn update_read_stats(&self, bytes: u64, errors: u64) {
        self.read_operations.fetch_add(1, Ordering::Relaxed);
        self.bytes_read.fetch_add(bytes, Ordering::Relaxed);
        if errors > 0 {
            self.errors.fetch_add(errors, Ordering::Relaxed);
        }
    }

    /// 更新写操作统计
    pub fn update_write_stats(&self, bytes: u64, errors: u64) {
        self.write_operations.fetch_add(1, Ordering::Relaxed);
        self.bytes_written.fetch_add(bytes, Ordering::Relaxed);
        if errors > 0 {
            self.errors.fetch_add(errors, Ordering::Relaxed);
        }
    }

    /// 更新中断统计
    pub fn update_interrupt_stats(&self, count: u64) {
        self.interrupts.fetch_add(count, Ordering::Relaxed);
    }
}

/// 设备驱动服务
pub struct DriverService {
    /// 服务ID
    service_id: ServiceId,
    /// 消息队列
    message_queue: Arc<Mutex<MessageQueue>>,
    /// 注册的驱动
    drivers: Arc<Mutex<BTreeMap<String, DriverInfo>>>,
    /// 注册的设备
    devices: Arc<Mutex<BTreeMap<u32, DeviceInfo>>>,
    /// 设备类型索引
    device_type_index: Arc<Mutex<BTreeMap<DeviceType, Vec<u32>>>>,
    /// 设备ID生成器
    next_device_id: AtomicU32,
    /// 驱动统计信息
    driver_stats: Arc<Mutex<DriverStats>>,
}

/// 驱动统计信息
#[derive(Debug)]
pub struct DriverStats {
    /// 总驱动数
    pub total_drivers: AtomicUsize,
    /// 活跃驱动数
    pub active_drivers: AtomicUsize,
    /// 总设备数
    pub total_devices: AtomicUsize,
    /// 已配置设备数
    pub configured_devices: AtomicUsize,
    /// 错误设备数
    pub error_devices: AtomicUsize,
}

impl DriverStats {
    /// 创建新的驱动统计信息
    pub const fn new() -> Self {
        Self {
            total_drivers: AtomicUsize::new(0),
            active_drivers: AtomicUsize::new(0),
            total_devices: AtomicUsize::new(0),
            configured_devices: AtomicUsize::new(0),
            error_devices: AtomicUsize::new(0),
        }
    }
}

impl DriverService {
    /// 创建新的设备驱动服务
    pub fn new() -> Result<Self, &'static str> {
        let service_id = 3; // 固定ID用于设备驱动服务
        let message_queue = Arc::new(Mutex::new(MessageQueue::new(service_id, service_id, 1024, 4096)));

        let service = Self {
            service_id,
            message_queue,
            drivers: Arc::new(Mutex::new(BTreeMap::new())),
            devices: Arc::new(Mutex::new(BTreeMap::new())),
            device_type_index: Arc::new(Mutex::new(BTreeMap::new())),
            next_device_id: AtomicU32::new(1),
            driver_stats: Arc::new(Mutex::new(DriverStats::new())),
        };

        Ok(service)
    }

    /// 注册设备驱动
    pub fn register_driver(&self, driver_info: DriverInfo) -> Result<(), &'static str> {
        let mut drivers = self.drivers.lock();

        if drivers.contains_key(&driver_info.name) {
            return Err("Driver already registered");
        }

        let driver_name = driver_info.name.clone();
        drivers.insert(driver_name.clone(), driver_info);

        // 更新统计信息
        {
            let stats = self.driver_stats.lock();
            stats.total_drivers.fetch_add(1, Ordering::Relaxed);
        }

        crate::println!("[driver] Registered driver: {}", driver_name);
        Ok(())
    }

    /// 注销设备驱动
    pub fn unregister_driver(&self, driver_name: &str) -> Result<(), &'static str> {
        let mut drivers = self.drivers.lock();

        if drivers.remove(driver_name).is_none() {
            return Err("Driver not found");
        }

        // 更新统计信息
        {
            let stats = self.driver_stats.lock();
            stats.total_drivers.fetch_sub(1, Ordering::Relaxed);
        }

        crate::println!("[driver] Unregistered driver: {}", driver_name);
        Ok(())
    }

    /// 注册设备
    pub fn register_device(&self, mut device_info: DeviceInfo) -> Result<u32, &'static str> {
        let device_id = self.next_device_id.fetch_add(1, Ordering::SeqCst);
        device_info.device_id = device_id;

        {
            let mut devices = self.devices.lock();
            devices.insert(device_id, device_info.clone());
        }

        // 更新设备类型索引
        {
            let mut type_index = self.device_type_index.lock();
            type_index.entry(device_info.device_type)
                .or_insert_with(Vec::new)
                .push(device_id);
        }

        // 更新统计信息
        {
            let stats = self.driver_stats.lock();
            stats.total_devices.fetch_add(1, Ordering::Relaxed);
        }

        crate::println!("[driver] Registered device: {} (ID: {}, Type: {:?})",
                 device_info.name, device_id, device_info.device_type);

        Ok(device_id)
    }

    /// 注销设备
    pub fn unregister_device(&self, device_id: u32) -> Result<(), &'static str> {
        let mut devices = self.devices.lock();

        if let Some(device) = devices.remove(&device_id) {
            // 更新设备类型索引
            {
                let mut type_index = self.device_type_index.lock();
                if let Some(device_list) = type_index.get_mut(&device.device_type) {
                    device_list.retain(|&id| id != device_id);
                    if device_list.is_empty() {
                        type_index.remove(&device.device_type);
                    }
                }
            }

            // 更新统计信息
            {
                let stats = self.driver_stats.lock();
                stats.total_devices.fetch_sub(1, Ordering::Relaxed);
            }

            crate::println!("[driver] Unregistered device: {} (ID: {})", device.name, device_id);
            Ok(())
        } else {
            Err("Device not found")
        }
    }

    /// 获取设备信息
    pub fn get_device_info(&self, device_id: u32) -> Option<DeviceInfo> {
        let devices = self.devices.lock();
        devices.get(&device_id).cloned()
    }

    /// 获取所有设备
    pub fn get_all_devices(&self) -> Vec<DeviceInfo> {
        let devices = self.devices.lock();
        devices.values().cloned().collect()
    }

    /// 按类型获取设备
    pub fn get_devices_by_type(&self, device_type: DeviceType) -> Vec<DeviceInfo> {
        let devices = self.devices.lock();
        devices.values()
            .filter(|device| device.device_type == device_type)
            .cloned()
            .collect()
    }

    /// 获取驱动信息
    pub fn get_driver_info(&self, driver_name: &str) -> Option<DriverInfo> {
        let drivers = self.drivers.lock();
        drivers.get(driver_name).cloned()
    }

    /// 获取所有驱动
    pub fn get_all_drivers(&self) -> Vec<DriverInfo> {
        let drivers = self.drivers.lock();
        drivers.values().cloned().collect()
    }

    /// 绑定设备到驱动
    pub fn bind_device_to_driver(&self, device_id: u32, driver_name: &str) -> Result<(), &'static str> {
        let mut devices = self.devices.lock();

        if let Some(device) = devices.get_mut(&device_id) {
            // 检查驱动是否存在
            {
                let drivers = self.drivers.lock();
                if !drivers.contains_key(driver_name) {
                    return Err("Driver not found");
                }
            }

            device.driver_name = Some(driver_name.to_string());
            device.status = DeviceStatus::Attached;

            // 更新统计信息
            {
                let stats = self.driver_stats.lock();
                stats.configured_devices.fetch_add(1, Ordering::Relaxed);
            }

            crate::println!("[driver] Bound device {} to driver {}",
                     device.name, driver_name);
            Ok(())
        } else {
            Err("Device not found")
        }
    }

    /// 解绑设备驱动
    pub fn unbind_device(&self, device_id: u32) -> Result<(), &'static str> {
        let mut devices = self.devices.lock();

        if let Some(device) = devices.get_mut(&device_id) {
            if let Some(driver_name) = &device.driver_name.clone() {
                device.driver_name = None;
                device.status = DeviceStatus::Detached;

                // 更新统计信息
                {
                    let stats = self.driver_stats.lock();
                    stats.configured_devices.fetch_sub(1, Ordering::Relaxed);
                }

                crate::println!("[driver] Unbound device {} from driver {}",
                         device.name, driver_name);
                Ok(())
            } else {
                Err("Device is not bound to any driver")
            }
        } else {
            Err("Device not found")
        }
    }

    /// 更新设备状态
    pub fn update_device_status(&self, device_id: u32, status: DeviceStatus) -> Result<(), &'static str> {
        let mut devices = self.devices.lock();

        if let Some(device) = devices.get_mut(&device_id) {
            let old_status = device.status;
            device.status = status;

            // 更新统计信息
            {
                let stats = self.driver_stats.lock();
                if status == DeviceStatus::Error && old_status != DeviceStatus::Error {
                    stats.error_devices.fetch_add(1, Ordering::Relaxed);
                } else if status != DeviceStatus::Error && old_status == DeviceStatus::Error {
                    stats.error_devices.fetch_sub(1, Ordering::Relaxed);
                }
            }

            crate::println!("[driver] Device {} status changed from {:?} to {:?}",
                     device.name, old_status, status);
            Ok(())
        } else {
            Err("Device not found")
        }
    }

    /// 获取驱动统计信息
    pub fn get_stats(&self) -> DriverStatsSnapshot {
        let stats = self.driver_stats.lock();
        DriverStatsSnapshot {
            total_drivers: stats.total_drivers.load(Ordering::Relaxed),
            active_drivers: stats.active_drivers.load(Ordering::Relaxed),
            total_devices: stats.total_devices.load(Ordering::Relaxed),
            configured_devices: stats.configured_devices.load(Ordering::Relaxed),
            error_devices: stats.error_devices.load(Ordering::Relaxed),
        }
    }

    /// 获取服务ID
    pub fn get_service_id(&self) -> ServiceId {
        self.service_id
    }
}

/// 驱动统计信息快照
#[derive(Debug, Clone)]
pub struct DriverStatsSnapshot {
    pub total_drivers: usize,
    pub active_drivers: usize,
    pub total_devices: usize,
    pub configured_devices: usize,
    pub error_devices: usize,
}

/// 设备驱动服务管理器
pub struct DriverManager {
    /// 设备驱动服务实例
    service: Arc<DriverService>,
    /// 是否已初始化
    initialized: bool,
}

impl DriverManager {
    /// 创建新的设备驱动管理器
    pub fn new() -> Result<Self, &'static str> {
        let service = Arc::new(DriverService::new()?);

        Ok(Self {
            service,
            initialized: false,
        })
    }

    /// 初始化设备驱动服务
    pub fn initialize(&mut self) -> Result<(), &'static str> {
        if self.initialized {
            return Ok(());
        }

        // 注册到服务注册表
        let registry = get_service_registry().ok_or("Service registry not initialized")?;
        let service_info = ServiceInfo::new(
            self.service.get_service_id(),
            "DriverService".to_string(),
            "Device driver management and abstraction service".to_string(),
            ServiceCategory::Device,
            InterfaceVersion::new(1, 0, 0),
            0, // owner_id - kernel owned
        );

        registry.register_service(service_info).map_err(|_| "Failed to register service")?;

        self.initialized = true;
        crate::println!("[driver] Device driver service initialized");
        Ok(())
    }

    /// 获取设备驱动服务引用
    pub fn get_service(&self) -> Arc<DriverService> {
        self.service.clone()
    }
}

// 全局设备驱动管理器实例
static mut DRIVER_MANAGER: Option<DriverManager> = None;
static DRIVER_MANAGER_INIT: core::sync::atomic::AtomicBool = core::sync::atomic::AtomicBool::new(false);

/// 初始化设备驱动服务
pub fn init() -> Result<(), &'static str> {
    if DRIVER_MANAGER_INIT.load(core::sync::atomic::Ordering::Relaxed) {
        return Ok(());
    }

    unsafe {
        let mut manager = DriverManager::new()?;
        manager.initialize()?;
        DRIVER_MANAGER = Some(manager);
    }

    DRIVER_MANAGER_INIT.store(true, core::sync::atomic::Ordering::Relaxed);
    Ok(())
}

/// 获取全局设备驱动服务
pub fn get_driver_service() -> Option<Arc<DriverService>> {
    unsafe {
        DRIVER_MANAGER.as_ref().map(|m| m.get_service())
    }
}

/// 获取驱动管理器 (兼容性接口)
pub fn get_driver_manager() -> Option<&'static DriverManager> {
    unsafe {
        DRIVER_MANAGER.as_ref()
    }
}

/// 获取驱动统计信息
pub fn get_stats() -> Option<DriverStatsSnapshot> {
    let service = get_driver_service()?;
    Some(service.get_stats())
}

/// 兼容性接口 - 保持与现有代码的兼容性
/// Register a new device
pub fn driver_register_device(
    device_type: u32,
    device_name: &str,
    device_class: &str,
    major: u32,
    minor: u32
) -> u32 {
    if let Some(service) = get_driver_service() {
        // 简化的设备类型映射
        let device_type_enum = match device_type {
            0 => DeviceType::Block,
            1 => DeviceType::Char,
            2 => DeviceType::Network,
            _ => DeviceType::Misc,
        };

        let device_info = DeviceInfo {
            device_id: 0, // 将在register_device中设置
            name: device_name.to_string(),
            device_type: device_type_enum,
            status: DeviceStatus::Detached,
            major,
            minor,
            vendor_id: None,
            product_id: None,
            device_class: device_class.to_string(),
            device_subclass: String::new(),
            driver_name: None,
            resources: DeviceResources {
                memory_regions: Vec::new(),
                io_ports: Vec::new(),
                irqs: Vec::new(),
                dma_channels: Vec::new(),
                clocks: Vec::new(),
            },
            stats: DeviceStats::new(),
        };

        service.register_device(device_info).unwrap_or(0)
    } else {
        0
    }
}

/// Unregister a device
pub fn driver_unregister_device(device_id: u32) -> bool {
    if let Some(service) = get_driver_service() {
        service.unregister_device(device_id).is_ok()
    } else {
        false
    }
}

/// Get device information by ID
pub fn driver_get_device_info(device_id: u32) -> Option<DeviceInfo> {
    let service = get_driver_service()?;
    service.get_device_info(device_id)
}

/// Get all devices
pub fn driver_get_all_devices() -> Vec<DeviceInfo> {
    if let Some(service) = get_driver_service() {
        service.get_all_devices()
    } else {
        Vec::new()
    }
}

/// Read from block device
pub fn driver_block_read(device: &impl BlockDevice, sector: usize, buf: &mut [u8]) {
    device.read(sector, buf);
}

/// Write to block device
pub fn driver_block_write(device: &impl BlockDevice, sector: usize, buf: &[u8]) {
    device.write(sector, buf);
}
