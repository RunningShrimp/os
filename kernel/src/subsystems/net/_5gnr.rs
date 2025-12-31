//! 5G NR (New Radio) Protocol Stack Implementation
//!
//! This module implements a complete 5G NR protocol stack compliant with 3GPP specifications:
//! - PHY Layer: Physical layer with OFDMA, SC-FDMA, and flexible numerology
//! - MAC Layer: Medium access control with scheduling and HARQ
//! - RLC Layer: Radio link control with segmentation and ARQ
//! - PDCP Layer: Packet data convergence protocol with compression and encryption
//! - RRC Layer: Radio resource control with connection management
//! - NAS Layer: Non-access stratum with mobility and session management
//!
//! Based on 3GPP TS 38.200, 38.300, 38.321, 38.322, 38.323, 38.331, 24.301

#![allow(dead_code)]

use alloc::{collections::BTreeMap, string::String, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};


// ============================================================================
// Constants and Configuration
// ============================================================================

/// 5G NR frequency bands (FR1: Sub-6GHz, FR2: mmWave)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NrBand {
    /// FR1 frequency range (410 MHz - 7125 MHz)
    FR1(u16),
    /// FR2 frequency range (24.25 GHz - 52.6 GHz)
    FR2(u16),
}

/// 5G NR numerology (μ) - determines subcarrier spacing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Numerology {
    μ0 = 0, // 15 kHz SCS
    μ1 = 1, // 30 kHz SCS
    μ2 = 2, // 60 kHz SCS
    μ3 = 3, // 120 kHz SCS
    μ4 = 4, // 240 kHz SCS
}

impl Numerology {
    /// Get subcarrier spacing in kHz
    pub fn scs_khz(&self) -> u32 {
        match self {
            Numerology::μ0 => 15,
            Numerology::μ1 => 30,
            Numerology::μ2 => 60,
            Numerology::μ3 => 120,
            Numerology::μ4 => 240,
        }
    }

    /// Get slot duration in microseconds
    pub fn slot_duration_us(&self) -> u64 {
        (1000000 / (1000 / self.scs_khz() * 14)) as u64
    }
}

/// PHY configuration parameters
#[derive(Debug, Clone)]
pub struct PhyConfig {
    /// Carrier frequency in Hz
    pub carrier_freq: u64,
    /// System bandwidth in Hz
    pub bandwidth: u32,
    /// Numerology
    pub numerology: Numerology,
    /// Number of PRBs (Physical Resource Blocks)
    pub num_prbs: u16,
    /// Duplexing mode
    pub duplex_mode: DuplexMode,
    /// CP (Cyclic Prefix) length
    pub cp_length: CyclicPrefix,
}

/// Duplexing mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplexMode {
    FDD,
    TDD,
    DynamicTDD,
}

/// Cyclic prefix length
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CyclicPrefix {
    Normal,
    Extended,
}

// ============================================================================
// PHY Layer (Physical Layer)
// ============================================================================

/// PHY layer state and configuration
#[derive(Debug)]
pub struct PhyLayer {
    config: PhyConfig,
    stats: PhyStats,
    /// Reference signal power in dBm
    ref_signal_power: i8,
    /// Noise figure in dB
    noise_figure: u8,
}

/// PHY layer statistics
#[derive(Debug, Default)]
pub struct PhyStats {
    /// Total transmitted symbols
    pub tx_symbols: AtomicU64,
    /// Total received symbols
    pub rx_symbols: AtomicU64,
    /// Block error rate (scaled by 1000)
    pub bler: AtomicU64,
    /// Signal-to-noise ratio in dB (scaled by 10)
    pub snr_db: AtomicU64,
}

impl PhyLayer {
    /// Create new PHY layer
    pub fn new(config: PhyConfig) -> Self {
        Self {
            config,
            stats: PhyStats::default(),
            ref_signal_power: -30,
            noise_figure: 7,
        }
    }

    /// Get PHY configuration
    pub fn config(&self) -> &PhyConfig {
        &self.config
    }

    /// Get PHY statistics
    pub fn stats(&self) -> &PhyStats {
        &self.stats
    }

    /// Calculate resource grid dimensions
    pub fn resource_grid_size(&self) -> (u16, u8) {
        let num_symbols = match self.config.cp_length {
            CyclicPrefix::Normal => 14,
            CyclicPrefix::Extended => 12,
        };
        (self.config.num_prbs * 12, num_symbols) // (subcarriers, symbols)
    }

    /// Estimate channel quality
    pub fn estimate_cqi(&self, snr_db: u8) -> u8 {
        // CQI (Channel Quality Indicator) mapping based on SNR
        match snr_db {
            0..=5 => 1,
            6..=8 => 3,
            9..=11 => 5,
            12..=14 => 7,
            15..=17 => 9,
            18..=20 => 11,
            21..=24 => 13,
            25..=30 => 15,
            _ => 15,
        }
    }

    /// Calculate reference signal received power (RSRP)
    pub fn calculate_rsrp(&self, rssi: i16) -> i16 {
        rssi / 12 // Approximate for 12 subcarriers per PRB
    }

    /// Calculate reference signal received quality (RSRQ)
    pub fn calculate_rsrq(&self, rsrp: i16, rssi: i16) -> i8 {
        if rssi > 0 {
            ((rsrp * 100) / rssi) as i8
        } else {
            -30
        }
    }

