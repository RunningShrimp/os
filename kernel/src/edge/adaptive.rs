//! Adaptive Resource Management
//!
//! Provides resource-aware scheduling, energy-efficient computing,
//! thermal-aware scheduling, and DVFS control for edge devices.

use alloc::{
    collections::{BTreeMap, BTreeSet},
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;

use crate::prelude::*;
use crate::perf::{PerformanceMetric, MetricType, MetricValue};
use crate::sched::{O1Scheduler, StatsSnapshot};
use crate::subsystems::sync::Mutex;

/// Task identifier
pub type TaskId = u64;

/// CPU frequency in MHz
pub type CpuFrequencyMHz = u64;

/// CPU utilization percentage (0-100)
pub type CpuUtilPercent = u64;

/// Temperature in Celsius
pub type TemperatureCelsius = i32;

/// Power consumption in milliwatts
pub type PowerMilliwatts = u64;

/// Quality of Service policy
#[derive(Debug, Clone)]
pub struct QoSPolicy {
    /// Policy ID
    pub policy_id: String,
    /// Policy name
    pub name: String,
    /// Priority class
    pub priority_class: PriorityClass,
    /// Target latency in milliseconds
    pub target_latency_ms: u64,
    /// Maximum allowable latency
    pub max_latency_ms: u64,
    /// Minimum throughput
    pub min_throughput: u64,
    /// CPU reservation (milliCPUs)
    pub cpu_reservation: u64,
    /// CPU limit (milliCPUs)
    pub cpu_limit: u64,
    /// Memory reservation in bytes
    pub memory_reservation: u64,
    /// Memory limit in bytes
    pub memory_limit: u64,
}

/// Priority class
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PriorityClass {
    /// Critical priority (e.g., safety-critical tasks)
    Critical = 3,
    /// High priority (e.g., interactive tasks)
    High = 2,
    /// Normal priority (e.g., background tasks)
    Normal = 1,
    /// Low priority (e.g., batch jobs)
    Low = 0,
}

/// Energy policy
#[derive(Debug, Clone)]
pub struct EnergyPolicy {
    /// Policy ID
    pub policy_id: String,
    /// Policy name
    pub name: String,
    /// Power consumption target
    pub power_target_mw: Option<PowerMilliwatts>,
    /// Energy budget in milliwatt-hours
    pub energy_budget_mwh: Option<u64>,
    /// Performance mode
    pub performance_mode: PerformanceMode,
    /// Allow thermal throttling
    pub allow_thermal_throttling: bool,
    /// DVFS enabled
    pub dvfs_enabled: bool,
    /// Low power mode threshold
    pub low_power_threshold: CpuUtilPercent,
}

/// Performance mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PerformanceMode {
    /// Maximum performance
    MaxPerformance,
    /// Balanced performance/power
    Balanced,
    /// Power saving
    PowerSaving,
    /// Ultra low power
    UltraLowPower,
}

/// Thermal state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalState {
    /// Normal temperature
    Normal,
    /// Warm (approaching limit)
    Warm {
        /// Current temperature
        current_temp: TemperatureCelsius,
        /// Threshold temperature
        threshold: TemperatureCelsius,
    },
    /// Hot (throttling recommended)
    Hot {
        /// Current temperature
        current_temp: TemperatureCelsius,
        /// Threshold temperature
        threshold: TemperatureCelsius,
    },
    /// Critical (immediate action required)
    Critical {
        /// Current temperature
        current_temp: TemperatureCelsius,
        /// Critical threshold
        critical_threshold: TemperatureCelsius,
    },
}

impl ThermalState {
    /// Check if thermal action is needed
    pub fn needs_action(&self) -> bool {
        matches!(self, ThermalState::Hot { .. } | ThermalState::Critical { .. })
    }

    /// Get recommended CPU throttle percentage (0-100)
    pub fn get_throttle_percent(&self) -> u64 {
        match self {
            ThermalState::Normal => 0,
            ThermalState::Warm { .. } => 10,
            ThermalState::Hot { .. } => 30,
            ThermalState::Critical { .. } => 50,
        }
    }
}

