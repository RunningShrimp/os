# Userspace I/O (UIO) Driver Framework

A comprehensive, production-ready framework for implementing device drivers in userspace with safe memory mapping and interrupt delivery.

## Overview

The UIO (Userspace I/O) framework enables safe and efficient userspace device drivers by providing:

- **Memory Mapping**: Direct access to device MMIO regions from userspace
- **Interrupt Handling**: Hardware interrupt delivery via eventfd mechanism
- **Device Management**: Registration, lifecycle, and enumeration
- **Sysfs Integration**: Runtime configuration and debugging interface
- **Character Device API**: Standard file operations for userspace interaction

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Userspace Application                    │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │
│  │  Device I/O  │  │ Interrupt    │  │  Config      │      │
│  │  (mmap)      │  │  (read/poll) │  │  (sysfs)     │      │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘      │
└─────────┼──────────────────┼──────────────────┼─────────────┘
          │                  │                  │
          ▼                  ▼                  ▼
┌─────────────────────────────────────────────────────────────┐
│                      Kernel UIO Framework                    │
│  ┌─────────────────────────────────────────────────────┐   │
│  │                    UIO Device                        │   │
│  │  ┌─────────┐  ┌──────────┐  ┌──────────────────┐   │   │
│  │  │ Memory  │  │Interrupt │  │  Device Registry │   │   │
│  │  │ Regions │  │Controller│  │                  │   │   │
│  │  └────┬────┘  └────┬─────┘  └────────┬─────────┘   │   │
│  └───────┼────────────┼──────────────────┼──────────────┘   │
│          │            │                  │                   │
│  ┌───────▼────────────▼──────────────────▼───────────────┐  │
│  │           Character Device (/dev/uioX)                 │  │
│  │           Sysfs Interface (/sys/class/uio/uioX)        │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
          │                    │
          ▼                    ▼
    ┌──────────┐        ┌────────────┐
    │ Hardware │        │ Interrupt  │
    │  Device  │◄───────│ Controller │
    └──────────┘        └────────────┘
```

## Components

### Core Modules

#### 1. **Device Management** (`device.rs`)
- `UioDevice`: Represents a userspace I/O device
- `UioDeviceRegistry`: Tracks all registered devices
- Device lifecycle: registration, activation, cleanup
- Minor number allocation and management

#### 2. **Memory Regions** (`memory.rs`)
- `UioMemRegion`: Descriptor for memory-mappable regions
- Support for MMIO, Port I/O, and custom regions
- Access control: read/write/execute permissions
- Region validation and mapping

#### 3. **Interrupt Handling** (`interrupt.rs`)
- `UioInterrupt`: Interrupt controller for UIO
- Eventfd integration for userspace notification
- CPU affinity management
- Interrupt statistics tracking

#### 4. **Userspace API** (`api.rs`)
- Character device file operations
- IOCTL commands for device control
- Memory mapping via mmap()
- Interrupt notification via read/poll

#### 5. **Sysfs Interface** (`sysfs.rs`)
- Device attributes in `/sys/class/uio/`
- Runtime configuration interface
- Debugging and statistics

#### 6. **Example Drivers** (`examples.rs`)
- Simple UIO device template
- PCI driver template
- Network card driver example
- GPIO controller driver

## Usage Examples

### Basic Device Creation

```rust
use kernel::drivers::uio::{UioDevice, UioMemRegion, UioDriver};

// Create a UIO device
let mut device = UioDevice::new("my_device", 0);

// Add memory regions
device.add_region(UioMemRegion::mmio(0xF0000000, 0x1000))?;
device.add_region(UioMemRegion::mmio(0xF0001000, 0x2000))?;

// Register interrupt handler
device.register_interrupt(|irq| {
    println!("Interrupt {}", irq);
    Ok(())
})?;

// Set IRQ number
device.set_irq(42);

// Register with subsystem
let mut driver = UioDriver::new();
driver.init()?;
driver.register_device(device)?;
```

### Using the Simple Driver Template

```rust
use kernel::drivers::uio::examples::SimpleUioDriver;

// Create and initialize a simple driver
let mut driver = SimpleUioDriver::new("simple", 0xF0000000, 42);
driver.init()?;

// Device is now accessible from userspace
// at /dev/uio0 and /sys/class/uio/uio0/
```

### PCI Driver Template

```rust
use kernel::drivers::uio::examples::UioPciDriver;

// Create PCI driver
let mut driver = UioPciDriver::new("my_pci", 0x1234, 0x5678);
driver.init()?;

// BARs automatically mapped as memory regions
// MSI interrupts configured
```

## Userspace API

### Device Operations

#### Open Device
```c
int fd = open("/dev/uio0", O_RDWR);
```

#### Map Memory Regions
```c
// Map first memory region
void *base = mmap(NULL, region_size,
                  PROT_READ | PROT_WRITE,
                  MAP_SHARED, fd, 0);

if (base == MAP_FAILED) {
    perror("mmap");
    return -1;
}

// Access device registers
uint32_t reg = *((volatile uint32_t *)(base + REGISTER_OFFSET));
```

#### Wait for Interrupts
```c
// Blocking read
uint32_t event_count;
read(fd, &event_count, sizeof(event_count));

