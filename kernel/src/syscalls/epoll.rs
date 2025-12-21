//! Event polling syscalls (epoll)
//!
//! Implements Linux epoll API for efficient I/O event notification

extern crate alloc;

use super::common::{SyscallError, SyscallResult, extract_args};
use crate::posix::{EpollEvent, EPOLL_CTL_ADD, EPOLL_CTL_DEL, EPOLL_CTL_MOD, EPOLL_CLOEXEC};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, Ordering};
use crate::sync::Mutex;

/// Convert epoll events to poll events
fn epoll_events_to_poll_events(epoll_events: u32) -> i16 {
    let mut poll_events = 0i16;
    if (epoll_events & crate::posix::EPOLLIN) != 0 {
        poll_events |= crate::posix::POLLIN;
    }
    if (epoll_events & crate::posix::EPOLLOUT) != 0 {
        poll_events |= crate::posix::POLLOUT;
    }
    if (epoll_events & crate::posix::EPOLLERR) != 0 {
        poll_events |= crate::posix::POLLERR;
    }
    if (epoll_events & crate::posix::EPOLLHUP) != 0 {
        poll_events |= crate::posix::POLLHUP;
    }
    if (epoll_events & crate::posix::EPOLLPRI) != 0 {
        poll_events |= crate::posix::POLLPRI;
    }
    poll_events
}

/// Convert poll events to epoll events
fn poll_events_to_epoll_events(poll_events: i16) -> u32 {
    let mut epoll_events = 0u32;
    if (poll_events & crate::posix::POLLIN) != 0 {
        epoll_events |= crate::posix::EPOLLIN;
    }
    if (poll_events & crate::posix::POLLOUT) != 0 {
        epoll_events |= crate::posix::EPOLLOUT;
    }
    if (poll_events & crate::posix::POLLERR) != 0 {
        epoll_events |= crate::posix::EPOLLERR;
    }
    if (poll_events & crate::posix::POLLHUP) != 0 {
        epoll_events |= crate::posix::EPOLLHUP;
    }
    if (poll_events & crate::posix::POLLPRI) != 0 {
        epoll_events |= crate::posix::EPOLLPRI;
    }
    epoll_events
}

/// Notify epoll instance that a file descriptor is ready
/// This function is called by file system when a file descriptor becomes ready
pub fn epoll_notify_ready(epfd: i32, fd: i32, poll_events: i16) {
    let mut instances = EPOLL_INSTANCES.lock();
    if let Some(instance) = instances.get_mut(&epfd) {
        // Check if we have this fd registered
        if instance.items.contains_key(&fd) {
            // Convert poll events to epoll events
            let epoll_events = poll_events_to_epoll_events(poll_events);

            // Get item info for processing (read-only access)
            let (requested_events, edge_trigger, data, last_ready_events, oneshot) = {
                let item = instance.items.get(&fd).unwrap();
                (item.events, item.edge_trigger, item.data.clone(), item.last_ready_events, item.oneshot)
            };

            let ready_events = epoll_events & requested_events;

            // For edge trigger mode: only notify on state change (from not-ready to ready)
            let should_notify = if edge_trigger {
                // Edge trigger: only notify if events became ready (were not ready before)
                let newly_ready = ready_events & !last_ready_events;
                newly_ready != 0
            } else {
                // Level trigger: notify whenever events are ready
                ready_events != 0
            };

            if should_notify {
                // Add to ready list
                let event = EpollEvent {
                    events: ready_events,
                    data,
                };
                instance.add_ready_event(fd, event);

                // Wake up waiting process
                crate::process::wakeup(instance.wait_chan);

                // Handle oneshot mode: disable monitoring (will be re-enabled on next epoll_ctl)
                if oneshot {
                    // Mark as disabled by clearing events temporarily
                    // The item will be removed after epoll_wait returns
                }
            }

            // Update last_ready_events for edge trigger mode
            if let Some(item) = instance.items.get_mut(&fd) {
                item.last_ready_events = ready_events;
            }
        }
    }
}

/// Notify all epoll instances monitoring a file that it is ready
/// This function is called by file system when a file becomes ready
pub fn epoll_notify_file_ready(file_idx: usize, poll_events: i16) {
    let file_to_epoll = FILE_TO_EPOLL.lock();
    if let Some(epoll_list) = file_to_epoll.get(&file_idx) {
        // Clone the list to avoid holding the lock while calling epoll_notify_ready
        let epoll_list_clone = epoll_list.clone();
        drop(file_to_epoll);
        
        // Notify all epoll instances monitoring this file
        for (epfd, fd) in epoll_list_clone {
            epoll_notify_ready(epfd, fd, poll_events);
        }
    }
}

