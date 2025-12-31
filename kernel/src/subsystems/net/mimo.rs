//! MIMO and Beamforming Implementation for 5G/6G
//!
//! This module implements advanced MIMO (Multiple-Input Multiple-Output) techniques:
//! - SU-MIMO (Single-User MIMO)
//! - MU-MIMO (Multi-User MIMO)
//! - Massive MIMO with up to 256 antenna elements
//! - Precoding techniques (ZF, MMSE, SVD)
//! - Beamforming algorithms (Analog, Digital, Hybrid)
//! - Channel estimation and feedback
//! - Spatial multiplexing and diversity
//!
//! Based on 3GPP TS 38.211, 38.214, 38.215 and IEEE 802.11n/ac/ax standards.

#![allow(dead_code)]

use alloc::{vec, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};

// ============================================================================
// MIMO Configuration
// ============================================================================

/// MIMO mode of operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MimoMode {
    /// Single-Input Single-Output
    Siso,
    /// Single-User MIMO
    SuMimo,
    /// Multi-User MIMO
    MuMimo,
    /// Massive MIMO
    MassiveMimo,
}

/// MIMO configuration
#[derive(Debug, Clone)]
pub struct MimoConfig {
    /// MIMO mode
    pub mode: MimoMode,
    /// Number of transmit antennas
    pub num_tx_antennas: u16,
    /// Number of receive antennas
    pub num_rx_antennas: u16,
    /// Number of spatial streams
    pub num_streams: u8,
    /// Transmission scheme
    pub transmission_scheme: TransmissionScheme,
}

/// Transmission scheme
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransmissionScheme {
    /// Spatial multiplexing
    SpatialMultiplexing,
    /// Transmit diversity
    TransmitDiversity,
    /// Beamforming
    Beamforming,
    /// Hybrid scheme
    Hybrid,
}

// ============================================================================
// Precoding Techniques
// ============================================================================

/// Precoding type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrecodingType {
    /// Zero-Forcing
    ZeroForcing,
    /// Minimum Mean Square Error
    MMSE,
    /// Singular Value Decomposition
    SVD,
    /// Maximum Ratio Transmission
    MRT,
    /// Random Beamforming
    Random,
}

/// Precoding matrix
#[derive(Debug, Clone)]
pub struct PrecodingMatrix {
    /// Number of transmit antennas
    pub num_tx: u16,
    /// Number of streams
    pub num_streams: u8,
    /// Real part of precoding matrix
    pub real: Vec<Vec<f32>>,
    /// Imaginary part of precoding matrix
    pub imag: Vec<Vec<f32>>,
}

impl PrecodingMatrix {
    /// Create new precoding matrix
    pub fn new(num_tx: u16, num_streams: u8) -> Self {
        let real = vec![vec![0.0; num_streams as usize]; num_tx as usize];
        let imag = vec![vec![0.0; num_streams as usize]; num_tx as usize];

        Self {
            num_tx,
            num_streams,
            real,
            imag,
        }
    }

    /// Apply precoding to input signal
    pub fn apply(&self, input: &[Complex]) -> Vec<Complex> {
        let mut output = Vec::with_capacity(self.num_tx as usize);

        for i in 0..self.num_tx as usize {
            let mut sum_real = 0.0;
            let mut sum_imag = 0.0;

            for j in 0..self.num_streams as usize {
                if j < input.len() {
                    sum_real += self.real[i][j] * input[j].real - self.imag[i][j] * input[j].imag;
                    sum_imag += self.real[i][j] * input[j].imag + self.imag[i][j] * input[j].real;
                }
            }

            output.push(Complex { real: sum_real, imag: sum_imag });
        }

        output
    }
}

/// Complex number representation
#[derive(Debug, Clone, Copy)]
pub struct Complex {
    pub real: f32,
    pub imag: f32,
}

impl Complex {
    /// Calculate magnitude
    pub fn magnitude(&self) -> f32 {
        f32::sqrt(self.real * self.real + self.imag * self.imag)
    }

    /// Calculate phase
    pub fn phase(&self) -> f32 {
        f32::atan2(self.imag, self.real)
    }

