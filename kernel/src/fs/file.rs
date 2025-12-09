// File abstraction for xv6-rust
// Provides unified file interface for regular files, devices, and pipes

extern crate alloc;
use alloc::sync::Arc;
use alloc::vec::Vec;
use crate::sync::{Mutex, Sleeplock};
use crate::process;
use crate::posix;
use crate::ipc::pipe::Pipe;

/// Maximum open files per process
pub const NOFILE: usize = 16;

/// Maximum open files system-wide
pub const NFILE: usize = 100;

/// File types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    None,
    Pipe,
    Inode,
    Device,
    Vfs,
    Socket,
    Inotify,
    EventFd,
    Signalfd,
    TimerFd,
    MemFd,
}

impl Default for FileType {
    fn default() -> Self {
        Self::None
    }
}

/// File structure
pub struct File {
    pub ftype: FileType,
    pub ref_count: i32,
    pub readable: bool,
    pub writable: bool,
    pub status_flags: i32,
    
    // For FD_INODE
    pub inode: Option<u32>,  // Inode number
    pub offset: usize,       // Current offset
    
    // For FD_PIPE
    pub pipe: Option<Arc<Sleeplock<Pipe>>>,
    
    // For FD_DEVICE
    pub major: i16,
    pub minor: i16,
    
    // For VFS file
    pub vfs_file: Option<crate::vfs::VfsFile>,
    pub event_subs: Vec<usize>,

    // For Socket
    pub socket: Option<crate::net::socket::Socket>,

    // For Inotify
    pub inotify_instance: Option<usize>,

    // For EventFd
    pub eventfd_instance: Option<usize>,

    // For Signalfd
    pub signalfd_instance: Option<usize>,

    // For TimerFd
    pub timerfd_instance: Option<usize>,

    // For MemFd
    pub memfd_instance: Option<usize>,
}

impl Default for File {
    fn default() -> Self {
        Self {
            ftype: FileType::None,
            ref_count: 0,
            readable: false,
            writable: false,
            status_flags: 0,
            inode: None,
            offset: 0,
            pipe: None,
            major: 0,
            minor: 0,
            vfs_file: None,
            event_subs: Vec::new(),
            socket: None,
            inotify_instance: None,
            eventfd_instance: None,
            signalfd_instance: None,
            timerfd_instance: None,
            memfd_instance: None,
        }
    }
}

impl File {
    pub const fn new() -> Self {
        Self {
            ftype: FileType::None,
            ref_count: 0,
            readable: false,
            writable: false,
            status_flags: 0,
            inode: None,
            offset: 0,
            pipe: None,
            major: 0,
            minor: 0,
            vfs_file: None,
            event_subs: Vec::new(),
            socket: None,
            inotify_instance: None,
            eventfd_instance: None,
            signalfd_instance: None,
            timerfd_instance: None,
            memfd_instance: None,
        }
    }

    /// Check if file is valid
    pub fn is_valid(&self) -> bool {
        self.ftype != FileType::None && self.ref_count > 0
    }