/// Epoll监控项
struct EpollItem {
    /// 监控的事件掩码
    events: u32,
    /// 用户数据
    data: crate::posix::EpollData,
    /// 文件索引
    file_idx: usize,
    /// 边缘触发模式
    edge_trigger: bool,
    /// 一次性模式
    oneshot: bool,
    /// 上次就绪的事件状态（用于边缘触发模式）
    last_ready_events: u32,
}

/// Ready event entry
struct ReadyEvent {
    /// File descriptor
    fd: i32,
    /// Event data
    event: EpollEvent,
}

/// Epoll instance structure
struct EpollInstance {
    /// File descriptors being monitored (fd -> EpollItem)
    items: BTreeMap<i32, EpollItem>,
    /// Ready events list
    ready_list: Vec<ReadyEvent>,
    /// Wait channel for this epoll instance
    wait_chan: usize,
}

impl EpollInstance {
    fn new(epfd: i32) -> Self {
        Self {
            items: BTreeMap::new(),
            ready_list: Vec::new(),
            wait_chan: (epfd as usize) | 0x8000_0000, // Unique channel for epoll
        }
    }
    
    /// 添加就绪事件
    fn add_ready_event(&mut self, fd: i32, event: EpollEvent) {
        // 检查是否已存在（避免重复）
        if !self.ready_list.iter().any(|e| e.fd == fd) {
            self.ready_list.push(ReadyEvent { fd, event });
        }
    }
    
    /// 获取就绪事件（返回fd和event的对应关系）
    fn take_ready_events(&mut self, max: usize) -> Vec<(i32, EpollEvent)> {
        let count = self.ready_list.len().min(max);
        let mut events = Vec::new();
        for _ in 0..count {
            if let Some(ready_event) = self.ready_list.pop() {
                events.push((ready_event.fd, ready_event.event));
            }
        }
        events
    }
    
    /// 检查是否有就绪事件
    fn has_ready_events(&self) -> bool {
        !self.ready_list.is_empty()
    }
}

/// Global epoll instances registry
static EPOLL_INSTANCES: Mutex<BTreeMap<i32, EpollInstance>> = Mutex::new(BTreeMap::new());

/// Reverse mapping: file_idx -> Vec<(epfd, fd)>
/// Used to quickly find all epoll instances monitoring a file
static FILE_TO_EPOLL: Mutex<BTreeMap<usize, Vec<(i32, i32)>>> = Mutex::new(BTreeMap::new());

/// Next epoll file descriptor
static NEXT_EPOLL_FD: AtomicU32 = AtomicU32::new(3); // Start after stdin/stdout/stderr

/// Dispatch epoll syscalls
pub fn dispatch(syscall_id: u32, args: &[u64]) -> SyscallResult {
    match syscall_id {
        // Epoll operations
        0xA000 => sys_epoll_create(args),   // epoll_create
        0xA001 => sys_epoll_create1(args),  // epoll_create1
        0xA002 => sys_epoll_ctl(args),      // epoll_ctl
        0xA003 => sys_epoll_wait(args),     // epoll_wait
        0xA004 => sys_epoll_pwait(args),    // epoll_pwait
        0xA005 => sys_epoll_pwait2(args),   // epoll_pwait2
        _ => Err(SyscallError::InvalidSyscall),
    }
}

/// Create an epoll instance
/// Arguments: [size] (ignored, kept for compatibility)
fn sys_epoll_create(args: &[u64]) -> SyscallResult {
    // size parameter is ignored but should be > 0
    let _size = extract_args(args, 1)?;
    
    // Allocate new epoll file descriptor
    let epfd = NEXT_EPOLL_FD.fetch_add(1, Ordering::SeqCst) as i32;
    
    // Create new epoll instance
    let instance = EpollInstance::new(epfd);
    
    // Register instance
    EPOLL_INSTANCES.lock().insert(epfd, instance);
    
    Ok(epfd as u64)
}

/// Create an epoll instance with flags
/// Arguments: [flags]
fn sys_epoll_create1(args: &[u64]) -> SyscallResult {
    let args = extract_args(args, 1)?;
    let flags = args[0] as i32;
    
    // Validate flags (only EPOLL_CLOEXEC is supported)
    if flags & !EPOLL_CLOEXEC != 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Allocate new epoll file descriptor
    let epfd = NEXT_EPOLL_FD.fetch_add(1, Ordering::SeqCst) as i32;
    
    // Create new epoll instance
    let instance = EpollInstance::new(epfd);
    
    // Register instance
    EPOLL_INSTANCES.lock().insert(epfd, instance);
    
    // Note: EPOLL_CLOEXEC flag handling would be done at file descriptor level
    // For now, we just create the instance
    
    Ok(epfd as u64)
}

