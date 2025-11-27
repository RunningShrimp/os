//! cat - concatenate files

#![no_std]
#![no_main]

use user::*;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    main();
    exit(0)
}

fn main() {
    // For now, just read from stdin and write to stdout
    let mut buf = [0u8; 512];
    
    loop {
        let n = read(STDIN, buf.as_mut_ptr(), buf.len());
        if n <= 0 {
            break;
        }
        write(STDOUT, buf.as_ptr(), n as usize);
    }
}
