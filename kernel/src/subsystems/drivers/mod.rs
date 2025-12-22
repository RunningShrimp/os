//! 驱动程序模块
//! 
//! 本模块提供可扩展的驱动程序架构，包括：
//! - 驱动程序接口
//! - 驱动程序管理器
//! - 设备抽象层
//! - 驱动程序生命周期管理
//! - 设备资源管理

pub mod driver_manager;
pub mod example_char_driver;
pub mod disk_io;
pub mod device_model;
pub mod device_discovery;
pub mod driver_registration;
pub mod basic_drivers;
pub mod pci_device_manager;
pub mod usb_device_manager;
pub mod gpu_driver_framework;

pub use driver_manager::*;
pub use example_char_driver::*;
pub use disk_io::*;
pub use device_model::*;
pub use device_discovery::*;
pub use driver_registration::*;
pub use basic_drivers::*;
pub use pci_device_manager::*;
pub use usb_device_manager::*;