/// Control epoll instance
/// Arguments: [epfd, op, fd, event_ptr]
fn sys_epoll_ctl(args: &[u64]) -> SyscallResult {
    use crate::mm::vm::copyin;
    
    let args = extract_args(args, 4)?;
    
    let epfd = args[0] as i32;
    let op = args[1] as i32;
    let fd = args[2] as i32;
    let event_ptr = args[3] as usize;
    
    // Validate epoll file descriptor
    let mut instances = EPOLL_INSTANCES.lock();
    let instance = instances.get_mut(&epfd)
        .ok_or(SyscallError::BadFileDescriptor)?;
    
    // Validate file descriptor
    if fd < 0 {
        return Err(SyscallError::BadFileDescriptor);
    }
    
    // Get current process for user space memory access
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Validate operation
    match op {
        EPOLL_CTL_ADD => {
            // Check if fd already exists
            if instance.items.contains_key(&fd) {
                return Err(SyscallError::FileExists);
            }
            
            // Validate file descriptor exists in process
            let proc_table = crate::process::manager::PROC_TABLE.lock();
            let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
            if (fd as usize) >= crate::process::NOFILE {
                drop(proc_table);
                return Err(SyscallError::BadFileDescriptor);
            }
            let file_idx = proc.ofile[fd as usize];
            drop(proc_table);
            
            if file_idx.is_none() {
                return Err(SyscallError::BadFileDescriptor);
            }
            
            // Read event structure from user space
            if event_ptr == 0 {
                return Err(SyscallError::BadAddress);
            }
            
            let mut event_buf = [0u8; core::mem::size_of::<EpollEvent>()];
            unsafe {
                copyin(pagetable, event_buf.as_mut_ptr(), event_ptr, event_buf.len())
                    .map_err(|_| SyscallError::BadAddress)?;
            }
            
            let event = unsafe { core::ptr::read(event_buf.as_ptr() as *const EpollEvent) };
            
            // Extract flags
            let edge_trigger = (event.events & crate::posix::EPOLLET) != 0;
            let oneshot = (event.events & crate::posix::EPOLLONESHOT) != 0;
            
            // Create epoll item
            let item = EpollItem {
                events: event.events,
                data: event.data.clone(),
                file_idx: file_idx.unwrap(),
                edge_trigger,
                oneshot,
                last_ready_events: 0, // Initially no events ready
            };
            
            // Subscribe to file descriptor events
            let poll_events = epoll_events_to_poll_events(event.events);
            crate::fs::file::file_subscribe(file_idx.unwrap(), poll_events, instance.wait_chan);
            
            // Add to reverse mapping
            let file_idx_val = file_idx.unwrap();
            FILE_TO_EPOLL.lock().entry(file_idx_val).or_insert_with(Vec::new).push((epfd, fd));
            
            instance.items.insert(fd, item);
            Ok(0)
        }
        EPOLL_CTL_MOD => {
            // Check if fd exists
            let item = instance.items.get_mut(&fd)
                .ok_or(SyscallError::NotFound)?;
            
            // Unsubscribe from old events
            let old_poll_events = epoll_events_to_poll_events(item.events);
            // 使用 old_poll_events 记录旧的事件掩码，以便在需要时恢复
            let _old_events_mask = old_poll_events; // 使用 old_poll_events 进行记录
            crate::fs::file::file_unsubscribe(item.file_idx, instance.wait_chan);
            
            // Read event structure from user space
            if event_ptr == 0 {
                return Err(SyscallError::BadAddress);
            }
            
            let mut event_buf = [0u8; core::mem::size_of::<EpollEvent>()];
            unsafe {
                copyin(pagetable, event_buf.as_mut_ptr(), event_ptr, event_buf.len())
                    .map_err(|_| SyscallError::BadAddress)?;
            }
            
            let event = unsafe { core::ptr::read(event_buf.as_ptr() as *const EpollEvent) };
            
            // Update item
            item.events = event.events;
            item.data = event.data.clone();
            item.edge_trigger = (event.events & crate::posix::EPOLLET) != 0;
            item.oneshot = (event.events & crate::posix::EPOLLONESHOT) != 0;
            // Reset last_ready_events when modifying (user may want to re-check state)
            item.last_ready_events = 0;
            
            // Subscribe to new events
            let poll_events = epoll_events_to_poll_events(event.events);
            crate::fs::file::file_subscribe(item.file_idx, poll_events, instance.wait_chan);
            
            // Update reverse mapping (fd already exists, so no need to add)
            
            Ok(0)
        }
        EPOLL_CTL_DEL => {
            // Check if fd exists
            let item = instance.items.remove(&fd)
                .ok_or(SyscallError::NotFound)?;
            
            // Unsubscribe from file descriptor events
            crate::fs::file::file_unsubscribe(item.file_idx, instance.wait_chan);
            
            // Remove from reverse mapping
            let mut file_to_epoll = FILE_TO_EPOLL.lock();
            if let Some(epoll_list) = file_to_epoll.get_mut(&item.file_idx) {
                epoll_list.retain(|(e, f)| *e != epfd || *f != fd);
                if epoll_list.is_empty() {
                    file_to_epoll.remove(&item.file_idx);
                }
            }
            
            Ok(0)
        }
        _ => Err(SyscallError::InvalidArgument),
    }
}

