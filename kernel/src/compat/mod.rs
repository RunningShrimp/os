// Cross-platform compatibility layer for NOS
//
// This module provides universal compatibility for running applications from
// Windows, macOS, Android, iOS, and Linux on NOS through a combination of
// binary translation, API compatibility layers, and runtime adaptation.
//
// Architecture Overview:
// 1. Universal Binary Loader - Supports ELF, PE, Mach-O, APK, IPA formats
// 2. System Call Translation Engine - JIT-accelerated foreign syscall translation
// 3. Foreign ABI Support Layer - Multiple calling convention support
//! 4. Memory Layout Manager - Cross-platform memory layout adaptation
//! 5. Platform Compatibility Libraries - Win32, Cocoa, Android, iOS APIs
//! 6. Graphics/UI Translation - DirectX, Metal, OpenGL, Vulkan translation
//! 7. Package Management System - Multi-platform package installation

#![allow(dead_code)]
#![allow(unused_imports)]

extern crate alloc;
extern crate hashbrown;

use core::ffi::{c_void, c_char, c_int, c_uint};
use core::hash::{BuildHasher, Hasher};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use alloc::{format, vec};
use hashbrown::HashMap;
use spin::Mutex;

/// Default hasher builder that implements Default trait
#[derive(Clone, Copy, Debug, Default)]
pub struct DefaultHasherBuilder;

impl BuildHasher for DefaultHasherBuilder {
    type Hasher = DefaultHasher;

    fn build_hasher(&self) -> Self::Hasher {
        DefaultHasher { state: 0 }
    }
}

/// Simple hasher implementation
#[derive(Clone, Copy, Debug)]
pub struct DefaultHasher {
    state: u64,
}

impl DefaultHasher {
    /// Create a new hasher
    pub fn new() -> Self {
        Self { state: 0 }
    }
}

impl Hasher for DefaultHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state = self.state.wrapping_mul(31).wrapping_add(byte as u64);
        }
    }
}

/// Helper function to create a HashMap with default hasher
pub fn new_hashmap<K, V>() -> HashMap<K, V, DefaultHasherBuilder> {
    HashMap::with_hasher(DefaultHasherBuilder)
}

// Re-export all compatibility components
pub mod loader;
pub mod syscall_translator;
pub mod abi;
pub mod memory;
pub mod platforms;
pub mod graphics;
pub mod package_manager;
pub mod sandbox;

// Common types used across the compatibility layer
pub type Result<T> = core::result::Result<T, CompatibilityError>;

/// Universal compatibility error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompatibilityError {
    /// Binary format not supported or corrupted
    InvalidBinaryFormat,
    /// Unsupported architecture or platform
    UnsupportedArchitecture,
    /// System call translation failed
    SyscallTranslationFailed,
    /// Memory allocation or mapping failed
    MemoryError,
    /// Foreign API not implemented
    UnsupportedApi,
    /// JIT compilation error
    CompilationError,
    /// Security restriction
    SecurityViolation,
    /// Invalid arguments
    InvalidArguments,
    /// Resource not found
    NotFound,
    /// Permission denied
    PermissionDenied,
    /// I/O error
    IoError,
}

/// Supported binary formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryFormat {
    /// ELF format (Linux/Unix)
    Elf,
    /// PE format (Windows)
    Pe,
    /// Mach-O format (macOS/iOS)
    MachO,
    /// APK format (Android)
    Apk,
    /// IPA format (iOS App Store)
    Ipa,
    /// Unknown format
    Unknown,
}

/// Supported target platforms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub enum TargetPlatform {
    /// Windows (x86/x64)
    Windows,
    /// macOS (x86/ARM)
    MacOS,
    /// Linux (x86/ARM/RISC-V)
    Linux,
    /// Android (ARM/x86)
    Android,
    /// iOS (ARM)
    IOS,
    /// Native NOS
    Nos,
}

/// CPU architectures we support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Architecture {
    /// 32-bit x86
    X86,
    /// 64-bit x86-64
    X86_64,
    /// 32-bit ARM
    Arm,
    /// 64-bit ARM
    AArch64,
    /// 32-bit RISC-V
    RiscV32,
    /// 64-bit RISC-V
    RiscV64,
}

/// Calling conventions we support
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CallingConvention {
    /// Microsoft x64 calling convention
    MicrosoftX64,
    /// System V AMD64 ABI (Linux/macOS)
    SystemVAMD64,
    /// ARM AArch64 AAPCS
    AArch64AAPCS,
    /// ARM AArch32 AAPCS
    AArch32AAPCS,
    /// RISC-V calling convention
    RiscV,
}

