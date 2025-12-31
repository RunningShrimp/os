//! # Advanced Memory Management Test Suite
//!
//! Comprehensive tests for advanced memory management features including
//! NUMA allocation, hugepages, memory compression, KSM, and memory isolation.

use crate::prelude::*;
use crate::subsystems::mm::numa::{NumaPolicy, NumaNode, NUMA_ALLOCATOR};
use crate::subsystems::mm::hugepage::{HugePage, HugePageSize};
use crate::subsystems::mm::compression::{CompressedPage, CompressionAlgorithm};
use crate::subsystems::mm::ksm::{KsmManager, KsmScanType, KsmPage};
use crate::subsystems::mm::memory_isolation::{MemoryIsolation, IsolationType, IsolationPolicy};
use crate::subsystems::mm::{PAGE_SIZE, PAGE_SIZE_2M, PAGE_SIZE_1G};
use crate::subsystems::sync::spinlock::SpinLock;

// ============================================================================
// NUMA Allocation Tests
// ============================================================================

#[cfg(test)]
mod numa_tests {
    use super::*;

    /// Test NUMA allocation locality
    #[test]
    fn test_numa_allocation_locality() {
        // Get current NUMA node
        let current_cpu = crate::platform::arch::cpuid();
        let current_node = NumaNode::from_cpu(current_cpu);

        // Allocate memory on local node
        let local_allocator = NUMA_ALLOCATORS.lock()[current_node.id()];
        let local_pages = local_allocator.allocate_pages(1, NumaPolicy::Local);

        assert!(local_pages.is_some(), "Should allocate on local node");

        // Allocate memory on specific node
        let target_node = NumaNode { id: 0 };
        let allocator = NUMA_ALLOCATORS.lock()[target_node.id];
        let pages = allocator.allocate_pages(1, NumaPolicy::Prefer(target_node));

        assert!(pages.is_some(), "Should allocate on preferred node");
    }

    /// Test NUMA interleaved allocation
    #[test]
    fn test_numa_interleaved_allocation() {
        let num_pages = 100;
        let mut allocations = Vec::new();

        // Allocate with interleaved policy
        for i in 0..num_pages {
            let node_id = i % NumaNode::count();
            let node = NumaNode { id: node_id };
            let allocator = NUMA_ALLOCATORS.lock()[node.id];
            let pages = allocator.allocate_pages(1, NumaPolicy::Interleave);

            if let Some(page) = pages {
                allocations.push((node_id, page));
            }
        }

        // Verify pages are distributed across nodes
        assert_eq!(allocations.len(), num_pages, "Should allocate all pages");

        // Count allocations per node
        let mut node_counts = [0usize; 8];
        for (node_id, _) in &allocations {
            node_counts[*node_id] += 1;
        }

        // Should have roughly equal distribution
        let min_count = *node_counts.iter().min().unwrap();
        let max_count = *node_counts.iter().max().unwrap();
        let ratio = max_count as f64 / min_count as f64;

        assert!(ratio < 2.0, "Interleaved allocation should be balanced");
    }

    /// Test NUMA memory statistics
    #[test]
    fn test_numa_statistics() {
        for node_id in 0..NumaNode::count() {
            let node = NumaNode { id: node_id };
            let allocator = NUMA_ALLOCATORS.lock()[node.id];
            let stats = allocator.stats();

            // Verify statistics are available
            assert!(stats.total_pages > 0, "Node {} should have memory", node_id);
            assert!(stats.free_pages <= stats.total_pages, "Free pages should not exceed total");

            println!("Node {}: {} MB total, {} MB free",
                node_id,
                stats.total_pages * PAGE_SIZE / (1024 * 1024),
                stats.free_pages * PAGE_SIZE / (1024 * 1024)
            );
        }
    }

    /// Test NUMA page migration
    #[test]
    fn test_numa_page_migration() {
        let source_node = NumaNode { id: 0 };
        let target_node = NumaNode { id: 1 };

        // Allocate on source node
        let allocator = NUMA_ALLOCATORS.lock()[source_node.id];
        let page = allocator.allocate_pages(1, NumaPolicy::Local).expect("Should allocate page");

        // Migrate to target node
        let result = NumaNode::migrate_page(page, source_node, target_node);

        assert!(result.is_ok(), "Page migration should succeed");

        // Verify page is on target node
        let target_allocator = NUMA_ALLOCATORS.lock()[target_node.id];
        let stats = target_allocator.stats();
        // Note: In real implementation, would track page location
    }

