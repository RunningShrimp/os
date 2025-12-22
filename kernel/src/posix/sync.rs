//! POSIX-compatible synchronization primitives
//!
//! This module provides POSIX-compliant versions of mutexes,
//! condition variables, and read-write locks for pthread API.

extern crate alloc;

use crate::subsystems::sync::primitives::{MutexEnhanced, CondVar, RwLockEnhanced};
use core::ptr::null_mut;
use core::cell::UnsafeCell;
use alloc::boxed::Box;
use crate::reliability::errno::{EOK, EINVAL, EPERM, EBUSY, EDEADLK, ETIMEDOUT};

// ============================================================================
// POSIX Mutex Types
// ============================================================================

/// POSIX mutex type (opaque pointer)
pub type PthreadMutexT = *mut PthreadMutexInternal;

/// Internal mutex structure
#[repr(C)]
pub struct PthreadMutexInternal {
    /// Enhanced mutex implementation
    mutex: MutexEnhanced<u8>,
    /// Mutex attributes
    kind: PthreadMutexKind,
    /// Owner thread ID for deadlock detection
    owner_tid: UnsafeCell<u64>,
    /// Lock depth for recursive mutexes
    depth: UnsafeCell<u32>,
    /// Mutex is initialized
    initialized: bool,
}

/// POSIX mutex types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PthreadMutexKind {
    Normal = 0,
    Recursive = 1,
    ErrorChecking = 2,
    Default = 3, // Different value to avoid conflict
}

/// POSIX mutex attribute types
#[repr(C)]
pub enum PthreadMutexAttrT {
    Normal = 0,
    Recursive = 1,
    ErrorChecking = 2,
    Default = 3, // Different value to avoid conflict
}

/// POSIX mutex constants
pub const PTHREAD_MUTEX_NORMAL: i32 = 0;
pub const PTHREAD_MUTEX_RECURSIVE: i32 = 1;
pub const PTHREAD_MUTEX_ERRORCHECK: i32 = 2;
pub const PTHREAD_MUTEX_DEFAULT: i32 = PTHREAD_MUTEX_NORMAL;

/// POSIX error codes
pub const PTHREAD_MUTEX_INITIALIZER: PthreadMutexT = null_mut();
pub const PTHREAD_COND_INITIALIZER: PthreadCondT = null_mut();

/// Create a default mutex initializer
pub const fn pthread_mutex_initializer() -> PthreadMutexT {
    null_mut()
}

// ============================================================================
// POSIX Condition Variable Types
// ============================================================================

/// POSIX condition variable type (opaque pointer)
pub type PthreadCondT = *mut PthreadCondInternal;

/// Internal condition variable structure
#[repr(C)]
pub struct PthreadCondInternal {
    /// Enhanced condition variable implementation
    condvar: CondVar,
    /// Clock ID for timed waits
    clock_id: i32,
    /// Condition variable attributes
    attrs: PthreadCondAttrT,
    /// Is initialized
    initialized: bool,
}

/// POSIX condition variable attribute types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PthreadCondAttrT {
    pub clock: i32,
}

/// POSIX condition variable constants
pub const PTHREAD_COND_NORMAL: i32 = 0;

// ============================================================================
// POSIX Read-Write Lock Types
// ============================================================================

/// POSIX read-write lock type (opaque pointer)
pub type PthreadRwlockT = *mut PthreadRwlockInternal;

/// Internal read-write lock structure
#[repr(C)]
pub struct PthreadRwlockInternal {
    /// Enhanced read-write lock implementation
    rwlock: RwLockEnhanced<u8>,
    /// Number of readers
    readers: core::cell::UnsafeCell<u32>,
    /// Number of writers waiting
    writer_waiters: core::cell::UnsafeCell<u32>,
    /// Owner thread ID
    owner_tid: core::cell::UnsafeCell<u64>,
    /// Is initialized
    initialized: bool,
}

/// POSIX read-write lock attribute types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PthreadRwlockAttrT {
    pub kind: PthreadRwlockKind,
}

