//! Microkernel interrupt handling
//!
//! Provides basic interrupt management for the microkernel layer.
//! This includes interrupt vector management, handler registration,
//! and interrupt statistics.

extern crate alloc;
use alloc::collections::BTreeMap;
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use crate::subsystems::sync::Mutex;
use crate::reliability::errno::{EINVAL, ENOENT, EBUSY};

/// Interrupt vector numbers (x86_64 example)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum InterruptVector {
    // Exceptions
    DivideError = 0,
    Debug = 1,
    NonMaskableInterrupt = 2,
    Breakpoint = 3,
    Overflow = 4,
    BoundRangeExceeded = 5,
    InvalidOpcode = 6,
    DeviceNotAvailable = 7,
    DoubleFault = 8,
    InvalidTSS = 10,
    SegmentNotPresent = 11,
    StackSegmentFault = 12,
    GeneralProtectionFault = 13,
    PageFault = 14,
    X87FloatingPoint = 16,
    AlignmentCheck = 17,
    MachineCheck = 18,
    SimdFloatingPoint = 19,
    Virtualization = 20,
    Security = 30,

    // IRQ lines (platform dependent)
    Timer = 32,
    Keyboard = 33,
    Cascade = 34,
    COM2 = 35,
    COM1 = 36,
    LPT2 = 37,
    Floppy = 38,
    LPT1 = 39,
    RTC = 40,
    Free1 = 41,
    Free2 = 42,
    Free3 = 43,
    Mouse = 44,
    MathCoprocesor = 45,
    ATA1 = 46,
    ATA2 = 47,

    // Software interrupts
    SystemCall = 0x80,

    // Custom interrupt vectors
    Custom(u8),
}

impl InterruptVector {
    pub fn as_u8(self) -> u8 {
        match self {
            InterruptVector::DivideError => 0,
            InterruptVector::Debug => 1,
            InterruptVector::NonMaskableInterrupt => 2,
            InterruptVector::Breakpoint => 3,
            InterruptVector::Overflow => 4,
            InterruptVector::BoundRangeExceeded => 5,
            InterruptVector::InvalidOpcode => 6,
            InterruptVector::DeviceNotAvailable => 7,
            InterruptVector::DoubleFault => 8,
            InterruptVector::InvalidTSS => 10,
            InterruptVector::SegmentNotPresent => 11,
            InterruptVector::StackSegmentFault => 12,
            InterruptVector::GeneralProtectionFault => 13,
            InterruptVector::PageFault => 14,
            InterruptVector::X87FloatingPoint => 16,
            InterruptVector::AlignmentCheck => 17,
            InterruptVector::MachineCheck => 18,
            InterruptVector::SimdFloatingPoint => 19,
            InterruptVector::Virtualization => 20,
            InterruptVector::Security => 30,
            InterruptVector::Timer => 32,
            InterruptVector::Keyboard => 33,
            InterruptVector::Cascade => 34,
            InterruptVector::COM2 => 35,
            InterruptVector::COM1 => 36,
            InterruptVector::LPT2 => 37,
            InterruptVector::Floppy => 38,
            InterruptVector::LPT1 => 39,
            InterruptVector::RTC => 40,
            InterruptVector::Free1 => 41,
            InterruptVector::Free2 => 42,
            InterruptVector::Free3 => 43,
            InterruptVector::Mouse => 44,
            InterruptVector::MathCoprocesor => 45,
            InterruptVector::ATA1 => 46,
            InterruptVector::ATA2 => 47,
            InterruptVector::SystemCall => 0x80,
            InterruptVector::Custom(n) => n,
        }
    }

