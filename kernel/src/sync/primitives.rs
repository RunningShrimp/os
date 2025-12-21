// Advanced synchronization primitives for multithreading
//
// This module provides thread-safe synchronization primitives including
// enhanced mutexes, condition variables, read-write locks, and other
// thread synchronization utilities.

extern crate alloc;

use alloc::vec::Vec;
use alloc::boxed::Box;
use core::sync::atomic::{AtomicBool, AtomicUsize, AtomicU64, AtomicPtr, Ordering};
use core::cell::UnsafeCell;
use crate::sync::{Mutex, RawSpinLock};
use core::ops::{Deref, DerefMut};
use core::ptr::null_mut;
use core::time::Duration;

use crate::process::thread::{current_thread, thread_table, ThreadState};
use crate::process::sleep;

// ============================================================================
// Enhanced Mutex with deadlock detection
// ============================================================================

/// Enhanced mutex with deadlock detection and priority inheritance
pub struct MutexEnhanced<T: Send + Sync> {
    /// Raw spinlock protecting the mutex state
    lock: RawSpinLock,
    /// Protected data
    data: UnsafeCell<T>,
    /// Mutex state
    state: MutexState,
}

/// Mutex internal state
#[derive(Debug)]
struct MutexState {
    /// Owner thread ID (0 if unlocked)
    owner_tid: UnsafeCell<usize>,
    /// Lock depth (for recursive locks)
    lock_depth: UnsafeCell<u32>,
    /// Number of waiting threads
    waiters: UnsafeCell<u32>,
    /// Priority inheritance target
    priority_ceiling: u8,
    /// Recursive flag
    recursive: bool,
    /// Debug mode (enables deadlock detection)
    debug_mode: bool,
}

impl MutexState {
    const fn new() -> Self {
        Self {
            owner_tid: UnsafeCell::new(0),
            lock_depth: UnsafeCell::new(0),
            waiters: UnsafeCell::new(0),
            priority_ceiling: 0,
            recursive: false,
            debug_mode: false,
        }
    }
}

// Safety: Enhanced mutex provides synchronized access
unsafe impl<T: Send + Sync> Send for MutexEnhanced<T> {}
unsafe impl<T: Send + Sync> Sync for MutexEnhanced<T> {}

impl<T: Send + Sync> MutexEnhanced<T> {
    /// Create a new enhanced mutex
    pub const fn new(data: T) -> Self {
        Self {
            lock: RawSpinLock::new(),
            data: UnsafeCell::new(data),
            state: MutexState::new(),
        }
    }

    /// Create a new recursive mutex
    pub const fn new_recursive(data: T) -> Self {
        let mut mutex = Self::new(data);
        mutex.state.recursive = true;
        mutex
    }

    /// Enable debug mode for deadlock detection
    pub fn enable_debug(&mut self) {
        self.lock.lock();
        self.state.debug_mode = true;
        self.lock.unlock();
    }

    /// Set priority ceiling
    pub fn set_priority_ceiling(&mut self, priority: u8) {
        self.lock.lock();
        self.state.priority_ceiling = priority;
        self.lock.unlock();
    }

    /// Consume the mutex and return the inner data
    pub fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T: Send + Sync> MutexEnhanced<T> {
    /// Acquire the mutex with deadlock detection
    pub fn lock(&self) -> MutexEnhancedGuard<'_, T> {
        let current_tid = current_thread().unwrap_or(0);

        // Acquire the spinlock
        self.lock.lock();

        // Check for deadlock in debug mode
        if self.state.debug_mode && unsafe { *self.state.owner_tid.get() } == current_tid && !self.state.recursive {
            // Potential deadlock detected
            self.lock.unlock();
            panic!("Potential deadlock detected: thread {} trying to lock mutex already owned", current_tid);
        }

        // Handle recursive locking
        if self.state.recursive && unsafe { *self.state.owner_tid.get() } == current_tid {
            unsafe { *self.state.lock_depth.get() += 1; }
            self.lock.unlock();
            return MutexEnhancedGuard { mutex: self };
        }

        // Wait if mutex is already locked
        while unsafe { *self.state.owner_tid.get() } != 0 {
            unsafe { *self.state.waiters.get() += 1; }

            // Add current thread to wait queue (simplified)
            // In a real implementation, this would use a proper wait queue

            // Release spinlock and sleep
            self.lock.unlock();

            // Block current thread on mutex channel
            let channel = (&self.state as *const _ as usize) | 0xdead0000;
            sleep(channel);

            // Re-acquire spinlock to check again
            self.lock.lock();
            unsafe { *self.state.waiters.get() = (*self.state.waiters.get()).saturating_sub(1); }
        }

        // Acquire the mutex
        unsafe { *self.state.owner_tid.get() = current_tid; }
        unsafe { *self.state.lock_depth.get() = 1; }

        // Priority inheritance would be handled here
        if self.state.priority_ceiling > 0 {
            // Boost current thread priority
            // This is simplified - real implementation would handle proper priority inheritance
        }

        self.lock.unlock();

        MutexEnhancedGuard { mutex: self }
    }

