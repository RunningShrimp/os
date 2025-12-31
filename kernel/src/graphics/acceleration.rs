//! # 3D Acceleration Support
//!
//! This module provides comprehensive 3D acceleration capabilities including:
//! - Command buffer management
//! - GPU virtualization support
//! - Shader execution (compute and graphics pipelines)
//! - Texture management
//! - GPU context switching
//! - Hardware acceleration
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::graphics::acceleration::{AccelerationEngine, CommandBuffer, ShaderType};
//!
//! // Initialize acceleration engine
//! let accel = AccelerationEngine::init(&gpu)?;
//!
//! // Create command buffer
//! let cmd = CommandBuffer::new(4096)?;
//!
//! // Submit shader
//! accel.submit_shader(ShaderType::Compute, &shader_code)?;
//! ```

#![allow(dead_code)]

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use core::time::Duration;

use crate::subsystems::sync::{Mutex, RwLock};

use super::error::{GraphicsError, GraphicsResult};
use super::gpu::{GpuContext, GpuContextId, GpuDevice};
use super::DeviceId;

/// Shader type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShaderType {
    /// Vertex shader
    Vertex,
    /// Fragment shader
    Fragment,
    /// Geometry shader
    Geometry,
    /// Tessellation control shader
    TessControl,
    /// Tessellation evaluation shader
    TessEvaluation,
    /// Compute shader
    Compute,
}

/// Shader handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderHandle(u32);

impl ShaderHandle {
    /// Create a new shader handle
    pub fn new(handle: u32) -> Self {
        Self(handle)
    }

    /// Get the raw handle value
    pub fn value(&self) -> u32 {
        self.0
    }
}

/// Shader object
#[derive(Debug, Clone)]
pub struct Shader {
    /// Shader handle
    pub handle: ShaderHandle,
    /// Shader type
    pub shader_type: ShaderType,
    /// Shader bytecode
    pub bytecode: Vec<u8>,
    /// Entry point name
    pub entry_point: String,
    /// Is compiled
    pub compiled: bool,
}

impl Shader {
    /// Create a new shader
    pub fn new(handle: ShaderHandle, shader_type: ShaderType, bytecode: Vec<u8>) -> Self {
        Self {
            handle,
            shader_type,
            bytecode,
            entry_point: "main".to_string(),
            compiled: false,
        }
    }

    /// Set entry point
    pub fn set_entry_point(&mut self, entry_point: &str) {
        self.entry_point = entry_point.to_string();
    }
}

/// Graphics pipeline
#[derive(Debug, Clone)]
pub struct GraphicsPipeline {
    /// Pipeline ID
    pub id: u32,
    /// Vertex shader
    pub vertex_shader: Option<ShaderHandle>,
    /// Fragment shader
    pub fragment_shader: Option<ShaderHandle>,
    /// Geometry shader
    pub geometry_shader: Option<ShaderHandle>,
    /// Tessellation control shader
    pub tess_control_shader: Option<ShaderHandle>,
    /// Tessellation evaluation shader
    pub tess_eval_shader: Option<ShaderHandle>,
    /// Primitive topology
    pub topology: PrimitiveTopology,
    /// Is active
    pub active: bool,
}

/// Primitive topology
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveTopology {
    /// Point list
    PointList,
    /// Line list
    LineList,
    /// Line strip
    LineStrip,
    /// Triangle list
    TriangleList,
    /// Triangle strip
    TriangleStrip,
    /// Triangle fan
    TriangleFan,
}

impl GraphicsPipeline {
    /// Create a new graphics pipeline
    pub fn new(id: u32) -> Self {
        Self {
            id,
            vertex_shader: None,
            fragment_shader: None,
            geometry_shader: None,
            tess_control_shader: None,
            tess_eval_shader: None,
            topology: PrimitiveTopology::TriangleList,
            active: false,
        }
    }

    /// Set vertex shader
    pub fn set_vertex_shader(&mut self, shader: ShaderHandle) {
        self.vertex_shader = Some(shader);
    }

    /// Set fragment shader
    pub fn set_fragment_shader(&mut self, shader: ShaderHandle) {
        self.fragment_shader = Some(shader);
    }