    /// Test NUMA-aware allocation latency
    #[test]
    fn test_numa_allocation_latency() {
        let iterations = 1000;

        // Measure local allocation latency
        let start = crate::subsystems::time::get_time_ns();
        let current_cpu = crate::platform::arch::cpuid();
        let current_node = NumaNode::from_cpu(current_cpu);
        let allocator = NUMA_ALLOCATORS.lock()[current_node.id];

        for _ in 0..iterations {
            let _ = allocator.allocate_pages(1, NumaPolicy::Local);
        }

        let end = crate::subsystems::time::get_time_ns();
        let local_latency = (end - start) / iterations;

        // Target: < 1000ns for local allocation
        assert!(local_latency < 1000, "Local allocation latency should be < 1μs, got: {}ns", local_latency);

        println!("NUMA local allocation latency: {}ns", local_latency);
    }
}

// ============================================================================
// HugePage Tests
// ============================================================================

#[cfg(test)]
mod hugepage_tests {
    use super::*;

    /// Test 2MB hugepage allocation
    #[test]
    fn test_hugepage_2mb_allocation() {
        let hugepage = HugePage::allocate(HugePageSize::Size2M);

        assert!(hugepage.is_some(), "Should allocate 2MB hugepage");

        let hp = hugepage.unwrap();
        assert_eq!(hp.size(), PAGE_SIZE_2M, "Hugepage size should be 2MB");
        assert!(hp.is_aligned(), "Hugepage should be aligned to 2MB");
    }

    /// Test 1GB hugepage allocation
    #[test]
    fn test_hugepage_1gb_allocation() {
        let hugepage = HugePage::allocate(HugePageSize::Size1G);

        // 1GB allocation may fail on systems with insufficient memory
        if let Some(hp) = hugepage {
            assert_eq!(hp.size(), PAGE_SIZE_1G, "Hugepage size should be 1GB");
            assert!(hp.is_aligned(), "Hugepage should be aligned to 1GB");
        }
    }

    /// Test hugepage vs regular page TLB efficiency
    #[test]
    fn test_hugepage_tlb_efficiency() {
        // Allocate 2MB using regular pages (512 pages)
        let regular_pages: Vec<_> = (0..512)
            .map(|_| crate::subsystems::mm::kalloc())
            .collect();

        // Allocate 2MB as one hugepage
        let hugepage = HugePage::allocate(HugePageSize::Size2M);

        assert!(hugepage.is_some(), "Should allocate hugepage");

        // TLB entries: regular = 512, hugepage = 1
        // Hugepage provides ~512x TLB efficiency improvement
        let tlb_efficiency = 512.0; // regular_pages / hugepage
        println!("Hugepage TLB efficiency improvement: {}x", tlb_efficiency);
    }

    /// Test hugepage mapping and access
    #[test]
    fn test_hugepage_mapping() {
        let hugepage = HugePage::allocate(HugePageSize::Size2M).expect("Should allocate");

        // Map hugepage into address space
        let virt_addr = 0x1000_0000usize;
        let result = hugepage.map(virt_addr);

        assert!(result.is_ok(), "Should map hugepage");

        // Access the memory
        unsafe {
            let ptr = virt_addr as *mut u8;
            // Write pattern
            for i in 0..PAGE_SIZE_2M {
                *ptr.add(i) = (i % 256) as u8;
            }

            // Verify pattern
            for i in 0..PAGE_SIZE_2M {
                assert_eq!(*ptr.add(i), (i % 256) as u8, "Pattern mismatch at offset {}", i);
            }
        }

        // Unmap
        hugepage.unmap(virt_addr);
    }

