//! Event-Driven State Management
//!
//! Provides state transition management driven by domain events.
//! This module implements a state machine pattern where state transitions
//! are triggered by domain events, enabling loose coupling between
//! boot components.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::vec;
use alloc::string::{String, ToString};
use alloc::boxed::Box;
use alloc::format;

use crate::domain::events::{
    DomainEvent, BootPhaseCompletedEvent,
    GraphicsInitializedEvent, KernelLoadedEvent
};

/// Boot state enumeration
///
/// Represents the various states of the boot process
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum BootState {
    /// Initial state - boot process not started
    Initial,
    /// Hardware detection in progress
    HardwareDetection,
    /// Memory initialization in progress
    MemoryInitialization,
    /// Graphics initialization in progress
    GraphicsInitialization,
    /// Kernel loading in progress
    KernelLoading,
    /// Kernel validation in progress
    KernelValidation,
    /// Boot parameter setup in progress
    BootParameterSetup,
    /// Ready to execute kernel
    ReadyForKernel,
    /// Boot completed successfully
    BootCompleted,
    /// Boot failed
    BootFailed,
    /// Recovery mode
    Recovery,
}

impl BootState {
    /// Get state name as string
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Initial => "Initial",
            Self::HardwareDetection => "HardwareDetection",
            Self::MemoryInitialization => "MemoryInitialization",
            Self::GraphicsInitialization => "GraphicsInitialization",
            Self::KernelLoading => "KernelLoading",
            Self::KernelValidation => "KernelValidation",
            Self::BootParameterSetup => "BootParameterSetup",
            Self::ReadyForKernel => "ReadyForKernel",
            Self::BootCompleted => "BootCompleted",
            Self::BootFailed => "BootFailed",
            Self::Recovery => "Recovery",
        }
    }

    /// Check if state is a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::BootCompleted | Self::BootFailed | Self::Recovery)
    }

    /// Check if state allows recovery
    pub fn allows_recovery(&self) -> bool {
        matches!(self, Self::BootFailed)
    }

    /// Get valid transitions from current state
    pub fn valid_transitions(&self) -> Vec<BootState> {
        match self {
            Self::Initial => vec![Self::HardwareDetection],
            Self::HardwareDetection => vec![Self::MemoryInitialization, Self::Recovery],
            Self::MemoryInitialization => vec![Self::GraphicsInitialization, Self::Recovery],
            Self::GraphicsInitialization => vec![Self::KernelLoading, Self::Recovery],
            Self::KernelLoading => vec![Self::KernelValidation, Self::Recovery],
            Self::KernelValidation => vec![Self::BootParameterSetup, Self::Recovery],
            Self::BootParameterSetup => vec![Self::ReadyForKernel, Self::Recovery],
            Self::ReadyForKernel => vec![Self::BootCompleted, Self::BootFailed],
            Self::BootCompleted => vec![],
            Self::BootFailed => vec![Self::Recovery],
            Self::Recovery => vec![Self::HardwareDetection],
        }
    }
}

/// State transition context
///
/// Provides context for state transitions
#[derive(Debug, Clone)]
pub struct StateTransitionContext {
    /// Previous state
    pub previous_state: BootState,
    /// Current state
    pub current_state: BootState,
    /// Event that triggered the transition
    pub triggering_event: String,
    /// Transition timestamp
    pub timestamp: u64,
    /// Additional metadata
    pub metadata: BTreeMap<String, String>,
}

impl StateTransitionContext {
    /// Create new state transition context
    ///
    /// # Arguments
    /// * `previous_state` - The previous state
    /// * `current_state` - The current state
    /// * `triggering_event` - The event that triggered the transition
    /// * `timestamp` - The transition timestamp
    ///
    /// # Returns
    /// New state transition context
    pub fn new(
        previous_state: BootState,
        current_state: BootState,
        triggering_event: String,
        timestamp: u64,
    ) -> Self {
        Self {
            previous_state,
            current_state,
            triggering_event,
            timestamp,
            metadata: BTreeMap::new(),
        }
    }

