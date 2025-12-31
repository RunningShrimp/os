//! Doppler Shift Compensation for Satellite Communications
//!
//! This module provides comprehensive Doppler shift estimation, tracking, and
//! compensation for satellite communication systems. It handles both orbital
//! Doppler effects and receiver clock drift.
//!
//! # Features
//! - Real-time Doppler estimation using PLL/FLL tracking loops
//! - Orbital mechanics for prediction-based compensation
//! - Multi-rate tracking (fast acquisition, steady-state tracking)
//! - Clock drift compensation
//! - Support for LEO, MEO, GEO, and deep space trajectories

#![allow(dead_code)]

extern crate alloc;

use alloc::vec::Vec;
use core::f64::consts::PI;
use core::time::Duration;

use super::sat_types::{
    EcefPosition, OrbitalElements, SatResult, Timestamp, Velocity, MU_EARTH,
};

// Import math functions from libm
use libm::{sin, cos, sqrt, atan2, ceil};

// ============================================================================
// Doppler Estimation
// ============================================================================

/// Doppler shift estimate (Hz)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DopplerShift {
    /// Frequency offset (Hz)
    pub frequency_offset: f64,

    /// Rate of change (Hz/s)
    pub rate: f64,

    /// Timestamp of estimate
    pub timestamp: Timestamp,
}

impl DopplerShift {
    /// Create new Doppler shift estimate
    pub fn new(frequency_offset: f64, rate: f64) -> Self {
        Self {
            frequency_offset,
            rate,
            timestamp: Timestamp::now(),
        }
    }

    /// Predict Doppler at future time
    pub fn predict(&self, dt: Duration) -> f64 {
        let dt_secs = dt.as_secs_f64();
        self.frequency_offset + self.rate * dt_secs
    }

    /// Calculate from relative velocity
    pub fn from_velocity(relative_velocity: f64, carrier_freq: f64) -> Self {
        let frequency_offset = carrier_freq * relative_velocity / 299_792_458.0; // Speed of light
        Self::new(frequency_offset, 0.0)
    }
}

/// Doppler estimator using frequency-locked loop
pub struct DopplerEstimator {
    /// Center frequency (Hz)
    carrier_freq: f64,

    /// Loop bandwidth (Hz)
    loop_bandwidth: f64,

    /// Current Doppler estimate
    estimate: DopplerShift,

    /// FLL integrator state
    integrator: f64,

    /// Lock detector
    lock_detector: LockDetector,
}

impl DopplerEstimator {
    /// Create new Doppler estimator
    pub fn new(carrier_freq: f64, loop_bandwidth: f64) -> Self {
        Self {
            carrier_freq,
            loop_bandwidth,
            estimate: DopplerShift::new(0.0, 0.0),
            integrator: 0.0,
            lock_detector: LockDetector::new(),
        }
    }

    /// Update estimate from frequency discriminator output
    pub fn update(&mut self, freq_error: f64, sample_period: Duration) -> DopplerShift {
        // FLL filter (proportional + integral)
        let dt = sample_period.as_secs_f64();
        self.integrator += freq_error * self.loop_bandwidth * dt;

        let freq_correction = freq_error * self.loop_bandwidth * 2.0 + self.integrator;

        // Update estimate
        self.estimate.frequency_offset += freq_correction * dt;
        self.estimate.timestamp = Timestamp::now();

        // Update lock detector
        self.lock_detector.update(freq_error);

        self.estimate
    }

    /// Get current estimate
    pub fn estimate(&self) -> DopplerShift {
        self.estimate
    }

    /// Check if locked
    pub fn is_locked(&self) -> bool {
        self.lock_detector.is_locked()
    }

    /// Reset estimator
    pub fn reset(&mut self) {
        self.estimate = DopplerShift::new(0.0, 0.0);
        self.integrator = 0.0;
        self.lock_detector.reset();
    }
}

/// Phase-locked loop for fine Doppler tracking
pub struct PhaseLockedLoop {
    /// Loop bandwidth (rad/s)
    bandwidth: f64,

    /// Damping factor
    damping: f64,

    /// Current phase estimate (rad)
    phase: f64,

    /// Current frequency estimate (rad/s)
    frequency: f64,

