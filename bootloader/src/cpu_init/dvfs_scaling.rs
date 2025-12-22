//! DVFS Scaling - Dynamic Voltage and Frequency Scaling
//!
//! Provides:
//! - Dynamic frequency scaling during boot
//! - Voltage adjustment
//! - Performance/power balance
//! - Frequency domain management

/// CPU frequency scaling modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingMode {
    /// Performance mode (maximum frequency)
    Performance,
    /// Balanced mode
    Balanced,
    /// Power saving mode (reduced frequency)
    PowerSave,
    /// Custom frequency
    Custom,
}

/// Frequency operating point (P-state)
#[derive(Debug, Clone, Copy)]
pub struct Pstate {
    /// Frequency in MHz
    pub frequency_mhz: u32,
    /// Voltage in mV
    pub voltage_mv: u16,
    /// Power in mW
    pub power_mw: u16,
    /// Transition latency in microseconds
    pub transition_latency_us: u16,
}

impl Pstate {
    /// Create P-state
    pub fn new(freq: u32, volt: u16, power: u16) -> Self {
        Pstate {
            frequency_mhz: freq,
            voltage_mv: volt,
            power_mw: power,
            transition_latency_us: 100,
        }
    }
}

/// Frequency scaling governor
#[derive(Debug, Clone, Copy)]
pub struct FrequencyGovernor {
    /// Governor ID
    pub id: u32,
    /// Current frequency MHz
    pub current_frequency: u32,
    /// Minimum frequency MHz
    pub min_frequency: u32,
    /// Maximum frequency MHz
    pub max_frequency: u32,
    /// Scaling mode
    pub mode: ScalingMode,
    /// Turbo enabled
    pub turbo_enabled: bool,
    /// Frequency transitions count
    pub transitions: u32,
}

impl FrequencyGovernor {
    /// Create frequency governor
    pub fn new(id: u32, max_freq: u32) -> Self {
        FrequencyGovernor {
            id,
            current_frequency: max_freq,
            min_frequency: max_freq / 4, // Min is 1/4 of max
            max_frequency: max_freq,
            mode: ScalingMode::Balanced,
            turbo_enabled: false,
            transitions: 0,
        }
    }

    /// Set scaling frequency
    pub fn set_frequency(&mut self, freq: u32) -> bool {
        if freq >= self.min_frequency && freq <= self.max_frequency {
            self.current_frequency = freq;
            self.transitions += 1;
            true
        } else {
            false
        }
    }

    /// Get current frequency
    pub fn get_frequency(&self) -> u32 {
        self.current_frequency
    }

    /// Set scaling mode
    pub fn set_mode(&mut self, mode: ScalingMode) {
        self.mode = mode;
        match mode {
            ScalingMode::Performance => self.current_frequency = self.max_frequency,
            ScalingMode::PowerSave => self.current_frequency = self.min_frequency,
            ScalingMode::Balanced => self.current_frequency = (self.min_frequency + self.max_frequency) / 2,
            ScalingMode::Custom => {},
        }
    }

    /// Get scaling mode
    pub fn get_mode(&self) -> ScalingMode {
        self.mode
    }

    /// Enable turbo
    pub fn enable_turbo(&mut self) {
        self.turbo_enabled = true;
    }

    /// Disable turbo
    pub fn disable_turbo(&mut self) {
        self.turbo_enabled = false;
    }

    /// Get transition count
    pub fn get_transitions(&self) -> u32 {
        self.transitions
    }
}

/// Voltage scaling controller
#[derive(Debug, Clone, Copy)]
pub struct VoltageController {
    /// Controller ID
    pub id: u32,
    /// Current voltage in mV
    pub current_voltage: u16,
    /// Minimum voltage in mV
    pub min_voltage: u16,
    /// Maximum voltage in mV
    pub max_voltage: u16,
    /// Voltage step in mV
    pub voltage_step: u16,
}

impl VoltageController {
    /// Create voltage controller
    pub fn new(id: u32, max_volt: u16) -> Self {
        VoltageController {
            id,
            current_voltage: max_volt,
            min_voltage: max_volt / 2,
            max_voltage: max_volt,
            voltage_step: 50,
        }
    }

    /// Set voltage
    pub fn set_voltage(&mut self, volt: u16) -> bool {
        if volt >= self.min_voltage && volt <= self.max_voltage {
            self.current_voltage = volt;
            true
        } else {
            false
        }
    }

    /// Get current voltage
    pub fn get_voltage(&self) -> u16 {
        self.current_voltage
    }

