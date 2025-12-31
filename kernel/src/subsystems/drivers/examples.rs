//! 示例驱动程序
//!
//! 本模块提供示例驱动程序实现，包括：
//! - 虚拟驱动（用于测试）
//! - 字符设备驱动
//! - 块设备驱动

use crate::prelude::*;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use spin::Mutex;
use core::sync::atomic::{AtomicU64, Ordering};

use super::framework::{
    Device, DeviceIdentifier, Driver, PowerState
};
use super::base::BaseDriver;
use crate::subsystems::drivers::driver_manager::{
    DeviceType, IoOperation, IoResult
};

// ============================================================================
// 虚拟驱动（用于测试）
// ============================================================================

/// 虚拟驱动状态
#[derive(Debug, Default)]
struct VirtualDriverState {
    /// 读计数
    read_count: AtomicU64,
    /// 写计数
    write_count: AtomicU64,
    /// 中断计数
    interrupt_count: AtomicU64,
    /// 最后的电源状态
    last_power_state: Mutex<PowerState>,
}

/// 虚拟驱动程序 - 用于测试驱动框架
pub struct VirtualDriver {
    /// 基础驱动
    base: BaseDriver,
    /// 驱动状态
    state: VirtualDriverState,
}

impl VirtualDriver {
    /// 创建新的虚拟驱动
    pub fn new(name: String) -> Self {
        let supported_devices = vec![
            DeviceIdentifier::new(DeviceType::Character, 0xFFFF, 0x0001),
            DeviceIdentifier::new(DeviceType::Block, 0xFFFF, 0x0002),
        ];

        Self {
            base: BaseDriver::new(name, "1.0.0".to_string(), supported_devices),
            state: VirtualDriverState::default(),
        }
    }

    /// 获取读计数
    pub fn read_count(&self) -> u64 {
        self.state.read_count.load(Ordering::SeqCst)
    }

    /// 获取写计数
    pub fn write_count(&self) -> u64 {
        self.state.write_count.load(Ordering::SeqCst)
    }

    /// 获取中断计数
    pub fn interrupt_count(&self) -> u64 {
        self.state.interrupt_count.load(Ordering::SeqCst)
    }

    /// 重置计数器
    pub fn reset_counters(&self) {
        self.state.read_count.store(0, Ordering::SeqCst);
        self.state.write_count.store(0, Ordering::SeqCst);
        self.state.interrupt_count.store(0, Ordering::SeqCst);
    }
}

impl Driver for VirtualDriver {
    fn name(&self) -> &str {
        self.base.name()
    }

    fn version(&self) -> &str {
        self.base.version()
    }

    fn supported_devices(&self) -> &[DeviceIdentifier] {
        self.base.supported_devices()
    }

    fn initialize(&mut self) -> Result<()> {
        self.base.initialize()?;
        crate::println!("virtual_driver: initialized");
        Ok(())
    }

    fn probe(&mut self, device: &dyn Device) -> Result<bool> {
        self.base.probe(device)
    }

    fn start(&mut self, device: &mut dyn Device) -> Result<()> {
        self.base.start(device)?;
        crate::println!(
            "virtual_driver: started device '{}'",
            device.name()
        );
        Ok(())
    }

    fn stop(&mut self, device: &mut dyn Device) -> Result<()> {
        self.base.stop(device)?;
        crate::println!(
            "virtual_driver: stopped device '{}'",
            device.name()
        );
        Ok(())
    }

    fn remove(&mut self, device: &mut dyn Device) -> Result<()> {
        self.base.remove(device)?;
        crate::println!(
            "virtual_driver: removed device '{}'",
            device.name()
        );
        Ok(())
    }

    fn suspend(&mut self, device: &mut dyn Device, state: PowerState) -> Result<()> {
        self.base.suspend(device, state)?;
        *self.state.last_power_state.lock() = state;
        Ok(())
    }

    fn resume(&mut self, device: &mut dyn Device) -> Result<()> {
        self.base.resume(device)?;
        *self.state.last_power_state.lock() = PowerState::D0;
        Ok(())
    }

    fn handle_interrupt(&mut self, device: &mut dyn Device, irq: u32) -> Result<()> {
        self.state.interrupt_count.fetch_add(1, Ordering::SeqCst);
        self.base.handle_interrupt(device, irq)?;
        Ok(())
    }