    /// Add metadata
    ///
    /// # Arguments
    /// * `key` - Metadata key
    /// * `value` - Metadata value
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// State transition rule
///
/// Defines how events trigger state transitions
pub struct StateTransitionRule {
    /// Target state
    pub target_state: BootState,
    /// Event types that trigger this transition
    pub triggering_events: Vec<&'static str>,
    /// Condition function for additional validation
    pub condition: Option<Box<dyn Fn(&dyn DomainEvent, &BootState) -> bool>>,
    /// Transition action
    pub action: Option<Box<dyn StateTransitionAction>>,
}

/// State transition action trait
///
/// Defines actions to execute during state transitions
pub trait StateTransitionAction: Send + Sync {
    /// Execute the transition action
    ///
    /// # Arguments
    /// * `context` - The transition context
    ///
    /// # Returns
    /// Result of the action execution
    fn execute(&self, context: &StateTransitionContext) -> Result<(), &'static str>;
}

/// Event-driven state manager
///
/// Manages boot state transitions driven by domain events
pub struct EventDrivenStateManager {
    /// Current boot state
    current_state: BootState,
    /// State transition history
    transition_history: Vec<StateTransitionContext>,
    /// State transition rules
    transition_rules: BTreeMap<BootState, Vec<StateTransitionRule>>,
    /// Maximum history size
    max_history_size: usize,
    /// Error recovery enabled
    error_recovery_enabled: bool,
}

impl EventDrivenStateManager {
    /// Create new event-driven state manager
    ///
    /// # Arguments
    /// * `max_history_size` - Maximum number of transitions to keep in history
    /// * `error_recovery_enabled` - Whether to enable automatic error recovery
    ///
    /// # Returns
    /// New event-driven state manager
    pub fn new(max_history_size: usize, error_recovery_enabled: bool) -> Self {
        let mut manager = Self {
            current_state: BootState::Initial,
            transition_history: Vec::new(),
            transition_rules: BTreeMap::new(),
            max_history_size,
            error_recovery_enabled,
        };
        
        manager.initialize_transition_rules();
        manager
    }

    /// Create state manager with default settings
    pub fn with_default_settings() -> Self {
        Self::new(100, true) // Default to 100 transitions, error recovery enabled
    }

