//! Dead letter queue (DLQ) for handling failed message processing.
//!
//! This module provides:
//! - Failed message capture and storage
//! - Configurable retry policies
//! - Message inspection and analysis
//! - Manual reprocessing of failed messages
//! - Dead letter queue management

use alloc::collections::VecDeque;
use alloc::string::{String, ToString};
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering};
use core::time::Duration;

use crate::collections::HashMap;
use crate::sync::{Mutex, RwLock};
use crate::messaging::persistence::MessageId;

/// Dead letter identifier
pub type DeadLetterId = u64;

/// Result type for DLQ operations
pub type DlqResult<T> = Result<T, DlqError>;

/// Errors that can occur in DLQ operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DlqError {
    /// Message not found in DLQ
    MessageNotFound(DeadLetterId),
    /// Retry limit exceeded
    RetryLimitExceeded,
    /// Invalid retry policy
    InvalidRetryPolicy(String),
    /// DLQ is full
    Full,
    /// System shutdown
    Shutdown,
}

impl core::fmt::Display for DlqError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            DlqError::MessageNotFound(id) => write!(f, "Dead letter not found: {}", id),
            DlqError::RetryLimitExceeded => write!(f, "Retry limit exceeded"),
            DlqError::InvalidRetryPolicy(msg) => write!(f, "Invalid retry policy: {}", msg),
            DlqError::Full => write!(f, "Dead letter queue is full"),
            DlqError::Shutdown => write!(f, "DLQ system shutdown"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for DlqError {}

/// Reason why a message failed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureReason {
    /// Processing error
    ProcessingError(String),
    /// Timeout
    Timeout,
    /// Validation failed
    ValidationFailed(String),
    /// System error
    SystemError(String),
    /// Unknown error
    Unknown(String),
}

impl FailureReason {
    /// Returns a string description
    pub fn as_str(&self) -> &str {
        match self {
            FailureReason::ProcessingError(msg) => msg,
            FailureReason::Timeout => "Processing timeout",
            FailureReason::ValidationFailed(msg) => msg,
            FailureReason::SystemError(msg) => msg,
            FailureReason::Unknown(msg) => msg,
        }
    }
}

/// Retry policy for failed messages
#[derive(Debug, Clone)]
pub struct RetryPolicy {
    /// Maximum number of retry attempts
    pub max_attempts: u32,
    /// Initial delay before first retry
    pub initial_delay: Duration,
    /// Multiplier for delay between retries (e.g., 2.0 for exponential backoff)
    pub backoff_multiplier: f64,
    /// Maximum delay between retries
    pub max_delay: Duration,
    /// Whether to use jitter (randomized delay)
    pub use_jitter: bool,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay: Duration::from_millis(100),
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(60),
            use_jitter: true,
        }
    }
}

impl RetryPolicy {
    /// Creates a new retry policy
    pub fn new(max_attempts: u32, initial_delay: Duration) -> Self {
        Self {
            max_attempts,
            initial_delay,
            ..Default::default()
        }
    }

    /// Creates a policy with exponential backoff
    pub fn exponential_backoff(max_attempts: u32, initial_delay: Duration) -> Self {
        Self {
            max_attempts,
            initial_delay,
            backoff_multiplier: 2.0,
            max_delay: Duration::from_secs(300),
            use_jitter: true,
        }
    }

    /// Creates a policy with fixed delay
    pub fn fixed_delay(max_attempts: u32, delay: Duration) -> Self {
        Self {
            max_attempts,
            initial_delay: delay,
            backoff_multiplier: 1.0,
            max_delay: delay,
            use_jitter: false,
        }
    }

    /// Calculates the delay for the next retry attempt
    pub fn next_delay(&self, attempt: u32) -> Duration {
        let delay_ms = self.initial_delay.as_millis() as f64
            * self.backoff_multiplier.powi(attempt as i32 - 1);

        let delay_ms = delay_ms.min(self.max_delay.as_millis() as f64) as u64;

        if self.use_jitter {
            // Add up to 25% jitter
            let jitter = (delay_ms as f64 * 0.25) as u64;
            let jitter_amount = random_u64() % (jitter + 1);
            Duration::from_millis(delay_ms + jitter_amount)
        } else {
            Duration::from_millis(delay_ms)
        }
    }
}

