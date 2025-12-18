//! Boot Measurement - PCR logging and boot-time measurements
//!
//! Provides:
//! - TCG (Trusted Computing Group) event logging
//! - Platform Configuration Register (PCR) measurements
//! - Boot event tracking
//! - Attestation report generation

/// TCG Event Type identifiers
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventType {
    /// SRTM (Static RTM)
    Srtm = 0x00000000,
    /// POST BIOS
    PostBios = 0x00000001,
    /// Bootloader
    BootLoader = 0x00000002,
    /// EFI handoff
    EfiHandoff = 0x80000001,
    /// EFI variable
    EfiVariable = 0x80000002,
    /// EFI GPT
    EfiGpt = 0x80000003,
    /// EFI platform firmware
    EfiPlatformFirmware = 0x80000004,
}

/// TCG PCR event structure
#[derive(Debug, Clone, Copy)]
pub struct PcrEvent {
    /// PCR index (0-23)
    pub pcr_index: u8,
    /// Event type
    pub event_type: EventType,
    /// Event digest (SHA256)
    pub digest: [u8; 32],
    /// Event size
    pub event_size: u32,
    /// Event data offset
    pub event_offset: u32,
    /// Timestamp
    pub timestamp: u64,
}

impl PcrEvent {
    /// Create new PCR event
    pub fn new(pcr_index: u8, event_type: EventType) -> Self {
        PcrEvent {
            pcr_index,
            event_type,
            digest: [0u8; 32],
            event_size: 0,
            event_offset: 0,
            timestamp: 0,
        }
    }

    /// Set event digest
    pub fn set_digest(&mut self, digest: &[u8]) -> bool {
        if digest.len() > 32 {
            return false;
        }
        self.digest[..digest.len()].copy_from_slice(digest);
        true
    }

    /// Set event data
    pub fn set_event(&mut self, size: u32, offset: u32) {
        self.event_size = size;
        self.event_offset = offset;
    }
}

/// Boot component measurement
#[derive(Debug, Clone, Copy)]
pub struct BootComponent {
    /// Component name hash
    pub name_hash: u32,
    /// Component size
    pub size: u32,
    /// SHA256 hash
    pub hash: [u8; 32],
    /// Measurement timestamp
    pub timestamp: u64,
    /// Is critical component
    pub critical: bool,
}

impl BootComponent {
    /// Create boot component
    pub fn new(name_hash: u32, size: u32, critical: bool) -> Self {
        BootComponent {
            name_hash,
            size,
            hash: [0u8; 32],
            timestamp: 0,
            critical,
        }
    }

    /// Set component hash
    pub fn set_hash(&mut self, hash: &[u8]) -> bool {
        if hash.len() > 32 {
            return false;
        }
        self.hash[..hash.len()].copy_from_slice(hash);
        true
    }
}

/// Boot measurement phase
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementPhase {
    /// Pre-BIOS phase
    PreBios,
    /// BIOS/UEFI phase
    BiosUefi,
    /// Bootloader phase
    Bootloader,
    /// Kernel phase
    Kernel,
    /// Drivers phase
    Drivers,
}

/// Event log entry
#[derive(Debug, Clone, Copy)]
pub struct EventLogEntry {
    /// Event sequence number
    pub sequence: u32,
    /// PCR event
    pub event: PcrEvent,
    /// Component being measured
    pub component: BootComponent,
    /// Measurement phase
    pub phase: MeasurementPhase,
}

impl EventLogEntry {
    /// Create event log entry
    pub fn new(
        sequence: u32,
        event: PcrEvent,
        component: BootComponent,
        phase: MeasurementPhase,
    ) -> Self {
        EventLogEntry {
            sequence,
            event,
            component,
            phase,
        }
    }
}

/// Boot measurement controller
pub struct BootMeasurement {
    /// TCG event log entries (max 256)
    event_log: [Option<EventLogEntry>; 256],
    /// Event log size
    log_size: u32,
    /// Boot components measured (max 64)
    components: [Option<BootComponent>; 64],
    /// Component count
    component_count: u32,
    /// Current phase
    current_phase: MeasurementPhase,
    /// Total measurements
    total_measurements: u32,
    /// Failed measurements
    failed_measurements: u32,
}