/// DVFS controller
pub struct DVFSController {
    /// Controller ID
    controller_id: u64,
    /// Current frequency
    current_frequency: AtomicU64,
    /// Minimum frequency
    min_frequency: CpuFrequencyMHz,
    /// Maximum frequency
    max_frequency: CpuFrequencyMHz,
    /// Supported frequencies
    supported_frequencies: Vec<CpuFrequencyMHz>,
    /// DVFS enabled
    enabled: AtomicBool,
    /// Governor policy
    governor: DVFSGovernor,
}

/// DVFS governor policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DVFSGovernor {
    /// Performance (max frequency)
    Performance,
    /// Powersave (min frequency)
    Powersave,
    /// Ondemand (scale based on load)
    Ondemand,
    /// Conservative (gradual scaling)
    Conservative,
    /// Conservative (schedutil)
    Schedutil,
}

impl DVFSController {
    /// Create a new DVFS controller
    pub fn new(
        min_freq: CpuFrequencyMHz,
        max_freq: CpuFrequencyMHz,
        governor: DVFSGovernor,
    ) -> Self {
        // Generate supported frequencies (in 100MHz steps)
        let mut supported_frequencies = Vec::new();
        let mut freq = min_freq;
        while freq <= max_freq {
            supported_frequencies.push(freq);
            freq += 100;
        }

        Self {
            controller_id: nos_api::event::get_time_ns(),
            current_frequency: AtomicU64::new(max_freq),
            min_frequency: min_freq,
            max_frequency: max_freq,
            supported_frequencies,
            enabled: AtomicBool::new(true),
            governor,
        }
    }

    /// Set frequency
    pub fn set_frequency(&self, frequency: CpuFrequencyMHz) -> Result<()> {
        if frequency < self.min_frequency || frequency > self.max_frequency {
            return Err(nos_api::Error::InvalidArgument);
        }

        self.current_frequency.store(frequency, Ordering::Relaxed);

        crate::println!("[dvfs] CPU frequency set to {} MHz", frequency);

        Ok(())
    }

    /// Get current frequency
    pub fn get_frequency(&self) -> CpuFrequencyMHz {
        self.current_frequency.load(Ordering::Relaxed)
    }

    /// Scale frequency based on utilization
    pub fn scale_frequency(&self, utilization: CpuUtilPercent) -> Result<()> {
        if !self.enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        let new_frequency = match self.governor {
            DVFSGovernor::Performance => self.max_frequency,
            DVFSGovernor::Powersave => self.min_frequency,
            DVFSGovernor::Ondemand => {
                if utilization > 80 {
                    self.max_frequency
                } else if utilization > 60 {
                    self.max_frequency * 3 / 4
                } else if utilization > 40 {
                    self.max_frequency / 2
                } else {
                    self.min_frequency
                }
            }
            DVFSGovernor::Conservative => {
                let current = self.get_frequency();
                let step = (self.max_frequency - self.min_frequency) / 10;

                if utilization > 80 {
                    (current + step).min(self.max_frequency)
                } else if utilization < 20 {
                    current.saturating_sub(step).max(self.min_frequency)
                } else {
                    current
                }
            }
            DVFSGovernor::Schedutil => {
                // Similar to ondemand but more responsive
                if utilization > 70 {
                    self.max_frequency
                } else if utilization > 30 {
                    self.min_frequency + (self.max_frequency - self.min_frequency) * utilization as u64 / 100
                } else {
                    self.min_frequency
                }
            }
        };

        self.set_frequency(new_frequency)
    }

    /// Enable/disable DVFS
    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
        crate::println!("[dvfs] DVFS {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Get supported frequencies
    pub fn get_supported_frequencies(&self) -> &[CpuFrequencyMHz] {
        &self.supported_frequencies
    }
}

/// Power state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerState {
    /// Active (CPU running at full speed)
    Active,
    /// Light sleep (some cores asleep)
    LightSleep,
    /// Deep sleep (most cores asleep)
    DeepSleep,
    /// Standby (ready to wake quickly)
    Standby,
    /// Suspend (to RAM)
    Suspend,
}

/// Resource allocation
#[derive(Debug, Clone)]
pub struct ResourceAllocation {
    /// Allocation ID
    pub allocation_id: u64,
    /// Task ID
    pub task_id: TaskId,
    /// CPU cores allocated
    pub cpu_cores: usize,
    /// CPU frequency MHz
    pub cpu_frequency: CpuFrequencyMHz,
    /// Memory bytes allocated
    pub memory_bytes: u64,
    /// Priority class
    pub priority: PriorityClass,
    /// QoS policy ID
    pub qos_policy_id: Option<String>,
    /// Energy policy ID
    pub energy_policy_id: Option<String>,
}