    /// Loop filter state
    filter_state: PllFilter,

    /// Lock detector
    lock_detector: LockDetector,
}

impl PhaseLockedLoop {
    /// Create new PLL
    pub fn new(bandwidth: f64, damping: f64) -> Self {
        Self {
            bandwidth,
            damping,
            phase: 0.0,
            frequency: 0.0,
            filter_state: PllFilter::new(bandwidth, damping),
            lock_detector: LockDetector::new(),
        }
    }

    /// Update PLL with phase error
    pub fn update(&mut self, phase_error: f64, dt: Duration) -> (f64, f64) {
        // Loop filter
        let (freq_correction, phase_correction) = self.filter_state.update(phase_error, dt);

        // Update estimates
        self.frequency = freq_correction;
        self.phase += phase_correction;

        // Normalize phase
        self.phase = ((self.phase + PI) % (2.0 * PI)) - PI;

        // Update lock detector
        self.lock_detector.update(phase_error);

        (self.phase, self.frequency)
    }

    /// Get frequency estimate in Hz
    pub fn frequency_hz(&self) -> f64 {
        self.frequency / (2.0 * PI)
    }

    /// Check if locked
    pub fn is_locked(&self) -> bool {
        self.lock_detector.is_locked()
    }
}

/// PLL loop filter
struct PllFilter {
    /// Natural frequency
    wn: f64,

    /// Damping factor
    zeta: f64,

    /// Integrator state
    integrator: f64,
}

impl PllFilter {
    fn new(bandwidth: f64, damping: f64) -> Self {
        Self {
            wn: bandwidth,
            zeta: damping,
            integrator: 0.0,
        }
    }

    fn update(&mut self, error: f64, dt: Duration) -> (f64, f64) {
        let dt = dt.as_secs_f64();

        // Second-order PLL filter
        let k1 = 2.0 * self.zeta * self.wn;
        let k2 = self.wn * self.wn;

        self.integrator += k2 * error * dt;

        let freq_out = k1 * error + self.integrator;
        let phase_out = freq_out * dt;

        (freq_out, phase_out)
    }
}

/// Lock detector for tracking loops
struct LockDetector {
    /// Window size for lock detection
    window_size: usize,

    /// Error history
    error_buffer: Vec<f64>,

    /// Lock threshold
    threshold: f64,

    /// Current lock state
    locked: bool,
}

impl LockDetector {
    fn new() -> Self {
        Self {
            window_size: 10,
            error_buffer: Vec::with_capacity(10),
            threshold: 0.1,
            locked: false,
        }
    }

    fn update(&mut self, error: f64) {
        self.error_buffer.push(error.abs());
        if self.error_buffer.len() > self.window_size {
            self.error_buffer.remove(0);
        }

        // Compute average error
        if self.error_buffer.len() >= self.window_size {
            let avg_error: f64 = self.error_buffer.iter().sum::<f64>() / self.error_buffer.len() as f64;
            self.locked = avg_error < self.threshold;
        }
    }

    fn is_locked(&self) -> bool {
        self.locked
    }

    fn reset(&mut self) {
        self.error_buffer.clear();
        self.locked = false;
    }
}

// ============================================================================
// Orbital Doppler Prediction
// ============================================================================

/// Orbital mechanics for Doppler prediction
pub struct OrbitalDopplerPredictor {
    /// Satellite orbital elements
    orbit: OrbitalElements,

    /// Observer position (ECEF)
    observer_pos: EcefPosition,

    /// Observer velocity (ECEF)
    observer_vel: Velocity,

    /// Carrier frequency (Hz)
    carrier_freq: f64,
}

impl OrbitalDopplerPredictor {
    /// Create new predictor
    pub fn new(
        orbit: OrbitalElements,
        observer_pos: EcefPosition,
        observer_vel: Velocity,
        carrier_freq: f64,
    ) -> Self {
        Self {
            orbit,
            observer_pos,
            observer_vel,
            carrier_freq,
        }
    }

