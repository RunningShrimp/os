//! CPU Frequency Management Framework
//!
//! This module implements dynamic CPU frequency scaling similar to Linux's cpufreq subsystem.
//! It provides:
//! - Frequency table management
//! - Governor implementations (performance, powersave, ondemand)
//! - Transition notifications and statistics
//! - Per-CPU and cluster-based frequency control
//!
//! ## Governors
//!
//! - **Performance**: Always uses maximum frequency
//! - **Powersave**: Always uses minimum frequency
//! - **Ondemand**: Dynamically adjusts based on CPU load
//! - **Conservative**: Gradually adjusts based on CPU load
//!
//! ## Example
//!
//! ```rust,ignore
//! use kernel::power::cpufreq::{CpuFreq, GovernorType};
//!
//! // Initialize CPU frequency manager
//! let mut cpufreq = CpuFreq::new(0)?;
//!
//! // Set governor
//! cpufreq.set_governor(GovernorType::Ondemand)?;
//!
//! // Adjust frequency based on load
//! cpufreq.update(75)?; // 75% CPU load
//! ```

use crate::error::UnifiedError;
use spin::{Mutex, Once};
use alloc::vec::Vec;

/// CPU frequency in MHz
pub type Frequency = u32;

/// CPU load percentage (0-100)
pub type LoadPercent = u8;

/// CPU identifier
pub type CpuId = u32;

/// Transition latency in nanoseconds
pub type LatencyNs = u64;

/// CPU frequency manager instance
static CPU_FREQ_MANAGER: Once<Mutex<CpuFreqManager>> = Once::new();

/// Initialize the global CPU frequency manager
pub fn init() -> Result<(), UnifiedError> {
    let manager = CpuFreqManager::new()?;
    CPU_FREQ_MANAGER.call_once(|| Mutex::new(manager));
    Ok(())
}

/// Get the global CPU frequency manager
pub fn get_manager() -> Option<&'static Mutex<CpuFreqManager>> {
    CPU_FREQ_MANAGER.get()
}

/// CPU frequency scaling governor type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GovernorType {
    /// Always use maximum frequency
    Performance,
    /// Always use minimum frequency
    Powersave,
    /// Dynamically adjust based on CPU load
    Ondemand {
        /// Load threshold to increase frequency (default: 80%)
        up_threshold: LoadPercent,
        /// Load threshold to decrease frequency (default: 20%)
        down_threshold: LoadPercent,
        /// Minimum time before frequency change (microseconds)
        sampling_rate: u32,
    },
    /// Gradually adjust based on CPU load
    Conservative {
        /// Load threshold to increase frequency (default: 80%)
        up_threshold: LoadPercent,
        /// Load threshold to decrease frequency (default: 20%)
        down_threshold: LoadPercent,
        /// Frequency adjustment step (MHz)
        step: Frequency,
    },
    /// Userspace governor (manual control)
    Userspace,
}

impl GovernorType {
    /// Get default configuration for ondemand governor
    pub fn ondemand_default() -> Self {
        GovernorType::Ondemand {
            up_threshold: 80,
            down_threshold: 20,
            sampling_rate: 10000, // 10ms
        }
    }

    /// Get default configuration for conservative governor
    pub fn conservative_default() -> Self {
        GovernorType::Conservative {
            up_threshold: 80,
            down_threshold: 20,
            step: 100, // 100MHz
        }
    }
}

/// CPU frequency transition event
#[derive(Debug, Clone)]
pub struct FreqTransition {
    /// Old frequency in MHz
    pub old_freq: Frequency,
    /// New frequency in MHz
    pub new_freq: Frequency,
    /// CPU identifier
    pub cpu_id: CpuId,
    /// Transition time (nanoseconds since boot)
    pub timestamp: u64,
    /// Transition latency
    pub latency: LatencyNs,
}

/// Statistics for CPU frequency transitions
#[derive(Debug, Clone)]
pub struct FreqStats {
    /// Total number of transitions
    pub total_transitions: u64,
    /// Time spent at each frequency
    pub time_in_state: Vec<(Frequency, u64)>,
    /// Last transition time
    pub last_transition: Option<FreqTransition>,
}

impl FreqStats {
    /// Create new frequency statistics
    pub fn new(frequencies: &[Frequency]) -> Self {
        let time_in_state = frequencies.iter().map(|&f| (f, 0)).collect();
        FreqStats {
            total_transitions: 0,
            time_in_state,
            last_transition: None,
        }
    }

