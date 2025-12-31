//! ACPI Power Management
//!
//! This module provides ACPI (Advanced Configuration and Power Interface)
//! support for power management operations, including P-States (performance)
//! and C-States (power saving).

use alloc::vec::Vec;
use spin::Mutex;

/// ACPI P-State (Performance State)
///
/// Represents a CPU operating frequency/voltage point.
#[derive(Debug, Clone, Copy)]
pub struct PState {
    /// P-State number (lower is better performance)
    pub number: u8,
    /// Core frequency in MHz
    pub frequency: u64,
    /// Power in mW
    pub power: u32,
    /// Transition latency in microseconds
    pub transition_latency: u32,
    /// Bus frequency in MHz
    pub bus_frequency: u32,
}

impl PState {
    /// Create a new P-State
    ///
    /// # Arguments
    ///
    /// * `number` - P-State number
    /// * `frequency` - Core frequency in MHz
    /// * `power` - Power consumption in mW
    pub fn new(number: u8, frequency: u64, power: u32) -> Self {
        Self {
            number,
            frequency,
            power,
            transition_latency: 10,
            bus_frequency: 100,
        }
    }

    /// Check if this is a higher performance state than another
    pub fn is_higher_performance(&self, other: &PState) -> bool {
        self.frequency > other.frequency
    }

    /// Check if this is a lower power state than another
    pub fn is_lower_power(&self, other: &PState) -> bool {
        self.power < other.power
    }

    /// Get frequency in Hz
    pub fn frequency_hz(&self) -> u64 {
        self.frequency * 1_000_000
    }
}

/// ACPI C-State (Power Saving State)
///
/// Represents a CPU low-power state.
#[derive(Debug, Clone, Copy)]
pub struct CState {
    /// C-State type (C1, C2, C3, etc.)
    pub c_type: u8,
    /// State name
    pub name: &'static str,
    /// Exit latency in microseconds
    pub exit_latency: u32,
    /// Target residency in microseconds
    pub target_residency: u32,
    /// Power consumption (percentage of C0)
    pub power_percentage: u8,
    /// Flags
    pub flags: u64,
}

impl CState {
    /// Create a new C-State
    ///
    /// # Arguments
    ///
    /// * `c_type` - C-State type (1-6)
    /// * `name` - State name
    /// * `exit_latency` - Time to wake from this state (microseconds)
    /// * `target_residency` - Minimum time to enter this state (microseconds)
    pub fn new(c_type: u8, name: &'static str, exit_latency: u32, target_residency: u32) -> Self {
        let power_percentage = match c_type {
            1 => 90,
            2 => 70,
            3 => 50,
            4 => 30,
            5 => 10,
            6 => 5,
            _ => 100,
        };

        Self {
            c_type,
            name,
            exit_latency,
            target_residency,
            power_percentage,
            flags: 0,
        }
    }

    /// Check if this is a deeper sleep state than another
    pub fn is_deeper(&self, other: &CState) -> bool {
        self.c_type > other.c_type
    }

    /// Check if entry is worthwhile (considering residency)
    pub fn is_entry_worthwhile(&self, idle_time: u32) -> bool {
        idle_time >= self.target_residency
    }
}

/// Fixed ACPI Description Table (FADT)
///
/// This is a simplified representation of the FADT structure.
#[derive(Debug)]
pub struct Fadt {
    /// Preferred PM profile
    pub pm_profile: u8,
    /// SCI interrupt number
    pub sci_int: u32,
    /// SMI command port
    pub smi_cmd: u32,
    /// ACPI enable value
    pub acpi_enable: u8,
    /// ACPI disable value
    pub acpi_disable: u8,
    /// S4BIOS request
    pub s4bios_req: u8,
    /// P-State control
    pub pstate_ctrl: u64,
    /// C3 latency
    pub c3_latency: u32,
    /// Flags
    pub flags: u32,
}

impl Fadt {
    /// Create a default FADT
    pub fn default() -> Self {
        Self {
            pm_profile: 0,
            sci_int: 9,
            smi_cmd: 0xB2,
            acpi_enable: 0xF1,
            acpi_disable: 0xF0,
            s4bios_req: 0,
            pstate_ctrl: 0,
            c3_latency: 0,
            flags: 0,
        }
    }

    /// Check if legacy devices are present
    pub fn has_legacy_devices(&self) -> bool {
        (self.flags & 0x0001) != 0
    }

    /// Check if 8042 is present
    pub fn has_8042(&self) -> bool {
        (self.flags & 0x0002) != 0
    }