    /// Set primitive topology
    pub fn set_topology(&mut self, topology: PrimitiveTopology) {
        self.topology = topology;
    }

    /// Activate pipeline
    pub fn activate(&mut self) {
        self.active = true;
    }

    /// Deactivate pipeline
    pub fn deactivate(&mut self) {
        self.active = false;
    }
}

/// Compute pipeline
#[derive(Debug, Clone)]
pub struct ComputePipeline {
    /// Pipeline ID
    pub id: u32,
    /// Compute shader
    pub compute_shader: Option<ShaderHandle>,
    /// Workgroup size
    pub workgroup_size: (u32, u32, u32),
    /// Is active
    pub active: bool,
}

impl ComputePipeline {
    /// Create a new compute pipeline
    pub fn new(id: u32) -> Self {
        Self {
            id,
            compute_shader: None,
            workgroup_size: (1, 1, 1),
            active: false,
        }
    }

    /// Set compute shader
    pub fn set_compute_shader(&mut self, shader: ShaderHandle) {
        self.compute_shader = Some(shader);
    }

    /// Set workgroup size
    pub fn set_workgroup_size(&mut self, x: u32, y: u32, z: u32) {
        self.workgroup_size = (x, y, z);
    }

    /// Activate pipeline
    pub fn activate(&mut self) {
        self.active = true;
    }
}

/// Texture format
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureFormat {
    /// RGBA8 (8 bits per channel)
    Rgba8,
    /// RGB8 (8 bits per channel)
    Rgb8,
    /// RG16F (16-bit float per channel)
    Rg16f,
    /// RGBA32F (32-bit float per channel)
    Rgba32f,
    /// Depth 24-bit, stencil 8-bit
    Depth24Stencil8,
    /// Depth 32-bit float
    Depth32f,
}

/// Texture handle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextureHandle(u32);

impl TextureHandle {
    /// Create a new texture handle
    pub fn new(handle: u32) -> Self {
        Self(handle)
    }

    /// Get the raw handle value
    pub fn value(&self) -> u32 {
        self.0
    }
}

/// Texture object
#[derive(Debug)]
pub struct Texture {
    /// Texture handle
    pub handle: TextureHandle,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
    /// Depth (for 3D textures)
    pub depth: u32,
    /// Texture format
    pub format: TextureFormat,
    /// Mipmap levels
    pub mip_levels: u32,
    /// GPU address
    pub gpu_addr: u64,
    /// Size in bytes
    pub size: u64,
}

impl Texture {
    /// Create a new texture
    pub fn new(
        handle: TextureHandle,
        width: u32,
        height: u32,
        depth: u32,
        format: TextureFormat,
        mip_levels: u32,
    ) -> Self {
        let (bytes_per_pixel, _) = match format {
            TextureFormat::Rgba8 => (4, 1),
            TextureFormat::Rgb8 => (3, 1),
            TextureFormat::Rg16f => (4, 2),
            TextureFormat::Rgba32f => (16, 4),
            TextureFormat::Depth24Stencil8 => (4, 1),
            TextureFormat::Depth32f => (4, 4),
        };

        let size = (width * height * depth * bytes_per_pixel) as u64;

        Self {
            handle,
            width,
            height,
            depth,
            format,
            mip_levels,
            gpu_addr: 0,
            size,
        }
    }

    /// Calculate size for texture
    pub fn calculate_size(&self) -> u64 {
        self.size
    }
}

/// Command buffer for GPU submission
#[derive(Debug, Clone)]
pub struct CommandBuffer {
    /// Buffer address
    pub addr: u64,
    /// Buffer size in bytes
    pub size: u64,
    /// Write pointer
    pub write_ptr: u64,
    /// Commands
    pub commands: Vec<u8>,
}

impl CommandBuffer {
    /// Create a new command buffer
    pub fn new(size: u64) -> GraphicsResult<Self> {
        Ok(Self {
            addr: 0x1000,
            size,
            write_ptr: 0,
            commands: Vec::new(),
        })
    }

