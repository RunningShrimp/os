//! Satellite Communication Common Types and Error Handling
//!
//! This module provides common data structures, error types, and constants
//! used across all satellite communication subsystems.
//!
//! # Standards Compliance
//! - ITU-R recommendations for satellite communication
//! - CCSDS (Consultative Committee for Space Data Systems) standards
//! - IETF RFCs for satellite network protocols

#![allow(dead_code)]

extern crate alloc;

use alloc::string::String;
use core::fmt;
use core::time::Duration;

// Use libm for mathematical functions in no_std environment
use libm::{cos, sin, sqrt, pow, log10, atan2};

// ============================================================================
// Error Types
// ============================================================================

/// Comprehensive error type for satellite communication operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SatComError {
    /// Signal quality is below minimum threshold
    SignalTooWeak,

    /// Signal exceeds maximum allowable strength
    SignalTooStrong,

    /// Carrier frequency offset too large
    FrequencyOffsetExceeded,

    /// Doppler shift compensation failed
    DopplerCompensationFailed,

    /// Signal synchronization failed
    SynchronizationFailed,

    /// Frame/checksum error
    ChecksumError,

    /// Bit error rate exceeded threshold
    BitErrorRateExceeded,

    /// Modulation/demodulation error
    ModulationError,

    /// Encoding/decoding error
    CodingError,

    /// Hardware device error
    HardwareError(String),

    /// Timeout occurred
    Timeout,

    /// Invalid parameter
    InvalidParameter(String),

    /// Buffer underrun/overrun
    BufferError,

    /// Correlation peak not found
    CorrelationNotFound,

    /// Tracking loop lost lock
    LockLost,

    /// Ephemeris data not available
    EphemerisNotAvailable,

    /// Almanac data not available
    AlmanacNotAvailable,

    /// Insufficient satellites for fix
    InsufficientSatellites,

    /// Position solution invalid
    PositionSolutionInvalid,

    /// Network/routing error
    NetworkError(String),

    /// Protocol error
    ProtocolError(String),

    /// Resource not available
    ResourceNotAvailable,

    /// Operation not supported
    NotSupported,

    /// Internal error
    InternalError(String),
}

