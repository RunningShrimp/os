//! Hardware Acceleration Module
//! 
//! This module provides comprehensive hardware acceleration support for the NOS kernel,
//! including CPU feature detection, GPU acceleration, cryptographic operations,
//! and vector processing optimizations.

use crate::error::unified::UnifiedError;
use crate::hw_accel::cpu::CPUAccelerator;
use crate::hw_accel::gpu::GPUAccelerator;
use crate::hw_accel::crypto::CryptoAccelerator;
use crate::hw_accel::vector::VectorAccelerator;
use crate::hw_accel::memory::MemoryAccelerator;

pub mod cpu;
pub mod gpu;
pub mod crypto;
pub mod vector;
pub mod memory;

/// Hardware acceleration statistics
#[derive(Debug, Clone)]
pub struct HwAccelStats {
    /// Total acceleration operations
    pub total_operations: u64,
    /// CPU acceleration operations
    pub cpu_operations: u64,
    /// GPU acceleration operations
    pub gpu_operations: u64,
    /// Crypto acceleration operations
    pub crypto_operations: u64,
    /// Vector acceleration operations
    pub vector_operations: u64,
    /// Memory acceleration operations
    pub memory_operations: u64,
    /// Total time saved by acceleration (in microseconds)
    pub total_time_saved_us: u64,
    /// Average acceleration ratio
    pub avg_acceleration_ratio: f64,
}

impl Default for HwAccelStats {
    fn default() -> Self {
        Self {
            total_operations: 0,
            cpu_operations: 0,
            gpu_operations: 0,
            crypto_operations: 0,
            vector_operations: 0,
            memory_operations: 0,
            total_time_saved_us: 0,
            avg_acceleration_ratio: 1.0,
        }
    }
}

/// Hardware acceleration system
pub struct HwAccelSystem {
    /// CPU accelerator
    cpu_accelerator: CPUAccelerator,
    /// GPU accelerator
    gpu_accelerator: GPUAccelerator,
    /// Crypto accelerator
    crypto_accelerator: CryptoAccelerator,
    /// Vector accelerator
    vector_accelerator: VectorAccelerator,
    /// Memory accelerator
    memory_accelerator: MemoryAccelerator,
    /// System statistics
    stats: spin::Mutex<HwAccelStats>,
    /// System active status
    active: spin::Mutex<bool>,
}

impl HwAccelSystem {
    /// Create a new hardware acceleration system
    pub fn new() -> Result<Self, UnifiedError> {
        let cpu_accelerator = CPUAccelerator::new()?;
        let gpu_accelerator = GPUAccelerator::new()?;
        let crypto_accelerator = CryptoAccelerator::new()?;
        let vector_accelerator = VectorAccelerator::new()?;
        let memory_accelerator = MemoryAccelerator::new()?;

        Ok(Self {
            cpu_accelerator,
            gpu_accelerator,
            crypto_accelerator,
            vector_accelerator,
            memory_accelerator,
            stats: spin::Mutex::new(HwAccelStats::default()),
            active: spin::Mutex::new(true),
        })
    }

    /// Initialize the hardware acceleration system
    pub fn initialize(&self) -> Result<(), UnifiedError> {
        // Initialize all accelerators
        self.cpu_accelerator.initialize()?;
        self.gpu_accelerator.initialize()?;
        self.crypto_accelerator.initialize()?;
        self.vector_accelerator.initialize()?;
        self.memory_accelerator.initialize()?;

        log::info!("Hardware acceleration system initialized successfully");
        Ok(())
    }

    /// Get the CPU accelerator
    pub fn cpu(&self) -> &CPUAccelerator {
        &self.cpu_accelerator
    }

    /// Get the GPU accelerator
    pub fn gpu(&self) -> &GPUAccelerator {
        &self.gpu_accelerator
    }

    /// Get the crypto accelerator
    pub fn crypto(&self) -> &CryptoAccelerator {
        &self.crypto_accelerator
    }

