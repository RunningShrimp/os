//! Retry logic with exponential backoff
//!
//! This module provides comprehensive retry mechanisms including exponential
//! backoff, jitter, and configurable retry policies.

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;
use core::time::Duration;

/// Unique identifier for a retry attempt
pub type AttemptId = u64;

/// Retry strategy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryStrategy {
    /// No retry
    None,
    /// Fixed delay between retries
    Fixed,
    /// Exponential backoff
    Exponential,
    /// Exponential backoff with jitter
    ExponentialWithJitter,
    /// Linear backoff
    Linear,
    /// Custom retry strategy
    Custom,
}

/// Retry configuration
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Maximum delay between retries
    pub max_delay: Option<Duration>,
    /// Multiplier for exponential backoff
    pub multiplier: f64,
    /// Jitter factor (0.0 to 1.0)
    pub jitter_factor: f64,
    /// Retry strategy
    pub strategy: RetryStrategy,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(1000),
            max_delay: Some(Duration::from_secs(60)),
            multiplier: 2.0,
            jitter_factor: 0.1,
            strategy: RetryStrategy::Exponential,
        }
    }
}

impl RetryConfig {
    /// Create a new retry config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum attempts
    pub fn with_max_attempts(mut self, max: u32) -> Self {
        self.max_attempts = max;
        self
    }

    /// Set initial delay
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Set maximum delay
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = Some(delay);
        self
    }

    /// Set multiplier for exponential backoff
    pub fn with_multiplier(mut self, multiplier: f64) -> Self {
        self.multiplier = multiplier;
        self
    }

    /// Set jitter factor
    pub fn with_jitter(mut self, jitter: f64) -> Self {
        self.jitter_factor = jitter.clamp(0.0, 1.0);
        self
    }

    /// Set retry strategy
    pub fn with_strategy(mut self, strategy: RetryStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Create exponential backoff config
    pub fn exponential(max_attempts: u32, initial_delay: Duration) -> Self {
        Self {
            max_attempts,
            initial_delay,
            max_delay: Some(Duration::from_secs(60)),
            multiplier: 2.0,
            jitter_factor: 0.0,
            strategy: RetryStrategy::Exponential,
        }
    }

    /// Create exponential backoff with jitter config
    pub fn exponential_with_jitter(
        max_attempts: u32,
        initial_delay: Duration,
        jitter: f64,
    ) -> Self {
        Self {
            max_attempts,
            initial_delay,
            max_delay: Some(Duration::from_secs(60)),
            multiplier: 2.0,
            jitter_factor: jitter.clamp(0.0, 1.0),
            strategy: RetryStrategy::ExponentialWithJitter,
        }
    }

    /// Create fixed delay config
    pub fn fixed(max_attempts: u32, delay: Duration) -> Self {
        Self {
            max_attempts,
            initial_delay: delay,
            max_delay: Some(delay),
            multiplier: 1.0,
            jitter_factor: 0.0,
            strategy: RetryStrategy::Fixed,
        }
    }

    /// Create linear backoff config
    pub fn linear(max_attempts: u32, initial_delay: Duration, increment: Duration) -> Self {
        Self {
            max_attempts,
            initial_delay,
            max_delay: Some(Duration::from_secs(60)),
            multiplier: increment.as_millis() as f64 / initial_delay.as_millis() as f64,
            jitter_factor: 0.0,
            strategy: RetryStrategy::Linear,
        }
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), RetryError> {
        if self.max_attempts == 0 {
            return Err(RetryError::InvalidConfig("max_attempts must be > 0".to_string()));
        }

        if self.initial_delay.as_millis() == 0 {
            return Err(RetryError::InvalidConfig("initial_delay must be > 0".to_string()));
        }

        if self.multiplier <= 0.0 {
            return Err(RetryError::InvalidConfig("multiplier must be > 0".to_string()));
        }

        if self.jitter_factor < 0.0 || self.jitter_factor > 1.0 {
            return Err(RetryError::InvalidConfig("jitter_factor must be in [0, 1]".to_string()));
        }

        if let Some(max_delay) = self.max_delay {
            if max_delay < self.initial_delay {
                return Err(RetryError::InvalidConfig(
                    "max_delay must be >= initial_delay".to_string()
                ));
            }
        }

        Ok(())
    }
}

