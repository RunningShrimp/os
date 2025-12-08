//! hello_world - Simple test program to verify system calls work

#![no_std]
#![no_main]

use user::*;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    main();
    exit(0)
}

fn main() {
    println("Hello from NOS user space!");

    // Test getpid
    let pid = getpid();
    print_int(pid);
    puts(" is my process ID\n");

    // Test file operations
    let fd = open(b"/tmp/test.txt\0".as_ptr(), O_CREATE | O_RDWR);
    if fd >= 0 {
        puts("Successfully created /tmp/test.txt\n");
        write(fd as i32, b"Hello NOS!".as_ptr(), 10);
        close(fd as i32);
    } else {
        puts("Failed to create file\n");
    }

    // Test pipe
    let mut pipe_fds = [0i32; 2];
    if pipe(pipe_fds.as_mut_ptr()) >= 0 {
        puts("Pipe created successfully\n");
        write(pipe_fds[1], b"test data".as_ptr(), 9);

        let mut read_buf = [0u8; 20];
        let bytes_read = read(pipe_fds[0], read_buf.as_mut_ptr(), 20);
        if bytes_read > 0 {
            puts("Read from pipe: ");
            write(STDOUT, read_buf.as_ptr(), bytes_read as usize);
            puts("\n");
        }

        close(pipe_fds[0]);
        close(pipe_fds[1]);
    } else {
        puts("Failed to create pipe\n");
    }

    // Test fork
    let child_pid = fork();
    if child_pid == 0 {
        // Child process
        puts("Child process running\n");
        exit(0);
    } else if child_pid > 0 {
        // Parent process
        puts("Parent waiting for child\n");
        let mut status = 0i32;
        wait(&mut status);
        puts("Child process exited\n");
    } else {
        puts("Fork failed\n");
    }

    println("All tests completed successfully!");

    // Infinite loop to prevent exit
    loop {}
}