    fn handle_io(&mut self, device: &mut dyn Device, operation: IoOperation) -> Result<IoResult> {
        match operation {
            IoOperation::Read { offset, size } => {
                self.state.read_count.fetch_add(1, Ordering::SeqCst);

                // 返回测试数据
                let data = vec![0xAA as u8; size as usize];

                crate::println!(
                    "virtual_driver: read from '{}' offset {} size {}",
                    device.name(),
                    offset,
                    size
                );

                Ok(IoResult::ReadResult {
                    data,
                    bytes_read: size,
                })
            }
            IoOperation::Write { offset, data } => {
                self.state.write_count.fetch_add(1, Ordering::SeqCst);

                crate::println!(
                    "virtual_driver: write to '{}' offset {} size {}",
                    device.name(),
                    offset,
                    data.len()
                );

                Ok(IoResult::WriteResult {
                    bytes_written: data.len() as u64,
                })
            }
            _ => self.base.handle_io(device, operation),
        }
    }

    fn cleanup(&mut self) -> Result<()> {
        self.base.cleanup()?;
        crate::println!("virtual_driver: cleaned up");
        Ok(())
    }
}

// ============================================================================
// 字符设备驱动
// ============================================================================

/// 字符设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharDeviceType {
    /// 串口设备
    Serial,
    /// 并口设备
    Parallel,
    /// 控制台设备
    Console,
    /// 虚拟终端
    Tty,
    /// 随机数生成器
    Random,
    /// 空设备
    Null,
}

/// 字符设备驱动
pub struct CharDeviceDriver {
    /// 基础驱动
    base: BaseDriver,
    /// 设备类型
    device_type: CharDeviceType,
    /// 主设备号
    major: u32,
    /// 次设备号起始
    minor_base: u32,
    /// 数据缓冲区
    buffer: Mutex<Vec<u8>>,
}

impl CharDeviceDriver {
    /// 创建新的字符设备驱动
    pub fn new(
        name: String,
        device_type: CharDeviceType,
        major: u32,
        minor_base: u32,
    ) -> Self {
        let supported_devices = vec![
            DeviceIdentifier::new(DeviceType::Character, 0xFF00, major as u16),
        ];

        Self {
            base: BaseDriver::new(name, "1.0.0".to_string(), supported_devices),
            device_type,
            major,
            minor_base,
            buffer: Mutex::new(Vec::new()),
        }
    }

    /// 获取设备类型
    pub fn device_type(&self) -> CharDeviceType {
        self.device_type
    }

    /// 获取主设备号
    pub fn major(&self) -> u32 {
        self.major
    }

    /// 获取次设备号起始
    pub fn minor_base(&self) -> u32 {
        self.minor_base
    }

    /// 读取设备
    pub fn read(&self, minor: u32, buf: &mut [u8]) -> Result<usize> {
        let _device_id = (self.major << 8) | (self.minor_base + minor);

        match self.device_type {
            CharDeviceType::Null => {
                // 空设备总是返回0
                Ok(0)
            }
            CharDeviceType::Random => {
                // 随机设备返回随机数据
                for byte in buf.iter_mut() {
                    *byte = 0x55; // 占位符
                }
                Ok(buf.len())
            }
            _ => {
                // 从缓冲区读取
                let buffer = self.buffer.lock();
                let bytes_to_read = core::cmp::min(buffer.len(), buf.len());
                buf[..bytes_to_read].copy_from_slice(&buffer[..bytes_to_read]);
                Ok(bytes_to_read)
            }
        }
    }

    /// 写入设备
    pub fn write(&self, minor: u32, buf: &[u8]) -> Result<usize> {
        let _device_id = (self.major << 8) | (self.minor_base + minor);

        match self.device_type {
            CharDeviceType::Null => {
                // 空设备忽略所有写入
                Ok(buf.len())
            }
            _ => {
                // 写入到缓冲区
                let mut buffer = self.buffer.lock();
                buffer.extend_from_slice(buf);
                Ok(buf.len())
            }
        }
    }

    /// IO控制
    pub fn ioctl(&self, minor: u32, cmd: u32, arg: u64) -> Result<u64> {
        let _device_id = (self.major << 8) | (self.minor_base + minor);

        crate::println!(
            "char_driver: ioctl on device {}:{}, cmd={}, arg={:#x}",
            self.major,
            minor,
            cmd,
            arg
        );

        // 返回0表示成功
        Ok(0)
    }
}

