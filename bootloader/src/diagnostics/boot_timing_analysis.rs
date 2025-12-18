//! Boot Timing Analysis - Detailed boot phase timing measurements
//!
//! Provides:
//! - Per-phase boot timing
//! - Bottleneck detection
//! - Performance metrics
//! - Timeline generation

/// Boot phases
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootPhase {
    /// Firmware initialization
    Firmware,
    /// Memory initialization
    Memory,
    /// CPU initialization
    Cpu,
    /// Device enumeration
    Devices,
    /// Kernel loading
    KernelLoad,
    /// Kernel launch
    KernelLaunch,
}

/// Timing measurement
#[derive(Debug, Clone, Copy)]
pub struct TimingMeasurement {
    /// Phase
    pub phase: BootPhase,
    /// Start time in milliseconds
    pub start_time: u64,
    /// End time in milliseconds
    pub end_time: u64,
    /// Valid flag
    pub valid: bool,
}

impl TimingMeasurement {
    /// Create timing measurement
    pub fn new(phase: BootPhase, start: u64) -> Self {
        TimingMeasurement {
            phase,
            start_time: start,
            end_time: start,
            valid: false,
        }
    }

    /// Complete measurement
    pub fn complete(&mut self, end_time: u64) {
        self.end_time = end_time;
        self.valid = true;
    }

    /// Get duration
    pub fn get_duration(&self) -> u64 {
        if self.valid {
            self.end_time - self.start_time
        } else {
            0
        }
    }
}

/// Boot timing analyzer
pub struct BootTimingAnalyzer {
    /// Measurements
    measurements: [Option<TimingMeasurement>; 32],
    /// Measurement count
    count: usize,
    /// Total boot time
    total_boot_time: u64,
    /// Boot start time
    boot_start_time: u64,
    /// Analysis completed
    analysis_complete: bool,
}

impl BootTimingAnalyzer {
    /// Create boot timing analyzer
    pub fn new() -> Self {
        BootTimingAnalyzer {
            measurements: [None; 32],
            count: 0,
            total_boot_time: 0,
            boot_start_time: 0,
            analysis_complete: false,
        }
    }

    /// Set boot start time
    pub fn set_boot_start_time(&mut self, time: u64) {
        self.boot_start_time = time;
    }

    /// Record phase start
    pub fn record_phase_start(&mut self, phase: BootPhase, time: u64) -> bool {
        if self.count < 32 {
            let measurement = TimingMeasurement::new(phase, time);
            self.measurements[self.count] = Some(measurement);
            self.count += 1;
            true
        } else {
            false
        }
    }

    /// Complete phase measurement
    pub fn complete_phase(&mut self, phase: BootPhase, end_time: u64) -> bool {
        // Find the matching phase measurement
        for i in (0..self.count).rev() {
            if let Some(m) = &mut self.measurements[i] {
                if m.phase == phase && !m.valid {
                    m.complete(end_time);
                    return true;
                }
            }
        }
        false
    }

    /// Get measurement by phase
    pub fn get_measurement(&self, phase: BootPhase) -> Option<&TimingMeasurement> {
        for i in 0..self.count {
            if let Some(m) = &self.measurements[i] {
                if m.phase == phase {
                    return Some(m);
                }
            }
        }
        None
    }

    /// Get measurement by index
    pub fn get_measurement_by_index(&self, index: usize) -> Option<&TimingMeasurement> {
        if index < self.count {
            self.measurements[index].as_ref()
        } else {
            None
        }
    }

    /// Calculate total boot time
    pub fn calculate_total_boot_time(&mut self) -> u64 {
        if self.count > 0 {
            if let Some(last) = self.get_measurement_by_index(self.count - 1) {
                self.total_boot_time = last.end_time - self.boot_start_time;
                self.analysis_complete = true;
            }
        }
        self.total_boot_time
    }

    /// Get total boot time
    pub fn get_total_boot_time(&self) -> u64 {
        self.total_boot_time
    }

    /// Find slowest phase
    pub fn get_slowest_phase(&self) -> Option<(BootPhase, u64)> {
        let mut slowest_phase = None;
        let mut max_duration = 0;

        for i in 0..self.count {
            if let Some(m) = &self.measurements[i] {
                if m.valid {
                    let duration = m.get_duration();
                    if duration > max_duration {
                        max_duration = duration;
                        slowest_phase = Some((m.phase, duration));
                    }
                }
            }
        }

        slowest_phase
    }