/// Cross-platform binary information
#[derive(Debug, Clone)]
pub struct BinaryInfo {
    /// Binary format type
    pub format: BinaryFormat,
    /// Target platform
    pub platform: TargetPlatform,
    /// Target architecture
    pub architecture: Architecture,
    /// Entry point address
    pub entry_point: usize,
    /// Binary size in bytes
    pub size: usize,
    /// Binary path
    pub path: alloc::string::String,
    /// Additional metadata
    pub metadata: BinaryMetadata,
}

/// Additional binary metadata
#[derive(Debug, Clone, Default)]
pub struct BinaryMetadata {
    /// Binary version if available
    pub version: Option<alloc::string::String>,
    /// Required libraries
    pub dependencies: Vec<alloc::string::String>,
    /// Minimum OS version
    pub min_os_version: Option<alloc::string::String>,
    /// Permissions required
    pub permissions: Vec<alloc::string::String>,
    /// Custom metadata
    pub custom: HashMap<alloc::string::String, alloc::string::String, DefaultHasherBuilder>,
}

/// Loaded binary handle
#[derive(Debug)]
pub struct LoadedBinary {
    /// Binary information
    pub info: BinaryInfo,
    /// Mapped memory regions
    pub memory_regions: Vec<MemoryRegion>,
    /// Entry point in NOS address space
    pub entry_point: usize,
    /// Platform-specific context
    pub platform_context: PlatformContext,
}

/// Memory region description
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// Virtual address in target process
    pub virtual_addr: usize,
    /// Physical address (if applicable)
    pub physical_addr: Option<usize>,
    /// Region size
    pub size: usize,
    /// Memory permissions
    pub permissions: MemoryPermissions,
    /// Region type
    pub region_type: MemoryRegionType,
}

/// Memory permissions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryPermissions {
    /// Readable
    pub read: bool,
    /// Writable
    pub write: bool,
    /// Executable
    pub execute: bool,
}

/// Memory region types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// Code segment
    Code,
    /// Data segment
    Data,
    /// Heap
    Heap,
    /// Stack
    Stack,
    /// Memory mapped file
    MappedFile,
    /// Shared memory
    SharedMemory,
    /// Device mapping
    DeviceMapping,
    /// Guard page
    Guard,
}

/// Platform-specific context
#[derive(Debug)]
pub struct PlatformContext {
    /// Target platform
    pub platform: TargetPlatform,
    /// Platform-specific data
    pub data: PlatformData,
}

/// Platform-specific data
#[derive(Debug)]
pub enum PlatformData {
    /// No specific data
    None,
    /// Windows-specific context
    Windows(WindowsContext),
    /// macOS-specific context
    MacOS(MacOSContext),
    /// Android-specific context
    Android(AndroidContext),
    /// iOS-specific context
    IOS(IOSContext),
    /// Linux-specific context
    Linux(LinuxContext),
}

/// Windows-specific context
#[derive(Debug, Default)]
pub struct WindowsContext {
    /// Windows API version
    pub api_version: Option<u32>,
    /// Required DLLs
    pub required_dlls: Vec<alloc::string::String>,
    /// Registry requirements
    pub registry_entries: Vec<RegistryEntry>,
}

/// macOS-specific context
#[derive(Debug, Default)]
pub struct MacOSContext {
    /// macOS version
    pub os_version: Option<(u32, u32, u32)>,
    /// Required frameworks
    pub frameworks: Vec<alloc::string::String>,
    /// App bundle info
    pub bundle_info: Option<BundleInfo>,
}

/// Android-specific context
#[derive(Debug, Default)]
pub struct AndroidContext {
    /// Android API level
    pub api_level: Option<u32>,
    /// Required permissions
    pub permissions: Vec<alloc::string::String>,
    /// Native libraries
    pub native_libs: Vec<alloc::string::String>,
}

/// iOS-specific context
#[derive(Debug, Default)]
pub struct IOSContext {
    /// iOS version
    pub os_version: Option<(u32, u32, u32)>,
    /// Required frameworks
    pub frameworks: Vec<alloc::string::String>,
    /// App bundle info
    pub bundle_info: Option<BundleInfo>,
}

/// Linux-specific context
#[derive(Debug, Default)]
pub struct LinuxContext {
    /// Required libraries
    pub libraries: Vec<alloc::string::String>,
    /// Linux distribution compatibility
    pub distro: Option<alloc::string::String>,
}

/// Registry entry (Windows)
#[derive(Debug, Clone)]
pub struct RegistryEntry {
    /// Registry key path
    pub key_path: alloc::string::String,
    /// Value name
    pub value_name: alloc::string::String,
    /// Value data
    pub value_data: RegistryData,
}

