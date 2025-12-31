//! VFIO Integration Tests
//!
//! Comprehensive tests for VFIO implementation covering:
//! - Container lifecycle
//! - IOMMU operations
//! - Device management
//! - DMA operations
//! - Interrupt handling
//! - Security policies

use super::*;
use alloc::sync::Arc;

/// Test container creation and destruction
#[test]
fn test_container_lifecycle() {
    let mgr = ContainerManager::global();

    // Create container
    let container = mgr.create_container().unwrap();
    let id = container.id();

    // Verify state
    assert_eq!(container.state(), ContainerState::Created);

    // Set IOMMU
    container.set_iommu(IommuType::Type1).unwrap();
    assert_eq!(container.state(), ContainerState::Initialized);

    // Destroy container
    mgr.destroy_container(id).unwrap();

    // Verify destroyed
    assert!(mgr.get_container(id).is_none());
}

/// Test IOMMU domain operations
#[test]
fn test_iommu_domain() {
    let domain = IommuDomain::new(IommuType::Type1).unwrap();

    // Test basic properties
    assert_eq!(domain.iommu_type(), IommuType::Type1);
    assert_eq!(domain.page_size(), 4096);
    assert!(!domain.is_device_attached(0, 0, 1, 0));

    // Test device attach/detach
    domain.attach_device(0, 0, 1, 0).unwrap();
    assert!(domain.is_device_attached(0, 0, 1, 0));

    domain.detach_device(0, 0, 1, 0).unwrap();
    assert!(!domain.is_device_attached(0, 0, 1, 0));

    // Test DMA map/unmap
    let iova = 0x1000;
    let user_addr = 0x7f0000000000;
    let size = 0x1000;

    domain.map(iova, user_addr, size, 0x3).unwrap();
    assert!(domain.get_mapping(iova).is_some());

    domain.unmap(iova, size).unwrap();
    assert!(domain.get_mapping(iova).is_none());

    // Test statistics
    let stats = domain.get_stats();
    assert_eq!(stats.iommu_type, IommuType::Type1);
    assert_eq!(stats.attached_devices, 0);
}

/// Test device lifecycle
#[test]
fn test_device_lifecycle() {
    let info = VfioDeviceInfo {
        name: alloc::string::String::from("test_device"),
        device_id: 0x1234,
        vendor_id: 0x5678,
        segment: 0,
        bus: 0,
        device: 1,
        function: 0,
        num_regions: 8,
        num_irqs: 3,
        flags: VFIO_DEVICE_FLAGS_PCI,
        is_vf: false,
        pf_device: None,
    };

    let device = VfioDevice::new(info).unwrap();

    // Check initial state
    assert_eq!(device.state(), DeviceState::Created);
    assert_eq!(device.ref_count(), 1);

    // Initialize device
    device.initialize().unwrap();
    assert_eq!(device.state(), DeviceState::Initialized);

    // Start device
    device.start().unwrap();
    assert_eq!(device.state(), DeviceState::Running);

    // Stop device
    device.stop().unwrap();
    assert_eq!(device.state(), DeviceState::Stopped);

    // Test reference counting
    device.ref_count_inc();
    assert_eq!(device.ref_count(), 2);
    assert_eq!(device.ref_count_dec(), 1);

    // Test regions
    let regions = device.get_regions();
    assert!(!regions.is_empty());
}

/// Test group operations
#[test]
fn test_group_operations() {
    let mgr = GroupManager::global();
    let group = mgr.create_group();

    // Check group properties
    assert_eq!(group.get_status(), GroupStatus::Viable);
    assert!(group.is_viable());

    // Set container
    group.set_container(123).unwrap();
    assert_eq!(group.get_container_id(), Some(123));

    // Try setting again (should fail)
    assert!(group.set_container(456).is_err());

    // Unset container
    group.unset_container().unwrap();
    assert_eq!(group.get_container_id(), None);

    // Test reference counting
    group.ref_count_inc();
    assert_eq!(group.ref_count(), 1);
    assert_eq!(group.ref_count_dec(), 0);
}

