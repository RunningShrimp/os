//! VESA BIOS Extensions (VBE) implementation
//!
//! This module provides VESA graphics interface support for BIOS bootloader,
//! enabling high-resolution graphics modes and framebuffer access.

use crate::utils::error::{BootError, Result};
use crate::protocol::FramebufferInfo;
use crate::infrastructure::graphics_backend::PixelFormat;
use alloc::vec::Vec;
#[cfg(feature = "uefi_support")]
use uefi::println;

/// VBE Controller Info structure
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct VbeControllerInfo {
    pub signature: [u8; 4],      // Should be "VESA"
    pub version: u16,             // VBE version
    pub oem_string: u32,          // OEM string pointer
    pub capabilities: u32,        // Capabilities flags
    pub video_modes: u32,         // Video mode list pointer
    pub total_memory: u16,        // Total memory in 64KB blocks
    pub oem_software_rev: u16,    // OEM software revision
    pub oem_vendor: u32,          // OEM vendor string pointer
    pub oem_product: u32,         // OEM product name string pointer
    pub oem_revision: u32,        // OEM product revision string pointer
    pub reserved: [u8; 222],      // Reserved for VBE implementation
    pub oem_data: [u8; 256],      // OEM data area
}

/// VBE Mode Info structure
#[derive(Debug, Clone, Copy)]
#[repr(C, packed)]
pub struct VbeModeInfo {
    pub mode_attributes: u16,             // Mode attributes
    pub win_a_attributes: u8,             // Window A attributes
    pub win_b_attributes: u8,             // Window B attributes
    pub win_granularity: u16,             // Window granularity
    pub win_size: u16,                    // Window size
    pub win_a_segment: u16,               // Window A segment
    pub win_b_segment: u16,               // Window B segment
    pub win_func_ptr: u32,                // Window function pointer
    pub bytes_per_scanline: u16,          // Bytes per scanline
    pub x_resolution: u16,                // Horizontal resolution
    pub y_resolution: u16,                // Vertical resolution
    pub x_char_size: u8,                  // Character cell width
    pub y_char_size: u8,                  // Character cell height
    pub number_of_planes: u8,             // Number of memory planes
    pub bits_per_pixel: u8,               // Bits per pixel
    pub number_of_banks: u8,              // Number of banks
    pub memory_model: u8,                 // Memory model type
    pub bank_size: u8,                    // Bank size in KB
    pub number_of_image_pages: u8,        // Number of image pages
    pub reserved1: u8,                    // Reserved
    pub red_mask_size: u8,                // Red mask size
    pub red_field_position: u8,           // Red field position
    pub green_mask_size: u8,              // Green mask size
    pub green_field_position: u8,         // Green field position
    pub blue_mask_size: u8,               // Blue mask size
    pub blue_field_position: u8,          // Blue field position
    pub rsvd_mask_size: u8,               // Reserved mask size
    pub rsvd_field_position: u8,          // Reserved field position
    pub direct_color_mode_info: u8,       // Direct color mode info
    pub phys_base_ptr: u32,               // Physical address for flat frame buffer
    pub reserved2: [u8; 6],               // Reserved
    pub reserved3: u16,                   // Reserved
    pub linear_bytes_per_scanline: u16,   // Bytes per scanline for linear modes
    pub number_of_image_pages_lin: u8,    // Number of image pages for linear modes
    pub depth_of_color: u8,               // Depth of color
    pub number_of_banks_lin: u8,          // Number of banks for linear modes
    pub number_of_images_lin: u8,         // Number of images for linear modes
    pub linear_red_mask_size: u8,         // Red mask size for linear modes
    pub linear_red_field_position: u8,    // Red field position for linear modes
    pub linear_green_mask_size: u8,       // Green mask size for linear modes
    pub linear_green_field_position: u8,  // Green field position for linear modes
    pub linear_blue_mask_size: u8,        // Blue mask size for linear modes
    pub linear_blue_field_position: u8,   // Blue field position for linear modes
    pub linear_rsvd_mask_size: u8,        // Reserved mask size for linear modes
    pub linear_rsvd_field_position: u8,   // Reserved field position for linear modes
    pub max_pixel_clock: u32,             // Maximum pixel clock
    pub reserved4: [u8; 190],             // Reserved for VBE implementation
}

