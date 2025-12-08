//! Graphics output support for the bootloader
//!
//! This module provides graphics functionality including framebuffer
//! management, basic drawing operations, and font rendering.

use crate::error::{BootError, Result};
use crate::protocol::FramebufferInfo;
use core::ptr;

#[cfg(feature = "vbe_support")]
pub mod vbe;

/// Graphics Manager
pub struct GraphicsManager {
    framebuffer: Option<FramebufferInfo>,
    initialized: bool,
}

impl GraphicsManager {
    /// Create a new graphics manager
    pub fn new() -> Self {
        Self {
            framebuffer: None,
            initialized: false,
        }
    }

    /// Initialize graphics with framebuffer information
    pub fn initialize(&mut self, framebuffer: FramebufferInfo) -> Result<()> {
        self.framebuffer = Some(framebuffer);
        self.initialized = true;

        println!("[graphics] Initialized with framebuffer:");
        println!("[graphics]   Resolution: {}x{}",
                 self.framebuffer.as_ref().unwrap().width,
                 self.framebuffer.as_ref().unwrap().height);
        println!("[graphics]   Address: {:#x}",
                 self.framebuffer.as_ref().unwrap().address);

        Ok(())
    }

    /// Clear the screen
    pub fn clear_screen(&self, color: u32) -> Result<()> {
        let fb = self.framebuffer.as_ref().ok_or(BootError::FeatureNotEnabled("Graphics"))?;

        let pixel_count = fb.width as usize * fb.height as usize;
        let pixels = unsafe {
            core::slice::from_raw_parts_mut(fb.address as *mut u32, pixel_count)
        };

        for pixel in pixels {
            *pixel = color;
        }

        Ok(())
    }

    /// Draw a pixel
    pub fn draw_pixel(&self, x: u32, y: u32, color: u32) -> Result<()> {
        let fb = self.framebuffer.as_ref().ok_or(BootError::FeatureNotEnabled("Graphics"))?;

        if x >= fb.width || y >= fb.height {
            return Ok(()); // Out of bounds, just ignore
        }

        let index = (y as usize) * (fb.stride as usize / 4) + (x as usize);
        let pixels = unsafe {
            core::slice::from_raw_parts_mut(fb.address as *mut u32,
                                             (fb.width as usize) * (fb.height as usize))
        };

        pixels[index] = color;
        Ok(())
    }

    /// Draw a filled rectangle
    pub fn fill_rect(&self, x: u32, y: u32, width: u32, height: u32, color: u32) -> Result<()> {
        let fb = self.framebuffer.as_ref().ok_or(BootError::FeatureNotEnabled("Graphics"))?;

        for py in y..(y + height).min(fb.height) {
            for px in x..(x + width).min(fb.width) {
                self.draw_pixel(px, py, color)?;
            }
        }

        Ok(())
    }

    /// Draw a rectangle outline
    pub fn draw_rect(&self, x: u32, y: u32, width: u32, height: u32, color: u32) -> Result<()> {
        // Top edge
        for px in x..(x + width) {
            self.draw_pixel(px, y, color)?;
        }

        // Bottom edge
        for px in x..(x + width) {
            self.draw_pixel(px, y + height - 1, color)?;
        }

        // Left edge
        for py in y..(y + height) {
            self.draw_pixel(x, py, color)?;
        }

        // Right edge
        for py in y..(y + height) {
            self.draw_pixel(x + width - 1, py, color)?;
        }

        Ok(())
    }

    /// Get framebuffer information
    pub fn get_framebuffer(&self) -> Option<&FramebufferInfo> {
        self.framebuffer.as_ref()
    }

    /// Check if graphics is initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Convert RGB to framebuffer pixel format
    pub fn rgb_to_pixel(&self, r: u8, g: u8, b: u8) -> u32 {
        let fb = self.framebuffer.as_ref().unwrap_or(&FramebufferInfo {
            address: 0,
            width: 0,
            height: 0,
            bytes_per_pixel: 4,
            stride: 0,
            pixel_format: 0,
        });

        match fb.pixel_format {
            0 => ((r as u32) << 16) | ((g as u32) << 8) | (b as u32), // RGB
            1 => ((b as u32) << 16) | ((g as u32) << 8) | (r as u32), // BGR
            _ => 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32), // RGBA with alpha
        }
    }

    /// Create common colors
    pub const fn color_black() -> u32 { 0x000000 }
    pub const fn color_white() -> u32 { 0xFFFFFF }
    pub const fn color_red() -> u32 { 0xFF0000 }
    pub const fn color_green() -> u32 { 0x00FF00 }
    pub const fn color_blue() -> u32 { 0x0000FF }
    pub const fn color_yellow() -> u32 { 0xFFFF00 }
    pub const fn color_cyan() -> u32 { 0x00FFFF }
    pub const fn color_magenta() -> u32 { 0xFF00FF }
    pub const fn color_gray() -> u32 { 0x808080 }
}

/// Simple font renderer
pub struct SimpleFont {
    graphics: *mut GraphicsManager,
}

