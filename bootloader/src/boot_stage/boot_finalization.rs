//! Boot Finalization - Final Boot Checks and Kernel Transfer
//!
//! Handles final bootloader operations including:
//! - Pre-kernel-entry system validation
//! - Final memory checks
//! - Interrupt vector setup
//! - Boot handoff to kernel

use core::fmt;
use alloc::string::String;
use alloc::format;

/// Finalization stage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinalizationStage {
    ValidateMemory,           // Memory validation
    CheckInterrupts,          // Interrupt system check
    ValidatePeripherals,      // Peripheral validation
    PrepareGDT,               // GDT preparation
    PreparePaging,            // Paging setup verification
    SetupIDT,                 // IDT preparation
    ValidateBootInfo,         // Boot info validation
    ReadyForTransfer,         // Ready for kernel transfer
}

impl fmt::Display for FinalizationStage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FinalizationStage::ValidateMemory => write!(f, "Validate Memory"),
            FinalizationStage::CheckInterrupts => write!(f, "Check Interrupts"),
            FinalizationStage::ValidatePeripherals => write!(f, "Validate Peripherals"),
            FinalizationStage::PrepareGDT => write!(f, "Prepare GDT"),
            FinalizationStage::PreparePaging => write!(f, "Prepare Paging"),
            FinalizationStage::SetupIDT => write!(f, "Setup IDT"),
            FinalizationStage::ValidateBootInfo => write!(f, "Validate Boot Info"),
            FinalizationStage::ReadyForTransfer => write!(f, "Ready For Transfer"),
        }
    }
}

/// Memory validation result
#[derive(Debug, Clone)]
pub struct MemoryValidation {
    pub total_pages: u32,
    pub available_pages: u32,
    pub reserved_pages: u32,
    pub kernel_pages: u32,
    pub is_valid: bool,
}

impl MemoryValidation {
    /// Create new memory validation
    pub fn new() -> Self {
        MemoryValidation {
            total_pages: 0,
            available_pages: 0,
            reserved_pages: 0,
            kernel_pages: 0,
            is_valid: false,
        }
    }

    /// Calculate memory usage percent
    pub fn usage_percent(&self) -> u32 {
        if self.total_pages == 0 {
            return 0;
        }
        ((self.kernel_pages as u64 * 100) / (self.total_pages as u64)) as u32
    }

    /// Check if memory is sufficient
    pub fn is_sufficient(&self) -> bool {
        self.available_pages > 0
            && self.kernel_pages <= self.total_pages
            && self.reserved_pages <= self.total_pages
    }
}

impl fmt::Display for MemoryValidation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Memory {{ total: {}, avail: {}, kernel: {}, usage: {}% }}",
            self.total_pages,
            self.available_pages,
            self.kernel_pages,
            self.usage_percent()
        )
    }
}

/// Interrupt system status
#[derive(Debug, Clone)]
pub struct InterruptStatus {
    pub pic_configured: bool,
    pub apic_configured: bool,
    pub idt_ready: bool,
    pub exception_handlers: u32,
    pub irq_handlers: u32,
}

impl InterruptStatus {
    /// Create new interrupt status
    pub fn new() -> Self {
        InterruptStatus {
            pic_configured: false,
            apic_configured: false,
            idt_ready: false,
            exception_handlers: 0,
            irq_handlers: 0,
        }
    }

    /// Check if interrupts are ready
    pub fn is_ready(&self) -> bool {
        self.idt_ready && (self.pic_configured || self.apic_configured)
    }

    /// Check if handlers are configured
    pub fn has_handlers(&self) -> bool {
        self.exception_handlers > 0 && self.irq_handlers > 0
    }
}

impl fmt::Display for InterruptStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Interrupts {{ PIC: {}, APIC: {}, IDT: {}, Handlers: {}/{} }}",
            self.pic_configured,
            self.apic_configured,
            self.idt_ready,
            self.exception_handlers,
            self.irq_handlers
        )
    }
}

/// GDT entry
#[derive(Debug, Clone)]
pub struct GdtEntry {
    pub base: u32,
    pub limit: u32,
    pub access: u8,
    pub flags: u8,
}