impl fmt::Display for SatComError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SatComError::SignalTooWeak => write!(f, "Signal strength below minimum threshold"),
            SatComError::SignalTooStrong => write!(f, "Signal strength exceeds maximum"),
            SatComError::FrequencyOffsetExceeded => write!(f, "Carrier frequency offset too large"),
            SatComError::DopplerCompensationFailed => write!(f, "Doppler shift compensation failed"),
            SatComError::SynchronizationFailed => write!(f, "Signal synchronization failed"),
            SatComError::ChecksumError => write!(f, "Checksum validation failed"),
            SatComError::BitErrorRateExceeded => write!(f, "Bit error rate exceeded threshold"),
            SatComError::ModulationError => write!(f, "Modulation/demodulation error"),
            SatComError::CodingError => write!(f, "Forward error coding/decoding error"),
            SatComError::HardwareError(msg) => write!(f, "Hardware error: {}", msg),
            SatComError::Timeout => write!(f, "Operation timed out"),
            SatComError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            SatComError::BufferError => write!(f, "Buffer underrun/overrun"),
            SatComError::CorrelationNotFound => write!(f, "Correlation peak not found"),
            SatComError::LockLost => write!(f, "Tracking loop lost lock"),
            SatComError::EphemerisNotAvailable => write!(f, "Ephemeris data not available"),
            SatComError::AlmanacNotAvailable => write!(f, "Almanac data not available"),
            SatComError::InsufficientSatellites => write!(f, "Insufficient satellites for fix"),
            SatComError::PositionSolutionInvalid => write!(f, "Position solution invalid"),
            SatComError::NetworkError(msg) => write!(f, "Network error: {}", msg),
            SatComError::ProtocolError(msg) => write!(f, "Protocol error: {}", msg),
            SatComError::ResourceNotAvailable => write!(f, "Resource not available"),
            SatComError::NotSupported => write!(f, "Operation not supported"),
            SatComError::InternalError(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

/// Result type for satellite operations
pub type SatResult<T> = Result<T, SatComError>;

// ============================================================================
// Physical Constants (ITU-R and CCSDS standards)
// ============================================================================

/// Speed of light in vacuum (m/s) - exact definition
pub const C: f64 = 299_792_458.0;

/// Earth's gravitational constant (m³/s²)
pub const MU_EARTH: f64 = 3_986_004_418_000.0;

/// Earth's rotation rate (rad/s)
pub const OMEGA_EARTH: f64 = 7.292_115_146_7e-5;

/// Earth's equatorial radius (m)
pub const R_EARTH: f64 = 6_378_137.0;

/// Earth's polar radius (m)
pub const R_EARTH_POLAR: f64 = 6_356_752.314_245;

/// GPS semi-major axis (m)
pub const A_GPS: f64 = 26_560_000.0;

/// GPS orbital period (s)
pub const T_GPS: f64 = 43_080.0;

/// L1 carrier frequency (Hz) - GPS
pub const F_L1: f64 = 1_575_420_000.0;

/// L2 carrier frequency (Hz) - GPS
pub const F_L2: f64 = 1_227_600_000.0;

/// L5 carrier frequency (Hz) - GPS
pub const F_L5: f64 = 1_176_450_000.0;

/// C/A code chipping rate (Hz)
pub const CA_CODE_RATE: f64 = 1_023_000.0;

/// P(Y) code chipping rate (Hz)
pub const P_CODE_RATE: f64 = 10_230_000.0;

/// Boltzmann constant (J/K)
pub const K_BOLTZMANN: f64 = 1.380_649e-23;

/// Planck constant (J·s)
pub const H_PLANCK: f64 = 6.626_070_15e-34;

// ============================================================================
// Signal Quality Metrics
// ============================================================================

/// Signal-to-noise ratio (dB)
pub type SNR = f64;

/// Carrier-to-noise density ratio (dB-Hz)
pub type Cn0 = f64;

/// Bit error rate
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Ber(pub f64);

impl Ber {
    /// Create new BER value with validation
    pub fn new(value: f64) -> Self {
        debug_assert!(value >= 0.0 && value <= 1.0, "BER must be between 0 and 1");
        Ber(value.clamp(0.0, 1.0))
    }

    /// Check if BER is acceptable for given modulation
    pub fn is_acceptable(&self, threshold: f64) -> bool {
        self.0 < threshold
    }

    /// Convert to dB
    #[inline]
    pub fn to_db(&self) -> f64 {
        10.0 * log10(self.0)
    }
}

/// Signal quality indicators
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignalQuality {
    Excellent,
    Good,
    Fair,
    Poor,
    Unusable,
}

impl SignalQuality {
    /// Determine quality from CN0
    pub fn from_cn0(cn0_db_hz: f64) -> Self {
        match cn0_db_hz {
            x if x >= 50.0 => SignalQuality::Excellent,
            x if x >= 45.0 => SignalQuality::Good,
            x if x >= 35.0 => SignalQuality::Fair,
            x if x >= 25.0 => SignalQuality::Poor,
            _ => SignalQuality::Unusable,
        }
    }

    /// Get minimum CN0 threshold for this quality level
    pub fn min_cn0(&self) -> f64 {
        match self {
            SignalQuality::Excellent => 50.0,
            SignalQuality::Good => 45.0,
            SignalQuality::Fair => 35.0,
            SignalQuality::Poor => 25.0,
            SignalQuality::Unusable => 0.0,
        }
    }
}

// ============================================================================
// Modulation Schemes
// ============================================================================

/// Digital modulation schemes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModulationScheme {
    /// Binary Phase Shift Keying
    BPSK,

    /// Quadrature Phase Shift Keying
    QPSK,

    /// 8-ary Phase Shift Keying
    PSK8,

    /// 16-ary Quadrature Amplitude Modulation
    QAM16,

    /// 32-ary Quadrature Amplitude Modulation
    QAM32,

    /// 64-ary Quadrature Amplitude Modulation
    QAM64,

    /// 256-ary Quadrature Amplitude Modulation
    QAM256,

    /// Offset QPSK
    OQPSK,

    /// Minimum Shift Keying
    MSK,

    /// Gaussian Minimum Shift Keying
    GMSK,

    /// Frequency Shift Keying
    FSK,
}

impl ModulationScheme {
    /// Get spectral efficiency (bits/s/Hz)
    pub fn spectral_efficiency(&self) -> f64 {
        match self {
            ModulationScheme::BPSK => 1.0,
            ModulationScheme::QPSK => 2.0,
            ModulationScheme::PSK8 => 3.0,
            ModulationScheme::QAM16 => 4.0,
            ModulationScheme::QAM32 => 5.0,
            ModulationScheme::QAM64 => 6.0,
            ModulationScheme::QAM256 => 8.0,
            ModulationScheme::OQPSK => 2.0,
            ModulationScheme::MSK => 1.0,
            ModulationScheme::GMSK => 1.0,
            ModulationScheme::FSK => 1.0,
        }
    }

