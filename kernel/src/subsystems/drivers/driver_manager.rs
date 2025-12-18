//! 可扩展的驱动程序架构模块
//! 
//! 本模块提供可扩展的驱动程序架构，包括：
//! - 驱动程序接口
//! - 驱动程序管理器
//! - 设备抽象层
//! - 驱动程序生命周期管理
//! - 设备资源管理

use nos_nos_error_handling::unified::KernelError;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;

/// 设备ID类型
pub type DeviceId = u32;
/// 驱动程序ID类型
pub type DriverId = u32;

/// 设备类型
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceType {
    /// 字符设备
    Character,
    /// 块设备
    Block,
    /// 网络设备
    Network,
    /// 输入设备
    Input,
    /// 显示设备
    Display,
    /// 音频设备
    Audio,
    /// USB设备
    Usb,
    /// PCI设备
    Pci,
    /// 自定义设备
    Custom(String),
}

/// 设备状态
#[derive(Debug, Clone, PartialEq)]
pub enum DeviceStatus {
    /// 未初始化
    Uninitialized,
    /// 初始化中
    Initializing,
    /// 已就绪
    Ready,
    /// 忙碌
    Busy,
    /// 错误
    Error,
    /// 已禁用
    Disabled,
    /// 已移除
    Removed,
}

/// 驱动程序状态
#[derive(Debug, Clone, PartialEq)]
pub enum DriverStatus {
    /// 未加载
    Unloaded,
    /// 加载中
    Loading,
    /// 已加载
    Loaded,
    /// 初始化中
    Initializing,
    /// 已初始化
    Initialized,
    /// 运行中
    Running,
    /// 停止中
    Stopping,
    /// 已停止
    Stopped,
    /// 卸载中
    Unloading,
    /// 错误
    Error,
}

/// 设备信息
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    /// 设备ID
    pub id: DeviceId,
    /// 设备名称
    pub name: String,
    /// 设备类型
    pub device_type: DeviceType,
    /// 设备状态
    pub status: DeviceStatus,
    /// 驱动程序ID
    pub driver_id: DriverId,
    /// 设备路径
    pub path: String,
    /// 设备版本
    pub version: String,
    /// 设备供应商
    pub vendor: String,
    /// 设备型号
    pub model: String,
    /// 设备序列号
    pub serial_number: String,
    /// 设备资源
    pub resources: DeviceResources,
    /// 设备能力
    pub capabilities: Vec<String>,
    /// 自定义属性
    pub attributes: BTreeMap<String, String>,
}

/// 设备资源
#[derive(Debug, Clone, Default)]
pub struct DeviceResources {
    /// 内存区域
    pub memory_regions: Vec<MemoryRegion>,
    /// I/O端口
    pub io_ports: Vec<IoPortRange>,
    /// 中断线
    pub interrupt_lines: Vec<InterruptLine>,
    /// DMA通道
    pub dma_channels: Vec<DmaChannel>,
    /// 自定义资源
    pub custom_resources: BTreeMap<String, Vec<u8>>,
}

/// 内存区域
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// 起始地址
    pub start_address: u64,
    /// 大小
    pub size: u64,
    /// 权限
    pub permissions: u32,
    /// 类型
    pub region_type: u32,
}

/// I/O端口范围
#[derive(Debug, Clone)]
pub struct IoPortRange {
    /// 起始端口
    pub start_port: u16,
    /// 端口数量
    pub port_count: u16,
}

/// 中断线
#[derive(Debug, Clone)]
pub struct InterruptLine {
    /// 中断号
    pub irq: u32,
    /// 中断类型
    pub interrupt_type: u32,
    /// 触发模式
    pub trigger_mode: u32,
}

/// DMA通道
#[derive(Debug, Clone)]
pub struct DmaChannel {
    /// 通道号
    pub channel: u32,
    /// 通道类型
    pub channel_type: u32,
    /// 最大传输大小
    pub max_transfer_size: u32,
}

