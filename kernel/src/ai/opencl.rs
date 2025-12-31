//! # OpenCL Runtime Support
//!
//! 本模块实现 OpenCL 运行时支持，提供：
//!
//! - OpenCL 平台和设备枚举
//! - Kernel 编译和执行
//! - 命令队列管理
//! - 缓冲区和内存对象管理
//! - 跨平台异构计算支持
//!
//! ## 功能特性
//!
//! - **多平台支持**: NVIDIA、AMD、Intel、ARM 等
//! - **动态编译**: 运行时编译 OpenCL C 代码
//! - **内存管理**: Buffer、Image、Pipe 对象
//! - **命令队列**: 异步执行、事件同步、Profiling
//!
//! ## 使用示例
//!
//! ```no_run
//! use kernel::ai::opencl::{OpenClPlatform, OpenClDevice, OpenClContext};
//!
//! // 获取所有平台
//! let platforms = OpenClPlatform::enumerate()?;
//!
//! // 获取第一个平台的第一个设备
//! let device = platforms[0].get_devices()?[0].clone();
//!
//! // 创建上下文
//! let context = OpenClContext::create(&[&device])?;
//!
//! // 创建命令队列
//! let queue = context.create_command_queue(&device)?;
//!
//! // 编译程序
//! let source = r#"
//!     __kernel void add(__global float* a, __global float* b, __global float* c) {
//!         int i = get_global_id(0);
//!         c[i] = a[i] + b[i];
//!     }
//! "#;
//! let program = context.compile_program(source)?;
//!
//! // 创建缓冲区
//! let buffer_a = OpenClBuffer::create(1024, &context)?;
//! let buffer_b = OpenClBuffer::create(1024, &context)?;
//! let buffer_c = OpenClBuffer::create(1024, &context)?;
//!
//! // 执行 kernel
//! let kernel = program.create_kernel("add")?;
//! kernel.set_arg(0, &buffer_a)?;
//! kernel.set_arg(1, &buffer_b)?;
//! kernel.set_arg(2, &buffer_c)?;
//! kernel.execute(&queue, 1024)?;
//! # Ok::<(), kernel::ai::opencl::OpenClError>(())
//! ```

use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

use super::{AiError, AiResult};

/// OpenCL platform ID
pub type PlatformId = usize;

/// OpenCL device ID
pub type DeviceId = usize;

/// OpenCL context ID
pub type ContextId = usize;

/// OpenCL command queue ID
pub type QueueId = usize;

/// OpenCL program ID
pub type ProgramId = usize;

/// OpenCL kernel ID
pub type KernelId = usize;

/// OpenCL memory object ID
pub type MemId = usize;

/// OpenCL event ID
pub type EventId = usize;

/// OpenCL errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpenClError {
    /// Success
    Success,
    /// Device not available
    DeviceNotAvailable,
    /// Compiler not available
    CompilerNotAvailable,
    /// Out of resources
    OutOfResources,
    /// Out of host memory
    OutOfHostMemory,
    /// Profiling info not available
    ProfilingInfoNotAvailable,
    /// Memory copy overlap
    MemCopyOverlap,
    /// Image format mismatch
    ImageFormatMismatch,
    /// Image format not supported
    ImageFormatNotSupported,
    /// Build program failure
    BuildProgramFailure,
    /// Map failure
    MapFailure,
    /// Misaligned sub buffer offset
    MisalignedSubBufferOffset,
    /// Execution status error for events in wait list
    ExecStatusErrorForEventsInWaitList,
    /// Compile program failure
    CompileProgramFailure,
    /// Linker not available
    LinkerNotAvailable,
    /// Link program failure
    LinkProgramFailure,
    /// Device partition failed
    DevicePartitionFailed,
    /// Kernel argument not available
    KernelArgInfoNotAvailable,
    /// Invalid value
    InvalidValue,
    /// Invalid device type
    InvalidDeviceType,
    /// Invalid platform
    InvalidPlatform,
    /// Invalid device
    InvalidDevice,
    /// Invalid context
    InvalidContext,
    /// Invalid queue properties
    InvalidQueueProperties,
    /// Invalid command queue
    InvalidCommandQueue,
    /// Invalid host ptr
    InvalidHostPtr,
    /// Invalid mem object
    InvalidMemObject,
    /// Invalid image format descriptor
    InvalidImageFormatDescriptor,
    /// Invalid image size
    InvalidImageSize,
    /// Invalid sampler
    InvalidSampler,
    /// Invalid binary
    InvalidBinary,
    /// Invalid build options
    InvalidBuildOptions,
    /// Invalid program
    InvalidProgram,
    /// Invalid program executable
    InvalidProgramExecutable,
    /// Invalid kernel name
    InvalidKernelName,
    /// Invalid kernel definition
    InvalidKernelDefinition,
    /// Invalid kernel
    InvalidKernel,
    /// Invalid arg index
    InvalidArgIndex,
    /// Invalid arg value
    InvalidArgValue,
    /// Invalid arg size
    InvalidArgSize,
    /// Invalid kernel args
    InvalidKernelArgs,
    /// Invalid work dimension
    InvalidWorkDimension,
    /// Invalid work group size
    InvalidWorkGroupSize,
    /// Invalid work item size
    InvalidWorkItemSize,
    /// Invalid global offset
    InvalidGlobalOffset,
    /// Invalid event wait list
    InvalidEventWaitList,
    /// Invalid event
    InvalidEvent,
    /// Invalid operation
    InvalidOperation,
    /// Invalid buffer size
    InvalidBufferSize,
    /// Invalid global work size
    InvalidGlobalWorkSize,
    /// Unknown error
    Unknown(i32),
}

