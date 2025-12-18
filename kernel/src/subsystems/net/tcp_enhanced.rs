//! Enhanced TCP Protocol Implementation
//!
//! This module provides a comprehensive TCP protocol implementation with advanced features
//! like selective acknowledgments, window scaling, and timestamps.

extern crate alloc;
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use core::sync::atomic::{AtomicU64, AtomicU32, AtomicBool, Ordering};
use crate::sync::{Mutex, Sleeplock};
use nos_nos_error_handling::unified::KernelError;

// Re-export existing TCP functionality
pub use super::tcp::*;

// ============================================================================
// Enhanced TCP Constants
// ============================================================================

/// Maximum number of outstanding segments
pub const MAX_OUTSTANDING_SEGMENTS: usize = 64;

/// Maximum receive window size
pub const MAX_RECEIVE_WINDOW: u32 = 65535;

/// Maximum send window size
pub const MAX_SEND_WINDOW: u32 = 65535;

/// Maximum segment lifetime (in seconds)
pub const MAX_SEGMENT_LIFETIME: u64 = 120;

/// Minimum retransmission timeout (in milliseconds)
pub const MIN_RTO_MS: u64 = 200;

/// Maximum retransmission timeout (in milliseconds)
pub const MAX_RTO_MS: u64 = 60000;

/// Initial retransmission timeout (in milliseconds)
pub const INITIAL_RTO_MS: u64 = 1000;

// ============================================================================
// Enhanced TCP Options
// ============================================================================

/// TCP option kinds
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TcpOptionKind {
    /// End of options list
    End = 0,
    /// No operation
    Nop = 1,
    /// Maximum segment size
    Mss = 2,
    /// Window scaling
    WindowScale = 3,
    /// Selective acknowledgments permitted
    SackPermitted = 4,
    /// Selective acknowledgments
    Sack = 5,
    /// Timestamps
    Timestamps = 8,
}

/// TCP option
#[derive(Debug, Clone)]
pub struct TcpOption {
    /// Option kind
    pub kind: TcpOptionKind,
    /// Option length
    pub length: u8,
    /// Option data
    pub data: Vec<u8>,
}

impl TcpOption {
    /// Create a new TCP option
    pub fn new(kind: TcpOptionKind, data: Vec<u8>) -> Self {
        let length = 2 + data.len() as u8;
        Self { kind, length, data }
    }

    /// Serialize option to bytes
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.length as usize);
        bytes.push(self.kind as u8);
        bytes.push(self.length);
        bytes.extend_from_slice(&self.data);
        bytes
    }

    /// Parse option from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, TcpError> {
        if bytes.len() < 2 {
            return Err(TcpError::InvalidOption);
        }

        let kind = bytes[0];
        let length = bytes[1];

        if length < 2 || bytes.len() < length as usize {
            return Err(TcpError::InvalidOption);
        }

        let data = bytes[2..length as usize].to_vec();

        Ok(Self {
            kind: match kind {
                0 => TcpOptionKind::End,
                1 => TcpOptionKind::Nop,
                2 => TcpOptionKind::Mss,
                3 => TcpOptionKind::WindowScale,
                4 => TcpOptionKind::SackPermitted,
                5 => TcpOptionKind::Sack,
                8 => TcpOptionKind::Timestamps,
                _ => return Err(TcpError::InvalidOption),
            },
            length,
            data,
        })
    }
}

/// MSS option
#[derive(Debug, Clone)]
pub struct MssOption {
    /// Maximum segment size
    pub mss: u16,
}

impl MssOption {
    /// Create a new MSS option
    pub fn new(mss: u16) -> Self {
        Self { mss }
    }

    /// Create option from MSS value
    pub fn to_option(&self) -> TcpOption {
        TcpOption::new(TcpOptionKind::Mss, self.mss.to_be_bytes().to_vec())
    }

    /// Parse MSS from option
    pub fn from_option(option: &TcpOption) -> Result<Self, TcpError> {
        if option.kind != TcpOptionKind::Mss || option.data.len() != 2 {
            return Err(TcpError::InvalidOption);
        }

        let mss = u16::from_be_bytes([option.data[0], option.data[1]]);
        Ok(Self { mss })
    }
}

