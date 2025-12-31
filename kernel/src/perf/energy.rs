//! Energy Model and Power Management
//!
//! This module provides comprehensive energy accounting and power management including:
//! - Energy model and power estimation (Intel RAPL - Running Average Power Limit)
//! - Energy-aware scheduling (EAS)
//! - Power capping (RAPL power limits)
//! - Thermal throttling (PROCHOT, thermal shutdown)
//! - Power policy engine (performance vs power)
//! - Energy preferences (balance_performance, balance_power, power_save)
//! - Power monitoring and alerts
//!
//! # Overview
//!
//! This module tracks and controls power consumption at the system level, enabling
//! fine-grained power management and thermal control.
//!
//! # Architecture
//!
//! ```text
//! +------------------------------------------------------------+
//! |                    Energy Manager                           |
//! +------------------------------------------------------------+
//! |  +--------------+  +--------------+  +-------------------+  |
//! |  | Intel RAPL   |  | Power Model  |  | Thermal Monitor   |  |
//! |  |              |  |              |  |                   |  |
//! |  +------+-------+  +------+-------+  +---------+---------+  |
//! |         |                 |                     |            |
//! |         +-----------------+---------------------+            |
//! |                           |                                 |
//! |                           v                                 |
//! |              +---------------------------+                 |
//! |              |   Power Policy Engine     |                 |
//! |              |  - Performance/Power      |                 |
//! |              |  - Power Capping          |                 |
//! |              +-----------+---------------+                 |
//! |                          |                                  |
//! |                          v                                  |
//! |    +---------------------------------------------------+    |
//! |    |              Power Domains                         |    |
//! |    |  Package -> DRAM -> GPU -> Platform                |    |
//! |    +---------------------------------------------------+    |
//! +------------------------------------------------------------+
//! ```

#![allow(dead_code)]
#![allow(unused_variables)]

use crate::prelude::*;
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, Ordering};

/// Power domain types
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PowerDomain {
    /// Package power (entire CPU package)
    Package,
    /// DRAM power
    Dram,
    /// GPU power (integrated)
    Gpu,
    /// Platform power (entire SoC)
    Platform,
}

impl PowerDomain {
    /// Get domain name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Package => "package",
            Self::Dram => "dram",
            Self::Gpu => "gpu",
            Self::Platform => "platform",
        }
    }

    /// Get domain description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Package => "CPU package power domain",
            Self::Dram => "DRAM memory power domain",
            Self::Gpu => "Integrated GPU power domain",
            Self::Platform => "Platform/SoC power domain",
        }
    }
}

/// Power limit configuration
#[derive(Debug, Clone)]
pub struct PowerLimit {
    /// Power limit in microwatts
    pub limit_uw: u32,
    /// Time window in microseconds
    pub time_window_us: u32,
    /// Enable flag
    pub enabled: bool,
    /// Clamp enable (prevent exceeding limit)
    pub clamp_enabled: bool,
}

impl PowerLimit {
    /// Create new power limit
    pub fn new(limit_uw: u32, time_window_us: u32) -> Self {
        Self {
            limit_uw,
            time_window_us,
            enabled: false,
            clamp_enabled: false,
        }
    }

