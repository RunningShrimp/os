//! Interrupt Handling for VFIO
//!
//! Provides interrupt abstraction for VFIO devices with support for:
//!
//! - **Legacy INTx**: Traditional pin-based interrupts
//! - **MSI**: Message Signaled Interrupts
//! - **MSI-X**: Extended MSI with up to 2048 vectors
//! - **eventfd**: Signal delivery to userspace via eventfd
//!
//! # Interrupt Flow
//!
//! ```text
//! Device                           Kernel                    Userspace
//! ------                           ------                    ---------
//! Interrupt occurs
//!     |
//!     v
//! IOMMU/Interrupt controller
//!     |
//!     v
//! VFIO interrupt handler
//!     |
//!     | Find eventfd for this IRQ
//!     v
//! eventfd_signal(efd)
//!     |
//!     v
//! Wake up userspace polling thread
//!     |
//!     v
//! Userspace handles interrupt
//! ```

use crate::drivers::vfio::{VfioError, VfioResult, MAX_MSIX_VECTORS};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU8, Ordering};
use spin::{Mutex, RwLock};

/// IRQ type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrqType {
    /// Legacy INTx (pin-based)
    Intx,

    /// Message Signaled Interrupts
    Msi,

    /// Extended MSI
    Msix,

    /// Generic eventfd
    Eventfd,
}

impl IrqType {
    /// Convert to VFIO index
    pub fn to_index(self) -> u32 {
        match self {
            Self::Intx => 0,
            Self::Msi => 1,
            Self::Msix => 2,
            Self::Eventfd => 3,
        }
    }

    /// Convert from VFIO index
    pub fn from_index(index: u32) -> Option<Self> {
        match index {
            0 => Some(Self::Intx),
            1 => Some(Self::Msi),
            2 => Some(Self::Msix),
            3 => Some(Self::Eventfd),
            _ => None,
        }
    }
}

/// IRQ information
#[derive(Debug, Clone)]
pub struct IrqInfo {
    /// IRQ index
    pub index: u32,

    /// IRQ type
    pub irq_type: IrqType,

    /// Number of IRQs of this type
    pub count: u32,

    /// IRQ flags
    pub flags: IrqFlags,
}

/// IRQ flags
#[derive(Debug, Clone, Copy)]
pub struct IrqFlags {
    /// Eventfd supported
    pub eventfd: bool,

    /// Maskable
    pub maskable: bool,

    /// Auto-masked
    pub automasked: bool,

    /// Resizable
    pub resizable: bool,
}

impl IrqFlags {
    /// Convert to flags bitmap
    pub fn as_u32(&self) -> u32 {
        let mut flags = 0u32;

        if self.eventfd {
            flags |= 0x1;
        }
        if self.maskable {
            flags |= 0x2;
        }
        if self.automasked {
            flags |= 0x4;
        }
        if self.resizable {
            flags |= 0x8;
        }

        flags
    }

    /// Convert from flags bitmap
    pub fn from_u32(flags: u32) -> Self {
        Self {
            eventfd: (flags & 0x1) != 0,
            maskable: (flags & 0x2) != 0,
            automasked: (flags & 0x4) != 0,
            resizable: (flags & 0x8) != 0,
        }
    }
}

/// VFIO interrupt manager
pub struct VfioInterrupt {
    device_id: u64,
    irqs: Mutex<Vec<IrqEntry>>,
    next_irq_handle: AtomicU32,
    stats: InterruptStats,
}

/// IRQ entry
struct IrqEntry {
    handle: u32,
    irq_type: IrqType,
    index: u32,
    vector: u32,
    eventfd: Option<i32>,
    enabled: bool,
    masked: bool,
    affinity: Option<CpuAffinity>,
}

impl IrqEntry {
    fn new(handle: u32, irq_type: IrqType, index: u32, vector: u32) -> Self {
        Self {
            handle,
            irq_type,
            index,
            vector,
            eventfd: None,
            enabled: false,
            masked: false,
            affinity: None,
        }
    }
}

/// CPU affinity for IRQ
#[derive(Debug, Clone, Copy)]
pub struct CpuAffinity {
    pub mask: u64,
}

impl CpuAffinity {
    pub fn new(mask: u64) -> Self {
        Self { mask }
    }

    pub fn single_cpu(cpu: u8) -> Self {
        Self {
            mask: 1u64 << cpu,
        }
    }