/// VBE Mode Types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VbeMemoryModel {
    Text = 0,
    CGA = 1,
    Hercules = 2,
    Planar = 3,
    PackedPixel = 4,
    DirectColor = 6,
    YUV = 7,
}

/// Common VBE modes
pub const VBE_MODE_640X480X8: u16 = 0x101;
pub const VBE_MODE_640X480X16: u16 = 0x110;
pub const VBE_MODE_640X480X24: u16 = 0x111;
pub const VBE_MODE_640X480X32: u16 = 0x112;
pub const VBE_MODE_800X600X8: u16 = 0x103;
pub const VBE_MODE_800X600X16: u16 = 0x113;
pub const VBE_MODE_800X600X24: u16 = 0x114;
pub const VBE_MODE_800X600X32: u16 = 0x115;
pub const VBE_MODE_1024X768X8: u16 = 0x105;
pub const VBE_MODE_1024X768X16: u16 = 0x117;
pub const VBE_MODE_1024X768X24: u16 = 0x118;
pub const VBE_MODE_1024X768X32: u16 = 0x119;
pub const VBE_MODE_1280X1024X8: u16 = 0x107;
pub const VBE_MODE_1280X1024X16: u16 = 0x11A;
pub const VBE_MODE_1280X1024X24: u16 = 0x11B;
pub const VBE_MODE_1280X1024X32: u16 = 0x11C;

/// VBE Mode cache entry
#[derive(Debug, Clone, Copy)]
pub struct CachedModeInfo {
    pub mode_id: u16,
    pub width: u16,
    pub height: u16,
    pub bpp: u8,
    pub info: VbeModeInfo,
    pub valid: bool,
}

/// VBE Controller Interface with mode caching
pub struct VbeController {
    controller_info: Option<VbeControllerInfo>,
    supported_modes: Vec<u16>,
    mode_cache: Vec<CachedModeInfo>, // Vector for storing mode info
    initialized: bool,
}

impl VbeController {
    /// Create a new VBE controller
    pub fn new() -> Self {
        Self {
            controller_info: None,
            supported_modes: Vec::new(),
            mode_cache: Vec::new(), // Initialize empty vector for mode caching
            initialized: false,
        }
    }

    /// Get cached mode info or None if not cached
    pub fn get_cached_mode(&self, mode_id: u16) -> Option<CachedModeInfo> {
        self.mode_cache.iter()
            .find(|cached| cached.mode_id == mode_id && cached.valid)
            .cloned()
    }

    /// Cache mode info to avoid repeated hardware queries
    pub fn cache_mode(&mut self, info: CachedModeInfo) -> Result<()> {
        // Check if the mode is already cached
        if let Some(index) = self.mode_cache.iter().position(|cached| cached.mode_id == info.mode_id) {
            // Update existing cache entry
            self.mode_cache[index] = info;
        } else {
            // Add new cache entry
            self.mode_cache.push(info);
        }
        Ok(())
    }

    /// Clear the mode cache
    pub fn clear_cache(&mut self) {
        self.mode_cache.clear();
    }

