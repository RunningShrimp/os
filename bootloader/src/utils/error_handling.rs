// Advanced error handling system for bootloader

use core::arch::asm;

/// Boot error codes with detailed classification
#[repr(u32)]
#[derive(Clone, Copy, Debug)]
pub enum BootErrorCode {
    // Console errors (100-109)
    ConsoleInitFailed = 100,
    ConsoleWriteFailed = 101,

    // Architecture errors (110-119)
    ArchNotSupported = 110,
    ArchInitFailed = 111,
    LongModeUnavailable = 112,
    CpuFeatureMissing = 113,

    // Memory errors (120-129)
    MemoryMapFailed = 120,
    MemoryAllocationFailed = 121,
    MemoryValidationFailed = 122,
    MemoryMappingFailed = 123,
    PageTableInitFailed = 124,

    // GDT/IDT errors (130-139)
    GdtInitFailed = 130,
    IdtInitFailed = 131,
    SegmentLoadFailed = 132,

    // Device errors (140-149)
    DeviceDetectFailed = 140,
    UartInitFailed = 141,
    InterruptControllerFailed = 142,
    TimerInitFailed = 143,

    // Boot protocol errors (150-159)
    MultibootMagicInvalid = 150,
    MultibootVersionUnsupported = 151,
    UefiTableInvalid = 152,
    UefiServiceFailed = 153,
    BiosBootFailed = 154,

    // ELF loader errors (160-169)
    ElfMagicInvalid = 160,
    ElfFormatInvalid = 161,
    ElfSegmentLoadFailed = 162,
    ElfRelocationFailed = 163,
    ElfEntryPointInvalid = 164,
    ElfBoundCheckFailed = 165,

    // Kernel errors (170-179)
    KernelNotFound = 170,
    KernelLoadFailed = 171,
    KernelValidationFailed = 172,
    KernelSignatureInvalid = 173,
    KernelVersionIncompatible = 174,

    // Security errors (180-189)
    SecurityValidationFailed = 180,
    SignatureVerificationFailed = 181,
    MemoryProtectionFailed = 182,
    IntegrityCheckFailed = 183,
    UnsignedKernel = 184,

    // Paging errors (190-199)
    PagingInitFailed = 190,
    PageTableAllocationFailed = 191,
    PageMappingFailed = 192,
    CrRegisterWriteFailed = 193,

    // POST errors (200-209)
    PostMemoryTestFailed = 200,
    PostCpuTestFailed = 201,
    PostInterruptTestFailed = 202,
    PostPagingTestFailed = 203,

    // Generic errors (210-219)
    UnknownError = 210,
    InternalError = 211,
    NotImplemented = 212,
    TimeoutError = 213,

    // Success
    Success = 0,
}