    /// Initialize state transition rules
    fn initialize_transition_rules(&mut self) {
        // Hardware detection transitions
        self.transition_rules.insert(
            BootState::Initial,
            vec![
                StateTransitionRule {
                    target_state: BootState::HardwareDetection,
                    triggering_events: vec!["BootPhaseStarted"],
                    condition: None,
                    action: Some(Box::new(HardwareDetectionAction::default())),
                },
            ],
        );

        self.transition_rules.insert(
            BootState::HardwareDetection,
            vec![
                StateTransitionRule {
                    target_state: BootState::MemoryInitialization,
                    triggering_events: vec!["BootPhaseCompleted"],
                    condition: Some(Box::new(|event, _state| {
                        if let Some(phase_event) = event.as_any().downcast_ref::<BootPhaseCompletedEvent>() {
                            phase_event.phase_name == "hardware_detection" && phase_event.success
                        } else {
                            false
                        }
                    })),
                    action: Some(Box::new(MemoryInitializationAction::default())),
                },
                StateTransitionRule {
                    target_state: BootState::Recovery,
                    triggering_events: vec!["ValidationFailed", "SystemError"],
                    condition: None,
                    action: Some(Box::new(RecoveryAction::default())),
                },
            ],
        );

        // Memory initialization transitions
        self.transition_rules.insert(
            BootState::MemoryInitialization,
            vec![
                StateTransitionRule {
                    target_state: BootState::GraphicsInitialization,
                    triggering_events: vec!["BootPhaseCompleted"],
                    condition: Some(Box::new(|event, _state| {
                        if let Some(phase_event) = event.as_any().downcast_ref::<BootPhaseCompletedEvent>() {
                            phase_event.phase_name == "memory_initialization" && phase_event.success
                        } else {
                            false
                        }
                    })),
                    action: Some(Box::new(GraphicsInitializationAction::default())),
                },
                StateTransitionRule {
                    target_state: BootState::Recovery,
                    triggering_events: vec!["ValidationFailed", "SystemError"],
                    condition: None,
                    action: Some(Box::new(RecoveryAction::default())),
                },
            ],
        );

        // Graphics initialization transitions
        self.transition_rules.insert(
            BootState::GraphicsInitialization,
            vec![
                StateTransitionRule {
                    target_state: BootState::KernelLoading,
                    triggering_events: vec!["GraphicsInitialized"],
                    condition: Some(Box::new(|event, _state| {
                        matches!(event.as_any().downcast_ref::<GraphicsInitializedEvent>(), Some(_))
                    })),
                    action: Some(Box::new(KernelLoadingAction::default())),
                },
                StateTransitionRule {
                    target_state: BootState::Recovery,
                    triggering_events: vec!["ValidationFailed", "SystemError"],
                    condition: None,
                    action: Some(Box::new(RecoveryAction::default())),
                },
            ],
        );

        // Kernel loading transitions
        self.transition_rules.insert(
            BootState::KernelLoading,
            vec![
                StateTransitionRule {
                    target_state: BootState::KernelValidation,
                    triggering_events: vec!["KernelLoaded"],
                    condition: Some(Box::new(|event, _state| {
                        matches!(event.as_any().downcast_ref::<KernelLoadedEvent>(), Some(_))
                    })),
                    action: Some(Box::new(KernelValidationAction::default())),
                },
                StateTransitionRule {
                    target_state: BootState::Recovery,
                    triggering_events: vec!["ValidationFailed", "SystemError"],
                    condition: None,
                    action: Some(Box::new(RecoveryAction::default())),
                },
            ],
        );

        // Kernel validation transitions
        self.transition_rules.insert(
            BootState::KernelValidation,
            vec![
                StateTransitionRule {
                    target_state: BootState::BootParameterSetup,
                    triggering_events: vec!["BootPhaseCompleted"],
                    condition: Some(Box::new(|event, _state| {
                        if let Some(phase_event) = event.as_any().downcast_ref::<BootPhaseCompletedEvent>() {
                            phase_event.phase_name == "kernel_validation" && phase_event.success
                        } else {
                            false
                        }
                    })),
                    action: Some(Box::new(BootParameterSetupAction::default())),
                },
                StateTransitionRule {
                    target_state: BootState::Recovery,
                    triggering_events: vec!["ValidationFailed", "SystemError"],
                    condition: None,
                    action: Some(Box::new(RecoveryAction::default())),
                },
            ],
        );

        // Boot parameter setup transitions
        self.transition_rules.insert(
            BootState::BootParameterSetup,
            vec![
                StateTransitionRule {
                    target_state: BootState::ReadyForKernel,
                    triggering_events: vec!["BootPhaseCompleted"],
                    condition: Some(Box::new(|event, _state| {
                        if let Some(phase_event) = event.as_any().downcast_ref::<BootPhaseCompletedEvent>() {
                            phase_event.phase_name == "boot_parameter_setup" && phase_event.success
                        } else {
                            false
                        }
                    })),
                    action: Some(Box::new(ReadyForKernelAction::default())),
                },
                StateTransitionRule {
                    target_state: BootState::Recovery,
                    triggering_events: vec!["ValidationFailed", "SystemError"],
                    condition: None,
                    action: Some(Box::new(RecoveryAction::default())),
                },
            ],
        );

        // Ready for kernel transitions
        self.transition_rules.insert(
            BootState::ReadyForKernel,
            vec![
                StateTransitionRule {
                    target_state: BootState::BootCompleted,
                    triggering_events: vec!["BootPhaseCompleted"],
                    condition: Some(Box::new(|event, _state| {
                        if let Some(phase_event) = event.as_any().downcast_ref::<BootPhaseCompletedEvent>() {
                            phase_event.phase_name == "kernel_execution" && phase_event.success
                        } else {
                            false
                        }
                    })),
                    action: Some(Box::new(BootCompletedAction::default())),
                },
                StateTransitionRule {
                    target_state: BootState::BootFailed,
                    triggering_events: vec!["SystemError"],
                    condition: None,
                    action: Some(Box::new(BootFailedAction::default())),
                },
            ],
        );
    }

