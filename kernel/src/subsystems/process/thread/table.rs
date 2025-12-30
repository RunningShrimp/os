use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};

use crate::subsystems::sync::{Once, Mutex};
use super::types::{Thread, Tid, Pid, ThreadState, ThreadType, MAX_THREADS, INVALID_TID};
use super::api::ThreadError;
use crate::subsystems::mm::{kalloc, kfree};
use crate::process::TrapFrame;

/// Thread table for managing all threads in the system
///
/// Uses an object pool pattern for efficient thread allocation and deallocation.
/// Maintains a free list of thread slots for O(1) allocation performance.
pub struct ThreadTable {
    /// Fixed-size array of thread slots
    threads: [Thread; MAX_THREADS],
    /// Next thread ID to allocate
    next_tid: AtomicUsize,
    /// PID to TID mapping for fast lookup
    pid_to_tids: BTreeMap<Pid, Vec<Tid>>,
    /// Free thread slots
    free_slots: Vec<usize>,
    /// Active thread count
    active_count: AtomicUsize,
}

impl ThreadTable {
    /// Create a new thread table with object pool
    pub fn new() -> Self {
        #[allow(invalid_value)]
        let mut threads: [Thread; MAX_THREADS] = unsafe {
            // SAFETY: Thread can be created as all-zeros for Unused state
            core::mem::MaybeUninit::uninit().assume_init()
        };

        // Initialize each thread
        for i in 0..MAX_THREADS {
            threads[i] = Thread::new();
        }

        // Pre-populate free_slots with all available indices for O(1) allocation
        let mut free_slots = Vec::with_capacity(MAX_THREADS);
        for i in 0..MAX_THREADS {
            free_slots.push(i);
        }

        Self {
            threads,
            next_tid: AtomicUsize::new(1),
            pid_to_tids: BTreeMap::new(),
            free_slots,
            active_count: AtomicUsize::new(0),
        }
    }

    /// Allocate a new thread using object pool
    ///
    /// This function provides O(1) allocation by reusing freed thread slots
    /// from the free_slots list, reducing memory fragmentation and allocation overhead.
    ///
    /// # Arguments
    ///
    /// * `pid` - Process ID that owns this thread
    /// * `thread_type` - Type of thread (Kernel, User, or Main)
    ///
    /// # Returns
    ///
    /// * `Ok(&mut Thread)` if allocation succeeds
    /// * `Err(ThreadError)` if no slots are available
    pub fn alloc_thread(
        &mut self,
        pid: Pid,
        thread_type: ThreadType,
    ) -> Result<&mut Thread, ThreadError> {
        // Get free slot from object pool (O(1))
        let slot_idx = match self.free_slots.pop() {
            Some(idx) => idx,
            None => {
                // Fallback to linear search if free_slots is empty (should rarely happen)
                let mut found_slot = None;
                for (i, thread) in self.threads.iter().enumerate() {
                    if thread.state == ThreadState::Unused {
                        found_slot = Some(i);
                        break;
                    }
                }
                match found_slot {
                    Some(idx) => idx,
                    None => return Err(ThreadError::NoSlotsAvailable),
                }
            },
        };

        let tid = self.next_tid.fetch_add(1, Ordering::SeqCst);
        if tid == INVALID_TID {
            return Err(ThreadError::InvalidThreadId);
        }

        let thread = &mut self.threads[slot_idx];
        thread.init(tid, pid, thread_type)?;

        // Add to PID mapping
        self.pid_to_tids
            .entry(pid)
            .or_insert_with(Vec::new)
            .push(tid);

        // Update active count
        self.active_count.fetch_add(1, Ordering::SeqCst);

        Ok(thread)
    }

    /// Find thread by TID
    pub fn find_thread(&mut self, tid: Tid) -> Option<&mut Thread> {
        if tid == 0 || tid >= MAX_THREADS {
            return None;
        }

        let thread = &mut self.threads[tid];
        if thread.state != ThreadState::Unused && thread.tid == tid {
            Some(thread)
        } else {
            None
        }
    }

    /// Find thread by TID (immutable)
    pub fn find_thread_ref(&self, tid: Tid) -> Option<&Thread> {
        if tid == 0 || tid >= MAX_THREADS {
            return None;
        }

        let thread = &self.threads[tid];
        if thread.state != ThreadState::Unused && thread.tid == tid {
            Some(thread)
        } else {
            None
        }
    }

    /// Find all threads for a process
    pub fn find_threads_by_pid(&self, pid: Pid) -> Vec<&Thread> {
        if let Some(tids) = self.pid_to_tids.get(&pid) {
            tids.iter()
                .filter_map(|&tid| self.find_thread_ref(tid))
                .collect()
        } else {
            Vec::new()
        }
    }