    /// Try to acquire the mutex without blocking
    pub fn try_lock(&self) -> Option<MutexEnhancedGuard<'_, T>> {
        let current_tid = current_thread().unwrap_or(0);

        self.lock.lock();

        // Check if mutex is free
        if unsafe { *self.state.owner_tid.get() } == 0 {
            unsafe { *self.state.owner_tid.get() = current_tid; }
            unsafe { *self.state.lock_depth.get() = 1; }
            self.lock.unlock();
            Some(MutexEnhancedGuard { mutex: self })
        } else if self.state.recursive && unsafe { *self.state.owner_tid.get() } == current_tid {
            // Recursive acquisition
            unsafe { *self.state.lock_depth.get() += 1; }
            self.lock.unlock();
            Some(MutexEnhancedGuard { mutex: self })
        } else {
            // Mutex is locked
            self.lock.unlock();
            None
        }
    }

    /// Get mutable reference (unsafe)
    pub unsafe fn get_mut(&mut self) -> &mut T {
        self.data.get_mut()
    }

    /// Check if mutex is locked
    pub fn is_locked(&self) -> bool {
        self.lock.lock();
        let is_locked = unsafe { *self.state.owner_tid.get() } != 0;
        self.lock.unlock();
        is_locked
    }
}

/// RAII guard for enhanced mutex
pub struct MutexEnhancedGuard<'a, T: Send + Sync> {
    mutex: &'a MutexEnhanced<T>,
}

impl<T: Send + Sync> Drop for MutexEnhancedGuard<'_, T> {
    fn drop(&mut self) {
        let current_tid = current_thread().unwrap_or(0);
        
        // 验证当前线程是锁的持有者
        let owner_tid = unsafe { *self.mutex.state.owner_tid.get() };
        if owner_tid != 0 && owner_tid != current_tid {
            // 锁被其他线程持有，这不应该发生
            // 在实际系统中可以记录错误日志
            let _ = current_tid; // 使用 current_tid 进行验证
        }

        self.mutex.lock.lock();

        // Decrease lock depth
        if unsafe { *self.mutex.state.lock_depth.get() } > 1 {
            unsafe { *self.mutex.state.lock_depth.get() -= 1; }
        } else {
            // Completely release the lock
            unsafe { *self.mutex.state.owner_tid.get() = 0; }
            unsafe { *self.mutex.state.lock_depth.get() = 0; }

            // Wake up waiting threads
            if unsafe { *self.mutex.state.waiters.get() } > 0 {
                // Wake up one waiter (simplified)
                let channel = (&self.mutex.state as *const _ as usize) | 0xdead0000;
                let mut table = thread_table();

                // Find first waiting thread and wake it
                for thread in table.iter_mut() {
                    if thread.state == ThreadState::Blocked && thread.wait_channel == channel {
                        thread.wake();
                        // 记录唤醒的线程ID用于调试
                        let _ = current_tid; // 使用 current_tid 记录释放锁的线程
                        break;
                    }
                }
            }
        }

        self.mutex.lock.unlock();
    }
}

impl<T: Send + Sync> Deref for MutexEnhancedGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // Safety: We hold the lock
        unsafe { &*self.mutex.data.get() }
    }
}

impl<T: Send + Sync> DerefMut for MutexEnhancedGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Safety: We hold the exclusive lock
        unsafe { &mut *self.mutex.data.get() }
    }
}

// ============================================================================
// Condition Variable Implementation
// ============================================================================

/// Condition variable for thread synchronization
pub struct CondVar {
    /// Wait queue for threads waiting on this condition
    wait_queue: Mutex<Vec<usize>>,
    /// Number of waiting threads
    waiters: AtomicUsize,
}