    /// Complex exponential
    pub fn exp(phase: f32) -> Self {
        Self {
            real: f32::cos(phase),
            imag: f32::sin(phase),
        }
    }
}

/// Precoder
#[derive(Debug)]
pub struct Precoder {
    precoding_type: PrecodingType,
    matrix: Option<PrecodingMatrix>,
}

impl Precoder {
    /// Create new precoder
    pub fn new(precoding_type: PrecodingType) -> Self {
        Self {
            precoding_type,
            matrix: None,
        }
    }

    /// Compute precoding matrix
    pub fn compute_matrix(&mut self, channel: &ChannelMatrix) -> Result<(), MimoError> {
        match self.precoding_type {
            PrecodingType::ZeroForcing => {
                self.matrix = Some(self.compute_zf(channel)?);
            }
            PrecodingType::MMSE => {
                self.matrix = Some(self.compute_mmse(channel)?);
            }
            PrecodingType::SVD => {
                self.matrix = Some(self.compute_svd(channel)?);
            }
            PrecodingType::MRT => {
                self.matrix = Some(self.compute_mrt(channel)?);
            }
            PrecodingType::Random => {
                self.matrix = Some(self.compute_random(channel));
            }
        }

        Ok(())
    }

    /// Compute Zero-Forcing precoding
    fn compute_zf(&self, channel: &ChannelMatrix) -> Result<PrecodingMatrix, MimoError> {
        // ZF: W = H^H (H H^H)^(-1)
        let num_tx = channel.num_tx;
        let num_streams = channel.num_rx.min(channel.num_tx) as u8;

        let mut matrix = PrecodingMatrix::new(num_tx, num_streams);

        // Simplified ZF: Use pseudo-inverse (H^H / ||H||^2)
        for i in 0..num_tx as usize {
            for j in 0..num_streams as usize {
                let h_conj = if j < channel.h[i].len() {
                    Complex { real: channel.h[i][j].real, imag: -channel.h[i][j].imag }
                } else {
                    Complex { real: 0.0, imag: 0.0 }
                };

                // Normalize
                let norm = 1.0 / (num_streams as f32);
                matrix.real[i][j] = h_conj.real * norm;
                matrix.imag[i][j] = h_conj.imag * norm;
            }
        }

        Ok(matrix)
    }

    /// Compute MMSE precoding
    fn compute_mmse(&self, channel: &ChannelMatrix) -> Result<PrecodingMatrix, MimoError> {
        // MMSE: W = H^H (H H^H + (1/SNR) I)^(-1)
        let num_tx = channel.num_tx;
        let num_streams = channel.num_rx.min(channel.num_tx) as u8;

        let mut matrix = PrecodingMatrix::new(num_tx, num_streams);

        // Simplified MMSE: Enhanced ZF with regularization
        let snr = 10.0; // 10 dB
        let regularization = 1.0 / snr;

        for i in 0..num_tx as usize {
            for j in 0..num_streams as usize {
                let h_conj = if j < channel.h[i].len() {
                    Complex { real: channel.h[i][j].real, imag: -channel.h[i][j].imag }
                } else {
                    Complex { real: 0.0, imag: 0.0 }
                };

                let norm = 1.0 / (num_streams as f32 + regularization);
                matrix.real[i][j] = h_conj.real * norm;
                matrix.imag[i][j] = h_conj.imag * norm;
            }
        }

        Ok(matrix)
    }

    /// Compute SVD-based precoding
    fn compute_svd(&self, _channel: &ChannelMatrix) -> Result<PrecodingMatrix, MimoError> {
        // SVD-based: Use right singular matrix
        // Simplified implementation
        let num_tx = _channel.num_tx;
        let num_streams = _channel.num_rx.min(_channel.num_tx) as u8;

        let mut matrix = PrecodingMatrix::new(num_tx, num_streams);

        // In real implementation, perform full SVD decomposition
        // Here we use a simplified approach
        for i in 0..num_tx as usize {
            for j in 0..num_streams as usize {
                if i == j as usize {
                    matrix.real[i][j] = 1.0 / f32::sqrt(num_tx as f32);
                } else {
                    matrix.real[i][j] = 0.0;
                }
                matrix.imag[i][j] = 0.0;
            }
        }

        Ok(matrix)
    }

