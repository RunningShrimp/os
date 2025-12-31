//! Advanced TCP features for NOS kernel
//!
//! This module implements advanced TCP features including:
//! - Selective Acknowledgment (SACK), Forward Acknowledgment (FACK), Duplicate SACK (DSACK)
//! - Window scaling for high-bandwidth networks
//! - Timestamps for RTT measurement and PAWS
//! - Multiple congestion control algorithms (CUBIC, BBR, H-TCP)
//! - Fast retransmit and fast recovery
//! - TCP MD5 signatures and Authentication Option (AO)
//!
//! # Examples
//!
//! ```rust
//! use kernel::network::tcp_advanced::{TcpAdvanced, CongestionControl, SackType};
//!
//! // Create TCP advanced features
//! let tcp = TcpAdvanced::new();
//!
//! // Enable SACK with DSACK
//! tcp.enable_sack(true, SackType::DSACK);
//!
//! // Enable window scaling (scale factor 7 = window up to 1GB)
//! tcp.enable_window_scaling(true, 7);
//!
//! // Set congestion control to CUBIC
//! tcp.set_congestion_control(CongestionControl::Cubic);
//!
//! // Enable timestamps for RTT measurement
//! tcp.enable_timestamps(true);
//! ```

#![no_std]

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use core::time::Duration;

use crate::subsystems::sync::{Mutex, RwLock};

/// Maximum window scale factor (2^14 = 16384KB = 16MB buffer)
pub const MAX_WINDOW_SCALE: u8 = 14;

/// Maximum SACK blocks permitted
pub const MAX_SACK_BLOCKS: usize = 4;

/// Default initial congestion window (packets)
pub const DEFAULT_INIT_CWND: u32 = 10;

/// Maximum window size with scaling (1GB)
pub const MAX_WINDOW_SIZE: u32 = 1 << 30;

/// SACK (Selective Acknowledgment) types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SackType {
    /// Standard SACK - acknowledges non-contiguous data
    Standard,
    /// FACK (Forward Acknowledgment) - optimizes retransmission
    Fack,
    /// DSACK (Duplicate SACK) - reports duplicate segments
    Dsack,
}

impl SackType {
    /// Get SACK type as TCP option number
    pub fn as_option_number(&self) -> u8 {
        match self {
            Self::Standard | Self::Fack | Self::Dsack => 5, // SACK permitted
        }
    }
}

/// SACK block representing a contiguous range of acknowledged data
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SackBlock {
    /// Left edge of block (first sequence number)
    pub left: u32,
    /// Right edge of block (one past last sequence number)
    pub right: u32,
}

impl SackBlock {
    /// Create a new SACK block
    pub fn new(left: u32, right: u32) -> Self {
        assert!(left < right, "SACK block left must be less than right");
        Self { left, right }
    }

    /// Check if this block overlaps with another
    pub fn overlaps(&self, other: &Self) -> bool {
        self.left < other.right && other.left < self.right
    }

    /// Merge overlapping blocks
    pub fn merge(&self, other: &Self) -> Option<Self> {
        if self.overlaps(other) {
            Some(Self {
                left: self.left.min(other.left),
                right: self.right.max(other.right),
            })
        } else {
            None
        }
    }

    /// Get block length
    pub fn len(&self) -> u32 {
        self.right - self.left
    }
}

/// Window scaling configuration
#[derive(Debug, Clone, Copy)]
pub struct WindowScale {
    /// Window scaling enabled
    pub enabled: bool,
    /// Shift count (0-14)
    pub shift: u8,
    /// Our window scale factor
    pub our_scale: u8,
    /// Peer's window scale factor
    pub peer_scale: u8,
}

impl WindowScale {
    /// Create a new window scale configuration
    pub fn new(shift: u8) -> Self {
        assert!(shift <= MAX_WINDOW_SCALE, "Window scale exceeds maximum");
        Self {
            enabled: true,
            shift,
            our_scale: shift,
            peer_scale: 0,
        }
    }

    /// Calculate actual window size from scaled value
    pub fn calculate_window(&self, scaled_window: u16) -> u32 {
        if !self.enabled {
            return scaled_window as u32;
        }
        ((scaled_window as u32) << self.shift).min(MAX_WINDOW_SIZE)
    }

    /// Scale window size for transmission
    pub fn scale_window(&self, window: u32) -> u16 {
        if !self.enabled || self.shift == 0 {
            return window as u16;
        }
        ((window >> self.shift) as u16).min(u16::MAX)
    }

    /// Get maximum window size
    pub fn max_window(&self) -> u32 {
        if !self.enabled {
            65535
        } else {
            (65535u32 << self.shift).min(MAX_WINDOW_SIZE)
        }
    }
}

