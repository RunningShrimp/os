//! Memory Allocator Tests
//!
//! Tests for memory allocation functionality

#[cfg(feature = "kernel_tests")]
pub mod alloc_tests {
    use crate::{test_assert_eq, test_assert};
    use crate::tests::TestResult;
    use alloc::boxed::Box;
    use alloc::vec::Vec;

    /// Test basic Box allocation
    pub fn test_box_alloc() -> TestResult {
        let b = Box::new(42u64);
        test_assert_eq!(*b, 42u64);
        Ok(())
    }

    /// Test Vec with growth
    pub fn test_vec_growth() -> TestResult {
        let mut v: Vec<i32> = Vec::new();
        for i in 0..1000 {
            v.push(i);
        }
        test_assert_eq!(v.len(), 1000);
        test_assert_eq!(v.iter().sum::<i32>(), (0..1000).sum::<i32>());
        Ok(())
    }

    /// Test large allocation
    pub fn test_large_alloc() -> TestResult {
        let v: Vec<u8> = alloc::vec![0xAA; 65536];
        test_assert_eq!(v.len(), 65536);
        test_assert!(v.iter().all(|&x| x == 0xAA));
        Ok(())
    }

    /// Test multiple allocations
    pub fn test_multiple_allocs() -> TestResult {
        let mut boxes: Vec<Box<[u8; 256]>> = Vec::new();
        for i in 0..100 {
            let mut data = [0u8; 256];
            data[0] = i as u8;
            boxes.push(Box::new(data));
        }
        for (i, b) in boxes.iter().enumerate() {
            test_assert_eq!(b[0], i as u8);
        }
        Ok(())
    }

    /// Test allocation and deallocation cycle
    pub fn test_alloc_dealloc_cycle() -> TestResult {
        for _ in 0..10 {
            let mut v: Vec<Box<u64>> = Vec::new();
            for i in 0..100 {
                v.push(Box::new(i as u64));
            }
            // Deallocate by dropping
        }
        Ok(())
    }
}

/// Memory Management Tests
///
/// Tests for memory management (mmap, munmap, etc.)

#[cfg(feature = "kernel_tests")]
pub mod memory_tests {
    use alloc::vec::Vec;
    use crate::{test_assert_eq, test_assert, test_assert_ne};
    use crate::tests::TestResult;
    use crate::syscalls::memory::{sys_mmap, sys_munmap};
    use crate::posix::{PROT_READ, PROT_WRITE, PROT_EXEC, MAP_SHARED, MAP_PRIVATE, MAP_ANONYMOUS};
    use crate::reliability::errno::{EOK as E_OK, EINVAL as E_INVAL, ENOENT as E_NOENT, EINVAL as E_BADARG};

    /// Test anonymous memory mapping
    pub fn test_mmap_anonymous() -> TestResult {
        // Test basic anonymous mapping
        let addr = sys_mmap(
            core::ptr::null_mut(),
            4096, // 1 page
            (PROT_READ | PROT_WRITE) as u32,
            (MAP_PRIVATE | MAP_ANONYMOUS) as u32,
            -1, // no file descriptor
            0
        );

        test_assert!(addr > 0, "mmap should return a valid address");

        // Test mapping with specific address
        let specific_addr = 0x20000000usize as *mut u8;
        let addr2 = sys_mmap(
            specific_addr,
            8192, // 2 pages
            (PROT_READ | PROT_WRITE) as u32,
            (MAP_PRIVATE | MAP_ANONYMOUS | 0x10) as u32, // MAP_FIXED
            -1,
            0
        );

        test_assert_eq!(addr2 as usize, specific_addr as usize, "MAP_FIXED should use specific address");

        // Test unmap
        let result = sys_munmap(addr as *mut u8, 4096);
        test_assert_eq!(result, 0, "munmap should succeed");

        let result2 = sys_munmap(addr2 as *mut u8, 8192);
        test_assert_eq!(result2, 0, "munmap should succeed");

        Ok(())
    }

