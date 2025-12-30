//! 通用驱动基类
//!
//! 本模块提供驱动程序的通用基础实现，包括：
//! - BaseDriver：通用驱动程序基类
//! - GenericDevice：通用设备实现
//! - 辅助函数和工具

use crate::prelude::*;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;
use core::sync::atomic::{AtomicBool, Ordering};

use super::framework::{
    Device, DeviceIdentifier, Driver, PowerState, IoPortRange, MmioRegion, Irq
};
use crate::subsystems::drivers::driver_manager::{
    DeviceId, DeviceType, DeviceStatus, IoOperation, IoResult
};

// ============================================================================
// 通用设备实现
// ============================================================================

/// 通用设备实现
pub struct GenericDevice {
    /// 设备ID
    id: DeviceId,
    /// 设备标识符
    identifier: DeviceIdentifier,
    /// 设备名称
    name: String,
    /// 设备状态
    status: Mutex<DeviceStatus>,
    /// 父设备
    parent: Option<Arc<dyn Device>>,
    /// 子设备列表
    children: Mutex<Vec<Arc<dyn Device>>>,
    /// I/O端口范围
    io_ports: Vec<IoPortRange>,
    /// MMIO区域
    mmio_regions: Vec<MmioRegion>,
    /// 中断资源
    irq: Option<Irq>,
    /// 设备属性
    properties: Mutex<BTreeMap<String, String>>,
    /// 初始化标志
    initialized: AtomicBool,
}

impl GenericDevice {
    /// 创建新的通用设备
    pub fn new(
        id: DeviceId,
        identifier: DeviceIdentifier,
        name: String,
    ) -> Self {
        Self {
            id,
            identifier,
            name,
            status: Mutex::new(DeviceStatus::Uninitialized),
            parent: None,
            children: Mutex::new(Vec::new()),
            io_ports: Vec::new(),
            mmio_regions: Vec::new(),
            irq: None,
            properties: Mutex::new(BTreeMap::new()),
            initialized: AtomicBool::new(false),
        }
    }

    /// 设置I/O端口范围
    pub fn with_io_ports(mut self, ports: Vec<IoPortRange>) -> Self {
        self.io_ports = ports;
        self
    }

    /// 设置MMIO区域
    pub fn with_mmio_regions(mut self, regions: Vec<MmioRegion>) -> Self {
        self.mmio_regions = regions;
        self
    }

    /// 设置中断资源
    pub fn with_irq(mut self, irq: Irq) -> Self {
        self.irq = Some(irq);
        self
    }

    /// 设置父设备
    pub fn with_parent(mut self, parent: Arc<dyn Device>) -> Self {
        self.parent = Some(parent);
        self
    }

    /// 初始化设备
    pub fn initialize(&self) -> Result<()> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        // 设置初始状态
        *self.status.lock() = DeviceStatus::Initializing;

        // 执行设备特定的初始化
        // 在实际实现中，这里会配置硬件

        *self.status.lock() = DeviceStatus::Ready;
        self.initialized.store(true, Ordering::SeqCst);

        Ok(())
    }
}

impl Device for GenericDevice {
    fn id(&self) -> DeviceId {
        self.id
    }

