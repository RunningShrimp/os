//! Unified Device Management Types
//!
//! This module provides canonical type definitions for device management.
//! All device-related types should be defined here and re-exported from
//! the platform module.

/// Device type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    /// Block device (disk, SSD, etc.)
    Block,
    /// Character device (tty, serial, etc.)
    Char,
    /// Network device (ethernet, wireless, etc.)
    Network,
    /// Graphics device (GPU, display, etc.)
    Graphics,
    /// Input device (keyboard, mouse, etc.)
    Input,
    /// Audio device (sound card, etc.)
    Audio,
    /// Other or unknown device type
    Other,
}

/// Device resource descriptor
#[derive(Debug, Clone)]
pub struct DeviceResources {
    /// Memory regions
    pub memory_regions: Vec<MemoryRegion>,
    /// Interrupt request lines
    pub irq_lines: Vec<u32>,
    /// I/O port ranges
    pub io_ports: Vec<IoPortRange>,
    /// DMA channels
    pub dma_channels: Vec<u8>,
    /// Interrupts (alias for irq_lines for compatibility)
    pub irqs: Vec<u32>,
}

/// Memory region descriptor
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    /// Start address
    pub start: usize,
    /// Size in bytes
    pub size: usize,
    /// End address (optional, can be derived from start + size)
    pub end: usize,
}

/// I/O port range
#[derive(Debug, Clone)]
pub struct IoPortRange {
    /// Start port
    pub start: u16,
    /// End port
    pub end: u16,
    /// Count of ports (optional, can be derived from end - start)
    pub count: u16,
}

/// Device status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    /// Device not initialized
    NotInitialized,
    /// Device is initializing
    Initializing,
    /// Device is running
    Running,
    /// Device is suspended
    Suspended,
    /// Device is stopped
    Stopped,
    /// Device is in error state
    Error,
    /// Unknown status (for compatibility)
    Unknown,
}

impl DeviceResources {
    /// Create a new empty device resources structure
    pub fn new() -> Self {
        Self {
            memory_regions: Vec::new(),
            irq_lines: Vec::new(),
            io_ports: Vec::new(),
            dma_channels: Vec::new(),
            irqs: Vec::new(),
        }
    }
}

impl Default for DeviceResources {
    fn default() -> Self {
        Self::new()
    }
}
