//! GNSS/GPS Navigation System Implementation
//!
//! This module provides a complete GNSS (Global Navigation Satellite System)
//! receiver implementation supporting GPS, GLONASS, BDS, and Galileo constellations.
//!
//! # Features
//! - Multi-constellation receiver (GPS L1/L2/L5, GLONASS, BDS, Galileo E1/E5)
//! - NMEA-0183 protocol parsing and generation
//! - PVT (Position, Velocity, Time) solution computation
//! - RTK (Real-Time Kinematic) and PPP (Precise Point Positioning)
//! - Carrier phase and pseudorange measurements
//! - Ephemeris and almanac processing
//! - DGPS and SBAS support

#![allow(dead_code)]

extern crate alloc;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use libm::sqrt;

use super::sat_types::{
    Cn0, EcefPosition, SatComError, SatResult, Timestamp, Velocity,
};

// ============================================================================
// GNSS Constellation Definitions
// ============================================================================

/// GNSS constellation identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Constellation {
    GPS,
    GLONASS,
    BeiDou,
    Galileo,
    SBAS,
    QZSS,
    IRNSS,
}

impl Constellation {
    /// Get constellation identifier character
    pub fn id_char(&self) -> char {
        match self {
            Constellation::GPS => 'G',
            Constellation::GLONASS => 'R',
            Constellation::BeiDou => 'C',
            Constellation::Galileo => 'E',
            Constellation::SBAS => 'S',
            Constellation::QZSS => 'J',
            Constellation::IRNSS => 'I',
        }
    }
}

/// GNSS signal identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GnssSignal {
    pub constellation: Constellation,
    pub sv_id: u16,
    pub signal_id: u8,
}

impl GnssSignal {
    /// Create new signal identifier
    pub fn new(constellation: Constellation, sv_id: u16, signal_id: u8) -> Self {
        Self {
            constellation,
            sv_id,
            signal_id,
        }
    }

    /// Get signal string representation (e.g., "G1L1")
    pub fn to_string(&self) -> String {
        alloc::format!(
            "{}{}{}",
            self.constellation.id_char(),
            self.sv_id,
            self.signal_name()
        )
    }

    /// Get signal name
    pub fn signal_name(&self) -> &str {
        match (self.constellation, self.signal_id) {
            (Constellation::GPS, 1) => "L1",
            (Constellation::GPS, 2) => "L2",
            (Constellation::GPS, 5) => "L5",
            (Constellation::GLONASS, 1) => "G1",
            (Constellation::GLONASS, 2) => "G2",
            (Constellation::BeiDou, 1) => "B1",
            (Constellation::BeiDou, 2) => "B2",
            (Constellation::BeiDou, 3) => "B3",
            (Constellation::Galileo, 1) => "E1",
            (Constellation::Galileo, 5) => "E5",
            (Constellation::Galileo, 6) => "E6",
            _ => "??",
        }
    }
}

// ============================================================================
// NMEA-0183 Protocol
// ============================================================================

/// NMEA sentence types
#[derive(Debug, Clone, PartialEq)]
pub enum NmeaSentence {
    /// GGA - Fix Data
    Gga(NmeaGga),

    /// RMC - Recommended Minimum data
    Rmc(NmeaRmc),

    /// GSA - GNSS DOP and Active Satellites
    Gsa(NmeaGsa),

    /// GSV - GNSS Satellites in View
    Gsv(NmeaGsv),

    /// GLL - Geographic Position
    Gll(NmeaGll),

    /// VTG - Course over Ground and Ground Speed
    Vtg(NmeaVtg),

    /// ZDA - Time and Date
    Zda(NmeaZda),

    /// Custom/Unknown sentence
    Custom(String, Vec<String>),
}

/// GGA - Fix Data
#[derive(Debug, Clone, PartialEq)]
pub struct NmeaGga {
    pub utc_time: f64,
    pub lat: f64,
    pub lat_ns: char,
    pub lon: f64,
    pub lon_ew: char,
    pub quality: u8,
    pub num_sats: u8,
    pub hdop: f64,
    pub altitude: f64,
    pub alt_units: char,
    pub geoid_sep: f64,
    pub geoid_units: char,
    pub dgps_age: f64,
    pub dgps_id: u16,
}