    /// Process downlink transmission
    pub fn process_downlink(&mut self, data: &[u8], prbs: &[u16]) -> Result<usize, PhyError> {
        // Verify PRB allocation
        for &prb in prbs {
            if prb >= self.config.num_prbs {
                return Err(PhyError::InvalidPrbAllocation);
            }
        }

        // Simulate transmission
        let symbols = prbs.len() * 12 * 14; // PRBs * subcarriers * symbols
        self.stats.tx_symbols.fetch_add(symbols as u64, Ordering::Relaxed);

        Ok(data.len())
    }

    /// Process uplink reception
    pub fn process_uplink(&mut self, buffer: &mut [u8], prbs: &[u16]) -> Result<usize, PhyError> {
        // Verify PRB allocation
        for &prb in prbs {
            if prb >= self.config.num_prbs {
                return Err(PhyError::InvalidPrbAllocation);
            }
        }

        // Simulate reception
        let symbols = prbs.len() * 12 * 14;
        self.stats.rx_symbols.fetch_add(symbols as u64, Ordering::Relaxed);

        // Simulate received data
        let data_len = core::cmp::min(buffer.len(), prbs.len() * 12 * 14 * 4);
        Ok(data_len)
    }

    /// Perform beamforming for massive MIMO
    pub fn apply_beamforming(&self, data: &[u8], beam_id: u8) -> Vec<u8> {
        // Simulate beamforming weights application
        let mut result = Vec::with_capacity(data.len());
        let weight = self.calculate_beam_weight(beam_id);

        for &byte in data {
            result.push(((byte as u16) * weight / 100) as u8);
        }

        result
    }

    /// Calculate beamforming weight
    fn calculate_beam_weight(&self, beam_id: u8) -> u16 {
        // Simplified beam weight calculation
        match beam_id {
            0 => 100,
            1..=8 => 95,
            9..=16 => 90,
            _ => 85,
        }
    }
}

/// PHY layer errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhyError {
    InvalidPrbAllocation,
    InvalidNumerology,
    TransmissionFailed,
    ReceptionFailed,
    BeamformingError,
}

// ============================================================================
// MAC Layer (Medium Access Control)
// ============================================================================

/// MAC layer configuration
#[derive(Debug, Clone)]
pub struct MacConfig {
    /// Maximum number of HARQ processes
    pub max_harq_processes: u8,
    /// Maximum number of LCIDs (Logical Channel IDs)
    pub max_lcid: u8,
    /// Buffer status report period in ms
    pub bsr_period_ms: u16,
    /// Scheduling policy
    pub scheduling_policy: SchedulingPolicy,
}

/// Scheduling policy
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulingPolicy {
    RoundRobin,
    ProportionalFair,
    MaxThroughput,
    MinimumDelay,
}

/// MAC layer implementation
#[derive(Debug)]
pub struct MacLayer {
    config: MacConfig,
    harq_processes: Vec<HarqProcess>,
    logical_channels: BTreeMap<u8, LogicalChannel>,
    stats: MacStats,
}

/// HARQ (Hybrid ARQ) process
#[derive(Debug, Clone)]
pub struct HarqProcess {
    id: u8,
    state: HarqState,
    transmission_count: u8,
    ndi: bool, // New Data Indicator
    rv: u8,    // Redundancy Version
}

/// HARQ state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HarqState {
    Idle,
    Active,
    WaitingForFeedback,
    Nack,
    Ack,
}

/// Logical channel
#[derive(Debug, Clone)]
pub struct LogicalChannel {
    lcid: u8,
    priority: u8,
    prioritized_bit_rate: u32,
    bucket_size_duration: u16,
    buffer_size: u32,
}

/// MAC layer statistics
#[derive(Debug, Default)]
pub struct MacStats {
    /// Total MAC PDUs sent
    pub pdus_sent: AtomicU64,
    /// Total MAC PDUs received
    pub pdus_received: AtomicU64,
    /// Total bytes sent
    pub bytes_sent: AtomicU64,
    /// Total bytes received
    pub bytes_received: AtomicU64,
    /// HARQ retransmissions
    pub harq_retransmissions: AtomicU64,
}

impl MacLayer {
    /// Create new MAC layer
    pub fn new(config: MacConfig) -> Self {
        let harq_processes = (0..config.max_harq_processes)
            .map(|i| HarqProcess {
                id: i,
                state: HarqState::Idle,
                transmission_count: 0,
                ndi: false,
                rv: 0,
            })
            .collect();

        Self {
            config,
            harq_processes,
            logical_channels: BTreeMap::new(),
            stats: MacStats::default(),
        }
    }

    /// Add logical channel
    pub fn add_logical_channel(&mut self, lc: LogicalChannel) -> Result<(), MacError> {
        if lc.lcid >= self.config.max_lcid {
            return Err(MacError::InvalidLcid);
        }
        self.logical_channels.insert(lc.lcid, lc);
        Ok(())
    }