    /// Record a frequency transition
    pub fn record_transition(&mut self, transition: FreqTransition) {
        self.total_transitions += 1;
        self.last_transition = Some(transition);
    }

    /// Update time in current state
    pub fn update_time(&mut self, freq: Frequency, duration_ns: u64) {
        if let Some(entry) = self.time_in_state.iter_mut().find(|(f, _)| *f == freq) {
            entry.1 += duration_ns;
        }
    }
}

/// CPU frequency scaling information
#[derive(Debug, Clone)]
pub struct FreqTableEntry {
    /// Frequency in MHz
    pub frequency: Frequency,
    /// Transition latency to this frequency (nanoseconds)
    pub latency: LatencyNs,
    /// Voltage at this frequency (millivolts)
    pub voltage: Option<u32>,
}

impl FreqTableEntry {
    /// Create new frequency table entry
    pub fn new(frequency: Frequency, latency: LatencyNs) -> Self {
        FreqTableEntry {
            frequency,
            latency,
            voltage: None,
        }
    }

    /// Create new frequency table entry with voltage
    pub fn with_voltage(frequency: Frequency, latency: LatencyNs, voltage: u32) -> Self {
        FreqTableEntry {
            frequency,
            latency,
            voltage: Some(voltage),
        }
    }
}

/// CPU frequency scaling device
#[derive(Debug)]
pub struct CpuFreq {
    /// CPU identifier
    cpu_id: CpuId,
    /// Current frequency
    cur_freq: Frequency,
    /// Minimum frequency
    min_freq: Frequency,
    /// Maximum frequency
    max_freq: Frequency,
    /// Frequency table
    freq_table: Vec<FreqTableEntry>,
    /// Current governor
    governor: GovernorType,
    /// Statistics
    stats: FreqStats,
    /// Transition latency
    transition_latency: LatencyNs,
    /// Last update time
    last_update: Option<u64>,
}

impl Clone for CpuFreq {
    fn clone(&self) -> Self {
        Self {
            cpu_id: self.cpu_id,
            cur_freq: self.cur_freq,
            min_freq: self.min_freq,
            max_freq: self.max_freq,
            freq_table: self.freq_table.clone(),
            governor: self.governor,
            stats: self.stats.clone(),
            transition_latency: self.transition_latency,
            last_update: self.last_update,
        }
    }
}

impl CpuFreq {
    /// Create new CPU frequency scaling device
    pub fn new(
        cpu_id: CpuId,
        freq_table: Vec<FreqTableEntry>,
        transition_latency: LatencyNs,
    ) -> Result<Self, UnifiedError> {
        if freq_table.is_empty() {
            return Err(UnifiedError::Other("Frequency table cannot be empty".into()));
        }

        let min_freq = freq_table.iter().map(|e| e.frequency).min().unwrap();
        let max_freq = freq_table.iter().map(|e| e.frequency).max().unwrap();
        let frequencies: Vec<Frequency> = freq_table.iter().map(|e| e.frequency).collect();

        Ok(CpuFreq {
            cpu_id,
            cur_freq: min_freq,
            min_freq,
            max_freq,
            freq_table,
            governor: GovernorType::Powersave,
            stats: FreqStats::new(&frequencies),
            transition_latency,
            last_update: None,
        })
    }

    /// Get CPU identifier
    pub fn cpu_id(&self) -> CpuId {
        self.cpu_id
    }

    /// Get current frequency
    pub fn cur_freq(&self) -> Frequency {
        self.cur_freq
    }

    /// Get minimum frequency
    pub fn min_freq(&self) -> Frequency {
        self.min_freq
    }

    /// Get maximum frequency
    pub fn max_freq(&self) -> Frequency {
        self.max_freq
    }

    /// Get frequency table
    pub fn freq_table(&self) -> &[FreqTableEntry] {
        &self.freq_table
    }

    /// Get current governor
    pub fn governor(&self) -> &GovernorType {
        &self.governor
    }

    /// Get statistics
    pub fn stats(&self) -> &FreqStats {
        &self.stats
    }

    /// Set minimum frequency
    pub fn set_min_freq(&mut self, freq: Frequency) -> Result<(), UnifiedError> {
        if freq < self.min_freq || freq > self.max_freq {
            return Err(UnifiedError::Other("Frequency out of range".into()));
        }
        self.min_freq = freq;
        if self.cur_freq < freq {
            self.set_frequency(freq)?;
        }
        Ok(())
    }