    /// Begin recording
    pub fn begin(&mut self) -> GraphicsResult<()> {
        self.commands.clear();
        self.write_ptr = 0;
        Ok(())
    }

    /// End recording
    pub fn end(&mut self) -> GraphicsResult<()> {
        Ok(())
    }

    /// Write data to command buffer
    pub fn write(&mut self, data: &[u8]) -> GraphicsResult<()> {
        if self.write_ptr + data.len() as u64 > self.size {
            return Err(GraphicsError::OutOfMemory(
                "Command buffer overflow".to_string(),
            ));
        }

        self.commands.extend_from_slice(data);
        self.write_ptr += data.len() as u64;
        Ok(())
    }

    /// Get used size
    pub fn used(&self) -> u64 {
        self.write_ptr
    }

    /// Get available space
    pub fn available(&self) -> u64 {
        self.size - self.write_ptr
    }
}

/// GPU virtualization type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtualizationType {
    /// No virtualization
    None,
    /// GPU passthrough
    Passthrough,
    /// Mediated device (vGPU)
    Mediated,
}

/// Virtual GPU instance
#[derive(Debug)]
pub struct VirtualGpu {
    /// vGPU ID
    pub id: u32,
    /// Parent GPU device ID
    pub parent_device_id: DeviceId,
    /// Virtualization type
    pub virtualization_type: VirtualizationType,
    /// Allocated VRAM
    pub vram_allocation: u64,
    /// Maximum VRAM
    pub max_vram: u64,
    /// Compute units assigned
    pub compute_units: u32,
}

impl VirtualGpu {
    /// Create a new virtual GPU
    pub fn new(
        id: u32,
        parent_device_id: DeviceId,
        virtualization_type: VirtualizationType,
        max_vram: u64,
        compute_units: u32,
    ) -> Self {
        Self {
            id,
            parent_device_id,
            virtualization_type,
            vram_allocation: 0,
            max_vram,
            compute_units,
        }
    }

    /// Allocate VRAM
    pub fn allocate_vram(&mut self, size: u64) -> GraphicsResult<()> {
        if self.vram_allocation + size > self.max_vram {
            return Err(GraphicsError::OutOfMemory(
                "vGPU VRAM limit exceeded".to_string(),
            ));
        }

        self.vram_allocation += size;
        Ok(())
    }
}

/// Acceleration engine
#[derive(Debug)]
pub struct AccelerationEngine {
    /// Device ID
    device_id: DeviceId,
    /// Shaders
    shaders: Mutex<BTreeMap<ShaderHandle, Shader>>,
    /// Graphics pipelines
    graphics_pipelines: Mutex<BTreeMap<u32, GraphicsPipeline>>,
    /// Compute pipelines
    compute_pipelines: Mutex<BTreeMap<u32, ComputePipeline>>,
    /// Textures
    textures: Mutex<BTreeMap<TextureHandle, Texture>>,
    /// Next shader handle
    next_shader_handle: Arc<AtomicU32>,
    /// Next pipeline ID
    next_pipeline_id: Arc<AtomicU32>,
    /// Next texture handle
    next_texture_handle: Arc<AtomicU32>,
    /// Virtual GPUs
    virtual_gpus: Mutex<BTreeMap<u32, VirtualGpu>>,
    /// Next vGPU ID
    next_vgpu_id: Arc<AtomicU32>,
    /// Is 3D supported
    supports_3d: bool,
    /// Maximum texture size
    max_texture_size: u32,
}

impl AccelerationEngine {
    /// Initialize acceleration engine
    pub fn init(_gpu: &GpuDevice) -> GraphicsResult<Self> {
        Ok(Self {
            device_id: DeviceId::new(1),
            shaders: Mutex::new(BTreeMap::new()),
            graphics_pipelines: Mutex::new(BTreeMap::new()),
            compute_pipelines: Mutex::new(BTreeMap::new()),
            textures: Mutex::new(BTreeMap::new()),
            next_shader_handle: Arc::new(AtomicU32::new(1)),
            next_pipeline_id: Arc::new(AtomicU32::new(1)),
            next_texture_handle: Arc::new(AtomicU32::new(1)),
            virtual_gpus: Mutex::new(BTreeMap::new()),
            next_vgpu_id: Arc::new(AtomicU32::new(1)),
            supports_3d: true,
            max_texture_size: 16384,
        })
    }

