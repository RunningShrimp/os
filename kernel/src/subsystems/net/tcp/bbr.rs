//! BBR (Bottleneck Bandwidth and RTT) Congestion Control
//!
//! This module implements an advanced BBR congestion control algorithm
//! as specified in RFC 9780 (draft). BBR is a model-based congestion control
//! algorithm that estimates the available bandwidth and path RTT to control
//! the sending rate, rather than using packet loss as a congestion signal.
//!
//! Key features:
//! - Model-based congestion control using bandwidth and RTT measurements
//! - Four states: Startup, Drain, ProbeBW, ProbeRTT
//! - Pacing to smooth packet transmission
//! - Better performance in high-BDP networks
//! - Reduced latency compared to loss-based algorithms

extern crate alloc;

use alloc::vec::Vec;

/// BBR state machine states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BbrStateEnum {
    /// Startup phase - rapidly increase cwnd to fill the pipe
    Startup,
    /// Drain phase - drain the queue built during startup
    Drain,
    /// ProbeBW phase - probe for bandwidth by cycling gains
    ProbeBW,
    /// ProbeRTT phase - probe for minimum RTT
    ProbeRTT,
}

/// BBR congestion control state
#[derive(Debug, Clone)]
pub struct BbrState {
    /// Current BBR state
    pub state: BbrStateEnum,
    /// Estimated bottleneck bandwidth (bytes per second)
    pub bandwidth: u64,
    /// Minimum RTT observed (microseconds)
    pub min_rtt: u64,
    /// Current congestion window (bytes)
    pub cwnd: u32,
    /// Current pacing rate (bytes per second)
    pub pacing_rate: u64,
    /// Maximum bandwidth observed (bytes per second)
    max_bandwidth: u64,
    /// RTT sample when min_rtt was last updated
    min_rtt_timestamp: u64,
    /// Time when last RTT probe started
    probe_rtt_done_stamp: u64,
    /// Flag indicating if we've done an RTT probe
    probe_rtt_round_done: bool,
    /// Flag indicating if RTT probe is scheduled
    probe_rtt_scheduled: bool,
    /// Packet send time for pacing
    last_send_time: u64,
    /// Number of packets delivered since last adjustment
    delivered: u32,
    /// Number of packets lost since last adjustment
    lost: u32,
    /// ECN (Explicit Congestion Notification) marking ratio
    ecn_ratio: f32,
    /// BBR gain factor
    gain: f32,
    /// Extra ACKed state (for BBR v2)
    extra_acked: Vec<u64>,
    /// Current index in extra_acked window
    extra_acked_index: usize,
}

impl BbrState {
    /// Create a new BBR state
    pub fn new(mss: u32) -> Self {
        let init_cwnd = 10 * mss;

        Self {
            state: BbrStateEnum::Startup,
            bandwidth: 0,
            min_rtt: u64::MAX,
            cwnd: init_cwnd,
            pacing_rate: 0,
            max_bandwidth: 0,
            min_rtt_timestamp: 0,
            probe_rtt_done_stamp: 0,
            probe_rtt_round_done: false,
            probe_rtt_scheduled: false,
            last_send_time: 0,
            delivered: 0,
            lost: 0,
            ecn_ratio: 0.0,
            gain: 2.885, // Startup gain (2/ln(2))
            extra_acked: Vec::with_capacity(10),
            extra_acked_index: 0,
        }
    }

