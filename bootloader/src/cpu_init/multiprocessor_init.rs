//! Multiprocessor Initialization - SMP Boot Support
//!
//! Handles multiprocessor initialization including:
//! - CPU detection and enumeration
//! - APIC configuration
//! - Application Processor (AP) startup
//! - CPU handoff and synchronization

use core::fmt;
use alloc::vec::Vec;
use alloc::string::String;
use alloc::format;

/// CPU type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuType {
    BootProcessor,        // BSP
    ApplicationProcessor, // AP
    Unknown,
}

impl fmt::Display for CpuType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CpuType::BootProcessor => write!(f, "BSP"),
            CpuType::ApplicationProcessor => write!(f, "AP"),
            CpuType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// CPU info
#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub cpu_id: u32,
    pub cpu_type: CpuType,
    pub apic_id: u32,
    pub enabled: bool,
    pub online: bool,
    pub features: u32,
}

impl CpuInfo {
    /// Create new CPU info
    pub fn new(id: u32, apic: u32, cpu_type: CpuType) -> Self {
        CpuInfo {
            cpu_id: id,
            cpu_type,
            apic_id: apic,
            enabled: false,
            online: false,
            features: 0,
        }
    }

    /// Enable CPU
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Bring CPU online
    pub fn bring_online(&mut self) {
        self.online = true;
    }

    /// Add CPU feature
    pub fn add_feature(&mut self, feature: u32) {
        self.features |= feature;
    }
}

impl fmt::Display for CpuInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CPU{}: {} (APIC: 0x{:x}, Online: {})",
            self.cpu_id, self.cpu_type, self.apic_id, self.online
        )
    }
}

/// APIC mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApicMode {
    PIC,        // Legacy PIC mode
    APIC,       // Local APIC
    X2APIC,     // Extended x2APIC
    Unknown,
}

impl fmt::Display for ApicMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApicMode::PIC => write!(f, "PIC"),
            ApicMode::APIC => write!(f, "APIC"),
            ApicMode::X2APIC => write!(f, "x2APIC"),
            ApicMode::Unknown => write!(f, "Unknown"),
        }
    }
}

/// APIC configuration
#[derive(Debug, Clone)]
pub struct ApicConfig {
    pub mode: ApicMode,
    pub lapic_address: u64,
    pub ioapic_address: u64,
    pub enabled: bool,
    pub interrupt_base: u32,
}

impl ApicConfig {
    /// Create new APIC config
    pub fn new() -> Self {
        ApicConfig {
            mode: ApicMode::Unknown,
            lapic_address: 0xFEE00000,  // Default LAPIC address
            ioapic_address: 0xFEC00000, // Default IOAPIC address
            enabled: false,
            interrupt_base: 32,
        }
    }

    /// Set APIC mode
    pub fn set_mode(&mut self, mode: ApicMode) {
        self.mode = mode;
    }

    /// Enable APIC
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Set interrupt base
    pub fn set_interrupt_base(&mut self, base: u32) {
        self.interrupt_base = base;
    }
}

impl fmt::Display for ApicConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "APIC {{ mode: {}, enabled: {}, base_irq: {} }}",
            self.mode, self.enabled, self.interrupt_base
        )
    }
}

/// AP startup status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApStartupStatus {
    Pending,
    Initializing,
    Running,
    Failed,
}

impl fmt::Display for ApStartupStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ApStartupStatus::Pending => write!(f, "Pending"),
            ApStartupStatus::Initializing => write!(f, "Initializing"),
            ApStartupStatus::Running => write!(f, "Running"),
            ApStartupStatus::Failed => write!(f, "Failed"),
        }
    }
}

/// AP startup record
#[derive(Debug, Clone)]
pub struct ApStartupRecord {
    pub cpu_id: u32,
    pub apic_id: u32,
    pub status: ApStartupStatus,
    pub attempts: u32,
    pub startup_time: u64,
    pub error_message: String,
}

impl ApStartupRecord {
    /// Create new AP startup record
    pub fn new(cpu_id: u32, apic_id: u32) -> Self {
        ApStartupRecord {
            cpu_id,
            apic_id,
            status: ApStartupStatus::Pending,
            attempts: 0,
            startup_time: 0,
            error_message: String::new(),
        }
    }

    /// Set startup time
    pub fn set_startup_time(&mut self, time: u64) {
        self.startup_time = time;
    }

    /// Set error
    pub fn set_error(&mut self, msg: &str) {
        self.error_message = String::from(msg);
        self.status = ApStartupStatus::Failed;
    }
}

impl fmt::Display for ApStartupRecord {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "AP{}: {} (Attempts: {})",
            self.cpu_id, self.status, self.attempts
        )
    }
}

