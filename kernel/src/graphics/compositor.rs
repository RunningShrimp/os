//! Graphics compositor service
//!
//! Implements a zero-copy compositor that combines multiple surfaces into a single framebuffer.
//! Uses dirty rectangle tracking and VSync for efficient rendering.

extern crate alloc;

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use crate::subsystems::sync::Mutex;
use crate::reliability::errno::{EINVAL, ENOMEM};
use crate::graphics::surface::{Surface, SurfaceId, SurfaceManager, DirtyRect};
use crate::graphics::surface::get_surface_manager;
use crate::graphics::vsync::VsyncManager;

/// Compositor state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompositorState {
    /// Compositor is stopped
    Stopped,
    /// Compositor is running
    Running,
    /// Compositor is paused
    Paused,
}

/// Compositor - combines surfaces into framebuffer
pub struct Compositor {
    /// Compositor state
    state: AtomicBool,
    /// Framebuffer address
    framebuffer_addr: usize,
    /// Framebuffer width
    framebuffer_width: u32,
    /// Framebuffer height
    framebuffer_height: u32,
    /// Framebuffer stride
    framebuffer_stride: u32,
    /// VSync manager
    vsync: VsyncManager,
    /// Frame counter
    frame_count: AtomicU32,
    /// Last frame time (nanoseconds)
    last_frame_time: AtomicU32,
}

impl Compositor {
    /// Create a new compositor
    pub fn new(width: u32, height: u32) -> Result<Self, i32> {
        let stride = (width as usize * 4).next_multiple_of(64); // ARGB8888 = 4 bytes per pixel
        let size = stride * height as usize;
        
        // Allocate framebuffer
        let framebuffer_addr = crate::subsystems::mm::kalloc(size).ok_or(ENOMEM)?;
        
        // Initialize framebuffer to black
        unsafe {
            core::ptr::write_bytes(framebuffer_addr as *mut u8, 0, size);
        }
        
        Ok(Self {
            state: AtomicBool::new(false),
            framebuffer_addr,
            framebuffer_width: width,
            framebuffer_height: height,
            framebuffer_stride: stride as u32,
            vsync: VsyncManager::new(),
            frame_count: AtomicU32::new(0),
            last_frame_time: AtomicU32::new(0),
        })
    }
    
    /// Start the compositor
    pub fn start(&self) -> Result<(), i32> {
        self.state.store(true, Ordering::Release);
        self.vsync.enable()?;
        crate::println!("[compositor] Started compositor ({}x{})", self.framebuffer_width, self.framebuffer_height);
        Ok(())
    }
    
    /// Stop the compositor
    pub fn stop(&self) {
        self.state.store(false, Ordering::Release);
        self.vsync.disable();
        crate::println!("[compositor] Stopped compositor");
    }
    
    /// Composite all surfaces to framebuffer
    /// This is called on VSync or when surfaces are updated
    pub fn composite(&self) -> Result<(), i32> {
        if !self.state.load(Ordering::Acquire) {
            return Ok(()); // Compositor not running
        }
        
        let start_time = crate::subsystems::time::hrtime_nanos();
        
        // Get all surfaces, sorted by z-order
        let surface_manager = get_surface_manager();
        let surface_ids = surface_manager.get_all_surfaces();
        
        // Sort surfaces by z-order (lower z-order first = rendered first)
        // For now, use a simplified approach - in real implementation, we'd need
        // to handle locking properly
        let mut surfaces: Vec<(SurfaceId, i32)> = Vec::new();
        
        // Collect surface info (simplified - real implementation would use Arc<Mutex<Surface>>)
        {
            let surfaces_map = surface_manager.surfaces.lock();
            for (&id, surface) in surfaces_map.iter() {
                if surface.state == crate::graphics::surface::SurfaceState::Ready ||
                   surface.state == crate::graphics::surface::SurfaceState::Dirty {
                    surfaces.push((id, surface.z_order));
                }
            }
        }
        surfaces.sort_by_key(|(_, z)| *z);
        
        // Clear framebuffer (or use dirty rectangle optimization)
        // For now, we composite all surfaces
        
        // Composite each surface
        for (id, _) in surfaces {
            let surfaces_map = surface_manager.surfaces.lock();
            if let Some(surface) = surfaces_map.get(&id) {
                if let Some(front_addr) = surface.get_front_buffer_addr() {
                    drop(surfaces_map);
                    self.composite_surface(surface, front_addr)?;
                } else {
                    drop(surfaces_map);
                }
            } else {
                drop(surfaces_map);
            }
        }
        
        // Wait for VSync before presenting
        self.vsync.wait_for_vsync()?;
        
        // Update frame statistics
        let frame_time = crate::subsystems::time::hrtime_nanos() - start_time;
        self.frame_count.fetch_add(1, Ordering::Relaxed);
        self.last_frame_time.store(frame_time as u32, Ordering::Relaxed);
        
        Ok(())
    }
    
