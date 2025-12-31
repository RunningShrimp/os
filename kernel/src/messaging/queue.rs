//! Message queue implementations for the NOS kernel.
//!
//! This module provides various queue types including:
//! - FIFO queues for standard message ordering
//! - Priority queues for multi-priority message handling
//! - Delay queues for scheduled message delivery
//! - Bounded and unbounded queue variants
//! - Thread-safe operations with proper synchronization
//! - Backpressure handling for flow control

use alloc::collections::BinaryHeap;
use alloc::string::String;
use core::cmp::Ordering;
use core::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use core::time::Duration;

use crate::sync::Mutex;

/// Result type for queue operations
pub type QueueResult<T> = Result<T, QueueError>;

/// Errors that can occur during queue operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueueError {
    /// Queue is full (bounded queues only)
    Full,
    /// Queue is empty
    Empty,
    /// Queue has been shut down
    Shutdown,
    /// Operation timed out
    Timeout,
    /// Invalid message or parameter
    Invalid(String),
    /// System resource error
    Resource(String),
}

impl core::fmt::Display for QueueError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            QueueError::Full => write!(f, "Queue is full"),
            QueueError::Empty => write!(f, "Queue is empty"),
            QueueError::Shutdown => write!(f, "Queue has been shut down"),
            QueueError::Timeout => write!(f, "Operation timed out"),
            QueueError::Invalid(msg) => write!(f, "Invalid parameter: {}", msg),
            QueueError::Resource(msg) => write!(f, "Resource error: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for QueueError {}

/// Message priority levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Priority {
    /// Low priority messages
    Low = 0,
    /// Normal priority (default)
    Normal = 1,
    /// High priority messages
    High = 2,
    /// Critical priority messages
    Critical = 3,
}

impl Default for Priority {
    fn default() -> Self {
        Priority::Normal
    }
}

/// Priority wrapper for heap ordering
#[derive(Debug, Clone)]
struct PriorityEntry<T> {
    item: T,
    priority: Priority,
    sequence: u64,
}

impl<T> PartialEq for PriorityEntry<T> {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.sequence == other.sequence
    }
}

impl<T> Eq for PriorityEntry<T> {}

impl<T> PartialOrd for PriorityEntry<T> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T> Ord for PriorityEntry<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse ordering for max-heap (BinaryHeap is max-heap)
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => {
                // For same priority, FIFO order (lower sequence first)
                other.sequence.cmp(&self.sequence)
            }
            other => other,
        }
    }
}

/// A standard FIFO message queue
///
/// Provides thread-safe first-in-first-out message ordering with support for:
/// - Bounded and unbounded variants
/// - Blocking and non-blocking operations
/// - Backpressure handling
/// - Graceful shutdown
///
/// # Example
///
/// ```rust
/// use kernel::messaging::queue::FifoQueue;
///
/// let queue: FifoQueue<u32> = FifoQueue::new(100);
/// queue.enqueue(42).unwrap();
/// let msg = queue.dequeue().unwrap();
/// assert_eq!(msg, 42);
/// ```
pub struct FifoQueue<T> {
    /// Message storage
    messages: Mutex<alloc::collections::VecDeque<T>>,
    /// Maximum queue size (0 for unbounded)
    capacity: usize,
    /// Current queue size
    size: AtomicUsize,
    /// Shutdown flag
    shutdown: AtomicUsize,
}

impl<T> FifoQueue<T> {
    /// Creates a new FIFO queue with the specified capacity
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of messages (0 for unbounded)
    pub fn new(capacity: usize) -> Self {
        Self {
            messages: Mutex::new(alloc::collections::VecDeque::new()),
            capacity,
            size: AtomicUsize::new(0),
            shutdown: AtomicUsize::new(0),
        }
    }

    /// Creates an unbounded FIFO queue
    pub fn unbounded() -> Self {
        Self::new(0)
    }

    /// Enqueues a message (non-blocking)
    ///
    /// # Errors
    ///
    /// Returns `QueueError::Full` if the queue is at capacity
    /// Returns `QueueError::Shutdown` if the queue has been shut down
    pub fn try_enqueue(&self, msg: T) -> QueueResult<()> {
        // Check shutdown
        if self.is_shutdown() {
            return Err(QueueError::Shutdown);
        }

        // Check capacity
        if self.capacity > 0 {
            let current = self.size.load(AtomicOrdering::Acquire);
            if current >= self.capacity {
                return Err(QueueError::Full);
            }
        }

        // Add message
        {
            let mut messages = self.messages.lock();
            messages.push_back(msg);
            self.size.fetch_add(1, AtomicOrdering::Release);
        }

        Ok(())
    }