impl GdtEntry {
    /// Create new GDT entry
    pub fn new() -> Self {
        GdtEntry {
            base: 0,
            limit: 0,
            access: 0,
            flags: 0,
        }
    }

    /// Create kernel code segment
    pub fn kernel_code() -> Self {
        GdtEntry {
            base: 0,
            limit: 0xFFFFFFFF,
            access: 0x9A,  // Code, execute/read
            flags: 0xCF,   // Granularity: 4KB, 64-bit
        }
    }

    /// Create kernel data segment
    pub fn kernel_data() -> Self {
        GdtEntry {
            base: 0,
            limit: 0xFFFFFFFF,
            access: 0x92,  // Data, read/write
            flags: 0xCF,   // Granularity: 4KB, 64-bit
        }
    }
}

impl fmt::Display for GdtEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "GDT {{ base: 0x{:x}, limit: 0x{:x}, access: 0x{:x} }}",
            self.base, self.limit, self.access
        )
    }
}

/// IDT entry
#[derive(Debug, Clone)]
pub struct IdtEntry {
    pub offset_low: u16,
    pub selector: u16,
    pub ist: u8,
    pub type_attr: u8,
    pub offset_high: u32,
}

impl IdtEntry {
    /// Create new IDT entry
    pub fn new() -> Self {
        IdtEntry {
            offset_low: 0,
            selector: 0x08,      // Kernel code selector
            ist: 0,
            type_attr: 0x8E,     // Interrupt gate
            offset_high: 0,
        }
    }

    /// Set handler address
    pub fn set_handler(&mut self, address: u64) {
        self.offset_low = (address & 0xFFFF) as u16;
        self.offset_high = ((address >> 16) & 0xFFFFFFFF) as u32;
    }
}

impl fmt::Display for IdtEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IDT {{ selector: 0x{:x}, type: 0x{:x} }}",
            self.selector, self.type_attr
        )
    }
}

/// Boot finalization status
#[derive(Debug, Clone)]
pub struct FinalizationStatus {
    pub current_stage: FinalizationStage,
    pub stages_completed: u32,
    pub is_ready: bool,
    pub error_message: String,
}

impl FinalizationStatus {
    /// Create new finalization status
    pub fn new() -> Self {
        FinalizationStatus {
            current_stage: FinalizationStage::ValidateMemory,
            stages_completed: 0,
            is_ready: false,
            error_message: String::new(),
        }
    }

    /// Set error
    pub fn set_error(&mut self, msg: &str) {
        self.error_message = String::from(msg);
        self.is_ready = false;
    }
}

impl fmt::Display for FinalizationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Finalization {{ stage: {}, completed: {}, ready: {} }}",
            self.current_stage, self.stages_completed, self.is_ready
        )
    }
}

/// Boot Finalization Manager
pub struct BootFinalization {
    status: FinalizationStatus,
    memory_val: MemoryValidation,
    interrupt_status: InterruptStatus,
    gdt_entries: u32,
    idt_entries: u32,
    is_multiboot_prepared: bool,
}

impl BootFinalization {
    /// Create new boot finalization manager
    pub fn new() -> Self {
        BootFinalization {
            status: FinalizationStatus::new(),
            memory_val: MemoryValidation::new(),
            interrupt_status: InterruptStatus::new(),
            gdt_entries: 0,
            idt_entries: 0,
            is_multiboot_prepared: false,
        }
    }

    /// Validate memory layout
    pub fn validate_memory(&mut self, total: u32, avail: u32, kernel: u32) -> bool {
        self.memory_val.total_pages = total;
        self.memory_val.available_pages = avail;
        self.memory_val.kernel_pages = kernel;
        self.memory_val.is_valid = self.memory_val.is_sufficient();

        if self.memory_val.is_valid {
            self.status.stages_completed += 1;
            true
        } else {
            self.status.set_error("Memory validation failed");
            false
        }
    }

