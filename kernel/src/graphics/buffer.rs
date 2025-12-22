//! Graphics buffer management (GBM-like)
//!
//! Provides kernel-level graphics buffer management similar to Generic Buffer Management (GBM).
//! This allows efficient allocation and management of graphics buffers in shared memory.

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use crate::subsystems::sync::Mutex;
use crate::reliability::errno::{EINVAL, ENOMEM};

/// Buffer handle
pub type BufferHandle = u32;

/// Buffer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferType {
    /// Linear buffer (CPU-accessible)
    Linear,
    /// Tiled buffer (GPU-optimized)
    Tiled,
    /// Scanout buffer (can be displayed directly)
    Scanout,
}

/// Buffer allocation flags
#[derive(Debug, Clone, Copy)]
pub struct BufferFlags {
    /// Buffer is readable by CPU
    pub cpu_read: bool,
    /// Buffer is writable by CPU
    pub cpu_write: bool,
    /// Buffer is readable by GPU
    pub gpu_read: bool,
    /// Buffer is writable by GPU
    pub gpu_write: bool,
    /// Buffer can be scanned out (displayed)
    pub scanout: bool,
}

impl Default for BufferFlags {
    fn default() -> Self {
        Self {
            cpu_read: true,
            cpu_write: true,
            gpu_read: true,
            gpu_write: true,
            scanout: false,
        }
    }
}

/// Graphics buffer
pub struct GraphicsBuffer {
    /// Buffer handle
    pub handle: BufferHandle,
    /// Buffer address
    pub addr: usize,
    /// Buffer size
    pub size: usize,
    /// Buffer width
    pub width: u32,
    /// Buffer height
    pub height: u32,
    /// Buffer stride
    pub stride: u32,
    /// Buffer format
    pub format: crate::graphics::surface::SurfaceFormat,
    /// Buffer type
    pub buffer_type: BufferType,
    /// Buffer flags
    pub flags: BufferFlags,
    /// Reference count
    pub ref_count: AtomicU32,
    /// GPU resource ID (if using VirtIO GPU, 0 = not using GPU)
    pub gpu_resource_id: u32,
}

impl GraphicsBuffer {
    /// Create a new graphics buffer
    pub fn new(
        handle: BufferHandle,
        width: u32,
        height: u32,
        format: crate::graphics::surface::SurfaceFormat,
        buffer_type: BufferType,
        flags: BufferFlags,
    ) -> Result<Self, i32> {
        let bytes_per_pixel = format.bytes_per_pixel();
        let stride = (width as usize * bytes_per_pixel).next_multiple_of(64); // 64-byte alignment
        let size = stride * height as usize;
        
        // Allocate buffer memory
        // For scanout buffers, we might use special memory regions
        let addr = if flags.scanout {
            // Allocate from scanout memory pool (if available)
            crate::subsystems::mm::kalloc(size).ok_or(ENOMEM)?
        } else {
            crate::subsystems::mm::kalloc(size).ok_or(ENOMEM)?
        };
        
        Ok(Self {
            handle,
            addr,
            size,
            width,
            height,
            stride: stride as u32,
            format,
            buffer_type,
            flags,
            ref_count: AtomicU32::new(1),
            gpu_resource_id: 0, // Not using GPU by default
        })
    }
    
    /// Acquire buffer (increment reference count)
    pub fn acquire(&self) {
        self.ref_count.fetch_add(1, Ordering::Relaxed);
    }
    
    /// Release buffer (decrement reference count)
    pub fn release(&self) -> bool {
        let count = self.ref_count.fetch_sub(1, Ordering::Acquire);
        if count == 1 {
            // Last reference - free the buffer
            unsafe {
                crate::subsystems::mm::kfree(self.addr, self.size);
            }
            true
        } else {
            false
        }
    }
}

/// Buffer manager - manages graphics buffers
pub struct BufferManager {
    /// Next buffer handle
    next_handle: AtomicU32,
    /// Buffers by handle
    buffers: Mutex<alloc::collections::BTreeMap<BufferHandle, GraphicsBuffer>>,
}

impl BufferManager {
    /// Create a new buffer manager
    pub fn new() -> Self {
        Self {
            next_handle: AtomicU32::new(1),
            buffers: Mutex::new(alloc::collections::BTreeMap::new()),
        }
    }
    
    /// Allocate a new buffer (with optional VirtIO GPU acceleration)
    pub fn allocate_buffer(
        &self,
        width: u32,
        height: u32,
        format: crate::graphics::surface::SurfaceFormat,
        buffer_type: BufferType,
        flags: BufferFlags,
    ) -> Result<BufferHandle, i32> {
        let handle = self.next_handle.fetch_add(1, Ordering::SeqCst);
        
        // Try to use VirtIO GPU for scanout buffers if available
        if flags.scanout {
            if let Some(gpu_device) = crate::drivers::virtio_gpu::get_virtio_gpu() {
                return self.allocate_gpu_buffer(handle, width, height, format, buffer_type, flags, gpu_device);
            }
        }
        
        // Fallback to standard allocation
        let buffer = GraphicsBuffer::new(handle, width, height, format, buffer_type, flags)?;
        
        {
            let mut buffers = self.buffers.lock();
            buffers.insert(handle, buffer);
        }
        
        crate::println!("[graphics] Allocated buffer {} ({}x{}, type: {:?})", handle, width, height, buffer_type);
        Ok(handle)
    }
    
