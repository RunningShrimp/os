//! Tests for advanced POSIX memory mapping features

use super::*;
use crate::syscalls::common::SyscallError;
use crate::posix;
use crate::subsystems::mm::vm::PAGE_SIZE;

#[test]
fn test_advanced_mmap_basic() {
    // Test basic mmap with advanced flags
    let length = 4096;
    let prot = posix::PROT_READ | posix::PROT_WRITE;
    let flags = posix::MAP_PRIVATE | posix::MAP_ANONYMOUS | MAP_LOCKED;
    let fd = -1;
    let offset = 0;

    let result = sys_mmap_advanced(&[0, length as u64, prot as u64, flags as u64, fd as u64, offset as u64]);
    assert!(result.is_ok());
    
    let addr = result.unwrap();
    assert_ne!(addr, 0);
    assert!(addr as usize % PAGE_SIZE == 0); // Should be page-aligned
}

#[test]
fn test_advanced_mmap_huge_pages() {
    // Test mmap with huge pages
    let length = 2 * 1024 * 1024; // 2MB
    let prot = posix::PROT_READ | posix::PROT_WRITE;
    let flags = posix::MAP_PRIVATE | posix::MAP_ANONYMOUS | MAP_HUGETLB | MAP_HUGE_2MB;
    let fd = -1;
    let offset = 0;

    let result = sys_mmap_advanced(&[0, length as u64, prot as u64, flags as u64, fd as u64, offset as u64]);
    assert!(result.is_ok());
    
    let addr = result.unwrap();
    assert_ne!(addr, 0);
}

#[test]
fn test_advanced_mmap_fixed() {
    // Test MAP_FIXED flag
    let length = 4096;
    let prot = posix::PROT_READ | posix::PROT_WRITE;
    let flags = posix::MAP_PRIVATE | posix::MAP_ANONYMOUS | posix::MAP_FIXED;
    let fd = -1;
    let offset = 0;
    let fixed_addr = 0x40000000; // Fixed address in user space

    let result = sys_mmap_advanced(&[fixed_addr as u64, length as u64, prot as u64, flags as u64, fd as u64, offset as u64]);
    assert!(result.is_ok());
    
    let addr = result.unwrap();
    assert_eq!(addr, fixed_addr as u64);
}

#[test]
fn test_advanced_mmap_invalid_flags() {
    // Test invalid flag combinations
    let length = 4096;
    let prot = posix::PROT_READ | posix::PROT_WRITE;
    let flags = posix::MAP_SHARED | posix::MAP_PRIVATE; // Invalid combination
    let fd = -1;
    let offset = 0;

    let result = sys_mmap_advanced(&[0, length as u64, prot as u64, flags as u64, fd as u64, offset as u64]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);
}

#[test]
fn test_mlock_munlock() {
    // First create a mapping
    let length = 4096;
    let prot = posix::PROT_READ | posix::PROT_WRITE;
    let flags = posix::MAP_PRIVATE | posix::MAP_ANONYMOUS;
    let fd = -1;
    let offset = 0;

    let result = sys_mmap_advanced(&[0, length as u64, prot as u64, flags as u64, fd as u64, offset as u64]);
    assert!(result.is_ok());
    
    let addr = result.unwrap() as usize;

    // Test mlock
    let lock_result = sys_mlock(&[addr as u64, length as u64]);
    assert!(lock_result.is_ok());

    // Test munlock
    let unlock_result = sys_munlock(&[addr as u64, length as u64]);
    assert!(unlock_result.is_ok());
}

#[test]
fn test_mlockall_munlockall() {
    // Test mlockall with current mappings
    let flags = MCL_CURRENT;
    let lockall_result = sys_mlockall(&[flags as u64]);
    assert!(lockall_result.is_ok());

    // Test munlockall
    let unlockall_result = sys_munlockall(&[]);
    assert!(unlockall_result.is_ok());
}

#[test]
fn test_mlockall_invalid_flags() {
    // Test invalid flags
    let flags = 0x12345678; // Invalid flags
    let result = sys_mlockall(&[flags as u64]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);
}