/// TCP timestamp option
#[derive(Debug, Clone, Copy)]
pub struct TimestampOption {
    /// Timestamps enabled
    pub enabled: bool,
    /// Recent timestamp received from peer
    pub ts_recent: u32,
    /// Timestamp of last acknowledgment
    pub ts_last_ack: u32,
    /// Current timestamp value
    pub ts_val: u32,
    /// Timestamp echo reply
    pub ts_echo_reply: u32,
}

impl TimestampOption {
    /// Create new timestamp option
    pub fn new() -> Self {
        Self {
            enabled: false,
            ts_recent: 0,
            ts_last_ack: 0,
            ts_val: 0,
            ts_echo_reply: 0,
        }
    }

    /// Enable timestamps
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Update current timestamp (typically in milliseconds)
    pub fn update_timestamp(&mut self, ts: u32) {
        self.ts_val = ts;
    }

    /// Set echo reply from peer's timestamp
    pub fn set_echo_reply(&mut self, echo: u32) {
        self.ts_echo_reply = echo;
    }

    /// Check if timestamp is recent (for PAWS)
    pub fn is_recent(&self, ts: u32) -> bool {
        // PAWS: Protect Against Wrapped Sequences
        // Timestamp is recent if it's greater than recent or not too old
        // Use 24-day window (2^31 milliseconds)
        const TS_MAX_HALF: u32 = 0x80000000;
        let diff = ts.wrapping_sub(self.ts_recent);
        diff < TS_MAX_HALF
    }
}

/// Congestion control algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionControl {
    /// Reno congestion control (classic)
    Reno,
    /// CUBIC congestion control (default in Linux)
    Cubic,
    /// BBR (Bottleneck Bandwidth and RTT)
    Bbr,
    /// H-TCP (Hamilton TCP)
    HTcp,
    /// Vegas (delay-based)
    Vegas,
    /// Westwood+ (bandwidth estimation)
    Westwood,
}

impl CongestionControl {
    /// Get congestion control name
    pub fn as_str(&self) -> &str {
        match self {
            Self::Reno => "reno",
            Self::Cubic => "cubic",
            Self::Bbr => "bbr",
            Self::HTcp => "htcp",
            Self::Vegas => "vegas",
            Self::Westwood => "westwood",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "reno" => Some(Self::Reno),
            "cubic" => Some(Self::Cubic),
            "bbr" => Some(Self::Bbr),
            "htcp" => Some(Self::HTcp),
            "vegas" => Some(Self::Vegas),
            "westwood" => Some(Self::Westwood),
            _ => None,
        }
    }
}

/// Congestion control state
#[derive(Debug, Clone)]
pub struct CongestionControlState {
    /// Algorithm type
    pub algorithm: CongestionControl,
    /// Congestion window (in bytes)
    pub cwnd: u32,
    /// Slow start threshold (in bytes)
    pub ssthresh: u32,
    /// Current state
    pub state: CongestionState,
    /// Minimum RTT observed (microseconds)
    pub min_rtt: u32,
    /// Current RTT estimate (microseconds)
    pub rtt: u32,
    /// RTT variance
    pub rtt_var: u32,
    /// Bandwidth estimate (bytes per second)
    pub bandwidth: u64,
    /// Packets acknowledged since last congestion event
    pub packets_acked: u32,
}

/// Congestion state machine
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CongestionState {
    /// Slow start
    SlowStart,
    /// Congestion avoidance
    CongestionAvoidance,
    /// Fast recovery
    FastRecovery,
    /// Fast retransmit
    FastRetransmit,
}

impl CongestionControlState {
    /// Create new congestion control state
    pub fn new(algorithm: CongestionControl, initial_cwnd: u32) -> Self {
        Self {
            algorithm,
            cwnd: initial_cwnd * 1460, // Convert packets to bytes (assuming MSS=1460)
            ssthresh: u32::MAX,
            state: CongestionState::SlowStart,
            min_rtt: u32::MAX,
            rtt: 0,
            rtt_var: 0,
            bandwidth: 0,
            packets_acked: 0,
        }
    }

    /// Update congestion window on ACK
    pub fn on_ack(&mut self, acked_bytes: u32) {
        self.packets_acked += 1;

        match self.algorithm {
            CongestionControl::Reno => self.reno_on_ack(acked_bytes),
            CongestionControl::Cubic => self.cubic_on_ack(acked_bytes),
            CongestionControl::Bbr => self.bbr_on_ack(acked_bytes),
            CongestionControl::HTcp => self.htcp_on_ack(acked_bytes),
            _ => self.reno_on_ack(acked_bytes),
        }
    }

