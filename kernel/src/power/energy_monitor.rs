//! Energy Monitoring System
//!
//! This module implements power and energy monitoring for the system.
//! It provides:
//! - Real-time power consumption monitoring
//! - Energy consumption tracking (millijoules)
//! - Power consumption statistics and analysis
//! - Power policy recommendations
//!
//! ## Metrics
//!
//! - **Instantaneous Power**: Current power draw (milliwatts)
//! - **Energy**: Total energy consumed (millijoules)
//! - **Power Efficiency**: Performance per watt
//! - **Battery**: Battery state and charge level
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::power::energy_monitor::{EnergyMonitor, PowerMetric};
//!
//! // Get energy monitor
//! let monitor = EnergyMonitor::get()?;
//!
//! // Get current power consumption
//! let power = monitor.get_current_power();
//!
//! // Get energy consumption over time window
//! let energy = monitor.get_energy_consumption(duration_ms)?;
//! ```

use crate::error::UnifiedError;
use spin::{Mutex, Once};
use alloc::vec::Vec;

/// Power in milliwatts (mW)
pub type PowerMw = u32;

/// Energy in millijoules (mJ = mW * ms)
pub type EnergyMj = u64;

/// Voltage in millivolts (mV)
pub type VoltageMv = u32;

/// Current in milliamps (mA)
pub type CurrentMa = u32;

/// Temperature in millidegrees Celsius
pub type TemperatureMc = i32;

/// Time in milliseconds
pub type TimeMs = u64;

/// Power metric type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerMetricType {
    /// CPU power
    CPU,
    /// GPU power
    GPU,
    /// Memory power
    Memory,
    /// Storage power
    Storage,
    /// Network power
    Network,
    /// Display power
    Display,
    /// Wireless power
    Wireless,
    /// Other device power
    Other,
}

impl PowerMetricType {
    /// Get metric name
    pub fn name(&self) -> &'static str {
        match self {
            PowerMetricType::CPU => "CPU",
            PowerMetricType::GPU => "GPU",
            PowerMetricType::Memory => "Memory",
            PowerMetricType::Storage => "Storage",
            PowerMetricType::Network => "Network",
            PowerMetricType::Display => "Display",
            PowerMetricType::Wireless => "Wireless",
            PowerMetricType::Other => "Other",
        }
    }
}

/// Power metric reading
#[derive(Debug, Clone)]
pub struct PowerMetric {
    /// Metric type
    pub metric_type: PowerMetricType,
    /// Current power consumption (milliwatts)
    pub power: PowerMw,
    /// Energy consumed since last reading (millijoules)
    pub energy: EnergyMj,
    /// Timestamp (milliseconds since boot)
    pub timestamp: TimeMs,
}

impl PowerMetric {
    /// Create new power metric
    pub fn new(metric_type: PowerMetricType, power: PowerMw, timestamp: TimeMs) -> Self {
        PowerMetric {
            metric_type,
            power,
            energy: 0,
            timestamp,
        }
    }

    /// Create metric with energy value
    pub fn with_energy(
        metric_type: PowerMetricType,
        power: PowerMw,
        energy: EnergyMj,
        timestamp: TimeMs,
    ) -> Self {
        PowerMetric {
            metric_type,
            power,
            energy,
            timestamp,
        }
    }
}

/// Battery state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryState {
    /// Battery is discharging
    Discharging,
    /// Battery is charging
    Charging,
    /// Battery is full
    Full,
    /// Battery is not present
    Absent,
}