#[test]
fn test_madvise_normal() {
    // First create a mapping
    let length = 4096;
    let prot = posix::PROT_READ | posix::PROT_WRITE;
    let flags = posix::MAP_PRIVATE | posix::MAP_ANONYMOUS;
    let fd = -1;
    let offset = 0;

    let result = sys_mmap_advanced(&[0, length as u64, prot as u64, flags as u64, fd as u64, offset as u64]);
    assert!(result.is_ok());
    
    let addr = result.unwrap() as usize;

    // Test madvise with MADV_NORMAL
    let madvise_result = sys_madvise(&[addr as u64, length as u64, MADV_NORMAL as u64]);
    assert!(madvise_result.is_ok());
}

#[test]
fn test_madvise_willneed() {
    // First create a mapping
    let length = 4096;
    let prot = posix::PROT_READ | posix::PROT_WRITE;
    let flags = posix::MAP_PRIVATE | posix::MAP_ANONYMOUS;
    let fd = -1;
    let offset = 0;

    let result = sys_mmap_advanced(&[0, length as u64, prot as u64, flags as u64, fd as u64, offset as u64]);
    assert!(result.is_ok());
    
    let addr = result.unwrap() as usize;

    // Test madvise with MADV_WILLNEED
    let madvise_result = sys_madvise(&[addr as u64, length as u64, MADV_WILLNEED as u64]);
    assert!(madvise_result.is_ok());
}

#[test]
fn test_madvise_dontneed() {
    // First create a mapping
    let length = 4096;
    let prot = posix::PROT_READ | posix::PROT_WRITE;
    let flags = posix::MAP_PRIVATE | posix::MAP_ANONYMOUS;
    let fd = -1;
    let offset = 0;

    let result = sys_mmap_advanced(&[0, length as u64, prot as u64, flags as u64, fd as u64, offset as u64]);
    assert!(result.is_ok());
    
    let addr = result.unwrap() as usize;

    // Test madvise with MADV_DONTNEED
    let madvise_result = sys_madvise(&[addr as u64, length as u64, MADV_DONTNEED as u64]);
    assert!(madvise_result.is_ok());
}

#[test]
fn test_madvise_hugepage() {
    // First create a mapping
    let length = 4096;
    let prot = posix::PROT_READ | posix::PROT_WRITE;
    let flags = posix::MAP_PRIVATE | posix::MAP_ANONYMOUS;
    let fd = -1;
    let offset = 0;

    let result = sys_mmap_advanced(&[0, length as u64, prot as u64, flags as u64, fd as u64, offset as u64]);
    assert!(result.is_ok());
    
    let addr = result.unwrap() as usize;

    // Test madvise with MADV_HUGEPAGE
    let madvise_result = sys_madvise(&[addr as u64, length as u64, MADV_HUGEPAGE as u64]);
    assert!(madvise_result.is_ok());
}

#[test]
fn test_madvise_invalid_advice() {
    // Test invalid advice
    let addr = 0x40000000;
    let length = 4096;
    let advice = 0x12345678; // Invalid advice

    let result = sys_madvise(&[addr as u64, length as u64, advice as u64]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);
}

#[test]
fn test_mincore() {
    // First create a mapping
    let length = 4096;
    let prot = posix::PROT_READ | posix::PROT_WRITE;
    let flags = posix::MAP_PRIVATE | posix::MAP_ANONYMOUS;
    let fd = -1;
    let offset = 0;

    let result = sys_mmap_advanced(&[0, length as u64, prot as u64, flags as u64, fd as u64, offset as u64]);
    assert!(result.is_ok());
    
    let addr = result.unwrap() as usize;

    // Create vector for mincore result
    let page_count = (length + PAGE_SIZE - 1) / PAGE_SIZE;
    let vec_size = (page_count + 7) / 8;
    let mut vec = vec![0u8; vec_size];

    // Test mincore
    let mincore_result = sys_mincore(&[addr as u64, length as u64, vec.as_mut_ptr() as u64]);
    assert!(mincore_result.is_ok());

    // Check that at least one bit is set (page should be resident)
    let mut any_resident = false;
    for byte in &vec {
        if *byte != 0 {
            any_resident = true;
            break;
        }
    }
    assert!(any_resident);
}