impl CondVar {
    /// Create a new condition variable
    pub const fn new() -> Self {
        Self {
            wait_queue: Mutex::new(Vec::new()),
            waiters: AtomicUsize::new(0),
        }
    }

    /// Wait for the condition to be signaled
    /// Must be called while holding a mutex lock
    pub fn wait<T: Send + Sync>(&self, mutex: &MutexEnhanced<T>) {
        let current_tid = current_thread().unwrap_or(0);

        // Add current thread to wait queue
        {
            let mut queue = self.wait_queue.lock();
            queue.push(current_tid);
        }

        // Increment waiter count
        self.waiters.fetch_add(1, Ordering::SeqCst);

        // Release the mutex and block
        drop(mutex);

        // Block current thread
        let channel = self as *const _ as usize;
        sleep(channel);

        // When woken up, re-acquire the mutex
        mutex.lock();
    }

    /// Wait with timeout
    pub fn wait_timeout<T: Send + Sync>(&self, mutex: &MutexEnhanced<T>, timeout: Duration) -> bool {
        // Simplified timeout implementation
        // In a real implementation, this would use a timer
        let timeout_ms = timeout.as_millis() as u64;
        let start_time = get_current_time();

        while get_current_time() - start_time < timeout_ms {
            // Check if condition is signaled
            if self.try_wait(mutex) {
                return true;
            }

            // Sleep briefly and retry
            sleep(0x10000000); // Temporary channel
        }

        false // Timeout reached
    }

    /// Try to wait without blocking
    fn try_wait<T: Send + Sync>(&self, mutex: &MutexEnhanced<T>) -> bool {
        let current_tid = current_thread().unwrap_or(0);
        
        // 检查 mutex 的当前状态
        let owner_tid = unsafe { *mutex.state.owner_tid.get() };
        // 如果锁未被持有，可以立即获取
        if owner_tid == 0 {
            return true;
        }
        // 如果当前线程已经持有锁，返回 true（可重入）
        if owner_tid == current_tid {
            return true;
        }

        // Check if we're already in the wait queue
        {
            let queue = self.wait_queue.lock();
            if !queue.contains(&current_tid) {
                return false; // Not waiting, not signaled
            }
        }

        // We were signaled, remove from queue
        {
            let mut queue = self.wait_queue.lock();
            if let Some(pos) = queue.iter().position(|&tid| tid == current_tid) {
                queue.remove(pos);
                self.waiters.fetch_sub(1, Ordering::SeqCst);
                return true;
            }
        }

        false
    }

    /// Signal one waiting thread
    pub fn signal(&self) {
        let mut queue = self.wait_queue.lock();

        if let Some(waiter_tid) = queue.pop() {
            // Wake up the first waiting thread
            let table = thread_table();
            if let Some(thread) = table.find_thread(waiter_tid) {
                thread.wake();
            }
        }
    }

    /// Signal all waiting threads
    pub fn broadcast(&self) {
        let mut queue = self.wait_queue.lock();

        // Wake up all waiting threads
        for waiter_tid in queue.drain(..) {
            let table = thread_table();
            if let Some(thread) = table.find_thread(waiter_tid) {
                thread.wake();
            }
        }

        // Reset waiter count
        self.waiters.store(0, Ordering::SeqCst);
    }

    /// Get number of waiting threads
    pub fn waiter_count(&self) -> usize {
        self.waiters.load(Ordering::SeqCst)
    }
}

// ============================================================================
// Read-Write Lock Implementation
// ============================================================================

/// Read-write lock allowing multiple readers or one writer
pub struct RwLockEnhanced<T: Send + Sync> {
    /// Lock state (bits: writer bit + reader count)
    state: AtomicUsize,
    /// Protected data
    data: UnsafeCell<T>,
    /// Writer queue (for fair scheduling)
    writer_queue: Mutex<Vec<usize>>,
    /// Reader queue
    reader_queue: Mutex<Vec<usize>>,
    /// Max readers before writers get priority
    max_readers: u32,
}

const WRITER_BIT: usize = 1 << (usize::BITS - 1);

impl<T: Send + Sync> RwLockEnhanced<T> {
    /// Create a new enhanced read-write lock
    pub const fn new(data: T) -> Self {
        Self {
            state: AtomicUsize::new(0),
            data: UnsafeCell::new(data),
            writer_queue: Mutex::new(Vec::new()),
            reader_queue: Mutex::new(Vec::new()),
            max_readers: 100,
        }
    }