    fn identifier(&self) -> &DeviceIdentifier {
        &self.identifier
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn parent(&self) -> Option<&Arc<dyn Device>> {
        self.parent.as_ref()
    }

    fn children(&self) -> &[Arc<dyn Device>] {
        // 注意：这里有一个生命周期问题
        // 在实际实现中，需要使用更复杂的方法来返回子设备列表
        &[]
    }

    fn add_child(&mut self, child: Arc<dyn Device>) -> Result<()> {
        let mut children = self.children.lock();
        children.push(child);
        Ok(())
    }

    fn remove_child(&mut self, child_id: DeviceId) -> Result<()> {
        let mut children = self.children.lock();
        let initial_len = children.len();
        children.retain(|device| device.id() != child_id);

        if children.len() < initial_len {
            Ok(())
        } else {
            Err(KernelError::NotFound)
        }
    }

    fn io_ports(&self) -> &[IoPortRange] {
        &self.io_ports
    }

    fn mmio_regions(&self) -> &[MmioRegion] {
        &self.mmio_regions
    }

    fn irq(&self) -> Option<&Irq> {
        self.irq.as_ref()
    }

    fn status(&self) -> DeviceStatus {
        self.status.lock().clone()
    }

    fn set_status(&mut self, status: DeviceStatus) {
        *self.status.lock() = status;
    }

    fn get_property(&self, key: &str) -> Option<String> {
        let props = self.properties.lock();
        props.get(key).cloned()
    }

    fn set_property(&mut self, key: &str, value: &str) -> Result<()> {
        let mut props = self.properties.lock();
        props.insert(key.to_string(), value.to_string());
        Ok(())
    }

    fn reset(&mut self) -> Result<()> {
        // 重置设备状态
        *self.status.lock() = DeviceStatus::Uninitialized;
        self.initialized.store(false, Ordering::SeqCst);
        Ok(())
    }
}

// ============================================================================
// 基础驱动程序实现
// ============================================================================

/// 基础驱动程序 - 提供通用的驱动程序实现
pub struct BaseDriver {
    /// 驱动程序名称
    name: String,
    /// 驱动程序版本
    version: String,
    /// 支持的设备列表
    supported_devices: Vec<DeviceIdentifier>,
    /// 管理的设备列表
    managed_devices: Mutex<BTreeMap<DeviceId, Arc<Mutex<dyn Device>>>>,
    /// 驱动程序状态
    initialized: AtomicBool,
    /// 当前电源状态
    power_state: Mutex<PowerState>,
    /// 统计信息
    stats: Mutex<DriverStatistics>,
}

/// 驱动程序统计信息
#[derive(Debug, Default, Clone)]
pub struct DriverStatistics {
    /// 探测次数
    pub probes: u64,
    /// 成功绑定次数
    pub bindings: u64,
    /// 中断处理次数
    pub interrupts: u64,
    /// I/O操作次数
    pub io_operations: u64,
    /// 错误次数
    pub errors: u64,
}

impl BaseDriver {
    /// 创建新的基础驱动程序
    pub fn new(
        name: String,
        version: String,
        supported_devices: Vec<DeviceIdentifier>,
    ) -> Self {
        Self {
            name,
            version,
            supported_devices,
            managed_devices: Mutex::new(BTreeMap::new()),
            initialized: AtomicBool::new(false),
            power_state: Mutex::new(PowerState::D0),
            stats: Mutex::new(DriverStatistics::default()),
        }
    }

    /// 添加支持的设备
    pub fn add_supported_device(&mut self, device: DeviceIdentifier) {
        self.supported_devices.push(device);
    }

    /// 获取管理的设备数量
    pub fn device_count(&self) -> usize {
        self.managed_devices.lock().len()
    }

    /// 获取统计信息
    pub fn get_statistics(&self) -> DriverStatistics {
        self.stats.lock().clone()
    }

    /// 重置统计信息
    pub fn reset_statistics(&self) {
        let mut stats = self.stats.lock();
        *stats = DriverStatistics::default();
    }

    /// 更新统计信息
    fn record_probe(&self) {
        let mut stats = self.stats.lock();
        stats.probes += 1;
    }

    fn record_binding(&self) {
        let mut stats = self.stats.lock();
        stats.bindings += 1;
    }

    fn record_interrupt(&self) {
        let mut stats = self.stats.lock();
        stats.interrupts += 1;
    }

    fn record_io(&self) {
        let mut stats = self.stats.lock();
        stats.io_operations += 1;
    }

    fn record_error(&self) {
        let mut stats = self.stats.lock();
        stats.errors += 1;
    }
}

impl Driver for BaseDriver {
    fn name(&self) -> &str {
        &self.name
    }

    fn version(&self) -> &str {
        &self.version
    }

    fn supported_devices(&self) -> &[DeviceIdentifier] {
        &self.supported_devices
    }

    fn initialize(&mut self) -> Result<()> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        crate::println!("base_driver: initializing driver '{}'", self.name);