    /// Read data from file
    pub fn read(&mut self, buf: &mut [u8]) -> isize {
        if !self.readable {
            return -1;
        }

        match self.ftype {
            FileType::Pipe => {
                if let Some(ref pipe) = self.pipe {
                    let chan_read = Arc::as_ptr(pipe) as usize | 0x01;
                    if (self.status_flags & crate::posix::O_NONBLOCK) != 0 {
                        let mut p = pipe.lock();
                        if p.nread == p.nwrite {
                            return crate::reliability::errno::errno_neg(crate::reliability::errno::EAGAIN);
                        }
                        match p.read(buf) {
                            Ok(n) => {
                                drop(p);
                                process::wakeup(Arc::as_ptr(pipe) as usize | 0x02);
                                process::wakeup(crate::syscalls::POLL_WAKE_CHAN);
                                {
                                    let p = pipe.lock();
                                    p.notify_read_ready();
                                }
                                return n as isize;
                            }
                            Err(_) => {
                                return -1;
                            }
                        }
                    }
                    loop {
                        let mut p = pipe.lock();
                        // If buffer empty
                        if p.nread == p.nwrite {
                            if p.writeopen {
                                drop(p);
                                process::sleep(chan_read);
                                continue;
                            } else {
                                // Writer closed: EOF
                                return 0;
                            }
                        }
                        match p.read(buf) {
                            Ok(n) => {
                                drop(p);
                                process::wakeup(Arc::as_ptr(pipe) as usize | 0x02);
                                return n as isize;
                            }
                            Err(_) => {
                                return -1;
                            }
                        }
                    }
                } else {
                    -1
                }
            }
            FileType::Inode => {
                // Simplified: return 0 (EOF) for now
                0
            }
            FileType::Device => {
                // Device read - simplified
                -1
            }
            FileType::Vfs => {
                if let Some(ref mut vfs_file) = self.vfs_file {
                    let addr = buf.as_mut_ptr() as usize;
                    let len = buf.len();
                    match vfs_file.read(addr, len) {
                        Ok(n) => n as isize,
                        Err(_) => -1,
                    }
                } else {
                    -1
                }
            }
            FileType::None => -1,
            FileType::Socket => {
                if let Some(ref mut socket) = self.socket {
                    match socket {
                        crate::net::socket::Socket::Tcp(tcp_socket) => {
                            match tcp_socket.recv(buf) {
                                Ok(n) => n as isize,
                                Err(_) => crate::reliability::errno::errno_neg(crate::reliability::errno::EIO),
                            }
                        }
                        crate::net::socket::Socket::Udp(udp_socket) => {
                            match udp_socket.recv_from(buf) {
                                Ok((n, _addr)) => n as isize,
                                Err(_) => crate::reliability::errno::errno_neg(crate::reliability::errno::EIO),
                            }
                        }
                        crate::net::socket::Socket::Raw(_raw_socket) => {
                            // Raw socket read not implemented
                            crate::reliability::errno::errno_neg(crate::reliability::errno::EOPNOTSUPP)
                        }
                    }
                } else {
                    -1
                }
            },
            FileType::Inotify => {
                if let Some(instance_idx) = self.inotify_instance {
                    if let Some(instance) = crate::syscalls::glib::get_inotify_instance(instance_idx) {
                        if instance.has_events() {
                            instance.read_events(buf) as isize
                        } else {
                            // No events available
                            if (self.status_flags & crate::posix::O_NONBLOCK) != 0 {
                                crate::reliability::errno::errno_neg(crate::reliability::errno::EAGAIN)
                            } else {
                                // Block until events are available (simplified - should use proper blocking)
                                0
                            }
                        }
                    } else {
                        -1
                    }
                } else {
                    -1
                }
            },
            FileType::EventFd => {
                if let Some(instance_idx) = self.eventfd_instance {
                    if let Some(instance) = crate::syscalls::glib::get_eventfd_instance(instance_idx) {
                        let nonblock = (self.status_flags & crate::posix::O_NONBLOCK) != 0;
                        match instance.read(buf, nonblock) {
                            Ok(n) => n as isize,
                            Err(crate::syscalls::common::SyscallError::WouldBlock) => {
                                crate::reliability::errno::errno_neg(crate::reliability::errno::EAGAIN)
                            },
                            Err(crate::syscalls::common::SyscallError::InvalidArgument) => {
                                crate::reliability::errno::errno_neg(crate::reliability::errno::EINVAL)
                            },
                            Err(_) => -1,
                        }
                    } else {
                        -1
                    }
                } else {
                    -1
                }
            },
            FileType::Signalfd => {
                if let Some(instance_idx) = self.signalfd_instance {
                    if let Some(instance) = crate::syscalls::glib::get_signalfd_instance(instance_idx) {
                        let nonblock = (self.status_flags & crate::posix::O_NONBLOCK) != 0;
                        match instance.read_signals(buf, nonblock) {
                            Ok(n) => n as isize,
                            Err(crate::syscalls::common::SyscallError::WouldBlock) => {
                                crate::reliability::errno::errno_neg(crate::reliability::errno::EAGAIN)
                            },
                            Err(crate::syscalls::common::SyscallError::InvalidArgument) => {
                                crate::reliability::errno::errno_neg(crate::reliability::errno::EINVAL)
                            },
                            Err(_) => -1,
                        }
                    } else {
                        -1
                    }
                } else {
                    -1
                }
            },
            FileType::TimerFd => {
                if let Some(instance_idx) = self.timerfd_instance {
                    if let Some(instance) = crate::syscalls::glib::get_timerfd_instance(instance_idx) {
                        let nonblock = (self.status_flags & crate::posix::O_NONBLOCK) != 0;
                        match instance.read_expirations(buf, nonblock) {
                            Ok(n) => n as isize,
                            Err(crate::syscalls::common::SyscallError::WouldBlock) => {
                                crate::reliability::errno::errno_neg(crate::reliability::errno::EAGAIN)
                            },
                            Err(crate::syscalls::common::SyscallError::InvalidArgument) => {
                                crate::reliability::errno::errno_neg(crate::reliability::errno::EINVAL)
                            },
                            Err(_) => -1,
                        }
                    } else {
                        -1
                    }
                } else {
                    -1
                }
            },
            FileType::MemFd => {
                if let Some(instance_idx) = self.memfd_instance {
                    if let Some(instance) = crate::syscalls::glib::get_memfd_instance(instance_idx) {
                        // For memfd, we need to handle offset-based reads
                        // For now, implement a simple read from offset 0
                        match instance.read(0, buf) {
                            Ok(n) => n as isize,
                            Err(crate::syscalls::common::SyscallError::PermissionDenied) => {
                                crate::reliability::errno::errno_neg(crate::reliability::errno::EPERM)
                            },
                            Err(crate::syscalls::common::SyscallError::InvalidArgument) => {
                                crate::reliability::errno::errno_neg(crate::reliability::errno::EINVAL)
                            },
                            Err(_) => -1,
                        }
                    } else {
                        -1
                    }
                } else {
                    -1
                }
            },
        }
    }