    /// Get the vector accelerator
    pub fn vector(&self) -> &VectorAccelerator {
        &self.vector_accelerator
    }

    /// Get the memory accelerator
    pub fn memory(&self) -> &MemoryAccelerator {
        &self.memory_accelerator
    }

    /// Get system statistics
    pub fn get_stats(&self) -> HwAccelStats {
        let mut stats = self.stats.lock();
        
        // Update stats from individual accelerators
        stats.cpu_operations = self.cpu_accelerator.get_operation_count();
        stats.gpu_operations = self.gpu_accelerator.get_operation_count();
        stats.crypto_operations = self.crypto_accelerator.get_operation_count();
        stats.vector_operations = self.vector_accelerator.get_operation_count();
        stats.memory_operations = self.memory_accelerator.get_operation_count();
        
        stats.total_operations = stats.cpu_operations + stats.gpu_operations + 
                                stats.crypto_operations + stats.vector_operations + 
                                stats.memory_operations;
        
        // Calculate average acceleration ratio
        let cpu_ratio = self.cpu_accelerator.get_acceleration_ratio();
        let gpu_ratio = self.gpu_accelerator.get_acceleration_ratio();
        let crypto_ratio = self.crypto_accelerator.get_acceleration_ratio();
        let vector_ratio = self.vector_accelerator.get_acceleration_ratio();
        let memory_ratio = self.memory_accelerator.get_acceleration_ratio();
        
        let total_ratio = cpu_ratio + gpu_ratio + crypto_ratio + vector_ratio + memory_ratio;
        stats.avg_acceleration_ratio = if total_ratio > 0.0 { total_ratio / 5.0 } else { 1.0 };
        
        // Calculate total time saved
        stats.total_time_saved_us = self.cpu_accelerator.get_time_saved_us() +
                                   self.gpu_accelerator.get_time_saved_us() +
                                   self.crypto_accelerator.get_time_saved_us() +
                                   self.vector_accelerator.get_time_saved_us() +
                                   self.memory_accelerator.get_time_saved_us();
        
        stats.clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = HwAccelStats::default();
        
        // Reset individual accelerator stats
        self.cpu_accelerator.reset_stats();
        self.gpu_accelerator.reset_stats();
        self.crypto_accelerator.reset_stats();
        self.vector_accelerator.reset_stats();
        self.memory_accelerator.reset_stats();
    }

    /// Check if the system is active
    pub fn is_active(&self) -> bool {
        *self.active.lock()
    }

    /// Activate or deactivate the system
    pub fn set_active(&self, active: bool) {
        let mut current = self.active.lock();
        *current = active;
        
        if active {
            log::info!("Hardware acceleration system activated");
        } else {
            log::info!("Hardware acceleration system deactivated");
        }
    }

    /// Get hardware acceleration recommendations
    pub fn get_recommendations(&self) -> Vec<String> {
        let mut recommendations = Vec::new();
        
        // CPU recommendations
        if !self.cpu_accelerator.is_optimized() {
            recommendations.push("Enable CPU instruction set extensions (SSE, AVX, NEON)".to_string());
        }
        
        // GPU recommendations
        if !self.gpu_accelerator.is_available() {
            recommendations.push("Consider GPU acceleration for parallel workloads".to_string());
        }
        
        // Crypto recommendations
        if !self.crypto_accelerator.has_hardware_support() {
            recommendations.push("Enable hardware cryptographic acceleration (AES-NI, ARM Crypto)".to_string());
        }
        
        // Vector recommendations
        if !self.vector_accelerator.is_optimized() {
            recommendations.push("Optimize vector operations with SIMD instructions".to_string());
        }
        
        // Memory recommendations
        if !self.memory_accelerator.is_optimized() {
            recommendations.push("Enable memory acceleration features (prefetch, cache optimization)".to_string());
        }
        
        recommendations
    }