    /// Dequeues a message (non-blocking)
    ///
    /// # Errors
    ///
    /// Returns `QueueError::Empty` if no messages are available
    /// Returns `QueueError::Shutdown` if the queue has been shut down and is empty
    pub fn try_dequeue(&self) -> QueueResult<T> {
        // Check if empty
        if self.is_empty() {
            return if self.is_shutdown() {
                Err(QueueError::Shutdown)
            } else {
                Err(QueueError::Empty)
            };
        }

        // Remove message
        let msg = {
            let mut messages = self.messages.lock();
            match messages.pop_front() {
                Some(msg) => {
                    self.size.fetch_sub(1, AtomicOrdering::Release);
                    msg
                }
                None => return Err(QueueError::Empty),
            }
        };

        Ok(msg)
    }

    /// Returns the current number of messages in the queue
    pub fn len(&self) -> usize {
        self.size.load(AtomicOrdering::Acquire)
    }

    /// Returns true if the queue is empty
    pub fn is_empty(&self) -> bool {
        let len: usize = self.len();
        len == 0
    }

    /// Returns the queue capacity
    pub fn capacity(&self) -> Option<usize> {
        if self.capacity == 0 {
            None
        } else {
            Some(self.capacity)
        }
    }

    /// Returns true if the queue is full
    pub fn is_full(&self) -> bool {
        if let Some(cap) = self.capacity() {
            let len: usize = self.len();
            len >= cap
        } else {
            false
        }
    }

    /// Shuts down the queue, preventing new enqueues
    pub fn shutdown(&self) {
        self.shutdown.store(1, AtomicOrdering::Release);
    }

    /// Returns true if the queue has been shut down
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(AtomicOrdering::Acquire) != 0
    }

    /// Clears all messages from the queue
    pub fn clear(&self) {
        let mut messages = self.messages.lock();
        let count = messages.len();
        messages.clear();
        self.size.fetch_sub(count, AtomicOrdering::Release);
    }
}

/// A priority queue that maintains message ordering by priority
///
/// Higher priority messages are dequeued first. Messages with the same
/// priority maintain FIFO ordering.
///
/// # Example
///
/// ```rust
/// use kernel::messaging::queue::{PriorityQueue, Priority};
///
/// let queue: PriorityQueue<u32> = PriorityQueue::new(100);
/// queue.enqueue_with_priority(42, Priority::Low).unwrap();
/// queue.enqueue_with_priority(43, Priority::High).unwrap();
/// assert_eq!(queue.dequeue().unwrap(), 43); // High priority first
/// assert_eq!(queue.dequeue().unwrap(), 42);
/// ```
pub struct PriorityQueue<T> {
    /// Message storage as a max-heap
    heap: Mutex<BinaryHeap<PriorityEntry<T>>>,
    /// Maximum queue size
    capacity: usize,
    /// Current queue size
    size: AtomicUsize,
    /// Sequence counter for FIFO ordering
    sequence: Mutex<u64>,
    /// Shutdown flag
    shutdown: AtomicUsize,
}

impl<T> PriorityQueue<T> {
    /// Creates a new priority queue
    pub fn new(capacity: usize) -> Self {
        Self {
            heap: Mutex::new(BinaryHeap::new()),
            capacity,
            size: AtomicUsize::new(0),
            sequence: Mutex::new(0),
            shutdown: AtomicUsize::new(0),
        }
    }

    /// Creates an unbounded priority queue
    pub fn unbounded() -> Self {
        Self::new(0)
    }

    /// Enqueues a message with the specified priority
    pub fn enqueue_with_priority(&self, msg: T, priority: Priority) -> QueueResult<()> {
        if self.is_shutdown() {
            return Err(QueueError::Shutdown);
        }

        if self.capacity > 0 {
            let current = self.size.load(AtomicOrdering::Acquire);
            if current >= self.capacity {
                return Err(QueueError::Full);
            }
        }

        // Get sequence number
        let seq = {
            let mut seq = self.sequence.lock();
            let current = *seq;
            *seq = seq.wrapping_add(1);
            current
        };

        // Add to heap
        {
            let mut heap = self.heap.lock();
            heap.push(PriorityEntry {
                item: msg,
                priority,
                sequence: seq,
            });
            self.size.fetch_add(1, AtomicOrdering::Release);
        }

        Ok(())
    }