    /// Write to file
    pub fn write(&mut self, buf: &[u8]) -> isize {
        if !self.writable {
            return -1;
        }

        match self.ftype {
            FileType::Pipe => {
                if let Some(ref pipe) = self.pipe {
                    let chan_write = Arc::as_ptr(pipe) as usize | 0x02;
                    if (self.status_flags & crate::posix::O_NONBLOCK) != 0 {
                        let mut p = pipe.lock();
                        if p.nwrite == p.nread + crate::ipc::pipe::PIPE_SIZE {
                            if !p.readopen {
                                let _ = crate::process::kill_proc(crate::process::getpid(), crate::ipc::signal::SIGPIPE);
                                return crate::reliability::errno::errno_neg(crate::reliability::errno::EPIPE);
                            }
                            return crate::reliability::errno::errno_neg(crate::reliability::errno::EAGAIN);
                        }
                        match p.write(buf) {
                            Ok(n) => {
                                drop(p);
                                process::wakeup(Arc::as_ptr(pipe) as usize | 0x01);
                                process::wakeup(crate::syscalls::POLL_WAKE_CHAN);
                                {
                                    let p = pipe.lock();
                                    p.notify_write_ready();
                                }
                                return n as isize;
                            }
                            Err(_) => {
                                return -1;
                            }
                        }
                    }
                    loop {
                        let mut p = pipe.lock();
                        // If buffer full
                        if p.nwrite == p.nread + crate::ipc::pipe::PIPE_SIZE {
                            if !p.readopen {
                                // No readers: send SIGPIPE and return EPIPE
                                let _ = crate::process::kill_proc(crate::process::getpid(), crate::ipc::signal::SIGPIPE);
                                return crate::reliability::errno::errno_neg(crate::reliability::errno::EPIPE);
                            }
                            drop(p);
                            process::sleep(chan_write);
                            continue;
                        }
                        match p.write(buf) {
                            Ok(n) => {
                                drop(p);
                                process::wakeup(Arc::as_ptr(pipe) as usize | 0x01);
                                return n as isize;
                            }
                            Err(_) => {
                                return -1;
                            }
                        }
                    }
                } else {
                    -1
                }
            }
            FileType::Inode => {
                // TODO: Write to inode
                if let Some(_inum) = self.inode {
                    buf.len() as isize
                } else {
                    -1
                }
            }
            FileType::Device => {
                // Write to device
                match self.major {
                    1 => {
                        // Console device
                        crate::drivers::console_write(buf) as isize
                    }
                    _ => -1,
                }
            }
            FileType::Vfs => {
                if let Some(ref mut vfs_file) = self.vfs_file {
                    // Convert buffer to address and length for VFS write
                    let addr = buf.as_ptr() as usize;
                    let len = buf.len();
                    if (self.status_flags & crate::posix::O_APPEND) != 0 {
                        // Seek to end before write
                        if let Ok(attr) = vfs_file.stat() {
                            let _ = vfs_file.seek(attr.size as usize);
                        }
                    }
                    match vfs_file.write(addr, len) {
                        Ok(n) => n as isize,
                        Err(_) => -1,
                    }
                } else {
                    -1
                }
            }
            FileType::None => -1,
            FileType::Socket => {
                if let Some(ref mut socket) = self.socket {
                    match socket {
                        crate::net::socket::Socket::Tcp(tcp_socket) => {
                            match tcp_socket.send(buf) {
                                Ok(n) => n as isize,
                                Err(_) => crate::reliability::errno::errno_neg(crate::reliability::errno::EIO),
                            }
                        }
                        crate::net::socket::Socket::Udp(udp_socket) => {
                            // For UDP sockets, need destination address
                            // For now, just return error
                            crate::reliability::errno::errno_neg(crate::reliability::errno::EDESTADDRREQ)
                        }
                        crate::net::socket::Socket::Raw(_raw_socket) => {
                            // Raw socket write not implemented
                            crate::reliability::errno::errno_neg(crate::reliability::errno::EOPNOTSUPP)
                        }
                    }
                } else {
                    -1
                }
            },
            FileType::EventFd => {
                if let Some(instance_idx) = self.eventfd_instance {
                    if let Some(instance) = crate::syscalls::glib::get_eventfd_instance(instance_idx) {
                        match instance.write(buf) {
                            Ok(n) => n as isize,
                            Err(crate::syscalls::common::SyscallError::InvalidArgument) => {
                                crate::reliability::errno::errno_neg(crate::reliability::errno::EINVAL)
                            },
                            Err(_) => -1,
                        }
                    } else {
                        -1
                    }
                } else {
                    -1
                }
            },
            FileType::Inotify | FileType::Signalfd | FileType::TimerFd => {
                // These file types don't support write operations
                crate::reliability::errno::errno_neg(crate::reliability::errno::EINVAL)
            },
            FileType::MemFd => {
                if let Some(instance_idx) = self.memfd_instance {
                    if let Some(instance) = crate::syscalls::glib::get_memfd_instance(instance_idx) {
                        // For memfd, we need to handle offset-based writes
                        // For now, implement a simple write at offset 0
                        match instance.write(0, buf) {
                            Ok(n) => n as isize,
                            Err(crate::syscalls::common::SyscallError::PermissionDenied) => {
                                crate::reliability::errno::errno_neg(crate::reliability::errno::EPERM)
                            },
                            Err(crate::syscalls::common::SyscallError::InvalidArgument) => {
                                crate::reliability::errno::errno_neg(crate::reliability::errno::EINVAL)
                            },
                            Err(_) => -1,
                        }
                    } else {
                        -1
                    }
                } else {
                    -1
                }
            },
        }
    }