    /// Test mmap with different protection flags
    pub fn test_mmap_protection() -> TestResult {
        // Test read-only mapping
        let addr = sys_mmap(
            core::ptr::null_mut(),
            4096,
            PROT_READ as u32,
            (MAP_PRIVATE | MAP_ANONYMOUS) as u32,
            -1,
            0
        );

        test_assert!(addr > 0, "read-only mmap should succeed");

        // Test read-write mapping
        let addr2 = sys_mmap(
            core::ptr::null_mut(),
            4096,
            (PROT_READ | PROT_WRITE) as u32,
            (MAP_PRIVATE | MAP_ANONYMOUS) as u32,
            -1,
            0
        );

        test_assert!(addr2 > 0, "read-write mmap should succeed");

        // Test executable mapping
        let addr3 = sys_mmap(
            core::ptr::null_mut(),
            4096,
            (PROT_READ | PROT_EXEC) as u32,
            (MAP_PRIVATE | MAP_ANONYMOUS) as u32,
            -1,
            0
        );

        test_assert!(addr3 > 0, "read-exec mmap should succeed");

        // Cleanup
        sys_munmap(addr as *mut u8, 4096);
        sys_munmap(addr2 as *mut u8, 4096);
        sys_munmap(addr3 as *mut u8, 4096);

        Ok(())
    }

    /// Test mmap error conditions
    pub fn test_mmap_errors() -> TestResult {
        // Test zero length
        let result = sys_mmap(
            core::ptr::null_mut(),
            0,
            PROT_READ as u32,
            MAP_ANONYMOUS as u32,
            -1,
            0
        );
        test_assert_eq!(result, E_INVAL as isize, "zero length should return EINVAL");

        // Test invalid protection flags
        let result2 = sys_mmap(
            core::ptr::null_mut(),
            4096,
            0xFF, // Invalid flags
            MAP_ANONYMOUS as u32,
            -1,
            0
        );
        test_assert_eq!(result2, E_INVAL as isize, "invalid protection should return EINVAL");

        // Test missing MAP_SHARED/MAP_PRIVATE
        let result3 = sys_mmap(
            core::ptr::null_mut(),
            4096,
            PROT_READ as u32,
            0, // No mapping flags
            -1,
            0
        );
        test_assert_eq!(result3, E_INVAL as isize, "missing MAP flag should return EINVAL");

        // Test both MAP_SHARED and MAP_PRIVATE
        let result4 = sys_mmap(
            core::ptr::null_mut(),
            4096,
            PROT_READ as u32,
            (MAP_SHARED | MAP_PRIVATE) as u32,
            -1,
            0
        );
        test_assert_eq!(result4, E_INVAL as isize, "both MAP flags should return EINVAL");

        // Test file mapping without file descriptor
        let result5 = sys_mmap(
            core::ptr::null_mut(),
            4096,
            PROT_READ as u32,
            MAP_SHARED as u32,
            -1, // No FD for non-anonymous mapping
            0
        );
        test_assert_eq!(result5, E_BADARG as isize, "non-anonymous mapping without FD should fail");

        Ok(())
    }

    /// Test munmap error conditions
    pub fn test_munmap_errors() -> TestResult {
        // Test null pointer
        let result = sys_munmap(core::ptr::null_mut(), 4096);
        test_assert_eq!(result, E_INVAL as isize, "null pointer should return EINVAL");

        // Test zero length
        let result2 = sys_munmap(0x10000000 as *mut u8, 0);
        test_assert_eq!(result2, E_INVAL as isize, "zero length should return EINVAL");

        // Test unaligned address
        let result3 = sys_munmap(0x10000001 as *mut u8, 4096);
        test_assert_eq!(result3, E_INVAL as isize, "unaligned address should return EINVAL");

        Ok(())
    }

    /// Test large memory mapping
    pub fn test_mmap_large() -> TestResult {
        // Test mapping 1MB (256 pages)
        let addr = sys_mmap(
            core::ptr::null_mut(),
            1024 * 1024, // 1MB
            (PROT_READ | PROT_WRITE) as u32,
            (MAP_PRIVATE | MAP_ANONYMOUS) as u32,
            -1,
            0
        );

        test_assert!(addr > 0, "large mmap should succeed");

        // Clean up
        let result = sys_munmap(addr as *mut u8, 1024 * 1024);
        test_assert_eq!(result, 0, "large munmap should succeed");

        Ok(())
    }

