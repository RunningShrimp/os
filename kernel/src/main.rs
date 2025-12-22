#![no_std]
#![no_main]
#![feature(c_variadic)]
#![allow(unsafe_op_in_unsafe_fn)]

// xv6-rust kernel main entry point

extern crate alloc;
// A minimal Unix-like kernel supporting RISC-V, AArch64, and x86_64

#[cfg(feature = "posix_layer")]
mod posix;
// errno is in reliability module

use core::sync::atomic::{AtomicBool, Ordering};

// Bootloader-based startup: Architecture-specific assembly is handled by bootloader
// Kernel entry points are defined below and called by the bootloader

// Kernel modules
mod platform;
mod subsystems;
mod services_unified; // Unified services module
mod security; // Security module

// Re-exports for compatibility with existing code in main.rs
use platform::{arch, boot, drivers, trap};
use subsystems::{fs, ipc, process, vfs};
use services_unified as services; // Use unified services module
use security::enhanced_permissions as permissions; // Use enhanced permissions

// Use nos-syscalls crate when feature is enabled
#[cfg(feature = "syscalls")]
use nos_syscalls as syscalls;

#[cfg(feature = "net_stack")]
use subsystems::net;

mod cpu; // cpu was missed in previous edit


mod compat;
mod security;
#[cfg(feature = "security_audit")]
mod security_audit;
// Use nos-error-handling crate when feature is enabled
#[cfg(feature = "error_handling")]
use nos_error_handling as error_handling;
#[cfg(feature = "debug_subsystems")]
mod debug;
mod reliability;
mod libc;
mod types;
mod collections;
#[cfg(feature = "graphics_subsystem")]
mod graphics;
#[cfg(feature = "web_engine")]
mod web;
mod benchmark;
#[cfg(feature = "observability")]
mod monitoring;

// Legacy infrastructure modules have been migrated to subsystems/
// Access them via crate::subsystems::mm, crate::subsystems::sync, crate::subsystems::time

#[cfg(feature = "kernel_tests")]
mod tests;


// Architecture name for logging
#[cfg(target_arch = "riscv64")]
const ARCH: &str = "riscv64";
#[cfg(target_arch = "aarch64")]
const ARCH: &str = "aarch64";
#[cfg(target_arch = "x86_64")]
const ARCH: &str = "x86_64";

// Boot synchronization
static STARTED: AtomicBool = AtomicBool::new(false);

/// Kernel main entry point called by bootloader
/// Called from bootloader with boot parameters
/// 
/// # Parameters
/// - rdi (x86_64): pointer to BootParameters structure
/// - x0 (aarch64): pointer to BootParameters structure  
/// - a0 (riscv64): pointer to BootParameters structure
#[unsafe(no_mangle)]
pub extern "C" fn rust_main(boot_params: *const boot::BootParameters) -> ! {
    rust_main_with_boot_info(boot_params)
}

/// Kernel main entry point with boot parameters
/// Called from bootloader with boot information
#[unsafe(no_mangle)]
pub extern "C" fn rust_main_with_boot_info(boot_params: *const boot::BootParameters) -> ! {
    // Initialize boot information if provided
    let boot_params_ref = if !boot_params.is_null() {
        unsafe {
            boot::init_from_boot_parameters(boot_params);
            Some(&*boot_params)
        }
    } else {
        None
    };

    // Call core initialization function
    core::init::init_kernel_core(boot_params_ref);

    // Mark boot complete
    STARTED.store(true, Ordering::SeqCst);

    // Run self-tests in debug builds
    #[cfg(debug_assertions)]
    #[cfg(feature = "kernel_tests")]
    run_tests();

    // Start the scheduler - this should never return
    // The scheduler will run the init process
    process::scheduler();
}

#[cfg(feature = "lazy_init")]
pub fn lazy_init_services() {
    monitoring::timeline::record("lazy_init_start");
    #[cfg(feature = "net_stack")]
    {
        net::init();
        crate::println!("[lazy] network stack initialized");
        monitoring::timeline::record("lazy_net_init");
    }
    #[cfg(feature = "graphics_subsystem")]
    {
        graphics::init();
        crate::println!("[lazy] graphics subsystem initialized");
        monitoring::timeline::record("lazy_graphics_init");
    }
    #[cfg(feature = "web_engine")]
    {
        web::init();
        crate::println!("[lazy] web engine subsystem initialized");
        monitoring::timeline::record("lazy_web_init");
    }
    monitoring::timeline::record("lazy_init_complete");
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
    crate::println!("[cpu{}] AP ready, entering scheduler", id);
    
    // Enter scheduler loop
    process::scheduler();
}

/// Run kernel self-tests
#[cfg(feature = "kernel_tests")]
fn run_tests() {
    // Run tests using the new test framework
    let (passed, failed, _skipped) = tests::run_all_tests();

    if failed > 0 {
        crate::println!();
        crate::println!("!!! {} TEST(S) FAILED !!!", failed);
        crate::println!();
    } else {
        crate::println!();
        crate::println!("All {} tests passed!", passed);
        crate::println!();
    }

    // Print test coverage report
    let coverage = tests::calculate_coverage();
    coverage.print_summary();
    
    // Also run the legacy tests that need fork/exec
    crate::println!("Running legacy integration tests...");
    test_pipe_fork_rw();
    test_exec_negative();
    test_exec_positive_minimal();
    test_paths_relative();
    crate::println!("Legacy tests completed.");
}

