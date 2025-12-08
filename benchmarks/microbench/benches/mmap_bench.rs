use criterion::{criterion_group, criterion_main, Criterion, black_box};

// Mock mmap/munmap implementation for benchmarking
// In a real scenario, this would call the actual syscalls
extern crate alloc;

#[derive(Clone, Copy)]
struct MockPage {
    addr: usize,
    size: usize,
}

// Simulate a simple page allocator for benchmarking
struct MockAllocator {
    pages: alloc::vec::Vec<MockPage>,
    next_addr: usize,
}

impl MockAllocator {
    fn new() -> Self {
        Self {
            pages: alloc::vec::Vec::new(),
            next_addr: 0x20000000, // Start from 512MB
        }
    }

    fn allocate(&mut self, size: usize) -> usize {
        let addr = self.next_addr;
        self.next_addr += size;
        self.pages.push(MockPage { addr, size });
        addr
    }

    fn deallocate(&mut self, addr: usize) -> Option<usize> {
        for (i, page) in self.pages.iter().enumerate() {
            if page.addr == addr {
                let size = page.size;
                self.pages.remove(i);
                return Some(size);
            }
        }
        None
    }

    fn find(&self, addr: usize) -> Option<&MockPage> {
        self.pages.iter().find(|p| p.addr == addr)
    }
}

// Simulate mmap implementation
fn mock_mmap(allocator: &mut MockAllocator, size: usize) -> usize {
    // Simulate validation and alignment
    let aligned_size = (size + 4095) & !4095; // Page-align
    allocator.allocate(aligned_size)
}

// Simulate munmap implementation
fn mock_munmap(allocator: &mut MockAllocator, addr: usize, size: usize) -> bool {
    let aligned_size = (size + 4095) & !4095;
    match allocator.deallocate(addr) {
        Some(deallocated) => deallocated == aligned_size,
        None => false,
    }
}

// Simulate process lookup (O(log n) with BTreeMap)
use alloc::collections::BTreeMap;

struct MockProcessTable {
    processes: BTreeMap<usize, &'static str>,
    pid_counter: usize,
}

impl MockProcessTable {
    fn new() -> Self {
        Self {
            processes: BTreeMap::new(),
            pid_counter: 1,
        }
    }

    fn add_process(&mut self, name: &'static str) -> usize {
        let pid = self.pid_counter;
        self.pid_counter += 1;
        self.processes.insert(pid, name);
        pid
    }

    fn find(&self, pid: usize) -> Option<&'static str> {
        self.processes.get(&pid).copied()
    }
}

fn bench_mmap_basic(c: &mut Criterion) {
    let mut allocator = MockAllocator::new();

    c.bench_function("mmap_single_page", |b| {
        b.iter(|| {
            let addr = mock_mmap(black_box(&mut allocator), 4096);
            black_box(addr);
            mock_munmap(black_box(&mut allocator), addr, 4096);
        });
    });

    c.bench_function("mmap_64_pages", |b| {
        b.iter(|| {
            let addr = mock_mmap(black_box(&mut allocator), 64 * 4096);
            black_box(addr);
            mock_munmap(black_box(&mut allocator), addr, 64 * 4096);
        });
    });

    c.bench_function("mmap_1mb", |b| {
        b.iter(|| {
            let addr = mock_mmap(black_box(&mut allocator), 1024 * 1024);
            black_box(addr);
            mock_munmap(black_box(&mut allocator), addr, 1024 * 1024);
        });
    });
}

fn bench_mmap_multiple(c: &mut Criterion) {
    c.bench_function("mmap_10_consecutive", |b| {
        b.iter(|| {
            let mut allocator = MockAllocator::new();
            let mut addrs = alloc::vec::Vec::new();

            // Allocate 10 pages
            for _ in 0..10 {
                let addr = mock_mmap(black_box(&mut allocator), 4096);
                addrs.push(addr);
            }

            // Free all pages
            for addr in addrs {
                mock_munmap(black_box(&mut allocator), addr, 4096);
            }
        });
    });

    c.bench_function("mmap_100_consecutive", |b| {
        b.iter(|| {
            let mut allocator = MockAllocator::new();
            let mut addrs = alloc::vec::Vec::new();

            // Allocate 100 pages
            for _ in 0..100 {
                let addr = mock_mmap(black_box(&mut allocator), 4096);
                addrs.push(addr);
            }

            // Free all pages
            for addr in addrs {
                mock_munmap(black_box(&mut allocator), addr, 4096);
            }
        });
    });
}

fn bench_process_lookup(c: &mut Criterion) {
    let mut table = MockProcessTable::new();

    // Add 1000 processes
    for i in 0..1000 {
        table.add_process(alloc::format!("process_{}", i).leak());
    }

    c.bench_function("process_lookup_existing", |b| {
        b.iter(|| {
            let pid = black_box(500); // Lookup existing process
            let name = table.find(pid);
            black_box(name);
        });
    });

    c.bench_function("process_lookup_nonexistent", |b| {
        b.iter(|| {
            let pid = black_box(2000); // Lookup nonexistent process
            let name = table.find(pid);
            black_box(name);
        });
    });

    c.bench_function("process_lookup_100_random", |b| {
        b.iter(|| {
            for i in 0..100 {
                let pid = black_box((i * 17) % 1500); // Mix of existing and non-existing
                let name = table.find(pid);
                black_box(name);
            }
        });
    });
}

fn bench_allocation_performance(c: &mut Criterion) {
    c.bench_function("allocation_deallocation_cycle", |b| {
        b.iter(|| {
            let mut allocator = MockAllocator::new();

            // Cycle of allocation and deallocation
            for _ in 0..50 {
                let addr = mock_mmap(black_box(&mut allocator), 4096);
                mock_munmap(black_box(&mut allocator), addr, 4096);
            }
        });
    });

    c.bench_function("fragmented_allocation", |b| {
        b.iter(|| {
            let mut allocator = MockAllocator::new();
            let mut addrs = alloc::vec::Vec::new();

            // Allocate many small regions
            for i in 0..100 {
                let addr = mock_mmap(black_box(&mut allocator), 4096);
                addrs.push(addr);

                // Deallocate every other one to create fragmentation
                if i % 2 == 1 {
                    mock_munmap(black_box(&mut allocator), addrs.remove(0), 4096);
                }
            }

            // Clean up remaining
            for addr in addrs {
                mock_munmap(black_box(&mut allocator), addr, 4096);
            }
        });
    });
}

criterion_group!(
    benches,
    bench_mmap_basic,
    bench_mmap_multiple,
    bench_process_lookup,
    bench_allocation_performance
);
criterion_main!(benches);