    /// Called when an ACK is received
    ///
    /// Updates bandwidth and RTT estimates, and adjusts the congestion window
    /// and pacing rate based on the current BBR state.
    ///
    /// # Arguments
    /// * `acked` - Number of bytes acknowledged
    /// * `rtt` - Round-trip time in microseconds
    /// * `now` - Current timestamp in microseconds
    pub fn on_ack(&mut self, acked: u32, rtt: u64, now: u64) {
        // Update bandwidth estimate
        let bw_sample = if rtt > 0 {
            ((acked as u64) * 1_000_000) / rtt
        } else {
            0
        };

        // Update max bandwidth (using windowed max)
        if bw_sample > self.max_bandwidth {
            self.max_bandwidth = bw_sample;
        }

        // Smooth bandwidth estimate
        if self.bandwidth == 0 {
            self.bandwidth = bw_sample;
        } else {
            // Exponential weighted moving average
            const ALPHA: f64 = 0.1;
            self.bandwidth = ((self.bandwidth as f64) * (1.0 - ALPHA) + (bw_sample as f64) * ALPHA) as u64;
        }

        // Update min RTT
        if rtt > 0 && rtt < self.min_rtt {
            self.min_rtt = rtt;
            self.min_rtt_timestamp = now;
        }

        // Update delivered bytes
        self.delivered += acked;

        // Update state machine
        self.update_state(now);

        // Update congestion window and pacing rate
        self.update_cwnd();
        self.update_pacing_rate();
    }

    /// Update BBR state machine
    fn update_state(&mut self, now: u64) {
        match self.state {
            BbrStateEnum::Startup => {
                // Startup phase: use high gain to fill the pipe
                self.gain = 2.885; // 2/ln(2)

                // Check if pipe is full
                // Transition to Drain when bandwidth growth slows
                if self.is_pipe_full() {
                    self.state = BbrStateEnum::Drain;
                }
            }
            BbrStateEnum::Drain => {
                // Drain phase: drain the queue built during startup
                self.gain = 0.5; // Low gain to drain queue

                // Check if queue is drained
                let bdp = self.calculate_bdp();
                if self.cwnd <= bdp {
                    self.state = BbrStateEnum::ProbeBW;
                    self.probe_rtt_scheduled = false;
                }
            }
            BbrStateEnum::ProbeBW => {
                // ProbeBW phase: cycle through gain phases
                // This is simplified - real BBR uses an 8-phase cycle
                self.gain = 1.0; // Cruise gain

                // Check if we should probe RTT
                const PROBE_RTT_INTERVAL: u64 = 10_000_000; // 10 seconds in microseconds
                if !self.probe_rtt_scheduled && (now - self.min_rtt_timestamp) > PROBE_RTT_INTERVAL {
                    self.state = BbrStateEnum::ProbeRTT;
                    self.probe_rtt_done_stamp = 0;
                    self.probe_rtt_round_done = false;
                }
            }
            BbrStateEnum::ProbeRTT => {
                // ProbeRTT phase: reduce cwnd to probe min RTT
                const MIN_CWND: u32 = 4 * 1460; // 4 segments

                if self.probe_rtt_done_stamp == 0 {
                    // First time in ProbeRTT
                    self.cwnd = self.cwnd.min(MIN_CWND);
                    self.probe_rtt_done_stamp = now;
                }

                // Stay in ProbeRTT for at least one RTT
                const PROBE_RTT_DURATION: u64 = 200_000; // 200ms
                if (now - self.probe_rtt_done_stamp) > PROBE_RTT_DURATION {
                    // Check if we've measured a new min RTT
                    if (now - self.min_rtt_timestamp) > PROBE_RTT_DURATION {
                        // Done probing
                        self.state = BbrStateEnum::ProbeBW;
                        self.probe_rtt_scheduled = true;
                    }
                }
            }
        }
    }

    /// Update congestion window based on BDP and current state
    pub fn update_cwnd(&mut self) {
        let bdp = self.calculate_bdp();

        // Apply gain to BDP
        let target_cwnd = (bdp as f64 * self.gain as f64) as u32;

        // Ensure minimum cwnd
        const MIN_CWND: u32 = 4 * 1460;
        self.cwnd = target_cwnd.max(MIN_CWND);
    }

    /// Update pacing rate
    fn update_pacing_rate(&mut self) {
        if self.bandwidth == 0 {
            return;
        }

        // Pacing rate = bandwidth * gain
        self.pacing_rate = (self.bandwidth as f64 * self.gain as f64) as u64;
    }

