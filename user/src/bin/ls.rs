//! ls - list directory contents

#![no_std]
#![no_main]

use user::*;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    main();
    exit(0)
}

fn main() {
    // For now, just print a placeholder message
    // Full implementation would need readdir syscall
    println("ls: directory listing not implemented yet");
}