    /// Set maximum frequency
    pub fn set_max_freq(&mut self, freq: Frequency) -> Result<(), UnifiedError> {
        if freq < self.min_freq || freq > self.max_freq {
            return Err(UnifiedError::Other("Frequency out of range".into()));
        }
        self.max_freq = freq;
        if self.cur_freq > freq {
            self.set_frequency(freq)?;
        }
        Ok(())
    }

    /// Set governor
    pub fn set_governor(&mut self, governor: GovernorType) -> Result<(), UnifiedError> {
        self.governor = governor;

        // Apply governor immediately
        match governor {
            GovernorType::Performance => {
                self.set_frequency(self.max_freq)?;
            }
            GovernorType::Powersave => {
                self.set_frequency(self.min_freq)?;
            }
            GovernorType::Ondemand { .. } | GovernorType::Conservative { .. } => {
                // Will be updated by load
            }
            GovernorType::Userspace => {
                // Manual control, no action needed
            }
        }

        Ok(())
    }

    /// Set frequency directly
    pub fn set_frequency(&mut self, freq: Frequency) -> Result<(), UnifiedError> {
        if freq < self.min_freq || freq > self.max_freq {
            return Err(UnifiedError::Other("Frequency out of range".into()));
        }

        // Check if frequency exists in table
        if !self.freq_table.iter().any(|e| e.frequency == freq) {
            return Err(UnifiedError::Other(
                "Frequency not in table".into(),
            ));
        }

        let old_freq = self.cur_freq;
        if old_freq == freq {
            return Ok(());
        }

        // Perform transition
        // In a real implementation, this would call hardware-specific code
        self.cur_freq = freq;

        // Record transition
        let transition = FreqTransition {
            old_freq,
            new_freq: freq,
            cpu_id: self.cpu_id,
            timestamp: self.get_timestamp(),
            latency: self.transition_latency,
        };
        self.stats.record_transition(transition);

        Ok(())
    }

    /// Update frequency based on CPU load
    pub fn update(&mut self, load: LoadPercent) -> Result<(), UnifiedError> {
        let now = self.get_timestamp();
        let target_freq = match &self.governor {
            GovernorType::Performance => Some(self.max_freq),
            GovernorType::Powersave => Some(self.min_freq),
            GovernorType::Ondemand {
                up_threshold,
                down_threshold,
                sampling_rate,
            } => {
                // Check sampling rate
                if let Some(last_update) = self.last_update {
                    let elapsed = now - last_update;
                    if elapsed < (*sampling_rate as u64) * 1000 {
                        return Ok(());
                    }
                }

                if load > *up_threshold {
                    Some(self.max_freq)
                } else if load < *down_threshold {
                    Some(self.min_freq)
                } else {
                    None // No change
                }
            }
            GovernorType::Conservative {
                up_threshold,
                down_threshold,
                step,
            } => {
                if load > *up_threshold {
                    // Increase gradually
                    let next_freq = (self.cur_freq + step).min(self.max_freq);
                    if next_freq != self.cur_freq {
                        Some(next_freq)
                    } else {
                        Some(self.max_freq)
                    }
                } else if load < *down_threshold {
                    // Decrease gradually
                    let next_freq = self.cur_freq.saturating_sub(*step);
                    if next_freq >= self.min_freq {
                        Some(next_freq)
                    } else {
                        Some(self.min_freq)
                    }
                } else {
                    None // No change
                }
            }
            GovernorType::Userspace => None, // Manual control
        };

        self.last_update = Some(now);

        if let Some(freq) = target_freq {
            if freq != self.cur_freq {
                self.set_frequency(freq)?;
            }
        }

        Ok(())
    }

    /// Get next higher frequency
    pub fn next_higher_freq(&self) -> Option<Frequency> {
        self.freq_table
            .iter()
            .find(|e| e.frequency > self.cur_freq)
            .map(|e| e.frequency)
    }

    /// Get next lower frequency
    pub fn next_lower_freq(&self) -> Option<Frequency> {
        self.freq_table
            .iter()
            .rev()
            .find(|e| e.frequency < self.cur_freq)
            .map(|e| e.frequency)
    }

    /// Get nearest frequency from table
    pub fn nearest_freq(&self, target: Frequency) -> Frequency {
        let mut nearest = self.freq_table[0].frequency;
        let mut min_diff = (target as i32 - nearest as i32).abs();

        for entry in &self.freq_table {
            let diff = (target as i32 - entry.frequency as i32).abs();
            if diff < min_diff {
                min_diff = diff;
                nearest = entry.frequency;
            }
        }

        nearest
    }

