//! RACK (Recent Acknowledgment) - TLP (Tail Loss Probe) Loss Recovery
//!
//! This module implements the RACK loss detection algorithm combined with
//! TLP (Tail Loss Probe) for efficient loss recovery in TCP.
//!
//! Key features:
//! - RACK: Uses time-based loss detection instead of duplicate ACKs
//! - TLP: Sends probe packets to detect tail losses without waiting for RTO
//! - Faster loss recovery in high-BDP networks
//! - Reduced spurious retransmissions
//! - Better interaction with pacing and SACK
//!
//! References:
//! - RFC 8985: RACK: A Time-Based Fast Loss Detection Algorithm
//! - RFC 8985: TLP: Tail Loss Probe Algorithm

extern crate alloc;

use alloc::vec::Vec;

/// RACK-TLP state for loss recovery
#[derive(Debug, Clone)]
pub struct RackState {
    /// RACK sent time (timestamp of the most recent ACKed packet)
    pub rack_sent_time: u64,
    /// RACK RTT estimate (microseconds)
    pub rack_rtt_us: u32,
    /// TLP high sequence number (highest sequence sent before TLP)
    pub tlp_high_seq: u32,
    /// Reordering window (microseconds) - threshold for detecting reordering
    pub reo_wnd: u32,
    /// Number of TLP packets sent
    tlp_count: u32,
    /// Maximum TLP retries
    max_tlp_retries: u32,
    /// TLP interval (microseconds)
    tlp_interval: u32,
    /// Time when last TLP was sent
    last_tlp_time: u64,
    /// Time when last packet was sent
    last_send_time: u64,
    /// Minimum RTT observed
    min_rtt: u32,
    /// RTT variance (for calculating RACK timeout)
    rtt_var: u32,
    /// RACK timeout (microseconds)
    rack_timeout: u32,
    /// Sequence numbers of detected lost packets
    lost_packets: Vec<u32>,
    /// Sequence numbers of packets sent but not yet ACKed
    outstanding_packets: Vec<(u32, u64)>, // (seq, send_time)
    /// Next expected sequence number
    next_expected: u32,
    /// Highest sequence number sent
    high_seq: u32,
    /// Number of DUPACKs received
    dup_acks: u32,
    /// Duplicate ACK threshold for fast retransmit
    dup_thresh: u32,
    /// RACK enabled flag
    enabled: bool,
    /// TLP enabled flag
    tlp_enabled: bool,
    /// Recovery point (sequence number)
    recovery_point: Option<u32>,
}

impl RackState {
    /// Create a new RACK state
    pub fn new() -> Self {
        Self {
            rack_sent_time: 0,
            rack_rtt_us: 0,
            tlp_high_seq: 0,
            reo_wnd: 1_000, // 1ms default reordering window
            tlp_count: 0,
            max_tlp_retries: 2,
            tlp_interval: 0, // Will be calculated based on RTT
            last_tlp_time: 0,
            last_send_time: 0,
            min_rtt: u32::MAX,
            rtt_var: 0,
            rack_timeout: 0,
            lost_packets: Vec::new(),
            outstanding_packets: Vec::new(),
            next_expected: 0,
            high_seq: 0,
            dup_acks: 0,
            dup_thresh: 3,
            enabled: true,
            tlp_enabled: true,
            recovery_point: None,
        }
    }

    /// Called when a packet is sent
    ///
    /// Records the packet in the outstanding list and updates timing.
    ///
    /// # Arguments
    /// * `seq` - Sequence number of the sent packet
    /// * `now` - Current timestamp in microseconds
    pub fn on_send(&mut self, seq: u32, now: u64) {
        // Record the packet as outstanding
        self.outstanding_packets.push((seq, now));

        // Update high sequence number
        if seq > self.high_seq {
            self.high_seq = seq;
        }

        // Update last send time
        self.last_send_time = now;
    }