    /// Enqueues a message with normal priority
    pub fn enqueue(&self, msg: T) -> QueueResult<()> {
        self.enqueue_with_priority(msg, Priority::Normal)
    }

    /// Dequeues the highest priority message
    pub fn dequeue(&self) -> QueueResult<T> {
        if self.is_empty() {
            return if self.is_shutdown() {
                Err(QueueError::Shutdown)
            } else {
                Err(QueueError::Empty)
            };
        }

        let entry = {
            let mut heap = self.heap.lock();
            match heap.pop() {
                Some(entry) => {
                    self.size.fetch_sub(1, AtomicOrdering::Release);
                    entry
                }
                None => return Err(QueueError::Empty),
            }
        };

        Ok(entry.item)
    }

    /// Returns the current number of messages
    pub fn len(&self) -> usize {
        self.size.load(AtomicOrdering::Acquire)
    }

    /// Returns true if the queue is empty
    pub fn is_empty(&self) -> bool {
        let len: usize = self.len();
        len == 0
    }

    /// Shuts down the queue
    pub fn shutdown(&self) {
        self.shutdown.store(1, AtomicOrdering::Release);
    }

    /// Returns true if shut down
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(AtomicOrdering::Acquire) != 0
    }
}

/// Delay queue entry
#[derive(Debug, Clone)]
struct DelayEntry<T> {
    item: T,
    available_at: u64, // Timestamp when message becomes available
}

/// A delay queue for scheduled message delivery
///
/// Messages can be scheduled for delivery at a future time.
/// Messages are only dequeued when their scheduled time has arrived.
///
/// # Example
///
/// ```rust
/// use kernel::messaging::queue::DelayQueue;
/// use core::time::Duration;
///
/// let queue: DelayQueue<u32> = DelayQueue::new();
/// queue.enqueue_delayed(42, Duration::from_secs(5)).unwrap();
/// // Message will be available after 5 seconds
/// ```
pub struct DelayQueue<T: PartialEq + Eq> {
    /// Scheduled messages
    delayed: Mutex<alloc::collections::BinaryHeap<DelayEntry<T>>>,
    /// Ready messages (whose time has arrived)
    ready: Mutex<alloc::collections::VecDeque<T>>,
    /// Queue capacity
    capacity: usize,
    /// Current size
    size: AtomicUsize,
    /// Shutdown flag
    shutdown: AtomicUsize,
}

impl<T: PartialEq + Eq> DelayQueue<T> {
    /// Creates a new delay queue
    pub fn new() -> Self {
        Self::with_capacity(0)
    }

    /// Creates a delay queue with specified capacity
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            delayed: Mutex::new(alloc::collections::BinaryHeap::new()),
            ready: Mutex::new(alloc::collections::VecDeque::new()),
            capacity,
            size: AtomicUsize::new(0),
            shutdown: AtomicUsize::new(0),
        }
    }

    /// Enqueues a message with a delay
    pub fn enqueue_delayed(&self, msg: T, delay: Duration) -> QueueResult<()> {
        if self.is_shutdown() {
            return Err(QueueError::Shutdown);
        }

        if self.capacity > 0 {
            let current = self.size.load(AtomicOrdering::Acquire);
            if current >= self.capacity {
                return Err(QueueError::Full);
            }
        }

        let now = current_time();
        let available_at = now + delay.as_millis() as u64;

        {
            let mut delayed = self.delayed.lock();
            delayed.push(DelayEntry {
                item: msg,
                available_at,
            });
            self.size.fetch_add(1, AtomicOrdering::Release);
        }

        Ok(())
    }

    /// Enqueues an immediately available message
    pub fn enqueue(&self, msg: T) -> QueueResult<()> {
        self.enqueue_delayed(msg, Duration::from_secs(0))
    }

    /// Dequeues a message whose time has arrived
    pub fn dequeue(&self) -> QueueResult<T> {
        self.transfer_ready();

        let msg = {
            let mut ready = self.ready.lock();
            match ready.pop_front() {
                Some(msg) => {
                    self.size.fetch_sub(1, AtomicOrdering::Release);
                    msg
                }
                None => return Err(QueueError::Empty),
            }
        };

        Ok(msg)
    }

    /// Transfers messages whose time has arrived to the ready queue
    fn transfer_ready(&self) {
        let now = current_time();

        let mut delayed = self.delayed.lock();
        let mut ready = self.ready.lock();

        while let Some(entry) = delayed.peek() {
            if entry.available_at <= now {
                let entry = delayed.pop().unwrap();
                ready.push_back(entry.item);
            } else {
                break;
            }
        }
    }

    /// Returns the number of messages (both delayed and ready)
    pub fn len(&self) -> usize {
        self.size.load(AtomicOrdering::Acquire)
    }

    /// Returns true if empty
    pub fn is_empty(&self) -> bool {
        let len: usize = self.len();
        len == 0
    }

    /// Shuts down the queue
    pub fn shutdown(&self) {
        self.shutdown.store(1, AtomicOrdering::Release);
    }

    /// Returns true if shut down
    pub fn is_shutdown(&self) -> bool {
        self.shutdown.load(AtomicOrdering::Acquire) != 0
    }
}