    /// Get current boot state
    pub fn current_state(&self) -> BootState {
        self.current_state
    }

    /// Get transition history
    pub fn transition_history(&self) -> &[StateTransitionContext] {
        &self.transition_history
    }

    /// Handle an event and potentially trigger state transition
    ///
    /// # Arguments
    /// * `event` - The domain event to handle
    ///
    /// # Returns
    /// Result indicating whether a state transition occurred
    pub fn handle_event(&mut self, event: &dyn DomainEvent) -> Result<(), &'static str> {
        let event_type = event.event_type();
        let timestamp = event.timestamp();
        
        // Get current state
        let current_state = self.current_state;
        
        // Check if there are transition rules for current state
        let matching_rule = if let Some(rules) = self.transition_rules.get(&current_state) {
            rules.iter().find(|rule| {
                // Check if event type matches rule
                if rule.triggering_events.contains(&event_type) {
                    // Check condition if present
                    rule.condition.as_ref().map_or(true, |condition| {
                        condition(event, &current_state)
                    })
                } else {
                    false
                }
            })
        } else {
            None
        };
        
        // If we found a matching rule, execute the transition
        if let Some(rule) = matching_rule {
            // Execute transition
            let previous_state = current_state;
            let target_state = rule.target_state;
            
            // Create transition context
            let mut context = StateTransitionContext::new(
                previous_state,
                target_state,
                format!("{}:{}", event_type, event.as_string()),
                timestamp,
            );
            
            // Add event metadata
            for (key, value) in event.metadata() {
                context = context.with_metadata(key.to_string(), value);
            }
            
            // Execute action if present
            let action_result = if let Some(ref action) = rule.action {
                action.execute(&context)
            } else {
                Ok(())
            };
            
            // Update state
            self.current_state = target_state;
            
            // Add to history
            self.add_to_history(context);
            
            // Check action result after all mutable operations
            action_result?;
            
            log::info!("State transition: {} -> {} triggered by {}", 
                     previous_state.as_str(), target_state.as_str(), event_type);
            
            return Ok(());
        }
        
        Ok(())
    }

    /// Force state transition
    ///
    /// # Arguments
    /// * `new_state` - The state to transition to
    /// * `reason` - Reason for the transition
    pub fn force_transition(&mut self, new_state: BootState, reason: &str) {
        let previous_state = self.current_state;
        self.current_state = new_state;
        
        let context = StateTransitionContext::new(
            previous_state,
            new_state,
            format!("forced: {}", reason),
            self.get_timestamp(),
        );
        
        self.add_to_history(context);
        
        log::warn!("Forced state transition: {} -> {} due to {}", 
                  previous_state.as_str(), new_state.as_str(), reason);
    }

    /// Check if transition is valid
    ///
    /// # Arguments
    /// * `from_state` - Current state
    /// * `to_state` - Target state
    ///
    /// # Returns
    /// True if transition is valid
    pub fn is_valid_transition(&self, from_state: BootState, to_state: BootState) -> bool {
        from_state.valid_transitions().contains(&to_state)
    }

    /// Get state transition statistics
    pub fn get_transition_stats(&self) -> StateTransitionStats {
        let mut stats = StateTransitionStats::new();
        
        for context in &self.transition_history {
            stats.add_transition(context.previous_state, context.current_state);
        }
        
        stats
    }

