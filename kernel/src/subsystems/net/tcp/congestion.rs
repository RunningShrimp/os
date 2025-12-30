//! TCP congestion control algorithms
//!
//! This module implements various TCP congestion control algorithms as specified in:
//! - RFC 5681: TCP Congestion Control (Reno)
//! - RFC 8312: CUBIC for Fast Long-Distance Networks
//! - RFC 9780: BBR v2 Congestion Control

extern crate alloc;

use alloc::boxed::Box;

/// Congestion control trait
///
/// All congestion control algorithms must implement this trait.
pub trait CongestionControl {
    /// Called when an ACK is received
    ///
    /// # Arguments
    /// * `acked` - Number of bytes acknowledged
    /// * `rtt` - Round-trip time in milliseconds
    fn on_ack(&mut self, acked: u32, rtt: u32);

    /// Called when packet loss is detected
    ///
    /// # Arguments
    /// * `lost` - Number of bytes lost
    fn on_loss(&mut self, lost: u32);

    /// Called when ECN (Explicit Congestion Notification) is received
    fn on_ecn(&mut self);

    /// Get current congestion window (in bytes)
    fn cwnd(&self) -> u32;

    /// Set congestion window (used for initialization)
    fn set_cwnd(&mut self, cwnd: u32);

    /// Get slow start threshold (in bytes)
    fn ssthresh(&self) -> u32;

    /// Get the name of this congestion control algorithm
    fn name(&self) -> &str;

    /// Reset state (e.g., on connection establishment)
    fn reset(&mut self);

    /// Clone the algorithm
    fn clone_box(&self) -> Box<dyn CongestionControl>;
}

/// New Reno congestion control
///
/// RFC 5681 + RFC 6582
///
/// New Reno improves upon the original Reno algorithm by using
/// partial ACKs to detect multiple packet losses in a window.
#[derive(Debug, Clone)]
pub struct Reno {
    /// Congestion window (bytes)
    cwnd: u32,
    /// Slow start threshold (bytes)
    ssthresh: u32,
    /// Initial congestion window
    init_cwnd: u32,
    /// Minimum congestion window
    min_cwnd: u32,
    /// Number of ACKs in current window
    acks_in_window: u32,
    /// Recovery point (sequence number)
    recovery_point: Option<u32>,
    /// Last ACKed sequence number
    last_acked: u32,
    /// Duplicate ACK count
    dup_acks: u32,
}

impl Reno {
    /// Create a new Reno algorithm instance
    pub fn new(mss: u32) -> Self {
        // Initial cwnd is typically 10 segments (RFC 6928)
        let init_cwnd = 10 * mss;

        Self {
            cwnd: init_cwnd,
            ssthresh: u32::MAX,
            init_cwnd,
            min_cwnd: 2 * mss,
            acks_in_window: 0,
            recovery_point: None,
            last_acked: 0,
            dup_acks: 0,
        }
    }

    /// Increase congestion window during congestion avoidance
    fn increase_cwnd(&mut self, acked: u32) {
        // Additive increase: increase by 1 MSS per RTT
        // Distributed over each ACK: increase by MSS * (acked / cwnd)
        let mss = self.init_cwnd / 10;
        let increase = ((acked as u64) * (mss as u64)) / (self.cwnd as u64);
        self.cwnd += increase as u32;
    }
}

impl CongestionControl for Reno {
    fn on_ack(&mut self, acked: u32, _rtt: u32) {
        // Note: In a real implementation, we would track the ACK sequence number
        // to detect duplicate ACKs and partial ACKs. This is simplified.

        if self.cwnd < self.ssthresh {
            // Slow start: exponential growth
            self.cwnd += acked;
        } else {
            // Congestion avoidance: linear growth
            self.increase_cwnd(acked);
        }

        // Reset duplicate ACK counter on new data ACK
        self.dup_acks = 0;
    }