    /// Calculate Doppler shift at current time
    pub fn calculate_doppler(&self) -> SatResult<DopplerShift> {
        // Get satellite position and velocity
        let sat_pos = self.orbit.to_ecef();
        let sat_vel = self.calculate_satellite_velocity()?;

        // Calculate range vector
        let rx = sat_pos.x - self.observer_pos.x;
        let ry = sat_pos.y - self.observer_pos.y;
        let rz = sat_pos.z - self.observer_pos.z;

        // Calculate relative velocity
        let vx = sat_vel.vx - self.observer_vel.vx;
        let vy = sat_vel.vy - self.observer_vel.vy;
        let vz = sat_vel.vz - self.observer_vel.vz;

        // Range rate (radial velocity)
        let range = sqrt(rx * rx + ry * ry + rz * rz);
        let range_rate = (rx * vx + ry * vy + rz * vz) / range;

        // Doppler shift
        let frequency_offset = self.carrier_freq * range_rate / 299_792_458.0; // Speed of light

        Ok(DopplerShift {
            frequency_offset,
            rate: 0.0, // Would require numerical differentiation
            timestamp: Timestamp::now(),
        })
    }

    /// Calculate satellite velocity from orbital elements
    fn calculate_satellite_velocity(&self) -> SatResult<Velocity> {
        // Simplified calculation using vis-viva equation
        let r = self.orbit.a * (1.0 - self.orbit.e * self.orbit.e)
            / (1.0 + self.orbit.e * cos(self.orbit.nu));
        let v = sqrt(MU_EARTH * (2.0 / r - 1.0 / self.orbit.a));

        // Velocity direction (tangent to orbit)
        let flight_path_angle = atan2(
            self.orbit.e * sin(self.orbit.nu),
            1.0 + self.orbit.e * cos(self.orbit.nu)
        );

        let vx = v * cos(self.orbit.nu + flight_path_angle);
        let vy = v * sin(self.orbit.nu + flight_path_angle);
        let vz = 0.0; // Simplified (in-plane)

        Ok(Velocity::new(vx, vy, vz))
    }

    /// Predict Doppler at future time
    pub fn predict_doppler(&self, _dt: Duration) -> SatResult<DopplerShift> {
        // Would require propagating orbit forward in time
        // For now, return current estimate
        self.calculate_doppler()
    }

    /// Update satellite orbit
    pub fn update_orbit(&mut self, orbit: OrbitalElements) {
        self.orbit = orbit;
    }

    /// Update observer position and velocity
    pub fn update_observer(&mut self, pos: EcefPosition, vel: Velocity) {
        self.observer_pos = pos;
        self.observer_vel = vel;
    }
}

// ============================================================================
// Clock Drift Compensation
// ============================================================================

/// Clock model for oscillator drift
pub struct ClockModel {
    /// Nominal frequency (Hz)
    nominal_freq: f64,

    /// Frequency offset (ppb)
    frequency_offset: f64,

    /// Drift rate (ppb/s)
    drift_rate: f64,

    /// Reference time
    reference_time: Timestamp,
}

impl ClockModel {
    /// Create new clock model
    pub fn new(nominal_freq: f64, initial_offset_ppb: f64) -> Self {
        Self {
            nominal_freq,
            frequency_offset: initial_offset_ppb,
            drift_rate: 0.0,
            reference_time: Timestamp::now(),
        }
    }

    /// Get actual frequency (Hz)
    pub fn actual_frequency(&self) -> f64 {
        self.nominal_freq * (1.0 + self.frequency_offset * 1e-9)
    }

    /// Get clock offset at time
    pub fn clock_offset(&self, time: Timestamp) -> f64 {
        let dt = time.duration_since(&self.reference_time).as_secs_f64();
        let total_offset_ppb = self.frequency_offset + self.drift_rate * dt;
        total_offset_ppb * dt * 1e-9
    }

    /// Update clock model from measurement
    pub fn update(&mut self, measured_freq: f64, dt: Duration) {
        let measured_offset_ppb = (measured_freq / self.nominal_freq - 1.0) * 1e9;

        // Update drift rate
        let dt_secs = dt.as_secs_f64();
        if dt_secs > 0.0 {
            self.drift_rate = (measured_offset_ppb - self.frequency_offset) / dt_secs;
        }

        self.frequency_offset = measured_offset_ppb;
    }

