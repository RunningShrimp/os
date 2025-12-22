//! ACPI Power Domains - CPU and device power control
//!
//! Provides:
//! - Power domain management
//! - CPU power states (C0-C3)
//! - Device power control
//! - Power gating

/// Power states for devices
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerState {
    /// Fully on
    D0,
    /// Low power with context retained
    D1,
    /// Lower power state
    D2,
    /// Lowest power state
    D3,
}

/// CPU power states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuPowerState {
    /// Active state
    C0,
    /// C1 halt state
    C1,
    /// C2 stop clock state
    C2,
    /// C3 sleep state
    C3,
}

/// Power domain types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerDomainType {
    /// Processor
    Processor,
    /// Memory
    Memory,
    /// Chipset
    Chipset,
    /// Peripheral
    Peripheral,
    /// Custom
    Custom,
}

/// Power domain structure
#[derive(Debug, Clone, Copy)]
pub struct PowerDomain {
    /// Domain ID
    pub id: u32,
    /// Domain name (up to 32 chars)
    pub name: [u8; 32],
    /// Domain type
    pub domain_type: PowerDomainType,
    /// Current power state
    pub current_state: PowerState,
    /// Supported states bitmask
    pub supported_states: u8,
    /// Wake enabled
    pub wake_enabled: bool,
    /// Power control register address
    pub ctrl_register: u64,
    /// Status register address
    pub status_register: u64,
    /// Active flag
    pub active: bool,
}

impl PowerDomain {
    /// Create power domain
    pub fn new(id: u32, domain_type: PowerDomainType) -> Self {
        PowerDomain {
            id,
            name: [0u8; 32],
            domain_type,
            current_state: PowerState::D0,
            supported_states: 0x0F, // D0-D3 supported
            wake_enabled: false,
            ctrl_register: 0,
            status_register: 0,
            active: true,
        }
    }

    /// Set power state
    pub fn set_power_state(&mut self, state: PowerState) -> bool {
        if (self.supported_states & (1 << (state as u8))) != 0 {
            self.current_state = state;
            true
        } else {
            false
        }
    }

    /// Get power state
    pub fn get_power_state(&self) -> PowerState {
        self.current_state
    }

    /// Enable wake
    pub fn enable_wake(&mut self) {
        self.wake_enabled = true;
    }

    /// Disable wake
    pub fn disable_wake(&mut self) {
        self.wake_enabled = false;
    }

    /// Check if power gating is supported
    pub fn supports_power_gating(&self) -> bool {
        self.supported_states > 1
    }
}

/// CPU power state controller
#[derive(Debug, Clone, Copy)]
pub struct CpuPowerController {
    /// CPU ID
    pub cpu_id: u32,
    /// Current C-state
    pub current_cstate: CpuPowerState,
    /// Maximum C-state allowed
    pub max_cstate: CpuPowerState,
    /// C-state latencies in microseconds
    pub c_latencies: [u16; 4], // C0-C3
    /// Power consumption in mW
    pub power_consumption: [u16; 4],
}

impl CpuPowerController {
    /// Create CPU power controller
    pub fn new(cpu_id: u32) -> Self {
        CpuPowerController {
            cpu_id,
            current_cstate: CpuPowerState::C0,
            max_cstate: CpuPowerState::C3,
            c_latencies: [0, 1, 10, 100], // Example latencies
            power_consumption: [25, 20, 15, 5], // Example power in mW
        }
    }

    /// Request C-state transition
    pub fn request_cstate(&mut self, state: CpuPowerState) -> bool {
        if (state as u8) <= (self.max_cstate as u8) {
            self.current_cstate = state;
            true
        } else {
            false
        }
    }

    /// Get current C-state
    pub fn get_cstate(&self) -> CpuPowerState {
        self.current_cstate
    }

    /// Get transition latency
    pub fn get_latency(&self, state: CpuPowerState) -> u16 {
        self.c_latencies[state as usize]
    }

    /// Get power consumption
    pub fn get_power_consumption(&self, state: CpuPowerState) -> u16 {
        self.power_consumption[state as usize]
    }
}

