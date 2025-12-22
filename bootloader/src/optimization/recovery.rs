//! Recovery mode support module
#![cfg(feature = "recovery_support")]

//! Recovery mode utilities for bootloader

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryMode {
    Safe,
    Diagnostic,
    Repair,
    Shell,
}

pub struct RecoveryEnvironment {
    mode: RecoveryMode,
}

impl RecoveryEnvironment {
    pub fn new(mode: RecoveryMode) -> Self {
        RecoveryEnvironment { mode }
    }

    pub fn mode(&self) -> RecoveryMode {
        self.mode
    }
}
