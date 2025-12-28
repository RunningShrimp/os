//! Health Monitoring Integration Module
//!
//! This module integrates health monitoring with graceful degradation.
//! When health issues are detected, it automatically triggers appropriate
//! degradation strategies to maintain system stability.

extern crate alloc;

use alloc::string::String;
use alloc::format;
use crate::subsystems::sync::Mutex;
use crate::monitoring::health::{HealthChecker, HealthStatus, HealthCheckResult, get_health_checker};
use crate::reliability::graceful_degradation::{GracefulDegradationManager, create_graceful_degradation_manager};

/// Health monitoring integration manager
pub struct HealthIntegrationManager {
    /// Health checker reference
    health_checker: &'static HealthChecker,
    /// Graceful degradation manager
    degradation_manager: alloc::sync::Arc<Mutex<GracefulDegradationManager>>,
    /// Integration enabled flag
    enabled: bool,
    /// Last health check time
    last_check_time: u64,
    /// Health check interval (nanoseconds)
    check_interval_ns: u64,
}

impl HealthIntegrationManager {
    /// Create a new health integration manager
    pub fn new() -> Self {
        Self {
            health_checker: get_health_checker(),
            degradation_manager: create_graceful_degradation_manager(),
            enabled: true,
            last_check_time: 0,
            check_interval_ns: 5_000_000_000, // 5 seconds
        }
    }

    /// Enable integration
    pub fn enable(&mut self) {
        self.enabled = true;
        crate::println!("[health-integration] Health monitoring integration enabled");
    }

    /// Disable integration
    pub fn disable(&mut self) {
        self.enabled = false;
        crate::println!("[health-integration] Health monitoring integration disabled");
    }

    /// Check health and trigger degradation if needed
    pub fn check_and_react(&mut self) -> Result<(), i32> {
        if !self.enabled {
            return Ok(());
        }

        let current_time = crate::subsystems::time::hrtime_nanos();
        if current_time - self.last_check_time < self.check_interval_ns {
            return Ok(());
        }

        self.last_check_time = current_time;

        // Run health checks
        let health_results = self.health_checker.check_health();
        let overall_status = self.health_checker.get_overall_status();

        // Process health results and trigger degradation if needed
        for result in &health_results {
            self.process_health_result(result)?;
        }

        // Trigger degradation based on overall status
        match overall_status {
            HealthStatus::Unhealthy => {
                self.trigger_critical_degradation()?;
            }
            HealthStatus::Degraded => {
                self.trigger_degraded_degradation()?;
            }
            HealthStatus::Healthy => {
                // System is healthy, check if we can recover
                self.check_recovery()?;
            }
        }

        Ok(())
    }

    /// Process individual health check result
    fn process_health_result(&self, result: &HealthCheckResult) -> Result<(), i32> {
        match result.status {
            HealthStatus::Unhealthy => {
                crate::println!("[health-integration] Critical health issue detected: {} - {}", 
                    result.name, result.message);
                self.trigger_component_degradation(&result.name, "critical")?;
            }
            HealthStatus::Degraded => {
                crate::println!("[health-integration] Degraded health detected: {} - {}", 
                    result.name, result.message);
                self.trigger_component_degradation(&result.name, "degraded")?;
            }
            HealthStatus::Healthy => {
                // Component is healthy, no action needed
            }
        }
        Ok(())
    }

    /// Trigger critical degradation
    fn trigger_critical_degradation(&self) -> Result<(), i32> {
        let mut manager = self.degradation_manager.lock();
        
        // Trigger degradation using existing strategy or create a default one
        // First, try to use an existing health-based strategy
        let strategy_id = "health-critical-strategy";
        
        // If strategy doesn't exist, we'll need to create it first
        // For now, use a simple approach: trigger with a default strategy ID
        match manager.trigger_degradation(
            strategy_id,
            "system",
            "Critical health issue detected - system is unhealthy",
        ) {
            Ok(session_id) => {
                crate::println!("[health-integration] Triggered critical degradation: {}", session_id);
                Ok(())
            }
            Err(e) => {
                // Strategy might not exist, log and continue
                crate::println!("[health-integration] Failed to trigger critical degradation (strategy may not exist): {}", e);
                // Don't return error - this is best-effort
                Ok(())
            }
        }
    }

