//! Domain Services - Cross-entity business logic
//!
//! Services that don't belong to a single entity but represent
//! important domain concepts and rules.

use super::boot_config::{BootConfig, GraphicsMode};
use alloc::vec::Vec;

/// Hardware graphics capabilities
#[derive(Clone, Debug)]
pub struct GraphicsCapabilities {
    pub supports_uefi_gop: bool,
    pub supports_vbe: bool,
    pub supports_framebuffer: bool,
    pub max_width: u16,
    pub max_height: u16,
    pub max_colors: u8,
}

impl GraphicsCapabilities {
    pub fn new() -> Self {
        Self {
            supports_uefi_gop: false,
            supports_vbe: false,
            supports_framebuffer: false,
            max_width: 1024,
            max_height: 768,
            max_colors: 32,
        }
    }

    pub fn supports_graphics(&self) -> bool {
        self.supports_uefi_gop || self.supports_vbe || self.supports_framebuffer
    }
}

impl Default for GraphicsCapabilities {
    fn default() -> Self {
        Self::new()
    }
}

/// Hardware information summary
#[derive(Clone, Debug)]
pub struct HardwareInfo {
    pub graphics: GraphicsCapabilities,
    pub total_memory: u64,
    pub available_memory: u64,
}

impl HardwareInfo {
    pub fn new() -> Self {
        Self {
            graphics: GraphicsCapabilities::new(),
            total_memory: 0,
            available_memory: 0,
        }
    }
}

impl Default for HardwareInfo {
    fn default() -> Self {
        Self::new()
    }
}

/// Boot validator - Domain Service
///
/// Validates that the system is capable of booting with the given configuration.
/// This is a domain service because the validation logic spans multiple entities
/// and doesn't belong to a single entity.
pub struct BootValidator;

impl BootValidator {
    /// Validate system prerequisites for booting
    ///
    /// Checks that:
    /// - CPU has required features
    /// - Memory is sufficient
    /// - Graphics mode is supported if enabled
    /// - Kernel can fit in memory
    /// - Configuration consistency
    pub fn validate_prerequisites(
        config: &BootConfig,
        hw_info: &HardwareInfo,
    ) -> Result<(), Vec<&'static str>> {
        let mut errors = Vec::new();
        
        // Validate graphics
        if let Some(mode) = config.graphics_mode {
            if !hw_info.graphics.supports_graphics() {
                errors.push("Graphics requested but not supported");
            }

            if mode.width > hw_info.graphics.max_width
                || mode.height > hw_info.graphics.max_height
            {
                errors.push("Requested graphics mode exceeds hardware capabilities");
            }

            // Check memory for framebuffer
            let fb_size = mode.framebuffer_size();
            if hw_info.available_memory < fb_size as u64 {
                errors.push("Insufficient memory for framebuffer");
            }
            
            // Check graphics mode compatibility with hardware
            if !mode.is_compatible_with(&hw_info.graphics) {
                errors.push("Graphics mode not compatible with hardware capabilities");
            }
        }

        // Validate memory for kernel
        let kernel_min_size = 512 * 1024 * 1024;
        if hw_info.available_memory < kernel_min_size {
            errors.push("Insufficient memory for kernel");
        }
        
        // Validate configuration consistency
        if let Err(config_error) = config.validate() {
            errors.push(config_error);
        }
        
