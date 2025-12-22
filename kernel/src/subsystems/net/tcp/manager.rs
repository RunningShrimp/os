//! TCP connection manager
//!
//! This module provides management for multiple TCP connections, including
//! connection tracking, socket allocation, and connection lifecycle management.

extern crate alloc;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use alloc::vec;
use core::sync::atomic::{AtomicU16, AtomicU32, Ordering};

use super::TcpState;
use super::state::{TcpStateMachine, TcpAction};
use crate::net::ipv4::Ipv4Addr;

/// TCP connection identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConnectionId {
    /// Local IP address
    pub local_ip: Ipv4Addr,
    /// Local port
    pub local_port: u16,
    /// Remote IP address
    pub remote_ip: Ipv4Addr,
    /// Remote port
    pub remote_port: u16,
}

impl ConnectionId {
    /// Create a new connection ID
    pub fn new(
        local_ip: Ipv4Addr,
        local_port: u16,
        remote_ip: Ipv4Addr,
        remote_port: u16,
    ) -> Self {
        Self {
            local_ip,
            local_port,
            remote_ip,
            remote_port,
        }
    }

    /// Check if this is a server connection (listening)
    pub fn is_server(&self) -> bool {
        self.remote_ip == Ipv4Addr::UNSPECIFIED || self.remote_port == 0
    }

    /// Create a wildcard connection ID (for listening sockets)
    pub fn wildcard(local_ip: Ipv4Addr, local_port: u16) -> Self {
        Self {
            local_ip,
            local_port,
            remote_ip: Ipv4Addr::UNSPECIFIED,
            remote_port: 0,
        }
    }
}

/// TCP connection
#[derive(Debug, Clone)]
pub struct TcpConnection {
    /// Connection ID
    pub id: ConnectionId,
    /// State machine
    pub state_machine: TcpStateMachine,
    /// Connection options
    pub options: TcpOptions,
    /// Statistics
    pub stats: TcpConnectionStats,
    /// Receive buffer
    pub recv_buffer: Vec<u8>,
    /// Send buffer
    pub send_buffer: Vec<u8>,
    /// Pending actions
    pub pending_actions: Vec<TcpAction>,
}

/// TCP connection options
#[derive(Debug, Clone)]
pub struct TcpOptions {
    /// Keep-alive enabled
    pub keep_alive: bool,
    /// Keep-alive interval (seconds)
    pub keep_alive_interval: u32,
    /// Keep-alive time (seconds)
    pub keep_alive_time: u32,
    /// Maximum keep-alive probes
    pub keep_alive_probes: u32,
    /// Nagle's algorithm enabled
    pub nagle_enabled: bool,
    /// Reuse address
    pub reuse_addr: bool,
    /// Reuse port
    pub reuse_port: bool,
    /// Receive buffer size
    pub recv_buf_size: u32,
    /// Send buffer size
    pub send_buf_size: u32,
}

impl Default for TcpOptions {
    fn default() -> Self {
        Self {
            keep_alive: false,
            keep_alive_interval: 30,
            keep_alive_time: 7200, // 2 hours
            keep_alive_probes: 9,
            nagle_enabled: true,
            reuse_addr: false,
            reuse_port: false,
            recv_buf_size: 8192,
            send_buf_size: 8192,
        }
    }
}

/// TCP connection statistics
#[derive(Debug, Clone, Default)]
pub struct TcpConnectionStats {
    /// Bytes transmitted
    pub bytes_tx: u64,
    /// Bytes received
    pub bytes_rx: u64,
    /// Packets transmitted
    pub packets_tx: u64,
    /// Packets received
    pub packets_rx: u64,
    /// Retransmissions
    pub retransmissions: u64,
    /// Connection establishment time
    pub establishment_time: Option<u64>,
    /// Data transfer time
    pub data_transfer_time: u64,
}

impl TcpConnection {
    /// Create a new TCP connection
    pub fn new(id: ConnectionId, options: TcpOptions, is_passive: bool) -> Self {
        Self {
            id,
            state_machine: TcpStateMachine::new(is_passive),
            options,
            stats: TcpConnectionStats::default(),
            recv_buffer: Vec::new(),
            send_buffer: Vec::new(),
            pending_actions: Vec::new(),
        }
    }