/// Registry data types
#[derive(Debug, Clone)]
pub enum RegistryData {
    /// String value
    String(alloc::string::String),
    /// DWORD value
    Dword(u32),
    /// Binary data
    Binary(Vec<u8>),
}

/// App bundle information (macOS/iOS)
#[derive(Debug, Clone)]
pub struct BundleInfo {
    /// Bundle identifier
    pub bundle_id: alloc::string::String,
    /// Bundle version
    pub version: alloc::string::String,
    /// Bundle display name
    pub display_name: alloc::string::String,
    /// Executable name
    pub executable: alloc::string::String,
}

/// Global compatibility layer state
static COMPATIBILITY_STATE: spin::Mutex<Option<CompatibilityState>> = spin::Mutex::new(None);

/// Compatibility layer state
#[derive(Default)]
pub struct CompatibilityState {
    /// Loaded binaries registry
    pub loaded_binaries: BTreeMap<usize, LoadedBinary>,
    /// Next binary handle ID
    pub next_binary_id: usize,
    /// Platform compatibility modules
    pub platform_modules: HashMap<TargetPlatform, Arc<dyn PlatformModule>, DefaultHasherBuilder>,
    /// JIT compiler instance
    pub jit_compiler: Option<JitCompiler>,
    /// Memory manager
    pub memory_manager: Arc<MemoryManager>,
    /// Statistics
    pub stats: CompatibilityStats,
}

/// Platform module trait
pub trait PlatformModule: Send + Sync {
    /// Get platform type
    fn platform(&self) -> TargetPlatform;
    /// Check if binary is compatible
    fn is_compatible(&self, info: &BinaryInfo) -> bool;
    /// Load platform-specific resources
    fn load_binary(&mut self, info: BinaryInfo) -> Result<LoadedBinary>;
    /// Create platform context
    fn create_context(&self, info: &BinaryInfo) -> Result<PlatformContext>;
}

/// JIT compiler interface
#[derive(Debug)]
pub struct JitCompiler {
    /// Code cache
    pub code_cache: HashMap<u64, CompiledCode, DefaultHasherBuilder>,
    /// Next cache ID
    pub next_cache_id: u64,
}

/// Compiled JIT code
#[derive(Debug)]
pub struct CompiledCode {
    /// Cache ID
    pub id: u64,
    /// Entry point address
    pub entry_point: usize,
    /// Code size in bytes
    pub size: usize,
    /// Source hash for validation
    pub source_hash: u64,
}

/// Memory manager for compatibility layer
#[derive(Debug)]
pub struct MemoryManager {
    /// Memory regions allocated for compatibility
    pub regions: Vec<MemoryRegion>,
    /// Next available address
    pub next_addr: usize,
    /// Allocation statistics
    pub stats: MemoryStats,
}

impl Default for MemoryManager {
    fn default() -> Self {
        Self {
            regions: Vec::new(),
            next_addr: 0,
            stats: MemoryStats::default(),
        }
    }
}

/// Memory allocation statistics
#[derive(Debug, Default)]
pub struct MemoryStats {
    /// Total allocated bytes
    pub total_allocated: usize,
    /// Peak allocation
    pub peak_allocation: usize,
    /// Number of allocations
    pub allocation_count: usize,
}

/// Compatibility layer statistics
#[derive(Debug, Default)]
pub struct CompatibilityStats {
    /// Number of binaries loaded
    pub binaries_loaded: usize,
    /// Number of system calls translated
    pub syscalls_translated: u64,
    /// JIT cache hits
    pub jit_cache_hits: u64,
    /// JIT cache misses
    pub jit_cache_misses: u64,
    /// Memory usage statistics
    pub memory_stats: MemoryStats,
}

impl MemoryPermissions {
    /// Create new permissions
    pub fn new(read: bool, write: bool, execute: bool) -> Self {
        Self { read, write, execute }
    }

    /// Read-only permission
    pub fn readonly() -> Self {
        Self::new(true, false, false)
    }

    /// Read-write permission
    pub fn readwrite() -> Self {
        Self::new(true, true, false)
    }

    /// Execute-only permission
    pub fn exec() -> Self {
        Self::new(false, false, true)
    }

    /// Read-execute permission
    pub fn read_exec() -> Self {
        Self::new(true, false, true)
    }

    /// Read-write-execute permission
    pub fn rwx() -> Self {
        Self::new(true, true, true)
    }

    /// Check if permissions are valid
    pub fn is_valid(&self) -> bool {
        // Cannot have write without read
        if self.write && !self.read {
            return false;
        }
        true
    }

