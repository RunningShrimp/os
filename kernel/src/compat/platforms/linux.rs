//! Linux Binary Compatibility Layer
//!
//! Provides compatibility for Linux applications on NOS:
//! - Linux syscalls mapping
//! - GLIBC compatibility
//! - Linux VFS compatibility
//! - Linux signal handling
//! - Linux threading (pthread)

extern crate alloc;

use alloc::vec;
use alloc::string::String;
use alloc::string::ToString;
use alloc::collections::BTreeMap;
use crate::compat::*;

/// Linux compatibility module
pub struct LinuxModule {
    syscall_table: LinuxSyscallTable,
    glibc_compatibility: GlibcCompatibility,
    vfs_compatibility: LinuxVfsCompatibility,
}

impl LinuxModule {
    pub fn new() -> Self {
        Self {
            syscall_table: LinuxSyscallTable::new(),
            glibc_compatibility: GlibcCompatibility::new(),
            vfs_compatibility: LinuxVfsCompatibility::new(),
        }
    }
}

impl PlatformModule for LinuxModule {
    fn platform(&self) -> TargetPlatform {
        TargetPlatform::Linux
    }

    fn is_compatible(&self, info: &BinaryInfo) -> bool {
        matches!(info.platform, TargetPlatform::Linux) &&
        matches!(info.format, BinaryFormat::Elf)
    }

    fn load_binary(&mut self, info: BinaryInfo) -> Result<LoadedBinary> {
        // Create memory regions based on ELF segments
        let memory_regions = vec![
            MemoryRegion {
                virtual_addr: 0x400000, // Standard Linux executable base
                physical_addr: None,
                size: info.size,
                permissions: MemoryPermissions::read_exec(),
                region_type: MemoryRegionType::Code,
            },
            MemoryRegion {
                virtual_addr: 0x600000, // Data segment
                physical_addr: None,
                size: 0x100000, // 1MB
                permissions: MemoryPermissions::readwrite(),
                region_type: MemoryRegionType::Data,
            },
        ];

        let entry_point = info.entry_point;
        Ok(LoadedBinary {
            info,
            memory_regions,
            entry_point: 0x400000 + entry_point,
            platform_context: PlatformContext {
                platform: TargetPlatform::Linux,
                data: PlatformData::Linux(LinuxContext::default()),
            },
        })
    }

    fn create_context(&self, info: &BinaryInfo) -> Result<PlatformContext> {
        Ok(PlatformContext {
            platform: TargetPlatform::Linux,
            data: PlatformData::Linux(LinuxContext {
                libraries: vec![
                    "libc.so.6".to_string(),
                    "libm.so.6".to_string(),
                    "libpthread.so.0".to_string(),
                ],
                distro: Some("Ubuntu".to_string()),
            }),
        })
    }
}

/// Linux system call table
#[derive(Debug)]
pub struct LinuxSyscallTable {
    syscalls: BTreeMap<u32, String>,
}

impl LinuxSyscallTable {
    pub fn new() -> Self {
        let mut table = Self {
            syscalls: BTreeMap::new(),
        };

        table.register_core_syscalls();
        table
    }

    fn register_core_syscalls(&mut self) {
        // Common Linux syscalls
        self.syscalls.insert(0, "sys_read".to_string());
        self.syscalls.insert(1, "sys_write".to_string());
        self.syscalls.insert(2, "sys_open".to_string());
        self.syscalls.insert(3, "sys_close".to_string());
        self.syscalls.insert(39, "sys_getpid".to_string());
        self.syscalls.insert(57, "sys_fork".to_string());
        self.syscalls.insert(60, "sys_exit".to_string());
    }
}

/// GLIBC compatibility layer
#[derive(Debug)]
pub struct GlibcCompatibility {
    // GLIBC implementation
}

impl GlibcCompatibility {
    pub fn new() -> Self {
        Self {}
    }
}

/// Linux VFS compatibility
#[derive(Debug)]
pub struct LinuxVfsCompatibility {
    // VFS compatibility implementation
}

impl LinuxVfsCompatibility {
    pub fn new() -> Self {
        Self {}
    }
}