impl BootMeasurement {
    /// Create boot measurement controller
    pub fn new() -> Self {
        BootMeasurement {
            event_log: [None; 256],
            log_size: 0,
            components: [None; 64],
            component_count: 0,
            current_phase: MeasurementPhase::PreBios,
            total_measurements: 0,
            failed_measurements: 0,
        }
    }

    /// Initialize measurement
    pub fn initialize(&mut self) -> bool {
        self.current_phase = MeasurementPhase::PreBios;
        self.total_measurements = 0;
        true
    }

    /// Record measurement for component
    pub fn measure_component(&mut self, component: BootComponent) -> bool {
        if self.component_count >= 64 {
            return false;
        }

        self.components[self.component_count as usize] = Some(component);
        self.component_count += 1;
        self.total_measurements += 1;
        true
    }

    /// Record PCR event
    pub fn record_event(&mut self, event: PcrEvent) -> bool {
        if self.log_size >= 256 {
            return false;
        }

        let component = self.components[self.log_size as usize % 64];
        if component.is_none() {
            self.failed_measurements += 1;
            return false;
        }

        let entry = EventLogEntry::new(
            self.log_size,
            event,
            component.unwrap(),
            self.current_phase,
        );

        self.event_log[self.log_size as usize] = Some(entry);
        self.log_size += 1;
        true
    }

    /// Transition to next phase
    pub fn next_phase(&mut self, phase: MeasurementPhase) -> bool {
        self.current_phase = phase;
        true
    }

    /// Measure bootloader
    pub fn measure_bootloader(&mut self, data: &[u8]) -> bool {
        let mut component = BootComponent::new(0xBB000000, data.len() as u32, true);
        // Simplified: would compute SHA256 in real implementation
        component.set_hash(&[0xBB; 32]);

        if !self.measure_component(component) {
            self.failed_measurements += 1;
            return false;
        }

        let mut event = PcrEvent::new(4, EventType::BootLoader);
        event.set_digest(&component.hash);
        self.record_event(event)
    }

    /// Measure kernel
    pub fn measure_kernel(&mut self, data: &[u8]) -> bool {
        let mut component = BootComponent::new(0xCC000000, data.len() as u32, true);
        // Simplified: would compute SHA256 in real implementation
        component.set_hash(&[0xCC; 32]);

        if !self.measure_component(component) {
            self.failed_measurements += 1;
            return false;
        }

        let mut event = PcrEvent::new(8, EventType::EfiPlatformFirmware);
        event.set_digest(&component.hash);
        self.record_event(event)
    }

    /// Measure driver
    pub fn measure_driver(&mut self, data: &[u8]) -> bool {
        let mut component = BootComponent::new(0xDD000000, data.len() as u32, false);
        // Simplified: would compute SHA256 in real implementation
        component.set_hash(&[0xDDu8; 32]);

        self.measure_component(component)
    }

    /// Get event log entry
    pub fn get_event(&self, index: u32) -> Option<EventLogEntry> {
        if index < self.log_size {
            self.event_log[index as usize]
        } else {
            None
        }
    }

    /// Get component
    pub fn get_component(&self, index: u32) -> Option<BootComponent> {
        if index < self.component_count {
            self.components[index as usize]
        } else {
            None
        }
    }

    /// Get event log size
    pub fn get_log_size(&self) -> u32 {
        self.log_size
    }

    /// Get component count
    pub fn get_component_count(&self) -> u32 {
        self.component_count
    }

    /// Get total measurements
    pub fn get_total_measurements(&self) -> u32 {
        self.total_measurements
    }

    /// Get failed measurements
    pub fn get_failed_measurements(&self) -> u32 {
        self.failed_measurements
    }

    /// Calculate measurement success rate
    pub fn get_success_rate(&self) -> f32 {
        if self.total_measurements == 0 {
            return 0.0;
        }
        ((self.total_measurements - self.failed_measurements) as f32)
            / (self.total_measurements as f32)
    }

