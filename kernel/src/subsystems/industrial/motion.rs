//! # Motion Control (CNC)
//!
//! Computer Numerical Control (CNC) motion control system with:
//! - G-code parsing and execution
//! - Interpolation algorithms (linear and circular)
//! - Trajectory planning
//! - Multi-axis coordination

use alloc::{
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};
use crate::subsystems::industrial::{
    error::{IndustrialError, IndustrialResult, MotionError},
    RealtimeConstraints,
};

/// G-code command
#[derive(Debug, Clone, PartialEq)]
pub struct GCodeCommand {
    pub line_number: usize,
    pub g_code: Option<u8>,
    pub m_code: Option<u8>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub z: Option<f64>,
    pub feed_rate: Option<f64>,
    pub spindle_speed: Option<f64>,
    pub tool_number: Option<u8>,
    pub comment: String,
}

/// Interpolation type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpolationType {
    Rapid,
    Linear,
    CircularCW,
    CircularCCW,
}

/// Axis position
#[derive(Debug, Clone, Copy)]
pub struct AxisPosition {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub a: f64,
    pub b: f64,
    pub c: f64,
}

impl Default for AxisPosition {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            a: 0.0,
            b: 0.0,
            c: 0.0,
        }
    }
}

impl AxisPosition {
    /// Distance from another position
    pub fn distance(&self, other: &Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        let dz = self.z - other.z;
        (dx * dx + dy * dy + dz * dz).sqrt()
    }
}

/// Trajectory point
#[derive(Debug, Clone)]
pub struct TrajectoryPoint {
    pub position: AxisPosition,
    pub velocity: f64,
    pub acceleration: f64,
    pub time_us: u64,
}

/// Axis configuration
#[derive(Debug, Clone)]
pub struct AxisConfig {
    pub max_velocity: f64,      // mm/s
    pub max_acceleration: f64,  // mm/s²
    pub max_jerk: f64,          // mm/s³
    pub position_error_max: f64, // mm
}

impl Default for AxisConfig {
    fn default() -> Self {
        Self {
            max_velocity: 1000.0,
            max_acceleration: 10000.0,
            max_jerk: 100000.0,
            position_error_max: 0.01,
        }
    }
}

/// Servo state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServoState {
    Disabled,
    Enabled,
    Fault,
    Homing,
    InPosition,
}

/// Servo controller
pub struct ServoController {
    pub axis: String,
    pub state: ServoState,
    pub current_position: f64,
    pub target_position: f64,
    pub current_velocity: f64,
    pub config: AxisConfig,
    pub position_error_um: i64,
}

impl ServoController {
    /// Create new servo controller
    pub fn new(axis: String, config: AxisConfig) -> Self {
        Self {
            axis,
            state: ServoState::Disabled,
            current_position: 0.0,
            target_position: 0.0,
            current_velocity: 0.0,
            config,
            position_error_um: 0,
        }
    }

    /// Enable servo
    pub fn enable(&mut self) -> IndustrialResult<()> {
        self.state = ServoState::Enabled;
        Ok(())
    }

    /// Disable servo
    pub fn disable(&mut self) {
        self.state = ServoState::Disabled;
    }

    /// Move to position
    pub fn move_to(&mut self, position: f64, velocity: f64) -> IndustrialResult<()> {
        if velocity > self.config.max_velocity {
            return Err(IndustrialError::Motion(MotionError::VelocityLimitExceeded {
                axis: 0,
                velocity,
                max_velocity: self.config.max_velocity,
            }));
        }

        self.target_position = position;
        self.current_velocity = velocity;
        Ok(())
    }

    /// Update position (called during interpolation)
    pub fn update_position(&mut self, position: f64) -> IndustrialResult<()> {
        self.position_error_um = ((position - self.current_position) * 1000.0) as i64;

        if self.position_error_um.unsigned_abs() > (self.config.position_error_max * 1000.0) as u64 {
            return Err(IndustrialError::Motion(MotionError::PositionErrorExceeded {
                axis: 0,
                error_um: self.position_error_um,
                max_error_um: (self.config.position_error_max * 1000.0) as i64,
            }));
        }

        self.current_position = position;
        Ok(())
    }

    /// Check if in position
    pub fn in_position(&self) -> bool {
        (self.target_position - self.current_position).abs() < self.config.position_error_max
    }
}

/// G-code parser
pub struct GCodeParser;