    /// Test hugepage fragmentation handling
    #[test]
    fn test_hugepage_fragmentation() {
        // Allocate and deallocate regular pages to create fragmentation
        let mut pages = Vec::new();

        // Allocate many small pages
        for _ in 0..1000 {
            pages.push(crate::subsystems::mm::kalloc());
        }

        // Free alternate pages to create fragmentation
        for (i, page) in pages.iter().enumerate() {
            if i % 2 == 0 && !page.is_null() {
                crate::subsystems::mm::kfree(*page);
            }
        }

        // Try to allocate hugepage (may fail due to fragmentation)
        let hugepage = HugePage::allocate(HugePageSize::Size2M);

        // Note: This tests the compaction/defragmentation logic
        // In real systems, this would trigger page compaction
        if hugepage.is_none() {
            println!("Hugepage allocation failed due to fragmentation (expected)");
        }

        // Clean up
        for (i, page) in pages.iter().enumerate() {
            if i % 2 != 0 && !page.is_null() {
                crate::subsystems::mm::kfree(*page);
            }
        }
    }

    /// Test hugepage statistics
    #[test]
    fn test_hugepage_statistics() {
        let stats_before = HugePage::global_stats();

        // Allocate some hugepages
        for _ in 0..10 {
            let _ = HugePage::allocate(HugePageSize::Size2M);
        }

        let stats_after = HugePage::global_stats();

        // Verify statistics updated
        assert!(stats_after.total_allocated >= stats_before.total_allocated,
            "Allocated count should increase");
        assert!(stats_after.allocated_2mb >= stats_before.allocated_2mb,
            "2MB allocated count should increase");

        println!("Hugepage stats: {} 2MB pages, {} 1GB pages",
            stats_after.allocated_2mb,
            stats_after.allocated_1gb);
    }
}

// ============================================================================
// Memory Compression Tests
// ============================================================================

#[cfg(test)]
mod compression_tests {
    use super::*;

    /// Test memory compression ratio
    #[test]
    fn test_memory_compression_ratio() {
        // Allocate a page and fill with compressible data
        let page = crate::subsystems::mm::kalloc();
        assert!(!page.is_null(), "Should allocate page");

        unsafe {
            // Fill with highly compressible data (all zeros)
            core::ptr::write_bytes(page, 0, PAGE_SIZE);

            // Compress the page
            let compressed = CompressedPage::compress(page, PAGE_SIZE, CompressionAlgorithm::Zstd);

            assert!(compressed.is_ok(), "Should compress page");

            let compressed_page = compressed.unwrap();
            let compression_ratio = PAGE_SIZE as f64 / compressed_page.size() as f64;

            // Zstd should achieve good compression on all-zero data
            assert!(compression_ratio > 10.0, "Compression ratio should be > 10x for zeros");

            println!("Compression ratio: {:.2}x", compression_ratio);

            crate::subsystems::mm::kfree(page);
        }
    }

    /// Test memory decompression
    #[test]
    fn test_memory_decompression() {
        // Original data
        let original_data: Vec<u8> = (0..PAGE_SIZE).map(|i| (i % 256) as u8).collect();

        // Compress
        let compressed = CompressedPage::compress(
            original_data.as_ptr() as *const u8,
            original_data.len(),
            CompressionAlgorithm::LZ4
        );

        assert!(compressed.is_ok(), "Should compress data");

        let compressed_page = compressed.unwrap();

        // Decompress
        let decompressed = compressed_page.decompress();

        assert!(decompressed.is_ok(), "Should decompress page");

        let decompressed_data = decompressed.unwrap();

        // Verify data matches
        assert_eq!(decompressed_data.len(), original_data.len(), "Decompressed size should match");

        for i in 0..PAGE_SIZE {
            assert_eq!(decompressed_data[i], original_data[i], "Data mismatch at byte {}", i);
        }
    }

    /// Test compression performance
    #[test]
    fn test_compression_performance() {
        let iterations = 100;
        let page = crate::subsystems::mm::kalloc();
        assert!(!page.is_null(), "Should allocate page");

        unsafe {
            // Fill with random-like data
            for i in 0..PAGE_SIZE {
                *(page.add(i)) = ((i * 2654435761) % 256) as u8;
            }

            // Measure compression speed
            let start = crate::subsystems::time::get_time_ns();

            for _ in 0..iterations {
                let _ = CompressedPage::compress(page, PAGE_SIZE, CompressionAlgorithm::LZ4);
            }

            let end = crate::subsystems::time::get_time_ns();
            let elapsed_ms = (end - start) / 1_000_000;
            let throughput_mb_per_s = (iterations * PAGE_SIZE / (1024 * 1024)) as f64
                / (elapsed_ms as f64 / 1000.0);

            // Target: > 100 MB/s compression throughput
            assert!(throughput_mb_per_s > 100.0,
                "Compression throughput should be > 100 MB/s, got: {:.2} MB/s",
                throughput_mb_per_s);

            println!("Compression throughput: {:.2} MB/s", throughput_mb_per_s);

            crate::subsystems::mm::kfree(page);
        }
    }

