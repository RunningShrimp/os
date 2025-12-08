//! POSIX Semaphore (semaphore.h) Implementation
//!
//! Implements POSIX named and unnamed semaphores for synchronization
//! between processes and threads.

extern crate alloc;

use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use alloc::boxed::Box;
use crate::sync::Mutex;
use crate::reliability::errno::{EOK, EINVAL, ENOENT, EAGAIN};
use crate::posix::SemT;

/// Semaphore descriptor
struct SemaphoreDescriptor {
    /// Semaphore name (for named semaphores)
    name: Option<alloc::string::String>,
    /// Internal semaphore implementation
    internal: Arc<crate::sync::primitives::Semaphore>,
    /// Reference count
    ref_count: core::sync::atomic::AtomicUsize,
    /// Process permissions
    mode: u32,
    /// Owner UID
    uid: crate::posix::Uid,
    /// Creator process
    creator_pid: crate::posix::Pid,
}

impl core::fmt::Debug for SemaphoreDescriptor {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SemaphoreDescriptor")
            .field("name", &self.name)
            .field("ref_count", &self.ref_count.load(core::sync::atomic::Ordering::Relaxed))
            .field("mode", &self.mode)
            .field("uid", &self.uid)
            .field("creator_pid", &self.creator_pid)
            .finish()
    }
}

/// Global named semaphore registry
static NAMED_SEMAPHORES: Mutex<BTreeMap<alloc::string::String, Arc<SemaphoreDescriptor>>> =
    Mutex::new(BTreeMap::new());

/// Maximum number of named semaphores
const MAX_NAMED_SEMAPHORES: usize = 256;

/// Maximum semaphore name length
const SEM_NAME_MAX: usize = 251;

// ============================================================================
// Unnamed Semaphore Functions
// ============================================================================

/// Initialize an unnamed semaphore
///
/// # Arguments
/// * `sem` - Pointer to semaphore to initialize
/// * `pshared` - Non-zero if semaphore is shared between processes
/// * `value` - Initial semaphore value
///
/// # Returns
/// * 0 on success, error code on failure
pub unsafe extern "C" fn sem_init(
    sem: *mut SemT,
    pshared: i32,
    value: u32,
) -> i32 {
    if sem.is_null() {
        return EINVAL;
    }

    // Create semaphore with initial value
    let internal = crate::sync::primitives::Semaphore::new(value);
    let semaphore = Box::into_raw(Box::new(SemaphoreDescriptor {
        name: None,
        internal: Arc::new(internal),
        ref_count: core::sync::atomic::AtomicUsize::new(1),
        mode: 0,
        uid: crate::process::getuid(),
        creator_pid: crate::process::getpid() as crate::posix::Pid,
    }));

    *sem = SemT { sem_internal: semaphore as *mut u8 };
    EOK
}

/// Destroy an unnamed semaphore
///
/// # Arguments
/// * `sem` - Pointer to semaphore to destroy
///
/// # Returns
/// * 0 on success, error code on failure
pub unsafe extern "C" fn sem_destroy(sem: *mut SemT) -> i32 {
    if sem.is_null() {
        return EINVAL;
    }

    let semaphore = (*sem).sem_internal as *mut SemaphoreDescriptor;
    if semaphore.is_null() {
        return EINVAL;
    }

    // Check if it's a named semaphore (can't destroy named semaphores with sem_destroy)
    let sem_ref = &*semaphore;
    if sem_ref.name.is_some() {
        return EINVAL;
    }

    // Free the semaphore
    drop(Box::from_raw(semaphore));
    *sem = SemT { sem_internal: core::ptr::null_mut() };

    EOK
}