    /// Compute Maximum Ratio Transmission
    fn compute_mrt(&self, channel: &ChannelMatrix) -> Result<PrecodingMatrix, MimoError> {
        // MRT: W = H^H / ||H^H||
        let num_tx = channel.num_tx;
        let num_streams = 1;

        let mut matrix = PrecodingMatrix::new(num_tx, num_streams);

        for i in 0..num_tx as usize {
            let h_row = &channel.h[i];
            let mut sum_real = 0.0;
            let mut sum_imag = 0.0;

            for h in h_row {
                sum_real += h.real;
                sum_imag += h.imag;
            }

            let norm = f32::sqrt(sum_real * sum_real + sum_imag * sum_imag);
            if norm > 0.0 {
                matrix.real[i][0] = sum_real / norm;
                matrix.imag[i][0] = sum_imag / norm;
            }
        }

        Ok(matrix)
    }

    /// Compute random precoding
    fn compute_random(&self, channel: &ChannelMatrix) -> PrecodingMatrix {
        let num_tx = channel.num_tx;
        let num_streams = channel.num_rx.min(channel.num_tx) as u8;

        let mut matrix = PrecodingMatrix::new(num_tx, num_streams);

        for i in 0..num_tx as usize {
            for j in 0..num_streams as usize {
                // Random phase
                let phase = (i * num_streams as usize + j) as f32 * 0.1;
                matrix.real[i][j] = f32::cos(phase);
                matrix.imag[i][j] = f32::sin(phase);
            }
        }

        matrix
    }

    /// Get precoding matrix
    pub fn get_matrix(&self) -> Option<&PrecodingMatrix> {
        self.matrix.as_ref()
    }
}

// ============================================================================
// Channel Estimation
// ============================================================================

/// Channel matrix representation
#[derive(Debug, Clone)]
pub struct ChannelMatrix {
    /// Number of transmit antennas
    pub num_tx: u16,
    /// Number of receive antennas
    pub num_rx: u16,
    /// Channel coefficients H[i][j] from TX j to RX i
    pub h: Vec<Vec<Complex>>,
}

impl ChannelMatrix {
    /// Create new channel matrix
    pub fn new(num_tx: u16, num_rx: u16) -> Self {
        let h = vec![vec![Complex { real: 0.0, imag: 0.0 }; num_tx as usize]; num_rx as usize];

        Self { num_tx, num_rx, h }
    }

    /// Generate random channel (Rayleigh fading)
    pub fn generate_rayleigh(&mut self) {
        for i in 0..self.num_rx as usize {
            for j in 0..self.num_tx as usize {
                // Complex Gaussian with unit variance
                let u1 = (i + j) as f32 * 0.1 + 0.5;
                let u2 = (i + j) as f32 * 0.15 + 0.3;
                let real = f32::sqrt(-2.0 * f32::ln(u1)) * f32::cos(2.0 * core::f32::consts::PI * u2);
                let imag = f32::sqrt(-2.0 * f32::ln(u1)) * f32::sin(2.0 * core::f32::consts::PI * u2);

                // Normalize to unit power
                let norm = 1.0 / f32::sqrt(2.0);
                self.h[i][j] = Complex { real: real * norm, imag: imag * norm };
            }
        }
    }

    /// Calculate channel capacity
    pub fn calculate_capacity(&self, snr_db: f32) -> f32 {
        let snr_linear = 10.0_f32.powf(snr_db / 10.0);
        let min_dim = self.num_tx.min(self.num_rx);

        // Simplified capacity calculation (with SVD)
        // C = sum(log2(1 + SNR * sigma_i^2))
        let mut capacity = 0.0;

        for _i in 0..min_dim {
            // Assume equal singular values for simplicity
            let sigma_sq = 1.0;
            capacity += f32::log2(1.0 + snr_linear * sigma_sq / min_dim as f32);
        }

        capacity
    }