/// POSIX read-write lock kinds
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PthreadRwlockKind {
    Default = 0,
    ReaderPreferred = 1,
    WriterPreferred = 2,
}

/// POSIX read-write lock constants
pub const PTHREAD_RWLOCK_INITIALIZER: PthreadRwlockT = null_mut();
pub const PTHREAD_RWLOCK_PREFER_READER_NP: i32 = 0;
pub const PTHREAD_RWLOCK_PREFER_WRITER_NP: i32 = 1;
pub const PTHREAD_RWLOCK_PREFER_WRITER_NONRECURSIVE_NP: i32 = 2;

// ============================================================================
// POSIX Barrier Types
// ============================================================================

/// POSIX barrier type (opaque pointer)
pub type PthreadBarrierT = *mut PthreadBarrierInternal;

/// Internal barrier structure
#[repr(C)]
pub struct PthreadBarrierInternal {
    /// Enhanced barrier implementation
    barrier: crate::subsystems::sync::primitives::Barrier,
    /// Barrier attributes
    attrs: PthreadBarrierAttrT,
    /// Is initialized
    initialized: bool,
}

/// POSIX barrier attribute types
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PthreadBarrierAttrT {
    pub pshared: i32,
}

/// POSIX barrier constants
pub const PTHREAD_BARRIER_SERIAL_THREAD: i32 = -1;

// ============================================================================
// POSIX Mutex Functions
// ============================================================================

/// Initialize a mutex
pub unsafe extern "C" fn pthread_mutex_init(
    mutex: *mut PthreadMutexT,
    attr: *const PthreadMutexAttrT,
) -> i32 {
    if mutex.is_null() {
        return EINVAL;
    }

    // Determine mutex kind from attributes
    let kind = if attr.is_null() {
        PthreadMutexKind::Default
    } else {
        match *attr {
            PthreadMutexAttrT::Normal => PthreadMutexKind::Normal,
            PthreadMutexAttrT::Recursive => PthreadMutexKind::Recursive,
            PthreadMutexAttrT::ErrorChecking => PthreadMutexKind::ErrorChecking,
            PthreadMutexAttrT::Default => PthreadMutexKind::Default,
        }
    };

    // Create mutex based on kind
    let internal = match kind {
        PthreadMutexKind::Recursive => {
            Box::into_raw(Box::new(PthreadMutexInternal {
                mutex: MutexEnhanced::new_recursive(0),
                kind,
                owner_tid: UnsafeCell::new(0),
                depth: UnsafeCell::new(0),
                initialized: true,
            }))
        }
        _ => {
            Box::into_raw(Box::new(PthreadMutexInternal {
                mutex: MutexEnhanced::new(0),
                kind,
                owner_tid: UnsafeCell::new(0),
                depth: UnsafeCell::new(0),
                initialized: true,
            }))
        }
    };

    *mutex = internal;
    EOK
}

/// Destroy a mutex
pub unsafe extern "C" fn pthread_mutex_destroy(mutex: *mut PthreadMutexT) -> i32 {
    if mutex.is_null() {
        return EINVAL;
    }

    let internal = *mutex;
    if internal.is_null() {
        return EINVAL;
    }

    let mutex_ref = &*internal;

    // Check if mutex is locked
    if mutex_ref.mutex.is_locked() {
        return EBUSY;
    }

    // Free the mutex
    drop(Box::from_raw(internal));
    *mutex = null_mut();

    EOK
}