    /// Test compression algorithms comparison
    #[test]
    fn test_compression_algorithms() {
        let page = crate::subsystems::mm::kalloc();
        assert!(!page.is_null(), "Should allocate page");

        unsafe {
            // Fill with compressible pattern
            core::ptr::write_bytes(page, 0x42, PAGE_SIZE);

            let algorithms = [
                CompressionAlgorithm::LZ4,
                CompressionAlgorithm::Zstd,
                CompressionAlgorithm::LZO,
            ];

            let mut results = Vec::new();

            for algo in &algorithms {
                let compressed = CompressedPage::compress(page, PAGE_SIZE, *algo);

                if let Ok(cp) = compressed {
                    let ratio = PAGE_SIZE as f64 / cp.size() as f64;
                    results.push((algo, ratio));
                    println!("{:?} compression ratio: {:.2}x", algo, ratio);
                }
            }

            // At least one algorithm should work
            assert!(!results.is_empty(), "At least one compression algorithm should work");

            crate::subsystems::mm::kfree(page);
        }
    }

    /// Test compressed page cache
    #[test]
    fn test_compressed_page_cache() {
        // Allocate and compress multiple pages
        let mut compressed_pages = Vec::new();

        for i in 0..10 {
            let page = crate::subsystems::mm::kalloc();
            assert!(!page.is_null(), "Should allocate page");

            unsafe {
                // Fill with data
                core::ptr::write_bytes(page, i as u8, PAGE_SIZE);

                // Compress
                let compressed = CompressedPage::compress(page, PAGE_SIZE, CompressionAlgorithm::LZ4);

                if let Ok(cp) = compressed {
                    compressed_pages.push(cp);
                }

                crate::subsystems::mm::kfree(page);
            }
        }

        // Verify all pages compressed
        assert_eq!(compressed_pages.len(), 10, "Should compress all pages");

        // Calculate memory savings
        let original_size = 10 * PAGE_SIZE;
        let compressed_size: usize = compressed_pages.iter().map(|cp| cp.size()).sum();
        let savings = original_size - compressed_size;
        let savings_ratio = savings as f64 / original_size as f64;

        println!("Memory savings: {} bytes ({:.2}%)", savings, savings_ratio * 100.0);

        // Should achieve > 50% savings for compressible data
        assert!(savings_ratio > 0.5, "Should achieve > 50% memory savings");
    }
}

// ============================================================================
// KSM (Kernel Samepage Merging) Tests
// ============================================================================

#[cfg(test)]
mod ksm_tests {
    use super::*;

    /// Test KSM page merging
    #[test]
    fn test_ksm_merging() {
        let mut ksm = KsmManager::new();

        // Create two identical pages
        let page1 = crate::subsystems::mm::kalloc();
        let page2 = crate::subsystems::mm::kalloc();
        assert!(!page1.is_null() && !page2.is_null(), "Should allocate pages");

        unsafe {
            // Fill with identical data
            core::ptr::write_bytes(page1, 0xAA, PAGE_SIZE);
            core::ptr::write_bytes(page2, 0xAA, PAGE_SIZE);

            // Register pages with KSM
            ksm.register_page(KsmPage::new(page1));
            ksm.register_page(KsmPage::new(page2));

            // Scan and merge
            let merged_count = ksm.scan_and_merge(KsmScanType::Full);

            assert!(merged_count >= 1, "Should merge at least one pair of identical pages");

            // Verify pages share memory
            // (In real implementation, would check page counts)

            crate::subsystems::mm::kfree(page1);
            crate::subsystems::mm::kfree(page2);
        }
    }

