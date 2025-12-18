/// Simple VGA Text Output for Bootloader
///
/// Provides basic text mode output for boot messages and diagnostics.
/// Uses standard 80x25 VGA text buffer at address 0xB8000.

/// VGA text buffer base address
pub const VGA_BUFFER: *mut u16 = 0xB8000 as *mut u16;

/// VGA color palette (16 colors)
#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    LightMagenta = 13,
    Yellow = 14,
    White = 15,
}

/// VGA text mode writer
pub struct VGAWriter {
    col: usize,
    row: usize,
    fg_color: Color,
    bg_color: Color,
}

impl VGAWriter {
    /// Create a new VGA writer
    pub fn new() -> Self {
        Self {
            col: 0,
            row: 0,
            fg_color: Color::White,
            bg_color: Color::Black,
        }
    }

    /// Clear the screen
    pub fn clear(&mut self) {
        let color_attr = self.color_byte();
        unsafe {
            for i in 0..2000 {
                VGA_BUFFER.add(i).write_volatile(
                    (b' ' as u16) | ((color_attr as u16) << 8)
                );
            }
        }
        self.col = 0;
        self.row = 0;
    }

    /// Write a single character
    pub fn write_char(&mut self, ch: u8) {
        match ch {
            b'\n' => {
                self.row += 1;
                self.col = 0;
            }
            b'\r' => {
                self.col = 0;
            }
            _ => {
                if self.row >= 25 {
                    self.scroll_up();
                    self.row = 24;
                }

                let idx = self.row * 80 + self.col;
                let color_attr = self.color_byte();
                
                unsafe {
                    VGA_BUFFER.add(idx).write_volatile(
                        (ch as u16) | ((color_attr as u16) << 8)
                    );
                }

                self.col += 1;
                if self.col >= 80 {
                    self.col = 0;
                    self.row += 1;
                }
            }
        }
    }

    /// Write a string
    pub fn write_str(&mut self, s: &str) {
        for &byte in s.as_bytes() {
            self.write_char(byte);
        }
    }

    /// Set foreground color
    pub fn set_fg_color(&mut self, color: Color) {
        self.fg_color = color;
    }

    /// Set background color
    pub fn set_bg_color(&mut self, color: Color) {
        self.bg_color = color;
    }

    /// Scroll screen up by one line
    fn scroll_up(&self) {
        let color_attr = self.color_byte();
        unsafe {
            // Copy rows 1-24 to rows 0-23
            core::ptr::copy(
                VGA_BUFFER.add(80),
                VGA_BUFFER,
                80 * 24,
            );
            
            // Clear last row
            for i in 0..80 {
                VGA_BUFFER.add(24 * 80 + i).write_volatile(
                    (b' ' as u16) | ((color_attr as u16) << 8)
                );
            }
        }
    }

    /// Get color byte (foreground + background)
    fn color_byte(&self) -> u8 {
        ((self.bg_color as u8) << 4) | (self.fg_color as u8)
    }
}

/// Global VGA writer (for panic/early output)
pub static mut VGA: VGAWriter = VGAWriter {
    col: 0,
    row: 0,
    fg_color: Color::White,
    bg_color: Color::Black,
};

/// Print to VGA text buffer
#[macro_export]
macro_rules! vga_print {
    ($($arg:tt)*) => {
        unsafe {
            use core::fmt::Write;
            let _ = write!($crate::vga::VGA, $($arg)*);
        }
    };
}

/// Print line to VGA text buffer
#[macro_export]
macro_rules! vga_println {
    () => {
        $crate::vga_print!("\n");
    };
    ($($arg:tt)*) => {
        $crate::vga_print!($($arg)*);
        $crate::vga_print!("\n");
    };
}

// Implement core::fmt::Write for VGAWriter
impl core::fmt::Write for VGAWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_str(s);
        Ok(())
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
        if let Some(byte) = char::from_u32(c as u32).and_then(|c| {
            if c.is_ascii() {
                Some(c as u8)
            } else {
                Some(b'?')
            }
        }) {
            self.write_char(byte);
            Ok(())
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vga_writer_creation() {
        let writer = VGAWriter::new();
        assert_eq!(writer.col, 0);
        assert_eq!(writer.row, 0);
    }

    #[test]
    fn test_color_byte() {
        let writer = VGAWriter::new();
        let byte = writer.color_byte();
        assert_eq!(byte, 0x0F); // White on black
    }
}
