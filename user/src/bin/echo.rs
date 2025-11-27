//! echo - write arguments to stdout

#![no_std]
#![no_main]

use user::*;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    // In a real implementation, we'd get argc/argv from the stack
    // For now, just print a placeholder
    println("echo: argument parsing not implemented yet");
    exit(0);
}
