//! Boot Menu System
//!
//! This module provides boot menu functionality for both BIOS and UEFI bootloaders,
//! allowing users to interactively select boot options, edit kernel parameters,
//! and configure system settings.

use crate::error::{BootError, Result};

#[cfg(feature = "bios_support")]
pub mod bios;

#[cfg(feature = "uefi_support")]
pub mod uefi;

/// Boot menu trait - all boot menu implementations must implement this
pub trait BootMenu {
    /// Display the boot menu and return the selected boot entry
    fn display_menu(&mut self) -> Result<BootMenuEntry>;

    /// Initialize the boot menu
    fn initialize(&mut self) -> Result<()>;

    /// Check if menu is initialized
    fn is_initialized(&self) -> bool;

    /// Get the current state
    fn get_state(&self) -> BootMenuState;

    /// Reboot the system
    fn reboot(&self) -> Result<()> {
        Err(BootError::FeatureNotEnabled("Reboot"))
    }
}

/// Boot menu entry
#[derive(Debug, Clone)]
pub struct BootMenuEntry {
    /// Display name
    pub name: String,
    /// Kernel path
    pub kernel_path: String,
    /// Command line arguments
    pub cmdline: String,
    /// Timeout in seconds (0 = manual boot)
    pub timeout: u8,
    /// Is this the default entry
    pub is_default: bool,
}

impl BootMenuEntry {
    /// Create a new boot menu entry
    pub fn new(name: String, kernel_path: String, cmdline: String) -> Self {
        Self {
            name,
            kernel_path,
            cmdline,
            timeout: 0,
            is_default: false,
        }
    }

    /// Create a default boot entry
    pub fn default_entry(name: String, kernel_path: String, cmdline: String) -> Self {
        let mut entry = Self::new(name, kernel_path, cmdline);
        entry.is_default = true;
        entry
    }

    /// Create an entry with timeout
    pub fn with_timeout(mut self, timeout: u8) -> Self {
        self.timeout = timeout;
        self
    }
}

/// Boot menu configuration
#[derive(Debug, Clone)]
pub struct BootMenuConfig {
    /// Menu entries
    pub entries: Vec<BootMenuEntry>,
    /// Global timeout (seconds)
    pub global_timeout: u8,
    /// Default entry index
    pub default_entry: usize,
    /// Show menu or boot directly
    pub show_menu: bool,
    /// Enable graphical menu
    pub graphical: bool,
    /// Menu title
    pub title: String,
}

impl BootMenuConfig {
    /// Create a new boot menu configuration
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            global_timeout: 5,
            default_entry: 0,
            show_menu: true,
            graphical: true,
            title: "NOS Operating System Boot Menu".to_string(),
        }
    }

    /// Add a boot entry
    pub fn add_entry(&mut self, entry: BootMenuEntry) {
        if entry.is_default {
            self.default_entry = self.entries.len();
        }
        self.entries.push(entry);
    }

    /// Get the default entry
    pub fn get_default_entry(&self) -> Option<&BootMenuEntry> {
        self.entries.get(self.default_entry)
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.entries.is_empty() {
            return Err(BootError::InvalidParameter("No boot entries configured"));
        }

        if self.default_entry >= self.entries.len() {
            return Err(BootError::InvalidParameter("Invalid default entry index"));
        }

        Ok(())
    }
}

/// Boot menu state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootMenuState {
    Initializing,
    ShowingMenu,
    TimeoutCountdown,
    EditingCmdline,
    WaitingForInput,
    BootSelected,
    Booting,
}

/// Create default boot menu configuration for NOS
pub fn create_default_config() -> BootMenuConfig {
    let mut config = BootMenuConfig::new();

    // Add normal boot entry
    config.add_entry(BootMenuEntry::default_entry(
        "NOS OS - Normal Boot".to_string(),
        "boot/kernel.bin".to_string(),
        "root=/dev/sda1 quiet splash".to_string(),
    ).with_timeout(5));

    // Add recovery boot entry
    config.add_entry(BootMenuEntry::new(
        "NOS OS - Recovery Mode".to_string(),
        "boot/kernel.bin".to_string(),
        "root=/dev/sda1 single recovery".to_string(),
    ));

    // Add debug boot entry
    config.add_entry(BootMenuEntry::new(
        "NOS OS - Debug Mode".to_string(),
        "boot/kernel.bin".to_string(),
        "root=/dev/sda1 debug=on console=ttyS0".to_string(),
    ));

    // Add fallback boot entry
    config.add_entry(BootMenuEntry::new(
        "NOS OS - Fallback".to_string(),
        "boot/kernel_fallback.bin".to_string(),
        "root=/dev/sda1".to_string(),
    ));

    config
}

/// Boot menu factory - creates appropriate boot menu based on features
pub fn create_boot_menu(config: BootMenuConfig) -> Result<Box<dyn BootMenu>> {
    #[cfg(feature = "bios_support")]
    {
        return Ok(Box::new(bios::BiosBootMenu::new(config)));
    }

    #[cfg(feature = "uefi_support")]
    {
        return Ok(Box::new(uefi::UefiBootMenu::new(config)));
    }

    #[cfg(not(any(feature = "bios_support", feature = "uefi_support")))]
    {
        Err(BootError::FeatureNotEnabled("Boot menu"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_boot_menu_entry_creation() {
        let entry = BootMenuEntry::new(
            "NOS OS".to_string(),
            "kernel.bin".to_string(),
            "quiet splash".to_string(),
        );

        assert_eq!(entry.name, "NOS OS");
        assert_eq!(entry.kernel_path, "kernel.bin");
        assert_eq!(entry.cmdline, "quiet splash");
        assert!(!entry.is_default);
        assert_eq!(entry.timeout, 0);
    }

    #[test]
    fn test_default_boot_entry() {
        let entry = BootMenuEntry::default_entry(
            "Default".to_string(),
            "kernel.bin".to_string(),
            "".to_string(),
        );

        assert!(entry.is_default);
    }

    #[test]
    fn test_boot_menu_config() {
        let mut config = BootMenuConfig::new();
        assert_eq!(config.entries.len(), 0);
        assert_eq!(config.global_timeout, 5);
        assert!(config.show_menu);
        assert!(config.graphical);

        // Empty config should fail validation
        assert!(config.validate().is_err());

        // Add an entry
        config.add_entry(BootMenuEntry::default_entry(
            "Default".to_string(),
            "kernel.bin".to_string(),
            "".to_string(),
        ));

        assert_eq!(config.entries.len(), 1);
        assert_eq!(config.default_entry, 0);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_create_default_config() {
        let config = create_default_config();

        assert_eq!(config.entries.len(), 4);
        assert_eq!(config.title, "NOS Operating System Boot Menu");
        assert_eq!(config.default_entry, 0);
        assert!(config.get_default_entry().is_some());
        assert!(config.validate().is_ok());

        // Check that the first entry is marked as default
        assert!(config.entries[0].is_default);
        assert_eq!(config.entries[0].name, "NOS OS - Normal Boot");
    }

    #[test]
    fn test_boot_menu_state() {
        use BootMenuState::*;

        assert_ne!(Initializing, ShowingMenu);
        assert_eq!(ShowingMenu, ShowingMenu);
        assert_ne!(BootSelected, Booting);
    }
}