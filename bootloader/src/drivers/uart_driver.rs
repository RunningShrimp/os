//! UART Driver - Serial port I/O for early boot debugging
//!
//! Provides:
//! - 16550A UART initialization and configuration
//! - Serial data transmission and reception
//! - Baud rate configuration
//! - Line control and status monitoring

/// Standard UART COM ports
pub const COM1_BASE: u16 = 0x3F8;
pub const COM2_BASE: u16 = 0x2F8;
pub const COM3_BASE: u16 = 0x3E8;
pub const COM4_BASE: u16 = 0x2E8;

/// UART register offsets
pub const UART_DATA: u16 = 0;        // TX/RX data
pub const UART_IER: u16 = 1;         // Interrupt enable
pub const UART_FCR: u16 = 2;         // FIFO control
pub const UART_LCR: u16 = 3;         // Line control
pub const UART_MCR: u16 = 4;         // Modem control
pub const UART_LSR: u16 = 5;         // Line status
pub const UART_MSR: u16 = 6;         // Modem status
pub const UART_DLAB: u16 = 0x80;     // Divisor latch access bit

/// Standard baud rates
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaudRate {
    /// 1200 baud
    Baud1200 = 960,
    /// 2400 baud
    Baud2400 = 480,
    /// 9600 baud
    Baud9600 = 12,
    /// 19200 baud
    Baud19200 = 6,
    /// 38400 baud
    Baud38400 = 3,
    /// 57600 baud
    Baud57600 = 2,
    /// 115200 baud
    Baud115200 = 1,
}

/// Data bits configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataBits {
    /// 5 data bits
    Bits5 = 0,
    /// 6 data bits
    Bits6 = 1,
    /// 7 data bits
    Bits7 = 2,
    /// 8 data bits
    Bits8 = 3,
}

/// Stop bits configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopBits {
    /// 1 stop bit
    One = 0,
    /// 2 stop bits
    Two = 4,
}

/// Parity configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    /// No parity
    None = 0,
    /// Odd parity
    Odd = 8,
    /// Even parity
    Even = 24,
}

/// UART line control flags
#[derive(Debug, Clone, Copy)]
pub struct LineControl {
    /// Data bits
    pub data_bits: DataBits,
    /// Stop bits
    pub stop_bits: StopBits,
    /// Parity mode
    pub parity: Parity,
    /// Break signal
    pub break_enabled: bool,
}

impl LineControl {
    /// Create line control configuration
    pub fn new(
        data_bits: DataBits,
        stop_bits: StopBits,
        parity: Parity,
    ) -> Self {
        LineControl {
            data_bits,
            stop_bits,
            parity,
            break_enabled: false,
        }
    }

    /// Encode to register value
    pub fn encode(&self) -> u8 {
        let mut value = (self.data_bits as u8) | (self.stop_bits as u8) | (self.parity as u8);
        if self.break_enabled {
            value |= 0x40;
        }
        value
    }
}

/// UART line status flags
#[derive(Debug, Clone, Copy)]
pub struct LineStatus {
    /// Data ready
    pub data_ready: bool,
    /// Overrun error
    pub overrun_error: bool,
    /// Parity error
    pub parity_error: bool,
    /// Framing error
    pub framing_error: bool,
    /// Break interrupt
    pub break_interrupt: bool,
    /// THR empty (transmitter holding register)
    pub thr_empty: bool,
    /// THRE and TSR empty
    pub tsr_empty: bool,
    /// FIFO error
    pub fifo_error: bool,
}

impl LineStatus {
    /// Create from register value
    pub fn from_register(value: u8) -> Self {
        LineStatus {
            data_ready: (value & 0x01) != 0,
            overrun_error: (value & 0x02) != 0,
            parity_error: (value & 0x04) != 0,
            framing_error: (value & 0x08) != 0,
            break_interrupt: (value & 0x10) != 0,
            thr_empty: (value & 0x20) != 0,
            tsr_empty: (value & 0x40) != 0,
            fifo_error: (value & 0x80) != 0,
        }
    }
}