    /// Add transition to history
    fn add_to_history(&mut self, context: StateTransitionContext) {
        self.transition_history.push(context);
        
        // Maintain history size limit
        while self.transition_history.len() > self.max_history_size {
            self.transition_history.remove(0);
        }
    }

    /// Get current timestamp
    fn get_timestamp(&self) -> u64 {
        // In a real implementation, this would use a proper timer
        // For now, use a simple counter
        use core::sync::atomic::{AtomicU64, Ordering};
        static TIMESTAMP_COUNTER: AtomicU64 = AtomicU64::new(0);
        TIMESTAMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    /// Attempt automatic error recovery
    pub fn attempt_error_recovery(&mut self) -> Result<(), &'static str> {
        if !self.error_recovery_enabled {
            return Err("Error recovery is disabled");
        }
        
        if self.current_state.allows_recovery() {
            log::info!("Attempting error recovery from state: {}", self.current_state.as_str());
            
            // Force transition to recovery state
            self.force_transition(BootState::Recovery, "error recovery");
            
            // In a real implementation, this would trigger recovery procedures
            Ok(())
        } else {
            Err("Current state does not allow recovery")
        }
    }

    /// Reset state machine to initial state
    pub fn reset(&mut self) {
        self.current_state = BootState::Initial;
        self.transition_history.clear();
        log::info!("State machine reset to initial state");
    }
}

/// State transition statistics
#[derive(Debug, Clone)]
pub struct StateTransitionStats {
    /// Transition counts by state
    transition_counts: BTreeMap<BootState, u64>,
    /// Total transitions
    total_transitions: u64,
}

impl StateTransitionStats {
    /// Create new state transition statistics
    pub fn new() -> Self {
        Self {
            transition_counts: BTreeMap::new(),
            total_transitions: 0,
        }
    }

    /// Add transition to statistics
    pub fn add_transition(&mut self, from_state: BootState, to_state: BootState) {
        *self.transition_counts.entry(from_state).or_insert(0) += 1;
        self.total_transitions += 1;
        // Validate that the transition is allowed
        if !from_state.valid_transitions().contains(&to_state) {
            log::warn!("Invalid state transition attempted: {} -> {}", from_state.as_str(), to_state.as_str());
        }
    }
}

// State transition actions

struct HardwareDetectionAction;
impl StateTransitionAction for HardwareDetectionAction {
    fn execute(&self, context: &StateTransitionContext) -> Result<(), &'static str> {
        log::info!("Hardware detection action executed at timestamp: {}", context.timestamp);
        log::debug!("Transition event: {}", context.triggering_event);
        // Validate state transition is allowed
        if context.previous_state.valid_transitions().contains(&context.current_state) {
            log::trace!("State transition validated: {} -> {}", context.previous_state.as_str(), context.current_state.as_str());
            Ok(())
        } else {
            Err("Invalid state transition for hardware detection")
        }
    }
}
impl Default for HardwareDetectionAction {
    fn default() -> Self { Self }
}

struct MemoryInitializationAction;
impl StateTransitionAction for MemoryInitializationAction {
    fn execute(&self, context: &StateTransitionContext) -> Result<(), &'static str> {
        log::info!("Memory initialization action executed at timestamp: {}", context.timestamp);
        // Check if hardware detection completed successfully
        if context.previous_state == BootState::HardwareDetection {
            log::debug!("Memory init triggered by: {}", context.triggering_event);
            Ok(())
        } else {
            Err("Memory initialization must follow hardware detection")
        }
    }
}
impl Default for MemoryInitializationAction {
    fn default() -> Self { Self }
}

struct GraphicsInitializationAction;
impl StateTransitionAction for GraphicsInitializationAction {
    fn execute(&self, context: &StateTransitionContext) -> Result<(), &'static str> {
        log::info!("Graphics initialization action executed at timestamp: {}", context.timestamp);
        // Verify previous initialization steps completed
        if context.previous_state == BootState::MemoryInitialization {
            log::debug!("Graphics init triggered by event: {}", context.triggering_event);
            // Add metadata about graphics initialization
            if let Some(gpu_info) = context.metadata.get("gpu_vendor") {
                log::info!("GPU vendor: {}", gpu_info);
            }
            Ok(())
        } else {
            Err("Graphics initialization must follow memory initialization")
        }
    }
}
impl Default for GraphicsInitializationAction {
    fn default() -> Self { Self }
}