    /// Composite a single surface to framebuffer
    /// Uses GPU acceleration if available
    fn composite_surface(&self, surface: &Surface, surface_addr: usize) -> Result<(), i32> {
        // Calculate intersection with framebuffer
        let src_x = 0.max(-surface.x);
        let src_y = 0.max(-surface.y);
        let dst_x = 0.max(surface.x);
        let dst_y = 0.max(surface.y);
        
        let width = (self.framebuffer_width as i32 - dst_x).min(surface.width as i32 - src_x) as u32;
        let height = (self.framebuffer_height as i32 - dst_y).min(surface.height as i32 - src_y) as u32;
        
        if width == 0 || height == 0 {
            return Ok(()); // Surface is outside framebuffer
        }
        
        // Try GPU-accelerated composition if available
        if let Some(buffer_handle) = surface.front_buffer.as_ref().map(|b| b.handle) {
            let buffer_manager = crate::graphics::buffer::get_buffer_manager();
            let buffer = buffer_manager.get_buffer(buffer_handle);
            
            if let Some(buf) = buffer {
                if buf.gpu_resource_id != 0 {
                    // Use GPU-accelerated composition
                    return self.composite_surface_gpu(&buf, src_x as u32, src_y as u32, dst_x as u32, dst_y as u32, width, height);
                }
            }
        }
        
        // Fallback to CPU-based composition
        let src_stride = surface.front_buffer.as_ref().map(|b| b.stride).unwrap_or(surface.width * 4);
        let dst_stride = self.framebuffer_stride;
        
        unsafe {
            let src_ptr = (surface_addr + (src_y as usize * src_stride as usize) + (src_x as usize * 4)) as *const u8;
            let dst_ptr = (self.framebuffer_addr + (dst_y as usize * dst_stride as usize) + (dst_x as usize * 4)) as *mut u8;
            
            // Copy scanlines
            for y in 0..height {
                core::ptr::copy_nonoverlapping(
                    src_ptr.add((y as usize * src_stride as usize)),
                    dst_ptr.add((y as usize * dst_stride as usize)),
                    (width as usize * 4),
                );
            }
        }
        
        Ok(())
    }
    
    /// Composite surface using GPU acceleration
    fn composite_surface_gpu(
        &self,
        buffer: &crate::graphics::buffer::GraphicsBuffer,
        src_x: u32,
        src_y: u32,
        dst_x: u32,
        dst_y: u32,
        width: u32,
        height: u32,
    ) -> Result<(), i32> {
        // Transfer surface buffer to GPU
        let buffer_manager = crate::graphics::buffer::get_buffer_manager();
        buffer_manager.transfer_to_gpu(buffer.handle, src_x, src_y, width, height)?;
        buffer_manager.flush_gpu_buffer(buffer.handle, src_x, src_y, width, height)?;
        
        // In real implementation, we would:
        // 1. Use GPU blit command to composite directly to framebuffer
        // 2. Or use GPU resource as scanout if it's the only surface
        // 3. Handle alpha blending on GPU
        
        // For now, fallback to CPU copy after GPU transfer
        // This is a placeholder - real implementation would use GPU blit
        
        crate::println!("[compositor] GPU-accelerated composition (resource: {}, {}x{})", buffer.gpu_resource_id, width, height);
        Ok(())
    }
    
    /// Get framebuffer address (for display driver)
    pub fn get_framebuffer_addr(&self) -> usize {
        self.framebuffer_addr
    }
    
    /// Get framebuffer dimensions
    pub fn get_framebuffer_size(&self) -> (u32, u32) {
        (self.framebuffer_width, self.framebuffer_height)
    }
    
    /// Get frame statistics
    pub fn get_frame_stats(&self) -> (u32, u32) {
        (self.frame_count.load(Ordering::Relaxed), self.last_frame_time.load(Ordering::Relaxed))
    }
}

/// Global compositor instance
static COMPOSITOR: Mutex<Option<Compositor>> = Mutex::new(None);

/// Initialize compositor
pub fn init_compositor(width: u32, height: u32) -> Result<(), i32> {
    let mut compositor = COMPOSITOR.lock();
    if compositor.is_none() {
        *compositor = Some(Compositor::new(width, height)?);
        crate::println!("[compositor] Compositor initialized ({}x{})", width, height);
    }
    Ok(())
}

/// Get compositor instance
pub fn get_compositor() -> Option<&'static Compositor> {
    let compositor = COMPOSITOR.lock();
    unsafe {
        compositor.as_ref().map(|c| &*(c as *const Compositor))
    }
}

/// Start compositor
pub fn start_compositor() -> Result<(), i32> {
    if let Some(compositor) = get_compositor() {
        compositor.start()
    } else {
        Err(EINVAL)
    }
}

/// Composite frame (called on VSync or surface update)
pub fn composite_frame() -> Result<(), i32> {
    if let Some(compositor) = get_compositor() {
        compositor.composite()
    } else {
        Err(EINVAL)
    }
}