    /// Get phase percentage of total
    pub fn get_phase_percentage(&self, phase: BootPhase) -> f64 {
        if let Some(m) = self.get_measurement(phase) {
            if self.total_boot_time > 0 {
                (m.get_duration() as f64 / self.total_boot_time as f64) * 100.0
            } else {
                0.0
            }
        } else {
            0.0
        }
    }

    /// Get measurement count
    pub fn get_measurement_count(&self) -> usize {
        self.count
    }

    /// Check if analysis complete
    pub fn is_analysis_complete(&self) -> bool {
        self.analysis_complete
    }

    /// Get average phase duration
    pub fn get_average_phase_duration(&self) -> u64 {
        if self.count > 0 {
            self.total_boot_time / self.count as u64
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_phases() {
        assert_ne!(BootPhase::Firmware, BootPhase::Memory);
    }

    #[test]
    fn test_timing_measurement_creation() {
        let m = TimingMeasurement::new(BootPhase::Firmware, 0);
        assert_eq!(m.phase, BootPhase::Firmware);
        assert!(!m.valid);
    }

    #[test]
    fn test_timing_measurement_complete() {
        let mut m = TimingMeasurement::new(BootPhase::Firmware, 0);
        m.complete(100);
        assert!(m.valid);
    }

    #[test]
    fn test_timing_measurement_duration() {
        let mut m = TimingMeasurement::new(BootPhase::Firmware, 100);
        m.complete(200);
        assert_eq!(m.get_duration(), 100);
    }

    #[test]
    fn test_analyzer_creation() {
        let analyzer = BootTimingAnalyzer::new();
        assert_eq!(analyzer.get_measurement_count(), 0);
        assert!(!analyzer.is_analysis_complete());
    }

    #[test]
    fn test_record_phase_start() {
        let mut analyzer = BootTimingAnalyzer::new();
        assert!(analyzer.record_phase_start(BootPhase::Firmware, 0));
    }

    #[test]
    fn test_complete_phase() {
        let mut analyzer = BootTimingAnalyzer::new();
        analyzer.record_phase_start(BootPhase::Firmware, 0);
        assert!(analyzer.complete_phase(BootPhase::Firmware, 100));
    }

    #[test]
    fn test_get_measurement() {
        let mut analyzer = BootTimingAnalyzer::new();
        analyzer.record_phase_start(BootPhase::Memory, 100);
        analyzer.complete_phase(BootPhase::Memory, 250);
        assert!(analyzer.get_measurement(BootPhase::Memory).is_some());
    }

    #[test]
    fn test_get_measurement_by_index() {
        let mut analyzer = BootTimingAnalyzer::new();
        analyzer.record_phase_start(BootPhase::Firmware, 0);
        assert!(analyzer.get_measurement_by_index(0).is_some());
    }

    #[test]
    fn test_calculate_total_boot_time() {
        let mut analyzer = BootTimingAnalyzer::new();
        analyzer.set_boot_start_time(0);
        analyzer.record_phase_start(BootPhase::Firmware, 0);
        analyzer.complete_phase(BootPhase::Firmware, 100);
        let total = analyzer.calculate_total_boot_time();
        assert!(total > 0);
    }

    #[test]
    fn test_get_total_boot_time() {
        let mut analyzer = BootTimingAnalyzer::new();
        analyzer.set_boot_start_time(0);
        analyzer.record_phase_start(BootPhase::Firmware, 0);
        analyzer.complete_phase(BootPhase::Firmware, 100);
        analyzer.calculate_total_boot_time();
        assert_eq!(analyzer.get_total_boot_time(), 100);
    }

    #[test]
    fn test_get_slowest_phase() {
        let mut analyzer = BootTimingAnalyzer::new();
        analyzer.record_phase_start(BootPhase::Firmware, 0);
        analyzer.complete_phase(BootPhase::Firmware, 100);
        analyzer.record_phase_start(BootPhase::Memory, 100);
        analyzer.complete_phase(BootPhase::Memory, 300);
        
        let slowest = analyzer.get_slowest_phase();
        assert!(slowest.is_some());
        assert_eq!(slowest.unwrap().0, BootPhase::Memory);
    }

    #[test]
    fn test_get_phase_percentage() {
        let mut analyzer = BootTimingAnalyzer::new();
        analyzer.set_boot_start_time(0);
        analyzer.record_phase_start(BootPhase::Firmware, 0);
        analyzer.complete_phase(BootPhase::Firmware, 100);
        analyzer.calculate_total_boot_time();
        
        let pct = analyzer.get_phase_percentage(BootPhase::Firmware);
        assert!(pct > 0.0);
    }

    #[test]
    fn test_multiple_phases() {
        let mut analyzer = BootTimingAnalyzer::new();
        analyzer.set_boot_start_time(0);
        analyzer.record_phase_start(BootPhase::Firmware, 0);
        analyzer.complete_phase(BootPhase::Firmware, 100);
        analyzer.record_phase_start(BootPhase::Memory, 100);
        analyzer.complete_phase(BootPhase::Memory, 250);
        analyzer.record_phase_start(BootPhase::Cpu, 250);
        analyzer.complete_phase(BootPhase::Cpu, 350);
        
        assert_eq!(analyzer.get_measurement_count(), 3);
    }

    #[test]
    fn test_get_average_phase_duration() {
        let mut analyzer = BootTimingAnalyzer::new();
        analyzer.set_boot_start_time(0);
        analyzer.record_phase_start(BootPhase::Firmware, 0);
        analyzer.complete_phase(BootPhase::Firmware, 100);
        analyzer.calculate_total_boot_time();
        
        let avg = analyzer.get_average_phase_duration();
        assert!(avg > 0);
    }

    #[test]
    fn test_incomplete_measurement() {
        let analyzer = BootTimingAnalyzer::new();
        let m = TimingMeasurement::new(BootPhase::Firmware, 0);
        assert_eq!(m.get_duration(), 0); // duration of incomplete is 0
        assert_eq!(analyzer.get_measurement_count(), 0); // analyzer should have no measurements
    }

    #[test]
    fn test_phase_order() {
        let mut analyzer = BootTimingAnalyzer::new();
        let phases = vec![
            BootPhase::Firmware,
            BootPhase::Memory,
            BootPhase::Cpu,
            BootPhase::Devices,
        ];

        for phase in phases {
            analyzer.record_phase_start(phase, 0);
        }

        assert_eq!(analyzer.get_measurement_count(), 4);
    }

    #[test]
    fn test_multiple_same_phase_records() {
        let mut analyzer = BootTimingAnalyzer::new();
        analyzer.record_phase_start(BootPhase::Firmware, 0);
        analyzer.record_phase_start(BootPhase::Firmware, 100);
        assert_eq!(analyzer.get_measurement_count(), 2);
    }

    #[test]
    fn test_boot_timing_all_phases() {
        let mut analyzer = BootTimingAnalyzer::new();
        analyzer.set_boot_start_time(0);
        
        let mut time = 0;
        let phases = vec![
            BootPhase::Firmware,
            BootPhase::Memory,
            BootPhase::Cpu,
            BootPhase::Devices,
            BootPhase::KernelLoad,
        ];

        for phase in phases {
            analyzer.record_phase_start(phase, time);
            time += 100;
            analyzer.complete_phase(phase, time);
        }

        analyzer.calculate_total_boot_time();
        assert!(analyzer.is_analysis_complete());
    }

    #[test]
    fn test_timing_accuracy() {
        let mut m = TimingMeasurement::new(BootPhase::Cpu, 1000);
        m.complete(2500);
        assert_eq!(m.get_duration(), 1500);
    }

    #[test]
    fn test_phase_percentage_sum() {
        let mut analyzer = BootTimingAnalyzer::new();
        analyzer.set_boot_start_time(0);
        analyzer.record_phase_start(BootPhase::Firmware, 0);
        analyzer.complete_phase(BootPhase::Firmware, 50);
        analyzer.record_phase_start(BootPhase::Memory, 50);
        analyzer.complete_phase(BootPhase::Memory, 150);
        analyzer.calculate_total_boot_time();
        
        let total_pct = analyzer.get_phase_percentage(BootPhase::Firmware) 
                      + analyzer.get_phase_percentage(BootPhase::Memory);
        assert!(total_pct <= 100.0);
    }
}