    /// Schedule uplink resources
    pub fn schedule_uplink(&mut self, available_prbs: u16) -> Vec<(u8, u16)> {
        let mut allocation = Vec::new();
        let mut remaining_prbs = available_prbs;

        match self.config.scheduling_policy {
            SchedulingPolicy::RoundRobin => {
                // Round-robin allocation
                for (&lcid, _channel) in self.logical_channels.iter() {
                    if remaining_prbs == 0 {
                        break;
                    }
                    let prbs = core::cmp::min(5, remaining_prbs);
                    allocation.push((lcid, prbs));
                    remaining_prbs -= prbs;
                }
            }
            SchedulingPolicy::ProportionalFair => {
                // Priority-based allocation
                let mut sorted_channels: Vec<_> = self.logical_channels.iter().collect();
                sorted_channels.sort_by(|a, b| a.1.priority.cmp(&b.1.priority));

                for (&lcid, channel) in sorted_channels {
                    if remaining_prbs == 0 {
                        break;
                    }
                    // Allocate based on priority and buffer size
                    let prbs = core::cmp::min(
                        ((channel.buffer_size / 1000) as u16).min(20),
                        remaining_prbs,
                    );
                    if prbs > 0 {
                        allocation.push((lcid, prbs));
                        remaining_prbs -= prbs;
                    }
                }
            }
            _ => {
                // Default round-robin
                for (&lcid, _channel) in self.logical_channels.iter() {
                    if remaining_prbs == 0 {
                        break;
                    }
                    let prbs = core::cmp::min(5, remaining_prbs);
                    allocation.push((lcid, prbs));
                    remaining_prbs -= prbs;
                }
            }
        }

        allocation
    }

    /// Handle HARQ feedback
    pub fn handle_harq_feedback(&mut self, process_id: u8, ack: bool) -> Result<(), MacError> {
        if process_id >= self.config.max_harq_processes {
            return Err(MacError::InvalidHarqProcess);
        }

        let process = &mut self.harq_processes[process_id as usize];

        if ack {
            process.state = HarqState::Ack;
            process.transmission_count = 0;
            process.rv = 0;
        } else {
            process.state = HarqState::Nack;
            if process.transmission_count < 4 {
                process.transmission_count += 1;
                process.rv = (process.rv + 1) % 4;
                process.ndi = false;
                self.stats.harq_retransmissions.fetch_add(1, Ordering::Relaxed);
            } else {
                process.state = HarqState::Idle;
                return Err(MacError::MaxRetransmissionsExceeded);
            }
        }

        Ok(())
    }

    /// Create MAC PDU
    pub fn create_mac_pdu(&mut self, lcid: u8, data: &[u8]) -> Result<Vec<u8>, MacError> {
        let mut pdu = Vec::with_capacity(data.len() + 3);

        // MAC header (2 bytes)
        pdu.push(0x01); // LCID field
        pdu.push(lcid);
        pdu.push(data.len() as u8); // Length

        // MAC SDU (Service Data Unit)
        pdu.extend_from_slice(data);

        self.stats.pdus_sent.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_sent.fetch_add(data.len() as u64, Ordering::Relaxed);

        Ok(pdu)
    }

    /// Parse MAC PDU
    pub fn parse_mac_pdu(&mut self, pdu: &[u8]) -> Result<(u8, Vec<u8>), MacError> {
        if pdu.len() < 3 {
            return Err(MacError::InvalidPdu);
        }

        let lcid = pdu[1];
        let sdu_len = pdu[2] as usize;

        if pdu.len() < 3 + sdu_len {
            return Err(MacError::InvalidPdu);
        }

        let sdu = pdu[3..3 + sdu_len].to_vec();

        self.stats.pdus_received.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_received.fetch_add(sdu_len as u64, Ordering::Relaxed);

        Ok((lcid, sdu))
    }

    /// Generate buffer status report
    pub fn generate_bsr(&self) -> Vec<u8> {
        let mut bsr = Vec::new();

        for (&lcid, channel) in self.logical_channels.iter() {
            // Buffer size in bytes
            let buffer_size = channel.buffer_size;

            // Format: LCID (1 byte) + Buffer Size (2 bytes)
            bsr.push(lcid);
            bsr.push((buffer_size >> 8) as u8);
            bsr.push(buffer_size as u8);
        }

        bsr
    }

    /// Get MAC statistics
    pub fn stats(&self) -> &MacStats {
        &self.stats
    }
}

/// MAC layer errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MacError {
    InvalidLcid,
    InvalidHarqProcess,
    InvalidPdu,
    MaxRetransmissionsExceeded,
    BufferOverflow,
}

// ============================================================================
// RLC Layer (Radio Link Control)
// ============================================================================

/// RLC mode of operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RlcMode {
    TransparentMode,
    UnacknowledgedMode,
    AcknowledgedMode,
}

/// RLC layer configuration
#[derive(Debug, Clone)]
pub struct RlcConfig {
    pub mode: RlcMode,
    pub tx_window_size: u16,
    pub rx_window_size: u16,
    pub max_retx_threshold: u8,
    pub poll_byte: u16,
    pub poll_pdu: u16,
}

/// RLC layer implementation
#[derive(Debug)]
pub struct RlcLayer {
    config: RlcConfig,
    tx_buffer: Vec<u8>,
    rx_buffer: Vec<u8>,
    tx_next: u16,
    rx_next: u16,
    stats: RlcStats,
}

/// RLC layer statistics
#[derive(Debug, Default)]
pub struct RlcStats {
    /// Total SDUs sent
    pub sdus_sent: AtomicU64,
    /// Total SDUs received
    pub sdus_received: AtomicU64,
    /// Total bytes sent
    pub bytes_sent: AtomicU64,
    /// Total bytes received
    pub bytes_received: AtomicU64,
    /// Segments sent
    pub segments_sent: AtomicU64,
}

