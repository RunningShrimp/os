//! GLib extensions and related syscalls

use super::common::{SyscallError, SyscallResult};
use crate::subsystems::sync::Mutex;
use alloc::{collections::VecDeque, string::String, vec::Vec};
use core::sync::atomic::{AtomicU32, Ordering};

// ============================================================================
// SignalFd Data Structures
// ============================================================================

/// SignalFd flags (Linux compatible)
pub mod signalfd_flags {
    pub const SFD_CLOEXEC: i32 = 0x02000000; // Close on exec
    pub const SFD_NONBLOCK: i32 = 0x00004000; // Non-blocking I/O
}

/// SignalFd siginfo structure (Linux compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SignalfdSiginfo {
    pub signo: u32,        // Signal number
    pub errno: i32,        // Error number
    pub code: i32,         // Signal code
    pub pid: u32,          // Sending process ID
    pub uid: u32,          // Sending user ID
    pub fd: i32,           // File descriptor (SIGIO)
    pub tid: u32,          // Kernel timer ID (POSIX timer)
    pub band: u32,         // Band event (SIGIO)
    pub overrun: u32,      // POSIX timer overrun count
    pub trapno: u32,       // Trap number that caused signal
    pub status: i32,       // Exit value or signal
    pub int: i32,          // Integer sent by sigqueue(3)
    pub ptr: usize,        // Pointer sent by sigqueue(3)
    pub utime: usize,      // User CPU time consumed (SIGVTALRM)
    pub stime: usize,      // System CPU time consumed (SIGVTALRM)
    pub addr: usize,       // Address that generated signal (SIGBUS, SIGFPE, SIGSEGV)
    pub addr_lsb: u16,     // Least significant bit of address (SIGBUS)
    pub lower: usize,      // Lower bound when address violation occurred (SIGBUS)
    pub upper: usize,      // Upper bound when address violation occurred (SIGBUS)
    pub pkey: u32,         // Protection key on PTE that caused fault (SIGSEGV)
    pub padding: [u8; 12], // Padding to 128 bytes total
}

/// SignalFd instance structure
#[derive(Debug)]
pub struct SignalfdInstance {
    /// Signal mask for filtering
    mask: crate::ipc::signal::SigSet,
    /// Signal queue
    signal_queue: VecDeque<SignalfdSiginfo>,
    /// Flags from signalfd4
    flags: i32,
    /// Maximum queue size
    max_queue_size: usize,
}

impl SignalfdInstance {
    pub fn new(mask: crate::ipc::signal::SigSet, flags: i32) -> Self {
        Self {
            mask,
            signal_queue: VecDeque::new(),
            flags,
            max_queue_size: 16384, // Default max signals
        }
    }

    /// Add a signal to the queue if it matches the mask
    pub fn enqueue_signal(&mut self, sig: crate::ipc::signal::Signal, info: crate::ipc::signal::SigInfo) -> bool {
        if !self.mask.contains(sig) {
            return false; // Signal not in mask
        }

        if self.signal_queue.len() >= self.max_queue_size {
            // Queue full, drop signal (Linux behavior)
            return false;
        }

        // Convert SigInfo to SignalfdSiginfo
        let siginfo = SignalfdSiginfo {
            signo: sig as u32,
            errno: info.errno,
            code: info.code,
            pid: info.pid as u32,
            uid: info.uid,
            status: info.status,
            addr: info.addr,
            ..Default::default()
        };

        self.signal_queue.push_back(siginfo);
        true
    }

    /// Read signals from queue
    pub fn read_signals(&mut self, buf: &mut [u8], nonblock: bool) -> Result<usize, SyscallError> {
        if self.signal_queue.is_empty() {
            if nonblock {
                return Err(SyscallError::WouldBlock);
            } else {
                // Blocking mode - would need to sleep here
                return Err(SyscallError::WouldBlock);
            }
        }

        let mut total_read = 0;
        let siginfo_size = core::mem::size_of::<SignalfdSiginfo>();

        while let Some(siginfo) = self.signal_queue.pop_front() {
            if total_read + siginfo_size <= buf.len() {
                // Copy siginfo to buffer
                let siginfo_bytes = unsafe {
                    core::slice::from_raw_parts(
                        &siginfo as *const SignalfdSiginfo as *const u8,
                        siginfo_size,
                    )
                };
                buf[total_read..total_read + siginfo_size].copy_from_slice(siginfo_bytes);
                total_read += siginfo_size;
            } else {
                // Put back the siginfo if it doesn't fit
                self.signal_queue.push_front(siginfo);
                break;
            }
        }

        Ok(total_read)
    }

    /// Check if there are signals available
    pub fn has_signals(&self) -> bool {
        !self.signal_queue.is_empty()
    }

    /// Update the signal mask
    pub fn update_mask(&mut self, new_mask: crate::ipc::signal::SigSet) {
        self.mask = new_mask;
    }

    /// Get current mask
    pub fn get_mask(&self) -> crate::ipc::signal::SigSet {
        self.mask
    }
}

// Global signalfd instances storage
pub static SIGNALFD_INSTANCES: Mutex<Vec<Option<SignalfdInstance>>> = Mutex::new(Vec::new());

// Public function for signal.rs to access signalfd instances

/// Allocate a signalfd instance and return index
fn alloc_signalfd_instance(mask: crate::ipc::signal::SigSet, flags: i32) -> Option<usize> {
    let mut instances = SIGNALFD_INSTANCES.lock();
    let instance = SignalfdInstance::new(mask, flags);

    // Find free slot or extend vector
    for (i, slot) in instances.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(instance);
            return Some(i);
        }
    }

    // No free slot, add new one
    instances.push(Some(instance));
    Some(instances.len() - 1)
}

/// Get signalfd instance by index
pub fn get_signalfd_instance(idx: usize) -> Option<&'static mut SignalfdInstance> {
    let mut instances = SIGNALFD_INSTANCES.lock();
    if let Some(ref mut instance) = instances.get_mut(idx) {
        // Convert to static mutable reference (unsafe but controlled)
        unsafe {
            let ptr = instance.as_mut().unwrap() as *mut SignalfdInstance;
            Some(&mut *ptr)
        }
    } else {
        None
    }
}

/// Send signal to all matching signalfd instances for a process
pub fn deliver_signal_to_signalfd(pid: usize, sig: crate::ipc::signal::Signal, info: crate::ipc::signal::SigInfo) -> bool {
    // Find all signalfd file descriptors for this process
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    if let Some(proc) = proc_table.find_ref(pid as crate::process::Pid) {
        let mut delivered = false;

        // Check all file descriptors for signalfd instances
        for fd in 0..crate::process::NOFILE as usize {
            if let Some(file_idx) = crate::process::fdlookup(fd as i32) {
                let file_table = crate::fs::file::FILE_TABLE.lock();
                if let Some(file) = file_table.get(file_idx) {
                    if file.ftype == crate::fs::file::FileType::Signalfd {
                        if let Some(signalfd_idx) = file.signalfd_instance {
                            if let Some(signalfd) = get_signalfd_instance(signalfd_idx) {
                                if signalfd.enqueue_signal(sig, info) {
                                    delivered = true;
                                }
                            }
                        }
                    }
                }
            }
        }

        delivered
    } else {
        false
    }
}

// ============================================================================
// EventFd Data Structures
// ============================================================================

/// EventFd flags (Linux compatible)
pub mod eventfd_flags {
    pub const EFD_SEMAPHORE: i32 = 0x00000001; // Provide semaphore-like semantics
    pub const EFD_CLOEXEC: i32  = 0x02000000; // Close on exec
    pub const EFD_NONBLOCK: i32 = 0x00004000; // Non-blocking I/O
}

/// EventFd instance structure
#[derive(Debug)]
pub struct EventFdInstance {
    /// 64-bit counter (atomic for thread safety)
    counter: core::sync::atomic::AtomicU64,
    /// Flags from eventfd2
    flags: i32,
    /// Wait queue for blocking operations (simplified - would need proper wait queue in real impl)
    wait_queue: alloc::vec::Vec<usize>,
}

impl EventFdInstance {
    pub fn new(initval: u32, flags: i32) -> Self {
        Self {
            counter: core::sync::atomic::AtomicU64::new(initval as u64),
            flags,
            wait_queue: alloc::vec::Vec::new(),
        }
    }