    /// Called when an ACK is received
    ///
    /// Updates RACK state and potentially detects packet loss.
    ///
    /// # Arguments
    /// * `ack` - Acknowledgment number
    /// * `now` - Current timestamp in microseconds
    /// * `rtt` - RTT sample for this ACK (if available)
    pub fn on_ack(&mut self, ack: u32, now: u64, rtt: Option<u32>) {
        // Update RTT estimates
        if let Some(rtt_sample) = rtt {
            self.update_rtt(rtt_sample);
        }

        // Remove acknowledged packets from outstanding list
        self.outstanding_packets.retain(|(seq, _)| *seq >= ack);

        // Update next expected sequence
        if ack > self.next_expected {
            self.next_expected = ack;
            self.dup_acks = 0;
        }

        // Update RACK sent time
        if !self.outstanding_packets.is_empty() {
            // Find the lowest outstanding sequence
            let lowest_outstanding = self.outstanding_packets[0].0;
            if lowest_outstanding > ack {
                // RACK sent time is the send time of the packet just below ACK
                if let Some(idx) = self.outstanding_packets.iter().position(|(seq, _)| *seq == ack) {
                    if idx > 0 {
                        self.rack_sent_time = self.outstanding_packets[idx - 1].1;
                    }
                }
            }
        }

        // Check for loss using RACK
        if self.enabled {
            self.detect_loss_rack(now);
        }
    }

    /// Called when a duplicate ACK is received
    ///
    /// # Arguments
    /// * `dup_ack` - Duplicate ACK number
    /// * `now` - Current timestamp in microseconds
    pub fn on_dup_ack(&mut self, dup_ack: u32, now: u64) {
        self.dup_acks += 1;

        // Traditional fast retransmit threshold
        if self.dup_acks >= self.dup_thresh {
            // Mark packets up to dup_ack as lost
            self.mark_lost(dup_ack);
            self.enter_recovery(dup_ack);
        }

        // RACK-based detection on dup ACKs
        if self.enabled {
            self.detect_loss_rack(now);
        }
    }

    /// Detect packet loss using RACK algorithm
    fn detect_loss_rack(&mut self, now: u64) {
        if self.outstanding_packets.is_empty() {
            return;
        }

        // Calculate RACK timeout
        self.calculate_rack_timeout();

        // Check each outstanding packet
        let mut lost = Vec::new();

        for &(seq, send_time) in &self.outstanding_packets {
            // Skip if send_time is 0 (shouldn't happen)
            if send_time == 0 {
                continue;
            }

            // Time since this packet was sent
            let time_since_send = now.saturating_sub(send_time);

            // RACK condition: packet is lost if:
            // 1. It was sent before RACK sent time
            // 2. AND RACK timeout has elapsed since it was sent
            if send_time < self.rack_sent_time && time_since_send > (self.rack_timeout as u64) {
                lost.push(seq);
            }
        }

        // Mark lost packets
        for seq in lost {
            self.mark_lost(seq);
        }
    }

    /// Update RTT estimates
    fn update_rtt(&mut self, rtt_sample: u32) {
        // Update minimum RTT
        if rtt_sample < self.min_rtt {
            self.min_rtt = rtt_sample;
        }

        // Update average RTT using Jacobson/Karels algorithm
        if self.rack_rtt_us == 0 {
            // First RTT sample
            self.rack_rtt_us = rtt_sample;
            self.rtt_var = rtt_sample / 2;
        } else {
            // Update RTT estimate
            let alpha = 8;
            let beta = 4;

            // rtt_var = (3 * rtt_var + |rtt_sample - rtt_est|) / 4
            self.rtt_var = ((3 * self.rtt_var as i32
                + (rtt_sample as i32 - self.rack_rtt_us as i32).abs())
                / beta) as u32;

            // rtt_est = (7 * rtt_est + rtt_sample) / 8
            self.rack_rtt_us = ((7 * self.rack_rtt_us as u32 + rtt_sample) / alpha) as u32;
        }

        // Update TLP interval based on RTT
        self.calculate_tlp_interval();
    }

    /// Calculate RACK timeout
    fn calculate_rack_timeout(&mut self) {
        // RACK timeout = SRTT + 4 * RTTVAR + reordering window
        self.rack_timeout = self
            .rack_rtt_us
            .saturating_add(4 * self.rtt_var)
            .saturating_add(self.reo_wnd);
    }

    /// Calculate TLP interval
    fn calculate_tlp_interval(&mut self) {
        if self.rack_rtt_us == 0 {
            // Default: 10ms
            self.tlp_interval = 10_000;
        } else {
            // TLP interval = min(2 * SRTT, 200ms)
            self.tlp_interval = (2 * self.rack_rtt_us).min(200_000);
        }
    }

    /// Mark a packet as lost
    fn mark_lost(&mut self, seq: u32) {
        // Add to lost packets if not already present
        if !self.lost_packets.contains(&seq) {
            self.lost_packets.push(seq);
        }

        // Remove from outstanding packets
        self.outstanding_packets.retain(|(s, _)| *s != seq);
    }

    /// Enter recovery phase
    fn enter_recovery(&mut self, seq: u32) {
        self.recovery_point = Some(seq);
        self.dup_acks = 0;
    }