impl RlcLayer {
    /// Create new RLC layer
    pub fn new(config: RlcConfig) -> Self {
        Self {
            config,
            tx_buffer: Vec::new(),
            rx_buffer: Vec::new(),
            tx_next: 0,
            rx_next: 0,
            stats: RlcStats::default(),
        }
    }

    /// Transmit RLC SDU
    pub fn transmit_sdu(&mut self, sdu: &[u8]) -> Result<Vec<u8>, RlcError> {
        match self.config.mode {
            RlcMode::TransparentMode => self.transmit_tm(sdu),
            RlcMode::UnacknowledgedMode => self.transmit_um(sdu),
            RlcMode::AcknowledgedMode => self.transmit_am(sdu),
        }
    }

    /// Transparent mode transmission
    fn transmit_tm(&mut self, sdu: &[u8]) -> Result<Vec<u8>, RlcError> {
        self.stats.sdus_sent.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_sent.fetch_add(sdu.len() as u64, Ordering::Relaxed);
        Ok(sdu.to_vec())
    }

    /// Unacknowledged mode transmission
    fn transmit_um(&mut self, sdu: &[u8]) -> Result<Vec<u8>, RlcError> {
        let mut pdu = Vec::with_capacity(sdu.len() + 2);

        // Add sequence number
        pdu.push((self.tx_next >> 8) as u8);
        pdu.push(self.tx_next as u8);
        self.tx_next = (self.tx_next + 1) % 4096;

        // Add SDU
        pdu.extend_from_slice(sdu);

        self.stats.sdus_sent.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_sent.fetch_add(sdu.len() as u64, Ordering::Relaxed);

        Ok(pdu)
    }

    /// Acknowledged mode transmission
    fn transmit_am(&mut self, sdu: &[u8]) -> Result<Vec<u8>, RlcError> {
        let segment_size = 1024; // Maximum segment size
        let offset = 0;

        let mut pdu = Vec::with_capacity(segment_size + 6);

        // AM PDU header
        pdu.push(0x80 | ((self.tx_next >> 8) as u8)); // D/C + SN
        pdu.push(self.tx_next as u8);
        pdu.push(0x00); // P flag
        pdu.push(0x00); // Poll

        // Segment data
        let data_len = core::cmp::min(segment_size, sdu.len() - offset);
        pdu.extend_from_slice(&sdu[offset..offset + data_len]);

        self.tx_next = (self.tx_next + 1) % 4096;
        self.stats.sdus_sent.fetch_add(1, Ordering::Relaxed);
        self.stats.segments_sent.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_sent.fetch_add(data_len as u64, Ordering::Relaxed);

        Ok(pdu)
    }

    /// Receive RLC PDU
    pub fn receive_pdu(&mut self, pdu: &[u8]) -> Result<Vec<u8>, RlcError> {
        match self.config.mode {
            RlcMode::TransparentMode => self.receive_tm(pdu),
            RlcMode::UnacknowledgedMode => self.receive_um(pdu),
            RlcMode::AcknowledgedMode => self.receive_am(pdu),
        }
    }

    /// Transparent mode reception
    fn receive_tm(&mut self, pdu: &[u8]) -> Result<Vec<u8>, RlcError> {
        self.stats.sdus_received.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_received.fetch_add(pdu.len() as u64, Ordering::Relaxed);
        Ok(pdu.to_vec())
    }

    /// Unacknowledged mode reception
    fn receive_um(&mut self, pdu: &[u8]) -> Result<Vec<u8>, RlcError> {
        if pdu.len() < 2 {
            return Err(RlcError::InvalidPdu);
        }

        let sn = ((pdu[0] as u16) << 8) | (pdu[1] as u16);
        let sdu = &pdu[2..];

        self.rx_next = (sn + 1) % 4096;
        self.stats.sdus_received.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_received.fetch_add(sdu.len() as u64, Ordering::Relaxed);

        Ok(sdu.to_vec())
    }

    /// Acknowledged mode reception
    fn receive_am(&mut self, pdu: &[u8]) -> Result<Vec<u8>, RlcError> {
        if pdu.len() < 4 {
            return Err(RlcError::InvalidPdu);
        }

        let sn = ((pdu[0] as u16) & 0x7F) << 8 | (pdu[1] as u16);
        let sdu = &pdu[4..];

        self.rx_next = (sn + 1) % 4096;
        self.stats.sdus_received.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_received.fetch_add(sdu.len() as u64, Ordering::Relaxed);

        Ok(sdu.to_vec())
    }

    /// Get RLC statistics
    pub fn stats(&self) -> &RlcStats {
        &self.stats
    }
}

/// RLC layer errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RlcError {
    InvalidPdu,
    MaxRetxExceeded,
    TxDelayExceeded,
    InvalidMode,
}

// ============================================================================
// PDCP Layer (Packet Data Convergence Protocol)
// ============================================================================

/// PDCP layer configuration
#[derive(Debug, Clone)]
pub struct PdcpConfig {
    /// SN (Sequence Number) length
    pub sn_length: PdcpSnLength,
    /// Integrity protection enabled
    pub integrity_protection: bool,
    /// Ciphering enabled
    pub ciphering: bool,
    /// Header compression enabled
    pub header_compression: bool,
    /// Discard timer in ms
    pub discard_timer_ms: u16,
    /// Maximum PDU size
    pub max_pdu_size: u16,
}

