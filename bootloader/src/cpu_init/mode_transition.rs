/// Mode Transition Implementation
///
/// Handles transitions between real mode, protected mode, and long mode.

/// CPU mode type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CPUMode {
    RealMode,
    ProtectedMode,
    LongMode,
}

impl CPUMode {
    pub fn description(&self) -> &'static str {
        match self {
            Self::RealMode => "Real Mode (16-bit)",
            Self::ProtectedMode => "Protected Mode (32-bit)",
            Self::LongMode => "Long Mode (64-bit)",
        }
    }

    pub fn bit_width(&self) -> u32 {
        match self {
            Self::RealMode => 16,
            Self::ProtectedMode => 32,
            Self::LongMode => 64,
        }
    }

    pub fn can_access_memory_above_1mb(&self) -> bool {
        !matches!(self, Self::RealMode)
    }
}

/// GDT descriptor
#[derive(Debug, Clone, Copy)]
pub struct GDTDescriptor {
    pub base: u32,
    pub limit: u16,
}

impl GDTDescriptor {
    pub fn new(base: u32, limit: u16) -> Self {
        Self { base, limit }
    }
}

/// IDT descriptor
#[derive(Debug, Clone, Copy)]
pub struct IDTDescriptor {
    pub base: u64,
    pub limit: u16,
}

impl IDTDescriptor {
    pub fn new(base: u64, limit: u16) -> Self {
        Self { base, limit }
    }
}

/// Paging structures
#[derive(Debug, Clone)]
pub struct PagingSetup {
    pml4_base: u64,
    paging_enabled: bool,
}

impl PagingSetup {
    pub fn new(pml4_base: u64) -> Self {
        Self {
            pml4_base,
            paging_enabled: false,
        }
    }

    pub fn enable_paging(&mut self) {
        self.paging_enabled = true;
    }

    pub fn is_enabled(&self) -> bool {
        self.paging_enabled
    }

    pub fn pml4_base(&self) -> u64 {
        self.pml4_base
    }
}

/// Mode transition manager
pub struct ModeTransitioner {
    current_mode: CPUMode,
    gdt: Option<GDTDescriptor>,
    idt: Option<IDTDescriptor>,
    paging: Option<PagingSetup>,
}

impl ModeTransitioner {
    /// Create new mode transitioner
    pub fn new() -> Self {
        Self {
            current_mode: CPUMode::RealMode,
            gdt: None,
            idt: None,
            paging: None,
        }
    }

    /// Set up GDT
    pub fn setup_gdt(&mut self, base: u32, limit: u16) {
        self.gdt = Some(GDTDescriptor::new(base, limit));
    }

    /// Set up IDT
    pub fn setup_idt(&mut self, base: u64, limit: u16) {
        self.idt = Some(IDTDescriptor::new(base, limit));
    }

    /// Set up paging
    pub fn setup_paging(&mut self, pml4_base: u64) {
        self.paging = Some(PagingSetup::new(pml4_base));
    }

    /// Transition to protected mode
    pub fn transition_to_protected_mode(&mut self) -> Result<(), &'static str> {
        if !self.gdt.is_some() {
            return Err("GDT not configured");
        }

        self.current_mode = CPUMode::ProtectedMode;

        // In real implementation:
        // 1. Disable interrupts (cli)
        // 2. Load GDT (lgdt instruction)
        // 3. Set PE bit in CR0
        // 4. Far jump to flush pipeline

        Ok(())
    }

    /// Transition to long mode
    pub fn transition_to_long_mode(&mut self) -> Result<(), &'static str> {
        if self.current_mode != CPUMode::ProtectedMode {
            return Err("Must be in protected mode");
        }

        if !self.idt.is_some() {
            return Err("IDT not configured");
        }

        if !self.paging.is_some() {
            return Err("Paging not configured");
        }

        // In real implementation:
        // 1. Disable interrupts
        // 2. Load IDT (lidt instruction)
        // 3. Set up paging structures
        // 4. Enable PAE (CR4.PAE = 1)
        // 5. Enable LME (IA32_EFER.LME = 1)
        // 6. Set PG bit in CR0
        // 7. Load CR3 with PML4 base
        // 8. Far jump to 64-bit code segment

        if let Some(paging) = &mut self.paging {
            paging.enable_paging();
        }

        self.current_mode = CPUMode::LongMode;

        Ok(())
    }

    /// Get current mode
    pub fn current_mode(&self) -> CPUMode {
        self.current_mode
    }

    /// Check if ready for transition
    pub fn is_ready_for_long_mode(&self) -> bool {
        self.gdt.is_some()
            && self.idt.is_some()
            && self.paging.is_some()
            && self.current_mode == CPUMode::ProtectedMode
    }

    /// Check if ready for protected mode
    pub fn is_ready_for_protected_mode(&self) -> bool {
        self.gdt.is_some() && self.current_mode == CPUMode::RealMode
    }

    /// Get mode info
    pub fn mode_info(&self) -> ModeInfo {
        ModeInfo {
            current_mode: self.current_mode,
            gdt_configured: self.gdt.is_some(),
            idt_configured: self.idt.is_some(),
            paging_configured: self.paging.is_some(),
        }
    }
}