    /// Read from eventfd
    pub fn read(&mut self, buf: &mut [u8], nonblock: bool) -> Result<usize, SyscallError> {
        let current = self.counter.load(Ordering::SeqCst);

        if current == 0 {
            if nonblock {
                return Err(SyscallError::WouldBlock);
            } else {
                // Blocking mode - would need to sleep here
                // For now, return EAGAIN to indicate blocking behavior
                return Err(SyscallError::WouldBlock);
            }
        }

        let value_to_read = if (self.flags & eventfd_flags::EFD_SEMAPHORE) != 0 {
            // Semaphore mode: read 1 and decrement by 1
            self.counter.fetch_sub(1, Ordering::SeqCst);
            1u64
        } else {
            // Counter mode: read current value and reset to 0
            self.counter.swap(0, Ordering::SeqCst)
        };

        // Write the 64-bit value to buffer (little-endian)
        if buf.len() < 8 {
            return Err(SyscallError::InvalidArgument);
        }

        buf[0..8].copy_from_slice(&value_to_read.to_le_bytes());
        Ok(8)
    }

    /// Write to eventfd
    pub fn write(&mut self, buf: &[u8]) -> Result<usize, SyscallError> {
        if buf.len() < 8 {
            return Err(SyscallError::InvalidArgument);
        }

        let value = u64::from_le_bytes(buf[0..8].try_into().unwrap());

        // Check for overflow (Linux behavior: writing 0xffffffffffffffff would overflow)
        if value == u64::MAX {
            return Err(SyscallError::InvalidArgument);
        }

        // Add to counter, checking for overflow
        let current = self.counter.load(Ordering::SeqCst);
        if current > u64::MAX - value {
            return Err(SyscallError::InvalidArgument);
        }

        self.counter.fetch_add(value, Ordering::SeqCst);

        // Wake up any waiting readers (simplified)
        // In real implementation, would wake wait queue

        Ok(8)
    }

    /// Get current counter value (for debugging/testing)
    pub fn get_counter(&self) -> u64 {
        self.counter.load(Ordering::SeqCst)
    }
}

// Global eventfd instances storage
pub static EVENTFD_INSTANCES: Mutex<Vec<Option<EventFdInstance>>> = Mutex::new(Vec::new());

// ============================================================================
// TimerFd Data Structures
// ============================================================================

/// TimerFd flags (Linux compatible)
pub mod timerfd_flags {
    pub const TFD_CLOEXEC: i32 = 0x02000000; // Close on exec
    pub const TFD_NONBLOCK: i32 = 0x00004000; // Non-blocking I/O
    pub const TFD_TIMER_ABSTIME: i32 = 1;     // Absolute time
    pub const TFD_TIMER_CANCEL_ON_SET: i32 = 2; // Cancel on set
}

/// TimerFd instance structure
#[derive(Debug)]
pub struct TimerFdInstance {
    /// Clock ID (CLOCK_REALTIME or CLOCK_MONOTONIC)
    clock_id: i32,
    /// Flags from timerfd_create
    flags: i32,
    /// Current timer settings
    current_spec: crate::posix::Itimerspec,
    /// Expiration count (number of times timer has expired)
    expiration_count: core::sync::atomic::AtomicU64,
    /// Next expiration time (nanoseconds since epoch or boot)
    next_expiration: core::sync::atomic::AtomicU64,
    /// Timer ID for kernel timer management
    timer_id: core::sync::atomic::AtomicU32,
    /// Is timer armed?
    armed: core::sync::atomic::AtomicBool,
}

impl TimerFdInstance {
    pub fn new(clock_id: i32, flags: i32) -> Self {
        Self {
            clock_id,
            flags,
            current_spec: crate::posix::Itimerspec::default(),
            expiration_count: core::sync::atomic::AtomicU64::new(0),
            next_expiration: core::sync::atomic::AtomicU64::new(0),
            timer_id: core::sync::atomic::AtomicU32::new(0),
            armed: core::sync::atomic::AtomicBool::new(false),
        }
    }

    /// Set timer specification
    pub fn set_time(&mut self, new_spec: crate::posix::Itimerspec, flags: i32) -> Result<crate::posix::Itimerspec, SyscallError> {
        let old_spec = self.current_spec;

        // Validate the new specification
        if new_spec.it_value.tv_sec < 0 || new_spec.it_value.tv_nsec < 0 ||
           new_spec.it_value.tv_nsec >= 1_000_000_000 ||
           new_spec.it_interval.tv_sec < 0 || new_spec.it_interval.tv_nsec < 0 ||
           new_spec.it_interval.tv_nsec >= 1_000_000_000 {
            return Err(SyscallError::InvalidArgument);
        }

        // Get current time based on clock
        let current_ns = match self.clock_id {
            crate::posix::CLOCK_REALTIME => crate::subsystems::time::timestamp_nanos(),
            crate::posix::CLOCK_MONOTONIC => crate::subsystems::time::hrtime_nanos(),
            _ => return Err(SyscallError::InvalidArgument),
        };

        // Calculate next expiration
        let mut next_expiration = 0u64;
        if new_spec.it_value.tv_sec != 0 || new_spec.it_value.tv_nsec != 0 {
            let initial_ns = (new_spec.it_value.tv_sec as u64) * 1_000_000_000 + (new_spec.it_value.tv_nsec as u64);

            if (flags & timerfd_flags::TFD_TIMER_ABSTIME) != 0 {
                // Absolute time
                next_expiration = initial_ns;
            } else {
                // Relative time
                next_expiration = current_ns + initial_ns;
            }
        }

        // Update timer state
        self.current_spec = new_spec;
        self.next_expiration.store(next_expiration, Ordering::SeqCst);
        self.armed.store(next_expiration > 0, Ordering::SeqCst);

        // Reset expiration count if timer is being set
        if next_expiration > 0 {
            self.expiration_count.store(0, Ordering::SeqCst);
        }

        // TODO: Register timer with kernel timer system for actual expiration handling

        Ok(old_spec)
    }

    /// Get current timer specification
    pub fn get_time(&self) -> crate::posix::Itimerspec {
        self.current_spec
    }

    /// Read expiration count (timerfd semantics)
    pub fn read_expirations(&mut self, buf: &mut [u8], nonblock: bool) -> Result<usize, SyscallError> {
        let count = self.expiration_count.swap(0, Ordering::SeqCst);

        if count == 0 {
            if nonblock {
                return Err(SyscallError::WouldBlock);
            } else {
                // Blocking mode - would need to wait for expiration
                // For now, return EAGAIN to indicate blocking behavior
                return Err(SyscallError::WouldBlock);
            }
        }

        // Write the 64-bit count to buffer (little-endian)
        if buf.len() < 8 {
            return Err(SyscallError::InvalidArgument);
        }

        buf[0..8].copy_from_slice(&count.to_le_bytes());
        Ok(8)
    }

    /// Handle timer expiration (called by timer system)
    pub fn on_expiration(&mut self) {
        // Increment expiration count
        self.expiration_count.fetch_add(1, Ordering::SeqCst);

        // If interval timer, schedule next expiration
        if self.current_spec.it_interval.tv_sec != 0 || self.current_spec.it_interval.tv_nsec != 0 {
            let interval_ns = (self.current_spec.it_interval.tv_sec as u64) * 1_000_000_000 +
                             (self.current_spec.it_interval.tv_nsec as u64);
            let current_expiration = self.next_expiration.load(Ordering::SeqCst);
            let next_expiration = current_expiration + interval_ns;
            self.next_expiration.store(next_expiration, Ordering::SeqCst);
            // TODO: Re-register timer with kernel timer system
        } else {
            // One-shot timer, disarm it
            self.armed.store(false, Ordering::SeqCst);
        }
    }

    /// Check if timer has expired events
    pub fn has_expirations(&self) -> bool {
        self.expiration_count.load(Ordering::SeqCst) > 0
    }

    /// Get clock ID
    pub fn clock_id(&self) -> i32 {
        self.clock_id
    }

    /// Get flags
    pub fn flags(&self) -> i32 {
        self.flags
    }
}

// Global timerfd instances storage
pub static TIMERFD_INSTANCES: Mutex<Vec<Option<TimerFdInstance>>> = Mutex::new(Vec::new());

// TimerFd management functions