    /// Enable power limit
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable power limit
    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

/// Energy statistics
#[derive(Debug, Clone)]
pub struct EnergyStats {
    /// Energy consumed (microjoules)
    pub energy_uj: u64,
    /// Power consumption (microwatts)
    pub power_uw: u32,
    /// Maximum power seen (microwatts)
    pub max_power_uw: u32,
    /// Accumulated energy (microjoules)
    pub accumulated_energy_uj: u64,
    /// Last update time (μs)
    pub last_update_us: u64,
}

impl Default for EnergyStats {
    fn default() -> Self {
        Self {
            energy_uj: 0,
            power_uw: 0,
            max_power_uw: 0,
            accumulated_energy_uj: 0,
            last_update_us: 0,
        }
    }
}

impl EnergyStats {
    /// Update energy reading
    pub fn update(&mut self, energy_uj: u64, current_time_us: u64) {
        let time_delta = current_time_us.saturating_sub(self.last_update_us);

        if time_delta > 0 && self.last_update_us > 0 {
            // Calculate average power: E = P * t, so P = E / t
            // Energy in μj, time in μs, so power in μW
            let energy_delta = energy_uj.saturating_sub(self.energy_uj);
            let power_uw = (energy_delta * 1_000_000) / time_delta;

            self.power_uw = power_uw as u32;
            self.max_power_uw = self.max_power_uw.max(self.power_uw);
        }

        self.energy_uj = energy_uj;
        self.accumulated_energy_uj += energy_uj;
        self.last_update_us = current_time_us;
    }
}

/// Thermal status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalStatus {
    /// Normal temperature
    Normal,
    /// Temperature elevated
    Elevated,
    /// Approaching thermal limit
    High,
    /// Thermal throttling active
    Throttling,
    /// Critical temperature (PROCHOT)
    Critical,
    /// Thermal shutdown imminent
    Shutdown,
}

impl ThermalStatus {
    /// Get status name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Elevated => "elevated",
            Self::High => "high",
            Self::Throttling => "throttling",
            Self::Critical => "critical",
            Self::Shutdown => "shutdown",
        }
    }

    /// Check if throttling is active
    pub fn is_throttling(&self) -> bool {
        matches!(self, Self::Throttling | Self::Critical | Self::Shutdown)
    }
}

/// Thermal zone information
#[derive(Debug, Clone)]
pub struct ThermalZone {
    /// Zone ID
    pub id: u32,
    /// Zone name
    pub name: String,
    /// Current temperature (millidegrees Celsius)
    pub current_temp_mc: i32,
    /// Trip points (temperature thresholds)
    pub trip_points: Vec<TripPoint>,
    /// Current thermal status
    pub status: ThermalStatus,
}

impl ThermalZone {
    /// Create new thermal zone
    pub fn new(id: u32, name: String) -> Self {
        Self {
            id,
            name,
            current_temp_mc: 0,
            trip_points: Vec::new(),
            status: ThermalStatus::Normal,
        }
    }

    /// Add trip point
    pub fn add_trip_point(&mut self, temp_mc: i32, trip_type: TripType) {
        self.trip_points.push(TripPoint { temp_mc, trip_type });
    }

    /// Update temperature
    pub fn update_temp(&mut self, temp_mc: i32) {
        self.current_temp_mc = temp_mc;

        // Check trip points
        self.status = ThermalStatus::Normal;
        for trip in &self.trip_points {
            if temp_mc >= trip.temp_mc {
                match trip.trip_type {
                    TripType::Passive => {
                        self.status = ThermalStatus::High;
                    }
                    TripType::Active => {
                        self.status = ThermalStatus::Throttling;
                    }
                    TripType::Critical => {
                        self.status = ThermalStatus::Critical;
                    }
                }
            }
        }
    }
}

/// Thermal trip point
#[derive(Debug, Clone, Copy)]
pub struct TripPoint {
    /// Temperature threshold (millidegrees Celsius)
    pub temp_mc: i32,
    /// Trip type
    pub trip_type: TripType,
}

/// Trip type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TripType {
    /// Passive cooling (throttling)
    Passive,
    /// Active cooling (fan)
    Active,
    /// Critical temperature (shutdown)
    Critical,
}

/// Power preference
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerPreference {
    /// Maximum performance
    Performance,
    /// Balanced performance
    BalancePerformance,
    /// Balanced power
    BalancePower,
    /// Power saving
    PowerSave,
}

