//! Signal types and definitions

/// Signal state management
#[derive(Debug, Clone)]
pub struct SignalState {
    /// Pending signals
    pending: u64,
    /// Blocked signals
    blocked: u64,
    /// Signal disposition
    disposition: [u8; 64], // One disposition per signal (0-63)
}

impl SignalState {
    /// Create a new signal state
    pub fn new() -> Self {
        Self {
            pending: 0,
            blocked: 0,
            disposition: [0; 64],
        }
    }

    /// Add a pending signal
    pub fn add_pending(&mut self, sig: u32) {
        if sig < 64 {
            self.pending |= 1 << sig;
        }
    }

    /// Remove a pending signal
    pub fn remove_pending(&mut self, sig: u32) {
        if sig < 64 {
            self.pending &= !(1 << sig);
        }
    }

    /// Check if a signal is pending
    pub fn is_pending(&self, sig: u32) -> bool {
        sig < 64 && (self.pending & (1 << sig)) != 0
    }

    /// Block a signal
    pub fn block(&mut self, sig: u32) {
        if sig < 64 {
            self.blocked |= 1 << sig;
        }
    }

    /// Unblock a signal
    pub fn unblock(&mut self, sig: u32) {
        if sig < 64 {
            self.blocked &= !(1 << sig);
        }
    }

    /// Check if a signal is blocked
    pub fn is_blocked(&self, sig: u32) -> bool {
        sig < 64 && (self.blocked & (1 << sig)) != 0
    }

    /// Set signal disposition
    pub fn set_disposition(&mut self, sig: u32, disposition: u8) {
        if sig < 64 {
            self.disposition[sig as usize] = disposition;
        }
    }

    /// Get signal disposition
    pub fn get_disposition(&self, sig: u32) -> u8 {
        if sig < 64 {
            self.disposition[sig as usize]
        } else {
            0
        }
    }

    /// Get pending signals
    pub fn get_pending(&self) -> u64 {
        self.pending
    }

    /// Get blocked signals
    pub fn get_blocked(&self) -> u64 {
        self.blocked
    }

    /// Clear all pending signals
    pub fn clear_pending(&mut self) {
        self.pending = 0;
    }
}

impl Default for SignalState {
    fn default() -> Self {
        Self::new()
    }
}