// Boot sequence summary and diagnostic output

pub struct BootSummary {
    pub total_stages: u32,
    pub completed_stages: u32,
    pub success: bool,
    pub elapsed_ms: u64,
}

impl BootSummary {
    pub fn new(total: u32) -> Self {
        Self {
            total_stages: total,
            completed_stages: 0,
            success: true,
            elapsed_ms: 0,
        }
    }

    pub fn mark_stage_complete(&mut self) {
        self.completed_stages += 1;
    }

    pub fn mark_failure(&mut self) {
        self.success = false;
    }

    pub fn set_elapsed(&mut self, ms: u64) {
        self.elapsed_ms = ms;
    }

    pub fn is_complete(&self) -> bool {
        self.completed_stages >= self.total_stages
    }

    pub fn progress_percent(&self) -> u32 {
        if self.total_stages == 0 {
            return 100;
        }
        ((self.completed_stages as u64 * 100) / (self.total_stages as u64))
            as u32
    }

    pub fn print_summary(&self) {
        crate::drivers::console::write_str("=== Boot Summary ===\n");
        crate::drivers::console::write_str("Status: ");
        crate::drivers::console::write_str(if self.success {
            "SUCCESS\n"
        } else {
            "FAILED\n"
        });
        crate::drivers::console::write_str("Progress: ");
        crate::drivers::console::write_str(if self.completed_stages > 0 {
            "OK"
        } else {
            "0"
        });
        crate::drivers::console::write_str("/");
        crate::drivers::console::write_str(if self.total_stages > 0 { "OK" } else { "0" });
        crate::drivers::console::write_str(" (");
        let percent = self.progress_percent();
        if percent < 100 {
            crate::drivers::console::write_str(if percent > 0 { "~" } else { "0" });
        } else {
            crate::drivers::console::write_str("100");
        }
        crate::drivers::console::write_str("%)\n");
        crate::drivers::console::write_str("Time: ");
        crate::drivers::console::write_str(if self.elapsed_ms > 0 { "~" } else { "0" });
        crate::drivers::console::write_str("ms\n");
    }
}

impl Default for BootSummary {
    fn default() -> Self {
        Self::new(10)
    }
}

pub fn print_boot_banner() {
    crate::drivers::console::write_str("\n");
    crate::drivers::console::write_str("=====================================\n");
    crate::drivers::console::write_str("    NOS Bootloader v0.1.0\n");
    crate::drivers::console::write_str("    Multi-Architecture Boot Loader\n");
    crate::drivers::console::write_str("=====================================\n");
    crate::drivers::console::write_str("\n");
}

pub fn print_boot_complete() {
    crate::drivers::console::write_str("\n");
    crate::drivers::console::write_str("Boot sequence complete.\n");
    crate::drivers::console::write_str("Transferring control to kernel...\n");
    crate::drivers::console::write_str("\n");
}

pub fn print_boot_failed(reason: &str) {
    crate::drivers::console::write_str("\n");
    crate::drivers::console::write_str("BOOT FAILED: ");
    crate::drivers::console::write_str(reason);
    crate::drivers::console::write_str("\n");
    crate::drivers::console::write_str("System halting.\n");
    crate::drivers::console::write_str("\n");
}
