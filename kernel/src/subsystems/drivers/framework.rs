//! 统一驱动框架
//!
//! 本模块提供统一的驱动程序接口和基础实现，包括：
//! - 驱动程序trait定义
//! - 设备抽象层
//! - 驱动程序生命周期管理
//! - 电源管理接口
//! - 中断处理接口

use crate::prelude::*;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::sync::Arc;
use spin::Mutex;

use crate::subsystems::drivers::driver_manager::{
    DeviceId, DeviceType, DeviceStatus,
    IoOperation, IoResult
};

// ============================================================================
// 电源管理
// ============================================================================

/// 设备电源状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PowerState {
    /// 设备完全断电
    D3Cold,
    /// 设备处于低功耗状态但保持上下文
    D3Hot,
    /// 设备处于睡眠状态
    D2,
    /// 设备处于低功耗状态
    D1,
    /// 设备完全工作
    D0,
}

impl Default for PowerState {
    fn default() -> Self {
        PowerState::D0
    }
}

impl PowerState {
    /// 获取电源状态名称
    pub fn name(&self) -> &'static str {
        match self {
            PowerState::D0 => "D0 (Full Power)",
            PowerState::D1 => "D1 (Low Power)",
            PowerState::D2 => "D2 (Sleep)",
            PowerState::D3Hot => "D3Hot (Context Lost)",
            PowerState::D3Cold => "D3Cold (Power Off)",
        }
    }

    /// 检查是否可以从一个状态转换到另一个状态
    pub fn can_transition_to(&self, target: &PowerState) -> bool {
        // 通常可以从任何状态转换到D0
        if *target == PowerState::D0 {
            return true;
        }

        // 从D0可以转换到任何状态
        if *self == PowerState::D0 {
            return true;
        }

        // 其他转换需要根据具体设备决定
        // 这里提供一个保守的实现
        matches!((*self, *target),
            (PowerState::D1, PowerState::D2) |
            (PowerState::D2, PowerState::D3Hot) |
            (PowerState::D3Hot, PowerState::D3Cold)
        )
    }
}

// ============================================================================
// 设备抽象
// ============================================================================

/// 设备标识符
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceIdentifier {
    /// 设备类型
    pub device_type: DeviceType,
    /// 供应商ID
    pub vendor_id: u16,
    /// 设备ID
    pub device_id: u16,
    /// 子系统ID
    pub subsystem_id: Option<u16>,
    /// 类别代码
    pub class_code: Option<u8>,
    /// 子类别代码
    pub subclass_code: Option<u8>,
    /// 序列号
    pub serial_number: Option<String>,
}

impl DeviceIdentifier {
    /// 创建新的设备标识符
    pub fn new(
        device_type: DeviceType,
        vendor_id: u16,
        device_id: u16,
    ) -> Self {
        Self {
            device_type,
            vendor_id,
            device_id,
            subsystem_id: None,
            class_code: None,
            subclass_code: None,
            serial_number: None,
        }
    }

    /// 创建完整的设备标识符
    pub fn with_details(
        device_type: DeviceType,
        vendor_id: u16,
        device_id: u16,
        subsystem_id: u16,
        class_code: u8,
        subclass_code: u8,
        serial_number: String,
    ) -> Self {
        Self {
            device_type,
            vendor_id,
            device_id,
            subsystem_id: Some(subsystem_id),
            class_code: Some(class_code),
            subclass_code: Some(subclass_code),
            serial_number: Some(serial_number),
        }
    }

    /// 检查是否匹配
    pub fn matches(&self, other: &DeviceIdentifier) -> bool {
        if self.device_type != other.device_type {
            return false;
        }

        if self.vendor_id != other.vendor_id {
            return false;
        }

        if self.device_id != other.device_id {
            return false;
        }

        true
    }

    /// 获取设备描述字符串
    pub fn description(&self) -> String {
        format!(
            "{}: VID:{:04X} DID:{:04X}",
            self.device_type_as_str(),
            self.vendor_id,
            self.device_id
        )
    }

    fn device_type_as_str(&self) -> &str {
        match self.device_type {
            DeviceType::Character => "char",
            DeviceType::Block => "block",
            DeviceType::Network => "network",
            DeviceType::Input => "input",
            DeviceType::Display => "display",
            DeviceType::Audio => "audio",
            DeviceType::Usb => "usb",
            DeviceType::Pci => "pci",
            DeviceType::Custom(ref s) => s.as_str(),
        }
    }
}

