//! XR-specific error types

use alloc::string::String;
use core::fmt;

/// XR operation errors
#[derive(Debug, Clone, PartialEq)]
pub enum XrError {
    /// Operation not supported
    NotSupported(String),

    /// Invalid argument
    InvalidArgument(String),

    /// Operation failed
    OperationFailed(String),

    /// Hardware error
    HardwareError(String),

    /// Sensor timeout
    SensorTimeout(String),

    /// Tracking lost
    TrackingLost(String),

    /// Initialization failed
    InitializationFailed(String),

    /// Out of memory
    OutOfMemory,

    /// Invalid state
    InvalidState(String),

    /// Feature not available
    FeatureNotAvailable(String),

    /// Timeout
    Timeout(String),

    /// Calibration required
    CalibrationRequired(String),

    /// Sensor error
    SensorError(String),

    /// Render error
    RenderError(String),

    /// Audio error
    AudioError(String),
}

impl fmt::Display for XrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotSupported(msg) => write!(f, "Not supported: {}", msg),
            Self::InvalidArgument(msg) => write!(f, "Invalid argument: {}", msg),
            Self::OperationFailed(msg) => write!(f, "Operation failed: {}", msg),
            Self::HardwareError(msg) => write!(f, "Hardware error: {}", msg),
            Self::SensorTimeout(msg) => write!(f, "Sensor timeout: {}", msg),
            Self::TrackingLost(msg) => write!(f, "Tracking lost: {}", msg),
            Self::InitializationFailed(msg) => write!(f, "Initialization failed: {}", msg),
            Self::OutOfMemory => write!(f, "Out of memory"),
            Self::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            Self::FeatureNotAvailable(msg) => write!(f, "Feature not available: {}", msg),
            Self::Timeout(msg) => write!(f, "Timeout: {}", msg),
            Self::CalibrationRequired(msg) => write!(f, "Calibration required: {}", msg),
            Self::SensorError(msg) => write!(f, "Sensor error: {}", msg),
            Self::RenderError(msg) => write!(f, "Render error: {}", msg),
            Self::AudioError(msg) => write!(f, "Audio error: {}", msg),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for XrError {}

/// Result type for XR operations
pub type XrResult<T> = Result<T, XrError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = XrError::TrackingLost("Camera occluded".into());
        assert_eq!(format!("{}", err), "Tracking lost: Camera occluded");
    }

    #[test]
    fn test_not_supported() {
        let err = XrError::NotSupported("Feature not available".into());
        assert!(matches!(err, XrError::NotSupported(_)));
    }
}
