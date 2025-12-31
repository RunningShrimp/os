//! TCP Fast Open (TFO)
//!
//! This module implements TCP Fast Open (RFC 7413), which allows data
//! to be sent in the initial SYN packet, reducing latency by one RTT.
//!
//! Key features:
//! - TFO cookies for security
//! - Fast open with data in SYN
//! - Cookie generation and validation
//! - Connection establishment without waiting for SYN-ACK
//! - Reduced latency for repeated connections
//!
//! References:
//! - RFC 7413: TCP Fast Open

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

/// TFO cookie length in bytes
pub const TFO_COOKIE_LEN: usize = 16;

/// TFO maximum cookie lifetime (seconds)
pub const TFO_COOKIE_MAX_AGE: u64 = 86400; // 24 hours

/// TFO state for a connection
#[derive(Debug, Clone)]
pub struct TfoState {
    /// TFO cookie (if available)
    pub cookie: Option<[u8; TFO_COOKIE_LEN]>,
    /// Cookie expiration time (timestamp)
    pub cookie_exp: u64,
    /// TFO enabled flag
    pub enabled: bool,
    /// TFO requested (client sent cookie in SYN)
    pub requested: bool,
    /// TFO accepted (server accepted cookie)
    pub accepted: bool,
    /// Data sent in SYN
    pub syn_data: Option<Vec<u8>>,
    /// Maximum data length in SYN
    max_syn_data: usize,
    /// Cookie secret (for generation/validation)
    secret: [u8; 32],
}

impl TfoState {
    /// Create a new TFO state
    pub fn new() -> Self {
        Self {
            cookie: None,
            cookie_exp: 0,
            enabled: true,
            requested: false,
            accepted: false,
            syn_data: None,
            max_syn_data: 1460, // Default MSS
            secret: Self::generate_secret(),
        }
    }

    /// Generate a secret key for cookie generation
    fn generate_secret() -> [u8; 32] {
        // In a real implementation, this would use a cryptographically secure RNG
        // For now, use a timestamp-based approach
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        let timestamp = COUNTER.fetch_add(1, Ordering::SeqCst);

        let mut secret = [0u8; 32];
        secret[0..8].copy_from_slice(&timestamp.to_be_bytes());
        // Fill with pseudo-random data
        for i in 8..32 {
            secret[i] = ((timestamp as u64).wrapping_mul(i as u64 + 1)) as u8;
        }
        secret
    }

    /// Generate a TFO cookie for a client
    ///
    /// # Arguments
    /// * `client_ip` - Client IP address (as u32)
    /// * `now` - Current timestamp (seconds since epoch)
    pub fn generate_cookie(&mut self, client_ip: u32, now: u64) {
        // Cookie includes: client IP, timestamp, and MAC
        let mut cookie = [0u8; TFO_COOKIE_LEN];

        // Add client IP
        cookie[0..4].copy_from_slice(&client_ip.to_be_bytes());

        // Add timestamp (lower 32 bits)
        cookie[4..8].copy_from_slice(&((now as u32).to_be_bytes()));

        // Add MAC (Message Authentication Code) using the secret
        let mac = self.calculate_mac(client_ip, now);
        cookie[8..TFO_COOKIE_LEN].copy_from_slice(&mac[..8]);

        // Store cookie and expiration
        self.cookie = Some(cookie);
        self.cookie_exp = now + TFO_COOKIE_MAX_AGE;
    }

    /// Calculate MAC for cookie
    fn calculate_mac(&self, client_ip: u32, timestamp: u64) -> [u8; 16] {
        // Simple MAC: HMAC-like construction
        let mut mac = [0u8; 16];

        // XOR client IP and timestamp with secret
        for i in 0..4 {
            mac[i] = ((client_ip >> (i * 8)) as u8) ^ self.secret[i];
        }

        for i in 0..8 {
            mac[i + 4] = ((timestamp >> (i * 8)) as u8) ^ self.secret[i + 4];
        }

        // Add some mixing
        for i in 0..16 {
            mac[i] = mac[i].wrapping_add(self.secret[i + 16]);
        }

        mac
    }