/// Lock a mutex
pub unsafe extern "C" fn pthread_mutex_lock(mutex: PthreadMutexT) -> i32 {
    if mutex.is_null() {
        return EINVAL;
    }

    let internal = &*mutex;
    if !internal.initialized {
        return EINVAL;
    }

    // Check for deadlock in error-checking mode
    if internal.kind == PthreadMutexKind::ErrorChecking {
        let current_tid = crate::process::thread::current_thread().unwrap_or(0);
        if *internal.owner_tid.get() == current_tid as u64 {
            return EDEADLK;
        }
    }

    // Acquire the mutex
    match internal.kind {
        PthreadMutexKind::Normal | PthreadMutexKind::ErrorChecking => {
            let _guard = internal.mutex.lock();
            *internal.owner_tid.get() = crate::process::thread::current_thread().unwrap_or(0) as u64;
            *internal.depth.get() = 1;
        }
        PthreadMutexKind::Recursive => {
            let current_tid = crate::process::thread::current_thread().unwrap_or(0);
            if *internal.owner_tid.get() == current_tid as u64 {
                // Recursive acquisition
                *internal.depth.get() += 1;
            } else {
                let _guard = internal.mutex.lock();
                *internal.owner_tid.get() = current_tid as u64;
                *internal.depth.get() = 1;
            }
        }
        PthreadMutexKind::Default => {
            let _guard = internal.mutex.lock();
            *internal.owner_tid.get() = crate::process::thread::current_thread().unwrap_or(0) as u64;
            *internal.depth.get() = 1;
        }
    }

    EOK
}

/// Try to lock a mutex
pub unsafe extern "C" fn pthread_mutex_trylock(mutex: PthreadMutexT) -> i32 {
    if mutex.is_null() {
        return EINVAL;
    }

    let internal = &*mutex;
    if !internal.initialized {
        return EINVAL;
    }

    // Try to acquire the mutex
    match internal.kind {
        PthreadMutexKind::Normal | PthreadMutexKind::ErrorChecking => {
            if let Some(_guard) = internal.mutex.try_lock() {
                *internal.owner_tid.get() = crate::process::thread::current_thread().unwrap_or(0) as u64;
                *internal.depth.get() = 1;
                EOK
            } else {
                EBUSY
            }
        }
        PthreadMutexKind::Recursive => {
            let current_tid = crate::process::thread::current_thread().unwrap_or(0);
            if *internal.owner_tid.get() == current_tid as u64 {
                // Recursive acquisition always succeeds
                *internal.depth.get() += 1;
                EOK
            } else if let Some(_guard) = internal.mutex.try_lock() {
                *internal.owner_tid.get() = current_tid as u64;
                *internal.depth.get() = 1;
                EOK
            } else {
                EBUSY
            }
        }
        PthreadMutexKind::Default => {
            if let Some(_guard) = internal.mutex.try_lock() {
                *internal.owner_tid.get() = crate::process::thread::current_thread().unwrap_or(0) as u64;
                *internal.depth.get() = 1;
                EOK
            } else {
                EBUSY
            }
        }
    }
}

/// Unlock a mutex
pub unsafe extern "C" fn pthread_mutex_unlock(mutex: PthreadMutexT) -> i32 {
    if mutex.is_null() {
        return EINVAL;
    }

    let internal = &*mutex;
    if !internal.initialized {
        return EINVAL;
    }

    let current_tid = crate::process::thread::current_thread().unwrap_or(0);

    // Check if current thread owns the mutex
    if unsafe { *internal.owner_tid.get() } != current_tid as u64 {
        return EPERM;
    }

    // Handle recursive unlock
    if unsafe { *internal.depth.get() } > 1 {
        unsafe { *internal.depth.get() -= 1; }
        return EOK;
    }

    // Full unlock
    unsafe { *internal.owner_tid.get() = 0; }
    unsafe { *internal.depth.get() = 0; }
    // Mutex is automatically unlocked when the guard goes out of scope

    EOK
}

// ============================================================================
// POSIX Mutex Attribute Functions
// ============================================================================

/// Initialize mutex attributes
pub unsafe extern "C" fn pthread_mutexattr_init(
    attr: *mut PthreadMutexAttrT,
) -> i32 {
    if attr.is_null() {
        return EINVAL;
    }

    *attr = PthreadMutexAttrT::Default;
    EOK
}

/// Destroy mutex attributes
pub unsafe extern "C" fn pthread_mutexattr_destroy(
    attr: *mut PthreadMutexAttrT,
) -> i32 {
    if attr.is_null() {
        return EINVAL;
    }

    // Nothing to clean up
    EOK
}

