//! xv6-rust kernel main entry point
//! A minimal Unix-like kernel supporting RISC-V, AArch64, and x86_64

#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]
mod posix;
mod errno;

use core::sync::atomic::{AtomicBool, Ordering};
use core::arch::global_asm;

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
mod log;
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
mod buddy;
mod platform;
mod gic;
mod gicv3;
mod syscon;

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

    // Initialize and mount VFS root (ramfs)
    vfs::ramfs::init();
    match vfs::mount("ramfs", "/", None, 0) {
        Ok(()) => {
            println!("[boot] VFS root mounted (ramfs)");
            if let Ok(attr) = vfs::vfs().stat("/") {
                println!("[vfs] root ino={} mode={:#o} size={}B", attr.ino, attr.mode.permissions(), attr.size);
            }
        }
        Err(e) => {
            println!("[boot] VFS mount failed: {:?}", e);
        }
    }

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
    #[cfg(target_arch = "aarch64")]
    { drivers::init_ap(); }
    
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
    test_pipe_nonblock();
    test_pipe_fill_nonblock();
    test_pipe_close_read();
    test_pipe_fork_rw();
    test_exec_negative();
    test_exec_positive_minimal();

    // Test VFS create/write/read
    test_vfs_io();
    test_paths_relative();

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

#[cfg(feature = "kernel_tests")]
fn test_pipe_nonblock() {
    use crate::posix::O_NONBLOCK;
    print!("  pipe-nb: ");
    if let Some((rfd_idx, wfd_idx)) = crate::pipe::pipe_alloc() {
        {
            let mut table = crate::file::FILE_TABLE.lock();
            if let Some(f) = table.get_mut(rfd_idx) { f.status_flags |= O_NONBLOCK; }
        }
        let mut buf = [0u8; 4];
        let ret = crate::file::file_read(rfd_idx, &mut buf);
        assert_eq!(ret, crate::errno::errno_neg(crate::errno::EAGAIN));
        crate::file::file_close(rfd_idx);
        crate::file::file_close(wfd_idx);
        println!("ok");
    } else {
        println!("skipped");
    }
}

#[cfg(feature = "kernel_tests")]
fn test_pipe_fill_nonblock() {
    use crate::posix::O_NONBLOCK;
    print!("  pipe-fill-nb: ");
    if let Some((rfd_idx, wfd_idx)) = crate::pipe::pipe_alloc() {
        {
            let mut table = crate::file::FILE_TABLE.lock();
            if let Some(f) = table.get_mut(wfd_idx) { f.status_flags |= O_NONBLOCK; f.writable = true; f.readable = false; }
            if let Some(f) = table.get_mut(rfd_idx) { f.readable = true; f.writable = false; }
        }
        let buf = [0xAAu8; 64];
        loop {
            let n = crate::file::file_write(wfd_idx, &buf);
            if n == crate::errno::errno_neg(crate::errno::EAGAIN) { break; }
            if n < 0 { panic!("write failed"); }
        }
        crate::file::file_close(rfd_idx);
        crate::file::file_close(wfd_idx);
        println!("ok");
    } else { println!("skipped"); }
}

#[cfg(feature = "kernel_tests")]
fn test_pipe_close_read() {
    print!("  pipe-close-rd: ");
    if let Some((rfd_idx, wfd_idx)) = crate::pipe::pipe_alloc() {
        // Close reader, then write should return EPIPE
        crate::file::file_close(rfd_idx);
        let buf = [0xBBu8; 16];
        let n = crate::file::file_write(wfd_idx, &buf);
        assert_eq!(n, crate::errno::errno_neg(crate::errno::EPIPE));
        crate::file::file_close(wfd_idx);
        println!("ok");
    } else { println!("skipped"); }
}

