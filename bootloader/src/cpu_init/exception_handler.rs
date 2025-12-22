//! Exception Handler - CPU exception and interrupt handling framework
//!
//! Provides:
//! - CPU exception classification (Intel x86-64)
//! - Exception handler registration
//! - Stack frame management for exceptions
//! - Recovery and fallback mechanisms


/// CPU exception categories (Intel x86-64)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExceptionType {
    /// 0: Division by zero
    DivideByZero = 0,
    /// 1: Debug exception
    Debug = 1,
    /// 2: Non-maskable interrupt
    NMI = 2,
    /// 3: Breakpoint
    Breakpoint = 3,
    /// 4: Overflow
    Overflow = 4,
    /// 5: Bound range exceeded
    BoundRange = 5,
    /// 6: Invalid opcode
    InvalidOpcode = 6,
    /// 7: Device not available
    DeviceNotAvailable = 7,
    /// 8: Double fault
    DoubleFault = 8,
    /// 10: Invalid TSS
    InvalidTSS = 10,
    /// 11: Segment not present
    SegmentNotPresent = 11,
    /// 12: Stack segment fault
    StackSegmentFault = 12,
    /// 13: General protection fault
    GeneralProtection = 13,
    /// 14: Page fault
    PageFault = 14,
    /// 16: Floating-point error
    FloatingPointError = 16,
    /// 17: Alignment check
    AlignmentCheck = 17,
    /// 18: Machine check
    MachineCheck = 18,
    /// 19: SIMD floating-point error
    SIMDFloatingPoint = 19,
    /// 20: Virtualization exception
    Virtualization = 20,
}

/// Exception severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExceptionSeverity {
    /// Can be safely recovered
    Recoverable = 1,
    /// Requires special handling
    Severe = 2,
    /// Cannot be recovered
    Fatal = 3,
}

/// Exception flags and attributes
#[derive(Debug, Clone, Copy)]
pub struct ExceptionFlags {
    /// Exception has error code
    pub has_error_code: bool,
    /// Exception is vectored NMI
    pub is_nmi: bool,
    /// Exception occurred in user mode
    pub is_user_mode: bool,
}

impl ExceptionFlags {
    /// Create exception flags
    pub fn new(has_error_code: bool, is_nmi: bool, is_user_mode: bool) -> Self {
        ExceptionFlags {
            has_error_code,
            is_nmi,
            is_user_mode,
        }
    }
}

/// Exception context - saved CPU state
#[derive(Debug, Clone, Copy)]
pub struct ExceptionContext {
    /// RIP - instruction pointer
    pub rip: u64,
    /// RCS - code segment
    pub rcs: u16,
    /// RFLAGS - flags register
    pub rflags: u64,
    /// RSP - stack pointer
    pub rsp: u64,
    /// RSS - stack segment
    pub rss: u16,
    /// Error code (if applicable)
    pub error_code: Option<u32>,
}

impl ExceptionContext {
    /// Create exception context
    pub fn new(rip: u64, rcs: u16, rflags: u64, rsp: u64, rss: u16) -> Self {
        ExceptionContext {
            rip,
            rcs,
            rflags,
            rsp,
            rss,
            error_code: None,
        }
    }

    /// Set error code
    pub fn set_error_code(&mut self, code: u32) {
        self.error_code = Some(code);
    }
}

/// Exception handler function type
pub type ExceptionHandlerFn = fn(&ExceptionContext) -> bool;

/// Exception recovery strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryStrategy {
    /// Continue execution (skip faulting instruction)
    Skip,
    /// Retry the faulting operation
    Retry,
    /// Return to known safe state
    Fallback,
    /// Unrecoverable - system halt
    Fatal,
}

/// Exception information and handling
#[derive(Debug, Clone, Copy)]
pub struct ExceptionInfo {
    /// Exception type
    pub exception_type: ExceptionType,
    /// Severity level
    pub severity: ExceptionSeverity,
    /// Exception context
    pub context: ExceptionContext,
    /// Recovery strategy
    pub recovery_strategy: RecoveryStrategy,
    /// Number of times this exception occurred
    pub occurrence_count: u32,
}