/// 驱动程序信息
#[derive(Debug, Clone)]
pub struct DriverInfo {
    /// 驱动程序ID
    pub id: DriverId,
    /// 驱动程序名称
    pub name: String,
    /// 驱动程序版本
    pub version: String,
    /// 驱动程序状态
    pub status: DriverStatus,
    /// 支持的设备类型
    pub supported_device_types: Vec<DeviceType>,
    /// 支持的设备ID列表
    pub supported_device_ids: Vec<String>,
    /// 驱动程序路径
    pub path: String,
    /// 驱动程序依赖
    pub dependencies: Vec<String>,
    /// 驱动程序能力
    pub capabilities: Vec<String>,
    /// 自定义属性
    pub attributes: BTreeMap<String, String>,
}

/// 驱动程序接口
pub trait Driver {
    /// 获取驱动程序信息
    fn get_info(&self) -> DriverInfo;
    
    /// 初始化驱动程序
    fn initialize(&mut self) -> Result<(), KernelError>;
    
    /// 清理驱动程序
    fn cleanup(&mut self) -> Result<(), KernelError>;
    
    /// 探测设备
    fn probe_device(&self, device_info: &DeviceInfo) -> Result<bool, KernelError>;
    
    /// 添加设备
    fn add_device(&mut self, device_info: &DeviceInfo) -> Result<(), KernelError>;
    
    /// 移除设备
    fn remove_device(&mut self, device_id: DeviceId) -> Result<(), KernelError>;
    
    /// 处理设备I/O
    fn handle_io(&mut self, device_id: DeviceId, operation: IoOperation) -> Result<IoResult, KernelError>;
    
    /// 获取设备状态
    fn get_device_status(&self, device_id: DeviceId) -> Result<DeviceStatus, KernelError>;
    
    /// 设置设备属性
    fn set_device_attribute(&mut self, device_id: DeviceId, name: &str, value: &str) -> Result<(), KernelError>;
    
    /// 获取设备属性
    fn get_device_attribute(&self, device_id: DeviceId, name: &str) -> Result<String, KernelError>;
    
    /// 暂停设备
    fn suspend_device(&mut self, device_id: DeviceId) -> Result<(), KernelError>;
    
    /// 恢复设备
    fn resume_device(&mut self, device_id: DeviceId) -> Result<(), KernelError>;
    
    /// 处理中断
    fn handle_interrupt(&mut self, device_id: DeviceId, interrupt_info: &InterruptInfo) -> Result<(), KernelError>;
}

/// I/O操作
#[derive(Debug, Clone)]
pub enum IoOperation {
    /// 读操作
    Read { offset: u64, size: u64 },
    /// 写操作
    Write { offset: u64, data: Vec<u8> },
    /// 控制操作
    Ioctl { command: u32, arg: u64 },
    /// 映射内存
    Mmap { offset: u64, size: u64, permissions: u32 },
    /// 取消映射内存
    Munmap { offset: u64, size: u64 },
}

/// I/O结果
#[derive(Debug, Clone)]
pub enum IoResult {
    /// 读操作结果
    ReadResult { data: Vec<u8>, bytes_read: u64 },
    /// 写操作结果
    WriteResult { bytes_written: u64 },
    /// 控制操作结果
    IoctlResult { result: u64 },
    /// 映射内存结果
    MmapResult { address: u64 },
    /// 取消映射内存结果
    MunmapResult,
}

/// 中断信息
#[derive(Debug, Clone)]
pub struct InterruptInfo {
    /// 中断号
    pub irq: u32,
    /// 中断类型
    pub interrupt_type: u32,
    /// 中断上下文
    pub context: u64,
    /// 中断数据
    pub data: Vec<u8>,
}

/// 驱动程序管理器
pub struct DriverManager {
    /// 驱动程序列表
    drivers: Arc<Mutex<BTreeMap<DriverId, Arc<Mutex<Box<dyn Driver>>>>>,
    /// 设备列表
    devices: Arc<Mutex<BTreeMap<DeviceId, DeviceInfo>>>,
    /// 设备到驱动程序的映射
    device_driver_mapping: Arc<Mutex<BTreeMap<DeviceId, DriverId>>>,
    /// 下一个驱动程序ID
    next_driver_id: Arc<Mutex<DriverId>>,
    /// 下一个设备ID
    next_device_id: Arc<Mutex<DeviceId>>,
    /// 管理器配置
    config: DriverManagerConfig,
    /// 驱动程序统计
    stats: Arc<Mutex<DriverStatistics>>,
}