/// Adaptive scheduler
pub struct AdaptiveScheduler {
    /// Scheduler ID
    scheduler_id: u64,
    /// DVFS controller
    dvfs: DVFSController,
    /// QoS policies
    qos_policies: Mutex<BTreeMap<String, QoSPolicy>>,
    /// Energy policies
    energy_policies: Mutex<BTreeMap<String, EnergyPolicy>>,
    /// Resource allocations
    allocations: Mutex<BTreeMap<TaskId, ResourceAllocation>>,
    /// Thermal state
    thermal_state: Mutex<ThermalState>,
    /// Current CPU utilization
    cpu_utilization: AtomicU64,
    /// Current power consumption
    power_consumption: AtomicU64,
    /// Adaptive scheduling enabled
    adaptive_enabled: AtomicBool,
    /// Performance counters
    performance_metrics: Mutex<BTreeMap<String, PerformanceMetric>>,
}

impl AdaptiveScheduler {
    /// Create a new adaptive scheduler
    pub fn new(min_freq: CpuFrequencyMHz, max_freq: CpuFrequencyMHz) -> Self {
        Self {
            scheduler_id: nos_api::event::get_time_ns(),
            dvfs: DVFSController::new(min_freq, max_freq, DVFSGovernor::Ondemand),
            qos_policies: Mutex::new(BTreeMap::new()),
            energy_policies: Mutex::new(BTreeMap::new()),
            allocations: Mutex::new(BTreeMap::new()),
            thermal_state: Mutex::new(ThermalState::Normal),
            cpu_utilization: AtomicU64::new(0),
            power_consumption: AtomicU64::new(0),
            adaptive_enabled: AtomicBool::new(true),
            performance_metrics: Mutex::new(BTreeMap::new()),
        }
    }

    /// Add QoS policy
    pub fn add_qos_policy(&self, policy: QoSPolicy) -> Result<()> {
        let mut policies = self.qos_policies.lock();
        policies.insert(policy.policy_id.clone(), policy);
        Ok(())
    }

    /// Add energy policy
    pub fn add_energy_policy(&self, policy: EnergyPolicy) -> Result<()> {
        let mut policies = self.energy_policies.lock();
        policies.insert(policy.policy_id.clone(), policy);
        Ok(())
    }

    /// Allocate resources for task
    pub fn allocate_resources(&self, task_id: TaskId, qos_policy_id: Option<String>) -> Result<ResourceAllocation> {
        let allocation_id = nos_api::event::get_time_ns();

        // Determine allocation parameters
        let (cpu_cores, cpu_freq, memory_bytes, priority) = if let Some(policy_id) = qos_policy_id {
            let policies = self.qos_policies.lock();
            let policy = policies.get(&policy_id)
                .ok_or(nos_api::Error::NotFound)?;

            (
                (policy.cpu_reservation + 999) / 1000, // Convert milliCPUs to cores
                self.dvfs.max_frequency,
                policy.memory_reservation,
                policy.priority_class,
            )
        } else {
            // Default allocation
            (1, self.dvfs.min_frequency, 128 * 1024 * 1024, PriorityClass::Normal)
        };

        let allocation = ResourceAllocation {
            allocation_id,
            task_id,
            cpu_cores,
            cpu_frequency: cpu_freq,
            memory_bytes,
            priority,
            qos_policy_id,
            energy_policy_id: None,
        };

        {
            let mut allocations = self.allocations.lock();
            allocations.insert(task_id, allocation.clone());
        }

        crate::println!("[adaptive] Allocated resources for task {}: {} cores, {} MHz, {} MB memory",
            task_id, cpu_cores, cpu_freq, memory_bytes / (1024 * 1024));

        Ok(allocation)
    }

    /// Deallocate resources
    pub fn deallocate_resources(&self, task_id: TaskId) -> Result<()> {
        let mut allocations = self.allocations.lock();
        allocations.remove(&task_id)
            .ok_or(nos_api::Error::NotFound)?;

        crate::println!("[adaptive] Deallocated resources for task {}", task_id);

        Ok(())
    }

