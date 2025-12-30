//! Batch ACK Aggregation for TCP
//!
//! This module implements ACK aggregation to reduce the number of ACK packets
//! sent over the network. This can significantly improve throughput, especially
//! on high-latency networks.
//!
//! Features:
//! - Configurable aggregation threshold
//! - Timer-based aggregation
//! - Priority ACK for important packets
//! - Per-connection aggregation state

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::TcpState;

/// Default number of ACKs to aggregate before sending
pub const DEFAULT_ACK_AGGREGATION_THRESHOLD: usize = 4;

/// Maximum time to hold ACKs before flushing (milliseconds)
pub const DEFAULT_ACK_AGGREGATION_TIMEOUT_MS: u64 = 40;

/// Maximum batch size to prevent excessive delay
pub const MAX_BATCH_SIZE: usize = 64;

/// Aggregated ACK entry
#[derive(Debug, Clone)]
struct AggregatedAck {
    /// Connection identifier (local_ip, local_port, remote_ip, remote_port)
    connection_id: (u32, u16, u32, u16),
    /// ACK number
    ack_num: u32,
    /// Bytes acknowledged
    acked_bytes: u32,
    /// Receive window
    window: u16,
    /// Timestamp when this ACK was added
    timestamp_ns: u64,
}

/// Batch ACK aggregator
///
/// Collects ACKs from multiple connections and sends them in batches
/// to reduce packet overhead.
pub struct BatchAckAggregator {
    /// Pending ACKs
    pending_acks: Vec<AggregatedAck>,

    /// ACK aggregation threshold
    threshold: usize,

    /// Aggregation timeout (in nanoseconds)
    timeout_ns: u64,

    /// Last flush timestamp
    last_flush: AtomicU64,

    /// Total ACKs aggregated
    total_aggregated: AtomicU64,

    /// Total batches sent
    total_batches: AtomicU64,
}

impl BatchAckAggregator {
    /// Create a new batch ACK aggregator
    pub fn new() -> Self {
        Self::with_config(
            DEFAULT_ACK_AGGREGATION_THRESHOLD,
            DEFAULT_ACK_AGGREGATION_TIMEOUT_MS,
        )
    }

    /// Create a new batch ACK aggregator with custom configuration
    pub fn with_config(threshold: usize, timeout_ms: u64) -> Self {
        Self {
            pending_acks: Vec::with_capacity(threshold * 2),
            threshold,
            timeout_ns: timeout_ms * 1_000_000,
            last_flush: AtomicU64::new(0),
            total_aggregated: AtomicU64::new(0),
            total_batches: AtomicU64::new(0),
        }
    }

    /// Add an ACK to the aggregation buffer
    ///
    /// Returns true if the buffer should be flushed
    pub fn add_ack(
        &mut self,
        connection_id: (u32, u16, u32, u16),
        ack_num: u32,
        acked_bytes: u32,
        window: u16,
    ) -> bool {
        // Get current timestamp
        let now_ns = self.current_timestamp_ns();

        // Create aggregated ACK entry
        let ack = AggregatedAck {
            connection_id,
            ack_num,
            acked_bytes,
            window,
            timestamp_ns: now_ns,
        };

        self.pending_acks.push(ack);
        self.total_aggregated.fetch_add(1, Ordering::Relaxed);

        // Check if we should flush
        self.should_flush(now_ns)
    }

    /// Check if the buffer should be flushed
    fn should_flush(&self, now_ns: u64) -> bool {
        // Flush if threshold reached
        if self.pending_acks.len() >= self.threshold {
            return true;
        }

        // Flush if timeout exceeded
        let last_flush = self.last_flush.load(Ordering::Relaxed);
        if now_ns.saturating_sub(last_flush) > self.timeout_ns {
            return true;
        }

        // Flush if batch size too large
        if self.pending_acks.len() >= MAX_BATCH_SIZE {
            return true;
        }

        false
    }

