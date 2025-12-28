//! Signal Service Module
//!
//! Provides signal-related services

use nos_api::Result;

/// Signal service
pub struct SignalService;

impl SignalService {
    pub fn new() -> Self {
        Self
    }
}

/// Get signal service
pub fn get_signal_service() -> &'static SignalService {
    static SERVICE: SignalService = SignalService::new();
    &SERVICE
}
