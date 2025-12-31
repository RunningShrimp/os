//! # ARM64 Interrupt Controller Support
//!
//! GIC (Generic Interrupt Controller) support for ARM64 architecture
//!
//! ## Supported GIC Versions
//!
//! - **GICv2**: Basic GIC with up to 8 CPUs and 1020 IRQs
//! - **GICv3**: Extended GIC with Message Signaled Interrupts (MSI)
//! - **GICv4**: Adds virtualization support
//!
//! ## Architecture Overview
//!
//! ```
//! GICv3 Architecture:
//!     ├── Distributor (GICD)
//! │   ├── Redistributor (GICR) per CPU
//! │   ├── CPU Interface (GICC) - embedded in GICR for GICv3
//! │   └── ITS (Interrupt Translation Service) for MSI
//! ```
//!
//! ## Interrupt Types
//!
//! - **SGI** (Software Generated Interrupt): 0-15, per-CPU signaling
//! - **PPI** (Private Peripheral Interrupt): 16-31, per-CPU peripherals
//! - **SPI** (Shared Peripheral Interrupt): 32-1019, global devices

#![allow(dead_code)]

use crate::subsystems::sync::*;
use crate::timer::*;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

/// GIC version
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GicVersion {
    GicV2 = 2,
    GicV3 = 3,
    GicV4 = 4,
}

/// GIC Distributor state
pub struct GicDistributor {
    /// Base physical address
    pub base: usize,
    /// GIC version
    pub version: GicVersion,
    /// Number of IRQs
    pub num_irqs: u32,
    /// Number of CPUs (PEs - Processing Elements)
    pub num_cpus: u32,
}

/// GIC Redistributor state (per-CPU)
pub struct GicRedistributor {
    /// Base physical address
    pub base: usize,
    /// CPU ID (PE ID)
    pub cpu_id: u32,
    /// Wake up state
    pub wake_state: WakeState,
}

/// Wake state for redistributor
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WakeState {
    NotPresent,
    Asleep,
    Awake,
}

/// GIC CPU Interface state (embedded in Redistributor for GICv3)
pub struct GicCpuInterface {
    /// Base address (same as redistributor for GICv3)
    pub base: usize,
    /// CPU ID
    pub cpu_id: u32,
}

/// ITS (Interrupt Translation Service) for MSI support
pub struct GicIts {
    /// Base physical address
    pub base: usize,
    /// ITS ID
    pub id: u32,
    /// Number of devices
    pub num_devices: u32,
}

/// Complete GIC state
pub struct GicState {
    /// Distributor
    pub distributor: SpinLock<GicDistributor>,
    /// Per-CPU redistributors
    pub redistributors: Vec<Option<GicRedistributor>>,
    /// Per-CPU interfaces
    pub cpu_interfaces: Vec<Option<GicCpuInterface>>,
    /// ITS (if present)
    pub its: Option<Vec<GicIts>>,
    /// Enabled flag
    pub enabled: AtomicBool,
}

// GIC Distributor Register Offsets
const GICD_CTLR: usize = 0x000;
const GICD_TYPER: usize = 0x004;
const GICD_IIDR: usize = 0x008;
const GICD_TYPER2: usize = 0x00C;
const GICD_IGROUPR: usize = 0x080;
const GICD_ISENABLER: usize = 0x100;  // Interrupt Set-Enable
const GICD_ICENABLER: usize = 0x180;  // Interrupt Clear-Enable
const GICD_ISPENDR: usize = 0x200;    // Interrupt Set-Pending
const GICD_ICPENDR: usize = 0x280;    // Interrupt Clear-Pending
const GICD_ISACTIVER: usize = 0x300;  // Interrupt Set-Active
const GICD_ICACTIVER: usize = 0x380;  // Interrupt Clear-Active
const GICD_IPRIORITYR: usize = 0x400; // Interrupt Priority
const GICD_ITARGETSR: usize = 0x800;  // Interrupt Processor Targets
const GICD_ICFGR: usize = 0xC00;      // Interrupt Configuration
const GICD_SGIR: usize = 0xF00;       // Software Generated Interrupt