/// Multiprocessor Initializer
pub struct MultiprocessorInit {
    cpus: Vec<CpuInfo>,
    apic_config: ApicConfig,
    ap_records: Vec<ApStartupRecord>,
    bsp_cpu_id: u32,
    total_cpus: u32,
    online_cpus: u32,
    is_mp_enabled: bool,
}

impl MultiprocessorInit {
    /// Create new multiprocessor initializer
    pub fn new() -> Self {
        MultiprocessorInit {
            cpus: Vec::new(),
            apic_config: ApicConfig::new(),
            ap_records: Vec::new(),
            bsp_cpu_id: 0,
            total_cpus: 0,
            online_cpus: 0,
            is_mp_enabled: false,
        }
    }

    /// Register CPU
    pub fn register_cpu(&mut self, mut info: CpuInfo) -> bool {
        if info.cpu_type == CpuType::BootProcessor {
            self.bsp_cpu_id = info.cpu_id;
            info.enable();
            info.bring_online();
            self.online_cpus += 1;
        }

        self.cpus.push(info);
        self.total_cpus += 1;
        true
    }

    /// Get CPU count
    pub fn get_cpu_count(&self) -> u32 {
        self.total_cpus
    }

    /// Get online CPU count
    pub fn get_online_cpus(&self) -> u32 {
        self.online_cpus
    }

    /// Get CPU info by ID
    pub fn get_cpu(&self, id: u32) -> Option<&CpuInfo> {
        self.cpus.iter().find(|c| c.cpu_id == id)
    }

    /// Configure APIC
    pub fn configure_apic(&mut self, mode: ApicMode) -> bool {
        self.apic_config.set_mode(mode);
        self.apic_config.enable();
        true
    }

    /// Get APIC config
    pub fn get_apic_config(&self) -> &ApicConfig {
        &self.apic_config
    }

    /// Record AP startup
    pub fn record_ap_startup(&mut self, record: ApStartupRecord) {
        self.ap_records.push(record);
    }

    /// Start AP
    pub fn start_ap(&mut self, cpu_id: u32) -> bool {
        let mut record = ApStartupRecord::new(cpu_id, cpu_id);
        record.status = ApStartupStatus::Initializing;
        record.attempts += 1;

        // Find CPU and enable it
        for cpu in &mut self.cpus {
            if cpu.cpu_id == cpu_id {
                cpu.enable();
                cpu.bring_online();
                self.online_cpus += 1;
                record.status = ApStartupStatus::Running;
                self.record_ap_startup(record);
                return true;
            }
        }

        record.set_error("CPU not found");
        self.record_ap_startup(record);
        false
    }

    /// Start all APs
    pub fn start_all_aps(&mut self) -> bool {
        let mut started = 0;
        let ap_count = self.cpus
            .iter()
            .filter(|c| c.cpu_type == CpuType::ApplicationProcessor)
            .count();

        for cpu in &mut self.cpus {
            if cpu.cpu_type == CpuType::ApplicationProcessor && !cpu.online {
                cpu.enable();
                cpu.bring_online();
                self.online_cpus += 1;
                started += 1;
            }
        }

        self.is_mp_enabled = started == ap_count;
        started == ap_count
    }

    /// Enable SMP
    pub fn enable_smp(&mut self) -> bool {
        if !self.apic_config.enabled {
            return false;
        }

        self.start_all_aps()
    }

    /// Get SMP status
    pub fn is_smp_enabled(&self) -> bool {
        self.is_mp_enabled && self.online_cpus > 1
    }

    /// Get startup success rate
    pub fn get_startup_success_rate(&self) -> u32 {
        if self.ap_records.is_empty() {
            return 0;
        }

        let successful = self.ap_records
            .iter()
            .filter(|r| r.status == ApStartupStatus::Running)
            .count();

        ((successful as u64 * 100) / (self.ap_records.len() as u64)) as u32
    }

    /// Get multiprocessor report
    pub fn mp_report(&self) -> String {
        let mut report = String::from("=== Multiprocessor Report ===\n");

        report.push_str(&format!("Total CPUs: {}\n", self.total_cpus));
        report.push_str(&format!("Online CPUs: {}\n", self.online_cpus));
        report.push_str(&format!("SMP Enabled: {}\n", self.is_mp_enabled));
        report.push_str(&format!("\n{}\n", self.apic_config));

        report.push_str("\n--- CPU List ---\n");
        for cpu in &self.cpus {
            report.push_str(&format!("{}\n", cpu));
        }

        report.push_str("\n--- AP Startup Records ---\n");
        for record in &self.ap_records {
            report.push_str(&format!("{}\n", record));
        }

        report.push_str(&format!("Startup Success Rate: {}%\n", self.get_startup_success_rate()));

        report
    }
}