/// Decrement a semaphore (block if necessary)
///
/// # Arguments
/// * `sem` - Semaphore to wait on
///
/// # Returns
/// * 0 on success, error code on failure
pub unsafe extern "C" fn sem_wait(sem: SemT) -> i32 {
    if sem.is_null() {
        return EINVAL;
    }

    let semaphore = sem.sem_internal as *const SemaphoreDescriptor;
    if semaphore.is_null() {
        return EINVAL;
    }

    let sem_ref = &*semaphore;
    sem_ref.internal.wait();

    EOK
}

/// Try to decrement a semaphore (non-blocking)
///
/// # Arguments
/// * `sem` - Semaphore to try to wait on
///
/// # Returns
/// * 0 on success, EAGAIN if semaphore is already locked, error code on failure
pub unsafe extern "C" fn sem_trywait(sem: SemT) -> i32 {
    if sem.is_null() {
        return EINVAL;
    }

    let semaphore = sem.sem_internal as *const SemaphoreDescriptor;
    if semaphore.is_null() {
        return EINVAL;
    }

    let sem_ref = &*semaphore;
    if sem_ref.internal.try_wait() {
        EOK
    } else {
        EAGAIN
    }
}

/// Decrement a semaphore with timeout
///
/// # Arguments
/// * `sem` - Semaphore to wait on
/// * `abs_timeout` - Absolute timeout
///
/// # Returns
/// * 0 on success, ETIMEDOUT if timeout occurs, error code on failure
pub unsafe extern "C" fn sem_timedwait(
    sem: SemT,
    abs_timeout: *const crate::posix::Timespec,
) -> i32 {
    if sem.is_null() || abs_timeout.is_null() {
        return EINVAL;
    }

    let timeout = &*abs_timeout;
    let duration_ns = timeout.tv_sec as u64 * 1_000_000_000 + timeout.tv_nsec as u64;

    let semaphore = sem.sem_internal as *const SemaphoreDescriptor;
    if semaphore.is_null() {
        return EINVAL;
    }

    let sem_ref = &*semaphore;
    if sem_ref.internal.wait_timeout(duration_ns) {
        EOK
    } else {
        crate::reliability::errno::ETIMEDOUT
    }
}

/// Increment a semaphore
///
/// # Arguments
/// * `sem` - Semaphore to post
///
/// # Returns
/// * 0 on success, error code on failure
pub unsafe extern "C" fn sem_post(sem: SemT) -> i32 {
    if sem.is_null() {
        return EINVAL;
    }

    let semaphore = sem.sem_internal as *const SemaphoreDescriptor;
    if semaphore.is_null() {
        return EINVAL;
    }

    let sem_ref = &*semaphore;
    sem_ref.internal.post();

    EOK
}

/// Get semaphore value
///
/// # Arguments
/// * `sem` - Semaphore to query
/// * `sval` - Pointer to store semaphore value
///
/// # Returns
/// * 0 on success, error code on failure
pub unsafe extern "C" fn sem_getvalue(sem: SemT, sval: *mut i32) -> i32 {
    if sem.is_null() || sval.is_null() {
        return EINVAL;
    }

    let semaphore = sem.sem_internal as *const SemaphoreDescriptor;
    if semaphore.is_null() {
        return EINVAL;
    }

    let sem_ref = &*semaphore;
    *sval = sem_ref.internal.value() as i32;

    EOK
}

// ============================================================================
// Named Semaphore Functions
// ============================================================================

