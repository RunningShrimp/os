// Boot information handling
//
// This module handles boot information provided by bootloaders,
// supporting both legacy direct boot and modern bootloader interfaces.

extern crate alloc;

// Re-export unified boot parameters from nos-api
pub use nos_api::boot::{
    BootParameters, BootProtocolType, FramebufferInfo, MemoryMap, MemoryMapEntry, MemoryType,
};

// Boot parameter validation
pub mod validator;

// Helper functions for compatibility are now in nos_api crate
// BootParameters::architecture_name() is available from nos_api

// Helper for memory map iteration
pub struct MemoryMapIter {
    current: usize,
    count: usize,
    entries: u64, // Pointer as u64
}

impl MemoryMapIter {
    pub fn new(memory_map: &MemoryMap) -> Self {
        Self {
            current: 0,
            count: memory_map.entry_count as usize,
            entries: memory_map.entries,
        }
    }
}

impl Iterator for MemoryMapIter {
    type Item = &'static MemoryMapEntry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current < self.count {
            let entry = unsafe { &*(self.entries as *const MemoryMapEntry).add(self.current) };
            self.current += 1;
            Some(entry)
        } else {
            None
        }
    }
}

// Helper functions for MemoryMap - create wrapper functions instead of inherent impl
pub fn memory_map_entries(memory_map: &MemoryMap) -> MemoryMapIter {
    MemoryMapIter::new(memory_map)
}

pub fn memory_map_usable_memory(memory_map: &MemoryMap) -> u64 {
    memory_map_entries(memory_map)
        .filter(|entry| entry.is_available != 0 && entry.mem_type == MemoryType::Usable as u32)
        .map(|entry| entry.size)
        .sum()
}

/// Global boot information storage
static mut BOOT_PARAMETERS: Option<BootParameters> = None;
static mut BOOT_INITIALIZED: bool = false;

/// Initialize boot information from bootloader parameters
pub fn init_from_boot_parameters(params: *const BootParameters) {
    use crate::platform::boot::validator::{BootParameterValidator, ValidationSeverity};

    unsafe {
        if !BOOT_INITIALIZED {
            // Perform comprehensive validation
            let validation_result = BootParameterValidator::validate(params);

            // Handle validation results
            if !validation_result.is_valid {
                crate::println!("[boot] Boot parameter validation detected issues:");

                for (error, severity) in validation_result.get_errors() {
                    match severity {
                        ValidationSeverity::Critical => {
                            crate::println!("[boot]   [CRITICAL] {}", error.description());
                        }
                        ValidationSeverity::Error => {
                            crate::println!("[boot]   [ERROR] {}", error.description());
                        }
                        ValidationSeverity::Warning => {
                            crate::println!("[boot]   [WARNING] {}", error.description());
                        }
                    }
                }

                // Handle critical errors
                if !validation_result.is_acceptable() {
                    crate::println!("[boot] CRITICAL: Cannot continue with invalid boot parameters");
                    crate::println!("[boot] Falling back to default boot parameters");
                    BOOT_PARAMETERS = Some(BootParameters::new());
                    BOOT_INITIALIZED = true;
                    return;
                }
            } else {
                crate::println!("[boot] Boot parameter validation: PASSED");
            }

            // Validation passed or acceptable - use the parameters
            if !params.is_null() {
                let params_ref = &*params;

                // Additional version compatibility check
                if !params_ref.is_version_compatible() {
                    crate::println!(
                        "[boot] WARNING: Boot parameters version {} may not be fully compatible \
                         (kernel version {})",
                        params_ref.version,
                        BootParameters::VERSION
                    );
                }

                // Additional architecture verification
                if !params_ref.validate_architecture() {
                    crate::println!(
                        "[boot] WARNING: Architecture mismatch detected - boot params: {}, kernel: {}",
                        params_ref.architecture_name(),
                        {
                            #[cfg(target_arch = "x86_64")]
                            {
                                "x86_64"
                            }
                            #[cfg(target_arch = "aarch64")]
                            {
                                "aarch64"
                            }
                            #[cfg(target_arch = "riscv64")]
                            {
                                "riscv64"
                            }
                            #[cfg(not(any(
                                target_arch = "x86_64",
                                target_arch = "aarch64",
                                target_arch = "riscv64"
                            )))]
                            {
                                "unknown"
                            }
                        }
                    );
                }

                BOOT_PARAMETERS = Some(*params_ref);
            } else {
                BOOT_PARAMETERS = Some(BootParameters::new());
            }

            BOOT_INITIALIZED = true;
        }
    }
}

