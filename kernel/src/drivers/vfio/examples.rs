//! VFIO Usage Examples
//!
//! This module provides practical examples for using VFIO in NOS.
//! These examples demonstrate common use cases for high-performance
//! userspace drivers.

use super::*;
use alloc::sync::Arc;

/// Example 1: Basic Device Assignment
///
/// Shows how to assign a PCI device to a userspace driver.
pub fn example_basic_device_assignment() -> VfioResult<()> {
    // 1. Create container
    let container_mgr = ContainerManager::global();
    let container = container_mgr.create_container()?;

    // 2. Set IOMMU type
    container.set_iommu(IommuType::Type1)?;

    // 3. Get device group
    let group_mgr = GroupManager::global();
    let group = group_mgr
        .get_group(0)
        .ok_or(VfioError::GroupNotAvailable)?;

    // 4. Attach group to container
    group.set_container(container.id())?;

    // 5. Get device
    let device = group
        .get_device("0000:01:00.0")
        .ok_or(VfioError::DeviceNotFound)?;

    // 6. Initialize and start device
    device.initialize()?;
    device.start()?;

    // 7. Map device regions for userspace access
    let regions = device.get_regions();
    for region in regions.iter().take(6) {
        // Map first 6 BARs
        if region.size > 0 && region.flags & VFIO_REGION_INFO_FLAG_MMAP != 0 {
            // In real implementation, this would call mmap()
            log::info!("Mapping region {}: size=0x{:x}", region.index, region.size);
        }
    }

    log::info!("Device 0000:01:00.0 assigned successfully");

    Ok(())
}

/// Example 2: High-Performance Networking with DPDK
///
/// Shows how to setup a network device for DPDK.
pub fn example_dpdk_networking() -> VfioResult<()> {
    // Create container with IOMMU
    let container = ContainerManager::global().create_container()?;
    container.set_iommu(IommuType::Type1)?;

    // Get network device group
    let group = GroupManager::global().get_group(0).unwrap();
    group.set_container(container.id())?;

    let device = group.get_device("0000:01:00.0").unwrap();
    device.initialize()?;
    device.start()?;

    // Setup DMA mappings for packet buffers
    let buffer_size = 64 * 1024; // 64KB
    let num_buffers = 4096;

    for i in 0..num_buffers {
        let user_addr = 0x7000000000 + (i * buffer_size) as u64;
        let iova = 0x1000 + (i * buffer_size) as u64;

        container.map_dma(
            iova,
            user_addr,
            buffer_size as u64,
            DMA_MAP_FLAG_READ | DMA_MAP_FLAG_WRITE,
        )?;
    }

    // Setup interrupts for RX/TX queues
    let intr = VfioInterrupt::new(device.info().device_id);

    // Configure MSI-X interrupts
    for queue in 0..8 {
        let eventfd = 100 + queue as i32; // Placeholder
        intr.register_irq(IrqType::Msix, 2, queue, Some(eventfd))?;
    }

    log::info!("DPDK networking device ready");

    Ok(())
}

/// Example 3: GPU Passthrough
///
/// Shows how to pass through a GPU to a VM.
pub fn example_gpu_passthrough() -> VfioResult<()> {
    // Create container for GPU
    let container = ContainerManager::global().create_container()?;
    container.set_iommu(IommuType::Type1)?;

    // Get GPU device
    let group = GroupManager::global().get_group(1).unwrap();
    group.set_container(container.id())?;

    // GPU and its audio function
    let gpu = group.get_device("0000:02:00.0").unwrap();
    let gpu_audio = group.get_device("0000:02:00.1").unwrap();

    gpu.initialize()?;
    gpu_audio.initialize()?;

    // Setup large DMA mappings for framebuffer
    let fb_size = 256 * 1024 * 1024; // 256MB framebuffer
    container.map_dma(
        0,
        0x8000000000,
        fb_size,
        DMA_MAP_FLAG_READ | DMA_MAP_FLAG_WRITE,
    )?;

    log::info!("GPU passthrough ready");

    Ok(())
}