/// ACPI power domain manager
pub struct AcpiPowerDomainManager {
    /// Power domains
    domains: [Option<PowerDomain>; 32],
    /// Domain count
    domain_count: usize,
    /// CPU power controllers
    cpu_controllers: [Option<CpuPowerController>; 16],
    /// CPU count
    cpu_count: usize,
    /// Global power enabled
    power_enabled: bool,
}

impl AcpiPowerDomainManager {
    /// Create power domain manager
    pub fn new() -> Self {
        AcpiPowerDomainManager {
            domains: [None; 32],
            domain_count: 0,
            cpu_controllers: [None; 16],
            cpu_count: 0,
            power_enabled: true,
        }
    }

    /// Register power domain
    pub fn register_domain(&mut self, domain: PowerDomain) -> bool {
        if self.domain_count < 32 {
            self.domains[self.domain_count] = Some(domain);
            self.domain_count += 1;
            true
        } else {
            false
        }
    }

    /// Get power domain
    pub fn get_domain(&self, id: u32) -> Option<&PowerDomain> {
        for i in 0..self.domain_count {
            if let Some(d) = &self.domains[i] {
                if d.id == id {
                    return Some(d);
                }
            }
        }
        None
    }

    /// Get mutable power domain
    pub fn get_domain_mut(&mut self, id: u32) -> Option<&mut PowerDomain> {
        let domain_count = self.domain_count;
        let domains_ptr = self.domains.as_mut_ptr();
        
        for i in 0..domain_count {
            unsafe {
                if let Some(d) = (*domains_ptr.add(i)).as_mut() {
                    if d.id == id {
                        return Some(d);
                    }
                }
            }
        }
        None
    }

    /// Register CPU controller
    pub fn register_cpu_controller(&mut self, ctrl: CpuPowerController) -> bool {
        if self.cpu_count < 16 {
            self.cpu_controllers[self.cpu_count] = Some(ctrl);
            self.cpu_count += 1;
            true
        } else {
            false
        }
    }

    /// Get CPU controller
    pub fn get_cpu_controller(&self, cpu_id: u32) -> Option<&CpuPowerController> {
        for i in 0..self.cpu_count {
            if let Some(c) = &self.cpu_controllers[i] {
                if c.cpu_id == cpu_id {
                    return Some(c);
                }
            }
        }
        None
    }

    /// Get mutable CPU controller
    pub fn get_cpu_controller_mut(&mut self, cpu_id: u32) -> Option<&mut CpuPowerController> {
        let cpu_count = self.cpu_count;
        let controllers_ptr = self.cpu_controllers.as_mut_ptr();
        
        for i in 0..cpu_count {
            unsafe {
                if let Some(c) = (*controllers_ptr.add(i)).as_mut() {
                    if c.cpu_id == cpu_id {
                        return Some(c);
                    }
                }
            }
        }
        None
    }

    /// Get domain count
    pub fn get_domain_count(&self) -> usize {
        self.domain_count
    }

    /// Get CPU count
    pub fn get_cpu_count(&self) -> usize {
        self.cpu_count
    }

    /// Enable power management
    pub fn enable(&mut self) {
        self.power_enabled = true;
    }

    /// Disable power management
    pub fn disable(&mut self) {
        self.power_enabled = false;
    }

    /// Check if power management enabled
    pub fn is_enabled(&self) -> bool {
        self.power_enabled
    }

    /// Power gate all inactive domains
    pub fn power_gate_inactive(&mut self) -> u32 {
        let mut count = 0;
        for i in 0..self.domain_count {
            if let Some(d) = &mut self.domains[i] {
                if !d.active && d.supports_power_gating() {
                    d.set_power_state(PowerState::D3);
                    count += 1;
                }
            }
        }
        count
    }