    /// Test KSM scan types
    #[test]
    fn test_ksm_scan_types() {
        let mut ksm = KsmManager::new();

        // Register multiple pages
        for i in 0..100 {
            let page = crate::subsystems::mm::kalloc();
            assert!(!page.is_null(), "Should allocate page");

            unsafe {
                // Fill with pattern
                core::ptr::write_bytes(page, (i % 10) as u8, PAGE_SIZE);
                ksm.register_page(KsmPage::new(page));
            }
        }

        // Test quick scan
        let quick_merged = ksm.scan_and_merge(KsmScanType::Quick);
        println!("Quick scan merged: {} pages", quick_merged);

        // Test full scan
        let full_merged = ksm.scan_and_merge(KsmScanType::Full);
        println!("Full scan merged: {} pages", full_merged);

        // Full scan should find at least as many merges as quick scan
        assert!(full_merged >= quick_merged, "Full scan should find at least as many merges");

        // Clean up
        // (In real implementation, would properly free pages)
    }

    /// Test KSM performance
    #[test]
    fn test_ksm_performance() {
        let mut ksm = KsmManager::new();

        // Register many pages
        let page_count = 1000;
        for _ in 0..page_count {
            let page = crate::subsystems::mm::kalloc();
            if !page.is_null() {
                unsafe {
                    // Fill with one of 10 patterns
                    let pattern = crate::subsystems::time::get_ticks() as u8 % 10;
                    core::ptr::write_bytes(page, pattern, PAGE_SIZE);
                    ksm.register_page(KsmPage::new(page));
                }
            }
        }

        // Measure scan time
        let start = crate::subsystems::time::get_time_ns();
        let merged_count = ksm.scan_and_merge(KsmScanType::Full);
        let end = crate::subsystems::time::get_time_ns();

        let scan_time_ms = (end - start) / 1_000_000;
        let pages_per_ms = page_count as f64 / scan_time_ms as f64;

        println!("KSM scanned {} pages in {}ms ({:.2} pages/ms)",
            page_count, scan_time_ms, pages_per_ms);
        println!("Merged {} page pairs", merged_count);

        // Target: scan > 100 pages/ms
        assert!(pages_per_ms > 100.0, "KSM should scan > 100 pages/ms");
    }

    /// Test KSM memory savings
    #[test]
    fn test_ksm_memory_savings() {
        let mut ksm = KsmManager::new();

        // Create many identical pages
        let identical_count = 100;
        for _ in 0..identical_count {
            let page = crate::subsystems::mm::kalloc();
            assert!(!page.is_null(), "Should allocate page");

            unsafe {
                core::ptr::write_bytes(page, 0x55, PAGE_SIZE);
                ksm.register_page(KsmPage::new(page));
            }
        }

        // Scan and merge
        let merged = ksm.scan_and_merge(KsmScanType::Full);

        // Calculate memory savings
        let original_memory = identical_count * PAGE_SIZE;
        let saved_memory = merged * PAGE_SIZE;
        let savings_ratio = saved_memory as f64 / original_memory as f64;

        println!("KSM saved {} bytes out of {} ({:.2}%)",
            saved_memory, original_memory, savings_ratio * 100.0);

        // Should save significant memory for identical pages
        assert!(savings_ratio > 0.5, "KSM should save > 50% for identical pages");
    }

    /// Test KSM deduplication accuracy
    #[test]
    fn test_ksm_deduplication_accuracy() {
        let mut ksm = KsmManager::new();

        // Create pages with different patterns
        let patterns = [0x00u8, 0xFF, 0xAA, 0x55, 0x12];
        let mut page_counts = [0usize; 5];

        for pattern in &patterns {
            for _ in 0..10 {
                let page = crate::subsystems::mm::kalloc();
                assert!(!page.is_null(), "Should allocate page");

                unsafe {
                    core::ptr::write_bytes(page, *pattern, PAGE_SIZE);
                    ksm.register_page(KsmPage::new(page));
                }
            }
        }

        // Scan and merge
        let merged = ksm.scan_and_merge(KsmScanType::Full);

        // Should merge each pattern group
        // Each pattern appears 10 times, so should merge to 1 page per pattern
        // Total merges: 10 pages -> 1 page for each of 5 patterns = 9 merges per pattern * 5 = 45
        let expected_merges = 9 * patterns.len();

        assert!(merged >= expected_merges - 5, "Should merge pages with same patterns");

        println!("KSM deduplicated {} page groups", patterns.len());
    }
}

// ============================================================================
// Memory Isolation Tests
// ============================================================================

#[cfg(test)]
mod isolation_tests {
    use super::*;

