// Boot checklist - verify all critical components before kernel jump

pub struct BootCheckList {
    console_ok: bool,
    arch_ok: bool,
    interrupts_ok: bool,
    memory_ok: bool,
    kernel_loaded_ok: bool,
    handoff_ok: bool,
}

impl BootCheckList {
    pub fn new() -> Self {
        Self {
            console_ok: false,
            arch_ok: false,
            interrupts_ok: false,
            memory_ok: false,
            kernel_loaded_ok: false,
            handoff_ok: false,
        }
    }

    pub fn check_console(&mut self, ok: bool) {
        self.console_ok = ok;
    }

    pub fn check_arch(&mut self, ok: bool) {
        self.arch_ok = ok;
    }

    pub fn check_interrupts(&mut self, ok: bool) {
        self.interrupts_ok = ok;
    }

    pub fn check_memory(&mut self, ok: bool) {
        self.memory_ok = ok;
    }

    pub fn check_kernel(&mut self, ok: bool) {
        self.kernel_loaded_ok = ok;
    }

    pub fn check_handoff(&mut self, ok: bool) {
        self.handoff_ok = ok;
    }

    pub fn all_critical_ok(&self) -> bool {
        self.console_ok
            && self.arch_ok
            && self.memory_ok
            && self.kernel_loaded_ok
    }

    pub fn print_checklist(&self) {
        crate::drivers::console::write_str("Pre-Jump Checklist:\n");
        crate::drivers::console::write_str("  Console: ");
        crate::drivers::console::write_str(if self.console_ok { "✓\n" } else { "✗\n" });
        crate::drivers::console::write_str("  Architecture: ");
        crate::drivers::console::write_str(if self.arch_ok { "✓\n" } else { "✗\n" });
        crate::drivers::console::write_str("  Interrupts: ");
        crate::drivers::console::write_str(if self.interrupts_ok {
            "✓\n"
        } else {
            "✗\n"
        });
        crate::drivers::console::write_str("  Memory: ");
        crate::drivers::console::write_str(if self.memory_ok { "✓\n" } else { "✗\n" });
        crate::drivers::console::write_str("  Kernel: ");
        crate::drivers::console::write_str(if self.kernel_loaded_ok {
            "✓\n"
        } else {
            "✗\n"
        });
        crate::drivers::console::write_str("  Handoff: ");
        crate::drivers::console::write_str(if self.handoff_ok { "✓\n" } else { "✗\n" });
    }
}

impl Default for BootCheckList {
    fn default() -> Self {
        Self::new()
    }
}