    /// Get condition number
    pub fn condition_number(&self) -> f32 {
        // Simplified: ratio of largest to smallest singular values
        // For Rayleigh fading, this is approximately the antenna ratio
        let max_dim = self.num_tx.max(self.num_rx) as f32;
        let min_dim = self.num_tx.min(self.num_rx) as f32;

        if min_dim > 0.0 {
            max_dim / min_dim
        } else {
            1.0
        }
    }
}

/// Channel estimator
#[derive(Debug)]
pub struct ChannelEstimator {
    estimation_method: EstimationMethod,
    pilots: Vec<Complex>,
}

/// Channel estimation method
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EstimationMethod {
    /// Least Squares
    LeastSquares,
    /// Minimum Mean Square Error
    MMSE,
    /// Blind estimation
    Blind,
}

impl ChannelEstimator {
    /// Create new channel estimator
    pub fn new(method: EstimationMethod, num_pilots: usize) -> Self {
        let pilots = (0..num_pilots)
            .map(|i| {
                let phase = i as f32 * 2.0 * core::f32::consts::PI / num_pilots as f32;
                Complex::exp(phase)
            })
            .collect();

        Self {
            estimation_method: method,
            pilots,
        }
    }

    /// Estimate channel from received pilots
    pub fn estimate(&self, received: &[Complex], num_tx: u16, num_rx: u16) -> ChannelMatrix {
        let mut channel = ChannelMatrix::new(num_tx, num_rx);

        match self.estimation_method {
            EstimationMethod::LeastSquares => {
                // LS: H = Y / X
                for rx in 0..num_rx as usize {
                    for tx in 0..num_tx as usize {
                        let pilot_idx = (rx * num_tx as usize + tx) % self.pilots.len();
                        let pilot = self.pilots[pilot_idx];

                        if pilot_idx < received.len() {
                            let rx_sample = received[pilot_idx];
                            let denom = pilot.real * pilot.real + pilot.imag * pilot.imag;

                            if denom > 0.0 {
                                channel.h[rx][tx] = Complex {
                                    real: (rx_sample.real * pilot.real + rx_sample.imag * pilot.imag) / denom,
                                    imag: (rx_sample.imag * pilot.real - rx_sample.real * pilot.imag) / denom,
                                };
                            }
                        }
                    }
                }
            }
            EstimationMethod::MMSE => {
                // MMSE estimation with noise consideration
                let snr = 10.0; // 10 dB
                let noise_var = 1.0 / (10.0_f32.powf(snr / 10.0));

                for rx in 0..num_rx as usize {
                    for tx in 0..num_tx as usize {
                        let pilot_idx = (rx * num_tx as usize + tx) % self.pilots.len();
                        let pilot = self.pilots[pilot_idx];

                        if pilot_idx < received.len() {
                            let rx_sample = received[pilot_idx];

                            // MMSE with regularization
                            let regularization = noise_var;
                            let denom = pilot.real * pilot.real + pilot.imag * pilot.imag + regularization;

                            if denom > 0.0 {
                                channel.h[rx][tx] = Complex {
                                    real: (rx_sample.real * pilot.real + rx_sample.imag * pilot.imag) / denom,
                                    imag: (rx_sample.imag * pilot.real - rx_sample.real * pilot.imag) / denom,
                                };
                            }
                        }
                    }
                }
            }
            EstimationMethod::Blind => {
                // Blind estimation: assume Rayleigh fading
                channel.generate_rayleigh();
            }
        }

        channel
    }

    /// Refine channel estimate
    pub fn refine(&self, channel: &mut ChannelMatrix, snr_db: f32) {
        // Apply smoothing/filtering
        let alpha = if snr_db > 20.0 {
            0.1
        } else if snr_db > 10.0 {
            0.2
        } else {
            0.3
        };

        // Temporal smoothing (simplified)
        for i in 0..channel.num_rx as usize {
            for j in 0..channel.num_tx as usize {
                // Add small random perturbation
                let perturbation = 1.0 - alpha;
                channel.h[i][j].real *= 1.0 + perturbation * 0.01;
                channel.h[i][j].imag *= 1.0 + perturbation * 0.01;
            }
        }
    }
}

// ============================================================================
// Beamforming
// ============================================================================

/// Beamforming type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BeamformingType {
    /// Analog beamforming
    Analog,
    /// Digital beamforming
    Digital,
    /// Hybrid beamforming
    Hybrid,
}