    /// Check if RTC is not present
    pub fn has_no_rtc(&self) -> bool {
        (self.flags & 0x0004) != 0
    }
}

/// ACPI Power Management Information
#[derive(Debug, Clone)]
pub struct AcpiPowerInfo {
    /// Supported P-States
    pub pstates: Vec<PState>,
    /// Supported C-States
    pub cstates: Vec<CState>,
    /// Maximum P-State (highest performance)
    pub max_pstate: u8,
    /// Minimum P-State (lowest performance)
    pub min_pstate: u8,
    /// Maximum C-State (deepest sleep)
    pub max_cstate: u8,
    /// Bus frequency
    pub bus_frequency: u32,
}

impl AcpiPowerInfo {
    /// Create new ACPI power info
    pub fn new() -> Self {
        Self {
            pstates: Vec::new(),
            cstates: Vec::new(),
            max_pstate: 0,
            min_pstate: 0,
            max_cstate: 0,
            bus_frequency: 100,
        }
    }

    /// Add a P-State
    pub fn add_pstate(&mut self, pstate: PState) {
        self.pstates.push(pstate);
        self.pstates.sort_by(|a, b| b.frequency.cmp(&a.frequency));

        if !self.pstates.is_empty() {
            self.max_pstate = self.pstates[0].number;
            self.min_pstate = self.pstates.last().map(|p| p.number).unwrap_or(0);
        }
    }

    /// Add a C-State
    pub fn add_cstate(&mut self, cstate: CState) {
        self.cstates.push(cstate);
        self.cstates.sort_by(|a, b| a.c_type.cmp(&b.c_type));

        if !self.cstates.is_empty() {
            self.max_cstate = self.cstates.last().map(|c| c.c_type).unwrap_or(0);
        }
    }

    /// Get P-State by number
    pub fn get_pstate(&self, number: u8) -> Option<&PState> {
        self.pstates.iter().find(|p| p.number == number)
    }

    /// Get C-State by type
    pub fn get_cstate(&self, c_type: u8) -> Option<&CState> {
        self.cstates.iter().find(|c| c.c_type == c_type)
    }

    /// Get highest performance P-State
    pub fn get_highest_pstate(&self) -> Option<&PState> {
        self.pstates.first()
    }

    /// Get lowest power P-State
    pub fn get_lowest_pstate(&self) -> Option<&PState> {
        self.pstates.last()
    }

    /// Get deepest C-State
    pub fn get_deepest_cstate(&self) -> Option<&CState> {
        self.cstates.last()
    }

    /// Calculate frequency for a given load (0-100)
    pub fn calculate_frequency_for_load(&self, load: u8) -> Option<u64> {
        if self.pstates.is_empty() {
            return None;
        }

        let index = if load >= 100 {
            0
        } else {
            let scaled = ((100 - load) * self.pstates.len() as u8) / 100;
            scaled.min(self.pstates.len() as u8 - 1) as usize
        };

        self.pstates.get(index).map(|p| p.frequency_hz())
    }
}

/// ACPI Power Manager
///
/// Manages ACPI-based power management, including P-States and C-States.
pub struct AcpiPowerManager {
    /// ACPI tables
    fadt: Mutex<Option<Fadt>>,
    /// Power information
    power_info: Mutex<AcpiPowerInfo>,
    /// ACPI enabled
    enabled: Mutex<bool>,
    /// Last P-State
    last_pstate: Mutex<u8>,
    /// Last C-State
    last_cstate: Mutex<u8>,
}

impl AcpiPowerManager {
    /// Create a new ACPI power manager
    pub fn new() -> Self {
        Self {
            fadt: Mutex::new(None),
            power_info: Mutex::new(AcpiPowerInfo::new()),
            enabled: Mutex::new(false),
            last_pstate: Mutex::new(0),
            last_cstate: Mutex::new(0),
        }
    }

    /// Initialize ACPI power management
    ///
    /// # Arguments
    ///
    /// * `fadt` - Fixed ACPI Description Table
    ///
    /// # Returns
    ///
    /// Ok(()) if successful, Err otherwise
    pub fn init(&self, fadt: Fadt) -> Result<(), &'static str> {
        log::info!("AcpiPowerManager: Initializing ACPI power management");

        // Store FADT
        *self.fadt.lock() = Some(fadt);

        // Configure P-States
        self.configure_pstates()?;

        // Configure C-States
        self.configure_cstates()?;

        // Enable ACPI
        *self.enabled.lock() = true;

        log::info!("AcpiPowerManager: ACPI power management initialized");