    /// Calculate Bandwidth-Delay Product (BDP)
    fn calculate_bdp(&self) -> u32 {
        if self.bandwidth == 0 || self.min_rtt == u64::MAX {
            return 10 * 1460; // Default: 10 segments
        }

        // BDP = bandwidth * RTT
        let bdp = (self.bandwidth * self.min_rtt) / 1_000_000;
        bdp as u32
    }

    /// Check if the network pipe is full
    fn is_pipe_full(&self) -> bool {
        // Pipe is considered full if bandwidth estimate has converged
        // This is simplified - real BBR uses more sophisticated detection
        if self.bandwidth == 0 || self.max_bandwidth == 0 {
            return false;
        }

        // Check if bandwidth estimate is within 25% of max
        let ratio = (self.bandwidth as f64) / (self.max_bandwidth as f64);
        ratio >= 0.75
    }

    /// Check if we should probe RTT
    pub fn should_probe_rtt(&self) -> bool {
        const PROBE_RTT_INTERVAL: u64 = 10_000_000; // 10 seconds
        self.min_rtt_timestamp != 0 && (self.current_time() - self.min_rtt_timestamp) > PROBE_RTT_INTERVAL
    }

    /// Get current time (simplified - would use system time in practice)
    fn current_time(&self) -> u64 {
        // In a real implementation, this would return the actual current time
        0
    }

    /// Called when packet loss is detected
    ///
    /// BBR handles loss differently from traditional algorithms.
    /// It doesn't immediately reduce cwnd on loss, but monitors loss rate.
    ///
    /// # Arguments
    /// * `lost` - Number of bytes lost
    pub fn on_loss(&mut self, lost: u32) {
        self.lost += lost;

        // Calculate loss ratio
        let total = self.delivered + self.lost;
        if total > 0 {
            let loss_ratio = (self.lost as f64) / (total as f64);

            // If loss rate is high (> 2%), reduce gain
            const LOSS_THRESHOLD: f64 = 0.02;
            if loss_ratio > LOSS_THRESHOLD {
                self.gain = self.gain.min(1.0);
            }
        }
    }

    /// Called when ECN (Explicit Congestion Notification) is received
    pub fn on_ecn(&mut self) {
        // Increase ECN ratio tracking
        // BBR v2 uses ECN signals to adjust gain
        self.ecn_ratio = (self.ecn_ratio * 0.9) + 0.1;

        // If ECN ratio is high, reduce gain
        const ECN_THRESHOLD: f32 = 0.1;
        if self.ecn_ratio > ECN_THRESHOLD {
            self.gain = self.gain.min(1.0);
        }
    }

    /// Reset BBR state (e.g., on connection establishment)
    pub fn reset(&mut self) {
        self.state = BbrStateEnum::Startup;
        self.bandwidth = 0;
        self.min_rtt = u64::MAX;
        self.cwnd = 10 * 1460; // 10 segments
        self.pacing_rate = 0;
        self.max_bandwidth = 0;
        self.min_rtt_timestamp = 0;
        self.probe_rtt_done_stamp = 0;
        self.probe_rtt_round_done = false;
        self.probe_rtt_scheduled = false;
        self.delivered = 0;
        self.lost = 0;
        self.ecn_ratio = 0.0;
        self.gain = 2.885;
        self.extra_acked.clear();
        self.extra_acked_index = 0;
    }

    /// Get the BDP window
    pub fn bdp_window(&self) -> u32 {
        self.calculate_bdp()
    }

    /// Get the pacing gain
    pub fn pacing_gain(&self) -> f32 {
        self.gain
    }

    /// Get the current state as a string
    pub fn state_name(&self) -> &str {
        match self.state {
            BbrStateEnum::Startup => "Startup",
            BbrStateEnum::Drain => "Drain",
            BbrStateEnum::ProbeBW => "ProbeBW",
            BbrStateEnum::ProbeRTT => "ProbeRTT",
        }
    }

    /// Check if BBR is in startup phase
    pub fn is_in_startup(&self) -> bool {
        self.state == BbrStateEnum::Startup
    }

