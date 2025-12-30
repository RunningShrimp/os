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

// Block device driver framework (Track R - Stage 3-1)
pub mod block;
pub mod block_request;
pub mod block_examples;

// 新增驱动框架模块
pub mod framework;
pub mod base;
pub mod pci_msi;
pub mod examples;

// Selectively re-export to avoid naming conflicts
pub use driver_manager::{DriverManager, DriverManagerConfig, DriverStatistics, get_driver_manager};
pub use driver_manager::{DeviceId, DriverId};
pub use driver_manager::{DeviceType, DeviceStatus, DriverStatus};
pub use driver_manager::{DeviceInfo, DriverInfo};
pub use driver_manager::{DeviceResources, MemoryRegion, IoPortRange, InterruptLine, DmaChannel};
pub use driver_manager::{IoOperation, IoResult, InterruptInfo};
pub use driver_manager::Driver;
pub use example_char_driver::*;
// Note: disk_io module has DiskIoDriver, not DiskIOManager
pub use disk_io::{DiskIoDriver, DiskIoType, DiskIoStatus, DiskIoPriority, DiskIoRequest};
pub use device_model::{DeviceModel, EnhancedDeviceInfo, DeviceClass, DevicePowerState, EnhancedDeviceModel, get_enhanced_device_model};
pub use device_discovery::{BusType, DiscoveryEvent, DiscoveryEventType, DeviceDiscoveryManager, get_device_discovery_manager};
pub use driver_registration::{DriverRegistrationManager, DriverRegistrationStatus, DriverPriority, get_driver_registration_manager};
pub use basic_drivers::*;
// Note: HotplugEventType is defined in both pci and usb modules - import from pci to avoid conflict
pub use pci_device_manager::{PciDeviceManager, PciAddress, PciConfigHeader, PciDeviceInfo, HotplugEventType, init as pci_init, get_pci_device_manager};
pub use usb_device_manager::{UsbDeviceManager, init as usb_init, get_usb_device_manager};

// 导出新框架
pub use framework::{
    Device, DeviceIdentifier, Driver as FrameworkDriver,
    PowerState, DriverManager as FrameworkDriverManager,
    Irq, IrqType, IrqTriggerMode
};
pub use base::{BaseDriver, GenericDevice, DriverBuilder};
pub use pci_msi::{MsiManager, MsiVector, MsiXTable};
pub use examples::{
    VirtualDriver, CharDeviceDriver, CharDeviceType,
    create_virtual_driver, create_null_driver, create_zero_driver,
    create_random_driver, create_tty_driver
};

// Re-export block device framework (Track R - Stage 3-1)
pub use block::{
    BlockDevice, Bio, BlockOp, BioFlags, BioStatus,
    BlockDeviceStats, RequestQueue, BlockDeviceManager,
    DEFAULT_SECTOR_SIZE, init_block_device_manager, get_block_manager, register_block_device,
};
pub use block_request::{
    BlockRequest, RequestFlags, SectorManager, BufferCache,
    IoScheduler, IoSchedulerImpl, BufferStrategy, ReadaheadPolicy,
    RequestOptimizer, DEFAULT_READAHEAD_SECTORS, MAX_READAHEAD_SECTORS,
};
pub use block_examples::{
    RamDisk, RamDiskConfig, RamDiskStats,
    VirtualBlockDevice, VirtualBlockConfig,
    PerfTest, PerfTestConfig, PerfTestResults,
    init_example_devices, demonstrate_basic_operations,
    mount_block_device, create_device_node,
};