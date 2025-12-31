//! Braille Display Driver and Encoding
//!
//! This module provides comprehensive Braille display support including:
//! - Braille cell encoding (Unicode and 8-dot)
//! - Refreshable Braille display driver
//! - Braille text rendering and translation
//! - Braille device communication protocol
//! - Contracted and uncontracted Braille support

use crate::subsystems::sync::spinlock::SpinLock;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicBool, Ordering};

/// Braille display configuration
#[derive(Debug, Clone)]
pub struct BrailleConfig {
    /// Number of cells in the display
    pub cell_count: u16,
    /// Cell width in dots (6 or 8)
    pub dot_count: u8,
    /// Enable status cells
    pub status_cells: bool,
    /// Number of status cells at the beginning
    pub status_cell_count: u8,
    /// Automatic translation mode
    pub translation_mode: TranslationMode,
    /// Cursor style
    pub cursor_style: CursorStyle,
    /// Refresh rate in milliseconds
    pub refresh_rate_ms: u32,
}

impl Default for BrailleConfig {
    fn default() -> Self {
        Self {
            cell_count: 40,
            dot_count: 8,
            status_cells: true,
            status_cell_count: 2,
            translation_mode: TranslationMode::Uncontracted,
            cursor_style: CursorStyle::Dots7And8,
            refresh_rate_ms: 100,
        }
    }
}

/// Braille translation mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationMode {
    /// No translation (direct Braille input)
    None,
    /// Uncontracted Braille (1-to-1 character mapping)
    Uncontracted,
    /// Contracted Braille (Grade 2, with abbreviations)
    Contracted,
    /// Computer Braille (8-bit, includes symbols)
    ComputerBraille,
}

/// Cursor style for Braille display
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    /// No cursor
    None,
    /// Dot 7 only
    Dot7,
    /// Dot 8 only
    Dot8,
    /// Dots 7 and 8
    Dots7And8,
    /// Dots 7 and 8 blinking
    BlinkingDots7And8,
    /// All dots blinking
    BlinkingAllDots,
}

/// Braille cell (8-dot representation)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BrailleCell {
    /// Bitmask for dots 1-8 (dot 1 = bit 0, dot 8 = bit 7)
    pub dots: u8,
}

impl BrailleCell {
    /// Create a new empty Braille cell
    pub fn new() -> Self {
        Self { dots: 0 }
    }

    /// Create a cell with specified dots
    pub fn from_bits(dots: u8) -> Self {
        Self { dots: dots & 0xFF }
    }

    /// Check if a specific dot is raised
    pub fn has_dot(&self, dot: u8) -> bool {
        if dot == 0 || dot > 8 {
            return false;
        }
        (self.dots & (1 << (dot - 1))) != 0
    }

    /// Set a specific dot
    pub fn set_dot(&mut self, dot: u8, raised: bool) {
        if dot == 0 || dot > 8 {
            return;
        }
        if raised {
            self.dots |= 1 << (dot - 1);
        } else {
            self.dots &= !(1 << (dot - 1));
        }
    }

    /// Check if cell is empty
    pub fn is_empty(&self) -> bool {
        self.dots == 0
    }

    /// Clear all dots
    pub fn clear(&mut self) {
        self.dots = 0;
    }

    /// Get Unicode Braille character (6-dot or 8-dot)
    pub fn to_unicode(self) -> char {
        if self.dots == 0 {
            return '\u{2800}'; // Blank Braille pattern
        }

        // Unicode Braille patterns start at U+2800
        // The encoding is different from our bit layout
        // Unicode: dots: 1(0), 2(1), 3(2), 4(3), 5(4), 6(5), 7(6), 8(7)
        // Ours:    dots: 1(0), 2(1), 3(2), 4(3), 5(4), 6(5), 7(6), 8(7)
        // Actually same layout, but we need to handle 6 vs 8 dot

        let unicode_offset = if self.dots & 0xC0 == 0 {
            // 6-dot pattern
            self.dots
        } else {
            // 8-dot pattern
            self.dots
        };

        unsafe { char::from_u32_unchecked(0x2800 + unicode_offset as u32) }
    }

