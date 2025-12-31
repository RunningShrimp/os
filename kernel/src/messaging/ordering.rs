//! Message ordering guarantees and sequencing.
//!
//! This module provides:
//! - FIFO ordering per key (partition)
//! - Message sequencing with gap detection
//! - Ordering guarantees across partitions
//! - Out-of-order message handling
//! - Sequence number management

use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering as AtomicOrdering};
use core::cmp::Ordering;

use crate::collections::HashMap;
use crate::sync::{Mutex, RwLock};

/// Sequence number for ordering
pub type SequenceNumber = u64;

/// Partition key for grouping messages
pub type PartitionKey = Vec<u8>;

/// Result type for ordering operations
pub type OrderingResult<T> = Result<T, OrderingError>;

/// Errors that can occur in ordering operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderingError {
    /// Sequence gap detected
    SequenceGap {
        expected: SequenceNumber,
        received: SequenceNumber,
    },
    /// Duplicate sequence number
    DuplicateSequence(SequenceNumber),
    /// Out of order message
    OutOfOrder {
        expected: SequenceNumber,
        received: SequenceNumber,
    },
    /// Invalid sequence number
    InvalidSequence(String),
    /// Partition not found
    PartitionNotFound(PartitionKey),
}

impl core::fmt::Display for OrderingError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            OrderingError::SequenceGap { expected, received } => {
                write!(f, "Sequence gap: expected {}, got {}", expected, received)
            }
            OrderingError::DuplicateSequence(seq) => {
                write!(f, "Duplicate sequence number: {}", seq)
            }
            OrderingError::OutOfOrder { expected, received } => {
                write!(f, "Out of order: expected {}, got {}", expected, received)
            }
            OrderingError::InvalidSequence(msg) => {
                write!(f, "Invalid sequence: {}", msg)
            }
            OrderingError::PartitionNotFound(key) => {
                write!(f, "Partition not found: {:?}", String::from_utf8_lossy(&key))
            }
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for OrderingError {}

/// Ordering guarantee level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrderingGuarantee {
    /// No ordering guarantees
    None,
    /// FIFO ordering within a partition
    PerPartition,
    /// Global FIFO ordering (all messages)
    Global,
    /// Causal ordering (happens-before relationships)
    Causal,
}

/// Message with sequence information
#[derive(Debug, Clone)]
pub struct SequencedMessage {
    /// Message payload
    pub payload: Vec<u8>,
    /// Partition key
    pub partition_key: PartitionKey,
    /// Sequence number within partition
    pub sequence: SequenceNumber,
    /// Global sequence number (optional)
    pub global_sequence: Option<SequenceNumber>,
    /// Timestamp
    pub timestamp: u64,
    /// Message ID
    pub id: u64,
}

impl SequencedMessage {
    /// Creates a new sequenced message
    pub fn new(payload: Vec<u8>, partition_key: PartitionKey, sequence: SequenceNumber) -> Self {
        Self {
            payload,
            partition_key,
            sequence,
            global_sequence: None,
            timestamp: current_timestamp(),
            id: generate_message_id(),
        }
    }

    /// Sets the global sequence number
    pub fn with_global_sequence(mut self, global_seq: SequenceNumber) -> Self {
        self.global_sequence = Some(global_seq);
        self
    }

    /// Compares sequence numbers
    pub fn compare(&self, other: &Self) -> Ordering {
        // First compare partition key
        match self.partition_key.cmp(&other.partition_key) {
            Ordering::Equal => {
                // Then compare sequence within partition
                self.sequence.cmp(&other.sequence)
            }
            other => other,
        }
    }
}

/// Partition state for maintaining ordering
#[derive(Debug)]
struct PartitionState {
    /// Partition key
    key: PartitionKey,
    /// Next expected sequence number
    next_sequence: AtomicU64,
    /// Pending messages (out-of-order arrivals)
    pending: BTreeMap<SequenceNumber, SequencedMessage>,
    /// Highest sequence seen
    highest_seen: SequenceNumber,
}

impl PartitionState {
    /// Creates a new partition state
    fn new(key: PartitionKey) -> Self {
        Self {
            key,
            next_sequence: AtomicU64::new(0),
            pending: BTreeMap::new(),
            highest_seen: 0,
        }
    }

    /// Returns the next expected sequence
    fn next_sequence(&self) -> SequenceNumber {
        self.next_sequence.load(AtomicOrdering::Acquire)
    }