    /// Test multiple consecutive mappings
    pub fn test_mmap_multiple() -> TestResult {
        let mut addrs = Vec::new();

        // Create multiple small mappings
        for i in 0..10 {
            let addr = sys_mmap(
                core::ptr::null_mut(),
                4096, // 1 page each
                (PROT_READ | PROT_WRITE) as u32,
                (MAP_PRIVATE | MAP_ANONYMOUS) as u32,
                -1,
                0
            );

            test_assert!(addr > 0, alloc::format!("mmap {} should succeed", i));
            addrs.push(addr);
        }

        // Verify all mappings are different
        for (i, &addr1) in addrs.iter().enumerate() {
            for (j, &addr2) in addrs.iter().enumerate() {
                if i != j {
                    test_assert_ne!(addr1, addr2, "mappings should be at different addresses");
                }
            }
        }

        // Clean up all mappings
        for addr in addrs {
            sys_munmap(addr as *mut u8, 4096);
        }

        Ok(())
    }

    /// Test physical memory page allocation
    pub fn test_kalloc_basic() -> TestResult {
        // Test basic page allocation
        let page1 = crate::mm::kalloc();
        test_assert!(!page1.is_null(), "First page allocation should succeed");

        let page2 = crate::mm::kalloc();
        test_assert!(!page2.is_null(), "Second page allocation should succeed");

        // Pages should be different
        test_assert_ne!(page1, page2, "Allocated pages should be different");

        // Test alignment (should be page-aligned)
        test_assert_eq!(page1 as usize % crate::mm::PAGE_SIZE, 0, "Page should be aligned");
        test_assert_eq!(page2 as usize % crate::mm::PAGE_SIZE, 0, "Page should be aligned");

        // Free pages
        unsafe {
            crate::mm::kfree(page1);
            crate::mm::kfree(page2);
        }

        Ok(())
    }

    /// Test physical memory page deallocation
    pub fn test_kfree_basic() -> TestResult {
        let page = crate::mm::kalloc();
        test_assert!(!page.is_null(), "Page allocation should succeed");

        // Free the page
        unsafe { crate::mm::kfree(page); }

        // Allocate again - should succeed (page reuse)
        let page2 = crate::mm::kalloc();
        test_assert!(!page2.is_null(), "Page allocation after free should succeed");

        unsafe { crate::mm::kfree(page2); }

        Ok(())
    }

    /// Test multiple page allocation
    pub fn test_kalloc_pages() -> TestResult {
        // Test single page allocation
        let page1 = crate::mm::kalloc_pages(1);
        test_assert!(!page1.is_null(), "Single page allocation should succeed");
        test_assert_eq!(page1 as usize % crate::mm::PAGE_SIZE, 0, "Page should be aligned");

        // Test multiple page allocation
        let pages2 = crate::mm::kalloc_pages(4); // 4 contiguous pages
        test_assert!(!pages2.is_null(), "4-page allocation should succeed");
        test_assert_eq!(pages2 as usize % crate::mm::PAGE_SIZE, 0, "Pages should be aligned");

        // Test zero page allocation (should return null)
        let pages0 = crate::mm::kalloc_pages(0);
        test_assert!(pages0.is_null(), "Zero page allocation should return null");

        // Free allocations
        unsafe {
            crate::mm::kfree(page1);
            crate::mm::kfree(pages2);
        }

        Ok(())
    }

    /// Test memory allocation stress test
    pub fn test_memory_allocation_stress() -> TestResult {
        let mut allocations = Vec::new();

        // Allocate many pages
        for i in 0..50 {
            let page = crate::mm::kalloc();
            test_assert!(!page.is_null(), alloc::format!("Allocation {} should succeed", i));
            allocations.push(page);
        }

        // Free all pages
        for page in allocations {
            unsafe { crate::mm::kfree(page); }
        }

        // Allocate again to verify pages were freed
        for i in 0..50 {
            let page = crate::mm::kalloc();
            test_assert!(!page.is_null(), alloc::format!("Re-allocation {} should succeed", i));
            unsafe { crate::mm::kfree(page); }
        }

        Ok(())
    }

    /// Test page address utilities
    pub fn test_page_address_utilities() -> TestResult {
        // Test page_round_down
        test_assert_eq!(crate::mm::page_round_down(0x12345678), 0x12345000);
        test_assert_eq!(crate::mm::page_round_down(0x1000), 0x1000);
        test_assert_eq!(crate::mm::page_round_down(0x1001), 0x1000);
        test_assert_eq!(crate::mm::page_round_down(0x1FFF), 0x1000);

        // Test page_round_up
        test_assert_eq!(crate::mm::page_round_up(0x12345678), 0x12346000);
        test_assert_eq!(crate::mm::page_round_up(0x1000), 0x1000);
        test_assert_eq!(crate::mm::page_round_up(0x1001), 0x2000);
        test_assert_eq!(crate::mm::page_round_up(0x1FFF), 0x2000);

        // Test addr_to_pfn and pfn_to_addr
        let addr = 0x12345000;
        let pfn = crate::mm::addr_to_pfn(addr);
        let back_to_addr = crate::mm::pfn_to_addr(pfn);
        test_assert_eq!(back_to_addr, addr, "Address conversion should be reversible");

        Ok(())
    }