/// 16550A UART controller
pub struct UartDriver {
    /// Base I/O address
    base_address: u16,
    /// Current baud rate
    baud_rate: BaudRate,
    /// Line control settings
    line_control: LineControl,
    /// Total bytes transmitted
    tx_count: u32,
    /// Total bytes received
    rx_count: u32,
    /// UART initialized flag
    initialized: bool,
}

impl UartDriver {
    /// Create UART driver instance
    pub fn new(base_address: u16) -> Self {
        UartDriver {
            base_address,
            baud_rate: BaudRate::Baud115200,
            line_control: LineControl::new(DataBits::Bits8, StopBits::One, Parity::None),
            tx_count: 0,
            rx_count: 0,
            initialized: false,
        }
    }

    /// Initialize UART with default settings
    pub fn initialize(&mut self) -> bool {
        self.initialize_with_config(BaudRate::Baud115200, self.line_control)
    }

    /// Initialize UART with custom configuration
    pub fn initialize_with_config(&mut self, baud: BaudRate, line_ctrl: LineControl) -> bool {
        self.baud_rate = baud;
        self.line_control = line_ctrl;

        // Disable interrupts
        self.write_register(UART_IER, 0x00);

        // Enable DLAB to set baud rate
        self.write_register(UART_LCR, UART_DLAB as u8);
        
        // Set baud rate divisor
        let divisor = baud as u16;
        self.write_register(UART_DATA, (divisor & 0xFF) as u8);
        self.write_register(UART_IER, ((divisor >> 8) & 0xFF) as u8);

        // Disable DLAB and set line control
        self.write_register(UART_LCR, line_ctrl.encode());

        // Enable FIFO
        self.write_register(UART_FCR, 0xC7);

        // Set modem control
        self.write_register(UART_MCR, 0x0B);

        self.initialized = true;
        true
    }

    /// Transmit single byte
    pub fn send_byte(&mut self, byte: u8) -> bool {
        if !self.initialized {
            return false;
        }

        // Wait for transmitter to be ready
        let mut timeout = 10000;
        while timeout > 0 {
            let status = self.read_register(UART_LSR);
            if (status & 0x20) != 0 {
                break;
            }
            timeout -= 1;
        }

        if timeout == 0 {
            return false;
        }

        self.write_register(UART_DATA, byte);
        self.tx_count += 1;
        true
    }

    /// Transmit multiple bytes
    pub fn send(&mut self, data: &[u8]) -> u32 {
        let mut sent = 0;
        for &byte in data {
            if self.send_byte(byte) {
                sent += 1;
            } else {
                break;
            }
        }
        sent as u32
    }

    /// Receive single byte
    pub fn recv_byte(&mut self) -> Option<u8> {
        if !self.initialized {
            return None;
        }

        let status = self.read_register(UART_LSR);
        if (status & 0x01) != 0 {
            let byte = self.read_register(UART_DATA);
            self.rx_count += 1;
            Some(byte)
        } else {
            None
        }
    }

    /// Receive with timeout
    pub fn recv_byte_timeout(&mut self, timeout: u32) -> Option<u8> {
        let mut remaining = timeout;
        while remaining > 0 {
            if let Some(byte) = self.recv_byte() {
                return Some(byte);
            }
            remaining -= 1;
        }
        None
    }

    /// Get line status
    pub fn get_line_status(&self) -> LineStatus {
        let status = self.read_register(UART_LSR);
        LineStatus::from_register(status)
    }

    /// Check if ready to transmit
    pub fn is_tx_ready(&self) -> bool {
        (self.read_register(UART_LSR) & 0x20) != 0
    }

    /// Check if data available to receive
    pub fn is_rx_available(&self) -> bool {
        (self.read_register(UART_LSR) & 0x01) != 0
    }

