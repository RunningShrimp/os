//! Signal Service Module
//!
//! Provides signal-related services

/// Signal service
pub struct SignalService;

impl SignalService {
    pub fn new() -> Self {
        Self
    }

    /// Const constructor for static initialization
    pub const fn const_new() -> Self {
        Self
    }
}

/// Get signal service
pub fn get_signal_service() -> &'static SignalService {
    static SERVICE: SignalService = SignalService::const_new();
    &SERVICE
}
