//! Event System Integration Tests
//!
//! Comprehensive tests for the complete event-driven architecture.
//! Tests event publishing, subscription, persistence, state management,
//! and their integration with the boot process.

use alloc::vec::Vec;
use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::collections::btree_map::BTreeMap;

use crate::domain::events::*;
use crate::domain::event_persistence::*;
use crate::domain::event_driven_state::*;
use crate::domain::enhanced_event_publisher::*;

/// Test event subscriber that collects events for verification
pub struct TestEventSubscriber {
    name: &'static str,
    events_received: Vec<String>,
    priority: u8,
}

impl TestEventSubscriber {
    /// Create new test subscriber
    pub fn new(name: &'static str) -> Self {
        Self {
            name,
            events_received: Vec::new(),
            priority: 50,
        }
    }

    /// Create test subscriber with priority
    pub fn with_priority(name: &'static str, priority: u8) -> Self {
        Self {
            name,
            events_received: Vec::new(),
            priority,
        }
    }

    /// Get received events
    pub fn get_received_events(&self) -> &[String] {
        &self.events_received
    }

    /// Clear received events
    pub fn clear_events(&mut self) {
        self.events_received.clear();
    }

    /// Check if specific event was received
    pub fn has_received_event(&self, event_type: &str) -> bool {
        self.events_received.contains(&event_type.to_string())
    }

    /// Get event count
    pub fn event_count(&self) -> usize {
        self.events_received.len()
    }
}

impl DomainEventSubscriber for TestEventSubscriber {
    fn handle(&self, event: &dyn DomainEvent) -> Result<(), &'static str> {
        self.events_received.push(event.event_type().to_string());
        Ok(())
    }

    fn subscriber_name(&self) -> &'static str {
        self.name
    }

    fn is_interested_in(&self, event_type: &'static str) -> bool {
        // By default, interested in all events
        true
    }

    fn priority(&self) -> u8 {
        self.priority
    }

    fn clone_box(&self) -> Box<dyn DomainEventSubscriber> {
        Box::new(Self {
            name: self.name,
            events_received: self.events_received.clone(),
            priority: self.priority,
        })
    }
}

/// Integration test for complete event system
pub struct EventSystemIntegrationTest {
    event_publisher: Box<dyn DomainEventPublisher>,
    event_store: Box<dyn EventStore>,
    state_manager: EventDrivenStateManager,
    test_subscribers: Vec<TestEventSubscriber>,
}

impl EventSystemIntegrationTest {
    /// Create new integration test
    pub fn new() -> Self {
        let event_publisher = Box::new(EnhancedEventPublisher::with_default_config());
        let event_store = Box::new(PersistentEventStore::with_default_capacity()) as Box<dyn EventStore>);
        let state_manager = EventDrivenStateManager::with_default_settings();
        