impl PowerPreference {
    /// Get preference name
    pub fn name(&self) -> &'static str {
        match self {
            Self::Performance => "performance",
            Self::BalancePerformance => "balance_performance",
            Self::BalancePower => "balance_power",
            Self::PowerSave => "power_save",
        }
    }

    /// Get description
    pub fn description(&self) -> &'static str {
        match self {
            Self::Performance => "Maximum performance, ignore power consumption",
            Self::BalancePerformance => "Favor performance over power savings",
            Self::BalancePower => "Favor power savings with reasonable performance",
            Self::PowerSave => "Maximum power savings, minimize performance",
        }
    }
}

/// RAPL (Running Average Power Limit) domain
#[derive(Debug)]
pub struct RaplDomain {
    /// Domain type
    pub domain_type: PowerDomain,
    /// Energy statistics
    pub stats: Mutex<EnergyStats>,
    /// Power limit
    pub power_limit: Mutex<PowerLimit>,
    /// Maximum power (microwatts)
    pub max_power_uw: u32,
    /// Minimum power (microwatts)
    pub min_power_uw: u32,
    /// Energy unit (microjoules per unit)
    pub energy_unit_uj: f64,
    /// Power unit (microwatts per unit)
    pub power_unit_uw: f64,
}

impl RaplDomain {
    /// Create new RAPL domain
    pub fn new(domain_type: PowerDomain) -> Self {
        let (max_power, min_power) = match domain_type {
            PowerDomain::Package => (150_000_000, 5_000_000), // 5W - 150W
            PowerDomain::Dram => (30_000_000, 1_000_000),    // 1W - 30W
            PowerDomain::Gpu => (50_000_000, 2_000_000),     // 2W - 50W
            PowerDomain::Platform => (200_000_000, 10_000_000), // 10W - 200W
        };

        Self {
            domain_type,
            stats: Mutex::new(EnergyStats::default()),
            power_limit: Mutex::new(PowerLimit::new(max_power, 1_000_000)), // 1 second window
            max_power_uw: max_power,
            min_power_uw: min_power,
            energy_unit_uj: 1.0,
            power_unit_uw: 1.0,
        }
    }

    /// Read energy counter
    pub fn read_energy(&self) -> Result<u64, EnergyError> {
        // In real implementation, would read MSR or ACPI
        // For now, return simulated value
        let energy = self.simulated_energy();
        Ok(energy)
    }

    /// Simulated energy reading
    fn simulated_energy(&self) -> u64 {
        let stats = self.stats.lock();
        let base_energy = stats.energy_uj;
        drop(stats);

        // Simulate energy accumulation based on time
        let time_ns = nos_api::event::get_time_ns();
        base_energy + (time_ns / 1000) // Rough approximation
    }

    /// Update energy statistics
    pub fn update_stats(&self) -> Result<(), EnergyError> {
        let energy = self.read_energy()?;
        let current_time = nos_api::event::get_time_ns() / 1000; // μs

        let mut stats = self.stats.lock();
        stats.update(energy, current_time);

        Ok(())
    }

    /// Set power limit
    pub fn set_power_limit(&self, limit_uw: u32, time_window_us: u32) -> Result<(), EnergyError> {
        if limit_uw < self.min_power_uw || limit_uw > self.max_power_uw {
            return Err(EnergyError::InvalidPowerLimit);
        }

        let mut power_limit = self.power_limit.lock();
        power_limit.limit_uw = limit_uw;
        power_limit.time_window_us = time_window_us;
        power_limit.enable();

        // In real implementation, would write to RAPL MSR

        Ok(())
    }

    /// Get power limit
    pub fn get_power_limit(&self) -> PowerLimit {
        self.power_limit.lock().clone()
    }

    /// Enable power limit
    pub fn enable_power_limit(&self) -> Result<(), EnergyError> {
        let mut power_limit = self.power_limit.lock();
        power_limit.enable();
        Ok(())
    }

    /// Disable power limit
    pub fn disable_power_limit(&self) -> Result<(), EnergyError> {
        let mut power_limit = self.power_limit.lock();
        power_limit.disable();
        Ok(())
    }

