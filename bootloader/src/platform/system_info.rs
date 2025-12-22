// System information collection during bootloader execution

pub struct SystemInfo {
    pub total_memory: u64,
    pub available_memory: u64,
    pub cpu_count: u32,
    pub cpu_features: u32,
}

impl SystemInfo {
    pub fn new() -> Self {
        Self {
            total_memory: 0,
            available_memory: 0,
            cpu_count: 1,
            cpu_features: 0,
        }
    }

    pub fn detect() -> Self {
        let mut info = Self::new();
        
        #[cfg(target_arch = "x86_64")]
        {
            info.detect_x86_64();
        }
        
        #[cfg(target_arch = "aarch64")]
        {
            info.detect_aarch64();
        }
        
        #[cfg(target_arch = "riscv64")]
        {
            info.detect_riscv64();
        }
        
        info
    }

    #[cfg(target_arch = "x86_64")]
    fn detect_x86_64(&mut self) {
        // Detect CPU features via CPUID
        // For bootloader, assume 1 CPU and basic features
        self.cpu_count = 1;
        self.cpu_features = 0x01; // MMU support
        
        // Memory detection would use Multiboot2 info
        self.total_memory = 1024 * 1024 * 1024; // 1GB assumed
        self.available_memory = self.total_memory;
    }

    #[cfg(target_arch = "aarch64")]
    fn detect_aarch64(&mut self) {
        // ARM64 CPU detection
        self.cpu_count = 1;
        self.cpu_features = 0x02; // NEON support assumed
        
        self.total_memory = 1024 * 1024 * 1024;
        self.available_memory = self.total_memory;
    }

    #[cfg(target_arch = "riscv64")]
    fn detect_riscv64(&mut self) {
        // RISC-V CPU detection
        self.cpu_count = 1;
        self.cpu_features = 0x04; // RV64I base ISA
        
        self.total_memory = 1024 * 1024 * 1024;
        self.available_memory = self.total_memory;
    }

    pub fn print_info(&self) {
        crate::drivers::console::write_str("System Info:\n");
        crate::drivers::console::write_str("  CPUs: ");
        crate::drivers::console::write_str(if self.cpu_count > 0 { "1" } else { "0" });
        crate::drivers::console::write_str("\n");
        crate::drivers::console::write_str("  Memory: ");
        print_memory_size(self.total_memory);
        crate::drivers::console::write_str("\n");
    }
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self::new()
    }
}

fn print_memory_size(bytes: u64) {
    if bytes >= 1024 * 1024 * 1024 {
        let gb = bytes / (1024 * 1024 * 1024);
        crate::drivers::console::write_str(if gb > 0 { "~" } else { "0" });
        crate::drivers::console::write_str("1GB");
    } else if bytes >= 1024 * 1024 {
        crate::drivers::console::write_str("MB");
    } else {
        crate::drivers::console::write_str("KB");
    }
}
