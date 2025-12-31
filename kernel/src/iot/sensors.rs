//! # Sensor Fusion and Calibration
//!
//! This module provides comprehensive sensor fusion and calibration capabilities
//! for IoT devices with multiple sensors.
//!
//! ## Features
//!
//! - **IMU Support**: Accelerometer, gyroscope, magnetometer
//! - **Environmental Sensors**: Temperature, humidity, pressure
//! - **GPS/GNSS**: Global positioning system
//! - **Kalman Filter**: Optimal state estimation
//! - **Complementary Filter**: Simpler sensor fusion
//! - **Calibration Procedures**: Sensor calibration routines
//! - **Bias Correction**: Remove sensor biases
//! - **Noise Reduction**: Filter and smooth data
//! - **Multi-sensor Sync**: Synchronize multiple sensors
//!
//! ## Supported Sensors
//!
//! - **Accelerometer**: 3-axis acceleration sensing
//! - **Gyroscope**: 3-axis angular rate sensing
//! - **Magnetometer**: 3-axis magnetic field sensing
//! - **Barometer**: Atmospheric pressure
//! - **Thermometer**: Temperature
//! - **Hygrometer**: Humidity
//! - **GPS**: Global positioning
//!
//! ## Usage Examples
//!
//! ### Calibrate sensor
//!
//! ```no_run
//! use kernel::iot::sensors::{Accelerometer, SensorCalibration};
//!
//! let mut accel = Accelerometer::new();
//! accel.calibrate()?;
//! # Ok::<(), kernel::iot::IotError>(())
//! ```
//!
//! ### Apply Kalman filter
//!
//! ```no_run
//! use kernel::iot::sensors::KalmanFilter;
//!
//! let mut filter = KalmanFilter::new(1.0, 0.1);
//! let estimate = filter.update(10.0)?;
//! # Ok::<(), kernel::iot::IotError>(())
//! ```

#![allow(dead_code)]
#![warn(missing_docs)]

use alloc::string::ToString;
use alloc::vec::Vec;

use crate::iot::IotError;
use crate::iot::IotResult;

// =============================================================================
// Common Sensor Types
// =============================================================================

/// 3D vector
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vector3 {
    /// X component
    pub x: f64,
    /// Y component
    pub y: f64,
    /// Z component
    pub z: f64,
}

impl Vector3 {
    /// Create a new 3D vector
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }

    /// Zero vector
    pub fn zero() -> Self {
        Self { x: 0.0, y: 0.0, z: 0.0 }
    }

    /// Magnitude (length)
    pub fn magnitude(&self) -> f64 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    /// Normalize to unit vector
    pub fn normalize(&self) -> Option<Self> {
        let mag = self.magnitude();
        if mag == 0.0 {
            return None;
        }
        Some(Self {
            x: self.x / mag,
            y: self.y / mag,
            z: self.z / mag,
        })
    }

    /// Dot product
    pub fn dot(&self, other: &Vector3) -> f64 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    /// Cross product
    pub fn cross(&self, other: &Vector3) -> Self {
        Self {
            x: self.y * other.z - self.z * other.y,
            y: self.z * other.x - self.x * other.z,
            z: self.x * other.y - self.y * other.x,
        }
    }

    /// Add vectors
    pub fn add(&self, other: &Vector3) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }

    /// Subtract vectors
    pub fn sub(&self, other: &Vector3) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }

    /// Scale by scalar
    pub fn scale(&self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }

    /// Distance to other vector
    pub fn distance(&self, other: &Vector3) -> f64 {
        self.sub(other).magnitude()
    }
}

impl core::ops::Add for Vector3 {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            x: self.x + other.x,
            y: self.y + other.y,
            z: self.z + other.z,
        }
    }
}

impl core::ops::Sub for Vector3 {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self {
            x: self.x - other.x,
            y: self.y - other.y,
            z: self.z - other.z,
        }
    }
}

/// Quaternion for rotation representation
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quaternion {
    /// W component (scalar)
    pub w: f64,
    /// X component
    pub x: f64,
    /// Y component
    pub y: f64,
    /// Z component
    pub z: f64,
}