impl fmt::Display for MultiprocessorInit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "MultiprocessorInit {{ cpus: {}, online: {}, smp: {} }}",
            self.total_cpus, self.online_cpus, self.is_mp_enabled
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpu_info_creation() {
        let cpu = CpuInfo::new(0, 0, CpuType::BootProcessor);
        assert_eq!(cpu.cpu_id, 0);
        assert_eq!(cpu.apic_id, 0);
    }

    #[test]
    fn test_cpu_info_enable() {
        let mut cpu = CpuInfo::new(0, 0, CpuType::BootProcessor);
        cpu.enable();
        assert!(cpu.enabled);
    }

    #[test]
    fn test_cpu_info_online() {
        let mut cpu = CpuInfo::new(0, 0, CpuType::BootProcessor);
        cpu.bring_online();
        assert!(cpu.online);
    }

    #[test]
    fn test_apic_config_creation() {
        let config = ApicConfig::new();
        assert_eq!(config.lapic_address, 0xFEE00000);
    }

    #[test]
    fn test_apic_config_mode() {
        let mut config = ApicConfig::new();
        config.set_mode(ApicMode::APIC);
        assert_eq!(config.mode, ApicMode::APIC);
    }

    #[test]
    fn test_apic_config_enable() {
        let mut config = ApicConfig::new();
        config.enable();
        assert!(config.enabled);
    }

    #[test]
    fn test_ap_startup_record() {
        let record = ApStartupRecord::new(1, 1);
        assert_eq!(record.status, ApStartupStatus::Pending);
    }

    #[test]
    fn test_multiprocessor_init_creation() {
        let mp = MultiprocessorInit::new();
        assert_eq!(mp.get_cpu_count(), 0);
    }

    #[test]
    fn test_multiprocessor_register_bsp() {
        let mut mp = MultiprocessorInit::new();
        let cpu = CpuInfo::new(0, 0, CpuType::BootProcessor);
        assert!(mp.register_cpu(cpu));
        assert_eq!(mp.get_cpu_count(), 1);
        assert_eq!(mp.get_online_cpus(), 1);
    }

    #[test]
    fn test_multiprocessor_register_ap() {
        let mut mp = MultiprocessorInit::new();
        let bsp = CpuInfo::new(0, 0, CpuType::BootProcessor);
        let ap = CpuInfo::new(1, 1, CpuType::ApplicationProcessor);
        mp.register_cpu(bsp);
        mp.register_cpu(ap);
        assert_eq!(mp.get_cpu_count(), 2);
    }

    #[test]
    fn test_multiprocessor_configure_apic() {
        let mut mp = MultiprocessorInit::new();
        assert!(mp.configure_apic(ApicMode::APIC));
        assert_eq!(mp.get_apic_config().mode, ApicMode::APIC);
    }

    #[test]
    fn test_multiprocessor_start_ap() {
        let mut mp = MultiprocessorInit::new();
        let bsp = CpuInfo::new(0, 0, CpuType::BootProcessor);
        let ap = CpuInfo::new(1, 1, CpuType::ApplicationProcessor);
        mp.register_cpu(bsp);
        mp.register_cpu(ap);
        assert!(mp.start_ap(1));
        assert_eq!(mp.get_online_cpus(), 2);
    }

    #[test]
    fn test_multiprocessor_start_all_aps() {
        let mut mp = MultiprocessorInit::new();
        let bsp = CpuInfo::new(0, 0, CpuType::BootProcessor);
        let ap1 = CpuInfo::new(1, 1, CpuType::ApplicationProcessor);
        let ap2 = CpuInfo::new(2, 2, CpuType::ApplicationProcessor);
        mp.register_cpu(bsp);
        mp.register_cpu(ap1);
        mp.register_cpu(ap2);
        assert!(mp.start_all_aps());
        assert_eq!(mp.get_online_cpus(), 3);
    }

    #[test]
    fn test_multiprocessor_enable_smp() {
        let mut mp = MultiprocessorInit::new();
        let bsp = CpuInfo::new(0, 0, CpuType::BootProcessor);
        let ap = CpuInfo::new(1, 1, CpuType::ApplicationProcessor);
        mp.register_cpu(bsp);
        mp.register_cpu(ap);
        mp.configure_apic(ApicMode::APIC);
        assert!(mp.enable_smp());
        assert!(mp.is_smp_enabled());
    }

    #[test]
    fn test_multiprocessor_report() {
        let mut mp = MultiprocessorInit::new();
        let bsp = CpuInfo::new(0, 0, CpuType::BootProcessor);
        mp.register_cpu(bsp);
        let report = mp.mp_report();
        assert!(report.contains("Multiprocessor Report"));
    }
}
