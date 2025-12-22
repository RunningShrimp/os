//! IDT Manager - Interrupt Descriptor Table initialization and management
//! 
//! Provides:
//! - IDT descriptor table setup
//! - Gate descriptor types (interrupt, trap, task)
//! - IDT loading into CPU
//! - Descriptor information querying

/// Gate descriptor types in IDT
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GateType {
    /// Task gate descriptor (32-bit only)
    TaskGate = 0x5,
    /// Interrupt gate descriptor
    InterruptGate = 0xE,
    /// Trap gate descriptor
    TrapGate = 0xF,
}

/// Privilege level for IDT entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrivilegeLevel {
    /// Ring 0 - Kernel mode
    Kernel = 0,
    /// Ring 1 - System code
    System = 1,
    /// Ring 2 - System data
    Data = 2,
    /// Ring 3 - User mode
    User = 3,
}

/// Flags for IDT gate descriptor
#[derive(Debug, Clone, Copy)]
pub struct GateFlags {
    /// Gate is present (valid entry)
    pub present: bool,
    /// Privilege level required to invoke
    pub dpl: PrivilegeLevel,
    /// Storage segment (always 0 for IDT)
    pub storage_segment: bool,
}

impl GateFlags {
    /// Create gate flags
    pub fn new(present: bool, dpl: PrivilegeLevel) -> Self {
        GateFlags {
            present,
            dpl,
            storage_segment: false,
        }
    }

    /// Encode flags into descriptor byte
    pub fn encode(&self) -> u8 {
        let mut byte = 0u8;
        if self.present {
            byte |= 0x80;
        }
        byte |= ((self.dpl as u8) & 0x3) << 5;
        byte
    }
}

/// IDT gate descriptor entry (8 bytes)
#[derive(Debug, Clone, Copy)]
pub struct GateDescriptor {
    /// Handler address (lower 16 bits)
    pub offset_lo: u16,
    /// Code segment selector
    pub selector: u16,
    /// Interrupt stack table index (0-7)
    pub ist: u8,
    /// Gate type and flags
    pub gate_type: GateType,
    /// Descriptor flags (present, DPL, etc)
    pub flags: GateFlags,
    /// Handler address (upper 48 bits)
    pub offset_mid: u16,
    pub offset_hi: u32,
    /// Reserved (must be zero)
    pub reserved: u32,
}

impl GateDescriptor {
    /// Create new gate descriptor
    pub fn new(
        handler: u64,
        selector: u16,
        gate_type: GateType,
        flags: GateFlags,
    ) -> Self {
        GateDescriptor {
            offset_lo: (handler & 0xFFFF) as u16,
            selector,
            ist: 0,
            gate_type,
            flags,
            offset_mid: ((handler >> 16) & 0xFFFF) as u16,
            offset_hi: ((handler >> 32) & 0xFFFFFFFF) as u32,
            reserved: 0,
        }
    }

    /// Get full handler address
    pub fn handler_address(&self) -> u64 {
        ((self.offset_hi as u64) << 32)
            | ((self.offset_mid as u64) << 16)
            | (self.offset_lo as u64)
    }

    /// Set interrupt stack table index (0-7)
    pub fn set_ist(&mut self, ist: u8) {
        self.ist = ist & 0x7;
    }
}

/// IDT (Interrupt Descriptor Table) manager
pub struct IdtManager {
    /// IDT entries (256 entries for 8086+ mode)
    entries: [GateDescriptor; 256],
    /// Number of valid entries
    valid_count: u32,
    /// Default code segment selector
    code_selector: u16,
    /// IDT register (for LIDT instruction)
    idt_register: IdtRegister,
}

/// IDT register format for LIDT instruction
#[repr(C, packed)]
pub struct IdtRegister {
    /// Size of IDT - 1
    pub limit: u16,
    /// Linear address of IDT
    pub base: u64,
}

impl IdtManager {
    /// Create new IDT manager
    pub fn new(code_selector: u16) -> Self {
        let mut manager = IdtManager {
            entries: [GateDescriptor::new(
                0,
                0,
                GateType::InterruptGate,
                GateFlags::new(false, PrivilegeLevel::Kernel),
            ); 256],
            valid_count: 0,
            code_selector,
            idt_register: IdtRegister { limit: 0, base: 0 },
        };
        manager.update_register();
        manager
    }

    /// Register interrupt handler
    pub fn register_handler(
        &mut self,
        vector: u8,
        handler: u64,
        gate_type: GateType,
        dpl: PrivilegeLevel,
    ) -> bool {
        if vector as usize >= self.entries.len() {
            return false;
        }

        let flags = GateFlags::new(true, dpl);
        self.entries[vector as usize] =
            GateDescriptor::new(handler, self.code_selector, gate_type, flags);

        self.valid_count = self.valid_count.max((vector as u32) + 1);
        true
    }

