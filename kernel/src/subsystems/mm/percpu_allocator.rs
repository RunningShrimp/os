extern crate alloc;

use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use core::sync::atomic::{AtomicPtr, AtomicUsize, AtomicBool, Ordering};

use crate::subsystems::mm::allocator::HybridAllocator;
use crate::arch::cpu_id;
use crate::subsystems::sync::Once;

const CACHE_LINE_SIZE: usize = 64;

#[repr(align(64))]
pub struct PerCpuAllocatorSlot {
    allocator: *mut HybridAllocator,
    initialized: AtomicBool,
    _padding: [u8; CACHE_LINE_SIZE - 16],
}

impl PerCpuAllocatorSlot {
    pub const fn uninit() -> Self {
        Self {
            allocator: core::ptr::null_mut(),
            initialized: AtomicBool::new(false),
            _padding: [0; CACHE_LINE_SIZE - 16],
        }
    }

    pub fn get(&self) -> Option<&HybridAllocator> {
        if self.initialized.load(Ordering::Acquire) && !self.allocator.is_null() {
            unsafe { Some(&*self.allocator) }
        } else {
            None
        }
    }

    pub fn get_mut(&self) -> Option<&mut HybridAllocator> {
        if self.initialized.load(Ordering::Acquire) && !self.allocator.is_null() {
            unsafe { Some(&mut *self.allocator) }
        } else {
            None
        }
    }

    pub fn initialize(&self, init_fn: impl FnOnce() -> HybridAllocator) {
        if !self.initialized.load(Ordering::Acquire) {
            if let Ok(false) = self.initialized.compare_exchange(
                false,
                true,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                unsafe {
                    let allocator = Box::leak(Box::new(init_fn()));
                    (self as *const Self as *mut Self).write_volatile(Self {
                        allocator,
                        initialized: AtomicBool::new(true),
                        _padding: [0; CACHE_LINE_SIZE - 16],
                    });
                }
            }
        }
    }
}

#[repr(align(64))]
pub struct PerCpuAllocator {
    slots: Vec<PerCpuAllocatorSlot>,
    max_cpus: usize,
    _padding: [u8; CACHE_LINE_SIZE - 24],
}

impl PerCpuAllocator {
    pub fn new(num_cpus: usize) -> Self {
        let cpu_count = core::cmp::max(1, num_cpus).min(256);
        let mut slots = Vec::with_capacity(cpu_count);
        for _ in 0..cpu_count {
            slots.push(PerCpuAllocatorSlot::uninit());
        }
        Self {
            slots,
            max_cpus: cpu_count,
            _padding: [0; CACHE_LINE_SIZE - 24],
        }
    }

    pub fn cpu_count(&self) -> usize {
        self.max_cpus
    }

    pub fn init_cpu(&self, cpu_id: usize, init_fn: impl FnOnce() -> HybridAllocator) {
        if cpu_id < self.max_cpus {
            self.slots[cpu_id].initialize(init_fn);
        }
    }

    pub unsafe fn alloc_on(&self, cpu_id: usize, layout: Layout) -> *mut u8 {
        let target = cpu_id.min(self.max_cpus - 1);
        if let Some(allocator) = self.slots[target].get() {
            allocator.alloc(layout)
        } else {
            core::ptr::null_mut()
        }
    }

    pub unsafe fn dealloc_on(&self, cpu_id: usize, ptr: *mut u8, layout: Layout) {
        let target = cpu_id.min(self.max_cpus - 1);
        if let Some(allocator) = self.slots[target].get() {
            allocator.dealloc(ptr, layout);
        }
    }

    pub unsafe fn init_cpu_allocator(
        &self,
        cpu_id: usize,
        slab_start: usize,
        slab_size: usize,
        buddy_start: usize,
        buddy_size: usize,
        page_size: usize,
    ) {
        self.init_cpu(cpu_id, || {
            let mut alloc = HybridAllocator::new();
            alloc.init(slab_start, slab_size, buddy_start, buddy_size, page_size);
            alloc
        });
    }
}