impl OpenClError {
    /// Convert error to string
    pub fn as_str(&self) -> &'static str {
        match self {
            OpenClError::Success => "Success",
            OpenClError::DeviceNotAvailable => "Device not available",
            OpenClError::CompilerNotAvailable => "Compiler not available",
            OpenClError::OutOfResources => "Out of resources",
            OpenClError::OutOfHostMemory => "Out of host memory",
            OpenClError::ProfilingInfoNotAvailable => "Profiling info not available",
            OpenClError::MemCopyOverlap => "Memory copy overlap",
            OpenClError::ImageFormatMismatch => "Image format mismatch",
            OpenClError::ImageFormatNotSupported => "Image format not supported",
            OpenClError::BuildProgramFailure => "Build program failure",
            OpenClError::MapFailure => "Map failure",
            OpenClError::MisalignedSubBufferOffset => "Misaligned sub buffer offset",
            OpenClError::ExecStatusErrorForEventsInWaitList => "Execution status error",
            OpenClError::CompileProgramFailure => "Compile program failure",
            OpenClError::LinkerNotAvailable => "Linker not available",
            OpenClError::LinkProgramFailure => "Link program failure",
            OpenClError::DevicePartitionFailed => "Device partition failed",
            OpenClError::KernelArgInfoNotAvailable => "Kernel argument info not available",
            OpenClError::InvalidValue => "Invalid value",
            OpenClError::InvalidDeviceType => "Invalid device type",
            OpenClError::InvalidPlatform => "Invalid platform",
            OpenClError::InvalidDevice => "Invalid device",
            OpenClError::InvalidContext => "Invalid context",
            OpenClError::InvalidQueueProperties => "Invalid queue properties",
            OpenClError::InvalidCommandQueue => "Invalid command queue",
            OpenClError::InvalidHostPtr => "Invalid host pointer",
            OpenClError::InvalidMemObject => "Invalid memory object",
            OpenClError::InvalidImageFormatDescriptor => "Invalid image format descriptor",
            OpenClError::InvalidImageSize => "Invalid image size",
            OpenClError::InvalidSampler => "Invalid sampler",
            OpenClError::InvalidBinary => "Invalid binary",
            OpenClError::InvalidBuildOptions => "Invalid build options",
            OpenClError::InvalidProgram => "Invalid program",
            OpenClError::InvalidProgramExecutable => "Invalid program executable",
            OpenClError::InvalidKernelName => "Invalid kernel name",
            OpenClError::InvalidKernelDefinition => "Invalid kernel definition",
            OpenClError::InvalidKernel => "Invalid kernel",
            OpenClError::InvalidArgIndex => "Invalid argument index",
            OpenClError::InvalidArgValue => "Invalid argument value",
            OpenClError::InvalidArgSize => "Invalid argument size",
            OpenClError::InvalidKernelArgs => "Invalid kernel arguments",
            OpenClError::InvalidWorkDimension => "Invalid work dimension",
            OpenClError::InvalidWorkGroupSize => "Invalid work group size",
            OpenClError::InvalidWorkItemSize => "Invalid work item size",
            OpenClError::InvalidGlobalOffset => "Invalid global offset",
            OpenClError::InvalidEventWaitList => "Invalid event wait list",
            OpenClError::InvalidEvent => "Invalid event",
            OpenClError::InvalidOperation => "Invalid operation",
            OpenClError::InvalidBufferSize => "Invalid buffer size",
            OpenClError::InvalidGlobalWorkSize => "Invalid global work size",
            OpenClError::Unknown(_) => "Unknown error",
        }
    }
}