/// Allocate a timerfd instance and return index
fn alloc_timerfd_instance(clock_id: i32, flags: i32) -> Option<usize> {
    // Validate clock ID
    match clock_id {
        crate::posix::CLOCK_REALTIME | crate::posix::CLOCK_MONOTONIC => {
            // Supported clocks
        }
        _ => return None,
    }

    // Validate flags
    let valid_flags = timerfd_flags::TFD_CLOEXEC | timerfd_flags::TFD_NONBLOCK;
    if (flags & !valid_flags) != 0 {
        return None;
    }

    let mut instances = TIMERFD_INSTANCES.lock();
    let instance = TimerFdInstance::new(clock_id, flags);

    // Find free slot or extend vector
    for (i, slot) in instances.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(instance);
            return Some(i);
        }
    }

    // No free slot, add new one
    instances.push(Some(instance));
    Some(instances.len() - 1)
}

/// Get timerfd instance by index
pub fn get_timerfd_instance(idx: usize) -> Option<&'static mut TimerFdInstance> {
    let mut instances = TIMERFD_INSTANCES.lock();
    if let Some(ref mut instance) = instances.get_mut(idx) {
        // Convert to static mutable reference (unsafe but controlled)
        unsafe {
            let ptr = instance.as_mut().unwrap() as *mut TimerFdInstance;
            Some(&mut *ptr)
        }
    } else {
        None
    }
}

// ============================================================================
// Inotify Data Structures
// ============================================================================

/// Inotify event structure (Linux compatible)
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InotifyEvent {
    /// Watch descriptor
    pub wd: i32,
    /// Mask of events
    pub mask: u32,
    /// Unique cookie associating related events
    pub cookie: u32,
    /// Size of name field
    pub len: u32,
    // Optional name (null-terminated)
    // name follows this structure in memory
}

/// Inotify event masks
pub mod inotify_mask {
    pub const IN_ACCESS: u32        = 0x00000001; // File was accessed
    pub const IN_MODIFY: u32        = 0x00000002; // File was modified
    pub const IN_ATTRIB: u32        = 0x00000004; // Metadata changed
    pub const IN_CLOSE_WRITE: u32   = 0x00000008; // Writtable file was closed
    pub const IN_CLOSE_NOWRITE: u32 = 0x00000010; // Unwrittable file closed
    pub const IN_CLOSE: u32         = IN_CLOSE_WRITE | IN_CLOSE_NOWRITE;
    pub const IN_OPEN: u32          = 0x00000020; // File was opened
    pub const IN_MOVED_FROM: u32   = 0x00000040; // File was moved from X
    pub const IN_MOVED_TO: u32      = 0x00000080; // File was moved to Y
    pub const IN_MOVE: u32          = IN_MOVED_FROM | IN_MOVED_TO;
    pub const IN_CREATE: u32        = 0x00000100; // Subfile was created
    pub const IN_DELETE: u32        = 0x00000200; // Subfile was deleted
    pub const IN_DELETE_SELF: u32   = 0x00000400; // Self was deleted
    pub const IN_MOVE_SELF: u32     = 0x00000800; // Self was moved
    pub const IN_UNMOUNT: u32       = 0x00002000; // Backing fs was unmounted
    pub const IN_Q_OVERFLOW: u32    = 0x00004000; // Event queued overflowed
    pub const IN_IGNORED: u32       = 0x00008000; // File was ignored
    pub const IN_ONLYDIR: u32       = 0x01000000; // only watch the path if it is a directory
    pub const IN_DONT_FOLLOW: u32   = 0x02000000; // don't follow a sym link
    pub const IN_EXCL_UNLINK: u32   = 0x04000000; // exclude events on unlinked objects
    pub const IN_MASK_ADD: u32      = 0x20000000; // add to the mask of an already existing watch
    pub const IN_ISDIR: u32         = 0x40000000; // event occurred against dir
    pub const IN_ONESHOT: u32       = 0x80000000; // only send event once
}

/// Inotify flags for init1
pub mod inotify_flags {
    pub const IN_CLOEXEC: i32 = 0x02000000;
    pub const IN_NONBLOCK: i32 = 0x00004000;
}

/// Watch descriptor structure
#[derive(Debug)]
pub struct WatchDescriptor {
    /// Watch descriptor ID
    pub wd: i32,
    /// Path being watched
    pub path: String,
    /// Event mask
    pub mask: u32,
    /// Device number
    pub dev: u64,
    /// Inode number
    pub ino: u64,
}

/// Inotify instance structure
#[derive(Debug)]
pub struct InotifyInstance {
    /// Next watch descriptor ID
    next_wd: AtomicU32,
    /// Watch descriptors
    watches: Vec<WatchDescriptor>,
    /// Event queue
    event_queue: VecDeque<Vec<u8>>, // Events as raw bytes
    /// Maximum queue size
    max_queue_size: usize,
    /// Flags from init1
    flags: i32,
}

impl InotifyInstance {
    pub fn new(flags: i32) -> Self {
        Self {
            next_wd: AtomicU32::new(1),
            watches: Vec::new(),
            event_queue: VecDeque::new(),
            max_queue_size: 16384, // Default max events
            flags,
        }
    }

    /// Add a watch
    pub fn add_watch(&mut self, path: &str, mask: u32) -> Result<i32, SyscallError> {
        // Check if path already watched
        for watch in &self.watches {
            if watch.path == path {
                return Err(SyscallError::FileExists);
            }
        }

        let wd = self.next_wd.fetch_add(1, Ordering::SeqCst) as i32;

        // Get inode info (simplified - in real implementation would get from VFS)
        let dev = 0; // TODO: get from VFS
        let ino = 0; // TODO: get from VFS

        let watch = WatchDescriptor {
            wd,
            path: alloc::string::String::from(path),
            mask,
            dev,
            ino,
        };

        self.watches.push(watch);
        Ok(wd)
    }

    /// Remove a watch
    pub fn rm_watch(&mut self, wd: i32) -> Result<(), SyscallError> {
        if let Some(pos) = self.watches.iter().position(|w| w.wd == wd) {
            self.watches.remove(pos);
            // Generate IN_IGNORED event
            self.generate_event(wd, inotify_mask::IN_IGNORED, 0, "");
            Ok(())
        } else {
            Err(SyscallError::InvalidArgument)
        }
    }

    /// Generate an event
    pub fn generate_event(&mut self, wd: i32, mask: u32, cookie: u32, name: &str) {
        if self.event_queue.len() >= self.max_queue_size {
            // Generate overflow event
            let overflow_event = InotifyEvent {
                wd: -1, // Special WD for overflow
                mask: inotify_mask::IN_Q_OVERFLOW,
                cookie: 0,
                len: 0,
            };
            let mut event_bytes = Vec::new();
            event_bytes.extend_from_slice(&unsafe {
                core::slice::from_raw_parts(
                    &overflow_event as *const InotifyEvent as *const u8,
                    core::mem::size_of::<InotifyEvent>(),
                )
            });
            self.event_queue.push_back(event_bytes);
            return;
        }

        let name_len = name.len() + 1; // +1 for null terminator
        let total_len = core::mem::size_of::<InotifyEvent>() + name_len;

        let event = InotifyEvent {
            wd,
            mask,
            cookie,
            len: name_len as u32,
        };

        let mut event_bytes = Vec::with_capacity(total_len);
        event_bytes.extend_from_slice(&unsafe {
            core::slice::from_raw_parts(
                &event as *const InotifyEvent as *const u8,
                core::mem::size_of::<InotifyEvent>(),
            )
        });

        // Add name with null terminator
        event_bytes.extend_from_slice(name.as_bytes());
        event_bytes.push(0); // null terminator

        self.event_queue.push_back(event_bytes);
    }

    /// Read events from queue
    pub fn read_events(&mut self, buf: &mut [u8]) -> usize {
        let mut total_read = 0;
        let mut temp_queue = VecDeque::new();

        while let Some(event_bytes) = self.event_queue.pop_front() {
            if total_read + event_bytes.len() <= buf.len() {
                buf[total_read..total_read + event_bytes.len()].copy_from_slice(&event_bytes);
                total_read += event_bytes.len();
            } else {
                // Put back the event if it doesn't fit
                temp_queue.push_front(event_bytes);
                break;
            }
        }

        // Put back any events that didn't fit
        while let Some(event) = temp_queue.pop_back() {
            self.event_queue.push_front(event);
        }

        total_read
    }

    /// Check if there are events available
    pub fn has_events(&self) -> bool {
        !self.event_queue.is_empty()
    }
}