        // 执行驱动程序特定的初始化
        // 在实际实现中，这里会分配资源、注册回调等

        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn probe(&mut self, device: &dyn Device) -> Result<bool> {
        self.record_probe();

        let device_id = device.identifier();

        for supported in &self.supported_devices {
            if supported.matches(device_id) {
                crate::println!(
                    "base_driver: device '{}' matches driver '{}'",
                    device.name(),
                    self.name
                );
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn start(&mut self, device: &mut dyn Device) -> Result<()> {
        crate::println!(
            "base_driver: starting device '{}' with driver '{}'",
            device.name(),
            self.name
        );

        // 添加到管理列表
        let device_id = device.id();
        let wrapper = Arc::new(Mutex::new(GenericDevice::new(
            device_id,
            device.identifier().clone(),
            device.name().to_string(),
        )));

        {
            let mut devices = self.managed_devices.lock();
            devices.insert(device_id, wrapper);
        }

        self.record_binding();
        device.set_status(DeviceStatus::Ready);

        Ok(())
    }

    fn stop(&mut self, device: &mut dyn Device) -> Result<()> {
        crate::println!(
            "base_driver: stopping device '{}' from driver '{}'",
            device.name(),
            self.name
        );

        // 从管理列表中移除
        let device_id = device.id();
        {
            let mut devices = self.managed_devices.lock();
            devices.remove(&device_id);
        }

        device.set_status(DeviceStatus::Disabled);

        Ok(())
    }

    fn remove(&mut self, device: &mut dyn Device) -> Result<()> {
        self.stop(device)?;
        device.set_status(DeviceStatus::Removed);
        Ok(())
    }

    fn suspend(&mut self, device: &mut dyn Device, state: PowerState) -> Result<()> {
        crate::println!(
            "base_driver: suspending device '{}' to state {:?}",
            device.name(),
            state
        );

        // 更新电源状态
        *self.power_state.lock() = state;

        // 在实际实现中，这里会保存设备状态

        Ok(())
    }

    fn resume(&mut self, device: &mut dyn Device) -> Result<()> {
        crate::println!(
            "base_driver: resuming device '{}'",
            device.name()
        );

        // 更新电源状态
        *self.power_state.lock() = PowerState::D0;

        // 在实际实现中，这里会恢复设备状态

        device.set_status(DeviceStatus::Ready);

        Ok(())
    }

    fn handle_interrupt(&mut self, device: &mut dyn Device, irq: u32) -> Result<()> {
        self.record_interrupt();

        crate::println!(
            "base_driver: handling interrupt {} for device '{}'",
            irq,
            device.name()
        );

        // 在实际实现中，这里会处理设备特定的中断

        Ok(())
    }

    fn handle_io(&mut self, device: &mut dyn Device, operation: IoOperation) -> Result<IoResult> {
        self.record_io();

        // 在实际实现中，这里会处理设备特定的I/O操作
        // 这里提供一个默认实现

        match operation {
            IoOperation::Read { offset, size } => {
                crate::println!(
                    "base_driver: read from device '{}' offset {} size {}",
                    device.name(),
                    offset,
                    size
                );
                // 返回空数据作为占位符
                Ok(IoResult::ReadResult {
                    data: Vec::new(),
                    bytes_read: 0,
                })
            }
            IoOperation::Write { offset, data } => {
                crate::println!(
                    "base_driver: write to device '{}' offset {} size {}",
                    device.name(),
                    offset,
                    data.len()
                );
                Ok(IoResult::WriteResult {
                    bytes_written: data.len() as u64,
                })
            }
            IoOperation::Ioctl { command, arg } => {
                crate::println!(
                    "base_driver: ioctl on device '{}' command {} arg {}",
                    device.name(),
                    command,
                    arg
                );
                Ok(IoResult::IoctlResult { result: 0 })
            }
            IoOperation::Mmap { offset, size, permissions } => {
                crate::println!(
                    "base_driver: mmap on device '{}' offset {} size {} perms {}",
                    device.name(),
                    offset,
                    size,
                    permissions
                );
                Ok(IoResult::MmapResult { address: 0 })
            }
            IoOperation::Munmap { offset, size } => {
                crate::println!(
                    "base_driver: munmap on device '{}' offset {} size {}",
                    device.name(),
                    offset,
                    size
                );
                Ok(IoResult::MunmapResult)
            }
        }
    }

    fn cleanup(&mut self) -> Result<()> {
        crate::println!("base_driver: cleaning up driver '{}'", self.name);

        // 清理所有管理的设备
        {
            let mut devices = self.managed_devices.lock();
            devices.clear();
        }

        self.initialized.store(false, Ordering::SeqCst);

        Ok(())
    }
}

// ============================================================================
// 驱动程序构建器
// ============================================================================

/// 驱动程序构建器 - 简化驱动程序的创建
pub struct DriverBuilder {
    name: String,
    version: String,
    supported_devices: Vec<DeviceIdentifier>,
}

impl DriverBuilder {
    /// 创建新的驱动程序构建器
    pub fn new(name: String) -> Self {
        Self {
            name,
            version: "1.0.0".to_string(),
            supported_devices: Vec::new(),
        }
    }

    /// 设置驱动程序版本
    pub fn version(mut self, version: String) -> Self {
        self.version = version;
        self
    }

    /// 添加支持的设备
    pub fn supported_device(mut self, device: DeviceIdentifier) -> Self {
        self.supported_devices.push(device);
        self
    }

    /// 添加多个支持的设备
    pub fn supported_devices(mut self, devices: Vec<DeviceIdentifier>) -> Self {
        self.supported_devices.extend(devices);
        self
    }

    /// 构建驱动程序
    pub fn build(self) -> BaseDriver {
        BaseDriver::new(
            self.name,
            self.version,
            self.supported_devices,
        )
    }
}

// ============================================================================
// 辅助函数
// ============================================================================

/// 创建简单的设备标识符
pub fn make_device_id(
    device_type: DeviceType,
    vendor_id: u16,
    device_id: u16,
) -> DeviceIdentifier {
    DeviceIdentifier::new(device_type, vendor_id, device_id)
}

/// 创建网络设备标识符
pub fn make_network_device_id(vendor_id: u16, device_id: u16) -> DeviceIdentifier {
    DeviceIdentifier::new(DeviceType::Network, vendor_id, device_id)
}

/// 创建块设备标识符
pub fn make_block_device_id(vendor_id: u16, device_id: u16) -> DeviceIdentifier {
    DeviceIdentifier::new(DeviceType::Block, vendor_id, device_id)
}

/// 创建字符设备标识符
pub fn make_char_device_id(vendor_id: u16, device_id: u16) -> DeviceIdentifier {
    DeviceIdentifier::new(DeviceType::Character, vendor_id, device_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_builder() {
        let device_id = make_network_device_id(0x8086, 0x100e);

        let driver = DriverBuilder::new("test_driver".to_string())
            .version("2.0.0".to_string())
            .supported_device(device_id)
            .build();

        assert_eq!(driver.name(), "test_driver");
        assert_eq!(driver.version(), "2.0.0");
        assert_eq!(driver.supported_devices().len(), 1);
    }

    #[test]
    fn test_generic_device() {
        let device_id = DeviceIdentifier::new(DeviceType::Network, 0x8086, 0x100e);
        let device = GenericDevice::new(1, device_id, "test_device".to_string());

        assert_eq!(device.id(), 1);
        assert_eq!(device.name(), "test_device");
        assert_eq!(device.status(), DeviceStatus::Uninitialized);
    }

    #[test]
    fn test_device_properties() {
        let device_id = DeviceIdentifier::new(DeviceType::Network, 0x8086, 0x100e);
        let mut device = GenericDevice::new(1, device_id, "test_device".to_string());

        device.set_property("test_key", "test_value").unwrap();
        assert_eq!(device.get_property("test_key"), Some("test_value".to_string()));
        assert_eq!(device.get_property("nonexistent"), None);
    }

    #[test]
    fn test_driver_statistics() {
        let device_id = make_network_device_id(0x8086, 0x100e);
        let mut driver = DriverBuilder::new("test".to_string())
            .supported_device(device_id)
            .build();

        let mut device = GenericDevice::new(1, device_id, "test".to_string());

        // 模拟一些操作
        let _ = driver.probe(&device);
        let _ = driver.start(&mut device);

        let stats = driver.get_statistics();
        assert_eq!(stats.probes, 1);
        assert_eq!(stats.bindings, 1);
    }
}
