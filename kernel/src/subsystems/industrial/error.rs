//! # Industrial Control System Error Types
//!
//! Comprehensive error handling for industrial automation systems.

use alloc::fmt;

/// Industrial result type
pub type IndustrialResult<T> = core::result::Result<T, IndustrialError>;

/// Industrial control system errors
#[derive(Debug, Clone, PartialEq)]
pub enum IndustrialError {
    /// SCADA system errors
    Scada(ScadaError),

    /// PLC errors
    Plc(PlcError),

    /// Protocol errors
    Protocol(ProtocolError),

    /// Control system errors
    Control(ControlError),

    /// Motion control errors
    Motion(MotionError),

    /// IIoT gateway errors
    Iiot(IiotError),

    /// Real-time violation
    RealtimeViolation {
        max_latency_us: u64,
        actual_latency_us: u64,
    },

    /// Safety violation
    SafetyViolation {
        required_sil: u8,
        actual_sil: u8,
    },

    /// Communication failure
    CommunicationFailed {
        protocol: &'static str,
        address: &'static str,
        reason: &'static str,
    },

    /// I/O error
    IoError {
        device: &'static str,
        operation: &'static str,
    },

    /// Timeout
    Timeout {
        operation: &'static str,
        timeout_ms: u64,
    },

    /// Invalid configuration
    InvalidConfiguration {
        parameter: &'static str,
        value: &'static str,
    },

    /// Hardware fault
    HardwareFault {
        component: &'static str,
        fault_code: u32,
    },

    /// Generic error
    Generic(&'static str),
}

impl fmt::Display for IndustrialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Scada(e) => write!(f, "SCADA error: {}", e),
            Self::Plc(e) => write!(f, "PLC error: {}", e),
            Self::Protocol(e) => write!(f, "Protocol error: {}", e),
            Self::Control(e) => write!(f, "Control error: {}", e),
            Self::Motion(e) => write!(f, "Motion error: {}", e),
            Self::Iiot(e) => write!(f, "IIoT error: {}", e),
            Self::RealtimeViolation { max_latency_us, actual_latency_us } => {
                write!(f, "Real-time violation: {}us > {}us", actual_latency_us, max_latency_us)
            }
            Self::SafetyViolation { required_sil, actual_sil } => {
                write!(f, "Safety violation: SIL{} < SIL{}", actual_sil, required_sil)
            }
            Self::CommunicationFailed { protocol, address, reason } => {
                write!(f, "Communication failed: {} to {} - {}", protocol, address, reason)
            }
            Self::IoError { device, operation } => {
                write!(f, "I/O error: {} operation on {}", operation, device)
            }
            Self::Timeout { operation, timeout_ms } => {
                write!(f, "Timeout: {} after {}ms", operation, timeout_ms)
            }
            Self::InvalidConfiguration { parameter, value } => {
                write!(f, "Invalid configuration: {} = {}", parameter, value)
            }
            Self::HardwareFault { component, fault_code } => {
                write!(f, "Hardware fault: {} (code: {})", component, fault_code)
            }
            Self::Generic(msg) => write!(f, "Industrial error: {}", msg),
        }
    }
}

/// SCADA system errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScadaError {
    /// Data acquisition failed
    DataAcquisitionFailed,

    /// Database error
    DatabaseError(&'static str),

    /// Alarm queue full
    AlarmQueueFull,

    /// Tag not found
    TagNotFound(alloc::string::String),

    /// Invalid tag value
    InvalidTagValue {
        tag: alloc::string::String,
        expected: &'static str,
        got: &'static str,
    },

    /// HMI connection lost
    HmiConnectionLost,

    /// Trend analysis error
    TrendAnalysisError(&'static str),

    /// Historical data error
    HistoricalDataError(&'static str),

    /// Server error
    ServerError(&'static str),
}

impl fmt::Display for ScadaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DataAcquisitionFailed => write!(f, "Data acquisition failed"),
            Self::DatabaseError(msg) => write!(f, "Database error: {}", msg),
            Self::AlarmQueueFull => write!(f, "Alarm queue is full"),
            Self::TagNotFound(tag) => write!(f, "Tag not found: {}", tag),
            Self::InvalidTagValue { tag, expected, got } => {
                write!(f, "Invalid value for tag {}: expected {}, got {}", tag, expected, got)
            }
            Self::HmiConnectionLost => write!(f, "HMI connection lost"),
            Self::TrendAnalysisError(msg) => write!(f, "Trend analysis error: {}", msg),
            Self::HistoricalDataError(msg) => write!(f, "Historical data error: {}", msg),
            Self::ServerError(msg) => write!(f, "Server error: {}", msg),
        }
    }
}

