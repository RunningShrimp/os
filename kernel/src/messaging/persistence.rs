//! Message persistence for durable messaging queues.
//!
//! This module provides durable message storage with:
//! - Message persistence across restarts
//! - Crash recovery
//! - Acknowledgment handling
//! - Transactional writes
//! - Storage backend abstraction

use alloc::collections::VecDeque;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::sync::Arc;
use alloc::vec::Vec;

use core::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use core::hash::{Hash, Hasher};

use crate::sync::{Mutex, RwLock};

/// Message identifier
pub type MessageId = u64;

/// Queue identifier
pub type QueueId = String;

/// Result type for persistence operations
pub type PersistenceResult<T> = Result<T, PersistenceError>;

/// Errors that can occur in persistence operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PersistenceError {
    /// Storage backend error
    StorageError(String),
    /// Message not found
    MessageNotFound(MessageId),
    /// Queue not found
    QueueNotFound(QueueId),
    /// Transaction failed
    TransactionFailed(String),
    /// Recovery failed
    RecoveryFailed(String),
    /// Checksum mismatch
    ChecksumMismatch,
    /// Corrupted data
    CorruptedData(String),
}

impl core::fmt::Display for PersistenceError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            PersistenceError::StorageError(msg) => write!(f, "Storage error: {}", msg),
            PersistenceError::MessageNotFound(id) => write!(f, "Message not found: {}", id),
            PersistenceError::QueueNotFound(id) => write!(f, "Queue not found: {}", id),
            PersistenceError::TransactionFailed(msg) => write!(f, "Transaction failed: {}", msg),
            PersistenceError::RecoveryFailed(msg) => write!(f, "Recovery failed: {}", msg),
            PersistenceError::ChecksumMismatch => write!(f, "Checksum mismatch"),
            PersistenceError::CorruptedData(msg) => write!(f, "Corrupted data: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for PersistenceError {}

/// Message delivery semantics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliverySemantics {
    /// At most once (no acknowledgment)
    AtMostOnce,
    /// At least once (may duplicate)
    AtLeastOnce,
    /// Exactly once (idempotent)
    ExactlyOnce,
}

/// Message acknowledgment status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckStatus {
    /// Message not yet acknowledged
    Pending,
    /// Message acknowledged
    Acknowledged,
    /// Message negatively acknowledged (will retry)
    NegativeAck,
}

/// Persistent message with metadata
#[derive(Debug, Clone)]
pub struct PersistentMessage {
    /// Unique message ID
    pub id: MessageId,
    /// Queue this message belongs to
    pub queue_id: QueueId,
    /// Message payload
    pub payload: Vec<u8>,
    /// Message timestamp
    pub timestamp: u64,
    /// Delivery count
    pub delivery_count: u32,
    /// Acknowledgment status
    pub ack_status: AckStatus,
    /// Time when this message will expire
    pub expires_at: Option<u64>,
    /// Message checksum
    pub checksum: u64,
    /// Custom headers
    pub headers: Vec<(String, String)>,
}

impl PersistentMessage {
    /// Creates a new persistent message
    pub fn new(queue_id: QueueId, payload: Vec<u8>) -> Self {
        let timestamp = current_timestamp();
        Self {
            id: generate_message_id(),
            queue_id,
            payload,
            timestamp,
            delivery_count: 0,
            ack_status: AckStatus::Pending,
            expires_at: None,
            checksum: 0, // Will be calculated
            headers: Vec::new(),
        }
    }

    /// Calculates the checksum for this message
    pub fn calculate_checksum(&self) -> u64 {
        let mut hasher = SimpleHasher::new();
        self.id.hash(&mut hasher);
        self.queue_id.hash(&mut hasher);
        self.payload.hash(&mut hasher);
        self.timestamp.hash(&mut hasher);
        hasher.finish()
    }

    /// Updates the checksum
    pub fn update_checksum(&mut self) {
        self.checksum = self.calculate_checksum();
    }

    /// Verifies the checksum
    pub fn verify_checksum(&self) -> bool {
        self.checksum == self.calculate_checksum()
    }

    /// Returns true if the message has expired
    pub fn is_expired(&self) -> bool {
        if let Some(expires_at) = self.expires_at {
            current_timestamp() > expires_at
        } else {
            false
        }
    }
}