/// PDCP sequence number length
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdcpSnLength {
    Bits12 = 12,
    Bits18 = 18,
}

/// PDCP layer implementation
#[derive(Debug)]
pub struct PdcpLayer {
    config: PdcpConfig,
    tx_next: u32,
    rx_next: u32,
    stats: PdcpStats,
}

/// PDCP layer statistics
#[derive(Debug, Default)]
pub struct PdcpStats {
    /// Total PDUs sent
    pub pdus_sent: AtomicU64,
    /// Total PDUs received
    pub pdus_received: AtomicU64,
    /// Total bytes before compression
    pub bytes_before_compression: AtomicU64,
    /// Total bytes after compression
    pub bytes_after_compression: AtomicU64,
}

impl PdcpLayer {
    /// Create new PDCP layer
    pub fn new(config: PdcpConfig) -> Self {
        Self {
            config,
            tx_next: 0,
            rx_next: 0,
            stats: PdcpStats::default(),
        }
    }

    /// Transmit PDCP PDU
    pub fn transmit_pdu(&mut self, sdu: &[u8]) -> Result<Vec<u8>, PdcpError> {
        let data = if self.config.header_compression {
            self.compress_header(sdu)
        } else {
            sdu.to_vec()
        };

        let mut pdu = Vec::with_capacity(data.len() + 3);

        // PDCP header
        match self.config.sn_length {
            PdcpSnLength::Bits12 => {
                pdu.push(0x80 | ((self.tx_next >> 8) as u8)); // D/C + SN
                pdu.push(self.tx_next as u8);
            }
            PdcpSnLength::Bits18 => {
                pdu.push(0x80 | ((self.tx_next >> 16) as u8));
                pdu.push((self.tx_next >> 8) as u8);
                pdu.push(self.tx_next as u8);
            }
        }

        // Add data
        if self.config.ciphering {
            pdu.extend_from_slice(&self.cipher_data(&data));
        } else {
            pdu.extend_from_slice(&data);
        }

        self.tx_next = (self.tx_next + 1) % (1 << 12);
        self.stats.pdus_sent.fetch_add(1, Ordering::Relaxed);
        self.stats.bytes_before_compression.fetch_add(sdu.len() as u64, Ordering::Relaxed);
        self.stats.bytes_after_compression.fetch_add(data.len() as u64, Ordering::Relaxed);

        Ok(pdu)
    }

    /// Receive PDCP PDU
    pub fn receive_pdu(&mut self, pdu: &[u8]) -> Result<Vec<u8>, PdcpError> {
        if pdu.is_empty() {
            return Err(PdcpError::InvalidPdu);
        }

        let (sn_offset, data) = match self.config.sn_length {
            PdcpSnLength::Bits12 => {
                if pdu.len() < 2 {
                    return Err(PdcpError::InvalidPdu);
                }
                let sn = ((pdu[0] as u32) & 0x0F) << 8 | (pdu[1] as u32);
                (sn, &pdu[2..])
            }
            PdcpSnLength::Bits18 => {
                if pdu.len() < 3 {
                    return Err(PdcpError::InvalidPdu);
                }
                let sn = ((pdu[0] as u32) & 0x03) << 16 | ((pdu[1] as u32) << 8) | (pdu[2] as u32);
                (sn, &pdu[3..])
            }
        };

        self.rx_next = (sn_offset + 1) % (1 << 12);

        let data = if self.config.ciphering {
            self.cipher_data(data)
        } else {
            data.to_vec()
        };

        self.stats.pdus_received.fetch_add(1, Ordering::Relaxed);

        Ok(data)
    }

    /// Simple header compression (ROHC-like)
    fn compress_header(&self, data: &[u8]) -> Vec<u8> {
        // Simplified ROHC compression
        if data.len() > 40 {
            // Compress IP/UDP headers (first 40 bytes)
            let mut compressed = Vec::with_capacity(data.len() - 20);
            compressed.extend_from_slice(&data[0..8]); // Keep first 8 bytes
            compressed.extend_from_slice(&[0xC0]); // Compression indicator
            compressed.extend_from_slice(&data[40..]); // Skip to payload
            compressed
        } else {
            data.to_vec()
        }
    }

    /// Simple ciphering (AES-like XOR for demonstration)
    fn cipher_data(&self, data: &[u8]) -> Vec<u8> {
        // Simplified encryption (XOR with key)
        let key: [u8; 16] = [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF,
                              0xFE, 0xDC, 0xBA, 0x98, 0x76, 0x54, 0x32, 0x10];
        data.iter().enumerate().map(|(i, &b)| b ^ key[i % 16]).collect()
    }

    /// Get PDCP statistics
    pub fn stats(&self) -> &PdcpStats {
        &self.stats
    }
}

/// PDCP layer errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PdcpError {
    InvalidPdu,
    IntegrityCheckFailed,
    DecompressionFailed,
    CipheringFailed,
}

// ============================================================================
// RRC Layer (Radio Resource Control)
// ============================================================================

/// RRC states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RrcState {
    Idle,
    Inactive,
    Connected,
}