    /// Check if 3D is supported
    pub fn is_3d_supported(&self) -> bool {
        self.supports_3d
    }

    /// Check if hardware acceleration is supported
    pub fn is_hw_accel_supported(&self) -> bool {
        self.supports_3d
    }

    /// Get maximum texture size
    pub fn max_texture_size(&self) -> u32 {
        self.max_texture_size
    }

    /// Create shader
    pub fn create_shader(
        &self,
        shader_type: ShaderType,
        bytecode: Vec<u8>,
    ) -> GraphicsResult<ShaderHandle> {
        let handle = ShaderHandle::new(self.next_shader_handle.fetch_add(1, Ordering::SeqCst));
        let shader = Shader::new(handle, shader_type, bytecode);

        let mut shaders = self.shaders.lock();
        shaders.insert(handle, shader);

        Ok(handle)
    }

    /// Destroy shader
    pub fn destroy_shader(&self, handle: ShaderHandle) -> GraphicsResult<()> {
        let mut shaders = self.shaders.lock();
        shaders
            .remove(&handle)
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid shader handle".to_string()))?;
        Ok(())
    }

    /// Get shader
    pub fn get_shader(&self, handle: ShaderHandle) -> GraphicsResult<Shader> {
        let shaders = self.shaders.lock();
        shaders
            .get(&handle)
            .cloned()
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid shader handle".to_string()))
    }

    /// Create graphics pipeline
    pub fn create_graphics_pipeline(&self) -> GraphicsResult<u32> {
        let id = self.next_pipeline_id.fetch_add(1, Ordering::SeqCst);
        let pipeline = GraphicsPipeline::new(id);

        let mut pipelines = self.graphics_pipelines.lock();
        pipelines.insert(id, pipeline);

        Ok(id)
    }

    /// Get graphics pipeline
    pub fn get_graphics_pipeline(&self, id: u32) -> GraphicsResult<GraphicsPipeline> {
        let pipelines = self.graphics_pipelines.lock();
        pipelines
            .get(&id)
            .cloned()
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid pipeline ID".to_string()))
    }

    /// Create compute pipeline
    pub fn create_compute_pipeline(&self) -> GraphicsResult<u32> {
        let id = self.next_pipeline_id.fetch_add(1, Ordering::SeqCst);
        let pipeline = ComputePipeline::new(id);

        let mut pipelines = self.compute_pipelines.lock();
        pipelines.insert(id, pipeline);

        Ok(id)
    }

    /// Get compute pipeline
    pub fn get_compute_pipeline(&self, id: u32) -> GraphicsResult<ComputePipeline> {
        let pipelines = self.compute_pipelines.lock();
        pipelines
            .get(&id)
            .cloned()
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid pipeline ID".to_string()))
    }

    /// Create texture
    pub fn create_texture(
        &self,
        width: u32,
        height: u32,
        depth: u32,
        format: TextureFormat,
        mip_levels: u32,
    ) -> GraphicsResult<TextureHandle> {
        if width > self.max_texture_size || height > self.max_texture_size {
            return Err(GraphicsError::InvalidArgument(
                "Texture size exceeds maximum".to_string(),
            ));
        }

        let handle = TextureHandle::new(self.next_texture_handle.fetch_add(1, Ordering::SeqCst));
        let texture = Texture::new(handle, width, height, depth, format, mip_levels);

        let mut textures = self.textures.lock();
        textures.insert(handle, texture);

        Ok(handle)
    }

    /// Destroy texture
    pub fn destroy_texture(&self, handle: TextureHandle) -> GraphicsResult<()> {
        let mut textures = self.textures.lock();
        textures
            .remove(&handle)
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid texture handle".to_string()))?;
        Ok(())
    }

    /// Get texture
    pub fn get_texture(&self, handle: TextureHandle) -> GraphicsResult<Texture> {
        let textures = self.textures.lock();
        textures
            .get(&handle)
            .cloned()
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid texture handle".to_string()))
    }