/// Simple hash function for checksums
struct SimpleHasher {
    state: u64,
}

impl SimpleHasher {
    fn new() -> Self {
        Self { state: 0xcbf29ce484222325 }
    }
}

impl Hasher for SimpleHasher {
    fn finish(&self) -> u64 {
        self.state
    }

    fn write(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.state = self.state.wrapping_mul(0x100000001b3);
            self.state ^= byte as u64;
        }
    }
}

/// Storage backend trait
///
/// Defines the interface for persistent storage implementations.
pub trait StorageBackend: Send + Sync {
    /// Stores a message
    fn store(&self, message: &PersistentMessage) -> PersistenceResult<()>;

    /// Retrieves a message by ID
    fn retrieve(&self, id: MessageId) -> PersistenceResult<PersistentMessage>;

    /// Deletes a message
    fn delete(&self, id: MessageId) -> PersistenceResult<()>;

    /// Lists all messages in a queue
    fn list_messages(&self, queue_id: &QueueId) -> PersistenceResult<Vec<PersistentMessage>>;

    /// Begins a transaction
    fn begin_transaction(&self) -> PersistenceResult<Box<dyn Transaction>>;

    /// Performs recovery after crash
    fn recover(&self) -> PersistenceResult<RecoveryReport>;
}

/// Transaction for atomic operations
pub trait Transaction: Send + Sync {
    /// Stores a message in this transaction
    fn store(&mut self, message: &PersistentMessage) -> PersistenceResult<()>;

    /// Deletes a message in this transaction
    fn delete(&mut self, id: MessageId) -> PersistenceResult<()>;

    /// Commits the transaction
    fn commit(self: Box<Self>) -> PersistenceResult<()>;

    /// Rolls back the transaction
    fn rollback(self: Box<Self>) -> PersistenceResult<()>;
}

/// Recovery report after crash recovery
#[derive(Debug, Clone)]
pub struct RecoveryReport {
    /// Messages recovered
    pub messages_recovered: usize,
    /// Messages corrupted
    pub messages_corrupted: usize,
    /// Messages expired
    pub messages_expired: usize,
    /// Queues recovered
    pub queues_recovered: usize,
    /// Recovery duration in milliseconds
    pub recovery_duration_ms: u64,
}

/// In-memory storage backend (for testing/fallback)
struct InMemoryStorage {
    /// Messages by ID
    messages: RwLock<HashMap<MessageId, PersistentMessage>>,
    /// Messages by queue
    queue_messages: RwLock<HashMap<QueueId, Vec<MessageId>>>,
}

impl InMemoryStorage {
    fn new() -> Self {
        Self {
            messages: RwLock::new(HashMap::new()),
            queue_messages: RwLock::new(HashMap::new()),
        }
    }
}

impl StorageBackend for InMemoryStorage {
    fn store(&self, message: &PersistentMessage) -> PersistenceResult<()> {
        // Store message
        {
            let mut messages = self.messages.write();
            messages.insert(message.id, message.clone());
        }

        // Add to queue index
        {
            let mut queue_messages = self.queue_messages.write();
            queue_messages
                .entry(message.queue_id.clone())
                .or_insert_with(Vec::new)
                .push(message.id);
        }

        Ok(())
    }

    fn retrieve(&self, id: MessageId) -> PersistenceResult<PersistentMessage> {
        let messages = self.messages.read();
        messages
            .get(&id)
            .cloned()
            .ok_or(PersistenceError::MessageNotFound(id))
    }

    fn delete(&self, id: MessageId) -> PersistenceResult<()> {
        // Get the message first to find queue
        let queue_id = {
            let messages = self.messages.read();
            messages
                .get(&id)
                .map(|m| m.queue_id.clone())
                .ok_or(PersistenceError::MessageNotFound(id))?
        };

        // Remove from messages
        {
            let mut messages = self.messages.write();
            messages.remove(&id);
        }

        // Remove from queue index
        {
            let mut queue_messages = self.queue_messages.write();
            if let Some(ids) = queue_messages.get_mut(&queue_id) {
                ids.retain(|&msg_id| msg_id != id);
            }
        }

        Ok(())
    }