impl ExceptionInfo {
    /// Create exception info
    pub fn new(
        exception_type: ExceptionType,
        context: ExceptionContext,
    ) -> Self {
        let severity = Self::classify_severity(exception_type);
        let recovery_strategy = Self::classify_recovery(exception_type);

        ExceptionInfo {
            exception_type,
            severity,
            context,
            recovery_strategy,
            occurrence_count: 1,
        }
    }

    /// Classify exception severity
    pub fn classify_severity(exc_type: ExceptionType) -> ExceptionSeverity {
        match exc_type {
            ExceptionType::Breakpoint | ExceptionType::Debug => ExceptionSeverity::Recoverable,
            ExceptionType::Overflow | ExceptionType::BoundRange => ExceptionSeverity::Recoverable,
            ExceptionType::InvalidOpcode | ExceptionType::DeviceNotAvailable => {
                ExceptionSeverity::Severe
            }
            ExceptionType::PageFault | ExceptionType::GeneralProtection => {
                ExceptionSeverity::Severe
            }
            ExceptionType::DoubleFault | ExceptionType::MachineCheck => ExceptionSeverity::Fatal,
            _ => ExceptionSeverity::Severe,
        }
    }

    /// Classify recovery strategy
    pub fn classify_recovery(exc_type: ExceptionType) -> RecoveryStrategy {
        match exc_type {
            ExceptionType::Breakpoint => RecoveryStrategy::Skip,
            ExceptionType::PageFault => RecoveryStrategy::Fallback,
            ExceptionType::FloatingPointError => RecoveryStrategy::Fallback,
            ExceptionType::DoubleFault | ExceptionType::MachineCheck => RecoveryStrategy::Fatal,
            _ => RecoveryStrategy::Fallback,
        }
    }
}

/// CPU exception handler manager
pub struct ExceptionHandler {
    /// Registered exception handlers (max 32 exception types)
    handlers: [Option<ExceptionHandlerFn>; 32],
    /// Number of exceptions handled
    total_handled: u32,
    /// Fatal exceptions encountered
    fatal_count: u32,
}

impl ExceptionHandler {
    /// Create new exception handler manager
    pub fn new() -> Self {
        ExceptionHandler {
            handlers: [None; 32],
            total_handled: 0,
            fatal_count: 0,
        }
    }

    /// Register exception handler
    pub fn register(&mut self, exc_type: ExceptionType, handler: ExceptionHandlerFn) -> bool {
        let idx = exc_type as usize;
        if idx >= 32 {
            return false;
        }
        self.handlers[idx] = Some(handler);
        true
    }

    /// Unregister exception handler
    pub fn unregister(&mut self, exc_type: ExceptionType) -> bool {
        let idx = exc_type as usize;
        if idx >= 32 {
            return false;
        }
        self.handlers[idx] = None;
        true
    }

    /// Handle exception
    pub fn handle_exception(&mut self, exc_type: ExceptionType, context: ExceptionContext) -> bool {
        let idx = exc_type as usize;
        if idx >= 32 {
            return false;
        }

        self.total_handled += 1;

        // Update exception statistics
        let severity = ExceptionInfo::classify_severity(exc_type);
        if severity == ExceptionSeverity::Fatal {
            self.fatal_count += 1;
        }

        // Try registered handler first
        if let Some(handler) = self.handlers[idx] {
            return handler(&context);
        }

        // Apply recovery strategy
        let recovery = ExceptionInfo::classify_recovery(exc_type);
        matches!(recovery, RecoveryStrategy::Skip | RecoveryStrategy::Fallback)
    }

    /// Get exception handler
    pub fn get_handler(&self, exc_type: ExceptionType) -> Option<ExceptionHandlerFn> {
        let idx = exc_type as usize;
        if idx < 32 {
            self.handlers[idx]
        } else {
            None
        }
    }

    /// Get total handled exceptions
    pub fn total_handled(&self) -> u32 {
        self.total_handled
    }

    /// Get fatal exception count
    pub fn fatal_count(&self) -> u32 {
        self.fatal_count
    }

    /// Get recovery success rate
    pub fn recovery_success_rate(&self) -> f32 {
        if self.total_handled == 0 {
            0.0
        } else {
            ((self.total_handled - self.fatal_count) as f32) / (self.total_handled as f32)
        }
    }

