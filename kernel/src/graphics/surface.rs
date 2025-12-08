//! Surface protocol for zero-copy graphics
//!
//! Implements a shared memory-based surface protocol that allows applications
//! to render directly to shared buffers without copying data through the kernel.

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use crate::sync::Mutex;
use crate::reliability::errno::{EINVAL, ENOMEM, ENOENT};

/// Surface ID type
pub type SurfaceId = u32;

/// Surface state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceState {
    /// Surface is being created
    Creating,
    /// Surface is ready for rendering
    Ready,
    /// Surface has pending updates
    Dirty,
    /// Surface is being composited
    Compositing,
    /// Surface is destroyed
    Destroyed,
}

/// Surface format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceFormat {
    /// ARGB8888 (32-bit)
    ARGB8888,
    /// RGB565 (16-bit)
    RGB565,
    /// RGBA8888 (32-bit)
    RGBA8888,
    /// YUV420 (planar)
    YUV420,
    /// NV12 (semi-planar)
    NV12,
}

impl SurfaceFormat {
    /// Get bytes per pixel for this format
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            SurfaceFormat::ARGB8888 | SurfaceFormat::RGBA8888 => 4,
            SurfaceFormat::RGB565 => 2,
            SurfaceFormat::YUV420 | SurfaceFormat::NV12 => 1, // Average
        }
    }
}

/// Surface buffer
pub struct SurfaceBuffer {
    /// Shared memory address
    pub addr: usize,
    /// Buffer size in bytes
    pub size: usize,
    /// Buffer width
    pub width: u32,
    /// Buffer height
    pub height: u32,
    /// Buffer stride (bytes per row)
    pub stride: u32,
    /// Format
    pub format: SurfaceFormat,
    /// Reference count
    pub ref_count: AtomicU32,
}

impl SurfaceBuffer {
    /// Create a new surface buffer
    pub fn new(width: u32, height: u32, format: SurfaceFormat) -> Result<Self, i32> {
        let bytes_per_pixel = format.bytes_per_pixel();
        let stride = (width as usize * bytes_per_pixel).next_multiple_of(64); // 64-byte alignment
        let size = stride * height as usize;
        
        // Allocate shared memory for the buffer
        // In real implementation, this would use shared memory allocation
        let addr = crate::mm::kalloc(size).ok_or(ENOMEM)?;
        
        Ok(Self {
            addr,
            size,
            width,
            height,
            stride: stride as u32,
            format,
            ref_count: AtomicU32::new(1),
        })
    }
    
    /// Increment reference count
    pub fn acquire(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Decrement reference count and free if zero
    pub fn release(&self) -> bool {
        let count = self.ref_count.fetch_sub(1, Ordering::Acquire);
        if count == 1 {
            // Last reference - free the buffer
            unsafe {
                crate::mm::kfree(self.addr, self.size);
            }
            true
        } else {
            false
        }
    }
}

/// Surface - represents a window or rendering surface
pub struct Surface {
    /// Surface ID
    pub id: SurfaceId,
    /// Surface state
    pub state: SurfaceState,
    /// Surface position
    pub x: i32,
    pub y: i32,
    /// Surface size
    pub width: u32,
    pub height: u32,
    /// Format
    pub format: SurfaceFormat,
    /// Front buffer (currently displayed)
    pub front_buffer: Option<SurfaceBuffer>,
    /// Back buffer (being rendered to)
    pub back_buffer: Option<SurfaceBuffer>,
    /// Dirty rectangle (region that needs updating)
    pub dirty_rect: Option<DirtyRect>,
    /// Z-order (higher = on top)
    pub z_order: i32,
    /// Owner process ID
    pub owner_pid: u32,
    /// Surface flags
    pub flags: SurfaceFlags,
}

/// Dirty rectangle
#[derive(Debug, Clone, Copy)]
pub struct DirtyRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Surface flags
#[derive(Debug, Clone, Copy)]
pub struct SurfaceFlags {
    /// Opaque surface (no alpha blending needed)
    pub opaque: bool,
    /// Surface is fullscreen
    pub fullscreen: bool,
    /// Surface should be scaled
    pub scaled: bool,
    /// Surface uses hardware acceleration
    pub hw_accel: bool,
}

impl Default for SurfaceFlags {
    fn default() -> Self {
        Self {
            opaque: false,
            fullscreen: false,
            scaled: false,
            hw_accel: false,
        }
    }
}

impl Surface {
    /// Create a new surface
    pub fn new(id: SurfaceId, width: u32, height: u32, format: SurfaceFormat, owner_pid: u32) -> Result<Self, i32> {
        // Create front and back buffers
        let front_buffer = SurfaceBuffer::new(width, height, format)?;
        let back_buffer = SurfaceBuffer::new(width, height, format)?;
        
        Ok(Self {
            id,
            state: SurfaceState::Ready,
            x: 0,
            y: 0,
            width,
            height,
            format,
            front_buffer: Some(front_buffer),
            back_buffer: Some(back_buffer),
            dirty_rect: None,
            z_order: 0,
            owner_pid,
            flags: SurfaceFlags::default(),
        })
    }
    
    /// Mark surface as dirty (needs compositing)
    pub fn mark_dirty(&mut self, rect: Option<DirtyRect>) {
        self.state = SurfaceState::Dirty;
        self.dirty_rect = rect;
    }
    
