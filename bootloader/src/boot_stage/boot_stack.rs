// Boot stack allocation and management

pub const BOOT_STACK_SIZE: usize = 64 * 1024; // 64KB
pub const BOOT_STACK_ALIGN: usize = 16; // 16-byte alignment

#[cfg(target_arch = "x86_64")]
pub const BOOT_STACK_ADDR: u64 = 0x200000;

#[cfg(target_arch = "aarch64")]
pub const BOOT_STACK_ADDR: u64 = 0x200000;

#[cfg(target_arch = "riscv64")]
pub const BOOT_STACK_ADDR: u64 = 0x200000;

pub struct BootStack {
    base: u64,
    top: u64,
    size: usize,
}

impl BootStack {
    pub fn new(base: u64, size: usize) -> Self {
        Self {
            base,
            top: base + (size as u64),
            size,
        }
    }

    /// Get stack pointer (top)
    pub fn pointer(&self) -> u64 {
        self.top
    }

    /// Get stack base
    pub fn base(&self) -> u64 {
        self.base
    }

    /// Get stack size
    pub fn size(&self) -> usize {
        self.size
    }

    /// Check if address is within stack bounds
    pub fn contains(&self, addr: u64) -> bool {
        addr >= self.base && addr < self.top
    }

    /// Check if stack is valid
    pub fn is_valid(&self) -> bool {
        self.size > 0 && self.size <= 1024 * 1024 // Max 1MB
    }
}

impl Default for BootStack {
    fn default() -> Self {
        Self::new(BOOT_STACK_ADDR, BOOT_STACK_SIZE)
    }
}

/// Initialize boot stack
pub fn init_boot_stack() -> BootStack {
    let stack = BootStack::default();
    crate::drivers::console::write_str("Boot stack: ");
    crate::drivers::console::write_str(if stack.is_valid() {
        "OK\n"
    } else {
        "INVALID\n"
    });
    stack
}

/// Setup kernel stack (called before kernel jump)
pub fn setup_kernel_stack(stack: &BootStack) -> bool {
    if !stack.is_valid() {
        return false;
    }

    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            // Set RSP to stack top
            let sp = stack.pointer();
            core::arch::asm!(
                "mov rsp, {}",
                in(reg) sp,
                options(nostack)
            );
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        unsafe {
            let sp = stack.pointer();
            core::arch::asm!(
                "mov sp, {}",
                in(reg) sp,
                options(nostack)
            );
        }
    }

    #[cfg(target_arch = "riscv64")]
    {
        unsafe {
            let sp = stack.pointer();
            core::arch::asm!(
                "mv sp, {}",
                in(reg) sp,
                options(nostack)
            );
        }
    }

    true
}

pub static mut BOOT_STACK: Option<BootStack> = None;

pub fn init_global_stack() {
    unsafe {
        BOOT_STACK = Some(init_boot_stack());
    }
}

pub fn get_boot_stack() -> Option<&'static BootStack> {
    unsafe { (*(&raw const BOOT_STACK)).as_ref() }
}
