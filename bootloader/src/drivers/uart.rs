// UART driver for aarch64 and riscv64 platforms
// Supports PL011 (ARM) and 16550 (generic)

use core::fmt::{self};

const PL011_UARTDR: u64 = 0x09000000; // PL011 data register (qemu virt)
#[allow(dead_code)]
const UART_16550_OFFSET: u64 = 0x10000000; // Generic UART base

pub struct Uart;

impl Uart {
    #[cfg(target_arch = "aarch64")]
    pub fn write_byte(byte: u8) {
        unsafe {
            let ptr = PL011_UARTDR as *mut u8;
            ptr.write_volatile(byte);
        }
    }

    #[cfg(target_arch = "riscv64")]
    pub fn write_byte(byte: u8) {
        unsafe {
            let ptr = (UART_16550_OFFSET + 0) as *mut u8;
            ptr.write_volatile(byte);
        }
    }

    #[cfg(not(any(target_arch = "aarch64", target_arch = "riscv64")))]
    pub fn write_byte(byte: u8) {
        // x86_64 uses COM1 port instead
        log::trace!("UART write (COM1): byte={:#x}, offset={:#x}", byte, UART_16550_OFFSET);
        // No-op for x86_64 in this module
    }

    pub fn write_str(s: &str) {
        for byte in s.bytes() {
            Self::write_byte(byte);
        }
    }
}

impl fmt::Write for Uart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        Uart::write_str(s);
        Ok(())
    }
}

pub fn init_uart() {
    #[cfg(target_arch = "aarch64")]
    {
        // PL011 is memory-mapped, ready to use
    }
    #[cfg(target_arch = "riscv64")]
    {
        // 16550 init (minimal for QEMU)
    }
}
