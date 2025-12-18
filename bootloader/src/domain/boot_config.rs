//! Boot Configuration - Pure Value Object
//!
//! Represents immutable bootloader configuration with validation rules.
//! Does NOT contain business logic beyond validation and construction.
//!
//! # Examples
//!
//! ```no_run
//! # use nos_bootloader::domain::boot_config::{GraphicsMode, BootConfig};
//! let mode = GraphicsMode::new(1024, 768, 32).expect("Valid graphics mode");
//! let config = BootConfig::default();
//! assert!(config.validate().is_ok());
//! ```

use alloc::boxed::Box;
use alloc::string::String;
use core::fmt;
use crate::domain::boot_services::GraphicsCapabilities;

/// Memory region type for boot configuration
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemoryRegionType {
    /// Available memory that can be used
    Available,
    /// Reserved memory that should not be touched
    Reserved,
    /// ACPI reclaimable memory
    AcpiReclaimable,
    /// ACPI non-volatile memory
    AcpiNvs,
    /// Memory mapped I/O
    Mmio,
    /// Unusable memory
    Unusable,
}

/// Memory region - Value Object
///
/// Represents a memory region with its type and attributes.
/// This is a value object that describes memory layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryRegion {
    /// Entity ID
    pub id: u64,
    /// Start physical address
    pub start: u64,
    /// End physical address (exclusive)
    pub end: u64,
    /// Type of memory region
    pub region_type: MemoryRegionType,
}

impl MemoryRegion {
    /// Create a new memory region
    pub fn new(start: u64, end: u64, region_type: MemoryRegionType) -> Result<Self, &'static str> {
        if start >= end {
            return Err("Invalid memory region: start address must be less than end address");
        }
        
        Ok(Self { 
            id: 0, // Temporary ID, will be set by repository
            start, 
            end, 
            region_type 
        })
    }
    
    /// Get the size of the memory region in bytes
    pub fn size(&self) -> u64 {
        self.end - self.start
    }
    
    /// Check if this region overlaps with another region
    pub fn overlaps(&self, other: &MemoryRegion) -> bool {
        self.start < other.end && other.start < self.end
    }
    
    /// Check if this region contains the given address
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.start && addr < self.end
    }
    
    /// Check if this region is available for use
    pub fn is_available(&self) -> bool {
        matches!(self.region_type, MemoryRegionType::Available)
    }
}

impl fmt::Display for MemoryRegion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:#x}-{:#x} ({:?})", self.start, self.end, self.region_type)
    }
}

/// Kernel information - Aggregate Root
///
/// Contains information about the kernel image to be loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct KernelInfo {
    /// Entity ID
    pub id: u64,
    /// Kernel image base address in memory
    pub address: u64,
    /// Kernel image size in bytes
    pub size: u64,
    /// Kernel entry point address
    pub entry_point: u64,
    /// Kernel command line
    pub cmdline: Option<[u8; 512]>,
    pub cmdline_len: usize,
    /// Kernel signature verification status
    pub signature_verified: bool,
}

impl KernelInfo {
    /// Create new kernel information
    pub fn new(address: u64, size: u64, entry_point: u64) -> Result<Self, &'static str> {
        if address == 0 {
            return Err("Kernel address cannot be zero");
        }
        
        if size == 0 {
            return Err("Kernel size cannot be zero");
        }
        
        if entry_point == 0 {
            return Err("Kernel entry point cannot be zero");
        }
        
        // Check if entry point is within the kernel image
        if entry_point < address || entry_point >= address + size {
            return Err("Kernel entry point must be within the kernel image");
        }
        
        Ok(Self {
            id: 0, // Temporary ID, will be set by repository
            address,
            size,
            entry_point,
            cmdline: None,
            cmdline_len: 0,
            signature_verified: false,
        })
    }
    
    /// Set kernel command line
    pub fn with_cmdline(mut self, cmdline: &[u8]) -> Result<Self, &'static str> {
        if cmdline.len() > 512 {
            return Err("Kernel command line too long");
        }
        
        let mut cmdline_array = [0u8; 512];
        cmdline_array[..cmdline.len()].copy_from_slice(cmdline);
        
        self.cmdline = Some(cmdline_array);
        self.cmdline_len = cmdline.len();
        Ok(self)
    }
    
    /// Set signature verification status
    pub fn with_signature_verified(mut self, verified: bool) -> Self {
        self.signature_verified = verified;
        self
    }
    
    /// Get kernel command line as slice
    pub fn get_cmdline(&self) -> &[u8] {
        if let Some(ref cmdline) = self.cmdline {
            &cmdline[..self.cmdline_len]
        } else {
            &[]
        }
    }
    
    /// Check if kernel is properly loaded in memory
    pub fn is_valid(&self) -> bool {
        self.address != 0 && self.size != 0 && self.entry_point != 0
    }
    
    /// Get the end address of the kernel image
    pub fn end_address(&self) -> u64 {
        self.address + self.size
    }
}