#[cfg(feature = "kernel_tests")]
fn test_pipe_fork_rw() {
    use crate::posix::O_NONBLOCK;
    print!("  pipe-fork: ");
    // Use sys_pipe to obtain process-level fds
    let mut pfds = [0i32; 2];
    let ret = crate::syscall::dispatch(crate::syscall::SysNum::Pipe as usize, &[pfds.as_mut_ptr() as usize, 0, 0, 0, 0, 0]);
    if ret != crate::syscall::E_OK { println!("skipped"); return; }
    let pid = crate::syscall::dispatch(crate::syscall::SysNum::Fork as usize, &[0,0,0,0,0,0]);
    if pid == 0 { // child
        // Read from pipe
        let mut buf = [0u8; 8];
        let r = crate::syscall::dispatch(crate::syscall::SysNum::Read as usize, &[pfds[0] as usize, buf.as_mut_ptr() as usize, buf.len(), 0,0,0]);
        assert_eq!(r, 5);
        assert_eq!(&buf[..5], b"hello");
        let _ = crate::syscall::dispatch(crate::syscall::SysNum::Close as usize, &[pfds[0] as usize, 0,0,0,0,0]);
        let _ = crate::syscall::dispatch(crate::syscall::SysNum::Exit as usize, &[0,0,0,0,0,0]);
    } else {
        // parent
        let _ = crate::syscall::dispatch(crate::syscall::SysNum::Fcntl as usize, &[pfds[1] as usize, crate::posix::F_SETFL as usize, O_NONBLOCK as usize, 0,0,0]);
        let _ = crate::syscall::dispatch(crate::syscall::SysNum::Write as usize, &[pfds[1] as usize, b"hello".as_ptr() as usize, 5, 0,0,0]);
        let mut status = 0i32;
        let _ = crate::syscall::dispatch(crate::syscall::SysNum::Wait as usize, &[(&mut status as *mut i32) as usize, 0,0,0,0,0]);
        let _ = crate::syscall::dispatch(crate::syscall::SysNum::Close as usize, &[pfds[1] as usize, 0,0,0,0,0]);
        println!("ok");
    }
}

#[cfg(feature = "kernel_tests")]
fn test_exec_negative() {
    print!("  exec-neg: ");
    let path = b"/bin/ls\0";
    let args: [*const u8; 1] = [core::ptr::null()];
    let ret = crate::syscall::dispatch(crate::syscall::SysNum::Exec as usize, &[path.as_ptr() as usize, args.as_ptr() as usize, 0,0,0,0]);
    assert_eq!(ret, crate::syscall::E_NOENT);
    println!("ok");
}

#[cfg(feature = "kernel_tests")]
fn test_exec_positive_minimal() {
    use crate::vfs::{FileMode, vfs};
    print!("  exec-pos: ");
    // Build a minimal ELF64 for current arch with one PT_LOAD segment
    let mut elf = [0u8; 4096];
    // ELF header
    elf[0..4].copy_from_slice(&[0x7f, b'E', b'L', b'F']);
    elf[4] = 2; // ELFCLASS64
    elf[5] = 1; // ELFDATA2LSB
    // e_type
    elf[16+16] = 2; elf[16+17] = 0; // ET_EXEC
    // e_machine (set per arch)
    #[cfg(target_arch="riscv64")] { elf[16+18] = (243u16 & 0xFF) as u8; elf[16+19] = (243u16 >> 8) as u8; }
    #[cfg(target_arch="aarch64")] { elf[16+18] = (183u16 & 0xFF) as u8; elf[16+19] = (183u16 >> 8) as u8; }
    #[cfg(target_arch="x86_64")] { elf[16+18] = (62u16 & 0xFF) as u8; elf[16+19] = (62u16 >> 8) as u8; }
    // e_version
    elf[16+20] = 1; elf[16+21] = 0; elf[16+22] = 0; elf[16+23] = 0;
    // e_entry
    let entry: u64 = 0x400000;
    elf[24..32].copy_from_slice(&entry.to_le_bytes());
    // e_phoff
    let phoff: u64 = 64; // after header
    elf[32..40].copy_from_slice(&phoff.to_le_bytes());
    // e_ehsize
    elf[52..54].copy_from_slice(&(64u16).to_le_bytes());
    // e_phentsize
    elf[54..56].copy_from_slice(&(56u16).to_le_bytes());
    // e_phnum
    elf[56..58].copy_from_slice(&(1u16).to_le_bytes());
    // Program header
    let ph_offset = phoff as usize;
    // p_type PT_LOAD
    elf[ph_offset..ph_offset+4].copy_from_slice(&(1u32).to_le_bytes());
    // p_flags PF_R|PF_X
    elf[ph_offset+4..ph_offset+8].copy_from_slice(&(5u32).to_le_bytes());
    // p_offset
    elf[ph_offset+8..ph_offset+16].copy_from_slice(&(0u64).to_le_bytes());
    // p_vaddr
    elf[ph_offset+16..ph_offset+24].copy_from_slice(&entry.to_le_bytes());
    // p_paddr
    elf[ph_offset+24..ph_offset+32].copy_from_slice(&(0u64).to_le_bytes());
    // p_filesz = 0, p_memsz = PAGE
    elf[ph_offset+32..ph_offset+40].copy_from_slice(&(0u64).to_le_bytes());
    elf[ph_offset+40..ph_offset+48].copy_from_slice(&(4096u64).to_le_bytes());
    // p_align
    elf[ph_offset+48..ph_offset+56].copy_from_slice(&(4096u64).to_le_bytes());
    // Write to ramfs and exec
    let path = "/bin/hello";
    let mut f = vfs().create(path, FileMode::new(FileMode::S_IFREG | 0o755)).expect("create failed");
    let _ = f.write(elf.as_ptr() as usize, elf.len());
    let args: [*const u8; 1] = [core::ptr::null()];
    let ret = crate::syscall::dispatch(crate::syscall::SysNum::Exec as usize, &[path.as_ptr() as usize, args.as_ptr() as usize, 0,0,0,0]);
    assert_eq!(ret, 0);
    println!("ok");
}