impl BootErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            // Console
            Self::ConsoleInitFailed => "Console initialization failed",
            Self::ConsoleWriteFailed => "Console write operation failed",

            // Architecture
            Self::ArchNotSupported => "Architecture not supported",
            Self::ArchInitFailed => "Architecture initialization failed",
            Self::LongModeUnavailable => "Long mode not available",
            Self::CpuFeatureMissing => "Required CPU feature missing",

            // Memory
            Self::MemoryMapFailed => "Memory map retrieval failed",
            Self::MemoryAllocationFailed => "Memory allocation failed",
            Self::MemoryValidationFailed => "Memory validation failed",
            Self::MemoryMappingFailed => "Memory mapping failed",
            Self::PageTableInitFailed => "Page table initialization failed",

            // GDT/IDT
            Self::GdtInitFailed => "GDT initialization failed",
            Self::IdtInitFailed => "IDT initialization failed",
            Self::SegmentLoadFailed => "Segment loading failed",

            // Device
            Self::DeviceDetectFailed => "Device detection failed",
            Self::UartInitFailed => "UART initialization failed",
            Self::InterruptControllerFailed => "Interrupt controller failed",
            Self::TimerInitFailed => "Timer initialization failed",

            // Boot protocol
            Self::MultibootMagicInvalid => "Invalid Multiboot magic",
            Self::MultibootVersionUnsupported => "Unsupported Multiboot version",
            Self::UefiTableInvalid => "Invalid UEFI system table",
            Self::UefiServiceFailed => "UEFI service call failed",
            Self::BiosBootFailed => "BIOS boot failed",

            // ELF
            Self::ElfMagicInvalid => "Invalid ELF magic number",
            Self::ElfFormatInvalid => "Invalid ELF format",
            Self::ElfSegmentLoadFailed => "ELF segment load failed",
            Self::ElfRelocationFailed => "ELF relocation failed",
            Self::ElfEntryPointInvalid => "Invalid ELF entry point",
            Self::ElfBoundCheckFailed => "ELF bounds check failed",

            // Kernel
            Self::KernelNotFound => "Kernel not found",
            Self::KernelLoadFailed => "Kernel load failed",
            Self::KernelValidationFailed => "Kernel validation failed",
            Self::KernelSignatureInvalid => "Invalid kernel signature",
            Self::KernelVersionIncompatible => "Kernel version incompatible",

            // Security
            Self::SecurityValidationFailed => "Security validation failed",
            Self::SignatureVerificationFailed => "Signature verification failed",
            Self::MemoryProtectionFailed => "Memory protection failed",
            Self::IntegrityCheckFailed => "Integrity check failed",
            Self::UnsignedKernel => "Unsigned kernel detected",

            // Paging
            Self::PagingInitFailed => "Paging initialization failed",
            Self::PageTableAllocationFailed => "Page table allocation failed",
            Self::PageMappingFailed => "Page mapping failed",
            Self::CrRegisterWriteFailed => "CR register write failed",

            // POST
            Self::PostMemoryTestFailed => "POST memory test failed",
            Self::PostCpuTestFailed => "POST CPU test failed",
            Self::PostInterruptTestFailed => "POST interrupt test failed",
            Self::PostPagingTestFailed => "POST paging test failed",

            // Generic
            Self::UnknownError => "Unknown error",
            Self::InternalError => "Internal error",
            Self::NotImplemented => "Not implemented",
            Self::TimeoutError => "Operation timeout",

            Self::Success => "Success",
        }
    }

    pub fn category(&self) -> &'static str {
        let code = *self as u32;
        match code {
            100..=109 => "CONSOLE",
            110..=119 => "ARCHITECTURE",
            120..=129 => "MEMORY",
            130..=139 => "GDT/IDT",
            140..=149 => "DEVICE",
            150..=159 => "PROTOCOL",
            160..=169 => "ELF_LOADER",
            170..=179 => "KERNEL",
            180..=189 => "SECURITY",
            190..=199 => "PAGING",
            200..=209 => "POST",
            210..=219 => "GENERIC",
            _ => "UNKNOWN",
        }
    }

    pub fn is_fatal(&self) -> bool {
        match self {
            // Fatal errors prevent boot continuation
            Self::ArchNotSupported => true,
            Self::LongModeUnavailable => true,
            Self::ElfMagicInvalid => true,
            Self::ElfFormatInvalid => true,
            Self::KernelNotFound => true,
            Self::UefiTableInvalid => true,
            Self::MultibootMagicInvalid => true,
            Self::SecurityValidationFailed => true,
            Self::SignatureVerificationFailed => true,
            Self::UnsignedKernel => true,
            _ => false,
        }
    }

    pub fn can_retry(&self) -> bool {
        match self {
            // Errors that might succeed on retry
            Self::DeviceDetectFailed => true,
            Self::UartInitFailed => true,
            Self::TimeoutError => true,
            Self::MemoryAllocationFailed => true,
            _ => false,
        }
    }
}

/// Boot error context and diagnostics
pub struct BootError {
    pub code: BootErrorCode,
    pub stage: u32,
    pub context: &'static str,
}

impl BootError {
    pub fn new(code: BootErrorCode, stage: u32, context: &'static str) -> Self {
        Self { code, stage, context }
    }