    /// Update CPU utilization
    pub fn update_cpu_utilization(&self, utilization: CpuUtilPercent) {
        self.cpu_utilization.store(utilization.min(100), Ordering::Relaxed);

        // Adapt DVFS based on utilization
        let _ = self.dvfs.scale_frequency(utilization);
    }

    /// Update thermal state
    pub fn update_thermal_state(&self, temperature: TemperatureCelsius) {
        let new_state = if temperature < 60 {
            ThermalState::Normal
        } else if temperature < 75 {
            ThermalState::Warm {
                current_temp: temperature,
                threshold: 75,
            }
        } else if temperature < 85 {
            ThermalState::Hot {
                current_temp: temperature,
                threshold: 85,
            }
        } else {
            ThermalState::Critical {
                current_temp: temperature,
                critical_threshold: 85,
            }
        };

        let mut state = self.thermal_state.lock();
        *state = new_state;

        // Take thermal action if needed
        if new_state.needs_action() {
            let throttle = new_state.get_throttle_percent();
            crate::println!("[adaptive] Thermal throttling: {}% (temp: {}°C)", throttle, temperature);

            // Reduce frequency
            let current_freq = self.dvfs.get_frequency();
            let new_freq = current_freq * (100 - throttle) / 100;
            let _ = self.dvfs.set_frequency(new_freq.max(self.dvfs.min_frequency));
        }
    }

    /// Update power consumption
    pub fn update_power_consumption(&self, power: PowerMilliwatts) {
        self.power_consumption.store(power, Ordering::Relaxed);
    }

    /// Get thermal state
    pub fn get_thermal_state(&self) -> ThermalState {
        let state = self.thermal_state.lock();
        *state
    }

    /// Get DVFS controller
    pub fn get_dvfs(&self) -> &DVFSController {
        &self.dvfs
    }

    /// Enable/disable adaptive scheduling
    pub fn set_adaptive_enabled(&self, enabled: bool) {
        self.adaptive_enabled.store(enabled, Ordering::Relaxed);
        crate::println!("[adaptive] Adaptive scheduling {}", if enabled { "enabled" } else { "disabled" });
    }

    /// Get scheduler statistics
    pub fn get_stats(&self) -> AdaptiveSchedulerStats {
        let allocations = self.allocations.lock();
        let qos_policies = self.qos_policies.lock();
        let energy_policies = self.energy_policies.lock();

        let mut priority_counts = [0usize; 4];
        for allocation in allocations.values() {
            priority_counts[allocation.priority as usize] += 1;
        }

        AdaptiveSchedulerStats {
            cpu_utilization: self.cpu_utilization.load(Ordering::Relaxed),
            power_consumption_mw: self.power_consumption.load(Ordering::Relaxed),
            current_frequency: self.dvfs.get_frequency(),
            thermal_state: self.get_thermal_state(),
            active_allocations: allocations.len(),
            qos_policy_count: qos_policies.len(),
            energy_policy_count: energy_policies.len(),
            critical_tasks: priority_counts[3],
            high_tasks: priority_counts[2],
            normal_tasks: priority_counts[1],
            low_tasks: priority_counts[0],
        }
    }

    /// Optimize resource allocation
    pub fn optimize_allocation(&self) -> Result<()> {
        if !self.adaptive_enabled.load(Ordering::Relaxed) {
            return Ok(());
        }

        crate::println!("[adaptive] Optimizing resource allocation");

        // Get current system state
        let cpu_util = self.cpu_utilization.load(Ordering::Relaxed);
        let thermal_state = self.get_thermal_state();
        let power_consumption = self.power_consumption.load(Ordering::Relaxed);

        // Update performance metrics
        {
            let mut metrics = self.performance_metrics.lock();
            metrics.insert(
                "adaptive_cpu_utilization".to_string(),
                PerformanceMetric {
                    name: "adaptive_cpu_utilization".to_string(),
                    metric_type: MetricType::Gauge,
                    value: MetricValue::Integer(cpu_util as i64),
                    unit: "percent".to_string(),
                    timestamp: nos_api::event::get_time_ns(),
                    tags: BTreeMap::new(),
                },
            );

            metrics.insert(
                "adaptive_power_consumption".to_string(),
                PerformanceMetric {
                    name: "adaptive_power_consumption".to_string(),
                    metric_type: MetricType::Gauge,
                    value: MetricValue::Integer(power_consumption as i64),
                    unit: "milliwatts".to_string(),
                    timestamp: nos_api::event::get_time_ns(),
                    tags: BTreeMap::new(),
                },
            );
        }

        // Apply thermal throttling if needed
        if thermal_state.needs_action() {
            let throttle = thermal_state.get_throttle_percent();
            crate::println!("[adaptive] Applying thermal throttle: {}%", throttle);
        }

        // Adjust DVFS based on utilization and power
        if power_consumption > 5000 { // > 5W
            crate::println!("[adaptive] High power consumption, reducing frequency");
            let current = self.dvfs.get_frequency();
            let _ = self.dvfs.set_frequency((current * 3 / 4).max(self.dvfs.min_frequency));
        }

        Ok(())
    }
}

