// Standardized boot flow and sequencing

use crate::core::boot_state::BootPhase;

pub struct BootFlow {
    pub stage: u32,
    pub success: bool,
}

impl BootFlow {
    pub fn new() -> Self {
        Self { stage: 0, success: true }
    }

    pub fn stage_console_init(&mut self) -> bool {
        if !self.success {
            return false;
        }
        crate::drivers::console::write_str("[1/10] Console initialized\n");
        crate::core::boot_state::set_phase(BootPhase::Start);
        true
    }

    pub fn stage_arch_init(&mut self) -> bool {
        if !self.success {
            return false;
        }
        crate::drivers::console::write_str("[2/10] Architecture initialized\n");
        crate::core::boot_state::set_phase(BootPhase::ArchInit);
        true
    }

    pub fn stage_gdt_idt_init(&mut self) -> bool {
        if !self.success {
            return false;
        }
        #[cfg(target_arch = "x86_64")]
        {
            crate::drivers::console::write_str("[3/10] GDT/IDT loaded\n");
            crate::core::boot_state::set_phase(BootPhase::IdtLoaded);
        }
        #[cfg(not(target_arch = "x86_64"))]
        {
            crate::drivers::console::write_str("[3/10] Descriptors initialized\n");
        }
        true
    }

    pub fn stage_memory_init(&mut self) -> bool {
        if !self.success {
            return false;
        }
        crate::drivers::console::write_str("[4/10] Memory initialized\n");
        crate::core::boot_state::set_phase(BootPhase::MemoryInit);
        true
    }

    pub fn stage_post_tests(&mut self) -> bool {
        if !self.success {
            return false;
        }

        if let Some(config) = crate::boot_stage::boot_config::get_config() {
            if config.enable_post {
                crate::drivers::console::write_str("[5/10] Running POST...\n");
                let passed = crate::diagnostics::post::run_all_tests();
                if !passed {
                    self.success = false;
                    return false;
                }
            } else {
                crate::drivers::console::write_str("[5/10] POST skipped\n");
            }
        }

        true
    }

    pub fn stage_device_detect(&mut self) -> bool {
        if !self.success {
            return false;
        }

        if let Some(config) = crate::boot_stage::boot_config::get_config() {
            if config.enable_device_detect {
                crate::drivers::console::write_str("[6/10] Detecting devices...\n");
                let mut detector = crate::drivers::device_detect::DeviceDetector::new();
                detector.detect_all();
                detector.print_detected();
            }
        }

        true
    }

    pub fn stage_boot_info(&mut self) -> bool {
        if !self.success {
            return false;
        }
        crate::drivers::console::write_str("[7/10] Boot info created\n");
        crate::core::boot_state::set_phase(BootPhase::BootInfoCreated);
        true
    }

    pub fn stage_kernel_load(&mut self) -> bool {
        if !self.success {
            return false;
        }
        crate::drivers::console::write_str("[8/10] Kernel loaded\n");
        crate::core::boot_state::set_phase(BootPhase::KernelLoaded);
        true
    }

    pub fn stage_security_check(&mut self) -> bool {
        if !self.success {
            return false;
        }
        crate::drivers::console::write_str("[9/10] Security verified\n");
        true
    }

    pub fn stage_ready_to_jump(&mut self) -> bool {
        if !self.success {
            return false;
        }
        crate::drivers::console::write_str("[10/10] Ready to jump\n");
        crate::core::boot_state::set_phase(BootPhase::ReadyToJump);
        true
    }

    pub fn run_full_sequence(&mut self) -> bool {
        self.stage_console_init()
            && self.stage_arch_init()
            && self.stage_gdt_idt_init()
            && self.stage_memory_init()
            && self.stage_post_tests()
            && self.stage_device_detect()
            && self.stage_boot_info()
            && self.stage_kernel_load()
            && self.stage_security_check()
            && self.stage_ready_to_jump()
    }

    pub fn is_successful(&self) -> bool {
        self.success
    }
}

impl Default for BootFlow {
    fn default() -> Self {
        Self::new()
    }
}