    /// Validate a TFO cookie
    ///
    /// # Arguments
    /// * `cookie` - Cookie to validate
    /// * `client_ip` - Client IP address
    /// * `now` - Current timestamp
    pub fn validate_cookie(&self, cookie: &[u8], client_ip: u32, now: u64) -> bool {
        // Check cookie length
        if cookie.len() != TFO_COOKIE_LEN {
            return false;
        }

        // Extract timestamp from cookie
        let cookie_bytes: [u8; TFO_COOKIE_LEN] = cookie.try_into().unwrap_or([0u8; TFO_COOKIE_LEN]);
        let timestamp = u32::from_be_bytes(cookie_bytes[4..8].try_into().unwrap_or([0u8; 4])) as u64;

        // Check if cookie has expired
        if now.saturating_sub(timestamp) > TFO_COOKIE_MAX_AGE {
            return false;
        }

        // Verify client IP matches
        let stored_ip = u32::from_be_bytes(cookie_bytes[0..4].try_into().unwrap_or([0u8; 4]));
        if stored_ip != client_ip {
            return false;
        }

        // Verify MAC
        let expected_mac = self.calculate_mac(client_ip, timestamp);
        let provided_mac = &cookie_bytes[8..TFO_COOKIE_LEN];

        // Constant-time comparison
        let mut match_count = 0u8;
        for i in 0..8 {
            match_count += expected_mac[i].wrapping_sub(provided_mac[i]);
        }

        match_count == 0
    }

    /// Send a TFO SYN with data
    ///
    /// # Arguments
    /// * `data` - Data to send in SYN
    pub fn send_fast_open(&mut self, data: &[u8]) -> Result<(), TfoError> {
        if !self.enabled {
            return Err(TfoError::Disabled);
        }

        if self.cookie.is_none() {
            return Err(TfoError::NoCookie);
        }

        // Check data length
        if data.len() > self.max_syn_data {
            return Err(TfoError::DataTooLong);
        }

        // Store data to send in SYN
        self.syn_data = Some(data.to_vec());
        self.requested = true;

        Ok(())
    }

    /// Accept a TFO connection with cookie
    ///
    /// # Arguments
    /// * `cookie` - Cookie from client
    /// * `client_ip` - Client IP address
    /// * `now` - Current timestamp
    pub fn accept_fast_open(&mut self, cookie: &[u8], client_ip: u32, now: u64) -> Result<(), TfoError> {
        if !self.enabled {
            return Err(TfoError::Disabled);
        }

        // Validate cookie
        if !self.validate_cookie(cookie, client_ip, now) {
            return Err(TfoError::InvalidCookie);
        }

        self.accepted = true;

        Ok(())
    }

    /// Check if cookie has expired
    ///
    /// # Arguments
    /// * `now` - Current timestamp
    pub fn is_cookie_expired(&self, now: u64) -> bool {
        if self.cookie_exp == 0 {
            return true;
        }

        now >= self.cookie_exp
    }

    /// Get the current cookie (if available)
    pub fn get_cookie(&self) -> Option<[u8; TFO_COOKIE_LEN]> {
        self.cookie
    }

    /// Get data to send in SYN
    pub fn get_syn_data(&self) -> Option<&[u8]> {
        self.syn_data.as_deref()
    }

    /// Clear the SYN data (after sending)
    pub fn clear_syn_data(&mut self) {
        self.syn_data = None;
    }

    /// Set maximum data length for SYN
    pub fn set_max_syn_data(&mut self, max: usize) {
        self.max_syn_data = max;
    }

    /// Enable or disable TFO
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Check if TFO is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Check if TFO was requested
    pub fn is_requested(&self) -> bool {
        self.requested
    }