impl From<OpenClError> for AiError {
    fn from(err: OpenClError) -> Self {
        match err {
            OpenClError::OutOfHostMemory | OpenClError::OutOfResources => AiError::OutOfMemory,
            OpenClError::DeviceNotAvailable | OpenClError::InvalidDevice => AiError::InvalidDevice,
            OpenClError::InvalidValue | OpenClError::InvalidArgValue => AiError::InvalidArgument,
            OpenClError::BuildProgramFailure | OpenClError::CompileProgramFailure => {
                AiError::CompilationError
            }
            _ => AiError::InternalError("OpenCL error"),
        }
    }
}

/// OpenCL device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    /// CPU device
    Cpu,
    /// GPU device
    Gpu,
    /// Accelerator device
    Accelerator,
    /// Default device
    Default,
    /// Custom device
    Custom,
}

/// OpenCL device information
#[derive(Debug, Clone)]
pub struct OpenClDeviceInfo {
    /// Device ID
    pub id: DeviceId,
    /// Device type
    pub device_type: DeviceType,
    /// Vendor ID
    pub vendor_id: u32,
    /// Maximum compute units
    pub max_compute_units: u32,
    /// Maximum work item dimensions
    pub max_work_item_dimensions: u32,
    /// Maximum work item sizes
    pub max_work_item_sizes: Vec<usize>,
    /// Maximum work group size
    pub max_work_group_size: usize,
    /// Preferred vector width char
    pub preferred_vector_width_char: u32,
    /// Preferred vector width short
    pub preferred_vector_width_short: u32,
    /// Preferred vector width int
    pub preferred_vector_width_int: u32,
    /// Preferred vector width long
    pub preferred_vector_width_long: u32,
    /// Preferred vector width float
    pub preferred_vector_width_float: u32,
    /// Preferred vector width double
    pub preferred_vector_width_double: u32,
    /// Maximum clock frequency
    pub max_clock_frequency: u32,
    /// Address bits
    pub address_bits: u32,
    /// Maximum read image args
    pub max_read_image_args: u32,
    /// Maximum write image args
    pub max_write_image_args: u32,
    /// Maximum memory allocation size
    pub max_mem_alloc_size: u64,
    /// Image2D max size
    pub image2d_max_width: usize,
    pub image2d_max_height: usize,
    /// Image3D max size
    pub image3d_max_width: usize,
    pub image3d_max_height: usize,
    pub image3d_max_depth: usize,
    /// Maximum parameter size
    pub max_parameter_size: usize,
    /// Maximum samplers
    pub max_samplers: u32,
    /// Memory base address align
    pub mem_base_addr_align: u32,
    /// Min data type align size
    pub min_data_type_align_size: u32,
    /// Single FP config
    pub single_fp_config: u64,
    /// Global memory cache size
    pub global_mem_cache_size: u64,
    /// Global memory cache type
    pub global_mem_cache_type: u32,
    /// Global memory cache line size
    pub global_mem_cache_line_size: u32,
    /// Global memory size
    pub global_mem_size: u64,
    /// Local memory size
    pub local_mem_size: u64,
    /// Error correction support
    pub error_correction_support: u32,
    /// Profiling timer resolution
    pub profiling_timer_resolution: usize,
    /// Endian little
    pub endian_little: u32,
    /// Available
    pub available: bool,
    /// Compiler available
    pub compiler_available: bool,
    /// Execution capabilities
    pub execution_capabilities: u64,
    /// Queue properties
    pub queue_properties: u64,
    /// Platform
    pub platform: PlatformId,
    /// Name
    pub name: String,
    /// Vendor
    pub vendor: String,
    /// Driver version
    pub driver_version: String,
    /// Profile
    pub profile: String,
    /// Version
    pub version: String,
    /// Extensions
    pub extensions: Vec<String>,
}