    /// Set baud rate
    pub fn set_baud_rate(&mut self, baud: BaudRate) -> bool {
        if !self.initialized {
            return false;
        }

        self.baud_rate = baud;
        
        // Enable DLAB
        self.write_register(UART_LCR, self.read_register(UART_LCR) | UART_DLAB as u8);
        
        // Set divisor
        let divisor = baud as u16;
        self.write_register(UART_DATA, (divisor & 0xFF) as u8);
        self.write_register(UART_IER, ((divisor >> 8) & 0xFF) as u8);
        
        // Disable DLAB
        self.write_register(UART_LCR, self.read_register(UART_LCR) & !(UART_DLAB as u8));
        
        true
    }

    /// Get transmitted byte count
    pub fn tx_count(&self) -> u32 {
        self.tx_count
    }

    /// Get received byte count
    pub fn rx_count(&self) -> u32 {
        self.rx_count
    }

    /// Get total I/O operations
    pub fn io_count(&self) -> u32 {
        self.tx_count + self.rx_count
    }

    /// Write to UART register (simulated)
    fn write_register(&self, _offset: u16, _value: u8) {
        // Real implementation would use outb() x86 instruction
    }

    /// Read from UART register (simulated)
    fn read_register(&self, _offset: u16) -> u8 {
        // Real implementation would use inb() x86 instruction
        0
    }

    /// Get UART driver report
    pub fn uart_report(&self) -> UartReport {
        UartReport {
            base_address: self.base_address,
            baud_rate: self.baud_rate,
            initialized: self.initialized,
            tx_count: self.tx_count,
            rx_count: self.rx_count,
        }
    }
}

/// UART statistics report
#[derive(Debug, Clone, Copy)]
pub struct UartReport {
    /// Base I/O address
    pub base_address: u16,
    /// Current baud rate
    pub baud_rate: BaudRate,
    /// Initialization status
    pub initialized: bool,
    /// Transmitted bytes
    pub tx_count: u32,
    /// Received bytes
    pub rx_count: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_baud_rates() {
        assert_eq!(BaudRate::Baud9600 as u16, 12);
        assert_eq!(BaudRate::Baud115200 as u16, 1);
    }

    #[test]
    fn test_data_bits() {
        assert_eq!(DataBits::Bits5 as u8, 0);
        assert_eq!(DataBits::Bits8 as u8, 3);
    }

    #[test]
    fn test_stop_bits() {
        assert_eq!(StopBits::One as u8, 0);
        assert_eq!(StopBits::Two as u8, 4);
    }

    #[test]
    fn test_parity_values() {
        assert_eq!(Parity::None as u8, 0);
        assert_eq!(Parity::Odd as u8, 8);
        assert_eq!(Parity::Even as u8, 24);
    }

    #[test]
    fn test_line_control_creation() {
        let lc = LineControl::new(DataBits::Bits8, StopBits::One, Parity::None);
        assert_eq!(lc.data_bits, DataBits::Bits8);
        assert_eq!(lc.stop_bits, StopBits::One);
        assert_eq!(lc.parity, Parity::None);
        assert!(!lc.break_enabled);
    }

    #[test]
    fn test_line_control_encode() {
        let lc = LineControl::new(DataBits::Bits8, StopBits::One, Parity::None);
        assert_eq!(lc.encode(), 3);
    }

    #[test]
    fn test_line_control_with_break() {
        let mut lc = LineControl::new(DataBits::Bits8, StopBits::One, Parity::None);
        lc.break_enabled = true;
        assert_eq!(lc.encode() & 0x40, 0x40);
    }

    #[test]
    fn test_line_status_from_register() {
        let status = LineStatus::from_register(0xFF);
        assert!(status.data_ready);
        assert!(status.overrun_error);
        assert!(status.thr_empty);
        assert!(status.tsr_empty);
    }