/// Dead letter entry
#[derive(Debug, Clone)]
pub struct DeadLetter {
    /// Unique dead letter ID
    pub id: DeadLetterId,
    /// Original message ID
    pub original_message_id: MessageId,
    /// Original queue/topic
    pub original_destination: String,
    /// Message payload
    pub payload: Vec<u8>,
    /// Number of delivery attempts
    pub attempt_count: u32,
    /// Reason for failure
    pub failure_reason: FailureReason,
    /// Timestamp when message was moved to DLQ
    pub dead_lettered_at: u64,
    /// Timestamp of first failure
    pub first_failure_at: u64,
    /// Next scheduled retry time
    pub next_retry_at: Option<u64>,
    /// Custom metadata
    pub metadata: HashMap<String, String>,
}

impl DeadLetter {
    /// Creates a new dead letter entry
    pub fn new(
        original_message_id: MessageId,
        original_destination: String,
        payload: Vec<u8>,
        failure_reason: FailureReason,
    ) -> Self {
        let now = current_timestamp();
        Self {
            id: generate_dead_letter_id(),
            original_message_id,
            original_destination,
            payload,
            attempt_count: 1,
            failure_reason,
            dead_lettered_at: now,
            first_failure_at: now,
            next_retry_at: None,
            metadata: HashMap::new(),
        }
    }

    /// Returns true if this dead letter is ready for retry
    pub fn is_ready_for_retry(&self) -> bool {
        if let Some(retry_time) = self.next_retry_at {
            current_timestamp() >= retry_time
        } else {
            false
        }
    }

    /// Adds metadata
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
}

/// Dead letter queue
///
/// Stores and manages failed messages with retry capabilities.
pub struct DeadLetterQueue {
    /// Queue identifier
    queue_id: String,
    /// Dead letter storage
    dead_letters: Mutex<VecDeque<DeadLetter>>,
    /// Index by original message ID
    by_original_id: Mutex<HashMap<MessageId, DeadLetterId>>,
    /// Maximum queue size
    capacity: usize,
    /// Retry policy
    retry_policy: RetryPolicy,
    /// Counter for generating IDs
    id_counter: AtomicU64,
    /// Shutdown flag
    shutdown: AtomicUsize,
}

impl DeadLetterQueue {
    /// Creates a new dead letter queue
    pub fn new(queue_id: String, capacity: usize) -> Self {
        Self {
            queue_id,
            dead_letters: Mutex::new(VecDeque::new()),
            by_original_id: Mutex::new(HashMap::new()),
            capacity,
            retry_policy: RetryPolicy::default(),
            id_counter: AtomicU64::new(1),
            shutdown: AtomicUsize::new(0),
        }
    }

    /// Creates a DLQ with custom retry policy
    pub fn with_policy(queue_id: String, capacity: usize, policy: RetryPolicy) -> Self {
        Self {
            queue_id,
            dead_letters: Mutex::new(VecDeque::new()),
            by_original_id: Mutex::new(HashMap::new()),
            capacity,
            retry_policy: policy,
            id_counter: AtomicU64::new(1),
            shutdown: AtomicUsize::new(0),
        }
    }

    /// Adds a failed message to the DLQ
    pub fn add(&self, dead_letter: DeadLetter) -> DlqResult<DeadLetterId> {
        if self.is_shutdown() {
            return Err(DlqError::Shutdown);
        }

        // Check capacity
        {
            let letters = self.dead_letters.lock();
            if letters.len() >= self.capacity {
                return Err(DlqError::Full);
            }
        }

        let id = dead_letter.id;
        let original_id = dead_letter.original_message_id;

        // Add to storage
        {
            let mut letters = self.dead_letters.lock();
            letters.push_back(dead_letter);
        }

        // Update index
        {
            let mut by_id = self.by_original_id.lock();
            by_id.insert(original_id, id);
        }

        Ok(id)
    }

    /// Gets a dead letter by ID
    pub fn get(&self, id: DeadLetterId) -> DlqResult<DeadLetter> {
        let letters = self.dead_letters.lock();
        letters
            .iter()
            .find(|dl| dl.id == id)
            .cloned()
            .ok_or(DlqError::MessageNotFound(id))
    }