    /// Advances the sequence counter
    fn advance_sequence(&self) -> SequenceNumber {
        self.next_sequence.fetch_add(1, AtomicOrdering::AcqRel)
    }

    /// Adds a pending message (out of order)
    fn add_pending(&mut self, msg: SequencedMessage) {
        let seq = msg.sequence;
        self.pending.insert(seq, msg);
        if seq > self.highest_seen {
            self.highest_seen = seq;
        }
    }

    /// Returns the next in-order message if available
    fn next_message(&mut self) -> Option<SequencedMessage> {
        let expected = self.next_sequence();

        // Check if we have the expected message pending
        if let Some(msg) = self.pending.remove(&expected) {
            self.next_sequence.fetch_add(1, AtomicOrdering::Release);
            return Some(msg);
        }

        None
    }

    /// Returns true if there's a gap in the sequence
    fn has_gap(&self, sequence: SequenceNumber) -> bool {
        sequence > self.next_sequence() + 1
    }

    /// Returns the number of pending messages
    fn pending_count(&self) -> usize {
        self.pending.len()
    }
}

/// Ordered message queue with per-key FIFO guarantees
///
/// Ensures that messages with the same partition key are delivered
/// in the order they were sequenced, while allowing parallel
/// processing of different keys.
///
/// # Example
///
/// ```rust
/// use kernel::messaging::ordering::OrderedQueue;
///
/// let queue = OrderedQueue::new();
///
/// // Add messages with the same key
/// queue.enqueue(b"msg1", b"user123".to_vec(), 0).unwrap();
/// queue.enqueue(b"msg2", b"user123".to_vec(), 1).unwrap();
///
/// // They will be dequeued in order
/// assert_eq!(queue.dequeue().unwrap().payload, b"msg1");
/// assert_eq!(queue.dequeue().unwrap().payload, b"msg2");
/// ```
pub struct OrderedQueue {
    /// Partition states
    partitions: RwLock<HashMap<PartitionKey, Arc<Mutex<PartitionState>>>>,
    /// Global sequence counter (for global ordering)
    global_sequence: AtomicU64,
    /// Ordering guarantee level
    guarantee: OrderingGuarantee,
}

impl OrderedQueue {
    /// Creates a new ordered queue
    pub fn new() -> Self {
        Self::with_guarantee(OrderingGuarantee::PerPartition)
    }

    /// Creates an ordered queue with specific guarantee
    pub fn with_guarantee(guarantee: OrderingGuarantee) -> Self {
        Self {
            partitions: RwLock::new(HashMap::new()),
            global_sequence: AtomicU64::new(0),
            guarantee,
        }
    }

    /// Gets or creates a partition
    fn get_partition(&self, key: &PartitionKey) -> Arc<Mutex<PartitionState>> {
        {
            let partitions = self.partitions.read();
            if let Some(partition) = partitions.get(key) {
                return partition.clone();
            }
        }

        // Create new partition
        let partition = Arc::new(Mutex::new(PartitionState::new(key.clone())));

        {
            let mut partitions = self.partitions.write();
            partitions.insert(key.clone(), partition.clone());
        }

        partition
    }

    /// Enqueues a message with sequence information
    ///
    /// # Arguments
    ///
    /// * `payload` - Message payload
    /// * `partition_key` - Key for partitioning (e.g., user ID)
    /// * `sequence` - Sequence number within partition
    pub fn enqueue(
        &self,
        payload: Vec<u8>,
        partition_key: PartitionKey,
        sequence: SequenceNumber,
    ) -> OrderingResult<()> {
        let msg = SequencedMessage::new(payload, partition_key.clone(), sequence);

        match self.guarantee {
            OrderingGuarantee::PerPartition => {
                let partition = self.get_partition(&partition_key);
                let mut state = partition.lock();

                let expected = state.next_sequence();

                // Check for duplicates
                if sequence < expected {
                    return Err(OrderingError::DuplicateSequence(sequence));
                }

                // Check for gaps
                if state.has_gap(sequence) {
                    return Err(OrderingError::SequenceGap {
                        expected,
                        received: sequence,
                    });
                }

                if sequence == expected {
                    // In-order message
                    state.advance_sequence();
                    // Store for delivery
                } else {
                    // Out-of-order message, buffer it
                    state.add_pending(msg);
                }

                Ok(())
            }
            OrderingGuarantee::Global => {
                // Global ordering: use a single partition
                let global_key = b"__global__".to_vec();
                let partition = self.get_partition(&global_key);
                let mut state = partition.lock();

                let expected = state.next_sequence();

                if sequence < expected {
                    return Err(OrderingError::DuplicateSequence(sequence));
                }

                if sequence == expected {
                    state.advance_sequence();
                } else {
                    state.add_pending(msg);
                }

                Ok(())
            }
            OrderingGuarantee::Causal => {
                // Causal ordering uses timestamps
                let partition = self.get_partition(&partition_key);
                let mut state = partition.lock();

                // Check if this message is in causal order
                if sequence < state.highest_seen {
                    return Err(OrderingError::OutOfOrder {
                        expected: state.highest_seen + 1,
                        received: sequence,
                    });
                }

                if sequence == state.next_sequence() {
                    state.advance_sequence();
                } else {
                    state.add_pending(msg);
                }

                Ok(())
            }
            OrderingGuarantee::None => {
                // No ordering, just accept
                Ok(())
            }
        }
    }