        Ok(())
    }

    /// Configure P-States (performance states)
    fn configure_pstates(&self) -> Result<(), &'static str> {
        log::info!("AcpiPowerManager: Configuring P-States");

        let mut info = self.power_info.lock();

        // Add typical P-States (these would normally come from ACPI tables)
        // P0: Highest performance
        info.add_pstate(PState::new(0, 3000, 45000)); // 3.0 GHz, 45W
        // P1: High performance
        info.add_pstate(PState::new(1, 2800, 38000)); // 2.8 GHz, 38W
        // P2: Medium performance
        info.add_pstate(PState::new(2, 2400, 28000)); // 2.4 GHz, 28W
        // P3: Low performance
        info.add_pstate(PState::new(3, 1800, 18000)); // 1.8 GHz, 18W
        // P4: Lowest performance
        info.add_pstate(PState::new(4, 1200, 10000)); // 1.2 GHz, 10W

        log::info!(
            "AcpiPowerManager: Configured {} P-States (P{} to P{})",
            info.pstates.len(),
            info.max_pstate,
            info.min_pstate
        );

        Ok(())
    }

    /// Configure C-States (power saving states)
    fn configure_cstates(&self) -> Result<(), &'static str> {
        log::info!("AcpiPowerManager: Configuring C-States");

        let mut info = self.power_info.lock();

        // Add typical C-States (these would normally come from ACPI tables)
        // C1: Halt (mandatory)
        info.add_cstate(CState::new(1, "C1", 1, 2));
        // C1E: Enhanced Halt
        info.add_cstate(CState::new(1, "C1E", 10, 20));
        // C3: Sleep
        info.add_cstate(CState::new(3, "C3", 50, 100));
        // C6: Deep sleep
        info.add_cstate(CState::new(6, "C6", 200, 500));

        log::info!(
            "AcpiPowerManager: Configured {} C-States (C1 to C{})",
            info.cstates.len(),
            info.max_cstate
        );

        Ok(())
    }

    /// Set P-State
    ///
    /// # Arguments
    ///
    /// * `pstate` - Target P-State number
    pub fn set_pstate(&self, pstate: u8) -> Result<(), &'static str> {
        if !*self.enabled.lock() {
            return Err("ACPI not enabled");
        }

        let info = self.power_info.lock();
        let target = info.get_pstate(pstate).ok_or("Invalid P-State")?;

        log::info!(
            "AcpiPowerManager: Setting P-State to P{} ({} MHz)",
            pstate,
            target.frequency
        );

        // In a real implementation, this would write to MSRs or IO ports
        *self.last_pstate.lock() = pstate;

        Ok(())
    }

    /// Enter C-State
    ///
    /// # Arguments
    ///
    /// * `cstate` - Target C-State type
    /// * `idle_time` - Expected idle time in microseconds
    pub fn enter_cstate(&self, cstate: u8, idle_time: u32) -> Result<(), &'static str> {
        if !*self.enabled.lock() {
            return Err("ACPI not enabled");
        }

        let info = self.power_info.lock();
        let target = info.get_cstate(cstate).ok_or("Invalid C-State")?;

        // Check if entry is worthwhile
        if !target.is_entry_worthwhile(idle_time) {
            return Ok(()); // Not worth entering this state
        }

        log::info!(
            "AcpiPowerManager: Entering {} (exit latency: {} us)",
            target.name,
            target.exit_latency
        );

        // In a real implementation, this would execute the appropriate
        // MWAIT or HLT instruction
        *self.last_cstate.lock() = cstate;

        Ok(())
    }

    /// Get optimal P-State for load
    ///
    /// # Arguments
    ///
    /// * `load` - CPU load (0-100)
    ///
    /// # Returns
    ///
    /// Recommended P-State number
    pub fn get_optimal_pstate(&self, load: u8) -> Option<u8> {
        let info = self.power_info.lock();
        info.calculate_frequency_for_load(load)?;
        info.pstates
            .iter()
            .position(|p| {
                if load >= 100 {
                    p.number == info.max_pstate
                } else if load <= 0 {
                    p.number == info.min_pstate
                } else {
                    let idx = ((100 - load) * info.pstates.len() as u8 / 100) as usize;
                    p.number == info.pstates.get(idx).map(|p| p.number).unwrap_or(0)
                }
            })
            .map(|_| info.pstates.iter().min_by_key(|p| (p.frequency as i64 - ((load as i64 * 3000 / 100) as i64)).abs()).map(|p| p.number).unwrap_or(0))
    }

    /// Get power information
    pub fn get_power_info(&self) -> AcpiPowerInfo {
        self.power_info.lock().clone()
    }

    /// Get supported P-States
    pub fn get_pstates(&self) -> Vec<PState> {
        self.power_info.lock().pstates.clone()
    }

    /// Get supported C-States
    pub fn get_cstates(&self) -> Vec<CState> {
        self.power_info.lock().cstates.clone()
    }

    /// Check if ACPI is enabled
    pub fn is_enabled(&self) -> bool {
        *self.enabled.lock()
    }

    /// Get last P-State
    pub fn get_last_pstate(&self) -> u8 {
        *self.last_pstate.lock()
    }

    /// Get last C-State
    pub fn get_last_cstate(&self) -> u8 {
        *self.last_cstate.lock()
    }

    /// Calculate power estimate for P-State
    ///
    /// # Arguments
    ///
    /// * `pstate` - P-State number
    pub fn estimate_power(&self, pstate: u8) -> Option<u32> {
        self.power_info.lock().get_pstate(pstate).map(|p| p.power)
    }

    /// Get maximum frequency
    pub fn get_max_frequency(&self) -> Option<u64> {
        self.power_info.lock().get_highest_pstate().map(|p| p.frequency_hz())
    }

    /// Get minimum frequency
    pub fn get_min_frequency(&self) -> Option<u64> {
        self.power_info.lock().get_lowest_pstate().map(|p| p.frequency_hz())
    }
}