/// Battery information
#[derive(Debug, Clone)]
pub struct BatteryInfo {
    /// Battery state
    pub state: BatteryState,
    /// Current capacity (percentage 0-100)
    pub capacity: u8,
    /// Remaining capacity (milliamp-hours)
    pub remaining_mah: u32,
    /// Full capacity (milliamp-hours)
    pub full_mah: u32,
    /// Current voltage (millivolts)
    pub voltage_mv: VoltageMv,
    /// Current draw (milliamps, positive = charging, negative = discharging)
    pub current_ma: i32,
    /// Temperature (millidegrees Celsius)
    pub temperature_mc: TemperatureMc,
    /// Time to empty (minutes, if discharging)
    pub time_to_empty_min: Option<u32>,
    /// Time to full (minutes, if charging)
    pub time_to_full_min: Option<u32>,
}

impl BatteryInfo {
    /// Create new battery info
    pub fn new() -> Self {
        BatteryInfo {
            state: BatteryState::Absent,
            capacity: 0,
            remaining_mah: 0,
            full_mah: 0,
            voltage_mv: 0,
            current_ma: 0,
            temperature_mc: 0,
            time_to_empty_min: None,
            time_to_full_min: None,
        }
    }

    /// Check if battery is present
    pub fn is_present(&self) -> bool {
        self.state != BatteryState::Absent
    }

    /// Get remaining energy (millijoules)
    pub fn remaining_energy_mj(&self) -> EnergyMj {
        // Energy = Voltage * Current * Time
        // For mAh, convert to mWh then to mJ
        let mwh = self.remaining_mah as u64 * self.voltage_mv as u64 / 1000;
        mwh * 3600 // Convert to millijoules
    }

    /// Get full energy (millijoules)
    pub fn full_energy_mj(&self) -> EnergyMj {
        let mwh = self.full_mah as u64 * self.voltage_mv as u64 / 1000;
        mwh * 3600
    }
}

/// Power statistics
#[derive(Debug, Clone)]
pub struct PowerStats {
    /// Total energy consumed (millijoules)
    pub total_energy_mj: EnergyMj,
    /// Average power consumption (milliwatts)
    pub average_power_mw: PowerMw,
    /// Peak power consumption (milliwatts)
    pub peak_power_mw: PowerMw,
    /// Minimum power consumption (milliwatts)
    pub min_power_mw: PowerMw,
    /// Number of samples
    pub sample_count: u64,
    /// Start time (milliseconds)
    pub start_time: TimeMs,
    /// End time (milliseconds)
    pub end_time: TimeMs,
}

impl PowerStats {
    /// Create new statistics
    pub fn new() -> Self {
        PowerStats {
            total_energy_mj: 0,
            average_power_mw: 0,
            peak_power_mw: 0,
            min_power_mw: u32::MAX,
            sample_count: 0,
            start_time: 0,
            end_time: 0,
        }
    }

    /// Update statistics with new reading
    pub fn update(&mut self, power: PowerMw, timestamp: TimeMs, duration_ms: u64) {
        // Update energy (millijoules = milliwatts * milliseconds)
        self.total_energy_mj += power as u64 * duration_ms;

        // Update peak and minimum
        if power > self.peak_power_mw {
            self.peak_power_mw = power;
        }
        if power < self.min_power_mw {
            self.min_power_mw = power;
        }

        // Update sample count
        self.sample_count += 1;

        // Update timestamps
        if self.start_time == 0 {
            self.start_time = timestamp;
        }
        self.end_time = timestamp;
    }

    /// Calculate average power
    pub fn calculate_average(&mut self, duration_ms: u64) {
        if duration_ms > 0 {
            self.average_power_mw = (self.total_energy_mj / duration_ms) as PowerMw;
        }
    }

    /// Get power efficiency score (0-100)
    pub fn efficiency_score(&self) -> u8 {
        if self.average_power_mw == 0 {
            return 100;
        }

        // Simple metric: lower peak-to-average ratio is better
        let ratio = self.peak_power_mw as f64 / self.average_power_mw as f64;
        let score = (100.0 / ratio).min(100.0).max(0.0);
        score as u8
    }
}