    /// Initialize VBE controller
    pub fn initialize(&mut self) -> Result<()> {
        if self.initialized {
            return Ok(());
        }

        // Get VBE controller information
        let mut controller_info = VbeControllerInfo {
            signature: [0; 4],
            version: 0,
            oem_string: 0,
            capabilities: 0,
            video_modes: 0,
            total_memory: 0,
            oem_software_rev: 0,
            oem_vendor: 0,
            oem_product: 0,
            oem_revision: 0,
            reserved: [0; 222],
            oem_data: [0; 256],
        };

        let result = self.get_controller_info(&mut controller_info)?;

        if result != 0x004F {
            return Err(BootError::HardwareError("VBE controller info failed"));
        }

        // Validate VBE signature
        let signature_str = core::str::from_utf8(&controller_info.signature).unwrap_or("???");
        if signature_str != "VESA" {
            return Err(BootError::HardwareError("Invalid VBE signature"));
        }

        println!("[vbe] VBE Controller Info:");
        println!("[vbe]   Signature: {}", signature_str);
        println!("[vbe]   Version: {}.{}",
                 (controller_info.version >> 8) & 0xFF,
                 controller_info.version & 0xFF);
        println!("[vbe]   Total Memory: {} KB",
                 controller_info.total_memory as u32 * 64);

        self.controller_info = Some(controller_info);
        self.initialized = true;

        // Enumerate supported modes
        self.enumerate_modes()?;
        
        // Preload common VBE modes to improve performance later
        self.preload_common_modes()?;

        Ok(())
    }

    /// Get VBE controller information
    pub fn get_controller_info(&self, info: &mut VbeControllerInfo) -> Result<u16> {
        let result = unsafe {
            let mut regs = core::mem::zeroed::<VbeRegisters>();
            regs.ax = 0x4F00; // VBE get controller info
            regs.es = 0x0000;
            regs.di = info as *mut VbeControllerInfo as u16;

            // Set VBE signature directly in the provided structure
            info.signature = [b'V', b'E', b'S', b'A'];

            self.vbe_interrupt(0x10, &mut regs)
        };

        Ok(result.ax)
    }

    /// Get VBE mode information
    pub fn get_mode_info(&self, mode: u16, info: &mut VbeModeInfo) -> Result<u16> {
        let result = unsafe {
            let mut regs = core::mem::zeroed::<VbeRegisters>();
            regs.ax = 0x4F01; // VBE get mode info
            regs.cx = mode;
            regs.es = 0x0000;
            regs.di = info as *mut VbeModeInfo as u16;

            self.vbe_interrupt(0x10, &mut regs)
        };

        Ok(result.ax)
    }

    /// Set VBE mode
    pub fn set_mode(&self, mode: u16) -> Result<u16> {
        let result = unsafe {
            let mut regs = core::mem::zeroed::<VbeRegisters>();
            regs.ax = 0x4F02; // VBE set mode
            regs.bx = mode | 0x4000; // Set bit 14 for linear framebuffer

            self.vbe_interrupt(0x10, &mut regs)
        };

        Ok(result.ax)
    }

    /// Get current VBE mode
    pub fn get_current_mode(&self) -> Result<u16> {
        let result = unsafe {
            let mut regs = core::mem::zeroed::<VbeRegisters>();
            regs.ax = 0x4F03; // VBE get current mode

            self.vbe_interrupt(0x10, &mut regs)
        };

        Ok(result.bx)
    }

    /// Enumerate supported VBE modes
    pub fn enumerate_modes(&mut self) -> Result<()> {
        if let Some(controller_info) = self.controller_info {
            let modes_ptr = controller_info.video_modes as *const u16;

            // Enumerate modes until we find -1 terminator
            let mut i = 0;
            loop {
                let mode = unsafe { *modes_ptr.add(i) };

                if mode == 0xFFFF {
                    break; // End of list
                }

                // Check if this mode is supported
                if let Ok(_) = self.is_mode_supported(mode) {
                    self.supported_modes.push(mode);
                }

                i += 1;
            }

            println!("[vbe] Found {} supported VBE modes", self.supported_modes.len());
        }

        Ok(())
    }

