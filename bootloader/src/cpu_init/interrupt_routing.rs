//! Interrupt Routing - PIC and APIC interrupt mapping and configuration
//!
//! Provides:
//! - 8259A PIC (Programmable Interrupt Controller) configuration
//! - APIC (Advanced PIC) setup and routing
//! - Interrupt-to-vector mapping
//! - Interrupt masking and priority management

/// PIC (8259A) base addresses
pub const PIC_MASTER_BASE: u16 = 0x20;
pub const PIC_SLAVE_BASE: u16 = 0xA0;

/// PIC command and data port offsets
pub const PIC_CMD_PORT: u16 = 0;
pub const PIC_DATA_PORT: u16 = 1;

/// PIC modes and configurations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PicMode {
    /// Single 8259A controller
    Single,
    /// Cascaded dual controllers (master + slave)
    Cascaded,
}

/// Interrupt priority levels (0-15 for 8259A)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum InterruptPriority {
    Level0 = 0,
    Level1 = 1,
    Level2 = 2,
    Level3 = 3,
    Level4 = 4,
    Level5 = 5,
    Level6 = 6,
    Level7 = 7,
    Level8 = 8,
    Level9 = 9,
    Level10 = 10,
    Level11 = 11,
    Level12 = 12,
    Level13 = 13,
    Level14 = 14,
    Level15 = 15,
}

/// APIC (Advanced PIC) configuration
#[derive(Debug, Clone, Copy)]
pub struct ApicConfig {
    /// APIC base address (typically 0xFEE00000)
    pub base_address: u64,
    /// APIC ID
    pub apic_id: u8,
    /// Is this the bootstrap APIC
    pub is_bsp: bool,
    /// APIC enabled
    pub enabled: bool,
}

impl ApicConfig {
    /// Create APIC configuration
    pub fn new(base_address: u64, apic_id: u8, is_bsp: bool) -> Self {
        ApicConfig {
            base_address,
            apic_id,
            is_bsp,
            enabled: false,
        }
    }
}

/// Interrupt source information
#[derive(Debug, Clone, Copy)]
pub struct InterruptSource {
    /// Interrupt request line (0-15)
    pub irq: u8,
    /// Target vector number (32-255)
    pub vector: u8,
    /// Priority level
    pub priority: InterruptPriority,
    /// Is masked (disabled)
    pub masked: bool,
}

impl InterruptSource {
    /// Create interrupt source
    pub fn new(irq: u8, vector: u8, priority: InterruptPriority) -> Self {
        InterruptSource {
            irq,
            vector,
            priority,
            masked: false,
        }
    }
}

/// 8259A PIC controller
pub struct Pic8259a {
    /// Controller mode
    mode: PicMode,
    /// Master PIC base address
    master_base: u16,
    /// Slave PIC base address
    slave_base: u16,
    /// Interrupt mask register (IMR)
    interrupt_mask: u16,
    /// Interrupt request register (IRR)
    #[allow(dead_code)]
    interrupt_request: u16,
    /// In-service register (ISR)
    #[allow(dead_code)]
    in_service: u16,
    /// Number of configured IRQs
    irq_count: u8,
}

impl Pic8259a {
    /// Create PIC manager
    pub fn new(mode: PicMode) -> Self {
        let (master_base, slave_base) = match mode {
            PicMode::Single => (PIC_MASTER_BASE, 0),
            PicMode::Cascaded => (PIC_MASTER_BASE, PIC_SLAVE_BASE),
        };

        Pic8259a {
            mode,
            master_base,
            slave_base,
            interrupt_mask: 0xFFFF,
            interrupt_request: 0,
            in_service: 0,
            irq_count: 0,
        }
    }

    /// Initialize PIC with ICW (Initialization Command Word)
    pub fn initialize(&mut self, vector_offset: u8) -> bool {
        // ICW1: Initialize sequence
        self.write_port(self.master_base + PIC_CMD_PORT, 0x11);
        
        // ICW2: Vector offset for master
        self.write_port(self.master_base + PIC_DATA_PORT, vector_offset);

        if self.mode == PicMode::Cascaded {
            // ICW1: Initialize slave
            self.write_port(self.slave_base + PIC_CMD_PORT, 0x11);
            
            // ICW2: Vector offset for slave
            self.write_port(self.slave_base + PIC_DATA_PORT, vector_offset + 8);

            // ICW3: Master - slave on IRQ2
            self.write_port(self.master_base + PIC_DATA_PORT, 0x04);
            
            // ICW3: Slave - connected to master IRQ2
            self.write_port(self.slave_base + PIC_DATA_PORT, 0x02);
        }

        // ICW4: 8086 mode
        self.write_port(self.master_base + PIC_DATA_PORT, 0x01);
        
        if self.mode == PicMode::Cascaded {
            self.write_port(self.slave_base + PIC_DATA_PORT, 0x01);
        }

        self.irq_count = if self.mode == PicMode::Cascaded { 16 } else { 8 };
        true
    }