    fn on_loss(&mut self, lost: u32) {
        // On loss detection, set ssthresh to half of current cwnd
        self.ssthresh = (self.cwnd / 2).max(self.min_cwnd);

        // Reduce cwnd
        self.cwnd = self.ssthresh;

        // Enter recovery phase if not already in recovery
        if self.recovery_point.is_none() {
            self.recovery_point = Some(self.last_acked + lost);
        }
    }

    fn on_ecn(&mut self) {
        // On ECN, reduce cwnd like a loss but don't enter fast recovery
        self.ssthresh = (self.cwnd / 2).max(self.min_cwnd);
        self.cwnd = self.ssthresh;
    }

    fn cwnd(&self) -> u32 {
        self.cwnd
    }

    fn set_cwnd(&mut self, cwnd: u32) {
        self.cwnd = cwnd.max(self.min_cwnd);
    }

    fn ssthresh(&self) -> u32 {
        self.ssthresh
    }

    fn name(&self) -> &str {
        "reno"
    }

    fn reset(&mut self) {
        self.cwnd = self.init_cwnd;
        self.ssthresh = u32::MAX;
        self.acks_in_window = 0;
        self.recovery_point = None;
        self.last_acked = 0;
        self.dup_acks = 0;
    }

    fn clone_box(&self) -> Box<dyn CongestionControl> {
        Box::new(self.clone())
    }
}

/// CUBIC congestion control
///
/// RFC 8312
///
/// CUBIC is designed for high-bandwidth, high-latency networks.
/// It uses a cubic function to update the congestion window
/// instead of the linear increase in standard TCP.
#[derive(Debug, Clone)]
pub struct Cubic {
    /// Congestion window (bytes)
    cwnd: u32,
    /// Slow start threshold (bytes)
    ssthresh: u32,
    /// Initial congestion window
    init_cwnd: u32,
    /// Minimum congestion window
    min_cwnd: u32,
    /// Time of last congestion event (ms)
    last_loss_time: u64,
    /// C parameter (scales the window growth)
    c: f32,
    /// Fast convergence enabled
    fast_convergence: bool,
    /// Current time (ms) - would be set by system
    current_time: u64,
}

impl Cubic {
    /// Create a new CUBIC algorithm instance
    pub fn new(mss: u32) -> Self {
        // Initial cwnd is typically 10 segments
        let init_cwnd = 10 * mss;

        // C is typically 0.4
        let c = 0.4;

        Self {
            cwnd: init_cwnd,
            ssthresh: u32::MAX,
            init_cwnd,
            min_cwnd: 2 * mss,
            last_loss_time: 0,
            c,
            fast_convergence: true,
            current_time: 0,
        }
    }

    /// Set the current time (for window calculation)
    pub fn set_time(&mut self, time: u64) {
        self.current_time = time;
    }

    /// Calculate the target window based on time since last loss
    fn calculate_target_window(&self) -> u32 {
        // Time since last loss in seconds
        let t = ((self.current_time - self.last_loss_time) as f64) / 1000.0;

        // K is the time period to reach the window size before last loss
        let w_max = self.ssthresh as f64;
        // K = cbrt(w_max / (4*C)) - approximate cube root
        let k_cubed = w_max / (4.0 * self.c as f64);
        let k = libm::pow(k_cubed, 1.0 / 3.0);

        // CUBIC function: W_cubic(t) = C * (t - K)^3 + w_max
        let t_minus_k = t - k;
        let t_minus_k_cubed = t_minus_k * t_minus_k * t_minus_k;
        let w_cubic = (self.c as f64) * t_minus_k_cubed + w_max;

        w_cubic as u32
    }
}