impl GCodeParser {
    /// Parse G-code line
    pub fn parse_line(line: &str, line_number: usize) -> IndustrialResult<GCodeCommand> {
        let mut command = GCodeCommand {
            line_number,
            g_code: None,
            m_code: None,
            x: None,
            y: None,
            z: None,
            feed_rate: None,
            spindle_speed: None,
            tool_number: None,
            comment: String::new(),
        };

        let words: Vec<&str> = line.split_whitespace().collect();

        for word in words {
            if word.starts_with(';') || word.starts_with('(') {
                command.comment = word[1..].to_string();
                break;
            }

            if word.len() < 2 {
                continue;
            }

            let letter = word.chars().next().unwrap();
            let number_str = &word[1..];

            if let Ok(number) = number_str.parse::<f64>() {
                match letter.to_ascii_uppercase() {
                    'G' => command.g_code = Some(number as u8),
                    'M' => command.m_code = Some(number as u8),
                    'X' => command.x = Some(number),
                    'Y' => command.y = Some(number),
                    'Z' => command.z = Some(number),
                    'F' => command.feed_rate = Some(number),
                    'S' => command.spindle_speed = Some(number),
                    'T' => command.tool_number = Some(number as u8),
                    _ => {}
                }
            }
        }

        Ok(command)
    }
}

/// Linear interpolator
pub struct LinearInterpolator {
    start: AxisPosition,
    end: AxisPosition,
    feed_rate: f64,
    total_length: f64,
    current_length: f64,
}

impl LinearInterpolator {
    /// Create new linear interpolator
    pub fn new(start: AxisPosition, end: AxisPosition, feed_rate: f64) -> Self {
        let total_length = start.distance(&end);

        Self {
            start,
            end,
            feed_rate,
            total_length,
            current_length: 0.0,
        }
    }

    /// Generate next point
    pub fn next_point(&mut self, dt_sec: f64) -> Option<AxisPosition> {
        if self.current_length >= self.total_length {
            return None;
        }

        let step = self.feed_rate * dt_sec;
        self.current_length = (self.current_length + step).min(self.total_length);

        let ratio = if self.total_length > 0.0 {
            self.current_length / self.total_length
        } else {
            1.0
        };

        Some(AxisPosition {
            x: self.start.x + (self.end.x - self.start.x) * ratio,
            y: self.start.y + (self.end.y - self.start.y) * ratio,
            z: self.start.z + (self.end.z - self.start.z) * ratio,
            ..Default::default()
        })
    }

    /// Check if complete
    pub fn is_complete(&self) -> bool {
        self.current_length >= self.total_length
    }
}

/// Circular interpolator
pub struct CircularInterpolator {
    center: AxisPosition,
    radius: f64,
    start_angle: f64,
    end_angle: f64,
    clockwise: bool,
    feed_rate: f64,
    current_angle: f64,
    z_start: f64,
    z_end: f64,
}

impl CircularInterpolator {
    /// Create new circular interpolator
    pub fn new(
        start: AxisPosition,
        end: AxisPosition,
        center: AxisPosition,
        clockwise: bool,
        feed_rate: f64,
    ) -> Self {
        let start_angle = (start.y - center.y).atan2(start.x - center.x);
        let end_angle = (end.y - center.y).atan2(end.x - center.x);

        let radius = ((start.x - center.x).powi(2) + (start.y - center.y).powi(2)).sqrt();

        Self {
            center,
            radius,
            start_angle,
            end_angle,
            clockwise,
            feed_rate,
            current_angle: start_angle,
            z_start: start.z,
            z_end: end.z,
        }
    }

    /// Generate next point
    pub fn next_point(&mut self, dt_sec: f64) -> Option<AxisPosition> {
        // Check if complete
        let angle_diff = if self.clockwise {
            self.start_angle - self.end_angle
        } else {
            self.end_angle - self.start_angle
        };

        let current_diff = if self.clockwise {
            self.start_angle - self.current_angle
        } else {
            self.current_angle - self.start_angle
        };

        if current_diff.abs() >= angle_diff.abs() {
            return None;
        }

        // Calculate angular velocity
        let angular_velocity = self.feed_rate / self.radius;
        let d_angle = if self.clockwise {
            -angular_velocity * dt_sec
        } else {
            angular_velocity * dt_sec
        };

        self.current_angle += d_angle;

        // Interpolate Z
        let angle_ratio = if angle_diff.abs() > 0.0 {
            current_diff.abs() / angle_diff.abs()
        } else {
            0.0
        };

        let z = self.z_start + (self.z_end - self.z_start) * angle_ratio;

        Some(AxisPosition {
            x: self.center.x + self.radius * self.current_angle.cos(),
            y: self.center.y + self.radius * self.current_angle.sin(),
            z,
            ..Default::default()
        })
    }

    /// Check if complete
    pub fn is_complete(&self) -> bool {
        let angle_diff = if self.clockwise {
            self.start_angle - self.end_angle
        } else {
            self.end_angle - self.start_angle
        };

        let current_diff = if self.clockwise {
            self.start_angle - self.current_angle
        } else {
            self.current_angle - self.start_angle
        };

        current_diff.abs() >= angle_diff.abs()
    }
}

/// CNC motion controller
pub struct CncMotionController {
    servos: BTreeMap<String, ServoController>,
    current_position: AxisPosition,
    interpolation_type: InterpolationType,
    interpolator: Option<LinearInterpolator>,
}

impl CncMotionController {
    /// Create new motion controller
    pub fn new() -> Self {
        Self {
            servos: BTreeMap::new(),
            current_position: AxisPosition::default(),
            interpolation_type: InterpolationType::Linear,
            interpolator: None,
        }
    }