    /// Mask (disable) interrupt
    pub fn mask_irq(&mut self, irq: u8) -> bool {
        if irq >= self.irq_count {
            return false;
        }

        self.interrupt_mask |= 1u16 << irq;
        self.update_mask_register();
        true
    }

    /// Unmask (enable) interrupt
    pub fn unmask_irq(&mut self, irq: u8) -> bool {
        if irq >= self.irq_count {
            return false;
        }

        self.interrupt_mask &= !(1u16 << irq);
        self.update_mask_register();
        true
    }

    /// Get interrupt mask
    pub fn get_mask(&self) -> u16 {
        self.interrupt_mask
    }

    /// Send End-Of-Interrupt (EOI) signal
    pub fn send_eoi(&mut self, irq: u8) -> bool {
        if irq >= self.irq_count {
            return false;
        }

        let eoi_command = 0x20u8;
        
        if irq < 8 {
            self.write_port(self.master_base + PIC_CMD_PORT, eoi_command);
        } else {
            self.write_port(self.slave_base + PIC_CMD_PORT, eoi_command);
            self.write_port(self.master_base + PIC_CMD_PORT, eoi_command);
        }

        true
    }

    /// Update mask register (OCW1)
    fn update_mask_register(&mut self) {
        self.write_port(self.master_base + PIC_DATA_PORT, (self.interrupt_mask & 0xFF) as u8);
        
        if self.mode == PicMode::Cascaded {
            self.write_port(self.slave_base + PIC_DATA_PORT, ((self.interrupt_mask >> 8) & 0xFF) as u8);
        }
    }

    /// Write to PIC port (simulated)
    fn write_port(&self, _port: u16, _value: u8) {
        // In real implementation, would use x86 I/O instructions
    }

    /// Read from PIC port (simulated)
    #[allow(dead_code)]
    fn read_port(&self, _port: u16) -> u8 {
        // In real implementation, would use x86 I/O instructions
        0
    }

    /// Get number of IRQs
    pub fn irq_count(&self) -> u8 {
        self.irq_count
    }

    /// Is IRQ masked
    pub fn is_masked(&self, irq: u8) -> bool {
        if irq >= self.irq_count {
            return true;
        }
        (self.interrupt_mask & (1u16 << irq)) != 0
    }
}

/// Interrupt routing table and manager
pub struct InterruptRouter {
    /// IRQ to vector mapping (16 entries for 8259A)
    routing_table: [Option<InterruptSource>; 16],
    /// APIC configuration
    apic_config: Option<ApicConfig>,
    /// PIC controller
    pic: Pic8259a,
    /// Total interrupts routed
    total_routed: u32,
    /// Current vector offset for PIC
    vector_offset: u8,
}

impl InterruptRouter {
    /// Create interrupt router
    pub fn new() -> Self {
        InterruptRouter {
            routing_table: [None; 16],
            apic_config: None,
            pic: Pic8259a::new(PicMode::Cascaded),
            total_routed: 0,
            vector_offset: 32,
        }
    }

    /// Initialize routing with PIC
    pub fn initialize_pic(&mut self, vector_offset: u8) -> bool {
        self.vector_offset = vector_offset;
        self.pic.initialize(vector_offset)
    }

    /// Register APIC configuration
    pub fn register_apic(&mut self, config: ApicConfig) -> bool {
        self.apic_config = Some(config);
        true
    }

    /// Route IRQ to vector
    pub fn route_irq(
        &mut self,
        irq: u8,
        vector: u8,
        priority: InterruptPriority,
    ) -> bool {
        if irq >= 16 {
            return false;
        }

        let source = InterruptSource::new(irq, vector, priority);
        self.routing_table[irq as usize] = Some(source);
        self.total_routed += 1;
        true
    }

    /// Mask interrupt at PIC
    pub fn mask_interrupt(&mut self, irq: u8) -> bool {
        if irq >= 16 {
            return false;
        }

        if let Some(source) = self.routing_table[irq as usize].as_mut() {
            source.masked = true;
        }

        self.pic.mask_irq(irq)
    }