    /// Increase voltage by one step
    pub fn increase_voltage(&mut self) -> bool {
        if self.current_voltage + self.voltage_step <= self.max_voltage {
            self.current_voltage += self.voltage_step;
            true
        } else {
            false
        }
    }

    /// Decrease voltage by one step
    pub fn decrease_voltage(&mut self) -> bool {
        if self.current_voltage >= self.min_voltage + self.voltage_step {
            self.current_voltage -= self.voltage_step;
            true
        } else {
            false
        }
    }
}

/// DVFS scaling manager
pub struct DvfsScalingManager {
    /// Frequency governors
    governors: [Option<FrequencyGovernor>; 16],
    /// Governor count
    governor_count: usize,
    /// Voltage controllers
    voltage_controllers: [Option<VoltageController>; 16],
    /// Controller count
    controller_count: usize,
    /// P-states database
    pstates: [Option<Pstate>; 32],
    /// P-state count
    pstate_count: usize,
    /// DVFS enabled
    enabled: bool,
}

impl DvfsScalingManager {
    /// Create DVFS scaling manager
    pub fn new() -> Self {
        DvfsScalingManager {
            governors: [None; 16],
            governor_count: 0,
            voltage_controllers: [None; 16],
            controller_count: 0,
            pstates: [None; 32],
            pstate_count: 0,
            enabled: true,
        }
    }

    /// Register frequency governor
    pub fn register_governor(&mut self, gov: FrequencyGovernor) -> bool {
        if self.governor_count < 16 {
            self.governors[self.governor_count] = Some(gov);
            self.governor_count += 1;
            true
        } else {
            false
        }
    }

    /// Get frequency governor
    pub fn get_governor(&self, id: u32) -> Option<&FrequencyGovernor> {
        for i in 0..self.governor_count {
            if let Some(g) = &self.governors[i] {
                if g.id == id {
                    return Some(g);
                }
            }
        }
        None
    }

    /// Get mutable frequency governor
    pub fn get_governor_mut(&mut self, id: u32) -> Option<&mut FrequencyGovernor> {
        let governor_count = self.governor_count;
        let governors_ptr = self.governors.as_mut_ptr();
        
        for i in 0..governor_count {
            unsafe {
                if let Some(g) = (*governors_ptr.add(i)).as_mut() {
                    if g.id == id {
                        return Some(g);
                    }
                }
            }
        }
        None
    }

    /// Register voltage controller
    pub fn register_voltage_controller(&mut self, ctrl: VoltageController) -> bool {
        if self.controller_count < 16 {
            self.voltage_controllers[self.controller_count] = Some(ctrl);
            self.controller_count += 1;
            true
        } else {
            false
        }
    }

    /// Get voltage controller
    pub fn get_voltage_controller(&self, id: u32) -> Option<&VoltageController> {
        for i in 0..self.controller_count {
            if let Some(c) = &self.voltage_controllers[i] {
                if c.id == id {
                    return Some(c);
                }
            }
        }
        None
    }

    /// Get mutable voltage controller
    pub fn get_voltage_controller_mut(&mut self, id: u32) -> Option<&mut VoltageController> {
        let controller_count = self.controller_count;
        let controllers_ptr = self.voltage_controllers.as_mut_ptr();
        
        for i in 0..controller_count {
            unsafe {
                if let Some(c) = (*controllers_ptr.add(i)).as_mut() {
                    if c.id == id {
                        return Some(c);
                    }
                }
            }
        }
        None
    }

    /// Add P-state
    pub fn add_pstate(&mut self, pstate: Pstate) -> bool {
        if self.pstate_count < 32 {
            self.pstates[self.pstate_count] = Some(pstate);
            self.pstate_count += 1;
            true
        } else {
            false
        }
    }

    /// Get P-state count
    pub fn get_pstate_count(&self) -> usize {
        self.pstate_count
    }

    /// Get P-state by index
    pub fn get_pstate(&self, index: usize) -> Option<&Pstate> {
        if index < self.pstate_count {
            self.pstates[index].as_ref()
        } else {
            None
        }
    }

    /// Scale all CPUs to mode
    pub fn scale_all_to_mode(&mut self, mode: ScalingMode) -> u32 {
        let mut count = 0;
        for i in 0..self.governor_count {
            if let Some(g) = &mut self.governors[i] {
                g.set_mode(mode);
                count += 1;
            }
        }
        count
    }