    /// Swap front and back buffers (zero-copy)
    pub fn swap_buffers(&mut self) -> Result<(), i32> {
        if self.back_buffer.is_none() {
            return Err(EINVAL);
        }
        
        // Zero-copy swap: just swap the pointers
        core::mem::swap(&mut self.front_buffer, &mut self.back_buffer);
        self.state = SurfaceState::Ready;
        self.dirty_rect = None;
        
        Ok(())
    }
    
    /// Get buffer address for rendering
    pub fn get_back_buffer_addr(&self) -> Option<usize> {
        self.back_buffer.as_ref().map(|b| b.addr)
    }
    
    /// Get front buffer address for compositing
    pub fn get_front_buffer_addr(&self) -> Option<usize> {
        self.front_buffer.as_ref().map(|b| b.addr)
    }
}

/// Surface manager - manages all surfaces in the system
pub struct SurfaceManager {
    /// Next surface ID
    next_surface_id: AtomicU32,
    /// Surfaces by ID
    surfaces: Mutex<alloc::collections::BTreeMap<SurfaceId, Surface>>,
    /// Surfaces by owner PID
    surfaces_by_pid: Mutex<alloc::collections::BTreeMap<u32, Vec<SurfaceId>>>,
}

impl SurfaceManager {
    /// Create a new surface manager
    pub fn new() -> Self {
        Self {
            next_surface_id: AtomicU32::new(1),
            surfaces: Mutex::new(alloc::collections::BTreeMap::new()),
            surfaces_by_pid: Mutex::new(alloc::collections::BTreeMap::new()),
        }
    }
    
    /// Create a new surface
    pub fn create_surface(&self, width: u32, height: u32, format: SurfaceFormat, owner_pid: u32) -> Result<SurfaceId, i32> {
        let id = self.next_surface_id.fetch_add(1, Ordering::SeqCst);
        
        let surface = Surface::new(id, width, height, format, owner_pid)?;
        
        {
            let mut surfaces = self.surfaces.lock();
            surfaces.insert(id, surface);
        }
        
        {
            let mut by_pid = self.surfaces_by_pid.lock();
            by_pid.entry(owner_pid).or_insert_with(Vec::new).push(id);
        }
        
        crate::println!("[graphics] Created surface {} ({}x{}, format: {:?})", id, width, height, format);
        Ok(id)
    }
    
    /// Get surface by ID (returns a reference - caller must handle locking)
    pub fn get_surface(&self, id: SurfaceId) -> Option<alloc::sync::Arc<Mutex<Surface>>> {
        // In real implementation, surfaces would be stored as Arc<Mutex<Surface>>
        // For now, return None as placeholder
        None
    }
    
    /// Get mutable surface by ID
    pub fn get_surface_mut(&self, id: SurfaceId) -> Option<alloc::sync::Arc<Mutex<Surface>>> {
        // Return a reference that can be locked
        // In real implementation, we'd use Arc<Mutex<Surface>> for thread safety
        None // Placeholder
    }
    
    /// Destroy a surface
    pub fn destroy_surface(&self, id: SurfaceId) -> Result<(), i32> {
        let mut surfaces = self.surfaces.lock();
        let surface = surfaces.remove(&id).ok_or(ENOENT)?;
        
        // Release buffers
        if let Some(front) = surface.front_buffer {
            front.release();
        }
        if let Some(back) = surface.back_buffer {
            back.release();
        }
        
        // Remove from PID index
        {
            let mut by_pid = self.surfaces_by_pid.lock();
            if let Some(pid_surfaces) = by_pid.get_mut(&surface.owner_pid) {
                pid_surfaces.retain(|&sid| sid != id);
                if pid_surfaces.is_empty() {
                    by_pid.remove(&surface.owner_pid);
                }
            }
        }
        
        crate::println!("[graphics] Destroyed surface {}", id);
        Ok(())
    }
    
    /// Get all surfaces (for compositor)
    pub fn get_all_surfaces(&self) -> Vec<SurfaceId> {
        let surfaces = self.surfaces.lock();
        surfaces.keys().copied().collect()
    }
    
    /// Get surfaces by owner PID
    pub fn get_surfaces_by_pid(&self, pid: u32) -> Vec<SurfaceId> {
        let by_pid = self.surfaces_by_pid.lock();
        by_pid.get(&pid).cloned().unwrap_or_default()
    }
}

/// Global surface manager instance
static SURFACE_MANAGER: Mutex<Option<SurfaceManager>> = Mutex::new(None);

/// Initialize surface manager
pub fn init_surface_manager() -> Result<(), i32> {
    let mut manager = SURFACE_MANAGER.lock();
    if manager.is_none() {
        *manager = Some(SurfaceManager::new());
        crate::println!("[graphics] Surface manager initialized");
    }
    Ok(())
}

/// Get surface manager
pub fn get_surface_manager() -> &'static SurfaceManager {
    static INIT_ONCE: crate::sync::Once = crate::sync::Once::new();
    INIT_ONCE.call_once(|| {
        let mut manager = SURFACE_MANAGER.lock();
        if manager.is_none() {
            *manager = Some(SurfaceManager::new());
        }
    });
    
    unsafe {
        // Safety: We've initialized it above
        &*(SURFACE_MANAGER.lock().as_ref().unwrap() as *const SurfaceManager)
    }
}