/// 驱动程序管理器配置
#[derive(Debug, Clone)]
pub struct DriverManagerConfig {
    /// 最大驱动程序数
    pub max_drivers: usize,
    /// 最大设备数
    pub max_devices: usize,
    /// 是否启用自动探测
    pub enable_auto_probe: bool,
    /// 是否启用热插拔
    pub enable_hotplug: bool,
    /// 驱动程序搜索路径
    pub driver_search_paths: Vec<String>,
    /// 设备扫描间隔（秒）
    pub device_scan_interval_seconds: u64,
}

/// 驱动程序统计
#[derive(Debug, Default, Clone)]
pub struct DriverStatistics {
    /// 总驱动程序数
    pub total_drivers: u64,
    /// 已加载驱动程序数
    pub loaded_drivers: u64,
    /// 总设备数
    pub total_devices: u64,
    /// 已初始化设备数
    pub initialized_devices: u64,
    /// I/O操作数
    pub io_operations: u64,
    /// 中断处理数
    pub interrupt_handlers: u64,
    /// 错误数
    pub errors: u64,
}

impl Default for DriverManagerConfig {
    fn default() -> Self {
        Self {
            max_drivers: 100,
            max_devices: 1000,
            enable_auto_probe: true,
            enable_hotplug: true,
            driver_search_paths: vec![
                "/lib/drivers".to_string(),
                "/usr/lib/drivers".to_string(),
            ],
            device_scan_interval_seconds: 10,
        }
    }
}

impl DriverManager {
    /// 创建新的驱动程序管理器
    pub fn new(config: DriverManagerConfig) -> Self {
        Self {
            drivers: Arc::new(Mutex::new(BTreeMap::new())),
            devices: Arc::new(Mutex::new(BTreeMap::new())),
            device_driver_mapping: Arc::new(Mutex::new(BTreeMap::new())),
            next_driver_id: Arc::new(Mutex::new(1)),
            next_device_id: Arc::new(Mutex::new(1)),
            config,
            stats: Arc::new(Mutex::new(DriverStatistics::default())),
        }
    }
    
    /// 使用默认配置创建驱动程序管理器
    pub fn with_default_config() -> Self {
        Self::new(DriverManagerConfig::default())
    }
    
    /// 注册驱动程序
    pub fn register_driver(&self, driver: Box<dyn Driver>) -> Result<DriverId, KernelError> {
        // 生成驱动程序ID
        let driver_id = {
            let mut next_id = self.next_driver_id.lock();
            let id = *next_id;
            *next_id += 1;
            id
        };
        
        // 获取驱动程序信息
        let driver_info = driver.get_info();
        
        // 检查驱动程序数量限制
        {
            let drivers = self.drivers.lock();
            if drivers.len() >= self.config.max_drivers {
                return Err(KernelError::OutOfSpace);
            }
        }
        
        // 添加到驱动程序列表
        {
            let mut drivers = self.drivers.lock();
            drivers.insert(driver_id, Arc::new(Mutex::new(driver)));
        }
        
        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.total_drivers += 1;
            stats.loaded_drivers += 1;
        }
        
        // 初始化驱动程序
        {
            let drivers = self.drivers.lock();
            if let Some(driver) = drivers.get(&driver_id) {
                let mut driver = driver.lock();
                if let Err(e) = driver.initialize() {
                    // 初始化失败，移除驱动程序
                    drop(driver);
                    drop(drivers);
                    let mut drivers = self.drivers.lock();
                    drivers.remove(&driver_id);
                    
                    // 更新统计
                    let mut stats = self.stats.lock();
                    stats.loaded_drivers -= 1;
                    stats.errors += 1;
                    
                    return Err(e);
                }
            }
        }
        
        // 如果启用自动探测，探测设备
        if self.config.enable_auto_probe {
            self.probe_devices_for_driver(driver_id);
        }
        