    /// Register interrupt gate (normal interrupt)
    pub fn register_interrupt(&mut self, vector: u8, handler: u64) -> bool {
        self.register_handler(
            vector,
            handler,
            GateType::InterruptGate,
            PrivilegeLevel::Kernel,
        )
    }

    /// Register trap gate (exception/syscall)
    pub fn register_trap(&mut self, vector: u8, handler: u64) -> bool {
        self.register_handler(
            vector,
            handler,
            GateType::TrapGate,
            PrivilegeLevel::Kernel,
        )
    }

    /// Register user-accessible trap gate
    pub fn register_user_trap(&mut self, vector: u8, handler: u64) -> bool {
        self.register_handler(
            vector,
            handler,
            GateType::TrapGate,
            PrivilegeLevel::User,
        )
    }

    /// Get IDT entry
    pub fn get_descriptor(&self, vector: u8) -> Option<GateDescriptor> {
        if self.entries[vector as usize].flags.present {
            Some(self.entries[vector as usize])
        } else {
            None
        }
    }

    /// Update IDT register with current table info
    fn update_register(&mut self) {
        let limit = if self.valid_count > 0 {
            (self.valid_count * 16 - 1) as u16
        } else {
            0
        };
        self.idt_register.limit = limit;
        self.idt_register.base = self.entries.as_ptr() as u64;
    }

    /// Load IDT into CPU (LIDT instruction)
    pub fn load(&mut self) -> bool {
        self.update_register();
        
        // Execute LIDT instruction with IDT register
        #[cfg(target_arch = "x86_64")]
        unsafe {
            use core::arch::asm;
            asm!("lidt [{}]", in(reg) &self.idt_register, options(nostack));
        }
        
        #[cfg(not(target_arch = "x86_64"))]
        {
            // Non-x86 architectures configure interrupt handlers differently
        }
        true
    }

    /// Get number of registered handlers
    pub fn registered_count(&self) -> u32 {
        let mut count = 0;
        for i in 0..self.valid_count as usize {
            if self.entries[i].flags.present {
                count += 1;
            }
        }
        count
    }

    /// Get IDT base address
    pub fn idt_base(&self) -> u64 {
        self.entries.as_ptr() as u64
    }

    /// Get IDT size in bytes
    pub fn idt_size(&self) -> u32 {
        self.valid_count * 16
    }

    /// Generate IDT report
    pub fn idt_report(&self) -> IdtReport {
        IdtReport {
            registered_handlers: self.registered_count(),
            valid_entries: self.valid_count,
            idt_base: self.idt_base(),
            idt_size: self.idt_size(),
        }
    }
}

/// IDT information report
#[derive(Debug, Clone, Copy)]
pub struct IdtReport {
    /// Number of registered handlers
    pub registered_handlers: u32,
    /// Total valid entries
    pub valid_entries: u32,
    /// IDT base address
    pub idt_base: u64,
    /// IDT total size in bytes
    pub idt_size: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gate_flags_encode() {
        let flags = GateFlags::new(true, PrivilegeLevel::Kernel);
        assert_eq!(flags.encode(), 0x80);

        let flags = GateFlags::new(true, PrivilegeLevel::User);
        assert_eq!(flags.encode(), 0xE0);
    }

    #[test]
    fn test_create_descriptor() {
        let desc = GateDescriptor::new(
            0x123456789ABCDEF0u64,
            0x08,
            GateType::InterruptGate,
            GateFlags::new(true, PrivilegeLevel::Kernel),
        );

        assert_eq!(desc.offset_lo, 0xDEF0);
        assert_eq!(desc.offset_mid, 0x9ABC);
        assert_eq!(desc.offset_hi, 0x12345678);
        assert_eq!(desc.handler_address(), 0x123456789ABCDEF0);
    }

    #[test]
    fn test_descriptor_ist_index() {
        let mut desc = GateDescriptor::new(
            0x1000,
            0x08,
            GateType::InterruptGate,
            GateFlags::new(true, PrivilegeLevel::Kernel),
        );
        desc.set_ist(5);
        assert_eq!(desc.ist, 5);

        desc.set_ist(10);
        assert_eq!(desc.ist, 2); // Masked to 3 bits
    }

    #[test]
    fn test_idt_manager_creation() {
        let manager = IdtManager::new(0x08);
        assert_eq!(manager.code_selector, 0x08);
        assert_eq!(manager.registered_count(), 0);
        assert_eq!(manager.valid_count, 0);
    }

    #[test]
    fn test_register_handler() {
        let mut manager = IdtManager::new(0x08);
        assert!(manager.register_interrupt(0, 0x1000));
        assert_eq!(manager.registered_count(), 1);
        assert_eq!(manager.valid_count, 1);
    }