    /// Get timestamp (nanoseconds since boot)
    fn get_timestamp(&self) -> u64 {
        // In a real implementation, this would read from a hardware timer
        // For now, return a dummy value
        0
    }
}

/// CPU frequency manager
#[derive(Debug)]
pub struct CpuFreqManager {
    /// Per-CPU frequency devices
    devices: Vec<Option<CpuFreq>>,
    /// Cluster management (for big.LITTLE systems)
    clusters: Vec<Vec<CpuId>>,
}

impl CpuFreqManager {
    /// Create new CPU frequency manager
    pub fn new() -> Result<Self, UnifiedError> {
        Ok(CpuFreqManager {
            devices: Vec::new(),
            clusters: Vec::new(),
        })
    }

    /// Add CPU frequency device
    pub fn add_cpu(
        &mut self,
        cpu_id: CpuId,
        freq_table: Vec<FreqTableEntry>,
        transition_latency: LatencyNs,
    ) -> Result<(), UnifiedError> {
        // Ensure vector is large enough
        if cpu_id as usize >= self.devices.len() {
            self.devices.resize(cpu_id as usize + 1, None);
        }

        let device = CpuFreq::new(cpu_id, freq_table, transition_latency)?;
        self.devices[cpu_id as usize] = Some(device);
        Ok(())
    }

    /// Get CPU frequency device
    pub fn get_cpu(&self, cpu_id: CpuId) -> Option<&CpuFreq> {
        self.devices.get(cpu_id as usize)?.as_ref()
    }

    /// Get CPU frequency device (mutable)
    pub fn get_cpu_mut(&mut self, cpu_id: CpuId) -> Option<&mut CpuFreq> {
        self.devices.get_mut(cpu_id as usize)?.as_mut()
    }

    /// Update all CPUs based on load
    pub fn update_all(&mut self, loads: &[(CpuId, LoadPercent)]) -> Result<(), UnifiedError> {
        for &(cpu_id, load) in loads {
            if let Some(device) = self.get_cpu_mut(cpu_id) {
                device.update(load)?;
            }
        }
        Ok(())
    }

    /// Set governor for all CPUs
    pub fn set_governor_all(&mut self, governor: GovernorType) -> Result<(), UnifiedError> {
        for device in self.devices.iter_mut().flatten() {
            device.set_governor(governor)?;
        }
        Ok(())
    }

    /// Create CPU cluster (for big.LITTLE)
    pub fn create_cluster(&mut self, cpus: Vec<CpuId>) -> Result<(), UnifiedError> {
        // Validate all CPUs exist
        for &cpu_id in &cpus {
            if self.get_cpu(cpu_id).is_none() {
                return Err(UnifiedError::Other(format!("CPU {} not found", cpu_id)));
            }
        }

        self.clusters.push(cpus);
        Ok(())
    }

    /// Synchronize frequency within cluster
    pub fn sync_cluster(&mut self, cluster_id: usize) -> Result<(), UnifiedError> {
        // Clone the cluster to avoid borrow conflicts
        let cluster_ids: Vec<CpuId> = self
            .clusters
            .get(cluster_id)
            .ok_or_else(|| UnifiedError::Other("Cluster not found".into()))?
            .to_vec();

        if cluster_ids.is_empty() {
            return Ok(());
        }

        // Get target frequency from first CPU
        let target_freq = if let Some(first_cpu) = self.get_cpu(cluster_ids[0]) {
            first_cpu.cur_freq()
        } else {
            return Ok(());
        };

        // Apply to all CPUs in cluster
        for cpu_id in cluster_ids {
            if let Some(device) = self.get_cpu_mut(cpu_id) {
                device.set_frequency(target_freq)?;
            }
        }

        Ok(())
    }

    /// Get all frequencies for monitoring
    pub fn get_all_frequencies(&self) -> Vec<(CpuId, Frequency)> {
        self.devices
            .iter()
            .enumerate()
            .filter_map(|(id, device)| device.as_ref().map(|d| (id as CpuId, d.cur_freq())))
            .collect()
    }

    /// Get all statistics
    pub fn get_all_stats(&self) -> Vec<(CpuId, &FreqStats)> {
        self.devices
            .iter()
            .enumerate()
            .filter_map(|(id, device)| device.as_ref().map(|d| (id as CpuId, d.stats())))
            .collect()
    }
}