impl Driver for CharDeviceDriver {
    fn name(&self) -> &str {
        self.base.name()
    }

    fn version(&self) -> &str {
        self.base.version()
    }

    fn supported_devices(&self) -> &[DeviceIdentifier] {
        self.base.supported_devices()
    }

    fn initialize(&mut self) -> Result<()> {
        self.base.initialize()?;
        crate::println!(
            "char_driver: initialized {} (major={})",
            self.name(),
            self.major
        );
        Ok(())
    }

    fn probe(&mut self, device: &dyn Device) -> Result<bool> {
        self.base.probe(device)
    }

    fn start(&mut self, device: &mut dyn Device) -> Result<()> {
        self.base.start(device)?;
        Ok(())
    }

    fn stop(&mut self, device: &mut dyn Device) -> Result<()> {
        self.base.stop(device)?;
        Ok(())
    }

    fn remove(&mut self, device: &mut dyn Device) -> Result<()> {
        self.base.remove(device)?;
        Ok(())
    }

    fn suspend(&mut self, device: &mut dyn Device, state: PowerState) -> Result<()> {
        self.base.suspend(device, state)
    }

    fn resume(&mut self, device: &mut dyn Device) -> Result<()> {
        self.base.resume(device)
    }

    fn handle_interrupt(&mut self, device: &mut dyn Device, irq: u32) -> Result<()> {
        self.base.handle_interrupt(device, irq)
    }

    fn handle_io(&mut self, device: &mut dyn Device, operation: IoOperation) -> Result<IoResult> {
        self.base.handle_io(device, operation)
    }

    fn cleanup(&mut self) -> Result<()> {
        self.base.cleanup()?;
        Ok(())
    }
}

// ============================================================================
// 工厂函数
// ============================================================================

/// 创建虚拟驱动
pub fn create_virtual_driver(name: String) -> Box<dyn Driver> {
    Box::new(VirtualDriver::new(name))
}

/// 创建null字符设备驱动
pub fn create_null_driver() -> CharDeviceDriver {
    CharDeviceDriver::new(
        "null".to_string(),
        CharDeviceType::Null,
        1,
        3,
    )
}

/// 创建zero字符设备驱动
pub fn create_zero_driver() -> CharDeviceDriver {
    CharDeviceDriver::new(
        "zero".to_string(),
        CharDeviceType::Null,
        1,
        5,
    )
}

/// 创建random字符设备驱动
pub fn create_random_driver() -> CharDeviceDriver {
    CharDeviceDriver::new(
        "random".to_string(),
        CharDeviceType::Random,
        1,
        8,
    )
}

/// 创建tty字符设备驱动
pub fn create_tty_driver(major: u32, minor_base: u32) -> CharDeviceDriver {
    CharDeviceDriver::new(
        format!("tty{}", major),
        CharDeviceType::Tty,
        major,
        minor_base,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_virtual_driver() {
        let mut driver = VirtualDriver::new("test_virtual".to_string());

        assert_eq!(driver.name(), "test_virtual");
        assert_eq!(driver.version(), "1.0.0");
        assert_eq!(driver.read_count(), 0);
        assert_eq!(driver.write_count(), 0);

        driver.reset_counters();
        assert_eq!(driver.read_count(), 0);
    }

    #[test]
    fn test_char_device_null() {
        let driver = create_null_driver();

        assert_eq!(driver.device_type(), CharDeviceType::Null);
        assert_eq!(driver.major(), 1);
        assert_eq!(driver.minor_base(), 3);

        // 测试null设备读写
        let mut buf = [0u8; 10];
        let read = driver.read(0, &mut buf).unwrap();
        assert_eq!(read, 0);

        let written = driver.write(0, &[1, 2, 3]).unwrap();
        assert_eq!(written, 3);
    }

    #[test]
    fn test_char_device_random() {
        let driver = create_random_driver();

        assert_eq!(driver.device_type(), CharDeviceType::Random);

        // 测试random设备
        let mut buf = [0u8; 10];
        let read = driver.read(0, &mut buf).unwrap();
        assert_eq!(read, 10);
    }
}