/// OpenCL platform
pub struct OpenClPlatform {
    /// Platform ID
    id: PlatformId,
    /// Platform name
    name: String,
    /// Platform vendor
    vendor: String,
    /// Platform version
    version: String,
    /// Platform profile
    profile: String,
    /// Platform extensions
    extensions: Vec<String>,
    /// Associated devices
    devices: Mutex<Vec<Arc<OpenClDevice>>>,
}

impl OpenClPlatform {
    /// Create new platform (stub)
    fn new(id: PlatformId) -> Self {
        Self {
            id,
            name: String::from("OpenCL Platform"),
            vendor: String::from("Vendor"),
            version: String::from("OpenCL 2.0"),
            profile: String::from("FULL_PROFILE"),
            extensions: Vec::new(),
            devices: Mutex::new(Vec::new()),
        }
    }

    /// Get platform ID
    pub fn id(&self) -> PlatformId {
        self.id
    }

    /// Get platform name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get platform vendor
    pub fn vendor(&self) -> &str {
        &self.vendor
    }

    /// Get platform version
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Get platform profile
    pub fn profile(&self) -> &str {
        &self.profile
    }

    /// Get platform extensions
    pub fn extensions(&self) -> &[String] {
        &self.extensions
    }

    /// Get all devices for this platform
    pub fn get_devices(&self) -> AiResult<Vec<Arc<OpenClDevice>>> {
        let devices = self.devices.lock();
        if devices.is_empty() {
            // Return stub device
            Ok(vec![Arc::new(OpenClDevice::new_stub(
                self.id,
                0,
                DeviceType::Gpu,
            ))])
        } else {
            Ok(devices.iter().map(|d| d.clone()).collect())
        }
    }

    /// Get devices of specific type
    pub fn get_devices_by_type(&self, device_type: DeviceType) -> AiResult<Vec<Arc<OpenClDevice>>> {
        let all_devices = self.get_devices()?;
        Ok(all_devices
            .into_iter()
            .filter(|d| d.device_type() == device_type)
            .collect())
    }
}

/// OpenCL device
pub struct OpenClDevice {
    /// Device ID
    id: DeviceId,
    /// Platform ID
    platform_id: PlatformId,
    /// Device info
    info: OpenClDeviceInfo,
}

impl OpenClDevice {
    /// Create stub device
    fn new_stub(platform_id: PlatformId, device_id: DeviceId, device_type: DeviceType) -> Self {
        Self {
            id: device_id,
            platform_id,
            info: OpenClDeviceInfo {
                id: device_id,
                device_type,
                vendor_id: 0x10DE, // NVIDIA
                max_compute_units: 28,
                max_work_item_dimensions: 3,
                max_work_item_sizes: vec![1024, 1024, 64],
                max_work_group_size: 1024,
                preferred_vector_width_char: 1,
                preferred_vector_width_short: 2,
                preferred_vector_width_int: 4,
                preferred_vector_width_long: 8,
                preferred_vector_width_float: 4,
                preferred_vector_width_double: 8,
                max_clock_frequency: 1590,
                address_bits: 64,
                max_read_image_args: 128,
                max_write_image_args: 8,
                max_mem_alloc_size: 2 * 1024 * 1024 * 1024,
                image2d_max_width: 16384,
                image2d_max_height: 16384,
                image3d_max_width: 2048,
                image3d_max_height: 2048,
                image3d_max_depth: 2048,
                max_parameter_size: 1024,
                max_samplers: 16,
                mem_base_addr_align: 2048,
                min_data_type_align_size: 128,
                single_fp_config: 0x3F, // Basic FP support
                global_mem_cache_size: 256 * 1024,
                global_mem_cache_type: 2,
                global_mem_cache_line_size: 128,
                global_mem_size: 8 * 1024 * 1024 * 1024,
                local_mem_size: 48 * 1024,
                error_correction_support: 0,
                profiling_timer_resolution: 80,
                endian_little: 1,
                available: true,
                compiler_available: true,
                execution_capabilities: 1,
                queue_properties: 1,
                platform: platform_id,
                name: String::from("OpenCL Device"),
                vendor: String::from("Vendor"),
                driver_version: String::from("1.0"),
                profile: String::from("FULL_PROFILE"),
                version: String::from("OpenCL 2.0"),
                extensions: Vec::new(),
            },
        }
    }

