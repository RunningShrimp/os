//! Auto Scaling Module
//! 
//! This module provides auto-scaling capabilities for NOS kernel,
//! including horizontal and vertical scaling, scaling policies, and metrics.

use crate::error::unified::UnifiedError;
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};
use spin::Mutex;

/// Scaling directions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingDirection {
    /// Scale up (increase resources)
    Up,
    /// Scale down (decrease resources)
    Down,
}

/// Scaling types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingType {
    /// Horizontal scaling (add/remove instances)
    Horizontal,
    /// Vertical scaling (increase/decrease resources)
    Vertical,
}

/// Scaling policies
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingPolicy {
    /// Manual scaling
    Manual,
    /// Scheduled scaling
    Scheduled,
    /// Metric-based scaling
    Metric,
    /// Predictive scaling
    Predictive,
}

/// Scaling metrics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScalingMetric {
    /// CPU utilization
    CPUUtilization,
    /// Memory utilization
    MemoryUtilization,
    /// Network throughput
    NetworkThroughput,
    /// Request rate
    RequestRate,
    /// Response time
    ResponseTime,
    /// Queue length
    QueueLength,
    /// Custom metric
    Custom,
}

/// Scaling rule
#[derive(Debug, Clone)]
pub struct ScalingRule {
    /// Rule ID
    pub id: u64,
    /// Rule name
    pub name: String,
    /// Service name
    pub service_name: String,
    /// Scaling type
    pub scaling_type: ScalingType,
    /// Scaling metric
    pub metric: ScalingMetric,
    /// Scale up threshold
    pub scale_up_threshold: f64,
    /// Scale down threshold
    pub scale_down_threshold: f64,
    /// Cooldown period (in seconds)
    pub cooldown_period: u32,
    /// Minimum instances
    pub min_instances: u32,
    /// Maximum instances
    pub max_instances: u32,
    /// Scale up amount
    pub scale_up_amount: u32,
    /// Scale down amount
    pub scale_down_amount: u32,
    /// Rule enabled
    pub enabled: bool,
    /// Created timestamp
    pub created_at: u64,
}

/// Scaling event
#[derive(Debug, Clone)]
pub struct ScalingEvent {
    /// Event ID
    pub id: u64,
    /// Rule ID
    pub rule_id: u64,
    /// Service name
    pub service_name: String,
    /// Scaling direction
    pub direction: ScalingDirection,
    /// Scaling type
    pub scaling_type: ScalingType,
    /// Old instance count
    pub old_instance_count: u32,
    /// New instance count
    pub new_instance_count: u32,
    /// Metric value
    pub metric_value: f64,
    /// Threshold value
    pub threshold_value: f64,
    /// Event timestamp
    pub timestamp: u64,
    /// Event reason
    pub reason: String,
}

/// Auto scaler
pub struct AutoScaler {
    /// Scaling rules
    rules: Mutex<BTreeMap<u64, ScalingRule>>,
    /// Scaling events
    events: Mutex<BTreeMap<u64, ScalingEvent>>,
    /// Service metrics
    metrics: Mutex<BTreeMap<String, BTreeMap<ScalingMetric, f64>>>,
    /// Next rule ID
    next_rule_id: AtomicU64,
    /// Next event ID
    next_event_id: AtomicU64,
    /// Statistics
    stats: Mutex<ScalingStats>,
    /// Active status
    active: bool,
}

/// Auto scaler statistics
#[derive(Debug, Clone)]
pub struct ScalingStats {
    /// Total scaling events
    pub total_events: u64,
    /// Scale up events
    pub scale_up_events: u64,
    /// Scale down events
    pub scale_down_events: u64,
    /// Horizontal scaling events
    pub horizontal_events: u64,
    /// Vertical scaling events
    pub vertical_events: u64,
    /// Total rules
    pub total_rules: u64,
    /// Active rules
    pub active_rules: u64,
    /// Total operations
    pub total_operations: u64,
}

impl Default for ScalingStats {
    fn default() -> Self {
        Self {
            total_events: 0,
            scale_up_events: 0,
            scale_down_events: 0,
            horizontal_events: 0,
            vertical_events: 0,
            total_rules: 0,
            active_rules: 0,
            total_operations: 0,
        }
    }
}

impl AutoScaler {
    /// Create a new auto scaler
    pub fn new() -> Result<Self, UnifiedError> {
        Ok(Self {
            rules: Mutex::new(BTreeMap::new()),
            events: Mutex::new(BTreeMap::new()),
            metrics: Mutex::new(BTreeMap::new()),
            next_rule_id: AtomicU64::new(1),
            next_event_id: AtomicU64::new(1),
            stats: Mutex::new(ScalingStats::default()),
            active: true,
        })
    }