    /// Unmask interrupt at PIC
    pub fn unmask_interrupt(&mut self, irq: u8) -> bool {
        if irq >= 16 {
            return false;
        }

        if let Some(source) = self.routing_table[irq as usize].as_mut() {
            source.masked = false;
        }

        self.pic.unmask_irq(irq)
    }

    /// Get interrupt source info
    pub fn get_routing(&self, irq: u8) -> Option<InterruptSource> {
        if irq < 16 {
            self.routing_table[irq as usize]
        } else {
            None
        }
    }

    /// Send EOI for interrupt
    pub fn acknowledge_irq(&mut self, irq: u8) -> bool {
        self.pic.send_eoi(irq)
    }

    /// Get total routed interrupts
    pub fn total_routed(&self) -> u32 {
        self.total_routed
    }

    /// Get enabled interrupt count
    pub fn enabled_count(&self) -> u32 {
        self.routing_table
            .iter()
            .filter(|r| r.is_some() && !r.unwrap().masked)
            .count() as u32
    }

    /// Get routing statistics
    pub fn routing_report(&self) -> RoutingReport {
        RoutingReport {
            total_routed: self.total_routed,
            enabled_interrupts: self.enabled_count(),
            pic_mask: self.pic.get_mask(),
            has_apic: self.apic_config.is_some(),
        }
    }
}

/// Interrupt routing statistics
#[derive(Debug, Clone, Copy)]
pub struct RoutingReport {
    /// Total interrupts routed
    pub total_routed: u32,
    /// Currently enabled interrupts
    pub enabled_interrupts: u32,
    /// PIC interrupt mask
    pub pic_mask: u16,
    /// APIC is configured
    pub has_apic: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pic_creation() {
        let pic = Pic8259a::new(PicMode::Cascaded);
        assert_eq!(pic.mode, PicMode::Cascaded);
        assert_eq!(pic.master_base, PIC_MASTER_BASE);
        assert_eq!(pic.slave_base, PIC_SLAVE_BASE);
    }

    #[test]
    fn test_pic_single_mode() {
        let pic = Pic8259a::new(PicMode::Single);
        assert_eq!(pic.mode, PicMode::Single);
        assert_eq!(pic.slave_base, 0);
    }

    #[test]
    fn test_pic_initialization() {
        let mut pic = Pic8259a::new(PicMode::Cascaded);
        assert!(pic.initialize(32));
        assert_eq!(pic.irq_count(), 16);
    }

    #[test]
    fn test_mask_irq() {
        let mut pic = Pic8259a::new(PicMode::Cascaded);
        pic.initialize(32);
        
        assert!(pic.mask_irq(0));
        assert!(pic.is_masked(0));
    }

    #[test]
    fn test_unmask_irq() {
        let mut pic = Pic8259a::new(PicMode::Cascaded);
        pic.initialize(32);
        
        pic.mask_irq(0);
        assert!(pic.unmask_irq(0));
        assert!(!pic.is_masked(0));
    }

    #[test]
    fn test_irq_out_of_range() {
        let mut pic = Pic8259a::new(PicMode::Cascaded);
        pic.initialize(32);
        
        assert!(!pic.mask_irq(20));
        assert!(!pic.unmask_irq(20));
    }

    #[test]
    fn test_interrupt_priority_ordering() {
        assert!(InterruptPriority::Level0 < InterruptPriority::Level1);
        assert!(InterruptPriority::Level7 < InterruptPriority::Level8);
        assert!(InterruptPriority::Level15 > InterruptPriority::Level0);
    }

    #[test]
    fn test_interrupt_source_creation() {
        let source = InterruptSource::new(3, 35, InterruptPriority::Level3);
        assert_eq!(source.irq, 3);
        assert_eq!(source.vector, 35);
        assert!(!source.masked);
    }

    #[test]
    fn test_apic_config_creation() {
        let config = ApicConfig::new(0xFEE00000, 0, true);
        assert_eq!(config.base_address, 0xFEE00000);
        assert_eq!(config.apic_id, 0);
        assert!(config.is_bsp);
        assert!(!config.enabled);
    }

    #[test]
    fn test_interrupt_router_creation() {
        let router = InterruptRouter::new();
        assert_eq!(router.total_routed(), 0);
        assert_eq!(router.enabled_count(), 0);
    }