/// PLC errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlcError {
    /// Compilation failed
    CompilationFailed(&'static str),

    /// Invalid program
    InvalidProgram(&'static str),

    /// Execution timeout
    ExecutionTimeout {
        cycle_time_us: u64,
        max_cycle_time_us: u64,
    },

    /// I/O module error
    IoModuleError {
        module: u8,
        error_code: u32,
    },

    /// Invalid ladder logic
    InvalidLadderLogic(&'static str),

    /// Stack overflow
    StackOverflow,

    /// Memory overflow
    MemoryOverflow,

    /// Program not loaded
    ProgramNotLoaded,

    /// Invalid operation
    InvalidOperation(&'static str),

    /// Runtime error
    RuntimeError(&'static str),
}

impl fmt::Display for PlcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CompilationFailed(msg) => write!(f, "Compilation failed: {}", msg),
            Self::InvalidProgram(msg) => write!(f, "Invalid program: {}", msg),
            Self::ExecutionTimeout { cycle_time_us, max_cycle_time_us } => {
                write!(f, "Execution timeout: {}us > {}us", cycle_time_us, max_cycle_time_us)
            }
            Self::IoModuleError { module, error_code } => {
                write!(f, "I/O module {} error: code {}", module, error_code)
            }
            Self::InvalidLadderLogic(msg) => write!(f, "Invalid ladder logic: {}", msg),
            Self::StackOverflow => write!(f, "Stack overflow"),
            Self::MemoryOverflow => write!(f, "Memory overflow"),
            Self::ProgramNotLoaded => write!(f, "Program not loaded"),
            Self::InvalidOperation(msg) => write!(f, "Invalid operation: {}", msg),
            Self::RuntimeError(msg) => write!(f, "Runtime error: {}", msg),
        }
    }
}

/// Protocol errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProtocolError {
    /// Invalid function code
    InvalidFunctionCode(u8),

    /// Invalid CRC
    InvalidCrc {
        expected: u16,
        actual: u16,
    },

    /// Checksum error
    ChecksumError,

    /// Timeout
    Timeout,

    /// Buffer overflow
    BufferOverflow,

    /// Invalid address
    InvalidAddress(u16),

    /// Invalid data length
    InvalidDataLength(usize),

    /// Exception response
    ExceptionResponse {
        function_code: u8,
        exception_code: u8,
    },

    /// Frame error
    FrameError(&'static str),

    /// Not supported
    NotSupported(&'static str),

    /// Connection lost
    ConnectionLost,
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFunctionCode(code) => write!(f, "Invalid function code: {}", code),
            Self::InvalidCrc { expected, actual } => {
                write!(f, "Invalid CRC: expected 0x{:04X}, got 0x{:04X}", expected, actual)
            }
            Self::ChecksumError => write!(f, "Checksum error"),
            Self::Timeout => write!(f, "Protocol timeout"),
            Self::BufferOverflow => write!(f, "Buffer overflow"),
            Self::InvalidAddress(addr) => write!(f, "Invalid address: {}", addr),
            Self::InvalidDataLength(len) => write!(f, "Invalid data length: {}", len),
            Self::ExceptionResponse { function_code, exception_code } => {
                write!(f, "Exception: function={}, exception={}", function_code, exception_code)
            }
            Self::FrameError(msg) => write!(f, "Frame error: {}", msg),
            Self::NotSupported(msg) => write!(f, "Not supported: {}", msg),
            Self::ConnectionLost => write!(f, "Connection lost"),
        }
    }
}