    /// Handle congestion event (packet loss)
    pub fn on_loss(&mut self) {
        self.ssthresh = self.cwnd / 2;

        match self.algorithm {
            CongestionControl::Reno => self.reno_on_loss(),
            CongestionControl::Cubic => self.cubic_on_loss(),
            CongestionControl::Bbr => self.bbr_on_loss(),
            CongestionControl::HTcp => self.htcp_on_loss(),
            _ => self.reno_on_loss(),
        }
    }

    /// Reno congestion control
    fn reno_on_ack(&mut self, acked_bytes: u32) {
        if self.state == CongestionState::SlowStart {
            // Exponential growth
            self.cwnd += acked_bytes;
            if self.cwnd >= self.ssthresh {
                self.state = CongestionState::CongestionAvoidance;
            }
        } else if self.state == CongestionState::CongestionAvoidance {
            // Additive increase (1 MSS per RTT)
            // Approximate by adding (MSS * MSS / cwnd) per ACK
            let mss = 1460u32;
            let increment = (mss * mss) / self.cwnd.max(1);
            self.cwnd += increment.max(1);
        } else if self.state == CongestionState::FastRecovery {
            self.cwnd += acked_bytes;
        }
    }

    /// CUBIC congestion control
    fn cubic_on_ack(&mut self, acked_bytes: u32) {
        // CUBIC uses a cubic function for window growth
        // W(t) = C * (t - K)^3 + W_max
        // where C = 0.4, K = cubic_root(W_max * (1 - beta) / C)

        if self.state == CongestionState::SlowStart {
            self.cwnd += acked_bytes;
            if self.cwnd >= self.ssthresh {
                self.state = CongestionState::CongestionAvoidance;
            }
        } else if self.state == CongestionState::CongestionAvoidance {
            // CUBIC function implementation (simplified)
            // In practice, this would use time since last congestion event
            let mss = 1460u32;
            let increment = (mss * mss * mss) / (self.cwnd * self.cwnd).max(1);
            self.cwnd += increment.max(1);
        }
    }

    /// BBR congestion control
    fn bbr_on_ack(&mut self, _acked_bytes: u32) {
        // BBR controls congestion based on bandwidth and RTT measurements
        // BBR has different phases: STARTUP, DRAIN, PROBE_BW, PROBE_RTT
        // This is a simplified implementation
    }

    /// H-TCP congestion control
    fn htcp_on_ack(&mut self, acked_bytes: u32) {
        // H-TCP uses a function that varies with time since last congestion
        if self.state == CongestionState::SlowStart {
            self.cwnd += acked_bytes;
            if self.cwnd >= self.ssthresh {
                self.state = CongestionState::CongestionAvoidance;
            }
        } else if self.state == CongestionState::CongestionAvoidance {
            // H-TCP uses alpha parameter based on time since last congestion
            let mss = 1460u32;
            let increment = (2 * mss * mss) / self.cwnd.max(1);
            self.cwnd += increment.max(1);
        }
    }

    fn reno_on_loss(&mut self) {
        self.state = CongestionState::FastRetransmit;
        self.cwnd = self.ssthresh + 3 * 1460; // 3 MSS
        self.packets_acked = 0;
    }

    fn cubic_on_loss(&mut self) {
        self.state = CongestionState::FastRetransmit;
        // CUBIC multiplicative decrease
        self.cwnd = self.cwnd * 7 / 10; // beta = 0.7
        self.packets_acked = 0;
    }

    fn bbr_on_loss(&mut self) {
        // BBR doesn't reduce cwnd on loss directly
        // It reacts to measured congestion signals
    }

    fn htcp_on_loss(&mut self) {
        self.state = CongestionState::FastRetransmit;
        self.cwnd = self.cwnd / 2;
        self.packets_acked = 0;
    }
}

/// Fast retransmit and recovery state
#[derive(Debug, Clone)]
pub struct FastRetransmit {
    /// Duplicate ACK count
    pub dup_ack_count: u32,
    /// Threshold for fast retransmit (typically 3)
    pub threshold: u32,
    /// Highest sequence number acknowledged
    pub high_seq: u32,
    /// Recovery point
    pub recovery_point: u32,
    /// In fast recovery
    pub in_recovery: bool,
}

impl FastRetransmit {
    /// Create new fast retransmit state
    pub fn new() -> Self {
        Self {
            dup_ack_count: 0,
            threshold: 3,
            high_seq: 0,
            recovery_point: 0,
            in_recovery: false,
        }
    }