impl CongestionControl for Cubic {
    fn on_ack(&mut self, acked: u32, _rtt: u32) {
        if self.cwnd < self.ssthresh {
            // Slow start: same as Reno
            self.cwnd += acked;
        } else {
            // CUBIC congestion avoidance
            let target = self.calculate_target_window();

            // Use the convex/concave nature of CUBIC
            if self.cwnd < target {
                // Concave region: window is below target
                // Increase towards target
                let increase = target - self.cwnd;
                self.cwnd += (increase / 2).max(acked);
            } else {
                // Convex region: window is above target
                // Use TCP-friendly region
                self.cwnd += acked;
            }
        }
    }

    fn on_loss(&mut self, _lost: u32) {
        // Record loss time
        self.last_loss_time = self.current_time;

        // Calculate new ssthresh based on fast convergence
        if self.fast_convergence && self.cwnd < self.ssthresh {
            // Fast convergence: reduce more aggressively
            self.ssthresh = (self.cwnd * 7 / 10).max(self.min_cwnd);
        } else {
            // Normal: reduce to 0.7 * cwnd
            self.ssthresh = (self.cwnd * 7 / 10).max(self.min_cwnd);
        }

        // Set cwnd to ssthresh + 3 segments (for fast recovery)
        let mss = self.init_cwnd / 10;
        self.cwnd = self.ssthresh + 3 * mss;
    }

    fn on_ecn(&mut self) {
        // On ECN, reduce like a loss
        self.ssthresh = (self.cwnd * 7 / 10).max(self.min_cwnd);
        self.cwnd = self.ssthresh;
    }

    fn cwnd(&self) -> u32 {
        self.cwnd
    }

    fn set_cwnd(&mut self, cwnd: u32) {
        self.cwnd = cwnd.max(self.min_cwnd);
    }

    fn ssthresh(&self) -> u32 {
        self.ssthresh
    }

    fn name(&self) -> &str {
        "cubic"
    }

    fn reset(&mut self) {
        self.cwnd = self.init_cwnd;
        self.ssthresh = u32::MAX;
        self.last_loss_time = self.current_time;
    }

    fn clone_box(&self) -> Box<dyn CongestionControl> {
        Box::new(self.clone())
    }
}

/// BBR v2 congestion control
///
/// RFC 9780 (draft)
///
/// BBR (Bottleneck Bandwidth and RTT) model-based congestion control.
/// It estimates the available bandwidth and path RTT to control the sending rate.
#[derive(Debug, Clone)]
pub struct Bbr {
    /// Current state
    state: BbrState,
    /// Congestion window (bytes)
    cwnd: u32,
    /// Initial congestion window
    init_cwnd: u32,
    /// Minimum congestion window
    min_cwnd: u32,
    /// Estimated bandwidth (bytes per second)
    bw_est: u32,
    /// Estimated RTT (milliseconds)
    rtt_est: u32,
    /// Minimum RTT observed (milliseconds)
    min_rtt: u32,
    /// Maximum bandwidth observed (bytes per second)
    max_bw: u32,
    /// Time of last min RTT sample
    min_rtt_time: u64,
    /// Current time (ms)
    current_time: u64,
    /// Probe RTT interval (ms)
    probe_rtt_interval: u64,
    /// BBR gain factor
    gain: f32,
    /// High gain for startup
    startup_gain: f32,
    /// Gain for bandwidth probing
    probe_up_gain: f32,
    /// Gain for draining queue
    drain_gain: f32,
    /// Gain for cruise (steady state)
    cruise_gain: f32,
}

impl Bbr {
    /// Create a new BBR algorithm instance
    pub fn new(mss: u32) -> Self {
        let init_cwnd = 10 * mss;

        Self {
            state: BbrState::Startup,
            cwnd: init_cwnd,
            init_cwnd,
            min_cwnd: 4 * mss,
            bw_est: 0,
            rtt_est: 100, // Default 100ms
            min_rtt: u32::MAX,
            max_bw: 0,
            min_rtt_time: 0,
            current_time: 0,
            probe_rtt_interval: 10_000, // 10 seconds
            gain: 2.885,               // Startup gain (2/ln(2))
            startup_gain: 2.885,
            probe_up_gain: 1.25,
            drain_gain: 0.5,
            cruise_gain: 1.0,
        }
    }