    /// Seek to offset
    pub fn seek(&mut self, offset: usize) -> isize {
        match self.ftype {
            FileType::Inode => {
                self.offset = offset;
                offset as isize
            }
            FileType::Vfs => {
                if let Some(ref mut vfs_file) = self.vfs_file {
                    vfs_file.seek(offset)
                } else {
                    -1
                }
            }
            _ => -1,
        }
    }
}

/// Global file table with object pool for efficient allocation
pub struct FTable {
    files: [File; NFILE],
    /// Free list for O(1) allocation (reuses freed file slots)
    free_list: alloc::vec::Vec<usize>,
    /// Track if free_list has been initialized
    free_list_initialized: bool,
}

impl FTable {
    pub const fn new() -> Self {
        Self {
            files: [const { File::new() }; NFILE],
            free_list: alloc::vec::Vec::new(),
            free_list_initialized: false,
        }
    }
    
    /// Initialize free list with all available file slots
    /// This is called lazily on first allocation to avoid const initialization issues
    fn ensure_free_list_initialized(&mut self) {
        if !self.free_list_initialized {
            // Populate free list with all available indices
            for i in 0..NFILE {
                self.free_list.push(i);
            }
            self.free_list_initialized = true;
        }
    }

    /// Allocate a file structure using object pool
    /// 
    /// This function provides O(1) allocation by reusing freed file slots
    /// from the free_list, reducing memory fragmentation and allocation overhead.
    /// 
    /// # Returns
    /// 
    /// * `Some(file_index)` if a file slot is available
    /// * `None` if all file slots are in use
    pub fn alloc(&mut self) -> Option<usize> {
        self.ensure_free_list_initialized();
        
        // Try to get a slot from free list first (O(1))
        if let Some(idx) = self.free_list.pop() {
            let file = &mut self.files[idx];
            // Sanity check: ensure the file is actually free
            if file.ref_count == 0 {
                file.ref_count = 1;
                return Some(idx);
            }
            // If not free, continue to linear search
        }
        
        // Fallback to linear search if free_list is empty or inconsistent
        // This should rarely happen in normal operation
        for (i, file) in self.files.iter_mut().enumerate() {
            if file.ref_count == 0 {
                file.ref_count = 1;
                return Some(i);
            }
        }
        None
    }