    /// Predict frequency error
    pub fn predict_error(&self, dt: Duration) -> f64 {
        let dt_secs = dt.as_secs_f64();
        (self.frequency_offset + self.drift_rate * dt_secs) * 1e-9
    }
}

/// Clock disciplining algorithm
pub struct ClockDiscipliner {
    /// Clock model
    clock: ClockModel,

    /// PLL for steering clock
    pll: PhaseLockedLoop,

    /// Time constant for updates
    time_constant: f64,
}

impl ClockDiscipliner {
    /// Create new clock discipliner
    pub fn new(nominal_freq: f64, time_constant: f64) -> Self {
        Self {
            clock: ClockModel::new(nominal_freq, 0.0),
            pll: PhaseLockedLoop::new(1.0, 0.707),
            time_constant,
        }
    }

    /// Update from reference time measurement
    pub fn update(&mut self, local_time: Timestamp, reference_time: Timestamp) -> f64 {
        let offset = reference_time.as_nanos() as f64 - local_time.as_nanos() as f64;
        let offset_secs = offset * 1e-9;

        // Update PLL
        let dt = Duration::from_secs_f64(self.time_constant);
        let (_phase, _freq) = self.pll.update(offset_secs, dt);

        // Update clock model
        let correction = self.pll.frequency_hz();
        self.clock.frequency_offset = correction * 1e9;

        correction
    }

    /// Get clock correction (Hz)
    pub fn correction(&self) -> f64 {
        self.clock.actual_frequency() - self.clock.nominal_freq
    }
}

// ============================================================================
// Doppler Compensation Engine
// ============================================================================

/// Complete Doppler compensation system
pub struct DopplerCompensator {
    /// Predictive Doppler estimator
    predictor: OrbitalDopplerPredictor,

    /// Tracking FLL/PLL
    tracker: DopplerEstimator,

    /// Clock model
    clock: ClockModel,

    /// Compensation mode
    mode: CompensationMode,

    /// Statistics
    stats: CompensationStats,
}

impl DopplerCompensator {
    /// Create new compensator
    pub fn new(
        orbit: OrbitalElements,
        observer_pos: EcefPosition,
        observer_vel: Velocity,
        carrier_freq: f64,
        clock_freq: f64,
    ) -> Self {
        Self {
            predictor: OrbitalDopplerPredictor::new(
                orbit,
                observer_pos,
                observer_vel,
                carrier_freq,
            ),
            tracker: DopplerEstimator::new(carrier_freq, 10.0),
            clock: ClockModel::new(clock_freq, 0.0),
            mode: CompensationMode::Hybrid,
            stats: CompensationStats::new(),
        }
    }

    /// Get current Doppler estimate
    pub fn estimate_doppler(&mut self) -> SatResult<DopplerShift> {
        match self.mode {
            CompensationMode::Predictive => self.predictor.calculate_doppler(),

            CompensationMode::Tracking => {
                // Return tracker estimate (updated externally)
                Ok(self.tracker.estimate())
            }

            CompensationMode::Hybrid => {
                // Blend predictive and tracking estimates
                let pred = self.predictor.calculate_doppler()?;
                let track = self.tracker.estimate();

                // Weight based on lock status
                let alpha = if self.tracker.is_locked() { 0.8 } else { 0.3 };

                let blended_freq = alpha * track.frequency_offset + (1.0 - alpha) * pred.frequency_offset;

                Ok(DopplerShift {
                    frequency_offset: blended_freq,
                    rate: pred.rate,
                    timestamp: Timestamp::now(),
                })
            }
        }
    }

    /// Update tracking estimate
    pub fn update_tracking(&mut self, freq_error: f64, dt: Duration) {
        self.tracker.update(freq_error, dt);
        self.stats.track_updates += 1;
    }

    /// Update clock model
    pub fn update_clock(&mut self, measured_freq: f64, dt: Duration) {
        self.clock.update(measured_freq, dt);
        self.stats.clock_updates += 1;
    }

    /// Compensate signal frequency
    pub fn compensate(&mut self, input_freq: f64) -> SatResult<f64> {
        let doppler = self.estimate_doppler()?;
        let clock_error = self.clock.actual_frequency() - self.clock.nominal_freq;

        let compensated = input_freq - doppler.frequency_offset - clock_error;

        self.stats.compensated_samples += 1;

        Ok(compensated)
    }

