//! Fundamental traits used throughout the NOS operating system

use crate::error::Result;

/// Core trait for all services in the system
pub trait Service {
    /// Returns the unique name of the service
    fn name(&self) -> &str;
    
    /// Returns the unique identifier of the service
    fn service_id(&self) -> &str {
        self.name()
    }
    
    /// Returns the version of the service
    fn version(&self) -> &str {
        "0.1.0"
    }
    
    /// Initializes the service
    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }
    
    /// Shuts down the service
    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

/// Trait for components that can be started and stopped
pub trait Startable {
    /// Starts the component
    fn start(&mut self) -> Result<()>;
    
    /// Stops the component
    fn stop(&mut self) -> Result<()>;
    
    /// Returns true if the component is running
    fn is_running(&self) -> bool;
}

/// Trait for components that can be configured
pub trait Configurable {
    /// Configuration type
    type Config;
    
    /// Configure the component with the given configuration
    fn configure(&mut self, config: Self::Config) -> Result<()>;
    
    /// Returns the current configuration
    fn config(&self) -> &Self::Config;
}

/// Trait for components that can be reset
pub trait Resettable {
    /// Resets the component to its initial state
    fn reset(&mut self) -> Result<()>;
}

/// Trait for components that can provide status information
pub trait StatusProvider {
    /// Status type
    type Status;
    
    /// Returns the current status
    fn status(&self) -> Self::Status;
}

/// Trait for resource management
pub trait ResourceManager {
    /// Resource type
    type Resource;
    
    /// Acquires a resource
    fn acquire(&mut self) -> Result<Self::Resource>;
    
    /// Releases a resource
    fn release(&mut self, resource: Self::Resource) -> Result<()>;
}

/// Trait for event handling
pub trait EventHandler {
    /// Event type
    type Event;
    
    /// Handles an event
    fn handle(&mut self, event: Self::Event) -> Result<()>;
}

/// Trait for logging
pub trait Logger {
    /// Logs a message with the specified level
    fn log(&self, level: LogLevel, message: &str);
    
    /// Logs a debug message
    fn debug(&self, message: &str) {
        self.log(LogLevel::Debug, message);
    }
    
    /// Logs an info message
    fn info(&self, message: &str) {
        self.log(LogLevel::Info, message);
    }
    
    /// Logs a warning message
    fn warn(&self, message: &str) {
        self.log(LogLevel::Warn, message);
    }
    
    /// Logs an error message
    fn error(&self, message: &str) {
        self.log(LogLevel::Error, message);
    }
}

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3,
}