    /// Duplicate a file (increment reference count)
    pub fn dup(&mut self, idx: usize) -> Option<usize> {
        if idx >= NFILE || self.files[idx].ref_count == 0 {
            return None;
        }
        self.files[idx].ref_count += 1;
        Some(idx)
    }

    /// Close a file (decrement reference count)
    /// 
    /// When the reference count reaches zero, the file is returned to the
    /// object pool for reuse, reducing memory fragmentation.
    pub fn close(&mut self, idx: usize) {
        if idx >= NFILE {
            return;
        }
        let file = &mut self.files[idx];
        file.ref_count -= 1;
        if file.ref_count == 0 {
            // Clean up
            if file.ftype == FileType::Pipe {
                if let Some(ref pipe) = file.pipe {
                    let mut p = pipe.lock();
                    if file.readable {
                        p.close_read();
                        drop(p);
                        process::wakeup(Arc::as_ptr(pipe) as usize | 0x02);
                        process::wakeup(crate::syscalls::POLL_WAKE_CHAN);
                    } else if file.writable {
                        p.close_write();
                        drop(p);
                        process::wakeup(Arc::as_ptr(pipe) as usize | 0x01);
                        process::wakeup(crate::syscalls::POLL_WAKE_CHAN);
                    }
                }
            }
            
            // Reset file to initial state
            file.ftype = FileType::None;
            file.pipe = None;
            file.inode = None;
            file.vfs_file = None;
            file.socket = None;
            file.inotify_instance = None;
            file.eventfd_instance = None;
            file.signalfd_instance = None;
            file.timerfd_instance = None;
            file.memfd_instance = None;
            file.readable = false;
            file.writable = false;
            file.status_flags = 0;
            file.offset = 0;
            file.major = 0;
            file.minor = 0;
            file.event_subs.clear();
            
            // Return to object pool for reuse (O(1))
            self.ensure_free_list_initialized();
            if self.free_list.len() < NFILE {
                self.free_list.push(idx);
            }
        }
    }

    /// Get file reference
    pub fn get(&self, idx: usize) -> Option<&File> {
        if idx >= NFILE || self.files[idx].ref_count == 0 {
            None
        } else {
            Some(&self.files[idx])
        }
    }

    /// Get mutable file reference
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut File> {
        if idx >= NFILE || self.files[idx].ref_count == 0 {
            None
        } else {
            Some(&mut self.files[idx])
        }
    }
}

pub static FILE_TABLE: Mutex<FTable> = Mutex::new(FTable::new());

/// Allocate a new file
pub fn file_alloc() -> Option<usize> {
    FILE_TABLE.lock().alloc()
}

/// Duplicate a file
pub fn file_dup(idx: usize) -> Option<usize> {
    FILE_TABLE.lock().dup(idx)
}

/// Close a file
pub fn file_close(idx: usize) {
    FILE_TABLE.lock().close(idx)
}

/// Create a new socket file
pub fn file_socket_new(socket: crate::net::socket::Socket, readable: bool, writable: bool) -> Option<usize> {
    let mut table = FILE_TABLE.lock();
    let idx = table.alloc()?;

    if let Some(file) = table.get_mut(idx) {
        file.ftype = FileType::Socket;
        file.ref_count = 1;
        file.readable = readable;
        file.writable = writable;
        file.status_flags = 0;
        file.socket = Some(socket);

        Some(idx)
    } else {
        None
    }
}

/// Get socket from file descriptor
pub fn file_get_socket(fd: usize) -> Option<crate::net::socket::Socket> {
    let table = FILE_TABLE.lock();
    if let Some(file) = table.get(fd) {
        if file.ftype == FileType::Socket {
            file.socket.clone()
        } else {
            None
        }
    } else {
        None
    }
}