/// Set mutex type
pub unsafe extern "C" fn pthread_mutexattr_settype(
    attr: *mut PthreadMutexAttrT,
    kind: i32,
) -> i32 {
    if attr.is_null() {
        return EINVAL;
    }

    let mutex_kind = match kind {
        PTHREAD_MUTEX_NORMAL => PthreadMutexAttrT::Normal,
        PTHREAD_MUTEX_RECURSIVE => PthreadMutexAttrT::Recursive,
        PTHREAD_MUTEX_ERRORCHECK => PthreadMutexAttrT::ErrorChecking,
        _ => return EINVAL,
    };

    *attr = mutex_kind;
    EOK
}

// ============================================================================
// POSIX Condition Variable Functions
// ============================================================================

/// Initialize a condition variable
pub unsafe extern "C" fn pthread_cond_init(
    cond: *mut PthreadCondT,
    attr: *const PthreadCondAttrT,
) -> i32 {
    if cond.is_null() {
        return EINVAL;
    }

    let attrs = if attr.is_null() {
        PthreadCondAttrT { clock: 0 } // CLOCK_REALTIME
    } else {
        *attr
    };

    let internal = Box::into_raw(Box::new(PthreadCondInternal {
        condvar: CondVar::new(),
        clock_id: attrs.clock,
        attrs,
        initialized: true,
    }));

    *cond = internal;
    EOK
}

/// Destroy a condition variable
pub unsafe extern "C" fn pthread_cond_destroy(cond: *mut PthreadCondT) -> i32 {
    if cond.is_null() {
        return EINVAL;
    }

    let internal = *cond;
    if internal.is_null() {
        return EINVAL;
    }

    let cond_ref = &*internal;
    if !cond_ref.initialized {
        return EINVAL;
    }

    // Check if any threads are waiting
    if cond_ref.condvar.waiter_count() > 0 {
        return EBUSY;
    }

    // Free the condition variable
    drop(Box::from_raw(internal));
    *cond = null_mut();

    EOK
}

/// Wait on a condition variable
pub unsafe extern "C" fn pthread_cond_wait(
    cond: PthreadCondT,
    mutex: PthreadMutexT,
) -> i32 {
    if cond.is_null() || mutex.is_null() {
        return EINVAL;
    }

    let cond_ref = &*cond;
    let mutex_ref = &*mutex;

    if !cond_ref.initialized || !mutex_ref.initialized {
        return EINVAL;
    }

    // Use the enhanced condition variable
    cond_ref.condvar.wait(&mutex_ref.mutex);
    EOK
}

/// Wait on a condition variable with timeout
pub unsafe extern "C" fn pthread_cond_timedwait(
    cond: PthreadCondT,
    mutex: PthreadMutexT,
    abstime: *const crate::posix::Timespec,
) -> i32 {
    if cond.is_null() || mutex.is_null() || abstime.is_null() {
        return EINVAL;
    }

    let cond_ref = &*cond;
    let mutex_ref = &*mutex;

    if !cond_ref.initialized || !mutex_ref.initialized {
        return EINVAL;
    }

    // Convert timespec to Duration (simplified)
    let timeout = core::time::Duration::from_secs((*abstime).tv_sec as u64)
        + core::time::Duration::from_nanos((*abstime).tv_nsec as u64);

    // Use the enhanced condition variable with timeout
    if cond_ref.condvar.wait_timeout(&mutex_ref.mutex, timeout) {
        EOK
    } else {
        ETIMEDOUT
    }
}

/// Signal one waiting thread
pub unsafe extern "C" fn pthread_cond_signal(cond: PthreadCondT) -> i32 {
    if cond.is_null() {
        return EINVAL;
    }

    let cond_ref = &*cond;
    if !cond_ref.initialized {
        return EINVAL;
    }

    cond_ref.condvar.signal();
    EOK
}

/// Broadcast to all waiting threads
pub unsafe extern "C" fn pthread_cond_broadcast(cond: PthreadCondT) -> i32 {
    if cond.is_null() {
        return EINVAL;
    }

    let cond_ref = &*cond;
    if !cond_ref.initialized {
        return EINVAL;
    }

    cond_ref.condvar.broadcast();
    EOK
}