    /// Set the current time
    pub fn set_time(&mut self, time: u64) {
        self.current_time = time;
    }

    /// Update bandwidth estimate
    fn update_bandwidth(&mut self, delivered: u32, rtt: u32) {
        let bw_sample = (delivered as u64) * 1000 / (rtt as u64);

        if bw_sample > self.max_bw as u64 {
            self.max_bw = bw_sample as u32;
        }

        // Smooth bandwidth estimate
        if self.bw_est == 0 {
            self.bw_est = bw_sample as u32;
        } else {
            // Exponential weighted moving average
            let alpha = 0.1;
            self.bw_est = (self.bw_est as f64 * (1.0 - alpha) + (bw_sample as f64) * alpha) as u32;
        }
    }

    /// Update RTT estimate
    fn update_rtt(&mut self, rtt: u32) {
        self.rtt_est = rtt;

        if rtt < self.min_rtt {
            self.min_rtt = rtt;
            self.min_rtt_time = self.current_time;
        }
    }

    /// Check if we should enter ProbeRTT state
    fn should_probe_rtt(&self) -> bool {
        self.current_time - self.min_rtt_time > self.probe_rtt_interval
    }

    /// Calculate pacing rate (bytes per second)
    pub fn pacing_rate(&self) -> u32 {
        (self.bw_est as f64 * self.gain as f64) as u32
    }

    /// Calculate target window based on BDP
    fn calculate_bdp_window(&self) -> u32 {
        // BDP = bandwidth * RTT
        let bdp = (self.bw_est as u64) * (self.min_rtt as u64) / 1000;
        bdp as u32
    }
}

impl CongestionControl for Bbr {
    fn on_ack(&mut self, acked: u32, rtt: u32) {
        // Update estimates
        self.update_bandwidth(acked, rtt);
        self.update_rtt(rtt);

        match self.state {
            BbrState::Startup => {
                // High gain to fill pipe
                self.gain = self.startup_gain;

                // Check if pipe is full (when bandwidth stops increasing)
                if self.bw_est > 0 && (self.bw_est as f64) < ((self.max_bw as f64) * 1.25) {
                    // Pipe is full, transition to Drain
                    self.state = BbrState::Drain;
                }
            }
            BbrState::Drain => {
                // Drain the queue
                self.gain = self.drain_gain;

                // When cwnd drops below BDP, transition to ProbeBW
                let bdp = self.calculate_bdp_window();
                if self.cwnd <= bdp {
                    self.state = BbrState::ProbeBW;
                }
            }
            BbrState::ProbeBW => {
                // Cycle through gain phases
                self.gain = self.cruise_gain;

                // Check if we should probe RTT
                if self.should_probe_rtt() {
                    self.state = BbrState::ProbeRTT;
                }
            }
            BbrState::ProbeRTT => {
                // Reduce cwnd to probe min RTT
                self.cwnd = self.cwnd.min(4 * self.init_cwnd);

                // Stay in ProbeRTT for at least one RTT
                if self.current_time - self.min_rtt_time > 200 {
                    // Transition back to ProbeBW
                    self.state = BbrState::ProbeBW;
                    self.min_rtt_time = self.current_time;
                }
            }
        }

        // Update cwnd based on pacing
        let _pacing_rate = self.pacing_rate();
        let bdp = self.calculate_bdp_window();
        self.cwnd = (bdp as f64 * self.gain as f64) as u32;
        self.cwnd = self.cwnd.max(self.min_cwnd).max(acked);
    }

    fn on_loss(&mut self, _lost: u32) {
        // BBR handles losses differently - doesn't reduce cwnd directly
        // Only reduce if losses are persistent
    }