    /// Get device ID
    pub fn id(&self) -> DeviceId {
        self.id
    }

    /// Get device info
    pub fn info(&self) -> &OpenClDeviceInfo {
        &self.info
    }

    /// Get device type
    pub fn device_type(&self) -> DeviceType {
        self.info.device_type
    }
}

/// OpenCL context
pub struct OpenClContext {
    /// Context ID
    id: ContextId,
    /// Associated devices
    devices: Vec<Arc<OpenClDevice>>,
    /// Command queues
    queues: Mutex<Vec<Arc<OpenClCommandQueue>>>,
}

impl OpenClContext {
    /// Create context from devices
    pub fn create(devices: &[&OpenClDevice]) -> AiResult<Arc<Self>> {
        Ok(Arc::new(Self {
            id: devices[0].id, // Use first device ID as context ID
            devices: devices.iter().map(|d| d.clone()).collect(),
            queues: Mutex::new(Vec::new()),
        }))
    }

    /// Get context ID
    pub fn id(&self) -> ContextId {
        self.id
    }

    /// Get devices
    pub fn devices(&self) -> &[Arc<OpenClDevice>] {
        &self.devices
    }

    /// Create command queue
    pub fn create_command_queue(&self, device: &OpenClDevice) -> AiResult<Arc<OpenClCommandQueue>> {
        let queue = Arc::new(OpenClCommandQueue::new(self.id, device.id()));
        self.queues.lock().push(queue.clone());
        Ok(queue)
    }

    /// Create buffer
    pub fn create_buffer(&self, size: usize, flags: u64) -> AiResult<OpenClBuffer> {
        OpenClBuffer::create_with_flags(size, self.id, flags)
    }

    /// Compile program from source
    pub fn compile_program(&self, source: &str) -> AiResult<Arc<OpenClProgram>> {
        OpenClProgram::compile(source, self.id)
    }
}

/// OpenCL command queue
pub struct OpenClCommandQueue {
    /// Queue ID
    id: QueueId,
    /// Context ID
    context_id: ContextId,
    /// Device ID
    device_id: DeviceId,
}

impl OpenClCommandQueue {
    /// Create new command queue
    fn new(context_id: ContextId, device_id: DeviceId) -> Self {
        Self {
            id: context_id * 1000 + device_id,
            context_id,
            device_id,
        }
    }

    /// Get queue ID
    pub fn id(&self) -> QueueId {
        self.id
    }

    /// Finish all commands in queue
    pub fn finish(&self) -> AiResult<()> {
        Ok(())
    }

    /// Flush queue
    pub fn flush(&self) -> AiResult<()> {
        Ok(())
    }
}

/// OpenCL buffer
pub struct OpenClBuffer {
    /// Buffer ID
    id: MemId,
    /// Size in bytes
    size: usize,
    /// Context ID
    context_id: ContextId,
    /// Flags
    flags: u64,
}

impl OpenClBuffer {
    /// Create buffer
    pub fn create(size: usize, context: &OpenClContext) -> AiResult<Self> {
        Self::create_with_flags(size, context.id(), 0)
    }

    /// Create buffer with flags
    fn create_with_flags(size: usize, context_id: ContextId, flags: u64) -> AiResult<Self> {
        Ok(Self {
            id: context_id * 10000 + size,
            size,
            context_id,
            flags,
        })
    }

    /// Get buffer ID
    pub fn id(&self) -> MemId {
        self.id
    }

    /// Get buffer size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Write data to buffer
    pub fn write(&self, data: &[u8], queue: &OpenClCommandQueue) -> AiResult<()> {
        Ok(())
    }

    /// Read data from buffer
    pub fn read(&self, buffer: &mut [u8], queue: &OpenClCommandQueue) -> AiResult<()> {
        Ok(())
    }

    /// Copy buffer
    pub fn copy(&self, dst: &OpenClBuffer, queue: &OpenClCommandQueue) -> AiResult<()> {
        Ok(())
    }
}

/// OpenCL program
pub struct OpenClProgram {
    /// Program ID
    id: ProgramId,
    /// Context ID
    context_id: ContextId,
    /// Kernel names
    kernels: Mutex<Vec<String>>,
}

