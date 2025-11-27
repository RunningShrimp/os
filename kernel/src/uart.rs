//! UART driver for xv6-rust
//! Provides serial console I/O for all supported architectures

// ============================================================================
// RISC-V UART (16550 compatible)
// ============================================================================

#[cfg(target_arch = "riscv64")]
mod imp {
    /// UART base address for QEMU virt machine
    const UART_BASE: usize = 0x10000000;

    // Register offsets
    const RBR: usize = 0; // Receive Buffer Register (read)
    const THR: usize = 0; // Transmit Holding Register (write)
    const IER: usize = 1; // Interrupt Enable Register
    const FCR: usize = 2; // FIFO Control Register
    const LCR: usize = 3; // Line Control Register
    const LSR: usize = 5; // Line Status Register

    // Line Status Register bits
    const LSR_RX_READY: u8 = 1 << 0; // Data ready
    const LSR_TX_IDLE: u8 = 1 << 5;  // THR empty

    #[inline]
    fn reg(offset: usize) -> *mut u8 {
        (UART_BASE + offset) as *mut u8
    }

    #[inline]
    fn read_reg(offset: usize) -> u8 {
        unsafe { core::ptr::read_volatile(reg(offset)) }
    }

    #[inline]
    fn write_reg(offset: usize, val: u8) {
        unsafe { core::ptr::write_volatile(reg(offset), val) }
    }

    /// Initialize UART
    pub fn init() {
        // Disable interrupts
        write_reg(IER, 0x00);

        // Enable DLAB (set baud rate divisor)
        write_reg(LCR, 0x80);

        // Set baud rate to 38400 (divisor = 3)
        write_reg(0, 0x03); // Divisor latch low
        write_reg(1, 0x00); // Divisor latch high

        // 8 bits, no parity, one stop bit
        write_reg(LCR, 0x03);

        // Enable FIFO, clear them, with 14-byte threshold
        write_reg(FCR, 0xC7);

        // Enable receive interrupts
        write_reg(IER, 0x01);
    }

    /// Check if transmit buffer is ready
    #[inline]
    fn tx_ready() -> bool {
        read_reg(LSR) & LSR_TX_IDLE != 0
    }

    /// Check if receive buffer has data
    #[inline]
    fn rx_ready() -> bool {
        read_reg(LSR) & LSR_RX_READY != 0
    }

    /// Write a single byte
    pub fn write_byte(b: u8) {
        while !tx_ready() {
            core::hint::spin_loop();
        }
        write_reg(THR, b);
    }

    /// Read a single byte (non-blocking)
    pub fn read_byte() -> Option<u8> {
        if rx_ready() {
            Some(read_reg(RBR))
        } else {
            None
        }
    }

    /// Write a string
    pub fn write_str(s: &str) {
        for &b in s.as_bytes() {
            if b == b'\n' {
                write_byte(b'\r');
            }
            write_byte(b);
        }
    }

    /// Handle UART interrupt
    pub fn intr() {
        while let Some(c) = read_byte() {
            crate::drivers::console_intr(c);
        }
    }
}

// ============================================================================
// AArch64 UART (PL011)
// ============================================================================

#[cfg(target_arch = "aarch64")]
mod imp {
    /// PL011 UART base address for QEMU virt machine
    const UART_BASE: usize = 0x09000000;

    // Register offsets (32-bit aligned)
    const DR: usize = 0x00;    // Data Register
    const FR: usize = 0x18;    // Flag Register
    const IBRD: usize = 0x24;  // Integer Baud Rate
    const FBRD: usize = 0x28;  // Fractional Baud Rate
    const LCR_H: usize = 0x2C; // Line Control Register
    const CR: usize = 0x30;    // Control Register
    const IMSC: usize = 0x38;  // Interrupt Mask Set/Clear

    // Flag Register bits
    const FR_RXFE: u32 = 1 << 4; // Receive FIFO empty
    const FR_TXFF: u32 = 1 << 5; // Transmit FIFO full

    #[inline]
    fn reg(offset: usize) -> *mut u32 {
        (UART_BASE + offset) as *mut u32
    }

    #[inline]
    fn read_reg(offset: usize) -> u32 {
        unsafe { core::ptr::read_volatile(reg(offset)) }
    }

    #[inline]
    fn write_reg(offset: usize, val: u32) {
        unsafe { core::ptr::write_volatile(reg(offset), val) }
    }