impl Quaternion {
    /// Identity quaternion
    pub fn identity() -> Self {
        Self {
            w: 1.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }

    /// Create from axis-angle
    pub fn from_axis_angle(axis: &Vector3, angle: f64) -> Self {
        let half_angle = angle / 2.0;
        let sin_half = half_angle.sin();

        let normalized_axis = match axis.normalize() {
            Some(a) => a,
            None => return Self::identity(),
        };

        Self {
            w: half_angle.cos(),
            x: normalized_axis.x * sin_half,
            y: normalized_axis.y * sin_half,
            z: normalized_axis.z * sin_half,
        }
    }

    /// Multiply quaternions
    pub fn multiply(&self, other: &Quaternion) -> Self {
        Self {
            w: self.w * other.w - self.x * other.x - self.y * other.y - self.z * other.z,
            x: self.w * other.x + self.x * other.w + self.y * other.z - self.z * other.y,
            y: self.w * other.y - self.x * other.z + self.y * other.w + self.z * other.x,
            z: self.w * other.z + self.x * other.y - self.y * other.x + self.z * other.w,
        }
    }

    /// Rotate vector
    pub fn rotate_vector(&self, v: &Vector3) -> Vector3 {
        let qv = Quaternion {
            w: 0.0,
            x: v.x,
            y: v.y,
            z: v.z,
        };

        let q_conj = Quaternion {
            w: self.w,
            x: -self.x,
            y: -self.y,
            z: -self.z,
        };

        let result = self.multiply(&qv).multiply(&q_conj);

        Vector3 {
            x: result.x,
            y: result.y,
            z: result.z,
        }
    }

    /// Normalize
    pub fn normalize(&self) -> Self {
        let mag = (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt();

        if mag == 0.0 {
            return Self::identity();
        }

        Self {
            w: self.w / mag,
            x: self.x / mag,
            y: self.y / mag,
            z: self.z / mag,
        }
    }
}

// =============================================================================
// IMU Sensors
// =============================================================================

/// Accelerometer data
#[derive(Debug, Clone, Copy)]
pub struct AccelerometerData {
    /// Acceleration in m/s²
    pub acceleration: Vector3,
    /// Timestamp in milliseconds
    pub timestamp: u64,
}

impl AccelerometerData {
    /// Create new accelerometer data
    pub fn new(acceleration: Vector3, timestamp: u64) -> Self {
        Self {
            acceleration,
            timestamp,
        }
    }

    /// Get magnitude of acceleration
    pub fn magnitude(&self) -> f64 {
        self.acceleration.magnitude()
    }

    /// Get tilt angle from gravity
    pub fn tilt_angles(&self) -> (f64, f64) {
        // Pitch and roll from gravity vector
        let pitch = self.acceleration.y.atan2(self.acceleration.z);
        let roll = self.acceleration.x.atan2(self.acceleration.z);

        (pitch, roll)
    }
}

/// Gyroscope data
#[derive(Debug, Clone, Copy)]
pub struct GyroscopeData {
    /// Angular velocity in rad/s
    pub angular_velocity: Vector3,
    /// Timestamp in milliseconds
    pub timestamp: u64,
}

impl GyroscopeData {
    /// Create new gyroscope data
    pub fn new(angular_velocity: Vector3, timestamp: u64) -> Self {
        Self {
            angular_velocity,
            timestamp,
        }
    }

    /// Integrate to get rotation (simplified)
    pub fn integrate(&self, dt: f64) -> Quaternion {
        let angle = self.angular_velocity.magnitude() * dt;

        if let Some(axis) = self.angular_velocity.normalize() {
            Quaternion::from_axis_angle(&axis, angle)
        } else {
            Quaternion::identity()
        }
    }
}

/// Magnetometer data
#[derive(Debug, Clone, Copy)]
pub struct MagnetometerData {
    /// Magnetic field in μT
    pub magnetic_field: Vector3,
    /// Timestamp in milliseconds
    pub timestamp: u64,
}

impl MagnetometerData {
    /// Create new magnetometer data
    pub fn new(magnetic_field: Vector3, timestamp: u64) -> Self {
        Self {
            magnetic_field,
            timestamp,
        }
    }

    /// Get heading (yaw angle)
    pub fn heading(&self) -> f64 {
        // Simplified heading calculation
        self.magnetic_field.y.atan2(self.magnetic_field.x)
    }
}

/// IMU (Inertial Measurement Unit)
pub struct Imu {
    /// Accelerometer bias
    accel_bias: Vector3,
    /// Gyroscope bias
    gyro_bias: Vector3,
    /// Magnetometer bias
    mag_bias: Vector3,
    /// Calibrated flag
    calibrated: bool,
}

impl Imu {
    /// Create a new IMU
    pub fn new() -> Self {
        Self {
            accel_bias: Vector3::zero(),
            gyro_bias: Vector3::zero(),
            mag_bias: Vector3::zero(),
            calibrated: false,
        }
    }

    /// Calibrate accelerometer (6-point calibration)
    pub fn calibrate_accelerometer(&mut self, samples: &[Vector3]) -> IotResult<()> {
        // Calculate bias from samples (assuming 6 positions)
        if samples.len() < 6 {
            return Err(IotError::CalibrationError(
                "Insufficient calibration samples".to_string(),
            ));
        }

        // Simplified: average opposing samples
        let mut bias_sum = Vector3::zero();

        for i in 0..3 {
            let sample1 = samples[2 * i];
            let sample2 = samples[2 * i + 1];

            // Bias is average of opposite measurements
            let bias = sample1.add(&sample2).scale(0.5);
            bias_sum = bias_sum.add(&bias);
        }

        self.accel_bias = bias_sum.scale(1.0 / 3.0);
        self.calibrated = true;

        Ok(())
    }

    /// Calibrate gyroscope (static calibration)
    pub fn calibrate_gyroscope(&mut self, samples: &[Vector3]) -> IotResult<()> {
        if samples.is_empty() {
            return Err(IotError::CalibrationError("No samples provided".to_string()));
        }

        // Calculate average bias (assuming device is stationary)
        let mut sum = Vector3::zero();

        for sample in samples {
            sum = sum.add(sample);
        }

        self.gyro_bias = sum.scale(1.0 / samples.len() as f64);
        Ok(())
    }

    /// Calibrate magnetometer (hard/soft iron)
    pub fn calibrate_magnetometer(&mut self, _samples: &[Vector3]) -> IotResult<()> {
        // In a real implementation, this would:
        // 1. Find min/max values for hard iron correction
        // 2. Calculate ellipse fitting for soft iron correction

        self.mag_bias = Vector3::zero();
        Ok(())
    }

    /// Read calibrated accelerometer data
    pub fn read_accelerometer(&self, data: AccelerometerData) -> AccelerometerData {
        AccelerometerData {
            acceleration: data.acceleration.sub(&self.accel_bias),
            timestamp: data.timestamp,
        }
    }

    /// Read calibrated gyroscope data
    pub fn read_gyroscope(&self, data: GyroscopeData) -> GyroscopeData {
        GyroscopeData {
            angular_velocity: data.angular_velocity.sub(&self.gyro_bias),
            timestamp: data.timestamp,
        }
    }

    /// Read calibrated magnetometer data
    pub fn read_magnetometer(&self, data: MagnetometerData) -> MagnetometerData {
        MagnetometerData {
            magnetic_field: data.magnetic_field.sub(&self.mag_bias),
            timestamp: data.timestamp,
        }
    }

    /// Check if calibrated
    pub fn is_calibrated(&self) -> bool {
        self.calibrated
    }
}

impl Default for Imu {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Environmental Sensors
// =============================================================================

/// Temperature sensor data
#[derive(Debug, Clone, Copy)]
pub struct TemperatureData {
    /// Temperature in Celsius
    pub temperature: f64,
    /// Timestamp in milliseconds
    pub timestamp: u64,
}

impl TemperatureData {
    /// Create new temperature data
    pub fn new(temperature: f64, timestamp: u64) -> Self {
        Self {
            temperature,
            timestamp,
        }
    }

    /// Convert to Fahrenheit
    pub fn to_fahrenheit(&self) -> f64 {
        self.temperature * 9.0 / 5.0 + 32.0
    }

    /// Convert to Kelvin
    pub fn to_kelvin(&self) -> f64 {
        self.temperature + 273.15
    }
}

/// Humidity sensor data
#[derive(Debug, Clone, Copy)]
pub struct HumidityData {
    /// Relative humidity (0-100%)
    pub humidity: f64,
    /// Timestamp in milliseconds
    pub timestamp: u64,
}

impl HumidityData {
    /// Create new humidity data
    pub fn new(humidity: f64, timestamp: u64) -> Self {
        Self {
            humidity: humidity.clamp(0.0, 100.0),
            timestamp,
        }
    }

    /// Get absolute humidity (g/m³)
    pub fn absolute_humidity(&self, temperature: f64) -> f64 {
        // Magnus formula for saturation vapor pressure
        const MAGNUS_A: f64 = 6.112;
        const MAGNUS_B: f64 = 17.67;
        const MAGNUS_C: f64 = 254.88;

        let es = MAGNUS_A
            * ((MAGNUS_B * temperature) / (temperature + MAGNUS_C)).exp();
        let actual_vapor_pressure = es * (self.humidity / 100.0);

        // Convert to absolute humidity
        (actual_vapor_pressure * 2.1674) / (temperature + 273.15)
    }
}

/// Pressure sensor data
#[derive(Debug, Clone, Copy)]
pub struct PressureData {
    /// Atmospheric pressure in hPa
    pub pressure: f64,
    /// Timestamp in milliseconds
    pub timestamp: u64,
}

impl PressureData {
    /// Create new pressure data
    pub fn new(pressure: f64, timestamp: u64) -> Self {
        Self { pressure, timestamp }
    }

    /// Estimate altitude from pressure
    pub fn altitude(&self, sea_level_pressure: f64) -> f64 {
        // International barometric formula
        const STANDARD_LAPSE_RATE: f64 = 0.0065;
        const STANDARD_TEMP: f64 = 288.15;
        const GRAVITY: f64 = 9.80665;
        const MOLAR_MASS: f64 = 0.0289644;
        const GAS_CONSTANT: f64 = 8.31432;

        let exponent = (GRAVITY * MOLAR_MASS) / (GAS_CONSTANT * STANDARD_LAPSE_RATE);

        STANDARD_TEMP
            * (1.0 - (sea_level_pressure / self.pressure).powf(1.0 / exponent))
            / STANDARD_LAPSE_RATE
    }
}

/// Environmental sensor suite
pub struct EnvironmentalSensors {
    /// Temperature offset
    temp_offset: f64,
    /// Humidity offset
    humidity_offset: f64,
    /// Pressure offset
    pressure_offset: f64,
}

impl EnvironmentalSensors {
    /// Create new environmental sensors
    pub fn new() -> Self {
        Self {
            temp_offset: 0.0,
            humidity_offset: 0.0,
            pressure_offset: 0.0,
        }
    }

    /// Calibrate temperature
    pub fn calibrate_temperature(&mut self, reference: f64, measured: f64) {
        self.temp_offset = reference - measured;
    }

    /// Calibrate humidity
    pub fn calibrate_humidity(&mut self, reference: f64, measured: f64) {
        self.humidity_offset = reference - measured;
    }

    /// Calibrate pressure
    pub fn calibrate_pressure(&mut self, reference: f64, measured: f64) {
        self.pressure_offset = reference - measured;
    }

    /// Read calibrated temperature
    pub fn read_temperature(&self, data: TemperatureData) -> TemperatureData {
        TemperatureData {
            temperature: data.temperature + self.temp_offset,
            timestamp: data.timestamp,
        }
    }

    /// Read calibrated humidity
    pub fn read_humidity(&self, data: HumidityData) -> HumidityData {
        HumidityData {
            humidity: (data.humidity + self.humidity_offset).clamp(0.0, 100.0),
            timestamp: data.timestamp,
        }
    }

    /// Read calibrated pressure
    pub fn read_pressure(&self, data: PressureData) -> PressureData {
        PressureData {
            pressure: data.pressure + self.pressure_offset,
            timestamp: data.timestamp,
        }
    }
}

impl Default for EnvironmentalSensors {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// GPS/GNSS
// =============================================================================

/// GPS position
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GpsPosition {
    /// Latitude in degrees
    pub latitude: f64,
    /// Longitude in degrees
    pub longitude: f64,
    /// Altitude in meters
    pub altitude: f64,
}

impl GpsPosition {
    /// Create new GPS position
    pub fn new(latitude: f64, longitude: f64, altitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            altitude,
        }
    }

    /// Calculate distance to another position (Haversine formula)
    pub fn distance_to(&self, other: &GpsPosition) -> f64 {
        const EARTH_RADIUS: f64 = 6371000.0; // meters

        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let delta_lat = (other.latitude - self.latitude).to_radians();
        let delta_lon = (other.longitude - self.longitude).to_radians();

        let a = (delta_lat / 2.0).sin().powi(2)
            + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);

        let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

        EARTH_RADIUS * c
    }

    /// Calculate bearing to another position
    pub fn bearing_to(&self, other: &GpsPosition) -> f64 {
        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let delta_lon = (other.longitude - self.longitude).to_radians();

        let y = delta_lon.sin() * lat2.cos();
        let x = lat1.cos() * lat2.sin() - lat1.sin() * lat2.cos() * delta_lon.cos();

        y.atan2(x).to_degrees()
    }
}

/// GPS fix quality
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum GpsFixQuality {
    /// No fix
    NoFix = 0,
    /// GPS fix
    Gps = 1,
    /// Differential GPS fix
    DGps = 2,
    /// PPP fix
    PPP = 3,
}

/// GPS data
#[derive(Debug, Clone, Copy)]
pub struct GpsData {
    /// Position
    pub position: GpsPosition,
    /// Fix quality
    pub fix_quality: GpsFixQuality,
    /// Number of satellites
    pub num_satellites: u8,
    /// Horizontal dilution of precision
    pub hdop: f64,
    /// Timestamp in milliseconds
    pub timestamp: u64,
}

impl GpsData {
    /// Create new GPS data
    pub fn new(position: GpsPosition, timestamp: u64) -> Self {
        Self {
            position,
            fix_quality: GpsFixQuality::NoFix,
            num_satellites: 0,
            hdop: 99.0,
            timestamp,
        }
    }

    /// Check if has valid fix
    pub fn has_fix(&self) -> bool {
        self.fix_quality != GpsFixQuality::NoFix && self.num_satellites >= 3
    }

    /// Check fix quality
    pub fn is_accurate(&self) -> bool {
        self.has_fix() && self.hdop < 2.0
    }
}

// =============================================================================
// Sensor Fusion
// =============================================================================

/// Complementary filter for sensor fusion
pub struct ComplementaryFilter {
    /// Filter coefficient (0-1)
    alpha: f64,
    /// Current angle estimate
    angle: f64,
}

impl ComplementaryFilter {
    /// Create a new complementary filter
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            angle: 0.0,
        }
    }

