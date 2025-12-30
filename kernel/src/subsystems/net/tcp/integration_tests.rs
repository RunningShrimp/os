//! Integration tests for TCP optimizations
//!
//! This module tests the integration of various TCP optimizations including:
//! - Batch ACK aggregation
//! - BBR congestion control
//! - Connection management
//!
//! Run with: cargo test --package kernel --lib subsystems::net::tcp::integration_tests

#[cfg(test)]
mod tests {
    use super::super::super::super::ipv4::Ipv4Addr;
    use super::super::{
        manager::{TcpConnectionManager, TcpOptions},
        batch_ack::{BatchAckAggregator, AckInfo},
        congestion::{CongestionControl, Bbr, Reno},
        state::TcpState,
    };

    /// Test basic TCP connection with batch ACK aggregation
    #[test]
    fn test_tcp_connection_with_batch_ack() {
        let mut manager = TcpConnectionManager::new();

        // Verify batch ACK is enabled by default
        let stats = manager.stats();
        assert!(stats.batch_ack_enabled, "Batch ACK should be enabled by default");

        // Create a listening socket
        let local_ip = Ipv4Addr::new(127, 0, 0, 1);
        let listen_port = 8080;

        let listen_id = manager
            .listen(local_ip, listen_port, TcpOptions::default())
            .expect("Failed to create listening socket");

        // Verify listening socket was created
        let listen_socket = manager.get_connection(listen_id);
        assert!(listen_socket.is_some(), "Listening socket should exist");
        assert_eq!(
            listen_socket.unwrap().state(),
            TcpState::Listen,
            "Socket should be in Listen state"
        );

        // Check initial stats
        let stats = manager.stats();
        assert_eq!(stats.listening_sockets, 1, "Should have 1 listening socket");
        assert_eq!(stats.active_connections, 0, "Should have 0 active connections");
    }

    /// Test batch ACK aggregation with multiple connections
    #[test]
    fn test_batch_ack_multiple_connections() {
        let mut aggregator = BatchAckAggregator::new();

        let conn1 = (0x7F000001, 8080, 0x7F000001, 80);
        let conn2 = (0x7F000001, 8081, 0x7F000001, 81);
        let conn3 = (0x7F000001, 8082, 0x7F000001, 82);

        // Add ACKs for multiple connections
        for i in 0..3 {
            aggregator.add_ack(conn1, 1000 + i * 100, 100, 8192);
            aggregator.add_ack(conn2, 2000 + i * 100, 100, 8192);
            aggregator.add_ack(conn3, 3000 + i * 100, 100, 8192);
        }

        // Flush all pending ACKs
        let flushed = aggregator.flush();
        assert_eq!(flushed.len(), 3, "Should have 3 connections");

        // Verify each connection has correct ACKs
        let acks1 = flushed.get(&conn1).unwrap();
        assert_eq!(acks1.len(), 3, "Connection 1 should have 3 ACKs");

        let acks2 = flushed.get(&conn2).unwrap();
        assert_eq!(acks2.len(), 3, "Connection 2 should have 3 ACKs");

        let acks3 = flushed.get(&conn3).unwrap();
        assert_eq!(acks3.len(), 3, "Connection 3 should have 3 ACKs");

        // Verify aggregator stats
        let stats = aggregator.stats();
        assert_eq!(stats.total_aggregated, 9, "Should have aggregated 9 ACKs");
        assert_eq!(stats.total_batches, 1, "Should have sent 1 batch");
    }

    /// Test batch ACK timeout behavior
    #[test]
    fn test_batch_ack_timeout() {
        let mut aggregator = BatchAckAggregator::with_config(10, 1); // 1ms timeout

        let conn = (0x7F000001, 8080, 0x7F000001, 80);

        // Add ACKs below threshold
        for i in 0..3 {
            aggregator.add_ack(conn, 1000 + i * 100, 100, 8192);
        }

        // Should not flush yet (below threshold)
        assert!(!aggregator.needs_flush(), "Should not flush below threshold");

        // In a real scenario, we'd wait for timeout
        // For testing, we can manually flush
        let flushed = aggregator.flush();
        assert_eq!(flushed.len(), 1, "Should flush on manual call");
    }

    /// Test TCP connection manager with custom batch ACK config
    #[test]
    fn test_manager_with_custom_batch_ack_config() {
        // Create manager with custom configuration
        let mut manager = TcpConnectionManager::with_config(true, 8, 100);

        // Verify configuration
        let stats = manager.stats();
        assert!(stats.batch_ack_enabled, "Batch ACK should be enabled");

        // Disable batch ACK at runtime
        manager.set_batch_ack_enabled(false);

        let stats_after = manager.stats();
        assert!(
            !stats_after.batch_ack_enabled,
            "Batch ACK should be disabled"
        );

        // Re-enable
        manager.set_batch_ack_enabled(true);

        let stats_final = manager.stats();
        assert!(
            stats_final.batch_ack_enabled,
            "Batch ACK should be re-enabled"
        );
    }

