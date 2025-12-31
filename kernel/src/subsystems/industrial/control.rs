//! # Real-time Control Loops
//!
//! Industrial control system implementation including:
//! - PID controllers
//! - Cascade control
//! - Feedforward control
//! - Real-time scheduling

use alloc::collections::VecDeque;
use crate::subsystems::industrial::{
    error::{ControlError, IndustrialError, IndustrialResult},
    RealtimeConstraints,
};

// Import math functions for no_std

/// PID controller
#[derive(Debug, Clone)]
pub struct PidController {
    /// Proportional gain
    pub kp: f64,
    /// Integral gain
    pub ki: f64,
    /// Derivative gain
    pub kd: f64,
    /// Integral term
    integral: f64,
    /// Previous error
    previous_error: f64,
    /// Output limits
    output_min: f64,
    output_max: f64,
    /// Integral windup limit
    integral_limit: f64,
}

impl PidController {
    /// Create new PID controller
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            previous_error: 0.0,
            output_min: f64::NEG_INFINITY,
            output_max: f64::INFINITY,
            integral_limit: f64::INFINITY,
        }
    }

    /// Set output limits
    pub fn with_output_limits(mut self, min: f64, max: f64) -> Self {
        self.output_min = min;
        self.output_max = max;
        self
    }

    /// Set integral windup limit
    pub fn with_integral_limit(mut self, limit: f64) -> Self {
        self.integral_limit = limit;
        self
    }

    /// Compute PID output
    pub fn compute(&mut self, setpoint: f64, measurement: f64, dt_sec: f64) -> IndustrialResult<f64> {
        // Validate parameters
        if self.kp < 0.0 || self.ki < 0.0 || self.kd < 0.0 {
            return Err(IndustrialError::Control(ControlError::InvalidPidParameters {
                parameter: "gain",
                value: self.kp,
            }));
        }

        if dt_sec <= 0.0 {
            return Err(IndustrialError::Control(ControlError::InvalidSamplingPeriod {
                period_us: (dt_sec * 1e6) as u64,
                min_period_us: 1,
            }));
        }

        // Calculate error
        let error = setpoint - measurement;

        // Proportional term
        let p = self.kp * error;

        // Integral term with windup protection
        self.integral += error * dt_sec;
        self.integral = self.integral.clamp(-self.integral_limit, self.integral_limit);
        let i = self.ki * self.integral;

        // Derivative term (on measurement to avoid derivative kick)
        let derivative = (measurement - self.previous_error) / dt_sec;
        let d = -self.kd * derivative;

        // Calculate output
        let output = p + i + d;

        // Apply output limits
        let output = output.clamp(self.output_min, self.output_max);

        // Store for next iteration
        self.previous_error = measurement;

        Ok(output)
    }

    /// Reset controller state
    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.previous_error = 0.0;
    }
}

/// Cascade controller
#[derive(Debug, Clone)]
pub struct CascadeController {
    /// Primary (outer) controller
    pub primary: PidController,
    /// Secondary (inner) controller
    pub secondary: PidController,
    /// Secondary setpoint limits
    secondary_sp_min: f64,
    secondary_sp_max: f64,
}

impl CascadeController {
    /// Create new cascade controller
    pub fn new(primary: PidController, secondary: PidController) -> Self {
        Self {
            primary,
            secondary,
            secondary_sp_min: f64::NEG_INFINITY,
            secondary_sp_max: f64::INFINITY,
        }
    }

    /// Set secondary setpoint limits
    pub fn with_secondary_limits(mut self, min: f64, max: f64) -> Self {
        self.secondary_sp_min = min;
        self.secondary_sp_max = max;
        self
    }