struct KernelLoadingAction;
impl StateTransitionAction for KernelLoadingAction {
    fn execute(&self, context: &StateTransitionContext) -> Result<(), &'static str> {
        log::info!("Kernel loading action started at timestamp: {}", context.timestamp);
        log::debug!("Event triggering kernel load: {}", context.triggering_event);
        // Check graphics initialization completed
        if context.previous_state == BootState::GraphicsInitialization {
            // Extract kernel path from context metadata if available
            if let Some(kernel_path) = context.metadata.get("kernel_path") {
                log::info!("Loading kernel from: {}", kernel_path);
            }
            Ok(())
        } else {
            Err("Kernel loading must follow graphics initialization")
        }
    }
}
impl Default for KernelLoadingAction {
    fn default() -> Self { Self }
}

struct KernelValidationAction;
impl StateTransitionAction for KernelValidationAction {
    fn execute(&self, context: &StateTransitionContext) -> Result<(), &'static str> {
        log::info!("Kernel validation action started at timestamp: {}", context.timestamp);
        // Verify kernel was loaded
        if context.previous_state == BootState::KernelLoading {
            log::debug!("Validating kernel loaded by: {}", context.triggering_event);
            // Check kernel information in metadata
            if let Some(checksum) = context.metadata.get("kernel_checksum") {
                log::info!("Kernel checksum: {}", checksum);
            }
            Ok(())
        } else {
            Err("Kernel validation must follow kernel loading")
        }
    }
}
impl Default for KernelValidationAction {
    fn default() -> Self { Self }
}

struct BootParameterSetupAction;
impl StateTransitionAction for BootParameterSetupAction {
    fn execute(&self, context: &StateTransitionContext) -> Result<(), &'static str> {
        log::info!("Boot parameter setup action started at timestamp: {}", context.timestamp);
        // Prepare boot parameters based on context
        if context.previous_state == BootState::KernelValidation {
            log::debug!("Setting up boot parameters triggered by: {}", context.triggering_event);
            // Extract boot parameters from metadata
            for (key, value) in &context.metadata {
                if key.starts_with("boot_param_") {
                    log::info!("Boot param {}: {}", key, value);
                }
            }
            Ok(())
        } else {
            Err("Boot parameter setup must follow kernel validation")
        }
    }
}
impl Default for BootParameterSetupAction {
    fn default() -> Self { Self }
}

struct ReadyForKernelAction;
impl StateTransitionAction for ReadyForKernelAction {
    fn execute(&self, context: &StateTransitionContext) -> Result<(), &'static str> {
        log::info!("Ready for kernel action started at timestamp: {}", context.timestamp);
        // Final verification before kernel handoff
        if context.previous_state == BootState::BootParameterSetup {
            log::debug!("Kernel ready after: {}", context.triggering_event);
            log::info!("All boot stages completed successfully, preparing for kernel handoff");
            // Log boot info if available
            if let Some(boot_info) = context.metadata.get("boot_info_addr") {
                log::info!("Boot info address: {}", boot_info);
            }
            Ok(())
        } else {
            Err("Ready for kernel state must follow boot parameter setup")
        }
    }
}
impl Default for ReadyForKernelAction {
    fn default() -> Self { Self }
}

struct BootCompletedAction;
impl StateTransitionAction for BootCompletedAction {
    fn execute(&self, context: &StateTransitionContext) -> Result<(), &'static str> {
        log::info!("Boot completed at timestamp: {}", context.timestamp);
        log::info!("Successful transition to kernel via: {}", context.triggering_event);
        log::info!("=== Boot process completed successfully ===");
        // Log final boot statistics
        log::debug!("Final state: {}", context.current_state.as_str());
        Ok(())
    }
}
impl Default for BootCompletedAction {
    fn default() -> Self { Self }
}