    /// Get required Eb/N0 for BER=1e-5 (approximate)
    pub fn required_eb_n0(&self) -> f64 {
        match self {
            ModulationScheme::BPSK => 9.6,
            ModulationScheme::QPSK => 9.6,
            ModulationScheme::PSK8 => 14.5,
            ModulationScheme::QAM16 => 18.5,
            ModulationScheme::QAM32 => 21.5,
            ModulationScheme::QAM64 => 24.5,
            ModulationScheme::QAM256 => 30.5,
            ModulationScheme::OQPSK => 9.6,
            ModulationScheme::MSK => 9.6,
            ModulationScheme::GMSK => 10.5,
            ModulationScheme::FSK => 13.5,
        }
    }
}

// ============================================================================
// Forward Error Correction Codes
// ============================================================================

/// Forward error correction schemes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FecScheme {
    /// No coding
    None,

    /// Convolutional coding (rate 1/2)
    Convolutional_1_2,

    /// Convolutional coding (rate 1/3)
    Convolutional_1_3,

    /// Convolutional coding (rate 2/3)
    Convolutional_2_3,

    /// Turbo coding (rate 1/2)
    Turbo_1_2,

    /// Turbo coding (rate 1/3)
    Turbo_1_3,

    /// LDPC coding
    Ldpc,

    /// Reed-Solomon
    ReedSolomon,

    /// Concatenated (RS + Convolutional)
    Concatenated,
}

impl FecScheme {
    /// Get code rate
    pub fn code_rate(&self) -> f64 {
        match self {
            FecScheme::None => 1.0,
            FecScheme::Convolutional_1_2 | FecScheme::Turbo_1_2 => 0.5,
            FecScheme::Convolutional_1_3 | FecScheme::Turbo_1_3 => 0.333,
            FecScheme::Convolutional_2_3 => 0.667,
            FecScheme::Ldpc | FecScheme::ReedSolomon | FecScheme::Concatenated => 0.5,
        }
    }

    /// Get coding gain at BER=1e-5 (dB)
    pub fn coding_gain(&self) -> f64 {
        match self {
            FecScheme::None => 0.0,
            FecScheme::Convolutional_1_2 => 5.0,
            FecScheme::Convolutional_1_3 => 5.5,
            FecScheme::Convolutional_2_3 => 4.5,
            FecScheme::Turbo_1_2 => 7.0,
            FecScheme::Turbo_1_3 => 7.5,
            FecScheme::Ldpc => 8.0,
            FecScheme::ReedSolomon => 4.0,
            FecScheme::Concatenated => 9.0,
        }
    }
}

// ============================================================================
// Position and Velocity
// ============================================================================

/// Geodetic position (WGS84)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeodeticPosition {
    /// Latitude (degrees)
    pub lat: f64,

    /// Longitude (degrees)
    pub lon: f64,

    /// Altitude above WGS84 ellipsoid (meters)
    pub alt: f64,
}

impl GeodeticPosition {
    /// Create new geodetic position
    pub fn new(lat: f64, lon: f64, alt: f64) -> Self {
        debug_assert!(lat >= -90.0 && lat <= 90.0);
        debug_assert!(lon >= -180.0 && lon <= 180.0);
        Self { lat, lon, alt }
    }

    /// Convert to ECEF coordinates
    pub fn to_ecef(&self) -> EcefPosition {
        let lat_rad = self.lat * core::f64::consts::PI / 180.0;
        let lon_rad = self.lon * core::f64::consts::PI / 180.0;

        // WGS84 ellipsoid parameters
        let a = R_EARTH;
        let f = 1.0 / 298.257_223_563;
        let e2 = 2.0 * f - f * f;

        let sin_lat = sin(lat_rad);
        let n = a / sqrt(1.0 - e2 * sin_lat * sin_lat);

        let x = (n + self.alt) * cos(lat_rad) * cos(lon_rad);
        let y = (n + self.alt) * cos(lat_rad) * sin(lon_rad);
        let z = (n * (1.0 - e2) + self.alt) * sin(lat_rad);

        EcefPosition { x, y, z }
    }
}