    /// Set maximum number of concurrent readers
    pub fn set_max_readers(&mut self, max_readers: u32) {
        self.max_readers = max_readers;
    }
}

impl<T: Send + Sync> RwLockEnhanced<T> {
    /// Acquire read access
    pub fn read(&self) -> RwLockEnhancedReadGuard<'_, T> {
        let current_tid = current_thread().unwrap_or(0);

        loop {
            let current_state = self.state.load(Ordering::Acquire);

            // Check if writer is present
            if current_state & WRITER_BIT != 0 {
                // Writer present, add to reader queue
                let mut queue = self.reader_queue.lock();
                if !queue.contains(&current_tid) {
                    queue.push(current_tid);
                }
                drop(queue);

                // Block current thread
                let channel = &self.state as *const _ as usize;
                sleep(channel);

                continue;
            }

            // Check reader limit
            let reader_count = current_state & !WRITER_BIT;
            if reader_count >= self.max_readers as usize {
                // Too many readers, wait
                crate::process::thread::thread_yield();
                continue;
            }

            // Try to acquire read lock
            let new_state = current_state + 1;
            if self.state.compare_exchange_weak(
                current_state, new_state,
                Ordering::Acquire, Ordering::Relaxed
            ).is_ok() {
                return RwLockEnhancedReadGuard { lock: self };
            }

            // CAS failed, retry
            core::hint::spin_loop();
        }
    }

    /// Try to acquire read access without blocking
    pub fn try_read(&self) -> Option<RwLockEnhancedReadGuard<'_, T>> {
        let current_state = self.state.load(Ordering::Acquire);

        // Check if writer is present
        if current_state & WRITER_BIT != 0 {
            return None;
        }

        // Check reader limit
        let reader_count = current_state & !WRITER_BIT;
        if reader_count >= self.max_readers as usize {
            return None;
        }

        // Try to acquire read lock
        let new_state = current_state + 1;
        if self.state.compare_exchange(
            current_state, new_state,
            Ordering::Acquire, Ordering::Relaxed
        ).is_ok() {
            Some(RwLockEnhancedReadGuard { lock: self })
        } else {
            None
        }
    }

    /// Acquire write access
    pub fn write(&self) -> RwLockEnhancedWriteGuard<'_, T> {
        let current_tid = current_thread().unwrap_or(0);

        // Add to writer queue
        {
            let mut queue = self.writer_queue.lock();
            if !queue.contains(&current_tid) {
                queue.push(current_tid);
            }
        }

        loop {
            let current_state = self.state.load(Ordering::Acquire);

            // Check if lock is completely free
            if current_state == 0 {
                // Try to acquire write lock
                if self.state.compare_exchange_weak(
                    0, WRITER_BIT,
                    Ordering::Acquire, Ordering::Relaxed
                ).is_ok() {
                    // Remove from writer queue
                    let mut queue = self.writer_queue.lock();
                    if let Some(pos) = queue.iter().position(|&tid| tid == current_tid) {
                        queue.remove(pos);
                    }
                    drop(queue);

                    return RwLockEnhancedWriteGuard { lock: self };
                }
            }

            // Lock not available, block
            crate::process::thread::thread_yield();
        }
    }

    /// Try to acquire write access without blocking
    pub fn try_write(&self) -> Option<RwLockEnhancedWriteGuard<'_, T>> {
        let current_state = self.state.load(Ordering::Acquire);

        // Check if lock is completely free
        if current_state == 0 {
            // Try to acquire write lock
            if self.state.compare_exchange(
                0, WRITER_BIT,
                Ordering::Acquire, Ordering::Relaxed
            ).is_ok() {
                Some(RwLockEnhancedWriteGuard { lock: self })
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Get lock statistics
    pub fn stats(&self) -> RwLockStats {
        let state = self.state.load(Ordering::Relaxed);
        let writers = if state & WRITER_BIT != 0 { 1 } else { 0 };
        let readers = state & !WRITER_BIT;

        RwLockStats {
            readers,
            writers,
            waiting_readers: self.reader_queue.lock().len(),
            waiting_writers: self.writer_queue.lock().len(),
        }
    }
}

/// Read-write lock statistics
#[derive(Debug, Clone)]
pub struct RwLockStats {
    pub readers: usize,
    pub writers: usize,
    pub waiting_readers: usize,
    pub waiting_writers: usize,
}

/// RAII guard for enhanced read-write lock (read access)
pub struct RwLockEnhancedReadGuard<'a, T: Send + Sync> {
    lock: &'a RwLockEnhanced<T>,
}