/// Check if we were booted by a bootloader
pub fn is_bootloader_boot() -> bool {
    unsafe { BOOT_INITIALIZED }
}

/// Get boot parameters if available
pub fn get_boot_parameters() -> Option<&'static BootParameters> {
    unsafe { BOOT_PARAMETERS.as_ref() }
}

/// Get memory map from boot parameters
pub fn get_memory_map() -> Option<&'static MemoryMap> {
    unsafe { BOOT_PARAMETERS.as_ref().map(|params| &params.memory_map) }
}

/// Get framebuffer information
pub fn get_framebuffer_info() -> Option<FramebufferInfo> {
    unsafe {
        BOOT_PARAMETERS.as_ref().and_then(|params| {
            if params.has_framebuffer() {
                Some(params.framebuffer)
            } else {
                None
            }
        })
    }
}

/// Get ACPI RSDP address
pub fn get_acpi_rsdp() -> Option<u64> {
    unsafe {
        BOOT_PARAMETERS.as_ref().and_then(|params| {
            if params.has_acpi() {
                Some(params.acpi_rsdp)
            } else {
                None
            }
        })
    }
}

/// Get device tree blob address
pub fn get_device_tree() -> Option<u64> {
    unsafe {
        BOOT_PARAMETERS.as_ref().and_then(|params| {
            if params.has_device_tree() {
                Some(params.device_tree)
            } else {
                None
            }
        })
    }
}

/// Get command line arguments
pub fn get_command_line() -> Option<&'static str> {
    unsafe {
        BOOT_PARAMETERS.as_ref().and_then(|params| {
            if params.has_command_line() {
                // Convert u64 pointer to &str
                // Safety: caller must ensure pointer is valid
                Some(core::str::from_utf8_unchecked(core::slice::from_raw_parts(
                    params.command_line as *const u8,
                    // Would need to find null terminator or use a length field
                    // For now, this is unsafe and simplified
                    256,
                )))
            } else {
                None
            }
        })
    }
}

/// Get boot timestamp
pub fn get_boot_timestamp() -> Option<u64> {
    unsafe { BOOT_PARAMETERS.as_ref().map(|params| params.timestamp) }
}

/// Get ASLR offset from boot parameters
pub fn get_aslr_offset() -> usize {
    unsafe {
        BOOT_PARAMETERS
            .as_ref()
            .map(|params| params.aslr_offset_usize())
            .unwrap_or(0)
    }
}

/// Check if ASLR is enabled
pub fn is_aslr_enabled() -> bool {
    unsafe {
        BOOT_PARAMETERS
            .as_ref()
            .map(|params| params.has_aslr())
            .unwrap_or(false)
    }
}

/// Initialize boot information for direct QEMU boot (legacy mode)
pub fn init_direct_boot() {
    // Create minimal boot parameters for direct QEMU boot
    let params = BootParameters {
        magic: BootParameters::MAGIC,
        version: 1,
        architecture: {
            #[cfg(target_arch = "x86_64")]
            {
                1
            }
            #[cfg(target_arch = "aarch64")]
            {
                2
            }
            #[cfg(target_arch = "riscv64")]
            {
                3
            }
            #[cfg(not(any(
                target_arch = "x86_64",
                target_arch = "aarch64",
                target_arch = "riscv64"
            )))]
            {
                0
            }
        },
        boot_protocol: BootProtocolType::Direct as u32,
        memory_map: MemoryMap { entry_count: 0, entries: 0 },
        framebuffer: FramebufferInfo {
            address: 0,
            width: 0,
            height: 0,
            bytes_per_pixel: 0,
            stride: 0,
            pixel_format: 0,
        },
        acpi_rsdp: 0,
        device_tree: 0,
        command_line: 0,
        timestamp: 0,
        bootloader_version: 0,
        aslr_offset: 0,
        reserved: [0; 7],
    };

    init_from_boot_parameters(&params as *const BootParameters);
}