/// Beam configuration
#[derive(Debug, Clone)]
pub struct BeamConfig {
    /// Beam ID
    pub beam_id: u16,
    /// Azimuth angle in degrees
    pub azimuth_deg: f32,
    /// Elevation angle in degrees
    pub elevation_deg: f32,
    /// Beam width in degrees
    pub beam_width_deg: f32,
}

/// Beamformer
#[derive(Debug)]
pub struct Beamformer {
    beamforming_type: BeamformingType,
    num_antennas: u16,
    num_rf_chains: u16,
    beams: Vec<BeamConfig>,
    active_beam: Option<u16>,
    stats: BeamformingStats,
}

/// Beamforming statistics
#[derive(Debug, Default)]
pub struct BeamformingStats {
    /// Beams formed
    pub beams_formed: AtomicU64,
    /// Beam switches
    pub beam_switches: AtomicU64,
    /// Average gain (dBi, scaled by 10)
    pub avg_gain_dbi: AtomicU64,
}

impl Beamformer {
    /// Create new beamformer
    pub fn new(
        beamforming_type: BeamformingType,
        num_antennas: u16,
        num_rf_chains: u16,
    ) -> Self {
        Self {
            beamforming_type,
            num_antennas,
            num_rf_chains,
            beams: Vec::new(),
            active_beam: None,
            stats: BeamformingStats::default(),
        }
    }

    /// Generate beam codebook
    pub fn generate_codebook(&mut self, num_beams: u16) -> Result<(), MimoError> {
        let beam_width = 360.0 / num_beams as f32;

        for i in 0..num_beams {
            let beam = BeamConfig {
                beam_id: i,
                azimuth_deg: (i as f32) * beam_width,
                elevation_deg: 0.0,
                beam_width_deg: beam_width,
            };

            self.beams.push(beam);
            self.stats.beams_formed.fetch_add(1, Ordering::Relaxed);
        }

        Ok(())
    }

    /// Calculate beamforming weights
    pub fn calculate_weights(&self, beam_id: u16) -> Result<Vec<Complex>, MimoError> {
        if let Some(beam) = self.beams.get(beam_id as usize) {
            let mut weights = Vec::with_capacity(self.num_antennas as usize);

            let wavelength = 3e8 / 28e9; // 28 GHz
            let antenna_spacing = wavelength / 2.0;

            let azimuth_rad = beam.azimuth_deg.to_radians();
            let elevation_rad = beam.elevation_deg.to_radians();

            for i in 0..self.num_antennas {
                // Uniform linear array steering vector
                let phase = 2.0 * core::f32::consts::PI *
                           (i as f32) * antenna_spacing *
                           f32::cos(azimuth_rad) * f32::sin(elevation_rad);

                weights.push(Complex::exp(-phase));
            }

            Ok(weights)
        } else {
            Err(MimoError::InvalidBeam)
        }
    }

    /// Apply beamforming
    pub fn apply_beamforming(&self, signal: &[Complex], weights: &[Complex]) -> Vec<Complex> {
        signal.iter().enumerate().map(|(i, &s)| {
            if i < weights.len() {
                Complex {
                    real: s.real * weights[i].real - s.imag * weights[i].imag,
                    imag: s.real * weights[i].imag + s.imag * weights[i].real,
                }
            } else {
                s
            }
        }).collect()
    }

    /// Switch beam
    pub fn switch_beam(&mut self, new_beam_id: u16) -> Result<(), MimoError> {
        if new_beam_id as usize >= self.beams.len() {
            return Err(MimoError::InvalidBeam);
        }

        self.active_beam = Some(new_beam_id);
        self.stats.beam_switches.fetch_add(1, Ordering::Relaxed);

        crate::log_info!("Beam switched to {}", new_beam_id);

        Ok(())
    }

    /// Get active beam
    pub fn get_active_beam(&self) -> Option<&BeamConfig> {
        self.active_beam.and_then(|id| self.beams.get(id as usize))
    }