    /// Gets a dead letter by original message ID
    pub fn get_by_original_id(&self, original_id: MessageId) -> DlqResult<DeadLetter> {
        let by_id = self.by_original_id.lock();
        let dl_id = *by_id
            .get(&original_id)
            .ok_or(DlqError::MessageNotFound(0))?;
        drop(by_id);

        self.get(dl_id)
    }

    /// Removes a dead letter from the queue
    pub fn remove(&self, id: DeadLetterId) -> DlqResult<DeadLetter> {
        let mut letters = self.dead_letters.lock();
        let pos = letters
            .iter()
            .position(|dl| dl.id == id)
            .ok_or(DlqError::MessageNotFound(id))?;

        let letter = letters.remove(pos).unwrap();

        // Remove from index
        let mut by_id = self.by_original_id.lock();
        by_id.remove(&letter.original_message_id);

        Ok(letter)
    }

    /// Lists all dead letters
    pub fn list(&self) -> Vec<DeadLetter> {
        let letters = self.dead_letters.lock();
        letters.iter().cloned().collect()
    }

    /// Returns dead letters ready for retry
    pub fn ready_for_retry(&self) -> Vec<DeadLetter> {
        let letters = self.dead_letters.lock();
        letters
            .iter()
            .filter(|dl| dl.is_ready_for_retry())
            .cloned()
            .collect()
    }

    /// Schedules a retry for a dead letter
    pub fn schedule_retry(&self, id: DeadLetterId) -> DlqResult<()> {
        let mut letters = self.dead_letters.lock();
        let pos = letters
            .iter()
            .position(|dl| dl.id == id)
            .ok_or(DlqError::MessageNotFound(id))?;

        let letter = &mut letters[pos];
        let attempt = letter.attempt_count + 1;

        if attempt > self.retry_policy.max_attempts {
            return Err(DlqError::RetryLimitExceeded);
        }

        // Update retry time
        letter.attempt_count = attempt;
        letter.next_retry_at = Some(
            current_timestamp() + self.retry_policy.next_delay(attempt).as_millis() as u64
        );

        Ok(())
    }

    /// Reprocesses a dead letter (removes it and returns the message)
    pub fn reprocess(&self, id: DeadLetterId) -> DlqResult<(Vec<u8>, String)> {
        let letter = self.remove(id)?;
        Ok((letter.payload, letter.original_destination))
    }

    /// Reprocesses all dead letters ready for retry
    pub fn reprocess_ready(&self) -> Vec<(Vec<u8>, String)> {
        let ready = self.ready_for_retry();
        let mut results = Vec::new();

        for letter in ready {
            if let Ok((payload, dest)) = self.reprocess(letter.id) {
                results.push((payload, dest));
            }
        }

        results
    }

    /// Clears the DLQ
    pub fn clear(&self) {
        let mut letters = self.dead_letters.lock();
        letters.clear();
    }

    /// Returns the number of dead letters
    pub fn len(&self) -> usize {
        self.dead_letters.lock().len()
    }

    /// Returns true if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Shuts down the DLQ
    pub fn shutdown(&self) {
        self.shutdown.store(1, AtomicOrdering::Release);
    }

    /// Returns true if shut down
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(AtomicOrdering::Acquire) != 0
    }
}

/// DLQ manager for multiple dead letter queues
pub struct DlqManager {
    /// Managed DLQs
    queues: RwLock<HashMap<String, Arc<DeadLetterQueue>>>,
    /// Default capacity
    default_capacity: usize,
    /// Default retry policy
    default_policy: RetryPolicy,
}

impl DlqManager {
    /// Creates a new DLQ manager
    pub fn new() -> Self {
        Self {
            queues: RwLock::new(HashMap::new()),
            default_capacity: 10000,
            default_policy: RetryPolicy::default(),
        }
    }