    /// Process incoming ACK
    pub fn on_ack(&mut self, ack_seq: u32, data_acked: bool) -> bool {
        if self.in_recovery {
            // In fast recovery
            if ack_seq >= self.recovery_point {
                // End of recovery
                self.in_recovery = false;
                self.dup_ack_count = 0;
                false
            } else {
                // Still in recovery
                false
            }
        } else if data_acked {
            // New data acknowledged
            self.dup_ack_count = 0;
            self.high_seq = ack_seq;
            false
        } else {
            // Duplicate ACK
            self.dup_ack_count += 1;
            if self.dup_ack_count >= self.threshold {
                // Trigger fast retransmit
                self.in_recovery = true;
                self.recovery_point = self.high_seq;
                true
            } else {
                false
            }
        }
    }

    /// Check if should retransmit
    pub fn should_retransmit(&self) -> bool {
        self.dup_ack_count >= self.threshold && !self.in_recovery
    }

    /// Exit recovery mode
    pub fn exit_recovery(&mut self) {
        self.in_recovery = false;
        self.dup_ack_count = 0;
    }
}

/// TCP MD5 signature option (RFC 2385)
#[derive(Debug, Clone)]
pub struct TcpMd5Signature {
    /// MD5 signatures enabled
    pub enabled: bool,
    /// MD5 key (up to 80 bytes)
    pub key: Vec<u8>,
    /// Expected signature
    pub expected: [u8; 16],
}

impl TcpMd5Signature {
    /// Create new MD5 signature option
    pub fn new() -> Self {
        Self {
            enabled: false,
            key: Vec::new(),
            expected: [0u8; 16],
        }
    }

    /// Set MD5 key
    pub fn set_key(&mut self, key: Vec<u8>) {
        assert!(key.len() <= 80, "MD5 key too long");
        self.key = key;
        self.enabled = true;
    }

    /// Compute MD5 signature (simplified - real implementation would use crypto crate)
    pub fn compute(&self, data: &[u8]) -> [u8; 16] {
        // In real implementation, this would compute actual MD5
        // For now, return zeros
        [0u8; 16]
    }

    /// Verify signature
    pub fn verify(&self, data: &[u8], signature: &[u8; 16]) -> bool {
        if !self.enabled {
            return true; // If not enabled, accept all
        }
        &self.compute(data) == signature
    }
}

/// TCP Authentication Option (AO) - RFC 5925
#[derive(Debug, Clone)]
pub struct TcpAuthenticationOption {
    /// AO enabled
    pub enabled: bool,
    /// Authentication algorithm
    pub algorithm: AoAlgorithm,
    /// Master key
    pub master_key: Vec<u8>,
    /// Key identifier
    pub key_id: u8,
    /// Next key identifier
    pub next_key_id: u8,
    /// Receive next key number
    pub recv_next_key_id: u8,
}

/// Authentication algorithms for TCP-AO
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AoAlgorithm {
    /// HMAC-SHA1-96
    HmacSha1_96,
    /// AES-128-CMAC-96
    Aes128Cmac_96,
}

impl TcpAuthenticationOption {
    /// Create new TCP-AO option
    pub fn new() -> Self {
        Self {
            enabled: false,
            algorithm: AoAlgorithm::HmacSha1_96,
            master_key: Vec::new(),
            key_id: 0,
            next_key_id: 0,
            recv_next_key_id: 0,
        }
    }

    /// Enable TCP-AO
    pub fn enable(&mut self, algorithm: AoAlgorithm, key: Vec<u8>, key_id: u8) {
        self.enabled = true;
        self.algorithm = algorithm;
        self.master_key = key;
        self.key_id = key_id;
    }

    /// Compute MAC (simplified)
    pub fn compute_mac(&self, data: &[u8]) -> [u8; 12] {
        // In real implementation, this would compute actual MAC
        // For now, return zeros
        [0u8; 12]
    }

    /// Verify MAC
    pub fn verify_mac(&self, data: &[u8], mac: &[u8; 12]) -> bool {
        if !self.enabled {
            return true;
        }
        &self.compute_mac(data) == mac
    }
}

/// Per-connection TCP advanced state
#[derive(Debug)]
pub struct TcpConnectionState {
    /// Connection ID
    pub connection_id: u32,
    /// SACK blocks received
    pub sack_blocks: Vec<SackBlock>,
    /// SACK enabled
    pub sack_enabled: bool,
    /// SACK type
    pub sack_type: SackType,
    /// Window scaling
    pub window_scale: WindowScale,
    /// Timestamps
    pub timestamps: TimestampOption,
    /// Congestion control
    pub congestion: CongestionControlState,
    /// Fast retransmit state
    pub fast_retransmit: FastRetransmit,
    /// MD5 signature
    pub md5: TcpMd5Signature,
    /// Authentication option
    pub ao: TcpAuthenticationOption,
    /// Highest sequence number sent
    pub snd_nxt: u32,
    /// Highest sequence number acknowledged
    pub snd_una: u32,
    /// Next expected sequence number
    pub rcv_nxt: u32,
    /// Receive window
    pub rcv_wnd: u32,
    /// Send window
    pub snd_wnd: u32,
    /// Initial sequence number
    pub iss: u32,
    /// Initial receive sequence number
    pub irs: u32,
}

