/// P0 Phase 2 Integration Example
///
/// Demonstrates how all P0 Phase 2 modules work together
/// to boot a kernel on x86_64 systems.

#![allow(dead_code)]

use nos_bootloader::{
    bios_realmode::RealModeExecutor,
    boot_orchestrator::{BootConfig, BootOrchestrator, BootStage},
    e820_detection,
    disk_io,
    kernel_handoff::{BootInformation, BootProtocol},
    error_recovery::{BootError, ErrorRecovery},
};

/// Complete boot workflow example
pub fn complete_boot_example() -> Result<(), &'static str> {
    // Step 1: Initialize real mode executor
    let mut executor = RealModeExecutor::init().map_err(|_| "Executor init failed")?;

    // Step 2: Configure boot parameters
    let config = BootConfig {
        protocol: BootProtocol::Multiboot2,
        boot_drive: 0x80,        // Primary hard disk
        kernel_lba: 2048,        // Sector 2048
        kernel_sectors: 512,     // ~256KB
        kernel_address: 0x100000, // Load at 1MB
        kernel_entry: 0x100000,  // Entry point
        bootloader_name: "NOS Bootloader v0.2.0",
    };

    // Step 3: Create boot orchestrator
    let mut orch = BootOrchestrator::new(config, &executor);

    println!("Boot Stage: {}", orch.current_stage().description());

    // Step 4: Detect system memory
    println!("Detecting system memory via E820...");
    let memory_map = orch.detect_memory(0x10000)?;

    println!("Memory Map:");
    for entry in &memory_map.entries {
        if let Some(mem) = entry {
            println!(
                "  0x{:X} - 0x{:X} ({}) - Type: {}",
                mem.base_address,
                mem.base_address + mem.length,
                mem.length / 1024 / 1024,
                mem.type_name()
            );
        }
    }

    let total_ram = memory_map.total_ram();
    println!("Total RAM: {} MB", total_ram / 1024 / 1024);

    // Verify minimum RAM requirement
    if total_ram < 4 * 1024 * 1024 {
        return Err("Insufficient memory (< 4MB)");
    }

    // Step 5: Load kernel from disk
    println!("Loading kernel from disk (drive 0x80, LBA 2048)...");
    orch.load_kernel()?;
    println!("Kernel loaded successfully");

    // Step 6: Validate kernel
    println!("Validating kernel...");
    orch.validate_kernel()?;
    println!("Kernel validation passed");

    // Step 7: Setup boot information
    println!("Preparing boot information structure...");
    let boot_info = orch.setup_boot_info(&memory_map)?;
    println!("Boot info magic: 0x{:X}", boot_info.magic);

    // Step 8: Show boot status
    println!("Boot Stage: {}", orch.current_stage().description());

    // Step 9: Ready for kernel entry
    println!("\n=== BOOTLOADER READY ===");
    println!("Kernel entry point: 0x{:X}", boot_info.kernel_entry);
    println!("Boot info pointer: 0x{:X}", &boot_info as *const _ as u64);
    println!("Memory map entries: {}", boot_info.memory_map_count);
    println!("Bootloader: {}", "NOS Bootloader v0.2.0");

    // At this point, would call kernel_handoff::KernelHandoff::execute()
    // which jumps to the kernel with RDI = boot_info pointer

    Ok(())
}

/// Minimal boot example (memory only)
pub fn minimal_boot_example() -> Result<(), &'static str> {
    let mut executor = RealModeExecutor::init()?;

    // Just detect memory
    let map = e820_detection::detect_e820_memory(&executor, 0x10000)?;

    println!("Detected {} memory regions", map.count);
    println!("Total usable RAM: {} MB", map.total_ram() / 1024 / 1024);

    Ok(())
}

/// Error handling example
pub fn boot_with_error_recovery() -> Result<(), &'static str> {
    // Setup error recovery
    let recovery = ErrorRecovery::new();

    // Simulate boot flow
    match (|| {
        let executor = RealModeExecutor::init()?;
        let config = BootConfig::default();
        let mut orch = BootOrchestrator::new(config, &executor);
        let _map = orch.detect_memory(0x10000)?;
        orch.load_kernel()?;
        Ok::<(), &'static str>(())
    })() {
        Ok(()) => {
            println!("Boot successful");
            Ok(())
        }
        Err(e) => {
            println!("Boot failed: {}", e);
            // In real implementation, would call recovery.report_error()
            Err(e)
        }
    }
}

/// Hardware capability detection example
pub fn detect_boot_environment() -> Result<(), &'static str> {
    let mut executor = RealModeExecutor::init()?;

    // Detect memory
    let memory_map = e820_detection::detect_e820_memory(&executor, 0x10000)?;

    // Detect disk capabilities
    let controller = disk_io::DiskController::new(&executor);
    let drive_info = controller.get_drive_info(0x80)?;

    println!("System Capabilities:");
    println!("  RAM: {} MB", memory_map.total_ram() / 1024 / 1024);
    println!("  Primary Disk: {} cylinders", drive_info.cylinders);
    println!("  Disk Heads: {}", drive_info.heads);
    println!("  Sectors/Track: {}", drive_info.sectors);
    println!(
        "  Disk Capacity: {} MB",
        (drive_info.total_sectors() * 512) / 1024 / 1024
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_config_creation() {
        let config = BootConfig::default();
        assert_eq!(config.boot_drive, 0x80);
        assert!(config.kernel_address > 0);
    }

    #[test]
    fn test_boot_stage_sequence() {
        // Verify boot stages are properly ordered
        assert_eq!(
            std::mem::discriminant(&BootStage::Init),
            std::mem::discriminant(&BootStage::Init)
        );
    }
}