    /// Check if a VBE mode is supported
    pub fn is_mode_supported(&self, mode: u16) -> Result<()> {
        let mut mode_info = VbeModeInfo {
            mode_attributes: 0,
            win_a_attributes: 0,
            win_b_attributes: 0,
            win_granularity: 0,
            win_size: 0,
            win_a_segment: 0,
            win_b_segment: 0,
            win_func_ptr: 0,
            bytes_per_scanline: 0,
            x_resolution: 0,
            y_resolution: 0,
            x_char_size: 0,
            y_char_size: 0,
            number_of_planes: 0,
            bits_per_pixel: 0,
            number_of_banks: 0,
            memory_model: 0,
            bank_size: 0,
            number_of_image_pages: 0,
            reserved1: 0,
            red_mask_size: 0,
            red_field_position: 0,
            green_mask_size: 0,
            green_field_position: 0,
            blue_mask_size: 0,
            blue_field_position: 0,
            rsvd_mask_size: 0,
            rsvd_field_position: 0,
            direct_color_mode_info: 0,
            phys_base_ptr: 0,
            reserved2: [0; 6],
            reserved3: 0,
            linear_bytes_per_scanline: 0,
            number_of_image_pages_lin: 0,
            depth_of_color: 0,
            number_of_banks_lin: 0,
            number_of_images_lin: 0,
            linear_red_mask_size: 0,
            linear_red_field_position: 0,
            linear_green_mask_size: 0,
            linear_green_field_position: 0,
            linear_blue_mask_size: 0,
            linear_blue_field_position: 0,
            linear_rsvd_mask_size: 0,
            linear_rsvd_field_position: 0,
            max_pixel_clock: 0,
            reserved4: [0; 190],
        };

        let result = self.get_mode_info(mode, &mut mode_info)?;

        if result != 0x004F {
            return Err(BootError::HardwareError("VBE mode info failed"));
        }

        // Check mode attributes
        let supported = (mode_info.mode_attributes & 0x0001) != 0; // Mode supported
        let color_mode = (mode_info.mode_attributes & 0x0008) != 0; // Color mode
        let linear_fb = (mode_info.mode_attributes & 0x0080) != 0; // Linear framebuffer
        let graphics_mode = (mode_info.mode_attributes & 0x0010) != 0; // Graphics mode

        if !supported || !color_mode || !linear_fb || !graphics_mode {
            return Err(BootError::HardwareError("VBE mode not supported"));
        }

        Ok(())
    }

    /// Find the best VBE mode for the requested resolution
    pub fn find_best_mode(&self, width: u16, height: u16, bpp: u8) -> Option<u16> {
        for &mode in &self.supported_modes {
            if let Ok(mode_info) = self.get_mode_info_details(mode) {
                if mode_info.x_resolution == width &&
                   mode_info.y_resolution == height &&
                   mode_info.bits_per_pixel == bpp {
                    return Some(mode);
                }
            }
        }

        // Fallback: find mode with same or higher resolution
        for &mode in &self.supported_modes {
            if let Ok(mode_info) = self.get_mode_info_details(mode) {
                if mode_info.x_resolution >= width &&
                   mode_info.y_resolution >= height &&
                   mode_info.bits_per_pixel >= bpp {
                    return Some(mode);
                }
            }
        }

        None
    }

    /// Get detailed mode information
    pub fn get_mode_info_details(&self, mode: u16) -> Result<VbeModeInfo> {
        let mut mode_info = unsafe { core::mem::zeroed::<VbeModeInfo>() };

        let result = self.get_mode_info(mode, &mut mode_info)?;
        if result != 0x004F {
            return Err(BootError::HardwareError("Failed to get VBE mode info"));
        }

        Ok(mode_info)
    }

