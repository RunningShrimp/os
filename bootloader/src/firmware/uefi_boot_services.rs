// UEFI Boot Services function pointers and protocol implementation

use core::ffi::c_void;

/// UEFI Boot Services function pointers
#[repr(C)]
pub struct UefiBootServicesTable {
    // Task Priority Services
    pub raise_tpl: Option<unsafe extern "efiapi" fn(new_tpl: usize) -> usize>,
    pub restore_tpl: Option<unsafe extern "efiapi" fn(old_tpl: usize)>,

    // Memory Services
    pub allocate_pages: Option<
        unsafe extern "efiapi" fn(
            alloc_type: u32,
            mem_type: u32,
            pages: u64,
            memory: *mut u64,
        ) -> u64,
    >,
    pub free_pages: Option<
        unsafe extern "efiapi" fn(memory: u64, pages: u64) -> u64,
    >,
    pub get_memory_map: Option<
        unsafe extern "efiapi" fn(
            memory_map_size: *mut usize,
            memory_map: *mut c_void,
            map_key: *mut usize,
            desc_size: *mut usize,
            desc_version: *mut u32,
        ) -> u64,
    >,
    pub allocate_pool: Option<
        unsafe extern "efiapi" fn(
            pool_type: u32,
            size: usize,
            buffer: *mut *mut c_void,
        ) -> u64,
    >,
    pub free_pool: Option<
        unsafe extern "efiapi" fn(buffer: *mut c_void) -> u64,
    >,

    // Event & Timer Services
    pub create_event: Option<
        unsafe extern "efiapi" fn(
            event_type: u32,
            notify_tpl: usize,
            notify_fn: *mut c_void,
            notify_ctx: *mut c_void,
            event: *mut *mut c_void,
        ) -> u64,
    >,
    pub set_timer: Option<
        unsafe extern "efiapi" fn(event: *mut c_void, typ: u32, trigger: u64) -> u64,
    >,
    pub wait_for_event: Option<
        unsafe extern "efiapi" fn(num_events: usize, events: *mut *mut c_void) -> u64,
    >,
    pub signal_event: Option<unsafe extern "efiapi" fn(event: *mut c_void) -> u64>,
    pub close_event: Option<unsafe extern "efiapi" fn(event: *mut c_void) -> u64>,
    pub check_event: Option<unsafe extern "efiapi" fn(event: *mut c_void) -> u64>,

    // Protocol Handler Services
    pub install_protocol_interface: Option<
        unsafe extern "efiapi" fn(
            handle: *mut *mut c_void,
            protocol: *const [u8; 16],
            interface_type: u32,
            interface: *mut c_void,
        ) -> u64,
    >,
    pub uninstall_protocol_interface: Option<
        unsafe extern "efiapi" fn(
            handle: *mut c_void,
            protocol: *const [u8; 16],
            interface: *mut c_void,
        ) -> u64,
    >,
    pub handle_protocol: Option<
        unsafe extern "efiapi" fn(
            handle: *mut c_void,
            protocol: *const [u8; 16],
            interface: *mut *mut c_void,
        ) -> u64,
    >,

    // Image Services
    pub load_image: Option<
        unsafe extern "efiapi" fn(
            boot_policy: u8,
            parent_image_handle: *mut c_void,
            device_path: *mut c_void,
            source_buffer: *mut u8,
            source_size: usize,
            image_handle: *mut *mut c_void,
        ) -> u64,
    >,
    pub start_image: Option<
        unsafe extern "efiapi" fn(image_handle: *mut c_void) -> u64,
    >,
    pub exit: Option<
        unsafe extern "efiapi" fn(
            image_handle: *mut c_void,
            exit_status: u64,
            exit_data_size: usize,
            exit_data: *mut *mut u8,
        ) -> u64,
    >,
    pub unload_image: Option<
        unsafe extern "efiapi" fn(image_handle: *mut c_void) -> u64,
    >,