/// Retry errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RetryError {
    /// Maximum retries exceeded
    MaxRetriesExceeded { attempts: u32 },
    /// Invalid configuration
    InvalidConfig(String),
    /// Retry cancelled
    Cancelled,
    /// Retry timeout
    Timeout,
}

impl fmt::Display for RetryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MaxRetriesExceeded { attempts } => {
                write!(f, "Maximum retries exceeded: {} attempts", attempts)
            }
            Self::InvalidConfig(msg) => {
                write!(f, "Invalid retry configuration: {}", msg)
            }
            Self::Cancelled => {
                write!(f, "Retry cancelled")
            }
            Self::Timeout => {
                write!(f, "Retry timeout")
            }
        }
    }
}

/// Result type for retry operations
pub type RetryResult<T> = Result<T, RetryError>;

/// Information about a retry attempt
#[derive(Debug, Clone)]
pub struct RetryAttempt {
    /// Attempt number (0-indexed)
    pub attempt_number: u32,
    /// Delay before this attempt
    pub delay: Duration,
    /// Timestamp of the attempt
    pub timestamp: u64,
    /// Whether the attempt succeeded
    pub succeeded: bool,
    /// Error message if failed
    pub error: Option<String>,
}

impl RetryAttempt {
    pub fn new(attempt_number: u32, delay: Duration, timestamp: u64) -> Self {
        Self {
            attempt_number,
            delay,
            timestamp,
            succeeded: false,
            error: None,
        }
    }

    pub fn with_success(mut self) -> Self {
        self.succeeded = true;
        self
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }
}

/// Retry history
#[derive(Debug, Clone)]
pub struct RetryHistory {
    /// Configuration used
    pub config: RetryConfig,
    /// All attempts made
    pub attempts: Vec<RetryAttempt>,
    /// Total time spent retrying
    pub total_duration: Duration,
    /// Final outcome
    pub outcome: RetryOutcome,
}

/// Outcome of retry attempts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetryOutcome {
    /// All attempts succeeded
    Success,
    /// All attempts failed
    Failed,
    /// Retry was cancelled
    Cancelled,
    /// Retry timed out
    TimedOut,
}

/// Retry executor
pub struct RetryExecutor {
    config: RetryConfig,
    history: RetryHistory,
    current_attempt: u32,
}

impl RetryExecutor {
    /// Create a new retry executor
    pub fn new(config: RetryConfig) -> Result<Self, RetryError> {
        config.validate()?;
        let config_clone = config.clone();
        Ok(Self {
            config,
            history: RetryHistory {
                config: config_clone,
                attempts: Vec::new(),
                total_duration: Duration::ZERO,
                outcome: RetryOutcome::Failed,
            },
            current_attempt: 0,
        })
    }

    /// Execute a function with retry logic
    pub fn execute<F, T>(&mut self, mut operation: F) -> RetryResult<T>
    where
        F: FnMut(u32) -> Result<T, String>,
    {
        loop {
            let attempt_number = self.current_attempt;
            let delay = self.calculate_delay(attempt_number);

            // Record attempt
            let mut attempt = RetryAttempt::new(attempt_number, delay, 0);

            // Execute the operation
            match operation(attempt_number) {
                Ok(result) => {
                    attempt = attempt.with_success();
                    self.history.attempts.push(attempt);
                    self.history.outcome = RetryOutcome::Success;
                    return Ok(result);
                }
                Err(error) => {
                    attempt = attempt.with_error(error.clone());

                    // Check if we should retry
                    if attempt_number >= self.config.max_attempts - 1 {
                        self.history.attempts.push(attempt);
                        self.history.outcome = RetryOutcome::Failed;
                        return Err(RetryError::MaxRetriesExceeded {
                            attempts: self.config.max_attempts,
                        });
                    }

                    self.history.attempts.push(attempt);
                    self.current_attempt += 1;

                    // Calculate next delay
                    let next_delay = self.calculate_delay(self.current_attempt);
                    self.history.total_duration += next_delay;

                    // In a real implementation, we would sleep here
                    // For now, just continue
                }
            }
        }
    }