    /// Create from Unicode Braille character
    pub fn from_unicode(c: char) -> Option<Self> {
        let code = c as u32;
        if code < 0x2800 || code > 0x28FF {
            return None;
        }

        Some(Self {
            dots: (code - 0x2800) as u8,
        })
    }
}

/// Standard Braille patterns (Grade 1 uncontracted)
pub struct BraillePatterns;

impl BraillePatterns {
    // Lowercase letters a-z
    pub const A: u8 = 0b00000001;  // dots 1
    pub const B: u8 = 0b00000011;  // dots 1,2
    pub const C: u8 = 0b00001001;  // dots 1,4
    pub const D: u8 = 0b00001011;  // dots 1,4,5
    pub const E: u8 = 0b00000011;  // dots 1,5
    pub const F: u8 = 0b00001011;  // dots 1,4,5
    pub const G: u8 = 0b00001101;  // dots 1,2,4
    pub const H: u8 = 0b00001111;  // dots 1,2,4,5
    pub const I: u8 = 0b00001001;  // dots 2,4
    pub const J: u8 = 0b00001011;  // dots 2,4,5
    pub const K: u8 = 0b00000101;  // dots 1,3
    pub const L: u8 = 0b00000111;  // dots 1,2,3
    pub const M: u8 = 0b00001101;  // dots 1,3,4
    pub const N: u8 = 0b00001111;  // dots 1,3,4,5
    pub const O: u8 = 0b00000111;  // dots 1,3,5
    pub const P: u8 = 0b00001111;  // dots 1,2,3,4
    pub const Q: u8 = 0b00001101;  // dots 1,2,3,4,5
    pub const R: u8 = 0b00001111;  // dots 1,2,3,4,5
    pub const S: u8 = 0b00001101;  // dots 2,3,4
    pub const T: u8 = 0b00001111;  // dots 2,3,4,5
    pub const U: u8 = 0b00000101;  // dots 1,3,6
    pub const V: u8 = 0b00000111;  // dots 1,2,3,6
    pub const W: u8 = 0b00001011;  // dots 2,4,5,6
    pub const X: u8 = 0b00001101;  // dots 1,3,4,6
    pub const Y: u8 = 0b00001111;  // dots 1,3,4,5,6
    pub const Z: u8 = 0b00000111;  // dots 1,3,5,6

    // Uppercase indicator (capital sign)
    pub const CAPITAL: u8 = 0b00000100;  // dot 3

    // Numbers (number indicator + a-j)
    pub const NUMBER: u8 = 0b00001110;  // dots 3,4,5,6

    // Punctuation
    pub const SPACE: u8 = 0b00000000;
    pub const COMMA: u8 = 0b00000010;  // dot 2
    pub const PERIOD: u8 = 0b00001011;  // dots 2,5,6
    pub const QUESTION: u8 = 0b00001011;  // dots 2,3,6
    pub const EXCLAMATION: u8 = 0b00001011;  // dots 2,3,4,6
    pub const COLON: u8 = 0b00001010;  // dots 2,5
    pub const SEMICOLON: u8 = 0b00001010;  // dots 2,3
    pub const HYPHEN: u8 = 0b00000010;  // dot 3,6
    pub const APOSTROPHE: u8 = 0b00000100;  // dot 3
    pub const QUOTE: u8 = 0b00001000;  // dot 3
    pub const PAREN_OPEN: u8 = 0b00001011;  // dots 1,2,3,5,6
    pub const PAREN_CLOSE: u8 = 0b00001111;  // dots 2,3,4,5,6
    pub const BRACKET_OPEN: u8 = 0b00001011;  // dots 2,4,5,6
    pub const BRACKET_CLOSE: u8 = 0b00001111;  // dots 2,4,5,6
}

/// Braille display device interface
pub trait BrailleDevice: Send + Sync {
    /// Write cells to the display
    fn write_cells(&self, cells: &[BrailleCell]) -> Result<(), BrailleError>;

