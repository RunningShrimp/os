//! # AI Accelerator Support
//!
//! 本模块为 NOS 内核提供 AI 加速器支持框架，实现：
//!
//! ## 核心功能
//!
//! - **多厂商支持**: CUDA、OpenCL、ROCm、OneAPI 等多种 AI 计算平台
//! - **统一抽象**: 为不同类型的 AI 加速器（GPU、TPU、NPU）提供统一接口
//! - **高性能计算**: 零拷贝数据传输、DMA 优化、异步执行
//! - **低延迟调度**: 智能任务调度、负载均衡、资源隔离
//! - **可扩展架构**: 插件式驱动框架、动态设备发现、热插拔支持
//!
//! ## 架构组件
//!
//! ### 计算平台支持
//!
//! - [`cuda`]: NVIDIA CUDA 驱动接口
//! - [`opencl`]: OpenCL 运行时支持
//! - [`accelerator`]: 通用 AI 加速器驱动框架
//!
//! ### 计算引擎
//!
//! - [`tensor`]: 张量计算引擎
//! - [`neural`]: 神经网络加速器
//! - [`scheduler`]: 异构计算调度器
//!
//! ## 设备类型
//!
//! - **GPU**: 通用图形处理单元（NVIDIA、AMD、Intel）
//! - **TPU**: 张量处理单元（Google、专用 ASIC）
//! - **NPU**: 神经网络处理单元（边缘 AI 芯片）
//! - **FPGA**: 可重构逻辑加速器
//!
//! ## 性能优化
//!
//! - 零拷贝数据传输
//! - 异步 kernel 执行
//! - 流水线并行
//! - 内存池管理
//! - 计算图优化
//!
//! ## 使用示例
//!
//! ```no_run
//! use kernel::ai::{Accelerator, AcceleratorType, Tensor, ComputeDevice};
//!
//! // 获取可用的加速器设备
//! let devices = Accelerator::enumerate_devices()?;
//! let gpu = &devices[0];
//!
//! // 创建张量并在设备上分配内存
//! let tensor = Tensor::zeros([1024, 1024], gpu)?;
//!
//! // 执行计算
//! let result = tensor.matmul(&tensor)?;
//!
//! // 异步执行
//! gpu.submit_task(async {
//!     let c = a.matmul(&b)?;
//!     Ok(c)
//! }).await?;
//! # Ok::<(), kernel::ai::AiError>(())
//! ```
//!
//! ## 设计原则
//!
//! 1. **性能优先**: 最小化数据移动，最大化计算密度
//! 2. **统一接口**: 屏蔽底层硬件差异，提供一致的 API
//! 3. **可扩展性**: 支持新硬件和新算法的无缝集成
//! 4. **安全性**: 内存隔离、权限控制、资源限制
//! 5. **可观测性**: 性能监控、调试支持、资源统计

pub mod cuda;
pub mod opencl;
pub mod accelerator;
pub mod tensor;
pub mod neural;
pub mod scheduler;

// Re-export main types
pub use accelerator::{
    Accelerator, AcceleratorType, AcceleratorDevice,
    AcceleratorCapabilities, AcceleratorInfo, MemoryType,
    ComputeDevice, DeviceMemory, DeviceStream,
};
pub use tensor::{
    Tensor, TensorShape, TensorDataType, TensorOps,
    MemoryLayout, ComputeGraph,
};
pub use neural::{
    NeuralEngine, LayerType, LayerConfig, InferenceSession,
    ModelRuntime, OptimizerType,
};
pub use scheduler::{
    ComputeScheduler, SchedulerPolicy, Task, TaskPriority,
    TaskStatus, ComputeResource, ResourceAllocation,
    HeterogeneousScheduler,
};
pub use cuda::{
    CudaDevice, CudaStream, CudaEvent, CudaModule,
    CudaKernel, CudaMemory, CudaError,
};
pub use opencl::{
    OpenClPlatform, OpenClDevice, OpenClContext,
    OpenClCommandQueue, OpenClProgram, OpenClBuffer,
    OpenClError,
};

/// AI accelerator errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiError {
    /// No accelerator available
    NoAccelerator,
    /// Invalid device
    InvalidDevice,
    /// Out of memory
    OutOfMemory,
    /// Invalid argument
    InvalidArgument,
    /// Operation not supported
    NotSupported,
    /// Compilation error
    CompilationError,
    /// Execution error
    ExecutionError,
    /// Timeout
    Timeout,
    /// Device lost
    DeviceLost,
    /// Internal error
    InternalError(&'static str),
}

impl core::fmt::Display for AiError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            AiError::NoAccelerator => write!(f, "No accelerator available"),
            AiError::InvalidDevice => write!(f, "Invalid device"),
            AiError::OutOfMemory => write!(f, "Out of memory"),
            AiError::InvalidArgument => write!(f, "Invalid argument"),
            AiError::NotSupported => write!(f, "Operation not supported"),
            AiError::CompilationError => write!(f, "Compilation error"),
            AiError::ExecutionError => write!(f, "Execution error"),
            AiError::Timeout => write!(f, "Operation timeout"),
            AiError::DeviceLost => write!(f, "Device lost"),
            AiError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for AiError {}

/// Result type for AI operations
pub type AiResult<T> = core::result::Result<T, AiError>;

/// AI accelerator statistics
#[derive(Debug, Clone)]
pub struct AcceleratorStats {
    /// Total memory allocated
    pub total_memory: usize,
    /// Used memory
    pub used_memory: usize,
    /// Number of active streams
    pub active_streams: usize,
    /// Number of pending tasks
    pub pending_tasks: usize,
    /// Number of completed tasks
    pub completed_tasks: usize,
    /// Average execution time (nanoseconds)
    pub avg_execution_time: u64,
    /// Peak memory usage
    pub peak_memory: usize,
}

impl Default for AcceleratorStats {
    fn default() -> Self {
        Self {
            total_memory: 0,
            used_memory: 0,
            active_streams: 0,
            pending_tasks: 0,
            completed_tasks: 0,
            avg_execution_time: 0,
            peak_memory: 0,
        }
    }
}

/// Initialize AI accelerator support
pub fn init() -> AiResult<()> {
    // Initialize CUDA
    #[cfg(feature = "cuda")]
    cuda::init()?;

    // Initialize OpenCL
    #[cfg(feature = "opencl")]
    opencl::init()?;

    // Initialize generic accelerator framework
    accelerator::init()?;

    // Initialize tensor engine
    tensor::init()?;

    // Initialize neural engine
    neural::init()?;

    // Initialize scheduler
    scheduler::init()?;

    Ok(())
}

/// Get global accelerator statistics
pub fn get_stats() -> AiResult<AcceleratorStats> {
    accelerator::get_global_stats()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AiError::NoAccelerator;
        assert_eq!(format!("{}", err), "No accelerator available");
    }

    #[test]
    fn test_stats_default() {
        let stats = AcceleratorStats::default();
        assert_eq!(stats.total_memory, 0);
        assert_eq!(stats.used_memory, 0);
    }
}
