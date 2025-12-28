//! TCP state machine implementation
//!
//! This module provides a complete TCP state machine implementation with
//! proper state transitions, timer management, and connection lifecycle.

extern crate alloc;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

use core::sync::atomic::{AtomicU64, Ordering};

use super::{TcpPacket, TcpState};
use super::tcp_flags;
use crate::net::ipv4::Ipv4Addr;

/// TCP connection state machine
#[derive(Debug, Clone)]
pub struct TcpStateMachine {
    /// Current state
    state: TcpState,
    /// Local sequence number
    local_seq: u32,
    /// Remote sequence number
    remote_seq: u32,
    /// Local acknowledgment number
    local_ack: u32,
    /// Remote acknowledgment number
    remote_ack: u32,
    /// Local window size
    local_window: u16,
    /// Remote window size
    remote_window: u16,
    /// Connection timestamps
    timestamps: TcpTimestamps,
    /// Retransmission queue
    retransmit_queue: VecDeque<TcpSegment>,
    /// Congestion control state
    congestion: TcpCongestionControl,
    /// Flow control state
    flow_control: TcpFlowControl,
}

/// TCP timestamps for timeout management
#[derive(Debug, Clone)]
pub struct TcpTimestamps {
    /// Connection establishment time
    pub connection_time: u64,
    /// Last data sent time
    pub last_send_time: u64,
    /// Last data received time
    pub last_recv_time: u64,
    /// Last ACK received time
    pub last_ack_time: u64,
}

/// TCP segment for retransmission
#[derive(Debug, Clone)]
pub struct TcpSegment {
    /// Sequence number
    pub seq: u32,
    /// Acknowledgment number
    pub ack: u32,
    /// Flags
    pub flags: u8,
    /// Window size
    pub window: u16,
    /// Data payload
    pub data: Vec<u8>,
    /// Number of retransmissions
    pub retransmit_count: u32,
    /// Last transmission time
    pub last_tx_time: u64,
    /// Retransmission timeout
    pub rto: u64,
}

/// TCP congestion control state
#[derive(Debug, Clone)]
pub struct TcpCongestionControl {
    /// Congestion window (in bytes)
    pub cwnd: u32,
    /// Slow start threshold
    pub ssthresh: u32,
    /// Current state
    pub state: CongestionState,
    /// Duplicate ACK count
    pub dup_ack_count: u32,
    /// Last ACK received
    pub last_ack: u32,
    /// Round-trip time estimator
    pub rtt_estimator: RttEstimator,
}

/// Congestion control states
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

/// RTT (Round-Trip Time) estimator
#[derive(Debug, Clone)]
pub struct RttEstimator {
    /// Smoothed RTT
    pub srtt: u32,
    /// RTT variance
    pub rttvar: u32,
    /// Retransmission timeout
    pub rto: u32,
    /// Minimum RTT seen
    pub min_rtt: u32,
}

/// TCP flow control state
#[derive(Debug, Clone)]
pub struct TcpFlowControl {
    /// Advertised window
    pub advertised_window: u16,
    /// Effective window
    pub effective_window: u32,
    /// Outstanding bytes
    pub outstanding_bytes: u32,
    /// Maximum window size
    pub max_window: u16,
}

impl TcpStateMachine {
    /// Create a new TCP state machine
    pub fn new(is_passive: bool) -> Self {
        let state = if is_passive {
            TcpState::Listen
        } else {
            TcpState::Closed
        };

        let now = Self::current_time();

        Self {
            state,
            local_seq: Self::generate_initial_seq(),
            remote_seq: 0,
            local_ack: 0,
            remote_ack: 0,
            local_window: 65535,
            remote_window: 0,
            timestamps: TcpTimestamps {
                connection_time: now,
                last_send_time: now,
                last_recv_time: now,
                last_ack_time: now,
            },
            retransmit_queue: VecDeque::new(),
            congestion: TcpCongestionControl::new(),
            flow_control: TcpFlowControl::new(),
        }
    }

    /// Generate initial sequence number
    fn generate_initial_seq() -> u32 {
        use core::sync::atomic::{AtomicU32, Ordering};
        static SEQ_GENERATOR: AtomicU32 = AtomicU32::new(1);
        SEQ_GENERATOR.fetch_add(1000, Ordering::Relaxed)
    }

    /// Get current time (in seconds)
    fn current_time() -> u64 {
        static TIMER: AtomicU64 = AtomicU64::new(0);
        TIMER.fetch_add(1, Ordering::Relaxed)
    }

