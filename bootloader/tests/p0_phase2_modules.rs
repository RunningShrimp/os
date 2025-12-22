#![no_std]
#![cfg_attr(feature = "test", feature(custom_test_frameworks))]

extern crate alloc;

use nos_bootloader::{bios_realmode, error_recovery, kernel_handoff};

#[test]
fn test_bios_realmode_context() {
    let mut ctx = bios_realmode::RealModeContext::new();
    ctx.eax = 0xE820;
    assert_eq!(ctx.eax, 0xE820);
}

#[test]
fn test_bios_realmode_executor() {
    let executor = bios_realmode::RealModeExecutor::init();
    assert!(executor.is_ok());
}

#[test]
fn test_error_recovery_boot_error() {
    let err = error_recovery::BootError::InvalidMagic;
    assert_eq!(err.code(), 0x01);
    let desc = err.description();
    assert!(!desc.is_empty());
}

#[test]
fn test_error_recovery_creation() {
    let recovery = error_recovery::ErrorRecovery::new();
    assert!(!recovery.error_code.is_empty());
}

#[test]
fn test_kernel_handoff_creation() {
    let handoff = kernel_handoff::KernelHandoff::new(0x100000);
    assert_eq!(handoff.boot_info().kernel_entry, 0x100000);
}

#[test]
fn test_boot_information_validation() {
    let mut boot_info = kernel_handoff::BootInformation::new(0x100000);
    assert!(boot_info.validate());
}