/// Example 4: NVMe Storage Acceleration
///
/// Shows how to use VFIO for direct NVMe device access.
pub fn example_nvme_passthrough() -> VfioResult<()> {
    let container = ContainerManager::global().create_container()?;
    container.set_iommu(IommuType::Type1)?;

    let group = GroupManager::global().get_group(2).unwrap();
    group.set_container(container.id())?;

    let nvme = group.get_device("0000:03:00.0").unwrap();
    nvme.initialize()?;

    // Map NVme registers (BAR0)
    let bar0 = nvme.get_region_info(0).unwrap();
    log::info!("NVMe BAR0: size=0x{:x}", bar0.size);

    // Setup queue pairs
    let num_queues = 64;
    let queue_size = 64 * 1024; // 64KB per queue

    for i in 0..num_queues {
        let queue_addr = 0x9000000000 + (i * queue_size * 2) as u64;
        let iova = 0x10000 + (i * queue_size * 2) as u64;

        // Map submission queue
        container.map_dma(iova, queue_addr, queue_size as u64, DMA_MAP_FLAG_WRITE)?;

        // Map completion queue
        container.map_dma(
            iova + queue_size as u64,
            queue_addr + queue_size as u64,
            queue_size as u64,
            DMA_MAP_FLAG_READ,
        )?;
    }

    log::info!("NVMe device ready with {} queue pairs", num_queues);

    Ok(())
}

/// Example 5: SR-IOV Virtual Functions
///
/// Shows how to use SR-IOV VFs for device sharing.
pub fn example_sr_iov_vfs() -> VfioResult<()> {
    let container1 = ContainerManager::global().create_container()?;
    let container2 = ContainerManager::global().create_container()?;

    container1.set_iommu(IommuType::Type1)?;
    container2.set_iommu(IommuType::Type1)?;

    // Assign VF0 to container1
    let group1 = GroupManager::global().get_group(10).unwrap();
    group1.set_container(container1.id())?;
    let vf0 = group1.get_device("0000:01:10.0").unwrap();
    vf0.initialize()?;

    // Assign VF1 to container2
    let group2 = GroupManager::global().get_group(11).unwrap();
    group2.set_container(container2.id())?;
    let vf1 = group2.get_device("0000:01:10.1").unwrap();
    vf1.initialize()?;

    log::info!("SR-IOV VFs assigned to different containers");

    Ok(())
}

/// Example 6: Security Policy Configuration
///
/// Shows how to configure security policies.
pub fn example_security_policy() -> VfioResult<()> {
    let sandbox = VfioSandbox::new();

    // Configure security policy
    sandbox
        .policy()
        .lock()
        .allow(Permission::Read)
        .allow(Permission::Write)
        .allow(Permission::Dma)
        .allow(Permission::Irq)
        .deny(Permission::Reset)
        .allow_device(0x10EE, 0x1234) // Xilinx device
        .allow_device(0x8086, 0x1572) // Intel NIC
        .block_device(0x1234, 0x5678) // Block test device
        .set_dma_limit(1 << 30) // 1GB limit
        .set_max_devices(4);

    // Grant access to devices
    sandbox.grant_device_access(1, 0x10EE, 0x1234, &sandbox.policy().lock().clone())?;
    sandbox.grant_device_access(2, 0x8086, 0x1572, &sandbox.policy().lock().clone())?;

    // Check permissions
    assert!(sandbox.check_permission(1, Permission::Dma).is_ok());
    assert!(sandbox.check_permission(1, Permission::Reset).is_err());

    log::info!("Security policy configured");

    Ok(())
}

/// Example 7: Batch DMA Operations
///
/// Shows how to use batch DMA mapping for efficiency.
pub fn example_batch_dma() -> VfioResult<()> {
    let dma_map = DmaMap::new();

    // Prepare batch operations
    let ops = &[
        DmaMapOp {
            user_addr: 0x7000000000,
            size: 64 * 1024,
            flags: DMA_MAP_FLAG_READ | DMA_MAP_FLAG_WRITE,
        },
        DmaMapOp {
            user_addr: 0x7000010000,
            size: 64 * 1024,
            flags: DMA_MAP_FLAG_READ | DMA_MAP_FLAG_WRITE,
        },
        DmaMapOp {
            user_addr: 0x7000020000,
            size: 64 * 1024,
            flags: DMA_MAP_FLAG_READ | DMA_MAP_FLAG_WRITE,
        },
        DmaMapOp {
            user_addr: 0x7000030000,
            size: 64 * 1024,
            flags: DMA_MAP_FLAG_READ | DMA_MAP_FLAG_WRITE,
        },
    ];

    // Batch map
    let iovas = dma_map.batch_map(ops)?;
    log::info!("Batch mapped {} DMA regions", iovas.len());

    // Use mappings...

    // Batch unmap
    let unmap_ops = iovas
        .iter()
        .map(|&iova| DmaUnmapOp {
            iova,
            size: 64 * 1024,
        })
        .collect::<Vec<_>>();

    dma_map.batch_unmap(&unmap_ops)?;

    log::info!("Batch unmapped {} DMA regions", unmap_ops.len());

    Ok(())
}