    /// Process incoming packet
    pub fn process_packet(&mut self, packet: &TcpPacket) -> TcpAction {
        self.timestamps.last_recv_time = Self::current_time();

        // Update remote sequence and acknowledgment numbers
        self.remote_seq = packet.seq_num();
        self.remote_ack = packet.ack_num();
        self.remote_window = packet.dst_port(); // Note: This is a hack, should use header.window_size

        match self.state {
            TcpState::Closed => self.handle_closed(packet),
            TcpState::Listen => self.handle_listen(packet),
            TcpState::SynSent => self.handle_syn_sent(packet),
            TcpState::SynReceived => self.handle_syn_received(packet),
            TcpState::Established => self.handle_established(packet),
            TcpState::FinWait1 => self.handle_fin_wait1(packet),
            TcpState::FinWait2 => self.handle_fin_wait2(packet),
            TcpState::CloseWait => self.handle_close_wait(packet),
            TcpState::Closing => self.handle_closing(packet),
            TcpState::LastAck => self.handle_last_ack(packet),
            TcpState::TimeWait => self.handle_time_wait(packet),
        }
    }

    /// Handle packets in CLOSED state
    fn handle_closed(&mut self, packet: &TcpPacket) -> TcpAction {
        if packet.has_flag(tcp_flags::RST) {
            TcpAction::None
        } else {
            // Send RST
            TcpAction::SendRst
        }
    }

    /// Handle packets in LISTEN state
    fn handle_listen(&mut self, packet: &TcpPacket) -> TcpAction {
        if packet.has_flag(tcp_flags::SYN) && !packet.has_flag(tcp_flags::ACK) {
            // Received SYN, transition to SYN_RECEIVED
            self.state = TcpState::SynReceived;
            self.remote_seq = packet.seq_num();
            self.local_ack = packet.seq_num() + 1; // SYN consumes one sequence number
            self.local_seq = Self::generate_initial_seq();

            // Send SYN-ACK
            TcpAction::SendSynAck
        } else if packet.has_flag(tcp_flags::RST) {
            TcpAction::None
        } else {
            // Send RST for other packets
            TcpAction::SendRst
        }
    }

    /// Handle packets in SYN_SENT state
    fn handle_syn_sent(&mut self, packet: &TcpPacket) -> TcpAction {
        if packet.has_flag(tcp_flags::SYN) && packet.has_flag(tcp_flags::ACK) {
            if packet.ack_num() == self.local_seq + 1 {
                // Received SYN-ACK, transition to ESTABLISHED
                self.state = TcpState::Established;
                self.remote_seq = packet.seq_num() + 1; // SYN consumes one sequence number
                self.local_ack = packet.ack_num();

                // Send ACK
                TcpAction::SendAck
            } else {
                TcpAction::SendRst
            }
        } else if packet.has_flag(tcp_flags::SYN) {
            // Simultaneous open
            self.state = TcpState::SynReceived;
            self.remote_seq = packet.seq_num() + 1;
            self.local_ack = packet.seq_num() + 1;

            TcpAction::SendSynAck
        } else if packet.has_flag(tcp_flags::RST) {
            // Connection reset
            self.state = TcpState::Closed;
            TcpAction::ConnectionReset
        } else {
            TcpAction::None
        }
    }

    /// Handle packets in SYN_RECEIVED state
    fn handle_syn_received(&mut self, packet: &TcpPacket) -> TcpAction {
        if packet.has_flag(tcp_flags::ACK) {
            if packet.ack_num() == self.local_seq + 1 {
                // Received ACK for SYN-ACK, transition to ESTABLISHED
                self.state = TcpState::Established;
                self.local_ack = packet.ack_num();

                TcpAction::ConnectionEstablished
            } else {
                TcpAction::SendRst
            }
        } else if packet.has_flag(tcp_flags::SYN) && !packet.has_flag(tcp_flags::ACK) {
            // Retransmit SYN-ACK (duplicate SYN)
            TcpAction::SendSynAck
        } else if packet.has_flag(tcp_flags::RST) {
            self.state = TcpState::Closed;
            TcpAction::ConnectionReset
        } else {
            TcpAction::None
        }
    }

