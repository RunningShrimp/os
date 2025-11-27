//! Init - the first user process
//! This process is responsible for starting the shell

#![no_std]
#![no_main]

use user::*;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    main();
    exit(0)
}

fn main() {
    println("init: starting");
    
    // Open console for stdin/stdout/stderr
    if open(b"/dev/console\0".as_ptr(), O_RDWR) < 0 {
        // Create console device if it doesn't exist
        mknod(b"/dev/console\0".as_ptr(), 1, 1);
        open(b"/dev/console\0".as_ptr(), O_RDWR);
    }
    dup(0); // stdout
    dup(0); // stderr
    
    println("init: console ready");
    
    loop {
        let pid = fork();
        if pid < 0 {
            println("init: fork failed");
            exit(1);
        }
        
        if pid == 0 {
            // Child: exec shell
            let argv: [*const u8; 2] = [b"sh\0".as_ptr(), core::ptr::null()];
            exec(b"/bin/sh\0".as_ptr(), argv.as_ptr());
            println("init: exec sh failed");
            exit(1);
        }
        
        // Parent: wait for shell to exit, then restart it
        loop {
            let mut status: i32 = 0;
            let wpid = wait(&mut status);
            if wpid == pid {
                // Shell exited, restart it
                break;
            } else if wpid < 0 {
                // No children
                break;
            }
            // Reap zombie child
        }
    }
}