    fn on_ecn(&mut self) {
        // On ECN, BBR may reduce its gain slightly
        if self.gain > self.cruise_gain {
            self.gain = self.cruise_gain;
        }
    }

    fn cwnd(&self) -> u32 {
        self.cwnd
    }

    fn set_cwnd(&mut self, cwnd: u32) {
        self.cwnd = cwnd.max(self.min_cwnd);
    }

    fn ssthresh(&self) -> u32 {
        // BBR doesn't use ssthresh in the traditional sense
        self.calculate_bdp_window()
    }

    fn name(&self) -> &str {
        "bbr"
    }

    fn reset(&mut self) {
        self.state = BbrState::Startup;
        self.cwnd = self.init_cwnd;
        self.bw_est = 0;
        self.rtt_est = 100;
        self.min_rtt = u32::MAX;
        self.max_bw = 0;
        self.gain = self.startup_gain;
    }

    fn clone_box(&self) -> Box<dyn CongestionControl> {
        Box::new(self.clone())
    }
}

/// BBR states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BbrState {
    /// Startup phase - filling the pipe
    Startup,
    /// Drain phase - draining the queue
    Drain,
    /// ProbeBW phase - probing bandwidth
    ProbeBW,
    /// ProbeRTT phase - probing min RTT
    ProbeRTT,
}

/// Create a congestion control algorithm by name
pub fn create_congestion_control(name: &str, mss: u32) -> Box<dyn CongestionControl> {
    match name.to_lowercase().as_str() {
        "cubic" => Box::new(Cubic::new(mss)),
        "bbr" => Box::new(Bbr::new(mss)),
        _ => Box::new(Reno::new(mss)), // Default to Reno
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reno_slow_start() {
        let mut reno = Reno::new(1460);
        reno.set_cwnd(1460 * 2);

        // Should be in slow start
        assert!(reno.cwnd() < reno.ssthresh());

        // ACK 1460 bytes
        reno.on_ack(1460, 100);

        // Should double during slow start
        assert_eq!(reno.cwnd(), 1460 * 3);
    }

    #[test]
    fn test_reno_congestion_avoidance() {
        let mut reno = Reno::new(1460);
        reno.set_cwnd(1460 * 10);
        reno.ssthresh = 1460 * 8;

        let old_cwnd = reno.cwnd();

        // ACK 1460 bytes
        reno.on_ack(1460, 100);

        // Should increase linearly (by less than MSS)
        assert!(reno.cwnd() > old_cwnd);
        assert!(reno.cwnd() < old_cwnd + 1460);
    }

    #[test]
    fn test_reno_loss() {
        let mut reno = Reno::new(1460);
        reno.set_cwnd(1460 * 20);

        // Simulate loss
        reno.on_loss(1460);

        // ssthresh should be halved
        assert_eq!(reno.ssthresh(), 1460 * 10);

        // cwnd should be reduced to ssthresh
        assert_eq!(reno.cwnd(), 1460 * 10);
    }

    #[test]
    fn test_cubic_creation() {
        let cubic = Cubic::new(1460);
        assert_eq!(cubic.name(), "cubic");
        assert!(cubic.cwnd() > 0);
    }

    #[test]
    fn test_bbr_creation() {
        let bbr = Bbr::new(1460);
        assert_eq!(bbr.name(), "bbr");
        assert_eq!(bbr.state, BbrState::Startup);
        assert!(bbr.cwnd() > 0);
    }

    #[test]
    fn test_congestion_control_factory() {
        let reno = create_congestion_control("reno", 1460);
        assert_eq!(reno.name(), "reno");

        let cubic = create_congestion_control("cubic", 1460);
        assert_eq!(cubic.name(), "cubic");

        let bbr = create_congestion_control("bbr", 1460);
        assert_eq!(bbr.name(), "bbr");

        let default = create_congestion_control("unknown", 1460);
        assert_eq!(default.name(), "reno");
    }
}
