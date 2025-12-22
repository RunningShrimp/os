pub mod arch;
pub mod drivers;
pub mod boot;
pub mod trap;

use crate::BootParameters;
use nos_api::Result;

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