    /// Read keys from the display
    fn read_keys(&self) -> Result<Vec<BrailleKey>, BrailleError>;

    /// Get device information
    fn get_info(&self) -> Result<BrailleDeviceInfo, BrailleError>;

    /// Check if device is connected
    fn is_connected(&self) -> bool;

    /// Set cursor position
    fn set_cursor(&self, position: Option<u16>) -> Result<(), BrailleError>;

    /// Reset the device
    fn reset(&self) -> Result<(), BrailleError>;
}

/// Braille device information
#[derive(Debug, Clone)]
pub struct BrailleDeviceInfo {
    /// Device name
    pub name: String,
    /// Model identifier
    pub model: String,
    /// Number of cells
    pub cell_count: u16,
    /// Supports 8-dot braille
    pub eight_dot: bool,
    /// Has cursor routing keys
    pub cursor_routing: bool,
    /// Has status cells
    pub status_cells: bool,
    /// Number of status cells
    pub status_cell_count: u8,
    /// Firmware version
    pub firmware_version: String,
}

/// Braille key press
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrailleKey {
    /// Key type
    pub key_type: BrailleKeyType,
    /// Key code
    pub code: u8,
    /// Key state (pressed/released)
    pub pressed: bool,
}

/// Braille key types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrailleKeyType {
    /// Braille dot (1-8)
    Dot(u8),
    /// Space bar
    Space,
    /// Cursor routing key
    RoutingKey(u16),
    /// Function key
    FunctionKey(u8),
    /// Pan left
    PanLeft,
    /// Pan right
    PanRight,
    /// Escape
    Escape,
    /// Enter
    Enter,
    /// Backspace
    Backspace,
    /// Tab
    Tab,
}

/// Braille error types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrailleError {
    /// Device not connected
    DeviceDisconnected,
    /// Communication error
    CommunicationError,
    /// Invalid cell count
    InvalidCellCount,
    /// Invalid operation
    InvalidOperation,
    /// Buffer overflow
    BufferOverflow,
    /// Device busy
    DeviceBusy,
    /// Not supported
    NotSupported,
}

/// Refreshable Braille display driver
pub struct RefreshableBrailleDisplay {
    config: SpinLock<BrailleConfig>,
    device: SpinLock<Option<Box<dyn BrailleDevice>>>,
    buffer: SpinLock<Vec<BrailleCell>>,
    cursor_position: SpinLock<Option<u16>>,
    viewport_offset: SpinLock<u16>,
    connected: AtomicBool,
}

impl RefreshableBrailleDisplay {
    /// Create a new Braille display instance
    pub fn new(config: BrailleConfig) -> Self {
        let cell_count = config.cell_count as usize;
        Self {
            config: SpinLock::new(config),
            device: SpinLock::new(None),
            buffer: SpinLock::new(vec![BrailleCell::new(); cell_count]),
            cursor_position: SpinLock::new(None),
            viewport_offset: SpinLock::new(0),
            connected: AtomicBool::new(false),
        }
    }

    /// Connect a Braille device
    pub fn connect(&self, device: Box<dyn BrailleDevice>) -> Result<(), BrailleError> {
        // Get device info
        let info = device.get_info()?;

        // Update config based on device capabilities
        let mut config = self.config.lock();
        config.cell_count = info.cell_count;
        config.status_cells = info.status_cells;
        config.status_cell_count = info.status_cell_count;

        // Resize buffer
        let mut buffer = self.buffer.lock();
        buffer.resize(info.cell_count as usize, BrailleCell::new());

        // Store device
        let mut dev = self.device.lock();
        *dev = Some(device);

        self.connected.store(true, Ordering::Release);

        Ok(())
    }

    /// Disconnect the device
    pub fn disconnect(&self) {
        self.connected.store(false, Ordering::Release);
        let mut dev = self.device.lock();
        *dev = None;
    }