#[cfg(feature = "kernel_tests")]
fn test_vfs_io() {
    use crate::vfs::{FileMode, vfs};
    print!("  vfs: ");
    // Create and write a file via VFS
    let path = "/hello";
    let mut file = vfs().create(path, FileMode::new(FileMode::S_IFREG | 0o644)).expect("create failed");
    let msg = b"world";
    let wrote = file.write(msg.as_ptr() as usize, msg.len()).expect("write failed");
    assert_eq!(wrote, msg.len());
    // Seek and read back
    let mut opened = vfs().open(path, 0).expect("open failed");
    let mut buf = [0u8; 8];
    let read = opened.read(buf.as_mut_ptr() as usize, msg.len()).expect("read failed");
    assert_eq!(read, msg.len());
    assert_eq!(&buf[..msg.len()], msg);
    println!("ok");
}

#[cfg(feature = "kernel_tests")]
fn test_paths_relative() {
    use crate::posix::{O_CREAT, O_RDWR};
    print!("  paths-rel: ");
    // mkdir /tmp and chdir
    let _ = crate::syscall::dispatch(crate::syscall::SysNum::Mkdir as usize, ["/tmp\0".as_ptr() as usize, 0,0,0,0,0].as_ref());
    let _ = crate::syscall::dispatch(crate::syscall::SysNum::Chdir as usize, ["/tmp\0".as_ptr() as usize, 0,0,0,0,0].as_ref());
    // open relative file foo
    let fd = crate::syscall::dispatch(
        crate::syscall::SysNum::Open as usize,
        ["foo\0".as_ptr() as usize, (O_CREAT|O_RDWR) as usize, 0o644, 0,0,0].as_ref(),
    );
    assert!(fd >= 0);
    // write
    let _ = crate::syscall::dispatch(crate::syscall::SysNum::Write as usize, [fd as usize, b"ok".as_ptr() as usize, 2, 0,0,0].as_ref());
    // link to bar
    let _ = crate::syscall::dispatch(crate::syscall::SysNum::Link as usize, ["foo\0".as_ptr() as usize, "bar\0".as_ptr() as usize, 0,0,0,0].as_ref());
    // unlink foo
    let _ = crate::syscall::dispatch(crate::syscall::SysNum::Unlink as usize, ["foo\0".as_ptr() as usize, 0,0,0,0,0].as_ref());
    // open absolute /tmp/bar
    let fd2 = crate::syscall::dispatch(crate::syscall::SysNum::Open as usize, ["/tmp/bar\0".as_ptr() as usize, O_RDWR as usize, 0, 0,0,0].as_ref());
    assert!(fd2 >= 0);
    println!("ok");
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