    pub fn all_cpus() -> Self {
        Self {
            mask: !0u64,
        }
    }
}

impl VfioInterrupt {
    /// Create new interrupt manager
    pub fn new(device_id: u64) -> Self {
        Self {
            device_id,
            irqs: Mutex::new(Vec::new()),
            next_irq_handle: AtomicU32::new(1),
            stats: InterruptStats::new(),
        }
    }

    /// Register IRQ
    ///
    /// # Arguments
    ///
    /// - `irq_type`: Type of interrupt
    /// - `index`: IRQ index
    /// - `vector`: Vector number (for MSI/MSI-X)
    /// - `eventfd`: Event file descriptor for signal delivery
    pub fn register_irq(
        &self,
        irq_type: IrqType,
        index: u32,
        vector: u32,
        eventfd: Option<i32>,
    ) -> VfioResult<u32> {
        // Validate vector number
        if irq_type == IrqType::Msix && vector >= MAX_MSIX_VECTORS {
            return Err(VfioError::InvalidArgument);
        }

        // Allocate handle
        let handle = self.next_irq_handle.fetch_add(1, Ordering::Relaxed);

        // Create entry
        let mut entry = IrqEntry::new(handle, irq_type, index, vector);
        entry.eventfd = eventfd;

        // Enable if eventfd provided
        if eventfd.is_some() {
            entry.enabled = true;
        }

        // Add to list
        let mut irqs = self.irqs.lock();
        irqs.push(entry);

        self.stats.irqs_registered.fetch_add(1, Ordering::Relaxed);

        Ok(handle)
    }

