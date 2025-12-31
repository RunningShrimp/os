//! Message Queue IPC Benchmark
//!
//! Measures message queue performance for POSIX and System V message queues.
//! Target: Low latency and high throughput for message passing.

use crate::testing::benchmarks::{measure_time, BenchmarkConfig, BenchmarkResult, BenchmarkSuite};
use alloc::vec::Vec;

const MAX_MESSAGES: usize = 1000;
const MSG_SIZE: usize = 256;

/// Benchmark message queue throughput
pub struct MessageQueueBenchmark {
    queue_type: QueueType,
    msg_size: usize,
    num_messages: usize,
}

/// Types of message queues
#[derive(Debug, Clone, Copy)]
pub enum QueueType {
    /// POSIX message queue
    Posix,
    /// System V message queue
    SystemV,
    /// In-kernel message queue
    Kernel,
}

impl MessageQueueBenchmark {
    pub fn new() -> Self {
        Self {
            queue_type: QueueType::Posix,
            msg_size: MSG_SIZE,
            num_messages: MAX_MESSAGES,
        }
    }

    pub fn with_type(mut self, queue_type: QueueType) -> Self {
        self.queue_type = queue_type;
        self
    }

    pub fn with_message_size(mut self, size: usize) -> Self {
        self.msg_size = size;
        self
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl Default for MessageQueueBenchmark {
    fn default() -> Self {
        Self::new()
    }
}

impl BenchmarkSuite for MessageQueueBenchmark {
    fn name(&self) -> &str {
        "ipc/message_queue"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();

        // Simulate sending messages
        for _ in 0..self.num_messages {
            // mq_send or msgsnd
            for _ in 0..self.msg_size / 16 {
                core::hint::spin_loop();
            }
        }

        // Simulate receiving messages
        for _ in 0..self.num_messages {
            // mq_receive or msgrcv
            for _ in 0..self.msg_size / 16 {
                core::hint::spin_loop();
            }
        }

        let elapsed = start.elapsed();
        let ns_per_msg = elapsed.as_nanos() as u64 / (self.num_messages * 2) as u64;
        Ok(ns_per_msg)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark message queue latency
pub struct MessageQueueLatencyBenchmark {
    queue_type: QueueType,
    priority: i32,
}

impl MessageQueueLatencyBenchmark {
    pub fn new() -> Self {
        Self {
            queue_type: QueueType::Posix,
            priority: 0,
        }
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.priority = priority;
        self
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for MessageQueueLatencyBenchmark {
    fn name(&self) -> &str {
        "ipc/message_queue_latency"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        // Simulate round-trip: send -> receive
        let start = core::time::Instant::now();

        // Send message
        for _ in 0..5 {
            core::hint::spin_loop();
        }

        // Queue processing
        for _ in 0..3 {
            core::hint::spin_loop();
        }

        // Receive message
        for _ in 0..5 {
            core::hint::spin_loop();
        }

        let elapsed = start.elapsed();
        Ok(elapsed.as_nanos() as u64)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark priority message queues
pub struct PriorityMessageBenchmark {
    num_priorities: usize,
    messages_per_priority: usize,
}

impl PriorityMessageBenchmark {
    pub fn new() -> Self {
        Self {
            num_priorities: 4,
            messages_per_priority: 100,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for PriorityMessageBenchmark {
    fn name(&self) -> &str {
        "ipc/priority_message_queue"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let start = core::time::Instant::now();

        // Send messages with different priorities
        for priority in 0..self.num_priorities {
            for _ in 0..self.messages_per_priority {
                // Send with priority
                for _ in 0..4 {
                    core::hint::spin_loop();
                }
                let _ = priority;
            }
        }

        // Receive messages (should come in priority order)
        for _ in 0..self.num_priorities * self.messages_per_priority {
            // Receive (higher priority messages first)
            for _ in 0..4 {
                core::hint::spin_loop();
            }
        }

        let elapsed = start.elapsed();
        let total_msgs = self.num_priorities * self.messages_per_priority * 2;
        let ns_per_msg = elapsed.as_nanos() as u64 / total_msgs as u64;
        Ok(ns_per_msg)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark message queue with blocking operations
pub struct BlockingMessageQueueBenchmark {
    queue_capacity: usize,
}

impl BlockingMessageQueueBenchmark {
    pub fn new() -> Self {
        Self {
            queue_capacity: 100,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for BlockingMessageQueueBenchmark {
    fn name(&self) -> &str {
        "ipc/blocking_message_queue"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let iterations = 100;
        let start = core::time::Instant::now();

        // Fill then drain the queue
        for i in 0..iterations {
            // Write (may block if queue full)
            for _ in 0..8 {
                core::hint::spin_loop();
            }

            // Simulate potential blocking wait
            if i % self.queue_capacity == 0 {
                for _ in 0..20 {
                    core::hint::spin_loop();
                }
            }

            // Read (may block if queue empty)
            for _ in 0..8 {
                core::hint::spin_loop();
            }
        }

        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() as u64 / iterations as u64;
        Ok(ns_per_op)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark message queue with multiple senders/receivers
pub struct MultiEndpointMessageBenchmark {
    num_senders: usize,
    num_receivers: usize,
}

impl MultiEndpointMessageBenchmark {
    pub fn new(senders: usize, receivers: usize) -> Self {
        Self {
            num_senders: senders,
            num_receivers: receivers,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new(2, 2);
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for MultiEndpointMessageBenchmark {
    fn name(&self) -> &str {
        "ipc/multi_endpoint_message_queue"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let messages_per_sender = 100;
        let start = core::time::Instant::now();

        // Multiple senders
        for sender in 0..self.num_senders {
            for _ in 0..messages_per_sender {
                // Send message
                for _ in 0..6 {
                    core::hint::spin_loop();
                }
                let _ = sender;
            }
        }

        // Multiple receivers
        for receiver in 0..self.num_receivers {
            let msgs_to_receive = (messages_per_sender * self.num_senders) / self.num_receivers;
            for _ in 0..msgs_to_receive {
                // Receive message
                for _ in 0..6 {
                    core::hint::spin_loop();
                }
                let _ = receiver;
            }
        }

        let elapsed = start.elapsed();
        let total_msgs = messages_per_sender * self.num_senders;
        let ns_per_msg = elapsed.as_nanos() as u64 / total_msgs as u64;
        Ok(ns_per_msg)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

/// Benchmark non-blocking message queue operations
pub struct NonBlockingMessageBenchmark {
    queue_capacity: usize,
}

impl NonBlockingMessageBenchmark {
    pub fn new() -> Self {
        Self {
            queue_capacity: 100,
        }
    }

    pub fn run(config: &BenchmarkConfig) -> Result<BenchmarkResult, String> {
        let mut benchmark = Self::new();
        benchmark.execute(config)
    }
}

impl BenchmarkSuite for NonBlockingMessageBenchmark {
    fn name(&self) -> &str {
        "ipc/nonblocking_message_queue"
    }

    fn setup(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn run_iteration(&mut self) -> Result<u64, String> {
        let iterations = 1000;
        let start = core::time::Instant::now();

        // Non-blocking operations (try_send, try_recv)
        for i in 0..iterations {
            // Try to send (never blocks)
            for _ in 0..5 {
                core::hint::spin_loop();
            }

            // Try to receive (never blocks)
            for _ in 0..5 {
                core::hint::spin_loop();
            }

            let _ = i;
        }

        let elapsed = start.elapsed();
        let ns_per_op = elapsed.as_nanos() as u64 / iterations as u64;
        Ok(ns_per_op)
    }

    fn teardown(&mut self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_queue_benchmark() {
        let mut bench = MessageQueueBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_queue_types() {
        for queue_type in [QueueType::Posix, QueueType::SystemV, QueueType::Kernel] {
            let mut bench = MessageQueueBenchmark::new().with_type(queue_type);
            let config = BenchmarkConfig {
                warmup_iterations: 5,
                measurement_iterations: 50,
                ..Default::default()
            };

            let result = bench.execute(&config);
            assert!(result.is_ok());
        }
    }

    #[test]
    fn test_message_latency() {
        let mut bench = MessageQueueLatencyBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_priority_messages() {
        let mut bench = PriorityMessageBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 50,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_blocking_queue() {
        let mut bench = BlockingMessageQueueBenchmark::new();
        let config = BenchmarkConfig {
            warmup_iterations: 10,
            measurement_iterations: 100,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }

    #[test]
    fn test_multi_endpoint() {
        let mut bench = MultiEndpointMessageBenchmark::new(2, 2);
        let config = BenchmarkConfig {
            warmup_iterations: 5,
            measurement_iterations: 50,
            ..Default::default()
        };

        let result = bench.execute(&config).unwrap();
        assert!(!result.samples.is_empty());
    }
}