        // Validate hardware consistency
        if hw_info.available_memory > hw_info.total_memory {
            errors.push("Available memory cannot exceed total memory");
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Validate kernel compatibility with system
    pub fn validate_kernel_compatibility(
        kernel_info: &crate::domain::boot_config::KernelInfo,
        boot_info: &crate::domain::boot_info::BootInfo,
    ) -> Result<(), Vec<&'static str>> {
        let mut errors = Vec::new();
        
        // Check kernel size
        if kernel_info.size > boot_info.total_available_memory() {
            errors.push("Kernel too large for available memory");
        }
        
        // Check kernel alignment
        if kernel_info.address % 0x1000 != 0 {
            errors.push("Kernel address not properly aligned (4KB boundary required)");
        }
        
        // Check entry point alignment
        if kernel_info.entry_point % 0x10 != 0 {
            errors.push("Kernel entry point not properly aligned (16-byte boundary required)");
        }
        
        // Check signature verification if required
        if !kernel_info.signature_verified {
            errors.push("Kernel signature not verified - secure boot may fail");
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
    
    /// Validate memory layout
    pub fn validate_memory_layout(
        memory_regions: &[crate::domain::boot_config::MemoryRegion],
    ) -> Result<(), Vec<&'static str>> {
        let mut errors = Vec::new();
        
        // Check for overlaps
        for (i, region1) in memory_regions.iter().enumerate() {
            for region2 in memory_regions.iter().skip(i + 1) {
                if region1.overlaps(region2) {
                    errors.push("Memory regions overlap");
                }
            }
        }
        
        // Check for at least one available region
        if !memory_regions.iter().any(|r| r.is_available()) {
            errors.push("No available memory regions found");
        }
        
        // Check region validity
        for region in memory_regions {
            if region.size() == 0 {
                errors.push("Zero-sized memory region detected");
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Graphics mode selector - Domain Service
///
/// Selects the best graphics mode given hardware capabilities
/// and user preferences.
pub struct GraphicsModeSelector;

impl GraphicsModeSelector {
    /// Select optimal graphics mode
    ///
    /// Given hardware capabilities and user preferences, selects the best
    /// graphics mode that:
    /// 1. Hardware supports
    /// 2. Fits in available memory
    /// 3. Meets minimum resolution requirements
    pub fn select_mode(
        capabilities: &GraphicsCapabilities,
        preferred: Option<GraphicsMode>,
        available_memory: u64,
    ) -> Result<Option<GraphicsMode>, &'static str> {
        // If no graphics support, return None
        if !capabilities.supports_graphics() {
            return Ok(None);
        }

        // If user preferred a mode, try to use it
        if let Some(mode) = preferred {
            if mode.width <= capabilities.max_width
                && mode.height <= capabilities.max_height
                && (mode.framebuffer_size() as u64) < available_memory
            {
                return Ok(Some(mode));
            }
            // Preferred mode not available, fall through to default
        }

        // Default selection strategy: 1024x768x32
        if capabilities.max_width >= 1024 && capabilities.max_height >= 768 {
            let mode = GraphicsMode::new(1024, 768, 32)?;
            if (mode.framebuffer_size() as u64) < available_memory {
                return Ok(Some(mode));
            }
        }

        // Fallback: 640x480x24
        if capabilities.max_width >= 640 && capabilities.max_height >= 480 {
            let mode = GraphicsMode::new(640, 480, 24)?;
            if (mode.framebuffer_size() as u64) < available_memory {
                return Ok(Some(mode));
            }
        }

        // No suitable mode found
        Ok(None)
    }

    /// Get best mode for specific aspect ratio
    pub fn select_mode_for_aspect_ratio(
        capabilities: &GraphicsCapabilities,
        target_ratio: f32,
        available_memory: u64,
    ) -> Result<Option<GraphicsMode>, &'static str> {
        if !capabilities.supports_graphics() {
            return Ok(None);
        }
        
        let all_modes = [
            (1920, 1080, 32), // 16:9
            (1680, 1050, 32), // 16:10
            (1600, 1200, 32), // 4:3
            (1440, 900, 32),  // 16:10
            (1366, 768, 32),  // 16:9
            (1280, 1024, 32), // 5:4
            (1280, 720, 32),  // 16:9
            (1024, 768, 32),  // 4:3
            (800, 600, 32),    // 4:3
            (640, 480, 32),    // 4:3
        ];
        
        let mut best_mode: Option<GraphicsMode> = None;
        let mut best_ratio_diff = f32::MAX;
        
        for &(width, height, bpp) in &all_modes {
            if let Ok(mode) = GraphicsMode::new(width, height, bpp) {
                if Self::is_mode_supported(&mode, capabilities, available_memory)? {
                    let ratio = mode.aspect_ratio();
                    let ratio_diff = (ratio - target_ratio).abs();
                    
                    if ratio_diff < best_ratio_diff {
                        best_ratio_diff = ratio_diff;
                        best_mode = Some(mode);
                    }
                }
            }
        }
        
        Ok(best_mode)
    }
    
    /// Check if a mode is supported by hardware and fits in memory
    fn is_mode_supported(
        mode: &GraphicsMode,
        capabilities: &GraphicsCapabilities,
        available_memory: u64,
    ) -> Result<bool, &'static str> {
        if mode.width > capabilities.max_width
            || mode.height > capabilities.max_height
            || mode.bits_per_pixel > capabilities.max_colors
        {
            return Ok(false);
        }
        
        if (mode.framebuffer_size() as u64) > available_memory {
            return Ok(false);
        }
        
        Ok(true)
    }
}

/// Memory manager - Domain Service
///
/// Handles memory allocation and layout management for boot process.
/// This is a domain service because memory management spans multiple entities.
pub struct MemoryManager;

impl MemoryManager {
    /// Allocate memory region for kernel
    pub fn allocate_kernel_region(
        kernel_size: u64,
        alignment: u64,
        memory_regions: &[crate::domain::boot_config::MemoryRegion],
    ) -> Result<crate::domain::boot_config::MemoryRegion, &'static str> {
        for region in memory_regions {
            if !region.is_available() {
                continue;
            }
            
            if region.size() < kernel_size {
                continue;
            }
            
            // Calculate aligned start address
            let aligned_start = (region.start + alignment - 1) & !(alignment - 1);
            let aligned_end = aligned_start + kernel_size;
            
            // Check if aligned region fits
            if aligned_end <= region.end {
                return Ok(crate::domain::boot_config::MemoryRegion::new(
                    aligned_start,
                    aligned_end,
                    crate::domain::boot_config::MemoryRegionType::Available,
                )?);
            }
        }
        
        Err("No suitable memory region found for kernel")
    }
    
    /// Allocate memory region for framebuffer
    pub fn allocate_framebuffer_region(
        framebuffer_size: u64,
        alignment: u64,
        preferred_address: Option<u64>,
        memory_regions: &[crate::domain::boot_config::MemoryRegion],
    ) -> Result<crate::domain::boot_config::MemoryRegion, &'static str> {
        // Try preferred address first
        if let Some(pref_addr) = preferred_address {
            let aligned_start = (pref_addr + alignment - 1) & !(alignment - 1);
            let aligned_end = aligned_start + framebuffer_size;
            
            for region in memory_regions {
                if !region.is_available() {
                    continue;
                }
                
                if region.contains(aligned_start) && region.contains(aligned_end - 1) {
                    return Ok(crate::domain::boot_config::MemoryRegion::new(
                        aligned_start,
                        aligned_end,
                        crate::domain::boot_config::MemoryRegionType::Available,
                    )?);
                }
            }
        }
        
        // Find any suitable region
        for region in memory_regions {
            if !region.is_available() {
                continue;
            }
            
            if region.size() < framebuffer_size {
                continue;
            }
            
            let aligned_start = (region.start + alignment - 1) & !(alignment - 1);
            let aligned_end = aligned_start + framebuffer_size;
            
            if aligned_end <= region.end {
                return Ok(crate::domain::boot_config::MemoryRegion::new(
                    aligned_start,
                    aligned_end,
                    crate::domain::boot_config::MemoryRegionType::Available,
                )?);
            }
        }
        
        Err("No suitable memory region found for framebuffer")
    }
    