    #[test]
    fn test_router_initialize_pic() {
        let mut router = InterruptRouter::new();
        assert!(router.initialize_pic(32));
        assert_eq!(router.vector_offset, 32);
    }

    #[test]
    fn test_router_register_apic() {
        let mut router = InterruptRouter::new();
        let config = ApicConfig::new(0xFEE00000, 0, true);
        assert!(router.register_apic(config));
        assert!(router.apic_config.is_some());
    }

    #[test]
    fn test_route_irq() {
        let mut router = InterruptRouter::new();
        router.initialize_pic(32);
        
        assert!(router.route_irq(0, 32, InterruptPriority::Level0));
        assert_eq!(router.total_routed(), 1);
    }

    #[test]
    fn test_route_multiple_irqs() {
        let mut router = InterruptRouter::new();
        router.initialize_pic(32);
        
        for i in 0..8 {
            assert!(router.route_irq(i, 32 + i, InterruptPriority::Level0));
        }
        
        assert_eq!(router.total_routed(), 8);
    }

    #[test]
    fn test_mask_interrupt() {
        let mut router = InterruptRouter::new();
        router.initialize_pic(32);
        router.route_irq(0, 32, InterruptPriority::Level0);
        
        assert!(router.mask_interrupt(0));
        
        let routing = router.get_routing(0).unwrap();
        assert!(routing.masked);
    }

    #[test]
    fn test_unmask_interrupt() {
        let mut router = InterruptRouter::new();
        router.initialize_pic(32);
        router.route_irq(0, 32, InterruptPriority::Level0);
        router.mask_interrupt(0);
        
        assert!(router.unmask_interrupt(0));
        
        let routing = router.get_routing(0).unwrap();
        assert!(!routing.masked);
    }

    #[test]
    fn test_get_routing() {
        let mut router = InterruptRouter::new();
        router.initialize_pic(32);
        router.route_irq(5, 37, InterruptPriority::Level5);
        
        let routing = router.get_routing(5).unwrap();
        assert_eq!(routing.irq, 5);
        assert_eq!(routing.vector, 37);
        assert_eq!(routing.priority, InterruptPriority::Level5);
    }

    #[test]
    fn test_acknowledge_irq() {
        let mut router = InterruptRouter::new();
        router.initialize_pic(32);
        
        assert!(router.acknowledge_irq(0));
        assert!(router.acknowledge_irq(10));
    }

    #[test]
    fn test_enabled_count() {
        let mut router = InterruptRouter::new();
        router.initialize_pic(32);
        
        router.route_irq(0, 32, InterruptPriority::Level0);
        router.route_irq(1, 33, InterruptPriority::Level1);
        
        assert_eq!(router.enabled_count(), 2);
        
        router.mask_interrupt(0);
        assert_eq!(router.enabled_count(), 1);
    }

    #[test]
    fn test_routing_report() {
        let mut router = InterruptRouter::new();
        router.initialize_pic(32);
        
        router.route_irq(0, 32, InterruptPriority::Level0);
        router.route_irq(1, 33, InterruptPriority::Level1);
        router.mask_interrupt(1);
        
        let report = router.routing_report();
        assert_eq!(report.total_routed, 2);
        assert_eq!(report.enabled_interrupts, 1);
        assert!(!report.has_apic);
    }

    #[test]
    fn test_out_of_range_route() {
        let mut router = InterruptRouter::new();
        assert!(!router.route_irq(20, 50, InterruptPriority::Level0));
    }

    #[test]
    fn test_pic_mask_register() {
        let mut pic = Pic8259a::new(PicMode::Cascaded);
        pic.initialize(32);
        
        pic.mask_irq(0);
        pic.mask_irq(5);
        
        let mask = pic.get_mask();
        assert_eq!(mask & 0x01, 0x01);
        assert_eq!(mask & 0x20, 0x20);
    }

    #[test]
    fn test_cascaded_vs_single_mode() {
        let cascaded = Pic8259a::new(PicMode::Cascaded);
        let single = Pic8259a::new(PicMode::Single);
        
        assert_eq!(cascaded.irq_count, 0); // Not initialized yet
        assert_eq!(single.irq_count, 0);
    }

    #[test]
    fn test_multiple_masking_operations() {
        let mut router = InterruptRouter::new();
        router.initialize_pic(32);
        
        for i in 0..8 {
            router.route_irq(i, 32 + i, InterruptPriority::Level0);
        }
        
        for i in 0..4 {
            router.mask_interrupt(i);
        }
        
        assert_eq!(router.enabled_count(), 4);
    }
}