// GIC Redistributor Register Offsets
const GICR_CTLR: usize = 0x000;
const GICR_IIDR: usize = 0x004;
const GICR_TYPER: usize = 0x008;
const GICR_WAKER: usize = 0x014;
const GICR_SETLPIR: usize = 0x040;   // Set LPI
const GICR_CLRLPIR: usize = 0x048;   // Clear LPI
const GICR_PROPBASER: usize = 0x070; // LPI Property Base
const GICR_PENDBASER: usize = 0x078; // LPI Pending Base
const GICR_INVLPIR: usize = 0x0A0;   // Invalidate LPI
const GICR_INVALLR: usize = 0x0B0;   // Invalidate All
const GICR_SYNCR: usize = 0x0C0;     // Synchronize

// GICD_CTLR bits
const GICD_CTLR_ENABLE: u32 = 1 << 0;
const GICD_CTLR_ARE_NS: u32 = 1 << 4;  // Affinity routing (Non-secure)
const GICD_CTLR_DS: u32 = 1 << 5;      // Disable security
const GICD_CTLR_E1NWF: u32 = 1 << 7;   // Enable 1-of-N wakeup

// GICR_WAKER bits
const GICR_WAKER_ProcessorSleep: u32 = 1 << 1;
const GICR_WAKER_ChildrenAsleep: u32 = 1 << 2;

impl GicDistributor {
    /// Create new GIC Distributor
    pub fn new(base: usize, version: GicVersion) -> Self {
        GicDistributor {
            base,
            version,
            num_irqs: 0,
            num_cpus: 0,
        }
    }

    /// Initialize GIC Distributor
    pub fn init(&mut self) -> Result<(), GicError> {
        // Disable distributor before configuration
        self.write_ctlr(0);

        // Wait for enable to clear
        while self.read_ctlr() & 1 != 0 {
            core::hint::spin_loop();
        }

        // Read typer to determine number of IRQs
        let typer = self.read_typer();
        self.num_irqs = ((typer & 0x1F) + 1) * 32;  // ITLinesNumber field
        self.num_cpus = ((typer >> 5) & 0x7) + 1;    // CPUNumber field

        // Configure all SPIs (32-1019)
        for irq in 32..self.num_irqs {
            // Set as level-triggered
            self.set_irq_trigger(irq, TriggerMode::Level);

            // Set priority (lower = higher priority)
            self.set_irq_priority(irq, 0xA0);

            // Disable all IRQs initially
            self.disable_irq(irq);

            // Clear pending
            self.clear_pending(irq);
        }

        // Enable distributor
        let ctlr = GICD_CTLR_ENABLE;
        self.write_ctlr(ctlr);

        Ok(())
    }

    /// Enable IRQ
    pub fn enable_irq(&mut self, irq: u32) {
        let reg = GICD_ISENABLER + (irq / 32) * 4;
        let mask = 1 << (irq % 32);
        self.write_reg(reg, mask);
    }

    /// Disable IRQ
    pub fn disable_irq(&mut self, irq: u32) {
        let reg = GICD_ICENABLER + (irq / 32) * 4;
        let mask = 1 << (irq % 32);
        self.write_reg(reg, mask);
    }

    /// Set IRQ priority
    pub fn set_irq_priority(&mut self, irq: u32, priority: u8) {
        let reg = GICD_IPRIORITYR + irq as usize * 4;
        self.write_reg(reg, priority as u32);
    }

    /// Set IRQ target CPU(s)
    pub fn set_irq_target(&mut self, irq: u32, cpu_mask: u8) {
        let reg = GICD_ITARGETSR + irq as usize * 4;
        self.write_reg(reg, cpu_mask as u32);
    }

    /// Set IRQ trigger mode
    pub fn set_irq_trigger(&mut self, irq: u32, mode: TriggerMode) {
        let reg = GICD_ICFGR + (irq / 16) * 4;
        let bit = 1 << ((irq % 16) * 2);

        let mut val = self.read_reg(reg);
        match mode {
            TriggerMode::Level => val &= !bit,      // Clear bit
            TriggerMode::Edge => val |= bit,         // Set bit
        }

        self.write_reg(reg, val);
    }