/// Control system errors
#[derive(Debug, Clone, PartialEq)]
pub enum ControlError {
    /// Invalid PID parameters
    InvalidPidParameters {
        parameter: &'static str,
        value: f64,
    },

    /// Saturation exceeded
    SaturationExceeded {
        output: f64,
        min: f64,
        max: f64,
    },

    /// Divergence detected
    DivergenceDetected {
        setpoint: f64,
        process_variable: f64,
    },

    /// Invalid sampling period
    InvalidSamplingPeriod {
        period_us: u64,
        min_period_us: u64,
    },

    /// Actuator fault
    ActuatorFault {
        actuator: &'static str,
        fault_code: u32,
    },

    /// Sensor fault
    SensorFault {
        sensor: &'static str,
        fault_code: u32,
    },

    /// Control loop error
    ControlLoopError(&'static str),

    /// Invalid setpoint
    InvalidSetpoint {
        setpoint: f64,
        min: f64,
        max: f64,
    },
}

impl fmt::Display for ControlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPidParameters { parameter, value } => {
                write!(f, "Invalid PID parameter {} = {}", parameter, value)
            }
            Self::SaturationExceeded { output, min, max } => {
                write!(f, "Saturation exceeded: {} outside [{}, {}]", output, min, max)
            }
            Self::DivergenceDetected { setpoint, process_variable } => {
                write!(f, "Divergence: PV={} diverging from SP={}", process_variable, setpoint)
            }
            Self::InvalidSamplingPeriod { period_us, min_period_us } => {
                write!(f, "Invalid sampling period: {}us < {}us", period_us, min_period_us)
            }
            Self::ActuatorFault { actuator, fault_code } => {
                write!(f, "Actuator fault: {} (code: {})", actuator, fault_code)
            }
            Self::SensorFault { sensor, fault_code } => {
                write!(f, "Sensor fault: {} (code: {})", sensor, fault_code)
            }
            Self::ControlLoopError(msg) => write!(f, "Control loop error: {}", msg),
            Self::InvalidSetpoint { setpoint, min, max } => {
                write!(f, "Invalid setpoint: {} outside [{}, {}]", setpoint, min, max)
            }
        }
    }
}

/// Motion control errors
#[derive(Debug, Clone, PartialEq)]
pub enum MotionError {
    /// Invalid G-code
    InvalidGCode {
        line: usize,
        code: alloc::string::String,
        reason: &'static str,
    },

    /// Interpolation error
    InterpolationError(&'static str),

    /// Axis fault
    AxisFault {
        axis: u8,
        fault_code: u32,
    },

    /// Position error exceeded
    PositionErrorExceeded {
        axis: u8,
        error_um: i64,
        max_error_um: i64,
    },

    /// Velocity limit exceeded
    VelocityLimitExceeded {
        axis: u8,
        velocity: f64,
        max_velocity: f64,
    },

    /// Acceleration limit exceeded
    AccelerationLimitExceeded {
        axis: u8,
        acceleration: f64,
        max_acceleration: f64,
    },

    /// Path planning error
    PathPlanningError(&'static str),

    /// Servo error
    ServoError {
        axis: u8,
        error_code: u32,
    },

    /// Coordinate system error
    CoordinateSystemError(&'static str),
}

impl fmt::Display for MotionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidGCode { line, code, reason } => {
                write!(f, "Invalid G-code at line {}: {} - {}", line, code, reason)
            }
            Self::InterpolationError(msg) => write!(f, "Interpolation error: {}", msg),
            Self::AxisFault { axis, fault_code } => {
                write!(f, "Axis {} fault (code: {})", axis, fault_code)
            }
            Self::PositionErrorExceeded { axis, error_um, max_error_um } => {
                write!(f, "Axis {} position error: {}um > {}um", axis, error_um, max_error_um)
            }
            Self::VelocityLimitExceeded { axis, velocity, max_velocity } => {
                write!(f, "Axis {} velocity: {} > {}", axis, velocity, max_velocity)
            }
            Self::AccelerationLimitExceeded { axis, acceleration, max_acceleration } => {
                write!(f, "Axis {} acceleration: {} > {}", axis, acceleration, max_acceleration)
            }
            Self::PathPlanningError(msg) => write!(f, "Path planning error: {}", msg),
            Self::ServoError { axis, error_code } => {
                write!(f, "Axis {} servo error (code: {})", axis, error_code)
            }
            Self::CoordinateSystemError(msg) => write!(f, "Coordinate system error: {}", msg),
        }
    }
}

