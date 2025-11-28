#![no_std]
#![no_main]

use user::*;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    main();
    exit(0)
}

fn main() {
    println("sigmask: begin");
    let mut old = SigSet::default();
    let mut new = SigSet::default();
    sigemptyset(&mut new);
    sigaddset(&mut new, SIGINT);
    let r = sigprocmask(SIG_SETMASK, &new as *const SigSet, &mut old as *mut SigSet);
    if r < 0 { perror("sigprocmask"); return; }
    let mut pend = SigSet::default();
    let _ = sigpending(&mut pend as *mut SigSet);
    println("sigmask: ok");
}