/// Mode information
#[derive(Debug, Clone)]
pub struct ModeInfo {
    pub current_mode: CPUMode,
    pub gdt_configured: bool,
    pub idt_configured: bool,
    pub paging_configured: bool,
}

impl ModeInfo {
    pub fn is_ready_for_kernel(&self) -> bool {
        self.current_mode == CPUMode::LongMode
            && self.gdt_configured
            && self.idt_configured
            && self.paging_configured
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_mode_description() {
        assert_eq!(CPUMode::RealMode.description(), "Real Mode (16-bit)");
        assert_eq!(CPUMode::LongMode.description(), "Long Mode (64-bit)");
    }

    #[test]
    fn test_cpu_mode_bit_width() {
        assert_eq!(CPUMode::RealMode.bit_width(), 16);
        assert_eq!(CPUMode::ProtectedMode.bit_width(), 32);
        assert_eq!(CPUMode::LongMode.bit_width(), 64);
    }

    #[test]
    fn test_cpu_mode_memory_access() {
        assert!(!CPUMode::RealMode.can_access_memory_above_1mb());
        assert!(CPUMode::ProtectedMode.can_access_memory_above_1mb());
    }

    #[test]
    fn test_gdt_descriptor_creation() {
        let gdt = GDTDescriptor::new(0x1000, 0xFFFF);
        assert_eq!(gdt.base, 0x1000);
        assert_eq!(gdt.limit, 0xFFFF);
    }

    #[test]
    fn test_idt_descriptor_creation() {
        let idt = IDTDescriptor::new(0x2000, 0xFFF);
        assert_eq!(idt.base, 0x2000);
        assert_eq!(idt.limit, 0xFFF);
    }

    #[test]
    fn test_paging_setup_creation() {
        let paging = PagingSetup::new(0x3000);
        assert_eq!(paging.pml4_base(), 0x3000);
        assert!(!paging.is_enabled());
    }

    #[test]
    fn test_paging_enable() {
        let mut paging = PagingSetup::new(0x3000);
        paging.enable_paging();
        assert!(paging.is_enabled());
    }

    #[test]
    fn test_mode_transitioner_creation() {
        let transitioner = ModeTransitioner::new();
        assert_eq!(transitioner.current_mode(), CPUMode::RealMode);
    }

    #[test]
    fn test_mode_transitioner_setup_gdt() {
        let mut transitioner = ModeTransitioner::new();
        transitioner.setup_gdt(0x1000, 0xFFFF);
        assert!(transitioner.is_ready_for_protected_mode());
    }

    #[test]
    fn test_mode_transitioner_to_protected() {
        let mut transitioner = ModeTransitioner::new();
        transitioner.setup_gdt(0x1000, 0xFFFF);
        assert!(transitioner.transition_to_protected_mode().is_ok());
        assert_eq!(transitioner.current_mode(), CPUMode::ProtectedMode);
    }

    #[test]
    fn test_mode_transitioner_to_long_mode() {
        let mut transitioner = ModeTransitioner::new();
        transitioner.setup_gdt(0x1000, 0xFFFF);
        transitioner.setup_idt(0x2000, 0xFFF);
        transitioner.setup_paging(0x3000);

        let _ = transitioner.transition_to_protected_mode();
        assert!(transitioner.transition_to_long_mode().is_ok());
        assert_eq!(transitioner.current_mode(), CPUMode::LongMode);
    }

    #[test]
    fn test_mode_info_creation() {
        let transitioner = ModeTransitioner::new();
        let info = transitioner.mode_info();
        assert_eq!(info.current_mode, CPUMode::RealMode);
        assert!(!info.is_ready_for_kernel());
    }

    #[test]
    fn test_mode_info_ready_for_kernel() {
        let mut transitioner = ModeTransitioner::new();
        transitioner.setup_gdt(0x1000, 0xFFFF);
        transitioner.setup_idt(0x2000, 0xFFF);
        transitioner.setup_paging(0x3000);

        let _ = transitioner.transition_to_protected_mode();
        let _ = transitioner.transition_to_long_mode();

        let info = transitioner.mode_info();
        assert!(info.is_ready_for_kernel());
    }
}