/// Power policy recommendation
#[derive(Debug, Clone)]
pub struct PowerPolicyRecommendation {
    /// Recommended CPU governor
    pub cpu_governor: Option<&'static str>,
    /// Recommended screen brightness (0-100)
    pub screen_brightness: Option<u8>,
    /// Recommended power state for devices
    pub device_power_saving: bool,
    /// Reason for recommendation
    pub reason: &'static str,
}

impl PowerPolicyRecommendation {
    /// Create new recommendation
    pub fn new(reason: &'static str) -> Self {
        PowerPolicyRecommendation {
            cpu_governor: None,
            screen_brightness: None,
            device_power_saving: false,
            reason,
        }
    }

    /// Create performance recommendation
    pub fn performance() -> Self {
        PowerPolicyRecommendation {
            cpu_governor: Some("performance"),
            screen_brightness: Some(100),
            device_power_saving: false,
            reason: "Maximum performance mode",
        }
    }

    /// Create balanced recommendation
    pub fn balanced() -> Self {
        PowerPolicyRecommendation {
            cpu_governor: Some("ondemand"),
            screen_brightness: Some(70),
            device_power_saving: false,
            reason: "Balanced performance and power",
        }
    }

    /// Create power saving recommendation
    pub fn power_saving() -> Self {
        PowerPolicyRecommendation {
            cpu_governor: Some("powersave"),
            screen_brightness: Some(30),
            device_power_saving: true,
            reason: "Maximum battery life",
        }
    }
}

/// Energy monitor
#[derive(Debug)]
pub struct EnergyMonitor {
    /// Current power readings per metric
    current_metrics: Vec<PowerMetric>,
    /// Overall statistics
    stats: PowerStats,
    /// Battery information
    battery: BatteryInfo,
    /// Historical power readings (last 100 samples)
    history: Vec<(TimeMs, PowerMw)>,
    /// Last update time
    last_update: TimeMs,
}

impl EnergyMonitor {
    /// Create new energy monitor
    pub fn new() -> Self {
        let current_metrics = vec![
            PowerMetric::new(PowerMetricType::CPU, 0, 0),
            PowerMetric::new(PowerMetricType::GPU, 0, 0),
            PowerMetric::new(PowerMetricType::Memory, 0, 0),
            PowerMetric::new(PowerMetricType::Storage, 0, 0),
            PowerMetric::new(PowerMetricType::Network, 0, 0),
            PowerMetric::new(PowerMetricType::Display, 0, 0),
        ];

        EnergyMonitor {
            current_metrics,
            stats: PowerStats::new(),
            battery: BatteryInfo::new(),
            history: Vec::new(),
            last_update: 0,
        }
    }

    /// Get current power consumption (total)
    pub fn get_current_power(&self) -> PowerMw {
        self.current_metrics.iter().map(|m| m.power).sum()
    }

    /// Get power consumption by metric type
    pub fn get_metric_power(&self, metric_type: PowerMetricType) -> PowerMw {
        self.current_metrics
            .iter()
            .find(|m| m.metric_type == metric_type)
            .map(|m| m.power)
            .unwrap_or(0)
    }

    /// Update power metric
    pub fn update_metric(&mut self, metric_type: PowerMetricType, power: PowerMw) {
        let timestamp = self.get_timestamp();
        let duration_ms = if self.last_update > 0 {
            timestamp - self.last_update
        } else {
            0
        };

        if let Some(metric) = self
            .current_metrics
            .iter_mut()
            .find(|m| m.metric_type == metric_type)
        {
            metric.power = power;
            metric.energy = power as u64 * duration_ms;
            metric.timestamp = timestamp;
        }

        // Update statistics
        let total_power = self.get_current_power();
        self.stats.update(total_power, timestamp, duration_ms);

        // Update history (keep last 100 samples)
        self.history.push((timestamp, total_power));
        if self.history.len() > 100 {
            self.history.remove(0);
        }

        self.last_update = timestamp;
    }