    /// Get energy consumed
    pub fn get_energy_consumed(&self) -> u64 {
        self.stats.lock().energy_uj
    }

    /// Get current power
    pub fn get_power(&self) -> u32 {
        self.stats.lock().power_uw
    }
}

/// Energy model for CPU capacity
#[derive(Debug, Clone)]
pub struct EnergyModel {
    /// CPU capacity (performance score)
    pub capacity: u32,
    /// Power at max frequency (microwatts)
    pub power_max_uw: u32,
    /// Power at min frequency (microwatts)
    pub power_min_uw: u32,
    /// Frequency scaling factor
    pub freq_factor: f64,
}

impl EnergyModel {
    /// Create new energy model
    pub fn new(capacity: u32, power_max_uw: u32, power_min_uw: u32) -> Self {
        Self {
            capacity,
            power_max_uw,
            power_min_uw,
            freq_factor: 1.0,
        }
    }

    /// Calculate power for utilization
    pub fn calculate_power(&self, utilization: u8) -> u32 {
        // Power = P_min + (P_max - P_min) * (util / 100)^2
        // Square factor accounts for voltage-frequency relationship
        let util_ratio = (utilization as f64) / 100.0;
        let power_range = self.power_max_uw - self.power_min_uw;

        self.power_min_uw + (power_range as f64 * util_ratio * util_ratio) as u32
    }

    /// Calculate energy for task
    pub fn calculate_energy(&self, utilization: u8, duration_us: u64) -> u64 {
        let power_uw = self.calculate_power(utilization);
        // Energy = Power * time (μW * μs = μJ)
        (power_uw as u64 * duration_us) / 1_000_000 // Convert to joules
    }
}

/// Power policy engine
#[derive(Debug)]
pub struct PowerPolicyEngine {
    /// Current power preference
    pub preference: Mutex<PowerPreference>,
    /// Power limits enabled
    pub limits_enabled: AtomicBool,
    /// Thermal throttling enabled
    pub thermal_throttling_enabled: AtomicBool,
}

impl PowerPolicyEngine {
    /// Create new power policy engine
    pub fn new() -> Self {
        Self {
            preference: Mutex::new(PowerPreference::BalancePerformance),
            limits_enabled: AtomicBool::new(true),
            thermal_throttling_enabled: AtomicBool::new(true),
        }
    }

    /// Set power preference
    pub fn set_preference(&self, pref: PowerPreference) {
        *self.preference.lock() = pref;
        log::info!("Power preference set to: {}", pref.name());
    }

    /// Get power preference
    pub fn get_preference(&self) -> PowerPreference {
        *self.preference.lock()
    }

    /// Enable power limits
    pub fn enable_limits(&self) {
        self.limits_enabled.store(true, Ordering::Relaxed);
    }

    /// Disable power limits
    pub fn disable_limits(&self) {
        self.limits_enabled.store(false, Ordering::Relaxed);
    }

    /// Check if limits are enabled
    pub fn limits_enabled(&self) -> bool {
        self.limits_enabled.load(Ordering::Relaxed)
    }

    /// Enable thermal throttling
    pub fn enable_thermal_throttling(&self) {
        self.thermal_throttling_enabled.store(true, Ordering::Relaxed);
    }

    /// Disable thermal throttling
    pub fn disable_thermal_throttling(&self) {
        self.thermal_throttling_enabled.store(false, Ordering::Relaxed);
    }