    pub fn from_u8(n: u8) -> Self {
        match n {
            0 => InterruptVector::DivideError,
            1 => InterruptVector::Debug,
            2 => InterruptVector::NonMaskableInterrupt,
            3 => InterruptVector::Breakpoint,
            4 => InterruptVector::Overflow,
            5 => InterruptVector::BoundRangeExceeded,
            6 => InterruptVector::InvalidOpcode,
            7 => InterruptVector::DeviceNotAvailable,
            8 => InterruptVector::DoubleFault,
            10 => InterruptVector::InvalidTSS,
            11 => InterruptVector::SegmentNotPresent,
            12 => InterruptVector::StackSegmentFault,
            13 => InterruptVector::GeneralProtectionFault,
            14 => InterruptVector::PageFault,
            16 => InterruptVector::X87FloatingPoint,
            17 => InterruptVector::AlignmentCheck,
            18 => InterruptVector::MachineCheck,
            19 => InterruptVector::SimdFloatingPoint,
            20 => InterruptVector::Virtualization,
            30 => InterruptVector::Security,
            32 => InterruptVector::Timer,
            33 => InterruptVector::Keyboard,
            34 => InterruptVector::Cascade,
            35 => InterruptVector::COM2,
            36 => InterruptVector::COM1,
            37 => InterruptVector::LPT2,
            38 => InterruptVector::Floppy,
            39 => InterruptVector::LPT1,
            40 => InterruptVector::RTC,
            41 => InterruptVector::Free1,
            42 => InterruptVector::Free2,
            43 => InterruptVector::Free3,
            44 => InterruptVector::Mouse,
            45 => InterruptVector::MathCoprocesor,
            46 => InterruptVector::ATA1,
            47 => InterruptVector::ATA2,
            0x80 => InterruptVector::SystemCall,
            _ => InterruptVector::Custom(n),
        }
    }
}

/// Interrupt handler function type
pub type InterruptHandler = extern "C" fn(&InterruptContext);

/// Interrupt context saved by hardware/low-level code
#[derive(Debug, Clone)]
pub struct InterruptContext {
    pub vector: InterruptVector,
    pub error_code: Option<u64>,
    pub rip: u64,         // Instruction pointer
    pub cs: u64,          // Code segment
    pub rflags: u64,      // CPU flags
    pub rsp: u64,         // Stack pointer
    pub ss: u64,          // Stack segment
    pub registers: Registers,
}

/// CPU registers at interrupt time
#[derive(Debug, Clone)]
pub struct Registers {
    pub rax: u64,
    pub rbx: u64,
    pub rcx: u64,
    pub rdx: u64,
    pub rsi: u64,
    pub rdi: u64,
    pub rbp: u64,
    pub r8: u64,
    pub r9: u64,
    pub r10: u64,
    pub r11: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            rax: 0, rbx: 0, rcx: 0, rdx: 0,
            rsi: 0, rdi: 0, rbp: 0,
            r8: 0, r9: 0, r10: 0, r11: 0,
            r12: 0, r13: 0, r14: 0, r15: 0,
        }
    }
}

impl InterruptContext {
    pub fn new(vector: InterruptVector) -> Self {
        Self {
            vector,
            error_code: None,
            rip: 0,
            cs: 0,
            rflags: 0,
            rsp: 0,
            ss: 0,
            registers: Registers::new(),
        }
    }

    pub fn is_exception(&self) -> bool {
        matches!(self.vector,
            InterruptVector::DivideError | InterruptVector::Debug |
            InterruptVector::NonMaskableInterrupt | InterruptVector::Breakpoint |
            InterruptVector::Overflow | InterruptVector::BoundRangeExceeded |
            InterruptVector::InvalidOpcode | InterruptVector::DeviceNotAvailable |
            InterruptVector::DoubleFault | InterruptVector::InvalidTSS |
            InterruptVector::SegmentNotPresent | InterruptVector::StackSegmentFault |
            InterruptVector::GeneralProtectionFault | InterruptVector::PageFault |
            InterruptVector::X87FloatingPoint | InterruptVector::AlignmentCheck |
            InterruptVector::MachineCheck | InterruptVector::SimdFloatingPoint |
            InterruptVector::Virtualization | InterruptVector::Security
        )
    }