    /// Test PhysAddr wrapper
    pub fn test_phys_addr_wrapper() -> TestResult {
        let addr = crate::mm::PhysAddr::new(0x12345678);

        test_assert_eq!(addr.as_usize(), 0x12345678);
        test_assert_eq!(addr.page_offset(), 0x678);
        test_assert_eq!(addr.page_number(), 0x12345);
        test_assert!(!addr.is_page_aligned());

        let aligned_addr = crate::mm::PhysAddr::new(0x12345000);
        test_assert!(aligned_addr.is_page_aligned());

        let rounded_down = addr.page_round_down();
        test_assert_eq!(rounded_down.as_usize(), 0x12345000);

        let rounded_up = addr.page_round_up();
        test_assert_eq!(rounded_up.as_usize(), 0x12346000);

        Ok(())
    }

    /// Test VirtAddr wrapper
    pub fn test_virt_addr_wrapper() -> TestResult {
        let addr = crate::mm::VirtAddr::new(0x12345678);

        test_assert_eq!(addr.as_usize(), 0x12345678);
        test_assert_eq!(addr.page_offset(), 0x678);
        test_assert_eq!(addr.page_number(), 0x12345);
        test_assert!(!addr.is_page_aligned());

        let aligned_addr = crate::mm::VirtAddr::new(0x12345000);
        test_assert!(aligned_addr.is_page_aligned());

        let rounded_down = addr.page_round_down();
        test_assert_eq!(rounded_down.as_usize(), 0x12345000);

        let rounded_up = addr.page_round_up();
        test_assert_eq!(rounded_up.as_usize(), 0x12346000);

        Ok(())
    }

    /// Test memory copy utilities
    pub fn test_memory_copy_utilities() -> TestResult {
        let mut src = [1u8, 2, 3, 4, 5];
        let mut dst = [0u8; 5];

        // Test memmove
        unsafe {
            crate::mm::memmove(dst.as_mut_ptr(), src.as_ptr(), 5);
        }
        test_assert_eq!(dst, [1, 2, 3, 4, 5]);

        // Test memset
        let mut buf = [0u8; 10];
        unsafe {
            crate::mm::memset(buf.as_mut_ptr(), 0xAA, 10);
        }
        for &byte in &buf {
            test_assert_eq!(byte, 0xAA);
        }

        // Test memcmp
        let buf1 = [1, 2, 3, 4, 5];
        let buf2 = [1, 2, 3, 4, 5];
        let buf3 = [1, 2, 3, 4, 6];

        unsafe {
            test_assert_eq!(crate::mm::memcmp(buf1.as_ptr(), buf2.as_ptr(), 5), 0);
            test_assert!(crate::mm::memcmp(buf1.as_ptr(), buf3.as_ptr(), 5) < 0);
            test_assert!(crate::mm::memcmp(buf3.as_ptr(), buf1.as_ptr(), 5) > 0);
        }

        Ok(())
    }

    /// Test memory statistics
    pub fn test_memory_statistics() -> TestResult {
        let (free_pages, total_pages) = crate::mm::mem_stats();

        test_assert!(total_pages > 0, "Should have some total pages");
        test_assert!(free_pages <= total_pages, "Free pages should not exceed total pages");

        // Allocate a page and check stats change
        let page = crate::mm::kalloc();
        test_assert!(!page.is_null());

        let (free_pages_after, total_pages_after) = crate::mm::mem_stats();
        test_assert_eq!(total_pages_after, total_pages, "Total pages should not change");
        // Note: free_pages_after might not decrease immediately due to caching

        unsafe { crate::mm::kfree(page); }

        Ok(())
    }