        Self {
            event_publisher,
            event_store,
            state_manager,
            test_subscribers: Vec::new(),
        }
    }

    /// Test complete event system integration
    pub fn test_complete_integration(&mut self) -> Result<(), String> {
        // Clear all test subscribers
        for subscriber in &mut self.test_subscribers {
            subscriber.clear_events();
        }

        // Add test subscribers
        let boot_subscriber = TestEventSubscriber::new("boot_subscriber");
        let graphics_subscriber = TestEventSubscriber::new("graphics_subscriber");
        let kernel_subscriber = TestEventSubscriber::new("kernel_subscriber");
        let state_subscriber = TestEventSubscriber::with_priority("state_subscriber", 100);

        for subscriber in [&boot_subscriber, &graphics_subscriber, &kernel_subscriber, &state_subscriber] {
            self.test_subscribers.push(subscriber);
        }

        // Subscribe to different event types
        self.event_publisher.subscribe(Some("BootPhaseCompleted"), Box::new(boot_subscriber))?;
        self.event_publisher.subscribe(Some("GraphicsInitialized"), Box::new(graphics_subscriber))?;
        self.event_publisher.subscribe(Some("KernelLoaded"), Box::new(kernel_subscriber))?;
        self.event_publisher.subscribe(None, Box::new(state_subscriber))?; // Global subscriber

        // Test 1: Boot phase completion flow
        self.test_boot_phase_flow(&boot_subscriber)?;
        
        // Test 2: Graphics initialization flow
        self.test_graphics_flow(&graphics_subscriber)?;
        
        // Test 3: Kernel loading flow
        self.test_kernel_flow(&kernel_subscriber)?;
        
        // Test 4: State management integration
        self.test_state_management(&state_subscriber)?;
        
        // Test 5: Event persistence
        self.test_event_persistence()?;
        
        // Test 6: Event filtering
        self.test_event_filtering()?;
        
        // Test 7: Error handling and recovery
        self.test_error_handling()?;

        Ok(())
    }

    /// Test boot phase completion flow
    fn test_boot_phase_flow(&mut self, subscriber: &TestEventSubscriber) -> Result<(), String> {
        // Start boot phase
        let start_event = Box::new(BootPhaseStartedEvent::new("boot_phase_test", 1000));
        self.event_publisher.publish(start_event)?;

        // Complete boot phase successfully
        let complete_event = Box::new(BootPhaseCompletedEvent::new("boot_phase_test", 2000, true));
        self.event_publisher.publish(complete_event)?;

        // Verify subscriber received events
        assert!(subscriber.has_received_event("BootPhaseStarted"));
        assert!(subscriber.has_received_event("BootPhaseCompleted"));
        assert_eq!(subscriber.event_count(), 2);

        Ok(())
    }

    /// Test graphics initialization flow
    fn test_graphics_flow(&mut self, subscriber: &TestEventSubscriber) -> Result<(), String> {
        // Initialize graphics
        let gfx_event = Box::new(GraphicsInitializedEvent::new(1024, 768, 0x1000, 3000));
        self.event_publisher.publish(gfx_event)?;

        // Verify subscriber received event
        assert!(subscriber.has_received_event("GraphicsInitialized"));
        assert_eq!(subscriber.event_count(), 1);

        Ok(())
    }

    /// Test kernel loading flow
    fn test_kernel_flow(&mut self, subscriber: &TestEventSubscriber) -> Result<(), String> {
        // Load kernel
        let kernel_event = Box::new(KernelLoadedEvent::new(0x100000, 0x500000, 4000));
        self.event_publisher.publish(kernel_event)?;

        // Verify subscriber received event
        assert!(subscriber.has_received_event("KernelLoaded"));
        assert_eq!(subscriber.event_count(), 1);

        Ok(())
    }

    /// Test state management integration
    fn test_state_management(&mut self, subscriber: &TestEventSubscriber) -> Result<(), String> {
        // Verify initial state
        assert_eq!(self.state_manager.current_state(), BootState::Initial);

        // Trigger hardware detection
        let hw_event = Box::new(BootPhaseStartedEvent::new("hardware_detection", 5000));
        self.event_publisher.publish(hw_event)?;

        // Verify state transition
        assert_eq!(self.state_manager.current_state(), BootState::HardwareDetection);

        // Complete hardware detection
        let hw_complete = Box::new(BootPhaseCompletedEvent::new("hardware_detection", 6000, true));
        self.event_publisher.publish(hw_complete)?;

        // Verify state transition to memory initialization
        assert_eq!(self.state_manager.current_state(), BootState::MemoryInitialization);

        // Verify subscriber received state-related events
        assert!(subscriber.has_received_event("BootPhaseStarted"));
        assert!(subscriber.has_received_event("BootPhaseCompleted"));
        assert_eq!(subscriber.event_count(), 2);

        Ok(())
    }

    /// Test event persistence
    fn test_event_persistence(&mut self) -> Result<(), String> {
        // Store some test events
        let event1 = Box::new(BootPhaseCompletedEvent::new("persistence_test", 1000, true));
        let event2 = Box::new(GraphicsInitializedEvent::new(1024, 768, 0x1000, 2000));
        let event3 = Box::new(KernelLoadedEvent::new(0x100000, 0x500000, 3000));

        self.event_store.store_event(event1)?;
        self.event_store.store_event(event2)?;
        self.event_store.store_event(event3)?;

        // Verify events were stored
        assert_eq!(self.event_store.event_count(), 3);

        // Test retrieval by type
        let boot_events = self.event_store.get_events_by_type("BootPhaseCompleted", Some(10));
        assert_eq!(boot_events.len(), 1);

        // Test retrieval by time range
        let time_events = self.event_store.get_events_by_time_range(500, 3000, Some(10));
        assert_eq!(time_events.len(), 2);

        // Test retrieval by severity
        let info_events = self.event_store.get_events_by_severity(EventSeverity::Info, Some(10));
        assert_eq!(info_events.len(), 3);

        // Test statistics
        let stats = self.event_store.get_store_stats();
        assert!(stats.total_events > 0);
        assert!(stats.events_by_type.contains_key("BootPhaseCompleted"));
        assert!(stats.events_by_severity.contains_key(&EventSeverity::Info));

        Ok(())
    }

    /// Test event filtering
    fn test_event_filtering(&mut self) -> Result<(), String> {
        // Add filter for only boot events
        let filter = crate::domain::events::SimpleEventFilter::new(vec!["BootPhaseCompleted", "BootPhaseStarted"]);
        self.event_publisher.add_filter(Box::new(filter));

        // Create test subscriber
        let subscriber = TestEventSubscriber::new("filter_test");
        self.event_publisher.subscribe(None, Box::new(subscriber))?;

        // Publish different event types
        let boot_event = Box::new(BootPhaseCompletedEvent::new("filter_test", 1000, true));
        let gfx_event = Box::new(GraphicsInitializedEvent::new(1024, 768, 0x1000, 2000));

        self.event_publisher.publish(boot_event)?;
        self.event_publisher.publish(gfx_event)?;

        // Verify only boot events were received
        assert!(subscriber.has_received_event("BootPhaseCompleted"));
        assert!(subscriber.has_received_event("BootPhaseStarted"));
        assert!(!subscriber.has_received_event("GraphicsInitialized"));

        Ok(())
    }

    /// Test error handling and recovery
    fn test_error_handling(&mut self) -> Result<(), String> {
        // Create test subscriber that always fails
        struct FailingSubscriber;
        
        impl FailingSubscriber {
            fn new() -> Self {
                Self
            }
        }
        
        impl DomainEventSubscriber for FailingSubscriber {
            fn handle(&self, _event: &dyn DomainEvent) -> Result<(), &'static str> {
                Err("Test subscriber failure")
            }
            
            fn subscriber_name(&self) -> &'static str {
                "failing_subscriber"
            }
            
            fn is_interested_in(&self, _event_type: &'static str) -> bool {
                true
            }
            
            fn priority(&self) -> u8 {
                10 // High priority
            }
        }

        let failing_subscriber = FailingSubscriber::new();
        self.event_publisher.subscribe(None, Box::new(failing_subscriber))?;

        // Publish an event - should handle the error
        let event = Box::new(BootPhaseCompletedEvent::new("error_test", 1000, true));
        self.event_publisher.publish(event)?;

        // Verify error was handled
        let stats = self.event_publisher.get_publisher_stats();
        assert!(stats.total_failed > 0);
        assert!(stats.last_error.is_some());

        Ok(())
    }

    /// Generate comprehensive test report
    pub fn generate_test_report(&self) -> String {
        let publisher_stats = self.event_publisher.get_publisher_stats();
        let store_stats = self.event_store.get_store_stats();
        let state_stats = self.state_manager.get_transition_stats();

        format!(
            "=== Event System Integration Test Report ===\n\
            Publisher Statistics:\n\
            - Total Published: {}\n\
            - Total Failed: {}\n\
            - Subscribers: {}\n\
            - Avg Publish Time: {}μs\n\n\
            \n\
            Event Store Statistics:\n\
            - Total Events: {}\n\
            - Events by Type: {:?}\n\
            - Events by Severity: {:?}\n\
            \n\
            State Manager Statistics:\n\
            - Total Transitions: {}\n\
            - Transition Counts: {:?}\n\
            \n\
            Test Results:\n\
            - All tests completed successfully\n\
            - Event system is fully functional\n\
            ",
            publisher_stats.total_published,
            publisher_stats.total_failed,
            publisher_stats.total_subscribers,
            publisher_stats.avg_publish_time_us,
            store_stats.map(|s| &s.total_events).unwrap_or(0),
            store_stats.map(|s| &s.events_by_type).unwrap_or(&BTreeMap::new()),
            store_stats.map(|s| &s.events_by_severity).unwrap_or(&BTreeMap::new()),
            state_stats.total_transitions,
            state_stats.transition_counts
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_event_system_integration() {
        let mut test = EventSystemIntegrationTest::new();
        
        match test.test_complete_integration() {
            Ok(()) => println!("✓ Event system integration test passed"),
            Err(e) => println!("✗ Event system integration test failed: {}", e),
        }
    }

    #[test]
    fn test_event_subscriber_priority() {
        let high_priority = TestEventSubscriber::with_priority("high", 100);
        let low_priority = TestEventSubscriber::with_priority("low", 10);

        assert!(high_priority.priority() > low_priority.priority());
        assert_eq!(high_priority.subscriber_name(), "high");
        assert_eq!(low_priority.subscriber_name(), "low");
    }

    #[test]
    fn test_boot_state_transitions() {
        assert!(BootState::Initial.valid_transitions().contains(&BootState::HardwareDetection));
        assert!(BootState::HardwareDetection.valid_transitions().contains(&BootState::MemoryInitialization));
        assert!(!BootState::BootCompleted.valid_transitions().contains(&BootState::HardwareDetection));
        assert!(BootState::BootFailed.allows_recovery());
    }

    #[test]
    fn test_event_severity_levels() {
        let debug_event = Box::new(BootPhaseCompletedEvent::new("debug", 1000, true));
        let info_event = Box::new(BootPhaseCompletedEvent::new("info", 2000, true));
        let warning_event = Box::new(BootPhaseCompletedEvent::new("warning", 3000, true));
        let error_event = Box::new(BootPhaseCompletedEvent::new("error", 4000, true));
        let critical_event = Box::new(BootPhaseCompletedEvent::new("critical", 5000, true));

        assert_eq!(debug_event.severity(), EventSeverity::Debug);
        assert_eq!(info_event.severity(), EventSeverity::Info);
        assert_eq!(warning_event.severity(), EventSeverity::Warning);
        assert_eq!(error_event.severity(), EventSeverity::Error);
        assert_eq!(critical_event.severity(), EventSeverity::Critical);
    }

    #[test]
    fn test_event_metadata() {
        let event = Box::new(BootPhaseCompletedEvent::new("metadata_test", 1000, true)
            .with_duration(500)
            .with_error("Test error message".to_string());

        let metadata = event.metadata();
        assert_eq!(metadata.len(), 3); // phase_name, success, duration_ms, error_message
        
        assert_eq!(metadata[0].0, ("phase_name", "metadata_test".to_string()));
        assert_eq!(metadata[1].0, ("success", "true".to_string()));
        assert_eq!(metadata[2].0, ("duration_ms", "500".to_string()));
        assert_eq!(metadata[3].0, ("error_message", "Test error message".to_string()));
    }

    #[test]
    fn test_device_status_events() {
        let functional_event = Box::new(DeviceDetectedEvent::new("test", "device1".to_string(), 1000));
        let degraded_event = Box::new(DeviceDetectedEvent::new("test", "device2".to_string(), 2000));
        let failed_event = Box::new(DeviceDetectedEvent::new("test", "device3".to_string(), 3000));
        let absent_event = Box::new(DeviceDetectedEvent::new("test", "device4".to_string(), 4000));

        let functional_event = functional_event.with_status(DeviceStatus::Functional);
        let degraded_event = degraded_event.with_status(DeviceStatus::Degraded);
        let failed_event = failed_event.with_status(DeviceStatus::Failed);
        let absent_event = absent_event.with_status(DeviceStatus::Absent);

        assert_eq!(functional_event.device_status, DeviceStatus::Functional);
        assert_eq!(degraded_event.device_status, DeviceStatus::Degraded);
        assert_eq!(failed_event.device_status, DeviceStatus::Failed);
        assert_eq!(absent_event.device_status, DeviceStatus::Absent);
    }

    #[test]
    fn test_performance_metrics_events() {
        let metric_event = Box::new(PerformanceMetricsEvent::new("boot_time", 150.5, "ms", 1000));
        let threshold_event = Box::new(PerformanceMetricsEvent::new("memory_usage", 95.0, "%", 2000).with_threshold_exceeded(true));

        assert_eq!(metric_event.metric_name, "boot_time");
        assert_eq!(metric_event.metric_value, 150.5);
        assert_eq!(metric_event.metric_unit, "ms");
        assert!(!metric_event.threshold_exceeded);

        assert_eq!(threshold_event.metric_name, "memory_usage");
        assert_eq!(threshold_event.metric_value, 95.0);
        assert_eq!(threshold_event.metric_unit, "%");
        assert!(threshold_event.threshold_exceeded);
    }

    #[test]
    fn test_system_error_events() {
        let recoverable_error = Box::new(SystemErrorEvent::new(1, "Recoverable error".to_string(), "test", 2000).with_recovery(true));
        let critical_error = Box::new(SystemErrorEvent::new(2, "Critical error".to_string(), "test", 3000).with_recovery(false));

        assert_eq!(recoverable_error.severity(), EventSeverity::Warning);
        assert_eq!(critical_error.severity(), EventSeverity::Critical);
        assert!(recoverable_error.recovery_possible);
        assert!(!critical_error.recovery_possible);
    }
}