    /// Unregister IRQ
    pub fn unregister_irq(&self, handle: u32) -> VfioResult<()> {
        let mut irqs = self.irqs.lock();

        let pos = irqs.iter().position(|irq| irq.handle == handle);

        if pos.is_none() {
            return Err(VfioError::InvalidArgument);
        }

        let irq = irqs.remove(pos.unwrap());

        // Clean up eventfd
        if let Some(efd) = irq.eventfd {
            // GH-#1362: Close eventfd
            // See: https://github.com/npos/kernel/issues/1362
            let _ = efd;
        }

        self.stats.irqs_unregistered.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Enable IRQ
    pub fn enable_irq(&self, handle: u32) -> VfioResult<()> {
        let mut irqs = self.irqs.lock();

        let irq = irqs
            .iter_mut()
            .find(|irq| irq.handle == handle)
            .ok_or(VfioError::InvalidArgument)?;

        if irq.eventfd.is_none() {
            return Err(VfioError::InvalidArgument);
        }

        irq.enabled = true;
        irq.masked = false;

        self.stats.irqs_enabled.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Disable IRQ
    pub fn disable_irq(&self, handle: u32) -> VfioResult<()> {
        let mut irqs = self.irqs.lock();

        let irq = irqs
            .iter_mut()
            .find(|irq| irq.handle == handle)
            .ok_or(VfioError::InvalidArgument)?;

        irq.enabled = false;

        self.stats.irqs_disabled.fetch_add(1, Ordering::Relaxed);

        Ok(())
    }

    /// Mask IRQ
    pub fn mask_irq(&self, handle: u32) -> VfioResult<()> {
        let mut irqs = self.irqs.lock();

        let irq = irqs
            .iter_mut()
            .find(|irq| irq.handle == handle)
            .ok_or(VfioError::InvalidArgument)?;

        if !irq.irq_type.maskable() {
            return Err(VfioError::NotSupported);
        }

        irq.masked = true;

        Ok(())
    }

    /// Unmask IRQ
    pub fn unmask_irq(&self, handle: u32) -> VfioResult<()> {
        let mut irqs = self.irqs.lock();

        let irq = irqs
            .iter_mut()
            .find(|irq| irq.handle == handle)
            .ok_or(VfioError::InvalidArgument)?;

        if !irq.irq_type.maskable() {
            return Err(VfioError::NotSupported);
        }

        irq.masked = false;

        Ok(())
    }

    /// Set IRQ affinity
    pub fn set_affinity(&self, handle: u32, affinity: CpuAffinity) -> VfioResult<()> {
        let mut irqs = self.irqs.lock();

        let irq = irqs
            .iter_mut()
            .find(|irq| irq.handle == handle)
            .ok_or(VfioError::InvalidArgument)?;

        irq.affinity = Some(affinity);

        Ok(())
    }

    /// Get IRQ affinity
    pub fn get_affinity(&self, handle: u32) -> Option<CpuAffinity> {
        let irqs = self.irqs.lock();
        irqs.iter()
            .find(|irq| irq.handle == handle)
            .and_then(|irq| irq.affinity)
    }

    /// Trigger IRQ (for testing)
    pub fn trigger_irq(&self, handle: u32) -> VfioResult<()> {
        let irqs = self.irqs.lock();

        let irq = irqs
            .iter()
            .find(|irq| irq.handle == handle)
            .ok_or(VfioError::InvalidArgument)?;

        if !irq.enabled || irq.masked {
            return Err(VfioError::InvalidArgument);
        }

        // Signal eventfd
        if let Some(efd) = irq.eventfd {
            self.signal_eventfd(efd)?;
            self.stats.irqs_triggered.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Handle interrupt (called from kernel interrupt handler)
    pub fn handle_interrupt(&self, irq_type: IrqType, vector: u32) -> VfioResult<()> {
        let irqs = self.irqs.lock();

        // Find matching IRQ
        let irq = irqs
            .iter()
            .find(|irq| irq.irq_type == irq_type && irq.vector == vector)
            .ok_or(VfioError::InvalidArgument)?;

        if !irq.enabled || irq.masked {
            return Ok(());
        }

        // Signal eventfd
        if let Some(efd) = irq.eventfd {
            drop(irqs); // Release lock before signaling
            self.signal_eventfd(efd)?;
            self.stats.interrupts_handled.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Signal eventfd
    fn signal_eventfd(&self, fd: i32) -> VfioResult<()> {
        // GH-#1363: Implement eventfd signaling
        // See: https://github.com/npos/kernel/issues/1363
        // eventfd_signal(fd, 1);
        Ok(())
    }

    /// Get IRQ info
    pub fn get_irq_info(&self, index: u32) -> Option<IrqInfo> {
        let irqs = self.irqs.lock();

        let irq_type = IrqType::from_index(index)?;

        let count = irqs
            .iter()
            .filter(|irq| irq.irq_type == irq_type)
            .count() as u32;

        let flags = IrqFlags {
            eventfd: true,
            maskable: irq_type.maskable(),
            automasked: false,
            resizable: irq_type == IrqType::Msix,
        };

        Some(IrqInfo {
            index,
            irq_type,
            count,
            flags,
        })
    }

    /// Get all IRQs
    pub fn get_all_irqs(&self) -> Vec<IrqInfo> {
        let mut infos = Vec::new();

        for i in 0..4 {
            if let Some(info) = self.get_irq_info(i) {
                infos.push(info);
            }
        }

        infos
    }

    /// Get statistics
    pub fn get_stats(&self) -> InterruptStatsSnapshot {
        let irqs = self.irqs.lock();

        let enabled_count = irqs.iter().filter(|irq| irq.enabled).count();

        InterruptStatsSnapshot {
            total_irqs: irqs.len(),
            enabled_irqs: enabled_count,
            irqs_registered: self.stats.irqs_registered.load(Ordering::Relaxed),
            irqs_unregistered: self.stats.irqs_unregistered.load(Ordering::Relaxed),
            irqs_enabled: self.stats.irqs_enabled.load(Ordering::Relaxed),
            irqs_disabled: self.stats.irqs_disabled.load(Ordering::Relaxed),
            irqs_triggered: self.stats.irqs_triggered.load(Ordering::Relaxed),
            interrupts_handled: self.stats.interrupts_handled.load(Ordering::Relaxed),
        }
    }
}

impl IrqType {
    /// Check if maskable
    pub fn maskable(self) -> bool {
        matches!(self, Self::Intx | Self::Msi)
    }
}

/// Interrupt statistics
struct InterruptStats {
    irqs_registered: AtomicU32,
    irqs_unregistered: AtomicU32,
    irqs_enabled: AtomicU32,
    irqs_disabled: AtomicU32,
    irqs_triggered: AtomicU64,
    interrupts_handled: AtomicU64,
}

impl InterruptStats {
    fn new() -> Self {
        Self {
            irqs_registered: AtomicU32::new(0),
            irqs_unregistered: AtomicU32::new(0),
            irqs_enabled: AtomicU32::new(0),
            irqs_disabled: AtomicU32::new(0),
            irqs_triggered: AtomicU64::new(0),
            interrupts_handled: AtomicU64::new(0),
        }
    }
}

/// Interrupt statistics snapshot
#[derive(Debug, Clone, Copy)]
pub struct InterruptStatsSnapshot {
    pub total_irqs: usize,
    pub enabled_irqs: usize,
    pub irqs_registered: u32,
    pub irqs_unregistered: u32,
    pub irqs_enabled: u32,
    pub irqs_disabled: u32,
    pub irqs_triggered: u64,
    pub interrupts_handled: u64,
}

/// Interrupt error type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterruptError {
    NotFound,
    AlreadyExists,
    InvalidType,
    InvalidVector,
    NotMaskable,
    NotSupported,
    EventfdError,
}

impl core::fmt::Display for InterruptError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotFound => write!(f, "IRQ not found"),
            Self::AlreadyExists => write!(f, "IRQ already exists"),
            Self::InvalidType => write!(f, "Invalid IRQ type"),
            Self::InvalidVector => write!(f, "Invalid IRQ vector"),
            Self::NotMaskable => write!(f, "IRQ is not maskable"),
            Self::NotSupported => write!(f, "Operation not supported"),
            Self::EventfdError => write!(f, "Eventfd error"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_irq_type_conversion() {
        assert_eq!(IrqType::Intx.to_index(), 0);
        assert_eq!(IrqType::Msi.to_index(), 1);
        assert_eq!(IrqType::Msix.to_index(), 2);

        assert_eq!(IrqType::from_index(0), Some(IrqType::Intx));
        assert_eq!(IrqType::from_index(1), Some(IrqType::Msi));
        assert_eq!(IrqType::from_index(99), None);
    }

    #[test]
    fn test_irq_flags() {
        let flags = IrqFlags {
            eventfd: true,
            maskable: true,
            automasked: false,
            resizable: false,
        };

        let raw = flags.as_u32();
        assert!(raw & 0x1 != 0);
        assert!(raw & 0x2 != 0);

        let decoded = IrqFlags::from_u32(raw);
        assert_eq!(decoded.eventfd, flags.eventfd);
        assert_eq!(decoded.maskable, flags.maskable);
    }

    #[test]
    fn test_interrupt_register() {
        let intr = VfioInterrupt::new(0);

        let handle = intr
            .register_irq(IrqType::Msix, 2, 0, Some(42))
            .unwrap();

        assert!(handle > 0);

        intr.unregister_irq(handle).unwrap();
    }

    #[test]
    fn test_interrupt_enable_disable() {
        let intr = VfioInterrupt::new(0);

        let handle = intr
            .register_irq(IrqType::Msix, 2, 0, Some(42))
            .unwrap();

        intr.disable_irq(handle).unwrap();
        intr.enable_irq(handle).unwrap();

        intr.unregister_irq(handle).unwrap();
    }

    #[test]
    fn test_interrupt_mask_unmask() {
        let intr = VfioInterrupt::new(0);

        // INTx is maskable
        let handle = intr
            .register_irq(IrqType::Intx, 0, 0, Some(42))
            .unwrap();

        intr.mask_irq(handle).unwrap();
        intr.unmask_irq(handle).unwrap();

        intr.unregister_irq(handle).unwrap();
    }

    #[test]
    fn test_interrupt_affinity() {
        let intr = VfioInterrupt::new(0);

        let handle = intr
            .register_irq(IrqType::Msix, 2, 0, Some(42))
            .unwrap();

        let affinity = CpuAffinity::single_cpu(0);
        intr.set_affinity(handle, affinity).unwrap();

        let retrieved = intr.get_affinity(handle).unwrap();
        assert_eq!(retrieved.mask, affinity.mask);

        intr.unregister_irq(handle).unwrap();
    }

    #[test]
    fn test_cpu_affinity() {
        let single = CpuAffinity::single_cpu(3);
        assert_eq!(single.mask, 1 << 3);

        let all = CpuAffinity::all_cpus();
        assert_eq!(all.mask, !0u64);
    }

    #[test]
    fn test_interrupt_stats() {
        let intr = VfioInterrupt::new(0);

        let handle = intr
            .register_irq(IrqType::Msix, 2, 0, Some(42))
            .unwrap();

        let stats = intr.get_stats();
        assert_eq!(stats.total_irqs, 1);

        intr.unregister_irq(handle).unwrap();
    }
}