/// Test DMA mapping operations
#[test]
fn test_dma_mapping() {
    let dma_map = DmaMap::new();

    // Test basic map/unmap
    let user_addr = 0x7f0000000000;
    let size = 0x1000;
    let flags = DMA_MAP_FLAG_READ | DMA_MAP_FLAG_WRITE;

    let iova = dma_map.map_user_pages(user_addr, size, flags).unwrap();

    let mapping = dma_map.get_mapping(iova).unwrap();
    assert_eq!(mapping.iova, iova);
    assert_eq!(mapping.user_addr, user_addr);
    assert_eq!(mapping.size, size);
    assert!(mapping.is_readable());
    assert!(mapping.is_writable());

    dma_map.unmap_user_pages(iova, size).unwrap();
    assert!(dma_map.get_mapping(iova).is_none());

    // Test scatter-gather
    let iova2 = dma_map.map_user_pages(user_addr, 0x3000, flags).unwrap();
    let sglist = dma_map.create_sglist(iova2, 0x3000).unwrap();
    assert_eq!(sglist.len(), 3);
    assert_eq!(sglist.total_size(), 0x3000);

    dma_map.unmap_user_pages(iova2, 0x3000).unwrap();
}

/// Test batch DMA operations
#[test]
fn test_batch_dma() {
    let dma_map = DmaMap::new();

    let ops = &[
        DmaMapOp {
            user_addr: 0x7f0000000000,
            size: 0x1000,
            flags: 0x3,
        },
        DmaMapOp {
            user_addr: 0x7f0000001000,
            size: 0x1000,
            flags: 0x3,
        },
        DmaMapOp {
            user_addr: 0x7f0000002000,
            size: 0x1000,
            flags: 0x3,
        },
    ];

    // Batch map
    let iovas = dma_map.batch_map(ops).unwrap();
    assert_eq!(iovas.len(), 3);

    // Batch unmap
    let unmap_ops = iovas
        .iter()
        .map(|&iova| DmaUnmapOp { iova, size: 0x1000 })
        .collect::<Vec<_>>();

    dma_map.batch_unmap(&unmap_ops).unwrap();
}

/// Test interrupt management
#[test]
fn test_interrupt_management() {
    let intr = VfioInterrupt::new(0);

    // Test IRQ registration
    let handle = intr
        .register_irq(IrqType::Msix, 2, 0, Some(42))
        .unwrap();
    assert!(handle > 0);

    // Test enable/disable
    intr.disable_irq(handle).unwrap();
    intr.enable_irq(handle).unwrap();

    // Test mask/unmask (INTx is maskable)
    let handle2 = intr
        .register_irq(IrqType::Intx, 0, 0, Some(43))
        .unwrap();

    intr.mask_irq(handle2).unwrap();
    intr.unmask_irq(handle2).unwrap();

    // Test CPU affinity
    let affinity = CpuAffinity::single_cpu(0);
    intr.set_affinity(handle, affinity).unwrap();

    let retrieved = intr.get_affinity(handle).unwrap();
    assert_eq!(retrieved.mask, affinity.mask);

    // Cleanup
    intr.unregister_irq(handle).unwrap();
    intr.unregister_irq(handle2).unwrap();
}

/// Test security policies
#[test]
fn test_security_policies() {
    let sandbox = VfioSandbox::new();

    // Set up policy
    sandbox
        .policy()
        .lock()
        .allow(Permission::Read)
        .allow(Permission::Write)
        .allow(Permission::Dma)
        .allow_device(0x1234, 0x5678)
        .set_dma_limit(0x10000)
        .set_max_devices(10);

    // Grant device access
    sandbox
        .grant_device_access(1, 0x1234, 0x5678, &SecurityPolicy::new())
        .unwrap();

    // Test allowed permission
    assert!(sandbox.check_permission(1, Permission::Read).is_ok());
    assert!(sandbox.check_permission(1, Permission::Dma).is_ok());

    // Test denied permission
    assert!(sandbox.check_permission(1, Permission::Reset).is_err());

    // Test DMA limits
    assert!(sandbox.check_dma(1, 0x1000).is_ok());
    assert!(sandbox.check_dma(1, 0x20000).is_err());

    // Test violations
    let _ = sandbox.check_permission(1, Permission::Reset);
    let violations = sandbox.get_violations();
    assert!(!violations.is_empty());

    sandbox.clear_violations();
    assert!(sandbox.get_violations().is_empty());
}