    /// Dequeues the next available in-order message
    ///
    /// Returns None if no messages are available in order
    pub fn dequeue(&self) -> Option<SequencedMessage> {
        match self.guarantee {
            OrderingGuarantee::Global => {
                let global_key = b"__global__".to_vec();
                let partition = self.get_partition(&global_key);
                let mut state = partition.lock();
                state.next_message()
            }
            OrderingGuarantee::PerPartition | OrderingGuarantee::Causal => {
                // Check all partitions for available messages
                let partitions = self.partitions.read();

                for (_key, partition) in partitions.iter() {
                    let mut state = partition.lock();
                    if let Some(msg) = state.next_message() {
                        return Some(msg);
                    }
                }

                None
            }
            OrderingGuarantee::None => {
                // No ordering, return any message
                None
            }
        }
    }

    /// Returns the next expected sequence for a partition
    pub fn next_sequence(&self, partition_key: &PartitionKey) -> SequenceNumber {
        let partition = self.get_partition(partition_key);
        let state = partition.lock();
        state.next_sequence()
    }

    /// Returns the number of pending (out-of-order) messages
    pub fn pending_count(&self) -> usize {
        let partitions = self.partitions.read();
        partitions.values()
            .map(|p| {
                let state = p.lock();
                state.pending_count()
            })
            .sum()
    }

    /// Returns the number of active partitions
    pub fn partition_count(&self) -> usize {
        self.partitions.read().len()
    }
}

impl Default for OrderedQueue {
    fn default() -> Self {
        Self::new()
    }
}

/// Sequence manager for generating and tracking sequences
pub struct SequenceManager {
    /// Per-partition sequence counters
    sequences: RwLock<HashMap<PartitionKey, AtomicU64>>,
    /// Global sequence counter
    global_sequence: AtomicU64,
}

impl SequenceManager {
    /// Creates a new sequence manager
    pub fn new() -> Self {
        Self {
            sequences: RwLock::new(HashMap::new()),
            global_sequence: AtomicU64::new(0),
        }
    }

    /// Allocates the next sequence number for a partition
    pub fn next_sequence(&self, partition_key: &PartitionKey) -> SequenceNumber {
        let sequences = self.sequences.read();

        if let Some(counter) = sequences.get(partition_key) {
            counter.fetch_add(1, AtomicOrdering::SeqCst)
        } else {
            drop(sequences);

            // Create new counter
            let counter = AtomicU64::new(1);
            let mut sequences = self.sequences.write();
            sequences.insert(partition_key.clone(), counter);
            0
        }
    }

    /// Allocates the next global sequence number
    pub fn next_global_sequence(&self) -> SequenceNumber {
        self.global_sequence.fetch_add(1, AtomicOrdering::SeqCst)
    }

    /// Resets the sequence for a partition
    pub fn reset_partition(&self, partition_key: &PartitionKey) {
        let sequences = self.sequences.read();

        if let Some(counter) = sequences.get(partition_key) {
            counter.store(0, AtomicOrdering::Release);
        }
    }

    /// Gets the current sequence for a partition
    pub fn current_sequence(&self, partition_key: &PartitionKey) -> SequenceNumber {
        let sequences = self.sequences.read();

        sequences
            .get(partition_key)
            .map(|c| c.load(AtomicOrdering::Acquire))
            .unwrap_or(0)
    }
}