static GLOBAL_PERCPU_ALLOC_PTR: AtomicPtr<PerCpuAllocator> = AtomicPtr::new(core::ptr::null_mut());
static GLOBAL_PERCPU_INIT: Once = Once::new();

pub fn init_global(num_cpus: usize) {
    GLOBAL_PERCPU_INIT.call_once(|| {
        let allocator = Box::leak(Box::new(PerCpuAllocator::new(num_cpus)));
        GLOBAL_PERCPU_ALLOC_PTR.store(allocator, Ordering::Release);
    });
}

pub fn with_global<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&PerCpuAllocator) -> R,
{
    let alloc = GLOBAL_PERCPU_ALLOC_PTR.load(Ordering::Acquire);
    if alloc.is_null() {
        None
    } else {
        Some(unsafe { f(&*alloc) })
    }
}

pub unsafe fn percpu_alloc(cpu_id: usize, layout: Layout) -> Option<*mut u8> {
    with_global(|alloc| alloc.alloc_on(cpu_id, layout))
}

pub unsafe fn percpu_dealloc(cpu_id: usize, ptr: *mut u8, layout: Layout) {
    with_global(|alloc| {
        alloc.dealloc_on(cpu_id, ptr, layout);
    });
}

struct FreeBlock {
    size: usize,
    next: Option<NonNull<FreeBlock>>,
}

#[repr(align(64))]
pub struct PerCpuLocalAllocator {
    freelist_head: AtomicPtr<FreeBlock>,
    allocated_count: AtomicUsize,
    size_class_freelists: [AtomicPtr<FreeBlock>; 9],
    size_class_counts: [AtomicUsize; 9],
    cache_hits: AtomicUsize,
    cache_misses: AtomicUsize,
    cache_evictions: AtomicUsize,
    _padding: [u8; CACHE_LINE_SIZE - 80],
}

impl PerCpuLocalAllocator {
    pub fn new() -> Self {
        Self {
            freelist_head: AtomicPtr::new(core::ptr::null_mut()),
            allocated_count: AtomicUsize::new(0),
            size_class_freelists: [const { AtomicPtr::new(core::ptr::null_mut()) }; 9],
            size_class_counts: [const { AtomicUsize::new(0) }; 9],
            cache_hits: AtomicUsize::new(0),
            cache_misses: AtomicUsize::new(0),
            cache_evictions: AtomicUsize::new(0),
            _padding: [0; CACHE_LINE_SIZE - 80],
        }
    }

    pub unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let size = layout.size().max(layout.align());
        if let Some(block) = self.try_alloc_from_freelist(size) {
            return block.as_ptr() as *mut u8;
        }
        self.alloc_from_global(layout)
    }

    pub unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let size = layout.size().max(layout.align());
        if size <= 4096 {
            self.add_to_freelist(ptr, size);
        } else {
            crate::subsystems::mm::allocator::get_global_allocator().dealloc(ptr, layout);
        }
    }

    fn try_alloc_from_freelist(&self, size: usize) -> Option<NonNull<FreeBlock>> {
        let mut head = self.freelist_head.load(Ordering::Acquire);
        loop {
            if head.is_null() {
                return None;
            }
            let block = unsafe { &*head };
            if block.size >= size {
                let next = block.next.map_or(core::ptr::null_mut(), |p| p.as_ptr());
                match self.freelist_head.compare_exchange_weak(
                    head,
                    next,
                    Ordering::AcqRel,
                    Ordering::Relaxed,
                ) {
                    Ok(_) => {
                        self.allocated_count.fetch_sub(1, Ordering::Relaxed);
                        return NonNull::new(head);
                    }
                    Err(new_head) => {
                        head = new_head;
                        continue;
                    }
                }
            }
            head = block.next.map_or(core::ptr::null_mut(), |p| p.as_ptr());
        }
    }

    pub unsafe fn alloc_from_global(&self, layout: Layout) -> *mut u8 {
        let ptr = crate::subsystems::mm::allocator::get_global_allocator().alloc(layout);
        if !ptr.is_null() {
            self.allocated_count.fetch_add(1, Ordering::Relaxed);
        }
        ptr
    }

    pub unsafe fn add_to_freelist(&self, ptr: *mut u8, size: usize) {
        let block = ptr as *mut FreeBlock;
        unsafe { (*block).size = size };
        let mut head = self.freelist_head.load(Ordering::Acquire);
        loop {
            unsafe { (*block).next = NonNull::new(head) };
            match self.freelist_head.compare_exchange_weak(
                head,
                block,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    self.allocated_count.fetch_sub(1, Ordering::Relaxed);
                    return;
                }
                Err(new_head) => {
                    head = new_head;
                    continue;
                }
            }
        }
    }

    pub fn stats(&self) -> (usize, usize) {
        let allocated = self.allocated_count.load(Ordering::Relaxed);
        let freelist_len = if self.freelist_head.load(Ordering::Acquire).is_null() {
            0
        } else {
            1
        };
        (allocated, freelist_len)
    }
}