    fn list_messages(&self, queue_id: &QueueId) -> PersistenceResult<Vec<PersistentMessage>> {
        let queue_messages = self.queue_messages.read();
        let messages = self.messages.read();

        let ids = queue_messages
            .get(queue_id)
            .ok_or(PersistenceError::QueueNotFound(queue_id.clone()))?;

        let result = ids
            .iter()
            .filter_map(|id| messages.get(id).cloned())
            .collect();

        Ok(result)
    }

    fn begin_transaction(&self) -> PersistenceResult<Box<dyn Transaction>> {
        // Create an Arc for this storage to pass to transaction
        // In a real implementation, InMemoryStorage would be wrapped in Arc from creation
        // For now, we'll create a workaround
        Err(PersistenceError::StorageError("Transaction not supported for non-Arc storage".into()))
    }

    fn recover(&self) -> PersistenceResult<RecoveryReport> {
        // In-memory storage doesn't need recovery
        Ok(RecoveryReport {
            messages_recovered: 0,
            messages_corrupted: 0,
            messages_expired: 0,
            queues_recovered: 0,
            recovery_duration_ms: 0,
        })
    }
}

/// In-memory transaction
struct InMemoryTransaction {
    storage: Arc<InMemoryStorage>,
    to_store: Vec<PersistentMessage>,
    to_delete: Vec<MessageId>,
    committed: bool,
}

impl InMemoryTransaction {
    fn new(storage: &Arc<InMemoryStorage>) -> Self {
        Self {
            storage: storage.clone(),
            to_store: Vec::new(),
            to_delete: Vec::new(),
            committed: false,
        }
    }
}

impl Transaction for InMemoryTransaction {
    fn store(&mut self, message: &PersistentMessage) -> PersistenceResult<()> {
        self.to_store.push(message.clone());
        Ok(())
    }

    fn delete(&mut self, id: MessageId) -> PersistenceResult<()> {
        self.to_delete.push(id);
        Ok(())
    }

    fn commit(mut self: Box<Self>) -> PersistenceResult<()> {
        self.committed = true;

        // Store all messages
        for msg in &self.to_store {
            self.storage.store(msg)?;
        }

        // Delete all messages
        for id in &self.to_delete {
            self.storage.delete(*id)?;
        }

        Ok(())
    }

    fn rollback(mut self: Box<Self>) -> PersistenceResult<()> {
        self.committed = false;
        self.to_store.clear();
        self.to_delete.clear();
        Ok(())
    }
}

/// Durable message queue
///
/// Provides persistent message storage with acknowledgment handling.
pub struct DurableQueue {
    /// Queue identifier
    queue_id: QueueId,
    /// Storage backend
    storage: Arc<dyn StorageBackend>,
    /// Delivery semantics
    semantics: DeliverySemantics,
    /// Pending messages (in memory)
    pending: Mutex<VecDeque<MessageId>>,
    /// Message counter
    message_counter: AtomicU64,
}

impl DurableQueue {
    /// Creates a new durable queue
    pub fn new(
        queue_id: QueueId,
        storage: Arc<dyn StorageBackend>,
        semantics: DeliverySemantics,
    ) -> Self {
        Self {
            queue_id,
            storage,
            semantics,
            pending: Mutex::new(VecDeque::new()),
            message_counter: AtomicU64::new(0),
        }
    }

    /// Enqueues a message with persistence
    pub fn enqueue(&self, payload: Vec<u8>) -> PersistenceResult<MessageId> {
        let mut message = PersistentMessage::new(self.queue_id.clone(), payload);
        message.update_checksum();

        // Verify checksum
        if !message.verify_checksum() {
            return Err(PersistenceError::ChecksumMismatch);
        }

        // Store message
        self.storage.store(&message)?;

        // Add to pending
        {
            let mut pending = self.pending.lock();
            pending.push_back(message.id);
        }

        self.message_counter.fetch_add(1, AtomicOrdering::SeqCst);

        Ok(message.id)
    }

    /// Dequeues a message for delivery
    pub fn dequeue(&self) -> PersistenceResult<PersistentMessage> {
        let msg_id = {
            let mut pending = self.pending.lock();
            pending
                .pop_front()
                .ok_or(PersistenceError::MessageNotFound(0))?
        };

        let mut message = self.storage.retrieve(msg_id)?;

        // Increment delivery count
        message.delivery_count += 1;

        Ok(message)
    }