    /// Initialize auto scaler
    pub fn initialize(&self) -> Result<(), UnifiedError> {
        log::info!("Initializing auto scaler");
        
        // Initialize auto scaling components
        // This would include setting up metric collectors, etc.
        
        log::info!("Auto scaler initialized");
        Ok(())
    }

    /// Create a scaling rule
    pub fn create_rule(
        &self,
        name: String,
        service_name: String,
        scaling_type: ScalingType,
        metric: ScalingMetric,
        scale_up_threshold: f64,
        scale_down_threshold: f64,
        cooldown_period: u32,
        min_instances: u32,
        max_instances: u32,
        scale_up_amount: u32,
        scale_down_amount: u32,
    ) -> Result<u64, UnifiedError> {
        if !self.active {
            return Err(UnifiedError::CloudNative("Auto scaler is not active".to_string()));
        }
        
        let rule_id = self.next_rule_id.fetch_add(1, Ordering::Relaxed);
        let current_time = self.get_timestamp();
        
        let rule = ScalingRule {
            id: rule_id,
            name: name.clone(),
            service_name: service_name.clone(),
            scaling_type,
            metric,
            scale_up_threshold,
            scale_down_threshold,
            cooldown_period,
            min_instances,
            max_instances,
            scale_up_amount,
            scale_down_amount,
            enabled: true,
            created_at: current_time,
        };
        
        {
            let mut rules = self.rules.lock();
            rules.insert(rule_id, rule);
        }
        
        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_rules += 1;
            stats.active_rules += 1;
            stats.total_operations += 1;
        }
        