    /// Handle packets in ESTABLISHED state
    fn handle_established(&mut self, packet: &TcpPacket) -> TcpAction {
        if packet.has_flag(tcp_flags::FIN) {
            // Received FIN, transition to CLOSE_WAIT
            self.state = TcpState::CloseWait;
            self.local_ack = packet.seq_num() + 1; // FIN consumes one sequence number

            // Send ACK
            TcpAction::SendAck
        } else if packet.has_flag(tcp_flags::RST) {
            self.state = TcpState::Closed;
            TcpAction::ConnectionReset
        } else if packet.has_flag(tcp_flags::SYN) {
            // Ignore duplicate SYN
            TcpAction::None
        } else if !packet.payload.is_empty() {
            // Handle data packet
            self.handle_data_packet(packet)
        } else if packet.has_flag(tcp_flags::ACK) {
            // Handle ACK
            self.handle_ack_packet(packet)
        } else {
            TcpAction::None
        }
    }

    /// Handle data packets
    fn handle_data_packet(&mut self, packet: &TcpPacket) -> TcpAction {
        // Update local acknowledgment
        self.local_ack = packet.seq_num() + packet.payload.len() as u32;

        // Update congestion control
        self.congestion.on_data_received(packet.seq_num() + packet.payload.len() as u32);

        // Return data to application
        TcpAction::DataReceived(packet.payload.clone())
    }

    /// Handle ACK packets
    fn handle_ack_packet(&mut self, packet: &TcpPacket) -> TcpAction {
        // Update flow control
        let newly_acked = packet.ack_num().saturating_sub(self.remote_ack);
        if newly_acked > 0 {
            self.flow_control.on_ack_received(newly_acked);
            self.remote_ack = packet.ack_num();
        }

        // Update congestion control
        self.congestion.on_ack_received(packet.ack_num());

        // Check if we can send more data
        if self.flow_control.can_send() && self.congestion.can_send() {
            TcpAction::CanSendData
        } else {
            TcpAction::None
        }
    }

    /// Handle packets in FIN_WAIT1 state
    fn handle_fin_wait1(&mut self, packet: &TcpPacket) -> TcpAction {
        if packet.has_flag(tcp_flags::ACK) {
            if packet.ack_num() == self.local_seq + 1 {
                // ACK for our FIN, transition to FIN_WAIT2
                self.state = TcpState::FinWait2;
                TcpAction::None
            } else {
                TcpAction::None
            }
        } else if packet.has_flag(tcp_flags::FIN) {
            // Received FIN before our FIN was ACKed
            self.state = TcpState::Closing;
            self.local_ack = packet.seq_num() + 1;
            TcpAction::SendAck
        } else if packet.has_flag(tcp_flags::RST) {
            self.state = TcpState::Closed;
            TcpAction::ConnectionReset
        } else {
            TcpAction::None
        }
    }

    /// Handle packets in FIN_WAIT2 state
    fn handle_fin_wait2(&mut self, packet: &TcpPacket) -> TcpAction {
        if packet.has_flag(tcp_flags::FIN) {
            // Received FIN, transition to TIME_WAIT
            self.state = TcpState::TimeWait;
            self.local_ack = packet.seq_num() + 1;

            TcpAction::SendAck
        } else if packet.has_flag(tcp_flags::RST) {
            self.state = TcpState::Closed;
            TcpAction::ConnectionReset
        } else {
            TcpAction::None
        }
    }

    /// Handle packets in CLOSE_WAIT state
    fn handle_close_wait(&mut self, packet: &TcpPacket) -> TcpAction {
        if packet.has_flag(tcp_flags::RST) {
            self.state = TcpState::Closed;
            TcpAction::ConnectionReset
        } else {
            TcpAction::None
        }
    }

    /// Handle packets in CLOSING state
    fn handle_closing(&mut self, packet: &TcpPacket) -> TcpAction {
        if packet.has_flag(tcp_flags::ACK) {
            if packet.ack_num() == self.local_seq + 1 {
                // ACK for our FIN, transition to TIME_WAIT
                self.state = TcpState::TimeWait;
                TcpAction::None
            } else {
                TcpAction::None
            }
        } else if packet.has_flag(tcp_flags::RST) {
            self.state = TcpState::Closed;
            TcpAction::ConnectionReset
        } else {
            TcpAction::None
        }
    }

    /// Handle packets in LAST_ACK state
    fn handle_last_ack(&mut self, packet: &TcpPacket) -> TcpAction {
        if packet.has_flag(tcp_flags::ACK) {
            if packet.ack_num() == self.local_seq + 1 {
                // ACK for our FIN, connection closed
                self.state = TcpState::Closed;
                TcpAction::ConnectionClosed
            } else {
                TcpAction::None
            }
        } else if packet.has_flag(tcp_flags::RST) {
            self.state = TcpState::Closed;
            TcpAction::ConnectionReset
        } else {
            TcpAction::None
        }
    }