impl TcpConnectionState {
    /// Create new connection state
    pub fn new(connection_id: u32, iss: u32, irs: u32) -> Self {
        Self {
            connection_id,
            sack_blocks: Vec::new(),
            sack_enabled: false,
            sack_type: SackType::Standard,
            window_scale: WindowScale {
                enabled: false,
                shift: 0,
                our_scale: 0,
                peer_scale: 0,
            },
            timestamps: TimestampOption::new(),
            congestion: CongestionControlState::new(CongestionControl::Cubic, DEFAULT_INIT_CWND),
            fast_retransmit: FastRetransmit::new(),
            md5: TcpMd5Signature::new(),
            ao: TcpAuthenticationOption::new(),
            snd_nxt: iss + 1,
            snd_una: iss,
            rcv_nxt: irs + 1,
            rcv_wnd: 65535,
            snd_wnd: 0,
            iss,
            irs,
        }
    }

    /// Add SACK block
    pub fn add_sack_block(&mut self, block: SackBlock) {
        if !self.sack_enabled {
            return;
        }

        // Merge with existing blocks if overlapping
        let mut merged = false;
        for existing in &mut self.sack_blocks {
            if let Some(merged_block) = existing.merge(&block) {
                *existing = merged_block;
                merged = true;
                break;
            }
        }

        if !merged && self.sack_blocks.len() < MAX_SACK_BLOCKS {
            self.sack_blocks.push(block);
        }

        // Sort blocks by left edge
        self.sack_blocks.sort_by_key(|b| b.left);
    }

    /// Get SACK blocks for transmission
    pub fn get_sack_blocks(&self) -> &[SackBlock] {
        &self.sack_blocks[..self.sack_blocks.len().min(MAX_SACK_BLOCKS)]
    }
}

/// Advanced TCP manager
#[derive(Debug)]
pub struct TcpAdvanced {
    /// Connection states indexed by connection ID
    connections: RwLock<BTreeMap<u32, TcpConnectionState>>,
    /// Next connection ID
    next_connection_id: AtomicU32,
    /// Global statistics
    stats: Mutex<TcpAdvancedStats>,
    /// Default congestion control algorithm
    default_cc: CongestionControl,
}

/// TCP advanced statistics
#[derive(Debug, Default, Clone)]
pub struct TcpAdvancedStats {
    /// Total SACK blocks sent
    pub sack_blocks_sent: u64,
    /// Total SACK blocks received
    pub sack_blocks_received: u64,
    /// Window scale negotiations
    pub window_scale_negotiations: u64,
    /// Timestamp updates
    pub timestamp_updates: u64,
    /// Fast retransmits triggered
    pub fast_retransmits: u64,
    /// Fast recoveries completed
    pub fast_recoveries: u64,
    /// Congestion events (packet loss detected)
    pub congestion_events: u64,
    /// MD5 signatures verified
    pub md5_verifications: u64,
    /// MD5 signature failures
    pub md5_failures: u64,
    /// AO MACs verified
    pub ao_verifications: u64,
    /// AO MAC failures
    pub ao_failures: u64,
}