    /// Get compensation statistics
    pub fn statistics(&self) -> &CompensationStats {
        &self.stats
    }

    /// Check if locked
    pub fn is_locked(&self) -> bool {
        self.tracker.is_locked()
    }
}

/// Compensation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompensationMode {
    /// Pure prediction from orbital mechanics
    Predictive,

    /// Pure tracking from signal measurements
    Tracking,

    /// Hybrid: blend prediction and tracking
    Hybrid,
}

/// Compensation statistics
#[derive(Debug, Clone, PartialEq)]
pub struct CompensationStats {
    /// Number of tracking updates
    pub track_updates: u64,

    /// Number of clock updates
    pub clock_updates: u64,

    /// Number of compensated samples
    pub compensated_samples: u64,

    /// Average residual error (Hz)
    pub average_residual: f64,

    /// Peak residual error (Hz)
    pub peak_residual: f64,
}

impl CompensationStats {
    fn new() -> Self {
        Self {
            track_updates: 0,
            clock_updates: 0,
            compensated_samples: 0,
            average_residual: 0.0,
            peak_residual: 0.0,
        }
    }
}

// ============================================================================
// Doppler-Aided Acquisition
// ============================================================================

/// Doppler search for signal acquisition
pub struct DopplerSearch {
    /// Carrier frequency (Hz)
    carrier_freq: f64,

    /// Search range (Hz)
    search_range: f64,

    /// Frequency step (Hz)
    frequency_step: f64,

    /// Current search bin
    current_bin: usize,

    /// Total number of bins
    total_bins: usize,
}

impl DopplerSearch {
    /// Create new search
    pub fn new(carrier_freq: f64, search_range: f64, frequency_step: f64) -> Self {
        let total_bins = ceil((2.0 * search_range) / frequency_step) as usize;

        Self {
            carrier_freq,
            search_range,
            frequency_step,
            current_bin: 0,
            total_bins,
        }
    }

    /// Get next frequency to try
    pub fn next_frequency(&mut self) -> Option<f64> {
        if self.current_bin < self.total_bins {
            let offset = -self.search_range + self.current_bin as f64 * self.frequency_step;
            self.current_bin += 1;
            Some(self.carrier_freq + offset)
        } else {
            None
        }
    }

    /// Reset search
    pub fn reset(&mut self) {
        self.current_bin = 0;
    }

    /// Get search progress (0.0 to 1.0)
    pub fn progress(&self) -> f64 {
        if self.total_bins > 0 {
            self.current_bin as f64 / self.total_bins as f64
        } else {
            0.0
        }
    }
}

/// Doppler-aided acquisition
pub struct DopplerAidedAcquisition {
    /// Predictive Doppler estimate
    predicted_doppler: DopplerShift,

    /// Search window (Hz)
    search_window: f64,

    /// Frequency step (Hz)
    frequency_step: f64,

    /// Confidence in prediction (0.0 to 1.0)
    prediction_confidence: f64,
}

impl DopplerAidedAcquisition {
    /// Create new acquisition helper
    pub fn new(predicted_doppler: DopplerShift, search_window: f64, frequency_step: f64) -> Self {
        Self {
            predicted_doppler,
            search_window,
            frequency_step,
            prediction_confidence: 0.8,
        }
    }

    /// Generate frequency search list (centered on prediction)
    pub fn search_frequencies(&self) -> Vec<f64> {
        let center = self.predicted_doppler.frequency_offset;
        let half_window = self.search_window / 2.0;

        let mut frequencies = Vec::new();
        let mut freq = center - half_window;

        while freq <= center + half_window {
            frequencies.push(freq);
            freq += self.frequency_step;
        }

        frequencies
    }

    /// Update prediction confidence
    pub fn update_confidence(&mut self, confidence: f64) {
        self.prediction_confidence = confidence.clamp(0.0, 1.0);
    }

    /// Get search window size (adaptive based on confidence)
    pub fn adaptive_window(&self) -> f64 {
        // Lower confidence -> larger search window
        self.search_window / self.prediction_confidence
    }
}