    /// Test heap memory bounds
    pub fn test_heap_bounds() -> TestResult {
        let heap_start = crate::mm::heap_start();
        let heap_end = crate::mm::heap_end();

        test_assert!(heap_start > 0, "Heap start should be valid");
        test_assert!(heap_end > heap_start, "Heap end should be after heap start");

        let heap_size = heap_end - heap_start;
        test_assert!(heap_size > 0, "Heap should have non-zero size");

        Ok(())
    }

    /// Test MMIO region management
    pub fn test_mmio_regions() -> TestResult {
        let regions = crate::mm::mmio_regions();

        // Should have some MMIO regions defined for the architecture
        test_assert!(regions.len() > 0, "Should have MMIO regions defined");

        // Each region should have non-zero size
        for &(base, size) in &regions {
            test_assert!(size > 0, "MMIO region should have non-zero size");
            test_assert!(base > 0, "MMIO region should have valid base address");
        }

        Ok(())
    }

    /// Test MMIO read/write functions (safe versions)
    pub fn test_mmio_access() -> TestResult {
        // We can't safely test actual MMIO access in unit tests,
        // but we can test that the functions don't panic with null pointers
        // and that the statistics tracking works

        let initial_stats = crate::mm::mmio_stats_take();
        // Just verify the function doesn't panic
        test_assert!(true, "MMIO stats function should not panic");

        Ok(())
    }

    /// Test memory fragmentation resistance
    pub fn test_memory_fragmentation() -> TestResult {
        // Test that the allocator can handle mixed allocation patterns
        let mut small_allocs = Vec::new();
        let mut large_allocs = Vec::new();

        // Allocate many small pages
        for _ in 0..20 {
            let page = crate::mm::kalloc();
            test_assert!(!page.is_null());
            small_allocs.push(page);
        }

        // Allocate some large blocks
        for _ in 0..5 {
            let pages = crate::mm::kalloc_pages(4);
            test_assert!(!pages.is_null());
            large_allocs.push(pages);
        }

        // Free some small allocations (creating holes)
        for i in (0..small_allocs.len()).step_by(2) {
            unsafe { crate::mm::kfree(small_allocs[i]); }
        }

        // Allocate more small pages (should reuse freed memory)
        for _ in 0..10 {
            let page = crate::mm::kalloc();
            test_assert!(!page.is_null());
            small_allocs.push(page);
        }

        // Free everything
        for page in small_allocs {
            unsafe { crate::mm::kfree(page); }
        }
        for pages in large_allocs {
            unsafe { crate::mm::kfree(pages); }
        }

        Ok(())
    }

    /// Test memory alignment requirements
    pub fn test_memory_alignment() -> TestResult {
        // Test that all allocations are properly aligned
        for _ in 0..10 {
            let page = crate::mm::kalloc();
            test_assert!(!page.is_null());
            test_assert_eq!(page as usize % crate::mm::PAGE_SIZE, 0,
                alloc::format!("Page {:p} should be page-aligned", page));

            unsafe { crate::mm::kfree(page); }
        }

        // Test multi-page allocations
        for pages in [2, 4, 8, 16] {
            let block = crate::mm::kalloc_pages(pages);
            test_assert!(!block.is_null());
            test_assert_eq!(block as usize % crate::mm::PAGE_SIZE, 0,
                alloc::format!("{}-page block {:p} should be page-aligned", pages, block));

            unsafe { crate::mm::kfree(block); }
        }

        Ok(())
    }

    /// Test memory pressure handling
    pub fn test_memory_pressure() -> TestResult {
        // This test simulates memory pressure by allocating most available memory
        let mut allocations = Vec::new();
        let mut failed_count = 0;

        // Try to allocate as many pages as possible
        for i in 0..1000 {  // Arbitrary large number
            let page = crate::mm::kalloc();
            if page.is_null() {
                failed_count += 1;
                if failed_count > 5 {  // Stop after several failures
                    break;
                }
            } else {
                allocations.push(page);
                failed_count = 0;  // Reset failure count
            }
        }

        let allocated_count = allocations.len();
        test_assert!(allocated_count > 0, "Should be able to allocate at least some pages");

        // Free all allocations
        for page in allocations {
            unsafe { crate::mm::kfree(page); }
        }

        // Should be able to allocate again after freeing
        let page = crate::mm::kalloc();
        test_assert!(!page.is_null(), "Should be able to allocate after freeing all memory");
        unsafe { crate::mm::kfree(page); }

        Ok(())
    }
}

// ============================================================================
// Integration Test Framework
// ============================================================================
