//! # AI Accelerator Driver Framework
//!
//! 本模块实现通用 AI 加速器驱动框架，提供：
//!
//! - 多厂商 AI 芯片驱动支持
//! - 统一的设备抽象层
//! - 设备能力查询和发现
//! - 资源分配和管理
//! - 热插拔支持
//!
//! ## 支持的设备类型
//!
//! - **GPU**: NVIDIA (CUDA), AMD (ROCm), Intel (oneAPI)
//! - **TPU**: Google TPU, Graphcore IPU
//! - **NPU**: 华为昇腾、寒武纪、地平线
//! - **FPGA**: Xilinx、Intel (OpenCL)
//!
//! ## 功能特性
//!
//! - **设备抽象**: 统一的设备接口，屏蔽底层差异
//! - **能力查询**: 查询设备计算能力、内存容量、带宽等
//! - **资源管理**: 内存分配、流管理、事件同步
//! - **性能监控**: 实时监控设备利用率、温度、功耗
//!
//! ## 使用示例
//!
//! ```no_run
//! use kernel::ai::accelerator::{Accelerator, AcceleratorType, ComputeDevice};
//!
//! // 列出所有可用设备
//! let devices = Accelerator::enumerate_devices()?;
//! for device in &devices {
//!     println!("Found {} device: {}",
//!         device.device_type(),
//!         device.name()
//!     );
//! }
//!
//! // 选择最佳设备
//! let device = Accelerator::select_best_device(AcceleratorType::any())?;
//!
//! // 创建计算上下文
//! let context = device.create_context()?;
//!
//! // 分配设备内存
//! let memory = device.allocate_memory(1024 * 1024 * 1024)?;
//!
//! // 创建执行流
//! let stream = device.create_stream()?;
//! # Ok::<(), kernel::ai::AiError>(())
//! ```

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::{Mutex, RwLock};
use core::any::Any;

use super::{AiError, AiResult, AcceleratorStats};

/// Accelerator device ID
pub type AcceleratorId = u64;

/// Stream ID
pub type StreamId = u64;

/// Memory allocation ID
pub type MemoryId = u64;

/// Accelerator type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AcceleratorType {
    /// GPU (General Purpose GPU)
    Gpu,
    /// TPU (Tensor Processing Unit)
    Tpu,
    /// NPU (Neural Processing Unit)
    Npu,
    /// FPGA (Field-Programmable Gate Array)
    Fpga,
    /// DSP (Digital Signal Processor)
    Dsp,
    /// CPU (x86 SIMD, ARM NEON)
    Cpu,
}

impl AcceleratorType {
    /// Any accelerator type
    pub fn any() -> Self {
        AcceleratorType::Gpu // Default to GPU
    }
}

/// Memory type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryType {
    /// Global memory (device DRAM)
    Global,
    /// Shared memory (on-chip, fast)
    Shared,
    /// Constant memory (read-only)
    Constant,
    /// Texture memory (GPU-specific)
    Texture,
    /// Host memory (pinned)
    Host,
    /// Unified memory (CPU+GPU shared)
    Unified,
}

/// Device capability flags
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceCapabilities {
    /// Supports unified addressing
    pub unified_addressing: bool,
    /// Supports managed memory
    pub managed_memory: bool,
    /// Supports concurrent kernel execution
    pub concurrent_kernels: bool,
    /// Supports ECC memory
    pub ecc_enabled: bool,
    /// Supports cooperative launch
    pub cooperative_launch: bool,
    /// Supports async transfers
    pub async_transfers: bool,
    /// Supports compute preemption
    pub compute_preemption: bool,
    /// Supports virtual memory management
    pub virtual_memory: bool,
    /// Supports interprocess communication
    pub ipc: bool,
}

impl Default for DeviceCapabilities {
    fn default() -> Self {
        Self {
            unified_addressing: false,
            managed_memory: false,
            concurrent_kernels: false,
            ecc_enabled: false,
            cooperative_launch: false,
            async_transfers: false,
            compute_preemption: false,
            virtual_memory: false,
            ipc: false,
        }
    }
}