/// I/O端口范围
#[derive(Debug, Clone)]
pub struct IoPortRange {
    /// 起始端口
    pub start_port: u16,
    /// 端口数量
    pub port_count: u16,
}

impl IoPortRange {
    /// 创建新的I/O端口范围
    pub fn new(start_port: u16, port_count: u16) -> Self {
        Self {
            start_port,
            port_count,
        }
    }

    /// 检查端口是否在范围内
    pub fn contains(&self, port: u16) -> bool {
        port >= self.start_port && port < (self.start_port + self.port_count)
    }

    /// 获取结束端口
    pub fn end_port(&self) -> u16 {
        self.start_port + self.port_count
    }
}

/// 内存映射I/O区域
#[derive(Debug, Clone)]
pub struct MmioRegion {
    /// 物理基地址
    pub physical_address: u64,
    /// 区域大小
    pub size: usize,
    /// 是否可缓存
    pub cacheable: bool,
    /// 权限
    pub writable: bool,
}

impl MmioRegion {
    /// 创建新的MMIO区域
    pub fn new(physical_address: u64, size: usize) -> Self {
        Self {
            physical_address,
            size,
            cacheable: false,
            writable: true,
        }
    }

    /// 创建只读MMIO区域
    pub fn read_only(physical_address: u64, size: usize) -> Self {
        Self {
            physical_address,
            size,
            cacheable: false,
            writable: false,
        }
    }

    /// 创建可缓存MMIO区域
    pub fn cacheable(physical_address: u64, size: usize) -> Self {
        Self {
            physical_address,
            size,
            cacheable: true,
            writable: true,
        }
    }

    /// 检查地址是否在区域内
    pub fn contains(&self, offset: usize) -> bool {
        offset < self.size
    }
}

/// 中断资源
#[derive(Debug, Clone)]
pub struct Irq {
    /// 中断号
    pub irq: u32,
    /// 中断类型
    pub interrupt_type: IrqType,
    /// 触发模式
    pub trigger_mode: IrqTriggerMode,
    /// 是否共享
    pub shareable: bool,
}

/// 中断类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqType {
    /// 传统IRQ
    Legacy,
    /// MSI (Message Signaled Interrupt)
    Msi,
    /// MSI-X
    MsiX,
    /// GPIO中断
    Gpio,
}

/// 中断触发模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqTriggerMode {
    /// 边沿触发
    Edge,
    /// 电平触发
    Level,
}

/// 设备抽象trait
pub trait Device: Send + Sync {
    /// 获取设备ID
    fn id(&self) -> DeviceId;

    /// 获取设备标识符
    fn identifier(&self) -> &DeviceIdentifier;

    /// 获取设备名称
    fn name(&self) -> &str;

    /// 获取设备父级
    fn parent(&self) -> Option<&Arc<dyn Device>>;

    /// 获取子设备
    fn children(&self) -> &[Arc<dyn Device>];

    /// 添加子设备
    fn add_child(&mut self, child: Arc<dyn Device>) -> Result<()>;

    /// 移除子设备
    fn remove_child(&mut self, child_id: DeviceId) -> Result<()>;

    /// 获取I/O端口范围
    fn io_ports(&self) -> &[IoPortRange];

    /// 获取MMIO区域
    fn mmio_regions(&self) -> &[MmioRegion];

    /// 获取中断资源
    fn irq(&self) -> Option<&Irq>;

    /// 获取设备状态
    fn status(&self) -> DeviceStatus;

    /// 设置设备状态
    fn set_status(&mut self, status: DeviceStatus);

    /// 检查设备是否可用
    fn is_ready(&self) -> bool {
        self.status() == DeviceStatus::Ready
    }

    /// 获取设备属性
    fn get_property(&self, key: &str) -> Option<String>;

    /// 设置设备属性
    fn set_property(&mut self, key: &str, value: &str) -> Result<()>;

    /// 重置设备
    fn reset(&mut self) -> Result<()>;
}

// ============================================================================
// 驱动程序接口
// ============================================================================

/// 驱动程序trait - 统一的驱动程序接口
pub trait Driver: Send {
    /// 获取驱动程序名称
    fn name(&self) -> &str;