    /// Get battery information
    pub fn get_battery_info(&self) -> &BatteryInfo {
        &self.battery
    }

    /// Update battery information
    pub fn update_battery(&mut self, battery: BatteryInfo) {
        self.battery = battery;
    }

    /// Get total energy consumption
    pub fn get_total_energy(&self) -> EnergyMj {
        self.stats.total_energy_mj
    }

    /// Get energy consumption over time window
    pub fn get_energy_consumption(&self, duration_ms: u64) -> EnergyMj {
        let now = self.get_timestamp();
        let start_time = now.saturating_sub(duration_ms);

        // Sum energy from history samples within window
        self.history
            .iter()
            .filter(|(timestamp, _)| *timestamp >= start_time)
            .map(|(_, power)| *power as u64)
            .sum::<u64>()
            / self.history.len().max(1) as u64
            * duration_ms
    }

    /// Get statistics
    pub fn get_stats(&self) -> &PowerStats {
        &self.stats
    }

    /// Get power consumption trend (last 10 samples)
    pub fn get_trend(&self) -> Vec<(TimeMs, PowerMw)> {
        self.history.iter().rev().take(10).rev().cloned().collect()
    }

    /// Estimate battery life (minutes)
    pub fn estimate_battery_life(&self) -> Option<u32> {
        if !self.battery.is_present() || self.battery.remaining_mah == 0 {
            return None;
        }

        let current_power_mw = self.get_current_power();
        if current_power_mw == 0 {
            return None;
        }

        // Calculate remaining energy
        let remaining_energy_mj = self.battery.remaining_energy_mj();

        // Battery life = energy / power
        // Result in milliseconds, convert to minutes
        let life_ms = remaining_energy_mj / current_power_mw as u64;
        Some((life_ms / 60000) as u32)
    }

    /// Get power policy recommendation
    pub fn get_recommendation(&self) -> PowerPolicyRecommendation {
        // Check battery state
        if self.battery.is_present() {
            match self.battery.capacity {
                0..=20 => return PowerPolicyRecommendation::power_saving(),
                21..=50 => return PowerPolicyRecommendation::balanced(),
                _ => {}
            }
        }

        // Check AC power status (assume on AC if no battery)
        if !self.battery.is_present() {
            return PowerPolicyRecommendation::performance();
        }

        // Check current power consumption
        let current_power = self.get_current_power();
        if current_power > 10000 {
            // High power consumption
            PowerPolicyRecommendation::power_saving()
        } else if current_power < 3000 {
            // Low power consumption
            PowerPolicyRecommendation::performance()
        } else {
            PowerPolicyRecommendation::balanced()
        }
    }

    /// Get power consumption breakdown
    pub fn get_breakdown(&self) -> Vec<(PowerMetricType, PowerMw)> {
        self.current_metrics
            .iter()
            .map(|m| (m.metric_type, m.power))
            .collect()
    }

    /// Calculate power efficiency (performance per watt)
    pub fn calculate_efficiency(&self) -> f64 {
        if self.stats.average_power_mw == 0 {
            return 0.0;
        }

        // Simple metric: efficiency score based on stats
        let score = self.stats.efficiency_score();
        score as f64 / 100.0
    }

    /// Get timestamp (milliseconds since boot)
    fn get_timestamp(&self) -> TimeMs {
        // In a real implementation, this would read from a timer
        0
    }
}

/// Global energy monitor instance
static ENERGY_MONITOR: Once<Mutex<EnergyMonitor>> = Once::new();

/// Initialize the energy monitor
pub fn init() -> Result<(), UnifiedError> {
    let monitor = EnergyMonitor::new();
    ENERGY_MONITOR.call_once(|| Mutex::new(monitor));
    Ok(())
}

/// Get the global energy monitor
pub fn get() -> Option<&'static Mutex<EnergyMonitor>> {
    ENERGY_MONITOR.get()
}