    /// Check interrupt system
    pub fn check_interrupt_system(&mut self, pic: bool, apic: bool, idt: bool) -> bool {
        self.interrupt_status.pic_configured = pic;
        self.interrupt_status.apic_configured = apic;
        self.interrupt_status.idt_ready = idt;

        if self.interrupt_status.is_ready() {
            self.status.stages_completed += 1;
            true
        } else {
            self.status.set_error("Interrupt system not ready");
            false
        }
    }

    /// Set GDT entries
    pub fn set_gdt_entries(&mut self, count: u32) -> bool {
        if count < 3 {
            // At least null, kernel code, kernel data
            self.status.set_error("Insufficient GDT entries");
            return false;
        }

        self.gdt_entries = count;
        self.status.stages_completed += 1;
        true
    }

    /// Set IDT entries
    pub fn set_idt_entries(&mut self, count: u32) -> bool {
        if count < 32 {
            // At least exception handlers
            self.status.set_error("Insufficient IDT entries");
            return false;
        }

        self.idt_entries = count;
        self.status.stages_completed += 1;
        true
    }

    /// Add exception handler
    pub fn add_exception_handler(&mut self) -> bool {
        if self.interrupt_status.exception_handlers < 32 {
            self.interrupt_status.exception_handlers += 1;
            true
        } else {
            false
        }
    }

    /// Add IRQ handler
    pub fn add_irq_handler(&mut self) -> bool {
        if self.interrupt_status.irq_handlers < 16 {
            self.interrupt_status.irq_handlers += 1;
            true
        } else {
            false
        }
    }

    /// Prepare multiboot boot info
    pub fn prepare_multiboot_info(&mut self) -> bool {
        if !self.memory_val.is_valid {
            self.status.set_error("Memory not validated");
            return false;
        }

        self.is_multiboot_prepared = true;
        self.status.stages_completed += 1;
        true
    }

    /// Finalize boot process
    pub fn finalize_boot(&mut self) -> bool {
        // All stages must be completed
        if self.status.stages_completed < 4 {
            self.status.set_error("Boot finalization incomplete");
            return false;
        }

        if !self.interrupt_status.is_ready() {
            self.status.set_error("Interrupt system not ready");
            return false;
        }

        if !self.memory_val.is_sufficient() {
            self.status.set_error("Insufficient memory");
            return false;
        }

        self.status.is_ready = true;
        self.status.current_stage = FinalizationStage::ReadyForTransfer;
        true
    }

    /// Get memory validation
    pub fn get_memory_validation(&self) -> &MemoryValidation {
        &self.memory_val
    }

    /// Get interrupt status
    pub fn get_interrupt_status(&self) -> &InterruptStatus {
        &self.interrupt_status
    }

    /// Check if ready for kernel transfer
    pub fn is_ready_for_kernel_transfer(&self) -> bool {
        self.status.is_ready
    }

    /// Get finalization status
    pub fn get_status(&self) -> &FinalizationStatus {
        &self.status
    }

    /// Get detailed finalization report
    pub fn finalization_report(&self) -> String {
        let mut report = String::from("=== Boot Finalization Report ===\n");

        report.push_str(&format!("Status: {}\n", self.status));
        report.push_str(&format!("Ready for Transfer: {}\n", self.status.is_ready));
        
        report.push_str(&format!("\n{}\n", self.memory_val));
        report.push_str(&format!("{}\n", self.interrupt_status));
        
        report.push_str(&format!("\nGDT Entries: {}\n", self.gdt_entries));
        report.push_str(&format!("IDT Entries: {}\n", self.idt_entries));
        report.push_str(&format!("Multiboot Prepared: {}\n", self.is_multiboot_prepared));

        if !self.status.error_message.is_empty() {
            report.push_str(&format!("\nError: {}\n", self.status.error_message));
        }

        report
    }
}

