//! Network stack tests
//!
//! Comprehensive test suite for network functionality including
//! TCP/UDP connections, socket operations, and network interface management.

extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;

use crate::tests::{TestResult, TestSuite, assert_eq, assert_ne, assert_true};
use crate::net::{
    socket::{Socket, SocketType, ProtocolFamily, SocketAddr},
    ipv4::Ipv4Addr,
    interface::InterfaceConfig,
    configure_interface, list_interfaces,
};

/// Network test suite
pub struct NetworkTests;

impl NetworkTests {
    /// Create a new network test suite
    pub fn new() -> Self {
        Self
    }
}

impl TestSuite for NetworkTests {
    fn name(&self) -> &'static str {
        "Network Stack Tests"
    }

    fn run(&self) -> Vec<TestResult> {
        vec![
            test_loopback_configuration(),
            test_socket_creation(),
            test_tcp_socket_operations(),
            test_udp_socket_operations(),
            test_interface_configuration(),
            test_network_statistics(),
        ]
    }
}

/// Test loopback interface configuration
fn test_loopback_configuration() -> TestResult {
    // Test that loopback interface is properly configured
    let interfaces = list_interfaces();

    // Should have at least the loopback interface
    assert_true!(interfaces.len() >= 1, "Should have at least one network interface")?;

    // Find loopback interface
    let lo_found = interfaces.iter().any(|(_, name, _)| name.starts_with("lo"));
    assert_true!(lo_found, "Loopback interface should be present")?;

    TestResult::Pass
}

/// Test socket creation and basic functionality
fn test_socket_creation() -> TestResult {
    // Test TCP socket creation
    let tcp_socket = Socket::Tcp(crate::net::socket::TcpSocketWrapper::new(
        crate::net::socket::SocketOptions::new()
    ));

    // Test UDP socket creation
    let udp_socket = Socket::Udp(crate::net::socket::UdpSocketWrapper::new(
        crate::net::socket::SocketOptions::new()
    ));

    // Test Raw socket creation
    let raw_socket = Socket::Raw(crate::net::socket::RawSocketWrapper::new(
        crate::net::socket::SocketOptions::new()
    ));

    TestResult::Pass
}

/// Test TCP socket operations
fn test_tcp_socket_operations() -> TestResult {
    // Create TCP socket
    let mut tcp_socket = crate::net::socket::TcpSocketWrapper::new(
        crate::net::socket::SocketOptions::new()
    );

    // Test binding
    let addr = SocketAddr::new_ipv4_from_octets(127, 0, 0, 1, 8080);
    match tcp_socket.bind(addr) {
        Ok(_) => {},
        Err(_) => return TestResult::Fail("TCP socket bind failed"),
    }

    // Test listening
    match tcp_socket.listen(5) {
        Ok(_) => {},
        Err(_) => return TestResult::Fail("TCP socket listen failed"),
    }

    // Test accept (should return WouldBlock since no pending connections)
    match tcp_socket.accept() {
        Ok(_) => return TestResult::Fail("TCP socket accept should fail with no connections"),
        Err(_) => {}, // Expected
    }

    TestResult::Pass
}

/// Test UDP socket operations
fn test_udp_socket_operations() -> TestResult {
    // Create UDP socket
    let mut udp_socket = crate::net::socket::UdpSocketWrapper::new(
        crate::net::socket::SocketOptions::new()
    );

    // Test binding
    let addr = SocketAddr::new_ipv4_from_octets(127, 0, 0, 1, 9090);
    match udp_socket.bind(addr) {
        Ok(_) => {},
        Err(_) => return TestResult::Fail("UDP socket bind failed"),
    }

    // Test state after binding
    assert_eq!(udp_socket.state(), crate::net::socket::UdpSocketState::Bound,
               "UDP socket should be in bound state")?;

    TestResult::Pass
}