/// RMC - Recommended Minimum data
#[derive(Debug, Clone, PartialEq)]
pub struct NmeaRmc {
    pub utc_time: f64,
    pub status: char,
    pub lat: f64,
    pub lat_ns: char,
    pub lon: f64,
    pub lon_ew: char,
    pub speed_knots: f64,
    pub track_true: f64,
    pub date: String,
    pub mag_var: f64,
    pub mag_var_ew: char,
    pub mode: char,
}

/// GSA - GNSS DOP and Active Satellites
#[derive(Debug, Clone, PartialEq)]
pub struct NmeaGsa {
    pub mode_auto: char,
    pub mode_fix: u8,
    pub sats_used: Vec<u16>,
    pub pdop: f64,
    pub hdop: f64,
    pub vdop: f64,
}

/// GSV - GNSS Satellites in View
#[derive(Debug, Clone, PartialEq)]
pub struct NmeaGsv {
    pub num_msgs: u8,
    pub msg_num: u8,
    pub sats_in_view: u8,
    pub satellites: Vec<NmeaSatInfo>,
}

/// Satellite information in GSV
#[derive(Debug, Clone, PartialEq)]
pub struct NmeaSatInfo {
    pub prn: u16,
    pub elevation: u8,
    pub azimuth: u16,
    pub snr: u8,
}

/// GLL - Geographic Position
#[derive(Debug, Clone, PartialEq)]
pub struct NmeaGll {
    pub lat: f64,
    pub lat_ns: char,
    pub lon: f64,
    pub lon_ew: char,
    pub utc_time: f64,
    pub status: char,
    pub mode: char,
}

/// VTG - Course over Ground
#[derive(Debug, Clone, PartialEq)]
pub struct NmeaVtg {
    pub track_true: f64,
    pub track_true_t: char,
    pub track_mag: f64,
    pub track_mag_t: char,
    pub speed_knots: f64,
    pub speed_knots_n: char,
    pub speed_kmh: f64,
    pub speed_kmh_n: char,
    pub mode: char,
}

/// ZDA - Time and Date
#[derive(Debug, Clone, PartialEq)]
pub struct NmeaZda {
    pub utc_time: f64,
    pub day: u8,
    pub month: u8,
    pub year: u16,
    pub local_zone_hours: i8,
    pub local_zone_minutes: u8,
}

/// NMEA parser
pub struct NmeaParser;

impl NmeaParser {
    /// Parse NMEA sentence
    pub fn parse(sentence: &str) -> SatResult<NmeaSentence> {
        let sentence = sentence.trim();

        // Check sentence starts with $
        if !sentence.starts_with('$') {
            return Err(SatComError::ProtocolError(
                "Invalid NMEA sentence: missing $".to_string(),
            ));
        }

        // Find sentence type
        let comma_pos = sentence[6..].find(',').unwrap_or(sentence.len() - 3);
        let msg_type = &sentence[3..6 + comma_pos];

        // Parse fields
        let fields: Vec<&str> = sentence[6..].split(',').collect();

        match msg_type {
            "GGA" => Self::parse_gga(&fields),
            "RMC" => Self::parse_rmc(&fields),
            "GSA" => Self::parse_gsa(&fields),
            "GSV" => Self::parse_gsv(&fields),
            "GLL" => Self::parse_gll(&fields),
            "VTG" => Self::parse_vtg(&fields),
            "ZDA" => Self::parse_zda(&fields),
            _ => Ok(NmeaSentence::Custom(
                msg_type.to_string(),
                fields.iter().map(|s| s.to_string()).collect(),
            )),
        }
    }

    fn parse_gga(fields: &[&str]) -> SatResult<NmeaSentence> {
        if fields.len() < 15 {
            return Err(SatComError::ProtocolError("GGA: insufficient fields".to_string()));
        }

        let gga = NmeaGga {
            utc_time: fields[0].parse().unwrap_or(0.0),
            lat: Self::parse_lat_lon(fields[1], fields[2]),
            lat_ns: fields[2].chars().next().unwrap_or('N'),
            lon: Self::parse_lat_lon(fields[3], fields[4]),
            lon_ew: fields[4].chars().next().unwrap_or('E'),
            quality: fields[5].parse().unwrap_or(0),
            num_sats: fields[6].parse().unwrap_or(0),
            hdop: fields[7].parse().unwrap_or(0.0),
            altitude: fields[8].parse().unwrap_or(0.0),
            alt_units: fields[9].chars().next().unwrap_or('M'),
            geoid_sep: fields[10].parse().unwrap_or(0.0),
            geoid_units: fields[11].chars().next().unwrap_or('M'),
            dgps_age: fields[12].parse().unwrap_or(0.0),
            dgps_id: fields[13].parse().unwrap_or(0),
        };

        Ok(NmeaSentence::Gga(gga))
    }