impl SimpleFont {
    /// Create a new simple font renderer
    pub fn new(graphics: &mut GraphicsManager) -> Self {
        Self {
            graphics,
        }
    }

    /// Draw a character
    pub fn draw_char(&self, x: u32, y: u32, ch: char, color: u32) -> Result<()> {
        // Simple 8x8 font rendering (very basic implementation)
        const FONT_WIDTH: u32 = 8;
        const FONT_HEIGHT: u32 = 8;

        // Use a very simple pattern for some characters
        let pattern = match ch {
            'A' => [
                0b00111000,
                0b01000100,
                0b01000100,
                0b01111100,
                0b01000100,
                0b01000100,
                0b01000100,
                0b00000000,
            ],
            'B' => [
                0b01111000,
                0b01000100,
                0b01000100,
                0b01111000,
                0b01000100,
                0b01000100,
                0b01111000,
                0b00000000,
            ],
            'C' => [
                0b00111100,
                0b01000000,
                0b01000000,
                0b01000000,
                0b01000000,
                0b01000000,
                0b00111100,
                0b00000000,
            ],
            _ => [0; 8],
        };

        for row in 0..FONT_HEIGHT {
            for col in 0..FONT_WIDTH {
                if (pattern[row as usize] >> (7 - col)) & 1 != 0 {
                    unsafe { (*self.graphics).draw_pixel(x + col, y + row, color)? };
                }
            }
        }

        Ok(())
    }

    /// Draw a string
    pub fn draw_string(&mut self, x: u32, y: u32, text: &str, color: u32) -> Result<()> {
        let mut current_x = x;

        for ch in text.chars() {
            if ch == ' ' {
                current_x += 4; // Space width
            } else {
                self.draw_char(current_x, y, ch, color)?;
                current_x += 9; // Character width + spacing
            }
        }

        Ok(())
    }
}

/// Display error screen
pub fn display_error_screen(title: &str, message: &str) -> Result<()> {
    let mut graphics = GraphicsManager::new();

    // Create a dummy framebuffer for error display
    let dummy_fb = FramebufferInfo {
        address: 0xB8000, // VGA text mode buffer
        width: 80,
        height: 25,
        bytes_per_pixel: 2,
        stride: 160,
        pixel_format: 0,
    };

    graphics.initialize(dummy_fb)?;

    // Clear screen with red background
    graphics.fill_rect(0, 0, 80, 25, GraphicsManager::color_red())?;

    // Draw title
    let mut font = SimpleFont::new(&mut graphics);
    font.draw_string(10, 5, title, GraphicsManager::color_white())?;

    // Draw message
    font.draw_string(10, 10, message, GraphicsManager::color_white())?;

    Ok(())
}

/// Display boot splash screen
pub fn display_boot_splash_screen() -> Result<()> {
    let mut graphics = GraphicsManager::new();

    // This would need real framebuffer from UEFI
    let dummy_fb = FramebufferInfo {
        address: 0,
        width: 1024,
        height: 768,
        bytes_per_pixel: 4,
        stride: 4096,
        pixel_format: 0,
    };

    graphics.initialize(dummy_fb)?;

    // Create gradient background
    let fb = graphics.get_framebuffer().unwrap();
    for y in 0..fb.height {
        let color_value = (y * 255 / fb.height) as u32;
        let color = ((color_value as u32) << 16) | (color_value << 8) | color_value;

        for x in 0..fb.width {
            graphics.draw_pixel(x, y, color)?;
        }
    }

    // Draw NOS logo (simplified)
    graphics.draw_rect(300, 200, 424, 368, GraphicsManager::color_black())?;
    graphics.fill_rect(310, 210, 404, 348, GraphicsManager::color_blue())?;

    // Draw text
    let mut font = SimpleFont::new(&mut graphics);
    font.draw_string(400, 350, "NOS OPERATING SYSTEM", GraphicsManager::color_white())?;
    font.draw_string(420, 380, "UEFI Bootloader v0.1.0", GraphicsManager::color_white())?;

    Ok(())
}

#[cfg(not(feature = "graphics_support"))]
pub struct GraphicsManager;

#[cfg(not(feature = "graphics_support"))]
impl GraphicsManager {
    pub fn new() -> Self {
        Self
    }

    pub fn initialize(&mut self, _framebuffer: FramebufferInfo) -> Result<()> {
        Err(BootError::FeatureNotEnabled("Graphics support"))
    }

    pub fn clear_screen(&self, _color: u32) -> Result<()> {
        Err(BootError::FeatureNotEnabled("Graphics support"))
    }

    pub fn get_framebuffer(&self) -> Option<&FramebufferInfo> {
        None
    }

    pub fn is_initialized(&self) -> bool {
        false
    }
}

#[cfg(not(feature = "graphics_support"))]
pub const fn display_error_screen(_title: &str, _message: &str) -> Result<()> {
    Err(BootError::FeatureNotEnabled("Graphics support"))
}

#[cfg(not(feature = "graphics_support"))]
pub const fn display_boot_splash_screen() -> Result<()> {
    Err(BootError::FeatureNotEnabled("Graphics support"))
}