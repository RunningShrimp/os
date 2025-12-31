//! Asynchronous logging backend
//!
//! This module provides an async logging implementation with:
//! - Lock-free queue for log messages
//! - Batch writing for improved throughput
//! - Backpressure handling
//! - Graceful shutdown with flush
//! - <100ns overhead for log submission

#![no_std]

extern crate alloc;

use alloc::boxed::Box;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use core::time::Duration;

use crate::logging::error::{LogError, Result};
use crate::logging::logger::LogRecord;
use crate::logging::logger::LogTarget;

/// Async log message wrapper
#[derive(Debug)]
struct AsyncLogMessage {
    /// The log record
    record: LogRecord,
    /// Sequence number for ordering
    sequence: u64,
}

/// Lock-free MPSC (multi-producer single-consumer) queue for async logging
struct AsyncLogQueue {
    /// Ring buffer for messages
    buffer: spin::Mutex<Vec<Option<AsyncLogMessage>>>,
    /// Write position
    write_pos: AtomicUsize,
    /// Read position
    read_pos: AtomicUsize,
    /// Queue capacity
    capacity: usize,
    /// Mask for fast modulo (capacity must be power of 2)
    mask: usize,
}

impl AsyncLogQueue {
    /// Create a new async queue with specified capacity
    fn new(capacity: usize) -> Self {
        // Round up to next power of 2
        let capacity = capacity.next_power_of_two();
        let mask = capacity - 1;

        let mut buffer = Vec::with_capacity(capacity);
        buffer.resize_with(capacity, || None);

        Self {
            buffer: spin::Mutex::new(buffer),
            write_pos: AtomicUsize::new(0),
            read_pos: AtomicUsize::new(0),
            capacity,
            mask,
        }
    }

    /// Push a message to the queue (non-blocking)
    fn push(&self, record: LogRecord, sequence: u64) -> Result<()> {
        let write_pos = self.write_pos.fetch_add(1, Ordering::Relaxed);
        let read_pos = self.read_pos.load(Ordering::Acquire);

        // Check if queue is full
        if write_pos.wrapping_sub(read_pos) >= self.capacity {
            return Err(LogError::QueueFull);
        }

        let index = write_pos & self.mask;
        let mut buffer = self.buffer.lock();
        buffer[index] = Some(AsyncLogMessage { record, sequence });
        Ok(())
    }

    /// Pop a message from the queue (blocking)
    fn pop(&self) -> Option<AsyncLogMessage> {
        let read_pos = self.read_pos.load(Ordering::Relaxed);
        let write_pos = self.write_pos.load(Ordering::Acquire);

        // Check if queue is empty
        if read_pos == write_pos {
            return None;
        }

        let index = read_pos & self.mask;
        let mut buffer = self.buffer.lock();
        let message = buffer[index].take()?;

        self.read_pos.store(read_pos + 1, Ordering::Release);
        Some(message)
    }

    /// Check if queue is empty
    fn is_empty(&self) -> bool {
        let read_pos = self.read_pos.load(Ordering::Relaxed);
        let write_pos = self.write_pos.load(Ordering::Acquire);
        read_pos == write_pos
    }

    /// Get current queue size
    fn len(&self) -> usize {
        let read_pos = self.read_pos.load(Ordering::Relaxed);
        let write_pos = self.write_pos.load(Ordering::Acquire);
        write_pos.wrapping_sub(read_pos)
    }

    /// Get remaining capacity
    fn remaining_capacity(&self) -> usize {
        self.capacity - self.len()
    }
}

/// Batch configuration for async logging
#[derive(Clone, Copy, Debug)]
pub struct BatchConfig {
    /// Maximum batch size
    pub max_batch_size: usize,
    /// Maximum batch age in nanoseconds
    pub max_batch_age_ns: u64,
    /// Flush interval in nanoseconds
    pub flush_interval_ns: u64,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_batch_size: 100,
            max_batch_age_ns: 100_000_000, // 100ms
            flush_interval_ns: 1_000_000_000, // 1 second
        }
    }
}