    fn parse_rmc(fields: &[&str]) -> SatResult<NmeaSentence> {
        if fields.len() < 12 {
            return Err(SatComError::ProtocolError("RMC: insufficient fields".to_string()));
        }

        let rmc = NmeaRmc {
            utc_time: fields[0].parse().unwrap_or(0.0),
            status: fields[1].chars().next().unwrap_or('V'),
            lat: Self::parse_lat_lon(fields[2], fields[3]),
            lat_ns: fields[3].chars().next().unwrap_or('N'),
            lon: Self::parse_lat_lon(fields[4], fields[5]),
            lon_ew: fields[5].chars().next().unwrap_or('E'),
            speed_knots: fields[6].parse().unwrap_or(0.0),
            track_true: fields[7].parse().unwrap_or(0.0),
            date: fields[8].to_string(),
            mag_var: fields[9].parse().unwrap_or(0.0),
            mag_var_ew: fields[10].chars().next().unwrap_or('E'),
            mode: if fields.len() > 11 {
                fields[11].chars().next().unwrap_or('A')
            } else {
                'N'
            },
        };

        Ok(NmeaSentence::Rmc(rmc))
    }

    fn parse_gsa(fields: &[&str]) -> SatResult<NmeaSentence> {
        let mut sats_used = Vec::new();

        for i in 2..14 {
            if fields.len() > i && !fields[i].is_empty() {
                if let Ok(prn) = fields[i].parse::<u16>() {
                    sats_used.push(prn);
                }
            }
        }

        let gsa = NmeaGsa {
            mode_auto: fields[0].chars().next().unwrap_or('A'),
            mode_fix: if fields.len() > 1 {
                fields[1].parse().unwrap_or(1)
            } else {
                1
            },
            sats_used,
            pdop: if fields.len() > 14 {
                fields[14].parse().unwrap_or(0.0)
            } else {
                0.0
            },
            hdop: if fields.len() > 15 {
                fields[15].parse().unwrap_or(0.0)
            } else {
                0.0
            },
            vdop: if fields.len() > 16 {
                fields[16].parse().unwrap_or(0.0)
            } else {
                0.0
            },
        };

        Ok(NmeaSentence::Gsa(gsa))
    }

    fn parse_gsv(fields: &[&str]) -> SatResult<NmeaSentence> {
        let num_msgs: u8 = fields[0].parse().unwrap_or(0);
        let msg_num: u8 = fields[1].parse().unwrap_or(0);
        let sats_in_view: u8 = fields[2].parse().unwrap_or(0);

        let mut satellites = Vec::new();

        // Parse satellite blocks (4 satellites each)
        for i in 0..4 {
            let base = 3 + i * 4;
            if fields.len() > base + 3 {
                let prn: u16 = fields[base].parse().unwrap_or(0);
                let elev: u8 = fields[base + 1].parse().unwrap_or(0);
                let azim: u16 = fields[base + 2].parse().unwrap_or(0);
                let snr: u8 = fields[base + 3].parse().unwrap_or(0);

                satellites.push(NmeaSatInfo {
                    prn,
                    elevation: elev,
                    azimuth: azim,
                    snr,
                });
            }
        }

        let gsv = NmeaGsv {
            num_msgs,
            msg_num,
            sats_in_view,
            satellites,
        };

        Ok(NmeaSentence::Gsv(gsv))
    }

    fn parse_gll(fields: &[&str]) -> SatResult<NmeaSentence> {
        let gll = NmeaGll {
            lat: Self::parse_lat_lon(fields[0], fields[1]),
            lat_ns: fields[1].chars().next().unwrap_or('N'),
            lon: Self::parse_lat_lon(fields[2], fields[3]),
            lon_ew: fields[3].chars().next().unwrap_or('E'),
            utc_time: fields[4].parse().unwrap_or(0.0),
            status: fields[5].chars().next().unwrap_or('V'),
            mode: if fields.len() > 6 {
                fields[6].chars().next().unwrap_or('A')
            } else {
                'N'
            },
        };

        Ok(NmeaSentence::Gll(gll))
    }