    /// Get beamforming statistics
    pub fn get_stats(&self) -> (u64, u64, u64) {
        (
            self.stats.beams_formed.load(Ordering::Relaxed),
            self.stats.beam_switches.load(Ordering::Relaxed),
            self.stats.avg_gain_dbi.load(Ordering::Relaxed),
        )
    }
}

// ============================================================================
// SU-MIMO and MU-MIMO
// ============================================================================

/// SU-MIMO system
#[derive(Debug)]
pub struct SuMimoSystem {
    config: MimoConfig,
    precoder: Precoder,
    beamformer: Beamformer,
}

impl SuMimoSystem {
    /// Create new SU-MIMO system
    pub fn new(config: MimoConfig) -> Self {
        let num_rf_chains = config.num_tx_antennas;
        let precoder = Precoder::new(PrecodingType::MMSE);
        let beamformer = Beamformer::new(BeamformingType::Digital, config.num_tx_antennas, num_rf_chains);

        Self {
            config,
            precoder,
            beamformer,
        }
    }

    /// Initialize SU-MIMO
    pub fn initialize(&mut self) -> Result<(), MimoError> {
        // Generate beam codebook
        let num_beams = 8;
        self.beamformer.generate_codebook(num_beams)?;

        // Compute initial precoding matrix (will be updated with channel estimate)
        let channel = ChannelMatrix::new(self.config.num_tx_antennas, self.config.num_rx_antennas);
        self.precoder.compute_matrix(&channel)?;

        Ok(())
    }

    /// Transmit with SU-MIMO
    pub fn transmit(&mut self, data: &[Complex], channel: &ChannelMatrix) -> Result<Vec<Complex>, MimoError> {
        // Update precoding based on channel
        self.precoder.compute_matrix(channel)?;

        // Apply precoding
        if let Some(matrix) = self.precoder.get_matrix() {
            let precoded = matrix.apply(data);
            Ok(precoded)
        } else {
            Err(MimoError::PrecodingError)
        }
    }

    /// Receive with SU-MIMO
    pub fn receive(&mut self, signal: &[Complex], channel: &ChannelMatrix) -> Result<Vec<Complex>, MimoError> {
        // Maximum Ratio Combining (MRC)
        let num_rx = self.config.num_rx_antennas as usize;
        let num_streams = self.config.num_streams as usize;

        let mut combined = Vec::with_capacity(num_streams);

        for stream_idx in 0..num_streams {
            let mut sum_real = 0.0;
            let mut sum_imag = 0.0;

            for antenna_idx in 0..num_rx {
                if antenna_idx < signal.len() && stream_idx < channel.h[antenna_idx].len() {
                    let h_conj = Complex {
                        real: channel.h[antenna_idx][stream_idx].real,
                        imag: -channel.h[antenna_idx][stream_idx].imag,
                    };

                    let sig = signal[antenna_idx];
                    sum_real += h_conj.real * sig.real - h_conj.imag * sig.imag;
                    sum_imag += h_conj.real * sig.imag + h_conj.imag * sig.real;
                }
            }

            combined.push(Complex { real: sum_real, imag: sum_imag });
        }

        Ok(combined)
    }
}

/// MU-MIMO user
#[derive(Debug, Clone)]
pub struct MuMimoUser {
    /// User ID
    pub user_id: u64,
    /// Number of antennas
    pub num_antennas: u16,
    /// Channel state
    pub channel: Option<ChannelMatrix>,
    /// Allocated streams
    pub num_streams: u8,
    /// SINR (dB, scaled by 10)
    pub sinr_db: i16,
}

/// MU-MIMO system
#[derive(Debug)]
pub struct MuMimoSystem {
    config: MimoConfig,
    users: Vec<MuMimoUser>,
    max_users: usize,
    scheduler: MuMimoScheduler,
}

/// MU-MIMO scheduler
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MuMimoScheduler {
    /// Round Robin
    RoundRobin,
    /// Maximum SINR
    MaxSinr,
    /// Proportional Fair
    ProportionalFair,
    /// Zero-Forcing
    ZeroForcing,
}

impl MuMimoSystem {
    /// Create new MU-MIMO system
    pub fn new(config: MimoConfig, max_users: usize, scheduler: MuMimoScheduler) -> Self {
        Self {
            config,
            users: Vec::new(),
            max_users,
            scheduler,
        }
    }