    /// Exit recovery phase
    pub fn exit_recovery(&mut self) {
        self.recovery_point = None;
        self.lost_packets.clear();
    }

    /// Schedule a TLP transmission
    ///
    /// Returns true if TLP should be sent
    pub fn schedule_tlp(&mut self, now: u64) -> bool {
        if !self.tlp_enabled {
            return false;
        }

        // Don't send TLP if in recovery
        if self.recovery_point.is_some() {
            return false;
        }

        // Don't send TLP if we've reached max retries
        if self.tlp_count >= self.max_tlp_retries {
            return false;
        }

        // Don't send TLP if too soon since last send
        if now.saturating_sub(self.last_send_time) < (self.tlp_interval as u64) {
            return false;
        }

        // Check if we have outstanding packets
        if self.outstanding_packets.is_empty() {
            return false;
        }

        // Send TLP
        self.tlp_high_seq = self.high_seq;
        self.tlp_count += 1;
        self.last_tlp_time = now;

        true
    }

    /// Detect lost packets
    ///
    /// Returns a list of sequence numbers that should be retransmitted
    pub fn detect_loss(&mut self) -> Vec<u32> {
        let lost = self.lost_packets.clone();
        self.lost_packets.clear();
        lost
    }

    /// Check if we're in recovery phase
    pub fn is_in_recovery(&self) -> bool {
        self.recovery_point.is_some()
    }

    /// Check if TLP should be scheduled
    pub fn should_schedule_tlp(&self, now: u64) -> bool {
        if !self.tlp_enabled {
            return false;
        }

        if self.recovery_point.is_some() {
            return false;
        }

        if self.tlp_count >= self.max_tlp_retries {
            return false;
        }

        if self.outstanding_packets.is_empty() {
            return false;
        }

        // Check if TLP timer has expired
        now.saturating_sub(self.last_send_time) >= (self.tlp_interval as u64)
    }

    /// Get the RACK timeout value
    pub fn rack_timeout(&self) -> u32 {
        self.rack_timeout
    }

    /// Get the current SRTT (Smoothed Round Trip Time)
    pub fn srtt(&self) -> u32 {
        self.rack_rtt_us
    }

    /// Get the RTT variance
    pub fn rtt_var(&self) -> u32 {
        self.rtt_var
    }

    /// Get the number of outstanding packets
    pub fn outstanding_count(&self) -> usize {
        self.outstanding_packets.len()
    }

    /// Reset RACK state (e.g., after recovery)
    pub fn reset(&mut self) {
        self.rack_sent_time = 0;
        self.tlp_count = 0;
        self.last_tlp_time = 0;
        self.lost_packets.clear();
        self.outstanding_packets.clear();
        self.recovery_point = None;
        self.dup_acks = 0;
    }

    /// Full reset (e.g., on connection establishment)
    pub fn full_reset(&mut self) {
        self.reset();
        self.rack_rtt_us = 0;
        self.min_rtt = u32::MAX;
        self.rtt_var = 0;
        self.rack_timeout = 0;
        self.next_expected = 0;
        self.high_seq = 0;
    }

    /// Enable or disable RACK
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Enable or disable TLP
    pub fn set_tlp_enabled(&mut self, enabled: bool) {
        self.tlp_enabled = enabled;
    }

    /// Set the reordering window
    pub fn set_reorder_window(&mut self, reo_wnd: u32) {
        self.reo_wnd = reo_wnd;
    }

    /// Set the duplicate ACK threshold
    pub fn set_dup_thresh(&mut self, thresh: u32) {
        self.dup_thresh = thresh;
    }
}