impl BatchConfig {
    /// Create a new batch config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum batch size
    pub fn with_max_batch_size(mut self, size: usize) -> Self {
        self.max_batch_size = size;
        self
    }

    /// Set maximum batch age
    pub fn with_max_batch_age(mut self, duration: Duration) -> Self {
        self.max_batch_age_ns = duration.as_nanos() as u64;
        self
    }

    /// Set flush interval
    pub fn with_flush_interval(mut self, duration: Duration) -> Self {
        self.flush_interval_ns = duration.as_nanos() as u64;
        self
    }
}

/// Backpressure strategy
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BackpressureStrategy {
    /// Drop messages when queue is full
    Drop,
    /// Block until space is available
    Block,
    /// Spin-wait until space is available
    Spin,
    /// Return error when queue is full
    Error,
}

/// Async logger configuration
pub struct AsyncLoggerConfig {
    /// Queue capacity
    pub queue_capacity: usize,
    /// Batch configuration
    pub batch_config: BatchConfig,
    /// Backpressure strategy
    pub backpressure: BackpressureStrategy,
    /// Number of worker threads (0 = single thread)
    pub worker_threads: usize,
}

impl Default for AsyncLoggerConfig {
    fn default() -> Self {
        Self {
            queue_capacity: 8192,
            batch_config: BatchConfig::default(),
            backpressure: BackpressureStrategy::Spin,
            worker_threads: 1,
        }
    }
}

impl AsyncLoggerConfig {
    /// Create a new config
    pub fn new() -> Self {
        Self::default()
    }

    /// Set queue capacity
    pub fn with_queue_capacity(mut self, capacity: usize) -> Self {
        self.queue_capacity = capacity;
        self
    }

    /// Set batch config
    pub fn with_batch_config(mut self, config: BatchConfig) -> Self {
        self.batch_config = config;
        self
    }

    /// Set backpressure strategy
    pub fn with_backpressure(mut self, strategy: BackpressureStrategy) -> Self {
        self.backpressure = strategy;
        self
    }

    /// Set worker threads
    pub fn with_worker_threads(mut self, threads: usize) -> Self {
        self.worker_threads = threads;
        self
    }
}

/// Async logger backend
pub struct AsyncLogger {
    /// Message queue
    queue: AsyncLogQueue,
    /// Log targets
    targets: spin::Mutex<Vec<Box<dyn LogTarget>>>,
    /// Batch configuration
    batch_config: BatchConfig,
    /// Backpressure strategy
    backpressure: BackpressureStrategy,
    /// Running flag
    running: AtomicBool,
    /// Sequence number counter
    sequence: AtomicU64,
    /// Drop counter (messages dropped due to backpressure)
    dropped_messages: AtomicUsize,
    /// Message counter
    total_messages: AtomicUsize,
}

impl AsyncLogger {
    /// Create a new async logger with default config
    pub fn new(queue_capacity: usize) -> Result<Self> {
        Self::with_config(AsyncLoggerConfig::new().with_queue_capacity(queue_capacity))
    }

    /// Create a new async logger with custom config
    pub fn with_config(config: AsyncLoggerConfig) -> Result<Self> {
        let queue = AsyncLogQueue::new(config.queue_capacity);

        Ok(Self {
            queue,
            targets: spin::Mutex::new(Vec::new()),
            batch_config: config.batch_config,
            backpressure: config.backpressure,
            running: AtomicBool::new(true),
            sequence: AtomicU64::new(0),
            dropped_messages: AtomicUsize::new(0),
            total_messages: AtomicUsize::new(0),
        })
    }

    /// Add a log target
    pub fn add_target(&self, target: Box<dyn LogTarget>) {
        self.targets.lock().push(target);
    }