    /// Check if TFO was accepted
    pub fn is_accepted(&self) -> bool {
        self.accepted
    }

    /// Reset TFO state (for new connections)
    pub fn reset(&mut self) {
        self.requested = false;
        self.accepted = false;
        self.syn_data = None;
    }

    /// Convert cookie to bytes for transmission
    pub fn cookie_to_bytes(&self) -> Vec<u8> {
        if let Some(cookie) = self.cookie {
            cookie.to_vec()
        } else {
            Vec::new()
        }
    }

    /// Parse cookie from received bytes
    pub fn cookie_from_bytes(&mut self, bytes: &[u8]) -> Result<(), TfoError> {
        if bytes.len() != TFO_COOKIE_LEN {
            return Err(TfoError::InvalidCookie);
        }

        let mut cookie = [0u8; TFO_COOKIE_LEN];
        cookie.copy_from_slice(bytes);
        self.cookie = Some(cookie);

        Ok(())
    }
}

impl Default for TfoState {
    fn default() -> Self {
        Self::new()
    }
}

/// TFO errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TfoError {
    /// TFO is disabled
    Disabled,
    /// No cookie available
    NoCookie,
    /// Invalid cookie
    InvalidCookie,
    /// Cookie expired
    CookieExpired,
    /// Data too long for SYN
    DataTooLong,
    /// Invalid state
    InvalidState,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tfo_creation() {
        let tfo = TfoState::new();
        assert!(tfo.enabled);
        assert!(tfo.cookie.is_none());
        assert!(!tfo.requested);
        assert!(!tfo.accepted);
    }

    #[test]
    fn test_tfo_generate_cookie() {
        let mut tfo = TfoState::new();
        let client_ip = 0xC0A80101u32; // 192.168.1.1
        let now = 1_000_000;

        tfo.generate_cookie(client_ip, now);

        assert!(tfo.cookie.is_some());
        assert!(!tfo.is_cookie_expired(now));
    }

    #[test]
    fn test_tfo_validate_cookie() {
        let mut tfo = TfoState::new();
        let client_ip = 0xC0A80101u32; // 192.168.1.1
        let now = 1_000_000;

        // Generate cookie
        tfo.generate_cookie(client_ip, now);
        let cookie = tfo.get_cookie().unwrap();

        // Validate cookie
        assert!(tfo.validate_cookie(&cookie, client_ip, now));
    }

    #[test]
    fn test_tfo_validate_invalid_cookie() {
        let tfo = TfoState::new();
        let client_ip = 0xC0A80101u32; // 192.168.1.1
        let now = 1_000_000;

        // Invalid cookie length
        assert!(!tfo.validate_cookie(&[1, 2, 3], client_ip, now));
    }

    #[test]
    fn test_tfo_cookie_expiration() {
        let mut tfo = TfoState::new();
        let client_ip = 0xC0A80101u32; // 192.168.1.1
        let now = 1_000_000;

        tfo.generate_cookie(client_ip, now);

        // Cookie should not be expired immediately
        assert!(!tfo.is_cookie_expired(now));

        // Cookie should be expired after max age
        assert!(tfo.is_cookie_expired(now + TFO_COOKIE_MAX_AGE + 1));
    }

    #[test]
    fn test_tfo_send_fast_open() {
        let mut tfo = TfoState::new();
        let client_ip = 0xC0A80101u32;
        let now = 1_000_000;

        // Generate cookie first
        tfo.generate_cookie(client_ip, now);

        // Send fast open
        let data = b"Hello, world!";
        let result = tfo.send_fast_open(data);

        assert!(result.is_ok());
        assert!(tfo.is_requested());
        assert_eq!(tfo.get_syn_data(), Some(data.as_slice()));
    }

    #[test]
    fn test_tfo_send_fast_open_no_cookie() {
        let mut tfo = TfoState::new();
        let data = b"Hello, world!";

        let result = tfo.send_fast_open(data);

        assert_eq!(result, Err(TfoError::NoCookie));
    }

    #[test]
    fn test_tfo_send_fast_open_disabled() {
        let mut tfo = TfoState::new();
        tfo.set_enabled(false);
        let data = b"Hello, world!";

        let result = tfo.send_fast_open(data);

        assert_eq!(result, Err(TfoError::Disabled));
    }

    #[test]
    fn test_tfo_send_fast_open_too_long() {
        let mut tfo = TfoState::new();
        let client_ip = 0xC0A80101u32;
        let now = 1_000_000;

        tfo.generate_cookie(client_ip, now);
        tfo.set_max_syn_data(10);

        let data = b"This is way too long data";
        let result = tfo.send_fast_open(data);

        assert_eq!(result, Err(TfoError::DataTooLong));
    }

    #[test]
    fn test_tfo_accept_fast_open() {
        let mut tfo = TfoState::new();
        let client_ip = 0xC0A80101u32;
        let now = 1_000_000;

        // Generate cookie
        tfo.generate_cookie(client_ip, now);
        let cookie = tfo.get_cookie().unwrap();

        // Accept fast open
        let result = tfo.accept_fast_open(&cookie, client_ip, now);

        assert!(result.is_ok());
        assert!(tfo.is_accepted());
    }

    #[test]
    fn test_tfo_accept_fast_open_invalid() {
        let mut tfo = TfoState::new();
        let client_ip = 0xC0A80101u32;
        let now = 1_000_000;

        // Try to accept with invalid cookie
        let result = tfo.accept_fast_open(&[1, 2, 3], client_ip, now);

        assert_eq!(result, Err(TfoError::InvalidCookie));
        assert!(!tfo.is_accepted());
    }

    #[test]
    fn test_tfo_clear_syn_data() {
        let mut tfo = TfoState::new();
        let client_ip = 0xC0A80101u32;
        let now = 1_000_000;

        tfo.generate_cookie(client_ip, now);
        tfo.send_fast_open(b"Hello").unwrap();

        assert!(tfo.get_syn_data().is_some());

        tfo.clear_syn_data();

        assert!(tfo.get_syn_data().is_none());
    }

    #[test]
    fn test_tfo_reset() {
        let mut tfo = TfoState::new();
        let client_ip = 0xC0A80101u32;
        let now = 1_000_000;

        tfo.generate_cookie(client_ip, now);
        tfo.send_fast_open(b"Hello").unwrap();

        tfo.reset();

        assert!(!tfo.is_requested());
        assert!(!tfo.is_accepted());
        assert!(tfo.get_syn_data().is_none());
    }

    #[test]
    fn test_tfo_cookie_to_bytes() {
        let mut tfo = TfoState::new();
        let client_ip = 0xC0A80101u32;
        let now = 1_000_000;

        tfo.generate_cookie(client_ip, now);

        let bytes = tfo.cookie_to_bytes();
        assert_eq!(bytes.len(), TFO_COOKIE_LEN);
    }

    #[test]
    fn test_tfo_cookie_from_bytes() {
        let mut tfo = TfoState::new();
        let client_ip = 0xC0A80101u32;
        let now = 1_000_000;

        tfo.generate_cookie(client_ip, now);
        let bytes = tfo.cookie_to_bytes();

        let mut tfo2 = TfoState::new();
        let result = tfo2.cookie_from_bytes(&bytes);

        assert!(result.is_ok());
        assert_eq!(tfo2.cookie, tfo.cookie);
    }

    #[test]
    fn test_tfo_configuration() {
        let mut tfo = TfoState::new();

        tfo.set_enabled(false);
        assert!(!tfo.is_enabled());

        tfo.set_max_syn_data(2048);
        assert_eq!(tfo.max_syn_data, 2048);
    }
}