impl fmt::Display for KernelInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Kernel Information:")?;
        writeln!(f, "  Address: {:#x}", self.address)?;
        writeln!(f, "  Size: {} bytes", self.size)?;
        writeln!(f, "  Entry Point: {:#x}", self.entry_point)?;
        writeln!(f, "  Signature Verified: {}", self.signature_verified)?;
        
        if self.cmdline_len > 0 {
            writeln!(f, "  Command Line: {}", String::from_utf8_lossy(self.get_cmdline()))?;
        }
        
        Ok(())
    }
}

/// Log verbosity level
///
/// Defines the logging detail level during boot:
/// - `Silent`: No output
/// - `Info`: Essential information only
/// - `Verbose`: Detailed operation information
/// - `Debug`: Complete debugging information
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogLevel {
    /// Silent - no output (0)
    Silent = 0,
    /// Info - essential information (1)
    Info = 1,
    /// Verbose - detailed information (2)
    Verbose = 2,
    /// Debug - complete debugging information (3)
    Debug = 3,
}

impl LogLevel {
    /// Check if level is verbose or higher
    pub fn is_verbose(&self) -> bool {
        matches!(self, Self::Verbose | Self::Debug)
    }

    /// Check if level is debug
    pub fn is_debug(&self) -> bool {
        matches!(self, Self::Debug)
    }
}

/// Graphics display mode - Value Object
///
/// Represents a valid graphics display mode with:
/// - Resolution constraints: 320-4096 x 200-2160 pixels
/// - Bits-per-pixel: 16, 24, or 32
/// - Immutable after creation
///
/// # Examples
///
/// ```no_run
/// # use nos_bootloader::domain::boot_config::GraphicsMode;
/// let mode = GraphicsMode::new(1024, 768, 32).expect("Valid mode");
/// assert_eq!(mode.width, 1024);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GraphicsMode {
    pub width: u16,
    pub height: u16,
    pub bits_per_pixel: u8,
}

impl GraphicsMode {
    /// Create a new graphics mode
    /// 
    /// # Errors
    /// Returns error if dimensions or color depth are invalid
    pub fn new(width: u16, height: u16, bits_per_pixel: u8) -> Result<Self, &'static str> {
        if width < 320 || width > 4096 {
            return Err("Invalid graphics width (320-4096)");
        }
        if height < 200 || height > 2160 {
            return Err("Invalid graphics height (200-2160)");
        }
        match bits_per_pixel {
            8 | 16 | 24 | 32 => Ok(Self {
                width,
                height,
                bits_per_pixel,
            }),
            _ => Err("Unsupported bits per pixel (8, 16, 24, 32)")
        }
    }
    
    /// Calculate the aspect ratio (width / height)
    pub fn aspect_ratio(&self) -> f32 {
        self.width as f32 / self.height as f32
    }

    /// Check if this graphics mode is compatible with the given hardware capabilities
    pub fn is_compatible_with(&self, caps: &GraphicsCapabilities) -> bool {
        // Check if resolution is within supported range
        if self.width > caps.max_width || self.height > caps.max_height {
            return false;
        }
        
        // Check if color depth is supported
        // For simplicity, assume max_colors >= 8 means 24/32 bit is supported
        if self.bits_per_pixel >= 24 && caps.max_colors < 8 {
            return false;
        }
        
        true
    }
    
    /// Check if mode is high resolution (>= 1024x768)
    ///
    /// # Returns
    /// `true` if width >= 1024 and height >= 768
    pub fn is_high_resolution(&self) -> bool {
        self.width >= 1024 && self.height >= 768
    }

    /// Bytes per scanline (with 64-byte alignment for cache efficiency)
    ///
    /// # Returns
    /// Scanline width in bytes, 64-byte aligned
    pub fn scanline_bytes(&self) -> usize {
        let bytes_per_pixel = (self.bits_per_pixel as usize + 7) / 8;
        let raw = self.width as usize * bytes_per_pixel;
        // Align to 64-byte boundary for cache efficiency
        ((raw + 63) / 64) * 64
    }

    /// Total framebuffer size in bytes
    ///
    /// # Returns
    /// Complete framebuffer size needed for this mode
    pub fn framebuffer_size(&self) -> usize {
        self.scanline_bytes() * self.height as usize
    }
}