    /// Initialize UART
    pub fn init() {
        // Disable UART
        write_reg(CR, 0);

        // Set baud rate (115200 at 24MHz clock)
        write_reg(IBRD, 13);
        write_reg(FBRD, 1);

        // 8 bits, no parity, 1 stop bit, enable FIFOs
        write_reg(LCR_H, (3 << 5) | (1 << 4));

        // Enable receive interrupt
        write_reg(IMSC, 1 << 4);

        // Enable UART, TX, RX
        write_reg(CR, (1 << 0) | (1 << 8) | (1 << 9));
    }

    /// Check if transmit FIFO has space
    #[inline]
    fn tx_ready() -> bool {
        read_reg(FR) & FR_TXFF == 0
    }

    /// Check if receive FIFO has data
    #[inline]
    fn rx_ready() -> bool {
        read_reg(FR) & FR_RXFE == 0
    }

    /// Write a single byte
    pub fn write_byte(b: u8) {
        while !tx_ready() {
            core::hint::spin_loop();
        }
        write_reg(DR, b as u32);
    }

    /// Read a single byte (non-blocking)
    pub fn read_byte() -> Option<u8> {
        if rx_ready() {
            Some(read_reg(DR) as u8)
        } else {
            None
        }
    }

    /// Write a string
    pub fn write_str(s: &str) {
        for &b in s.as_bytes() {
            if b == b'\n' {
                write_byte(b'\r');
            }
            write_byte(b);
        }
    }

    /// Handle UART interrupt
    pub fn intr() {
        while let Some(c) = read_byte() {
            crate::drivers::console_intr(c);
        }
    }
}

// ============================================================================
// x86_64 UART (16550 on COM1)
// ============================================================================

#[cfg(target_arch = "x86_64")]
mod imp {
    /// COM1 port base address
    const COM1: u16 = 0x3F8;

    // Register offsets
    const DATA: u16 = 0;
    const IER: u16 = 1;
    const FCR: u16 = 2;
    const LCR: u16 = 3;
    const MCR: u16 = 4;
    const LSR: u16 = 5;

    // Line Status Register bits
    const LSR_RX_READY: u8 = 1 << 0;
    const LSR_TX_IDLE: u8 = 1 << 5;

    #[inline]
    unsafe fn outb(port: u16, val: u8) {
        core::arch::asm!("out dx, al", in("dx") port, in("al") val, options(nostack, preserves_flags));
    }

    #[inline]
    unsafe fn inb(port: u16) -> u8 {
        let val: u8;
        core::arch::asm!("in al, dx", out("al") val, in("dx") port, options(nostack, preserves_flags));
        val
    }

    /// Initialize UART
    pub fn init() {
        unsafe {
            // Disable interrupts
            outb(COM1 + IER, 0x00);

            // Enable DLAB
            outb(COM1 + LCR, 0x80);

            // Set baud rate to 38400
            outb(COM1 + DATA, 0x03);
            outb(COM1 + IER, 0x00);

            // 8 bits, no parity, one stop bit
            outb(COM1 + LCR, 0x03);

            // Enable FIFO
            outb(COM1 + FCR, 0xC7);

            // IRQs enabled, RTS/DSR set
            outb(COM1 + MCR, 0x0B);

            // Enable receive interrupts
            outb(COM1 + IER, 0x01);
        }
    }

    #[inline]
    fn tx_ready() -> bool {
        unsafe { inb(COM1 + LSR) & LSR_TX_IDLE != 0 }
    }

    #[inline]
    fn rx_ready() -> bool {
        unsafe { inb(COM1 + LSR) & LSR_RX_READY != 0 }
    }

    /// Write a single byte
    pub fn write_byte(b: u8) {
        while !tx_ready() {
            core::hint::spin_loop();
        }
        unsafe { outb(COM1 + DATA, b) }
    }

    /// Read a single byte (non-blocking)
    pub fn read_byte() -> Option<u8> {
        if rx_ready() {
            Some(unsafe { inb(COM1 + DATA) })
        } else {
            None
        }
    }

    /// Write a string
    pub fn write_str(s: &str) {
        for &b in s.as_bytes() {
            if b == b'\n' {
                write_byte(b'\r');
            }
            write_byte(b);
        }
    }

    /// Handle UART interrupt
    pub fn intr() {
        while let Some(c) = read_byte() {
            crate::drivers::console_intr(c);
        }
    }
}

// ============================================================================
// Public interface
// ============================================================================

pub use imp::{init, write_byte, write_str, read_byte};

/// Handle UART interrupt
pub fn intr() {
    imp::intr();
}