impl fmt::Display for BootFinalization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "BootFinalization {{ status: {}, ready: {} }}",
            self.status, self.status.is_ready
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_validation_creation() {
        let mem = MemoryValidation::new();
        assert_eq!(mem.total_pages, 0);
        assert!(!mem.is_valid);
    }

    #[test]
    fn test_memory_validation_usage_percent() {
        let mut mem = MemoryValidation::new();
        mem.total_pages = 1000;
        mem.kernel_pages = 250;
        assert_eq!(mem.usage_percent(), 25);
    }

    #[test]
    fn test_memory_validation_sufficient() {
        let mut mem = MemoryValidation::new();
        mem.total_pages = 1000;
        mem.available_pages = 500;
        mem.kernel_pages = 300;
        assert!(mem.is_sufficient());
    }

    #[test]
    fn test_interrupt_status_creation() {
        let int_status = InterruptStatus::new();
        assert!(!int_status.pic_configured);
        assert!(!int_status.idt_ready);
    }

    #[test]
    fn test_interrupt_status_ready() {
        let mut int_status = InterruptStatus::new();
        int_status.pic_configured = true;
        int_status.idt_ready = true;
        assert!(int_status.is_ready());
    }

    #[test]
    fn test_gdt_entry_creation() {
        let entry = GdtEntry::new();
        assert_eq!(entry.base, 0);
    }

    #[test]
    fn test_gdt_entry_kernel_code() {
        let entry = GdtEntry::kernel_code();
        assert_eq!(entry.access, 0x9A);
        assert_eq!(entry.limit, 0xFFFFFFFF);
    }

    #[test]
    fn test_gdt_entry_kernel_data() {
        let entry = GdtEntry::kernel_data();
        assert_eq!(entry.access, 0x92);
    }

    #[test]
    fn test_idt_entry_creation() {
        let entry = IdtEntry::new();
        assert_eq!(entry.selector, 0x08);
        assert_eq!(entry.type_attr, 0x8E);
    }

    #[test]
    fn test_idt_entry_handler() {
        let mut entry = IdtEntry::new();
        entry.set_handler(0x1234567890ABCDEF);
        assert_eq!(entry.offset_low, 0xCDEF);
    }

    #[test]
    fn test_finalization_status_creation() {
        let status = FinalizationStatus::new();
        assert!(!status.is_ready);
    }

    #[test]
    fn test_finalization_status_error() {
        let mut status = FinalizationStatus::new();
        status.set_error("Test error");
        assert!(!status.is_ready);
    }

    #[test]
    fn test_boot_finalization_creation() {
        let finalization = BootFinalization::new();
        assert!(!finalization.is_ready_for_kernel_transfer());
    }

    #[test]
    fn test_boot_finalization_validate_memory() {
        let mut finalization = BootFinalization::new();
        assert!(finalization.validate_memory(1000, 500, 300));
        assert!(finalization.get_memory_validation().is_valid);
    }

    #[test]
    fn test_boot_finalization_check_interrupts() {
        let mut finalization = BootFinalization::new();
        assert!(finalization.check_interrupt_system(true, false, true));
        assert!(finalization.get_interrupt_status().is_ready());
    }

    #[test]
    fn test_boot_finalization_gdt_entries() {
        let mut finalization = BootFinalization::new();
        assert!(finalization.set_gdt_entries(5));
        assert!(!finalization.set_gdt_entries(2));
    }

    #[test]
    fn test_boot_finalization_idt_entries() {
        let mut finalization = BootFinalization::new();
        assert!(finalization.set_idt_entries(256));
        assert!(!finalization.set_idt_entries(16));
    }

    #[test]
    fn test_boot_finalization_add_handlers() {
        let mut finalization = BootFinalization::new();
        for _ in 0..32 {
            assert!(finalization.add_exception_handler());
        }
        for _ in 0..16 {
            assert!(finalization.add_irq_handler());
        }
    }

    #[test]
    fn test_boot_finalization_finalize() {
        let mut finalization = BootFinalization::new();
        finalization.validate_memory(1000, 500, 300);
        finalization.check_interrupt_system(true, false, true);
        finalization.set_gdt_entries(5);
        finalization.set_idt_entries(256);
        
        assert!(finalization.finalize_boot());
        assert!(finalization.is_ready_for_kernel_transfer());
    }

    #[test]
    fn test_boot_finalization_report() {
        let finalization = BootFinalization::new();
        let report = finalization.finalization_report();
        assert!(report.contains("Boot Finalization Report"));
    }
}