static mut PER_CPU_ALLOCATORS: Option<[PerCpuLocalAllocator; 256]> = None;
static PER_CPU_ALLOCATORS_INIT: Once = Once::new();

pub fn init_percpu_allocators() {
    PER_CPU_ALLOCATORS_INIT.call_once(|| {
        unsafe {
            let mut allocators: [PerCpuLocalAllocator; 256] = core::mem::zeroed();
            for i in 0..256 {
                allocators[i] = PerCpuLocalAllocator::new();
            }
            PER_CPU_ALLOCATORS = Some(allocators);
        }
    });
}

pub fn current_cpu_allocator() -> &'static PerCpuLocalAllocator {
    if !PER_CPU_ALLOCATORS_INIT.is_completed() {
        init_percpu_allocators();
    }
    let cpu_id = cpu_id() as usize;
    unsafe {
        PER_CPU_ALLOCATORS
            .as_ref()
            .map(|allocators| &allocators[cpu_id % 256])
            .unwrap_or_else(|| {
                init_percpu_allocators();
                PER_CPU_ALLOCATORS.as_ref().unwrap().get(0).unwrap()
            })
    }
}

pub struct PerCpuGlobalAllocator;

unsafe impl GlobalAlloc for PerCpuGlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        current_cpu_allocator().alloc(layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        current_cpu_allocator().dealloc(ptr, layout)
    }
}

pub fn flush_all_caches() {
    unsafe {
        if let Some(ref allocators) = PER_CPU_ALLOCATORS {
            for i in 0..allocators.len() {
                let allocator = &allocators[i];
                let (allocated, freelist_len) = allocator.stats();
                if freelist_len > 0 {}
            }
        }
    }
}

pub fn get_all_cpu_stats() -> Vec<(usize, (usize, usize))> {
    let mut stats = Vec::new();
    unsafe {
        if let Some(ref allocators) = PER_CPU_ALLOCATORS {
            for i in 0..allocators.len() {
                let allocator = &allocators[i];
                let alloc_stats = allocator.stats();
                stats.push((i, alloc_stats));
            }
        }
    }
    stats
}

pub fn balance_caches() {
    let mut stats = get_all_cpu_stats();
    stats.sort_by_key(|&(_, (allocated, _))| allocated);
}

pub fn warmup_caches(cpu_id: usize, count: usize) {
    unsafe {
        if let Some(ref allocators) = PER_CPU_ALLOCATORS {
            if cpu_id >= allocators.len() {
                return;
            }
            let allocator = &allocators[cpu_id];
            for _ in 0..count {
                let layout = Layout::from_size_align(64, 8).unwrap();
                let ptr = allocator.alloc_from_global(layout);
                if !ptr.is_null() {
                    allocator.add_to_freelist(ptr, 64);
                }
            }
        }
    }
}