// Global inotify instances storage
pub static INOTIFY_INSTANCES: Mutex<Vec<Option<InotifyInstance>>> = Mutex::new(Vec::new());

// ============================================================================
// MemFd Data Structures
// ============================================================================

/// MemFd flags (Linux compatible)
pub mod memfd_flags {
    pub const MFD_CLOEXEC: i32 = 0x00000001; // Close on exec
    pub const MFD_ALLOW_SEALING: i32 = 0x00000002; // Allow sealing
    pub const MFD_HUGETLB: i32 = 0x00000004; // Huge TLB
    pub const MFD_NO_SEAL: i32 = 0x00000008; // No sealing (for testing)
}

/// File sealing flags (Linux compatible)
pub mod fcntl_seals {
    pub const F_SEAL_SEAL: u32 = 0x00000001; // Prevent adding more seals
    pub const F_SEAL_SHRINK: u32 = 0x00000002; // Prevent file from shrinking
    pub const F_SEAL_GROW: u32 = 0x00000004; // Prevent file from growing
    pub const F_SEAL_WRITE: u32 = 0x00000008; // Prevent write operations
    pub const F_SEAL_FUTURE_WRITE: u32 = 0x00000010; // Prevent future write operations
    pub const F_SEAL_EXEC: u32 = 0x00000020; // Prevent execution
}

/// MemFd instance structure
#[derive(Debug)]
pub struct MemFdInstance {
    /// Name of the anonymous memory file
    name: String,
    /// Size of the memory file
    size: usize,
    /// Current seals applied to the file
    seals: u32,
    /// Flags from memfd_create
    flags: i32,
    /// Memory pages allocated for this file
    pages: Vec<usize>, // Store physical addresses instead of raw pointers for thread safety
    /// Maximum allowed size
    max_size: usize,
    /// Reference count for shared access
    ref_count: core::sync::atomic::AtomicU32,
}

impl MemFdInstance {
    /// Create a new MemFd instance
    pub fn new(name: String, flags: i32) -> Self {
        Self {
            name,
            size: 0,
            seals: 0,
            flags,
            pages: Vec::new(),
            max_size: usize::MAX, // Default to max size
            ref_count: core::sync::atomic::AtomicU32::new(1),
        }
    }

    /// Read from the memory file
    pub fn read(&mut self, offset: usize, buf: &mut [u8]) -> Result<usize, SyscallError> {
        if offset >= self.size {
            return Ok(0); // EOF
        }

        let bytes_to_read = core::cmp::min(buf.len(), self.size - offset);
        let mut bytes_read = 0;

        while bytes_read < bytes_to_read {
            let page_idx = (offset + bytes_read) / crate::subsystems::mm::PAGE_SIZE;
            let page_offset = (offset + bytes_read) % crate::subsystems::mm::PAGE_SIZE;
            let chunk_size = core::cmp::min(
                bytes_to_read - bytes_read,
                crate::subsystems::mm::PAGE_SIZE - page_offset,
            );

            if page_idx >= self.pages.len() {
                break;
            }

            let page_addr = self.pages[page_idx];
            if page_addr == 0 {
                // Zero page for unmapped regions
                for i in 0..chunk_size {
                    buf[bytes_read + i] = 0;
                }
            } else {
                unsafe {
                    let page_ptr = crate::subsystems::mm::vm::phys_to_kernel_ptr(page_addr);
                    let src = page_ptr.add(page_offset);
                    let dst = buf.as_mut_ptr().add(bytes_read);
                    core::ptr::copy_nonoverlapping(src, dst, chunk_size);
                }
            }

            bytes_read += chunk_size;
        }

        Ok(bytes_read)
    }

    /// Write to the memory file
    pub fn write(&mut self, offset: usize, buf: &[u8]) -> Result<usize, SyscallError> {
        // Check write seal
        if (self.seals & fcntl_seals::F_SEAL_WRITE) != 0 {
            return Err(SyscallError::PermissionDenied);
        }

        // Check grow seal
        if offset + buf.len() > self.size &&
           (self.seals & fcntl_seals::F_SEAL_GROW) != 0 {
            return Err(SyscallError::PermissionDenied);
        }

        // Check max size
        if offset + buf.len() > self.max_size {
            return Err(SyscallError::InvalidArgument);
        }

        // Ensure pages are allocated
        let required_pages = (offset + buf.len() + crate::subsystems::mm::PAGE_SIZE - 1) / crate::subsystems::mm::PAGE_SIZE;
        while self.pages.len() < required_pages {
            let page = unsafe { crate::subsystems::mm::kalloc() };
            if page.is_null() {
                return Err(SyscallError::OutOfMemory);
            }
            unsafe { core::ptr::write_bytes(page, 0, crate::subsystems::mm::PAGE_SIZE); }
            let page_addr = page as usize;
            self.pages.push(page_addr);
        }

        let mut bytes_written = 0;
        while bytes_written < buf.len() {
            let page_idx = (offset + bytes_written) / crate::subsystems::mm::PAGE_SIZE;
            let page_offset = (offset + bytes_written) % crate::subsystems::mm::PAGE_SIZE;
            let chunk_size = core::cmp::min(
                buf.len() - bytes_written,
                crate::subsystems::mm::PAGE_SIZE - page_offset,
            );

            let page_addr = self.pages[page_idx];
            unsafe {
                let page_ptr = crate::subsystems::mm::vm::phys_to_kernel_ptr(page_addr);
                let src = buf.as_ptr().add(bytes_written);
                let dst = page_ptr.add(page_offset);
                core::ptr::copy_nonoverlapping(src, dst, chunk_size);
            }

            bytes_written += chunk_size;
        }

        // Update file size
        let new_size = core::cmp::max(self.size, offset + buf.len());
        self.size = new_size;

        Ok(bytes_written)
    }

    /// Seal the file with specified seals
    pub fn add_seals(&mut self, new_seals: u32) -> Result<u32, SyscallError> {
        // Check if sealing is allowed
        if (self.flags & memfd_flags::MFD_ALLOW_SEALING) == 0 {
            return Err(SyscallError::PermissionDenied);
        }

        // Check if seal seal is already set
        if (self.seals & fcntl_seals::F_SEAL_SEAL) != 0 {
            return Err(SyscallError::PermissionDenied);
        }

        // Apply new seals
        self.seals |= new_seals;
        Ok(self.seals)
    }

    /// Get current seals
    pub fn get_seals(&self) -> u32 {
        self.seals
    }

    /// Get file size
    pub fn get_size(&self) -> usize {
        self.size
    }

    /// Truncate file to specified size
    pub fn truncate(&mut self, new_size: usize) -> Result<(), SyscallError> {
        // Check shrink seal
        if new_size < self.size && (self.seals & fcntl_seals::F_SEAL_SHRINK) != 0 {
            return Err(SyscallError::PermissionDenied);
        }

        // Check grow seal
        if new_size > self.size && (self.seals & fcntl_seals::F_SEAL_GROW) != 0 {
            return Err(SyscallError::PermissionDenied);
        }

        // Update size
        self.size = new_size;
        Ok(())
    }

    /// Increment reference count
    pub fn inc_ref(&self) {
        self.ref_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Decrement reference count and return if it reached zero
    pub fn dec_ref(&self) -> bool {
        self.ref_count.fetch_sub(1, Ordering::SeqCst) == 1
    }

    /// Get file name
    pub fn get_name(&self) -> &str {
        &self.name
    }

    /// Get flags
    pub fn get_flags(&self) -> i32 {
        self.flags
    }
}

impl Drop for MemFdInstance {
    fn drop(&mut self) {
        // Free all allocated pages
        for page_addr in &self.pages {
            if *page_addr != 0 {
                unsafe { crate::subsystems::mm::kfree(*page_addr as *mut u8) };
            }
        }
        self.pages.clear();
    }
}

// Global memfd instances storage
pub static MEMFD_INSTANCES: Mutex<Vec<Option<MemFdInstance>>> = Mutex::new(Vec::new());

/// Allocate a memfd instance and return index
fn alloc_memfd_instance(name: String, flags: i32) -> Option<usize> {
    let mut instances = MEMFD_INSTANCES.lock();
    let instance = MemFdInstance::new(name, flags);

    // Find free slot or extend vector
    for (i, slot) in instances.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(instance);
            return Some(i);
        }
    }

    // No free slot, add new one
    instances.push(Some(instance));
    Some(instances.len() - 1)
}

