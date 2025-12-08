use criterion::{criterion_group, criterion_main, Criterion, black_box};

// Mock buffer cache implementation for benchmarking
// In a real scenario, this would use the actual BufCache implementation

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct MockCacheKey {
    dev: u32,
    blockno: u32,
}

impl MockCacheKey {
    fn new(dev: u32, blockno: u32) -> Self {
        Self { dev, blockno }
    }
}

#[derive(Debug, Clone)]
struct MockBuf {
    data: [u8; 1024],
    refcnt: u32,
    valid: bool,
}

impl MockBuf {
    fn new() -> Self {
        Self {
            data: [0; 1024],
            refcnt: 0,
            valid: false,
        }
    }
}

struct MockBufCacheLinear {
    bufs: Vec<MockBuf>,
}

impl MockBufCacheLinear {
    fn new(capacity: usize) -> Self {
        Self {
            bufs: (0..capacity).map(|_| MockBuf::new()).collect(),
        }
    }

    fn find_buffer(&mut self, dev: u32, blockno: u32) -> Option<usize> {
        // Linear search - O(n)
        for (i, buf) in self.bufs.iter().enumerate() {
            if buf.valid && i == ((dev ^ blockno) % self.bufs.len()) {
                return Some(i);
            }
        }
        None
    }

    fn insert_buffer(&mut self, dev: u32, blockno: u32) -> usize {
        let index = (dev ^ blockno) % self.bufs.len();
        self.bufs[index].valid = true;
        self.bufs[index].refcnt += 1;
        index
    }
}

struct MockBufCacheHash {
    bufs: Vec<MockBuf>,
    cache: std::collections::HashMap<MockCacheKey, usize>,
    free_indices: std::vec::Vec<usize>,
}

impl MockBufCacheHash {
    fn new(capacity: usize) -> Self {
        let mut free_indices: std::vec::Vec<usize> = (0..capacity).collect();
        free_indices.reverse(); // For efficient pop

        Self {
            bufs: (0..capacity).map(|_| MockBuf::new()).collect(),
            cache: std::collections::HashMap::new(),
            free_indices,
        }
    }

    fn find_buffer(&mut self, dev: u32, blockno: u32) -> Option<usize> {
        // Hash lookup - O(1) average case
        let key = MockCacheKey::new(dev, blockno);
        self.cache.get(&key).copied()
    }

    fn insert_buffer(&mut self, dev: u32, blockno: u32) -> usize {
        let key = MockCacheKey::new(dev, blockno);

        if let Some(&index) = self.cache.get(&key) {
            self.bufs[index].refcnt += 1;
            return index;
        }

        let index = self.free_indices.pop().unwrap_or(0);
        self.cache.insert(key, index);
        self.bufs[index].valid = true;
        self.bufs[index].refcnt = 1;
        index
    }
}

fn benchmark_linear_search(c: &mut Criterion) {
    let mut cache = MockBufCacheLinear::new(1000);
    let mut rng = fastrand::Rng::new();

    // Pre-populate some entries
    for i in 0..500 {
        let dev = rng.u32(..);
        let blockno = rng.u32(..);
        cache.insert_buffer(dev, blockno);
    }

    c.bench_function("buffer_cache_linear_search", |b| {
        b.iter(|| {
            let dev = rng.u32(..);
            let blockno = rng.u32(..);
            black_box(cache.find_buffer(dev, blockno))
        })
    });
}

fn benchmark_hash_lookup(c: &mut Criterion) {
    let mut cache = MockBufCacheHash::new(1000);
    let mut rng = fastrand::Rng::new();

    // Pre-populate some entries
    for i in 0..500 {
        let dev = rng.u32(..);
        let blockno = rng.u32(..);
        cache.insert_buffer(dev, blockno);
    }

    c.bench_function("buffer_cache_hash_lookup", |b| {
        b.iter(|| {
            let dev = rng.u32(..);
            let blockno = rng.u32(..);
            black_box(cache.find_buffer(dev, blockno))
        })
    });
}

fn benchmark_linear_insert(c: &mut Criterion) {
    let mut cache = MockBufCacheLinear::new(1000);
    let mut rng = fastrand::Rng::new();

    c.bench_function("buffer_cache_linear_insert", |b| {
        b.iter(|| {
            let dev = rng.u32(..);
            let blockno = rng.u32(..);
            black_box(cache.insert_buffer(dev, blockno))
        })
    });
}

fn benchmark_hash_insert(c: &mut Criterion) {
    let mut cache = MockBufCacheHash::new(1000);
    let mut rng = fastrand::Rng::new();

    c.bench_function("buffer_cache_hash_insert", |b| {
        b.iter(|| {
            let dev = rng.u32(..);
            let blockno = rng.u32(..);
            black_box(cache.insert_buffer(dev, blockno))
        })
    });
}

fn benchmark_mixed_operations(c: &mut Criterion) {
    let mut rng = fastrand::Rng::new();

    // Benchmark linear cache with mixed operations
    c.bench_function("buffer_cache_linear_mixed", |b| {
        b.iter(|| {
            let mut cache = MockBufCacheLinear::new(100);

            for _ in 0..100 {
                let dev = rng.u32(..);
                let blockno = rng.u32(..);

                if rng.bool() {
                    // 70% find operations
                    black_box(cache.find_buffer(dev, blockno));
                } else {
                    // 30% insert operations
                    black_box(cache.insert_buffer(dev, blockno));
                }
            }
        })
    });

    // Benchmark hash cache with mixed operations
    c.bench_function("buffer_cache_hash_mixed", |b| {
        b.iter(|| {
            let mut cache = MockBufCacheHash::new(100);

            for _ in 0..100 {
                let dev = rng.u32(..);
                let blockno = rng.u32(..);

                if rng.bool() {
                    // 70% find operations
                    black_box(cache.find_buffer(dev, blockno));
                } else {
                    // 30% insert operations
                    black_box(cache.insert_buffer(dev, blockno));
                }
            }
        })
    });
}

fn benchmark_cache_collision(c: &mut Criterion) {
    // Test hash collision resistance by using similar keys
    let mut cache = MockBufCacheHash::new(1000);
    let mut rng = fastrand::Rng::new();

    c.bench_function("buffer_cache_hash_collision", |b| {
        b.iter(|| {
            // Use keys that might cause collisions
            let base = rng.u32(..100);
            for i in 0..10 {
                let dev = base;
                let blockno = i as u32;
                black_box(cache.find_buffer(dev, blockno));
            }
        })
    });
}

criterion_group!(
    benches,
    benchmark_linear_search,
    benchmark_hash_lookup,
    benchmark_linear_insert,
    benchmark_hash_insert,
    benchmark_mixed_operations,
    benchmark_cache_collision
);
criterion_main!(benches);