/// Test IOMMU type conversions
#[test]
fn test_iommu_type_conversion() {
    assert_eq!(IommuType::Type1.as_u32(), 1);
    assert_eq!(IommuType::Type1v2.as_u32(), 2);
    assert_eq!(IommuType::SPAPR.as_u32(), 3);

    assert_eq!(IommuType::from_u32(1), Some(IommuType::Type1));
    assert_eq!(IommuType::from_u32(2), Some(IommuType::Type1v2));
    assert_eq!(IommuType::from_u32(3), Some(IommuType::SPAPR));
    assert_eq!(IommuType::from_u32(999), None);
}

/// Test IRQ type conversions
#[test]
fn test_irq_type_conversion() {
    assert_eq!(IrqType::Intx.to_index(), 0);
    assert_eq!(IrqType::Msi.to_index(), 1);
    assert_eq!(IrqType::Msix.to_index(), 2);
    assert_eq!(IrqType::Eventfd.to_index(), 3);

    assert_eq!(IrqType::from_index(0), Some(IrqType::Intx));
    assert_eq!(IrqType::from_index(1), Some(IrqType::Msi));
    assert_eq!(IrqType::from_index(2), Some(IrqType::Msix));
    assert_eq!(IrqType::from_index(3), Some(IrqType::Eventfd));
    assert_eq!(IrqType::from_index(99), None);
}

/// Test PCI device ID
#[test]
fn test_pci_device_id() {
    let id = PciDeviceId::new(0, 1, 0, 0);
    assert_eq!(id.to_string(), "0000:01:00.0");

    let id2 = PciDeviceId::from_str("0000:01:00.0").unwrap();
    assert_eq!(id, id2);

    let id3 = PciDeviceId::from_str("0000:02:01.3").unwrap();
    assert_eq!(id3.segment, 0);
    assert_eq!(id3.bus, 2);
    assert_eq!(id3.device, 1);
    assert_eq!(id3.function, 3);
}

/// Test PCI device
#[test]
fn test_pci_device() {
    let id = PciDeviceId::new(0, 0, 1, 0);
    let mut device = PciDevice::new(id, 0x1234, 0x5678, 0x020000);

    assert_eq!(device.vendor_id(), 0x1234);
    assert_eq!(device.device_id(), 0x5678);
    assert!(!device.is_vf());
    assert!(device.pf_id().is_none());

    device.set_vf(true);
    assert!(device.is_vf());

    let pf_id = PciDeviceId::new(0, 0, 0, 0);
    device.set_pf_id(pf_id);
    assert_eq!(device.pf_id(), Some(pf_id));

    device.enable().unwrap();
    assert!(device.is_enabled());

    device.disable().unwrap();
    assert!(!device.is_enabled());
}

/// Test VGA arbiter
#[test]
fn test_vga_arbiter() {
    let arbiter = VgaArbiter::new();

    let device1 = PciDeviceId::new(0, 0, 1, 0);
    let device2 = PciDeviceId::new(0, 0, 2, 0);

    arbiter.register_device(device1).unwrap();

    // First acquire should succeed
    assert_eq!(arbiter.try_acquire(device1).unwrap(), true);
    assert_eq!(arbiter.get_owner(), Some(device1));

    // Second acquire should fail
    assert_eq!(arbiter.try_acquire(device2).unwrap(), false);

    // Release
    arbiter.release(device1).unwrap();
    assert_eq!(arbiter.get_owner(), None);
}