struct BootFailedAction;
impl StateTransitionAction for BootFailedAction {
    fn execute(&self, context: &StateTransitionContext) -> Result<(), &'static str> {
        log::error!("=== Boot failed at timestamp: {} ===", context.timestamp);
        log::error!("Boot failure triggered by: {}", context.triggering_event);
        log::error!("Failed at state: {}", context.previous_state.as_str());
        // Extract error information from metadata
        if let Some(error_code) = context.metadata.get("error_code") {
            log::error!("Error code: {}", error_code);
        }
        if let Some(error_msg) = context.metadata.get("error_message") {
            log::error!("Error message: {}", error_msg);
        }
        Ok(())
    }
}
impl Default for BootFailedAction {
    fn default() -> Self { Self }
}

struct RecoveryAction;
impl StateTransitionAction for RecoveryAction {
    fn execute(&self, context: &StateTransitionContext) -> Result<(), &'static str> {
        log::warn!("Recovery action executed: {}", context.triggering_event);
        Ok(())
    }
}
impl Default for RecoveryAction {
    fn default() -> Self { Self }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_state_transitions() {
        assert!(BootState::Initial.valid_transitions().contains(&BootState::HardwareDetection));
        assert!(!BootState::BootCompleted.valid_transitions().contains(&BootState::HardwareDetection));
        assert!(BootState::BootFailed.allows_recovery());
        assert!(!BootState::ReadyForKernel.allows_recovery());
    }

    #[test]
    fn test_state_manager_creation() {
        let manager = EventDrivenStateManager::new(10, true);
        assert_eq!(manager.current_state(), BootState::Initial);
        assert_eq!(manager.transition_history().len(), 0);
    }

    #[test]
    fn test_event_driven_transition() {
        let mut manager = EventDrivenStateManager::with_default_settings();
        
        // Start hardware detection
        let start_event = Box::new(BootPhaseStartedEvent::new("hardware_detection", 1000));
        assert!(manager.handle_event(start_event.as_ref()).is_ok());
        assert_eq!(manager.current_state(), BootState::HardwareDetection);
        
        // Complete hardware detection
        let complete_event = Box::new(BootPhaseCompletedEvent::new("hardware_detection", 2000, true));
        assert!(manager.handle_event(complete_event.as_ref()).is_ok());
        assert_eq!(manager.current_state(), BootState::MemoryInitialization);
    }

    #[test]
    fn test_error_recovery() {
        let mut manager = EventDrivenStateManager::new(10, true);
        
        // Force into failed state
        manager.force_transition(BootState::BootFailed, "test failure");
        assert_eq!(manager.current_state(), BootState::BootFailed);
        
        // Attempt recovery
        assert!(manager.attempt_error_recovery().is_ok());
        assert_eq!(manager.current_state(), BootState::Recovery);
    }

    #[test]
    fn test_transition_history() {
        let mut manager = EventDrivenStateManager::with_default_settings();
        
        // Perform some transitions
        manager.force_transition(BootState::HardwareDetection, "test1");
        manager.force_transition(BootState::MemoryInitialization, "test2");
        manager.force_transition(BootState::GraphicsInitialization, "test3");
        
        let history = manager.transition_history();
        assert_eq!(history.len(), 3);
        
        let stats = manager.get_transition_stats();
        assert_eq!(stats.total_transitions, 3);
    }

    #[test]
    fn test_invalid_transition() {
        let mut manager = EventDrivenStateManager::with_default_settings();
        
        // Try invalid transition
        assert!(!manager.is_valid_transition(BootState::Initial, BootState::KernelLoading));
        
        // Force transition should still work
        manager.force_transition(BootState::KernelLoading, "forced");
        assert_eq!(manager.current_state(), BootState::KernelLoading);
    }
}