    /// Add servo axis
    pub fn add_servo(&mut self, name: String, config: AxisConfig) -> IndustrialResult<()> {
        let servo = ServoController::new(name.clone(), config);
        self.servos.insert(name, servo);
        Ok(())
    }

    /// Execute G-code command
    pub fn execute_command(&mut self, command: &GCodeCommand) -> IndustrialResult<()> {
        match command.g_code {
            Some(0) | Some(1) => {
                // Rapid or Linear move
                let mut target = self.current_position;

                if let Some(x) = command.x {
                    target.x = x;
                }
                if let Some(y) = command.y {
                    target.y = y;
                }
                if let Some(z) = command.z {
                    target.z = z;
                }

                let feed_rate = command.feed_rate.unwrap_or(100.0);

                self.linear_move(target, feed_rate)?;
            }
            Some(2) | Some(3) => {
                // Circular arc
                return Err(IndustrialError::Motion(MotionError::InterpolationError(
                    "Circular interpolation not yet implemented",
                )));
            }
            Some(g) if g >= 4 && g <= 9 => {
                // Canned cycles
                return Err(IndustrialError::Motion(MotionError::InterpolationError(
                    "Canned cycles not yet implemented",
                )));
            }
            _ => {}
        }

        Ok(())
    }

    /// Linear move
    pub fn linear_move(&mut self, target: AxisPosition, feed_rate: f64) -> IndustrialResult<()> {
        let interpolator = LinearInterpolator::new(self.current_position, target, feed_rate);
        self.interpolator = Some(interpolator);
        self.interpolation_type = InterpolationType::Linear;

        Ok(())
    }

    /// Update motion (called in real-time loop)
    pub fn update(&mut self, dt_sec: f64) -> IndustrialResult<bool> {
        if let Some(interpolator) = &mut self.interpolator {
            if let Some(position) = interpolator.next_point(dt_sec) {
                self.current_position = position;

                // Update servos
                if let Some(servo) = self.servos.get_mut("X") {
                    servo.update_position(position.x)?;
                }
                if let Some(servo) = self.servos.get_mut("Y") {
                    servo.update_position(position.y)?;
                }
                if let Some(servo) = self.servos.get_mut("Z") {
                    servo.update_position(position.z)?;
                }

                Ok(false)  // Not complete
            } else {
                self.interpolator = None;
                Ok(true)  // Complete
            }
        } else {
            Ok(true)  // No motion in progress
        }
    }

    /// Get current position
    pub fn current_position(&self) -> &AxisPosition {
        &self.current_position
    }

    /// Get real-time constraints
    pub fn realtime_constraints(&self) -> RealtimeConstraints {
        RealtimeConstraints::hard_realtime()
    }
}

impl Default for CncMotionController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gcode_parser() {
        let line = "G00 X100 Y50 F1000";
        let command = GCodeParser::parse_line(line, 1).unwrap();
        assert_eq!(command.g_code, Some(0));
        assert_eq!(command.x, Some(100.0));
        assert_eq!(command.y, Some(50.0));
        assert_eq!(command.feed_rate, Some(1000.0));
    }

    #[test]
    fn test_axis_position_distance() {
        let p1 = AxisPosition { x: 0.0, y: 0.0, z: 0.0, ..Default::default() };
        let p2 = AxisPosition { x: 3.0, y: 4.0, z: 0.0, ..Default::default() };
        assert_eq!(p1.distance(&p2), 5.0);
    }

    #[test]
    fn test_linear_interpolator() {
        let start = AxisPosition { x: 0.0, y: 0.0, z: 0.0, ..Default::default() };
        let end = AxisPosition { x: 100.0, y: 0.0, z: 0.0, ..Default::default() };
        let mut interp = LinearInterpolator::new(start, end, 1000.0);

        let point = interp.next_point(0.1).unwrap();
        assert!(point.x > 0.0);
    }

    #[test]
    fn test_servo_controller() {
        let config = AxisConfig::default();
        let mut servo = ServoController::new("X".into(), config);

        servo.enable().unwrap();
        assert_eq!(servo.state, ServoState::Enabled);

        servo.move_to(100.0, 500.0).unwrap();
        assert_eq!(servo.target_position, 100.0);
    }

    #[test]
    fn test_cnc_controller() {
        let mut cnc = CncMotionController::new();
        cnc.add_servo("X".into(), AxisConfig::default()).unwrap();
        cnc.add_servo("Y".into(), AxisConfig::default()).unwrap();

        let command = GCodeCommand {
            line_number: 1,
            g_code: Some(1),
            m_code: None,
            x: Some(100.0),
            y: Some(50.0),
            z: None,
            feed_rate: Some(1000.0),
            spindle_speed: None,
            tool_number: None,
            comment: String::new(),
        };

        cnc.execute_command(&command).unwrap();
        assert!(cnc.interpolator.is_some());
    }
}