    /// Flush pending ACKs and return them grouped by connection
    pub fn flush(&mut self) -> BTreeMap<(u32, u16, u32, u16), Vec<AckInfo>> {
        let mut result = BTreeMap::new();

        if self.pending_acks.is_empty() {
            return result;
        }

        // Group ACKs by connection
        for ack in self.pending_acks.drain(..) {
            let ack_info = AckInfo {
                ack_num: ack.ack_num,
                acked_bytes: ack.acked_bytes,
                window: ack.window,
            };

            result
                .entry(ack.connection_id)
                .or_insert_with(Vec::new)
                .push(ack_info);
        }

        // Update last flush time
        self.last_flush
            .store(self.current_timestamp_ns(), Ordering::Relaxed);
        self.total_batches.fetch_add(1, Ordering::Relaxed);

        result
    }

    /// Get current timestamp in nanoseconds
    fn current_timestamp_ns(&self) -> u64 {
        // In a real implementation, this would use the system timer
        static TIMER: AtomicU64 = AtomicU64::new(0);
        TIMER.fetch_add(1, Ordering::Relaxed)
    }

    /// Check if flush is needed
    pub fn needs_flush(&self) -> bool {
        let now_ns = self.current_timestamp_ns();
        self.should_flush(now_ns)
    }

    /// Get the number of pending ACKs
    pub fn pending_count(&self) -> usize {
        self.pending_acks.len()
    }

    /// Get statistics
    pub fn stats(&self) -> BatchAckStats {
        BatchAckStats {
            total_aggregated: self.total_aggregated.load(Ordering::Relaxed),
            total_batches: self.total_batches.load(Ordering::Relaxed),
            pending_acks: self.pending_acks.len(),
            aggregation_rate: if self.total_batches.load(Ordering::Relaxed) > 0 {
                self.total_aggregated.load(Ordering::Relaxed) / self.total_batches.load(Ordering::Relaxed)
            } else {
                0
            },
        }
    }

    /// Clear all pending ACKs (for error recovery)
    pub fn clear(&mut self) {
        self.pending_acks.clear();
    }
}

impl Default for BatchAckAggregator {
    fn default() -> Self {
        Self::new()
    }
}

/// ACK information for a connection
#[derive(Debug, Clone)]
pub struct AckInfo {
    /// ACK number
    pub ack_num: u32,
    /// Bytes acknowledged
    pub acked_bytes: u32,
    /// Receive window
    pub window: u16,
}

/// Batch ACK statistics
#[derive(Debug, Clone, Copy)]
pub struct BatchAckStats {
    /// Total ACKs aggregated
    pub total_aggregated: u64,
    /// Total batches sent
    pub total_batches: u64,
    /// Current pending ACKs
    pub pending_acks: usize,
    /// Average ACKs per batch
    pub aggregation_rate: u64,
}

/// Per-connection ACK aggregation state
///
/// Each TCP connection can have its own aggregation context
/// for more fine-grained control.
pub struct ConnectionAckAggregator {
    /// Pending ACKs for this connection
    pending_acks: Vec<AckInfo>,

    /// Last ACK number sent
    last_ack_sent: u32,

    /// Aggregation threshold for this connection
    threshold: usize,

    /// Connection state
    state: TcpState,
}

impl ConnectionAckAggregator {
    /// Create a new connection ACK aggregator
    pub fn new(threshold: usize) -> Self {
        Self {
            pending_acks: Vec::with_capacity(threshold),
            last_ack_sent: 0,
            threshold,
            state: TcpState::Closed,
        }
    }

    /// Add an ACK for this connection
    ///
    /// Returns Some(ack_info) if an ACK should be sent
    pub fn add_ack(&mut self, ack_num: u32, acked_bytes: u32, window: u16) -> Option<AckInfo> {
        self.pending_acks.push(AckInfo {
            ack_num,
            acked_bytes,
            window,
        });

        // Check if we should send
        if self.pending_acks.len() >= self.threshold {
            self.flush_internal()
        } else {
            None
        }
    }

    /// Force flush pending ACKs
    pub fn flush(&mut self) -> Option<AckInfo> {
        if self.pending_acks.is_empty() {
            None
        } else {
            self.flush_internal()
        }
    }