    /// Evaluate policy and return action
    pub fn evaluate(&self, thermal_status: ThermalStatus) -> PolicyAction {
        let pref = self.get_preference();

        match thermal_status {
            ThermalStatus::Normal => match pref {
                PowerPreference::Performance => PolicyAction::MaxPerformance,
                PowerPreference::BalancePerformance => PolicyAction::HighPerformance,
                PowerPreference::BalancePower => PolicyAction::Balanced,
                PowerPreference::PowerSave => PolicyAction::PowerSave,
            },
            ThermalStatus::Elevated => {
                if pref == PowerPreference::Performance {
                    PolicyAction::HighPerformance
                } else {
                    PolicyAction::Balanced
                }
            }
            ThermalStatus::High => PolicyAction::ModerateThrottle,
            ThermalStatus::Throttling => PolicyAction::Throttle,
            ThermalStatus::Critical => PolicyAction::SevereThrottle,
            ThermalStatus::Shutdown => PolicyAction::EmergencyShutdown,
        }
    }
}

/// Policy action
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyAction {
    /// Maximum performance
    MaxPerformance,
    /// High performance
    HighPerformance,
    /// Balanced
    Balanced,
    /// Power saving
    PowerSave,
    /// Moderate throttling
    ModerateThrottle,
    /// Throttling
    Throttle,
    /// Severe throttling
    SevereThrottle,
    /// Emergency shutdown
    EmergencyShutdown,
}

/// Energy manager
pub struct EnergyManager {
    /// RAPL domains
    pub rapl_domains: BTreeMap<PowerDomain, Arc<RaplDomain>>,
    /// Thermal zones
    pub thermal_zones: BTreeMap<u32, Arc<Mutex<ThermalZone>>>,
    /// Energy models (per CPU)
    pub energy_models: BTreeMap<u32, EnergyModel>,
    /// Power policy engine
    pub policy_engine: Arc<PowerPolicyEngine>,
    /// Enabled flag
    pub enabled: AtomicBool,
    /// Alert callbacks
    pub alert_callbacks: Mutex<Vec<AlertCallback>>,
}

impl core::fmt::Debug for EnergyManager {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("EnergyManager")
            .field("rapl_domains", &self.rapl_domains)
            .field("thermal_zones", &self.thermal_zones)
            .field("energy_models", &self.energy_models)
            .field("policy_engine", &self.policy_engine)
            .field("enabled", &self.enabled)
            .field("alert_callbacks", &"<callbacks>")
            .finish()
    }
}

impl EnergyManager {
    /// Create new energy manager
    pub fn new() -> Self {
        let mut rapl_domains = BTreeMap::new();

        // Create standard RAPL domains
        rapl_domains.insert(PowerDomain::Package, Arc::new(RaplDomain::new(PowerDomain::Package)));
        rapl_domains.insert(PowerDomain::Dram, Arc::new(RaplDomain::new(PowerDomain::Dram)));
        rapl_domains.insert(PowerDomain::Platform, Arc::new(RaplDomain::new(PowerDomain::Platform)));

        Self {
            rapl_domains,
            thermal_zones: BTreeMap::new(),
            energy_models: BTreeMap::new(),
            policy_engine: Arc::new(PowerPolicyEngine::new()),
            enabled: AtomicBool::new(false),
            alert_callbacks: Mutex::new(Vec::new()),
        }
    }

    /// Initialize manager
    pub fn init(&mut self) -> Result<(), EnergyError> {
        if self.enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Create thermal zones with Mutex for interior mutability
        let cpu_zone = Arc::new(Mutex::new(ThermalZone::new(0, "cpu".to_string())));
        {
            let mut zone = cpu_zone.lock();
            zone.add_trip_point(80_000, TripType::Passive); // 80°C
            zone.add_trip_point(90_000, TripType::Active);   // 90°C
            zone.add_trip_point(100_000, TripType::Critical); // 100°C
        }
        self.thermal_zones.insert(0, cpu_zone);

        // Create energy models for CPUs
        for cpu in 0..4 {
            let model = EnergyModel::new(cpu, 30_000_000, 5_000_000);
            self.energy_models.insert(cpu, model);
        }

        self.enabled.store(true, Ordering::Relaxed);
        log::info!("Energy manager initialized");

        Ok(())
    }