    /// Compute cascade output
    pub fn compute(
        &mut self,
        primary_setpoint: f64,
        primary_measurement: f64,
        secondary_measurement: f64,
        dt_sec: f64,
    ) -> IndustrialResult<(f64, f64)> {
        // Primary controller produces setpoint for secondary
        let secondary_sp = self.primary.compute(primary_setpoint, primary_measurement, dt_sec)?;

        // Clamp secondary setpoint
        let secondary_sp = secondary_sp.clamp(self.secondary_sp_min, self.secondary_sp_max);

        // Secondary controller produces actual output
        let output = self.secondary.compute(secondary_sp, secondary_measurement, dt_sec)?;

        Ok((secondary_sp, output))
    }

    /// Reset both controllers
    pub fn reset(&mut self) {
        self.primary.reset();
        self.secondary.reset();
    }
}

/// Feedforward controller
#[derive(Debug, Clone)]
pub struct FeedforwardController {
    /// Feedforward gain
    pub gain: f64,
    /// Lead compensator time constant
    pub lead: f64,
    /// Lag compensator time constant
    pub lag: f64,
    /// Filter state
    filter_state: f64,
}

impl FeedforwardController {
    /// Create new feedforward controller
    pub fn new(gain: f64, lead: f64, lag: f64) -> Self {
        Self {
            gain,
            lead,
            lag,
            filter_state: 0.0,
        }
    }

    /// Compute feedforward output
    pub fn compute(&mut self, disturbance: f64, dt_sec: f64) -> f64 {
        // Lead-lag compensator
        let alpha = dt_sec / (self.lag + dt_sec);
        self.filter_state = alpha * self.gain * disturbance + (1.0 - alpha) * self.filter_state;
        self.filter_state
    }

    /// Reset filter state
    pub fn reset(&mut self) {
        self.filter_state = 0.0;
    }
}

/// Combined feedforward + feedback controller
#[derive(Debug, Clone)]
pub struct FeedforwardFeedbackController {
    pub feedforward: FeedforwardController,
    pub feedback: PidController,
}

impl FeedforwardFeedbackController {
    /// Create new combined controller
    pub fn new(feedforward: FeedforwardController, feedback: PidController) -> Self {
        Self {
            feedforward,
            feedback,
        }
    }

    /// Compute combined output
    pub fn compute(
        &mut self,
        setpoint: f64,
        measurement: f64,
        disturbance: f64,
        dt_sec: f64,
    ) -> IndustrialResult<f64> {
        let ff = self.feedforward.compute(disturbance, dt_sec);
        let fb = self.feedback.compute(setpoint, measurement, dt_sec)?;
        Ok(ff + fb)
    }

    /// Reset both controllers
    pub fn reset(&mut self) {
        self.feedforward.reset();
        self.feedback.reset();
    }
}

/// Sample and hold
#[derive(Debug, Clone)]
pub struct SampleAndHold {
    sampled_value: Option<f64>,
    last_sample_time_us: u64,
    sampling_period_us: u64,
}

impl SampleAndHold {
    /// Create new sample and hold
    pub fn new(sampling_period_us: u64) -> Self {
        Self {
            sampled_value: None,
            last_sample_time_us: 0,
            sampling_period_us,
        }
    }

    /// Sample value
    pub fn sample(&mut self, value: f64, timestamp_us: u64) -> IndustrialResult<()> {
        if self.sampled_value.is_some() && timestamp_us < self.last_sample_time_us + self.sampling_period_us {
            return Err(IndustrialError::Control(ControlError::InvalidSamplingPeriod {
                period_us: self.sampling_period_us,
                min_period_us: timestamp_us - self.last_sample_time_us,
            }));
        }

        self.sampled_value = Some(value);
        self.last_sample_time_us = timestamp_us;
        Ok(())
    }

    /// Get held value
    pub fn hold(&self) -> Option<f64> {
        self.sampled_value
    }

    /// Reset
    pub fn reset(&mut self) {
        self.sampled_value = None;
        self.last_sample_time_us = 0;
    }
}