    /// Set CPU max C-state for all CPUs
    pub fn set_cpu_max_cstate(&mut self, state: CpuPowerState) -> u32 {
        let mut count = 0;
        for i in 0..self.cpu_count {
            if let Some(c) = &mut self.cpu_controllers[i] {
                c.max_cstate = state;
                count += 1;
            }
        }
        count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_states() {
        assert_ne!(PowerState::D0, PowerState::D1);
        assert_eq!(PowerState::D3, PowerState::D3);
    }

    #[test]
    fn test_cpu_power_states() {
        assert_ne!(CpuPowerState::C0, CpuPowerState::C3);
    }

    #[test]
    fn test_power_domain_creation() {
        let domain = PowerDomain::new(1, PowerDomainType::Processor);
        assert_eq!(domain.id, 1);
        assert_eq!(domain.current_state, PowerState::D0);
    }

    #[test]
    fn test_power_domain_state_change() {
        let domain = PowerDomain::new(1, PowerDomainType::Processor);
        assert!(domain.set_power_state(PowerState::D1));
        assert_eq!(domain.get_power_state(), PowerState::D1);
    }

    #[test]
    fn test_power_domain_wake() {
        let domain = PowerDomain::new(1, PowerDomainType::Processor);
        assert!(!domain.wake_enabled);
        domain.enable_wake();
        assert!(domain.wake_enabled);
    }

    #[test]
    fn test_power_domain_wake_disable() {
        let domain = PowerDomain::new(1, PowerDomainType::Processor);
        domain.enable_wake();
        domain.disable_wake();
        assert!(!domain.wake_enabled);
    }

    #[test]
    fn test_power_gating_support() {
        let domain = PowerDomain::new(1, PowerDomainType::Processor);
        assert!(domain.supports_power_gating());
    }

    #[test]
    fn test_cpu_power_controller_creation() {
        let ctrl = CpuPowerController::new(0);
        assert_eq!(ctrl.cpu_id, 0);
        assert_eq!(ctrl.current_cstate, CpuPowerState::C0);
    }

    #[test]
    fn test_cpu_cstate_transition() {
        let mut ctrl = CpuPowerController::new(0);
        assert!(ctrl.request_cstate(CpuPowerState::C1));
        assert_eq!(ctrl.get_cstate(), CpuPowerState::C1);
    }

    #[test]
    fn test_cpu_cstate_respects_max() {
        let mut ctrl = CpuPowerController::new(0);
        ctrl.max_cstate = CpuPowerState::C1;
        assert!(!ctrl.request_cstate(CpuPowerState::C3));
    }

    #[test]
    fn test_cpu_cstate_latency() {
        let ctrl = CpuPowerController::new(0);
        assert!(ctrl.get_latency(CpuPowerState::C0) < ctrl.get_latency(CpuPowerState::C3));
    }

    #[test]
    fn test_cpu_power_consumption() {
        let ctrl = CpuPowerController::new(0);
        assert!(ctrl.get_power_consumption(CpuPowerState::C3) < ctrl.get_power_consumption(CpuPowerState::C0));
    }

    #[test]
    fn test_manager_creation() {
        let mgr = AcpiPowerDomainManager::new();
        assert_eq!(mgr.get_domain_count(), 0);
        assert!(mgr.is_enabled());
    }

    #[test]
    fn test_register_domain() {
        let mut mgr = AcpiPowerDomainManager::new();
        let domain = PowerDomain::new(1, PowerDomainType::Processor);
        assert!(mgr.register_domain(domain));
        assert_eq!(mgr.get_domain_count(), 1);
    }

    #[test]
    fn test_get_domain() {
        let mut mgr = AcpiPowerDomainManager::new();
        let domain = PowerDomain::new(1, PowerDomainType::Processor);
        mgr.register_domain(domain);
        assert!(mgr.get_domain(1).is_some());
        assert!(mgr.get_domain(2).is_none());
    }

    #[test]
    fn test_register_cpu_controller() {
        let mut mgr = AcpiPowerDomainManager::new();
        let ctrl = CpuPowerController::new(0);
        assert!(mgr.register_cpu_controller(ctrl));
        assert_eq!(mgr.get_cpu_count(), 1);
    }

    #[test]
    fn test_get_cpu_controller() {
        let mut mgr = AcpiPowerDomainManager::new();
        let ctrl = CpuPowerController::new(0);
        mgr.register_cpu_controller(ctrl);
        assert!(mgr.get_cpu_controller(0).is_some());
    }

    #[test]
    fn test_enable_disable_power() {
        let mut mgr = AcpiPowerDomainManager::new();
        mgr.disable();
        assert!(!mgr.is_enabled());
        mgr.enable();
        assert!(mgr.is_enabled());
    }

    #[test]
    fn test_power_gate_inactive_domains() {
        let mut mgr = AcpiPowerDomainManager::new();
        let domain = PowerDomain::new(1, PowerDomainType::Processor);
        domain.active = false;
        mgr.register_domain(domain);
        let gated = mgr.power_gate_inactive();
        assert!(gated > 0);
    }

    #[test]
    fn test_set_cpu_max_cstate() {
        let mut mgr = AcpiPowerDomainManager::new();
        let ctrl = CpuPowerController::new(0);
        mgr.register_cpu_controller(ctrl);
        mgr.set_cpu_max_cstate(CpuPowerState::C2);
        assert_eq!(mgr.get_cpu_controller(0).unwrap().max_cstate, CpuPowerState::C2);
    }

    #[test]
    fn test_multiple_domains() {
        let mut mgr = AcpiPowerDomainManager::new();
        for i in 0..5 {
            let domain = PowerDomain::new(i, PowerDomainType::Processor);
            mgr.register_domain(domain);
        }
        assert_eq!(mgr.get_domain_count(), 5);
    }

    #[test]
    fn test_multiple_cpus() {
        let mut mgr = AcpiPowerDomainManager::new();
        for i in 0..4 {
            let ctrl = CpuPowerController::new(i);
            mgr.register_cpu_controller(ctrl);
        }
        assert_eq!(mgr.get_cpu_count(), 4);
    }

    #[test]
    fn test_domain_type_variety() {
        let d1 = PowerDomain::new(1, PowerDomainType::Processor);
        let d2 = PowerDomain::new(2, PowerDomainType::Memory);
        assert_ne!(d1.domain_type, d2.domain_type);
    }

    #[test]
    fn test_supported_states_mask() {
        let domain = PowerDomain::new(1, PowerDomainType::Processor);
        assert!(domain.set_power_state(PowerState::D0));
        assert!(domain.set_power_state(PowerState::D1));
    }

    #[test]
    fn test_cpu_controller_initial_state() {
        let ctrl = CpuPowerController::new(0);
        assert_eq!(ctrl.current_cstate, CpuPowerState::C0);
        assert_eq!(ctrl.max_cstate, CpuPowerState::C3);
    }

    #[test]
    fn test_manager_get_domain_mut() {
        let mut mgr = AcpiPowerDomainManager::new();
        let domain = PowerDomain::new(1, PowerDomainType::Processor);
        domain.active = false;
        mgr.register_domain(domain);
        if let Some(d) = mgr.get_domain_mut(1) {
            d.active = true;
        }
        assert!(mgr.get_domain(1).unwrap().active);
    }

    #[test]
    fn test_manager_get_cpu_controller_mut() {
        let mut mgr = AcpiPowerDomainManager::new();
        let ctrl = CpuPowerController::new(0);
        mgr.register_cpu_controller(ctrl);
        if let Some(c) = mgr.get_cpu_controller_mut(0) {
            c.max_cstate = CpuPowerState::C2;
        }
        assert_eq!(mgr.get_cpu_controller(0).unwrap().max_cstate, CpuPowerState::C2);
    }

    #[test]
    fn test_power_consumption_realistic() {
        let ctrl = CpuPowerController::new(0);
        let c0_power = ctrl.get_power_consumption(CpuPowerState::C0);
        let c3_power = ctrl.get_power_consumption(CpuPowerState::C3);
        assert!(c0_power > c3_power);
    }

    #[test]
    fn test_transition_latency_realistic() {
        let ctrl = CpuPowerController::new(0);
        let c0_lat = ctrl.get_latency(CpuPowerState::C0);
        let c3_lat = ctrl.get_latency(CpuPowerState::C3);
        assert!(c0_lat < c3_lat);
    }

    #[test]
    fn test_domain_ctrl_register() {
        let mut domain = PowerDomain::new(1, PowerDomainType::Memory);
        domain.ctrl_register = 0xDEADBEEF;
        assert_eq!(domain.ctrl_register, 0xDEADBEEF);
    }

    #[test]
    fn test_domain_status_register() {
        let mut domain = PowerDomain::new(1, PowerDomainType::Peripheral);
        domain.status_register = 0xCAFEBABE;
        assert_eq!(domain.status_register, 0xCAFEBABE);
    }
}