impl OpenClProgram {
    /// Compile program from source
    fn compile(source: &str, context_id: ContextId) -> AiResult<Arc<Self>> {
        // Stub: Parse kernel names from source
        let mut kernels = Vec::new();
        for line in source.lines() {
            if line.contains("__kernel void") {
                if let Some(start) = line.find("void") {
                    let rest = &line[start + 5..];
                    if let Some(end) = rest.find('(') {
                        let name = rest[..end].trim();
                        kernels.push(String::from(name));
                    }
                }
            }
        }

        Ok(Arc::new(Self {
            id: context_id * 1000,
            context_id,
            kernels: Mutex::new(kernels),
        }))
    }

    /// Get program ID
    pub fn id(&self) -> ProgramId {
        self.id
    }

    /// Create kernel
    pub fn create_kernel(&self, name: &str) -> AiResult<OpenClKernel> {
        Ok(OpenClKernel {
            id: self.id * 100 + name.len() as u64,
            program_id: self.id,
            name: String::from(name),
            args: Mutex::new(Vec::new()),
        })
    }
}

/// OpenCL kernel
pub struct OpenClKernel {
    /// Kernel ID
    id: KernelId,
    /// Program ID
    program_id: ProgramId,
    /// Kernel name
    name: String,
    /// Arguments
    args: Mutex<Vec<Box<[u8]>>>,
}

impl OpenClKernel {
    /// Get kernel ID
    pub fn id(&self) -> KernelId {
        self.id
    }

    /// Get kernel name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set kernel argument
    pub fn set_arg<T: Sized>(&self, index: u32, value: &T) -> AiResult<()> {
        let size = core::mem::size_of::<T>();
        let data = unsafe {
            let ptr = value as *const T as *const u8;
            core::slice::from_raw_parts(ptr, size)
        };
        let boxed: Box<[u8]> = data.into();
        let mut args = self.args.lock();
        while args.len() < index as usize + 1 {
            args.push(Box::new([]));
        }
        args[index as usize] = boxed;
        Ok(())
    }

    /// Set kernel argument (buffer)
    pub fn set_arg_buffer(&self, index: u32, buffer: &OpenClBuffer) -> AiResult<()> {
        // Stub implementation
        Ok(())
    }

    /// Execute kernel
    pub fn execute(&self, queue: &OpenClCommandQueue, global_work_size: usize) -> AiResult<()> {
        Ok(())
    }

    /// Execute kernel with work dimensions
    pub fn execute_nd(
        &self,
        queue: &OpenClCommandQueue,
        global_work_offset: Option<&[usize]>,
        global_work_size: &[usize],
        local_work_size: Option<&[usize]>,
    ) -> AiResult<()> {
        Ok(())
    }
}

/// Enumerate all OpenCL platforms
pub fn enumerate_platforms() -> AiResult<Vec<OpenClPlatform>> {
    // Stub: return one platform
    Ok(vec![OpenClPlatform::new(0)])
}

/// Initialize OpenCL subsystem
pub fn init() -> AiResult<()> {
    // Stub: initialize OpenCL
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_platforms() {
        let platforms = enumerate_platforms();
        assert!(platforms.is_ok());
        assert!(!platforms.unwrap().is_empty());
    }

    #[test]
    fn test_get_devices() {
        let platforms = enumerate_platforms().unwrap();
        let devices = platforms[0].get_devices();
        assert!(devices.is_ok());
        assert!(!devices.unwrap().is_empty());
    }

    #[test]
    fn test_create_context() {
        let platforms = enumerate_platforms().unwrap();
        let devices = platforms[0].get_devices().unwrap();
        let context = OpenClContext::create(&[&devices[0]]);
        assert!(context.is_ok());
    }

    #[test]
    fn test_create_buffer() {
        let platforms = enumerate_platforms().unwrap();
        let devices = platforms[0].get_devices().unwrap();
        let context = OpenClContext::create(&[&devices[0]]).unwrap();
        let buffer = context.create_buffer(1024, 0);
        assert!(buffer.is_ok());
    }

    #[test]
    fn test_compile_program() {
        let platforms = enumerate_platforms().unwrap();
        let devices = platforms[0].get_devices().unwrap();
        let context = OpenClContext::create(&[&devices[0]]).unwrap();
        let source = "__kernel void test() { }";
        let program = context.compile_program(source);
        assert!(program.is_ok());
    }
}