    /// Trigger degraded degradation
    fn trigger_degraded_degradation(&self) -> Result<(), i32> {
        let mut manager = self.degradation_manager.lock();
        
        // Trigger degradation using existing strategy
        let strategy_id = "health-degraded-strategy";
        
        match manager.trigger_degradation(
            strategy_id,
            "system",
            "Degraded health detected - system performance may be impacted",
        ) {
            Ok(session_id) => {
                crate::println!("[health-integration] Triggered degraded degradation: {}", session_id);
                Ok(())
            }
            Err(e) => {
                // Strategy might not exist, log and continue
                crate::println!("[health-integration] Failed to trigger degraded degradation (strategy may not exist): {}", e);
                // Don't return error - this is best-effort
                Ok(())
            }
        }
    }

    /// Trigger component-specific degradation
    fn trigger_component_degradation(&self, component: &str, severity: &str) -> Result<(), i32> {
        let mut manager = self.degradation_manager.lock();
        
        let strategy_id = format!("{}-health-{}-strategy", component, severity);
        
        match manager.trigger_degradation(
            &strategy_id,
            component,
            &format!("{} health issue detected (severity: {})", component, severity),
        ) {
            Ok(_) => Ok(()),
            Err(_) => {
                // Strategy might not exist, log and continue
                crate::println!("[health-integration] Failed to trigger component degradation for {} (strategy may not exist)", component);
                Ok(())
            }
        }
    }

    /// Check if system can recover from degradation
    fn check_recovery(&self) -> Result<(), i32> {
        let mut manager = self.degradation_manager.lock();
        
        // Check active degradations and attempt recovery
        // This is a simplified implementation - in production, this would
        // check recovery conditions and trigger recovery actions
        
        Ok(())
    }
}

/// Global health integration manager instance
static HEALTH_INTEGRATION_MANAGER: Mutex<Option<HealthIntegrationManager>> = Mutex::new(None);

/// Initialize health integration
pub fn init_health_integration() -> Result<(), i32> {
    let mut manager = HEALTH_INTEGRATION_MANAGER.lock();
    if manager.is_none() {
        *manager = Some(HealthIntegrationManager::new());
        crate::println!("[health-integration] Health monitoring integration initialized");
    }
    Ok(())
}

/// Get health integration manager
pub fn get_health_integration_manager() -> Option<&'static Mutex<HealthIntegrationManager>> {
    unsafe {
        // This is safe because we only access it after initialization
        if HEALTH_INTEGRATION_MANAGER.lock().is_some() {
            Some(&HEALTH_INTEGRATION_MANAGER)
        } else {
            None
        }
    }
}

/// Check health and react (convenience function)
pub fn check_health_and_react() -> Result<(), i32> {
    if let Some(manager_mutex) = get_health_integration_manager() {
        let mut manager = manager_mutex.lock();
        manager.check_and_react()
    } else {
        // Integration not initialized, just run health check
        let checker = get_health_checker();
        let _ = checker.check_health();
        Ok(())
    }
}

/// Trigger degradation from error-handling health monitor
/// This function is called when the error-handling module detects health issues
pub fn trigger_degradation_from_error_handling(component: &str, severity: &str, message: &str) -> Result<(), i32> {
    if let Some(manager_mutex) = get_health_integration_manager() {
        let manager = manager_mutex.lock();
        manager.trigger_component_degradation(component, severity)?;
        crate::println!("[health-integration] Triggered degradation from error-handling: {} - {} ({})", 
            component, message, severity);
        Ok(())
    } else {
        crate::println!("[health-integration] Warning: Health integration not initialized, cannot trigger degradation");
        Ok(())
    }
}