    /// Free a thread and return it to the object pool
    ///
    /// This function cleans up thread resources and returns the thread slot
    /// to the free_slots pool for reuse, reducing memory fragmentation.
    ///
    /// # Arguments
    ///
    /// * `tid` - Thread ID to free
    ///
    /// # Returns
    ///
    /// * `Ok(())` if the thread was successfully freed
    /// * `Err(ThreadError)` if the thread ID is invalid
    pub fn free_thread(&mut self, tid: Tid) -> Result<(), ThreadError> {
        if tid == 0 || tid >= MAX_THREADS {
            return Err(ThreadError::InvalidThreadId);
        }

        let thread = &mut self.threads[tid];
        if thread.state == ThreadState::Unused || thread.tid != tid {
            return Err(ThreadError::InvalidThreadId);
        }

        let pid = thread.pid;

        // Clean up thread resources
        thread.cleanup();

        // Remove from PID mapping
        if let Some(tids) = self.pid_to_tids.get_mut(&pid) {
            tids.retain(|&t| t != tid);
            if tids.is_empty() {
                self.pid_to_tids.remove(&pid);
            }
        }

        // Return slot to object pool for reuse (O(1))
        // Only add back if free_slots is not full (prevent unbounded growth)
        if self.free_slots.len() < MAX_THREADS {
            self.free_slots.push(tid);
        }

        // Update active count
        self.active_count.fetch_sub(1, Ordering::SeqCst);

        Ok(())
    }

    /// Get number of active threads
    pub fn active_count(&self) -> usize {
        self.active_count.load(Ordering::SeqCst)
    }

    /// Get maximum number of threads
    pub fn max_threads(&self) -> usize {
        MAX_THREADS
    }

    /// Get iterator over all threads
    pub fn iter(&self) -> impl Iterator<Item = &Thread> {
        self.threads
            .iter()
            .filter(|t| t.state != ThreadState::Unused)
    }

    /// Get mutable iterator over all threads
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Thread> {
        self.threads
            .iter_mut()
            .filter(|t| t.state != ThreadState::Unused)
    }
}

/// Thread resource pools for efficient allocation
/// Reuses freed kernel stacks and trapframes to reduce memory fragmentation
struct ThreadResourcePools {
    stack_pool: Mutex<alloc::vec::Vec<usize>>, // Reusable kernel stack addresses
    trapframe_pool: Mutex<alloc::vec::Vec<usize>>, // Reusable trapframe addresses
}

impl ThreadResourcePools {
    fn new() -> Self {
        Self {
            stack_pool: Mutex::new(alloc::vec::Vec::new()),
            trapframe_pool: Mutex::new(alloc::vec::Vec::new()),
        }
    }

    /// Get a kernel stack from pool or allocate new one
    fn alloc_stack(&self) -> Option<usize> {
        let mut pool = self.stack_pool.lock();
        pool.pop().or_else(|| {
            let stack = kalloc();
            if stack.is_null() {
                None
            } else {
                Some(stack as usize)
            }
        })
    }

    /// Return kernel stack to pool for reuse
    fn free_stack(&self, stack_addr: usize) {
        if stack_addr != 0 {
            let mut pool = self.stack_pool.lock();
            // Limit pool size to prevent unbounded growth
            if pool.len() < MAX_THREADS {
                pool.push(stack_addr);
            } else {
                // Pool is full, free the memory
                drop(pool);
                unsafe {
                    kfree(stack_addr as *mut u8);
                }
            }
        }
    }

    /// Get a trapframe from pool or allocate new one
    fn alloc_trapframe(&self) -> Option<usize> {
        let mut pool = self.trapframe_pool.lock();
        pool.pop().or_else(|| {
            let tf = kalloc() as *mut TrapFrame;
            if tf.is_null() {
                None
            } else {
                Some(tf as usize)
            }
        })
    }

    /// Return trapframe to pool for reuse
    fn free_trapframe(&self, tf_addr: usize) {
        if tf_addr != 0 {
            let mut pool = self.trapframe_pool.lock();
            // Limit pool size to prevent unbounded growth
            if pool.len() < MAX_THREADS {
                pool.push(tf_addr);
            } else {
                // Pool is full, free the memory
                drop(pool);
                unsafe {
                    kfree(tf_addr as *mut u8);
                }
            }
        }
    }
}

/// Global thread resource pools
static THREAD_RESOURCE_POOLS: Once = Once::new();
static mut THREAD_POOLS: Option<ThreadResourcePools> = None;

fn get_thread_pools() -> &'static ThreadResourcePools {
    unsafe {
        THREAD_RESOURCE_POOLS.call_once(|| {
            THREAD_POOLS = Some(ThreadResourcePools::new());
        });
        THREAD_POOLS.as_ref().unwrap()
    }
}

/// Global thread table instance
pub static mut THREAD_TABLE: Option<ThreadTable> = None;
pub static THREAD_TABLE_INIT: Once = Once::new();