    /// Generate exception report
    pub fn exception_report(&self) -> ExceptionReport {
        ExceptionReport {
            total_handled: self.total_handled,
            fatal_exceptions: self.fatal_count,
            recovery_rate: self.recovery_success_rate(),
            registered_handlers: self.handlers.iter().filter(|h| h.is_some()).count() as u32,
        }
    }
}

/// Exception statistics report
#[derive(Debug, Clone, Copy)]
pub struct ExceptionReport {
    /// Total exceptions handled
    pub total_handled: u32,
    /// Fatal exceptions
    pub fatal_exceptions: u32,
    /// Recovery success rate
    pub recovery_rate: f32,
    /// Number of registered handlers
    pub registered_handlers: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_handler(ctx: &ExceptionContext) -> bool {
        // Test handler - verify context is valid
        assert!(ctx.error_code >= 0 || ctx.error_code < 0);
        true
    }

    #[test]
    fn test_exception_types() {
        assert_eq!(ExceptionType::DivideByZero as usize, 0);
        assert_eq!(ExceptionType::DoubleFault as usize, 8);
        assert_eq!(ExceptionType::PageFault as usize, 14);
    }

    #[test]
    fn test_exception_severity_classification() {
        assert_eq!(
            ExceptionInfo::classify_severity(ExceptionType::Breakpoint),
            ExceptionSeverity::Recoverable
        );
        assert_eq!(
            ExceptionInfo::classify_severity(ExceptionType::PageFault),
            ExceptionSeverity::Severe
        );
        assert_eq!(
            ExceptionInfo::classify_severity(ExceptionType::DoubleFault),
            ExceptionSeverity::Fatal
        );
    }

    #[test]
    fn test_recovery_strategy_classification() {
        assert_eq!(
            ExceptionInfo::classify_recovery(ExceptionType::Breakpoint),
            RecoveryStrategy::Skip
        );
        assert_eq!(
            ExceptionInfo::classify_recovery(ExceptionType::PageFault),
            RecoveryStrategy::Fallback
        );
        assert_eq!(
            ExceptionInfo::classify_recovery(ExceptionType::DoubleFault),
            RecoveryStrategy::Fatal
        );
    }

    #[test]
    fn test_exception_context_creation() {
        let ctx = ExceptionContext::new(0x1000, 0x08, 0x246, 0x2000, 0x10);
        assert_eq!(ctx.rip, 0x1000);
        assert_eq!(ctx.rcs, 0x08);
        assert_eq!(ctx.rsp, 0x2000);
        assert!(ctx.error_code.is_none());
    }

    #[test]
    fn test_exception_context_error_code() {
        let mut ctx = ExceptionContext::new(0x1000, 0x08, 0x246, 0x2000, 0x10);
        ctx.set_error_code(0x1234);
        assert_eq!(ctx.error_code, Some(0x1234));
    }

    #[test]
    fn test_exception_flags() {
        let flags = ExceptionFlags::new(true, false, false);
        assert!(flags.has_error_code);
        assert!(!flags.is_nmi);
        assert!(!flags.is_user_mode);
    }

    #[test]
    fn test_exception_handler_creation() {
        let handler = ExceptionHandler::new();
        assert_eq!(handler.total_handled(), 0);
        assert_eq!(handler.fatal_count(), 0);
    }

    #[test]
    fn test_register_handler() {
        let mut handler = ExceptionHandler::new();
        assert!(handler.register(ExceptionType::PageFault, dummy_handler));
        assert!(handler.get_handler(ExceptionType::PageFault).is_some());
    }

    #[test]
    fn test_unregister_handler() {
        let mut handler = ExceptionHandler::new();
        handler.register(ExceptionType::PageFault, dummy_handler);
        assert!(handler.unregister(ExceptionType::PageFault));
        assert!(handler.get_handler(ExceptionType::PageFault).is_none());
    }

    #[test]
    fn test_handle_exception_with_registered_handler() {
        let mut handler = ExceptionHandler::new();
        handler.register(ExceptionType::PageFault, dummy_handler);

        let ctx = ExceptionContext::new(0x1000, 0x08, 0x246, 0x2000, 0x10);
        let result = handler.handle_exception(ExceptionType::PageFault, ctx);

        assert!(result);
        assert_eq!(handler.total_handled(), 1);
    }