    #[test]
    fn test_register_multiple_handlers() {
        let mut manager = IdtManager::new(0x08);
        for i in 0..10 {
            assert!(manager.register_interrupt(i, 0x1000 + (i as u64) * 0x100));
        }
        assert_eq!(manager.registered_count(), 10);
        assert_eq!(manager.valid_count, 10);
    }

    #[test]
    fn test_register_trap_handler() {
        let mut manager = IdtManager::new(0x08);
        assert!(manager.register_trap(1, 0x2000));
        
        let desc = manager.get_descriptor(1).unwrap();
        assert_eq!(desc.gate_type, GateType::TrapGate);
        assert_eq!(desc.flags.present, true);
    }

    #[test]
    fn test_register_user_trap() {
        let mut manager = IdtManager::new(0x08);
        assert!(manager.register_user_trap(0x80, 0x3000));

        let desc = manager.get_descriptor(0x80).unwrap();
        assert_eq!(desc.gate_type, GateType::TrapGate);
        assert_eq!(desc.flags.dpl, PrivilegeLevel::User);
    }

    #[test]
    fn test_get_descriptor() {
        let mut manager = IdtManager::new(0x08);
        manager.register_interrupt(5, 0x5000);

        let desc = manager.get_descriptor(5);
        assert!(desc.is_some());
        assert_eq!(desc.unwrap().handler_address(), 0x5000);

        let none_desc = manager.get_descriptor(10);
        assert!(none_desc.is_none());
    }

    #[test]
    fn test_invalid_vector() {
        let mut manager = IdtManager::new(0x08);
        assert!(!manager.register_interrupt(255, 0x1000)); // Out of bounds
    }

    #[test]
    fn test_idt_base_and_size() {
        let mut manager = IdtManager::new(0x08);
        manager.register_interrupt(0, 0x1000);
        
        assert!(manager.idt_base() > 0);
        assert_eq!(manager.idt_size(), 16); // 1 entry * 16 bytes
    }

    #[test]
    fn test_idt_register_update() {
        let mut manager = IdtManager::new(0x08);
        manager.register_interrupt(0, 0x1000);
        
        assert_eq!(unsafe { core::ptr::read_unaligned(&manager.idt_register.limit) }, 15); // (1 * 16) - 1
    }

    #[test]
    fn test_handler_address_reconstruction() {
        for addr in [0x1000u64, 0xFFFFFFFFFFFFFFFF, 0x123456789ABCDEF0] {
            let desc = GateDescriptor::new(
                addr,
                0x08,
                GateType::InterruptGate,
                GateFlags::new(true, PrivilegeLevel::Kernel),
            );
            assert_eq!(desc.handler_address(), addr);
        }
    }

    #[test]
    fn test_privilege_level_ordering() {
        assert!(PrivilegeLevel::Kernel < PrivilegeLevel::System);
        assert!(PrivilegeLevel::System < PrivilegeLevel::Data);
        assert!(PrivilegeLevel::Data < PrivilegeLevel::User);
    }

    #[test]
    fn test_gate_types() {
        assert_eq!(GateType::TaskGate as u8, 0x5);
        assert_eq!(GateType::InterruptGate as u8, 0xE);
        assert_eq!(GateType::TrapGate as u8, 0xF);
    }

    #[test]
    fn test_sequential_registration() {
        let mut manager = IdtManager::new(0x08);
        for i in 0..20 {
            assert!(manager.register_interrupt(i, 0x1000 + i as u64 * 0x10));
        }
        
        assert_eq!(manager.registered_count(), 20);
        
        for i in 0..20 {
            let desc = manager.get_descriptor(i).unwrap();
            assert_eq!(desc.handler_address(), 0x1000 + i as u64 * 0x10);
        }
    }

    #[test]
    fn test_idt_report() {
        let mut manager = IdtManager::new(0x08);
        manager.register_interrupt(0, 0x1000);
        manager.register_interrupt(10, 0x2000);

        let report = manager.idt_report();
        assert_eq!(report.registered_handlers, 2);
        assert_eq!(report.valid_entries, 11);
        assert!(report.idt_base > 0);
        assert_eq!(report.idt_size, 11 * 16);
    }

    #[test]
    fn test_different_gate_types() {
        let mut manager = IdtManager::new(0x08);
        manager.register_interrupt(0, 0x1000); // InterruptGate
        manager.register_trap(1, 0x2000);     // TrapGate

        let int_desc = manager.get_descriptor(0).unwrap();
        let trap_desc = manager.get_descriptor(1).unwrap();

        assert_eq!(int_desc.gate_type, GateType::InterruptGate);
        assert_eq!(trap_desc.gate_type, GateType::TrapGate);
    }

    #[test]
    fn test_descriptor_selector_preservation() {
        let mut manager = IdtManager::new(0x10);
        manager.register_interrupt(5, 0x5000);

        let desc = manager.get_descriptor(5).unwrap();
        assert_eq!(desc.selector, 0x10);
    }
}