    #[test]
    fn test_uart_driver_creation() {
        let uart = UartDriver::new(COM1_BASE);
        assert_eq!(uart.base_address, COM1_BASE);
        assert_eq!(uart.baud_rate, BaudRate::Baud115200);
        assert!(!uart.initialized);
    }

    #[test]
    fn test_uart_initialization() {
        let mut uart = UartDriver::new(COM1_BASE);
        assert!(uart.initialize());
        assert!(uart.initialized);
    }

    #[test]
    fn test_uart_custom_init() {
        let mut uart = UartDriver::new(COM1_BASE);
        let lc = LineControl::new(DataBits::Bits7, StopBits::One, Parity::Even);
        assert!(uart.initialize_with_config(BaudRate::Baud9600, lc));
        assert_eq!(uart.baud_rate, BaudRate::Baud9600);
    }

    #[test]
    fn test_uart_set_baud_rate() {
        let mut uart = UartDriver::new(COM1_BASE);
        uart.initialize();
        assert!(uart.set_baud_rate(BaudRate::Baud9600));
        assert_eq!(uart.baud_rate, BaudRate::Baud9600);
    }

    #[test]
    fn test_uart_send_byte() {
        let mut uart = UartDriver::new(COM1_BASE);
        uart.initialize();
        assert!(uart.send_byte(0x41));
        assert_eq!(uart.tx_count, 1);
    }

    #[test]
    fn test_uart_send_multiple() {
        let mut uart = UartDriver::new(COM1_BASE);
        uart.initialize();
        let data = b"Hello";
        let sent = uart.send(data);
        assert_eq!(sent, 5);
        assert_eq!(uart.tx_count, 5);
    }

    #[test]
    fn test_uart_counts() {
        let mut uart = UartDriver::new(COM1_BASE);
        uart.initialize();
        uart.send_byte(0x41);
        assert_eq!(uart.tx_count(), 1);
        assert_eq!(uart.rx_count(), 0);
        assert_eq!(uart.io_count(), 1);
    }

    #[test]
    fn test_uart_report() {
        let mut uart = UartDriver::new(COM1_BASE);
        uart.initialize();
        let report = uart.uart_report();
        assert_eq!(report.base_address, COM1_BASE);
        assert!(report.initialized);
        assert_eq!(report.tx_count, 0);
    }

    #[test]
    fn test_uart_com_ports() {
        assert_eq!(COM1_BASE, 0x3F8);
        assert_eq!(COM2_BASE, 0x2F8);
        assert_eq!(COM3_BASE, 0x3E8);
        assert_eq!(COM4_BASE, 0x2E8);
    }

    #[test]
    fn test_uart_line_status_fields() {
        let status = LineStatus::from_register(0b10100001);
        assert!(status.data_ready);
        assert!(status.tsr_empty);
        assert!(!status.overrun_error);
    }

    #[test]
    fn test_uart_before_init() {
        let uart = UartDriver::new(COM1_BASE);
        assert!(!uart.is_tx_ready());
        assert!(!uart.is_rx_available());
    }

    #[test]
    fn test_uart_multiple_instances() {
        let uart1 = UartDriver::new(COM1_BASE);
        let uart2 = UartDriver::new(COM2_BASE);
        assert_eq!(uart1.base_address, COM1_BASE);
        assert_eq!(uart2.base_address, COM2_BASE);
    }

    #[test]
    fn test_line_control_even_parity() {
        let lc = LineControl::new(DataBits::Bits8, StopBits::One, Parity::Even);
        assert_eq!(lc.parity, Parity::Even);
        assert_eq!(lc.encode() & 0x18, 0x18);
    }

    #[test]
    fn test_uart_send_without_init() {
        let mut uart = UartDriver::new(COM1_BASE);
        assert!(!uart.send_byte(0x41));
    }

    #[test]
    fn test_uart_status_with_errors() {
        let status = LineStatus::from_register(0x0E);
        assert!(status.overrun_error);
        assert!(status.parity_error);
        assert!(status.framing_error);
        assert!(!status.data_ready);
    }
}