/// Window scale option
#[derive(Debug, Clone)]
pub struct WindowScaleOption {
    /// Shift count
    pub shift: u8,
}

impl WindowScaleOption {
    /// Create a new window scale option
    pub fn new(shift: u8) -> Self {
        Self { shift }
    }

    /// Create option from shift count
    pub fn to_option(&self) -> TcpOption {
        TcpOption::new(TcpOptionKind::WindowScale, vec![self.shift])
    }

    /// Parse shift count from option
    pub fn from_option(option: &TcpOption) -> Result<Self, TcpError> {
        if option.kind != TcpOptionKind::WindowScale || option.data.is_empty() {
            return Err(TcpError::InvalidOption);
        }

        Ok(Self { shift: option.data[0] })
    }
}

/// Timestamp option
#[derive(Debug, Clone)]
pub struct TimestampOption {
    /// Timestamp value
    pub timestamp: u32,
    /// Echo timestamp
    pub echo: u32,
}

impl TimestampOption {
    /// Create a new timestamp option
    pub fn new(timestamp: u32, echo: u32) -> Self {
        Self { timestamp, echo }
    }

    /// Create option from timestamp values
    pub fn to_option(&self) -> TcpOption {
        let mut data = Vec::with_capacity(8);
        data.extend_from_slice(&self.timestamp.to_be_bytes());
        data.extend_from_slice(&self.echo.to_be_bytes());
        TcpOption::new(TcpOptionKind::Timestamps, data)
    }

    /// Parse timestamp from option
    pub fn from_option(option: &TcpOption) -> Result<Self, TcpError> {
        if option.kind != TcpOptionKind::Timestamps || option.data.len() != 8 {
            return Err(TcpError::InvalidOption);
        }

        let timestamp = u32::from_be_bytes([option.data[0], option.data[1], option.data[2], option.data[3]]);
        let echo = u32::from_be_bytes([option.data[4], option.data[5], option.data[6], option.data[7]]);

        Ok(Self { timestamp, echo })
    }
}

/// SACK option
#[derive(Debug, Clone)]
pub struct SackOption {
    /// SACK blocks
    pub blocks: Vec<SackBlock>,
}

/// SACK block
#[derive(Debug, Clone)]
pub struct SackBlock {
    /// Left edge of block
    pub left: u32,
    /// Right edge of block
    pub right: u32,
}

impl SackBlock {
    /// Create a new SACK block
    pub fn new(left: u32, right: u32) -> Self {
        Self { left, right }
    }

    /// Create block from values
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8);
        bytes.extend_from_slice(&self.left.to_be_bytes());
        bytes.extend_from_slice(&self.right.to_be_bytes());
        bytes
    }

    /// Parse block from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, TcpError> {
        if bytes.len() < 8 {
            return Err(TcpError::InvalidOption);
        }

        let left = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
        let right = u32::from_be_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);

        Ok(Self { left, right })
    }
}

impl SackOption {
    /// Create a new SACK option
    pub fn new(blocks: Vec<SackBlock>) -> Self {
        Self { blocks }
    }

    /// Create option from SACK blocks
    pub fn to_option(&self) -> TcpOption {
        let mut data = Vec::with_capacity(2 + self.blocks.len() * 8);
        data.push(self.blocks.len() as u8); // Number of blocks
        data.push(0); // Reserved

        for block in &self.blocks {
            data.extend_from_slice(&block.to_bytes());
        }

        TcpOption::new(TcpOptionKind::Sack, data)
    }

    /// Parse SACK from option
    pub fn from_option(option: &TcpOption) -> Result<Self, TcpError> {
        if option.kind != TcpOptionKind::Sack || option.data.len() < 2 {
            return Err(TcpError::InvalidOption);
        }

        let block_count = option.data[0] as usize;
        if option.data.len() < 2 + block_count * 8 {
            return Err(TcpError::InvalidOption);
        }

        let mut blocks = Vec::with_capacity(block_count);
        for i in 0..block_count {
            let offset = 2 + i * 8;
            let block = SackBlock::from_bytes(&option.data[offset..offset + 8])?;
            blocks.push(block);
        }

        Ok(Self { blocks })
    }
}

// ============================================================================
// Enhanced TCP Statistics
// ============================================================================