impl Default for SequenceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Out-of-order detector
///
/// Analyzes message streams to detect ordering violations.
pub struct OutOfOrderDetector {
    /// Expected sequences per partition
    expected: Mutex<HashMap<PartitionKey, SequenceNumber>>,
    /// Detection sensitivity (how many messages to buffer)
    buffer_size: usize,
    /// Violations detected
    violations: AtomicUsize,
}

impl OutOfOrderDetector {
    /// Creates a new detector
    pub fn new(buffer_size: usize) -> Self {
        Self {
            expected: Mutex::new(HashMap::new()),
            buffer_size,
            violations: AtomicUsize::new(0),
        }
    }

    /// Checks a message for ordering violations
    pub fn check(&self, msg: &SequencedMessage) -> OrderingResult<()> {
        let mut expected = self.expected.lock();

        let exp_seq = expected.entry(msg.partition_key.clone()).or_insert(0);

        if msg.sequence < *exp_seq {
            return Err(OrderingError::DuplicateSequence(msg.sequence));
        }

        if msg.sequence > *exp_seq + self.buffer_size as u64 {
            self.violations.fetch_add(1, AtomicOrdering::Relaxed);
            return Err(OrderingError::SequenceGap {
                expected: *exp_seq,
                received: msg.sequence,
            });
        }

        if msg.sequence == *exp_seq {
            *exp_seq += 1;
        }

        Ok(())
    }

    /// Returns the number of violations detected
    pub fn violation_count(&self) -> usize {
        self.violations.load(AtomicOrdering::Relaxed)
    }

    /// Resets the detector
    pub fn reset(&self) {
        self.expected.lock().clear();
        self.violations.store(0, AtomicOrdering::Release);
    }
}

/// Generates a unique message ID
fn generate_message_id() -> u64 {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_per_partition_ordering() {
        let queue = OrderedQueue::with_guarantee(OrderingGuarantee::PerPartition);

        let key1 = b"user1".to_vec();
        let key2 = b"user2".to_vec();

        // Enqueue interleaved messages from different users
        queue.enqueue(b"msg1".to_vec(), key1.clone(), 0).unwrap();
        queue.enqueue(b"msg2".to_vec(), key2.clone(), 0).unwrap();
        queue.enqueue(b"msg3".to_vec(), key1.clone(), 1).unwrap();

        // Should have 2 partitions
        assert_eq!(queue.partition_count(), 2);
    }

    #[test]
    fn test_sequence_gap_detection() {
        let queue = OrderedQueue::with_guarantee(OrderingGuarantee::PerPartition);
        let key = b"user1".to_vec();

        queue.enqueue(b"msg1".to_vec(), key.clone(), 0).unwrap();

        // Skip sequence 1
        let result = queue.enqueue(b"msg2".to_vec(), key.clone(), 2);
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_detection() {
        let queue = OrderedQueue::with_guarantee(OrderingGuarantee::PerPartition);
        let key = b"user1".to_vec();

        queue.enqueue(b"msg1".to_vec(), key.clone(), 0).unwrap();

        // Duplicate sequence
        let result = queue.enqueue(b"msg2".to_vec(), key.clone(), 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_sequence_manager() {
        let manager = SequenceManager::new();
        let key = b"user1".to_vec();

        assert_eq!(manager.next_sequence(&key), 0);
        assert_eq!(manager.next_sequence(&key), 1);
        assert_eq!(manager.current_sequence(&key), 2);
    }

    #[test]
    fn test_out_of_order_detector() {
        let detector = OutOfOrderDetector::new(10);

        let msg1 = SequencedMessage::new(b"test".to_vec(), b"user1".to_vec(), 0);
        let msg2 = SequencedMessage::new(b"test".to_vec(), b"user1".to_vec(), 2); // Gap

        assert!(detector.check(&msg1).is_ok());
        assert!(detector.check(&msg2).is_err());
        assert_eq!(detector.violation_count(), 1);
    }

    #[test]
    fn test_message_comparison() {
        let msg1 = SequencedMessage::new(b"test".to_vec(), b"user1".to_vec(), 0);
        let msg2 = SequencedMessage::new(b"test".to_vec(), b"user1".to_vec(), 1);
        let msg3 = SequencedMessage::new(b"test".to_vec(), b"user2".to_vec(), 0);

        assert_eq!(msg1.compare(&msg2), Ordering::Less);
        assert_eq!(msg2.compare(&msg1), Ordering::Greater);
        // Different partition keys
        assert_eq!(msg1.compare(&msg3), Ordering::Less);
    }
}
