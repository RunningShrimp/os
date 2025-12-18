// Enhanced console with VGA color support for x86_64

use core::fmt::{self, Write};

pub const VGA_BUFFER: usize = 0xB8000;
pub const VGA_WIDTH: usize = 80;
pub const VGA_HEIGHT: usize = 25;

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
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub fn new(foreground: Color, background: Color) -> Self {
        Self(((background as u8) << 4) | (foreground as u8))
    }
}

impl Clone for ColorCode {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

impl Copy for ColorCode {}

#[repr(C)]
pub struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

impl Clone for ScreenChar {
    fn clone(&self) -> Self {
        Self {
            ascii_character: self.ascii_character,
            color_code: self.color_code,
        }
    }
}

impl Copy for ScreenChar {}

pub struct VgaWriter {
    col: usize,
    row: usize,
    color_code: ColorCode,
}

impl VgaWriter {
    pub fn new() -> Self {
        Self {
            col: 0,
            row: 0,
            color_code: ColorCode::new(Color::White, Color::Black),
        }
    }

    pub fn clear_screen(&mut self) {
        for row in 0..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                self.write_char_at(b' ', row, col);
            }
        }
        self.col = 0;
        self.row = 0;
    }

    fn write_char_at(&mut self, ch: u8, row: usize, col: usize) {
        if row >= VGA_HEIGHT || col >= VGA_WIDTH {
            return;
        }
        let index = (row * VGA_WIDTH + col) * 2;
        unsafe {
            let ptr = (VGA_BUFFER + index) as *mut ScreenChar;
            (*ptr).ascii_character = ch;
            (*ptr).color_code = self.color_code;
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => {
                self.col = 0;
                self.row += 1;
                if self.row >= VGA_HEIGHT {
                    self.scroll();
                }
            }
            b'\r' => {
                self.col = 0;
            }
            _ => {
                self.write_char_at(byte, self.row, self.col);
                self.col += 1;
                if self.col >= VGA_WIDTH {
                    self.col = 0;
                    self.row += 1;
                    if self.row >= VGA_HEIGHT {
                        self.scroll();
                    }
                }
            }
        }
    }

    fn scroll(&mut self) {
        for row in 1..VGA_HEIGHT {
            for col in 0..VGA_WIDTH {
                let src_index = (row * VGA_WIDTH + col) * 2;
                let dst_index = ((row - 1) * VGA_WIDTH + col) * 2;
                unsafe {
                    let src = (VGA_BUFFER + src_index) as *const ScreenChar;
                    let dst = (VGA_BUFFER + dst_index) as *mut ScreenChar;
                    let ch = *src;
                    *dst = ch;
                }
            }
        }
        for col in 0..VGA_WIDTH {
            self.write_char_at(b' ', VGA_HEIGHT - 1, col);
        }
        self.row = VGA_HEIGHT - 1;
    }

    pub fn set_color(&mut self, fg: Color, bg: Color) {
        self.color_code = ColorCode::new(fg, bg);
    }
}

impl fmt::Write for VgaWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for byte in s.bytes() {
            self.write_byte(byte);
        }
        Ok(())
    }
}

pub static mut VGA_WRITER: Option<VgaWriter> = None;

pub fn init_vga() {
    unsafe {
        VGA_WRITER = Some(VgaWriter::new());
        if let Some(ref mut writer) = VGA_WRITER {
            writer.clear_screen();
        }
    }
}

pub fn write_vga(s: &str) {
    unsafe {
        if let Some(ref mut writer) = VGA_WRITER {
            let _ = writer.write_str(s);
        }
    }
}

pub fn write_byte_vga(byte: u8) {
    unsafe {
        if let Some(ref mut writer) = VGA_WRITER {
            writer.write_byte(byte);
        }
    }
}
