//! TCP congestion control tests
//!
//! Comprehensive tests for TCP congestion control algorithms

extern crate alloc;

use crate::subsystems::net::tcp::congestion::*;

#[cfg(test)]
mod reno_tests {
    use super::*;

    #[test]
    fn test_reno_creation() {
        let reno = Reno::new(1460);
        assert_eq!(reno.name(), "reno");
        assert!(reno.cwnd() > 0);
        assert_eq!(reno.ssthresh(), u32::MAX); // Initially infinity
    }

    #[test]
    fn test_reno_slow_start() {
        let mut reno = Reno::new(1460);
        reno.set_cwnd(1460 * 2);
        reno.ssthresh = 1460 * 10;

        let old_cwnd = reno.cwnd();

        // ACK 1460 bytes in slow start
        reno.on_ack(1460, 100);

        // Should double (additive increase during slow start)
        assert_eq!(reno.cwnd(), old_cwnd + 1460);
    }

    #[test]
    fn test_reno_congestion_avoidance() {
        let mut reno = Reno::new(1460);
        reno.set_cwnd(1460 * 20); // Large window
        reno.ssthresh = 1460 * 10; // Lower ssthresh

        let old_cwnd = reno.cwnd();

        // ACK 1460 bytes in congestion avoidance
        reno.on_ack(1460, 100);

        // Should increase by approximately MSS^2 / cwnd (much less than MSS)
        assert!(reno.cwnd() > old_cwnd);
        assert!(reno.cwnd() < old_cwnd + 1460);
    }

    #[test]
    fn test_reno_packet_loss() {
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
    fn test_reno_ecn() {
        let mut reno = Reno::new(1460);
        reno.set_cwnd(1460 * 20);

        // ECN signal
        reno.on_ecn();

        // Should reduce like loss but not as dramatically
        assert_eq!(reno.ssthresh(), 1460 * 10);
        assert_eq!(reno.cwnd(), 1460 * 10);
    }

    #[test]
    fn test_reno_reset() {
        let mut reno = Reno::new(1460);
        reno.set_cwnd(1460 * 20);
        reno.ssthresh = 1460 * 10;

        reno.reset();

        // Should return to initial state
        assert!(reno.cwnd() > 0);
        assert_eq!(reno.ssthresh(), u32::MAX);
    }

    #[test]
    fn test_reno_clone() {
        let reno1 = Reno::new(1460);
        let mut reno2 = reno1.clone_box();

        reno2.set_cwnd(1460 * 5);

        // Original should be unchanged
        assert!(reno1.cwnd() < reno2.cwnd());
    }
}

#[cfg(test)]
mod cubic_tests {
    use super::*;

    #[test]
    fn test_cubic_creation() {
        let cubic = Cubic::new(1460);
        assert_eq!(cubic.name(), "cubic");
        assert!(cubic.cwnd() > 0);
    }

    #[test]
    fn test_cubic_slow_start() {
        let mut cubic = Cubic::new(1460);
        cubic.set_cwnd(1460 * 5);
        cubic.ssthresh = 1460 * 10;

        let old_cwnd = cubic.cwnd();

        // ACK during slow start
        cubic.on_ack(1460, 100);

        // Should increase (more aggressively than Reno)
        assert!(cubic.cwnd() > old_cwnd);
    }

    #[test]
    fn test_cubic_packet_loss() {
        let mut cubic = Cubic::new(1460);
        cubic.set_time(0);
        cubic.set_cwnd(1460 * 20);

        // Simulate loss
        cubic.on_loss(1460);

        // Should reduce to ~70% of current cwnd
        assert_eq!(cubic.cwnd(), 1460 * 14);
        assert_eq!(cubic.ssthresh(), 1460 * 14);
    }

    #[test]
    fn test_cubic_fast_convergence() {
        let mut cubic = Cubic::new(1460);
        cubic.set_time(0);
        cubic.set_cwnd(1460 * 20);
        cubic.ssthresh = 1460 * 15; // Lower than cwnd

        cubic.on_loss(1460);

        // Fast convergence: reduce more when cwnd < ssthresh
        assert!(cubic.cwnd() < 1460 * 20);
    }