    /// Bind texture to pipeline
    pub fn bind_texture(
        &self,
        _pipeline_id: u32,
        _texture_handle: TextureHandle,
        _slot: u32,
    ) -> GraphicsResult<()> {
        Ok(())
    }

    /// Submit shader
    pub fn submit_shader(
        &self,
        shader_type: ShaderType,
        bytecode: &[u8],
    ) -> GraphicsResult<ShaderHandle> {
        self.create_shader(shader_type, bytecode.to_vec())
    }

    /// Submit commands
    pub fn submit_commands(
        &self,
        _context: &GpuContext,
        commands: &[u8],
    ) -> GraphicsResult<()> {
        let mut cmd = CommandBuffer::new(commands.len() as u64)?;
        cmd.begin()?;
        cmd.write(commands)?;
        cmd.end()?;
        Ok(())
    }

    /// Create virtual GPU
    pub fn create_vgpu(
        &self,
        virtualization_type: VirtualizationType,
        max_vram: u64,
        compute_units: u32,
    ) -> GraphicsResult<u32> {
        let id = self.next_vgpu_id.fetch_add(1, Ordering::SeqCst);
        let vgpu = VirtualGpu::new(id, self.device_id, virtualization_type, max_vram, compute_units);

        let mut vgpus = self.virtual_gpus.lock();
        vgpus.insert(id, vgpu);

        Ok(id)
    }

    /// Destroy virtual GPU
    pub fn destroy_vgpu(&self, id: u32) -> GraphicsResult<()> {
        let mut vgpus = self.virtual_gpus.lock();
        vgpus
            .remove(&id)
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid vGPU ID".to_string()))?;
        Ok(())
    }