    pub fn is_irq(&self) -> bool {
        self.vector.as_u8() >= 32 && self.vector.as_u8() <= 47
    }

    pub fn is_software_interrupt(&self) -> bool {
        matches!(self.vector, InterruptVector::SystemCall | InterruptVector::Custom(_))
    }
}

/// Interrupt handler entry
#[derive(Debug)]
pub struct InterruptHandlerEntry {
    pub handler: InterruptHandler,
    pub enabled: bool,
    pub priority: u8,
    pub call_count: AtomicU64,
    pub total_time: AtomicU64, // Total time spent in handler (nanoseconds)
}

impl Clone for InterruptHandlerEntry {
    fn clone(&self) -> Self {
        Self::new(self.handler, self.priority)
    }
}

impl InterruptHandlerEntry {
    pub fn new(handler: InterruptHandler, priority: u8) -> Self {
        Self {
            handler,
            enabled: true,
            priority,
            call_count: AtomicU64::new(0),
            total_time: AtomicU64::new(0),
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn increment_call_count(&self) {
        self.call_count.fetch_add(1, Ordering::SeqCst);
    }

    pub fn add_execution_time(&self, time_ns: u64) {
        self.total_time.fetch_add(time_ns, Ordering::SeqCst);
    }

    pub fn get_call_count(&self) -> u64 {
        self.call_count.load(Ordering::SeqCst)
    }

    pub fn get_total_time(&self) -> u64 {
        self.total_time.load(Ordering::SeqCst)
    }

    pub fn get_average_time(&self) -> u64 {
        let call_count = self.get_call_count();
        if call_count == 0 {
            0
        } else {
            self.get_total_time() / call_count
        }
    }
}

/// Interrupt statistics
#[derive(Debug)]
pub struct InterruptStats {
    pub total_interrupts: AtomicU64,
    pub exceptions: AtomicU64,
    pub irqs: AtomicU64,
    pub software_interrupts: AtomicU64,
    pub spurious_interrupts: AtomicU64,
}

impl InterruptStats {
    pub const fn new() -> Self {
        Self {
            total_interrupts: AtomicU64::new(0),
            exceptions: AtomicU64::new(0),
            irqs: AtomicU64::new(0),
            software_interrupts: AtomicU64::new(0),
            spurious_interrupts: AtomicU64::new(0),
        }
    }

    pub fn increment_total(&self) {
        self.total_interrupts.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_exceptions(&self) {
        self.exceptions.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_irqs(&self) {
        self.irqs.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_software_interrupts(&self) {
        self.software_interrupts.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_spurious(&self) {
        self.spurious_interrupts.fetch_add(1, Ordering::SeqCst);
    }

    pub fn get_total(&self) -> u64 {
        self.total_interrupts.load(Ordering::SeqCst)
    }

    pub fn get_exceptions(&self) -> u64 {
        self.exceptions.load(Ordering::SeqCst)
    }

    pub fn get_irqs(&self) -> u64 {
        self.irqs.load(Ordering::SeqCst)
    }

    pub fn get_software_interrupts(&self) -> u64 {
        self.software_interrupts.load(Ordering::SeqCst)
    }

    pub fn get_spurious(&self) -> u64 {
        self.spurious_interrupts.load(Ordering::SeqCst)
    }
}

/// Interrupt vector table
pub struct VectorTable {
    pub entries: Mutex<BTreeMap<u8, InterruptHandlerEntry>>,
    pub stats: InterruptStats,
}

impl VectorTable {
    pub fn new() -> Self {
        Self {
            entries: Mutex::new(BTreeMap::new()),
            stats: InterruptStats::new(),
        }
    }

    pub fn register_handler(&self, vector: u8, handler: InterruptHandler, priority: u8) -> Result<(), i32> {
        let mut entries = self.entries.lock();

        if entries.contains_key(&vector) {
            return Err(EBUSY);
        }

        let entry = InterruptHandlerEntry::new(handler, priority);
        entries.insert(vector, entry);

        Ok(())
    }

    pub fn unregister_handler(&self, vector: u8) -> Result<(), i32> {
        let mut entries = self.entries.lock();

        if entries.remove(&vector).is_none() {
            return Err(ENOENT);
        }

        Ok(())
    }

    pub fn get_handler(&self, vector: u8) -> Option<InterruptHandler> {
        let entries = self.entries.lock();
        entries.get(&vector).filter(|e| e.enabled).map(|e| e.handler)
    }

    pub fn enable_handler(&self, vector: u8) -> Result<(), i32> {
        let mut entries = self.entries.lock();

        let entry = entries.get_mut(&vector).ok_or(ENOENT)?;
        entry.enable();

        Ok(())
    }

    pub fn disable_handler(&self, vector: u8) -> Result<(), i32> {
        let mut entries = self.entries.lock();

        let entry = entries.get_mut(&vector).ok_or(ENOENT)?;
        entry.disable();

        Ok(())
    }

    pub fn handle_interrupt(&self, context: &mut InterruptContext) {
        let vector = context.vector.as_u8();

        // Update statistics
        self.stats.increment_total();

        if context.is_exception() {
            self.stats.increment_exceptions();
        } else if context.is_irq() {
            self.stats.increment_irqs();
        } else if context.is_software_interrupt() {
            self.stats.increment_software_interrupts();
        }

        // Find and call handler
        let entries = self.entries.lock();
        if let Some(entry) = entries.get(&vector) {
            if entry.enabled {
                entry.increment_call_count();

                let start_time = get_current_time();
                (entry.handler)(context);
                let end_time = get_current_time();

                entry.add_execution_time(end_time - start_time);

                // Update global interrupt statistics
                super::MICROKERNEL_STATS.interrupt_count.fetch_add(1, Ordering::SeqCst);
            } else {
                // Handler disabled
                self.stats.increment_spurious();
            }
        } else {
            // No handler registered
            self.stats.increment_spurious();
        }
    }

    pub fn get_stats(&self) -> InterruptStats {
        InterruptStats {
            total_interrupts: AtomicU64::new(self.stats.get_total()),
            exceptions: AtomicU64::new(self.stats.get_exceptions()),
            irqs: AtomicU64::new(self.stats.get_irqs()),
            software_interrupts: AtomicU64::new(self.stats.get_software_interrupts()),
            spurious_interrupts: AtomicU64::new(self.stats.get_spurious()),
        }
    }

    pub fn get_handler_stats(&self, vector: u8) -> Option<InterruptHandlerEntry> {
        let entries = self.entries.lock();
        entries.get(&vector).cloned()
    }
}

/// Microkernel interrupt manager
pub struct MicroInterruptHandler {
    pub vector_table: VectorTable,
    pub interrupt_count: AtomicUsize,
    pub nested_count: AtomicUsize,
    pub current_interrupt: AtomicU64,
}

impl MicroInterruptHandler {
    pub fn new() -> Self {
        Self {
            vector_table: VectorTable::new(),
            interrupt_count: AtomicUsize::new(0),
            nested_count: AtomicUsize::new(0),
            current_interrupt: AtomicU64::new(0),
        }
    }

    pub fn register_interrupt_handler(&self, vector: InterruptVector, handler: InterruptHandler, priority: u8) -> Result<(), i32> {
        self.vector_table.register_handler(vector.as_u8(), handler, priority)
    }

    pub fn unregister_interrupt_handler(&self, vector: InterruptVector) -> Result<(), i32> {
        self.vector_table.unregister_handler(vector.as_u8())
    }

    pub fn enable_interrupt(&self, vector: InterruptVector) -> Result<(), i32> {
        self.vector_table.enable_handler(vector.as_u8())
    }

    pub fn disable_interrupt(&self, vector: InterruptVector) -> Result<(), i32> {
        self.vector_table.disable_handler(vector.as_u8())
    }

    pub fn handle_interrupt(&mut self, vector: u8, error_code: Option<u64>, context: &mut InterruptContext) {
        self.interrupt_count.fetch_add(1, Ordering::SeqCst);
        self.nested_count.fetch_add(1, Ordering::SeqCst);
        self.current_interrupt.store(vector as u64, Ordering::SeqCst);

        context.vector = InterruptVector::from_u8(vector);
        context.error_code = error_code;

        self.vector_table.handle_interrupt(context);

        self.nested_count.fetch_sub(1, Ordering::SeqCst);
        if self.nested_count.load(Ordering::SeqCst) == 0 {
            self.current_interrupt.store(0, Ordering::SeqCst);
        }
    }

    pub fn is_in_interrupt(&self) -> bool {
        self.nested_count.load(Ordering::SeqCst) > 0
    }

    pub fn get_current_interrupt(&self) -> Option<InterruptVector> {
        let vector = self.current_interrupt.load(Ordering::SeqCst);
        if vector == 0 {
            None
        } else {
            Some(InterruptVector::from_u8(vector as u8))
        }
    }

    pub fn get_interrupt_count(&self) -> usize {
        self.interrupt_count.load(Ordering::SeqCst)
    }

    pub fn get_nested_count(&self) -> usize {
        self.nested_count.load(Ordering::SeqCst)
    }

    pub fn get_stats(&self) -> InterruptStats {
        self.vector_table.get_stats()
    }
}

/// Default interrupt handlers
extern "C" fn default_exception_handler(context: &InterruptContext) {
    crate::println!("Exception {}: Error code: {:?}",
        context.vector.as_u8(),
        context.error_code
    );
    crate::println!("RIP: 0x{:x}, RSP: 0x{:x}, RFLAGS: 0x{:x}",
        context.rip, context.rsp, context.rflags
    );

    // In a real system, this would terminate the current process or panic
    panic!("Unhandled exception");
}

extern "C" fn default_irq_handler(_context: &InterruptContext) {
    // Default IRQ handler - acknowledge and return
    // In a real system, this would handle the specific IRQ
}

extern "C" fn default_system_call_handler(context: &InterruptContext) {
    // System call handler would be called here
    // In a real system, this would dispatch to the appropriate system call
    crate::println!("System call from RIP: 0x{:x}", context.rip);
}

extern "C" fn default_spurious_handler(_context: &InterruptContext) {
    // Spurious interrupt - do nothing but count
}

/// Global interrupt handler
static mut GLOBAL_INTERRUPT_HANDLER: Option<MicroInterruptHandler> = None;
static INTERRUPT_INIT: AtomicUsize = AtomicUsize::new(0);

/// Initialize interrupt subsystem
pub fn init() -> Result<(), i32> {
    if INTERRUPT_INIT.load(Ordering::SeqCst) != 0 {
        return Ok(());
    }

    let handler = MicroInterruptHandler::new();

    // Register default handlers
    handler.register_interrupt_handler(InterruptVector::GeneralProtectionFault, default_exception_handler, 0)?;
    handler.register_interrupt_handler(InterruptVector::PageFault, default_exception_handler, 0)?;
    handler.register_interrupt_handler(InterruptVector::Timer, default_irq_handler, 0)?;
    handler.register_interrupt_handler(InterruptVector::SystemCall, default_system_call_handler, 0)?;

    unsafe {
        GLOBAL_INTERRUPT_HANDLER = Some(handler);
    }

    INTERRUPT_INIT.store(1, Ordering::SeqCst);
    Ok(())
}

/// Get global interrupt handler
pub fn get_interrupt_handler() -> Option<&'static mut MicroInterruptHandler> {
    unsafe {
        GLOBAL_INTERRUPT_HANDLER.as_mut()
    }
}

/// Enable interrupts globally
pub fn enable_interrupts() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("sti");
    }
}

/// Disable interrupts globally
pub fn disable_interrupts() {
    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!("cli");
    }
}

/// Check if interrupts are enabled
pub fn are_interrupts_enabled() -> bool {
    #[cfg(target_arch = "x86_64")]
    {
        let rflags: u64;
        unsafe {
            core::arch::asm!("pushfq; pop {}", out(reg) rflags);
        }
        (rflags & (1 << 9)) != 0 // IF flag
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        false // Placeholder for other architectures
    }
}

/// Get current time in nanoseconds
fn get_current_time() -> u64 {
    crate::subsystems::time::get_time_ns()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_vector() {
        assert_eq!(InterruptVector::Timer.as_u8(), 32);
        assert_eq!(InterruptVector::SystemCall.as_u8(), 0x80);

        let custom = InterruptVector::Custom(200);
        assert_eq!(custom.as_u8(), 200);

        assert_eq!(InterruptVector::from_u8(32), InterruptVector::Timer);
        assert_eq!(InterruptVector::from_u8(0x80), InterruptVector::SystemCall);
        assert_eq!(InterruptVector::from_u8(200), InterruptVector::Custom(200));
    }

    #[test]
    fn test_interrupt_context() {
        let context = InterruptContext::new(InterruptVector::PageFault);
        assert_eq!(context.vector, InterruptVector::PageFault);
        assert!(context.is_exception());
        assert!(!context.is_irq());
        assert!(!context.is_software_interrupt());

        let irq_context = InterruptContext::new(InterruptVector::Timer);
        assert!(!irq_context.is_exception());
        assert!(irq_context.is_irq());
        assert!(!irq_context.is_software_interrupt());

        let syscall_context = InterruptContext::new(InterruptVector::SystemCall);
        assert!(!syscall_context.is_exception());
        assert!(!syscall_context.is_irq());
        assert!(syscall_context.is_software_interrupt());
    }

    #[test]
    fn test_interrupt_handler_entry() {
        extern "C" fn test_handler(_context: &InterruptContext) {}

        let mut entry = InterruptHandlerEntry::new(test_handler, 5);
        assert!(entry.enabled);
        assert_eq!(entry.priority, 5);
        assert_eq!(entry.get_call_count(), 0);
        assert_eq!(entry.get_average_time(), 0);

        entry.increment_call_count();
        entry.add_execution_time(1000);

        assert_eq!(entry.get_call_count(), 1);
        assert_eq!(entry.get_total_time(), 1000);
        assert_eq!(entry.get_average_time(), 1000);

        entry.disable();
        assert!(!entry.enabled);
    }

    #[test]
    fn test_interrupt_stats() {
        let stats = InterruptStats::new();

        assert_eq!(stats.get_total(), 0);
        assert_eq!(stats.get_exceptions(), 0);
        assert_eq!(stats.get_irqs(), 0);

        stats.increment_total();
        stats.increment_exceptions();
        stats.increment_irqs();

        assert_eq!(stats.get_total(), 3);
        assert_eq!(stats.get_exceptions(), 1);
        assert_eq!(stats.get_irqs(), 1);
    }

    #[test]
    fn test_vector_table() {
        let table = VectorTable::new();

        extern "C" fn test_handler(_context: &InterruptContext) {}

        assert_eq!(table.register_handler(50, test_handler, 5), Ok(()));

        // Handler should be enabled by default
        assert_eq!(table.get_handler(50), Some(test_handler));

        // Disable handler
        assert_eq!(table.disable_handler(50), Ok(()));
        assert_eq!(table.get_handler(50), None);

        // Enable handler
        assert_eq!(table.enable_handler(50), Ok(()));
        assert_eq!(table.get_handler(50), Some(test_handler));

        // Unregister handler
        assert_eq!(table.unregister_handler(50), Ok(()));
        assert_eq!(table.get_handler(50), None);
    }
}
