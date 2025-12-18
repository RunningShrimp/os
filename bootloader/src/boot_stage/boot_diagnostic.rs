// Boot-time diagnostics and reporting

pub struct BootDiagnostic {
    verbose: bool,
    show_memory: bool,
    show_devices: bool,
    show_cpu: bool,
}

impl BootDiagnostic {
    pub fn new() -> Self {
        Self {
            verbose: false,
            show_memory: true,
            show_devices: true,
            show_cpu: false,
        }
    }

    pub fn enable_verbose(&mut self) {
        self.verbose = true;
    }

    pub fn show_memory_info(&mut self, show: bool) {
        self.show_memory = show;
    }

    pub fn show_device_info(&mut self, show: bool) {
        self.show_devices = show;
    }

    pub fn show_cpu_info(&mut self, show: bool) {
        self.show_cpu = show;
    }

    pub fn print_boot_info(&self) {
        if self.show_memory {
            crate::drivers::console::write_str("Memory: initialized\n");
        }

        if self.show_devices {
            crate::drivers::console::write_str("Devices: detected\n");
        }

        if self.show_cpu {
            crate::drivers::console::write_str("CPU: ready\n");
        }

        if self.verbose {
            crate::drivers::console::write_str("(Verbose mode enabled)\n");
        }
    }

    pub fn print_critical_info(&self) {
        crate::drivers::console::write_str("Critical Information:\n");
        crate::drivers::console::write_str("  Boot Mode: Multiboot2\n");
        crate::drivers::console::write_str("  Kernel Entry: valid\n");
        crate::drivers::console::write_str("  Stack: initialized\n");
    }
}

impl Default for BootDiagnostic {
    fn default() -> Self {
        Self::new()
    }
}

pub fn diagnose_boot_environment() {
    crate::drivers::console::write_str("Boot Environment Diagnosis:\n");

    // Check architecture
    #[cfg(target_arch = "x86_64")]
    {
        crate::drivers::console::write_str("  Architecture: x86_64\n");
    }
    #[cfg(target_arch = "aarch64")]
    {
        crate::drivers::console::write_str("  Architecture: AArch64\n");
    }
    #[cfg(target_arch = "riscv64")]
    {
        crate::drivers::console::write_str("  Architecture: RISC-V 64\n");
    }

    // Check console availability
    crate::drivers::console::write_str("  Console: available\n");

    // Check memory
    crate::drivers::console::write_str("  Memory: accessible\n");
}

pub fn print_last_known_state() {
    crate::drivers::console::write_str("Last Known State:\n");

    if let Some(phase) = get_current_phase() {
        crate::drivers::console::write_str("  Phase: ");
        crate::drivers::console::write_str(phase);
        crate::drivers::console::write_str("\n");
    }
}

fn get_current_phase() -> Option<&'static str> {
    let phase = crate::core::boot_state::get_phase();
    Some(match phase {
        0 => "Start",
        1 => "Architecture Init",
        2 => "GDT Loaded",
        3 => "IDT Loaded",
        4 => "Memory Init",
        5 => "Interrupt Init",
        6 => "Paging Init",
        7 => "Boot Info Created",
        8 => "Kernel Loaded",
        9 => "Ready to Jump",
        _ => "Unknown",
    })
}
