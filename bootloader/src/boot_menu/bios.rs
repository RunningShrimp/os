//! BIOS Boot Menu System
//!
//! This module provides an interactive boot menu for BIOS bootloader,
//! allowing users to select kernel options, boot parameters, and configure
//! system settings.

use crate::error::{BootError, Result};
use crate::graphics::{GraphicsManager, SimpleFont};
use crate::protocol::FramebufferInfo;
use core::ptr;

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

/// Boot menu display modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMode {
    Text,
    Graphics,
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

/// BIOS Boot Menu
pub struct BiosBootMenu {
    config: BootMenuConfig,
    state: BootMenuState,
    selected_entry: usize,
    timeout_counter: u8,
    display_mode: DisplayMode,
    graphics_manager: Option<GraphicsManager>,
    framebuffer_info: Option<FramebufferInfo>,
    cmdline_buffer: [u8; 256],
    cmdline_length: usize,
}

impl BiosBootMenu {
    /// Create a new BIOS boot menu
    pub fn new(config: BootMenuConfig) -> Self {
        Self {
            config,
            state: BootMenuState::Initializing,
            selected_entry: 0,
            timeout_counter: 0,
            display_mode: DisplayMode::Text,
            graphics_manager: None,
            framebuffer_info: None,
            cmdline_buffer: [0; 256],
            cmdline_length: 0,
        }
    }

    /// Initialize the boot menu
    pub fn initialize(&mut self, framebuffer_info: Option<FramebufferInfo>) -> Result<()> {
        println!("[boot_menu] Initializing BIOS boot menu...");

        // Validate configuration
        self.config.validate()?;

        // Setup display mode
        if let Some(fb_info) = framebuffer_info.clone() {
            if self.config.graphical {
                self.setup_graphics_display(fb_info)?;
                self.display_mode = DisplayMode::Graphics;
            } else {
                self.setup_text_display()?;
                self.display_mode = DisplayMode::Text;
            }
        } else {
            self.setup_text_display()?;
            self.display_mode = DisplayMode::Text;
        }

        // Initialize state
        self.selected_entry = self.config.default_entry;
        self.timeout_counter = self.config.global_timeout;
        self.state = BootMenuState::ShowingMenu;

        println!("[boot_menu] Boot menu initialized with {} entries", self.config.entries.len());

        Ok(())
    }

    /// Setup graphics display
    fn setup_graphics_display(&mut self, fb_info: FramebufferInfo) -> Result<()> {
        let mut graphics = GraphicsManager::new();
        graphics.initialize(fb_info)?;

        // Clear screen
        graphics.clear_screen(0x1A1A2E)?; // Dark blue background

        // Draw title
        let mut font = SimpleFont::new(&mut graphics);
        let title_x = 50;
        let title_y = 50;
        font.draw_string(title_x, title_y, &self.config.title, 0xFFFFFF)?;

        // Draw boot entries
        for (i, entry) in self.config.entries.iter().enumerate() {
            let y = title_y + 60 + (i as u32 * 30);
            let color = if i == self.selected_entry { 0x00FF00 } else { 0xFFFFFF };
            let prefix = if i == self.selected_entry { "> " } else { "  " };

            let display_text = format!("{}{}: {}", prefix, i + 1, entry.name);
            font.draw_string(title_x + 20, y, &display_text, color)?;
        }

        // Draw instructions
        let instructions_y = title_y + 60 + (self.config.entries.len() as u32 * 30) + 40;
        font.draw_string(title_x, instructions_y, "↑↓: Select  ENTER: Boot  E: Edit  ESC: Reboot", 0xCCCCCC)?;

        self.graphics_manager = Some(graphics);
        self.framebuffer_info = Some(fb_info);

        Ok(())
    }

    /// Setup text display
    fn setup_text_display(&self) -> Result<()> {
        // Clear screen using VGA BIOS
        unsafe {
            let mut regs = BiosRegisters {
                eax: 0x0000, // Clear screen
                ebx: 0x0007, // Page 0, attribute 7 (white on black)
                ecx: 0x0000,
                edx: 0x184F, // Clear entire screen
                esi: 0,
                edi: 0,
                ebp: 0,
            };

            self.bios_int10(&mut regs);
        }

        // Print menu title
        self.print_text(0, 0, &self.config.title)?;

        // Print boot entries
        for (i, entry) in self.config.entries.iter().enumerate() {
            let line = i + 2;
            let prefix = if i == self.selected_entry { "> " } else { "  " };
            let text = format!("{}{}: {}", prefix, i + 1, entry.name);
            self.print_text(line, 0, &text)?;
        }

        // Print instructions
        let instructions_line = 2 + self.config.entries.len() + 1;
        self.print_text(instructions_line, 0, "↑↓: Select  ENTER: Boot  E: Edit  ESC: Reboot")?;

        Ok(())
    }