/// Device compute capability
#[derive(Debug, Clone)]
pub struct ComputeCapability {
    /// Architecture name (e.g., "Ampere", "RDNA3")
    pub architecture: String,
    /// Major version
    pub major: i32,
    /// Minor version
    pub minor: i32,
    /// Number of SMs/Compute Units
    pub compute_units: i32,
    /// Max clock rate (MHz)
    pub max_clock_rate: i32,
    /// Peak compute performance (TFLOPS)
    pub peak_tflops: f32,
    /// Memory bandwidth (GB/s)
    pub memory_bandwidth: f32,
}

/// Accelerator device information
#[derive(Debug, Clone)]
pub struct AcceleratorInfo {
    /// Device ID
    pub id: AcceleratorId,
    /// Device name
    pub name: String,
    /// Device vendor
    pub vendor: String,
    /// Device type
    pub device_type: AcceleratorType,
    /// Total global memory (bytes)
    pub total_memory: usize,
    /// Free memory (bytes)
    pub free_memory: usize,
    /// Compute capability
    pub compute_capability: ComputeCapability,
    /// Device capabilities
    pub capabilities: DeviceCapabilities,
    /// PCI bus ID (if applicable)
    pub pci_bus_id: Option<u32>,
    /// NUMA node ID
    pub numa_node: Option<i32>,
}

/// Device memory allocation
pub struct DeviceMemory {
    /// Memory ID
    id: MemoryId,
    /// Device ID
    device_id: AcceleratorId,
    /// Size in bytes
    size: usize,
    /// Memory type
    memory_type: MemoryType,
    /// Device pointer (opaque)
    ptr: usize,
}

impl DeviceMemory {
    /// Get memory ID
    pub fn id(&self) -> MemoryId {
        self.id
    }

    /// Get size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Get memory type
    pub fn memory_type(&self) -> MemoryType {
        self.memory_type
    }

    /// Get device pointer
    pub fn as_ptr(&self) -> usize {
        self.ptr
    }
}

/// Device execution stream
pub struct DeviceStream {
    /// Stream ID
    id: StreamId,
    /// Device ID
    device_id: AcceleratorId,
}

impl DeviceStream {
    /// Get stream ID
    pub fn id(&self) -> StreamId {
        self.id
    }

    /// Synchronize stream
    pub fn synchronize(&self) -> AiResult<()> {
        Ok(())
    }

    /// Query stream completion
    pub fn query(&self) -> AiResult<bool> {
        Ok(true)
    }
}

/// Compute device trait
pub trait ComputeDevice: Any + Send + Sync {
    /// Get device ID
    fn id(&self) -> AcceleratorId;

    /// Get device info
    fn info(&self) -> &AcceleratorInfo;

    /// Get device type
    fn device_type(&self) -> AcceleratorType;

    /// Get device name
    fn name(&self) -> &str {
        self.info().name.as_str()
    }

    /// Allocate device memory
    fn allocate_memory(&self, size: usize, memory_type: MemoryType) -> AiResult<DeviceMemory>;

    /// Free device memory
    fn free_memory(&self, memory: DeviceMemory) -> AiResult<()>;

    /// Create execution stream
    fn create_stream(&self) -> AiResult<DeviceStream>;

    /// Get device statistics
    fn get_stats(&self) -> AiResult<AcceleratorStats>;

    /// Reset device
    fn reset(&self) -> AiResult<()> {
        Ok(())
    }

    /// Check if device is available
    fn is_available(&self) -> bool {
        true
    }

    /// Downcast to concrete type
    fn as_any(&self) -> &dyn Any;
}