// ============================================================================
// Legacy integration tests (require fork/exec syscalls)
// ============================================================================

#[cfg(feature = "kernel_tests")]
fn test_pipe_fork_rw() {
    use crate::posix::O_NONBLOCK;
    print!("  pipe-fork: ");
    // Use sys_pipe to obtain process-level fds
    let mut pfds = [0i32; 2];
    let ret = crate::syscalls::dispatch(crate::syscalls::SysNum::Pipe as usize, &[pfds.as_mut_ptr() as usize, 0, 0, 0, 0, 0]);
    if ret != 0 { crate::println!("skipped"); return; }
    let pid = crate::syscalls::dispatch(crate::syscalls::SysNum::Fork as usize, &[0,0,0,0,0,0]);
    if pid == 0 { // child
        // Read from pipe
        let mut buf = [0u8; 8];
        let r = crate::syscalls::dispatch(crate::syscalls::SysNum::Read as usize, &[pfds[0] as usize, buf.as_mut_ptr() as usize, buf.len(), 0,0,0]);
        assert_eq!(r, 5);
        assert_eq!(&buf[..5], b"hello");
        let _ = crate::syscalls::dispatch(crate::syscalls::SysNum::Close as usize, &[pfds[0] as usize, 0,0,0,0,0]);
        let _ = crate::syscalls::dispatch(crate::syscalls::SysNum::Exit as usize, &[0,0,0,0,0,0]);
    } else {
        // parent
        let _ = crate::syscalls::dispatch(crate::syscalls::SysNum::Fcntl as usize, &[pfds[1] as usize, crate::posix::F_SETFL as usize, O_NONBLOCK as usize, 0,0,0]);
        let _ = crate::syscalls::dispatch(crate::syscalls::SysNum::Write as usize, &[pfds[1] as usize, b"hello".as_ptr() as usize, 5, 0,0,0]);
        let mut status = 0i32;
        let _ = crate::syscalls::dispatch(crate::syscalls::SysNum::Wait as usize, &[(&mut status as *mut i32) as usize, 0,0,0,0,0]);
        let _ = crate::syscalls::dispatch(crate::syscalls::SysNum::Close as usize, &[pfds[1] as usize, 0,0,0,0,0]);
        crate::println!("ok");
    }
}

#[cfg(feature = "kernel_tests")]
fn test_exec_negative() {
    print!("  exec-neg: ");
    let path = b"/bin/ls\0";
    let args: [*const u8; 1] = [core::ptr::null()];
    let ret = crate::syscalls::dispatch(crate::syscalls::SysNum::Exec as usize, &[path.as_ptr() as usize, args.as_ptr() as usize, 0,0,0,0]);
    assert_eq!(ret, crate::reliability::errno::ENOENT as isize);
    crate::println!("ok");
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
    let ret = crate::syscalls::dispatch(crate::syscalls::SysNum::Exec as usize, &[path.as_ptr() as usize, args.as_ptr() as usize, 0,0,0,0]);
    assert_eq!(ret, 0);
    crate::println!("ok");
}

#[cfg(feature = "kernel_tests")]
fn test_paths_relative() {
    use crate::posix::{O_CREAT, O_RDWR};
    print!("  paths-rel: ");
    // mkdir /tmp and chdir
    let _ = crate::syscalls::dispatch(crate::syscalls::SysNum::Mkdir as usize, ["/tmp\0".as_ptr() as usize, 0,0,0,0,0].as_ref());
    let _ = crate::syscalls::dispatch(crate::syscalls::SysNum::Chdir as usize, ["/tmp\0".as_ptr() as usize, 0,0,0,0,0].as_ref());
    // open relative file foo
    let fd = crate::syscalls::dispatch(
        crate::syscalls::SysNum::Open as usize,
        ["foo\0".as_ptr() as usize, (O_CREAT|O_RDWR) as usize, 0o644, 0,0,0].as_ref(),
    );
    assert!(fd >= 0);
    // write
    let _ = crate::syscalls::dispatch(crate::syscalls::SysNum::Write as usize, [fd as usize, b"ok".as_ptr() as usize, 2, 0,0,0].as_ref());
    // link to bar
    let _ = crate::syscalls::dispatch(crate::syscalls::SysNum::Link as usize, ["foo\0".as_ptr() as usize, "bar\0".as_ptr() as usize, 0,0,0,0].as_ref());
    // unlink foo
    let _ = crate::syscalls::dispatch(crate::syscalls::SysNum::Unlink as usize, ["foo\0".as_ptr() as usize, 0,0,0,0,0].as_ref());
    // open absolute /tmp/bar
    let fd2 = crate::syscalls::dispatch(crate::syscalls::SysNum::Open as usize, ["/tmp/bar\0".as_ptr() as usize, O_RDWR as usize, 0, 0,0,0].as_ref());
    assert!(fd2 >= 0);
    crate::println!("ok");
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    // Use enhanced panic handler
    crate::error::panic_handler::enhanced_panic_handler(info)
}