    /// Get RAPL domain
    pub fn get_rapl_domain(&self, domain: PowerDomain) -> Option<Arc<RaplDomain>> {
        self.rapl_domains.get(&domain).cloned()
    }

    /// Get thermal zone
    pub fn get_thermal_zone(&self, zone_id: u32) -> Option<Arc<Mutex<ThermalZone>>> {
        self.thermal_zones.get(&zone_id).cloned()
    }

    /// Update all energy statistics
    pub fn update_stats(&self) -> Result<(), EnergyError> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        // Update all RAPL domains
        for domain in self.rapl_domains.values() {
            domain.update_stats()?;
        }

        Ok(())
    }

    /// Get power limit
    pub fn get_power_limit(&self, domain: PowerDomain) -> Option<PowerLimit> {
        let rapl_domain = self.get_rapl_domain(domain)?;
        Some(rapl_domain.get_power_limit())
    }

    /// Set power limit
    pub fn set_power_limit(&self, domain: PowerDomain, limit_uw: u32) -> Result<(), EnergyError> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Err(EnergyError::Disabled);
        }

        let rapl_domain = self
            .get_rapl_domain(domain)
            .ok_or(EnergyError::DomainNotFound)?;

        // Default time window: 1 second
        rapl_domain.set_power_limit(limit_uw, 1_000_000)
    }

    /// Get energy consumed
    pub fn get_energy_consumed(&self, domain: PowerDomain) -> Result<u64, EnergyError> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Err(EnergyError::Disabled);
        }

        let rapl_domain = self
            .get_rapl_domain(domain)
            .ok_or(EnergyError::DomainNotFound)?;

        Ok(rapl_domain.get_energy_consumed())
    }

    /// Get current power
    pub fn get_current_power(&self, domain: PowerDomain) -> Result<u32, EnergyError> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Err(EnergyError::Disabled);
        }

        let rapl_domain = self
            .get_rapl_domain(domain)
            .ok_or(EnergyError::DomainNotFound)?;

        Ok(rapl_domain.get_power())
    }

    /// Get thermal status
    pub fn get_thermal_status(&self) -> ThermalStatus {
        let zone = self.thermal_zones.get(&0);
        if let Some(zone) = zone {
            zone.lock().status
        } else {
            ThermalStatus::Normal
        }
    }

    /// Update thermal zone temperature
    pub fn update_temperature(&self, zone_id: u32, temp_mc: i32) -> Result<(), EnergyError> {
        let zone = self
            .thermal_zones
            .get(&zone_id)
            .ok_or(EnergyError::ZoneNotFound)?;

        let mut zone_guard = zone.lock();
        let old_status = zone_guard.status;
        zone_guard.update_temp(temp_mc);

        // Check for status changes
        if zone_guard.status != old_status {
            self.notify_alerts(zone_id, zone_guard.status);
        }

        Ok(())
    }

    /// Notify alert callbacks
    fn notify_alerts(&self, zone_id: u32, status: ThermalStatus) {
        let callbacks = self.alert_callbacks.lock();
        for callback in callbacks.iter() {
            callback.call(zone_id, status);
        }
    }

    /// Add alert callback
    pub fn add_alert_callback(&self, callback: AlertCallback) {
        self.alert_callbacks.lock().push(callback);
    }

    /// Set power preference
    pub fn set_power_preference(&self, pref: PowerPreference) -> Result<(), EnergyError> {
        self.policy_engine.set_preference(pref);
        Ok(())
    }

    /// Get power preference
    pub fn get_power_preference(&self) -> PowerPreference {
        self.policy_engine.get_preference()
    }

    /// Get energy model for CPU
    pub fn get_energy_model(&self, cpu_id: u32) -> Option<EnergyModel> {
        self.energy_models.get(&cpu_id).cloned()
    }

    /// Calculate optimal CPU for task (energy-aware scheduling)
    pub fn calculate_optimal_cpu(&self, utilization: u8) -> Option<u32> {
        if self.energy_models.is_empty() {
            return None;
        }

        let pref = self.get_power_preference();
        let mut best_cpu = 0;
        let mut best_score = i64::MIN;

        for (&cpu_id, model) in &self.energy_models {
            let energy = model.calculate_energy(utilization, 1000); // 1ms
            let capacity = model.capacity;

            // Score combines energy and capacity
            let score = match pref {
                PowerPreference::Performance => {
                    (capacity as i64 * 2) - (energy as i64 / 1000)
                }
                PowerPreference::BalancePerformance => {
                    (capacity as i64) - (energy as i64 / 500)
                }
                PowerPreference::BalancePower => {
                    (capacity as i64 / 2) - (energy as i64 / 100)
                }
                PowerPreference::PowerSave => -(energy as i64),
            };

            if score > best_score {
                best_score = score;
                best_cpu = cpu_id;
            }
        }

        Some(best_cpu)
    }

    /// Get statistics
    pub fn get_stats(&self) -> EnergyManagerStats {
        let mut domain_stats = BTreeMap::new();

        for (domain, rapl) in &self.rapl_domains {
            let stats = rapl.stats.lock();
            domain_stats.insert(
                *domain,
                DomainEnergyStats {
                    energy_uj: stats.energy_uj,
                    power_uw: stats.power_uw,
                    max_power_uw: stats.max_power_uw,
                },
            );
        }

        EnergyManagerStats {
            domain_stats,
            thermal_status: self.get_thermal_status(),
            power_preference: self.get_power_preference(),
        }
    }
}