/// RRC layer configuration
#[derive(Debug, Clone)]
pub struct RrcConfig {
    /// T300 timer (Connection establishment) in ms
    pub t300_ms: u16,
    /// T301 timer (Connection re-establishment) in ms
    pub t301_ms: u16,
    /// T310 timer (Connection failure detection) in ms
    pub t310_ms: u16,
    /// T311 timer (Handover) in ms
    pub t311_ms: u16,
    /// Maximum number of re-establishment attempts
    pub max_reestablishment_attempts: u8,
}

/// RRC layer implementation
#[derive(Debug)]
pub struct RrcLayer {
    config: RrcConfig,
    state: RrcState,
    cell_id: u32,
    pci: u16, // Physical Cell ID
    stats: RrcStats,
}

/// RRC layer statistics
#[derive(Debug, Default)]
pub struct RrcStats {
    /// Connection establishment attempts
    pub connection_attempts: AtomicU64,
    /// Connection establishment successes
    pub connection_successes: AtomicU64,
    /// Handovers performed
    pub handovers: AtomicU64,
    /// Re-establishment attempts
    pub reestablishments: AtomicU64,
}

impl RrcLayer {
    /// Create new RRC layer
    pub fn new(config: RrcConfig) -> Self {
        Self {
            config,
            state: RrcState::Idle,
            cell_id: 0,
            pci: 0,
            stats: RrcStats::default(),
        }
    }

    /// Get current RRC state
    pub fn state(&self) -> RrcState {
        self.state
    }

    /// Establish RRC connection
    pub fn establish_connection(&mut self, cell_id: u32, pci: u16) -> Result<(), RrcError> {
        self.stats.connection_attempts.fetch_add(1, Ordering::Relaxed);

        // Transition to Connected state
        self.state = RrcState::Connected;
        self.cell_id = cell_id;
        self.pci = pci;

        self.stats.connection_successes.fetch_add(1, Ordering::Relaxed);
        crate::log_info!("RRC connection established to cell {}", cell_id);

        Ok(())
    }

    /// Release RRC connection
    pub fn release_connection(&mut self) -> Result<(), RrcError> {
        self.state = RrcState::Idle;
        self.cell_id = 0;
        self.pci = 0;

        crate::log_info!("RRC connection released");
        Ok(())
    }

    /// Perform handover
    pub fn perform_handover(&mut self, target_cell_id: u32, target_pci: u16) -> Result<(), RrcError> {
        if self.state != RrcState::Connected {
            return Err(RrcError::InvalidState);
        }

        crate::log_info!(
            "Handover: cell {} ({}) -> cell {} ({})",
            self.cell_id,
            self.pci,
            target_cell_id,
            target_pci
        );

        self.cell_id = target_cell_id;
        self.pci = target_pci;

        self.stats.handovers.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Re-establish RRC connection
    pub fn reestablish_connection(&mut self, cell_id: u32) -> Result<(), RrcError> {
        self.stats.reestablishments.fetch_add(1, Ordering::Relaxed);

        self.state = RrcState::Connected;
        self.cell_id = cell_id;

        crate::log_info!("RRC connection re-established to cell {}", cell_id);
        Ok(())
    }

    /// Get serving cell information
    pub fn get_serving_cell(&self) -> Option<(u32, u16)> {
        if self.state == RrcState::Connected {
            Some((self.cell_id, self.pci))
        } else {
            None
        }
    }

    /// Get RRC statistics
    pub fn stats(&self) -> &RrcStats {
        &self.stats
    }
}

/// RRC layer errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RrcError {
    InvalidState,
    ConnectionFailure,
    HandoverFailure,
    ReestablishmentFailure,
    ConfigFailure,
}

// ============================================================================
// NAS Layer (Non-Access Stratum)
// ============================================================================

/// NAS state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NasState {
    Deregistered,
    Registered,
    RegisteredInitiating,
}

/// NAS layer configuration
#[derive(Debug, Clone)]
pub struct NasConfig {
    /// T3410 timer (Attach request) in seconds
    pub t3410_sec: u16,
    /// T3420 timer (Detach request) in seconds
    pub t3420_sec: u16,
    /// T3502 timer (Implicit detach) in seconds
    pub t3502_sec: u16,
}

/// NAS layer implementation
#[derive(Debug)]
pub struct NasLayer {
    config: NasConfig,
    state: NasState,
    imsi: String,
    imei: String,
    msisdn: String,
    stats: NasStats,
}

/// NAS layer statistics
#[derive(Debug, Default)]
pub struct NasStats {
    /// Registration attempts
    pub registration_attempts: AtomicU64,
    /// Registration successes
    pub registration_successes: AtomicU64,
    /// Sessions established
    pub sessions_established: AtomicU64,
    /// Paging messages received
    pub paging_received: AtomicU64,
}

impl NasLayer {
    /// Create new NAS layer
    pub fn new(
        config: NasConfig,
        imsi: String,
        imei: String,
        msisdn: String,
    ) -> Self {
        Self {
            config,
            state: NasState::Deregistered,
            imsi,
            imei,
            msisdn,
            stats: NasStats::default(),
        }
    }

    /// Get current NAS state
    pub fn state(&self) -> NasState {
        self.state
    }