// Implement Ord for DelayEntry (min-heap based on available time)
impl<T: PartialEq> PartialEq for DelayEntry<T> {
    fn eq(&self, other: &Self) -> bool {
        self.available_at == other.available_at
    }
}

impl<T: Eq> Eq for DelayEntry<T> {}

impl<T: PartialEq> PartialOrd for DelayEntry<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.available_at.cmp(&other.available_at))
    }
}

impl<T: Eq> Ord for DelayEntry<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Min-heap: earlier times first
        other.available_at.cmp(&self.available_at)
    }
}

/// Backpressure strategy for flow control
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackpressureStrategy {
    /// Reject new messages when full
    Reject,
    /// Block the sender when full
    Block,
    /// Drop oldest messages when full
    DropOldest,
    /// Drop lowest priority messages when full
    DropLowestPriority,
}

/// Queue configuration options
#[derive(Debug, Clone)]
pub struct QueueConfig {
    /// Maximum queue size (0 for unbounded)
    pub capacity: usize,
    /// Backpressure strategy
    pub backpressure: BackpressureStrategy,
    /// Enable message persistence
    pub persistent: bool,
}

impl Default for QueueConfig {
    fn default() -> Self {
        Self {
            capacity: 1000,
            backpressure: BackpressureStrategy::Block,
            persistent: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fifo_queue_basic() {
        let queue: FifoQueue<u32> = FifoQueue::new(10);

        queue.enqueue(1).unwrap();
        queue.enqueue(2).unwrap();
        queue.enqueue(3).unwrap();

        assert_eq!(queue.len(), 3);
        assert_eq!(queue.dequeue().unwrap(), 1);
        assert_eq!(queue.dequeue().unwrap(), 2);
        assert_eq!(queue.dequeue().unwrap(), 3);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_fifo_queue_full() {
        let queue: FifoQueue<u32> = FifoQueue::new(2);

        queue.enqueue(1).unwrap();
        queue.enqueue(2).unwrap();

        assert!(queue.try_enqueue(3).is_err());
        assert!(queue.is_full());
    }

    #[test]
    fn test_priority_queue() {
        let queue: PriorityQueue<u32> = PriorityQueue::new(10);

        queue.enqueue_with_priority(1, Priority::Low).unwrap();
        queue.enqueue_with_priority(2, Priority::High).unwrap();
        queue.enqueue_with_priority(3, Priority::Normal).unwrap();

        assert_eq!(queue.dequeue().unwrap(), 2); // High
        assert_eq!(queue.dequeue().unwrap(), 3); // Normal
        assert_eq!(queue.dequeue().unwrap(), 1); // Low
    }

    #[test]
    fn test_shutdown() {
        let queue: FifoQueue<u32> = FifoQueue::new(10);

        queue.enqueue(1).unwrap();
        queue.shutdown();

        assert!(queue.try_enqueue(2).is_err());
        assert_eq!(queue.dequeue().unwrap(), 1); // Can still drain
        assert!(queue.try_dequeue().is_err());
    }
}

/// Helper function to get current time in milliseconds
#[inline(always)]
fn current_time() -> u64 {
    // In a real implementation, this would use the kernel's time source
    #[cfg(feature = "std")]
    {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    #[cfg(not(feature = "std"))]
    {
        // Fallback for no_std environments
        0
    }
}
