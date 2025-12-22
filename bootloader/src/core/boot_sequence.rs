/// Boot Sequence Configuration and Setup
///
/// Provides GDT/IDT configuration and boot-time memory layout setup
/// necessary for real mode operations and long mode transitions.

use core::mem;

/// Global Descriptor Table (GDT) entry
#[derive(Debug, Clone, Copy)]
pub struct GDTEntry {
    pub limit_low: u16,
    pub base_low: u16,
    pub base_mid: u8,
    pub access: u8,
    pub limit_high_flags: u8,
    pub base_high: u8,
}

impl GDTEntry {
    /// Create null GDT entry
    pub fn null() -> Self {
        Self {
            limit_low: 0,
            base_low: 0,
            base_mid: 0,
            access: 0,
            limit_high_flags: 0,
            base_high: 0,
        }
    }

    /// Create 64-bit code segment descriptor
    pub fn code64() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0,
            base_mid: 0,
            access: 0x9A,  // P=1, DPL=0, S=1, Type=A (execute/read)
            limit_high_flags: 0xA0,  // L=1 (64-bit), D=0, G=1 (4KB granularity)
            base_high: 0,
        }
    }

    /// Create 64-bit data segment descriptor
    pub fn data64() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0,
            base_mid: 0,
            access: 0x92,  // P=1, DPL=0, S=1, Type=2 (read/write)
            limit_high_flags: 0xC0,  // L=0, D=1, G=1 (4KB granularity)
            base_high: 0,
        }
    }

    /// Create 16-bit real mode segment (base 0, limit 64KB)
    pub fn real_mode() -> Self {
        Self {
            limit_low: 0xFFFF,
            base_low: 0,
            base_mid: 0,
            access: 0x9E,  // Present, Ring 0, Code, Execute/Read
            limit_high_flags: 0x00,  // 64KB limit, byte granularity
            base_high: 0,
        }
    }
}

/// Global Descriptor Table
pub struct GlobalDescriptorTable {
    entries: [GDTEntry; 3],
}

impl GlobalDescriptorTable {
    /// Create a new GDT with standard entries
    pub fn new() -> Self {
        Self {
            entries: [
                GDTEntry::null(),      // Index 0: Null descriptor
                GDTEntry::code64(),    // Index 1: 64-bit code (0x08)
                GDTEntry::data64(),    // Index 2: 64-bit data (0x10)
            ],
        }
    }

    /// Add real mode segment to GDT
    pub fn add_real_mode(&mut self) {
        if self.entries.len() < 4 {
            // Would expand entries array in real implementation
        }
    }

    /// Get GDT base address
    pub fn base(&self) -> u64 {
        self.entries.as_ptr() as u64
    }

    /// Get GDT limit (size - 1)
    pub fn limit(&self) -> u16 {
        (mem::size_of_val(&self.entries) - 1) as u16
    }

    /// Get size in bytes
    pub fn size(&self) -> usize {
        mem::size_of_val(&self.entries)
    }
}

/// Interrupt Descriptor Table (IDT) entry
#[derive(Debug, Clone, Copy)]
pub struct IDTEntry {
    pub offset_low: u16,
    pub selector: u16,
    pub ist: u8,
    pub flags: u8,
    pub offset_mid: u16,
    pub offset_high: u32,
    pub reserved: u32,
}