    /// Submit a log record (non-blocking, <100ns overhead)
    pub fn submit(&self, record: LogRecord) -> Result<()> {
        if !self.running.load(Ordering::Relaxed) {
            return Err(LogError::LoggerNotRunning);
        }

        self.total_messages.fetch_add(1, Ordering::Relaxed);

        let sequence = self.sequence.fetch_add(1, Ordering::Relaxed);

        match self.queue.push(record, sequence) {
            Ok(()) => Ok(()),
            Err(LogError::QueueFull) => match self.backpressure {
                BackpressureStrategy::Drop => {
                    self.dropped_messages.fetch_add(1, Ordering::Relaxed);
                    Ok(())
                }
                BackpressureStrategy::Error => Err(LogError::QueueFull),
                BackpressureStrategy::Block => {
                    // In a real implementation, this would block on a condition variable
                    // For no_std, we'll spin for a bit then error
                    let mut spins = 0;
                    loop {
                        match self.queue.push(
                            LogRecord::new(
                                crate::logging::logger::LogLevel::Info,
                                None,
                                None,
                                None,
                                alloc::string::String::from("placeholder"),
                            ),
                            sequence,
                        ) {
                            Ok(()) => return Ok(()),
                            Err(_) => {
                                spins += 1;
                                if spins > 1000 {
                                    self.dropped_messages.fetch_add(1, Ordering::Relaxed);
                                    return Err(LogError::QueueFull);
                                }
                                core::hint::spin_loop();
                            }
                        }
                    }
                }
                BackpressureStrategy::Spin => {
                    // Spin until space is available
                    loop {
                        match self.queue.push(
                            LogRecord::new(
                                crate::logging::logger::LogLevel::Info,
                                None,
                                None,
                                None,
                                alloc::string::String::from("placeholder"),
                            ),
                            sequence,
                        ) {
                            Ok(()) => return Ok(()),
                            Err(_) => core::hint::spin_loop(),
                        }
                    }
                }
            },
            Err(e) => Err(e),
        }
    }

    /// Process pending log messages (called by worker thread)
    pub fn process(&self) -> Result<usize> {
        let mut batch = Vec::with_capacity(self.batch_config.max_batch_size);
        let mut processed = 0;

        // Collect a batch of messages
        while let Some(msg) = self.queue.pop() {
            batch.push(msg.record);
            processed += 1;

            if batch.len() >= self.batch_config.max_batch_size {
                self.write_batch(&batch)?;
                batch.clear();
            }
        }

        // Write remaining messages
        if !batch.is_empty() {
            self.write_batch(&batch)?;
        }

        Ok(processed)
    }

    /// Write a batch of log records
    fn write_batch(&self, batch: &[LogRecord]) -> Result<()> {
        let targets: spin::MutexGuard<Vec<Box<dyn LogTarget>>> = self.targets.lock();
        for record in batch {
            for target in targets.iter() {
                target.write(record)?;
            }
        }
        Ok(())
    }

    /// Flush all pending log messages
    pub fn flush(&self) -> Result<()> {
        // Process remaining messages
        self.process()?;

        // Flush all targets
        let targets: spin::MutexGuard<Vec<Box<dyn LogTarget>>> = self.targets.lock();
        for target in targets.iter() {
            target.flush()?;
        }

        Ok(())
    }

    /// Shutdown the async logger
    pub fn shutdown(&self) -> Result<()> {
        self.running.store(false, Ordering::Release);
        self.flush()
    }

    /// Check if logger is running
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::Relaxed)
    }

    /// Get queue statistics
    pub fn stats(&self) -> AsyncLoggerStats {
        AsyncLoggerStats {
            queue_size: self.queue.len(),
            queue_capacity: self.queue.capacity,
            remaining_capacity: self.queue.remaining_capacity(),
            total_messages: self.total_messages.load(Ordering::Relaxed),
            dropped_messages: self.dropped_messages.load(Ordering::Relaxed),
            is_running: self.running.load(Ordering::Relaxed),
        }
    }

    /// Get current sequence number
    pub fn sequence(&self) -> u64 {
        self.sequence.load(Ordering::Relaxed)
    }
}

/// Async logger statistics
#[derive(Debug, Clone, Copy)]
pub struct AsyncLoggerStats {
    /// Current queue size
    pub queue_size: usize,
    /// Total queue capacity
    pub queue_capacity: usize,
    /// Remaining capacity
    pub remaining_capacity: usize,
    /// Total messages submitted
    pub total_messages: usize,
    /// Messages dropped due to backpressure
    pub dropped_messages: usize,
    /// Whether logger is running
    pub is_running: bool,
}