/// Enhanced TCP statistics
#[derive(Debug, Default, Clone)]
pub struct EnhancedTcpStats {
    /// Total connections attempted
    pub connections_attempted: u64,
    /// Total connections established
    pub connections_established: u64,
    /// Total connections failed
    pub connections_failed: u64,
    /// Total connections reset
    pub connections_reset: u64,
    /// Total connections timed out
    pub connections_timed_out: u64,
    /// Total bytes transmitted
    pub bytes_transmitted: u64,
    /// Total bytes received
    pub bytes_received: u64,
    /// Total packets transmitted
    pub packets_transmitted: u64,
    /// Total packets received
    pub packets_received: u64,
    /// Total retransmissions
    pub retransmissions: u64,
    /// Total duplicate ACKs
    pub duplicate_acks: u64,
    /// Total out-of-order packets
    pub out_of_order_packets: u64,
    /// Total fast retransmits
    pub fast_retransmits: u64,
    /// Total partial ACKs
    pub partial_acks: u64,
    /// Total window updates
    pub window_updates: u64,
    /// Average RTT in microseconds
    pub avg_rtt_us: u64,
    /// Minimum RTT in microseconds
    pub min_rtt_us: u64,
    /// Maximum RTT in microseconds
    pub max_rtt_us: u64,
    /// Current congestion window
    pub current_cwnd: u32,
    /// Current send window
    pub current_swnd: u32,
    /// Current receive window
    pub current_rwnd: u32,
    /// Last activity timestamp
    pub last_activity_timestamp: u64,
}

// ============================================================================
// Enhanced TCP Implementation
// ============================================================================

/// Enhanced TCP implementation
pub struct EnhancedTcp {
    /// TCP connection manager
    connection_manager: Mutex<super::tcp::manager::TcpConnectionManager>,
    /// TCP statistics
    stats: Mutex<EnhancedTcpStats>,
    /// Configuration
    config: TcpConfig,
    /// Initialized flag
    initialized: AtomicBool,
}

/// TCP configuration
#[derive(Debug, Clone)]
pub struct TcpConfig {
    /// Default MSS
    pub default_mss: u16,
    /// Default window size
    pub default_window: u16,
    /// Enable window scaling
    pub enable_window_scaling: bool,
    /// Enable timestamps
    pub enable_timestamps: bool,
    /// Enable SACK
    pub enable_sack: bool,
    /// Enable selective ACK
    pub enable_selective_ack: bool,
    /// Enable fast retransmit
    pub enable_fast_retransmit: bool,
    /// Enable fast recovery
    pub enable_fast_recovery: bool,
    /// Initial congestion window
    pub initial_cwnd: u32,
    /// Maximum retransmission attempts
    pub max_retransmit_attempts: u32,
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            default_mss: 1460,
            default_window: 65535,
            enable_window_scaling: true,
            enable_timestamps: true,
            enable_sack: true,
            enable_selective_ack: true,
            enable_fast_retransmit: true,
            enable_fast_recovery: true,
            initial_cwnd: 10 * 1460,
            max_retransmit_attempts: 5,
        }
    }
}

impl EnhancedTcp {
    /// Create a new enhanced TCP implementation
    pub fn new(config: TcpConfig) -> Self {
        Self {
            connection_manager: Mutex::new(super::tcp::manager::TcpConnectionManager::new()),
            stats: Mutex::new(EnhancedTcpStats::default()),
            config,
            initialized: AtomicBool::new(false),
        }
    }