/// Update socket in file descriptor
pub fn file_update_socket(fd: usize, socket: crate::net::socket::Socket) -> bool {
    let mut table = FILE_TABLE.lock();
    if let Some(file) = table.get_mut(fd) {
        if file.ftype == FileType::Socket {
            file.socket = Some(socket);
            true
        } else {
            false
        }
    } else {
        false
    }
}

/// Read from file
pub fn file_read(idx: usize, buf: &mut [u8]) -> isize {
    match FILE_TABLE.lock().get_mut(idx) {
        Some(f) => f.read(buf),
        None => -1,
    }
}

/// Write to file
pub fn file_write(idx: usize, buf: &[u8]) -> isize {
    match FILE_TABLE.lock().get_mut(idx) {
        Some(f) => f.write(buf),
        None => -1,
    }
}

/// Get file status
pub fn file_stat(idx: usize) -> Result<crate::posix::Stat, ()> {
    match FILE_TABLE.lock().get(idx) {
        Some(f) => {
            match f.ftype {
                FileType::Vfs => {
                    if let Some(ref vfs_file) = f.vfs_file {
                        match vfs_file.stat() {
                            Ok(attr) => {
                                let stat = crate::posix::Stat {
                                    st_dev: 0,  // Simplified
                                    st_ino: attr.ino as crate::posix::Ino,
                                    st_mode: attr.mode.0 as crate::posix::Mode,
                                    st_nlink: attr.nlink as crate::posix::Nlink,
                                    st_uid: attr.uid as crate::posix::Uid,
                                    st_gid: attr.gid as crate::posix::Gid,
                                    st_rdev: attr.rdev as crate::posix::Dev,
                                    st_size: attr.size as crate::posix::Off,
                                    st_blksize: attr.blksize as crate::posix::Blksize,
                                    st_blocks: attr.blocks as crate::posix::Blkcnt,
                                    st_atime: attr.atime as crate::posix::Time,
                                    st_atime_nsec: 0,
                                    st_mtime: attr.mtime as crate::posix::Time,
                                    st_mtime_nsec: 0,
                                    st_ctime: attr.ctime as crate::posix::Time,
                                    st_ctime_nsec: 0,
                                };
                                Ok(stat)
                            },
                            Err(_) => Err(()),
                        }
                    } else {
                        Err(())
                    }
                },
                _ => {
                    // For other file types, return basic info
                    let stat = crate::posix::Stat {
                        st_dev: 0,
                        st_ino: 0,
                        st_mode: 0,
                        st_nlink: 1,
                        st_uid: 0,
                        st_gid: 0,
                        st_rdev: 0,
                        st_size: 0,
                        st_blksize: 0,
                        st_blocks: 0,
                        st_atime: 0,
                        st_atime_nsec: 0,
                        st_mtime: 0,
                        st_mtime_nsec: 0,
                        st_ctime: 0,
                        st_ctime_nsec: 0,
                    };
                    Ok(stat)
                }
            }
        },
        None => Err(()),
    }
}

pub fn file_subscribe(idx: usize, events: i16, chan: usize) {
    let mut table = FILE_TABLE.lock();
    if let Some(f) = table.get_mut(idx) {
        match f.ftype {
            FileType::Pipe => {
                if let Some(ref pipe) = f.pipe {
                    let mut p = pipe.lock();
                    if (events & crate::posix::POLLIN) != 0 { p.subscribe_read(chan); }
                    if (events & crate::posix::POLLOUT) != 0 { p.subscribe_write(chan); }
                }
            }
            FileType::Device => {
                crate::drivers::device_subscribe(f.major, f.minor, events, chan);
            }
            _ => {}
        }
    }
}

pub fn file_unsubscribe(idx: usize, chan: usize) {
    let mut table = FILE_TABLE.lock();
    if let Some(f) = table.get_mut(idx) {
        match f.ftype {
            FileType::Pipe => {
                if let Some(ref pipe) = f.pipe {
                    let mut p = pipe.lock();
                    p.unsubscribe_read(chan);
                    p.unsubscribe_write(chan);
                }
            }
            FileType::Device => {
                crate::drivers::device_unsubscribe(f.major, f.minor, chan);
            }
            _ => {}
        }
    }
}