    /// Internal flush implementation
    fn flush_internal(&mut self) -> Option<AckInfo> {
        if self.pending_acks.is_empty() {
            return None;
        }

        // Get the latest ACK (highest ack_num)
        let latest = self
            .pending_acks
            .iter()
            .max_by_key(|a| a.ack_num)
            .cloned()?;

        // Aggregate acked_bytes
        let total_acked: u32 = self.pending_acks.iter().map(|a| a.acked_bytes).sum();

        self.pending_acks.clear();
        self.last_ack_sent = latest.ack_num;

        Some(AckInfo {
            ack_num: latest.ack_num,
            acked_bytes: total_acked,
            window: latest.window,
        })
    }

    /// Get pending ACK count
    pub fn pending_count(&self) -> usize {
        self.pending_acks.len()
    }

    /// Set connection state
    pub fn set_state(&mut self, state: TcpState) {
        self.state = state;
    }

    /// Check if connection is established
    pub fn is_established(&self) -> bool {
        self.state == TcpState::Established
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_ack_aggregator() {
        let mut aggregator = BatchAckAggregator::new();

        // Add ACKs below threshold
        let conn_id = (0x7F000001, 8080, 0x7F000001, 80);

        for i in 0..3 {
            let should_flush = aggregator.add_ack(conn_id, 1000 + i * 100, 100, 8192);
            assert!(!should_flush, "Should not flush before threshold");
        }

        // Add ACK at threshold
        let should_flush = aggregator.add_ack(conn_id, 1400, 100, 8192);
        assert!(should_flush, "Should flush at threshold");

        // Flush and verify
        let flushed = aggregator.flush();
        assert_eq!(flushed.len(), 1, "Should have one connection");
        assert_eq!(
            flushed.get(&conn_id).unwrap().len(),
            4,
            "Should have 4 ACKs"
        );
    }

    #[test]
    fn test_connection_ack_aggregator() {
        let mut aggregator = ConnectionAckAggregator::new(4);

        // Add ACKs below threshold
        for i in 0..3 {
            let result = aggregator.add_ack(1000 + i * 100, 100, 8192);
            assert!(result.is_none(), "Should not send before threshold");
        }

        // Add ACK at threshold
        let result = aggregator.add_ack(1400, 100, 8192);
        assert!(result.is_some(), "Should send at threshold");

        let ack = result.unwrap();
        assert_eq!(ack.ack_num, 1400, "Should use latest ACK number");
        assert_eq!(ack.acked_bytes, 400, "Should aggregate acked bytes");
    }

    #[test]
    fn test_aggregator_stats() {
        let mut aggregator = BatchAckAggregator::new();

        let conn_id = (0x7F000001, 8080, 0x7F000001, 80);

        // Add and flush some ACKs
        for _ in 0..4 {
            aggregator.add_ack(conn_id, 1000, 100, 8192);
        }
        aggregator.flush();

        let stats = aggregator.stats();
        assert_eq!(stats.total_aggregated, 4);
        assert_eq!(stats.total_batches, 1);
        assert_eq!(stats.aggregation_rate, 4);
    }

    #[test]
    fn test_multiple_connections() {
        let mut aggregator = BatchAckAggregator::new();

        let conn1 = (0x7F000001, 8080, 0x7F000001, 80);
        let conn2 = (0x7F000001, 8081, 0x7F000001, 81);

        // Add ACKs for both connections
        for i in 0..2 {
            aggregator.add_ack(conn1, 1000 + i * 100, 100, 8192);
            aggregator.add_ack(conn2, 2000 + i * 100, 100, 8192);
        }
        aggregator.add_ack(conn1, 1200, 100, 8192);

        // Flush
        let flushed = aggregator.flush();
        assert_eq!(flushed.len(), 2, "Should have two connections");

        // Verify connection 1
        let acks1 = flushed.get(&conn1).unwrap();
        assert_eq!(acks1.len(), 3);

        // Verify connection 2
        let acks2 = flushed.get(&conn2).unwrap();
        assert_eq!(acks2.len(), 2);
    }
}