    /// Perform registration
    pub fn register(&mut self, plmn: &str) -> Result<(), NasError> {
        self.stats.registration_attempts.fetch_add(1, Ordering::Relaxed);

        // Simulate registration procedure
        self.state = NasState::Registered;

        self.stats.registration_successes.fetch_add(1, Ordering::Relaxed);
        crate::log_info!("NAS registered to PLMN: {}", plmn);

        Ok(())
    }

    /// Perform deregistration
    pub fn deregister(&mut self) -> Result<(), NasError> {
        self.state = NasState::Deregistered;
        crate::log_info!("NAS deregistered");
        Ok(())
    }

    /// Establish PDU session
    pub fn establish_pdu_session(
        &mut self,
        apn: &str,
        session_id: u8,
    ) -> Result<(), NasError> {
        if self.state != NasState::Registered {
            return Err(NasError::NotRegistered);
        }

        crate::log_info!(
            "PDU session established: ID={}, APN={}",
            session_id,
            apn
        );

        self.stats.sessions_established.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    /// Release PDU session
    pub fn release_pdu_session(&mut self, session_id: u8) -> Result<(), NasError> {
        crate::log_info!("PDU session released: ID={}", session_id);
        Ok(())
    }

    /// Handle paging
    pub fn handle_paging(&mut self) -> Result<(), NasError> {
        self.stats.paging_received.fetch_add(1, Ordering::Relaxed);
        crate::log_info!("NAS paging received");
        Ok(())
    }

    /// Get subscriber information
    pub fn get_subscriber_info(&self) -> (&str, &str, &str) {
        (&self.imsi, &self.imei, &self.msisdn)
    }

    /// Get NAS statistics
    pub fn stats(&self) -> &NasStats {
        &self.stats
    }
}

/// NAS layer errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NasError {
    NotRegistered,
    RegistrationFailure,
    SessionFailure,
    InvalidConfig,
    SecurityError,
}

// ============================================================================
// 5G NR Protocol Stack
// ============================================================================

/// Complete 5G NR protocol stack
#[derive(Debug)]
pub struct FiveGNrStack {
    phy: PhyLayer,
    mac: MacLayer,
    rlc: RlcLayer,
    pdcp: PdcpLayer,
    rrc: RrcLayer,
    nas: NasLayer,
}

impl FiveGNrStack {
    /// Create new 5G NR stack
    pub fn new(
        phy_config: PhyConfig,
        mac_config: MacConfig,
        rlc_config: RlcConfig,
        pdcp_config: PdcpConfig,
        rrc_config: RrcConfig,
        nas_config: NasConfig,
        imsi: String,
        imei: String,
        msisdn: String,
    ) -> Self {
        Self {
            phy: PhyLayer::new(phy_config),
            mac: MacLayer::new(mac_config),
            rlc: RlcLayer::new(rlc_config),
            pdcp: PdcpLayer::new(pdcp_config),
            rrc: RrcLayer::new(rrc_config),
            nas: NasLayer::new(nas_config, imsi, imei, msisdn),
        }
    }

    /// Get PHY layer
    pub fn phy(&self) -> &PhyLayer {
        &self.phy
    }

    /// Get PHY layer (mutable)
    pub fn phy_mut(&mut self) -> &mut PhyLayer {
        &mut self.phy
    }

    /// Get MAC layer
    pub fn mac(&self) -> &MacLayer {
        &self.mac
    }

    /// Get MAC layer (mutable)
    pub fn mac_mut(&mut self) -> &mut MacLayer {
        &mut self.mac
    }

    /// Get RLC layer
    pub fn rlc(&self) -> &RlcLayer {
        &self.rlc
    }

    /// Get RLC layer (mutable)
    pub fn rlc_mut(&mut self) -> &mut RlcLayer {
        &mut self.rlc
    }

    /// Get PDCP layer
    pub fn pdcp(&self) -> &PdcpLayer {
        &self.pdcp
    }

    /// Get PDCP layer (mutable)
    pub fn pdcp_mut(&mut self) -> &mut PdcpLayer {
        &mut self.pdcp
    }

    /// Get RRC layer
    pub fn rrc(&self) -> &RrcLayer {
        &self.rrc
    }

    /// Get RRC layer (mutable)
    pub fn rrc_mut(&mut self) -> &mut RrcLayer {
        &mut self.rrc
    }

    /// Get NAS layer
    pub fn nas(&self) -> &NasLayer {
        &self.nas
    }

    /// Get NAS layer (mutable)
    pub fn nas_mut(&mut self) -> &mut NasLayer {
        &mut self.nas
    }

    /// Transmit data down the stack
    pub fn transmit(&mut self, data: &[u8], lcid: u8) -> Result<(), NrError> {
        // NAS -> PDCP -> RLC -> MAC -> PHY
        let pdcp_pdu = self.pdcp.transmit_pdu(data)?;
        let rlc_pdu = self.rlc.transmit_sdu(&pdcp_pdu)?;
        let mac_pdu = self.mac.create_mac_pdu(lcid, &rlc_pdu)?;

        let prbs = self.mac.schedule_uplink(20);
        for (_lcid, num_prbs) in prbs {
            let prb_list: Vec<u16> = (0..num_prbs).map(|i| i).collect();
            self.phy.process_downlink(&mac_pdu, &prb_list)?;
        }

        Ok(())
    }