    // Miscellaneous Services
    pub exit_boot_services: Option<
        unsafe extern "efiapi" fn(image_handle: *mut c_void, map_key: usize) -> u64,
    >,
}

/// UEFI Runtime Services function pointers
#[repr(C)]
pub struct UefiRuntimeServicesTable {
    pub get_time: Option<unsafe extern "efiapi" fn() -> u64>,
    pub set_time: Option<unsafe extern "efiapi" fn() -> u64>,
    pub get_wakeup_time: Option<unsafe extern "efiapi" fn() -> u64>,
    pub set_wakeup_time: Option<unsafe extern "efiapi" fn() -> u64>,
    pub set_virtual_address_map: Option<unsafe extern "efiapi" fn() -> u64>,
    pub convert_pointer: Option<unsafe extern "efiapi" fn() -> u64>,
    pub get_variable: Option<unsafe extern "efiapi" fn() -> u64>,
    pub get_next_variable_name: Option<unsafe extern "efiapi" fn() -> u64>,
    pub set_variable: Option<unsafe extern "efiapi" fn() -> u64>,
}

/// UEFI System Table header
#[repr(C)]
pub struct UefiSystemTableHeader {
    pub signature: u64,           // "IBI\x20SYS" = 0x5453595320494249
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

/// UEFI System Table
#[repr(C)]
pub struct UefiSystemTable {
    pub header: UefiSystemTableHeader,
    pub firmware_vendor: *const u16,
    pub firmware_revision: u32,
    pub console_in_handle: *mut c_void,
    pub con_in: *mut c_void,
    pub console_out_handle: *mut c_void,
    pub con_out: *mut c_void,
    pub console_err_handle: *mut c_void,
    pub std_err: *mut c_void,
    pub runtime_services: *mut UefiRuntimeServicesTable,
    pub boot_services: *mut UefiBootServicesTable,
    pub num_table_entries: usize,
    pub configuration_table: *mut UefiConfigurationTable,
}

#[repr(C)]
pub struct UefiConfigurationTable {
    pub vendor_guid: [u8; 16],
    pub vendor_table: *mut c_void,
}

// UEFI Status codes
pub const EFI_SUCCESS: u64 = 0;
pub const EFI_INVALID_PARAMETER: u64 = 0x8000000000000002;
pub const EFI_OUT_OF_RESOURCES: u64 = 0x8000000000000005;
pub const EFI_DEVICE_ERROR: u64 = 0x8000000000000007;

// Memory Allocation Types
pub const ALLOCATE_ANY_PAGES: u32 = 0;
pub const ALLOCATE_MAX_ADDRESS: u32 = 1;
pub const ALLOCATE_ADDRESS: u32 = 2;

// Memory Types
pub const EFI_RESERVED_MEMORY_TYPE: u32 = 0;
pub const EFI_LOADER_CODE: u32 = 1;
pub const EFI_LOADER_DATA: u32 = 2;
pub const EFI_BOOT_SERVICES_CODE: u32 = 3;
pub const EFI_BOOT_SERVICES_DATA: u32 = 4;
pub const EFI_RUNTIME_SERVICES_CODE: u32 = 5;
pub const EFI_RUNTIME_SERVICES_DATA: u32 = 6;
pub const EFI_CONVENTIONAL_MEMORY: u32 = 7;
pub const EFI_UNUSABLE_MEMORY: u32 = 8;
pub const EFI_ACPI_RECLAIM_MEMORY: u32 = 9;
pub const EFI_ACPI_MEMORY_NVS: u32 = 10;
pub const EFI_MEMORY_MAPPED_IO: u32 = 11;
pub const EFI_MEMORY_MAPPED_IO_PORT_SPACE: u32 = 12;
pub const EFI_PAL_CODE: u32 = 13;
pub const EFI_MAX_MEMORY_TYPE: u32 = 14;

/// UEFI Memory Descriptor
#[repr(C)]
pub struct UefiMemoryDescriptor {
    pub typ: u32,
    pub pad: u32,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub number_of_pages: u64,
    pub attribute: u64,
}

pub struct UefiBootContext {
    pub system_table: *mut UefiSystemTable,
    pub image_handle: *mut c_void,
}

impl UefiBootContext {
    pub fn new(
        system_table: *mut UefiSystemTable,
        image_handle: *mut c_void,
    ) -> Self {
        Self {
            system_table,
            image_handle,
        }
    }