impl fmt::Display for GraphicsMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}x{}x{}", self.width, self.height, self.bits_per_pixel)
    }
}

/// Boot Configuration - Immutable Value Object
///
/// Encapsulates all bootloader configuration including:
/// - Logging level and verbosity
/// - Graphics display mode
/// - Feature flags (paging, memory checks, etc.)
/// - Memory requirements
///
/// This is a pure value object with no business logic beyond validation.
/// It represents the configuration state at a point in time.
///
/// # Validation
/// - `validate()` ensures all configuration parameters are consistent
/// - Graphics mode is checked for valid dimensions
/// - Memory requirements are verified against available RAM
///
/// # Examples
///
/// ```no_run
/// # use nos_bootloader::domain::boot_config::BootConfig;
/// let config = BootConfig::default();
/// assert!(config.validate().is_ok());
/// ```
#[derive(Clone, Debug)]
pub struct BootConfig {
    pub verbosity: LogLevel,
    pub enable_post: bool,
    pub enable_paging: bool,
    pub enable_memory_check: bool,
    pub enable_device_detect: bool,
    pub graphics_mode: Option<GraphicsMode>,
    pub kernel_path: Option<[u8; 256]>,
    pub kernel_path_len: usize,
    pub cmdline: Option<[u8; 512]>,
    pub cmdline_len: usize,
}

impl BootConfig {
    /// Create default boot configuration
    pub fn new() -> Self {
        Self {
            verbosity: LogLevel::Info,
            enable_post: true,
            enable_paging: true,
            enable_memory_check: true,
            enable_device_detect: true,
            graphics_mode: GraphicsMode::new(1024, 768, 32).ok(),
            kernel_path: None,
            kernel_path_len: 0,
            cmdline: None,
            cmdline_len: 0,
        }
    }

    /// Get kernel path as slice
    pub fn get_kernel_path(&self) -> &[u8] {
        if let Some(ref path) = self.kernel_path {
            &path[..self.kernel_path_len]
        } else {
            &[]
        }
    }

    /// Get command line as slice
    pub fn get_cmdline(&self) -> &[u8] {
        if let Some(ref cmdline) = self.cmdline {
            &cmdline[..self.cmdline_len]
        } else {
            &[]
        }
    }

    /// Validate configuration consistency
    ///
    /// Checks that graphics mode settings are valid, memory checks don't
    /// conflict with paging settings, etc.
    pub fn validate(&self) -> Result<(), &'static str> {
        // If graphics mode is set, verify it's valid
        if let Some(mode) = self.graphics_mode {
            if mode.framebuffer_size() > 256 * 1024 * 1024 {
                return Err("Graphics mode requires too much memory (>256MB)");
            }
            
            // High resolution graphics requires paging
            if !self.enable_paging && mode.is_high_resolution() {
                return Err("High resolution graphics requires paging to be enabled");
            }
        }

        // Validate kernel path length
        if self.kernel_path_len > 256 {
            return Err("Kernel path too long");
        }
        
        // Validate command line length
        if self.cmdline_len > 512 {
            return Err("Command line too long");
        }
        
        // Memory check requires paging
        if self.enable_memory_check && !self.enable_paging {
            return Err("Memory check requires paging to be enabled");
        }
        
        // POST and device detection are recommended together
        if self.enable_post && !self.enable_device_detect {
            return Err("POST without device detection may miss critical hardware issues");
        }
        
        Ok(())
    }
}

impl Default for BootConfig {
    fn default() -> Self {
        Self::new()
    }
}

use crate::domain::AggregateRoot;
use crate::domain::repositories::{EntityId, RepositoryError};

impl AggregateRoot for KernelInfo {
    fn clone_aggregate(&self) -> Box<dyn AggregateRoot> {
        Box::new(self.clone())
    }
    
    fn id(&self) -> EntityId {
        EntityId::new(self.id)
    }
    
    fn set_id(&mut self, id: EntityId) {
        self.id = id.value();
    }
    