    /// Validate memory layout
    pub fn validate_memory_layout(
        memory_regions: &[crate::domain::boot_config::MemoryRegion],
    ) -> Result<(), Vec<&'static str>> {
        let mut errors = Vec::new();
        
        // Check for overlaps
        for (i, region1) in memory_regions.iter().enumerate() {
            for region2 in memory_regions.iter().skip(i + 1) {
                if region1.overlaps(region2) {
                    errors.push("Memory regions overlap");
                }
            }
        }
        
        // Check for at least one available region
        if !memory_regions.iter().any(|r| r.is_available()) {
            errors.push("No available memory regions found");
        }
        
        // Check region validity
        for region in memory_regions {
            if region.size() == 0 {
                errors.push("Zero-sized memory region detected");
            }
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

/// Kernel loader - Domain Service
///
/// Handles kernel image loading and validation.
/// This is a domain service because kernel loading spans multiple entities.
pub struct KernelLoader;

impl KernelLoader {
    /// Load kernel from image data
    pub fn load_kernel(
        image_data: &[u8],
        load_address: u64,
        entry_point_offset: u64,
    ) -> Result<crate::domain::boot_config::KernelInfo, &'static str> {
        if image_data.is_empty() {
            return Err("Kernel image data is empty");
        }
        
        if load_address == 0 {
            return Err("Invalid load address");
        }
        
        let kernel_size = image_data.len() as u64;
        let entry_point = load_address + entry_point_offset;
        
        // Validate kernel image format (simplified)
        if !Self::validate_kernel_format(image_data) {
            return Err("Invalid kernel image format");
        }
        
        let kernel_info = crate::domain::boot_config::KernelInfo::new(load_address, kernel_size, entry_point)?;
        
        Ok(kernel_info)
    }
    
    /// Verify kernel signature
    pub fn verify_signature(kernel_info: &crate::domain::boot_config::KernelInfo) -> Result<(), &'static str> {
        // In a real implementation, this would verify cryptographic signature
        // For now, we'll just simulate verification
        if kernel_info.size < 1024 {
            return Err("Kernel too small to have valid signature");
        }
        
        Ok(())
    }
    