    /// Handle packets in TIME_WAIT state
    fn handle_time_wait(&mut self, packet: &TcpPacket) -> TcpAction {
        if packet.has_flag(tcp_flags::RST) {
            self.state = TcpState::Closed;
            TcpAction::ConnectionReset
        } else {
            // Ignore other packets in TIME_WAIT
            TcpAction::None
        }
    }

    /// Initiate active open (connect)
    pub fn active_open(&mut self, _remote_addr: Ipv4Addr, _remote_port: u16) -> TcpAction {
        if self.state != TcpState::Closed {
            return TcpAction::Error;
        }

        self.state = TcpState::SynSent;
        self.local_seq = Self::generate_initial_seq();

        TcpAction::SendSyn
    }

    /// Initiate passive open (listen)
    pub fn passive_open(&mut self) -> TcpAction {
        if self.state != TcpState::Closed {
            return TcpAction::Error;
        }

        self.state = TcpState::Listen;
        TcpAction::None
    }

    /// Send data
    pub fn send_data(&mut self, data: Vec<u8>) -> TcpAction {
        if self.state != TcpState::Established {
            return TcpAction::Error;
        }

        if data.is_empty() {
            return TcpAction::None;
        }

        let max_send = self.flow_control.get_send_window().min(self.congestion.get_send_window());
        if max_send == 0 || data.len() as u32 > max_send {
            return TcpAction::None; // Window is full
        }

        // Add to retransmission queue
        let segment = TcpSegment {
            seq: self.local_seq,
            ack: self.local_ack,
            flags: tcp_flags::ACK | tcp_flags::PSH,
            window: self.local_window,
            data: data.clone(),
            retransmit_count: 0,
            last_tx_time: Self::current_time(),
            rto: self.congestion.rtt_estimator.rto as u64,
        };

        self.retransmit_queue.push_back(segment);
        self.local_seq += data.len() as u32;
        self.flow_control.on_data_sent(data.len() as u32);

        TcpAction::SendData(data)
    }

    /// Close connection
    pub fn close(&mut self) -> TcpAction {
        match self.state {
            TcpState::Established => {
                self.state = TcpState::FinWait1;
                TcpAction::SendFin
            }
            TcpState::CloseWait => {
                self.state = TcpState::LastAck;
                TcpAction::SendFin
            }
            TcpState::Listen => {
                self.state = TcpState::Closed;
                TcpAction::ConnectionClosed
            }
            _ => TcpAction::Error,
        }
    }

    /// Get current state
    pub fn state(&self) -> TcpState {
        self.state
    }

    /// Check if connection is established
    pub fn is_established(&self) -> bool {
        self.state == TcpState::Established
    }

    /// Check if connection is closed
    pub fn is_closed(&self) -> bool {
        self.state == TcpState::Closed
    }

    /// Check if we can send data
    pub fn can_send(&self) -> bool {
        self.state == TcpState::Established &&
        self.flow_control.can_send() &&
        self.congestion.can_send()
    }

    /// Get retransmission timeout check
    pub fn check_timeouts(&mut self) -> Vec<TcpAction> {
        let mut actions = Vec::new();
        let now = Self::current_time();

        // Check retransmission timeouts
        for segment in &mut self.retransmit_queue {
            if now - segment.last_tx_time >= segment.rto {
                segment.retransmit_count += 1;
                segment.last_tx_time = now;
                segment.rto = core::cmp::min(segment.rto * 2, 60000); // Exponential backoff, max 60s

                actions.push(TcpAction::Retransmit(segment.data.clone()));
            }
        }

        // Check TIME_WAIT timeout
        if self.state == TcpState::TimeWait {
            if now - self.timestamps.connection_time >= 120 { // 2 minutes TIME_WAIT
                self.state = TcpState::Closed;
                actions.push(TcpAction::ConnectionClosed);
            }
        }

        actions
    }
}

/// TCP action returned by state machine
#[derive(Debug, Clone)]
pub enum TcpAction {
    /// No action
    None,
    /// Send SYN
    SendSyn,
    /// Send SYN-ACK
    SendSynAck,
    /// Send ACK
    SendAck,
    /// Send RST
    SendRst,
    /// Send FIN
    SendFin,
    /// Send data
    SendData(Vec<u8>),
    /// Retransmit data
    Retransmit(Vec<u8>),
    /// Data received
    DataReceived(Vec<u8>),
    /// Can send more data
    CanSendData,
    /// Connection established
    ConnectionEstablished,
    /// Connection closed
    ConnectionClosed,
    /// Connection reset
    ConnectionReset,
    /// Error occurred
    Error,
}