    fn validate(&self) -> Result<(), RepositoryError> {
        if self.address == 0 {
            return Err(RepositoryError::ValidationError("Kernel address cannot be zero"));
        }
        
        if self.size == 0 {
            return Err(RepositoryError::ValidationError("Kernel size cannot be zero"));
        }
        
        if self.entry_point == 0 {
            return Err(RepositoryError::ValidationError("Kernel entry point cannot be zero"));
        }
        
        // Check if entry point is within the kernel image
        if self.entry_point < self.address || self.entry_point >= self.address + self.size {
            return Err(RepositoryError::ValidationError("Kernel entry point must be within the kernel image"));
        }
        
        Ok(())
    }
    
    fn entity_type() -> &'static str
    where
        Self: Sized,
    {
        "KernelInfo"
    }

    fn entity_type_dyn(&self) -> &'static str {
        Self::entity_type()
    }
}

impl AggregateRoot for GraphicsInfo {
    fn clone_aggregate(&self) -> Box<dyn AggregateRoot> {
        Box::new(self.clone())
    }
    
    fn id(&self) -> EntityId {
        EntityId::new(self.id)
    }
    
    fn set_id(&mut self, id: EntityId) {
        self.id = id.value();
    }
    
    fn validate(&self) -> Result<(), RepositoryError> {
        if self.framebuffer_address == 0 {
            return Err(RepositoryError::ValidationError("Framebuffer address cannot be zero"));
        }
        
        if self.stride == 0 {
            return Err(RepositoryError::ValidationError("Framebuffer stride cannot be zero"));
        }
        
        Ok(())
    }
    
    fn entity_type() -> &'static str
    where
        Self: Sized,
    {
        "GraphicsInfo"
    }

    fn entity_type_dyn(&self) -> &'static str {
        Self::entity_type()
    }
}

impl AggregateRoot for MemoryRegion {
    fn clone_aggregate(&self) -> Box<dyn AggregateRoot> {
        Box::new(self.clone())
    }
    
    fn id(&self) -> EntityId {
        EntityId::new(self.id)
    }
    
    fn set_id(&mut self, id: EntityId) {
        self.id = id.value();
    }
    
    fn validate(&self) -> Result<(), RepositoryError> {
        if self.start >= self.end {
            return Err(RepositoryError::ValidationError("Invalid memory region: start address must be less than end address"));
        }
        
        Ok(())
    }
    
    fn entity_type() -> &'static str
    where
        Self: Sized,
    {
        "MemoryRegion"
    }

    fn entity_type_dyn(&self) -> &'static str {
        Self::entity_type()
    }
}

impl fmt::Display for BootConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Boot Configuration:")?;
        writeln!(f, "  Verbosity: {:?}", self.verbosity)?;
        writeln!(f, "  POST: {}", if self.enable_post { "enabled" } else { "disabled" })?;
        writeln!(f, "  Paging: {}", if self.enable_paging { "enabled" } else { "disabled" })?;
        
        if let Some(mode) = self.graphics_mode {
            writeln!(f, "  Graphics Mode: {}", mode)?;
        } else {
            writeln!(f, "  Graphics: disabled")?;
        }

        Ok(())
    }
}

/// Graphics information - Aggregate Root
///
/// Contains information about graphics output configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphicsInfo {
    /// Entity ID
    pub id: u64,
    /// Graphics mode
    pub mode: GraphicsMode,
    /// Framebuffer physical address
    pub framebuffer_address: usize,
    /// Framebuffer size in bytes
    pub framebuffer_size: usize,
    /// Framebuffer stride (bytes per scanline)
    pub stride: u32,
    /// Color mask information
    pub red_mask: u8,
    pub green_mask: u8,
    pub blue_mask: u8,
    pub reserved_mask: u8,
}

impl GraphicsInfo {
    /// Create new graphics information
    pub fn new(
        mode: GraphicsMode,
        framebuffer_address: usize,
        stride: u32,
    ) -> Result<Self, &'static str> {
        if framebuffer_address == 0 {
            return Err("Framebuffer address cannot be zero");
        }
        
        if stride == 0 {
            return Err("Framebuffer stride cannot be zero");
        }
        
        let framebuffer_size = mode.framebuffer_size();
        