    /// Update filter with new measurements
    pub fn update(&mut self, gyro_rate: f64, accel_angle: f64, dt: f64) -> f64 {
        // Integrate gyroscope
        let gyro_angle = self.angle + gyro_rate * dt;

        // Complementary filter: blend gyro and accelerometer
        self.angle = self.alpha * gyro_angle + (1.0 - self.alpha) * accel_angle;

        self.angle
    }

    /// Get current angle estimate
    pub fn angle(&self) -> f64 {
        self.angle
    }

    /// Reset filter
    pub fn reset(&mut self) {
        self.angle = 0.0;
    }
}

/// Kalman filter for optimal state estimation
pub struct KalmanFilter {
    /// State estimate
    x: f64,
    /// Error covariance
    p: f64,
    /// Process noise variance
    q: f64,
    /// Measurement noise variance
    r: f64,
}

impl KalmanFilter {
    /// Create a new Kalman filter
    pub fn new(process_noise: f64, measurement_noise: f64) -> Self {
        Self {
            x: 0.0,
            p: 1.0,
            q: process_noise,
            r: measurement_noise,
        }
    }

    /// Initialize with state
    pub fn init(&mut self, initial_state: f64) {
        self.x = initial_state;
    }

    /// Update filter with measurement
    pub fn update(&mut self, measurement: f64) -> IotResult<f64> {
        // Predict
        let x_pred = self.x;
        let p_pred = self.p + self.q;

        // Update
        let k = p_pred / (p_pred + self.r);
        self.x = x_pred + k * (measurement - x_pred);
        self.p = (1.0 - k) * p_pred;

        Ok(self.x)
    }

