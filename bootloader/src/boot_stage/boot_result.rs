// Boot result and error handling utilities

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum BootError {
    Success = 0,
    NoMemory = 1,
    InvalidKernel = 2,
    InvalidBootInfo = 3,
    ArchNotSupported = 4,
    InterruptInitFailed = 5,
    MemoryMapInvalid = 6,
    KernelLoadFailed = 7,
    NoBootDevice = 8,
    FramebufferInitFailed = 9,
}

impl BootError {
    pub fn message(&self) -> &'static str {
        match self {
            Self::Success => "Boot successful",
            Self::NoMemory => "Out of memory",
            Self::InvalidKernel => "Invalid kernel image",
            Self::InvalidBootInfo => "Invalid boot info",
            Self::ArchNotSupported => "Unsupported architecture",
            Self::InterruptInitFailed => "Interrupt initialization failed",
            Self::MemoryMapInvalid => "Memory map invalid",
            Self::KernelLoadFailed => "Kernel load failed",
            Self::NoBootDevice => "Boot device not found",
            Self::FramebufferInitFailed => "Framebuffer init failed",
        }
    }

    pub fn code(&self) -> u32 {
        *self as u32
    }

    pub fn print(&self) {
        crate::drivers::console::write_str("ERROR: ");
        crate::drivers::console::write_str(self.message());
        crate::drivers::console::write_str(" (code ");
        print_code(self.code());
        crate::drivers::console::write_str(")\n");
    }
}

fn print_code(code: u32) {
    if code < 10 {
        crate::drivers::console::write_str("0");
    }
    if code < 100 {
        crate::drivers::console::write_str("0");
    }
}

#[repr(C)]
pub struct BootSuccess {
    pub kernel_entry: u64,
    pub boot_params_addr: u64,
    pub stack_top: u64,
}

impl BootSuccess {
    pub fn new(entry: u64) -> Self {
        Self {
            kernel_entry: entry,
            boot_params_addr: 0,
            stack_top: 0x200000,
        }
    }

    pub fn ready_to_jump(&self) -> bool {
        self.kernel_entry != 0 && self.stack_top != 0
    }
}

pub fn boot_halt(error: BootError) -> ! {
    error.print();
    crate::drivers::console::write_str("System halting.\n");
    loop {}
}

pub fn boot_success(result: BootSuccess) -> ! {
    crate::drivers::console::write_str("Boot sequence complete.\n");
    crate::drivers::console::write_str("Transferring control to kernel...\n");
    
    // Jump to kernel
    unsafe {
        #[cfg(target_arch = "x86_64")]
        core::arch::asm!("jmp {}", in(reg) result.kernel_entry);
        
        #[cfg(target_arch = "aarch64")]
        core::arch::asm!("br {}", in(reg) result.kernel_entry);
        
        #[cfg(target_arch = "riscv64")]
        core::arch::asm!("jr {}", in(reg) result.kernel_entry);
    }
    loop {}
}
