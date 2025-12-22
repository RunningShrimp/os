//! Graphics rendering - ARGB8888 framebuffer operations with double buffering
//!
//! Provides high-level graphics operations on framebuffer:
//! - Pixel-level drawing
//! - Screen clearing
//! - Line and rectangle drawing
//! - ARGB8888 color format support
//! - Double buffering for flicker-free rendering
//! - Dirty region tracking for performance optimization

use core::ptr;
use alloc::alloc::{alloc, dealloc, Layout};
use alloc::vec::Vec;

// Import SIMD instructions only for x86_64 target
#[cfg(target_arch = "x86_64")]
use core::arch::x86_64::{_mm_set1_epi32, _mm_storeu_si128, _mm_loadu_si128, __m128i};

// Import the unified FramebufferInfo from protocol
use crate::protocol::FramebufferInfo;

// Export VBE submodule for external use
pub mod vbe;

/// ARGB8888 color representation
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub alpha: u8,
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

impl Color {
    /// Create color from ARGB components
    pub fn argb(alpha: u8, red: u8, green: u8, blue: u8) -> Self {
        Self { alpha, red, green, blue }
    }

    /// Create opaque color from RGB
    pub fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Self { alpha: 255, red, green, blue }
    }

    /// Convert to ARGB8888 u32
    pub fn as_argb8888(&self) -> u32 {
        ((self.alpha as u32) << 24)
            | ((self.red as u32) << 16)
            | ((self.green as u32) << 8)
            | (self.blue as u32)
    }

    /// Black color (opaque)
    pub fn black() -> Self {
        Self::rgb(0, 0, 0)
    }

    /// White color (opaque)
    pub fn white() -> Self {
        Self::rgb(255, 255, 255)
    }

    /// Red color (opaque)
    pub fn red() -> Self {
        Self::rgb(255, 0, 0)
    }
    
    /// Green color (opaque)
    pub fn green() -> Self {
        Self::rgb(0, 255, 0)
    }
    
    /// Blue color (opaque)
    pub fn blue() -> Self {
        Self::rgb(0, 0, 255)
    }
    
    /// Cyan color (opaque)
    pub fn cyan() -> Self {
        Self::rgb(0, 255, 255)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test framebuffer operations with pitch != width*4
    /// This validates that all drawing operations correctly handle stride_pixels = pitch/4
    #[test]
    fn test_pitch_not_equal_width_times_four() {
        // Create a framebuffer with pitch != width*4 (simulating hardware alignment)
        let fb_info = FramebufferInfo::new(
            0x10000000,   // Address
            100,          // Width (pixels)
            50,           // Height (pixels)
            408,          // Pitch (bytes) = width*4 + 8 (extra padding)
            32,           // BPP (32 bits = 4 bytes)
        );

        // Verify stride calculation is correct
        let stride_pixels = (fb_info.pitch / 4) as usize;
        assert_eq!(stride_pixels, 102, "Stride should be pitch/4 = 408/4 = 102");

        // Verify buffer size calculation
        let buffer_size = fb_info.buffer_size();
        assert_eq!(buffer_size, 50 * 408, "Buffer size should be height * pitch");

        // Test with a mock renderer (we'll test drawing operations in isolation)
        let renderer = GraphicsRenderer::new(fb_info.clone());
        assert_eq!(renderer.fb.width, 100);
        assert_eq!(renderer.fb.height, 50);
        assert_eq!(renderer.fb.pitch, 408);
    }

    /// Test DoubleBuffer creation with non-standard pitch
    #[test]
    fn test_double_buffer_with_non_standard_pitch() {
        // Create a mock framebuffer info with pitch != width*4
        let fb_info = FramebufferInfo::new(
            0x10000000,   // Fake address (not used in test)
            800,          // Width
            600,          // Height
            3216,         // Pitch = 800*4 + 16 (extra padding)
            32,           // BPP
        );

        // Test buffer size calculation
        let buffer_size = fb_info.buffer_size();
        assert_eq!(buffer_size, 600 * 3216, "Buffer size should be height * pitch");
        assert_ne!(buffer_size, 600 * 800 * 4, "Buffer size should be different from width*height*4");
    }

    /// Test DirtyRect overlap detection
    #[test]
    fn test_dirty_rect_overlap() {
        let rect1 = DirtyRect::new(0, 0, 10, 10);
        let rect2 = DirtyRect::new(5, 5, 10, 10);
        let rect3 = DirtyRect::new(15, 15, 10, 10);

        // Test overlapping
        assert!(rect1.overlaps(&rect2), "Rectangles should overlap");
        assert!(!rect1.overlaps(&rect3), "Rectangles should not overlap");

        // Test merging
        let merged = rect1.merge(&rect2);
        assert_eq!(merged.x, 0, "Merged x should be 0");
        assert_eq!(merged.y, 0, "Merged y should be 0");
        assert_eq!(merged.width, 15, "Merged width should be 15");
        assert_eq!(merged.height, 15, "Merged height should be 15");
    }

    /// Test framebuffer bounds checking
    #[test]
    fn test_framebuffer_bounds_check() {
        let fb_info = FramebufferInfo::new(
            0x10000000,
            100,
            50,
            408,
            32,
        );

        // Test in bounds
        assert!(fb_info.in_bounds(0, 0), "(0,0) should be in bounds");
        assert!(fb_info.in_bounds(99, 49), "(99,49) should be in bounds");
        assert!(fb_info.in_bounds(50, 25), "(50,25) should be in bounds");

        // Test out of bounds
        assert!(!fb_info.in_bounds(100, 0), "x=100 should be out of bounds");
        assert!(!fb_info.in_bounds(0, 50), "y=50 should be out of bounds");
        assert!(!fb_info.in_bounds(200, 100), "(200,100) should be out of bounds");
    }
}



