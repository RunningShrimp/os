# Driver Model

## Overview

The NOS kernel provides a comprehensive, trait-based driver framework for managing hardware devices. The driver model supports multiple bus types, automatic device discovery, hot-plug events, and power management.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Application / Userspace                │
└────────────────────┬────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│                System Call Layer                    │
└────────────────────┬────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│                  VFS Layer                        │
│  - /dev/* device files                           │
│  - /sys/* device attributes                     │
└────────────────────┬────────────────────────────────┘
                     │
        ┌────────────┴────────────┐
        ▼                         ▼
┌──────────────┐      ┌──────────────┐
│   VFS Device │      │   Driver     │
│   Interface  │◄────►│  Framework   │
└──────────────┘      └──────┬───────┘
                              │
        ┌─────────────────────┴─────────────────────┐
        ▼                                       ▼
┌──────────────────┐                    ┌──────────────────┐
│  Device Model   │                    │ Device Discovery │
│  - Hierarchy   │                    │ - PCI scan     │
│  - Power mgmt   │                    │ - USB enum    │
│  - Classes      │                    │ - ACPI tables  │
└────────┬─────────┘                    └────────┬─────────┘
         │                                       │
         ▼                                       ▼
┌─────────────────────────────────────────────────────────────┐
│                  Bus Managers                        │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐     │
│  │   PCI     │  │   USB     │  │  Platform │     │
│  │  Manager  │  │  Manager  │  │  Manager  │     │
│  └──────────┘  └──────────┘  └──────────┘     │
└─────────────────────────────────────────────────────────────┘
         │               │               │
         ▼               ▼               ▼
┌─────────────────────────────────────────────────────────────┐
│                   Concrete Drivers                      │
│  - ConsoleDriver (character)                          │
│  - BlockDeviceDriver (storage)                        │
│  - NetworkDeviceDriver (network)                       │
│  - Vendor-specific drivers                              │
└─────────────────────────────────────────────────────────────┘
```

## Module Structure

| Module | Description |
|--------|-------------|
| `mod.rs` | Main driver module, exports all components |
| `driver_manager.rs` | Core driver framework and DriverManager |
| `driver_registration.rs` | Driver registration and compatibility checking |
| `device_model.rs` | Enhanced device model with hierarchy |
| `device_discovery.rs` | Automatic device enumeration |
| `pci_device_manager.rs` | PCI device management |
| `usb_device_manager.rs` | USB device management |
| `example_char_driver.rs` | Example character driver |
| `basic_drivers.rs` | Basic reference drivers |

## Core Traits

### Driver

All drivers must implement the `Driver` trait:

```rust
pub trait Driver {
    fn get_info(&self) -> DriverInfo;
    fn initialize(&mut self) -> Result<(), KernelError>;
    fn cleanup(&mut self) -> Result<(), KernelError>;
    fn probe_device(&self, device_info: &DeviceInfo) -> Result<bool, KernelError>;
    fn add_device(&mut self, device_info: &DeviceInfo) -> Result<(), KernelError>;
    fn remove_device(&mut self, device_id: DeviceId) -> Result<(), KernelError>;
    fn handle_io(&mut self, device_id: DeviceId, operation: IoOperation) -> Result<IoResult, KernelError>;
    fn suspend(&mut self, device_id: DeviceId) -> Result<(), KernelError>;
    fn resume(&mut self, device_id: DeviceId) -> Result<(), KernelError>;
    fn reset(&mut self, device_id: DeviceId) -> Result<(), KernelError>;
}
```

### DeviceModel

Enhanced device model with hierarchy and power management:

```rust
pub trait DeviceModel {
    fn register_device(&mut self, device_info: EnhancedDeviceInfo) -> Result<DeviceId, KernelError>;
    fn unregister_device(&mut self, device_id: DeviceId) -> Result<(), KernelError>;
    fn get_device_info(&self, device_id: DeviceId) -> Result<EnhancedDeviceInfo, KernelError>;
    fn find_devices_by_class(&self, device_class: DeviceClass) -> Result<Vec<DeviceId>, KernelError>;
    fn get_parent_device(&self, device_id: DeviceId) -> Result<Option<DeviceId>, KernelError>;
    fn get_child_devices(&self, device_id: DeviceId) -> Result<Vec<DeviceId>, KernelError>;
    fn set_power_state(&mut self, device_id: DeviceId, state: DevicePowerState) -> Result<(), KernelError>;
    fn get_performance_metrics(&self, device_id: DeviceId) -> Result<DeviceMetrics, KernelError>;
}
```

### BusDiscovery

Interface for bus-specific device discovery:

```rust
pub trait BusDiscovery {
    fn scan(&self) -> Result<Vec<DeviceInfo>, KernelError>;
    fn get_bus_type(&self) -> BusType;
    fn handle_hotplug(&self, event: HotplugEvent) -> Result<(), KernelError>;
    fn enumerate_children(&self, parent_id: DeviceId) -> Result<Vec<DeviceInfo>, KernelError>;
}
```

## Key Structures

### DriverInfo

Metadata about a driver:

```rust
pub struct DriverInfo {
    pub name: String,
    pub version: (u32, u32, u32),
    pub author: String,
    pub description: String,
    pub license: String,
    pub supported_devices: Vec<DeviceId>,
}
```

### DeviceInfo

Basic device information:

```rust
pub struct DeviceInfo {
    pub id: DeviceId,
    pub name: String,
    pub device_type: DeviceType,
    pub bus_type: BusType,
    pub resources: DeviceResources,
    pub parent_id: Option<DeviceId>,
}
```

### EnhancedDeviceInfo

Comprehensive device information with power and performance:

```rust
pub struct EnhancedDeviceInfo {
    pub base: DeviceInfo,
    pub device_class: DeviceClass,
    pub power_state: DevicePowerState,
    pub capabilities: DeviceCapabilities,
    pub metrics: DeviceMetrics,
    pub children: Vec<DeviceId>,
}
```

### DeviceResources

Resources assigned to a device:

```rust
pub struct DeviceResources {
    pub memory_regions: Vec<MemoryRegion>,
    pub io_ports: Vec<IoPortRange>,
    pub interrupts: Vec<InterruptLine>,
    pub dma_channels: Vec<DmaChannel>,
}
```

## Device Classes

```rust
pub enum DeviceClass {
    Unknown,
    Processor,
    Memory,
    Network,
    Display,
    Storage,
    Input,
    Audio,
    Serial,
    Parallel,
    Timer,
    Watchdog,
    Platform,
    Crypto,
    Graphics,
}
```

## Bus Types

```rust
pub enum BusType {
    Pci,
    Isa,
    Usb,
    I2c,
    Spi,
    Sdio,
    Platform,
    Acpi,
    Fdt,
}
```

## Power States

```rust
pub enum DevicePowerState {
    D0,   // Fully on
    D1,   // Partial power savings
    D2,   // Significant power savings
    D3Hot, // Can wake up
    D3Cold,// Cannot wake up
}
```

## Driver Registration

### Registering a Driver

```rust
use crate::subsystems::drivers::driver_registration::{DriverRegistrationManager, DriverRegistrationInfo};

let registration = DriverRegistrationInfo {
    name: "my_driver".to_string(),
    version: (1, 0, 0),
    author: "Author Name".to_string(),
    driver: Arc::new(Mutex::new(MyDriver::new())),
    compatibility: vec![DriverCompatibility::PciClass(0x02, 0x00, 0x00)],
};

DriverRegistrationManager::register_driver(registration)?;
```

### Automatic Device Binding

When a device is discovered, the driver manager:
1. Queries registered drivers for compatibility
2. Calls `probe_device()` on compatible drivers
3. Binds the first driver that successfully probes
4. Calls `add_device()` on the bound driver

### Hot-plug Events

```rust
pub enum HotplugEvent {
    DeviceAdded(DeviceInfo),
    DeviceRemoved(DeviceId),
    DeviceStateChanged(DeviceId, DeviceState),
}
```

## I/O Operations

```rust
pub enum IoOperation {
    Read { offset: u64, buffer: *mut u8, length: usize },
    Write { offset: u64, buffer: *const u8, length: usize },
    Ioctl { command: u32, arg: usize },
    Mmap { offset: u64, size: usize, flags: u32 },
    Poll { events: u32 },
}
```

## Example: Character Driver

```rust
pub struct ConsoleDriver {
    devices: HashMap<DeviceId, ConsoleDevice>,
}

impl Driver for ConsoleDriver {
    fn get_info(&self) -> DriverInfo {
        DriverInfo {
            name: "console".to_string(),
            version: (1, 0, 0),
            author: "NOS Team".to_string(),
            description: "Console/TTY driver".to_string(),
            license: "MIT".to_string(),
            supported_devices: vec![],
        }
    }

    fn initialize(&mut self) -> Result<(), KernelError> {
        self.devices = HashMap::new();
        Ok(())
    }

    fn handle_io(&mut self, device_id: DeviceId, operation: IoOperation) -> Result<IoResult, KernelError> {
        match operation {
            IoOperation::Read { buffer, length, .. } => {
                self.read(device_id, unsafe { core::slice::from_raw_parts_mut(buffer, length) })
            }
            IoOperation::Write { buffer, length, .. } => {
                self.write(device_id, unsafe { core::slice::from_raw_parts(buffer, length) })
            }
            _ => Err(KernelError::NotSupported),
        }
    }
}
```

## PCI Device Manager

The PCI manager handles PCI-specific device enumeration and management:

```rust
pub struct PciDeviceManager {
    devices: HashMap<DeviceId, PciDeviceInfo>,
    config_space: Mutex<HashMap<PciAddress, PciConfigHeader>>,
}

impl PciDeviceManager {
    pub fn scan_bus(&mut self) -> Result<Vec<DeviceId>, KernelError> {
        let mut found = Vec::new();
        for bus in 0..256 {
            for device in 0..32 {
                for function in 0..8 {
                    let addr = PciAddress::new(bus, device, function);
                    if let Some(config) = self.read_config(addr)? {
                        let info = PciDeviceInfo::from_config(addr, &config)?;
                        self.devices.insert(info.id, info);
                        found.push(info.id);
                    }
                }
            }
        }
        Ok(found)
    }

    pub fn enable_device(&self, device_id: DeviceId) -> Result<(), KernelError> {
        let mut config = self.config_space.lock();
        let pci_addr = self.get_pci_address(device_id)?;
        let header = config.get_mut(&pci_addr).unwrap();
        header.command |= PCI_COMMAND_BUS_MASTER | PCI_COMMAND_IO_SPACE;
        Ok(())
    }
}
```

## Integration Points

### VFS Integration

Devices are accessible through the VFS:

```rust
// /dev/console
// /dev/sda
// /dev/tty0
```

### Sysfs Integration

Device attributes exposed through sysfs:

```bash
/sys/devices/pci0000:00/0000:00:1f.0/vendor
/sys/devices/pci0000:00/0000:00:1f.0/device
/sys/devices/pci0000:00/0000:00:1f.0/power/control
```

## Future Improvements

- [ ] Implement driver signing and verification
- [ ] Add driver module loading/unloading
- [ ] Implement device tree (FDT) support
- [ ] Add more bus types (I2C, SPI, SDIO)
- [ ] Implement power management policies
- [ ] Add performance monitoring dashboard
- [ ] Implement device throttling
- [ ] Add NUMA-aware device management
- [ ] Implement virtual device drivers (vhost, vGPU)