impl IDTEntry {
    /// Create null IDT entry
    pub fn null() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            flags: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    /// Create interrupt gate entry
    pub fn interrupt_gate(handler: u64, selector: u16) -> Self {
        Self {
            offset_low: (handler & 0xFFFF) as u16,
            selector,
            ist: 0,
            flags: 0x8E,  // Present=1, DPL=0, Type=14 (Interrupt Gate)
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: ((handler >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }

    /// Create trap gate entry
    pub fn trap_gate(handler: u64, selector: u16) -> Self {
        Self {
            offset_low: (handler & 0xFFFF) as u16,
            selector,
            ist: 0,
            flags: 0x8F,  // Present=1, DPL=0, Type=15 (Trap Gate)
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_high: ((handler >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }
}

/// Interrupt Descriptor Table
pub struct InterruptDescriptorTable {
    entries: [IDTEntry; 32],  // Support first 32 interrupts (for exception handling)
}

impl InterruptDescriptorTable {
    /// Create new IDT
    pub fn new() -> Self {
        Self {
            entries: [IDTEntry::null(); 32],
        }
    }

    /// Set IDT entry
    pub fn set_entry(&mut self, index: usize, entry: IDTEntry) {
        if index < self.entries.len() {
            self.entries[index] = entry;
        }
    }

    /// Get IDT base address
    pub fn base(&self) -> u64 {
        self.entries.as_ptr() as u64
    }

    /// Get IDT limit (size - 1)
    pub fn limit(&self) -> u16 {
        (mem::size_of_val(&self.entries) - 1) as u16
    }
}

/// Boot memory layout configuration
pub struct BootMemoryLayout {
    /// Bootloader code segment
    pub bootloader_base: u64,
    pub bootloader_size: u64,
    
    /// Stack area
    pub stack_base: u64,
    pub stack_size: u64,
    
    /// Heap area
    pub heap_base: u64,
    pub heap_size: u64,
    
    /// Real mode buffer (for BIOS calls)
    pub realmode_buffer: u64,
    pub realmode_buffer_size: u64,
    
    /// Kernel load area
    pub kernel_base: u64,
    pub kernel_max_size: u64,
}

impl BootMemoryLayout {
    /// Create default boot memory layout
    pub fn new() -> Self {
        Self {
            bootloader_base: 0x7C00,      // Traditional bootloader address
            bootloader_size: 0x200,       // 512 bytes
            
            stack_base: 0x7FFF0000,       // Stack grows downward
            stack_size: 64 * 1024,        // 64 KB
            
            heap_base: 0x7FFE0000,        // Heap grows upward
            heap_size: 256 * 1024,        // 256 KB
            
            realmode_buffer: 0x10000,     // 64 KB area for real mode operations
            realmode_buffer_size: 64 * 1024,
            
            kernel_base: 0x100000,        // 1 MB (traditional kernel load address)
            kernel_max_size: 16 * 1024 * 1024,  // 16 MB max kernel size
        }
    }

    /// Validate memory layout (no overlaps)
    pub fn validate(&self) -> Result<(), &'static str> {
        // Check for overlaps
        if self.stack_base < self.stack_size {
            return Err("Stack would overflow low memory");
        }
        
        if self.heap_base + self.heap_size > self.stack_base - self.stack_size {
            return Err("Heap and stack would overlap");
        }
        
        if self.kernel_base < self.realmode_buffer + self.realmode_buffer_size {
            return Err("Kernel load area overlaps with real mode buffer");
        }

        Ok(())
    }

    /// Get total usable bootloader memory
    pub fn total_usable(&self) -> u64 {
        self.stack_base + self.stack_size
    }
}

/// Boot sequence state
#[derive(Debug, Clone, Copy)]
pub enum BootSequenceState {
    Uninitialized,
    MemoryLayoutSet,
    GDTLoaded,
    IDTLoaded,
    RealModeReady,
    BootInfoPrepared,
    KernelReady,
}

/// Boot sequence configuration
pub struct BootSequence {
    state: BootSequenceState,
    memory_layout: BootMemoryLayout,
    gdt: GlobalDescriptorTable,
    idt: InterruptDescriptorTable,
}

impl BootSequence {
    /// Create new boot sequence
    pub fn new() -> Self {
        Self {
            state: BootSequenceState::Uninitialized,
            memory_layout: BootMemoryLayout::new(),
            gdt: GlobalDescriptorTable::new(),
            idt: InterruptDescriptorTable::new(),
        }
    }

    /// Validate memory layout
    pub fn validate_memory(&mut self) -> Result<(), &'static str> {
        self.memory_layout.validate()?;
        self.state = BootSequenceState::MemoryLayoutSet;
        Ok(())
    }

    /// Load GDT
    pub fn load_gdt(&mut self) -> Result<(), &'static str> {
        // In real implementation, would execute LGDT instruction
        self.state = BootSequenceState::GDTLoaded;
        Ok(())
    }

    /// Load IDT
    pub fn load_idt(&mut self) -> Result<(), &'static str> {
        // In real implementation, would execute LIDT instruction
        self.state = BootSequenceState::IDTLoaded;
        Ok(())
    }

    /// Prepare for real mode operations
    pub fn prepare_real_mode(&mut self) -> Result<(), &'static str> {
        if !matches!(self.state, BootSequenceState::IDTLoaded) {
            return Err("IDT not loaded");
        }
        self.state = BootSequenceState::RealModeReady;
        Ok(())
    }

    /// Get current state
    pub fn state(&self) -> BootSequenceState {
        self.state
    }

    /// Get memory layout
    pub fn memory_layout(&self) -> &BootMemoryLayout {
        &self.memory_layout
    }

    /// Get GDT
    pub fn gdt(&self) -> &GlobalDescriptorTable {
        &self.gdt
    }

    /// Get IDT
    pub fn idt(&self) -> &InterruptDescriptorTable {
        &self.idt
    }

    /// Get mutable GDT
    pub fn gdt_mut(&mut self) -> &mut GlobalDescriptorTable {
        &mut self.gdt
    }

    /// Get mutable IDT
    pub fn idt_mut(&mut self) -> &mut InterruptDescriptorTable {
        &mut self.idt
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gdt_creation() {
        let gdt = GlobalDescriptorTable::new();
        assert_eq!(gdt.entries.len(), 3);
        assert!(gdt.limit() > 0);
    }

    #[test]
    fn test_idt_creation() {
        let idt = InterruptDescriptorTable::new();
        assert_eq!(idt.entries.len(), 32);
    }

    #[test]
    fn test_memory_layout_creation() {
        let layout = BootMemoryLayout::new();
        assert!(layout.validate().is_ok());
    }

    #[test]
    fn test_memory_layout_validation() {
        let layout = BootMemoryLayout::new();
        assert!(layout.validate().is_ok());
        assert!(layout.total_usable() > 0);
    }

    #[test]
    fn test_boot_sequence_state_machine() {
        let mut seq = BootSequence::new();
        assert!(matches!(seq.state(), BootSequenceState::Uninitialized));

        assert!(seq.validate_memory().is_ok());
        assert!(matches!(seq.state(), BootSequenceState::MemoryLayoutSet));

        assert!(seq.load_gdt().is_ok());
        assert!(matches!(seq.state(), BootSequenceState::GDTLoaded));

        assert!(seq.load_idt().is_ok());
        assert!(matches!(seq.state(), BootSequenceState::IDTLoaded));

        assert!(seq.prepare_real_mode().is_ok());
        assert!(matches!(seq.state(), BootSequenceState::RealModeReady));
    }

    #[test]
    fn test_gdt_entries() {
        let null = GDTEntry::null();
        assert_eq!(null.access, 0);

        let code = GDTEntry::code64();
        assert!(code.access != 0);

        let data = GDTEntry::data64();
        assert!(data.access != 0);

        let real = GDTEntry::real_mode();
        assert_eq!(real.limit_low, 0xFFFF);
    }

    #[test]
    fn test_idt_entry_creation() {
        let handler_addr = 0x1000u64;
        let selector = 0x08u16;

        let int_gate = IDTEntry::interrupt_gate(handler_addr, selector);
        assert_eq!(int_gate.selector, selector);
        assert_eq!(int_gate.offset_low, (handler_addr & 0xFFFF) as u16);

        let trap_gate = IDTEntry::trap_gate(handler_addr, selector);
        assert_eq!(trap_gate.selector, selector);
    }
}