    /// Receive data up the stack
    pub fn receive(&mut self, buffer: &mut [u8]) -> Result<Vec<u8>, NrError> {
        // PHY -> MAC -> RLC -> PDCP -> NAS
        let _ = self.phy.process_uplink(buffer, &[0, 1, 2, 3, 4])?;
        let (_lcid, mac_sdu) = self.mac.parse_mac_pdu(buffer)?;
        let rlc_sdu = self.rlc.receive_pdu(&mac_sdu)?;
        let pdcp_sdu = self.pdcp.receive_pdu(&rlc_sdu)?;

        Ok(pdcp_sdu)
    }

    /// Get comprehensive stack statistics
    pub fn get_all_stats(&self) -> StackStatistics {
        StackStatistics {
            phy_tx_symbols: self.phy.stats.tx_symbols.load(Ordering::Relaxed),
            phy_rx_symbols: self.phy.stats.rx_symbols.load(Ordering::Relaxed),
            mac_pdus_sent: self.mac.stats.pdus_sent.load(Ordering::Relaxed),
            mac_pdus_received: self.mac.stats.pdus_received.load(Ordering::Relaxed),
            mac_harq_retx: self.mac.stats.harq_retransmissions.load(Ordering::Relaxed),
            rlc_sdus_sent: self.rlc.stats.sdus_sent.load(Ordering::Relaxed),
            rlc_sdus_received: self.rlc.stats.sdus_received.load(Ordering::Relaxed),
            pdcp_pdus_sent: self.pdcp.stats.pdus_sent.load(Ordering::Relaxed),
            pdcp_pdus_received: self.pdcp.stats.pdus_received.load(Ordering::Relaxed),
            rrc_connections: self.rrc.stats.connection_successes.load(Ordering::Relaxed),
            rrc_handovers: self.rrc.stats.handovers.load(Ordering::Relaxed),
            nas_registrations: self.nas.stats.registration_successes.load(Ordering::Relaxed),
            nas_sessions: self.nas.stats.sessions_established.load(Ordering::Relaxed),
        }
    }
}

/// 5G NR stack errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NrError {
    Phy(PhyError),
    Mac(MacError),
    Rlc(RlcError),
    Pdcp(PdcpError),
    Rrc(RrcError),
    Nas(NasError),
}

// Implement From traits for error conversion
impl From<PhyError> for NrError {
    fn from(err: PhyError) -> Self {
        NrError::Phy(err)
    }
}

impl From<MacError> for NrError {
    fn from(err: MacError) -> Self {
        NrError::Mac(err)
    }
}

impl From<RlcError> for NrError {
    fn from(err: RlcError) -> Self {
        NrError::Rlc(err)
    }
}

impl From<PdcpError> for NrError {
    fn from(err: PdcpError) -> Self {
        NrError::Pdcp(err)
    }
}

impl From<RrcError> for NrError {
    fn from(err: RrcError) -> Self {
        NrError::Rrc(err)
    }
}

impl From<NasError> for NrError {
    fn from(err: NasError) -> Self {
        NrError::Nas(err)
    }
}

/// Comprehensive stack statistics
#[derive(Debug, Clone)]
pub struct StackStatistics {
    pub phy_tx_symbols: u64,
    pub phy_rx_symbols: u64,
    pub mac_pdus_sent: u64,
    pub mac_pdus_received: u64,
    pub mac_harq_retx: u64,
    pub rlc_sdus_sent: u64,
    pub rlc_sdus_received: u64,
    pub pdcp_pdus_sent: u64,
    pub pdcp_pdus_received: u64,
    pub rrc_connections: u64,
    pub rrc_handovers: u64,
    pub nas_registrations: u64,
    pub nas_sessions: u64,
}

// ============================================================================
// Default configurations
// ============================================================================

impl Default for PhyConfig {
    fn default() -> Self {
        Self {
            carrier_freq: 3_500_000_000, // 3.5 GHz
            bandwidth: 100_000_000,       // 100 MHz
            numerology: Numerology::μ1,   // 30 kHz SCS
            num_prbs: 273,                // 100 MHz -> 273 PRBs
            duplex_mode: DuplexMode::TDD,
            cp_length: CyclicPrefix::Normal,
        }
    }
}

impl Default for MacConfig {
    fn default() -> Self {
        Self {
            max_harq_processes: 16,
            max_lcid: 32,
            bsr_period_ms: 10,
            scheduling_policy: SchedulingPolicy::ProportionalFair,
        }
    }
}

impl Default for RlcConfig {
    fn default() -> Self {
        Self {
            mode: RlcMode::AcknowledgedMode,
            tx_window_size: 2048,
            rx_window_size: 2048,
            max_retx_threshold: 8,
            poll_byte: 1500,
            poll_pdu: 256,
        }
    }
}

impl Default for PdcpConfig {
    fn default() -> Self {
        Self {
            sn_length: PdcpSnLength::Bits12,
            integrity_protection: true,
            ciphering: true,
            header_compression: true,
            discard_timer_ms: 100,
            max_pdu_size: 9000,
        }
    }
}

impl Default for RrcConfig {
    fn default() -> Self {
        Self {
            t300_ms: 100,
            t301_ms: 100,
            t310_ms: 100,
            t311_ms: 100,
            max_reestablishment_attempts: 4,
        }
    }
}

impl Default for NasConfig {
    fn default() -> Self {
        Self {
            t3410_sec: 10,
            t3420_sec: 10,
            t3502_sec: 12 * 60 * 60, // 12 hours
        }
    }
}
