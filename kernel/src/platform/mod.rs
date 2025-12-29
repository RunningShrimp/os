pub mod arch;
pub mod boot;
pub mod device;
pub mod drivers;
pub mod trap;
pub mod mmio;

// Re-export device types for convenience
pub use device::{DeviceResources, DeviceStatus, DeviceType, IoPortRange, MemoryRegion};

// Re-export MMIO functions
pub use mmio::{
    mmio_read8, mmio_read16, mmio_read32, mmio_read64,
    mmio_write8, mmio_write16, mmio_write32, mmio_write64,
};

use nos_api::Result;

use crate::BootParameters;

/// Initialize platform subsystems
pub fn init_platform(boot_params: &BootParameters) -> Result<()> {
    // Store boot parameters
    boot::init_from_boot_parameters(boot_params);

    // Early architecture initialization
    arch::early_init();

    // Print boot information
    boot::print_boot_info();

    // Initialize memory from boot info
    boot::init_memory_from_boot_info();

    // Initialize framebuffer
    boot::init_framebuffer_from_boot_info();

    // Initialize ACPI
    boot::init_acpi_from_boot_info();

    // Initialize device tree
    boot::init_device_tree_from_boot_info();

    // Initialize device manager
    drivers::device_manager::init()?;

    // Initialize devices
    drivers::init();

    // Initialize trap handling
    trap::init();

    Ok(())
}

/// Shutdown platform subsystems
pub fn shutdown_platform() -> Result<()> {
    // Platform-specific shutdown procedures would be implemented here
    // For now, we'll just return Ok(()) as a placeholder
    Ok(())
}