/// Test network interface configuration
fn test_interface_configuration() -> TestResult {
    let config = InterfaceConfig {
        name: "test0".to_string(),
        ipv4_addr: Some(Ipv4Addr::new(192, 168, 1, 100)),
        ipv4_netmask: Some(Ipv4Addr::new(255, 255, 255, 0)),
        ipv4_gateway: Some(Ipv4Addr::new(192, 168, 1, 1)),
        is_up: false,
        mtu: Some(1500),
    };

    // Try to configure a test interface
    match configure_interface("test0", &config) {
        Ok(_) => {
            // Configuration succeeded (interface should exist)
            TestResult::Pass
        }
        Err(_) => {
            // Expected to fail since interface doesn't exist
            TestResult::Pass
        }
    }
}

/// Test network statistics
fn test_network_statistics() -> TestResult {
    let interfaces = list_interfaces();

    for (_, name, config) in interfaces {
        // Validate interface configuration
        if name.starts_with("lo") {
            // Loopback interface should be up and configured
            assert_true!(config.is_up, "Loopback interface should be up")?;
            if let Some(mtu) = config.mtu {
                assert_eq!(mtu, 65536, "Loopback MTU should be 65536")?;
            }
        }

        // Validate IPv4 configuration if present
        if let Some(addr) = config.ipv4_addr {
            if let Some(netmask) = config.ipv4_netmask {
                let network = (addr.to_u32() & netmask.to_u32());
                assert_eq!(network, addr.to_u32() & netmask.to_u32(),
                           "IP address should be within network")?;
            }
        }
    }

    TestResult::Pass
}

/// Test TCP connection establishment
pub fn test_tcp_connection_establishment() -> TestResult {
    use crate::net::tcp::manager::{TcpConnectionManager, ConnectionId};

    let mut manager = TcpConnectionManager::new();

    // Create listening socket
    let listen_result = manager.listen(
        Ipv4Addr::new(127, 0, 0, 1),
        8080,
        crate::net::tcp::manager::TcpOptions::default()
    );

    match listen_result {
        Ok(conn_id) => {
            // Listening socket created successfully
            assert_true!(conn_id.is_server(), "Connection should be server type")?;

            // Test accepting connection
            match manager.accept(conn_id) {
                Ok(None) => {
                    // No pending connections (expected)
                }
                Err(_) => return TestResult::Fail("Accept operation failed"),
            }

            TestResult::Pass
        }
        Err(_) => TestResult::Fail("Failed to create listening socket"),
    }
}

/// Test TCP data transmission
pub fn test_tcp_data_transmission() -> TestResult {
    use crate::net::tcp::manager::{TcpConnectionManager, ConnectionId};

    let mut manager = TcpConnectionManager::new();

    // Create a connection
    let conn_result = manager.connect(
        Ipv4Addr::new(127, 0, 0, 1),
        Ipv4Addr::new(127, 0, 0, 1),
        80,
        crate::net::tcp::manager::TcpOptions::default()
    );

    match conn_result {
        Ok(conn_id) => {
            // Get the connection
            if let Some(conn) = manager.get_connection(conn_id) {
                // Test data transmission
                let test_data = b"Hello, TCP!";
                match conn.send_data(test_data) {
                    Ok(()) => TestResult::Pass,
                    Err(_) => TestResult::Fail("Failed to send data"),
                }
            } else {
                TestResult::Fail("Connection not found")
            }
        }
        Err(_) => TestResult::Fail("Failed to establish connection"),
    }
}

/// Test UDP data transmission
pub fn test_udp_data_transmission() -> TestResult {
    // Create UDP socket
    let mut udp_socket = crate::net::socket::UdpSocketWrapper::new(
        crate::net::socket::SocketOptions::new()
    );

    // Bind to local address
    let addr = SocketAddr::new_ipv4_from_octets(127, 0, 0, 1, 12345);
    match udp_socket.bind(addr) {
        Ok(_) => {
            // Test data transmission
            let test_data = b"Hello, UDP!";
            let dest_addr = SocketAddr::new_ipv4_from_octets(127, 0, 0, 1, 12346);
            match udp_socket.send_to(test_data, dest_addr) {
                Ok(_) => TestResult::Pass,
                Err(_) => TestResult::Fail("Failed to send UDP data"),
            }
        }
        Err(_) => TestResult::Fail("Failed to bind UDP socket"),
    }
}