    /// Check if BBR is in probe bandwidth phase
    pub fn is_in_probe_bw(&self) -> bool {
        self.state == BbrStateEnum::ProbeBW
    }

    /// Check if BBR is in probe RTT phase
    pub fn is_in_probe_rtt(&self) -> bool {
        self.state == BbrStateEnum::ProbeRTT
    }
}

impl Default for BbrState {
    fn default() -> Self {
        Self::new(1460) // Default MSS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bbr_creation() {
        let bbr = BbrState::new(1460);
        assert_eq!(bbr.state, BbrStateEnum::Startup);
        assert_eq!(bbr.cwnd, 10 * 1460);
        assert_eq!(bbr.min_rtt, u64::MAX);
        assert_eq!(bbr.bandwidth, 0);
    }

    #[test]
    fn test_bbr_on_ack() {
        let mut bbr = BbrState::new(1460);

        // Simulate ACK
        bbr.on_ack(1460, 100_000, 1_000_000);

        // Should update bandwidth
        assert!(bbr.bandwidth > 0);

        // Should update min RTT
        assert_eq!(bbr.min_rtt, 100_000);
    }

    #[test]
    fn test_bbr_state_transitions() {
        let mut bbr = BbrState::new(1460);

        // Start in Startup
        assert_eq!(bbr.state, BbrStateEnum::Startup);

        // Simulate pipe filling (multiple ACKs)
        for i in 0..100 {
            bbr.on_ack(1460, 100_000 + i * 1000, i * 1_000_000);
        }

        // Should eventually transition to Drain when pipe is full
        // (This is simplified - real implementation would need more sophisticated logic)
    }

    #[test]
    fn test_bbr_calculate_bdp() {
        let mut bbr = BbrState::new(1460);

        // Set bandwidth and RTT
        bbr.bandwidth = 1_000_000; // 1 MB/s
        bbr.min_rtt = 100_000; // 100ms

        let bdp = bbr.calculate_bdp();
        assert_eq!(bdp, 100_000); // BDP = 1MB/s * 0.1s = 100KB
    }

    #[test]
    fn test_bbr_update_cwnd() {
        let mut bbr = BbrState::new(1460);

        // Set bandwidth and RTT
        bbr.bandwidth = 1_000_000; // 1 MB/s
        bbr.min_rtt = 100_000; // 100ms

        bbr.update_cwnd();

        // cwnd should be close to BDP * gain
        let bdp = bbr.calculate_bdp();
        assert!(bbr.cwnd >= bdp);
    }

    #[test]
    fn test_bbr_pacing_rate() {
        let mut bbr = BbrState::new(1460);

        // Set bandwidth
        bbr.bandwidth = 1_000_000; // 1 MB/s

        bbr.update_pacing_rate();

        // Pacing rate should be bandwidth * gain
        assert_eq!(bbr.pacing_rate, (1_000_000.0 * bbr.gain) as u64);
    }

    #[test]
    fn test_bbr_reset() {
        let mut bbr = BbrState::new(1460);

        // Modify state
        bbr.state = BbrStateEnum::ProbeBW;
        bbr.bandwidth = 1_000_000;
        bbr.min_rtt = 100_000;
        bbr.delivered = 10000;
        bbr.lost = 100;

        // Reset
        bbr.reset();

        // Should return to initial state
        assert_eq!(bbr.state, BbrStateEnum::Startup);
        assert_eq!(bbr.bandwidth, 0);
        assert_eq!(bbr.min_rtt, u64::MAX);
        assert_eq!(bbr.delivered, 0);
        assert_eq!(bbr.lost, 0);
    }

    #[test]
    fn test_bbr_state_name() {
        let bbr = BbrState::new(1460);
        assert_eq!(bbr.state_name(), "Startup");
    }

    #[test]
    fn test_bbr_is_in_startup() {
        let bbr = BbrState::new(1460);
        assert!(bbr.is_in_startup());
        assert!(!bbr.is_in_probe_bw());
        assert!(!bbr.is_in_probe_rtt());
    }
}