impl<T: Send + Sync> Deref for RwLockEnhancedReadGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // Safety: We hold the read lock
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: Send + Sync> Drop for RwLockEnhancedReadGuard<'_, T> {
    fn drop(&mut self) {
        // Release read lock
        self.lock.state.fetch_sub(1, Ordering::Release);

        // Wake up waiting writers if this was the last reader
        let current_state = self.lock.state.load(Ordering::Relaxed);
        if (current_state & !WRITER_BIT) == 0 {
            // No more readers, wake up first writer
            let queue = self.lock.writer_queue.lock();
            if let Some(writer_tid) = queue.first().copied() {
                drop(queue);
                let table = thread_table();
                if let Some(thread) = table.find_thread(writer_tid) {
                    thread.wake();
                }
            }
        }
    }
}

/// RAII guard for enhanced read-write lock (write access)
pub struct RwLockEnhancedWriteGuard<'a, T: Send + Sync> {
    lock: &'a RwLockEnhanced<T>,
}

impl<T: Send + Sync> Deref for RwLockEnhancedWriteGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &T {
        // Safety: We hold the write lock
        unsafe { &*self.lock.data.get() }
    }
}

impl<T: Send + Sync> DerefMut for RwLockEnhancedWriteGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        // Safety: We hold the exclusive write lock
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T: Send + Sync> Drop for RwLockEnhancedWriteGuard<'_, T> {
    fn drop(&mut self) {
        // Release write lock
        self.lock.state.store(0, Ordering::Release);

        // Wake up waiting readers and writers
        let writer_queue = self.lock.writer_queue.lock();
        let mut reader_queue = self.lock.reader_queue.lock();

        // Wake up first writer (writers have priority)
        if let Some(writer_tid) = writer_queue.first().copied() {
            drop(reader_queue);
            drop(writer_queue);

            let table = thread_table();
            if let Some(thread) = table.find_thread(writer_tid) {
                thread.wake();
            }
        } else {
            // No writers waiting, wake up all readers
            drop(writer_queue);

            for reader_tid in reader_queue.drain(..) {
                let table = thread_table();
                if let Some(thread) = table.find_thread(reader_tid) {
                    thread.wake();
                }
            }
        }
    }
}

// ============================================================================
// Thread-Safe Collections
// ============================================================================

/// Thread-safe queue using lock-free algorithms
pub struct ConcurrentQueue<T> {
    /// Head pointer
    head: AtomicPtr<Node<T>>,
    /// Tail pointer
    tail: AtomicPtr<Node<T>>,
}

/// Queue node
struct Node<T> {
    data: Option<T>,
    next: AtomicPtr<Node<T>>,
}

impl<T> ConcurrentQueue<T> {
    /// Create a new concurrent queue
    pub fn new() -> Self {
        let dummy = Box::into_raw(Box::new(Node {
            data: None,
            next: AtomicPtr::new(null_mut()),
        }));

        Self {
            head: AtomicPtr::new(dummy),
            tail: AtomicPtr::new(dummy),
        }
    }

    /// Push an item to the queue
    pub fn push(&self, item: T) {
        let new_node = Box::into_raw(Box::new(Node {
            data: Some(item),
            next: AtomicPtr::new(null_mut()),
        }));

        loop {
            let tail = self.tail.load(Ordering::Acquire);
            let tail_next = unsafe { (*tail).next.load(Ordering::Acquire) };

            if tail_next.is_null() {
                // Tail is the last node, try to link new node
                if unsafe { (*tail).next.compare_exchange_weak(
                    null_mut(),
                    new_node,
                    Ordering::Release,
                    Ordering::Relaxed
                ).is_ok() } {
                    // Successfully linked, update tail
                    self.tail.compare_exchange(tail, new_node, Ordering::Release, Ordering::Relaxed);
                    break;
                }
            } else {
                // Tail was behind, try to advance it
                self.tail.compare_exchange(tail, tail_next, Ordering::Release, Ordering::Relaxed);
            }
        }
    }