    /// Initialize the enhanced TCP implementation
    pub fn initialize(&self) -> Result<(), KernelError> {
        if self.initialized.load(Ordering::SeqCst) {
            return Ok(());
        }

        crate::println!("tcp: initializing enhanced TCP implementation");
        crate::println!("tcp: MSS={}, window scaling={}, timestamps={}, SACK={}",
                      self.config.default_mss,
                      self.config.enable_window_scaling,
                      self.config.enable_timestamps,
                      self.config.enable_sack);

        self.initialized.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Create a listening socket
    pub fn listen(&self, local_ip: super::ipv4::Ipv4Addr, local_port: u16) -> Result<super::tcp::manager::ConnectionId, TcpError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TcpError::NotInitialized);
        }

        let options = super::tcp::manager::TcpOptions {
            keep_alive: true,
            keep_alive_interval: 30,
            keep_alive_time: 7200,
            keep_alive_probes: 9,
            nagle_enabled: true,
            reuse_addr: true,
            reuse_port: false,
            recv_buf_size: 65536,
            send_buf_size: 65536,
        };

        let mut manager = self.connection_manager.lock();
        let conn_id = manager.listen(local_ip, local_port, options)?;

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.connections_attempted += 1;
            stats.last_activity_timestamp = self.get_current_time();
        }

        crate::println!("tcp: listening on {}:{}", local_ip, local_port);
        Ok(conn_id)
    }

    /// Connect to a remote host
    pub fn connect(
        &self,
        local_ip: super::ipv4::Ipv4Addr,
        remote_ip: super::ipv4::Ipv4Addr,
        remote_port: u16,
    ) -> Result<super::tcp::manager::ConnectionId, TcpError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TcpError::NotInitialized);
        }

        let options = super::tcp::manager::TcpOptions {
            keep_alive: true,
            keep_alive_interval: 30,
            keep_alive_time: 7200,
            keep_alive_probes: 9,
            nagle_enabled: true,
            reuse_addr: true,
            reuse_port: false,
            recv_buf_size: 65536,
            send_buf_size: 65536,
        };

        let mut manager = self.connection_manager.lock();
        let conn_id = manager.connect(local_ip, remote_ip, remote_port, options)?;

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.connections_attempted += 1;
            stats.last_activity_timestamp = self.get_current_time();
        }

        crate::println!("tcp: connecting to {}:{} from {}", remote_ip, remote_port, local_ip);
        Ok(conn_id)
    }

    /// Accept a new connection
    pub fn accept(&self, listening_id: super::tcp::manager::ConnectionId) -> Result<Option<super::tcp::manager::ConnectionId>, TcpError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TcpError::NotInitialized);
        }

        let mut manager = self.connection_manager.lock();
        let conn_id = manager.accept(listening_id)?;

        if let Some(ref new_conn_id) = conn_id {
            // Update statistics
            {
                let mut stats = self.stats.lock();
                stats.connections_established += 1;
                stats.last_activity_timestamp = self.get_current_time();
            }

            crate::println!("tcp: accepted new connection");
        }

        Ok(conn_id)
    }

    /// Send data on a connection
    pub fn send(
        &self,
        conn_id: super::tcp::manager::ConnectionId,
        data: &[u8],
    ) -> Result<(), TcpError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TcpError::NotInitialized);
        }

        let mut manager = self.connection_manager.lock();
        let connection = manager.get_connection_mut(conn_id)
            .ok_or(TcpError::ConnectionNotFound)?;

        connection.send_data(data)?;

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.bytes_transmitted += data.len() as u64;
            stats.packets_transmitted += 1;
            stats.last_activity_timestamp = self.get_current_time();
        }

        Ok(())
    }

    /// Receive data from a connection
    pub fn receive(
        &self,
        conn_id: super::tcp::manager::ConnectionId,
        buffer: &mut [u8],
    ) -> Result<usize, TcpError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TcpError::NotInitialized);
        }

        let mut manager = self.connection_manager.lock();
        let connection = manager.get_connection_mut(conn_id)
            .ok_or(TcpError::ConnectionNotFound)?;

        let bytes_read = connection.receive_data(buffer)?;

        // Update statistics
        if bytes_read > 0 {
            {
                let mut stats = self.stats.lock();
                stats.bytes_received += bytes_read as u64;
                stats.packets_received += 1;
                stats.last_activity_timestamp = self.get_current_time();
            }
        }

        Ok(bytes_read)
    }

    /// Close a connection
    pub fn close(&self, conn_id: super::tcp::manager::ConnectionId) -> Result<(), TcpError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TcpError::NotInitialized);
        }

        let mut manager = self.connection_manager.lock();
        manager.close(conn_id)?;

        // Update statistics
        {
            let mut stats = self.stats.lock();
            stats.last_activity_timestamp = self.get_current_time();
        }

        crate::println!("tcp: closed connection");
        Ok(())
    }

    /// Process an incoming packet
    pub fn process_packet(
        &self,
        local_ip: super::ipv4::Ipv4Addr,
        local_port: u16,
        remote_ip: super::ipv4::Ipv4Addr,
        remote_port: u16,
        packet: &[u8],
    ) -> Result<(), TcpError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TcpError::NotInitialized);
        }

        let mut manager = self.connection_manager.lock();
        let results = manager.process_packet(local_ip, local_port, remote_ip, remote_port, packet);

        for (conn_id, actions) in results {
            match actions {
                Ok(action_list) => {
                    for action in action_list {
                        self.handle_action(conn_id, &action)?;
                    }
                }
                Err(e) => {
                    crate::println!("tcp: error processing packet for connection {:?}: {:?}", conn_id, e);
                }
            }
        }

        Ok(())
    }

    /// Check timeouts for all connections
    pub fn check_timeouts(&self) -> Result<(), TcpError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TcpError::NotInitialized);
        }

        let mut manager = self.connection_manager.lock();
        let timeout_results = manager.check_timeouts();

        for (conn_id, actions) in timeout_results {
            for action in actions {
                self.handle_action(conn_id, &action)?;
            }
        }

        // Cleanup closed connections
        manager.cleanup();

        Ok(())
    }

    /// Get connection statistics
    pub fn get_connection_stats(&self, conn_id: super::tcp::manager::ConnectionId) -> Result<super::tcp::manager::TcpConnectionStats, TcpError> {
        if !self.initialized.load(Ordering::SeqCst) {
            return Err(TcpError::NotInitialized);
        }

        let manager = self.connection_manager.lock();
        let connection = manager.get_connection(conn_id)
            .ok_or(TcpError::ConnectionNotFound)?;

        Ok(connection.get_stats().clone())
    }

    /// Get enhanced TCP statistics
    pub fn get_stats(&self) -> EnhancedTcpStats {
        self.stats.lock().clone()
    }

    /// Reset enhanced TCP statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = EnhancedTcpStats::default();
    }

    /// Handle a TCP action
    fn handle_action(&self, conn_id: super::tcp::manager::ConnectionId, action: &super::tcp::state::TcpAction) -> Result<(), TcpError> {
        match action {
            super::tcp::state::TcpAction::ConnectionEstablished => {
                // Update statistics
                let mut stats = self.stats.lock();
                stats.connections_established += 1;
            }
            super::tcp::state::TcpAction::ConnectionClosed => {
                // Update statistics
                let mut stats = self.stats.lock();
                stats.last_activity_timestamp = self.get_current_time();
            }
            super::tcp::state::TcpAction::ConnectionReset => {
                // Update statistics
                let mut stats = self.stats.lock();
                stats.connections_reset += 1;
                stats.last_activity_timestamp = self.get_current_time();
            }
            super::tcp::state::TcpAction::Retransmit(_) => {
                // Update statistics
                let mut stats = self.stats.lock();
                stats.retransmissions += 1;
            }
            super::tcp::state::TcpAction::DataReceived(_) => {
                // Update statistics
                let mut stats = self.stats.lock();
                stats.packets_received += 1;
            }
            _ => {}
        }

        Ok(())
    }

    /// Get current time in milliseconds
    fn get_current_time(&self) -> u64 {
        // In a real implementation, this would get the current time
        // from the system clock
        0
    }
}

/// Enhanced TCP errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TcpError {
    /// Invalid option
    InvalidOption,
    /// Not initialized
    NotInitialized,
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
    /// Invalid packet
    InvalidPacket,
    /// Packet too small
    PacketTooSmall,
    /// Invalid header length
    InvalidHeaderLength,
    /// Invalid checksum
    InvalidChecksum,
}

impl Default for EnhancedTcp {
    fn default() -> Self {
        Self::new(TcpConfig::default())
    }
}

/// Global enhanced TCP instance
static mut ENHANCED_TCP: Option<EnhancedTcp> = None;

/// Initialize enhanced TCP
pub fn init() -> Result<(), KernelError> {
    unsafe {
        let tcp = EnhancedTcp::new(TcpConfig::default());
        tcp.initialize()?;
        ENHANCED_TCP = Some(tcp);
    }
    Ok(())
}

/// Get enhanced TCP instance
pub fn get_enhanced_tcp() -> Option<&'static EnhancedTcp> {
    unsafe { ENHANCED_TCP.as_ref() }
}