    /// Perform system optimization
    pub fn optimize(&self) -> Result<(), UnifiedError> {
        if !self.is_active() {
            return Err(UnifiedError::HwAccel("Hardware acceleration system is not active".to_string()));
        }
        
        // Optimize individual accelerators
        self.cpu_accelerator.optimize()?;
        self.gpu_accelerator.optimize()?;
        self.crypto_accelerator.optimize()?;
        self.vector_accelerator.optimize()?;
        self.memory_accelerator.optimize()?;
        
        log::info!("Hardware acceleration system optimized");
        Ok(())
    }

    /// Get system status
    pub fn get_status(&self) -> HwAccelStatus {
        HwAccelStatus {
            active: self.is_active(),
            cpu_available: self.cpu_accelerator.is_available(),
            cpu_optimized: self.cpu_accelerator.is_optimized(),
            gpu_available: self.gpu_accelerator.is_available(),
            gpu_optimized: self.gpu_accelerator.is_optimized(),
            crypto_available: self.crypto_accelerator.is_available(),
            crypto_optimized: self.crypto_accelerator.is_optimized(),
            vector_available: self.vector_accelerator.is_available(),
            vector_optimized: self.vector_accelerator.is_optimized(),
            memory_available: self.memory_accelerator.is_available(),
            memory_optimized: self.memory_accelerator.is_optimized(),
            stats: self.get_stats(),
        }
    }
}

/// Hardware acceleration system status
#[derive(Debug, Clone)]
pub struct HwAccelStatus {
    /// System active status
    pub active: bool,
    /// CPU accelerator availability
    pub cpu_available: bool,
    /// CPU accelerator optimization status
    pub cpu_optimized: bool,
    /// GPU accelerator availability
    pub gpu_available: bool,
    /// GPU accelerator optimization status
    pub gpu_optimized: bool,
    /// Crypto accelerator availability
    pub crypto_available: bool,
    /// Crypto accelerator optimization status
    pub crypto_optimized: bool,
    /// Vector accelerator availability
    pub vector_available: bool,
    /// Vector accelerator optimization status
    pub vector_optimized: bool,
    /// Memory accelerator availability
    pub memory_available: bool,
    /// Memory accelerator optimization status
    pub memory_optimized: bool,
    /// System statistics
    pub stats: HwAccelStats,
}

/// Global hardware acceleration system instance
static mut HW_ACCEL_SYSTEM: Option<HwAccelSystem> = None;
static HW_ACCEL_INIT: spin::Once = spin::Once::new();

/// Initialize the global hardware acceleration system
pub fn init_hw_accel() -> Result<(), UnifiedError> {
    HW_ACCEL_INIT.call_once(|| {
        match HwAccelSystem::new() {
            Ok(system) => {
                if let Err(e) = system.initialize() {
                    log::error!("Failed to initialize hardware acceleration system: {}", e);
                    return;
                }
                unsafe {
                    HW_ACCEL_SYSTEM = Some(system);
                }
                log::info!("Global hardware acceleration system initialized");
            }
            Err(e) => {
                log::error!("Failed to create hardware acceleration system: {}", e);
            }
        }
    });
    Ok(())
}

/// Get the global hardware acceleration system
pub fn get_hw_accel_system() -> Option<&'static HwAccelSystem> {
    unsafe { HW_ACCEL_SYSTEM.as_ref() }
}

/// Check if hardware acceleration is available
pub fn is_hw_accel_available() -> bool {
    if let Some(system) = get_hw_accel_system() {
        system.is_active()
    } else {
        false
    }
}

/// Initialize hardware acceleration subsystem
pub fn init() -> Result<(), UnifiedError> {
    log::info!("Initializing hardware acceleration subsystem");
    
    // Initialize global hardware acceleration system
    init_hw_accel()?;
    
    log::info!("Hardware acceleration subsystem initialized");
    Ok(())
}

/// Shutdown hardware acceleration subsystem
pub fn shutdown() -> Result<(), UnifiedError> {
    log::info!("Shutting down hardware acceleration subsystem");
    
    // Shutdown global hardware acceleration system
    // In a real implementation, this would clean up resources
    
    log::info!("Hardware acceleration subsystem shutdown complete");
    Ok(())
}