impl TcpCongestionControl {
    /// Create new congestion control state
    pub fn new() -> Self {
        Self {
            cwnd: 10 * 1460, // Initial congestion window (10 * MSS)
            ssthresh: u32::MAX,
            state: CongestionState::SlowStart,
            dup_ack_count: 0,
            last_ack: 0,
            rtt_estimator: RttEstimator::new(),
        }
    }

    /// Handle new ACK
    pub fn on_ack_received(&mut self, ack: u32) {
        if ack > self.last_ack {
            // New ACK received
            let acknowledged = ack - self.last_ack;
            self.last_ack = ack;
            self.dup_ack_count = 0;

            match self.state {
                CongestionState::SlowStart => {
                    self.cwnd += acknowledged;
                    if self.cwnd >= self.ssthresh {
                        self.state = CongestionState::CongestionAvoidance;
                    }
                }
                CongestionState::CongestionAvoidance => {
                    self.cwnd += (acknowledged * 1460) / self.cwnd; // AIMD
                }
                CongestionState::FastRecovery => {
                    self.cwnd += acknowledged;
                    self.state = CongestionState::CongestionAvoidance;
                }
                CongestionState::FastRetransmit => {
                    self.state = CongestionState::FastRecovery;
                }
            }
        } else if ack == self.last_ack {
            // Duplicate ACK
            self.dup_ack_count += 1;
            if self.dup_ack_count == 3 {
                // Fast retransmit
                self.state = CongestionState::FastRetransmit;
                self.ssthresh = self.cwnd / 2;
                self.cwnd = self.ssthresh + 3 * 1460;
            }
        }
    }

    /// Handle data received
    pub fn on_data_received(&mut self, _seq: u32) {
        // Update RTT measurement if we have timing information
        self.rtt_estimator.update_measurement(1.0); // Placeholder RTT
    }

    /// Check if we can send
    pub fn can_send(&self) -> bool {
        self.cwnd > 0
    }

    /// Get send window
    pub fn get_send_window(&self) -> u32 {
        self.cwnd
    }

    /// Handle packet loss
    pub fn on_packet_loss(&mut self) {
        self.ssthresh = self.cwnd / 2;
        self.cwnd = 1460; // Reset to 1 MSS
        self.state = CongestionState::SlowStart;
    }
}

impl RttEstimator {
    /// Create new RTT estimator
    pub fn new() -> Self {
        Self {
            srtt: 1000,      // 1 second initial
            rttvar: 500,      // 0.5 second initial variance
            rto: 3000,        // 3 second initial timeout
            min_rtt: u32::MAX,
        }
    }

    /// Update RTT measurement
    pub fn update_measurement(&mut self, rtt: f64) {
        let rtt_ms = (rtt * 1000.0) as u32;

        if rtt_ms < self.min_rtt {
            self.min_rtt = rtt_ms;
        }

        // Jacobson/Karels algorithm
        let rtt_diff = if self.srtt > rtt_ms {
            self.srtt - rtt_ms
        } else {
            rtt_ms - self.srtt
        };

        self.rttvar = (3 * self.rttvar + rtt_diff) / 4;
        self.srtt = (7 * self.srtt + rtt_ms) / 8;

        // Calculate RTO
        self.rto = self.srtt + 4 * self.rttvar;
        self.rto = core::cmp::max(self.rto, 1000); // Minimum 1 second
        self.rto = core::cmp::min(self.rto, 60000); // Maximum 60 seconds
    }
}

impl TcpFlowControl {
    /// Create new flow control state
    pub fn new() -> Self {
        Self {
            advertised_window: 65535,
            effective_window: 0,
            outstanding_bytes: 0,
            max_window: 65535,
        }
    }

    /// Handle ACK received
    pub fn on_ack_received(&mut self, bytes_acked: u32) {
        self.outstanding_bytes = self.outstanding_bytes.saturating_sub(bytes_acked);
        self.update_effective_window();
    }

    /// Handle data sent
    pub fn on_data_sent(&mut self, bytes_sent: u32) {
        self.outstanding_bytes += bytes_sent;
        self.update_effective_window();
    }

    /// Update remote window
    pub fn update_remote_window(&mut self, remote_window: u16) {
        self.advertised_window = remote_window;
        self.update_effective_window();
    }

    /// Update effective window
    fn update_effective_window(&mut self) {
        self.effective_window = (self.advertised_window as u32).saturating_sub(self.outstanding_bytes);
    }

    /// Check if we can send
    pub fn can_send(&self) -> bool {
        self.effective_window > 0
    }

    /// Get send window
    pub fn get_send_window(&self) -> u32 {
        self.effective_window
    }
}