    /// Display the boot menu and handle user input
    pub fn display_menu(&mut self) -> Result<&BootMenuEntry> {
        println!("[boot_menu] Displaying boot menu...");

        loop {
            match self.state {
                BootMenuState::ShowingMenu => {
                    self.handle_menu_input()?;
                }
                BootMenuState::TimeoutCountdown => {
                    self.handle_timeout()?;
                }
                BootMenuState::EditingCmdline => {
                    self.handle_cmdline_edit()?;
                }
                BootMenuState::WaitingForInput => {
                    self.wait_for_boot_confirmation()?;
                }
                BootMenuState::BootSelected | BootMenuState::Booting => {
                    return Ok(&self.config.entries[self.selected_entry]);
                }
                _ => {}
            }
        }
    }

    /// Handle menu input
    fn handle_menu_input(&mut self) -> Result<()> {
        let key = self.wait_for_keypress(100)?; // 100ms timeout

        match key {
            Some(0x48) => { // Up arrow
                if self.selected_entry > 0 {
                    self.selected_entry -= 1;
                    self.refresh_menu_display()?;
                }
            }
            Some(0x50) => { // Down arrow
                if self.selected_entry < self.config.entries.len() - 1 {
                    self.selected_entry += 1;
                    self.refresh_menu_display()?;
                }
            }
            Some(0x1C) => { // Enter
                self.state = BootMenuState::BootSelected;
                println!("[boot_menu] Selected boot entry: {}", self.config.entries[self.selected_entry].name);
            }
            Some(0x12) => { // E key
                self.state = BootMenuState::EditingCmdline;
                self.setup_cmdline_edit()?;
            }
            Some(0x01) => { // ESC
                self.reboot_system()?;
            }
            None => {
                // No key pressed, handle timeout
                if self.timeout_counter > 0 {
                    self.state = BootMenuState::TimeoutCountdown;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Handle timeout countdown
    fn handle_timeout(&mut self) -> Result<()> {
        let key = self.wait_for_keypress(1000)?; // 1 second

        if key.is_some() {
            self.state = BootMenuState::ShowingMenu;
            return Ok(());
        }

        self.timeout_counter -= 1;

        if self.timeout_counter == 0 {
            // Boot default entry
            self.selected_entry = self.config.default_entry;
            self.state = BootMenuState::BootSelected;
            println!("[boot_menu] Timeout reached, booting default entry");
        } else {
            // Update timeout display
            self.update_timeout_display()?;
        }

        Ok(())
    }

    /// Handle command line editing
    fn handle_cmdline_edit(&mut self) -> Result<()> {
        let key = self.wait_for_keypress(100)?;

        match key {
            Some(0x1C) => { // Enter
                // Apply command line and go back to menu
                let cmdline_str = core::str::from_utf8(&self.cmdline_buffer[..self.cmdline_length])
                    .unwrap_or("");
                self.config.entries[self.selected_entry].cmdline = cmdline_str.to_string();
                self.state = BootMenuState::ShowingMenu;
                self.refresh_menu_display()?;
            }
            Some(0x01) => { // ESC
                // Cancel editing
                self.state = BootMenuState::ShowingMenu;
                self.refresh_menu_display()?;
            }
            Some(0x0E) => { // Backspace
                if self.cmdline_length > 0 {
                    self.cmdline_length -= 1;
                    self.update_cmdline_display()?;
                }
            }
            Some(scancode) => {
                // Handle alphanumeric keys (simplified)
                if let Some(ch) = self.scancode_to_char(scancode) {
                    if self.cmdline_length < self.cmdline_buffer.len() - 1 {
                        self.cmdline_buffer[self.cmdline_length] = ch as u8;
                        self.cmdline_length += 1;
                        self.update_cmdline_display()?;
                    }
                }
            }
            None => {}
        }

        Ok(())
    }

    /// Setup command line editing display
    fn setup_cmdline_edit(&mut self) -> Result<()> {
        let entry = &self.config.entries[self.selected_entry];
        let prompt = format!("Edit kernel parameters for: {}", entry.name);

        match self.display_mode {
            DisplayMode::Graphics => {
                if let Some(graphics) = &mut self.graphics_manager {
                    // Clear screen
                    graphics.clear_screen(0x1A1A2E)?;

                    let mut font = SimpleFont::new(graphics);
                    font.draw_string(50, 50, &prompt, 0xFFFFFF)?;
                    font.draw_string(50, 80, "Command line: ", 0xFFFFFF)?;
                    font.draw_string(50, 110, "Press ENTER to save, ESC to cancel", 0xCCCCCC)?;
                }
            }
            DisplayMode::Text => {
                self.clear_screen()?;
                self.print_text(0, 0, &prompt)?;
                self.print_text(2, 0, "Command line: ")?;
                self.print_text(4, 0, "Press ENTER to save, ESC to cancel")?;
            }
        }

        // Initialize cmdline buffer with current parameters
        let current_cmdline = entry.cmdline.as_bytes();
        self.cmdline_length = current_cmdline.len().min(self.cmdline_buffer.len() - 1);
        self.cmdline_buffer[..self.cmdline_length].copy_from_slice(&current_cmdline[..self.cmdline_length]);

        Ok(())
    }

    /// Refresh menu display
    fn refresh_menu_display(&mut self) -> Result<()> {
        match self.display_mode {
            DisplayMode::Graphics => {
                if let (Some(fb_info), Some(graphics)) = (self.framebuffer_info, &mut self.graphics_manager) {
                    self.setup_graphics_display(fb_info)?;
                }
            }
            DisplayMode::Text => {
                self.setup_text_display()?;
            }
        }
        Ok(())
    }

    /// Update timeout display
    fn update_timeout_display(&mut self) -> Result<()> {
        let timeout_msg = format!("Booting default entry in {} seconds...", self.timeout_counter);

        match self.display_mode {
            DisplayMode::Graphics => {
                if let Some(graphics) = &self.graphics_manager {
                    let mut font = SimpleFont::new(graphics);
                    font.draw_string(50, 400, &timeout_msg, 0xFFFF00)?;
                }
            }
            DisplayMode::Text => {
                let line = 2 + self.config.entries.len() + 2;
                self.print_text(line, 0, &timeout_msg)?;
            }
        }

        Ok(())
    }

    /// Update command line display
    fn update_cmdline_display(&mut self) -> Result<()> {
        let cmdline_str = core::str::from_utf8(&self.cmdline_buffer[..self.cmdline_length])
            .unwrap_or("");

        match self.display_mode {
            DisplayMode::Graphics => {
                if let Some(graphics) = &self.graphics_manager {
                    // Clear the command line area
                    graphics.fill_rect(50, 80, 700, 20, 0x1A1A2E)?;

                    let mut font = SimpleFont::new(graphics);
                    font.draw_string(50, 80, &format!("Command line: {}", cmdline_str), 0xFFFFFF)?;
                }
            }
            DisplayMode::Text => {
                self.print_text(2, 14, &format!("Command line: {}", cmdline_str))?;
            }
        }

        Ok(())
    }

    /// Wait for boot confirmation (optional)
    fn wait_for_boot_confirmation(&mut self) -> Result<()> {
        println!("[boot_menu] Booting {}...", self.config.entries[self.selected_entry].name);

        // Small delay before booting
        for _ in 0..10 {
            self.wait_for_keypress(100)?;
        }

        self.state = BootMenuState::Booting;
        Ok(())
    }

    /// Reboot the system
    fn reboot_system(&self) -> Result<()> {
        println!("[boot_menu] Rebooting system...");

        unsafe {
            // Use keyboard controller reset
            let mut value = ptr::read_volatile(0x64 as *const u8);

            // Wait for keyboard controller ready
            for _ in 0..100000 {
                value = ptr::read_volatile(0x64 as *const u8);
                if value & 0x02 == 0 {
                    break;
                }
            }

            // Write reset command
            ptr::write_volatile(0x64 as *mut u8, 0xFE);

            // Should not reach here
            loop {
                core::hint::spin_loop();
            }
        }
    }

    /// Wait for keypress with timeout
    fn wait_for_keypress(&self, timeout_ms: u32) -> Result<Option<u8>> {
        // Simple keyboard polling (would be more sophisticated in real BIOS)
        for _ in 0..timeout_ms {
            let key = unsafe { self.get_key() };
            if key != 0 {
                return Ok(Some(key));
            }
        }
        Ok(None)
    }

    /// Get key from keyboard
    unsafe fn get_key(&self) -> u8 {
        // Check if key is available
        let mut regs = BiosRegisters {
            eax: 0x0100, // Get key status
            ebx: 0,
            ecx: 0,
            edx: 0,
            esi: 0,
            edi: 0,
            ebp: 0,
        };

        let result = self.bios_int16(&mut regs);

        if result.eax & 0xFF != 0 {
            // Key available, read it
            regs.eax = 0x0000; // Get key
            let result = self.bios_int16(&mut regs);
            (result.eax & 0xFF) as u8
        } else {
            0
        }
    }

    /// Print text to screen
    fn print_text(&self, line: u8, column: u8, text: &str) -> Result<()> {
        for (i, ch) in text.chars().enumerate() {
            unsafe {
                let mut regs = BiosRegisters {
                    eax: 0x0E00 | (ch as u32), // Write character
                    ebx: 0x0007, // Page 0, attribute 7 (white on black)
                    ecx: 0,
                    edx: ((line as u32) << 8) | ((column + i as u8) as u32),
                    esi: 0,
                    edi: 0,
                    ebp: 0,
                };

                self.bios_int10(&mut regs);
            }
        }
        Ok(())
    }

    /// Clear screen
    fn clear_screen(&self) -> Result<()> {
        unsafe {
            let mut regs = BiosRegisters {
                eax: 0x0000, // Clear screen
                ebx: 0x0007, // Page 0, attribute 7
                ecx: 0x0000,
                edx: 0x184F, // Clear entire screen
                esi: 0,
                edi: 0,
                ebp: 0,
            };

            self.bios_int10(&mut regs);
        }
        Ok(())
    }

    /// Convert scancode to character (simplified)
    fn scancode_to_char(&self, scancode: u8) -> Option<char> {
        // Very simplified scancode to character mapping
        match scancode {
            0x02 => Some('1'), 0x03 => Some('2'), 0x04 => Some('3'), 0x05 => Some('4'),
            0x06 => Some('5'), 0x07 => Some('6'), 0x08 => Some('7'), 0x09 => Some('8'),
            0x0A => Some('9'), 0x0B => Some('0'), 0x10 => Some('Q'), 0x11 => Some('W'),
            0x12 => Some('E'), 0x13 => Some('R'), 0x14 => Some('T'), 0x15 => Some('Y'),
            0x16 => Some('U'), 0x17 => Some('I'), 0x18 => Some('O'), 0x19 => Some('P'),
            0x1E => Some('A'), 0x1F => Some('S'), 0x20 => Some('D'), 0x21 => Some('F'),
            0x22 => Some('G'), 0x23 => Some('H'), 0x24 => Some('J'), 0x25 => Some('K'),
            0x26 => Some('L'), 0x2C => Some('Z'), 0x2D => Some('X'), 0x2E => Some('C'),
            0x2F => Some('V'), 0x30 => Some('B'), 0x31 => Some('N'), 0x32 => Some('M'),
            0x39 => Some(' '), 0x1A => Some('['), 0x1B => Some(']'), 0x27 => Some(';'),
            0x28 => Some('\''), 0x29 => Some('`'), 0x2B => Some('\\'), 0x33 => Some(','),
            0x34 => Some('.'), 0x35 => Some('/'), 0x56 => Some('-'), 0x57 => Some('+'),
            _ => None,
        }
    }

    /// BIOS interrupt 0x10 handler
    unsafe fn bios_int10(&self, regs: &mut BiosRegisters) -> BiosRegisters {
        // In a real BIOS environment, this would trigger actual BIOS interrupt
        // For demonstration, we'll return the registers unchanged
        *regs
    }

    /// BIOS interrupt 0x16 handler
    unsafe fn bios_int16(&self, regs: &mut BiosRegisters) -> BiosRegisters {
        // In a real BIOS environment, this would trigger actual BIOS interrupt
        // For demonstration, we'll return empty result
        BiosRegisters { eax: 0, ebx: 0, ecx: 0, edx: 0, esi: 0, edi: 0, ebp: 0 }
    }

    /// Get the selected boot entry
    pub fn get_selected_entry(&self) -> &BootMenuEntry {
        &self.config.entries[self.selected_entry]
    }

    /// Get the current state
    pub fn get_state(&self) -> BootMenuState {
        self.state
    }

    /// Get the configuration
    pub fn get_config(&self) -> &BootMenuConfig {
        &self.config
    }
}

/// BIOS Registers for interrupt calls
#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct BiosRegisters {
    eax: u32,
    ebx: u32,
    ecx: u32,
    edx: u32,
    esi: u32,
    edi: u32,
    ebp: u32,
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
    }

    #[test]
    fn test_boot_menu_config() {
        let mut config = BootMenuConfig::new();
        assert_eq!(config.entries.len(), 0);
        assert!(config.validate().is_err());

        config.add_entry(BootMenuEntry::default_entry(
            "Default".to_string(),
            "kernel.bin".to_string(),
            "".to_string(),
        ));

        assert_eq!(config.entries.len(), 1);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_scancode_conversion() {
        let menu = BiosBootMenu::new(BootMenuConfig::new());
        assert_eq!(menu.scancode_to_char(0x10), Some('Q'));
        assert_eq!(menu.scancode_to_char(0x39), Some(' '));
        assert_eq!(menu.scancode_to_char(0xFF), None);
    }
}