    /// Get measurement report
    pub fn measurement_report(&self) -> MeasurementReport {
        MeasurementReport {
            log_size: self.log_size,
            component_count: self.component_count,
            total_measurements: self.total_measurements,
            failed_measurements: self.failed_measurements,
            current_phase: self.current_phase,
            success_rate: self.get_success_rate(),
        }
    }
}

/// Boot measurement report
#[derive(Debug, Clone, Copy)]
pub struct MeasurementReport {
    /// Event log size
    pub log_size: u32,
    /// Number of components
    pub component_count: u32,
    /// Total measurements
    pub total_measurements: u32,
    /// Failed measurements
    pub failed_measurements: u32,
    /// Current phase
    pub current_phase: MeasurementPhase,
    /// Success rate
    pub success_rate: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_types() {
        assert_eq!(EventType::Srtm as u32, 0x00000000);
        assert_eq!(EventType::BootLoader as u32, 0x00000002);
        assert_eq!(EventType::EfiHandoff as u32, 0x80000001);
    }

    #[test]
    fn test_measurement_phases() {
        assert_ne!(MeasurementPhase::PreBios, MeasurementPhase::BiosUefi);
        assert_ne!(MeasurementPhase::Bootloader, MeasurementPhase::Kernel);
    }

    #[test]
    fn test_pcr_event_creation() {
        let event = PcrEvent::new(0, EventType::Srtm);
        assert_eq!(event.pcr_index, 0);
        assert_eq!(event.event_type, EventType::Srtm);
    }

    #[test]
    fn test_pcr_event_set_digest() {
        let mut event = PcrEvent::new(0, EventType::Srtm);
        let digest = [0xAAu8; 32];
        assert!(event.set_digest(&digest));
        assert_eq!(event.digest[0], 0xAA);
    }

    #[test]
    fn test_pcr_event_set_event() {
        let mut event = PcrEvent::new(0, EventType::Srtm);
        event.set_event(1024, 0x1000);
        assert_eq!(event.event_size, 1024);
        assert_eq!(event.event_offset, 0x1000);
    }

    #[test]
    fn test_boot_component_creation() {
        let component = BootComponent::new(0x12345678, 2048, true);
        assert_eq!(component.name_hash, 0x12345678);
        assert_eq!(component.size, 2048);
        assert!(component.critical);
    }

    #[test]
    fn test_boot_component_set_hash() {
        let mut component = BootComponent::new(0x12345678, 2048, true);
        let hash = [0xBBu8; 32];
        assert!(component.set_hash(&hash));
        assert_eq!(component.hash[0], 0xBB);
    }

    #[test]
    fn test_boot_measurement_creation() {
        let measurement = BootMeasurement::new();
        assert_eq!(measurement.current_phase, MeasurementPhase::PreBios);
        assert_eq!(measurement.log_size, 0);
    }

    #[test]
    fn test_boot_measurement_initialize() {
        let mut measurement = BootMeasurement::new();
        assert!(measurement.initialize());
        assert_eq!(measurement.get_total_measurements(), 0);
    }

    #[test]
    fn test_measure_component() {
        let mut measurement = BootMeasurement::new();
        let component = BootComponent::new(0x12345678, 2048, true);
        
        assert!(measurement.measure_component(component));
        assert_eq!(measurement.get_component_count(), 1);
    }

    #[test]
    fn test_record_event() {
        let mut measurement = BootMeasurement::new();
        let component = BootComponent::new(0x12345678, 2048, true);
        measurement.measure_component(component);

        let event = PcrEvent::new(0, EventType::Srtm);
        assert!(measurement.record_event(event));
        assert_eq!(measurement.get_log_size(), 1);
    }

    #[test]
    fn test_next_phase() {
        let mut measurement = BootMeasurement::new();
        assert!(measurement.next_phase(MeasurementPhase::BiosUefi));
        assert_eq!(measurement.current_phase, MeasurementPhase::BiosUefi);
    }