/// Alert callback
#[derive(Clone)]
pub struct AlertCallback {
    /// Callback function
    pub callback: Arc<dyn Fn(u32, ThermalStatus) + Send + Sync>,
}

impl AlertCallback {
    /// Create new callback
    pub fn new<F>(callback: F) -> Self
    where
        F: Fn(u32, ThermalStatus) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }

    /// Call the callback
    pub fn call(&self, zone_id: u32, status: ThermalStatus) {
        (self.callback)(zone_id, status);
    }
}

impl core::fmt::Debug for AlertCallback {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "AlertCallback")
    }
}

/// Domain energy statistics
#[derive(Debug, Clone)]
pub struct DomainEnergyStats {
    /// Energy consumed (microjoules)
    pub energy_uj: u64,
    /// Current power (microwatts)
    pub power_uw: u32,
    /// Maximum power (microwatts)
    pub max_power_uw: u32,
}

/// Energy manager statistics
#[derive(Debug, Clone)]
pub struct EnergyManagerStats {
    /// Per-domain statistics
    pub domain_stats: BTreeMap<PowerDomain, DomainEnergyStats>,
    /// Thermal status
    pub thermal_status: ThermalStatus,
    /// Power preference
    pub power_preference: PowerPreference,
}

/// Energy errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyError {
    /// Domain not found
    DomainNotFound,
    /// Thermal zone not found
    ZoneNotFound,
    /// Invalid power limit
    InvalidPowerLimit,
    /// Hardware error
    HardwareError,
    /// Disabled
    Disabled,
    /// Not supported
    NotSupported,
}

impl core::fmt::Display for EnergyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DomainNotFound => write!(f, "Power domain not found"),
            Self::ZoneNotFound => write!(f, "Thermal zone not found"),
            Self::InvalidPowerLimit => write!(f, "Invalid power limit"),
            Self::HardwareError => write!(f, "Hardware error"),
            Self::Disabled => write!(f, "Energy management is disabled"),
            Self::NotSupported => write!(f, "Operation not supported"),
        }
    }
}

/// Global energy manager
static GLOBAL_ENERGY_MANAGER: Mutex<Option<EnergyManager>> = Mutex::new(None);