    /// 获取驱动程序版本
    fn version(&self) -> &str {
        "1.0.0"
    }

    /// 获取支持的设备列表
    fn supported_devices(&self) -> &[DeviceIdentifier];

    /// 初始化驱动程序
    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    /// 探测设备
    ///
    /// 检查驱动程序是否支持指定的设备
    fn probe(&mut self, device: &dyn Device) -> Result<bool> {
        let device_id = device.identifier();
        for supported in self.supported_devices() {
            if supported.matches(device_id) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// 启动设备
    ///
    /// 在设备被驱动程序接管后调用
    fn start(&mut self, _device: &mut dyn Device) -> Result<()> {
        Ok(())
    }

    /// 停止设备
    ///
    /// 在设备从驱动程序移除前调用
    fn stop(&mut self, _device: &mut dyn Device) -> Result<()> {
        Ok(())
    }

    /// 移除设备
    ///
    /// 设备从系统中移除时调用
    fn remove(&mut self, device: &mut dyn Device) -> Result<()> {
        self.stop(device)
    }

    /// 暂停设备
    ///
    /// 设备进入低功耗状态时调用
    fn suspend(&mut self, device: &mut dyn Device, state: PowerState) -> Result<()>;

    /// 恢复设备
    ///
    /// 设备从低功耗状态恢复时调用
    fn resume(&mut self, device: &mut dyn Device) -> Result<()>;

    /// 设置电源状态
    fn set_power_state(&mut self, device: &mut dyn Device, state: PowerState) -> Result<()> {
        match state {
            PowerState::D0 => self.resume(device),
            _ => self.suspend(device, state),
        }
    }

    /// 处理中断
    ///
    /// 设备产生中断时调用
    fn handle_interrupt(&mut self, device: &mut dyn Device, irq: u32) -> Result<()>;

    /// 处理I/O操作
    ///
    /// 处理来自用户空间的I/O请求
    fn handle_io(&mut self, device: &mut dyn Device, operation: IoOperation) -> Result<IoResult>;

    /// 获取设备状态
    fn get_device_status(&self, device: &dyn Device) -> Result<DeviceStatus> {
        Ok(device.status())
    }

    /// 清理驱动程序资源
    fn cleanup(&mut self) -> Result<()> {
        Ok(())
    }
}

// ============================================================================
// 驱动程序注册表
// ============================================================================

/// 驱动程序管理器 - 管理驱动程序的注册和设备匹配
pub struct DriverManager {
    /// 已注册的驱动程序
    drivers: Mutex<Vec<Box<dyn Driver>>>,
    /// 设备到驱动程序的映射
    device_driver_map: Mutex<BTreeMap<DeviceId, usize>>,
    /// 驱动程序名称到索引的映射
    driver_name_map: Mutex<BTreeMap<String, usize>>,
}

impl DriverManager {
    /// 创建新的驱动程序管理器
    pub fn new() -> Self {
        Self {
            drivers: Mutex::new(Vec::new()),
            device_driver_map: Mutex::new(BTreeMap::new()),
            driver_name_map: Mutex::new(BTreeMap::new()),
        }
    }

    /// 注册驱动程序
    pub fn register(&self, driver: Box<dyn Driver>) -> Result<()> {
        let name = driver.name().to_string();

        // 检查驱动程序是否已注册
        {
            let name_map = self.driver_name_map.lock();
            if name_map.contains_key(&name) {
                return Err(KernelError::InvalidArgument);
            }
        }

        // 添加驱动程序
        let index = {
            let mut drivers = self.drivers.lock();
            let index = drivers.len();
            drivers.push(driver);
            index
        };

        // 记录名称映射
        {
            let mut name_map = self.driver_name_map.lock();
            name_map.insert(name.clone(), index);
        }

        crate::println!("driver_manager: registered driver '{}'", name);
        Ok(())
    }

    /// 注销驱动程序
    pub fn unregister(&self, name: &str) -> Result<()> {
        // 查找驱动程序索引
        let index = {
            let name_map = self.driver_name_map.lock();
            *name_map.get(name).ok_or(KernelError::NotFound)?
        };

        // 获取驱动程序管理的所有设备
        let device_ids: Vec<DeviceId> = {
            let map = self.device_driver_map.lock();
            map.iter()
                .filter(|&(_, &driver_idx)| driver_idx == index)
                .map(|(&device_id, _)| device_id)
                .collect()
        };

        // 移除所有设备映射
        {
            let mut map = self.device_driver_map.lock();
            for device_id in device_ids {
                map.remove(&device_id);
            }
        }

        // 移除驱动程序
        {
            let mut drivers = self.drivers.lock();
            if index < drivers.len() {
                let _ = drivers[index].cleanup();
                drivers.remove(index);
            }
        }

        // 更新名称映射
        {
            let mut name_map = self.driver_name_map.lock();
            name_map.remove(name);
        }

        crate::println!("driver_manager: unregistered driver '{}'", name);
        Ok(())
    }

    /// 为设备探测并匹配驱动程序
    pub fn probe(&self, _device: &dyn Device) -> Result<Option<usize>> {
        let drivers = self.drivers.lock();

        for (_index, _driver) in drivers.iter().enumerate() {
            // Since we can't get mutable access, we'll assume all drivers return false for now
            // In a real implementation, this would require a different design
            // For now, just return None
        }

        Ok(None)
    }

    /// 绑定设备到驱动程序
    pub fn bind(&self, device_id: DeviceId, driver_index: usize) -> Result<()> {
        let mut map = self.device_driver_map.lock();
        map.insert(device_id, driver_index);
        Ok(())
    }

    /// 解绑设备
    pub fn unbind(&self, device_id: DeviceId) -> Result<()> {
        let mut map = self.device_driver_map.lock();
        map.remove(&device_id).ok_or(KernelError::NotFound)?;
        Ok(())
    }

    /// 获取设备的驱动程序
    pub fn get_driver(&self, device_id: DeviceId) -> Option<Box<dyn Driver>> {
        let map = self.device_driver_map.lock();
        let _index = *map.get(&device_id)?;

        let _drivers = self.drivers.lock();
        // 注意：这里不能返回驱动程序的引用，因为需要释放锁
        // 实际实现中应该使用Arc<Mutex<dyn Driver>>
        None
    }

    /// 获取所有已注册的驱动程序数量
    pub fn driver_count(&self) -> usize {
        self.drivers.lock().len()
    }

    /// 获取所有已绑定的设备数量
    pub fn bound_device_count(&self) -> usize {
        self.device_driver_map.lock().len()
    }
}

impl Default for DriverManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 比较两个设备标识符是否兼容
pub fn device_ids_match(id1: &DeviceIdentifier, id2: &DeviceIdentifier) -> bool {
    if id1.device_type != id2.device_type {
        return false;
    }

    if id1.vendor_id != id2.vendor_id {
        return false;
    }

    if id1.device_id != id2.device_id {
        return false;
    }

    true
}

/// 验证电源状态转换
pub fn validate_power_transition(from: PowerState, to: PowerState) -> bool {
    from.can_transition_to(&to)
}

/// 计算电源状态延迟（微秒）
pub fn power_state_delay(state: PowerState) -> u64 {
    match state {
        PowerState::D0 => 0,
        PowerState::D1 => 100,
        PowerState::D2 => 1000,
        PowerState::D3Hot => 10000,
        PowerState::D3Cold => 100000,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_state_transitions() {
        assert!(PowerState::D0.can_transition_to(&PowerState::D1));
        assert!(PowerState::D0.can_transition_to(&PowerState::D3Cold));
        assert!(!PowerState::D3Cold.can_transition_to(&PowerState::D1));
    }

    #[test]
    fn test_device_identifier() {
        let id1 = DeviceIdentifier::new(DeviceType::Network, 0x8086, 0x100e);
        let id2 = DeviceIdentifier::new(DeviceType::Network, 0x8086, 0x100e);
        let id3 = DeviceIdentifier::new(DeviceType::Network, 0x8086, 0x100f);

        assert!(id1.matches(&id2));
        assert!(!id1.matches(&id3));
    }

    #[test]
    fn test_io_port_range() {
        let range = IoPortRange::new(0x1000, 16);
        assert!(range.contains(0x1000));
        assert!(range.contains(0x100F));
        assert!(!range.contains(0x1010));
        assert_eq!(range.end_port(), 0x1010);
    }

    #[test]
    fn test_mmio_region() {
        let region = MmioRegion::new(0xF0000000, 4096);
        assert!(region.contains(0));
        assert!(region.contains(4095));
        assert!(!region.contains(4096));
    }
}