/// Control loop statistics
#[derive(Debug, Clone)]
pub struct ControlLoopStats {
    pub setpoint: f64,
    pub process_variable: f64,
    pub control_output: f64,
    pub error: f64,
    pub integral: f64,
    pub derivative: f64,
}

/// Real-time control loop
pub struct RealtimeControlLoop {
    controller: PidController,
    setpoint: f64,
    stats_history: VecDeque<ControlLoopStats>,
    max_history: usize,
    execution_count: u64,
}

impl RealtimeControlLoop {
    /// Create new control loop
    pub fn new(controller: PidController, setpoint: f64) -> Self {
        Self {
            controller,
            setpoint,
            stats_history: VecDeque::new(),
            max_history: 1000,
            execution_count: 0,
        }
    }

    /// Execute control cycle
    pub fn execute(&mut self, measurement: f64, dt_sec: f64) -> IndustrialResult<f64> {
        let output = self.controller.compute(self.setpoint, measurement, dt_sec)?;

        // Record statistics
        let stats = ControlLoopStats {
            setpoint: self.setpoint,
            process_variable: measurement,
            control_output: output,
            error: self.setpoint - measurement,
            integral: self.controller.integral,
            derivative: (measurement - self.controller.previous_error) / dt_sec,
        };

        if self.stats_history.len() >= self.max_history {
            self.stats_history.pop_front();
        }
        self.stats_history.push_back(stats);

        self.execution_count += 1;
        Ok(output)
    }

    /// Update setpoint
    pub fn set_setpoint(&mut self, setpoint: f64) -> IndustrialResult<()> {
        if setpoint.is_nan() || setpoint.is_infinite() {
            return Err(IndustrialError::Control(ControlError::InvalidSetpoint {
                setpoint,
                min: f64::NEG_INFINITY,
                max: f64::INFINITY,
            }));
        }

        self.setpoint = setpoint;
        Ok(())
    }

    /// Get statistics
    pub fn get_statistics(&self) -> Option<&ControlLoopStats> {
        self.stats_history.back()
    }

    /// Get execution count
    pub fn execution_count(&self) -> u64 {
        self.execution_count
    }

    /// Reset controller
    pub fn reset(&mut self) {
        self.controller.reset();
        self.stats_history.clear();
        self.execution_count = 0;
    }

    /// Get real-time constraints
    pub fn realtime_constraints(&self) -> RealtimeConstraints {
        RealtimeConstraints::hard_realtime()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pid_controller() {
        let mut pid = PidController::new(1.0, 0.1, 0.01);
        let output = pid.compute(10.0, 8.0, 0.1).unwrap();
        assert!(output > 0.0);  // Positive output for positive error
    }

    #[test]
    fn test_pid_limits() {
        let mut pid = PidController::new(1.0, 0.0, 0.0)
            .with_output_limits(0.0, 100.0);
        let output = pid.compute(1000.0, 0.0, 0.1).unwrap();
        assert!(output <= 100.0);
    }

    #[test]
    fn test_cascade_controller() {
        let primary = PidController::new(1.0, 0.0, 0.0);
        let secondary = PidController::new(0.5, 0.0, 0.0);
        let mut cascade = CascadeController::new(primary, secondary);

        let (sp, output) = cascade.compute(10.0, 8.0, 9.0, 0.1).unwrap();
        assert!(sp > 0.0);
        assert!(output > 0.0);
    }

    #[test]
    fn test_sample_and_hold() {
        let mut sah = SampleAndHold::new(1000);
        sah.sample(5.0, 0).unwrap();
        assert_eq!(sah.hold(), Some(5.0));
    }

    #[test]
    fn test_control_loop() {
        let pid = PidController::new(1.0, 0.1, 0.01);
        let mut loop_ctrl = RealtimeControlLoop::new(pid, 10.0);

        let output = loop_ctrl.execute(8.0, 0.1).unwrap();
        assert!(output > 0.0);
        assert_eq!(loop_ctrl.execution_count(), 1);
    }
}