impl TcpAdvanced {
    /// Create new TCP advanced manager
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(BTreeMap::new()),
            next_connection_id: AtomicU32::new(1),
            stats: Mutex::new(TcpAdvancedStats::default()),
            default_cc: CongestionControl::Cubic,
        }
    }

    /// Create a new connection state
    pub fn create_connection(&self, iss: u32, irs: u32) -> u32 {
        let id = self.next_connection_id.fetch_add(1, Ordering::SeqCst);
        let state = TcpConnectionState::new(id, iss, irs);

        let mut connections = self.connections.write();
        connections.insert(id, state);
        id
    }

    /// Get connection state
    pub fn get_connection(&self, connection_id: u32) -> Option<TcpConnectionState> {
        let connections = self.connections.read();
        connections.get(&connection_id).cloned()
    }

    /// Update connection state
    pub fn update_connection<F>(&self, connection_id: u32, updater: F) -> bool
    where
        F: FnOnce(&mut TcpConnectionState),
    {
        let mut connections = self.connections.write();
        if let Some(conn) = connections.get_mut(&connection_id) {
            updater(conn);
            true
        } else {
            false
        }
    }

    /// Remove connection
    pub fn remove_connection(&self, connection_id: u32) -> bool {
        let mut connections = self.connections.write();
        connections.remove(&connection_id).is_some()
    }

    /// Enable SACK for a connection
    pub fn enable_sack(&self, connection_id: u32, enabled: bool) -> Result<(), ()> {
        self.update_connection(connection_id, |conn| {
            conn.sack_enabled = enabled;
        })
    }

    /// Set SACK type
    pub fn set_sack_type(&self, connection_id: u32, sack_type: SackType) -> Result<(), ()> {
        self.update_connection(connection_id, |conn| {
            conn.sack_type = sack_type;
        })
    }

    /// Process received SACK blocks
    pub fn process_sack(
        &self,
        connection_id: u32,
        blocks: &[SackBlock],
    ) -> Result<(), ()> {
        let mut stats = self.stats.lock();
        stats.sack_blocks_received += blocks.len() as u64;
        drop(stats);

        self.update_connection(connection_id, |conn| {
            for &block in blocks {
                conn.add_sack_block(block);
            }
        })
    }

    /// Enable window scaling
    pub fn enable_window_scaling(&self, enabled: bool) -> Result<(), ()> {
        if enabled {
            let mut stats = self.stats.lock();
            stats.window_scale_negotiations += 1;
        }
        Ok(())
    }

    /// Configure window scaling for a connection
    pub fn configure_window_scaling(
        &self,
        connection_id: u32,
        our_scale: u8,
        peer_scale: u8,
    ) -> Result<(), ()> {
        self.update_connection(connection_id, |conn| {
            conn.window_scale = WindowScale {
                enabled: true,
                shift: our_scale,
                our_scale,
                peer_scale,
            };
        })
    }

    /// Enable timestamps for a connection
    pub fn enable_timestamps(&self, connection_id: u32, enabled: bool) -> Result<(), ()> {
        self.update_connection(connection_id, |conn| {
            if enabled {
                conn.timestamps.enable();
            }
        })
    }

    /// Update timestamp for a connection
    pub fn update_timestamp(&self, connection_id: u32, ts_val: u32, ts_echo: u32) -> Result<(), ()> {
        let mut stats = self.stats.lock();
        stats.timestamp_updates += 1;
        drop(stats);

        self.update_connection(connection_id, |conn| {
            conn.timestamps.update_timestamp(ts_val);
            conn.timestamps.set_echo_reply(ts_echo);
        })
    }

    /// Set congestion control algorithm
    pub fn set_congestion_control(&self, algorithm: CongestionControl) -> Result<(), ()> {
        let mut guard = self.connections.write();
        for conn in guard.values_mut() {
            conn.congestion.algorithm = algorithm;
        }
        Ok(())
    }

    /// Configure connection with specific congestion control
    pub fn configure_congestion_control(
        &self,
        connection_id: u32,
        algorithm: CongestionControl,
    ) -> Result<(), ()> {
        self.update_connection(connection_id, |conn| {
            conn.congestion.algorithm = algorithm;
        })
    }

    /// Process ACK and update congestion control
    pub fn process_ack(
        &self,
        connection_id: u32,
        ack_seq: u32,
        acked_bytes: u32,
    ) -> Result<bool, ()> {
        let mut stats = self.stats.lock();

        self.update_connection(connection_id, |conn| {
            // Check if new data was acknowledged
            let data_acked = ack_seq > conn.snd_una;
            let should_retransmit = conn.fast_retransmit.on_ack(ack_seq, data_acked);

            if should_retransmit {
                stats.fast_retransmits += 1;
            }

            if data_acked {
                conn.snd_una = ack_seq;
                conn.congestion.on_ack(acked_bytes);
            }

            if !conn.fast_retransmit.in_recovery && conn.fast_retransmit.dup_ack_count == 0 {
                stats.fast_recoveries += 1;
            }
        });

        // Return whether fast retransmit was triggered
        let connections = self.connections.read();
        if let Some(conn) = connections.get(&connection_id) {
            Ok(conn.fast_retransmit.should_retransmit())
        } else {
            Err(())
        }
    }

    /// Handle packet loss event
    pub fn handle_packet_loss(&self, connection_id: u32) -> Result<(), ()> {
        let mut stats = self.stats.lock();
        stats.congestion_events += 1;
        drop(stats);

        self.update_connection(connection_id, |conn| {
            conn.congestion.on_loss();
        })
    }

    /// Enable MD5 signatures for a connection
    pub fn enable_md5(&self, connection_id: u32, key: Vec<u8>) -> Result<(), ()> {
        self.update_connection(connection_id, |conn| {
            conn.md5.set_key(key);
        })
    }

    /// Verify MD5 signature
    pub fn verify_md5(&self, connection_id: u32, data: &[u8], signature: &[u8; 16]) -> Result<bool, ()> {
        let connections = self.connections.read();
        if let Some(conn) = connections.get(&connection_id) {
            let mut stats = self.stats.lock();
            if conn.md5.verify(data, signature) {
                stats.md5_verifications += 1;
                Ok(true)
            } else {
                stats.md5_failures += 1;
                Ok(false)
            }
        } else {
            Err(())
        }
    }

    /// Enable TCP-AO for a connection
    pub fn enable_ao(
        &self,
        connection_id: u32,
        algorithm: AoAlgorithm,
        key: Vec<u8>,
        key_id: u8,
    ) -> Result<(), ()> {
        self.update_connection(connection_id, |conn| {
            conn.ao.enable(algorithm, key, key_id);
        })
    }

    /// Verify TCP-AO MAC
    pub fn verify_ao(&self, connection_id: u32, data: &[u8], mac: &[u8; 12]) -> Result<bool, ()> {
        let connections = self.connections.read();
        if let Some(conn) = connections.get(&connection_id) {
            let mut stats = self.stats.lock();
            if conn.ao.verify_mac(data, mac) {
                stats.ao_verifications += 1;
                Ok(true)
            } else {
                stats.ao_failures += 1;
                Ok(false)
            }
        } else {
            Err(())
        }
    }

    /// Get statistics
    pub fn get_stats(&self) -> TcpAdvancedStats {
        let stats = self.stats.lock();
        stats.clone()
    }

    /// Reset statistics
    pub fn reset_stats(&self) {
        let mut stats = self.stats.lock();
        *stats = TcpAdvancedStats::default();
    }
}

