//! xv6-rust kernel main entry point
//! A minimal Unix-like kernel supporting RISC-V, AArch64, and x86_64

#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]
mod posix;
mod errno;

use core::arch::global_asm;
use core::sync::atomic::{AtomicBool, Ordering};

// Architecture-specific startup code
#[cfg(all(feature = "baremetal", target_arch = "riscv64"))]
global_asm!(include_str!("../start-riscv64.S"));

#[cfg(all(feature = "baremetal", target_arch = "aarch64"))]
global_asm!(include_str!("../start-aarch64.S"));

#[cfg(all(feature = "baremetal", target_arch = "x86_64"))]
global_asm!(include_str!("../start-x86_64.S"));

// Kernel modules
mod uart;
mod arch;
mod mm;
mod console;
mod alloc;
mod sync;
mod process;
mod syscall;
mod drivers;
mod fs;
mod time;
mod syscalls;
mod trap;
mod vm;
mod file;
mod pipe;
mod cpu;
mod elf;
mod exec;
mod slab;
mod vfs;
mod signal;

// Architecture name for logging
#[cfg(target_arch = "riscv64")]
const ARCH: &str = "riscv64";
#[cfg(target_arch = "aarch64")]
const ARCH: &str = "aarch64";
#[cfg(target_arch = "x86_64")]
const ARCH: &str = "x86_64";

// Boot synchronization
static STARTED: AtomicBool = AtomicBool::new(false);

/// Kernel main entry point
/// Called from architecture-specific startup code
#[unsafe(no_mangle)]
pub extern "C" fn rust_main() -> ! {
    // Early hardware initialization (UART, etc.)
    arch::early_init();
    
    println!();
    println!("xv6-rust kernel booting on {}...", ARCH);
    println!();

    // Initialize boot CPU
    cpu::init_boot_cpu();
    println!("[boot] boot CPU initialized");

    // Initialize trap handling early
    trap::init();
    println!("[boot] trap handlers initialized");

    // Initialize physical memory allocator
    mm::init();
    println!("[boot] physical memory initialized");

    // Initialize kernel heap allocator
    // (alloc::init is called automatically via #[global_allocator])
    println!("[boot] kernel heap ready");

    // Initialize virtual memory / page tables
    vm::init();
    println!("[boot] virtual memory initialized");

    // Initialize timer
    time::init();
    println!("[boot] timer initialized");

    // Initialize drivers
    drivers::init();
    println!("[boot] drivers initialized");

    // Initialize file system
    fs::init();
    println!("[boot] filesystem initialized");

    // Initialize process subsystem
    process::init();
    println!("[boot] process subsystem initialized");

    // Start other CPUs
    cpu::start_aps();
    cpu::boot_complete();
    println!("[boot] SMP initialization complete ({} CPUs)", cpu::ncpus());

    // Mark boot complete
    STARTED.store(true, Ordering::SeqCst);

    println!();
    println!("xv6-rust kernel ready!");
    println!();

    // Run self-tests in debug builds
    #[cfg(debug_assertions)]
    #[cfg(feature = "kernel_tests")]
    run_tests();

    // Start the scheduler - this should never return
    // The scheduler will run the init process
    process::scheduler();
}

/// Entry point for Application Processors (APs)
/// Called from architecture-specific AP startup code
#[unsafe(no_mangle)]
pub extern "C" fn rust_main_ap() -> ! {
    // Initialize this CPU
    cpu::init_ap();
    
    // Initialize trap handling for this CPU
    trap::init();
    
    // Initialize timer for this CPU  
    time::init();
    
    let id = cpu::cpuid();
    println!("[cpu{}] AP ready, entering scheduler", id);
    
    // Enter scheduler loop
    process::scheduler();
}

/// Run kernel self-tests
#[cfg(feature = "kernel_tests")]
fn run_tests() {
    println!("Running kernel self-tests...");
    println!();

    // Test heap allocation
    test_alloc();

    // Test synchronization primitives
    test_sync();

    // Test file table
    test_file();

    // Test pipe
    test_pipe();

    println!();
    println!("All tests passed!");
    println!();
}

#[cfg(feature = "kernel_tests")]
fn test_alloc() {
    extern crate alloc as _alloc;
    use _alloc::vec::Vec;
    use _alloc::boxed::Box;

    print!("  alloc: ");

    // Test Vec
    let mut v: Vec<i32> = Vec::new();
    for i in 0..10 {
        v.push(i);
    }
    assert_eq!(v.iter().sum::<i32>(), 45);

    // Test Box
    let b = Box::new(42);
    assert_eq!(*b, 42);

    println!("ok");
}

#[cfg(feature = "kernel_tests")]
fn test_sync() {
    use crate::sync::{SpinLock, Mutex};

    print!("  sync: ");

    // Test SpinLock basic acquire/release
    let sl = SpinLock::new();
    sl.lock();
    assert!(sl.is_locked());
    sl.unlock();
    assert!(!sl.is_locked());

    // Test Mutex
    let mutex: Mutex<i32> = Mutex::new(0);
    {
        let mut guard = mutex.lock();
        *guard = 100;
    }
    assert_eq!(*mutex.lock(), 100);

    println!("ok");
}

#[cfg(feature = "kernel_tests")]
fn test_file() {
    print!("  file: ");

    if let Some(fd) = file::file_alloc() {
        file::file_close(fd);
    }

    println!("ok");
}

#[cfg(feature = "kernel_tests")]
fn test_pipe() {
    print!("  pipe: ");

    let result = pipe::pipe_alloc();
    if let Some((read_fd, write_fd)) = result {
        // Write to pipe
        let data = b"hello";
        let written = pipe::pipe_write(write_fd, data);
        assert_eq!(written, 5);

        // Read from pipe
        let mut buf = [0u8; 16];
        let read = pipe::pipe_read(read_fd, &mut buf);
        assert_eq!(read, 5);
        assert_eq!(&buf[..5], b"hello");

        file::file_close(read_fd);
        file::file_close(write_fd);
        println!("ok");
    } else {
        println!("skipped (no alloc)");
    }
}

/// Panic handler
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Disable interrupts
    arch::intr_off();

    println!();
    println!("!!! KERNEL PANIC !!!");
    
    if let Some(location) = info.location() {
        println!("at {}:{}:{}", location.file(), location.line(), location.column());
    }
    
    println!("{}", info.message());

    println!();
    println!("System halted.");

    // Halt the CPU
    loop {
        arch::wfi();
    }
}
