//! sh - a simple shell

#![no_std]
#![no_main]

use user::*;

const MAXARGS: usize = 10;
const MAXCMD: usize = 100;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    main();
    exit(0)
}

fn main() {
    let mut buf = [0u8; MAXCMD];
    
    loop {
        puts("$ ");
        
        // Read command
        let n = read(STDIN, buf.as_mut_ptr(), MAXCMD - 1);
        if n <= 0 {
            break;
        }
        
        buf[n as usize] = 0;
        
        // Remove trailing newline
        if n > 0 && buf[(n - 1) as usize] == b'\n' {
            buf[(n - 1) as usize] = 0;
        }
        
        // Empty command
        if buf[0] == 0 {
            continue;
        }
        
        // Built-in: cd
        if buf[0] == b'c' && buf[1] == b'd' && buf[2] == b' ' {
            if chdir(&buf[3] as *const u8) < 0 {
                puts("cd: cannot change directory\n");
            }
            continue;
        }
        
        // Built-in: exit
        if strcmp(buf.as_ptr(), b"exit\0".as_ptr()) == 0 {
            break;
        }
        
        // Fork and exec
        let pid = fork();
        if pid < 0 {
            puts("fork failed\n");
            continue;
        }
        
        if pid == 0 {
            // Child: exec command
            run_command(&buf);
            puts("exec failed\n");
            exit(1);
        }
        
        // Parent: wait for child
        let mut status: i32 = 0;
        wait(&mut status);
    }
}

fn run_command(cmd: &[u8]) {
    // Parse command into argv
    let mut argv_storage: [[u8; 32]; MAXARGS] = [[0; 32]; MAXARGS];
    let mut argv_ptrs: [*const u8; MAXARGS + 1] = [core::ptr::null(); MAXARGS + 1];
    let mut argc = 0;
    
    let mut i = 0;
    while i < cmd.len() && cmd[i] != 0 && argc < MAXARGS {
        // Skip whitespace
        while i < cmd.len() && (cmd[i] == b' ' || cmd[i] == b'\t') {
            i += 1;
        }
        
        if i >= cmd.len() || cmd[i] == 0 {
            break;
        }
        
        // Copy argument
        let mut j = 0;
        while i < cmd.len() && cmd[i] != 0 && cmd[i] != b' ' && cmd[i] != b'\t' && j < 31 {
            argv_storage[argc][j] = cmd[i];
            i += 1;
            j += 1;
        }
        argv_storage[argc][j] = 0;
        argv_ptrs[argc] = argv_storage[argc].as_ptr();
        argc += 1;
    }
    
    if argc == 0 {
        return;
    }
    
    // Try to execute
    // First try with /bin/ prefix
    let mut path_buf = [0u8; 64];
    path_buf[..5].copy_from_slice(b"/bin/");
    let mut k = 5;
    for c in &argv_storage[0] {
        if *c == 0 {
            break;
        }
        path_buf[k] = *c;
        k += 1;
    }
    path_buf[k] = 0;
    
    exec(path_buf.as_ptr(), argv_ptrs.as_ptr());
    
    // Try without prefix
    exec(argv_ptrs[0], argv_ptrs.as_ptr());
}