    /// Process incoming packet
    pub fn process_packet(&mut self, packet: &[u8]) -> Result<Vec<TcpAction>, TcpError> {
        use super::TcpPacket;
        // PacketError在当前作用域中未使用，暂时注释掉
        // use crate::net::packet::PacketError;

        let tcp_packet = TcpPacket::from_bytes(packet)
            .map_err(|_| TcpError::InvalidPacket)?;

        let action = self.state_machine.process_packet(&tcp_packet);

        // Update statistics
        self.stats.packets_rx += 1;
        self.stats.bytes_rx += tcp_packet.payload.len() as u64;

        // Handle action
        match &action {
            TcpAction::DataReceived(data) => {
                self.recv_buffer.extend_from_slice(&data);
                Ok(vec![action])
            }
            TcpAction::ConnectionEstablished => {
                self.stats.establishment_time = Some(get_current_time());
                Ok(vec![action])
            }
            _ => Ok(vec![action]),
        }
    }

    /// Send data
    pub fn send_data(&mut self, data: &[u8]) -> Result<(), TcpError> {
        if !self.state_machine.is_established() {
            return Err(TcpError::NotConnected);
        }

        if self.send_buffer.len() + data.len() > self.options.send_buf_size as usize {
            return Err(TcpError::BufferFull);
        }

        self.send_buffer.extend_from_slice(data);
        Ok(())
    }

    /// Receive data
    pub fn receive_data(&mut self, buf: &mut [u8]) -> Result<usize, TcpError> {
        if self.recv_buffer.is_empty() {
            return Ok(0);
        }

        let copy_len = core::cmp::min(buf.len(), self.recv_buffer.len());
        buf[..copy_len].copy_from_slice(&self.recv_buffer[..copy_len]);
        self.recv_buffer.drain(..copy_len);

        Ok(copy_len)
    }

    /// Get pending actions
    pub fn get_pending_actions(&mut self) -> Vec<TcpAction> {
        let actions = self.pending_actions.clone();
        self.pending_actions.clear();
        actions
    }

    /// Add pending action
    pub fn add_pending_action(&mut self, action: TcpAction) {
        self.pending_actions.push(action);
    }

    /// Check timeouts
    pub fn check_timeouts(&mut self) -> Vec<TcpAction> {
        self.state_machine.check_timeouts()
    }

    /// Get connection state
    pub fn state(&self) -> TcpState {
        self.state_machine.state()
    }

    /// Check if connection is established
    pub fn is_established(&self) -> bool {
        self.state_machine.is_established()
    }

    /// Check if connection is closed
    pub fn is_closed(&self) -> bool {
        self.state_machine.is_closed()
    }

    /// Get connection statistics
    pub fn get_stats(&self) -> &TcpConnectionStats {
        &self.stats
    }
}

/// TCP connection manager
pub struct TcpConnectionManager {
    /// Active connections
    connections: BTreeMap<ConnectionId, TcpConnection>,
    /// Listening sockets
    listening_sockets: BTreeMap<ConnectionId, TcpConnection>,
    /// Next port to allocate
    next_port: AtomicU16,
    /// Port allocation bitmap (simplified)
    allocated_ports: BTreeMap<u16, bool>,
    /// Connection ID counter
    next_connection_id: AtomicU32,
}

impl TcpConnectionManager {
    /// Create a new TCP connection manager
    pub fn new() -> Self {
        Self {
            connections: BTreeMap::new(),
            listening_sockets: BTreeMap::new(),
            next_port: AtomicU16::new(1024), // Start from port 1024
            allocated_ports: BTreeMap::new(),
            next_connection_id: AtomicU32::new(1),
        }
    }