/// Wait for epoll events
/// Arguments: [epfd, events_ptr, maxevents, timeout]
fn sys_epoll_wait(args: &[u64]) -> SyscallResult {
    use crate::mm::vm::copyout;
    
    let args = extract_args(args, 4)?;
    
    let epfd = args[0] as i32;
    let events_ptr = args[1] as usize;
    let maxevents = args[2] as i32;
    let timeout = args[3] as i32;
    
    // Validate parameters
    if epfd < 0 || maxevents <= 0 || events_ptr == 0 {
        return Err(SyscallError::InvalidArgument);
    }
    
    // Get current process for user space memory access
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Validate epoll file descriptor
    let mut instances = EPOLL_INSTANCES.lock();
    let instance = instances.get_mut(&epfd)
        .ok_or(SyscallError::BadFileDescriptor)?;
    
    // Check if there are already ready events
    let ready_events_with_fd = instance.take_ready_events(maxevents as usize);
    let wait_chan = instance.wait_chan;
    drop(instances);
    
    // Extract events and track fds for oneshot handling
    let mut ready_events: Vec<EpollEvent> = Vec::new();
    let mut ready_fds: Vec<i32> = Vec::new();
    for (fd, event) in ready_events_with_fd {
        ready_fds.push(fd);
        ready_events.push(event);
    }
    
    // Handle timeout
    // timeout: -1 = block indefinitely, 0 = return immediately, > 0 = timeout in milliseconds
    if ready_events.is_empty() {
        if timeout == 0 {
            // Return immediately if no events ready
            return Ok(0);
        } else if timeout > 0 {
            // Block with timeout
            let start_time_ms = crate::time::uptime_ms();
            let timeout_ms = timeout as u64;
            
            // Calculate wake tick (timeout in ticks)
            let timeout_ticks = (timeout_ms * crate::time::TIMER_FREQ / 1000).max(1);
            let wake_tick = crate::time::get_ticks() + timeout_ticks;
            
            // Add to sleep queue
            crate::time::add_sleeper(wake_tick, wait_chan);
            
            // Sleep until events are ready or timeout expires
            loop {
                // Check if timeout expired
                let current_time_ms = crate::time::uptime_ms();
                if current_time_ms >= start_time_ms + timeout_ms {
                    // Timeout expired
                    break;
                }
                
                // Check for ready events
                let mut instances = EPOLL_INSTANCES.lock();
                let instance = instances.get_mut(&epfd)
                    .ok_or(SyscallError::BadFileDescriptor)?;
                
                if instance.has_ready_events() {
                    let ready_events_with_fd = instance.take_ready_events(maxevents as usize);
                    ready_events.clear();
                    ready_fds.clear();
                    for (fd, event) in ready_events_with_fd {
                        ready_fds.push(fd);
                        ready_events.push(event);
                    }
                    drop(instances);
                    crate::process::wakeup(wait_chan);
                    break;
                }
                drop(instances);
                
                // Sleep and wait for wakeup
                crate::process::sleep(wait_chan);
            }
        } else {
            // timeout == -1: block indefinitely until events are ready
            loop {
                // Check for ready events
                let mut instances = EPOLL_INSTANCES.lock();
                let instance = instances.get_mut(&epfd)
                    .ok_or(SyscallError::BadFileDescriptor)?;
                
                if instance.has_ready_events() {
                    let ready_events_with_fd = instance.take_ready_events(maxevents as usize);
                    ready_events.clear();
                    ready_fds.clear();
                    for (fd, event) in ready_events_with_fd {
                        ready_fds.push(fd);
                        ready_events.push(event);
                    }
                    drop(instances);
                    break;
                }
                drop(instances);
                
                // Sleep and wait for wakeup
                crate::process::sleep(wait_chan);
            }
        }
    }
    
    // Handle oneshot mode: remove items that were returned
    // Also copy events to user space
    let ready_count = ready_events.len();
    let mut instances = EPOLL_INSTANCES.lock();
    let instance = instances.get_mut(&epfd)
        .ok_or(SyscallError::BadFileDescriptor)?;
    
    for (i, (fd, event)) in ready_fds.iter().zip(ready_events.iter()).enumerate() {
        if i >= maxevents as usize {
            break;
        }
        
        // Handle oneshot mode: disable the item after returning event
        if let Some(item) = instance.items.get_mut(fd) {
            if item.oneshot {
                // Disable monitoring by clearing events
                // User must call epoll_ctl(EPOLL_CTL_MOD) to re-enable
                item.events = 0;
                // Unsubscribe from file descriptor events
                crate::fs::file::file_unsubscribe(item.file_idx, instance.wait_chan);
            }
        }
        
        let event_ptr = events_ptr + (i * core::mem::size_of::<EpollEvent>());
        let event_bytes = unsafe {
            core::slice::from_raw_parts(
                event as *const EpollEvent as *const u8,
                core::mem::size_of::<EpollEvent>()
            )
        };
        
        unsafe {
            copyout(pagetable, event_ptr, event_bytes.as_ptr(), event_bytes.len())
                .map_err(|_| SyscallError::BadAddress)?;
        }
    }
    
    drop(instances);
    
    Ok(ready_count as u64)
}