/// Get memfd instance by index
pub fn get_memfd_instance(idx: usize) -> Option<&'static mut MemFdInstance> {
    let mut instances = MEMFD_INSTANCES.lock();
    if let Some(ref mut instance) = instances.get_mut(idx) {
        // Convert to static mutable reference (unsafe but controlled)
        unsafe {
            let ptr = instance.as_mut().unwrap() as *mut MemFdInstance;
            Some(&mut *ptr)
        }
    } else {
        None
    }
}

// Public function for file.rs to access inotify instances

/// Dispatch GLib-related syscalls and extensions
pub fn dispatch(syscall_id: u32, args: &[u64]) -> SyscallResult {
    match syscall_id {
        // GLib extensions and related operations
        0xB000 => sys_getrandom(args),      // getrandom
        0xB001 => sys_memfd_create(args),   // memfd_create
        0xB002 => sys_eventfd(args),        // eventfd
        0xB003 => sys_eventfd2(args),       // eventfd2
        0xB004 => sys_timerfd_create(args), // timerfd_create
        0xB005 => sys_timerfd_settime(args), // timerfd_settime
        0xB006 => sys_timerfd_gettime(args), // timerfd_gettime
        0xB007 => sys_signalfd(args),       // signalfd
        0xB008 => sys_signalfd4(args),      // signalfd4
        0xB009 => sys_inotify_init(args),   // inotify_init
        0xB00A => sys_inotify_init1(args),  // inotify_init1
        0xB00B => sys_inotify_add_watch(args), // inotify_add_watch
        0xB00C => sys_inotify_rm_watch(args), // inotify_rm_watch
        // Test syscall for inotify (temporary)
        0xFFFF => {
            // Simple test: create inotify instance and add a watch
            let fd = sys_inotify_init(&[])?;
            let wd = sys_inotify_add_watch(&[fd as u64, "/tmp\0".as_ptr() as usize as u64, inotify_mask::IN_ACCESS as u64])?;
            Ok(wd)
        }
        _ => Err(SyscallError::InvalidSyscall),
    }
}

// Placeholder implementations - to be replaced with actual syscall logic

fn sys_getrandom(args: &[u64]) -> SyscallResult {
    use super::common::extract_args;
    use crate::subsystems::mm::vm::copyout;
    
    let args = extract_args(args, 3)?;
    let buf_ptr = args[0] as usize;
    let buf_len = args[1] as usize;
    let flags = args[2] as u32;
    
    // Validate buffer
    if buf_ptr == 0 || buf_len == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Limit buffer size to prevent excessive work
    let max_len = 256usize;
    let actual_len = buf_len.min(max_len);
    
    // Get current process for page table access
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Generate pseudo-random bytes using a simple LFSR-based RNG
    // In a real kernel, this would use hardware RNG if available
    let mut rand_bytes = [0u8; 256];
    let seed = crate::subsystems::time::timestamp_nanos() as u64;
    let mut state = seed;
    
    for i in 0..actual_len {
        // Linear feedback shift register for pseudo-random generation
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        rand_bytes[i] = (state & 0xFF) as u8;
    }
    
    // Copy random bytes to user space
    unsafe {
        copyout(pagetable, buf_ptr, rand_bytes.as_ptr(), actual_len)
            .map_err(|_| SyscallError::BadAddress)?;
    }
    
    let _ = flags; // Would handle GRND_RANDOM, GRND_NONBLOCK flags
    
    Ok(actual_len as u64)
}