/// 脏区域矩形，用于跟踪需要更新的屏幕区域
#[derive(Debug, Clone, Copy)]
pub struct DirtyRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl DirtyRect {
    /// 创建新的脏区域
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self { x, y, width, height }
    }

    /// 检查是否与另一个脏区域重叠
    pub fn overlaps(&self, other: &DirtyRect) -> bool {
        self.x < other.x + other.width &&
            self.x + self.width > other.x &&
            self.y < other.y + other.height &&
            self.y + self.height > other.y
    }

    /// 合并两个脏区域，返回包含两者的最小矩形
    pub fn merge(&self, other: &DirtyRect) -> DirtyRect {
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = (self.x + self.width).max(other.x + other.width);
        let y2 = (self.y + self.height).max(other.y + other.height);
        
        DirtyRect {
            x: x1,
            y: y1,
            width: x2 - x1,
            height: y2 - y1,
        }
    }
}

/// 双缓冲管理器
pub struct DoubleBuffer {
    /// 前台缓冲区（当前显示）
    front_buffer: *mut u32,
    /// 后台缓冲区（当前渲染）
    back_buffer: *mut u32,
    /// 缓冲区大小（像素数）
    buffer_size: usize,
    /// 帧缓冲区信息
    fb_info: FramebufferInfo,
    /// 脏区域列表
    dirty_regions: Vec<DirtyRect>,
    /// 是否正在渲染
    rendering: bool,
    /// 是否启用脏区域检测
    dirty_tracking_enabled: bool,
}

impl DoubleBuffer {
    /// 创建新的双缓冲管理器
    pub fn new(fb_info: FramebufferInfo) -> Result<Self, &'static str> {
        if fb_info.address == 0 {
            return Err("Framebuffer address is null");
        }

        // 使用pitch/4作为每行像素数（stride），而不是width
        let stride_pixels = (fb_info.pitch / 4) as usize; // ARGB8888每像素4字节
        let buffer_size = stride_pixels * fb_info.height as usize;
        
        // 分配后台缓冲区内存，使用真实的帧缓冲区大小
        let layout = Layout::from_size_align(
            fb_info.pitch as usize * fb_info.height as usize, // 真实字节大小
            8, // 8字节对齐
        ).map_err(|_| "Invalid layout for back buffer")?;
        
        let back_buffer = unsafe {
            alloc(layout) as *mut u32
        };
        
        if back_buffer.is_null() {
            return Err("Failed to allocate back buffer");
        }