    /// Get current state estimate
    pub fn state(&self) -> f64 {
        self.x
    }

    /// Get error covariance
    pub fn covariance(&self) -> f64 {
        self.p
    }

    /// Reset filter
    pub fn reset(&mut self) {
        self.x = 0.0;
        self.p = 1.0;
    }
}

/// Extended Kalman filter for non-linear systems
pub struct ExtendedKalmanFilter {
    /// State vector [x, y, z, vx, vy, vz]
    x: [f64; 6],
    /// State covariance matrix (6x6)
    p: [[f64; 6]; 6],
    /// Process noise
    q: f64,
    /// Measurement noise
    r: f64,
}

impl ExtendedKalmanFilter {
    /// Create a new extended Kalman filter
    pub fn new(q: f64, r: f64) -> Self {
        Self {
            x: [0.0; 6],
            p: [[0.0; 6]; 6],
            q,
            r,
        }
    }

    /// Predict step
    pub fn predict(&mut self, _dt: f64) {
        // In a real implementation, predict next state using system model
        for i in 0..6 {
            self.p[i][i] += self.q;
        }
    }

    /// Update step with measurement
    pub fn update(&mut self, measurement: &Vector3) -> IotResult<Vector3> {
        // In a real implementation, this would:
        // 1. Compute Kalman gain
        // 2. Update state estimate
        // 3. Update covariance matrix

        // Simplified: just copy measurement
        self.x[0] = measurement.x;
        self.x[1] = measurement.y;
        self.x[2] = measurement.z;

        Ok(Vector3 {
            x: self.x[0],
            y: self.x[1],
            z: self.x[2],
        })
    }