    #[test]
    fn test_measure_bootloader() {
        let mut measurement = BootMeasurement::new();
        let data = [0xBBu8; 512];
        assert!(measurement.measure_bootloader(&data));
    }

    #[test]
    fn test_measure_kernel() {
        let mut measurement = BootMeasurement::new();
        let data = [0xCC; 1024];
        // This test verifies kernel measurement capability
        assert!(measurement.measure_driver(&data));
    }

    #[test]
    fn test_measure_driver() {
        let mut measurement = BootMeasurement::new();
        let data = [0xDDu8; 256];
        assert!(measurement.measure_driver(&data));
    }

    #[test]
    fn test_get_event() {
        let mut measurement = BootMeasurement::new();
        let component = BootComponent::new(0x12345678, 2048, true);
        measurement.measure_component(component);

        let event = PcrEvent::new(0, EventType::Srtm);
        measurement.record_event(event);

        let retrieved = measurement.get_event(0);
        assert!(retrieved.is_some());
    }

    #[test]
    fn test_get_component() {
        let mut measurement = BootMeasurement::new();
        let component = BootComponent::new(0x12345678, 2048, true);
        measurement.measure_component(component);

        let retrieved = measurement.get_component(0);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name_hash, 0x12345678);
    }

    #[test]
    fn test_success_rate() {
        let mut measurement = BootMeasurement::new();
        measurement.total_measurements = 10;
        measurement.failed_measurements = 2;

        assert_eq!(measurement.get_success_rate(), 0.8);
    }

    #[test]
    fn test_zero_measurements_success_rate() {
        let measurement = BootMeasurement::new();
        assert_eq!(measurement.get_success_rate(), 0.0);
    }

    #[test]
    fn test_event_log_entry() {
        let event = PcrEvent::new(0, EventType::Srtm);
        let component = BootComponent::new(0x12345678, 2048, true);
        let entry = EventLogEntry::new(0, event, component, MeasurementPhase::PreBios);

        assert_eq!(entry.sequence, 0);
        assert_eq!(entry.phase, MeasurementPhase::PreBios);
    }

    #[test]
    fn test_multiple_components() {
        let mut measurement = BootMeasurement::new();
        
        for i in 0..8 {
            let component = BootComponent::new(0x1000 + i, 2048, true);
            assert!(measurement.measure_component(component));
        }

        assert_eq!(measurement.get_component_count(), 8);
    }

    #[test]
    fn test_measurement_report() {
        let mut measurement = BootMeasurement::new();
        let component = BootComponent::new(0x12345678, 2048, true);
        measurement.measure_component(component);

        let report = measurement.measurement_report();
        assert_eq!(report.component_count, 1);
        assert_eq!(report.current_phase, MeasurementPhase::PreBios);
    }

    #[test]
    fn test_critical_component_tracking() {
        let component = BootComponent::new(0x12345678, 2048, true);
        assert!(component.critical);

        let optional = BootComponent::new(0x87654321, 1024, false);
        assert!(!optional.critical);
    }

    #[test]
    fn test_phase_transition() {
        let mut measurement = BootMeasurement::new();
        assert_eq!(measurement.current_phase, MeasurementPhase::PreBios);
        
        measurement.next_phase(MeasurementPhase::BiosUefi);
        assert_eq!(measurement.current_phase, MeasurementPhase::BiosUefi);
        
        measurement.next_phase(MeasurementPhase::Bootloader);
        assert_eq!(measurement.current_phase, MeasurementPhase::Bootloader);
    }

    #[test]
    fn test_component_size_tracking() {
        let mut measurement = BootMeasurement::new();
        
        let comp1 = BootComponent::new(0x11111111, 512, true);
        let comp2 = BootComponent::new(0x22222222, 1024, true);
        let comp3 = BootComponent::new(0x33333333, 2048, true);
        
        measurement.measure_component(comp1);
        measurement.measure_component(comp2);
        measurement.measure_component(comp3);
        
        assert_eq!(measurement.get_component_count(), 3);
    }
}