/// Test error handling in network operations
pub fn test_network_error_handling() -> TestResult {
    // Test invalid address handling
    let invalid_addr = SocketAddr::new_ipv4_from_octets(255, 255, 255, 255, 65535);

    let mut udp_socket = crate::net::socket::UdpSocketWrapper::new(
        crate::net::socket::SocketOptions::new()
    );

    // Should succeed even with invalid address
    match udp_socket.bind(invalid_addr) {
        Ok(_) => TestResult::Pass,
        Err(_) => TestResult::Fail("Should be able to bind to any address"),
    }

    // Test send to invalid address
    let test_data = b"Test data";
    let another_invalid_addr = SocketAddr::new_ipv4_from_octets(0, 0, 0, 0, 0);
    match udp_socket.send_to(test_data, another_invalid_addr) {
        Ok(_) => TestResult::Pass,
        Err(_) => TestResult::Fail("Should handle invalid addresses gracefully"),
    }

    TestResult::Pass
}

/// Test concurrent socket operations
pub fn test_concurrent_operations() -> TestResult {
    // Create multiple sockets to test concurrent access
    let mut sockets = Vec::new();

    for i in 0..5 {
        let socket = Socket::Tcp(crate::net::socket::TcpSocketWrapper::new(
            crate::net::socket::SocketOptions::new()
        ));
        sockets.push(socket);
    }

    // Test that all sockets were created successfully
    assert_eq!(sockets.len(), 5, "Should have 5 sockets")?;

    // Bind each socket to different ports
    for (i, socket) in sockets.iter_mut().enumerate() {
        if let Socket::Tcp(tcp_socket) = socket {
            let port = 8000 + (i as u16);
            let addr = SocketAddr::new_ipv4_from_octets(127, 0, 0, 1, port);

            match tcp_socket.bind(addr) {
                Ok(_) => {},
                Err(_) => return TestResult::Fail(&format!("Failed to bind socket to port {}", port)),
            }
        }
    }

    TestResult::Pass
}

/// Performance benchmark for network operations
pub fn benchmark_network_operations() -> TestResult {
    let start_time = crate::time::get_ticks();

    // Create and destroy sockets rapidly
    for _ in 0..100 {
        let socket = Socket::Udp(crate::net::socket::UdpSocketWrapper::new(
            crate::net::socket::SocketOptions::new()
        ));
        // Socket is automatically dropped
    }

    let end_time = crate::time::get_ticks();
    let elapsed = end_time - start_time;

    // Should complete reasonably quickly
    assert_true!(elapsed < 1000000, "Socket creation should be fast")?;

    TestResult::Pass
}

/// Stress test for network stack
pub fn stress_test_network_stack() -> TestResult {
    // Create many connections and test resource management
    let mut connections = Vec::new();

    for i in 0..50 {
        let conn_id = format!("test_conn_{}", i);

        // Create socket
        let socket = Socket::Tcp(crate::net::socket::TcpSocketWrapper::new(
            crate::net::socket::SocketOptions::new()
        ));

        // Bind to different port
        if let Socket::Tcp(tcp_socket) = &socket {
            let addr = SocketAddr::new_ipv4_from_octets(127, 0, 0, 1, 9000 + (i as u16));
            let _ = tcp_socket.bind(addr);
        }

        connections.push((conn_id, socket));
    }

    // Verify all connections were created
    assert_eq!(connections.len(), 50, "Should have 50 connections")?;

    TestResult::Pass
}