        log::info!("Created scaling rule '{}' with ID: {}", name, rule_id);
        Ok(rule_id)
    }

    /// Update a scaling rule
    pub fn update_rule(
        &self,
        rule_id: u64,
        name: Option<String>,
        scale_up_threshold: Option<f64>,
        scale_down_threshold: Option<f64>,
        cooldown_period: Option<u32>,
        min_instances: Option<u32>,
        max_instances: Option<u32>,
        scale_up_amount: Option<u32>,
        scale_down_amount: Option<u32>,
        enabled: Option<bool>,
    ) -> Result<(), UnifiedError> {
        if !self.active {
            return Err(UnifiedError::CloudNative("Auto scaler is not active".to_string()));
        }
        
        {
            let mut rules = self.rules.lock();
            if let Some(rule) = rules.get_mut(&rule_id) {
                // Update rule fields
                if let Some(n) = name {
                    rule.name = n;
                }
                if let Some(t) = scale_up_threshold {
                    rule.scale_up_threshold = t;
                }
                if let Some(t) = scale_down_threshold {
                    rule.scale_down_threshold = t;
                }
                if let Some(p) = cooldown_period {
                    rule.cooldown_period = p;
                }
                if let Some(m) = min_instances {
                    rule.min_instances = m;
                }
                if let Some(m) = max_instances {
                    rule.max_instances = m;
                }
                if let Some(a) = scale_up_amount {
                    rule.scale_up_amount = a;
                }
                if let Some(a) = scale_down_amount {
                    rule.scale_down_amount = a;
                }
                if let Some(e) = enabled {
                    rule.enabled = e;
                }
                
                // Update statistics
                let mut stats = self.stats.lock();
                stats.total_operations += 1;
                
                log::info!("Updated scaling rule '{}' with ID: {}", rule.name, rule_id);
                Ok(())
            } else {
                Err(UnifiedError::CloudNative(
                    format!("Scaling rule {} not found", rule_id)
                ))
            }
        }
    }

    /// Enable or disable a scaling rule
    pub fn set_rule_enabled(&self, rule_id: u64, enabled: bool) -> Result<(), UnifiedError> {
        if !self.active {
            return Err(UnifiedError::CloudNative("Auto scaler is not active".to_string()));
        }
        
        {
            let mut rules = self.rules.lock();
            if let Some(rule) = rules.get_mut(&rule_id) {
                rule.enabled = enabled;
                
                // Update statistics
                let mut stats = self.stats.lock();
                if enabled {
                    stats.active_rules += 1;
                } else {
                    stats.active_rules = stats.active_rules.saturating_sub(1);
                }
                stats.total_operations += 1;
                
                log::info!("{} scaling rule '{}' with ID: {}", 
                           if enabled { "Enabled" } else { "Disabled" }, rule.name, rule_id);
                Ok(())
            } else {
                Err(UnifiedError::CloudNative(
                    format!("Scaling rule {} not found", rule_id)
                ))
            }
        }
    }

    /// Update service metrics
    pub fn update_metrics(
        &self,
        service_name: String,
        metric: ScalingMetric,
        value: f64,
    ) -> Result<(), UnifiedError> {
        if !self.active {
            return Err(UnifiedError::CloudNative("Auto scaler is not active".to_string()));
        }
        
        {
            let mut metrics = self.metrics.lock();
            let service_metrics = metrics.entry(service_name.clone()).or_insert_with(BTreeMap::new);
            service_metrics.insert(metric, value);
        }
        
        log::debug!("Updated metric {:?} for service '{}': {}", metric, service_name, value);
        Ok(())
    }

    /// Evaluate scaling rules
    pub fn evaluate_rules(&self) -> Result<Vec<ScalingEvent>, UnifiedError> {
        if !self.active {
            return Err(UnifiedError::CloudNative("Auto scaler is not active".to_string()));
        }
        
        let current_time = self.get_timestamp();
        let mut events = Vec::new();
        
        {
            let rules = self.rules.lock();
            let metrics = self.metrics.lock();
            
            for rule in rules.values() {
                if !rule.enabled {
                    continue;
                }
                
                // Get service metrics
                if let Some(service_metrics) = metrics.get(&rule.service_name) {
                    if let Some(&metric_value) = service_metrics.get(&rule.metric) {
                        // Check if scaling is needed
                        let should_scale_up = *metric_value > rule.scale_up_threshold;
                        let should_scale_down = *metric_value < rule.scale_down_threshold;
                        
                        if should_scale_up || should_scale_down {
                            let direction = if should_scale_up {
                                ScalingDirection::Up
                            } else {
                                ScalingDirection::Down
                            };
                            
                            let event_id = self.next_event_id.fetch_add(1, Ordering::Relaxed);
                            let event = ScalingEvent {
                                id: event_id,
                                rule_id: rule.id,
                                service_name: rule.service_name.clone(),
                                direction,
                                scaling_type: rule.scaling_type,
                                old_instance_count: 0, // Would be determined by service manager
                                new_instance_count: 0, // Would be determined by service manager
                                metric_value: *metric_value,
                                threshold_value: if should_scale_up {
                                    rule.scale_up_threshold
                                } else {
                                    rule.scale_down_threshold
                                },
                                timestamp: current_time,
                                reason: format!("Metric {:?} exceeded threshold", rule.metric),
                            };
                            
                            events.push(event);
                        }
                    }
                }
            }
        }
        
        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.total_events += events.len() as u64;
            for event in &events {
                match event.direction {
                    ScalingDirection::Up => stats.scale_up_events += 1,
                    ScalingDirection::Down => stats.scale_down_events += 1,
                }
                match event.scaling_type {
                    ScalingType::Horizontal => stats.horizontal_events += 1,
                    ScalingType::Vertical => stats.vertical_events += 1,
                }
            }
            stats.total_operations += 1;
        }
        
        log::info!("Evaluated scaling rules, generated {} events", events.len());
        Ok(events)
    }

    /// Get rule information
    pub fn get_rule_info(&self, rule_id: u64) -> Result<ScalingRule, UnifiedError> {
        let rules = self.rules.lock();
        if let Some(rule) = rules.get(&rule_id) {
            Ok(rule.clone())
        } else {
            Err(UnifiedError::CloudNative(
                format!("Scaling rule {} not found", rule_id)
            ))
        }
    }

    /// Get scaling events
    pub fn get_scaling_events(&self, service_name: Option<String>) -> Vec<ScalingEvent> {
        let events = self.events.lock();
        if let Some(name) = service_name {
            events.values()
                .filter(|e| e.service_name == name)
                .cloned()
                .collect()
        } else {
            events.values().cloned().collect()
        }
    }

    /// Get service metrics
    pub fn get_service_metrics(&self, service_name: &str) -> BTreeMap<ScalingMetric, f64> {
        let metrics = self.metrics.lock();
        metrics.get(service_name).cloned().unwrap_or_default()
    }

    /// List all rules
    pub fn list_rules(&self) -> Vec<ScalingRule> {
        let rules = self.rules.lock();
        rules.values().cloned().collect()
    }

    /// Get scaling events count
    pub fn get_scaling_events(&self) -> u64 {
        let stats = self.stats.lock();
        stats.total_events
    }

    /// Check if scaler is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Activate or deactivate scaler
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
        
        if active {
            log::info!("Auto scaler activated");
        } else {
            log::info!("Auto scaler deactivated");
        }
    }

    /// Optimize auto scaler
    pub fn optimize(&self) -> Result<(), UnifiedError> {
        if !self.active {
            return Err(UnifiedError::CloudNative("Auto scaler is not active".to_string()));
        }
        
        // Optimize auto scaling
        // This would include rule optimization, metric collection, etc.
        
        log::info!("Auto scaler optimized");
        Ok(())
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = ScalingStats::default();
    }

    /// Get current timestamp (in microseconds)
    fn get_timestamp(&self) -> u64 {
        // In a real implementation, this would use a high-precision timer
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }
}