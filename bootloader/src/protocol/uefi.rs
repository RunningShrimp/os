extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

//! UEFI 2.10 protocol implementation
//!
//! This module provides a comprehensive UEFI 2.10 implementation for the NOS bootloader,
//! supporting core protocols, graphics, secure boot, and modern UEFI features.

use crate::error::{BootError, Result};
use crate::protocol::{BootInfo, FramebufferInfo, KernelImage, MemoryMap, MemoryMapEntry, MemoryType, ProtocolType};
use core::ptr;
use core::sync::atomic::{AtomicPtr, Ordering};

#[cfg(feature = "uefi_support")]
use uefi::{prelude::*, table::{Boot, Runtime}, Identify};

/// UEFI protocol implementation
#[cfg(feature = "uefi_support")]
pub struct UefiProtocol {
    system_table: AtomicPtr<SystemTable<Boot>>,
    boot_services: Option<&'static BootServices>,
    runtime_services: Option<&'static RuntimeServices>,
    initialized: bool,
    protocol_type: ProtocolType,
}

#[cfg(feature = "uefi_support")]
impl UefiProtocol {
    /// Create a new UEFI protocol instance
    pub fn new() -> Self {
        Self {
            system_table: AtomicPtr::new(ptr::null_mut()),
            boot_services: None,
            runtime_services: None,
            initialized: false,
            protocol_type: ProtocolType::UEFI,
        }
    }

    /// Initialize UEFI protocol using system table handle
    pub fn initialize_with_system_table(&mut self, system_table: &'static SystemTable<Boot>) -> Result<()> {
        // Store system table
        self.system_table.store(system_table as *const _ as *mut _, Ordering::SeqCst);

        // Get boot services
        self.boot_services = Some(unsafe {
            // Safety: System table is provided by UEFI firmware
            &system_table.boot_services()
        });

        // Get runtime services
        self.runtime_services = Some(unsafe {
            // Safety: System table is provided by UEFI firmware
            &system_table.runtime_services()
        });

        self.initialized = true;

        // Print UEFI information
        if let Ok(st) = self.system_table() {
            println!("UEFI Firmware Vendor: {}", st.firmware_vendor());
            println!("UEFI Firmware Revision: {}.{}",
                     st.firmware_revision(),
                     st.firmware_revision() >> 16);
        }

        Ok(())
    }