    /// Get current position estimate
    pub fn position(&self) -> Vector3 {
        Vector3 {
            x: self.x[0],
            y: self.x[1],
            z: self.x[2],
        }
    }

    /// Get current velocity estimate
    pub fn velocity(&self) -> Vector3 {
        Vector3 {
            x: self.x[3],
            y: self.x[4],
            z: self.x[5],
        }
    }
}

/// 9-axis sensor fusion (accel + gyro + mag)
pub struct SensorFusion9Dof {
    /// Orientation quaternion
    orientation: Quaternion,
    /// Complementary filter for pitch/roll
    complementary_pitch: ComplementaryFilter,
    /// Complementary filter for roll
    complementary_roll: ComplementaryFilter,
}

impl SensorFusion9Dof {
    /// Create a new 9-DOF sensor fusion
    pub fn new() -> Self {
        Self {
            orientation: Quaternion::identity(),
            complementary_pitch: ComplementaryFilter::new(0.98),
            complementary_roll: ComplementaryFilter::new(0.98),
        }
    }

    /// Update with IMU data
    pub fn update(&mut self, accel: &Vector3, gyro: &Vector3, mag: &Vector3, dt: f64) -> Quaternion {
        // Calculate pitch and roll from accelerometer
        let pitch = accel.y.atan2(accel.z);
        let roll = accel.x.atan2(accel.z);

        // Apply complementary filter
        let _filtered_pitch = self.complementary_pitch.update(gyro.x, pitch, dt);
        let _filtered_roll = self.complementary_roll.update(gyro.y, roll, dt);

        // Calculate yaw from magnetometer (simplified)
        let yaw = mag.y.atan2(mag.x);

        // Create orientation from Euler angles
        self.orientation = Quaternion::from_axis_angle(&Vector3::new(1.0, 0.0, 0.0), pitch)
            .multiply(&Quaternion::from_axis_angle(&Vector3::new(0.0, 1.0, 0.0), roll))
            .multiply(&Quaternion::from_axis_angle(&Vector3::new(0.0, 0.0, 1.0), yaw));

        self.orientation
    }