fn sys_memfd_create(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let name_ptr = args[0] as usize;
    let flags = args[1] as i32;

    // Validate flags
    let valid_flags = memfd_flags::MFD_CLOEXEC |
                    memfd_flags::MFD_ALLOW_SEALING |
                    memfd_flags::MFD_HUGETLB;
    if (flags & !valid_flags) != 0 {
        return Err(SyscallError::InvalidArgument);
    }

    // Get current process for user space memory access
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::process::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(proc_table);

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Read name from user space (max 256 chars)
    const MAX_NAME_LEN: usize = 256;
    let mut name_buf = [0u8; MAX_NAME_LEN];
    let name_len = unsafe {
        crate::subsystems::mm::vm::copyinstr(pagetable, name_ptr, name_buf.as_mut_ptr(), MAX_NAME_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };

    // Convert to string, removing null terminator if present
    let name = if name_len > 0 && name_buf[name_len - 1] == 0 {
        alloc::string::String::from_utf8_lossy(&name_buf[..name_len - 1]).into_owned()
    } else {
        alloc::string::String::from_utf8_lossy(&name_buf[..name_len]).into_owned()
    };

    // If name is empty, use a default name
    let final_name = if name.is_empty() {
        alloc::format!("memfd:{}", pid)
    } else {
        name
    };

    // Allocate memfd instance
    let instance_idx = alloc_memfd_instance(final_name, flags)
        .ok_or(SyscallError::OutOfMemory)?;

    // Create file in file table
    let file_idx = crate::fs::file::file_alloc()
        .ok_or(SyscallError::OutOfMemory)?;

    // Initialize file as memfd type
    {
        let mut table = crate::fs::file::FILE_TABLE.lock();
        if let Some(file) = table.get_mut(file_idx) {
            file.ftype = crate::fs::file::FileType::MemFd;
            file.readable = true;
            file.writable = true;
            file.memfd_instance = Some(instance_idx);
            
            // Set close-on-exec flag if specified
            if (flags & memfd_flags::MFD_CLOEXEC) != 0 {
                file.status_flags |= crate::posix::O_CLOEXEC;
            }
        } else {
            return Err(SyscallError::OutOfMemory);
        }
    }

    // Allocate file descriptor
    let fd = crate::process::fdalloc(file_idx)
        .ok_or(SyscallError::OutOfMemory)?;

    Ok(fd as u64)
}

fn sys_eventfd(args: &[u64]) -> SyscallResult {
    if args.len() < 1 {
        return Err(SyscallError::InvalidArgument);
    }

    let initval = args[0] as u32;

    // Allocate eventfd instance
    let instance_idx = alloc_eventfd_instance(initval, 0).ok_or(SyscallError::OutOfMemory)?;

    // Create file in file table
    let file_idx = crate::fs::file::file_alloc().ok_or(SyscallError::OutOfMemory)?;

    // Initialize file as eventfd type
    {
        let mut table = crate::fs::file::FILE_TABLE.lock();
        if let Some(file) = table.get_mut(file_idx) {
            file.ftype = crate::fs::file::FileType::EventFd;
            file.readable = true;
            file.writable = true;
            file.eventfd_instance = Some(instance_idx);
        } else {
            return Err(SyscallError::OutOfMemory);
        }
    }

    // Allocate file descriptor
    let fd = crate::process::fdalloc(file_idx).ok_or(SyscallError::OutOfMemory)?;

    Ok(fd as u64)
}

fn sys_eventfd2(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let initval = args[0] as u32;
    let flags = args[1] as i32;

    // Validate flags
    let valid_flags = eventfd_flags::EFD_SEMAPHORE | eventfd_flags::EFD_CLOEXEC | eventfd_flags::EFD_NONBLOCK;
    if (flags & !valid_flags) != 0 {
        return Err(SyscallError::InvalidArgument);
    }

    // Allocate eventfd instance
    let instance_idx = alloc_eventfd_instance(initval, flags).ok_or(SyscallError::OutOfMemory)?;

    // Create file in file table
    let file_idx = crate::fs::file::file_alloc().ok_or(SyscallError::OutOfMemory)?;

    // Initialize file as eventfd type
    {
        let mut table = crate::fs::file::FILE_TABLE.lock();
        if let Some(file) = table.get_mut(file_idx) {
            file.ftype = crate::fs::file::FileType::EventFd;
            file.readable = true;
            file.writable = true;
            file.eventfd_instance = Some(instance_idx);

            // Set non-blocking flag if specified
            if (flags & eventfd_flags::EFD_NONBLOCK) != 0 {
                file.status_flags |= crate::posix::O_NONBLOCK;
            }

            // Set close-on-exec flag if specified
            if (flags & eventfd_flags::EFD_CLOEXEC) != 0 {
                file.status_flags |= crate::posix::O_CLOEXEC;
            }
        } else {
            return Err(SyscallError::OutOfMemory);
        }
    }

    // Allocate file descriptor
    let fd = crate::process::fdalloc(file_idx).ok_or(SyscallError::OutOfMemory)?;

    Ok(fd as u64)
}

fn sys_timerfd_create(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let clockid = args[0] as i32;
    let flags = args[1] as i32;

    // Allocate timerfd instance
    let instance_idx = alloc_timerfd_instance(clockid, flags).ok_or(SyscallError::OutOfMemory)?;

    // Create file in file table
    let file_idx = crate::fs::file::file_alloc().ok_or(SyscallError::OutOfMemory)?;

    // Initialize file as timerfd type
    {
        let mut table = crate::fs::file::FILE_TABLE.lock();
        if let Some(file) = table.get_mut(file_idx) {
            file.ftype = crate::fs::file::FileType::TimerFd;
            file.readable = true;
            file.writable = false;
            file.timerfd_instance = Some(instance_idx);

            // Set non-blocking flag if specified
            if (flags & timerfd_flags::TFD_NONBLOCK) != 0 {
                file.status_flags |= crate::posix::O_NONBLOCK;
            }

            // Set close-on-exec flag if specified
            if (flags & timerfd_flags::TFD_CLOEXEC) != 0 {
                file.status_flags |= crate::posix::O_CLOEXEC;
            }
        } else {
            return Err(SyscallError::OutOfMemory);
        }
    }

    // Allocate file descriptor
    let fd = crate::process::fdalloc(file_idx).ok_or(SyscallError::OutOfMemory)?;

    Ok(fd as u64)
}

fn sys_timerfd_settime(args: &[u64]) -> SyscallResult {
    if args.len() < 4 {
        return Err(SyscallError::InvalidArgument);
    }

    let fd = args[0] as i32;
    let flags = args[1] as i32;
    let new_value_ptr = args[2] as usize;
    let old_value_ptr = args[3] as usize;

    // Validate flags
    let valid_flags = timerfd_flags::TFD_TIMER_ABSTIME | timerfd_flags::TFD_TIMER_CANCEL_ON_SET;
    if (flags & !valid_flags) != 0 {
        return Err(SyscallError::InvalidArgument);
    }

    // Get current process for user space memory access
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::process::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(proc_table);

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Get file index from fd
    let file_idx = crate::process::fdlookup(fd).ok_or(SyscallError::BadFileDescriptor)?;

    // Check if it's a timerfd file
    {
        let table = crate::fs::file::FILE_TABLE.lock();
        let file = table.get(file_idx).ok_or(SyscallError::BadFileDescriptor)?;
        if file.ftype != crate::fs::file::FileType::TimerFd {
            return Err(SyscallError::InvalidArgument);
        }
    }

    // Read new timer value from user space
    let mut new_value = crate::posix::Itimerspec::default();
    unsafe {
        crate::subsystems::mm::vm::copyin(pagetable, core::ptr::addr_of_mut!(new_value) as *mut u8, new_value_ptr, core::mem::size_of::<crate::posix::Itimerspec>())
            .map_err(|_| SyscallError::BadAddress)?;
    }

    // Get timerfd instance
    let table = crate::fs::file::FILE_TABLE.lock();
    let file = table.get(file_idx).ok_or(SyscallError::BadFileDescriptor)?;
    let instance_idx = file.timerfd_instance.ok_or(SyscallError::InvalidArgument)?;

    if let Some(instance) = get_timerfd_instance(instance_idx) {
        // Set the timer
        let old_value = instance.set_time(new_value, flags)?;

        // Copy old value back to user space if requested
        if old_value_ptr != 0 {
            unsafe {
                crate::subsystems::mm::vm::copyout(pagetable, old_value_ptr, core::ptr::addr_of!(old_value) as *const u8, core::mem::size_of::<crate::posix::Itimerspec>())
                    .map_err(|_| SyscallError::BadAddress)?;
            }
        }

        Ok(0)
    } else {
        Err(SyscallError::InvalidArgument)
    }
}

fn sys_timerfd_gettime(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let fd = args[0] as i32;
    let curr_value_ptr = args[1] as usize;

    if curr_value_ptr == 0 {
        return Err(SyscallError::BadAddress);
    }

    // Get current process for user space memory access
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::process::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(proc_table);

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Get file index from fd
    let file_idx = crate::process::fdlookup(fd).ok_or(SyscallError::BadFileDescriptor)?;

    // Check if it's a timerfd file
    {
        let table = crate::fs::file::FILE_TABLE.lock();
        let file = table.get(file_idx).ok_or(SyscallError::BadFileDescriptor)?;
        if file.ftype != crate::fs::file::FileType::TimerFd {
            return Err(SyscallError::InvalidArgument);
        }
    }

    // Get timerfd instance
    let table = crate::fs::file::FILE_TABLE.lock();
    let file = table.get(file_idx).ok_or(SyscallError::BadFileDescriptor)?;
    let instance_idx = file.timerfd_instance.ok_or(SyscallError::InvalidArgument)?;

    if let Some(instance) = get_timerfd_instance(instance_idx) {
        let curr_value = instance.get_time();

        // Copy current value to user space
        unsafe {
            crate::subsystems::mm::vm::copyout(pagetable, curr_value_ptr, core::ptr::addr_of!(curr_value) as *const u8, core::mem::size_of::<crate::posix::Itimerspec>())
                .map_err(|_| SyscallError::BadAddress)?;
        }

        Ok(0)
    } else {
        Err(SyscallError::InvalidArgument)
    }
}

fn sys_signalfd(args: &[u64]) -> SyscallResult {
    if args.len() < 3 {
        return Err(SyscallError::InvalidArgument);
    }

    let fd = args[0] as i32;
    let mask_ptr = args[1] as usize;
    let flags = 0; // signalfd doesn't take flags, always 0

    // Call signalfd4 with flags = 0
    sys_signalfd4(&[fd as u64, mask_ptr as u64, flags as u64])
}

fn sys_signalfd4(args: &[u64]) -> SyscallResult {
    if args.len() < 3 {
        return Err(SyscallError::InvalidArgument);
    }

    let fd = args[0] as i32;
    let mask_ptr = args[1] as usize;
    let flags = args[2] as i32;

    // Validate flags
    let valid_flags = signalfd_flags::SFD_CLOEXEC | signalfd_flags::SFD_NONBLOCK;
    if (flags & !valid_flags) != 0 {
        return Err(SyscallError::InvalidArgument);
    }

    // Get current process
    let pid = crate::process::myproc().ok_or(SyscallError::NotFound)? as usize;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::NotFound)?;
    let pagetable = proc.pagetable;
    drop(proc_table);

    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }

    // Read signal mask from user space
    let mut mask = crate::ipc::signal::SigSet::empty();
    unsafe {
        crate::subsystems::mm::vm::copyin(pagetable, core::ptr::addr_of_mut!(mask) as *mut u8, mask_ptr, core::mem::size_of::<crate::ipc::signal::SigSet>())
            .map_err(|_| SyscallError::BadAddress)?;
    }

    if fd == -1 {
        // Create new signalfd
        let instance_idx = alloc_signalfd_instance(mask, flags).ok_or(SyscallError::OutOfMemory)?;

        // Create file in file table
        let file_idx = crate::fs::file::file_alloc().ok_or(SyscallError::OutOfMemory)?;

        // Initialize file as signalfd type
        {
            let mut table = crate::fs::file::FILE_TABLE.lock();
            if let Some(file) = table.get_mut(file_idx) {
                file.ftype = crate::fs::file::FileType::Signalfd;
                file.readable = true;
                file.writable = false;
                file.signalfd_instance = Some(instance_idx);

                // Set non-blocking flag if specified
                if (flags & signalfd_flags::SFD_NONBLOCK) != 0 {
                    file.status_flags |= crate::posix::O_NONBLOCK;
                }

                // Set close-on-exec flag if specified
                if (flags & signalfd_flags::SFD_CLOEXEC) != 0 {
                    file.status_flags |= crate::posix::O_CLOEXEC;
                }
            } else {
                return Err(SyscallError::OutOfMemory);
            }
        }

        // Allocate file descriptor
        let fd = crate::process::fdalloc(file_idx).ok_or(SyscallError::OutOfMemory)?;

        Ok(fd as u64)
    } else {
        // Modify existing signalfd
        // Get file index from fd
        let file_idx = crate::process::fdlookup(fd).ok_or(SyscallError::BadFileDescriptor)?;

        // Check if it's a signalfd file
        {
            let table = crate::fs::file::FILE_TABLE.lock();
            let file = table.get(file_idx).ok_or(SyscallError::BadFileDescriptor)?;
            if file.ftype != crate::fs::file::FileType::Signalfd {
                return Err(SyscallError::InvalidArgument);
            }
        }

        // Update the mask
        let table = crate::fs::file::FILE_TABLE.lock();
        let file = table.get(file_idx).ok_or(SyscallError::BadFileDescriptor)?;
        if let Some(instance_idx) = file.signalfd_instance {
            if let Some(instance) = get_signalfd_instance(instance_idx) {
                instance.update_mask(mask);
                Ok(fd as u64)
            } else {
                Err(SyscallError::InvalidArgument)
            }
        } else {
            Err(SyscallError::InvalidArgument)
        }
    }
}