/// Open a named semaphore
///
/// # Arguments
/// * `name` - Semaphore name
/// * `oflag` - Open flags (O_CREAT, O_EXCL)
/// * `mode` - Permission bits (if creating)
/// * `value` - Initial value (if creating)
///
/// # Returns
/// * Pointer to semaphore on success, SEM_FAILED on failure
pub unsafe extern "C" fn sem_open(
    name: *const i8,
    oflag: i32,
    mode: u32,
    value: u32,
) -> SemT {
    if name.is_null() {
        return SemT { sem_internal: core::ptr::null_mut() };
    }

    // Convert name to string
    let name_len = crate::libc::strlen(name);
    if name_len == 0 || name_len >= SEM_NAME_MAX {
        return SemT { sem_internal: core::ptr::null_mut() };
    }

    let name_str = alloc::string::String::from_utf8_lossy(
        core::slice::from_raw_parts(name as *const u8, name_len)
    ).into_owned();

    // Check for invalid name characters
    if name_str.starts_with('/') {
        return SemT { sem_internal: core::ptr::null_mut() };
    }

    let is_creating = (oflag & crate::posix::O_CREAT) != 0;
    let is_exclusive = (oflag & crate::posix::O_EXCL) != 0;

    let mut registry = NAMED_SEMAPHORES.lock();

    if let Some(existing) = registry.get(&name_str) {
        // Semaphore already exists
        if is_creating && is_exclusive {
            return SemT { sem_internal: core::ptr::null_mut() };
        }

        // Increment reference count and return existing semaphore
        existing.ref_count.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        SemT { sem_internal: Arc::into_raw(existing.clone()) as *mut u8 }
    } else if is_creating {
        // Create new semaphore
        if registry.len() >= MAX_NAMED_SEMAPHORES {
            return SemT { sem_internal: core::ptr::null_mut() };
        }

        let internal = crate::sync::primitives::Semaphore::new(value);
        let semaphore = Arc::new(SemaphoreDescriptor {
            name: Some(name_str.clone()),
            internal: Arc::new(internal),
            ref_count: core::sync::atomic::AtomicUsize::new(1),
            mode: mode & 0o777,
            uid: crate::process::getuid(),
            creator_pid: crate::process::getpid() as crate::posix::Pid,
        });

        registry.insert(name_str, semaphore.clone());
        SemT { sem_internal: Arc::into_raw(semaphore) as *mut u8 }
    } else {
        // Semaphore doesn't exist and O_CREAT not specified
        SemT { sem_internal: core::ptr::null_mut() }
    }
}

/// Close a named semaphore
///
/// # Arguments
/// * `sem` - Semaphore to close
///
/// # Returns
/// * 0 on success, error code on failure
pub unsafe extern "C" fn sem_close(sem: SemT) -> i32 {
    if sem.is_null() {
        return EINVAL;
    }

    let semaphore = sem.sem_internal as *const SemaphoreDescriptor;
    if semaphore.is_null() {
        return EINVAL;
    }

    let sem_ref = &*semaphore;

    // Only named semaphores can be closed with sem_close
    if sem_ref.name.is_none() {
        return EINVAL;
    }

    // Decrement reference count
    let old_count = sem_ref.ref_count.fetch_sub(1, core::sync::atomic::Ordering::SeqCst);
    if old_count <= 1 {
        // This was the last reference, remove from registry
        if let Some(name) = &sem_ref.name {
            let mut registry = NAMED_SEMAPHORES.lock();
            registry.remove(name);
        }
    }

    // Release the Arc reference
    drop(Arc::from_raw(semaphore));

    EOK
}

/// Unlink (remove) a named semaphore
///
/// # Arguments
/// * `name` - Name of semaphore to remove
///
/// # Returns
/// * 0 on success, error code on failure
pub unsafe extern "C" fn sem_unlink(name: *const i8) -> i32 {
    if name.is_null() {
        return EINVAL;
    }

    // Convert name to string
    let name_len = crate::libc::strlen(name);
    if name_len == 0 || name_len >= SEM_NAME_MAX {
        return EINVAL;
    }

    let name_str = alloc::string::String::from_utf8_lossy(
        core::slice::from_raw_parts(name as *const u8, name_len)
    ).into_owned();

    let mut registry = NAMED_SEMAPHORES.lock();
    match registry.remove(&name_str) {
        Some(_) => EOK,
        None => ENOENT,
    }
}

/// Failed semaphore return value
pub const SEM_FAILED: SemT = SemT { sem_internal: core::ptr::null_mut() };