impl Default for RackState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rack_creation() {
        let rack = RackState::new();
        assert_eq!(rack.rack_sent_time, 0);
        assert_eq!(rack.rack_rtt_us, 0);
        assert_eq!(rack.reo_wnd, 1_000);
        assert!(rack.enabled);
        assert!(rack.tlp_enabled);
    }

    #[test]
    fn test_rack_on_send() {
        let mut rack = RackState::new();
        let now = 1_000_000;

        rack.on_send(1000, now);

        assert_eq!(rack.outstanding_count(), 1);
        assert_eq!(rack.high_seq, 1000);
        assert_eq!(rack.last_send_time, now);
    }

    #[test]
    fn test_rack_on_ack() {
        let mut rack = RackState::new();
        let now = 1_000_000;

        // Send packets
        rack.on_send(1000, now);
        rack.on_send(2000, now + 1000);

        // ACK packet 1000
        rack.on_ack(1000, now + 100_000, Some(100_000));

        // Should update RTT
        assert_eq!(rack.rack_rtt_us, 100_000);

        // Should remove packet 1000 from outstanding
        assert_eq!(rack.outstanding_count(), 1);
    }

    #[test]
    fn test_rack_update_rtt() {
        let mut rack = RackState::new();

        // First RTT sample
        rack.update_rtt(100_000);

        assert_eq!(rack.rack_rtt_us, 100_000);
        assert_eq!(rack.min_rtt, 100_000);

        // Second RTT sample
        rack.update_rtt(120_000);

        // Should update SRTT and RTTVAR
        assert!(rack.rack_rtt_us > 100_000);
        assert!(rack.rtt_var > 0);
    }

    #[test]
    fn test_rack_calculate_timeout() {
        let mut rack = RackState::new();

        // Set RTT values
        rack.rack_rtt_us = 100_000;
        rack.rtt_var = 10_000;
        rack.reo_wnd = 1_000;

        rack.calculate_rack_timeout();

        // Expected: 100000 + 4*10000 + 1000 = 141000
        assert_eq!(rack.rack_timeout, 141_000);
    }

    #[test]
    fn test_rack_schedule_tlp() {
        let mut rack = RackState::new();
        let now = 1_000_000;

        // Set up state
        rack.rack_rtt_us = 100_000;
        rack.calculate_tlp_interval();

        // Send a packet
        rack.on_send(1000, now);

        // Should schedule TLP after interval
        let should_send = rack.schedule_tlp(now + 200_000);
        assert!(should_send);
        assert_eq!(rack.tlp_count, 1);
    }

    #[test]
    fn test_rack_no_tlp_in_recovery() {
        let mut rack = RackState::new();
        let now = 1_000_000;

        // Enter recovery
        rack.recovery_point = Some(1000);

        // Should not schedule TLP
        let should_send = rack.schedule_tlp(now + 200_000);
        assert!(!should_send);
    }

    #[test]
    fn test_rack_dup_ack() {
        let mut rack = RackState::new();
        let now = 1_000_000;

        // Receive duplicate ACKs
        for _ in 0..3 {
            rack.on_dup_ack(1000, now);
        }

        // Should mark packet as lost
        assert!(rack.lost_packets.contains(&1000));
        assert!(rack.is_in_recovery());
    }

    #[test]
    fn test_rack_reset() {
        let mut rack = RackState::new();

        // Modify state
        rack.rack_sent_time = 1000;
        rack.tlp_count = 2;
        rack.lost_packets.push(100);
        rack.outstanding_packets.push((1000, 1000));
        rack.recovery_point = Some(1000);

        // Reset
        rack.reset();

        assert_eq!(rack.rack_sent_time, 0);
        assert_eq!(rack.tlp_count, 0);
        assert!(rack.lost_packets.is_empty());
        assert!(rack.outstanding_packets.is_empty());
        assert!(rack.recovery_point.is_none());
    }

    #[test]
    fn test_rack_full_reset() {
        let mut rack = RackState::new();

        // Modify state
        rack.rack_rtt_us = 100_000;
        rack.min_rtt = 100_000;
        rack.rtt_var = 10_000;
        rack.next_expected = 5000;
        rack.high_seq = 5000;

        // Full reset
        rack.full_reset();

        assert_eq!(rack.rack_rtt_us, 0);
        assert_eq!(rack.min_rtt, u32::MAX);
        assert_eq!(rack.rtt_var, 0);
        assert_eq!(rack.next_expected, 0);
        assert_eq!(rack.high_seq, 0);
    }

    #[test]
    fn test_rack_detect_loss() {
        let mut rack = RackState::new();

        // Mark packets as lost
        rack.mark_lost(1000);
        rack.mark_lost(2000);

        // Detect loss should return and clear
        let lost = rack.detect_loss();
        assert_eq!(lost.len(), 2);
        assert!(lost.contains(&1000));
        assert!(lost.contains(&2000));

        // Should be cleared
        assert!(rack.lost_packets.is_empty());
    }

    #[test]
    fn test_rack_configuration() {
        let mut rack = RackState::new();

        rack.set_enabled(false);
        assert!(!rack.enabled);

        rack.set_tlp_enabled(false);
        assert!(!rack.tlp_enabled);

        rack.set_reorder_window(5_000);
        assert_eq!(rack.reo_wnd, 5_000);

        rack.set_dup_thresh(2);
        assert_eq!(rack.dup_thresh, 2);
    }
}