/// Get current power consumption
pub fn get_current_power() -> PowerMw {
    if let Some(monitor) = get() {
        monitor.lock().get_current_power()
    } else {
        0
    }
}

/// Update power metric
pub fn update_metric(metric_type: PowerMetricType, power: PowerMw) {
    if let Some(monitor) = get() {
        monitor.lock().update_metric(metric_type, power);
    }
}

/// Get battery information
pub fn get_battery_info() -> BatteryInfo {
    if let Some(monitor) = get() {
        monitor.lock().get_battery_info().clone()
    } else {
        BatteryInfo::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_monitor_creation() {
        let monitor = EnergyMonitor::new();
        assert_eq!(monitor.get_current_power(), 0);
    }

    #[test]
    fn test_power_metrics() {
        let mut monitor = EnergyMonitor::new();

        monitor.update_metric(PowerMetricType::CPU, 5000);
        monitor.update_metric(PowerMetricType::Memory, 2000);

        assert_eq!(monitor.get_metric_power(PowerMetricType::CPU), 5000);
        assert_eq!(monitor.get_metric_power(PowerMetricType::Memory), 2000);
        assert_eq!(monitor.get_current_power(), 7000);
    }

    #[test]
    fn test_battery_info() {
        let mut battery = BatteryInfo::new();
        battery.state = BatteryState::Discharging;
        battery.capacity = 80;
        battery.remaining_mah = 4000;
        battery.full_mah = 5000;
        battery.voltage_mv = 3700;

        assert!(battery.is_present());
        assert_eq!(battery.capacity, 80);

        let remaining_energy = battery.remaining_energy_mj();
        assert!(remaining_energy > 0);
    }

    #[test]
    fn test_statistics() {
        let mut stats = PowerStats::new();
        stats.update(1000, 0, 100);
        stats.update(2000, 100, 100);
        stats.update(1500, 200, 100);

        stats.calculate_average(300);

        assert_eq!(stats.peak_power_mw, 2000);
        assert_eq!(stats.min_power_mw, 1000);
        assert_eq!(stats.sample_count, 3);
    }

    #[test]
    fn test_recommendations() {
        let monitor = EnergyMonitor::new();

        let perf = PowerPolicyRecommendation::performance();
        assert_eq!(perf.cpu_governor, Some("performance"));

        let balanced = PowerPolicyRecommendation::balanced();
        assert_eq!(balanced.cpu_governor, Some("ondemand"));

        let saving = PowerPolicyRecommendation::power_saving();
        assert_eq!(saving.cpu_governor, Some("powersave"));
    }

    #[test]
    fn test_battery_life_estimation() {
        let mut monitor = EnergyMonitor::new();

        let mut battery = BatteryInfo::new();
        battery.state = BatteryState::Discharging;
        battery.capacity = 100;
        battery.remaining_mah = 5000;
        battery.full_mah = 5000;
        battery.voltage_mv = 3700;

        monitor.update_battery(battery);
        monitor.update_metric(PowerMetricType::CPU, 5000);

        let life = monitor.estimate_battery_life();
        assert!(life.is_some());
        assert!(life.unwrap() > 0);
    }

    #[test]
    fn test_power_breakdown() {
        let mut monitor = EnergyMonitor::new();

        monitor.update_metric(PowerMetricType::CPU, 5000);
        monitor.update_metric(PowerMetricType::GPU, 3000);
        monitor.update_metric(PowerMetricType::Memory, 2000);

        let breakdown = monitor.get_breakdown();
        assert_eq!(breakdown.len(), 6); // All metrics
        assert_eq!(breakdown[0].1, 5000); // CPU
    }

    #[test]
    fn test_efficiency_score() {
        let mut stats = PowerStats::new();
        stats.update(1000, 0, 100);
        stats.update(1500, 100, 100);
        stats.calculate_average(200);

        let score = stats.efficiency_score();
        assert!(score <= 100);
        assert!(score > 0);
    }
}