// Or use poll/select/epoll
struct pollfd pfd = { .fd = fd, .events = POLLIN };
poll(&pfd, 1, -1);
```

#### Control Device
```c
// Enable interrupt
ioctl(fd, UIO_ENABLE_INTERRUPT, 0);

// Disable interrupt
ioctl(fd, UIO_DISABLE_INTERRUPT, 0);

// Get device info
struct uio_info info;
ioctl(fd, UIO_GET_INFO, &info);
```

### Sysfs Interface

Device attributes available in `/sys/class/uio/uioX/`:

- `name` - Device name
- `version` - Driver version
- `maps` - List of memory regions
- `maps/mapN/addr` - Physical address of region N
- `maps/mapN/size` - Size of region N
- `maps/mapN/name` - Name of region N
- `event` - Interrupt event counter
- `irq` - IRQ number

Example:
```bash
$ cat /sys/class/uio/uio0/name
my_device

$ cat /sys/class/uio/uio0/maps
Map0: addr=0xF0000000 size=0x1000 name=mmio
Map1: addr=0xF0001000 size=0x2000 name=mmio

$ cat /sys/class/uio/uio0/irq
42
```

## Memory Regions

### Region Types

1. **MMIO** - Memory-mapped I/O regions
   - Device registers
   - Frame buffers
   - DMA descriptors

2. **Port I/O** - Legacy x86 I/O ports
   - Serial ports
   - Legacy hardware

3. **Custom** - Application-defined regions
   - Shared memory
   - Custom address spaces

### Access Permissions

```rust
// Read-only
let perms = UioMemPermissions::read_only();

// Read-write
let perms = UioMemPermissions::read_write();

// Write-only
let perms = UioMemPermissions::write_only();

// Custom
let perms = UioMemPermissions::new(true, false, false);
```

## Interrupt Handling

### Interrupt Flow

```
Hardware Interrupt
    ↓
Kernel IRQ Handler
    ↓
UioInterrupt::trigger()
    ↓
Eventfd Notification
    ↓
Userspace Application (via read/poll)
    ↓
Application Handles Device
    ↓
Acknowledge Interrupt (write to device)
```

### Interrupt Options

1. **Blocking Read** - Simplest approach
   ```c
   uint32_t count;
   read(fd, &count, sizeof(count));
   ```

2. **Poll/Select** - Event-driven
   ```c
   struct pollfd pfd = { .fd = fd, .events = POLLIN };
   poll(&pfd, 1, -1);
   ```

3. **Eventfd Integration** - Advanced scenarios
   - Integrates with epoll
   - Thread-safe notification
   - Multiple event sources

## Error Handling

All UIO operations return `UioResult<T>`:

```rust
pub enum UioError {
    DeviceNotFound,
    InvalidDevice,
    RegionMapped,
    InvalidRegion,
    InterruptError,
    InvalidOperation,
    PermissionDenied,
    NoMemory,
    DeviceBusy,
    InvalidParam,
}
```

## Security Considerations

1. **Access Control**
   - File permissions on `/dev/uioX`
   - Capability checks for privileged operations
   - Memory region access validation

2. **Memory Safety**
   - Physical address validation
   - Size and alignment checks
   - Permission enforcement

3. **Interrupt Safety**
   - Handler validation
   - Proper cleanup on removal
   - Race-free registration

## Platform Support

### Required Features

- Memory management unit (MMU)
- Interrupt controller
- Device model infrastructure
- Sysfs support

### Platform-Specific Code

The framework isolates platform-specific code:
- Interrupt registration
- Memory mapping implementation
- Device model integration
- Sysfs attribute creation

## Performance Characteristics

### Memory Access
- Direct mapping: Near-native speed
- No context switches for I/O
- Cache-coherent access

### Interrupt Latency
- Eventfd notification: ~1-2 μs
- Poll/epoll integration: Zero-copy
- Minimal kernel overhead

### Scalability
- Supports up to 256 devices
- 8 memory regions per device
- Concurrent access supported

## Testing

Run the test suite:

```bash
# Unit tests
cargo test --lib drivers::uio::tests

# Integration tests
cargo test --lib drivers::uio::tests::integration_tests

# Benchmarks
cargo test --lib drivers::uio::tests::bench_tests -- --nocapture
```

## Best Practices

1. **Device Design**
   - Keep interrupt handlers fast
   - Use proper synchronization
   - Validate all inputs

2. **Userspace Driver**
   - Use memory barriers for MMIO
   - Handle spurious interrupts
   - Implement proper cleanup

3. **Resource Management**
   - Always unmap memory on exit
   - Close file descriptors properly
   - Handle signals gracefully

## Limitations

1. No DMA support (use vfio-pci instead)
2. Limited to simple device types
3. Requires careful interrupt handling
4. Platform-dependent features

## Alternatives

- **UIO** - Simple, minimal overhead
- **VFIO** - Full device assignment, DMA support
- **Kernel Drivers** - Maximum control, complex

## References

- Linux UIO documentation: `Documentation/driver-api/uio-howto.rst`
- UIO kernel source: `drivers/uio/`
- Userspace I/O design patterns

## License

Part of the NOS kernel project.

## Contributing

When adding new features:
1. Update this documentation
2. Add comprehensive tests
3. Ensure backwards compatibility
4. Follow coding standards
5. Update examples as needed