/// Earth-Centered Earth-Fixed position
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EcefPosition {
    /// X coordinate (meters)
    pub x: f64,

    /// Y coordinate (meters)
    pub y: f64,

    /// Z coordinate (meters)
    pub z: f64,
}

impl EcefPosition {
    /// Create new ECEF position
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Convert to geodetic coordinates
    pub fn to_geodetic(&self) -> GeodeticPosition {
        // WGS84 ellipsoid parameters
        let a = R_EARTH;
        let f = 1.0 / 298.257_223_563;
        let e2 = 2.0 * f - f * f;

        let p = sqrt(self.x * self.x + self.y * self.y);
        let theta = atan2(self.z * a, p * (1.0 - f) * sqrt(a * a - self.z * self.z));

        // Iterative solution for latitude
        let sin_theta = sin(theta);
        let cos_theta = cos(theta);
        let mut lat = atan2(
            self.z + e2 * (1.0 - f) * a * sin_theta * sin_theta * sin_theta,
            p - e2 * a * cos_theta * cos_theta * cos_theta,
        );

        // Refine latitude (2 iterations)
        for _ in 0..2 {
            let sin_lat = sin(lat);
            let n = a / sqrt(1.0 - e2 * sin_lat * sin_lat);
            lat = atan2(self.z + e2 * n * sin(lat), p);
        }

        let lon = atan2(self.y, self.x);
        let n = a / sqrt(1.0 - e2 * pow(sin(lat), 2.0));
        let alt = p / cos(lat) - n;

        GeodeticPosition {
            lat: lat * 180.0 / core::f64::consts::PI,
            lon: lon * 180.0 / core::f64::consts::PI,
            alt,
        }
    }

    /// Calculate distance to another ECEF position
    pub fn distance_to(&self, other: &EcefPosition) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        sqrt(dx * dx + dy * dy + dz * dz)
    }
}

/// Velocity in ECEF coordinates
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Velocity {
    /// X velocity (m/s)
    pub vx: f64,

    /// Y velocity (m/s)
    pub vy: f64,

    /// Z velocity (m/s)
    pub vz: f64,
}

impl Velocity {
    /// Create new velocity
    pub fn new(vx: f64, vy: f64, vz: f64) -> Self {
        Self { vx, vy, vz }
    }

    /// Calculate speed magnitude
    pub fn speed(&self) -> f64 {
        sqrt(self.vx * self.vx + self.vy * self.vy + self.vz * self.vz)
    }
}

// ============================================================================
// Timing and Synchronization
// ============================================================================

/// High-precision timestamp (nanoseconds since epoch)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    nanos: u64,
}

impl Timestamp {
    /// Create timestamp from nanoseconds
    pub fn from_nanos(nanos: u64) -> Self {
        Self { nanos }
    }

    /// Create timestamp from seconds
    pub fn from_secs(secs: f64) -> Self {
        Self {
            nanos: (secs * 1e9) as u64,
        }
    }

    /// Get nanoseconds
    pub fn as_nanos(&self) -> u64 {
        self.nanos
    }

    /// Get seconds
    pub fn as_secs(&self) -> f64 {
        self.nanos as f64 / 1e9
    }

    /// Get current timestamp
    pub fn now() -> Self {
        Self {
            nanos: crate::subsystems::time::timestamp_nanos(),
        }
    }

    /// Calculate time difference
    pub fn duration_since(&self, earlier: &Timestamp) -> Duration {
        Duration::from_nanos(self.nanos.saturating_sub(earlier.nanos))
    }
}

// ============================================================================
// Satellite Orbital Elements
// ============================================================================

/// Keplerian orbital elements
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OrbitalElements {
    /// Semi-major axis (m)
    pub a: f64,

    /// Eccentricity
    pub e: f64,

    /// Inclination (rad)
    pub i: f64,

    /// Right ascension of ascending node (rad)
    pub raan: f64,

    /// Argument of periapsis (rad)
    pub argp: f64,

    /// True anomaly (rad)
    pub nu: f64,

    /// Epoch timestamp
    pub epoch: Timestamp,
}

impl OrbitalElements {
    /// Calculate orbital period
    pub fn period(&self) -> f64 {
        2.0 * core::f64::consts::PI * sqrt(pow(self.a, 3.0) / MU_EARTH)
    }