    /// Add user
    pub fn add_user(&mut self, user: MuMimoUser) -> Result<(), MimoError> {
        if self.users.len() >= self.max_users {
            return Err(MimoError::MaxUsersExceeded);
        }

        self.users.push(user);
        Ok(())
    }

    /// Schedule users for transmission
    pub fn schedule_users(&mut self) -> Result<Vec<usize>, MimoError> {
        let num_streams = self.config.num_streams as usize;
        let mut selected = Vec::new();

        match self.scheduler {
            MuMimoScheduler::RoundRobin => {
                // Simple round-robin based on user order
                for i in 0..self.users.len().min(num_streams) {
                    selected.push(i);
                }
            }
            MuMimoScheduler::MaxSinr => {
                // Sort by SINR and select top users
                let mut sorted_users: Vec<_> = self.users.iter().enumerate().collect();
                sorted_users.sort_by(|a, b| b.1.sinr_db.cmp(&a.1.sinr_db));

                for (idx, _) in sorted_users.into_iter().take(num_streams) {
                    selected.push(idx);
                }
            }
            MuMimoScheduler::ProportionalFair => {
                // Balance between SINR and fairness
                let mut scores: Vec<(usize, f32)> = self.users.iter().enumerate().map(|(i, u)| {
                    let sinr = 10.0_f32.powf(u.sinr_db as f32 / 100.0);
                    let metric = sinr / (i as f32 + 1.0); // Simple fairness weight
                    (i, metric)
                }).collect();

                scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

                for (idx, _) in scores.into_iter().take(num_streams) {
                    selected.push(idx);
                }
            }
            MuMimoScheduler::ZeroForcing => {
                // Semi-orthogonal user selection
                // Simplified: select users with uncorrelated channels
                for i in 0..self.users.len().min(num_streams) {
                    let mut orthogonal = true;
                    for &selected_idx in &selected {
                        if let (Some(ch1), Some(ch2)) = (&self.users[i].channel, &self.users[selected_idx].channel) {
                            // Check correlation (simplified)
                            let correlation = self.calculate_correlation(ch1, ch2);
                            if correlation > 0.3 {
                                orthogonal = false;
                                break;
                            }
                        }
                    }

                    if orthogonal {
                        selected.push(i);
                    }
                }
            }
        }

        Ok(selected)
    }

    /// Calculate channel correlation
    fn calculate_correlation(&self, ch1: &ChannelMatrix, ch2: &ChannelMatrix) -> f32 {
        // Simplified correlation calculation
        let mut sum = 0.0;
        let min_rx = ch1.num_rx.min(ch2.num_rx) as usize;
        let min_tx = ch1.num_tx.min(ch2.num_tx) as usize;

        for i in 0..min_rx {
            for j in 0..min_tx {
                sum += ch1.h[i][j].real * ch2.h[i][j].real +
                       ch1.h[i][j].imag * ch2.h[i][j].imag;
            }
        }

        let norm = (min_rx * min_tx) as f32;
        if norm > 0.0 {
            sum / norm
        } else {
            0.0
        }
    }

    /// Get user scheduling statistics
    pub fn get_user_stats(&self) -> Vec<(u64, u8, i16)> {
        self.users.iter().map(|u| (u.user_id, u.num_streams, u.sinr_db)).collect()
    }
}

// ============================================================================
// MIMO Errors
// ============================================================================

/// MIMO errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MimoError {
    InvalidBeam,
    InvalidConfiguration,
    PrecodingError,
    ChannelEstimationError,
    MaxUsersExceeded,
    InsufficientAntennas,
}

// ============================================================================
// Default Implementations
// ============================================================================

impl Default for MimoConfig {
    fn default() -> Self {
        Self {
            mode: MimoMode::SuMimo,
            num_tx_antennas: 4,
            num_rx_antennas: 4,
            num_streams: 4,
            transmission_scheme: TransmissionScheme::SpatialMultiplexing,
        }
    }
}

impl Default for ChannelMatrix {
    fn default() -> Self {
        Self::new(4, 4)
    }
}