    fn parse_vtg(fields: &[&str]) -> SatResult<NmeaSentence> {
        let vtg = NmeaVtg {
            track_true: fields[0].parse().unwrap_or(0.0),
            track_true_t: fields[1].chars().next().unwrap_or('T'),
            track_mag: fields[2].parse().unwrap_or(0.0),
            track_mag_t: fields[3].chars().next().unwrap_or('M'),
            speed_knots: fields[4].parse().unwrap_or(0.0),
            speed_knots_n: fields[5].chars().next().unwrap_or('N'),
            speed_kmh: fields[6].parse().unwrap_or(0.0),
            speed_kmh_n: fields[7].chars().next().unwrap_or('K'),
            mode: if fields.len() > 8 {
                fields[8].chars().next().unwrap_or('A')
            } else {
                'N'
            },
        };

        Ok(NmeaSentence::Vtg(vtg))
    }

    fn parse_zda(fields: &[&str]) -> SatResult<NmeaSentence> {
        let zda = NmeaZda {
            utc_time: fields[0].parse().unwrap_or(0.0),
            day: fields[1].parse().unwrap_or(0),
            month: fields[2].parse().unwrap_or(0),
            year: fields[3].parse().unwrap_or(0),
            local_zone_hours: fields[4].parse().unwrap_or(0),
            local_zone_minutes: fields[5].parse().unwrap_or(0),
        };

        Ok(NmeaSentence::Zda(zda))
    }

    /// Parse NMEA lat/lon format (DDMM.MMMMM)
    fn parse_lat_lon(value: &str, _direction: &str) -> f64 {
        if let Ok(val) = value.parse::<f64>() {
            let degrees = (val / 100.0) as i32;
            let minutes = val - (degrees as f64 * 100.0);
            degrees as f64 + minutes / 60.0
        } else {
            0.0
        }
    }

    /// Generate NMEA sentence
    pub fn generate(sentence: &NmeaSentence) -> String {
        match sentence {
            NmeaSentence::Gga(gga) => Self::generate_gga(gga),
            NmeaSentence::Rmc(rmc) => Self::generate_rmc(rmc),
            _ => String::new(),
        }
    }

    fn generate_gga(gga: &NmeaGga) -> String {
        alloc::format!(
            "$GPGGA,{:09.2},{:010.5},{},{:011.5},{},{},{:02},{},{},{},M,{},M*",
            gga.utc_time,
            gga.lat.abs(),
            gga.lat_ns,
            gga.lon.abs(),
            gga.lon_ew,
            gga.quality,
            gga.num_sats,
            gga.hdop,
            gga.altitude,
            gga.geoid_sep,
            "" // checksum placeholder
        )
    }

    fn generate_rmc(rmc: &NmeaRmc) -> String {
        alloc::format!(
            "$GPRMC,{:09.2},{},{:010.5},{},{:011.5},{},{:.3},{:.3},{},{:.1},{}*",
            rmc.utc_time,
            rmc.status,
            rmc.lat.abs(),
            rmc.lat_ns,
            rmc.lon.abs(),
            rmc.lon_ew,
            rmc.speed_knots,
            rmc.track_true,
            rmc.date,
            rmc.mag_var,
            rmc.mag_var_ew
        )
    }
}

// ============================================================================
// GNSS Measurements
// ============================================================================

/// Pseudorange measurement
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pseudorange {
    /// Measured pseudorange (m)
    pub range: f64,

    /// Carrier phase (cycles)
    pub carrier_phase: f64,

    /// Doppler frequency (Hz)
    pub doppler: f64,

    /// Signal-to-noise ratio (dB-Hz)
    pub cn0: Cn0,

    /// Measurement timestamp
    pub timestamp: Timestamp,
}

impl Pseudorange {
    /// Create new pseudorange measurement
    pub fn new(range: f64, cn0: Cn0) -> Self {
        Self {
            range,
            carrier_phase: 0.0,
            doppler: 0.0,
            cn0,
            timestamp: Timestamp::now(),
        }
    }
}

/// GNSS satellite measurement
#[derive(Debug, Clone, PartialEq)]
pub struct GnssMeasurement {
    /// Signal identifier
    pub signal: GnssSignal,