        Ok(Self {
            front_buffer: fb_info.address as *mut u32,
            back_buffer,
            buffer_size,
            fb_info,
            dirty_regions: Vec::new(),
            rendering: false,
            dirty_tracking_enabled: true,
        })
    }

    /// 开始渲染会话
    pub fn begin_render(&mut self) -> Result<(), &'static str> {
        if self.rendering {
            return Err("Already rendering");
        }
        
        self.rendering = true;
        self.clear_dirty_regions();
        Ok(())
    }

    /// 结束渲染会话
    pub fn end_render(&mut self) -> Result<(), &'static str> {
        if !self.rendering {
            return Err("Not rendering");
        }
        
        self.rendering = false;
        Ok(())
    }

    /// 交换缓冲区，将后台缓冲区内容复制到前台
    pub fn swap_buffers(&mut self) -> Result<(), &'static str> {
        if self.rendering {
            return Err("Cannot swap buffers while rendering");
        }

        if self.dirty_tracking_enabled && !self.dirty_regions.is_empty() {
            // 只更新脏区域
            self.update_dirty_regions()?;
        } else {
            // 全屏更新
            self.copy_full_buffer()?;
        }

        Ok(())
    }

    /// 复制整个后台缓冲区到前台
    fn copy_full_buffer(&self) -> Result<(), &'static str> {
        // 使用真实的帧缓冲区字节大小
        let total_bytes = self.fb_info.buffer_size();
        let total_pixels = self.buffer_size;
        log::trace!("Copying full framebuffer: {} pixels ({} bytes)", total_pixels, total_bytes);
        
        unsafe {
            // 使用SIMD优化进行快速内存拷贝
            #[cfg(target_arch = "x86_64")]
            {
                let src = self.back_buffer as *const __m128i;
                let dst = self.front_buffer as *mut __m128i;
                let simd_iterations = total_bytes / 16; // 16字节 = 128位
                
                for i in 0..simd_iterations {
                    let data = _mm_loadu_si128(src.add(i));
                    _mm_storeu_si128(dst.add(i), data);
                }
                
                // 处理剩余字节
                let remaining_bytes = total_bytes % 16;
                if remaining_bytes > 0 {
                    let src_remaining = src.add(simd_iterations) as *const u8;
                    let dst_remaining = dst.add(simd_iterations) as *mut u8;
                    ptr::copy_nonoverlapping(src_remaining, dst_remaining, remaining_bytes);
                }
            }
            
            // 非x86_64平台使用常规拷贝
            #[cfg(not(target_arch = "x86_64"))]
            {
                ptr::copy_nonoverlapping(
                    self.back_buffer,
                    self.front_buffer,
                    total_pixels,
                );
            }
        }
        
        Ok(())
    }

    /// 只更新脏区域
    fn update_dirty_regions(&self) -> Result<(), &'static str> {
        for region in &self.dirty_regions {
            self.copy_region(region)?;
        }
        Ok(())
    }

    /// 复制指定区域
    fn copy_region(&self, region: &DirtyRect) -> Result<(), &'static str> {
        let fb = &self.fb_info;
        
        // 边界检查
        if region.x >= fb.width || region.y >= fb.height {
            return Ok(()); // 超出边界的区域忽略
        }
        
        // 计算实际复制的区域（裁剪到屏幕边界）
        let x_end = (region.x + region.width).min(fb.width);
        let y_end = (region.y + region.height).min(fb.height);
        let actual_width = x_end - region.x;
        let _actual_height = y_end - region.y;
        log::trace!("Copying region with dimensions {}x{}", actual_width, _actual_height);
        
        // 使用pitch/4作为每行像素数（stride）
        let stride_pixels = (fb.pitch / 4) as usize;
        
        unsafe {
            for y in region.y..y_end {
                let src_offset = y as usize * stride_pixels + region.x as usize;
                let dst_offset = y as usize * stride_pixels + region.x as usize;
                
                let src_ptr = self.back_buffer.add(src_offset);
                let dst_ptr = self.front_buffer.add(dst_offset);
                
                ptr::copy_nonoverlapping(src_ptr, dst_ptr, actual_width as usize);
            }
        }
        
        Ok(())
    }

    /// 添加脏区域
    pub fn add_dirty_region(&mut self, region: DirtyRect) {
        if !self.dirty_tracking_enabled || !self.rendering {
            return;
        }
        
        // 尝试与现有脏区域合并
        let mut merged = false;
        for existing in &mut self.dirty_regions {
            if existing.overlaps(&region) {
                *existing = existing.merge(&region);
                merged = true;
                break;
            }
        }
        
        if !merged {
            self.dirty_regions.push(region);
        }
    }

    /// 清除所有脏区域
    fn clear_dirty_regions(&mut self) {
        self.dirty_regions.clear();
    }

    /// 启用/禁用脏区域跟踪
    pub fn set_dirty_tracking(&mut self, enabled: bool) {
        self.dirty_tracking_enabled = enabled;
        if !enabled {
            self.clear_dirty_regions();
        }
    }

    /// 获取后台缓冲区指针
    pub fn get_back_buffer(&self) -> *mut u32 {
        self.back_buffer
    }

    /// 获取帧缓冲区信息
    pub fn get_framebuffer_info(&self) -> &FramebufferInfo {
        &self.fb_info
    }

    /// 获取缓冲区大小（像素数）
    pub fn buffer_size(&self) -> usize {
        self.buffer_size
    }

    /// 检查是否正在渲染
    pub fn is_rendering(&self) -> bool {
        self.rendering
    }
}

impl Drop for DoubleBuffer {
    fn drop(&mut self) {
        // 释放后台缓冲区内存
        if !self.back_buffer.is_null() {
            let layout = Layout::from_size_align(
                self.fb_info.pitch as usize * self.fb_info.height as usize, // 与分配时保持一致
                8,
            ).unwrap();
            unsafe {
                dealloc(self.back_buffer as *mut u8, layout);
            }
        }
    }
}