    /// Calculate position from orbital elements
    pub fn to_ecef(&self) -> EcefPosition {
        // Calculate distance from central body
        let r = self.a * (1.0 - self.e * self.e) / (1.0 + self.e * cos(self.nu));

        // Position in orbital plane
        let x_orb = r * cos(self.nu);
        let y_orb = r * sin(self.nu);

        // Rotate to ECEF
        let cos_raan = cos(self.raan);
        let sin_raan = sin(self.raan);
        let cos_argp = cos(self.argp);
        let sin_argp = sin(self.argp);
        let cos_i = cos(self.i);
        let sin_i = sin(self.i);

        let x = x_orb * (cos_raan * cos_argp - sin_raan * sin_argp * cos_i)
            - y_orb * (cos_raan * sin_argp + sin_raan * cos_argp * cos_i);

        let y = x_orb * (sin_raan * cos_argp + cos_raan * sin_argp * cos_i)
            - y_orb * (sin_raan * sin_argp - cos_raan * cos_argp * cos_i);

        let z = x_orb * (sin_argp * sin_i) + y_orb * (cos_argp * sin_i);

        EcefPosition { x, y, z }
    }
}

// ============================================================================
// RF Parameters
// ============================================================================

/// Radio frequency configuration
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RfConfig {
    /// Center frequency (Hz)
    pub center_freq: f64,

    /// Bandwidth (Hz)
    pub bandwidth: f64,

    /// Transmit power (dBm)
    pub tx_power: f64,

    /// Receive gain (dB)
    pub rx_gain: f64,

    /// Noise figure (dB)
    pub noise_figure: f64,
}

impl RfConfig {
    /// Create new RF configuration
    pub fn new(center_freq: f64, bandwidth: f64) -> Self {
        Self {
            center_freq,
            bandwidth,
            tx_power: 0.0,
            rx_gain: 0.0,
            noise_figure: 5.0,
        }
    }

    /// Calculate wavelength (m)
    pub fn wavelength(&self) -> f64 {
        C / self.center_freq
    }

    /// Calculate thermal noise power (dBm)
    pub fn thermal_noise_power(&self, temperature: f64) -> f64 {
        // kTB in dBm
        let tb = K_BOLTZMANN * temperature * self.bandwidth;
        10.0 * log10(tb / 0.001)
    }

    /// Calculate free space path loss (dB)
    pub fn free_space_path_loss(&self, distance: f64) -> f64 {
        let lambda = self.wavelength();
        20.0 * log10(4.0 * core::f64::consts::PI * distance / lambda)
    }
}

// ============================================================================
// Network Statistics
// ============================================================================

/// Satellite link statistics
#[derive(Debug, Clone, PartialEq)]
pub struct LinkStatistics {
    /// Total bytes transmitted
    pub tx_bytes: u64,

    /// Total bytes received
    pub rx_bytes: u64,

    /// Total packets transmitted
    pub tx_packets: u64,

    /// Total packets received
    pub rx_packets: u64,

    /// Packets with errors
    pub error_packets: u64,

    /// Packets retransmitted
    pub retransmit_packets: u64,

    /// Current signal quality
    pub signal_quality: SignalQuality,

    /// Current CN0 (dB-Hz)
    pub cn0: Cn0,

    /// Average bit error rate
    pub average_ber: Ber,

    /// Round-trip time
    pub rtt: Duration,
}

impl LinkStatistics {
    /// Create new statistics
    pub fn new() -> Self {
        Self {
            tx_bytes: 0,
            rx_bytes: 0,
            tx_packets: 0,
            rx_packets: 0,
            error_packets: 0,
            retransmit_packets: 0,
            signal_quality: SignalQuality::Unusable,
            cn0: 0.0,
            average_ber: Ber(0.0),
            rtt: Duration::from_secs(0),
        }
    }

    /// Calculate packet error rate
    pub fn packet_error_rate(&self) -> f64 {
        if self.rx_packets == 0 {
            return 0.0;
        }
        self.error_packets as f64 / self.rx_packets as f64
    }

    /// Calculate throughput (bytes/s)
    pub fn throughput(&self) -> f64 {
        let total_bytes = self.tx_bytes + self.rx_bytes;
        let duration_secs = self.rtt.as_secs_f64();
        if duration_secs > 0.0 {
            total_bytes as f64 / duration_secs
        } else {
            0.0
        }
    }
}

impl Default for LinkStatistics {
    fn default() -> Self {
        Self::new()
    }
}
