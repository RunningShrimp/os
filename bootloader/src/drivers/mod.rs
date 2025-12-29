//! Device Drivers - UART, timer, enumeration, TPM, display (P5-P7)

pub mod console;
pub mod console_vga;
pub mod device_detect;
pub mod device_enumeration;
pub mod timer_driver;
pub mod tpm_driver;
pub mod uart_driver;
pub mod vga;
// Hardware drivers for bootloader
pub mod uart;