/// IIoT gateway errors
#[derive(Debug, Clone, PartialEq)]
pub enum IiotError {
    /// MQTT connection failed
    MqttConnectionFailed {
        broker: alloc::string::String,
        reason: &'static str,
    },

    /// OPC UA error
    OpcUaError(&'static str),

    /// Protocol conversion error
    ProtocolConversionError {
        from: &'static str,
        to: &'static str,
        reason: &'static str,
    },

    /// Data aggregation error
    DataAggregationError(&'static str),

    /// Edge computation error
    EdgeComputationError(&'static str),

    /// Authentication failed
    AuthenticationFailed {
        method: &'static str,
        reason: &'static str,
    },

    /// Buffer overflow
    BufferOverflow,

    /// Gateway overload
    GatewayOverload {
        throughput: f64,
        max_throughput: f64,
    },
}

impl fmt::Display for IiotError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MqttConnectionFailed { broker, reason } => {
                write!(f, "MQTT connection to {} failed: {}", broker, reason)
            }
            Self::OpcUaError(msg) => write!(f, "OPC UA error: {}", msg),
            Self::ProtocolConversionError { from, to, reason } => {
                write!(f, "Protocol conversion {} -> {} failed: {}", from, to, reason)
            }
            Self::DataAggregationError(msg) => write!(f, "Data aggregation error: {}", msg),
            Self::EdgeComputationError(msg) => write!(f, "Edge computation error: {}", msg),
            Self::AuthenticationFailed { method, reason } => {
                write!(f, "Authentication failed ({}): {}", method, reason)
            }
            Self::BufferOverflow => write!(f, "Buffer overflow"),
            Self::GatewayOverload { throughput, max_throughput } => {
                write!(f, "Gateway overload: {} > {}", throughput, max_throughput)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = IndustrialError::RealtimeViolation {
            max_latency_us: 100,
            actual_latency_us: 200,
        };
        assert_eq!(format!("{}", err), "Real-time violation: 200us > 100us");
    }

    #[test]
    fn test_scada_error() {
        let err = ScadaError::TagNotFound("temperature".into());
        assert_eq!(format!("{}", err), "Tag not found: temperature");
    }

    #[test]
    fn test_plc_error() {
        let err = PlcError::ExecutionTimeout {
            cycle_time_us: 2000,
            max_cycle_time_us: 1000,
        };
        assert!(format!("{}", err).contains("2000us"));
    }

    #[test]
    fn test_protocol_error() {
        let err = ProtocolError::InvalidCrc {
            expected: 0x1234,
            actual: 0x5678,
        };
        assert_eq!(format!("{}", err), "Invalid CRC: expected 0x1234, got 0x5678");
    }

    #[test]
    fn test_control_error() {
        let err = ControlError::SaturationExceeded {
            output: 110.0,
            min: 0.0,
            max: 100.0,
        };
        assert!(format!("{}", err).contains("110"));
    }

    #[test]
    fn test_motion_error() {
        let err = MotionError::InvalidGCode {
            line: 10,
            code: "G00 X100".into(),
            reason: "invalid syntax",
        };
        assert!(format!("{}", err).contains("line 10"));
    }

    #[test]
    fn test_iiot_error() {
        let err = IiotError::MqttConnectionFailed {
            broker: "localhost:1883".into(),
            reason: "connection refused",
        };
        assert!(format!("{}", err).contains("localhost:1883"));
    }
}