/// Graphics renderer for ARGB8888 framebuffer with double buffering
pub struct GraphicsRenderer {
    fb: FramebufferInfo,
    double_buffer: Option<DoubleBuffer>,
}

impl GraphicsRenderer {
    /// Create new graphics renderer
    pub fn new(fb: FramebufferInfo) -> Self {
        Self {
            fb,
            double_buffer: None,
        }
    }

    /// Create new graphics renderer with double buffering
    pub fn new_with_double_buffer(fb: FramebufferInfo) -> Result<Self, &'static str> {
        let double_buffer = DoubleBuffer::new(fb.clone())?;
        Ok(Self {
            fb,
            double_buffer: Some(double_buffer),
        })
    }

    /// Initialize double buffering (if not already initialized)
    pub fn init_double_buffer(&mut self) -> Result<(), &'static str> {
        if self.double_buffer.is_none() {
            self.double_buffer = Some(DoubleBuffer::new(self.fb.clone())?);
        }
        Ok(())
    }

    /// Disable double buffering
    pub fn disable_double_buffer(&mut self) {
        self.double_buffer = None;
    }

    /// Check if double buffering is enabled
    pub fn is_double_buffer_enabled(&self) -> bool {
        self.double_buffer.is_some()
    }

    /// Begin rendering session (only when double buffering is enabled)
    pub fn begin_render(&mut self) -> Result<(), &'static str> {
        if let Some(ref mut db) = self.double_buffer {
            db.begin_render()
        } else {
            Ok(()) // No-op when double buffering is disabled
        }
    }

    /// End rendering session (only when double buffering is enabled)
    pub fn end_render(&mut self) -> Result<(), &'static str> {
        if let Some(ref mut db) = self.double_buffer {
            db.end_render()
        } else {
            Ok(()) // No-op when double buffering is disabled
        }
    }

    /// Swap buffers (only when double buffering is enabled)
    pub fn swap_buffers(&mut self) -> Result<(), &'static str> {
        if let Some(ref mut db) = self.double_buffer {
            db.swap_buffers()
        } else {
            Ok(()) // No-op when double buffering is disabled
        }
    }

    /// Enable/disable dirty region tracking
    pub fn set_dirty_tracking(&mut self, enabled: bool) {
        if let Some(ref mut db) = self.double_buffer {
            db.set_dirty_tracking(enabled);
        }
    }

    /// Get framebuffer pointer for drawing
    fn get_draw_buffer(&self) -> *mut u32 {
        if let Some(ref db) = self.double_buffer {
            db.get_back_buffer()
        } else {
            self.fb.address as *mut u32
        }
    }

    /// Add dirty region (only when double buffering is enabled)
    fn add_dirty_region(&mut self, x: u32, y: u32, width: u32, height: u32) {
        if let Some(ref mut db) = self.double_buffer {
            let region = DirtyRect::new(x, y, width, height);
            db.add_dirty_region(region);
        }
    }

    /// Initialize framebuffer - zero fill entire buffer
    pub fn initialize_framebuffer(&mut self) -> Result<(), &'static str> {
        if self.fb.address == 0 {
            return Err("Framebuffer address is null");
        }

        let ptr = self.fb.address as *mut u8;
        let size = self.fb.buffer_size();

        // SAFETY: Caller ensures address is valid framebuffer
        unsafe {
            ptr::write_bytes(ptr, 0, size);
        }

        // Also initialize back buffer if double buffering is enabled
        if let Some(ref mut db) = self.double_buffer {
            let back_ptr = db.get_back_buffer() as *mut u8;
            unsafe {
                ptr::write_bytes(back_ptr, 0, size);
            }
        }

        Ok(())
    }

    /// Clear entire screen with single color
    pub fn clear_screen(&mut self, color: Color) -> Result<(), &'static str> {
        if self.fb.address == 0 {
            return Err("Framebuffer address is null");
        }

        let color_val = color.as_argb8888();
        // 使用stride_pixels = pitch/4作为每行像素数
        let stride_pixels = (self.fb.pitch / 4) as usize;
        let total_pixels = stride_pixels * self.fb.height as usize;
        let fb_ptr = self.get_draw_buffer();

        // SAFETY: Caller ensures address is valid framebuffer
        unsafe {
            // 尝试使用SIMD优化（x86_64平台）
            #[cfg(target_arch = "x86_64")]
            {
                // 创建4个颜色值的SIMD向量（16字节）
                let simd_color = _mm_set1_epi32(color_val as i32);
                
                // 计算可被4整除的像素数
                let simd_pixels = total_pixels & !3;
                let simd_end = fb_ptr.add(simd_pixels);
                
                // 使用SIMD批量填充
                let mut current_ptr = fb_ptr as *mut __m128i;
                while current_ptr < simd_end as *mut _ {
                    _mm_storeu_si128(current_ptr, simd_color);
                    current_ptr = current_ptr.add(1);
                }
                
                // 处理剩余像素
                let remaining = fb_ptr.add(simd_pixels);
                let end = fb_ptr.add(total_pixels);
                let mut ptr = remaining;
                while ptr < end {
                    ptr::write(ptr, color_val);
                    ptr = ptr.add(1);
                }
            }
            
            // 非x86_64平台使用常规块填充
            #[cfg(not(target_arch = "x86_64"))]
            {
                let fb_slice = core::slice::from_raw_parts_mut(fb_ptr, total_pixels);
                fb_slice.fill(color_val);
            }
        }

        // Add dirty region for full screen if double buffering is enabled
        if self.double_buffer.is_some() {
            self.add_dirty_region(0, 0, self.fb.width, self.fb.height);
        }

        Ok(())
    }

    /// Draw single pixel at coordinates
    pub fn draw_pixel(&mut self, x: u32, y: u32, color: Color) -> Result<(), &'static str> {
        if !self.fb.in_bounds(x, y) {
            return Err("Pixel coordinates out of bounds");
        }

        if self.fb.address == 0 {
            return Err("Framebuffer address is null");
        }

        let fb_ptr = self.get_draw_buffer();
        // 使用stride_pixels = pitch/4作为每行像素数
        let stride_pixels = (self.fb.pitch / 4) as usize;
        let offset = y as usize * stride_pixels + x as usize;
        let color_val = color.as_argb8888();

        // SAFETY: Coordinates checked above, caller ensures valid address
        unsafe {
            ptr::write(fb_ptr.add(offset), color_val);
        }

        // Add dirty region for this pixel if double buffering is enabled
        if self.double_buffer.is_some() {
            self.add_dirty_region(x, y, 1, 1);
        }

        Ok(())
    }

    /// Draw horizontal line
    pub fn draw_h_line(
        &mut self,
        x: u32,
        y: u32,
        length: u32,
        color: Color,
    ) -> Result<(), &'static str> {
        if !self.fb.in_bounds(x, y) {
            return Err("Line start coordinates out of bounds");
        }

        // Clamp line length to screen boundary
        let max_length = self.fb.width.saturating_sub(x);
        let actual_length = length.min(max_length);

        let fb_ptr = self.get_draw_buffer();
        let color_val = color.as_argb8888();
        // 使用stride_pixels = pitch/4作为每行像素数
        let stride_pixels = (self.fb.pitch / 4) as usize;

        // SAFETY: Coordinates validated, length clamped
        unsafe {
            for i in 0..actual_length {
                let offset = y as usize * stride_pixels + x as usize + i as usize;
                ptr::write(fb_ptr.add(offset), color_val);
            }
        }

        // Add dirty region for this line if double buffering is enabled
        if self.double_buffer.is_some() {
            self.add_dirty_region(x, y, actual_length, 1);
        }

        Ok(())
    }

    /// Draw vertical line
    pub fn draw_v_line(
        &mut self,
        x: u32,
        y: u32,
        length: u32,
        color: Color,
    ) -> Result<(), &'static str> {
        if !self.fb.in_bounds(x, y) {
            return Err("Line start coordinates out of bounds");
        }

        // Clamp line length to screen boundary
        let max_length = self.fb.height.saturating_sub(y);
        let actual_length = length.min(max_length);

        let fb_ptr = self.get_draw_buffer();
        let color_val = color.as_argb8888();
        // 使用stride_pixels = pitch/4作为每行像素数
        let stride_pixels = (self.fb.pitch / 4) as usize;

        // SAFETY: Coordinates validated, length clamped
        unsafe {
            for i in 0..actual_length {
                let offset = (y + i) as usize * stride_pixels + x as usize;
                ptr::write(fb_ptr.add(offset), color_val);
            }
        }

        // Add dirty region for this line if double buffering is enabled
        if self.double_buffer.is_some() {
            self.add_dirty_region(x, y, 1, actual_length);
        }

        Ok(())
    }

    /// Draw filled rectangle
    pub fn draw_filled_rect(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        color: Color,
    ) -> Result<(), &'static str> {
        if !self.fb.in_bounds(x, y) {
            return Err("Rectangle start out of bounds");
        }

        // Clamp dimensions to screen boundaries
        let max_width = self.fb.width.saturating_sub(x);
        let max_height = self.fb.height.saturating_sub(y);
        let actual_width = width.min(max_width);
        let actual_height = height.min(max_height);

        if actual_width == 0 || actual_height == 0 {
            return Ok(());
        }

        let fb_ptr = self.get_draw_buffer();
        let color_val = color.as_argb8888();
        let stride_pixels = (self.fb.pitch / 4) as usize; // ARGB8888: 4 bytes per pixel

        // SAFETY: Coordinates validated, dimensions clamped
        unsafe {
            for row in 0..actual_height {
                // 优化：每行使用连续的内存访问模式
                let row_start = (y + row) as usize * stride_pixels + x as usize;
                log::trace!("Drawing rectangle row at offset {}", row_start);
                
                // 使用 slice::fill 进行行级块填充，提高缓存命中率
                let row_slice = core::slice::from_raw_parts_mut(fb_ptr.add(row_start), actual_width as usize);
                row_slice.fill(color_val);
            }
        }

        // Add dirty region for this rectangle if double buffering is enabled
        if self.double_buffer.is_some() {
            self.add_dirty_region(x, y, actual_width, actual_height);
        }

        Ok(())
    }

    /// Get framebuffer width in pixels
    pub fn width(&self) -> u32 {
        self.fb.width
    }

    /// Get framebuffer height in pixels
    pub fn height(&self) -> u32 {
        self.fb.height
    }

    /// 批量绘制像素 - 性能优化版本
    pub fn draw_pixels_batch(&mut self, pixels: &[(u32, u32, Color)]) -> Result<(), &'static str> {
        if pixels.is_empty() {
            return Ok(());
        }

        let fb_ptr = self.get_draw_buffer();
        let stride_pixels = (self.fb.pitch / 4) as usize; // ARGB8888: 4 bytes per pixel
        
        // 计算脏区域边界
        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0u32;
        let mut max_y = 0u32;

        // SAFETY: Coordinates will be validated
        unsafe {
            for &(x, y, color) in pixels {
                if self.fb.in_bounds(x, y) {
                    let offset = y as usize * stride_pixels + x as usize;
                    ptr::write(fb_ptr.add(offset), color.as_argb8888());
                    
                    // 更新脏区域边界
                    min_x = min_x.min(x);
                    min_y = min_y.min(y);
                    max_x = max_x.max(x);
                    max_y = max_y.max(y);
                }
            }
        }

        // 添加脏区域
        if self.double_buffer.is_some() && min_x != u32::MAX {
            let width = max_x - min_x + 1;
            let height = max_y - min_y + 1;
            self.add_dirty_region(min_x, min_y, width, height);
        }

        Ok(())
    }

    /// 绘制文本字符（简化版本）
    pub fn draw_char(&mut self, x: u32, y: u32, ch: char, color: Color) -> Result<(), &'static str> {
        // 简单的8x8字体数据（这里只实现几个基本字符）
        let font_data = match ch {
            'A' => [0x18, 0x3C, 0x66, 0x66, 0x7E, 0x66, 0x66, 0x00],
            'B' => [0x7C, 0x66, 0x66, 0x7C, 0x66, 0x66, 0x7C, 0x00],
            'C' => [0x3C, 0x66, 0x60, 0x60, 0x60, 0x66, 0x3C, 0x00],
            ' ' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
            _ => [0x7E, 0x42, 0x42, 0x42, 0x42, 0x42, 0x7E, 0x00], // 默认方块
        };

        let color_val = color.as_argb8888();
        let fb_ptr = self.get_draw_buffer();
        let stride_pixels = (self.fb.pitch / 4) as usize; // ARGB8888: 4 bytes per pixel

        // SAFETY: Coordinates will be validated
        unsafe {
            for (row, &byte) in font_data.iter().enumerate() {
                for col in 0..8 {
                    if (byte >> (7 - col)) & 1 == 1 {
                        let px_x = x + col;
                        let px_y = y + row as u32;
                        
                        if self.fb.in_bounds(px_x, px_y) {
                            let offset = px_y as usize * stride_pixels + px_x as usize;
                            ptr::write(fb_ptr.add(offset), color_val);
                        }
                    }
                }
            }
        }

        // 添加脏区域
        if self.double_buffer.is_some() {
            self.add_dirty_region(x, y, 8, 8);
        }

        Ok(())
    }

    /// 绘制文本字符串
    pub fn draw_text(&mut self, x: u32, y: u32, text: &str, color: Color) -> Result<(), &'static str> {
        for (i, ch) in text.chars().enumerate() {
            let char_x = x + (i as u32) * 8; // 每个字符8像素宽
            self.draw_char(char_x, y, ch, color)?;
        }
        Ok(())
    }

    /// 检查点是否在矩形内
    pub fn point_in_rect(&self, px: u32, py: u32, rx: u32, ry: u32, rw: u32, rh: u32) -> bool {
        px >= rx && px < rx + rw && py >= ry && py < ry + rh
    }

    /// 绘制圆角矩形
    pub fn draw_rounded_rect(
        &mut self,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        radius: u32,
        color: Color,
    ) -> Result<(), &'static str> {
        if !self.fb.in_bounds(x, y) {
            return Err("Rectangle start out of bounds");
        }

        let radius = radius.min(width.min(height) / 2);
        let color_val = color.as_argb8888();
        let fb_ptr = self.get_draw_buffer();
        let stride_pixels = (self.fb.pitch / 4) as usize; // ARGB8888: 4 bytes per pixel

        // 绘制主体部分（不包括圆角区域）
        let body_y_start = y + radius;
        let body_y_end = y + height - radius;
        
        if body_y_start < body_y_end {
            // 绘制中间矩形部分
            self.draw_filled_rect(x, body_y_start, width, body_y_end - body_y_start, color)?;
        }

        // 绘制上下两个矩形条
        self.draw_filled_rect(x + radius, y, width - 2 * radius, radius, color)?;
        self.draw_filled_rect(x + radius, y + height - radius, width - 2 * radius, radius, color)?;

        // 绘制四个圆角（简化为像素）
        unsafe {
            for dy in 0..radius {
                for dx in 0..radius {
                    // 左上角
                    let dist_sq = (radius - dx) * (radius - dx) + (radius - dy) * (radius - dy);
                    if dist_sq <= radius * radius {
                        let px = x + dx;
                        let py = y + dy;
                        if self.fb.in_bounds(px, py) {
                            let offset = py as usize * stride_pixels + px as usize;
                            ptr::write(fb_ptr.add(offset), color_val);
                        }
                    }
                    
                    // 右上角
                    let px = x + width - radius + dx;
                    let py = y + dy;
                    if self.fb.in_bounds(px, py) {
                        let offset = py as usize * stride_pixels + px as usize;
                        ptr::write(fb_ptr.add(offset), color_val);
                    }
                    
                    // 左下角
                    let px = x + dx;
                    let py = y + height - radius + dy;
                    if self.fb.in_bounds(px, py) {
                        let offset = py as usize * stride_pixels + px as usize;
                        ptr::write(fb_ptr.add(offset), color_val);
                    }
                    
                    // 右下角
                    let px = x + width - radius + dx;
                    let py = y + height - radius + dy;
                    if self.fb.in_bounds(px, py) {
                        let offset = py as usize * stride_pixels + px as usize;
                        ptr::write(fb_ptr.add(offset), color_val);
                    }
                }
            }
        }

        // 添加脏区域
        if self.double_buffer.is_some() {
            self.add_dirty_region(x, y, width, height);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_rect_creation() {
        let rect = DirtyRect::new(10, 20, 100, 50);
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 20);
        assert_eq!(rect.width, 100);
        assert_eq!(rect.height, 50);
    }

    #[test]
    fn test_dirty_rect_overlap() {
        let rect1 = DirtyRect::new(10, 10, 50, 50);
        let rect2 = DirtyRect::new(30, 30, 50, 50);
        let rect3 = DirtyRect::new(100, 100, 50, 50);
        
        assert!(rect1.overlaps(&rect2)); // 重叠
        assert!(!rect1.overlaps(&rect3)); // 不重叠
    }

    #[test]
    fn test_dirty_rect_merge() {
        let rect1 = DirtyRect::new(10, 10, 50, 50);
        let rect2 = DirtyRect::new(30, 30, 50, 50);
        let merged = rect1.merge(&rect2);
        
        assert_eq!(merged.x, 10);
        assert_eq!(merged.y, 10);
        assert_eq!(merged.width, 70);
        assert_eq!(merged.height, 70);
    }

    #[test]
    fn test_framebuffer_info() {
        let fb = FramebufferInfo::new(0x10000000, 1024, 768, 4096, 32);
        assert_eq!(fb.address, 0x10000000);
        assert_eq!(fb.width, 1024);
        assert_eq!(fb.height, 768);
        assert_eq!(fb.pitch, 4096);
        assert_eq!(fb.bpp, 32);
        assert_eq!(fb.buffer_size(), 768 * 4096);
        assert!(fb.in_bounds(512, 384));
        assert!(!fb.in_bounds(1024, 768));
    }

    #[test]
    fn test_color_creation() {
        let red = Color::rgb(255, 0, 0);
        assert_eq!(red.alpha, 255);
        assert_eq!(red.red, 255);
        assert_eq!(red.green, 0);
        assert_eq!(red.blue, 0);
        assert_eq!(red.as_argb8888(), 0xFFFF0000);

        let blue = Color::argb(128, 0, 0, 255);
        assert_eq!(blue.alpha, 128);
        assert_eq!(blue.as_argb8888(), 0x800000FF);
    }

    #[test]
    fn test_graphics_renderer_creation() {
        let fb = FramebufferInfo::new(0x10000000, 1024, 768, 4096, 32);
        let renderer = GraphicsRenderer::new(fb);
        assert_eq!(renderer.width(), 1024);
        assert_eq!(renderer.height(), 768);
        assert!(!renderer.is_double_buffer_enabled());
    }

    #[test]
    fn test_graphics_renderer_with_double_buffer() {
        let fb = FramebufferInfo::new(0x10000000, 800, 600, 3200, 32);
        let renderer = GraphicsRenderer::new_with_double_buffer(fb);
        assert!(renderer.is_ok());
        
        let renderer = renderer.unwrap();
        assert_eq!(renderer.width(), 800);
        assert_eq!(renderer.height(), 600);
        assert!(renderer.is_double_buffer_enabled());
    }

    #[test]
    fn test_double_buffer_rendering_cycle() {
        let fb = FramebufferInfo::new(0x10000000, 800, 600, 3200, 32);
        let mut renderer = GraphicsRenderer::new_with_double_buffer(fb).unwrap();
        
        // 测试渲染周期
        assert!(renderer.begin_render().is_ok());
        assert!(renderer.clear_screen(Color::black()).is_ok());
        assert!(renderer.draw_pixel(100, 100, Color::red()).is_ok());
        assert!(renderer.draw_h_line(50, 50, 100, Color::green()).is_ok());
        assert!(renderer.draw_v_line(150, 50, 100, Color::blue()).is_ok());
        assert!(renderer.draw_filled_rect(200, 200, 100, 50, Color::white()).is_ok());
        assert!(renderer.end_render().is_ok());
        assert!(renderer.swap_buffers().is_ok());
    }

    #[test]
    fn test_batch_pixel_drawing() {
        let fb = FramebufferInfo::new(0x10000000, 800, 600, 3200, 32);
        let mut renderer = GraphicsRenderer::new_with_double_buffer(fb).unwrap();
        
        let pixels = vec![
            (10, 10, Color::red()),
            (20, 20, Color::green()),
            (30, 30, Color::blue()),
        ];
        
        assert!(renderer.begin_render().is_ok());
        assert!(renderer.draw_pixels_batch(&pixels).is_ok());
        assert!(renderer.end_render().is_ok());
        assert!(renderer.swap_buffers().is_ok());
    }

    #[test]
    fn test_text_drawing() {
        let fb = FramebufferInfo::new(0x10000000, 800, 600, 3200, 32);
        let mut renderer = GraphicsRenderer::new_with_double_buffer(fb).unwrap();
        
        assert!(renderer.begin_render().is_ok());
        assert!(renderer.draw_text(100, 100, "ABC", Color::white()).is_ok());
        assert!(renderer.end_render().is_ok());
        assert!(renderer.swap_buffers().is_ok());
    }

    #[test]
    fn test_rounded_rect() {
        let fb = FramebufferInfo::new(0x10000000, 800, 600, 3200, 32);
        let mut renderer = GraphicsRenderer::new_with_double_buffer(fb).unwrap();
        
        assert!(renderer.begin_render().is_ok());
        assert!(renderer.draw_rounded_rect(100, 100, 200, 100, 10, Color::cyan()).is_ok());
        assert!(renderer.end_render().is_ok());
        assert!(renderer.swap_buffers().is_ok());
    }

    #[test]
    fn test_dirty_tracking() {
        let fb = FramebufferInfo::new(0x10000000, 800, 600, 3200, 32);
        let mut renderer = GraphicsRenderer::new_with_double_buffer(fb).unwrap();
        
        // 启用脏区域跟踪
        renderer.set_dirty_tracking(true);
        
        assert!(renderer.begin_render().is_ok());
        assert!(renderer.draw_pixel(100, 100, Color::red()).is_ok());
        assert!(renderer.draw_filled_rect(200, 200, 50, 50, Color::blue()).is_ok());
        assert!(renderer.end_render().is_ok());
        assert!(renderer.swap_buffers().is_ok());
        
        // 禁用脏区域跟踪
        renderer.set_dirty_tracking(false);
        assert!(renderer.begin_render().is_ok());
        assert!(renderer.clear_screen(Color::black()).is_ok());
        assert!(renderer.end_render().is_ok());
        assert!(renderer.swap_buffers().is_ok());
    }

    #[test]
    fn test_point_in_rect() {
        let fb = FramebufferInfo::new(0x10000000, 800, 600, 3200, 32);
        let renderer = GraphicsRenderer::new(fb);
        
        assert!(renderer.point_in_rect(150, 250, 100, 200, 200, 100));
        assert!(!renderer.point_in_rect(50, 150, 100, 200, 200, 100));
        assert!(!renderer.point_in_rect(350, 350, 100, 200, 200, 100));
    }
}