        Ok(Self {
            id: 0, // Temporary ID, will be set by repository
            mode,
            framebuffer_address,
            framebuffer_size,
            stride,
            red_mask: 0,
            green_mask: 0,
            blue_mask: 0,
            reserved_mask: 0,
        })
    }
    
    /// Set color mask information
    pub fn with_color_masks(
        mut self,
        red_mask: u8,
        green_mask: u8,
        blue_mask: u8,
        reserved_mask: u8,
    ) -> Self {
        self.red_mask = red_mask;
        self.green_mask = green_mask;
        self.blue_mask = blue_mask;
        self.reserved_mask = reserved_mask;
        self
    }
    
    /// Check if graphics information is valid
    pub fn is_valid(&self) -> bool {
        self.framebuffer_address != 0 && self.stride != 0
    }
    
    /// Get bytes per pixel
    pub fn bytes_per_pixel(&self) -> usize {
        (self.mode.bits_per_pixel as usize + 7) / 8
    }
    
    /// Get total framebuffer size in bytes
    pub fn total_framebuffer_size(&self) -> usize {
        self.framebuffer_size
    }
}

impl fmt::Display for GraphicsInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Graphics Information:")?;
        writeln!(f, "  Mode: {}x{}x{}",
                 self.mode.width, self.mode.height, self.mode.bits_per_pixel)?;
        writeln!(f, "  Framebuffer Address: {:#x}", self.framebuffer_address)?;
        writeln!(f, "  Framebuffer Size: {} bytes", self.framebuffer_size)?;
        writeln!(f, "  Stride: {} bytes", self.stride)?;
        writeln!(f, "  Color Masks: R:{}, G:{}, B:{}, Reserved:{}",
                 self.red_mask, self.green_mask, self.blue_mask, self.reserved_mask)?;
        Ok(())
    }
}

/// Boot phase - Value Object
///
/// Represents current phase of the boot process.
/// This is a value object that defines boot state transitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootPhase {
    /// Initial boot phase
    Initialization,
    /// Hardware detection
    HardwareDetection,
    /// Memory initialization
    MemoryInitialization,
    /// Graphics initialization
    GraphicsInitialization,
    /// Kernel loading
    KernelLoading,
    /// Kernel load complete
    KernelLoadComplete,
    /// Ready to jump to kernel
    ReadyForKernel,
}

impl BootPhase {
    /// Get phase name as string
    pub fn name(&self) -> &'static str {
        match self {
            Self::Initialization => "initialization",
            Self::HardwareDetection => "hardware_detection",
            Self::MemoryInitialization => "memory_initialization",
            Self::GraphicsInitialization => "graphics_initialization",
            Self::KernelLoading => "kernel_loading",
            Self::KernelLoadComplete => "kernel_load_complete",
            Self::ReadyForKernel => "ready_for_kernel",
        }
    }
    
    /// Check if transition to target phase is allowed
    pub fn can_transition_to(&self, target: &BootPhase) -> bool {
        use BootPhase::*;
        
        match (self, target) {
            (Initialization, HardwareDetection) => true,
            (HardwareDetection, MemoryInitialization) => true,
            (MemoryInitialization, GraphicsInitialization) => true,
            (MemoryInitialization, KernelLoading) => true, // Graphics optional
            (GraphicsInitialization, KernelLoading) => true,
            (KernelLoading, KernelLoadComplete) => true,
            (KernelLoadComplete, ReadyForKernel) => true,
            _ => false,
        }
    }
    
    /// Check if boot process is complete
    pub fn is_complete(&self) -> bool {
        matches!(self, Self::ReadyForKernel)
    }
    
    /// Check if graphics is required for this phase
    pub fn requires_graphics(&self) -> bool {
        matches!(self, Self::GraphicsInitialization)
    }
}

impl fmt::Display for BootPhase {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_graphics_mode_validation() {
        assert!(GraphicsMode::new(1024, 768, 32).is_ok());
        assert!(GraphicsMode::new(320, 200, 8).is_ok());
        assert!(GraphicsMode::new(100, 768, 32).is_err());  // Width too small
        assert!(GraphicsMode::new(1024, 768, 24).is_ok());
        assert!(GraphicsMode::new(1024, 768, 15).is_err()); // Invalid bpp
    }

    #[test]
    fn test_graphics_mode_size_calculation() {
        let mode = GraphicsMode::new(1024, 768, 32).unwrap();
        assert_eq!(mode.bits_per_pixel, 32);
        assert!(mode.is_high_resolution());
        assert!(mode.framebuffer_size() > 0);
    }

    #[test]
    fn test_boot_config_validation() {
        let config = BootConfig::new();
        assert!(config.validate().is_ok());
    }
}