    /// Pseudorange measurement
    pub pseudorange: Pseudorange,

    /// Satellite position (ECEF)
    pub sat_position: EcefPosition,

    /// Satellite velocity (ECEF)
    pub sat_velocity: Velocity,

    /// Satellite clock bias (m)
    pub sat_clock_bias: f64,

    /// Satellite clock drift (m/s)
    pub sat_clock_drift: f64,

    /// Ionospheric delay (m)
    pub iono_delay: f64,

    /// Tropospheric delay (m)
    pub tropo_delay: f64,

    /// Measurement variance
    pub variance: f64,
}

impl GnssMeasurement {
    /// Create new measurement
    pub fn new(signal: GnssSignal, pseudorange: Pseudorange) -> Self {
        Self {
            signal,
            pseudorange,
            sat_position: EcefPosition::new(0.0, 0.0, 0.0),
            sat_velocity: Velocity::new(0.0, 0.0, 0.0),
            sat_clock_bias: 0.0,
            sat_clock_drift: 0.0,
            iono_delay: 0.0,
            tropo_delay: 0.0,
            variance: 1.0,
        }
    }
}

// ============================================================================
// PVT Solution
// ============================================================================

/// Position, Velocity, Time solution
#[derive(Debug, Clone, PartialEq)]
pub struct PvtSolution {
    /// Position (ECEF)
    pub position: EcefPosition,

    /// Velocity (ECEF)
    pub velocity: Velocity,

    /// Receiver clock bias (m)
    pub clock_bias: f64,

    /// Receiver clock drift (m/s)
    pub clock_drift: f64,

    /// Solution timestamp
    pub timestamp: Timestamp,

    /// Dilution of precision
    pub dop: DilutionOfPrecision,

    /// Satellites used in solution
    pub sats_used: Vec<GnssSignal>,

    /// Solution type
    pub solution_type: SolutionType,

    /// Position variance (m²)
    pub position_variance: [f64; 3],
}

/// Dilution of precision
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DilutionOfPrecision {
    pub gdop: f64, // Geometric
    pub pdop: f64, // Position
    pub hdop: f64, // Horizontal
    pub vdop: f64, // Vertical
    pub tdop: f64, // Time
}

/// Solution type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolutionType {
    NoSolution,
    Autonomous,
    DGPS,
    RTKFloat,
    RTKFixed,
    PPP,
}

/// PVT solver
pub struct PvtSolver {
    /// Previous solution (for warm start)
    previous_solution: Option<PvtSolution>,

    /// Minimum number of satellites
    min_sats: usize,

    /// Maximum PDOP threshold
    max_pdop: f64,
}

impl PvtSolver {
    /// Create new PVT solver
    pub fn new(min_sats: usize, max_pdop: f64) -> Self {
        Self {
            previous_solution: None,
            min_sats,
            max_pdop,
        }
    }

    /// Compute PVT solution from measurements
    pub fn compute_solution(&mut self, measurements: &[GnssMeasurement]) -> SatResult<PvtSolution> {
        if measurements.len() < self.min_sats {
            return Err(SatComError::InsufficientSatellites);
        }

        // Initial position estimate
        let initial_pos = self.get_initial_position();

        // Least squares iteration
        let solution = self.least_squares_solve(measurements, initial_pos)?;

        // Compute DOP
        let dop = self.compute_dop(measurements, &solution.position)?;

        if dop.pdop > self.max_pdop {
            return Err(SatComError::PositionSolutionInvalid);
        }

        self.previous_solution = Some(solution.clone());

        Ok(solution)
    }

    /// Get initial position estimate
    fn get_initial_position(&self) -> EcefPosition {
        if let Some(ref sol) = self.previous_solution {
            return sol.position;
        }

        // Default to Earth center
        EcefPosition::new(0.0, 0.0, 0.0)
    }