    /// Gets or creates a DLQ for the given queue/topic
    pub fn get_queue(&self, queue_id: &str) -> Arc<DeadLetterQueue> {
        {
            let queues = self.queues.read();
            if let Some(queue) = queues.get(queue_id) {
                return queue.clone();
            }
        }

        // Create new DLQ
        let dlq = Arc::new(DeadLetterQueue::new(
            queue_id.to_string(),
            self.default_capacity,
        ));

        {
            let mut queues = self.queues.write();
            queues.insert(queue_id.to_string(), dlq.clone());
        }

        dlq
    }

    /// Creates a DLQ with custom policy
    pub fn create_queue_with_policy(
        &self,
        queue_id: &str,
        policy: RetryPolicy,
    ) -> Arc<DeadLetterQueue> {
        let dlq = Arc::new(DeadLetterQueue::with_policy(
            queue_id.to_string(),
            self.default_capacity,
            policy,
        ));

        {
            let mut queues = self.queues.write();
            queues.insert(queue_id.to_string(), dlq.clone());
        }

        dlq
    }

    /// Lists all DLQ IDs
    pub fn list_queues(&self) -> Vec<String> {
        let queues = self.queues.read();
        queues.keys().cloned().collect()
    }

    /// Returns statistics for all DLQs
    pub fn stats(&self) -> DlqStats {
        let queues = self.queues.read();
        let total_letters: usize = queues.values().map(|q| q.len()).sum();
        let queue_count = queues.len();

        DlqStats {
            total_dead_letters: total_letters,
            queue_count,
        }
    }
}

impl Default for DlqManager {
    fn default() -> Self {
        Self::new()
    }
}

/// DLQ statistics
#[derive(Debug, Clone)]
pub struct DlqStats {
    /// Total number of dead letters across all queues
    pub total_dead_letters: usize,
    /// Number of DLQs
    pub queue_count: usize,
}

/// Generates a unique dead letter ID
fn generate_dead_letter_id() -> DeadLetterId {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, AtomicOrdering::SeqCst)
}

/// Returns the current timestamp
fn current_timestamp() -> u64 {
    #[cfg(feature = "std")]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    #[cfg(not(feature = "std"))]
    {
        0
    }
}

/// Simple random number generator for jitter
fn random_u64() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, AtomicOrdering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_policy() {
        let policy = RetryPolicy::exponential_backoff(3, Duration::from_millis(100));

        assert_eq!(policy.next_delay(1).as_millis(), 100);
        assert_eq!(policy.next_delay(2).as_millis(), 200);
        assert_eq!(policy.next_delay(3).as_millis(), 400);
    }

    #[test]
    fn test_dlq_add_remove() {
        let dlq = DeadLetterQueue::new("test".to_string(), 100);

        let letter = DeadLetter::new(
            1,
            "test_queue".to_string(),
            b"test".to_vec(),
            FailureReason::ProcessingError("test error".to_string()),
        );

        let id = dlq.add(letter).unwrap();
        assert!(dlq.get(id).is_ok());

        dlq.remove(id).unwrap();
        assert!(dlq.is_empty());
    }

    #[test]
    fn test_dlq_retry() {
        let dlq = DeadLetterQueue::new("test".to_string(), 100);

        let letter = DeadLetter::new(
            1,
            "test_queue".to_string(),
            b"test".to_vec(),
            FailureReason::Timeout,
        );

        let id = dlq.add(letter).unwrap();
        dlq.schedule_retry(id).unwrap();

        let retrieved = dlq.get(id).unwrap();
        assert!(retrieved.next_retry_at.is_some());
    }

    #[test]
    fn test_dlq_capacity() {
        let dlq = DeadLetterQueue::new("test".to_string(), 2);

        dlq.add(DeadLetter::new(
            1, "q".to_string(), vec![], FailureReason::Unknown("".to_string())
        )).unwrap();
        dlq.add(DeadLetter::new(
            2, "q".to_string(), vec![], FailureReason::Unknown("".to_string())
        )).unwrap();

        // Third message should fail
        assert!(dlq.add(DeadLetter::new(
            3, "q".to_string(), vec![], FailureReason::Unknown("".to_string())
        )).is_err());
    }

    #[test]
    fn test_manager() {
        let manager = DlqManager::new();
        let dlq = manager.get_queue("test_queue");

        assert_eq!(dlq.queue_id, "test_queue");
        assert_eq!(manager.list_queues().len(), 1);
    }
}