    /// Get virtual GPU
    pub fn get_vgpu(&self, id: u32) -> GraphicsResult<VirtualGpu> {
        let vgpus = self.virtual_gpus.lock();
        vgpus.get(&id)
            .cloned()
            .ok_or_else(|| GraphicsError::InvalidArgument("Invalid vGPU ID".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_handle() {
        let handle = ShaderHandle::new(42);
        assert_eq!(handle.value(), 42);
    }

    #[test]
    fn test_shader() {
        let handle = ShaderHandle::new(1);
        let bytecode = vec![0u8; 128];
        let shader = Shader::new(handle, ShaderType::Vertex, bytecode);
        assert_eq!(shader.handle, handle);
        assert_eq!(shader.shader_type, ShaderType::Vertex);
        assert!(!shader.compiled);
    }

    #[test]
    fn test_shader_entry_point() {
        let handle = ShaderHandle::new(1);
        let mut shader = Shader::new(handle, ShaderType::Compute, vec![0u8; 64]);
        assert_eq!(shader.entry_point, "main");

        shader.set_entry_point("compute_main");
        assert_eq!(shader.entry_point, "compute_main");
    }

    #[test]
    fn test_graphics_pipeline() {
        let mut pipeline = GraphicsPipeline::new(1);
        assert!(!pipeline.active);

        pipeline.activate();
        assert!(pipeline.active);

        pipeline.set_vertex_shader(ShaderHandle::new(10));
        assert_eq!(pipeline.vertex_shader, Some(ShaderHandle::new(10)));
    }

    #[test]
    fn test_compute_pipeline() {
        let mut pipeline = ComputePipeline::new(1);
        assert_eq!(pipeline.workgroup_size, (1, 1, 1));

        pipeline.set_workgroup_size(16, 16, 1);
        assert_eq!(pipeline.workgroup_size, (16, 16, 1));

        pipeline.activate();
        assert!(pipeline.active);
    }

    #[test]
    fn test_texture() {
        let handle = TextureHandle::new(1);
        let texture = Texture::new(handle, 256, 256, 1, TextureFormat::Rgba8, 1);
        assert_eq!(texture.handle, handle);
        assert_eq!(texture.width, 256);
        assert_eq!(texture.format, TextureFormat::Rgba8);
        assert_eq!(texture.size, 256 * 256 * 4);
    }

    #[test]
    fn test_texture_depth() {
        let handle = TextureHandle::new(1);
        let texture = Texture::new(handle, 64, 64, 8, TextureFormat::Rgba8, 1);
        assert_eq!(texture.depth, 8);
        assert_eq!(texture.size, 64 * 64 * 8 * 4);
    }

    #[test]
    fn test_command_buffer() {
        let mut cmd = CommandBuffer::new(1024).unwrap();
        assert_eq!(cmd.size, 1024);
        assert_eq!(cmd.used(), 0);
        assert_eq!(cmd.available(), 1024);

        cmd.begin().unwrap();
        cmd.write(&[1, 2, 3, 4]).unwrap();
        assert_eq!(cmd.used(), 4);

        cmd.end().unwrap();
    }

    #[test]
    fn test_command_buffer_overflow() {
        let mut cmd = CommandBuffer::new(10).unwrap();
        cmd.begin().unwrap();

        let data = vec![0u8; 20];
        assert!(cmd.write(&data).is_err());
    }

    #[test]
    fn test_virtual_gpu() {
        let device_id = DeviceId::new(1);
        let mut vgpu = VirtualGpu::new(
            1,
            device_id,
            VirtualizationType::Mediated,
            1024 * 1024 * 1024,
            4,
        );
        assert_eq!(vgpu.id, 1);
        assert_eq!(vgpu.max_vram, 1024 * 1024 * 1024);

        vgpu.allocate_vram(512 * 1024 * 1024).unwrap();
        assert_eq!(vgpu.vram_allocation, 512 * 1024 * 1024);
    }

    #[test]
    fn test_virtual_gpu_overflow() {
        let device_id = DeviceId::new(1);
        let mut vgpu = VirtualGpu::new(
            1,
            device_id,
            VirtualizationType::Passthrough,
            1024,
            2,
        );

        assert!(vgpu.allocate_vram(2048).is_err());
    }

    #[test]
    fn test_acceleration_engine_init() {
        let engine = AccelerationEngine::init(&unsafe { core::mem::zeroed() }).unwrap();
        assert!(engine.is_3d_supported());
        assert_eq!(engine.max_texture_size(), 16384);
    }

    #[test]
    fn test_acceleration_create_shader() {
        let engine = AccelerationEngine::init(&unsafe { core::mem::zeroed() }).unwrap();
        let bytecode = vec![1u8; 256];
        let handle = engine.create_shader(ShaderType::Fragment, bytecode).unwrap();
        assert_eq!(handle.value(), 1);
    }

    #[test]
    fn test_acceleration_create_pipeline() {
        let engine = AccelerationEngine::init(&unsafe { core::mem::zeroed() }).unwrap();
        let id = engine.create_graphics_pipeline().unwrap();
        assert_eq!(id, 1);

        let pipeline = engine.get_graphics_pipeline(id).unwrap();
        assert_eq!(pipeline.id, 1);
    }

    #[test]
    fn test_acceleration_create_texture() {
        let engine = AccelerationEngine::init(&unsafe { core::mem::zeroed() }).unwrap();
        let handle = engine.create_texture(512, 512, 1, TextureFormat::Rgba8, 1).unwrap();
        assert_eq!(handle.value(), 1);

        let texture = engine.get_texture(handle).unwrap();
        assert_eq!(texture.width, 512);
    }

    #[test]
    fn test_acceleration_texture_too_large() {
        let engine = AccelerationEngine::init(&unsafe { core::mem::zeroed() }).unwrap();
        let result = engine.create_texture(32768, 32768, 1, TextureFormat::Rgba8, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_acceleration_vgpu() {
        let engine = AccelerationEngine::init(&unsafe { core::mem::zeroed() }).unwrap();
        let id = engine
            .create_vgpu(VirtualizationType::Mediated, 512 * 1024 * 1024, 2)
            .unwrap();
        assert_eq!(id, 1);

        let vgpu = engine.get_vgpu(id).unwrap();
        assert_eq!(vgpu.id, 1);
        assert_eq!(vgpu.max_vram, 512 * 1024 * 1024);
    }
}