/// Initialize energy subsystem
pub fn init_energy() -> Result<(), EnergyError> {
    let mut manager = GLOBAL_ENERGY_MANAGER.lock();

    if manager.is_some() {
        return Ok(());
    }

    let mut energy_manager = EnergyManager::new();
    energy_manager.init()?;

    *manager = Some(energy_manager);

    log::info!("Energy subsystem initialized");
    Ok(())
}

/// Get power limit
pub fn get_power_limit(domain: PowerDomain) -> Option<PowerLimit> {
    let manager = GLOBAL_ENERGY_MANAGER.lock();
    manager.as_ref()?.get_power_limit(domain)
}

/// Set power limit
pub fn set_power_limit(domain: PowerDomain, limit_uw: u32) -> Result<(), EnergyError> {
    let manager = GLOBAL_ENERGY_MANAGER.lock();
    let manager = manager.as_ref().ok_or(EnergyError::Disabled)?;
    manager.set_power_limit(domain, limit_uw)
}

/// Get energy consumed
pub fn get_energy_consumed(domain: PowerDomain) -> Result<u64, EnergyError> {
    let manager = GLOBAL_ENERGY_MANAGER.lock();
    let manager = manager.as_ref().ok_or(EnergyError::Disabled)?;
    manager.get_energy_consumed(domain)
}

/// Get thermal status
pub fn get_thermal_status() -> ThermalStatus {
    let manager = GLOBAL_ENERGY_MANAGER.lock();
    if let Some(manager) = manager.as_ref() {
        manager.get_thermal_status()
    } else {
        ThermalStatus::Normal
    }
}

/// Set power preference
pub fn set_power_preference(pref: PowerPreference) -> Result<(), EnergyError> {
    let manager = GLOBAL_ENERGY_MANAGER.lock();
    let manager = manager.as_ref().ok_or(EnergyError::Disabled)?;
    manager.set_power_preference(pref)
}

/// Get power preference
pub fn get_power_preference() -> PowerPreference {
    let manager = GLOBAL_ENERGY_MANAGER.lock();
    if let Some(manager) = manager.as_ref() {
        manager.get_power_preference()
    } else {
        PowerPreference::BalancePerformance
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_power_domain() {
        assert_eq!(PowerDomain::Package.name(), "package");
        assert_eq!(PowerDomain::Dram.name(), "dram");
    }

    #[test]
    fn test_power_limit() {
        let mut limit = PowerLimit::new(50_000_000, 1_000_000);
        assert!(!limit.enabled);
        assert_eq!(limit.limit_uw, 50_000_000);

        limit.enable();
        assert!(limit.enabled);
    }

    #[test]
    fn test_thermal_zone() {
        let mut zone = ThermalZone::new(0, "cpu".to_string());
        zone.add_trip_point(80_000, TripType::Passive);

        assert_eq!(zone.status, ThermalStatus::Normal);

        zone.update_temp(85_000);
        assert_eq!(zone.status, ThermalStatus::High);
    }

    #[test]
    fn test_energy_model() {
        let model = EnergyModel::new(1000, 30_000_000, 5_000_000);

        let power = model.calculate_power(50);
        assert!(power > 5_000_000 && power < 30_000_000);

        let energy = model.calculate_energy(50, 1_000_000);
        assert!(energy > 0);
    }

    #[test]
    fn test_rapl_domain() {
        let domain = RaplDomain::new(PowerDomain::Package);
        assert_eq!(domain.domain_type, PowerDomain::Package);
        assert_eq!(domain.max_power_uw, 150_000_000);
    }

    #[test]
    fn test_power_policy() {
        let engine = PowerPolicyEngine::new();

        let action = engine.evaluate(ThermalStatus::Normal);
        assert_eq!(action, PolicyAction::HighPerformance);

        engine.set_preference(PowerPreference::PowerSave);
        let action = engine.evaluate(ThermalStatus::Normal);
        assert_eq!(action, PolicyAction::PowerSave);
    }
}