/// Seek in file
pub fn file_lseek(idx: usize, offset: i64, whence: i32) -> isize {
    match FILE_TABLE.lock().get_mut(idx) {
        Some(f) => {
            let current_size = match f.ftype {
                FileType::Vfs => {
                    if let Some(ref vfs_file) = f.vfs_file {
                        match vfs_file.stat() {
                            Ok(attr) => attr.size as i64,
                            Err(_) => return -1,
                        }
                    } else {
                        0
                    }
                }
                FileType::Inode => {
                    // Simplified: assume size is 0 for now
                    0
                }
                _ => return -1,
            };

            let new_offset = match whence {
                crate::posix::SEEK_SET => offset,
                crate::posix::SEEK_CUR => f.offset as i64 + offset,
                crate::posix::SEEK_END => current_size + offset,
                _ => return -1,
            };

            if new_offset < 0 {
                return -1;
            }

            let new_offset_usize = new_offset as usize;
            match f.seek(new_offset_usize) {
                -1 => -1,
                offset => offset,
            }
        }
        None => -1,
    }
}

pub fn file_poll(idx: usize) -> i16 {
    let mut ev: i16 = 0;
    let mut table = FILE_TABLE.lock();
    let f = match table.get_mut(idx) { Some(x) => x, None => return posix::POLLERR };
    match f.ftype {
        FileType::Pipe => {
            if let Some(ref pipe) = f.pipe {
                let p = pipe.lock();
                let readable = p.nread < p.nwrite;
                let writable = p.nwrite < p.nread + crate::ipc::pipe::PIPE_SIZE;
                if readable { ev |= posix::POLLIN; }
                if writable { ev |= posix::POLLOUT; }
                if !p.readopen { ev |= posix::POLLHUP; }
            } else {
                ev |= posix::POLLERR;
            }
        }
        FileType::Device => {
            ev |= crate::drivers::device_poll(f.major, f.minor);
        }
        FileType::Socket => { ev |= posix::POLLERR; }
        _ => {}
    }
    ev
}

/// Truncate file
pub fn file_truncate(idx: usize, size: u64) -> Result<(), ()> {
    match FILE_TABLE.lock().get_mut(idx) {
        Some(f) => {
            match f.ftype {
                FileType::Vfs => {
                    if let Some(ref vfs_file) = f.vfs_file {
                        match vfs_file.truncate(size) {
                            Ok(_) => Ok(()),
                            Err(_) => Err(()),
                        }
                    } else {
                        Err(())
                    }
                },
                _ => Err(()),
            }
        },
        None => Err(()),
    }
}

/// Change file mode
pub fn file_chmod(idx: usize, mode: u32) -> Result<(), ()> {
    // Get current process UID
    let current_uid = crate::process::getuid();
    let is_root = current_uid == 0;
    
    match FILE_TABLE.lock().get_mut(idx) {
        Some(f) => {
            match f.ftype {
                FileType::Vfs => {
                    if let Some(ref vfs_file) = f.vfs_file {
                        match vfs_file.stat() {
                            Ok(attr) => {
                                // Check permissions: only owner or root can change file mode
                                if !is_root && attr.uid != current_uid {
                                    return Err(());
                                }
                                
                                // Preserve the file type, only change the permissions
                                let mut new_attr = attr;
                                new_attr.mode = crate::vfs::FileMode::new((new_attr.mode.0 & !0o7777) | (mode & 0o7777));
                                match vfs_file.set_attr(&new_attr) {
                                    Ok(_) => Ok(()),
                                    Err(_) => Err(()),
                                }
                            },
                            Err(_) => Err(()),
                        }
                    } else {
                        Err(())
                    }
                },
                _ => Err(()),
            }
        },
        None => Err(()),
    }
}

/// Change file owner and group
/// Only root can change file ownership (POSIX requirement)
pub fn file_chown(idx: usize, uid: u32, gid: u32) -> Result<(), ()> {
    // Get current process UID
    let current_uid = crate::process::getuid();
    let is_root = current_uid == 0;
    
    // Only root can change file ownership
    if !is_root {
        return Err(());
    }
    
    match FILE_TABLE.lock().get_mut(idx) {
        Some(f) => {
            match f.ftype {
                FileType::Vfs => {
                    if let Some(ref vfs_file) = f.vfs_file {
                        match vfs_file.stat() {
                            Ok(mut attr) => {
                                attr.uid = uid;
                                attr.gid = gid;
                                match vfs_file.set_attr(&attr) {
                                    Ok(_) => Ok(()),
                                    Err(_) => Err(()),
                                }
                            },
                            Err(_) => Err(()),
                        }
                    } else {
                        Err(())
                    }
                },
                _ => Err(()),
            }
        },
        None => Err(()),
    }
}