    /// Acknowledges a message (removes it)
    pub fn acknowledge(&self, msg_id: MessageId) -> PersistenceResult<()> {
        match self.semantics {
            DeliverySemantics::AtMostOnce => {
                // No acknowledgment needed
                Ok(())
            }
            DeliverySemantics::AtLeastOnce | DeliverySemantics::ExactlyOnce => {
                // Remove the message
                self.storage.delete(msg_id)
            }
        }
    }

    /// Negatively acknowledges a message (will be retried)
    pub fn negative_acknowledge(&self, msg_id: MessageId) -> PersistenceResult<()> {
        match self.semantics {
            DeliverySemantics::AtLeastOnce | DeliverySemantics::ExactlyOnce => {
                // Re-add to pending
                let mut pending = self.pending.lock();
                pending.push_back(msg_id);
                Ok(())
            }
            DeliverySemantics::AtMostOnce => {
                // Cannot nack with at-most-once
                Err(PersistenceError::TransactionFailed(
                    "NACK not supported with at-most-once semantics".into()
                ))
            }
        }
    }

    /// Returns the number of pending messages
    pub fn len(&self) -> usize {
        self.pending.lock().len()
    }

    /// Returns true if empty
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Message persistence manager
///
/// Manages multiple durable queues and provides recovery functionality.
pub struct PersistenceManager {
    /// Storage backend
    storage: Arc<dyn StorageBackend>,
    /// Managed queues
    queues: RwLock<HashMap<QueueId, Arc<DurableQueue>>>,
    /// Default delivery semantics
    default_semantics: DeliverySemantics,
}

impl PersistenceManager {
    /// Creates a new persistence manager
    pub fn new(storage: Arc<dyn StorageBackend>) -> Self {
        Self {
            storage,
            queues: RwLock::new(HashMap::new()),
            default_semantics: DeliverySemantics::AtLeastOnce,
        }
    }

    /// Creates or gets a queue
    pub fn get_queue(&self, queue_id: QueueId) -> Arc<DurableQueue> {
        {
            let queues = self.queues.read();
            if let Some(queue) = queues.get(&queue_id) {
                return queue.clone();
            }
        }

        // Create new queue
        let queue = Arc::new(DurableQueue::new(
            queue_id.clone(),
            self.storage.clone(),
            self.default_semantics,
        ));

        {
            let mut queues = self.queues.write();
            queues.insert(queue_id.clone(), queue.clone());
        }

        queue
    }

    /// Recovers from crash
    pub fn recover(&self) -> PersistenceResult<RecoveryReport> {
        self.storage.recover()
    }

    /// Sets the default delivery semantics
    pub fn set_semantics(&mut self, semantics: DeliverySemantics) {
        self.default_semantics = semantics;
    }
}

/// Generates a unique message ID
fn generate_message_id() -> MessageId {
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

// HashMap reimplementation for no_std
use alloc::collections::BTreeMap as HashMap;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_checksum() {
        let mut msg = PersistentMessage::new("test".to_string(), b"hello".to_vec());
        msg.update_checksum();
        assert!(msg.verify_checksum());

        // Corrupt the payload
        msg.payload[0] = 0xff;
        assert!(!msg.verify_checksum());
    }

    #[test]
    fn test_durable_queue() {
        let storage = Arc::new(InMemoryStorage::new());
        let queue = DurableQueue::new(
            "test_queue".to_string(),
            storage,
            DeliverySemantics::AtLeastOnce,
        );

        let msg_id = queue.enqueue(b"test message".to_vec()).unwrap();
        assert!(msg_id > 0);

        let msg = queue.dequeue().unwrap();
        assert_eq!(msg.payload, b"test message");

        queue.acknowledge(msg_id).unwrap();
        assert!(queue.is_empty());
    }

    #[test]
    fn test_expiration() {
        let mut msg = PersistentMessage::new("test".to_string(), b"hello".to_vec());
        assert!(!msg.is_expired());

        msg.expires_at = Some(current_timestamp() - 1000);
        assert!(msg.is_expired());
    }
}