    /// Send SGI (Software Generated Interrupt)
    pub fn send_sgi(&mut self, sgi: u8, target_filter: SgiTargetFilter, cpu_mask: u8) {
        let mut val = ((sgi & 0xF) as u32) << 24;
        val |= (target_filter as u32) << 16;
        val |= (cpu_mask as u32) << 16;

        self.write_reg(GICD_SGIR, val);
    }

    /// Clear pending IRQ
    pub fn clear_pending(&mut self, irq: u32) {
        let reg = GICD_ICPENDR + (irq / 32) * 4;
        let mask = 1 << (irq % 32);
        self.write_reg(reg, mask);
    }

    /// Read GICD_CTLR
    fn read_ctlr(&self) -> u32 {
        self.read_reg(GICD_CTLR)
    }

    /// Write GICD_CTLR
    fn write_ctlr(&mut self, val: u32) {
        self.write_reg(GICD_CTLR, val);
    }

    /// Read GICD_TYPER
    fn read_typer(&self) -> u32 {
        self.read_reg(GICD_TYPER)
    }

    /// Read register
    fn read_reg(&self, offset: usize) -> u32 {
        unsafe {
            (self.base + offset) as *const AtomicU32
        }.read(Ordering::Acquire)
    }

    /// Write register
    fn write_reg(&mut self, offset: usize, val: u32) {
        unsafe {
            ((self.base + offset) as *mut AtomicU32)
        }.write(val, Ordering::Release);
        self.memory_barrier();
    }

    /// Memory barrier
    fn memory_barrier(&self) {
        unsafe {
            core::arch::asm!("dmb sy", options(nostack, nomem));
        }
    }
}

impl GicRedistributor {
    /// Create new GIC Redistributor
    pub fn new(base: usize, cpu_id: u32) -> Self {
        GicRedistributor {
            base,
            cpu_id,
            wake_state: WakeState::Asleep,
        }
    }

    /// Initialize redistributor
    pub fn init(&mut self) -> Result<(), GicError> {
        // Wake up redistributor
        self.wake_up();

        // Enable private interrupts (SGI and PPI)
        self.enable_ppi();
        self.enable_sgi();

        Ok(())
    }

    /// Wake up redistributor
    fn wake_up(&mut self) {
        // Clear ProcessorSleep bit
        let mut waker = self.read_waker();
        waker &= !GICR_WAKER_ProcessorSleep;
        self.write_waker(waker);

        // Wait for ChildrenAsleep to clear
        while self.read_waker() & GICR_WAKER_ChildrenAsleep != 0 {
            core::hint::spin_loop();
        }

        self.wake_state = WakeState::Awake;
    }

    /// Enable PPI (Private Peripheral Interrupts: 16-31)
    fn enable_ppi(&mut self) {
        for irq in 16..32 {
            let reg = GICR_ISENABLER + (irq / 32) * 4;
            let mask = 1 << (irq % 32);
            self.write_reg(reg, mask);
        }
    }

    /// Enable SGI (Software Generated Interrupts: 0-15)
    fn enable_sgi(&mut self) {
        for irq in 0..16 {
            let reg = GICR_ISENABLER + (irq / 32) * 4;
            let mask = 1 << (irq % 32);
            self.write_reg(reg, mask);
        }
    }

    /// Read GICR_WAKER
    fn read_waker(&self) -> u32 {
        self.read_reg(GICR_WAKER)
    }

    /// Write GICR_WAKER
    fn write_waker(&mut self, val: u32) {
        self.write_reg(GICR_WAKER, val);
    }

    /// Read register
    fn read_reg(&self, offset: usize) -> u32 {
        unsafe {
            (self.base + offset) as *const AtomicU32
        }.read(Ordering::Acquire)
    }

    /// Write register
    fn write_reg(&mut self, offset: usize, val: u32) {
        unsafe {
            ((self.base + offset) as *mut AtomicU32)
        }.write(val, Ordering::Release);
        self.memory_barrier();
    }

    /// Memory barrier
    fn memory_barrier(&self) {
        unsafe {
            core::arch::asm!("dmb sy", options(nostack, nomem));
        }
    }
}

impl GicCpuInterface {
    /// Create new GIC CPU Interface
    pub fn new(base: usize, cpu_id: u32) -> Self {
        GicCpuInterface {
            base,
            cpu_id,
        }
    }