impl Default for TcpAdvanced {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sack_block_creation() {
        let block = SackBlock::new(1000, 2000);
        assert_eq!(block.left, 1000);
        assert_eq!(block.right, 2000);
        assert_eq!(block.len(), 1000);
    }

    #[test]
    fn test_sack_block_overlap() {
        let block1 = SackBlock::new(1000, 2000);
        let block2 = SackBlock::new(1500, 2500);
        assert!(block1.overlaps(&block2));

        let block3 = SackBlock::new(3000, 4000);
        assert!(!block1.overlaps(&block3));
    }

    #[test]
    fn test_sack_block_merge() {
        let block1 = SackBlock::new(1000, 2000);
        let block2 = SackBlock::new(1500, 2500);
        let merged = block1.merge(&block2);
        assert_eq!(merged, Some(SackBlock::new(1000, 2500)));
    }

    #[test]
    fn test_window_scale() {
        let ws = WindowScale::new(7);
        assert_eq!(ws.shift, 7);
        assert_eq!(ws.max_window(), 1 << 30);

        // Test scaling
        let scaled = ws.scale_window(65535);
        assert_eq!(scaled, 511); // 65535 >> 7

        let window = ws.calculate_window(511);
        assert_eq!(window, 65536); // 511 << 7
    }

    #[test]
    fn test_timestamp_option() {
        let mut ts = TimestampOption::new();
        assert!(!ts.enabled);

        ts.enable();
        assert!(ts.enabled);

        ts.update_timestamp(1000);
        assert_eq!(ts.ts_val, 1000);

        ts.set_echo_reply(500);
        assert_eq!(ts.ts_echo_reply, 500);

        // Test PAWS
        assert!(ts.is_recent(2000));
        assert!(!ts.is_recent(100));
    }

    #[test]
    fn test_congestion_control_from_str() {
        assert_eq!(CongestionControl::from_str("cubic"), Some(CongestionControl::Cubic));
        assert_eq!(CongestionControl::from_str("bbr"), Some(CongestionControl::Bbr));
        assert_eq!(CongestionControl::from_str("unknown"), None);
    }

    #[test]
    fn test_congestion_state_reno() {
        let mut cc = CongestionControlState::new(CongestionControl::Reno, 10);
        assert_eq!(cc.state, CongestionState::SlowStart);

        // Simulate slow start
        let initial_cwnd = cc.cwnd;
        cc.on_ack(1460);
        assert!(cc.cwnd > initial_cwnd);

        // Trigger loss
        cc.on_loss();
        assert_eq!(cc.state, CongestionState::FastRetransmit);
        assert!(cc.cwnd < initial_cwnd);
    }