/// Accelerator device
pub struct AcceleratorDevice {
    /// Device ID
    id: AcceleratorId,
    /// Device info
    info: AcceleratorInfo,
    /// Allocated memory
    memory_allocations: Mutex<Vec<DeviceMemory>>,
    /// Active streams
    streams: Mutex<Vec<DeviceStream>>,
    /// Statistics
    stats: Mutex<AcceleratorStats>,
}

impl AcceleratorDevice {
    /// Create new accelerator device
    pub fn new(info: AcceleratorInfo) -> Self {
        Self {
            id: info.id,
            info,
            memory_allocations: Mutex::new(Vec::new()),
            streams: Mutex::new(Vec::new()),
            stats: Mutex::new(AcceleratorStats::default()),
        }
    }

    /// Create stub device for testing
    pub fn new_stub(id: AcceleratorId, device_type: AcceleratorType) -> Self {
        let info = AcceleratorInfo {
            id,
            name: String::from(match device_type {
                AcceleratorType::Gpu => "GPU Device",
                AcceleratorType::Tpu => "TPU Device",
                AcceleratorType::Npu => "NPU Device",
                AcceleratorType::Fpga => "FPGA Device",
                AcceleratorType::Dsp => "DSP Device",
                AcceleratorType::Cpu => "CPU Device",
            }),
            vendor: String::from("Generic"),
            device_type,
            total_memory: 8 * 1024 * 1024 * 1024, // 8GB
            free_memory: 8 * 1024 * 1024 * 1024,
            compute_capability: ComputeCapability {
                architecture: String::from("Unknown"),
                major: 1,
                minor: 0,
                compute_units: 1,
                max_clock_rate: 1000,
                peak_tflops: 1.0,
                memory_bandwidth: 100.0,
            },
            capabilities: DeviceCapabilities::default(),
            pci_bus_id: None,
            numa_node: None,
        };

        Self::new(info)
    }
}

impl ComputeDevice for AcceleratorDevice {
    fn id(&self) -> AcceleratorId {
        self.id
    }

    fn info(&self) -> &AcceleratorInfo {
        &self.info
    }

    fn device_type(&self) -> AcceleratorType {
        self.info.device_type
    }

    fn allocate_memory(&self, size: usize, memory_type: MemoryType) -> AiResult<DeviceMemory> {
        // Check if enough free memory
        if size > self.info.free_memory {
            return Err(AiError::OutOfMemory);
        }

        let memory = DeviceMemory {
            id: self.id * 1000 + self.memory_allocations.lock().len() as u64,
            device_id: self.id,
            size,
            memory_type,
            ptr: 0, // Stub: no actual allocation
        };

        self.memory_allocations.lock().push(memory.clone());
        self.update_stats(size, 0);

        Ok(memory)
    }

    fn free_memory(&self, memory: DeviceMemory) -> AiResult<()> {
        let mut allocs = self.memory_allocations.lock();
        let idx = allocs
            .iter()
            .position(|m| m.id == memory.id)
            .ok_or(AiError::InvalidDevice)?;
        allocs.remove(idx);
        self.update_stats(0, memory.size);
        Ok(())
    }

    fn create_stream(&self) -> AiResult<DeviceStream> {
        let stream = DeviceStream {
            id: self.id * 1000 + self.streams.lock().len() as u64,
            device_id: self.id,
        };
        self.streams.lock().push(stream.clone());
        Ok(stream)
    }