#[test]
fn test_mincore_invalid_args() {
    // Test with null vector
    let result = sys_mincore(&[0x40000000, 4096, 0]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);

    // Test with zero length
    let vec = vec![0u8; 1];
    let result = sys_mincore(&[0x40000000, 0, vec.as_ptr() as u64]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);
}

#[test]
fn test_remap_file_pages() {
    // First create a file-backed mapping
    let length = 4096;
    let prot = posix::PROT_READ | posix::PROT_WRITE;
    let flags = posix::MAP_PRIVATE;
    let fd = 0; // Assume fd 0 is valid
    let offset = 0;

    let result = sys_mmap_advanced(&[0, length as u64, prot as u64, flags as u64, fd as u64, offset as u64]);
    assert!(result.is_ok());
    
    let addr = result.unwrap() as usize;

    // Test remap_file_pages
    let prot_remap = posix::PROT_READ | posix::PROT_WRITE;
    let pgoff = 4096; // Page offset
    let flags_remap = 0;

    let remap_result = sys_remap_file_pages(&[
        addr as u64, 
        length as u64, 
        prot_remap as u64, 
        pgoff as u64,
        flags_remap as u64
    ]);
    assert!(remap_result.is_ok());
}

#[test]
fn test_remap_file_pages_anonymous() {
    // Test with anonymous mapping (should fail)
    let length = 4096;
    let prot = posix::PROT_READ | posix::PROT_WRITE;
    let flags = posix::MAP_PRIVATE | posix::MAP_ANONYMOUS;
    let fd = -1;
    let offset = 0;

    let result = sys_mmap_advanced(&[0, length as u64, prot as u64, flags as u64, fd as u64, offset as u64]);
    assert!(result.is_ok());
    
    let addr = result.unwrap() as usize;

    // Test remap_file_pages (should fail for anonymous mapping)
    let prot_remap = posix::PROT_READ | posix::PROT_WRITE;
    let pgoff = 4096;
    let flags_remap = 0;

    let remap_result = sys_remap_file_pages(&[
        addr as u64, 
        length as u64, 
        prot_remap as u64, 
        pgoff as u64,
        flags_remap as u64
    ]);
    assert!(remap_result.is_err());
    assert_eq!(remap_result.unwrap_err(), SyscallError::InvalidArgument);
}

#[test]
fn test_get_huge_page_size() {
    // Test various huge page sizes
    assert_eq!(get_huge_page_size(MAP_HUGE_2MB).unwrap(), 2 * 1024 * 1024);
    assert_eq!(get_huge_page_size(MAP_HUGE_1MB).unwrap(), 1024 * 1024);
    assert_eq!(get_huge_page_size(MAP_HUGE_64KB).unwrap(), 64 * 1024);
    assert_eq!(get_huge_page_size(MAP_HUGE_1GB).unwrap(), 1024 * 1024 * 1024);
    
    // Test default (no specific size)
    assert_eq!(get_huge_page_size(0).unwrap(), 2 * 1024 * 1024);
}

#[test]
fn test_memory_region() {
    // Test MemoryRegion struct
    let region = MemoryRegion::new(0x40000000, 4096, posix::PROT_READ | posix::PROT_WRITE, 
                                  posix::MAP_PRIVATE, -1, 0);
    
    assert_eq!(region.start, 0x40000000);
    assert_eq!(region.end, 0x40001000);
    assert_eq!(region.size, 4096);
    assert_eq!(region.prot, posix::PROT_READ | posix::PROT_WRITE);
    assert_eq!(region.flags, posix::MAP_PRIVATE);
    assert_eq!(region.fd, -1);
    assert_eq!(region.offset, 0);
    assert!(!region.locked);
    assert_eq!(region.advice, MADV_NORMAL);
    assert_eq!(region.page_size, PAGE_SIZE);
    assert_eq!(region.ref_count, 1);

    // Test contains
    assert!(region.contains(0x40000000));
    assert!(region.contains(0x40000fff));
    assert!(!region.contains(0x40001000));
    assert!(!region.contains(0x3fffffff));

    // Test overlaps
    assert!(region.overlaps(0x40000000, 0x40001000));
    assert!(region.overlaps(0x40000500, 0x40001500));
    assert!(region.overlaps(0x3ffff000, 0x40001000));
    assert!(!region.overlaps(0x40001000, 0x40002000));
    assert!(!region.overlaps(0x3fffe000, 0x3ffff000));

    // Test aligned methods
    assert_eq!(region.aligned_start(), 0x40000000);
    assert_eq!(region.aligned_end(), 0x40001000);
    assert_eq!(region.page_count(), 1);
}