/// Allocate an inotify instance and return file descriptor
fn alloc_inotify_instance(flags: i32) -> Option<usize> {
    let mut instances = INOTIFY_INSTANCES.lock();
    let instance = InotifyInstance::new(flags);

    // Find free slot or extend vector
    for (i, slot) in instances.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(instance);
            return Some(i);
        }
    }

    // No free slot, add new one
    instances.push(Some(instance));
    Some(instances.len() - 1)
}

/// Get inotify instance by index
pub fn get_inotify_instance(idx: usize) -> Option<&'static mut InotifyInstance> {
    let mut instances = INOTIFY_INSTANCES.lock();
    if let Some(ref mut instance) = instances.get_mut(idx) {
        // Convert to static mutable reference (unsafe but controlled)
        unsafe {
            let ptr = instance.as_mut().unwrap() as *mut InotifyInstance;
            Some(&mut *ptr)
        }
    } else {
        None
    }
}

/// Allocate an eventfd instance and return index
fn alloc_eventfd_instance(initval: u32, flags: i32) -> Option<usize> {
    let mut instances = EVENTFD_INSTANCES.lock();
    let instance = EventFdInstance::new(initval, flags);

    // Find free slot or extend vector
    for (i, slot) in instances.iter_mut().enumerate() {
        if slot.is_none() {
            *slot = Some(instance);
            return Some(i);
        }
    }

    // No free slot, add new one
    instances.push(Some(instance));
    Some(instances.len() - 1)
}

/// Get eventfd instance by index
pub fn get_eventfd_instance(idx: usize) -> Option<&'static mut EventFdInstance> {
    let mut instances = EVENTFD_INSTANCES.lock();
    if let Some(ref mut instance) = instances.get_mut(idx) {
        // Convert to static mutable reference (unsafe but controlled)
        unsafe {
            let ptr = instance.as_mut().unwrap() as *mut EventFdInstance;
            Some(&mut *ptr)
        }
    } else {
        None
    }
}

fn sys_inotify_init(_args: &[u64]) -> SyscallResult {
    sys_inotify_init1(&[0])
}

fn sys_inotify_init1(args: &[u64]) -> SyscallResult {
    if args.len() < 1 {
        return Err(SyscallError::InvalidArgument);
    }

    let flags = args[0] as i32;

    // Allocate inotify instance
    let instance_idx = alloc_inotify_instance(flags).ok_or(SyscallError::OutOfMemory)?;

    // Create file in file table
    let file_idx = crate::fs::file::file_alloc().ok_or(SyscallError::OutOfMemory)?;

    // Initialize file as inotify type
    {
        let mut table = crate::fs::file::FILE_TABLE.lock();
        if let Some(file) = table.get_mut(file_idx) {
            file.ftype = crate::fs::file::FileType::Inotify;
            file.readable = true;
            file.writable = false;
            file.inotify_instance = Some(instance_idx);
        } else {
            return Err(SyscallError::OutOfMemory);
        }
    }

    // Allocate file descriptor
    let fd = crate::process::fdalloc(file_idx).ok_or(SyscallError::OutOfMemory)?;

    Ok(fd as u64)
}

fn sys_inotify_add_watch(args: &[u64]) -> SyscallResult {
    if args.len() < 3 {
        return Err(SyscallError::InvalidArgument);
    }

    let fd = args[0] as i32;
    let pathname_ptr = args[1] as usize;
    let mask = args[2] as u32;

    // Get current process pagetable
    let pagetable = crate::process::myproc()
        .and_then(|pid| {
            let table = crate::process::PROC_TABLE.lock();
            table.find_ref(pid).map(|proc| proc.pagetable)
        })
        .ok_or(SyscallError::BadFileDescriptor)?;

    // Get file index from fd
    let file_idx = crate::process::fdlookup(fd).ok_or(SyscallError::BadFileDescriptor)?;

    // Check if it's an inotify file
    {
        let table = crate::fs::file::FILE_TABLE.lock();
        let file = table.get(file_idx).ok_or(SyscallError::BadFileDescriptor)?;
        if file.ftype != crate::fs::file::FileType::Inotify {
            return Err(SyscallError::InvalidArgument);
        }
    }

    // Read pathname from user space
    const MAX_PATH_LEN: usize = 512;
    let mut path_buf = [0u8; MAX_PATH_LEN];
    let path_len = unsafe {
        crate::subsystems::mm::vm::copyinstr(pagetable, pathname_ptr, path_buf.as_mut_ptr(), MAX_PATH_LEN)
            .map_err(|_| SyscallError::BadAddress)?
    };

    let pathname = alloc::string::String::from_utf8_lossy(&path_buf[..path_len]).into_owned();

    // Get inotify instance
    let table = crate::fs::file::FILE_TABLE.lock();
    let file = table.get(file_idx).ok_or(SyscallError::BadFileDescriptor)?;
    let instance_idx = file.inotify_instance.ok_or(SyscallError::InvalidArgument)?;

    if let Some(instance) = get_inotify_instance(instance_idx) {
        let wd = instance.add_watch(&pathname, mask)?;
        Ok(wd as u64)
    } else {
        Err(SyscallError::InvalidArgument)
    }
}

fn sys_inotify_rm_watch(args: &[u64]) -> SyscallResult {
    if args.len() < 2 {
        return Err(SyscallError::InvalidArgument);
    }

    let fd = args[0] as i32;
    let wd = args[1] as i32;

    // Get file index from fd
    let file_idx = crate::process::fdlookup(fd).ok_or(SyscallError::BadFileDescriptor)?;

    // Check if it's an inotify file
    {
        let table = crate::fs::file::FILE_TABLE.lock();
        let file = table.get(file_idx).ok_or(SyscallError::BadFileDescriptor)?;
        if file.ftype != crate::fs::file::FileType::Inotify {
            return Err(SyscallError::InvalidArgument);
        }
    }

    // Get inotify instance
    let table = crate::fs::file::FILE_TABLE.lock();
    let file = table.get(file_idx).ok_or(SyscallError::BadFileDescriptor)?;
    let instance_idx = file.inotify_instance.ok_or(SyscallError::InvalidArgument)?;

    if let Some(instance) = get_inotify_instance(instance_idx) {
        instance.rm_watch(wd)?;
        Ok(0)
    } else {
        Err(SyscallError::InvalidArgument)
    }
}

// ============================================================================
// MemFd Test Module
// ============================================================================

#[cfg(test)]
pub mod memfd_test {
    use super::*;
    use crate::syscalls;
    
    // Simple test result type
    pub type TestResult = Result<(), &'static str>;
    
    // Simple test macros
    macro_rules! test_assert {
        ($cond:expr, $msg:expr) => {
            if !($cond) {
                return Err($msg);
            }
        };
    }
    
    macro_rules! test_assert_eq {
        ($left:expr, $right:expr, $msg:expr) => {
            if ($left) != ($right) {
                return Err($msg);
            }
        };
    }
    
    macro_rules! test_assert_ne {
        ($left:expr, $right:expr, $msg:expr) => {
            if ($left) == ($right) {
                return Err($msg);
            }
        };
    }

    /// Run all memfd tests
    pub fn run_all_memfd_tests() -> TestResult {
        test_memfd_create_basic()?;
        test_memfd_create_flags()?;
        test_memfd_read_write()?;
        test_memfd_sealing()?;
        test_memfd_truncate()?;
        test_memfd_close_on_exec()?;
        crate::println!("All memfd_create tests passed");
        Ok(())
    }