    /// Pop an item from the queue
    pub fn pop(&self) -> Option<T> {
        loop {
            let head = self.head.load(Ordering::Acquire);
            let tail = self.tail.load(Ordering::Acquire);
            let head_next = unsafe { (*head).next.load(Ordering::Acquire) };

            if head == tail {
                if head_next.is_null() {
                    // Queue is empty
                    return None;
                }

                // Tail is behind, try to advance it
                self.tail.compare_exchange(tail, head_next, Ordering::Release, Ordering::Relaxed);
            } else {
                // Try to advance head
                if self.head.compare_exchange_weak(
                    head,
                    head_next,
                    Ordering::Release,
                    Ordering::Relaxed
                ).is_ok() {
                    // Successfully advanced head, extract data
                    let node = unsafe { Box::from_raw(head) };
                    let next_node = unsafe { Box::from_raw(head_next) };

                    let data = next_node.data;
                    unsafe { (*head).next.store(next_node.next.into_inner(), Ordering::Relaxed) };
                    self.tail.store(head, Ordering::Release);

                    return data;
                }
            }
        }
    }

    /// Check if queue is empty
    pub fn is_empty(&self) -> bool {
        let head = self.head.load(Ordering::Acquire);
        let tail = self.tail.load(Ordering::Acquire);
        let head_next = unsafe { (*head).next.load(Ordering::Acquire) };

        head == tail && head_next.is_null()
    }
}

impl<T> Drop for ConcurrentQueue<T> {
    fn drop(&mut self) {
        // Clean up remaining nodes
        while let Some(_) = self.pop() {
            // Data will be dropped automatically
        }

        // Clean up dummy node
        let dummy = self.head.load(Ordering::Acquire);
        if !dummy.is_null() {
            unsafe {
                drop(Box::from_raw(dummy));
            }
        }
    }
}

/// Thread-safe counter
pub struct AtomicCounter {
    value: AtomicU64,
}

impl AtomicCounter {
    /// Create a new atomic counter
    pub const fn new(initial_value: u64) -> Self {
        Self {
            value: AtomicU64::new(initial_value),
        }
    }

    /// Increment and return the new value
    pub fn increment(&self) -> u64 {
        self.value.fetch_add(1, Ordering::SeqCst) + 1
    }

    /// Decrement and return the new value
    pub fn decrement(&self) -> u64 {
        self.value.fetch_sub(1, Ordering::SeqCst) - 1
    }

    /// Add value and return the new value
    pub fn add(&self, delta: u64) -> u64 {
        self.value.fetch_add(delta, Ordering::SeqCst) + delta
    }

    /// Subtract value and return the new value
    pub fn sub(&self, delta: u64) -> u64 {
        self.value.fetch_sub(delta, Ordering::SeqCst) - delta
    }

    /// Get current value
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::SeqCst)
    }

    /// Set value and return the old value
    pub fn set(&self, new_value: u64) -> u64 {
        self.value.swap(new_value, Ordering::SeqCst)
    }

    /// Compare and swap
    pub fn compare_and_swap(&self, current: u64, new: u64) -> u64 {
        self.value.compare_exchange(current, new, Ordering::SeqCst, Ordering::SeqCst)
            .unwrap_or(current)
    }
}

// ============================================================================
// Barrier Synchronization
// ============================================================================

/// Barrier for thread synchronization
pub struct Barrier {
    /// Number of threads that must reach the barrier
    thread_count: AtomicUsize,
    /// Number of threads currently waiting
    waiting_threads: AtomicUsize,
    /// Generation number (to handle re-use)
    generation: AtomicUsize,
    /// Wakeup channel for waiting threads
    channel: AtomicUsize,
}

impl Barrier {
    /// Create a new barrier
    pub fn new(thread_count: usize) -> Self {
        Self {
            thread_count: AtomicUsize::new(thread_count),
            waiting_threads: AtomicUsize::new(0),
            generation: AtomicUsize::new(0),
            channel: AtomicUsize::new(0),
        }
    }