// ============================================================================
// POSIX Condition Variable Attribute Functions
// ============================================================================

/// Initialize condition variable attributes
pub unsafe extern "C" fn pthread_condattr_init(
    attr: *mut PthreadCondAttrT,
) -> i32 {
    if attr.is_null() {
        return EINVAL;
    }

    *attr = PthreadCondAttrT { clock: 0 };
    EOK
}

/// Destroy condition variable attributes
pub unsafe extern "C" fn pthread_condattr_destroy(
    attr: *mut PthreadCondAttrT,
) -> i32 {
    if attr.is_null() {
        return EINVAL;
    }

    // Nothing to clean up
    EOK
}

/// Set clock for condition variable
pub unsafe extern "C" fn pthread_condattr_setclock(
    attr: *mut PthreadCondAttrT,
    clock: i32,
) -> i32 {
    if attr.is_null() {
        return EINVAL;
    }

    (*attr).clock = clock;
    EOK
}

// ============================================================================
// POSIX Read-Write Lock Functions
// ============================================================================

/// Initialize a read-write lock
pub unsafe extern "C" fn pthread_rwlock_init(
    rwlock: *mut PthreadRwlockT,
    attr: *const PthreadRwlockAttrT,
) -> i32 {
    if rwlock.is_null() {
        return EINVAL;
    }

    let attrs = if attr.is_null() {
        PthreadRwlockAttrT { kind: PthreadRwlockKind::Default }
    } else {
        *attr
    };

    // Configure lock based on attributes
    let mut lock = RwLockEnhanced::new(0);

    match attrs.kind {
        PthreadRwlockKind::ReaderPreferred => {
            lock.set_max_readers(100); // Allow more readers
        }
        PthreadRwlockKind::WriterPreferred => {
            lock.set_max_readers(10); // Prioritize writers
        }
        PthreadRwlockKind::Default => {
            lock.set_max_readers(50); // Balanced approach
        }
    }

    let internal = Box::into_raw(Box::new(PthreadRwlockInternal {
        rwlock: lock,
        readers: core::cell::UnsafeCell::new(0),
        writer_waiters: core::cell::UnsafeCell::new(0),
        owner_tid: core::cell::UnsafeCell::new(0),
        initialized: true,
    }));

    *rwlock = internal;
    EOK
}

/// Destroy a read-write lock
pub unsafe extern "C" fn pthread_rwlock_destroy(rwlock: *mut PthreadRwlockT) -> i32 {
    if rwlock.is_null() {
        return EINVAL;
    }

    let internal = *rwlock;
    if internal.is_null() {
        return EINVAL;
    }

    let rwlock_ref = &*internal;
    if !rwlock_ref.initialized {
        return EINVAL;
    }

    // Check if lock is in use
    let stats = rwlock_ref.rwlock.stats();
    if stats.readers > 0 || stats.writers > 0 {
        return EBUSY;
    }

    // Free the read-write lock
    drop(Box::from_raw(internal));
    *rwlock = null_mut();

    EOK
}

/// Acquire read lock
pub unsafe extern "C" fn pthread_rwlock_rdlock(rwlock: PthreadRwlockT) -> i32 {
    if rwlock.is_null() {
        return EINVAL;
    }

    let rwlock_ref = &*rwlock;
    if !rwlock_ref.initialized {
        return EINVAL;
    }

    // Use enhanced read-write lock
    let _guard = rwlock_ref.rwlock.read();
    unsafe { *rwlock_ref.readers.get() += 1; }

    EOK
}

/// Try to acquire read lock
pub unsafe extern "C" fn pthread_rwlock_tryrdlock(rwlock: PthreadRwlockT) -> i32 {
    if rwlock.is_null() {
        return EINVAL;
    }

    let rwlock_ref = &*rwlock;
    if !rwlock_ref.initialized {
        return EINVAL;
    }

    // Try to acquire read lock
    if let Some(_guard) = rwlock_ref.rwlock.try_read() {
        unsafe { *rwlock_ref.readers.get() += 1};
        EOK
    } else {
        EBUSY
    }
}