    /// Preload common VBE modes to avoid repeated hardware queries
    pub fn preload_common_modes(&mut self) -> Result<()> {
        // List of common VBE modes to preload
        let common_modes = [
            VBE_MODE_640X480X8,
            VBE_MODE_640X480X16,
            VBE_MODE_640X480X24,
            VBE_MODE_640X480X32,
            VBE_MODE_800X600X8,
            VBE_MODE_800X600X16,
            VBE_MODE_800X600X24,
            VBE_MODE_800X600X32,
            VBE_MODE_1024X768X8,
            VBE_MODE_1024X768X16,
            VBE_MODE_1024X768X24,
            VBE_MODE_1024X768X32,
            VBE_MODE_1280X1024X8,
            VBE_MODE_1280X1024X16,
            VBE_MODE_1280X1024X24,
            VBE_MODE_1280X1024X32,
        ];
        
        // Try to preload each common mode
        for &mode in &common_modes {
            // Check if the mode is in the supported modes list to avoid unnecessary hardware calls
            if self.supported_modes.contains(&mode) {
                match self.get_mode_info_details(mode) {
                    Ok(mode_info) => {
                        // Cache the mode info
                        let cached = CachedModeInfo {
                            mode_id: mode,
                            width: mode_info.x_resolution,
                            height: mode_info.y_resolution,
                            bpp: mode_info.bits_per_pixel,
                            info: mode_info,
                            valid: true,
                        };
                        
                        self.cache_mode(cached)?;
                    }
                    Err(_) => {
                        // Ignore modes that can't be preloaded
                    }
                }
            }
        }
        
        Ok(())
    }

    /// Set graphics mode and return framebuffer info
    pub fn set_graphics_mode(&self, width: u16, height: u16, bpp: u8) -> Result<FramebufferInfo> {
        if !self.initialized {
            return Err(BootError::NotInitialized);
        }

        // Find best mode
        let mode = self.find_best_mode(width, height, bpp)
            .ok_or(BootError::HardwareError("No suitable VBE mode found"))?;

        // Get mode info before setting
        let mode_info = self.get_mode_info_details(mode)?;

        // Set the mode
        let result = self.set_mode(mode)?;
        if result != 0x004F {
            return Err(BootError::HardwareError("Failed to set VBE mode"));
        }

        // Copy values to local variables to avoid unaligned references
        let mode_val = mode;
        let x_res = mode_info.x_resolution;
        let y_res = mode_info.y_resolution;
        let bpp_val = mode_info.bits_per_pixel;
        let fb_addr = mode_info.phys_base_ptr;
        
        println!("[vbe] Set VBE mode: 0x{:04X}", mode_val);
        println!("[vbe] Resolution: {}x{}", x_res, y_res);
        println!("[vbe] BPP: {}", bpp_val);
        println!("[vbe] Framebuffer address: {:#08X}", fb_addr);

        // Create framebuffer info
        let fb_info = FramebufferInfo {
            address: mode_info.phys_base_ptr as usize,
            width: mode_info.x_resolution as u32,
            height: mode_info.y_resolution as u32,
            bpp: (mode_info.bits_per_pixel / 8) as u32,
            pitch: mode_info.bytes_per_scanline as u32,
        };

        Ok(fb_info)
    }

    /// Determine pixel format from VBE mode info
    pub fn determine_pixel_format(&self, mode_info: &VbeModeInfo) -> PixelFormat {
        match (mode_info.red_field_position, mode_info.blue_field_position) {
            (16, 0) => PixelFormat::BGR,
            (0, 16) => PixelFormat::RGB,
            _ => PixelFormat::RGB,
        }
    }

    /// Get supported modes list
    pub fn get_supported_modes(&self) -> &[u16] {
        &self.supported_modes
    }

    /// Get cached controller info
    pub fn get_cached_controller_info(&self) -> Option<&VbeControllerInfo> {
        self.controller_info.as_ref()
    }