    /// Check if device is connected
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Acquire)
    }

    /// Write text to the display
    pub fn write_text(&self, text: &str) -> Result<(), BrailleError> {
        if !self.is_connected() {
            return Err(BrailleError::DeviceDisconnected);
        }

        let config = self.config.lock();
        let cells = self.translate_text(text, &config.translation_mode);
        drop(config);

        self.write_cells(&cells)
    }

    /// Write cells directly to the display
    pub fn write_cells(&self, cells: &[BrailleCell]) -> Result<(), BrailleError> {
        if !self.is_connected() {
            return Err(BrailleError::DeviceDisconnected);
        }

        let config = self.config.lock();
        let cell_count = config.cell_count as usize;
        let status_count = config.status_cell_count as usize;
        drop(config);

        // Update buffer
        let mut buffer = self.buffer.lock();
        let available_cells = cell_count - status_count;

        if cells.len() > available_cells {
            return Err(BrailleError::InvalidCellCount);
        }

        // Clear buffer
        for cell in buffer.iter_mut() {
            cell.clear();
        }

        // Copy cells to buffer (after status cells)
        for (i, &cell) in cells.iter().enumerate() {
            if i < available_cells {
                buffer[status_count + i] = cell;
            }
        }

        // Apply cursor
        let cursor_pos = self.cursor_position.lock();
        if let Some(pos) = *cursor_pos {
            if pos < available_cells as u16 {
                let config = self.config.lock();
                match config.cursor_style {
                    CursorStyle::Dot7 => {
                        buffer[status_count + pos as usize].set_dot(7, true);
                    }
                    CursorStyle::Dot8 => {
                        buffer[status_count + pos as usize].set_dot(8, true);
                    }
                    CursorStyle::Dots7And8 => {
                        buffer[status_count + pos as usize].set_dot(7, true);
                        buffer[status_count + pos as usize].set_dot(8, true);
                    }
                    _ => {}
                }
            }
        }
        drop(cursor_pos);
        drop(buffer);

        // Write to device
        let device = self.device.lock();
        if let Some(ref dev) = *device {
            dev.write_cells(&self.buffer.lock())?;
        }

        Ok(())
    }

    /// Set cursor position
    pub fn set_cursor(&self, position: Option<u16>) -> Result<(), BrailleError> {
        *self.cursor_position.lock() = position;

        let device = self.device.lock();
        if let Some(ref dev) = *device {
            dev.set_cursor(position)?;
        }

        Ok(())
    }

    /// Get cursor position
    pub fn get_cursor(&self) -> Option<u16> {
        *self.cursor_position.lock()
    }

    /// Translate text to Braille cells
    pub fn translate_text(&self, text: &str, mode: &TranslationMode) -> Vec<BrailleCell> {
        match mode {
            TranslationMode::None => {
                // No translation, treat as ASCII/Unicode Braille
                text.chars().filter_map(|c| BrailleCell::from_unicode(c)).collect()
            }
            TranslationMode::Uncontracted => {
                // Uncontracted (Grade 1) Braille
                Self::translate_uncontracted(text)
            }
            TranslationMode::Contracted => {
                // Contracted (Grade 2) Braille
                Self::translate_contracted(text)
            }
            TranslationMode::ComputerBraille => {
                // Computer Braille (8-bit)
                Self::translate_computer_braille(text)
            }
        }
    }

    /// Translate to uncontracted Braille (Grade 1)
    fn translate_uncontracted(text: &str) -> Vec<BrailleCell> {
        let mut cells = Vec::new();
        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            let cell = match c {
                ' ' => BrailleCell { dots: BraillePatterns::SPACE },
                'a' | 'A' => BrailleCell { dots: BraillePatterns::A },
                'b' | 'B' => BrailleCell { dots: BraillePatterns::B },
                'c' | 'C' => BrailleCell { dots: BraillePatterns::C },
                'd' | 'D' => BrailleCell { dots: BraillePatterns::D },
                'e' | 'E' => BrailleCell { dots: BraillePatterns::E },
                'f' | 'F' => BrailleCell { dots: BraillePatterns::F },
                'g' | 'G' => BrailleCell { dots: BraillePatterns::G },
                'h' | 'H' => BrailleCell { dots: BraillePatterns::H },
                'i' | 'I' => BrailleCell { dots: BraillePatterns::I },
                'j' | 'J' => BrailleCell { dots: BraillePatterns::J },
                'k' | 'K' => BrailleCell { dots: BraillePatterns::K },
                'l' | 'L' => BrailleCell { dots: BraillePatterns::L },
                'm' | 'M' => BrailleCell { dots: BraillePatterns::M },
                'n' | 'N' => BrailleCell { dots: BraillePatterns::N },
                'o' | 'O' => BrailleCell { dots: BraillePatterns::O },
                'p' | 'P' => BrailleCell { dots: BraillePatterns::P },
                'q' | 'Q' => BrailleCell { dots: BraillePatterns::Q },
                'r' | 'R' => BrailleCell { dots: BraillePatterns::R },
                's' | 'S' => BrailleCell { dots: BraillePatterns::S },
                't' | 'T' => BrailleCell { dots: BraillePatterns::T },
                'u' | 'U' => BrailleCell { dots: BraillePatterns::U },
                'v' | 'V' => BrailleCell { dots: BraillePatterns::V },
                'w' | 'W' => BrailleCell { dots: BraillePatterns::W },
                'x' | 'X' => BrailleCell { dots: BraillePatterns::X },
                'y' | 'Y' => BrailleCell { dots: BraillePatterns::Y },
                'z' | 'Z' => BrailleCell { dots: BraillePatterns::Z },
                ',' => BrailleCell { dots: BraillePatterns::COMMA },
                '.' => BrailleCell { dots: BraillePatterns::PERIOD },
                '?' => BrailleCell { dots: BraillePatterns::QUESTION },
                '!' => BrailleCell { dots: BraillePatterns::EXCLAMATION },
                ':' => BrailleCell { dots: BraillePatterns::COLON },
                ';' => BrailleCell { dots: BraillePatterns::SEMICOLON },
                '-' => BrailleCell { dots: BraillePatterns::HYPHEN },
                '\'' => BrailleCell { dots: BraillePatterns::APOSTROPHE },
                '"' => BrailleCell { dots: BraillePatterns::QUOTE },
                '(' => BrailleCell { dots: BraillePatterns::PAREN_OPEN },
                ')' => BrailleCell { dots: BraillePatterns::PAREN_CLOSE },
                '0'..='9' => BrailleCell { dots: 0 }, // Simplified
                _ => BrailleCell::new(),
            };

            // Add capital sign for uppercase
            if c.is_ascii_uppercase() {
                cells.push(BrailleCell { dots: BraillePatterns::CAPITAL });
            }

            cells.push(cell);
        }

        cells
    }

    /// Translate to contracted Braille (Grade 2)
    fn translate_contracted(text: &str) -> Vec<BrailleCell> {
        // Grade 2 contracted Braille with common abbreviations
        let text_lower = text.to_lowercase();
        let mut cells = Vec::new();
        let mut i = 0;
        let chars: Vec<char> = text_lower.chars().collect();

        while i < chars.len() {
            // Check for common contractions
            let (cell, advance) = if i + 2 < chars.len() {
                let three_chars = &text_lower[i..i+3];
                match three_chars {
                    "and" => (BrailleCell { dots: 0b00001111 }, 3),
                    "for" => (BrailleCell { dots: 0b00001111 }, 3),
                    "the" => (BrailleCell { dots: 0b00001111 }, 3),
                    "with" => (BrailleCell { dots: 0b00001111 }, 4),
                    _ => (Self::char_to_cell(chars[i]), 1),
                }
            } else if i + 1 < chars.len() {
                let two_chars = &text_lower[i..i+2];
                match two_chars {
                    "ch" => (BrailleCell { dots: 0b00001001 }, 2),
                    "sh" => (BrailleCell { dots: 0b00001001 }, 2),
                    "th" => (BrailleCell { dots: 0b00001011 }, 2),
                    "ing" => (BrailleCell { dots: 0b00001111 }, 3),
                    _ => (Self::char_to_cell(chars[i]), 1),
                }
            } else {
                (Self::char_to_cell(chars[i]), 1)
            };

            cells.push(cell);
            i += advance;
        }

        cells
    }

    /// Translate to computer Braille (8-bit encoding)
    fn translate_computer_braille(text: &str) -> Vec<BrailleCell> {
        // Computer Braille uses 8-dot patterns to represent all 256 bytes
        text.chars().map(|c| {
            let byte = c as u8;
            // Simple 1-to-1 mapping for demonstration
            BrailleCell { dots: byte }
        }).collect()
    }

    /// Convert character to Braille cell
    fn char_to_cell(c: char) -> BrailleCell {
        match c {
            ' ' => BrailleCell { dots: BraillePatterns::SPACE },
            'a' => BrailleCell { dots: BraillePatterns::A },
            'b' => BrailleCell { dots: BraillePatterns::B },
            'c' => BrailleCell { dots: BraillePatterns::C },
            'd' => BrailleCell { dots: BraillePatterns::D },
            'e' => BrailleCell { dots: BraillePatterns::E },
            'f' => BrailleCell { dots: BraillePatterns::F },
            'g' => BrailleCell { dots: BraillePatterns::G },
            'h' => BrailleCell { dots: BraillePatterns::H },
            'i' => BrailleCell { dots: BraillePatterns::I },
            'j' => BrailleCell { dots: BraillePatterns::J },
            'k' => BrailleCell { dots: BraillePatterns::K },
            'l' => BrailleCell { dots: BraillePatterns::L },
            'm' => BrailleCell { dots: BraillePatterns::M },
            'n' => BrailleCell { dots: BraillePatterns::N },
            'o' => BrailleCell { dots: BraillePatterns::O },
            'p' => BrailleCell { dots: BraillePatterns::P },
            'q' => BrailleCell { dots: BraillePatterns::Q },
            'r' => BrailleCell { dots: BraillePatterns::R },
            's' => BrailleCell { dots: BraillePatterns::S },
            't' => BrailleCell { dots: BraillePatterns::T },
            'u' => BrailleCell { dots: BraillePatterns::U },
            'v' => BrailleCell { dots: BraillePatterns::V },
            'w' => BrailleCell { dots: BraillePatterns::W },
            'x' => BrailleCell { dots: BraillePatterns::X },
            'y' => BrailleCell { dots: BraillePatterns::Y },
            'z' => BrailleCell { dots: BraillePatterns::Z },
            '0' => BrailleCell { dots: BraillePatterns::A },  // Number indicator + a
            '1' => BrailleCell { dots: BraillePatterns::B },
            '2' => BrailleCell { dots: BraillePatterns::C },
            '3' => BrailleCell { dots: BraillePatterns::D },
            '4' => BrailleCell { dots: BraillePatterns::E },
            '5' => BrailleCell { dots: BraillePatterns::F },
            '6' => BrailleCell { dots: BraillePatterns::G },
            '7' => BrailleCell { dots: BraillePatterns::H },
            '8' => BrailleCell { dots: BraillePatterns::I },
            '9' => BrailleCell { dots: BraillePatterns::J },
            _ => BrailleCell::new(),
        }
    }

    /// Pan viewport left
    pub fn pan_left(&self) -> Result<(), BrailleError> {
        let mut offset = self.viewport_offset.lock();
        if *offset > 0 {
            *offset -= 1;
        }
        Ok(())
    }

    /// Pan viewport right
    pub fn pan_right(&self) -> Result<(), BrailleError> {
        let mut offset = self.viewport_offset.lock();
        *offset += 1;
        Ok(())
    }

    /// Get viewport offset
    pub fn get_viewport_offset(&self) -> u16 {
        *self.viewport_offset.lock()
    }

    /// Set viewport offset
    pub fn set_viewport_offset(&self, offset: u16) {
        *self.viewport_offset.lock() = offset;
    }

    /// Clear the display
    pub fn clear(&self) -> Result<(), BrailleError> {
        let config = self.config.lock();
        let cell_count = config.cell_count as usize;
        drop(config);

        let cells = vec![BrailleCell::new(); cell_count];
        self.write_cells(&cells)
    }

    /// Read input keys
    pub fn read_keys(&self) -> Result<Vec<BrailleKey>, BrailleError> {
        if !self.is_connected() {
            return Err(BrailleError::DeviceDisconnected);
        }

        let device = self.device.lock();
        if let Some(ref dev) = *device {
            Ok(dev.read_keys()?)
        } else {
            Ok(Vec::new())
        }
    }
}