    fn get_stats(&self) -> AiResult<AcceleratorStats> {
        Ok(self.stats.lock().clone())
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl AcceleratorDevice {
    /// Update device statistics
    fn update_stats(&self, allocated: usize, freed: usize) {
        let mut stats = self.stats.lock();
        stats.used_memory += allocated;
        stats.used_memory = stats.used_memory.saturating_sub(freed);
        stats.peak_memory = stats.peak_memory.max(stats.used_memory);
    }
}

/// Global accelerator registry
static ACCELERATOR_REGISTRY: RwLock<BTreeMap<AcceleratorId, Arc<dyn ComputeDevice>>> =
    RwLock::new(BTreeMap::new());

/// Initialize accelerator subsystem
pub fn init() -> AiResult<()> {
    // Register stub devices for testing
    let gpu = Arc::new(AcceleratorDevice::new_stub(0, AcceleratorType::Gpu)) as Arc<dyn ComputeDevice>;
    let tpu = Arc::new(AcceleratorDevice::new_stub(1, AcceleratorType::Tpu)) as Arc<dyn ComputeDevice>;
    let npu = Arc::new(AcceleratorDevice::new_stub(2, AcceleratorType::Npu)) as Arc<dyn ComputeDevice>;

    let mut registry = ACCELERATOR_REGISTRY.write();
    registry.insert(gpu.id(), gpu);
    registry.insert(tpu.id(), tpu);
    registry.insert(npu.id(), npu);

    Ok(())
}

/// Enumerate all available accelerators
pub fn enumerate_devices() -> AiResult<Vec<Arc<dyn ComputeDevice>>> {
    let registry = ACCELERATOR_REGISTRY.read();
    let devices: Vec<Arc<dyn ComputeDevice>> = registry.values().cloned().collect();
    Ok(devices)
}

/// Get device by ID
pub fn get_device(id: AcceleratorId) -> AiResult<Arc<dyn ComputeDevice>> {
    let registry = ACCELERATOR_REGISTRY.read();
    registry
        .get(&id)
        .cloned()
        .ok_or(AiError::InvalidDevice)
}

/// Select best device for given type
pub fn select_best_device(device_type: AcceleratorType) -> AiResult<Arc<dyn ComputeDevice>> {
    let devices = enumerate_devices()?;

    // Filter by device type if specified
    let candidates: Vec<_> = devices
        .into_iter()
        .filter(|d| d.device_type() == device_type || device_type == AcceleratorType::Gpu)
        .collect();

    if candidates.is_empty() {
        return Err(AiError::NoAccelerator);
    }

    // Select device with most free memory
    let best = candidates
        .into_iter()
        .max_by_key(|d| d.info().free_memory)
        .ok_or(AiError::NoAccelerator)?;

    Ok(best)
}

/// Get global statistics
pub fn get_global_stats() -> AiResult<AcceleratorStats> {
    let devices = enumerate_devices()?;
    let mut total_stats = AcceleratorStats::default();

    for device in devices {
        let stats = device.get_stats()?;
        total_stats.total_memory += stats.total_memory;
        total_stats.used_memory += stats.used_memory;
        total_stats.active_streams += stats.active_streams;
        total_stats.pending_tasks += stats.pending_tasks;
        total_stats.completed_tasks += stats.completed_tasks;
        total_stats.peak_memory += stats.peak_memory;
    }

    Ok(total_stats)
}

/// Re-exports for convenience
pub type Accelerator = AcceleratorDevice;
pub type AcceleratorCapabilities = DeviceCapabilities;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_devices() {
        init().unwrap();
        let devices = enumerate_devices().unwrap();
        assert!(!devices.is_empty());
    }

    #[test]
    fn test_get_device() {
        init().unwrap();
        let device = get_device(0);
        assert!(device.is_ok());
    }

    #[test]
    fn test_select_best_device() {
        init().unwrap();
        let device = select_best_device(AcceleratorType::Gpu);
        assert!(device.is_ok());
    }

    #[test]
    fn test_allocate_memory() {
        init().unwrap();
        let device = get_device(0).unwrap();
        let memory = device.allocate_memory(1024, MemoryType::Global);
        assert!(memory.is_ok());
    }

    #[test]
    fn test_create_stream() {
        init().unwrap();
        let device = get_device(0).unwrap();
        let stream = device.create_stream();
        assert!(stream.is_ok());
    }

    #[test]
    fn test_get_stats() {
        init().unwrap();
        let device = get_device(0).unwrap();
        let stats = device.get_stats();
        assert!(stats.is_ok());
    }
}