    /// Allocate buffer using VirtIO GPU
    fn allocate_gpu_buffer(
        &self,
        handle: BufferHandle,
        width: u32,
        height: u32,
        format: crate::graphics::surface::SurfaceFormat,
        buffer_type: BufferType,
        flags: BufferFlags,
        gpu_device: &crate::subsystems::sync::Mutex<crate::drivers::virtio_gpu::VirtioGpuDevice>,
    ) -> Result<BufferHandle, i32> {
        // Convert format to VirtIO GPU format
        let gpu_format = match format {
            crate::graphics::surface::SurfaceFormat::ARGB8888 => 1, // VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM
            crate::graphics::surface::SurfaceFormat::RGBA8888 => 2, // VIRTIO_GPU_FORMAT_R8G8B8A8_UNORM
            crate::graphics::surface::SurfaceFormat::RGB565 => 3,   // VIRTIO_GPU_FORMAT_B5G6R5_UNORM
            _ => return Err(crate::reliability::errno::EINVAL),
        };
        
        // Create GPU resource
        let gpu_device_guard = gpu_device.lock();
        let resource_id = gpu_device_guard.create_resource_2d(width, height, gpu_format)?;
        drop(gpu_device_guard);
        
        // Allocate backing memory
        let bytes_per_pixel = format.bytes_per_pixel();
        let stride = (width as usize * bytes_per_pixel).next_multiple_of(64);
        let size = stride * height as usize;
        let addr = crate::subsystems::mm::kalloc(size).ok_or(crate::reliability::errno::ENOMEM)?;
        
        // Attach backing pages to GPU resource
        // In real implementation, we'd get physical pages and attach them
        let gpu_device_guard = gpu_device.lock();
        // For now, we'll attach a placeholder - in real implementation, we'd convert addr to pages
        gpu_device_guard.attach_backing(resource_id, vec![addr])?;
        drop(gpu_device_guard);
        
        // Create buffer with GPU resource ID
        let buffer = GraphicsBuffer {
            handle,
            addr,
            size,
            width,
            height,
            stride: stride as u32,
            format,
            buffer_type,
            flags,
            ref_count: AtomicU32::new(1),
            gpu_resource_id: resource_id,
        };
        
        {
            let mut buffers = self.buffers.lock();
            buffers.insert(handle, buffer);
        }
        
        crate::println!("[graphics] Allocated GPU-accelerated buffer {} ({}x{}, GPU resource: {})", handle, width, height, resource_id);
        Ok(handle)
    }
    
    /// Get buffer by handle
    pub fn get_buffer(&self, handle: BufferHandle) -> Option<GraphicsBuffer> {
        let buffers = self.buffers.lock();
        buffers.get(&handle).cloned()
    }
    
    /// Release a buffer
    pub fn release_buffer(&self, handle: BufferHandle) -> Result<(), i32> {
        let mut buffers = self.buffers.lock();
        let buffer = buffers.remove(&handle).ok_or(EINVAL)?;
        
        // Release GPU resource if using GPU
        if buffer.gpu_resource_id != 0 {
            if let Some(gpu_device) = crate::drivers::virtio_gpu::get_virtio_gpu() {
                let gpu_device_guard = gpu_device.lock();
                // In real implementation, we'd call resource_unref
                crate::println!("[graphics] Releasing GPU resource {} for buffer {}", buffer.gpu_resource_id, handle);
                drop(gpu_device_guard);
            }
        }
        
        buffer.release();
        
        crate::println!("[graphics] Released buffer {}", handle);
        Ok(())
    }
    
    /// Transfer buffer to GPU (for GPU-accelerated buffers)
    pub fn transfer_to_gpu(&self, handle: BufferHandle, x: u32, y: u32, width: u32, height: u32) -> Result<(), i32> {
        let buffers = self.buffers.lock();
        let buffer = buffers.get(&handle).ok_or(EINVAL)?;
        
        if buffer.gpu_resource_id == 0 {
            return Err(EINVAL); // Not a GPU buffer
        }
        
        if let Some(gpu_device) = crate::drivers::virtio_gpu::get_virtio_gpu() {
            let gpu_device_guard = gpu_device.lock();
            gpu_device_guard.transfer_to_host_2d(buffer.gpu_resource_id, x, y, width, height)?;
            drop(gpu_device_guard);
        }
        
        Ok(())
    }
    
    /// Flush GPU buffer (for GPU-accelerated buffers)
    pub fn flush_gpu_buffer(&self, handle: BufferHandle, x: u32, y: u32, width: u32, height: u32) -> Result<(), i32> {
        let buffers = self.buffers.lock();
        let buffer = buffers.get(&handle).ok_or(EINVAL)?;
        
        if buffer.gpu_resource_id == 0 {
            return Err(EINVAL); // Not a GPU buffer
        }
        
        if let Some(gpu_device) = crate::drivers::virtio_gpu::get_virtio_gpu() {
            let gpu_device_guard = gpu_device.lock();
            gpu_device_guard.flush_resource(buffer.gpu_resource_id, x, y, width, height)?;
            drop(gpu_device_guard);
        }
        
        Ok(())
    }
}

/// Global buffer manager instance
static BUFFER_MANAGER: Mutex<Option<BufferManager>> = Mutex::new(None);

/// Initialize buffer manager
pub fn init_buffer_manager() -> Result<(), i32> {
    let mut manager = BUFFER_MANAGER.lock();
    if manager.is_none() {
        *manager = Some(BufferManager::new());
        crate::println!("[graphics] Buffer manager initialized");
    }
    Ok(())
}

/// Get buffer manager
pub fn get_buffer_manager() -> &'static BufferManager {
    static INIT_ONCE: crate::subsystems::sync::Once = crate::subsystems::sync::Once::new();
    INIT_ONCE.call_once(|| {
        let mut manager = BUFFER_MANAGER.lock();
        if manager.is_none() {
            *manager = Some(BufferManager::new());
        }
    });
    
    unsafe {
        // Safety: We've initialized it above
        &*(BUFFER_MANAGER.lock().as_ref().unwrap() as *const BufferManager)
    }
}