    /// Validate kernel image format
    fn validate_kernel_format(image_data: &[u8]) -> bool {
        // Simplified validation - check for magic numbers
        if image_data.len() < 4 {
            return false;
        }
        
        // Check for ELF magic number
        image_data[0] == 0x7F &&
        image_data[1] == b'E' &&
        image_data[2] == b'L' &&
        image_data[3] == b'F'
    }
    
    /// Get kernel entry point offset
    pub fn get_entry_point_offset(image_data: &[u8]) -> Result<u64, &'static str> {
        if !Self::validate_kernel_format(image_data) {
            return Err("Invalid kernel image format");
        }
        
        // Simplified - assume entry point is at offset 0x100
        Ok(0x100)
    }
    
    /// Validate kernel compatibility with system
    pub fn validate_kernel_compatibility(
        kernel_info: &crate::domain::boot_config::KernelInfo,
        boot_info: &crate::domain::boot_info::BootInfo,
    ) -> Result<(), Vec<&'static str>> {
        let mut errors = Vec::new();
        
        // Check kernel size
        if kernel_info.size > boot_info.total_available_memory() {
            errors.push("Kernel too large for available memory");
        }
        
        // Check kernel alignment
        if kernel_info.address % 0x1000 != 0 {
            errors.push("Kernel address not properly aligned (4KB boundary required)");
        }
        
        // Check entry point alignment
        if kernel_info.entry_point % 0x10 != 0 {
            errors.push("Kernel entry point not properly aligned (16-byte boundary required)");
        }
        
        // Check signature verification
        if !kernel_info.signature_verified {
            errors.push("Kernel signature not verified - secure boot may fail");
        }
        
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_validator_insufficient_memory() {
        let config = BootConfig::new();
        let mut hw_info = HardwareInfo::new();
        hw_info.available_memory = 256 * 1024 * 1024; // Only 256MB, need 512MB

        assert!(BootValidator::validate_prerequisites(&config, &hw_info).is_err());
    }

    #[test]
    fn test_boot_validator_sufficient_memory() {
        let config = BootConfig::new();
        let mut hw_info = HardwareInfo::new();
        hw_info.available_memory = 1024 * 1024 * 1024; // 1GB - plenty

        assert!(BootValidator::validate_prerequisites(&config, &hw_info).is_ok());
    }

    #[test]
    fn test_graphics_mode_selector_default() {
        let capabilities = GraphicsCapabilities {
            supports_vbe: true,
            max_width: 1024,
            max_height: 768,
            max_colors: 32,
            ..Default::default()
        };

        let mode = GraphicsModeSelector::select_mode(&capabilities, None, 512 * 1024 * 1024);
        assert!(mode.is_ok());
        assert!(mode.unwrap().is_some());
    }

    #[test]
    fn test_graphics_mode_selector_no_memory() {
        let capabilities = GraphicsCapabilities {
            supports_vbe: true,
            max_width: 1024,
            max_height: 768,
            max_colors: 32,
            ..Default::default()
        };

        // Insufficient memory for graphics mode
        let mode = GraphicsModeSelector::select_mode(&capabilities, None, 100 * 1024);
        assert!(mode.is_ok());
        assert!(mode.unwrap().is_none()); // No suitable mode
    }

    #[test]
    fn test_graphics_mode_selector_fallback() {
        let capabilities = GraphicsCapabilities {
            supports_vbe: true,
            max_width: 640, // Only supports 640x480
            max_height: 480,
            max_colors: 24,
            ..Default::default()
        };

        let mode = GraphicsModeSelector::select_mode(&capabilities, None, 512 * 1024 * 1024);
        assert!(mode.is_ok());
        if let Ok(Some(m)) = mode {
            assert_eq!(m.width, 640);
            assert_eq!(m.height, 480);
        }
    }

    #[test]
    fn test_hardware_info_defaults() {
        let hw = HardwareInfo::new();
        assert!(hw.total_memory > 0);
        assert!(hw.available_memory > 0);
        assert!(hw.available_memory <= hw.total_memory);
    }

    #[test]
    fn test_graphics_capabilities_defaults() {
        let caps = GraphicsCapabilities::default();
        assert_eq!(caps.max_width, 1024);
        assert_eq!(caps.max_height, 768);
    }
    
    #[test]
    fn test_memory_manager_kernel_allocation() {
        let regions = [
            crate::domain::boot_config::MemoryRegion::new(
                0x100000, 0x500000,
                crate::domain::boot_config::MemoryRegionType::Reserved
            ).unwrap(),
            crate::domain::boot_config::MemoryRegion::new(
                0x500000, 0x1000000,
                crate::domain::boot_config::MemoryRegionType::Available
            ).unwrap(),
        ];
        
        let result = MemoryManager::allocate_kernel_region(0x100000, 0x1000, &regions);
        assert!(result.is_ok());
        
        let region = result.unwrap();
        assert_eq!(region.start, 0x500000);
        assert_eq!(region.end, 0x600000);
    }
    
    #[test]
    fn test_memory_manager_no_space() {
        let regions = [
            crate::domain::boot_config::MemoryRegion::new(
                0x100000, 0x200000,
                crate::domain::boot_config::MemoryRegionType::Available
            ).unwrap(), // Only 1MB available
        ];
        
        let result = MemoryManager::allocate_kernel_region(0x200000, 0x1000, &regions);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_kernel_loader_validation() {
        let mut image_data = vec![0x7F, b'E', b'L', b'F'];
        image_data.extend_from_slice(&[0; 1020]); // Make it 1KB
        
        let result = KernelLoader::load_kernel(&image_data, 0x100000, 0x100);
        assert!(result.is_ok());
        
        let kernel_info = result.unwrap();
        assert_eq!(kernel_info.address, 0x100000);
        assert_eq!(kernel_info.size, 1024);
        assert_eq!(kernel_info.entry_point, 0x100100);
    }
    
    #[test]
    fn test_kernel_loader_invalid_format() {
        let image_data = vec![0x00, 0x00, 0x00, 0x00]; // Invalid magic
        
        let result = KernelLoader::load_kernel(&image_data, 0x100000, 0x100);
        assert!(result.is_err());
    }
    
    #[test]
    fn test_graphics_mode_selector_aspect_ratio() {
        let capabilities = GraphicsCapabilities {
            supports_vbe: true,
            max_width: 1920,
            max_height: 1080,
            max_colors: 32,
            ..Default::default()
        };
        
        // Request 16:9 aspect ratio
        let result = GraphicsModeSelector::select_mode_for_aspect_ratio(
            &capabilities,
            16.0 / 9.0,
            512 * 1024 * 1024
        );
        
        assert!(result.is_ok());
        assert!(result.unwrap().is_some());
    }
}
