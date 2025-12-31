//! # SCADA (Supervisory Control and Data Acquisition) System
//!
//! Comprehensive SCADA implementation with:
//! - Real-time data acquisition
//! - Historical database
//! - Alarm management
//! - HMI interface
//! - Trend analysis

use alloc::{
    collections::BTreeMap,
    string::String,
    sync::Arc,
    vec::Vec,
};
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use crate::subsystems::industrial::{
    error::{IndustrialError, IndustrialResult, ScadaError},
    SystemHealth, SystemStatus,
};

/// SCADA system configuration
#[derive(Debug, Clone)]
pub struct ScadaConfig {
    /// Maximum number of tags
    pub max_tags: usize,
    /// Data acquisition interval in milliseconds
    pub acquisition_interval_ms: u64,
    /// Historical data retention in hours
    pub history_retention_hours: u32,
    /// Maximum alarms
    pub max_alarms: usize,
    /// Enable HMI
    pub enable_hmi: bool,
    /// Enable trend analysis
    pub enable_trends: bool,
}

impl Default for ScadaConfig {
    fn default() -> Self {
        Self {
            max_tags: 50000,
            acquisition_interval_ms: 100,  // 10 Hz
            history_retention_hours: 24 * 30,  // 30 days
            max_alarms: 10000,
            enable_hmi: true,
            enable_trends: true,
        }
    }
}

/// Tag data type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagType {
    Boolean,
    Integer,
    Float,
    String,
}

/// Tag quality (IEC 61131-3)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagQuality {
    Good = 0,
    Bad = 1,
    Uncertain = 2,
    /// Sensor failure
    SensorFailure = 3,
    /// Communication lost
    CommFailure = 4,
    /// Out of service
    OutOfService = 5,
}

/// Tag value with quality and timestamp
#[derive(Debug, Clone)]
pub struct TagValue {
    pub value: TagData,
    pub quality: TagQuality,
    pub timestamp_us: u64,
}

/// Tag data
#[derive(Debug, Clone)]
pub enum TagData {
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
}