/// Acquire write lock
pub unsafe extern "C" fn pthread_rwlock_wrlock(rwlock: PthreadRwlockT) -> i32 {
    if rwlock.is_null() {
        return EINVAL;
    }

    let rwlock_ref = &*rwlock;
    if !rwlock_ref.initialized {
        return EINVAL;
    }

    // Use enhanced read-write lock
    let _guard = rwlock_ref.rwlock.write();
    unsafe { *rwlock_ref.owner_tid.get() = crate::process::thread::current_thread().unwrap_or(0) as u64}

    EOK
}

/// Try to acquire write lock
pub unsafe extern "C" fn pthread_rwlock_trywrlock(rwlock: PthreadRwlockT) -> i32 {
    if rwlock.is_null() {
        return EINVAL;
    }

    let rwlock_ref = &*rwlock;
    if !rwlock_ref.initialized {
        return EINVAL;
    }

    // Try to acquire write lock
    if let Some(_guard) = rwlock_ref.rwlock.try_write() {
        unsafe { *rwlock_ref.owner_tid.get() = crate::process::thread::current_thread().unwrap_or(0) as u64}
        EOK
    } else {
        EBUSY
    }
}

/// Release read lock
pub unsafe extern "C" fn pthread_rwlock_unlock(rwlock: PthreadRwlockT) -> i32 {
    if rwlock.is_null() {
        return EINVAL;
    }

    let rwlock_ref = &*rwlock;
    if !rwlock_ref.initialized {
        return EINVAL;
    }

    let current_tid = crate::process::thread::current_thread().unwrap_or(0);

    // Determine if we're unlocking a read or write lock
    if unsafe { *rwlock_ref.owner_tid.get() == current_tid as u64 } {
        // Unlocking write lock
        unsafe { *rwlock_ref.owner_tid.get() = 0}
        // Write lock guard is automatically dropped
    } else {
        // Unlocking read lock
        if unsafe { *rwlock_ref.readers.get() > 0 } {
            unsafe { *rwlock_ref.readers.get() -= 1};
            // Read lock guard is automatically dropped
        }
    }

    EOK
}

// ============================================================================
// POSIX Barrier Functions
// ============================================================================

/// Initialize a barrier
pub unsafe extern "C" fn pthread_barrier_init(
    barrier: *mut PthreadBarrierT,
    attr: *const PthreadBarrierAttrT,
    count: u32,
) -> i32 {
    if barrier.is_null() || count == 0 {
        return EINVAL;
    }

    let attrs = if attr.is_null() {
        PthreadBarrierAttrT { pshared: 0 } // Private to process
    } else {
        *attr
    };

    let internal = Box::into_raw(Box::new(PthreadBarrierInternal {
        barrier: crate::subsystems::sync::primitives::Barrier::new(count as usize),
        attrs,
        initialized: true,
    }));

    *barrier = internal;
    EOK
}

/// Destroy a barrier
pub unsafe extern "C" fn pthread_barrier_destroy(barrier: *mut PthreadBarrierT) -> i32 {
    if barrier.is_null() {
        return EINVAL;
    }

    let internal = *barrier;
    if internal.is_null() {
        return EINVAL;
    }

    let barrier_ref = &*internal;
    if !barrier_ref.initialized {
        return EINVAL;
    }

    // Free the barrier
    drop(Box::from_raw(internal));
    *barrier = null_mut();

    EOK
}

/// Wait at a barrier
pub unsafe extern "C" fn pthread_barrier_wait(barrier: PthreadBarrierT) -> i32 {
    if barrier.is_null() {
        return EINVAL;
    }

    let barrier_ref = &*barrier;
    if !barrier_ref.initialized {
        return EINVAL;
    }

    if barrier_ref.barrier.wait() {
        PTHREAD_BARRIER_SERIAL_THREAD
    } else {
        0
    }
}