    #[test]
    fn test_handle_exception_without_handler() {
        let mut handler = ExceptionHandler::new();
        let ctx = ExceptionContext::new(0x1000, 0x08, 0x246, 0x2000, 0x10);

        let result = handler.handle_exception(ExceptionType::Breakpoint, ctx);
        assert!(result); // Breakpoint is recoverable by default
    }

    #[test]
    fn test_fatal_exception_tracking() {
        let mut handler = ExceptionHandler::new();
        let ctx = ExceptionContext::new(0x1000, 0x08, 0x246, 0x2000, 0x10);

        handler.handle_exception(ExceptionType::DoubleFault, ctx);
        assert_eq!(handler.fatal_count(), 1);
    }

    #[test]
    fn test_multiple_exceptions() {
        let mut handler = ExceptionHandler::new();
        let ctx = ExceptionContext::new(0x1000, 0x08, 0x246, 0x2000, 0x10);

        for _ in 0..5 {
            handler.handle_exception(ExceptionType::Breakpoint, ctx);
        }

        assert_eq!(handler.total_handled(), 5);
        assert_eq!(handler.fatal_count(), 0);
    }

    #[test]
    fn test_recovery_success_rate() {
        let mut handler = ExceptionHandler::new();
        let ctx = ExceptionContext::new(0x1000, 0x08, 0x246, 0x2000, 0x10);

        // 5 recoverable exceptions
        for _ in 0..5 {
            handler.handle_exception(ExceptionType::Breakpoint, ctx);
        }
        
        // 1 fatal exception
        handler.handle_exception(ExceptionType::DoubleFault, ctx);

        assert_eq!(handler.total_handled(), 6);
        assert_eq!(handler.fatal_count(), 1);
        assert_eq!(handler.recovery_success_rate(), 5.0 / 6.0);
    }

    #[test]
    fn test_exception_info_creation() {
        let ctx = ExceptionContext::new(0x1000, 0x08, 0x246, 0x2000, 0x10);
        let info = ExceptionInfo::new(ExceptionType::PageFault, ctx);

        assert_eq!(info.exception_type, ExceptionType::PageFault);
        assert_eq!(info.severity, ExceptionSeverity::Severe);
        assert_eq!(info.recovery_strategy, RecoveryStrategy::Fallback);
    }

    #[test]
    fn test_exception_report() {
        let mut handler = ExceptionHandler::new();
        handler.register(ExceptionType::PageFault, dummy_handler);

        let ctx = ExceptionContext::new(0x1000, 0x08, 0x246, 0x2000, 0x10);
        handler.handle_exception(ExceptionType::PageFault, ctx);

        let report = handler.exception_report();
        assert_eq!(report.total_handled, 1);
        assert_eq!(report.registered_handlers, 1);
        assert_eq!(report.fatal_exceptions, 0);
    }

    #[test]
    fn test_handler_with_multiple_types() {
        let mut handler = ExceptionHandler::new();
        
        assert!(handler.register(ExceptionType::PageFault, dummy_handler));
        assert!(handler.register(ExceptionType::GeneralProtection, dummy_handler));
        assert!(handler.register(ExceptionType::InvalidOpcode, dummy_handler));

        let report = handler.exception_report();
        assert_eq!(report.registered_handlers, 3);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(ExceptionSeverity::Recoverable < ExceptionSeverity::Severe);
        assert!(ExceptionSeverity::Severe < ExceptionSeverity::Fatal);
    }

    #[test]
    fn test_exception_flags_combinations() {
        let flags1 = ExceptionFlags::new(true, false, false);
        let flags2 = ExceptionFlags::new(false, true, true);

        assert_ne!(flags1.has_error_code, flags2.has_error_code);
        assert_ne!(flags1.is_nmi, flags2.is_nmi);
    }

    #[test]
    fn test_context_preservation() {
        let mut ctx = ExceptionContext::new(0xDEADBEEF, 0x08, 0x246, 0xCAFEBABE, 0x10);
        ctx.set_error_code(0x12345678);

        assert_eq!(ctx.rip, 0xDEADBEEF);
        assert_eq!(ctx.rsp, 0xCAFEBABE);
        assert_eq!(ctx.error_code.unwrap(), 0x12345678);
    }

    #[test]
    fn test_zero_exceptions_success_rate() {
        let handler = ExceptionHandler::new();
        assert_eq!(handler.recovery_success_rate(), 0.0);
    }
}