    /// Calculate delay for a given attempt
    fn calculate_delay(&self, attempt: u32) -> Duration {
        let base_delay = match self.config.strategy {
            RetryStrategy::None => Duration::ZERO,
            RetryStrategy::Fixed => self.config.initial_delay,
            RetryStrategy::Exponential => {
                let millis = self.config.initial_delay.as_millis() as f64
                    * self.config.multiplier.powi(attempt as i32);
                Duration::from_millis(millis as u64)
            }
            RetryStrategy::ExponentialWithJitter => {
                let base_millis = self.config.initial_delay.as_millis() as f64
                    * self.config.multiplier.powi(attempt as i32);

                // Add jitter
                let jitter_range = base_millis * self.config.jitter_factor;
                let jitter = (rand64() % ((jitter_range * 2.0) as u64)) as f64 - jitter_range;

                let millis = base_millis + jitter;
                Duration::from_millis(millis.max(0.0) as u64)
            }
            RetryStrategy::Linear => {
                let millis = self.config.initial_delay.as_millis() as u64
                    + (self.config.initial_delay.as_millis() as f64
                        * self.config.multiplier * attempt as f64) as u64;
                Duration::from_millis(millis)
            }
            RetryStrategy::Custom => self.config.initial_delay,
        };

        // Apply max delay cap
        if let Some(max_delay) = self.config.max_delay {
            base_delay.min(max_delay)
        } else {
            base_delay
        }
    }

    /// Get retry history
    pub fn history(&self) -> &RetryHistory {
        &self.history
    }

    /// Get current attempt number
    pub fn current_attempt(&self) -> u32 {
        self.current_attempt
    }

    /// Check if more retries are available
    pub fn can_retry(&self) -> bool {
        self.current_attempt < self.config.max_attempts
    }

    /// Reset the executor
    pub fn reset(&mut self) {
        self.current_attempt = 0;
        self.history = RetryHistory {
            config: self.config.clone(),
            attempts: Vec::new(),
            total_duration: Duration::ZERO,
            outcome: RetryOutcome::Failed,
        };
    }
}

/// Simple random number generator for jitter
fn rand64() -> u64 {
    // Simple LCG - in real implementation would use proper RNG
    static mut STATE: u64 = 1;
    unsafe {
        STATE = STATE.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        STATE
    }
}

/// Retry state machine for managing retry logic
pub struct RetryStateMachine {
    config: RetryConfig,
    attempt: u32,
    state: RetryState,
    last_attempt_time: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RetryState {
    Idle,
    Waiting,
    Attempting,
    Completed,
    Failed,
}

impl RetryStateMachine {
    /// Create a new retry state machine
    pub fn new(config: RetryConfig) -> Result<Self, RetryError> {
        config.validate()?;
        Ok(Self {
            config,
            attempt: 0,
            state: RetryState::Idle,
            last_attempt_time: None,
        })
    }

    /// Start retry process
    pub fn start(&mut self) -> Result<Duration, RetryError> {
        if self.attempt >= self.config.max_attempts {
            self.state = RetryState::Failed;
            return Err(RetryError::MaxRetriesExceeded {
                attempts: self.attempt,
            });
        }

        self.state = RetryState::Waiting;
        Ok(self.calculate_delay())
    }

    /// Record a failed attempt
    pub fn record_failure(&mut self, _error: String) -> Result<Duration, RetryError> {
        self.attempt += 1;

        if self.attempt >= self.config.max_attempts {
            self.state = RetryState::Failed;
            return Err(RetryError::MaxRetriesExceeded {
                attempts: self.attempt,
            });
        }

        self.state = RetryState::Waiting;
        Ok(self.calculate_delay())
    }