    #[test]
    fn test_cubic_window_calculation() {
        let mut cubic = Cubic::new(1460);
        cubic.set_time(1000);
        cubic.last_loss_time = 0;
        cubic.ssthresh = 1460 * 10;
        cubic.cwnd = 1460 * 8;

        let target = cubic.calculate_target_window();

        // Target should be based on cubic function
        assert!(target > 0);
    }

    #[test]
    fn test_cubic_reset() {
        let mut cubic = Cubic::new(1460);
        cubic.set_cwnd(1460 * 20);
        cubic.ssthresh = 1460 * 10;

        cubic.reset();

        // Should return to initial state
        assert!(cubic.cwnd() > 0);
    }
}

#[cfg(test)]
mod bbr_tests {
    use super::*;

    #[test]
    fn test_bbr_creation() {
        let bbr = Bbr::new(1460);
        assert_eq!(bbr.name(), "bbr");
        assert_eq!(bbr.state, BbrState::Startup);
        assert!(bbr.cwnd() > 0);
    }

    #[test]
    fn test_bbr_startup_phase() {
        let mut bbr = Bbr::new(1460);
        bbr.set_time(1000);

        // Process ACKs in startup
        bbr.on_ack(1460, 50);
        bbr.on_ack(1460, 50);
        bbr.on_ack(1460, 50);

        // Should still be in startup with high gain
        assert_eq!(bbr.state, BbrState::Startup);
        assert!(bbr.gain > 1.0);
    }

    #[test]
    fn test_bdr_drain_phase() {
        let mut bbr = Bbr::new(1460);
        bbr.set_time(1000);

        // Force entry to drain by setting bandwidth estimate
        bbr.bw_est = 1000000;
        bbr.max_bw = 1000000;

        bbr.on_ack(1460, 50);

        // Should transition to drain
        assert_eq!(bbr.state, BbrState::Drain);
        assert!(bbr.gain < 1.0);
    }

    #[test]
    fn test_bbr_bandwidth_estimation() {
        let mut bbr = Bbr::new(1460);

        bbr.update_bandwidth(14600, 100); // 14600 bytes in 100ms = 146000 bytes/s

        assert!(bbr.bw_est > 0);
        assert!(bbr.max_bw > 0);
    }

    #[test]
    fn test_bbr_rtt_estimation() {
        let mut bbr = Bbr::new(1460);

        bbr.update_rtt(100);
        assert_eq!(bbr.rtt_est, 100);
        assert_eq!(bbr.min_rtt, 100);

        bbr.update_rtt(80);
        assert_eq!(bbr.min_rtt, 80); // Should track minimum
    }

    #[test]
    fn test_bbr_pacing_rate() {
        let mut bbr = Bbr::new(1460);
        bbr.bw_est = 1000000; // 1 MB/s

        let pacing = bbr.pacing_rate();

        // Pacing rate should be based on bandwidth and gain
        assert!(pacing > 1000000); // Higher due to gain in startup
    }

    #[test]
    fn test_bbr_bdp_calculation() {
        let mut bbr = Bbr::new(1460);
        bbr.bw_est = 1000000; // 1 MB/s
        bbr.min_rtt = 100;    // 100ms

        let bdp = bbr.calculate_bdp_window();

        // BDP = bandwidth * RTT = 1MB/s * 0.1s = 100KB
        assert!(bdp > 0);
        assert!(bdp < 200000); // Should be approximately 100KB
    }

    #[test]
    fn test_bbr_reset() {
        let mut bbr = Bbr::new(1460);
        bbr.state = BbrState::ProbeBW;
        bbr.set_cwnd(1460 * 20);

        bbr.reset();

        // Should return to startup
        assert_eq!(bbr.state, BbrState::Startup);
        assert!(bbr.cwnd() > 0);
    }

    #[test]
    fn test_bbr_transition_to_probe_bw() {
        let mut bbr = Bbr::new(1460);
        bbr.set_time(1000);

        // Set bandwidth and RTT
        bbr.bw_est = 1000000;
        bbr.max_bw = 1000000;
        bbr.min_rtt = 50;
        bbr.cwnd = 50000; // Below BDP

        // Force drain to complete
        bdr.state = BbrState::Drain;
        bbr.on_ack(1460, 50);

        // Should transition to ProbeBW
        assert_eq!(bbr.state, BbrState::ProbeBW);
    }
}

#[cfg(test)]
mod factory_tests {
    use super::*;

