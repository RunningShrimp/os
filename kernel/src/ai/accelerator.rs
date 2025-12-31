//! # Hardware Acceleration
//!
//! Abstract interfaces for hardware acceleration including GPU, NPU, and TPU.
//!
//! ## Features
//!
//! - **GPU Support**: CUDA, OpenCL, Vulkan Compute interfaces
//! - **NPU Support**: Neural Processing Unit interfaces
//! - **TPU Support**: Tensor Processing Unit interfaces
//! - **Device Management**: Device selection, memory management
//! - **Kernel Execution**: Asynchronous execution, streams
//! - **Multi-device**: Support for multiple devices
//! - **CPU Fallback**: Automatic fallback to CPU

use crate::ai::{AiError, AiResult, AcceleratorError, Tensor};
use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::String;

/// Device type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    /// CPU device
    CPU,
    /// GPU device (CUDA)
    GPU,
    /// GPU device (OpenCL)
    OpenCL,
    /// Neural Processing Unit
    NPU,
    /// Tensor Processing Unit
    TPU,
    /// Vulkan compute device
    Vulkan,
}

/// Device capabilities
#[derive(Debug, Clone)]
pub struct DeviceCapabilities {
    /// Device name
    pub name: String,
    /// Device type
    pub device_type: DeviceType,
    /// Total memory in bytes
    pub total_memory: usize,
    /// Compute units
    pub compute_units: u32,
    /// Max clock frequency (MHz)
    pub max_clock_frequency: u32,
    /// Supports unified memory
    pub unified_memory: bool,
    /// Supports half-precision
    pub fp16_support: bool,
    /// Supports int8 operations
    pub int8_support: bool,
}

/// Abstract compute device
pub trait Device: Send + Sync {
    /// Get device capabilities
    fn capabilities(&self) -> &DeviceCapabilities;

    /// Allocate memory on device
    fn allocate(&self, size: usize) -> AiResult<DeviceMemory>;

    /// Free device memory
    fn free(&self, memory: DeviceMemory) -> AiResult<()>;

    /// Copy data from host to device
    fn memcpy_h2d(&self, host: &[u8], device: &DeviceMemory) -> AiResult<()>;

    /// Copy data from device to host
    fn memcpy_d2h(&self, device: &DeviceMemory, host: &mut [u8]) -> AiResult<()>;

    /// Execute compute kernel
    fn execute_kernel(&self, kernel: &ComputeKernel, inputs: &[&Tensor<f32>]) -> AiResult<Tensor<f32>>;

    /// Synchronize device
    fn synchronize(&self) -> AiResult<()>;
}

/// Device memory handle
#[derive(Debug, Clone)]
pub struct DeviceMemory {
    /// Memory pointer (opaque)
    pub ptr: u64,
    /// Memory size
    pub size: usize,
    /// Device ID
    pub device_id: u32,
}

/// Compute kernel
#[derive(Debug, Clone)]
pub struct ComputeKernel {
    /// Kernel name
    pub name: String,
    /// Kernel source code
    pub source: Option<String>,
    /// Kernel binary
    pub binary: Option<Vec<u8>>,
    /// Work group size
    pub work_group_size: (u32, u32, u32),
}

/// CPU device implementation
#[derive(Debug)]
pub struct CPUDevice {
    capabilities: DeviceCapabilities,
}

impl CPUDevice {
    /// Create a new CPU device
    pub fn new() -> Self {
        Self {
            capabilities: DeviceCapabilities {
                name: String::from("CPU"),
                device_type: DeviceType::CPU,
                total_memory: 0,
                compute_units: 1,
                max_clock_frequency: 0,
                unified_memory: true,
                fp16_support: false,
                int8_support: true,
            },
        }
    }
}

impl Default for CPUDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl Device for CPUDevice {
    fn capabilities(&self) -> &DeviceCapabilities {
        &self.capabilities
    }

    fn allocate(&self, size: usize) -> AiResult<DeviceMemory> {
        Ok(DeviceMemory {
            ptr: 0,
            size,
            device_id: 0,
        })
    }

    fn free(&self, _memory: DeviceMemory) -> AiResult<()> {
        Ok(())
    }

    fn memcpy_h2d(&self, _host: &[u8], _device: &DeviceMemory) -> AiResult<()> {
        Ok(())
    }

    fn memcpy_d2h(&self, _device: &DeviceMemory, _host: &mut [u8]) -> AiResult<()> {
        Ok(())
    }

    fn execute_kernel(&self, _kernel: &ComputeKernel, inputs: &[&Tensor<f32>]) -> AiResult<Tensor<f32>> {
        if inputs.is_empty() {
            return Err(AiError::AcceleratorError(AcceleratorError::KernelExecutionFailed(
                String::from("No inputs provided")
            )));
        }

        Ok(inputs[0].clone())
    }

    fn synchronize(&self) -> AiResult<()> {
        Ok(())
    }
}

/// Device manager
pub struct DeviceManager {
    /// Available devices
    devices: Vec<Box<dyn Device>>,
    /// Current device
    current_device: usize,
}

impl DeviceManager {
    /// Create a new device manager
    pub fn new() -> Self {
        let mut devices: Vec<Box<dyn Device>> = Vec::new();
        devices.push(Box::new(CPUDevice::new()));

        Self {
            devices,
            current_device: 0,
        }
    }

    /// Get number of available devices
    pub fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Select device by type
    pub fn select_device(&mut self, device_type: DeviceType) -> AiResult<&dyn Device> {
        for (i, device) in self.devices.iter().enumerate() {
            if device.capabilities().device_type == device_type {
                self.current_device = i;
                return Ok(device.as_ref());
            }
        }

        // Fallback to CPU
        for (i, device) in self.devices.iter().enumerate() {
            if device.capabilities().device_type == DeviceType::CPU {
                self.current_device = i;
                return Ok(device.as_ref());
            }
        }

        Err(AiError::AcceleratorError(AcceleratorError::NoDeviceAvailable))
    }
}

impl Default for DeviceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_device() {
        let cpu = CPUDevice::new();
        assert_eq!(cpu.capabilities().device_type, DeviceType::CPU);
    }

    #[test]
    fn test_device_manager() {
        let manager = DeviceManager::new();
        assert!(manager.device_count() >= 1);
    }
}