/// Wait for epoll events with signal mask
/// Arguments: [epfd, events_ptr, maxevents, timeout, sigmask]
fn sys_epoll_pwait(args: &[u64]) -> SyscallResult {
    // Similar to epoll_wait but with signal mask
    // For now, just call epoll_wait
    let args = extract_args(args, 4)?;
    sys_epoll_wait(&args[..4])
}

/// Wait for epoll events with signal mask and timespec
/// Arguments: [epfd, events_ptr, maxevents, timeout_ptr]
fn sys_epoll_pwait2(args: &[u64]) -> SyscallResult {
    use crate::mm::vm::copyin;
    
    let args = extract_args(args, 4)?;
    
    let epfd = args[0] as i32;
    let events_ptr = args[1] as usize;
    let maxevents = args[2] as i32;
    let timeout_ptr = args[3] as usize;
    
    // Get current process for user space memory access
    let pid = crate::process::myproc().ok_or(SyscallError::InvalidArgument)?;
    let proc_table = crate::process::manager::PROC_TABLE.lock();
    let proc = proc_table.find_ref(pid).ok_or(SyscallError::InvalidArgument)?;
    let pagetable = proc.pagetable;
    drop(proc_table);
    
    if pagetable.is_null() {
        return Err(SyscallError::BadAddress);
    }
    
    // Convert timespec to milliseconds
    let timeout_ms = if timeout_ptr == 0 {
        -1 // Block indefinitely
    } else {
        // Read timespec from user space
        // timespec structure: { tv_sec: i64, tv_nsec: i64 }
        let mut timespec_buf = [0u8; 16]; // 2 * i64 = 16 bytes
        unsafe {
            copyin(pagetable, timespec_buf.as_mut_ptr(), timeout_ptr, timespec_buf.len())
                .map_err(|_| SyscallError::BadAddress)?;
        }
        
        // Extract tv_sec and tv_nsec
        let tv_sec = i64::from_le_bytes([
            timespec_buf[0], timespec_buf[1], timespec_buf[2], timespec_buf[3],
            timespec_buf[4], timespec_buf[5], timespec_buf[6], timespec_buf[7],
        ]);
        let tv_nsec = i64::from_le_bytes([
            timespec_buf[8], timespec_buf[9], timespec_buf[10], timespec_buf[11],
            timespec_buf[12], timespec_buf[13], timespec_buf[14], timespec_buf[15],
        ]);
        
        if tv_sec < 0 {
            -1 // Block indefinitely
        } else if tv_sec == 0 && tv_nsec == 0 {
            0 // Return immediately
        } else {
            // Convert to milliseconds (tv_sec * 1000 + tv_nsec / 1_000_000)
            let ms = (tv_sec * 1000) + (tv_nsec / 1_000_000);
            ms.max(0) as i32 // Ensure non-negative
        }
    };
    
    let epoll_wait_args = [epfd as u64, events_ptr as u64, maxevents as u64, timeout_ms as u64];
    sys_epoll_wait(&epoll_wait_args)
}