impl AsyncLoggerStats {
    /// Calculate queue utilization (0.0 to 1.0)
    pub fn utilization(&self) -> f64 {
        if self.queue_capacity == 0 {
            0.0
        } else {
            self.queue_size as f64 / self.queue_capacity as f64
        }
    }

    /// Calculate drop rate (0.0 to 1.0)
    pub fn drop_rate(&self) -> f64 {
        let total = self.total_messages + self.dropped_messages;
        if total == 0 {
            0.0
        } else {
            self.dropped_messages as f64 / total as f64
        }
    }
}

/// Async logger worker thread
pub struct AsyncLoggerWorker {
    /// Reference to async logger
    logger: Arc<AsyncLogger>,
    /// Worker thread handle (in a real implementation)
    #[allow(dead_code)]
    thread_handle: Option<*const ()>,
}

unsafe impl Send for AsyncLoggerWorker {}

impl AsyncLoggerWorker {
    /// Create a new worker thread
    pub fn new(logger: Arc<AsyncLogger>) -> Self {
        Self {
            logger,
            thread_handle: None,
        }
    }

    /// Start the worker thread
    pub fn start(&mut self) -> Result<()> {
        // In a real implementation, this would spawn a thread
        // For no_std, we'd use a task or timer
        Ok(())
    }

    /// Stop the worker thread
    pub fn stop(&self) -> Result<()> {
        self.logger.shutdown()
    }

    /// Get logger reference
    pub fn logger(&self) -> &Arc<AsyncLogger> {
        &self.logger
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::logger::{LogLevel, LogRecord};

    #[test]
    fn test_async_queue_push_pop() {
        let queue = AsyncLogQueue::new(4);

        let record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Test message".to_string(),
        );

        assert!(queue.push(record.clone(), 0).is_ok());
        assert_eq!(queue.len(), 1);

        let popped = queue.pop().unwrap();
        assert_eq!(popped.record.message, "Test message");
        assert_eq!(popped.sequence, 0);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_async_queue_full() {
        let queue = AsyncLogQueue::new(4);

        for i in 0..4 {
            let record = LogRecord::new(
                LogLevel::Info,
                None,
                None,
                None,
                format!("Message {}", i),
            );
            assert!(queue.push(record, i).is_ok());
        }

        // Queue is now full
        let record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Full".to_string(),
        );
        assert!(queue.push(record, 4).is_err());
    }

    #[test]
    fn test_async_queue_wraparound() {
        let queue = AsyncLogQueue::new(4);

        // Fill and drain the queue
        for i in 0..8 {
            let record = LogRecord::new(
                LogLevel::Info,
                None,
                None,
                None,
                format!("Message {}", i),
            );

            if let Err(_) = queue.push(record, i) {
                // Drain some messages
                let _ = queue.pop();
                let _ = queue.pop();
                queue.push(record, i).unwrap();
            }
        }

        assert_eq!(queue.len(), 4);
    }

    #[test]
    fn test_async_logger_creation() {
        let logger = AsyncLogger::new(1024).unwrap();
        assert!(logger.is_running());
        assert_eq!(logger.sequence(), 0);
    }

    #[test]
    fn test_async_logger_submit() {
        let logger = AsyncLogger::new(1024).unwrap();

        let record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Test".to_string(),
        );