/// Simulated Braille device for testing
pub struct SimulatedBrailleDevice {
    info: BrailleDeviceInfo,
    cells: SpinLock<Vec<BrailleCell>>,
}

impl SimulatedBrailleDevice {
    pub fn new(cell_count: u16) -> Self {
        Self {
            info: BrailleDeviceInfo {
                name: "Simulated Braille Display".into(),
                model: "Sim-40".into(),
                cell_count,
                eight_dot: true,
                cursor_routing: true,
                status_cells: true,
                status_cell_count: 2,
                firmware_version: "1.0.0".into(),
            },
            cells: SpinLock::new(vec![BrailleCell::new(); cell_count as usize]),
        }
    }
}

impl BrailleDevice for SimulatedBrailleDevice {
    fn write_cells(&self, cells: &[BrailleCell]) -> Result<(), BrailleError> {
        let mut buffer = self.cells.lock();
        if cells.len() != buffer.len() {
            return Err(BrailleError::InvalidCellCount);
        }

        buffer.copy_from_slice(cells);
        log::debug!("Braille display updated: {} cells", cells.len());
        Ok(())
    }

    fn read_keys(&self) -> Result<Vec<BrailleKey>, BrailleError> {
        // No keys in simulation
        Ok(Vec::new())
    }

    fn get_info(&self) -> Result<BrailleDeviceInfo, BrailleError> {
        Ok(self.info.clone())
    }