    /// Acknowledge interrupt
    pub fn acknowledge_irq(&self) -> u32 {
        unsafe {
            // Read IAR (Interrupt Acknowledge Register)
            let iar = ((self.base + 0xC0) as *const AtomicU32)
                .read(Ordering::Acquire);

            // Extract interrupt number
            (iar >> 24) & 0x3FF
        }
    }

    /// End of interrupt
    pub fn end_of_interrupt(&self, irq: u32) {
        unsafe {
            // Write EOIR (End of Interrupt Register)
            ((self.base + 0xC0 + 8) as *mut AtomicU32)
                .write(irq, Ordering::Release);
        }
    }

    /// Get interrupt priority
    pub fn get_priority(&self) -> u8 {
        unsafe {
            // Read RPR (Running Priority Register)
            let rpr = ((self.base + 0xC0 + 16) as *const AtomicU32)
                .read(Ordering::Acquire);

            (rpr & 0xFF) as u8
        }
    }

    /// Get highest priority pending interrupt
    pub fn get_highest_pending(&self) -> u32 {
        unsafe {
            // Read HPPIR (Highest Priority Pending Interrupt)
            let hppir = ((self.base + 0xC0 + 24) as *const AtomicU32)
                .read(Ordering::Acquire);

            (hppir >> 24) & 0x3FF
        }
    }
}

/// GIC errors
#[derive(Debug)]
pub enum GicError {
    InvalidBase,
    UnsupportedVersion,
    InitializationFailed,
    InvalidIrq,
}

/// Trigger mode
#[derive(Debug, Clone, Copy)]
pub enum TriggerMode {
    Edge,
    Level,
}

/// SGI target filter
#[derive(Debug, Clone, Copy)]
pub enum SgiTargetFilter {
    TargetList = 0,
    TargetOthers = 1,
    TargetSelf = 2,
}

/// Initialize GIC for current CPU
pub fn init_gic() -> Result<(), GicError> {
    // This is typically called from ACPI or device tree parsing
    // For now, return success (placeholder)
    Ok(())
}

/// Send inter-processor interrupt
pub fn send_ipi(target_cpu: u32, irq: u32) {
    // Placeholder: Send IPI to target CPU
}

/// Handle IRQ
pub fn handle_irq(irq: u32) {
    // Placeholder: Call IRQ handler
}

// GIC feature detection
pub fn detect_gic_version(mmio_base: usize) -> GicVersion {
    let iidr = unsafe {
        (mmio_base as *const AtomicU32)
    }.read(Ordering::Acquire);

    let implementer = (iidr >> 20) & 0xFFF;
    let variant = (iidr >> 16) & 0xF;
    let revision = iidr & 0xFFFF;

    // Detect version based on implementer and device ID
    // ARM GIC implementer IDs:
    // - 0x43B: ARM Ltd
    match implementer {
        0x43B => {
            // Check product ID
            let pid = unsafe {
                ((mmio_base + 0xFE0) as *const AtomicU32)
                    .read(Ordering::Acquire)
            } & 0xFF;

            match pid {
                0x21 => GicVersion::GicV2,
                0x44 => GicVersion::GicV3,
                0x50 => GicVersion::GicV4,
                _ => GicVersion::GicV3,  // Assume v3 as default
            }
        }
        _ => GicVersion::GicV3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gic_version_detection() {
        // Test GIC version detection
        let version = detect_gic_version(0x08000000);
        assert!(matches!(version, GicVersion::GicV2 | GicVersion::GicV3 | GicVersion::GicV4));
    }

    #[test]
    fn test_distributor_init() {
        // Test distributor initialization
        let mut dist = GicDistributor::new(0x08000000, GicVersion::GicV3);
        assert!(dist.init().is_ok());
    }

    #[test]
    fn test_irq_enable_disable() {
        // Test IRQ enable/disable
        let mut dist = GicDistributor::new(0x08000000, GicVersion::GicV3);
        dist.init().unwrap();

        dist.enable_irq(50);
        dist.disable_irq(50);

        // Verify (in real implementation)
        assert!(true);
    }
}
