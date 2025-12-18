//! Simple console output for bootloader debugging

pub fn write_byte(_byte: u8) {
    log::trace!("Writing byte to console");
    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            // Write to COM1 serial port for x86_64
            core::arch::asm!("out dx, al", in("al") _byte, in("dx") 0x3F8u16);
        }
    }
    
    #[cfg(target_arch = "aarch64")]
    {
        // AArch64: Would write to UART (stub for now)
    }
    
    #[cfg(target_arch = "riscv64")]
    {
        // RISC-V: Would write to SBI console (stub for now)
    }
}

pub fn write_str(s: &str) {
    for byte in s.bytes() {
        write_byte(byte);
    }
}

/// Simple write macro for console output
#[macro_export]
macro_rules! write {
    ($($arg:tt)*) => {
        {
            use core::fmt::Write;
            let mut writer = $crate::console::Writer;
            let _ = write!(writer, $($arg)*);
        }
    };
}

pub struct Writer;

impl core::fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        write_str(s);
        Ok(())
    }
}