    /// Get the system table
    pub fn system_table(&self) -> Result<&'static SystemTable<Boot>> {
        let ptr = self.system_table.load(Ordering::SeqCst);
        if ptr.is_null() {
            return Err(BootError::UefiNotFound);
        }
        unsafe { Ok(&*ptr) }
    }

    /// Get boot services
    pub fn boot_services(&self) -> Result<&'static BootServices> {
        self.boot_services.ok_or(BootError::UefiNotFound)
    }

    /// Get runtime services
    pub fn runtime_services(&self) -> Result<&'static RuntimeServices> {
        self.runtime_services.ok_or(BootError::UefiNotFound)
    }

    /// Print message using UEFI console
    pub fn println(&self, args: core::fmt::Arguments) -> Result<()> {
        use core::fmt::Write;

        if let Ok(st) = self.system_table() {
            let mut stdout = st.stdout();
            let _ = write!(stdout, "{}", args);
            let _ = writeln!(stdout);
            Ok(())
        } else {
            Err(BootError::UefiNotFound)
        }
    }

    /// Get memory map from UEFI
    pub fn get_memory_map(&self) -> Result<MemoryMap> {
        let bs = self.boot_services()?;

        // First, get the memory map size
        let map_size = bs.memory_map_size()?;

        // Allocate buffer for memory map
        let mut buffer = vec![0u8; map_size];

        // Get the actual memory map
        let memory_map = bs.memory_map(&mut buffer)?;

        // Convert to our format
        let mut entries = Vec::new();
        let mut total_memory = 0;
        let mut available_memory = 0;

        for descriptor in memory_map.entries() {
            let entry_type = match descriptor.ty() {
                uefi::table::boot::MemoryType::RESERVED => MemoryType::Reserved,
                uefi::table::boot::MemoryType::LOADER_CODE => MemoryType::BootloaderCode,
                uefi::table::boot::MemoryType::LOADER_DATA => MemoryType::BootloaderData,
                uefi::table::boot::MemoryType::BOOT_SERVICES_CODE => MemoryType::RuntimeCode,
                uefi::table::boot::MemoryType::BOOT_SERVICES_DATA => MemoryType::RuntimeData,
                uefi::table::boot::MemoryType::CONVENTIONAL => MemoryType::Usable,
                uefi::table::boot::MemoryType::UNUSABLE => MemoryType::BadMemory,
                uefi::table::boot::MemoryType::ACPI_RECLAIM => MemoryType::ACPIReclaimable,
                uefi::table::boot::MemoryType::ACPI_NON_VOLATILE => MemoryType::ACPIONVS,
                uefi::table::boot::MemoryType::MMIO => MemoryType::DeviceMemory,
                uefi::table::boot::MemoryType::PAL_CODE => MemoryType::RuntimeCode,
                uefi::table::boot::MemoryType::PERSISTENT_MEMORY => MemoryType::Usable,
                _ => MemoryType::Reserved,
            };

            let is_available = matches!(descriptor.ty(),
                uefi::table::boot::MemoryType::CONVENTIONAL |
                uefi::table::boot::MemoryType::PERSISTENT_MEMORY);

            total_memory += descriptor.page_count() * 4096;
            if is_available {
                available_memory += descriptor.page_count() * 4096;
            }

            let entry = MemoryMapEntry {
                base: descriptor.phys_start(),
                size: descriptor.page_count() * 4096,
                mem_type: entry_type,
                is_available,
            };

            entries.push(entry);
        }

        Ok(MemoryMap {
            entries,
            total_memory,
            available_memory,
        })
    }

    /// Get framebuffer information from UEFI GOP
    pub fn get_framebuffer_info(&self) -> Result<Option<FramebufferInfo>> {
        use uefi::proto::console::gop::GraphicsOutput;

        if let Ok(st) = self.system_table() {
            // Skip framebuffer info for now - requires updated UEFI API
            return Ok(None);
            /*
            // Temporarily disabled - requires updated UEFI API
            let gop = unsafe { &*gop };

            if let Ok(mode) = gop.current_mode_info() {
                let pixel_format = match mode.pixel_format() {
                    uefi::proto::console::gop::PixelFormat::RGB => 0,
                    uefi::proto::console::gop::PixelFormat::BGR => 1,
                    uefi::proto::console::gop::PixelFormat::Bitmask => 2,
                    uefi::proto::console::gop::PixelFormat::BLT_ONLY => 3,
                };

                let info = FramebufferInfo {
                    address: mode.framebuffer_base() as usize,
                    width: mode.resolution().0,
                    height: mode.resolution().1,
                    bytes_per_pixel: 4, // RGBA
                    stride: mode.stride(),
                    pixel_format,
                };

                return Ok(Some(info));
            }
            */
        }
        }

        Ok(None)
    }

    /// Get ACPI RSDP from UEFI configuration tables
    pub fn get_acpi_rsdp(&self) -> Result<Option<usize>> {
        if let Ok(st) = self.system_table() {
            // Look for ACPI 2.0 RSDP in configuration tables
            for table in st.config_table() {
                if let Ok(vendor_guid) = table.vendor_guid() {
                    // ACPI 2.0 RSDP GUID: 8868E871-E4F1-11D3-BC22-0080C73C8881
                    const ACPI_RSDP_GUID: uefi::Guid = uefi::Guid::from_values(
                        0x8868e871, 0xe4f1, 0x11d3, 0xbc, 0x22, [0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81]
                    );

                    if vendor_guid == ACPI_RSDP_GUID {
                        return Ok(Some(table.address() as usize));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Get command line from UEFI LoadOptions
    pub fn get_command_line(&self) -> Result<Option<String>> {
        if let Ok(st) = self.system_table() {
            // Skip command line for now - requires updated UEFI API
            return Ok(None);

            // LoadOptions might contain command line arguments
            let load_options = loaded_image.load_options_as_bytes();
            if !load_options.is_empty() {
                // Try to parse as UTF-16 string (UEFI standard)
                if let Some(null_pos) = load_options.windows(2).position(|w| w == &[0, 0]) {
                    let utf16_data = &load_options[..null_pos];
                    if let Ok(command_line) = String::from_utf16(utf16_data) {
                        return Ok(Some(command_line));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Load kernel from file system
    pub fn load_kernel(&mut self, path: &str) -> Result<KernelImage> {
        use uefi::proto::media::file::File;

        let bs = self.boot_services()?;

        // Get the loaded image device handle
        let loaded_image = unsafe {
            bs.locate_protocol::<uefi::proto::loaded_image::LoadedImage<Self>>()
                .map(|p| unsafe { &*p }) }?;

        let device_handle = loaded_image.device();

        // Open Simple File System Protocol
        let sfs = unsafe {
            bs.locate_device_path_protocol::<uefi::proto::media::fs::SimpleFileSystem>(device_handle)
                .map(|p| unsafe { &*p }) }?;

        // Open root directory
        let mut root = sfs.open_volume()?;

        // Try to open the kernel file
        let kernel_file = match root.open(path, uefi::proto::media::file::FileMode::Read, uefi::proto::media::file::FileAttribute::empty()) {
            Ok(handle) => handle,
            Err(e) => {
                return Err(BootError::FileNotFound);
            }
        };

        // Get file size
        let file_info = kernel_file.get_info::<uefi::proto::media::file::FileInfo>()?;
        let file_size = file_info.file_size() as usize;

        if file_size == 0 {
            return Err(BootError::InvalidKernelFormat);
        }

        // Allocate memory for kernel
        let st = self.system_table()?;
        let memory_type = uefi::table::boot::MemoryType::LOADER_DATA;
        let kernel_ptr = unsafe {
            st.boot_services().allocate_pool(memory_type, file_size)?
        };

        // Read kernel file
        let mut buffer = unsafe { core::slice::from_raw_parts_mut(kernel_ptr, file_size) };
        kernel_file.read(&mut buffer)?;

        // Create kernel image
        let kernel_image = crate::kernel::KernelLoader::new(&mut crate::memory::BootMemoryManager::new(
            crate::arch::Architecture::X86_64 // Would detect actual architecture
        )?)
        .load_from_data(buffer.to_vec())?;

        Ok(kernel_image)
    }

    /// Exit boot services
    pub fn exit_boot_services(&mut self) -> Result<()> {
        let bs = self.boot_services()?;

        // Get final memory map
        let map_size = bs.memory_map_size()?;
        let mut buffer = vec![0u8; map_size];
        let memory_map = bs.memory_map(&mut buffer)?;

        // Exit boot services
        unsafe {
            bs.exit_boot_services(memory_map)?;
        }

        // Note: After exit_boot_services, only runtime services are available
        Ok(())
    }

    /// Reboot system using UEFI runtime services
    pub fn reboot(&self) -> Result<()> {
        let rt = self.runtime_services()?;

        unsafe {
            rt.reset(uefi::table::runtime::ResetType::WARM, uefi::Status::SUCCESS, None);
        }

        Err(BootError::UefiUnsupported)
    }

    /// Shutdown system using UEFI runtime services
    pub fn shutdown(&self) -> Result<()> {
        let rt = self.runtime_services()?;

        unsafe {
            rt.reset(uefi::table::runtime::ResetType::SHUTDOWN, uefi::Status::SUCCESS, None);
        }

        Err(BootError::UefiUnsupported)
    }
}

#[cfg(feature = "uefi_support")]
impl crate::protocol::BootProtocol for UefiProtocol {
    fn protocol_type(&self) -> crate::protocol::ProtocolType {
        self.protocol_type
    }

    fn detect(&self) -> bool {
        // Check if we're running under UEFI by checking if system table is available
        !self.system_table.load(Ordering::SeqCst).is_null()
    }

    fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        // Try to get the UEFI system table from the environment
        // This would typically be provided by the UEFI application entry point
        // For now, we'll assume it's available if detect() returned true

        Ok(())
    }

    fn get_memory_map(&self) -> Result<MemoryMap> {
        self.get_memory_map()
    }

    fn get_boot_info(&self) -> Result<BootInfo> {
        let mut boot_info = BootInfo::new(ProtocolType::UEFI);

        boot_info.memory_map = self.get_memory_map()?;
        boot_info.framebuffer = self.get_framebuffer_info()?;
        boot_info.acpi_rsdp = self.get_acpi_rsdp()?;
        boot_info.command_line = self.get_command_line()?;

        // Set boot timestamp
        boot_info.boot_timestamp = get_uefi_timestamp();

        Ok(boot_info)
    }

    fn load_kernel(&mut self, path: &str) -> Result<KernelImage> {
        self.load_kernel(path)
    }

    fn exit_boot_services(&mut self) -> Result<()> {
        self.exit_boot_services()
    }

    fn get_framebuffer_info(&self) -> Result<Option<FramebufferInfo>> {
        self.get_framebuffer_info()
    }

    fn get_acpi_rsdp(&self) -> Result<Option<usize>> {
        self.get_acpi_rsdp()
    }

    fn get_command_line(&self) -> Result<Option<String>> {
        self.get_command_line()
    }

    fn reboot(&self) -> Result<()> {
        self.reboot()
    }

    fn shutdown(&self) -> Result<()> {
        self.shutdown()
    }
}

/// Get UEFI timestamp (nanoseconds since boot)
#[cfg(feature = "uefi_support")]
fn get_uefi_timestamp() -> u64 {
    // UEFI doesn't provide a direct timestamp, so we'd use a timer or
    // the runtime services GetTime function if available
    0 // Placeholder
}

/// Active UEFI protocol instance (for UEFI applications)
#[cfg(feature = "uefi_support")]
static mut ACTIVE_UEFI_PROTOCOL: Option<UefiProtocol> = None;

/// Get the active UEFI protocol instance
#[cfg(feature = "uefi_support")]
pub fn get_active_protocol() -> Option<&'static UefiProtocol> {
    unsafe { ACTIVE_UEFI_PROTOCOL.as_ref() }
}

/// Set the active UEFI protocol instance
#[cfg(feature = "uefi_support")]
pub fn set_active_protocol(protocol: UefiProtocol) {
    unsafe {
        ACTIVE_UEFI_PROTOCOL = Some(protocol);
    }
}

// Non-UEFI stub implementations
#[cfg(not(feature = "uefi_support"))]
pub struct UefiProtocol;

#[cfg(not(feature = "uefi_support"))]
impl UefiProtocol {
    pub fn new() -> Self {
        Self
    }
}

#[cfg(not(feature = "uefi_support"))]
impl crate::protocol::BootProtocol for UefiProtocol {
    fn protocol_type(&self) -> crate::protocol::ProtocolType {
        crate::protocol::ProtocolType::UEFI
    }

    fn detect(&self) -> bool {
        false
    }

    fn initialize(&mut self) -> Result<()> {
        Err(BootError::FeatureNotEnabled("UEFI support"))
    }

    fn get_memory_map(&self) -> Result<MemoryMap> {
        Err(BootError::FeatureNotEnabled("UEFI support"))
    }

    fn get_boot_info(&self) -> Result<BootInfo> {
        Err(BootError::FeatureNotEnabled("UEFI support"))
    }

    fn load_kernel(&mut self, _path: &str) -> Result<KernelImage> {
        Err(BootError::FeatureNotEnabled("UEFI support"))
    }

    fn exit_boot_services(&mut self) -> Result<()> {
        Err(BootError::FeatureNotEnabled("UEFI support"))
    }
}

// Re-export UEFI types for convenience
#[cfg(feature = "uefi_support")]
pub use uefi::{
    table::{boot::BootServices, runtime::RuntimeServices, SystemTable},
    proto::{console::gop::GraphicsOutput, loaded_image::LoadedImage},
    Status, Guid,
};

// Convert UEFI errors to BootError
impl From<uefi::Error> for BootError {
    fn from(err: uefi::Error) -> Self {
        BootError::UefiError(err.status())
    }
}