/// Print boot information
pub fn print_boot_info() {
    if let Some(params) = get_boot_parameters() {
        crate::println!("[boot] Boot parameters:");
        crate::println!("[boot]   Magic: {:#x}", params.magic);
        crate::println!("[boot]   Version: {}", params.version);
        crate::println!("[boot]   Architecture: {}", params.architecture_name());
        crate::println!("[boot]   Protocol: {:?}", params.protocol_type());

        if let Some(memory_map) = get_memory_map() {
            crate::println!("[boot]   Memory map entries: {}", memory_map.entry_count);
            crate::println!(
                "[boot]   Usable memory: {} MB",
                memory_map_usable_memory(memory_map) / (1024 * 1024)
            );
        }

        if params.has_framebuffer() {
            let fb = params.framebuffer;
            crate::println!(
                "[boot]   Framebuffer: {}x{}x{}",
                fb.width,
                fb.height,
                fb.bytes_per_pixel
            );
        }

        if params.has_acpi() {
            if params.has_acpi() {
                crate::println!("[boot]   ACPI RSDP: {:#x}", params.acpi_rsdp);
            }
        }

        if params.has_device_tree() {
            if params.has_device_tree() {
                crate::println!("[boot]   Device Tree: {:#x}", params.device_tree);
            }
        }

        if params.has_command_line() {
            if params.has_command_line() {
                // Would need to safely read command line string
                crate::println!("[boot]   Command line: (available)");
            }
        }

        if let Some(timestamp) = get_boot_timestamp() {
            crate::println!("[boot]   Boot timestamp: {} ns", timestamp);
        }

        if params.has_aslr() {
            crate::println!("[boot]   ASLR: enabled (offset: {:#x})", params.aslr_offset);
        } else {
            crate::println!("[boot]   ASLR: disabled");
        }
    } else {
        crate::println!("[boot] No boot parameters available (legacy mode)");
    }
}

/// Initialize memory management from boot information
pub fn init_memory_from_boot_info() {
    if let Some(params) = get_boot_parameters() {
        // Use params for validation/logging
        let _boot_params = &params; // Use params for validation

        // Initialize memory management using bootloader-provided memory map
        if let Some(memory_map) = get_memory_map() {
            crate::println!("[boot] Initializing memory from bootloader memory map");

            // Count usable memory regions
            let usable_regions = memory_map_entries(memory_map)
                .filter(|entry| entry.is_available != 0 && entry.mem_type == MemoryType::Usable as u32)
                .count();

            if usable_regions > 0 {
                crate::println!("[boot] Found {} usable memory regions", usable_regions);
                // In a real implementation, we'd pass this information to the memory manager
            } else {
                crate::println!("[boot] Warning: No usable memory regions found");
            }
        }
    }
}

/// Initialize framebuffer from boot information
pub fn init_framebuffer_from_boot_info() {
    if let Some(fb_info) = get_framebuffer_info() {
        crate::println!("[boot] Initializing framebuffer from bootloader");
        crate::println!("[boot]   Address: {:#x}", fb_info.address);
        crate::println!("[boot]   Resolution: {}x{}", fb_info.width, fb_info.height);
        crate::println!(
            "[boot]   Format: {} BPP, stride: {}",
            fb_info.bytes_per_pixel,
            fb_info.stride
        );

        // In a real implementation, we'd initialize the framebuffer driver here
    }
}

/// Initialize ACPI from boot information
pub fn init_acpi_from_boot_info() {
    if let Some(rsdp) = get_acpi_rsdp() {
        crate::println!("[boot] Initializing ACPI from bootloader");
        crate::println!("[boot]   RSDP at: {:#x}", rsdp);

        // In a real implementation, we'd initialize ACPI subsystem here
    }
}

/// Initialize device tree from boot information
pub fn init_device_tree_from_boot_info() {
    if let Some(dtb) = get_device_tree() {
        crate::println!("[boot] Initializing device tree from bootloader");
        crate::println!("[boot]   DTB at: {:#x}", dtb);

        // In a real implementation, we'd parse the device tree here
    }
}