    #[test]
    fn test_fast_retransmit() {
        let mut fr = FastRetransmit::new();
        assert!(!fr.should_retransmit());

        // Send duplicate ACKs
        assert!(!fr.on_ack(1000, false)); // 1st dup ACK
        assert!(!fr.on_ack(1000, false)); // 2nd dup ACK
        assert!(fr.on_ack(1000, false));  // 3rd dup ACK - should trigger

        assert!(fr.in_recovery);
        assert_eq!(fr.dup_ack_count, 3);
    }

    #[test]
    fn test_tcp_connection_state() {
        let conn = TcpConnectionState::new(1, 1000, 5000);
        assert_eq!(conn.connection_id, 1);
        assert_eq!(conn.iss, 1000);
        assert_eq!(conn.irs, 5000);
        assert_eq!(conn.snd_nxt, 1001);
        assert_eq!(conn.rcv_nxt, 5001);
    }

    #[test]
    fn test_sack_blocks_addition() {
        let mut conn = TcpConnectionState::new(1, 1000, 5000);
        conn.sack_enabled = true;

        let block1 = SackBlock::new(6000, 7000);
        let block2 = SackBlock::new(8000, 9000);

        conn.add_sack_block(block1);
        conn.add_sack_block(block2);

        assert_eq!(conn.sack_blocks.len(), 2);
        assert_eq!(conn.get_sack_blocks(), &[block1, block2]);
    }

    #[test]
    fn test_tcp_advanced_create_connection() {
        let tcp = TcpAdvanced::new();
        let id = tcp.create_connection(1000, 5000);
        assert!(id > 0);

        let conn = tcp.get_connection(id);
        assert!(conn.is_some());
        let conn = conn.unwrap();
        assert_eq!(conn.connection_id, id);
        assert_eq!(conn.iss, 1000);
        assert_eq!(conn.irs, 5000);
    }

    #[test]
    fn test_tcp_advanced_sack() {
        let tcp = TcpAdvanced::new();
        let id = tcp.create_connection(1000, 5000);

        assert!(tcp.enable_sack(id, true).is_ok());

        let blocks = vec![SackBlock::new(6000, 7000), SackBlock::new(8000, 9000)];
        assert!(tcp.process_sack(id, &blocks).is_ok());

        let conn = tcp.get_connection(id).unwrap();
        assert_eq!(conn.sack_blocks.len(), 2);
    }

    #[test]
    fn test_tcp_advanced_congestion_control() {
        let tcp = TcpAdvanced::new();
        assert!(tcp.set_congestion_control(CongestionControl::Bbr).is_ok());

        let id = tcp.create_connection(1000, 5000);
        assert!(tcp.configure_congestion_control(id, CongestionControl::Cubic).is_ok());

        let conn = tcp.get_connection(id).unwrap();
        assert_eq!(conn.congestion.algorithm, CongestionControl::Cubic);
    }

    #[test]
    fn test_tcp_advanced_process_ack() {
        let tcp = TcpAdvanced::new();
        let id = tcp.create_connection(1000, 5000);

        // Process ACK that acknowledges new data
        let result = tcp.process_ack(id, 6000, 1000);
        assert!(result.is_ok());
        assert!(!result.unwrap()); // No fast retransmit yet

        // Process duplicate ACKs
        tcp.process_ack(id, 6000, 0).unwrap();
        tcp.process_ack(id, 6000, 0).unwrap();
        let result = tcp.process_ack(id, 6000, 0).unwrap();
        assert!(result); // Should trigger fast retransmit
    }

    #[test]
    fn test_tcp_advanced_stats() {
        let tcp = TcpAdvanced::new();
        let id = tcp.create_connection(1000, 5000);

        tcp.enable_sack(id, true).unwrap();
        tcp.process_sack(id, &[SackBlock::new(6000, 7000)]).unwrap();

        let stats = tcp.get_stats();
        assert!(stats.sack_blocks_received > 0);

        tcp.reset_stats();
        let stats = tcp.get_stats();
        assert_eq!(stats.sack_blocks_received, 0);
    }

    #[test]
    fn test_md5_signature() {
        let mut md5 = TcpMd5Signature::new();
        assert!(!md5.enabled);

        md5.set_key(vec![1, 2, 3, 4, 5]);
        assert!(md5.enabled);
        assert_eq!(md5.key, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_tcp_ao() {
        let mut ao = TcpAuthenticationOption::new();
        assert!(!ao.enabled);

        ao.enable(AoAlgorithm::HmacSha1_96, vec![1, 2, 3], 5);
        assert!(ao.enabled);
        assert_eq!(ao.algorithm, AoAlgorithm::HmacSha1_96);
        assert_eq!(ao.key_id, 5);
    }
}