/// Default frequency table for x86_64 processors (example)
pub fn default_x86_freq_table() -> Vec<FreqTableEntry> {
    vec![
        FreqTableEntry::new(800, 10_000),   // 800 MHz, 10μs latency
        FreqTableEntry::new(1200, 10_000),  // 1.2 GHz
        FreqTableEntry::new(1600, 10_000),  // 1.6 GHz
        FreqTableEntry::new(2000, 10_000),  // 2.0 GHz
        FreqTableEntry::new(2400, 10_000),  // 2.4 GHz
        FreqTableEntry::new(2800, 10_000),  // 2.8 GHz
        FreqTableEntry::new(3200, 10_000),  // 3.2 GHz
    ]
}

/// Default frequency table for ARM processors (example)
pub fn default_arm_freq_table() -> Vec<FreqTableEntry> {
    vec![
        FreqTableEntry::new(300, 20_000),   // 300 MHz, 20μs latency
        FreqTableEntry::new(600, 20_000),   // 600 MHz
        FreqTableEntry::new(900, 20_000),   // 900 MHz
        FreqTableEntry::new(1200, 20_000),  // 1.2 GHz
        FreqTableEntry::new(1500, 20_000),  // 1.5 GHz
        FreqTableEntry::new(1800, 20_000),  // 1.8 GHz
        FreqTableEntry::new(2000, 20_000),  // 2.0 GHz
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cpufreq_creation() {
        let freq_table = default_x86_freq_table();
        let cpufreq = CpuFreq::new(0, freq_table, 10_000).unwrap();
        assert_eq!(cpufreq.cpu_id(), 0);
        assert_eq!(cpufreq.cur_freq(), 800);
        assert_eq!(cpufreq.min_freq(), 800);
        assert_eq!(cpufreq.max_freq(), 3200);
    }

    #[test]
    fn test_frequency_transition() {
        let freq_table = default_x86_freq_table();
        let mut cpufreq = CpuFreq::new(0, freq_table, 10_000).unwrap();

        cpufreq.set_frequency(2000).unwrap();
        assert_eq!(cpufreq.cur_freq(), 2000);
        assert_eq!(cpufreq.stats().total_transitions, 1);
    }

    #[test]
    fn test_performance_governor() {
        let freq_table = default_x86_freq_table();
        let mut cpufreq = CpuFreq::new(0, freq_table, 10_000).unwrap();

        cpufreq.set_governor(GovernorType::Performance).unwrap();
        assert_eq!(cpufreq.cur_freq(), 3200);
    }

    #[test]
    fn test_powersave_governor() {
        let freq_table = default_x86_freq_table();
        let mut cpufreq = CpuFreq::new(0, freq_table, 10_000).unwrap();

        cpufreq.set_governor(GovernorType::Powersave).unwrap();
        assert_eq!(cpufreq.cur_freq(), 800);
    }

    #[test]
    fn test_ondemand_governor() {
        let freq_table = default_x86_freq_table();
        let mut cpufreq = CpuFreq::new(0, freq_table, 10_000).unwrap();

        cpufreq
            .set_governor(GovernorType::ondemand_default())
            .unwrap();

        // High load should increase frequency
        cpufreq.update(90).unwrap();
        assert_eq!(cpufreq.cur_freq(), 3200);

        // Low load should decrease frequency
        cpufreq.update(10).unwrap();
        assert_eq!(cpufreq.cur_freq(), 800);
    }

    #[test]
    fn test_nearest_frequency() {
        let freq_table = default_x86_freq_table();
        let cpufreq = CpuFreq::new(0, freq_table, 10_000).unwrap();

        assert_eq!(cpufreq.nearest_freq(900), 800);
        assert_eq!(cpufreq.nearest_freq(1000), 1200);
        assert_eq!(cpufreq.nearest_freq(2200), 2400);
    }

    #[test]
    fn test_frequency_limits() {
        let freq_table = default_x86_freq_table();
        let mut cpufreq = CpuFreq::new(0, freq_table, 10_000).unwrap();

        cpufreq.set_min_freq(1200).unwrap();
        cpufreq.set_max_freq(2400).unwrap();

        assert_eq!(cpufreq.min_freq(), 1200);
        assert_eq!(cpufreq.max_freq(), 2400);
        assert_eq!(cpufreq.cur_freq(), 1200);

        // Setting frequency outside limits should fail
        assert!(cpufreq.set_frequency(800).is_err());
        assert!(cpufreq.set_frequency(3200).is_err());
    }
}