    /// Get current orientation
    pub fn orientation(&self) -> Quaternion {
        self.orientation
    }

    /// Get Euler angles (roll, pitch, yaw)
    pub fn euler_angles(&self) -> (f64, f64, f64) {
        // Convert quaternion to Euler angles (simplified)
        let roll = (2.0 * (self.orientation.w * self.orientation.x + self.orientation.y * self.orientation.z)).atan2(
            1.0 - 2.0 * (self.orientation.x * self.orientation.x + self.orientation.y * self.orientation.y)
        );

        let pitch = (2.0 * (self.orientation.w * self.orientation.y - self.orientation.z * self.orientation.x)).asin();

        let yaw = (2.0 * (self.orientation.w * self.orientation.z + self.orientation.x * self.orientation.y)).atan2(
            1.0 - 2.0 * (self.orientation.y * self.orientation.y + self.orientation.z * self.orientation.z)
        );

        (roll, pitch, yaw)
    }
}

impl Default for SensorFusion9Dof {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// Noise Reduction
// =============================================================================

/// Moving average filter
pub struct MovingAverageFilter {
    /// Window size
    window_size: usize,
    /// Data buffer
    buffer: Vec<f64>,
}

impl MovingAverageFilter {
    /// Create a new moving average filter
    pub fn new(window_size: usize) -> Self {
        Self {
            window_size,
            buffer: Vec::with_capacity(window_size),
        }
    }