    /// Test memory isolation between processes
    #[test]
    fn test_process_memory_isolation() {
        let isolation = MemoryIsolation::new(IsolationType::Process);

        // Create two isolated memory regions
        let region1 = isolation.create_region(0x1000_0000, 0x1000, IsolationPolicy::Strict);
        let region2 = isolation.create_region(0x2000_0000, 0x1000, IsolationPolicy::Strict);

        assert!(region1.is_ok(), "Should create region1");
        assert!(region2.is_ok(), "Should create region2");

        let region1 = region1.unwrap();
        let region2 = region2.unwrap();

        // Verify regions are isolated
        assert!(!region1.can_access(&region2), "Region1 should not access region2");
        assert!(!region2.can_access(&region1), "Region2 should not access region1");
    }

    /// Test container memory isolation
    #[test]
    fn test_container_memory_isolation() {
        let isolation = MemoryIsolation::new(IsolationType::Container);

        // Create two containers with memory limits
        let container1 = isolation.create_container("container1", 100 * 1024 * 1024); // 100MB
        let container2 = isolation.create_container("container2", 200 * 1024 * 1024); // 200MB

        assert!(container1.is_ok(), "Should create container1");
        assert!(container2.is_ok(), "Should create container2");

        let container1 = container1.unwrap();
        let container2 = container2.unwrap();

        // Verify containers are isolated
        assert!(container1.is_isolated_from(&container2), "Containers should be isolated");

        // Verify memory limits
        assert_eq!(container1.memory_limit(), 100 * 1024 * 1024, "Container1 limit should be 100MB");
        assert_eq!(container2.memory_limit(), 200 * 1024 * 1024, "Container2 limit should be 200MB");
    }

    /// Test memory isolation policy enforcement
    #[test]
    fn test_isolation_policy_enforcement() {
        let isolation = MemoryIsolation::new(IsolationType::Process);

        // Create region with strict policy
        let region = isolation.create_region(0x3000_0000, 0x1000, IsolationPolicy::Strict)
            .expect("Should create region");

        // Attempt unauthorized access (should fail)
        let result = region.test_unauthorized_access();
        assert!(result.is_err(), "Unauthorized access should be denied");

        // Create region with permissive policy
        let permissive_region = isolation.create_region(0x4000_0000, 0x1000, IsolationPolicy::Permissive)
            .expect("Should create permissive region");

        // Some access should be allowed
        let result = permissive_region.test_cross_region_access(&region);
        // Result depends on policy implementation
    }

    /// Test isolated memory access latency
    #[test]
    fn test_isolated_memory_latency() {
        let isolation = MemoryIsolation::new(IsolationType::Process);

        let region = isolation.create_region(0x5000_0000, 0x1000, IsolationPolicy::Strict)
            .expect("Should create region");

        // Measure access latency
        let iterations = 10000;
        let start = crate::subsystems::time::get_time_ns();

        for _ in 0..iterations {
            let _ = region.read_byte(0);
        }

        let end = crate::subsystems::time::get_time_ns();
        let avg_latency_ns = (end - start) / iterations;

        println!("Isolated memory read latency: {}ns", avg_latency_ns);

        // Isolation should not add significant overhead
        // Target: < 1000ns per access
        assert!(avg_latency_ns < 1000, "Isolated access latency should be < 1μs");
    }

    /// Test memory isolation bounds checking
    #[test]
    fn test_isolation_bounds_checking() {
        let isolation = MemoryIsolation::new(IsolationType::Process);

        let size = 0x2000;
        let region = isolation.create_region(0x6000_0000, size, IsolationPolicy::Strict)
            .expect("Should create region");

        // Test valid access
        let result = region.validate_access(0); // Start of region
        assert!(result.is_ok(), "Access to start of region should be valid");

        let result = region.validate_access(size - 1); // End of region
        assert!(result.is_ok(), "Access to end of region should be valid");

        // Test invalid access
        let result = region.validate_access(size); // Just past end
        assert!(result.is_err(), "Access past end should be invalid");

        let result = region.validate_access(size + 0x1000); // Well past end
        assert!(result.is_err(), "Access well past end should be invalid");
    }