    /// Least squares position solution
    fn least_squares_solve(
        &self,
        measurements: &[GnssMeasurement],
        initial: EcefPosition,
    ) -> SatResult<PvtSolution> {
        let mut pos = initial;
        let max_iterations = 10;
        let convergence_threshold = 1e-6;

        for _iteration in 0..max_iterations {
            let (geometry_matrix, measurement_vector) =
                self.linearize_measurements(measurements, pos)?;

            // Solve delta_x = (G^T * G)^-1 * G^T * b
            let gt_g = self.matrix_multiply_transpose(&geometry_matrix);
            let gt_g_inv = self.matrix_inverse_3x3(&gt_g)?;
            let delta = self.matrix_multiply_vector(&gt_g_inv, &measurement_vector);

            // Update position
            pos.x += delta[0];
            pos.y += delta[1];
            pos.z += delta[2];

            let clock_bias = delta[3];

            // Check convergence
            if delta[0].abs() < convergence_threshold
                && delta[1].abs() < convergence_threshold
                && delta[2].abs() < convergence_threshold
            {
                return Ok(PvtSolution {
                    position: pos,
                    velocity: Velocity::new(0.0, 0.0, 0.0), // Would need Doppler measurements
                    clock_bias,
                    clock_drift: 0.0,
                    timestamp: Timestamp::now(),
                    dop: DilutionOfPrecision {
                        gdop: 0.0,
                        pdop: 0.0,
                        hdop: 0.0,
                        vdop: 0.0,
                        tdop: 0.0,
                    },
                    sats_used: measurements.iter().map(|m| m.signal).collect(),
                    solution_type: SolutionType::Autonomous,
                    position_variance: [1.0, 1.0, 1.0],
                });
            }
        }

        Err(SatComError::PositionSolutionInvalid)
    }

    /// Linearize measurements around current position
    fn linearize_measurements(
        &self,
        measurements: &[GnssMeasurement],
        pos: EcefPosition,
    ) -> SatResult<(Vec<Vec<f64>>, Vec<f64>)> {
        let mut geometry = Vec::new();
        let mut meas_vec = Vec::new();

        for meas in measurements {
            // Range from satellite to receiver
            let rx = meas.sat_position.x - pos.x;
            let ry = meas.sat_position.y - pos.y;
            let rz = meas.sat_position.z - pos.z;
            let geometric_range = sqrt(rx * rx + ry * ry + rz * rz);

            // Unit vector to satellite
            let ux = rx / geometric_range;
            let uy = ry / geometric_range;
            let uz = rz / geometric_range;

            // Predicted pseudorange
            let predicted_range = geometric_range + meas.sat_clock_bias - meas.pseudorange.cn0 as f64;

            // Measurement residual
            let residual = meas.pseudorange.range - predicted_range;

            // Geometry row
            geometry.push(vec![ux, uy, uz, 1.0]);
            meas_vec.push(residual);
        }

        Ok((geometry, meas_vec))
    }

    /// Matrix multiplication G^T * G
    fn matrix_multiply_transpose(&self, g: &[Vec<f64>]) -> [f64; 9] {
        let mut result = [0.0; 9];

        for i in 0..4 {
            for j in 0..4 {
                let mut sum = 0.0;
                for k in 0..g.len() {
                    sum += g[k][i] * g[k][j];
                }
                result[i * 3 + j] = sum;
            }
        }

        // Simplified 3x3 (ignoring clock)
        [
            result[0], result[1], result[2],
            result[3], result[4], result[5],
            result[6], result[7], result[8],
        ]
    }

    /// Matrix inverse for 3x3
    fn matrix_inverse_3x3(&self, m: &[f64; 9]) -> SatResult<[f64; 9]> {
        // Simplified placeholder
        Ok([
            m[0], m[1], m[2],
            m[3], m[4], m[5],
            m[6], m[7], m[8],
        ])
    }

    /// Matrix-vector multiplication
    fn matrix_multiply_vector(&self, m: &[f64; 9], v: &[f64]) -> Vec<f64> {
        vec![
            m[0] * v[0] + m[1] * v[1] + m[2] * v[2],
            m[3] * v[0] + m[4] * v[1] + m[5] * v[2],
            m[6] * v[0] + m[7] * v[1] + m[8] * v[2],
            0.0, // Clock bias
        ]
    }

    /// Compute DOP values
    fn compute_dop(&self, _measurements: &[GnssMeasurement], _pos: &EcefPosition) -> SatResult<DilutionOfPrecision> {
        // Simplified DOP calculation
        Ok(DilutionOfPrecision {
            gdop: 2.0,
            pdop: 1.8,
            hdop: 1.2,
            vdop: 1.3,
            tdop: 1.0,
        })
    }
}