impl TagData {
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i64> {
        match self {
            Self::Integer(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_float(&self) -> Option<f64> {
        match self {
            Self::Float(f) => Some(*f),
            Self::Integer(i) => Some(*i as f64),
            _ => None,
        }
    }
}

/// Tag definition
#[derive(Debug, Clone)]
pub struct Tag {
    pub name: String,
    pub tag_type: TagType,
    pub description: String,
    pub unit: Option<String>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
    pub logging_enabled: bool,
    pub alarm_enabled: bool,
}

impl Tag {
    pub fn new(name: impl Into<String>, tag_type: TagType) -> Self {
        Self {
            name: name.into(),
            tag_type,
            description: String::new(),
            unit: None,
            min_value: None,
            max_value: None,
            logging_enabled: true,
            alarm_enabled: false,
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_unit(mut self, unit: impl Into<String>) -> Self {
        self.unit = Some(unit.into());
        self
    }

    pub fn with_range(mut self, min: f64, max: f64) -> Self {
        self.min_value = Some(min);
        self.max_value = Some(max);
        self
    }
}

/// Alarm severity
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum AlarmSeverity {
    Info = 0,
    Warning = 1,
    Minor = 2,
    Major = 3,
    Critical = 4,
}

/// Alarm state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlarmState {
    Active,
    Acknowledged,
    Cleared,
}

/// Alarm
#[derive(Debug, Clone)]
pub struct Alarm {
    pub id: u64,
    pub tag_name: String,
    pub severity: AlarmSeverity,
    pub state: AlarmState,
    pub message: String,
    pub timestamp_us: u64,
    pub acknowledged_by: Option<String>,
    pub cleared_timestamp_us: Option<u64>,
}

/// Historical data point
#[derive(Debug, Clone)]
pub struct HistoryPoint {
    pub tag_name: String,
    pub value: TagData,
    pub quality: TagQuality,
    pub timestamp_us: u64,
}

/// Trend data
#[derive(Debug, Clone)]
pub struct TrendData {
    pub tag_name: String,
    pub start_time_us: u64,
    pub end_time_us: u64,
    pub samples: Vec<(u64, f64)>,  // (timestamp, value)
}

/// SCADA system
pub struct ScadaSystem {
    config: ScadaConfig,
    tags: BTreeMap<String, Tag>,
    current_values: BTreeMap<String, TagValue>,
    historical_data: Vec<HistoryPoint>,
    alarms: Vec<Alarm>,
    alarm_counter: Arc<AtomicU64>,
    running: Arc<AtomicBool>,
    acquisition_count: Arc<AtomicU64>,
}

impl ScadaSystem {
    /// Create new SCADA system
    pub fn new() -> Self {
        Self::with_config(ScadaConfig::default())
    }

    /// Create SCADA system with configuration
    pub fn with_config(config: ScadaConfig) -> Self {
        Self {
            config,
            tags: BTreeMap::new(),
            current_values: BTreeMap::new(),
            historical_data: Vec::new(),
            alarms: Vec::new(),
            alarm_counter: Arc::new(AtomicU64::new(0)),
            running: Arc::new(AtomicBool::new(false)),
            acquisition_count: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Initialize SCADA system
    pub fn initialize(&mut self) -> IndustrialResult<()> {
        log_info!("Initializing SCADA system");
        log_info!("Maximum tags: {}", self.config.max_tags);
        log_info!("Acquisition interval: {}ms", self.config.acquisition_interval_ms);
        Ok(())
    }

    /// Add tag
    pub fn add_tag(&mut self, tag: Tag) -> IndustrialResult<()> {
        if self.tags.len() >= self.config.max_tags {
            return Err(IndustrialError::Scada(ScadaError::DatabaseError("Maximum tags exceeded")));
        }

        let name = tag.name.clone();
        self.tags.insert(name.clone(), tag);
        log_debug!("Added tag: {}", name);
        Ok(())
    }

    /// Remove tag
    pub fn remove_tag(&mut self, name: &str) -> IndustrialResult<()> {
        if !self.tags.contains_key(name) {
            return Err(IndustrialError::Scada(ScadaError::TagNotFound(name.into())));
        }

        self.tags.remove(name);
        self.current_values.remove(name);
        Ok(())
    }

    /// Update tag value
    pub fn update_tag(&mut self, name: &str, value: TagValue) -> IndustrialResult<()> {
        if !self.tags.contains_key(name) {
            return Err(IndustrialError::Scada(ScadaError::TagNotFound(name.into())));
        }

        // Store current value
        self.current_values.insert(name.into(), value.clone());

        // Log to historical database
        let (logging_enabled, alarm_enabled) = {
            let tag = self.tags.get(name).unwrap();
            (tag.logging_enabled, tag.alarm_enabled)
        };

        if logging_enabled {
            self.log_historical(name, &value);
        }

        // Check alarms
        if alarm_enabled {
            self.check_alarms(name, &value)?;
        }

        Ok(())
    }

    /// Get current tag value
    pub fn get_tag_value(&self, name: &str) -> IndustrialResult<&TagValue> {
        self.current_values.get(name)
            .ok_or_else(|| IndustrialError::Scada(ScadaError::TagNotFound(name.into())))
    }

    /// Get all tags
    pub fn tags(&self) -> &BTreeMap<String, Tag> {
        &self.tags
    }

    /// Get all current values
    pub fn current_values(&self) -> &BTreeMap<String, TagValue> {
        &self.current_values
    }

    /// Log historical data
    fn log_historical(&mut self, name: &str, value: &TagValue) {
        let point = HistoryPoint {
            tag_name: name.into(),
            value: value.value.clone(),
            quality: value.quality,
            timestamp_us: value.timestamp_us,
        };

        self.historical_data.push(point);

        // Prune old data
        let retention_us = self.config.history_retention_hours as u64 * 3600 * 1_000_000;
        let cutoff_us = value.timestamp_us.saturating_sub(retention_us);

        self.historical_data.retain(|p| p.timestamp_us >= cutoff_us);
    }

    /// Check for alarms
    fn check_alarms(&mut self, name: &str, value: &TagValue) -> IndustrialResult<()> {
        let tag = self.tags.get(name).unwrap();

        // Check range alarms
        if let (Some(min), Some(max)) = (tag.min_value, tag.max_value) {
            if let TagData::Float(f) = &value.value {
                if *f < min || *f > max {
                    self.generate_alarm(
                        name,
                        AlarmSeverity::Major,
                        format!("Value {} outside range [{}, {}]", f, min, max),
                    );
                }
            }
        }

        // Check quality alarms
        if matches!(value.quality, TagQuality::Bad | TagQuality::SensorFailure | TagQuality::CommFailure) {
            self.generate_alarm(
                name,
                AlarmSeverity::Critical,
                format!("Bad quality: {:?}", value.quality),
            );
        }

        Ok(())
    }

    /// Generate alarm
    fn generate_alarm(&mut self, tag_name: &str, severity: AlarmSeverity, message: String) {
        if self.alarms.len() >= self.config.max_alarms {
            // Remove oldest cleared alarm
            if let Some(pos) = self.alarms.iter().position(|a| a.state == AlarmState::Cleared) {
                self.alarms.remove(pos);
            } else {
                return;  // Cannot add more alarms
            }
        }

        let id = self.alarm_counter.fetch_add(1, Ordering::SeqCst);
        let alarm = Alarm {
            id,
            tag_name: tag_name.into(),
            severity,
            state: AlarmState::Active,
            message,
            timestamp_us: self.get_timestamp(),
            acknowledged_by: None,
            cleared_timestamp_us: None,
        };

        self.alarms.push(alarm);
        log_warn!("Alarm generated: {} - {}", tag_name, severity);
    }

    /// Acknowledge alarm
    pub fn acknowledge_alarm(&mut self, id: u64, user: &str) -> IndustrialResult<()> {
        let alarm = self.alarms.iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| IndustrialError::Scada(ScadaError::DatabaseError("Alarm not found")))?;

        if alarm.state != AlarmState::Active {
            return Err(IndustrialError::Scada(ScadaError::DatabaseError("Alarm not active")));
        }

        alarm.state = AlarmState::Acknowledged;
        alarm.acknowledged_by = Some(user.into());
        log_info!("Alarm {} acknowledged by {}", id, user);
        Ok(())
    }

    /// Clear alarm
    pub fn clear_alarm(&mut self, id: u64) -> IndustrialResult<()> {
        let timestamp = self.get_timestamp();

        let alarm = self.alarms.iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| IndustrialError::Scada(ScadaError::DatabaseError("Alarm not found")))?;

        alarm.state = AlarmState::Cleared;
        alarm.cleared_timestamp_us = Some(timestamp);
        log_info!("Alarm {} cleared", id);
        Ok(())
    }

    /// Get all alarms
    pub fn alarms(&self) -> &[Alarm] {
        &self.alarms
    }

    /// Get active alarms
    pub fn active_alarms(&self) -> Vec<&Alarm> {
        self.alarms.iter()
            .filter(|a| a.state == AlarmState::Active)
            .collect()
    }

    /// Query historical data
    pub fn query_history(&self, tag_name: &str, start_us: u64, end_us: u64) -> Vec<&HistoryPoint> {
        self.historical_data.iter()
            .filter(|p| p.tag_name == tag_name && p.timestamp_us >= start_us && p.timestamp_us <= end_us)
            .collect()
    }

    /// Analyze trends
    pub fn analyze_trends(&self, tag_name: &str, duration_us: u64) -> IndustrialResult<TrendData> {
        let end_time = self.get_timestamp();
        let start_time = end_time.saturating_sub(duration_us);

        let samples: Vec<(u64, f64)> = self.historical_data.iter()
            .filter(|p| p.tag_name == tag_name && p.timestamp_us >= start_time)
            .filter_map(|p| {
                if let TagData::Float(f) = &p.value {
                    Some((p.timestamp_us, *f))
                } else {
                    None
                }
            })
            .collect();

        if samples.is_empty() {
            return Err(IndustrialError::Scada(ScadaError::TrendAnalysisError("No data available")));
        }

        Ok(TrendData {
            tag_name: tag_name.into(),
            start_time_us: start_time,
            end_time_us: end_time,
            samples,
        })
    }

    /// Calculate statistics for tag
    pub fn calculate_statistics(&self, tag_name: &str, duration_us: u64) -> Option<TagStatistics> {
        let end_time = self.get_timestamp();
        let start_time = end_time.saturating_sub(duration_us);

        let values: Vec<f64> = self.historical_data.iter()
            .filter(|p| p.tag_name == tag_name && p.timestamp_us >= start_time)
            .filter_map(|p| p.value.as_float())
            .collect();

        if values.is_empty() {
            return None;
        }

        let min = values.iter().cloned().reduce(f64::min)?;
        let max = values.iter().cloned().reduce(f64::max)?;
        let sum: f64 = values.iter().sum();
        let avg = sum / values.len() as f64;

        // Calculate standard deviation
        let variance = values.iter()
            .map(|v| (v - avg).powi(2))
            .sum::<f64>() / values.len() as f64;
        let std_dev = variance.sqrt();

        Some(TagStatistics {
            min,
            max,
            avg,
            std_dev,
            sample_count: values.len(),
        })
    }

    /// Start data acquisition
    pub fn start_data_acquisition(&self) -> IndustrialResult<()> {
        if self.running.load(Ordering::SeqCst) {
            return Err(IndustrialError::Scada(ScadaError::ServerError("Already running")));
        }

        self.running.store(true, Ordering::SeqCst);
        log_info!("Data acquisition started");
        Ok(())
    }

    /// Stop data acquisition
    pub fn stop_data_acquisition(&self) -> IndustrialResult<()> {
        self.running.store(false, Ordering::SeqCst);
        log_info!("Data acquisition stopped");
        Ok(())
    }

    /// Get acquisition count
    pub fn acquisition_count(&self) -> u64 {
        self.acquisition_count.load(Ordering::SeqCst)
    }

    /// Get system health
    pub fn get_health(&self) -> SystemHealth {
        let active_alarms = self.active_alarms().len() as u32;
        let status = if active_alarms > 0 {
            SystemStatus::Error
        } else {
            SystemStatus::Running
        };

        SystemHealth::new(status)
    }

    /// Get current timestamp in microseconds
    fn get_timestamp(&self) -> u64 {
        // In real implementation, get from system clock
        0
    }
}

/// Tag statistics
#[derive(Debug, Clone)]
pub struct TagStatistics {
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub std_dev: f64,
    pub sample_count: usize,
}

/// HMI interface
pub struct HmiInterface {
    scada: Arc<ScadaSystem>,
}

impl HmiInterface {
    pub fn new(scada: Arc<ScadaSystem>) -> Self {
        Self { scada }
    }

    /// Get tag value for display
    pub fn get_display_value(&self, tag_name: &str) -> IndustrialResult<String> {
        let value = self.scada.get_tag_value(tag_name)?;
        let tag = self.scada.tags().get(tag_name)
            .ok_or_else(|| IndustrialError::Scada(ScadaError::TagNotFound(tag_name.into())))?;

        match &value.value {
            TagData::Boolean(b) => Ok(if *b { "TRUE".into() } else { "FALSE".into() }),
            TagData::Integer(i) => {
                let unit = tag.unit.as_deref().unwrap_or("");
                Ok(format!("{} {}", i, unit))
            }
            TagData::Float(f) => {
                let unit = tag.unit.as_deref().unwrap_or("");
                Ok(format!("{:.2} {}", f, unit))
            }
            TagData::String(s) => Ok(s.clone()),
        }
    }

    /// Get alarm summary
    pub fn get_alarm_summary(&self) -> AlarmSummary {
        let alarms = self.scada.alarms();
        let active = alarms.iter().filter(|a| a.state == AlarmState::Active).count();
        let acknowledged = alarms.iter().filter(|a| a.state == AlarmState::Acknowledged).count();
        let critical = alarms.iter().filter(|a| a.severity == AlarmSeverity::Critical && a.state == AlarmState::Active).count();

        AlarmSummary {
            total_alarms: alarms.len(),
            active_alarms: active,
            acknowledged_alarms: acknowledged,
            critical_alarms: critical,
        }
    }
}

/// Alarm summary
#[derive(Debug, Clone)]
pub struct AlarmSummary {
    pub total_alarms: usize,
    pub active_alarms: usize,
    pub acknowledged_alarms: usize,
    pub critical_alarms: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scada_creation() {
        let scada = ScadaSystem::new();
        assert_eq!(scada.tags().len(), 0);
    }

    #[test]
    fn test_tag_operations() {
        let mut scada = ScadaSystem::new();
        let tag = Tag::new("temperature", TagType::Float)
            .with_unit("°C")
            .with_range(-50.0, 150.0);

        scada.add_tag(tag).unwrap();
        assert_eq!(scada.tags().len(), 1);
    }

    #[test]
    fn test_tag_value_update() {
        let mut scada = ScadaSystem::new();
        let tag = Tag::new("pressure", TagType::Float);
        scada.add_tag(tag).unwrap();

        let value = TagValue {
            value: TagData::Float(101.3),
            quality: TagQuality::Good,
            timestamp_us: 1000,
        };

        scada.update_tag("pressure", value).unwrap();
        assert!(scada.get_tag_value("pressure").is_ok());
    }

    #[test]
    fn test_alarm_generation() {
        let mut scada = ScadaSystem::new();
        let tag = Tag::new("level", TagType::Float)
            .with_range(0.0, 100.0)
            .with_alarm_enabled();

        // Enable alarm on the tag
        let mut tag_with_alarm = tag.clone();
        tag_with_alarm.alarm_enabled = true;

        scada.add_tag(tag_with_alarm).unwrap();

        let value = TagValue {
            value: TagData::Float(150.0),  // Out of range
            quality: TagQuality::Good,
            timestamp_us: 1000,
        };

        scada.update_tag("level", value).unwrap();
        assert!(!scada.active_alarms().is_empty());
    }

    #[test]
    fn test_alarm_acknowledgment() {
        let mut scada = ScadaSystem::new();
        let tag = Tag::new("flow", TagType::Float)
            .with_range(0.0, 100.0);

        let mut tag_with_alarm = tag.clone();
        tag_with_alarm.alarm_enabled = true;

        scada.add_tag(tag_with_alarm).unwrap();

        let value = TagValue {
            value: TagData::Float(150.0),
            quality: TagQuality::Good,
            timestamp_us: 1000,
        };

        scada.update_tag("flow", value).unwrap();

        let active = scada.active_alarms();
        let alarm_id = active[0].id;

        scada.acknowledge_alarm(alarm_id, "operator").unwrap();
        let alarm = &scada.alarms()[0];
        assert_eq!(alarm.state, AlarmState::Acknowledged);
    }

    #[test]
    fn test_statistics() {
        let mut scada = ScadaSystem::new();
        let tag = Tag::new("voltage", TagType::Float);
        scada.add_tag(tag).unwrap();

        for i in 0..10 {
            let value = TagValue {
                value: TagData::Float(i as f64 * 10.0),
                quality: TagQuality::Good,
                timestamp_us: i * 1000,
            };
            scada.update_tag("voltage", value).unwrap();
        }

        let stats = scada.calculate_statistics("voltage", 10000).unwrap();
        assert_eq!(stats.min, 0.0);
        assert_eq!(stats.max, 90.0);
        assert_eq!(stats.sample_count, 10);
    }
}