    /// Test basic memfd_create functionality
    pub fn test_memfd_create_basic() -> TestResult {
        crate::println!("Testing basic memfd_create...");
        
        // Test memfd_create with empty name
        let result = syscalls::dispatch(0xB001, &[0u64, 0u64]); // memfd_create("", 0)
        test_assert!(result > 0, "memfd_create with empty name should succeed");
        
        let fd = result as i32;
        
        // Test memfd_create with name
        let name_ptr = "test_memfd\0".as_ptr() as usize;
        let result2 = syscalls::dispatch(0xB001, &[name_ptr as u64, 0u64]);
        test_assert!(result2 > 0, "memfd_create with name should succeed");
        
        let fd2 = result2 as i32;
        
        // File descriptors should be different
        test_assert_ne!(fd, fd2, "Different memfd_create calls should return different FDs");
        
        crate::println!("✓ Basic memfd_create test passed");
        Ok(())
    }

    /// Test memfd_create with different flags
    pub fn test_memfd_create_flags() -> TestResult {
        crate::println!("Testing memfd_create flags...");
        
        // Test MFD_CLOEXEC
        let name_ptr = "test_cloexec\0".as_ptr() as usize;
        let result = syscalls::dispatch(0xB001, &[name_ptr as u64, memfd_flags::MFD_CLOEXEC as u64]);
        test_assert!(result > 0, "memfd_create with MFD_CLOEXEC should succeed");
        
        // Test MFD_ALLOW_SEALING
        let name_ptr2 = "test_sealing\0".as_ptr() as usize;
        let result2 = syscalls::dispatch(0xB001, &[name_ptr2 as u64, memfd_flags::MFD_ALLOW_SEALING as u64]);
        test_assert!(result2 > 0, "memfd_create with MFD_ALLOW_SEALING should succeed");
        
        // Test both flags
        let name_ptr3 = "test_both\0".as_ptr() as usize;
        let result3 = syscalls::dispatch(0xB001, &[name_ptr3 as u64, (memfd_flags::MFD_CLOEXEC | memfd_flags::MFD_ALLOW_SEALING) as u64]);
        test_assert!(result3 > 0, "memfd_create with both flags should succeed");
        
        // Test invalid flags
        let name_ptr4 = "test_invalid\0".as_ptr() as usize;
        let result4 = syscalls::dispatch(0xB001, &[name_ptr4 as u64, 0xFFFFFFFFu64]);
        test_assert!(result4 < 0, "memfd_create with invalid flags should fail");
        
        crate::println!("✓ Memfd_create flags test passed");
        Ok(())
    }

    /// Test memfd read/write operations
    pub fn test_memfd_read_write() -> TestResult {
        crate::println!("Testing memfd read/write...");
        
        // Create a memfd with sealing allowed
        let name_ptr = "test_rw\0".as_ptr() as usize;
        let result = syscalls::dispatch(0xB001, &[name_ptr as u64, memfd_flags::MFD_ALLOW_SEALING as u64]);
        test_assert!(result > 0, "memfd_create for read/write test should succeed");
        
        let fd = result as i32;
        
        // Write data to memfd
        let test_data = b"Hello, memfd!";
        let write_result = syscalls::dispatch(0x2001, &[fd as u64, test_data.as_ptr() as u64, test_data.len() as u64]); // write
        test_assert!(write_result > 0, "Write to memfd should succeed");
        
        // Read data back
        let mut read_buf = [0u8; 64];
        let read_result = syscalls::dispatch(0x2000, &[fd as u64, read_buf.as_mut_ptr() as u64, read_buf.len() as u64]); // read
        test_assert!(read_result > 0, "Read from memfd should succeed");
        
        let bytes_read = read_result as usize;
        test_assert_eq!(bytes_read, test_data.len(), "Should read same number of bytes as written");
        test_assert_eq!(&read_buf[..bytes_read], test_data, "Read data should match written data");
        
        crate::println!("✓ Memfd read/write test passed");
        Ok(())
    }

    /// Test memfd sealing operations
    pub fn test_memfd_sealing() -> TestResult {
        crate::println!("Testing memfd sealing...");
        
        // Create a memfd with sealing allowed
        let name_ptr = "test_seal\0".as_ptr() as usize;
        let result = syscalls::dispatch(0xB001, &[name_ptr as u64, memfd_flags::MFD_ALLOW_SEALING as u64]);
        test_assert!(result > 0, "memfd_create for sealing test should succeed");
        
        let fd = result as i32;
        
        // Write some data first
        let test_data = b"Seal test data";
        let write_result = syscalls::dispatch(0x2001, &[fd as u64, test_data.as_ptr() as u64, test_data.len() as u64]);
        test_assert!(write_result > 0, "Write before sealing should succeed");
        
        // Add F_SEAL_WRITE seal
        let fcntl_cmd = 1033; // F_ADD_SEALS (Linux)
        let seals = fcntl_seals::F_SEAL_WRITE;
        let seal_result = syscalls::dispatch(0x2006, &[fd as u64, fcntl_cmd as u64, seals as u64]); // fcntl
        test_assert!(seal_result >= 0, "Adding F_SEAL_WRITE should succeed");
        
        // Try to write again - should fail
        let more_data = b"This should fail";
        let write_result2 = syscalls::dispatch(0x2001, &[fd as u64, more_data.as_ptr() as u64, more_data.len() as u64]);
        test_assert!(write_result2 < 0, "Write after F_SEAL_WRITE should fail");
        
        // Read should still work
        let mut read_buf = [0u8; 64];
        let read_result = syscalls::dispatch(0x2000, &[fd as u64, read_buf.as_mut_ptr() as u64, read_buf.len() as u64]);
        test_assert!(read_result > 0, "Read after sealing should still work");
        
        crate::println!("✓ Memfd sealing test passed");
        Ok(())
    }

    /// Test memfd truncate operations
    pub fn test_memfd_truncate() -> TestResult {
        crate::println!("Testing memfd truncate...");
        
        // Create a memfd
        let name_ptr = "test_truncate\0".as_ptr() as usize;
        let result = syscalls::dispatch(0xB001, &[name_ptr as u64, memfd_flags::MFD_ALLOW_SEALING as u64]);
        test_assert!(result > 0, "memfd_create for truncate test should succeed");
        
        let fd = result as i32;
        
        // Write some data
        let test_data = b"Truncate test data - this is longer";
        let write_result = syscalls::dispatch(0x2001, &[fd as u64, test_data.as_ptr() as u64, test_data.len() as u64]);
        test_assert!(write_result > 0, "Write before truncate should succeed");
        
        // Truncate file
        let truncate_cmd = 93; // ftruncate
        let new_size = 10u64;
        let truncate_result = syscalls::dispatch(0x2006, &[fd as u64, truncate_cmd as u64, new_size]); // fcntl/ftruncate
        test_assert!(truncate_result >= 0, "Truncate should succeed");
        
        // Read back - should only get truncated amount
        let mut read_buf = [0u8; 64];
        let read_result = syscalls::dispatch(0x2000, &[fd as u64, read_buf.as_mut_ptr() as u64, read_buf.len() as u64]);
        test_assert!(read_result > 0, "Read after truncate should succeed");
        
        let bytes_read = read_result as usize;
        test_assert_eq!(bytes_read, new_size as usize, "Should read truncated amount");
        
        crate::println!("✓ Memfd truncate test passed");
        Ok(())
    }

    /// Test memfd close-on-exec flag
    pub fn test_memfd_close_on_exec() -> TestResult {
        crate::println!("Testing memfd close-on-exec...");
        
        // Create a memfd with MFD_CLOEXEC
        let name_ptr = "test_cloexec\0".as_ptr() as usize;
        let result = syscalls::dispatch(0xB001, &[name_ptr as u64, memfd_flags::MFD_CLOEXEC as u64]);
        test_assert!(result > 0, "memfd_create with MFD_CLOEXEC should succeed");
        
        let fd = result as i32;
        
        // Check if O_CLOEXEC flag is set (using fcntl with F_GETFD)
        let getfd_cmd = 1; // F_GETFD
        let flag_result = syscalls::dispatch(0x2006, &[fd as u64, getfd_cmd as u64, 0u64]); // fcntl
        test_assert!(flag_result >= 0, "Getting file descriptor flags should succeed");
        
        let flags = flag_result as i32;
        test_assert!((flags & crate::posix::O_CLOEXEC) != 0, "O_CLOEXEC flag should be set");
        
        crate::println!("✓ Memfd close-on-exec test passed");
        Ok(())
    }
}