/// Adaptive scheduler statistics
#[derive(Debug, Clone)]
pub struct AdaptiveSchedulerStats {
    /// Current CPU utilization percentage
    pub cpu_utilization: CpuUtilPercent,
    /// Current power consumption in milliwatts
    pub power_consumption_mw: PowerMilliwatts,
    /// Current CPU frequency in MHz
    pub current_frequency: CpuFrequencyMHz,
    /// Current thermal state
    pub thermal_state: ThermalState,
    /// Number of active allocations
    pub active_allocations: usize,
    /// Number of QoS policies
    pub qos_policy_count: usize,
    /// Number of energy policies
    pub energy_policy_count: usize,
    /// Critical priority tasks
    pub critical_tasks: usize,
    /// High priority tasks
    pub high_tasks: usize,
    /// Normal priority tasks
    pub normal_tasks: usize,
    /// Low priority tasks
    pub low_tasks: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dvfs_controller() {
        let controller = DVFSController::new(800, 2400, DVFSGovernor::Ondemand);
        assert_eq!(controller.get_frequency(), 2400);

        controller.set_frequency(1600).unwrap();
        assert_eq!(controller.get_frequency(), 1600);
    }

    #[test]
    fn test_dvfs_scaling() {
        let controller = DVFSController::new(800, 2400, DVFSGovernor::Ondemand);

        // High utilization should increase frequency
        controller.scale_frequency(90).unwrap();
        assert_eq!(controller.get_frequency(), 2400);

        // Low utilization should decrease frequency
        controller.scale_frequency(20).unwrap();
        assert_eq!(controller.get_frequency(), 800);
    }

    #[test]
    fn test_thermal_state() {
        let state = ThermalState::Normal;
        assert!(!state.needs_action());
        assert_eq!(state.get_throttle_percent(), 0);

        let state = ThermalState::Hot {
            current_temp: 80,
            threshold: 85,
        };
        assert!(state.needs_action());
        assert_eq!(state.get_throttle_percent(), 30);
    }

    #[test]
    fn test_adaptive_scheduler() {
        let scheduler = AdaptiveScheduler::new(800, 2400);

        let stats = scheduler.get_stats();
        assert_eq!(stats.active_allocations, 0);
        assert_eq!(stats.qos_policy_count, 0);
    }

    #[test]
    fn test_qos_policy() {
        let policy = QoSPolicy {
            policy_id: "test-policy".to_string(),
            name: "Test Policy".to_string(),
            priority_class: PriorityClass::High,
            target_latency_ms: 10,
            max_latency_ms: 50,
            min_throughput: 1000,
            cpu_reservation: 500,
            cpu_limit: 1000,
            memory_reservation: 128 * 1024 * 1024,
            memory_limit: 256 * 1024 * 1024,
        };

        assert_eq!(policy.priority_class, PriorityClass::High);
        assert_eq!(policy.cpu_reservation, 500);
    }

    #[test]
    fn test_energy_policy() {
        let policy = EnergyPolicy {
            policy_id: "test-energy".to_string(),
            name: "Test Energy Policy".to_string(),
            power_target_mw: Some(3000),
            energy_budget_mwh: Some(10000),
            performance_mode: PerformanceMode::Balanced,
            allow_thermal_throttling: true,
            dvfs_enabled: true,
            low_power_threshold: 20,
        };

        assert_eq!(policy.performance_mode, PerformanceMode::Balanced);
        assert!(policy.dvfs_enabled);
    }
}