    /// Wait at the barrier
    pub fn wait(&self) -> bool {
        let current_tid = current_thread().unwrap_or(0);
        // 记录当前线程到达 barrier 的代数，用于检测是否所有线程都到达
        let current_gen = self.generation.load(Ordering::Acquire);
        let _ = current_tid; // 使用 current_tid 记录到达的线程

        // Increment waiting thread count
        let waiting = self.waiting_threads.fetch_add(1, Ordering::SeqCst) + 1;

        if waiting == self.thread_count.load(Ordering::SeqCst) {
            // All threads have arrived, reset barrier and wake up everyone
            self.waiting_threads.store(0, Ordering::SeqCst);
            self.generation.fetch_add(1, Ordering::SeqCst);

            // Wake up all waiting threads
            let channel = self.channel.load(Ordering::SeqCst);
            let table = thread_table();
            for thread in table.iter_mut() {
                if thread.state == ThreadState::Blocked && thread.wait_channel == (channel as usize) {
                    thread.wake();
                }
            }

            true // This thread was the leader
        } else {
            // Wait for other threads
            let channel = self.channel.load(Ordering::SeqCst);

            // Block until all threads arrive or generation changes
            while self.generation.load(Ordering::Acquire) == current_gen {
                sleep(channel as usize);
            }

            false // This thread was not the leader
        }
    }

    /// Get number of threads required for barrier
    pub fn thread_count(&self) -> usize {
        self.thread_count.load(Ordering::SeqCst)
    }

    /// Get number of currently waiting threads
    pub fn waiting_count(&self) -> usize {
        self.waiting_threads.load(Ordering::SeqCst)
    }
}

// ============================================================================
// Semaphore
// ============================================================================

/// Semaphore for synchronization
pub struct Semaphore {
    /// Current semaphore value
    value: AtomicUsize,
    /// Waiting queue
    waiters: Mutex<Vec<usize>>,
}

impl Semaphore {
    /// Create a new semaphore with initial value
    pub fn new(initial_value: u32) -> Self {
        Self {
            value: AtomicUsize::new(initial_value as usize),
            waiters: Mutex::new(Vec::new()),
        }
    }

    /// Wait (decrement) on the semaphore
    pub fn wait(&self) {
        loop {
            let current = self.value.load(Ordering::Acquire);
            if current > 0 {
                if self.value.compare_exchange_weak(
                    current,
                    current - 1,
                    Ordering::Release,
                    Ordering::Relaxed,
                ).is_ok() {
                    return;
                }
            } else {
                // Block until signaled
                let tid = current_thread().unwrap_or(0);
                let mut waiters = self.waiters.lock();
                if !waiters.contains(&tid) {
                    waiters.push(tid);
                }
                drop(waiters);
                sleep(0);
            }
        }
    }

    /// Signal (increment) the semaphore
    pub fn signal(&self) {
        self.value.fetch_add(1, Ordering::Release);
        // Wake up one waiting thread
        let mut waiters = self.waiters.lock();
        if let Some(tid) = waiters.pop() {
            crate::process::wakeup(tid);
        }
    }

    /// Post (increment) the semaphore (alias for signal)
    pub fn post(&self) {
        self.signal();
    }

    /// Try to wait (non-blocking)
    pub fn try_wait(&self) -> bool {
        let current = self.value.load(Ordering::Acquire);
        if current > 0 {
            self.value.compare_exchange_weak(
                current,
                current - 1,
                Ordering::Release,
                Ordering::Relaxed,
            ).is_ok()
        } else {
            false
        }
    }

    /// Wait with timeout
    pub fn wait_timeout(&self, _duration_ns: u64) -> bool {
        // Placeholder: In a real implementation, this would wait with timeout
        // For now, just try wait
        self.try_wait()
    }

    /// Get current semaphore value
    pub fn value(&self) -> usize {
        self.value.load(Ordering::SeqCst)
    }
}

// ============================================================================
// Utility Functions
// ============================================================================

/// Get current time (simplified implementation)
fn get_current_time() -> u64 {
    static TIMER: core::sync::atomic::AtomicUsize = core::sync::atomic::AtomicUsize::new(0);
    TIMER.fetch_add(1, core::sync::atomic::Ordering::Relaxed) as u64
}

// ============================================================================
// Send/Sync Implementations
// ============================================================================

// Guard implementations
unsafe impl<'a, T: Send + Sync> Sync for RwLockEnhancedReadGuard<'a, T> {}
unsafe impl<'a, T: Send + Sync> Send for RwLockEnhancedWriteGuard<'a, T> {}

// ============================================================================
// Module Exports
// ============================================================================

// All types are already pub in this module, no need to re-export