    /// Allocate a new port
    pub fn allocate_port(&mut self) -> Result<u16, TcpError> {
        let start_port = self.next_port.load(Ordering::Relaxed);

        for i in 0..=65535 {
            let port = start_port.wrapping_add(i);

            // Skip well-known ports and system ports
            if port < 1024 {
                continue;
            }

            if !self.allocated_ports.contains_key(&port) {
                self.allocated_ports.insert(port, true);
                self.next_port.store(port.wrapping_add(1), Ordering::Relaxed);
                return Ok(port);
            }
        }

        Err(TcpError::NoPortsAvailable)
    }

    /// Deallocate a port
    pub fn deallocate_port(&mut self, port: u16) {
        self.allocated_ports.remove(&port);
    }

    /// Create a listening socket
    pub fn listen(
        &mut self,
        local_ip: Ipv4Addr,
        local_port: u16,
        options: TcpOptions,
    ) -> Result<ConnectionId, TcpError> {
        let port = if local_port == 0 {
            self.allocate_port()?
        } else {
            if self.allocated_ports.contains_key(&local_port) {
                return Err(TcpError::PortInUse);
            }
            self.allocated_ports.insert(local_port, true);
            local_port
        };

        let conn_id = ConnectionId::wildcard(local_ip, port);
        let mut connection = TcpConnection::new(conn_id, options.clone(), true);

        // Start listening
        let action = connection.state_machine.passive_open();
        connection.add_pending_action(action);

        self.listening_sockets.insert(conn_id, connection);
        Ok(conn_id)
    }

    /// Connect to a remote host
    pub fn connect(
        &mut self,
        local_ip: Ipv4Addr,
        remote_ip: Ipv4Addr,
        remote_port: u16,
        options: TcpOptions,
    ) -> Result<ConnectionId, TcpError> {
        let local_port = self.allocate_port()?;
        let conn_id = ConnectionId::new(local_ip, local_port, remote_ip, remote_port);

        let mut connection = TcpConnection::new(conn_id, options.clone(), false);

        // Start connection
        let action = connection.state_machine.active_open(remote_ip, remote_port);
        connection.add_pending_action(action);

        self.connections.insert(conn_id, connection);
        Ok(conn_id)
    }

    /// Accept a new connection
    pub fn accept(&mut self, listening_id: ConnectionId) -> Result<Option<ConnectionId>, TcpError> {
        let listening_socket = self.listening_sockets.get_mut(&listening_id)
            .ok_or(TcpError::InvalidConnection)?;

        // Check for pending connections (simplified implementation)
        // In a real implementation, this would use a pending connections queue
        let mut has_new_connection = false;
        for action in listening_socket.get_pending_actions() {
            match action {
                TcpAction::ConnectionEstablished => {
                    has_new_connection = true;
                    break;
                }
                _ => {}
            }
        }

        if has_new_connection {
            // Create new connection for accepted socket
            // This is simplified - would need actual remote address from SYN
            let new_conn_id = ConnectionId::new(
                listening_socket.id.local_ip,
                listening_socket.id.local_port,
                Ipv4Addr::UNSPECIFIED, // Would come from actual SYN
                0, // Would come from actual SYN
            );

            let new_connection = TcpConnection::new(new_conn_id, listening_socket.options.clone(), true);
            self.connections.insert(new_conn_id, new_connection);

            Ok(Some(new_conn_id))
        } else {
            Ok(None)
        }
    }

    /// Close a connection
    pub fn close(&mut self, conn_id: ConnectionId) -> Result<(), TcpError> {
        if let Some(mut connection) = self.connections.remove(&conn_id) {
            let action = connection.state_machine.close();
            connection.add_pending_action(action);

            // Put connection back to handle closure process
            self.connections.insert(conn_id, connection);

            // Deallocate port if it's a listening socket
            if self.listening_sockets.contains_key(&conn_id) {
                self.deallocate_port(conn_id.local_port);
                self.listening_sockets.remove(&conn_id);
            }

            Ok(())
        } else {
            Err(TcpError::InvalidConnection)
        }
    }

    /// Find connection by 4-tuple
    pub fn find_connection(
        &self,
        local_ip: Ipv4Addr,
        local_port: u16,
        remote_ip: Ipv4Addr,
        remote_port: u16,
    ) -> Option<&TcpConnection> {
        let conn_id = ConnectionId::new(local_ip, local_port, remote_ip, remote_port);
        self.connections.get(&conn_id)
    }