/// Test SR-IOV VF
#[test]
fn test_sr_iov_vf() {
    let pf_id = PciDeviceId::new(0, 0, 0, 0);
    let pf = Arc::new(PciDevice::new(pf_id, 0x1234, 0x5678, 0x020000));

    let vf = SrioVf::new(pf.clone(), 0);
    assert_eq!(vf.index(), 0);
    assert_eq!(vf.pf_device().id(), pf_id);

    let vf_id = vf.id();
    assert_eq!(vf_id.function, 1); // PF function (0) + 1

    let mut vf = vf;
    vf.enable().unwrap();
    assert!(vf.is_enabled());

    vf.disable().unwrap();
    assert!(!vf.is_enabled());
}

/// Test container DMA operations
#[test]
fn test_container_dma() {
    let mgr = ContainerManager::global();
    let container = mgr.create_container().unwrap();

    // Set IOMMU
    container.set_iommu(IommuType::Type1).unwrap();

    // Test DMA map/unmap
    let iova = 0x1000;
    let user_addr = 0x7f0000000000;
    let size = 0x1000;

    container
        .map_dma(iova, user_addr, size, DMA_MAP_FLAG_READ | DMA_MAP_FLAG_WRITE)
        .unwrap();

    container.unmap_dma(iova, size).unwrap();

    // Cleanup
    mgr.destroy_container(container.id()).ok();
}

/// Test manager statistics
#[test]
fn test_manager_statistics() {
    let container_mgr = ContainerManager::global();

    let container1 = container_mgr.create_container().unwrap();
    let _container2 = container_mgr.create_container().unwrap();

    let stats = container_mgr.get_stats();
    assert!(stats.active_containers >= 2);

    // Cleanup
    container_mgr.destroy_container(container1.id()).ok();
}

/// Test CPU affinity
#[test]
fn test_cpu_affinity() {
    let single = CpuAffinity::single_cpu(3);
    assert_eq!(single.mask, 1 << 3);

    let all = CpuAffinity::all_cpus();
    assert_eq!(all.mask, !0u64);

    let custom = CpuAffinity::new(0xF0);
    assert_eq!(custom.mask, 0xF0);
}

/// Test permission flags
#[test]
fn test_permission_flags() {
    assert_eq!(Permission::Read.as_flag(), 0x1);
    assert_eq!(Permission::Write.as_flag(), 0x2);
    assert_eq!(Permission::Mmap.as_flag(), 0x4);
    assert_eq!(Permission::Dma.as_flag(), 0x8);

    assert_eq!(Permission::from_flag(0x1), Some(Permission::Read));
    assert_eq!(Permission::from_flag(0x99), None);
}

/// Test IRQ info
#[test]
fn test_irq_info() {
    let intr = VfioInterrupt::new(0);

    let info = intr.get_irq_info(0).unwrap();
    assert_eq!(info.index, 0);
    assert_eq!(info.irq_type, IrqType::Intx);

    let infos = intr.get_all_irqs();
    assert!(!infos.is_empty());
}

/// Test device region operations
#[test]
fn test_device_regions() {
    let info = VfioDeviceInfo {
        name: alloc::string::String::from("test"),
        device_id: 0x1234,
        vendor_id: 0x5678,
        segment: 0,
        bus: 0,
        device: 1,
        function: 0,
        num_regions: 8,
        num_irqs: 3,
        flags: VFIO_DEVICE_FLAGS_PCI,
        is_vf: false,
        pf_device: None,
    };

    let device = VfioDevice::new(info).unwrap();
    device.initialize().unwrap();

    // Test region operations
    let regions = device.get_regions();
    assert!(!regions.is_empty());

    for region in regions {
        if region.size > 0 {
            // Test region read/write
            let mut data = [0u8; 8];
            let _ = device.region_read(region.index, 0, &mut data);
            let _ = device.region_write(region.index, 0, &data);
        }
    }
}

/// Test error handling
#[test]
fn test_error_handling() {
    // Test invalid operations
    let mgr = ContainerManager::global();

    // Destroy non-existent container
    assert!(mgr.destroy_container(9999).is_err());

    // Get non-existent container
    assert!(mgr.get_container(9999).is_none());

    let mgr = GroupManager::global();
    assert!(mgr.get_group(9999).is_none());
}