/// Example 8: Interrupt Affinity
///
/// Shows how to set CPU affinity for interrupts.
pub fn example_interrupt_affinity() -> VfioResult<()> {
    let container = ContainerManager::global().create_container()?;
    container.set_iommu(IommuType::Type1)?;

    let group = GroupManager::global().get_group(0).unwrap();
    group.set_container(container.id())?;

    let device = group.get_device("0000:01:00.0").unwrap();
    device.initialize()?;

    // Setup interrupts with CPU affinity
    let intr = VfioInterrupt::new(device.info().device_id);

    // Pin different queues to different CPUs
    for queue in 0..8 {
        let eventfd = 200 + queue as i32;
        let handle = intr.register_irq(IrqType::Msix, 2, queue, Some(eventfd))?;

        // Set affinity to specific CPU
        let cpu = queue % 4;
        let affinity = CpuAffinity::single_cpu(cpu as u8);
        intr.set_affinity(handle, affinity)?;

        log::info!("Queue {} affinity: CPU {}", queue, cpu);
    }

    log::info!("Interrupt affinity configured");

    Ok(())
}

/// Example 9: Live Migration Preparation
///
/// Shows how to prepare device state for live migration.
pub fn example_live_migration() -> VfioResult<()> {
    let container = ContainerManager::global().create_container()?;
    container.set_iommu(IommuType::Type1)?;

    let group = GroupManager::global().get_group(0).unwrap();
    group.set_container(container.id())?;

    let device = group.get_device("0000:01:00.0").unwrap();
    device.initialize()?;

    // Save device state
    let state = VfioDeviceState {
        device_id: device.info().device_id,
        regions: device.get_regions(),
        irqs: device.get_irqs(),
        dma_mappings: container.iommu()?.get_mappings(),
    };

    log::info!("Device state saved for migration");

    // On target system:
    // 1. Create container with same IOMMU type
    // 2. Attach device
    // 3. Restore DMA mappings
    // 4. Restore interrupt configuration
    // 5. Resume device

    Ok(())
}

/// Device state for migration
struct VfioDeviceState {
    device_id: u16,
    regions: alloc::vec::Vec<VfioRegionInfo>,
    irqs: alloc::vec::Vec<IrqInfo>,
    dma_mappings: alloc::vec::Vec<crate::drivers::vfio::iommu::DmaMapping>,
}

/// Example 10: Performance Monitoring
///
/// Shows how to monitor VFIO performance.
pub fn example_performance_monitoring() -> VfioResult<()> {
    // Get global statistics
    let stats = get_stats();

    log::info!("VFIO Statistics:");
    log::info!("  Active containers: {}", stats.active_containers);
    log::info!("  Active devices: {}", stats.active_devices);
    log::info!("  Total DMA mappings: {}", stats.total_dma_mappings);
    log::info!("  Total DMA size: {} bytes", stats.total_dma_size);
    log::info!("  Active interrupts: {}", stats.active_interrupts);

    // Get per-container statistics
    let container_mgr = ContainerManager::global();
    if let Some(container) = container_mgr.get_container(1) {
        let container_stats = container.get_stats();
        log::info!("Container 1:");
        log::info!("  Groups: {}", container_stats.groups_count);
        log::info!("  DMA mappings: {}", container_stats.dma_mappings);
        log::info!("  Total DMA: {} bytes", container_stats.total_dma_size);
    }

    // Get IOMMU statistics
    let container = container_mgr.create_container()?;
    container.set_iommu(IommuType::Type1)?;
    let iommu = container.iommu()?;
    let iommu_stats = iommu.get_stats();

    log::info!("IOMMU {}:", iommu.id());
    log::info!("  Type: {:?}", iommu_stats.iommu_type);
    log::info!("  Attached devices: {}", iommu_stats.attached_devices);
    log::info!("  Active mappings: {}", iommu_stats.active_mappings);
    log::info!("  Mapped pages: {}", iommu_stats.mapped_pages);

    Ok(())
}
