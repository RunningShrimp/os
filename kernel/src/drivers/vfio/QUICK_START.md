# VFIO Quick Start Guide

## Basic Usage

### 1. Create Container and Assign Device

```rust
use kernel::drivers::vfio::*;

// Create container
let container = ContainerManager::global().create_container()?;
container.set_iommu(IommuType::Type1)?;

// Get and attach group
let group = GroupManager::global().get_group(0)?;
group.set_container(container.id())?;

// Get device
let device = group.get_device("0000:01:00.0")?;
device.initialize()?;
device.start()?;

// Setup DMA
container.map_dma(0x1000, 0x7f0000000000, 0x1000,
    DMA_MAP_FLAG_READ | DMA_MAP_FLAG_WRITE)?;

// Setup interrupts
let intr = VfioInterrupt::new(device.info().device_id);
intr.register_irq(IrqType::Msix, 2, 0, Some(eventfd))?;
```

## Common Patterns

### DPDK High-Performance Networking
```rust
// See examples.rs: example_dpdk_networking()
// - Zero-copy packet buffers
// - MSI-X interrupts per queue
// - Batch DMA operations
```

### GPU Passthrough
```rust
// See examples.rs: example_gpu_passthrough()
// - GPU + audio function assignment
// - Large framebuffer DMA
// - VGA arbitration
```

### NVMe Storage
```rust
// See examples.rs: example_nvme_passthrough()
// - Direct queue pair access
// - Submission/completion queue DMA
// - High IOPS with minimal latency
```

## API Reference

### Container
- `ContainerManager::global()` - Get global manager
- `create_container()` - Create new container
- `set_iommu(type)` - Set IOMMU type
- `map_dma(iova, addr, size, flags)` - Map DMA
- `unmap_dma(iova, size)` - Unmap DMA

### Device
- `initialize()` - Initialize device
- `start()` - Start device
- `get_region_info(index)` - Get region info
- `get_irq_info(index)` - Get IRQ info

### Interrupt
- `register_irq(type, index, vector, eventfd)` - Register IRQ
- `enable_irq(handle)` - Enable interrupt
- `set_affinity(handle, affinity)` - Set CPU affinity

### Security
- `grant_device_access(id, vendor, device, policy)` - Grant access
- `check_permission(id, permission)` - Check permission
- `check_dma(id, size)` - Check DMA limits

## Constants

```rust
// DMA flags
DMA_MAP_FLAG_READ = 0x1
DMA_MAP_FLAG_WRITE = 0x2

// Device flags
VFIO_DEVICE_FLAGS_PCI = 0x1
VFIO_DEVICE_FLAGS_RESET = 0x8

// Region flags
VFIO_REGION_INFO_FLAG_READ = 0x1
VFIO_REGION_INFO_FLAG_WRITE = 0x2
VFIO_REGION_INFO_FLAG_MMAP = 0x4
```

## Error Handling

All operations return `VfioResult<T>`:

```rust
use kernel::drivers::vfio::{VfioError, VfioResult};

fn my_vfio_operation() -> VfioResult<()> {
    let container = ContainerManager::global()
        .create_container()?;  // Returns error if failed

    // ... do work ...

    Ok(())
}
```

## Testing

Run tests:
```bash
cargo test --package kernel --lib drivers::vfio
```

## Performance Tips

1. **Use batch DMA operations** for multiple mappings
2. **Set CPU affinity** for interrupts to optimize cache locality
3. **Pin to memory** to prevent swapping during DMA
4. **Use scatter-gather** for non-contiguous buffers
5. **Enable MSI-X** for better interrupt scalability

## Security Best Practices

1. **Always set IOMMU** - Don't bypass IOMMU protection
2. **Use allowlist** - Only allow known devices
3. **Set DMA limits** - Prevent excessive DMA mappings
4. **Check permissions** - Enforce least privilege
5. **Monitor violations** - Track security violations

## Troubleshooting

### Device Not Found
```
Error: DeviceNotFound
→ Check device BDF: "0000:01:00.0"
→ Verify device is in correct group
```

### IOMMU Errors
```
Error: IommuError
→ Check IOMMU is enabled in hardware
→ Verify IOMMU type is supported
→ Check device attachment
```

### DMA Failures
```
Error: DmaError
→ Verify addresses are page-aligned (4KB)
→ Check DMA size limits
→ Ensure IOMMU is configured
```

### Permission Denied
```
Error: PermissionDenied
→ Check security policy
→ Verify device is allowed
→ Grant required permissions
```

## More Information

- Full documentation: `README.md`
- Usage examples: `examples.rs`
- Test cases: `tests.rs`
- Implementation summary: `VFIO_IMPLEMENTATION_SUMMARY.md`