    pub fn report(&self) {
        crate::drivers::console::write_str("\n=== BOOT ERROR ===\n");
        crate::drivers::console::write_str("Code: ");
        crate::drivers::console::write_str(self.code.as_str());
        crate::drivers::console::write_str("\n");

        crate::drivers::console::write_str("Category: ");
        crate::drivers::console::write_str(self.code.category());
        crate::drivers::console::write_str("\n");

        crate::drivers::console::write_str("Stage: ");
        crate::drivers::console::write_str("Boot stage ");
        crate::drivers::console::write_str("\n");

        crate::drivers::console::write_str("Context: ");
        crate::drivers::console::write_str(self.context);
        crate::drivers::console::write_str("\n");

        if self.code.is_fatal() {
            crate::drivers::console::write_str("Severity: FATAL\n");
        } else {
            crate::drivers::console::write_str("Severity: ERROR\n");
        }

        if self.code.can_retry() {
            crate::drivers::console::write_str("Retry: Possible\n");
        }

        crate::drivers::console::write_str("==================\n\n");
    }

    pub fn abort(&self) -> ! {
        self.report();
        // Use HLT instruction to halt the CPU instead of busy waiting
        loop {
            unsafe {
                asm!("hlt", options(nomem, nostack));
            }
        }
    }

    pub fn abort_if_fatal(&self) -> Result<(), &'static str> {
        if self.code.is_fatal() {
            self.abort();
        }
        Ok(())
    }
}

/// Error recovery strategies
pub enum RecoveryStrategy {
    /// Continue boot without this component
    ContinueWithoutComponent,
    /// Retry the operation
    Retry(u32), // number of retries
    /// Fall back to alternative
    FallBack,
    /// Abort boot
    Abort,
}

/// Error handler with recovery logic
pub struct ErrorHandler {
    errors: [Option<BootError>; 10],
    count: usize,
}

impl ErrorHandler {
    pub fn new() -> Self {
        Self {
            errors: [None, None, None, None, None, None, None, None, None, None],
            count: 0,
        }
    }

    pub fn record_error(&mut self, error: BootError) {
        if self.count < self.errors.len() {
            self.errors[self.count] = Some(error);
            self.count += 1;
        }
    }

    pub fn report_all(&self) {
        if self.count > 0 {
            crate::drivers::console::write_str("\n=== Boot Error Summary ===\n");
            crate::drivers::console::write_str("Total errors: ");
            // Would use write_dec here
            crate::drivers::console::write_str("\n\n");

            for error in self.errors.iter().flatten() {
                error.report();
            }
        }
    }

    pub fn has_fatal_errors(&self) -> bool {
        self.errors.iter().any(|e| {
            if let Some(error) = e {
                error.code.is_fatal()
            } else {
                false
            }
        })
    }

    pub fn error_count(&self) -> usize {
        self.count
    }
}

impl Default for ErrorHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Error context for diagnostics
pub struct DiagnosticContext {
    pub boot_stage: u32,
    pub last_successful_stage: u32,
    pub error_handler: ErrorHandler,
}

impl DiagnosticContext {
    pub fn new() -> Self {
        Self {
            boot_stage: 0,
            last_successful_stage: 0,
            error_handler: ErrorHandler::new(),
        }
    }

    pub fn set_stage(&mut self, stage: u32) {
        self.boot_stage = stage;
        self.last_successful_stage = stage;
    }

    pub fn stage_failed(&mut self, error: BootError) {
        error.report();
        self.error_handler.record_error(error);
    }

    pub fn print_diagnostics(&self) {
        crate::drivers::console::write_str("\n=== Boot Diagnostics ===\n");
        crate::drivers::console::write_str("Current stage: ");
        crate::drivers::console::write_str("\n");

        crate::drivers::console::write_str("Last successful: ");
        crate::drivers::console::write_str("\n");

        crate::drivers::console::write_str("Total errors: ");
        crate::drivers::console::write_str("\n");

        self.error_handler.report_all();
    }
}

impl Default for DiagnosticContext {
    fn default() -> Self {
        Self::new()
    }
}
