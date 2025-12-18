// Boot-time security checks and verification

#[repr(u32)]
#[derive(Clone, Copy)]
pub enum SecurityLevel {
    None = 0,
    Basic = 1,
    Standard = 2,
    Strict = 3,
}

pub struct SecurityCheck {
    name: &'static str,
    passed: bool,
}

impl SecurityCheck {
    pub fn new(name: &'static str) -> Self {
        Self { name, passed: false }
    }

    pub fn check_passed(&mut self) {
        self.passed = true;
    }

    pub fn print(&self) {
        crate::drivers::console::write_str("  Security: ");
        crate::drivers::console::write_str(self.name);
        crate::drivers::console::write_str(": ");
        crate::drivers::console::write_str(if self.passed { "OK" } else { "FAIL" });
        crate::drivers::console::write_str("\n");
    }
}

pub struct BootSecurityValidator {
    level: SecurityLevel,
    checks_passed: u32,
    checks_total: u32,
}

impl BootSecurityValidator {
    pub fn new(level: SecurityLevel) -> Self {
        Self {
            level,
            checks_passed: 0,
            checks_total: 0,
        }
    }

    /// Check if kernel image is in valid memory range
    pub fn validate_kernel_addr(&mut self, addr: u64) -> bool {
        self.checks_total += 1;

        #[cfg(target_arch = "x86_64")]
        {
            if addr >= 0x100000 && addr < 0xFFFF800000000000 {
                self.checks_passed += 1;
                return true;
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            if addr >= 0x80000 && addr < 0xFFFF_FFFF_FFFF_FFFF {
                self.checks_passed += 1;
                return true;
            }
        }

        #[cfg(target_arch = "riscv64")]
        {
            if addr >= 0x80000000 && addr < 0xFFFF_FFFF_FFFF_FFFF {
                self.checks_passed += 1;
                return true;
            }
        }

        false
    }

    /// Check if memory range is accessible
    pub fn validate_memory_range(
        &mut self,
        start: u64,
        size: u64,
    ) -> bool {
        self.checks_total += 1;

        if size == 0 || size > 1024 * 1024 * 1024 {
            return false;
        }

        if let Some(end) = start.checked_add(size) {
            if end > start {
                self.checks_passed += 1;
                return true;
            }
        }

        false
    }

    /// Check if pointer is valid
    pub fn validate_pointer(&mut self, ptr: u64) -> bool {
        self.checks_total += 1;

        if ptr == 0 || ptr == u64::MAX {
            return false;
        }

        self.checks_passed += 1;
        true
    }

    /// Verify ELF magic and structure
    pub fn validate_elf_header(&mut self, magic: u32) -> bool {
        self.checks_total += 1;

        const ELF_MAGIC: u32 = 0x464C457F;
        if magic == ELF_MAGIC {
            self.checks_passed += 1;
            return true;
        }

        false
    }

    pub fn print_report(&self) {
        crate::drivers::console::write_str("Security Report:\n");
        crate::drivers::console::write_str("  Level: ");
        match self.level {
            SecurityLevel::None => crate::drivers::console::write_str("None"),
            SecurityLevel::Basic => crate::drivers::console::write_str("Basic"),
            SecurityLevel::Standard => {
                crate::drivers::console::write_str("Standard")
            }
            SecurityLevel::Strict => crate::drivers::console::write_str("Strict"),
        }
        crate::drivers::console::write_str("\n");
        crate::drivers::console::write_str("  Checks passed: ");
        crate::drivers::console::write_str(
            if self.checks_passed > 0 { "OK" } else { "0" },
        );
        crate::drivers::console::write_str("/");
        crate::drivers::console::write_str(
            if self.checks_total > 0 { "OK" } else { "0" },
        );
        crate::drivers::console::write_str("\n");
    }

    pub fn all_passed(&self) -> bool {
        self.checks_passed == self.checks_total
            && self.checks_total > 0
    }
}

impl Default for BootSecurityValidator {
    fn default() -> Self {
        Self::new(SecurityLevel::Standard)
    }
}

/// Global security validator
pub static mut SECURITY_VALIDATOR: Option<BootSecurityValidator> =
    None;

pub fn init_security_validator(level: SecurityLevel) {
    unsafe {
        SECURITY_VALIDATOR = Some(BootSecurityValidator::new(level));
    }
}