#[test]
fn test_memory_region_stats() {
    // Get initial stats
    let (regions, size, locked) = get_memory_region_stats();
    
    // Create a mapping
    let length = 4096;
    let prot = posix::PROT_READ | posix::PROT_WRITE;
    let flags = posix::MAP_PRIVATE | posix::MAP_ANONYMOUS;
    let fd = -1;
    let offset = 0;

    let result = sys_mmap_advanced(&[0, length as u64, prot as u64, flags as u64, fd as u64, offset as u64]);
    assert!(result.is_ok());

    // Check updated stats
    let (new_regions, new_size, new_locked) = get_memory_region_stats();
    assert_eq!(new_regions, regions + 1);
    assert_eq!(new_size, size + length);
    assert_eq!(new_locked, locked);
}

#[test]
fn test_advanced_mmap_zero_length() {
    // Test with zero length
    let result = sys_mmap_advanced(&[0, 0, posix::PROT_READ as u64, posix::MAP_PRIVATE as u64, -1i64 as u64, 0]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);
}

#[test]
fn test_advanced_mmap_invalid_prot() {
    // Test with invalid protection flags
    let result = sys_mmap_advanced(&[0, 4096, 0x12345678, posix::MAP_PRIVATE as u64, -1i64 as u64, 0]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);
}

#[test]
fn test_advanced_mmap_invalid_flags() {
    // Test with invalid mapping flags
    let result = sys_mmap_advanced(&[0, 4096, posix::PROT_READ as u64, 0x12345678, -1i64 as u64, 0]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);
}

#[test]
fn test_advanced_mmap_no_shared_or_private() {
    // Test without MAP_SHARED or MAP_PRIVATE
    let flags = posix::MAP_ANONYMOUS; // Missing MAP_SHARED or MAP_PRIVATE
    let result = sys_mmap_advanced(&[0, 4096, posix::PROT_READ as u64, flags as u64, -1i64 as u64, 0]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);
}

#[test]
fn test_advanced_mmap_fixed_zero_addr() {
    // Test MAP_FIXED with zero address (should fail)
    let flags = posix::MAP_PRIVATE | posix::MAP_ANONYMOUS | posix::MAP_FIXED;
    let result = sys_mmap_advanced(&[0, 4096, posix::PROT_READ as u64, flags as u64, -1i64 as u64, 0]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);
}

#[test]
fn test_mlock_invalid_args() {
    // Test with zero address
    let result = sys_mlock(&[0, 4096]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);

    // Test with zero length
    let result = sys_mlock(&[0x40000000, 0]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);
}

#[test]
fn test_munlock_invalid_args() {
    // Test with zero address
    let result = sys_munlock(&[0, 4096]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);

    // Test with zero length
    let result = sys_munlock(&[0x40000000, 0]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);
}

#[test]
fn test_madvise_invalid_args() {
    // Test with zero address
    let result = sys_madvise(&[0, 4096, MADV_NORMAL as u64]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);

    // Test with zero length
    let result = sys_madvise(&[0x40000000, 0, MADV_NORMAL as u64]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);
}

#[test]
fn test_remap_file_pages_invalid_args() {
    // Test with zero address
    let result = sys_remap_file_pages(&[0, 4096, posix::PROT_READ as u64, 0, 0]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);

    // Test with zero length
    let result = sys_remap_file_pages(&[0x40000000, 0, posix::PROT_READ as u64, 0, 0]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);
}

#[test]
fn test_remap_file_pages_unaligned_addr() {
    // Test with unaligned address
    let result = sys_remap_file_pages(&[0x40000001, 4096, posix::PROT_READ as u64, 0, 0]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), SyscallError::InvalidArgument);
}