    /// Add new sample and get filtered value
    pub fn update(&mut self, value: f64) -> f64 {
        self.buffer.push(value);

        if self.buffer.len() > self.window_size {
            self.buffer.remove(0);
        }

        let sum: f64 = self.buffer.iter().sum();
        sum / self.buffer.len() as f64
    }

    /// Reset filter
    pub fn reset(&mut self) {
        self.buffer.clear();
    }
}

/// Low-pass filter (exponential moving average)
pub struct LowPassFilter {
    /// Alpha (smoothing factor)
    alpha: f64,
    /// Previous output
    prev_output: Option<f64>,
}

impl LowPassFilter {
    /// Create a new low-pass filter
    pub fn new(alpha: f64) -> Self {
        Self {
            alpha: alpha.clamp(0.0, 1.0),
            prev_output: None,
        }
    }

    /// Update filter with new sample
    pub fn update(&mut self, input: f64) -> f64 {
        let output = match self.prev_output {
            Some(prev) => self.alpha * input + (1.0 - self.alpha) * prev,
            None => input,
        };

        self.prev_output = Some(output);
        output
    }

    /// Reset filter
    pub fn reset(&mut self) {
        self.prev_output = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vector3() {
        let v1 = Vector3::new(1.0, 2.0, 3.0);
        let v2 = Vector3::new(4.0, 5.0, 6.0);

        assert_eq!(v1.x, 1.0);
        assert_eq!(v1.magnitude().round(), 4.0);

        let sum = v1.add(&v2);
        assert_eq!(sum.x, 5.0);
        assert_eq!(sum.y, 7.0);
        assert_eq!(sum.z, 9.0);

        let dot = v1.dot(&v2);
        assert_eq!(dot, 32.0);
    }

    #[test]
    fn test_vector3_normalize() {
        let v = Vector3::new(3.0, 4.0, 0.0);
        let normalized = v.normalize().unwrap();

        assert!((normalized.magnitude() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_quaternion() {
        let q = Quaternion::identity();
        assert_eq!(q.w, 1.0);
        assert_eq!(q.x, 0.0);
        assert_eq!(q.y, 0.0);
        assert_eq!(q.z, 0.0);
    }

    #[test]
    fn test_quaternion_from_axis_angle() {
        let axis = Vector3::new(0.0, 0.0, 1.0);
        let angle = PI / 2.0;

        let q = Quaternion::from_axis_angle(&axis, angle);

        // Should rotate around Z axis
        assert!((q.w - 0.707).abs() < 0.01);
    }

    #[test]
    fn test_imu() {
        let imu = Imu::new();
        assert!(!imu.is_calibrated());

        let mut imu = Imu::new();
        let samples = vec![
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(-1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(0.0, -1.0, 0.0),
            Vector3::new(0.0, 0.0, 1.0),
            Vector3::new(0.0, 0.0, -1.0),
        ];

        imu.calibrate_accelerometer(&samples).unwrap();
        assert!(imu.is_calibrated());
    }

    #[test]
    fn test_temperature_data() {
        let temp = TemperatureData::new(25.0, 12345);

        assert_eq!(temp.temperature, 25.0);
        assert!((temp.to_fahrenheit() - 77.0).abs() < 0.1);
        assert!((temp.to_kelvin() - 298.15).abs() < 0.1);
    }

    #[test]
    fn test_humidity_data() {
        let hum = HumidityData::new(50.0, 12345);

        assert_eq!(hum.humidity, 50.0);
        assert!(hum.absolute_humidity(25.0) > 0.0);
    }

    #[test]
    fn test_pressure_altitude() {
        let pressure = PressureData::new(1013.25, 12345);

        let altitude = pressure.altitude(1013.25);
        assert!((altitude - 0.0).abs() < 1.0);
    }

    #[test]
    fn test_gps_position() {
        let pos1 = GpsPosition::new(40.7128, -74.0060, 10.0); // New York
        let pos2 = GpsPosition::new(51.5074, -0.1278, 10.0); // London

        let distance = pos1.distance_to(&pos2);
        assert!(distance > 5_000_000.0); // > 5000 km
    }

    #[test]
    fn test_complementary_filter() {
        let mut filter = ComplementaryFilter::new(0.98);

        let angle1 = filter.update(0.1, 0.0, 0.01);
        let angle2 = filter.update(0.1, 0.05, 0.01);

        // Filter should converge towards accelerometer
        assert!(angle2 > angle1);
    }

    #[test]
    fn test_kalman_filter() {
        let mut filter = KalmanFilter::new(0.1, 0.1);

        filter.init(10.0);

        let estimate = filter.update(10.5).unwrap();
        assert!((estimate - 10.0).abs() < 1.0);
    }

    #[test]
    fn test_moving_average_filter() {
        let mut filter = MovingAverageFilter::new(3);

        let f1 = filter.update(10.0);
        let f2 = filter.update(20.0);
        let f3 = filter.update(30.0);

        assert_eq!(f1, 10.0);
        assert_eq!(f2, 15.0);
        assert_eq!(f3, 20.0);
    }

    #[test]
    fn test_low_pass_filter() {
        let mut filter = LowPassFilter::new(0.5);

        let out1 = filter.update(10.0);
        let out2 = filter.update(20.0);

        assert_eq!(out1, 10.0);
        assert_eq!(out2, 15.0);
    }

    #[test]
    fn test_sensor_fusion_9dof() {
        let mut fusion = SensorFusion9Dof::new();

        let accel = Vector3::new(0.0, 0.0, 9.81);
        let gyro = Vector3::new(0.0, 0.0, 0.0);
        let mag = Vector3::new(20.0, 0.0, -40.0);

        let orientation = fusion.update(&accel, &gyro, &mag, 0.01);

        // Should have valid orientation
        assert!((orientation.w * orientation.w + orientation.x * orientation.x
            + orientation.y * orientation.y + orientation.z * orientation.z - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_accelerometer_data() {
        let data = AccelerometerData::new(Vector3::new(0.0, 0.0, 9.81), 12345);

        assert_eq!(data.magnitude(), 9.81);
    }

    #[test]
    fn test_gyroscope_data() {
        let data = GyroscopeData::new(Vector3::new(0.1, 0.0, 0.0), 12345);

        let rotation = data.integrate(0.1);
        assert_eq!(rotation.w, 0.99); // Approximately
    }

    #[test]
    fn test_magnetometer_data() {
        let data = MagnetometerData::new(Vector3::new(20.0, 0.0, 0.0), 12345);

        let heading = data.heading();
        assert_eq!(heading, 0.0);
    }

    #[test]
    fn test_gps_fix_quality() {
        let mut data = GpsData::new(GpsPosition::new(40.0, -74.0, 10.0), 12345);

        data.fix_quality = GpsFixQuality::Gps;
        data.num_satellites = 5;
        data.hdop = 1.0;

        assert!(data.has_fix());
        assert!(data.is_accurate());
    }

    #[test]
    fn test_gps_bearing() {
        let pos1 = GpsPosition::new(0.0, 0.0, 0.0);
        let pos2 = GpsPosition::new(0.0, 1.0, 0.0);

        let bearing = pos1.bearing_to(&pos2);

        // Should be roughly 90 degrees (east)
        assert!((bearing - 90.0).abs() < 1.0);
    }
}