    /// Find connection by ID
    pub fn get_connection(&self, conn_id: ConnectionId) -> Option<&TcpConnection> {
        self.connections.get(&conn_id)
            .or_else(|| self.listening_sockets.get(&conn_id))
    }

    /// Get mutable connection by ID
    pub fn get_connection_mut(&mut self, conn_id: ConnectionId) -> Option<&mut TcpConnection> {
        self.connections.get_mut(&conn_id)
            .or_else(|| self.listening_sockets.get_mut(&conn_id))
    }

    /// Get all connections
    pub fn get_all_connections(&self) -> Vec<&TcpConnection> {
        self.connections.values().collect()
    }

    /// Get all listening sockets
    pub fn get_listening_sockets(&self) -> Vec<&TcpConnection> {
        self.listening_sockets.values().collect()
    }

    /// Process packet for all matching connections
    pub fn process_packet(
        &mut self,
        local_ip: Ipv4Addr,
        local_port: u16,
        remote_ip: Ipv4Addr,
        remote_port: u16,
        packet: &[u8],
    ) -> Vec<(ConnectionId, Result<Vec<TcpAction>, TcpError>)> {
        let mut results = Vec::new();

        // Try to find exact match first
        let conn_id = ConnectionId::new(local_ip, local_port, remote_ip, remote_port);
        if let Some(connection) = self.connections.get_mut(&conn_id) {
            let actions = connection.process_packet(packet);
            results.push((conn_id, actions));
            return results;
        }

        // Try to find listening socket
        let listen_id = ConnectionId::wildcard(local_ip, local_port);
        if let Some(listening_socket) = self.listening_sockets.get_mut(&listen_id) {
            let actions = listening_socket.process_packet(packet);
            results.push((listen_id, actions));
        }

        results
    }

    /// Check timeouts for all connections
    pub fn check_timeouts(&mut self) -> Vec<(ConnectionId, Vec<TcpAction>)> {
        let mut results = Vec::new();

        for (conn_id, connection) in &mut self.connections {
            let actions = connection.check_timeouts();
            if !actions.is_empty() {
                results.push((*conn_id, actions));
            }
        }

        for (conn_id, connection) in &mut self.listening_sockets {
            let actions = connection.check_timeouts();
            if !actions.is_empty() {
                results.push((*conn_id, actions));
            }
        }

        results
    }

    /// Cleanup closed connections
    pub fn cleanup(&mut self) {
        self.connections.retain(|_, conn| !conn.is_closed());

        // Also clean up very old listening sockets
        let _now = get_current_time();
        self.listening_sockets.retain(|_, _conn| {
            // Keep listening sockets alive indefinitely (simplified)
            true
        });
    }

    /// Get manager statistics
    pub fn stats(&self) -> TcpManagerStats {
        TcpManagerStats {
            active_connections: self.connections.len(),
            listening_sockets: self.listening_sockets.len(),
            allocated_ports: self.allocated_ports.len(),
        }
    }
}

impl Default for TcpConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// TCP manager statistics
#[derive(Debug, Clone)]
pub struct TcpManagerStats {
    /// Number of active connections
    pub active_connections: usize,
    /// Number of listening sockets
    pub listening_sockets: usize,
    /// Number of allocated ports
    pub allocated_ports: usize,
}

/// TCP errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TcpError {
    /// Invalid packet format
    InvalidPacket,
    /// Connection not found
    ConnectionNotFound,
    /// Invalid connection state
    InvalidConnection,
    /// Not connected
    NotConnected,
    /// Buffer full
    BufferFull,
    /// Port in use
    PortInUse,
    /// No ports available
    NoPortsAvailable,
    /// Connection reset
    ConnectionReset,
    /// Connection timeout
    ConnectionTimeout,
}

/// Get current time (simplified implementation)
fn get_current_time() -> u64 {
    static TIMER: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0);
    TIMER.fetch_add(1, core::sync::atomic::Ordering::Relaxed)
}