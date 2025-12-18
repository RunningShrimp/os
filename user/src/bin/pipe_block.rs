#![no_std]
#![no_main]

use user::{puts, println, pipe, read, write, close, poll, fork, wait, exit, PollFd};

#[unsafe(no_mangle)]
pub extern "C" fn main() -> ! {
    println("pipe_block test");
    let mut pfds = [0i32; 2];
    if pipe(pfds.as_mut_ptr()) < 0 { println("pipe failed"); loop {} }

    // Fork: parent writer, child reader
    let pid = fork();
    if pid == 0 {
        // child: read until receives sentinel
        let mut buf = [0u8; 64];
        let mut total = 0usize;
        loop {
            let n = read(pfds[0], buf.as_mut_ptr(), buf.len());
            if n <= 0 { break; }
            total += n as usize;
            // After some data, break to unblock parent
            if total >= 4096 { break; }
        }
        let _ = close(pfds[0]);
        exit(0);
    } else {
        // parent: write large data to fill and block, then resume after child reads
        let chunk = [0xAAu8; 256];
        let mut _wrote = 0usize;
        for _ in 0..(PIPE_TARGET()) {
            let n = write(pfds[1], chunk.as_ptr(), chunk.len());
            if n < 0 { break; }
            _wrote += n as usize;
        }
        // poll for readability on reader side
        let mut pfd = [PollFd{fd: pfds[0], events: user::POLLIN, revents: 0}];
        let _ = poll(pfd.as_mut_ptr(), 1, 0);
        let mut status = 0i32;
        let _ = wait(&mut status as *mut i32);
        // After child read, write should proceed
        let n2 = write(pfds[1], chunk.as_ptr(), chunk.len());
        if n2 != chunk.len() as isize { println("resume write failed"); }
        let _ = close(pfds[1]);
        puts("pipe_block done\n");
        loop {}
    }
}

#[inline]
fn PIPE_TARGET() -> usize { 4096 / 256 + 32 }