        Ok(driver_id)
    }
    
    /// 注销驱动程序
    pub fn unregister_driver(&self, driver_id: DriverId) -> Result<(), KernelError> {
        // 检查驱动程序是否存在
        let exists = {
            let drivers = self.drivers.lock();
            drivers.contains_key(&driver_id)
        };
        
        if !exists {
            return Err(KernelError::NotFound);
        }
        
        // 获取驱动程序管理的设备列表
        let managed_devices = {
            let mapping = self.device_driver_mapping.lock();
            mapping.iter()
                .filter(|(_, &did)| did == driver_id)
                .map(|(&did, _)| did)
                .collect::<Vec<_>>()
        };
        
        // 移除所有管理的设备
        for device_id in managed_devices {
            let _ = self.remove_device(device_id);
        }
        
        // 清理驱动程序
        {
            let drivers = self.drivers.lock();
            if let Some(driver) = drivers.get(&driver_id) {
                let mut driver = driver.lock();
                let _ = driver.cleanup();
            }
        }
        
        // 从驱动程序列表中移除
        {
            let mut drivers = self.drivers.lock();
            drivers.remove(&driver_id);
        }
        
        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.loaded_drivers -= 1;
        }
        
        Ok(())
    }
    
    /// 添加设备
    pub fn add_device(&self, mut device_info: DeviceInfo) -> Result<DeviceId, KernelError> {
        // 生成设备ID
        let device_id = {
            let mut next_id = self.next_device_id.lock();
            let id = *next_id;
            *next_id += 1;
            id
        };
        
        device_info.id = device_id;
        
        // 检查设备数量限制
        {
            let devices = self.devices.lock();
            if devices.len() >= self.config.max_devices {
                return Err(KernelError::OutOfSpace);
            }
        }
        
        // 查找合适的驱动程序
        let driver_id = self.find_driver_for_device(&device_info)?;
        
        // 添加设备到设备列表
        {
            let mut devices = self.devices.lock();
            devices.insert(device_id, device_info.clone());
        }
        
        // 建立设备到驱动程序的映射
        {
            let mut mapping = self.device_driver_mapping.lock();
            mapping.insert(device_id, driver_id);
        }
        
        // 通知驱动程序添加设备
        {
            let drivers = self.drivers.lock();
            if let Some(driver) = drivers.get(&driver_id) {
                let mut driver = driver.lock();
                if let Err(e) = driver.add_device(&device_info) {
                    // 添加设备失败，清理
                    drop(driver);
                    drop(drivers);
                    
                    let mut devices = self.devices.lock();
                    devices.remove(&device_id);
                    
                    let mut mapping = self.device_driver_mapping.lock();
                    mapping.remove(&device_id);
                    
                    // 更新统计
                    let mut stats = self.stats.lock();
                    stats.errors += 1;
                    
                    return Err(e);
                }
            }
        }
        
        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.total_devices += 1;
            stats.initialized_devices += 1;
        }
        
        Ok(device_id)
    }
    
    /// 移除设备
    pub fn remove_device(&self, device_id: DeviceId) -> Result<(), KernelError> {
        // 检查设备是否存在
        let device_info = {
            let devices = self.devices.lock();
            match devices.get(&device_id) {
                Some(info) => info.clone(),
                None => return Err(KernelError::NotFound),
            }
        };
        
        // 获取驱动程序ID
        let driver_id = {
            let mapping = self.device_driver_mapping.lock();
            match mapping.get(&device_id) {
                Some(&id) => id,
                None => return Err(KernelError::NotFound),
            }
        };
        
        // 通知驱动程序移除设备
        {
            let drivers = self.drivers.lock();
            if let Some(driver) = drivers.get(&driver_id) {
                let mut driver = driver.lock();
                let _ = driver.remove_device(device_id);
            }
        }
        
        // 从设备列表中移除
        {
            let mut devices = self.devices.lock();
            devices.remove(&device_id);
        }
        
        // 移除设备到驱动程序的映射
        {
            let mut mapping = self.device_driver_mapping.lock();
            mapping.remove(&device_id);
        }
        
        // 更新统计
        {
            let mut stats = self.stats.lock();
            stats.total_devices -= 1;
            if device_info.status == DeviceStatus::Ready {
                stats.initialized_devices -= 1;
            }
        }
        
        Ok(())
    }
    
    /// 处理设备I/O
    pub fn handle_device_io(&self, device_id: DeviceId, operation: IoOperation) -> Result<IoResult, KernelError> {
        // 获取驱动程序ID
        let driver_id = {
            let mapping = self.device_driver_mapping.lock();
            match mapping.get(&device_id) {
                Some(&id) => id,
                None => return Err(KernelError::NotFound),
            }
        };
        
        // 调用驱动程序处理I/O
        {
            let drivers = self.drivers.lock();
            if let Some(driver) = drivers.get(&driver_id) {
                let mut driver = driver.lock();
                let result = driver.handle_io(device_id, operation);
                
                // 更新统计
                {
                    let mut stats = self.stats.lock();
                    stats.io_operations += 1;
                    
                    if result.is_err() {
                        stats.errors += 1;
                    }
                }
                
                return result;
            }
        }
        
        Err(KernelError::NotFound)
    }
    
    /// 处理中断
    pub fn handle_interrupt(&self, device_id: DeviceId, interrupt_info: InterruptInfo) -> Result<(), KernelError> {
        // 获取驱动程序ID
        let driver_id = {
            let mapping = self.device_driver_mapping.lock();
            match mapping.get(&device_id) {
                Some(&id) => id,
                None => return Err(KernelError::NotFound),
            }
        };
        
        // 调用驱动程序处理中断
        {
            let drivers = self.drivers.lock();
            if let Some(driver) = drivers.get(&driver_id) {
                let mut driver = driver.lock();
                let result = driver.handle_interrupt(device_id, &interrupt_info);
                
                // 更新统计
                {
                    let mut stats = self.stats.lock();
                    stats.interrupt_handlers += 1;
                    
                    if result.is_err() {
                        stats.errors += 1;
                    }
                }
                
                return result;
            }
        }
        
        Err(KernelError::NotFound)
    }
    
    /// 获取设备信息
    pub fn get_device_info(&self, device_id: DeviceId) -> Option<DeviceInfo> {
        let devices = self.devices.lock();
        devices.get(&device_id).cloned()
    }
    
    /// 获取所有设备
    pub fn get_all_devices(&self) -> Vec<DeviceInfo> {
        let devices = self.devices.lock();
        devices.values().cloned().collect()
    }
    
    /// 获取驱动程序信息
    pub fn get_driver_info(&self, driver_id: DriverId) -> Option<DriverInfo> {
        let drivers = self.drivers.lock();
        drivers.get(&driver_id).map(|driver| driver.lock().get_info())
    }
    
    /// 获取所有驱动程序
    pub fn get_all_drivers(&self) -> Vec<DriverInfo> {
        let drivers = self.drivers.lock();
        drivers.values()
            .map(|driver| driver.lock().get_info())
            .collect()
    }
    
    /// 获取驱动程序统计
    pub fn get_statistics(&self) -> DriverStatistics {
        self.stats.lock().clone()
    }
    
    /// 为驱动程序探测设备
    fn probe_devices_for_driver(&self, driver_id: DriverId) {
        // 获取现有设备列表
        let devices = {
            let devices = self.devices.lock();
            devices.values().cloned().collect::<Vec<_>>()
        };
        
        // 获取驱动程序
        let driver = {
            let drivers = self.drivers.lock();
            drivers.get(&driver_id).cloned()
        };
        
        if let Some(driver) = driver {
            let mut driver = driver.lock();
            
            // 探测每个设备
            for device in devices {
                // 检查设备是否已有驱动程序
                let has_driver = {
                    let mapping = self.device_driver_mapping.lock();
                    mapping.contains_key(&device.id)
                };
                
                if !has_driver {
                    // 尝试探测设备
                    if let Ok(true) = driver.probe_device(&device) {
                        // 设备被驱动程序支持，添加设备
                        let _ = self.add_device(device);
                    }
                }
            }
        }
    }
    
    /// 查找设备的驱动程序
    fn find_driver_for_device(&self, device_info: &DeviceInfo) -> Result<DriverId, KernelError> {
        let drivers = self.drivers.lock();
        
        for (driver_id, driver) in drivers.iter() {
            let driver_info = driver.lock().get_info();
            
            // 检查设备类型是否支持
            if driver_info.supported_device_types.contains(&device_info.device_type) {
                // 检查设备ID是否支持
                if driver_info.supported_device_ids.is_empty() || 
                   driver_info.supported_device_ids.contains(&device_info.name) {
                    return Ok(*driver_id);
                }
            }
        }
        
        Err(KernelError::NotFound)
    }
}