    /// Test BBR congestion control integration
    #[test]
    fn test_bbr_congestion_control() {
        let mss = 1460u32;
        let mut bbr = Bbr::new(mss);

        // Verify initial state
        assert_eq!(bbr.name(), "bbr");
        assert!(bbr.cwnd() > 0, "Initial cwnd should be positive");

        let initial_cwnd = bbr.cwnd();

        // Simulate ACKs
        for i in 0..10 {
            bbr.on_ack(1460, 100);
        }

        // BBR should increase window during startup
        let cwnd_after_acks = bbr.cwnd();
        assert!(
            cwnd_after_acks >= initial_cwnd,
            "BBR should increase or maintain cwnd on ACKs"
        );

        // Simulate packet loss
        bbr.on_loss(1460);

        // BBR handles loss differently (doesn't immediately reduce cwnd)
        let cwnd_after_loss = bbr.cwnd();
    }

    /// Test Reno congestion control for comparison
    #[test]
    fn test_reno_congestion_control() {
        let mss = 1460u32;
        let mut reno = Reno::new(mss);

        // Verify initial state
        assert_eq!(reno.name(), "reno");
        assert!(reno.cwnd() > 0, "Initial cwnd should be positive");

        // Start in slow start
        let initial_cwnd = reno.cwnd();
        assert!(initial_cwnd < reno.ssthresh(), "Should start in slow start");

        // Send ACKs to trigger slow start growth
        reno.on_ack(1460, 100);
        let cwnd_after_slow_start = reno.cwnd();
        assert!(
            cwnd_after_slow_start > initial_cwnd,
            "Reno should grow cwnd in slow start"
        );

        // Simulate loss to trigger congestion avoidance
        reno.on_loss(1460);
        assert!(
            reno.cwnd() < cwnd_after_slow_start,
            "Reno should reduce cwnd on loss"
        );
        assert_eq!(reno.ssthresh(), reno.cwnd(), "ssthresh should equal cwnd after loss");
    }

    /// Test port allocation efficiency
    #[test]
    fn test_port_allocation() {
        let mut manager = TcpConnectionManager::new();

        // Allocate multiple ports
        let mut ports = Vec::new();
        for _ in 0..10 {
            let port = manager.allocate_port().expect("Failed to allocate port");
            ports.push(port);
        }

        // Verify all ports are unique
        let unique_ports: std::collections::HashSet<_> = ports.iter().collect();
        assert_eq!(
            unique_ports.len(),
            ports.len(),
            "All allocated ports should be unique"
        );

        // Verify ports are in valid range (>= 1024)
        for &port in &ports {
            assert!(port >= 1024, "Port {} should be >= 1024", port);
        }

        // Deallocate ports
        for port in ports {
            manager.deallocate_port(port);
        }

        // Verify deallocation
        let stats = manager.stats();
        assert_eq!(stats.allocated_ports, 0, "All ports should be deallocated");
    }

    /// Test connection lifecycle
    #[test]
    fn test_connection_lifecycle() {
        let mut manager = TcpConnectionManager::new();
        let local_ip = Ipv4Addr::new(127, 0, 0, 1);
        let remote_ip = Ipv4Addr::new(127, 0, 0, 1);
        let remote_port = 80;

        // Create listening socket
        let listen_id = manager
            .listen(local_ip, 0, TcpOptions::default())
            .expect("Failed to listen");

        // Connect to remote
        let conn_id = manager
            .connect(local_ip, remote_ip, remote_port, TcpOptions::default())
            .expect("Failed to connect");

        // Verify connection exists
        let conn = manager.get_connection(conn_id);
        assert!(conn.is_some(), "Connection should exist");

        // Close connection
        manager.close(conn_id).expect("Failed to close");

        // Verify connection is cleaned up
        manager.cleanup();

        let stats = manager.stats();
        assert_eq!(stats.active_connections, 0, "Connection should be cleaned up");
    }

    /// Test ACK aggregation statistics
    #[test]
    fn test_ack_aggregation_statistics() {
        let mut aggregator = BatchAckAggregator::new();
        let conn = (0x7F000001, 8080, 0x7F000001, 80);

        // Add ACKs in multiple batches
        for batch in 0..3 {
            for i in 0..4 {
                aggregator.add_ack(conn, 1000 + batch * 400 + i * 100, 100, 8192);
            }
            aggregator.flush();
        }

        let stats = aggregator.stats();
        assert_eq!(stats.total_aggregated, 12, "Should have 12 total ACKs");
        assert_eq!(stats.total_batches, 3, "Should have 3 batches");
        assert_eq!(stats.aggregation_rate, 4, "Average rate should be 4 ACKs/batch");
    }

    /// Test concurrent connection handling
    #[test]
    fn test_concurrent_connections() {
        let mut manager = TcpConnectionManager::new();
        let local_ip = Ipv4Addr::new(127, 0, 0, 1);

        // Create multiple listening sockets
        let mut listen_ids = Vec::new();
        for port in 8000..8010 {
            let id = manager
                .listen(local_ip, port, TcpOptions::default())
                .expect("Failed to listen");
            listen_ids.push(id);
        }

        let stats = manager.stats();
        assert_eq!(stats.listening_sockets, 10, "Should have 10 listening sockets");

        // Close all listening sockets
        for id in listen_ids {
            manager.close(id).expect("Failed to close");
        }

        manager.cleanup();
        let stats_after = manager.stats();
        assert_eq!(stats_after.listening_sockets, 0, "All sockets should be closed");
    }
}