    /// Convert to page table flags
    pub fn to_pte_flags(&self) -> usize {
        let mut flags = 0;
        if self.read {
            flags |= 1; // Present
        }
        if self.write {
            flags |= 2; // Writable
        }
        if self.execute {
            flags |= 4; // Executable (NX disable if not set)
        }
        flags
    }
}

/// Initialize the compatibility layer
pub fn init_compatibility_layer() -> Result<()> {
    let mut state = COMPATIBILITY_STATE.lock();
    if state.is_some() {
        return Ok(()); // Already initialized
    }

    // Create memory manager
    let memory_manager = Arc::new(MemoryManager {
        regions: Vec::new(),
        next_addr: 0x40000000usize, // Start at 1GB for compatibility binaries
        stats: MemoryStats::default(),
    });

    // Initialize platform modules
    let mut platform_modules: HashMap<TargetPlatform, Arc<dyn PlatformModule>, DefaultHasherBuilder> = HashMap::with_hasher(DefaultHasherBuilder);

    // Register platform modules (will be implemented in submodules)
    // platform_modules.insert(TargetPlatform::Windows, Arc::new(platforms::windows::WindowsModule::new()));
    // platform_modules.insert(TargetPlatform::MacOS, Arc::new(platforms::macos::MacOSModule::new()));
    // platform_modules.insert(TargetPlatform::Linux, Arc::new(platforms::linux::LinuxModule::new()));
    // platform_modules.insert(TargetPlatform::Android, Arc::new(platforms::android::AndroidModule::new()));
    // platform_modules.insert(TargetPlatform::IOS, Arc::new(platforms::ios::IOSModule::new()));

    *state = Some(CompatibilityState {
        loaded_binaries: BTreeMap::new(),
        next_binary_id: 1,
        platform_modules,
        jit_compiler: Some(JitCompiler {
            code_cache: HashMap::with_hasher(DefaultHasherBuilder),
            next_cache_id: 1,
        }),
        memory_manager,
        stats: CompatibilityStats::default(),
    });

    crate::println!("[compat] Cross-platform compatibility layer initialized");
    Ok(())
}

/// Get global compatibility state
pub fn get_compatibility_state() -> spin::MutexGuard<'static, Option<CompatibilityState>> {
    COMPATIBILITY_STATE.lock()
}

/// Detect binary format from file data
pub fn detect_binary_format(data: &[u8]) -> BinaryFormat {
    if data.len() < 4 {
        return BinaryFormat::Unknown;
    }

    // Check magic numbers
    match &data[0..4] {
        [0x7f, b'E', b'L', b'F'] => BinaryFormat::Elf,
        [b'M', b'Z'] => BinaryFormat::Pe,
        [0xfe, 0xed, 0xfa, 0xce] | [0xfe, 0xed, 0xfa, 0xcf] => BinaryFormat::MachO,
        [0x50, 0x4b, 0x03, 0x04] => {
            // Could be APK or IPA - check further
            if data.len() > 30 && &data[30..34] == b"Android" {
                BinaryFormat::Apk
            } else {
                BinaryFormat::Unknown
            }
        }
        _ => BinaryFormat::Unknown,
    }
}

/// Detect target architecture from binary data
pub fn detect_architecture(data: &[u8], format: BinaryFormat) -> Architecture {
    match format {
        BinaryFormat::Elf => {
            if data.len() >= 19 {
                match data[18] {
                    0x03 => Architecture::X86,
                    0x3e => Architecture::X86_64,
                    0x28 => Architecture::Arm,
                    0xb7 => Architecture::AArch64,
                    0xf3 => Architecture::RiscV32,
                    0xb3 => Architecture::RiscV64,
                    _ => Architecture::X86_64, // Default
                }
            } else {
                Architecture::X86_64
            }
        }
        BinaryFormat::Pe => {
            if data.len() >= 68 {
                let machine = u16::from_le_bytes([data[60], data[61]]);
                match machine {
                    0x014c => Architecture::X86,
                    0x8664 => Architecture::X86_64,
                    0x01c0 => Architecture::Arm,
                    0xaa64 => Architecture::AArch64,
                    _ => Architecture::X86_64,
                }
            } else {
                Architecture::X86_64
            }
        }
        BinaryFormat::MachO => {
            if data.len() >= 4 {
                let cputype = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
                match cputype {
                    0x00000007 => Architecture::X86,
                    0x01000007 => Architecture::X86_64,
                    0x0000000c => Architecture::Arm,
                    0x0100000c => Architecture::AArch64,
                    _ => Architecture::X86_64,
                }
            } else {
                Architecture::X86_64
            }
        }
        _ => Architecture::X86_64, // Default
    }
}

/// Module initialization functions
pub fn init() -> Result<()> {
    init_compatibility_layer()
}