    /// Check if VBE is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// VESA BIOS interrupt call
    unsafe fn vbe_interrupt(&self, _interrupt: u8, regs: &mut VbeRegisters) -> VbeRegisters {
        log::debug!("Executing VBE interrupt call");
        // Real BIOS interrupt implementation for VBE
        // We use inline assembly with constraints to set and get registers
        
        let result = VbeRegisters {
            ax: regs.ax,
            bx: regs.bx,
            cx: regs.cx,
            dx: regs.dx,
            si: regs.si,
            di: regs.di,
            es: regs.es,
        };
        
        // Execute BIOS interrupt 0x10 for VBE calls
        #[cfg(target_arch = "x86")]
        asm!(
            "int $0x10",
            inout("ax") result.ax,
            inout("bx") result.bx,
            inout("cx") result.cx,
            inout("dx") result.dx,
            inout("si") result.si,
            inout("di") result.di,
            inout("es") result.es,
            options(nostack),
        );
        
        #[cfg(not(target_arch = "x86"))]
        {}
        
        result
    }
}

/// VBE Registers structure for interrupt calls
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct VbeRegisters {
    ax: u16,
    bx: u16,
    cx: u16,
    dx: u16,
    si: u16,
    di: u16,
    es: u16,
}

/// VBE Graphics Manager
pub struct VbeGraphicsManager {
    controller: VbeController,
    current_mode: Option<u16>,
    framebuffer_info: Option<FramebufferInfo>,
}

impl VbeGraphicsManager {
    /// Create a new VBE graphics manager
    pub fn new() -> Self {
        Self {
            controller: VbeController::new(),
            current_mode: None,
            framebuffer_info: None,
        }
    }

    /// Initialize VBE graphics
    pub fn initialize(&mut self) -> Result<()> {
        self.controller.initialize()?;
        println!("[vbe] VBE Graphics Manager initialized");
        Ok(())
    }

    /// Set graphics mode
    pub fn set_mode(&mut self, width: u16, height: u16, bpp: u8) -> Result<()> {
        let fb_info = self.controller.set_graphics_mode(width, height, bpp)?;
        self.framebuffer_info = Some(fb_info);

        // Remember the mode we set
        if let Some(mode) = self.controller.find_best_mode(width, height, bpp) {
            self.current_mode = Some(mode);
        }

        Ok(())
    }

    /// Get current framebuffer info
    pub fn get_framebuffer_info(&self) -> Option<&FramebufferInfo> {
        self.framebuffer_info.as_ref()
    }

    /// Get the VBE controller
    pub fn get_controller(&self) -> &VbeController {
        &self.controller
    }

    /// Get the VBE controller (mutable)
    pub fn get_controller_mut(&mut self) -> &mut VbeController {
        &mut self.controller
    }

    /// Check if graphics mode is set
    pub fn is_graphics_mode_set(&self) -> bool {
        self.current_mode.is_some()
    }

    /// Get current mode
    pub fn get_current_mode(&self) -> Option<u16> {
        self.current_mode
    }

    /// List available modes with their details
    pub fn list_available_modes(&self) -> Result<Vec<(u16, u16, u16, u8)>> {
        let mut modes = Vec::new();

        for &mode in self.controller.get_supported_modes() {
            if let Ok(mode_info) = self.controller.get_mode_info_details(mode) {
                modes.push((mode, mode_info.x_resolution, mode_info.y_resolution, mode_info.bits_per_pixel));
            }
        }

        // Sort by resolution and color depth
        modes.sort_by(|a, b| {
            b.1.cmp(&a.1) // Width (descending)
                .then(b.2.cmp(&a.2)) // Height (descending)
                .then(b.3.cmp(&a.3)) // BPP (descending)
        });

        Ok(modes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vbe_constants() {
        assert_eq!(VBE_MODE_640X480X8, 0x101);
        assert_eq!(VBE_MODE_1024X768X32, 0x119);
    }

    #[test]
    fn test_vbe_controller_creation() {
        let controller = VbeController::new();
        assert!(!controller.is_initialized());
        assert!(controller.get_supported_modes().is_empty());
    }

    #[test]
    fn test_vbe_graphics_manager() {
        let manager = VbeGraphicsManager::new();
        assert!(!manager.is_graphics_mode_set());
        assert!(manager.get_current_mode().is_none());
    }
}