    /// Get memory map from UEFI
    pub unsafe fn get_memory_map(&self) -> Result<(usize, usize, u32), &'static str> {
        let st = &*self.system_table;
        let bs = &*st.boot_services;

        if let Some(get_mem_map) = bs.get_memory_map {
            let mut map_size: usize = 0;
            let mut map_key: usize = 0;
            let mut desc_size: usize = 0;
            let mut desc_ver: u32 = 0;

            // First call to get size
            let status = get_mem_map(
                &mut map_size,
                core::ptr::null_mut(),
                &mut map_key,
                &mut desc_size,
                &mut desc_ver,
            );

            if status != EFI_SUCCESS {
                return Err("Failed to get memory map size");
            }

            crate::drivers::console::write_str("Memory map size: ");
            crate::drivers::console::write_str("bytes\n");

            Ok((map_size, desc_size, desc_ver))
        } else {
            Err("GetMemoryMap function pointer not available")
        }
    }

    /// Allocate memory pages
    pub unsafe fn allocate_pages(
        &self,
        pages: u64,
    ) -> Result<u64, &'static str> {
        let st = &*self.system_table;
        let bs = &*st.boot_services;

        if let Some(alloc) = bs.allocate_pages {
            let mut addr: u64 = 0;

            let status = alloc(
                ALLOCATE_ANY_PAGES,
                EFI_LOADER_DATA,
                pages,
                &mut addr,
            );

            if status == EFI_SUCCESS {
                crate::drivers::console::write_str("Pages allocated\n");
                Ok(addr)
            } else {
                Err("AllocatePages failed")
            }
        } else {
            Err("AllocatePages function pointer not available")
        }
    }

    /// Load kernel image
    pub unsafe fn load_image(
        &self,
        source_buffer: *mut u8,
        source_size: usize,
    ) -> Result<*mut c_void, &'static str> {
        let st = &*self.system_table;
        let bs = &*st.boot_services;

        if let Some(load_image) = bs.load_image {
            let mut image_handle: *mut c_void = core::ptr::null_mut();

            let status = load_image(
                0,
                self.image_handle,
                core::ptr::null_mut(),
                source_buffer,
                source_size,
                &mut image_handle,
            );

            if status == EFI_SUCCESS {
                crate::drivers::console::write_str("Kernel image loaded\n");
                Ok(image_handle)
            } else {
                Err("LoadImage failed")
            }
        } else {
            Err("LoadImage function pointer not available")
        }
    }

    /// Exit boot services
    pub unsafe fn exit_boot_services(&self, map_key: usize) -> Result<(), &'static str> {
        let st = &*self.system_table;
        let bs = &*st.boot_services;

        if let Some(exit_bs) = bs.exit_boot_services {
            let status = exit_bs(self.image_handle, map_key);

            if status == EFI_SUCCESS {
                crate::drivers::console::write_str("Boot services exited\n");
                Ok(())
            } else {
                Err("ExitBootServices failed")
            }
        } else {
            Err("ExitBootServices function pointer not available")
        }
    }
}

/// Validate UEFI System Table
pub unsafe fn validate_uefi_system_table(
    st: *const UefiSystemTable,
) -> Result<(), &'static str> {
    if st.is_null() {
        return Err("Null system table");
    }

    let header = &(*st).header;

    // Check signature: "IBI\x20SYS" = 0x5453595320494249
    if header.signature != 0x5453595320494249 {
        return Err("Invalid EFI signature");
    }

    if (*st).boot_services.is_null() {
        return Err("Boot services null");
    }

    crate::drivers::console::write_str("UEFI system table validated\n");
    Ok(())
}