    /// Record a successful attempt
    pub fn record_success(&mut self) {
        self.state = RetryState::Completed;
    }

    /// Calculate next retry delay
    fn calculate_delay(&self) -> Duration {
        let base_delay = match self.config.strategy {
            RetryStrategy::Exponential => {
                let millis = self.config.initial_delay.as_millis() as f64
                    * self.config.multiplier.powi(self.attempt as i32);
                Duration::from_millis(millis as u64)
            }
            RetryStrategy::ExponentialWithJitter => {
                let base_millis = self.config.initial_delay.as_millis() as f64
                    * self.config.multiplier.powi(self.attempt as i32);
                let jitter_range = base_millis * self.config.jitter_factor;
                let jitter = (rand64() % ((jitter_range * 2.0) as u64)) as f64 - jitter_range;
                Duration::from_millis((base_millis + jitter).max(0.0) as u64)
            }
            _ => self.config.initial_delay,
        };

        if let Some(max_delay) = self.config.max_delay {
            base_delay.min(max_delay)
        } else {
            base_delay
        }
    }

    /// Get current attempt number
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Check if can retry
    pub fn can_retry(&self) -> bool {
        self.attempt < self.config.max_attempts
    }

    /// Reset the state machine
    pub fn reset(&mut self) {
        self.attempt = 0;
        self.state = RetryState::Idle;
        self.last_attempt_time = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.strategy, RetryStrategy::Exponential);
    }

    #[test]
    fn test_retry_config_builder() {
        let config = RetryConfig::new()
            .with_max_attempts(5)
            .with_initial_delay(Duration::from_millis(500))
            .with_strategy(RetryStrategy::Fixed);

        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.initial_delay, Duration::from_millis(500));
        assert_eq!(config.strategy, RetryStrategy::Fixed);
    }

    #[test]
    fn test_exponential_backoff() {
        let config = RetryConfig::exponential(3, Duration::from_millis(100));
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.strategy, RetryStrategy::Exponential);
    }

    #[test]
    fn test_retry_executor() {
        let config = RetryConfig::fixed(3, Duration::from_millis(100));
        let mut executor = RetryExecutor::new(config).unwrap();

        let mut attempt_count = 0;
        let result = executor.execute(|attempt| {
            attempt_count = attempt;
            if attempt < 2 {
                Err("Failed".to_string())
            } else {
                Ok("Success")
            }
        });

        assert!(result.is_ok());
        assert_eq!(attempt_count, 2);
    }

    #[test]
    fn test_retry_max_attempts() {
        let config = RetryConfig::fixed(2, Duration::from_millis(100));
        let mut executor = RetryExecutor::new(config).unwrap();

        let result = executor.execute(|_| Err("Always fails".to_string()));

        assert!(matches!(result, Err(RetryError::MaxRetriesExceeded { .. })));
    }

    #[test]
    fn test_retry_state_machine() {
        let config = RetryConfig::fixed(3, Duration::from_millis(100));
        let mut sm = RetryStateMachine::new(config).unwrap();

        assert!(sm.can_retry());

        sm.start().unwrap();
        sm.record_failure("Error".to_string()).unwrap();

        assert_eq!(sm.attempt(), 1);
        assert!(sm.can_retry());
    }

    #[test]
    fn test_delay_calculation() {
        let config = RetryConfig::exponential(3, Duration::from_millis(100));
        let executor = RetryExecutor::new(config).unwrap();

        // Check that delays increase exponentially
        let delay0 = executor.calculate_delay(0);
        let delay1 = executor.calculate_delay(1);
        let delay2 = executor.calculate_delay(2);

        assert!(delay1 > delay0);
        assert!(delay2 > delay1);
    }

    #[test]
    fn test_invalid_config() {
        let config = RetryConfig::new()
            .with_max_attempts(0);

        assert!(config.validate().is_err());
    }
}