    /// Scale all CPUs to frequency
    pub fn scale_all_to_frequency(&mut self, freq: u32) -> u32 {
        let mut count = 0;
        for i in 0..self.governor_count {
            if let Some(g) = &mut self.governors[i] {
                if g.set_frequency(freq) {
                    count += 1;
                }
            }
        }
        count
    }

    /// Get total transitions
    pub fn get_total_transitions(&self) -> u32 {
        let mut total = 0;
        for i in 0..self.governor_count {
            if let Some(g) = &self.governors[i] {
                total += g.get_transitions();
            }
        }
        total
    }

    /// Enable DVFS
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable DVFS
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if DVFS enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Get governor count
    pub fn get_governor_count(&self) -> usize {
        self.governor_count
    }

    /// Get controller count
    pub fn get_controller_count(&self) -> usize {
        self.controller_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaling_modes() {
        assert_ne!(ScalingMode::Performance, ScalingMode::PowerSave);
    }

    #[test]
    fn test_pstate_creation() {
        let pstate = Pstate::new(2400, 1200, 50);
        assert_eq!(pstate.frequency_mhz, 2400);
    }

    #[test]
    fn test_governor_creation() {
        let gov = FrequencyGovernor::new(0, 2400);
        assert_eq!(gov.id, 0);
        assert_eq!(gov.max_frequency, 2400);
    }

    #[test]
    fn test_governor_set_frequency() {
        let mut gov = FrequencyGovernor::new(0, 2400);
        assert!(gov.set_frequency(1800));
        assert_eq!(gov.get_frequency(), 1800);
    }

    #[test]
    fn test_governor_set_mode_performance() {
        let mut gov = FrequencyGovernor::new(0, 2400);
        gov.set_mode(ScalingMode::Performance);
        assert_eq!(gov.get_frequency(), 2400);
    }

    #[test]
    fn test_governor_set_mode_powersave() {
        let mut gov = FrequencyGovernor::new(0, 2400);
        gov.set_mode(ScalingMode::PowerSave);
        assert!(gov.get_frequency() < gov.max_frequency);
    }

    #[test]
    fn test_governor_turbo() {
        let mut gov = FrequencyGovernor::new(0, 2400);
        assert!(!gov.turbo_enabled);
        gov.enable_turbo();
        assert!(gov.turbo_enabled);
    }

    #[test]
    fn test_governor_transitions() {
        let mut gov = FrequencyGovernor::new(0, 2400);
        assert_eq!(gov.get_transitions(), 0);
        gov.set_frequency(1800);
        assert_eq!(gov.get_transitions(), 1);
    }

    #[test]
    fn test_voltage_controller_creation() {
        let ctrl = VoltageController::new(0, 1200);
        assert_eq!(ctrl.current_voltage, 1200);
    }

    #[test]
    fn test_voltage_controller_set() {
        let mut ctrl = VoltageController::new(0, 1200);
        assert!(ctrl.set_voltage(1100));
        assert_eq!(ctrl.get_voltage(), 1100);
    }

    #[test]
    fn test_voltage_increase() {
        let mut ctrl = VoltageController::new(0, 1200);
        let initial = ctrl.get_voltage();
        assert!(ctrl.increase_voltage());
        assert!(ctrl.get_voltage() > initial);
    }

    #[test]
    fn test_voltage_decrease() {
        let mut ctrl = VoltageController::new(0, 1200);
        assert!(ctrl.decrease_voltage());
        assert!(ctrl.get_voltage() < 1200);
    }

    #[test]
    fn test_dvfs_manager_creation() {
        let mgr = DvfsScalingManager::new();
        assert_eq!(mgr.get_governor_count(), 0);
        assert!(mgr.is_enabled());
    }

    #[test]
    fn test_register_governor() {
        let mut mgr = DvfsScalingManager::new();
        let gov = FrequencyGovernor::new(0, 2400);
        assert!(mgr.register_governor(gov));
        assert_eq!(mgr.get_governor_count(), 1);
    }

    #[test]
    fn test_get_governor() {
        let mut mgr = DvfsScalingManager::new();
        let gov = FrequencyGovernor::new(0, 2400);
        mgr.register_governor(gov);
        assert!(mgr.get_governor(0).is_some());
    }

    #[test]
    fn test_register_voltage_controller() {
        let mut mgr = DvfsScalingManager::new();
        let ctrl = VoltageController::new(0, 1200);
        assert!(mgr.register_voltage_controller(ctrl));
    }

    #[test]
    fn test_get_voltage_controller() {
        let mut mgr = DvfsScalingManager::new();
        let ctrl = VoltageController::new(0, 1200);
        mgr.register_voltage_controller(ctrl);
        assert!(mgr.get_voltage_controller(0).is_some());
    }

    #[test]
    fn test_add_pstate() {
        let mut mgr = DvfsScalingManager::new();
        let pstate = Pstate::new(2400, 1200, 50);
        assert!(mgr.add_pstate(pstate));
        assert_eq!(mgr.get_pstate_count(), 1);
    }

    #[test]
    fn test_get_pstate() {
        let mut mgr = DvfsScalingManager::new();
        let pstate = Pstate::new(2400, 1200, 50);
        mgr.add_pstate(pstate);
        assert!(mgr.get_pstate(0).is_some());
    }

    #[test]
    fn test_scale_all_to_mode() {
        let mut mgr = DvfsScalingManager::new();
        mgr.register_governor(FrequencyGovernor::new(0, 2400));
        mgr.register_governor(FrequencyGovernor::new(1, 2400));
        let count = mgr.scale_all_to_mode(ScalingMode::Performance);
        assert_eq!(count, 2);
    }

    #[test]
    fn test_scale_all_to_frequency() {
        let mut mgr = DvfsScalingManager::new();
        mgr.register_governor(FrequencyGovernor::new(0, 2400));
        let count = mgr.scale_all_to_frequency(1800);
        assert!(count > 0);
    }

    #[test]
    fn test_total_transitions() {
        let mut mgr = DvfsScalingManager::new();
        let gov = FrequencyGovernor::new(0, 2400);
        mgr.register_governor(gov);
        mgr.scale_all_to_frequency(1800);
        assert!(mgr.get_total_transitions() > 0);
    }

    #[test]
    fn test_enable_disable_dvfs() {
        let mut mgr = DvfsScalingManager::new();
        mgr.disable();
        assert!(!mgr.is_enabled());
        mgr.enable();
        assert!(mgr.is_enabled());
    }

    #[test]
    fn test_multiple_pstates() {
        let mut mgr = DvfsScalingManager::new();
        for i in 0..8 {
            let pstate = Pstate::new(2400 - i * 300, 1200 - i * 50, 50);
            mgr.add_pstate(pstate);
        }
        assert_eq!(mgr.get_pstate_count(), 8);
    }

    #[test]
    fn test_frequency_bounds() {
        let gov = FrequencyGovernor::new(0, 2400);
        assert_eq!(gov.max_frequency, 2400);
        assert!(gov.min_frequency > 0);
    }

    #[test]
    fn test_voltage_bounds() {
        let ctrl = VoltageController::new(0, 1200);
        assert_eq!(ctrl.max_voltage, 1200);
        assert!(ctrl.min_voltage > 0);
    }

    #[test]
    fn test_mode_transitions() {
        let mut gov = FrequencyGovernor::new(0, 2400);
        gov.set_mode(ScalingMode::PowerSave);
        let ps_freq = gov.get_frequency();
        gov.set_mode(ScalingMode::Performance);
        let perf_freq = gov.get_frequency();
        assert!(perf_freq > ps_freq);
    }

    #[test]
    fn test_governor_turbo_disable() {
        let mut gov = FrequencyGovernor::new(0, 2400);
        gov.enable_turbo();
        gov.disable_turbo();
        assert!(!gov.turbo_enabled);
    }

    #[test]
    fn test_voltage_step() {
        let ctrl = VoltageController::new(0, 1200);
        assert!(ctrl.voltage_step > 0);
    }

    #[test]
    fn test_get_governor_mut() {
        let mut mgr = DvfsScalingManager::new();
        mgr.register_governor(FrequencyGovernor::new(0, 2400));
        if let Some(g) = mgr.get_governor_mut(0) {
            g.enable_turbo();
        }
        assert!(mgr.get_governor(0).unwrap().turbo_enabled);
    }

    #[test]
    fn test_get_voltage_controller_mut() {
        let mut mgr = DvfsScalingManager::new();
        mgr.register_voltage_controller(VoltageController::new(0, 1200));
        if let Some(c) = mgr.get_voltage_controller_mut(0) {
            c.increase_voltage();
        }
        assert!(mgr.get_voltage_controller(0).is_some());
    }

    #[test]
    fn test_pstate_latency() {
        let pstate = Pstate::new(2400, 1200, 50);
        assert!(pstate.transition_latency_us > 0);
    }

    #[test]
    fn test_scaling_mode_custom() {
        let mut gov = FrequencyGovernor::new(0, 2400);
        gov.set_mode(ScalingMode::Custom);
        assert_eq!(gov.get_mode(), ScalingMode::Custom);
    }
}