    fn is_connected(&self) -> bool {
        true
    }

    fn set_cursor(&self, _position: Option<u16>) -> Result<(), BrailleError> {
        Ok(())
    }

    fn reset(&self) -> Result<(), BrailleError> {
        let mut buffer = self.cells.lock();
        for cell in buffer.iter_mut() {
            cell.clear();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_braille_cell() {
        let cell = BrailleCell::new();
        assert!(cell.is_empty());

        let mut cell = BrailleCell::from_bits(0b00000001);
        assert!(cell.has_dot(1));
        assert!(!cell.has_dot(2));

        cell.set_dot(2, true);
        assert!(cell.has_dot(2));
        assert_eq!(cell.dots, 0b00000011);
    }

    #[test]
    fn test_braille_unicode() {
        let cell = BrailleCell::from_bits(0b00000001);
        assert_eq!(cell.to_unicode(), '\u{2801}');

        if let Some(decoded) = BrailleCell::from_unicode('\u{2801}') {
            assert_eq!(decoded.dots, 0b00000001);
        }
    }

    #[test]
    fn test_refreshable_display() {
        let config = BrailleConfig::default();
        let display = RefreshableBrailleDisplay::new(config);

        let device = Box::new(SimulatedBrailleDevice::new(40));
        display.connect(device).unwrap();

        assert!(display.is_connected());

        display.write_text("hello").ok();
        display.clear().ok();
    }

    #[test]
    fn test_uncontracted_translation() {
        let text = "abc";
        let cells = RefreshableBrailleDisplay::translate_uncontracted(text);

        assert_eq!(cells.len(), 3);
        assert_eq!(cells[0].dots, BraillePatterns::A);
        assert_eq!(cells[1].dots, BraillePatterns::B);
        assert_eq!(cells[2].dots, BraillePatterns::C);
    }
}