    #[test]
    fn test_factory_reno() {
        let cc = create_congestion_control("reno", 1460);
        assert_eq!(cc.name(), "reno");
    }

    #[test]
    fn test_factory_cubic() {
        let cc = create_congestion_control("cubic", 1460);
        assert_eq!(cc.name(), "cubic");
    }

    #[test]
    fn test_factory_bbr() {
        let cc = create_congestion_control("bbr", 1460);
        assert_eq!(cc.name(), "bbr");
    }

    #[test]
    fn test_factory_case_insensitive() {
        let cc = create_congestion_control("CUBIC", 1460);
        assert_eq!(cc.name(), "cubic");

        let cc = create_congestion_control("Bbr", 1460);
        assert_eq!(cc.name(), "bbr");
    }

    #[test]
    fn test_factory_default() {
        let cc = create_congestion_control("unknown", 1460);
        assert_eq!(cc.name(), "reno"); // Default to Reno
    }

    #[test]
    fn test_factory_different_mss() {
        let cc1 = create_congestion_control("reno", 1460);
        let cc2 = create_congestion_control("reno", 536);

        // Both should have different initial windows
        assert_ne!(cc1.cwnd(), cc2.cwnd());
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_congestion_control_comparison() {
        let mss = 1460;

        let mut reno = Reno::new(mss);
        let mut cubic = Cubic::new(mss);
        let mut bbr = Bbr::new(mss);

        // All should start with reasonable windows
        assert!(reno.cwnd() > 0);
        assert!(cubic.cwnd() > 0);
        assert!(bbr.cwnd() > 0);

        // Simulate some ACKs
        for _ in 0..10 {
            reno.on_ack(mss, 100);
            cubic.on_ack(mss, 100);
            bbr.on_ack(mss, 100);
        }

        // All should have increased
        assert!(reno.cwnd() > mss * 10);
        assert!(cubic.cwnd() > mss * 10);
        assert!(bbr.cwnd() > mss * 10);
    }

    #[test]
    fn test_loss_recovery() {
        let mss = 1460;

        let mut reno = Reno::new(mss);
        let mut cubic = Cubic::new(mss);

        // Build up window
        for _ in 0..20 {
            reno.on_ack(mss, 100);
            cubic.on_ack(mss, 100);
        }

        let reno_before = reno.cwnd();
        let cubic_before = cubic.cwnd();

        // Simulate loss
        reno.on_loss(mss);
        cubic.on_loss(mss);

        // Reno should reduce by half
        assert!(reno.cwnd() < reno_before);
        assert_eq!(reno.cwnd(), reno_before / 2);

        // CUBIC should reduce to ~70%
        assert!(cubic.cwnd() < cubic_before);
        assert!(cubic.cwnd() > cubic_before / 2);
    }

    #[test]
    fn test_algorithm_swapping() {
        let mss = 1460;

        // Start with Reno
        let mut cc: Box<dyn CongestionControl> = Box::new(Reno::new(mss));
        assert_eq!(cc.name(), "reno");

        // Simulate some traffic
        cc.on_ack(mss, 100);
        cc.on_ack(mss, 100);

        // Swap to CUBIC
        let old_cwnd = cc.cwnd();
        cc = create_congestion_control("cubic", mss);
        cc.set_cwnd(old_cwnd); // Preserve window

        assert_eq!(cc.name(), "cubic");
        assert_eq!(cc.cwnd(), old_cwnd);

        // Continue with CUBIC
        cc.on_ack(mss, 100);
        assert!(cc.cwnd() > old_cwnd);
    }

    #[test]
    fn test_slow_start_to_congestion_avoidance() {
        let mss = 1460;
        let mut reno = Reno::new(mss);

        // Initially in slow start
        assert!(reno.cwnd() < reno.ssthresh());

        // Build up to threshold
        while reno.cwnd() < reno.ssthresh() {
            reno.on_ack(mss, 100);
        }

        // Now in congestion avoidance
        let cwnd_ca = reno.cwnd();
        reno.on_ack(mss, 100);

        // Should increase much slower now
        let increase = reno.cwnd() - cwnd_ca;
        assert!(increase < mss);
    }
}