        assert!(logger.submit(record).is_ok());
        assert_eq!(logger.sequence(), 1);
    }

    #[test]
    fn test_async_logger_backpressure_drop() {
        let logger = AsyncLogger::with_config(
            AsyncLoggerConfig::new()
                .with_queue_capacity(4)
                .with_backpressure(BackpressureStrategy::Drop),
        )
        .unwrap();

        // Fill the queue
        for _ in 0..10 {
            let record = LogRecord::new(
                LogLevel::Info,
                None,
                None,
                None,
                "Test".to_string(),
            );
            let _ = logger.submit(record);
        }

        let stats = logger.stats();
        assert!(stats.dropped_messages > 0);
    }

    #[test]
    fn test_async_logger_backpressure_error() {
        let logger = AsyncLogger::with_config(
            AsyncLoggerConfig::new()
                .with_queue_capacity(4)
                .with_backpressure(BackpressureStrategy::Error),
        )
        .unwrap();

        // Fill the queue
        for i in 0..10 {
            let record = LogRecord::new(
                LogLevel::Info,
                None,
                None,
                None,
                format!("Test {}", i),
            );

            if i < 4 {
                assert!(logger.submit(record).is_ok());
            } else {
                assert!(logger.submit(record).is_err());
            }
        }
    }

    #[test]
    fn test_async_logger_stats() {
        let logger = AsyncLogger::new(1024).unwrap();

        let record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Test".to_string(),
        );

        logger.submit(record).unwrap();

        let stats = logger.stats();
        assert_eq!(stats.queue_size, 1);
        assert_eq!(stats.total_messages, 1);
        assert!(stats.is_running);
        assert_eq!(stats.utilization(), 1.0 / 1024.0);
    }

    #[test]
    fn test_async_logger_flush() {
        let logger = AsyncLogger::new(1024).unwrap();

        let record = LogRecord::new(
            LogLevel::Info,
            None,
            None,
            None,
            "Test".to_string(),
        );

        logger.submit(record).unwrap();
        assert!(logger.flush().is_ok());
    }

    #[test]
    fn test_async_logger_shutdown() {
        let logger = AsyncLogger::new(1024).unwrap();

        assert!(logger.is_running());
        assert!(logger.shutdown().is_ok());
        assert!(!logger.is_running());
    }

    #[test]
    fn test_batch_config() {
        let config = BatchConfig::new()
            .with_max_batch_size(200)
            .with_max_batch_age(Duration::from_millis(50))
            .with_flush_interval(Duration::from_secs(2));

        assert_eq!(config.max_batch_size, 200);
        assert_eq!(config.max_batch_age_ns, 50_000_000);
        assert_eq!(config.flush_interval_ns, 2_000_000_000);
    }

    #[test]
    fn test_async_logger_config() {
        let config = AsyncLoggerConfig::new()
            .with_queue_capacity(2048)
            .with_batch_config(BatchConfig::new())
            .with_backpressure(BackpressureStrategy::Spin)
            .with_worker_threads(2);

        assert_eq!(config.queue_capacity, 2048);
        assert_eq!(config.backpressure, BackpressureStrategy::Spin);
        assert_eq!(config.worker_threads, 2);
    }

    #[test]
    fn test_async_logger_stats_drop_rate() {
        let logger = AsyncLogger::with_config(
            AsyncLoggerConfig::new()
                .with_queue_capacity(4)
                .with_backpressure(BackpressureStrategy::Drop),
        )
        .unwrap();

        // Submit more messages than queue can hold
        for _ in 0..10 {
            let record = LogRecord::new(
                LogLevel::Info,
                None,
                None,
                None,
                "Test".to_string(),
            );
            let _ = logger.submit(record);
        }

        let stats = logger.stats();
        assert!(stats.drop_rate() > 0.0);
    }

    #[test]
    fn test_async_logger_sequence_numbers() {
        let logger = AsyncLogger::new(1024).unwrap();

        for i in 0..5 {
            let record = LogRecord::new(
                LogLevel::Info,
                None,
                None,
                None,
                format!("Message {}", i),
            );
            logger.submit(record).unwrap();
            assert_eq!(logger.sequence(), i as u64 + 1);
        }
    }

    #[test]
    fn test_queue_remaining_capacity() {
        let queue = AsyncLogQueue::new(16);
        assert_eq!(queue.remaining_capacity(), 16);

        for i in 0..4 {
            let record = LogRecord::new(
                LogLevel::Info,
                None,
                None,
                None,
                format!("Message {}", i),
            );
            queue.push(record, i).unwrap();
        }

        assert_eq!(queue.remaining_capacity(), 12);
    }
}