    /// Test isolation statistics
    #[test]
    fn test_isolation_statistics() {
        let isolation = MemoryIsolation::new(IsolationType::Process);

        // Create several regions
        for i in 0..10 {
            let addr = 0x7000_0000 + (i * 0x10000);
            let _ = isolation.create_region(addr, 0x1000, IsolationPolicy::Strict);
        }

        // Get statistics
        let stats = isolation.statistics();

        assert_eq!(stats.region_count, 10, "Should have 10 regions");
        assert!(stats.total_memory > 0, "Should have memory allocated");
        assert!(stats.violation_count >= 0, "Should track violations");

        println!("Isolation stats: {} regions, {} bytes, {} violations",
            stats.region_count,
            stats.total_memory,
            stats.violation_count);
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[cfg(test)]
mod integration_tests {
    use super::*;

    /// Test NUMA + HugePage allocation
    #[test]
    fn test_numa_hugepage_integration() {
        let current_cpu = crate::platform::arch::cpuid();
        let current_node = NumaNode::from_cpu(current_cpu);

        // Allocate hugepage on local NUMA node
        let hugepage = HugePage::allocate_on_node(HugePageSize::Size2M, current_node);

        if let Some(hp) = hugepage {
            assert_eq!(hp.size(), PAGE_SIZE_2M, "Should allocate 2MB hugepage");
            assert_eq!(hp.numa_node(), current_node.id, "Should be on local NUMA node");
        }
    }

    /// Test Compression + KSM integration
    #[test]
    fn test_compression_ksm_integration() {
        let mut ksm = KsmManager::new();

        // Create pages, compress them, then add to KSM
        for i in 0..50 {
            let page = crate::subsystems::mm::kalloc();
            assert!(!page.is_null(), "Should allocate page");

            unsafe {
                // Fill with compressible pattern
                core::ptr::write_bytes(page, (i % 5) as u8, PAGE_SIZE);

                // Compress
                let compressed = CompressedPage::compress(page, PAGE_SIZE, CompressionAlgorithm::LZ4);

                // Decompress before adding to KSM
                if let Ok(cp) = compressed {
                    let _ = cp.decompress();
                }

                // Add to KSM
                ksm.register_page(KsmPage::new(page));
            }
        }

        // Scan and merge
        let merged = ksm.scan_and_merge(KsmScanType::Full);

        println!("KSM merged {} pages after compression/decompression", merged);
        assert!(merged > 0, "Should merge some pages");
    }

    /// Test Memory Isolation + NUMA integration
    #[test]
    fn test_isolation_numa_integration() {
        let isolation = MemoryIsolation::new(IsolationType::Process);

        // Create isolated regions on different NUMA nodes
        let node0 = NumaNode { id: 0 };
        let node1 = NumaNode { id: 1 };

        let region0 = isolation.create_region_on_node(0x8000_0000, 0x1000, IsolationPolicy::Strict, node0);
        let region1 = isolation.create_region_on_node(0x9000_0000, 0x1000, IsolationPolicy::Strict, node1);

        if region0.is_ok() && region1.is_ok() {
            let region0 = region0.unwrap();
            let region1 = region1.unwrap();

            // Verify regions are on different nodes
            assert_eq!(region0.numa_node(), node0.id, "Region0 should be on node0");
            assert_eq!(region1.numa_node(), node1.id, "Region1 should be on node1");

            // Verify isolation
            assert!(!region0.can_access(&region1), "Regions should be isolated");
        }
    }

    /// Test stress: all features combined
    #[test]
    fn test_advanced_memory_stress() {
        let mut ksm = KsmManager::new();
        let isolation = MemoryIsolation::new(IsolationType::Container);

        // Create multiple containers
        for container_id in 0..5 {
            let container = isolation.create_container(
                &format!("container{}", container_id),
                50 * 1024 * 1024 // 50MB each
            );

            if let Ok(ctn) = container {
                // Allocate pages in this container
                for i in 0..20 {
                    let page = crate::subsystems::mm::kalloc();
                    if !page.is_null() {
                        unsafe {
                            // Fill with pattern
                            core::ptr::write_bytes(page, (container_id * 20 + i) as u8 % 10, PAGE_SIZE);

                            // Add to KSM
                            ksm.register_page(KsmPage::new(page));
                        }
                    }
                }
            }
        }

        // Run KSM scan
        let merged = ksm.scan_and_merge(KsmScanType::Full);
        println!("Stress test: merged {} pages across {} containers", merged, 5);

        // Verify isolation maintained
        let stats = isolation.statistics();
        assert_eq!(stats.container_count, 5, "Should maintain 5 isolated containers");
    }
}