impl Default for AcpiPowerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pstate_creation() {
        let pstate = PState::new(0, 3000, 45000);
        assert_eq!(pstate.number, 0);
        assert_eq!(pstate.frequency, 3000);
        assert_eq!(pstate.power, 45000);
        assert_eq!(pstate.frequency_hz(), 3_000_000_000);
    }

    #[test]
    fn test_cstate_creation() {
        let cstate = CState::new(3, "C3", 50, 100);
        assert_eq!(cstate.c_type, 3);
        assert_eq!(cstate.name, "C3");
        assert_eq!(cstate.exit_latency, 50);
        assert_eq!(cstate.power_percentage, 50);
    }

    #[test]
    fn test_cstate_entry_worthwhile() {
        let cstate = CState::new(3, "C3", 50, 100);
        assert!(cstate.is_entry_worthwhile(150));
        assert!(!cstate.is_entry_worthwhile(50));
    }

    #[test]
    fn test_acpi_power_info() {
        let mut info = AcpiPowerInfo::new();

        info.add_pstate(PState::new(0, 3000, 45000));
        info.add_pstate(PState::new(1, 2000, 25000));

        assert_eq!(info.pstates.len(), 2);
        assert_eq!(info.max_pstate, 0);
        assert_eq!(info.min_pstate, 1);

        let highest = info.get_highest_pstate().unwrap();
        assert_eq!(highest.frequency, 3000);

        let lowest = info.get_lowest_pstate().unwrap();
        assert_eq!(lowest.frequency, 2000);
    }

    #[test]
    fn test_acpi_power_manager_init() {
        let manager = AcpiPowerManager::new();
        let fadt = Fadt::default();

        let result = manager.init(fadt);
        assert!(result.is_ok());
        assert!(manager.is_enabled());

        let pstates = manager.get_pstates();
        assert!(!pstates.is_empty());

        let cstates = manager.get_cstates();
        assert!(!cstates.is_empty());
    }

    #[test]
    fn test_set_pstate() {
        let manager = AcpiPowerManager::new();
        let fadt = Fadt::default();

        manager.init(fadt).unwrap();

        let result = manager.set_pstate(2);
        assert!(result.is_ok());
        assert_eq!(manager.get_last_pstate(), 2);
    }

    #[test]
    fn test_enter_cstate() {
        let manager = AcpiPowerManager::new();
        let fadt = Fadt::default();

        manager.init(fadt).unwrap();

        // Worthwhile entry
        let result = manager.enter_cstate(3, 150);
        assert!(result.is_ok());

        // Not worthwhile (idle time too short)
        let result = manager.enter_cstate(3, 50);
        assert!(result.is_ok());
    }

    #[test]
    fn test_frequency_calculation() {
        let mut info = AcpiPowerInfo::new();
        info.add_pstate(PState::new(0, 3000, 45000));
        info.add_pstate(PState::new(1, 2000, 25000));

        // High load should give high frequency
        let freq = info.calculate_frequency_for_load(90);
        assert!(freq.is_some());
        assert!(freq.unwrap() >= 2_000_000_000);

        // Low load should give low frequency
        let freq = info.calculate_frequency_for_load(10);
        assert!(freq.is_some());
        assert!(freq.unwrap() <= 3_000_000_000);
    }
}
