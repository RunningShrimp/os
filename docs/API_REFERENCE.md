# NOS Kernel API Reference

## Executive Summary

This document provides a comprehensive reference for all public APIs in the NOS kernel, including memory management, process management, filesystem, networking, and security APIs.

**Document Version**: 1.0
**Last Updated**: 2025-01-01
**Target Audience**: Kernel developers, systems programmers
**Total Word Count**: ~5,200 words

---

## Table of Contents

1. [Memory Management APIs](#1-memory-management-apis)
2. [Process Management APIs](#2-process-management-apis)
3. [Filesystem APIs](#3-filesystem-apis)
4. [Network APIs](#4-network-apis)
5. [Synchronization APIs](#5-synchronization-apis)
6. [Security APIs](#6-security-apis)
7. [System Call Reference](#7-system-call-reference)

---

## 1. Memory Management APIs

### 1.1 Page Allocation

**allocate_pages()** - Allocate contiguous physical pages

```rust
pub fn allocate_pages(order: u8) -> Option<Page>
```

**Description**: Allocates 2^order contiguous physical pages.

**Parameters**:
- `order`: Log2 of number of pages (0 = 1 page, 1 = 2 pages, ...)

**Returns**: `Some(Page)` containing physical page number, or `None` if out of memory

**Example**:
```rust
// Allocate 8 pages (order = 3)
if let Some(page) = allocate_pages(3) {
    let phys_addr = page * PAGE_SIZE;
    println!("Allocated physical page: 0x{:x}", phys_addr);
} else {
    println!("Out of memory");
}
```

**Implementation**: `kernel/src/subsystems/mm/phys.rs:120-180`

**Thread Safety**: Yes (uses internal locking)

**Context**: Process context only

---

**free_pages()** - Free allocated pages

```rust
pub fn free_pages(page: Page, order: u8)
```

**Description**: Frees 2^order contiguous physical pages previously allocated.

**Parameters**:
- `page`: Physical page number to free
- `order`: Order used in allocation (must match)

**Panics**: If page or order is invalid

**Example**:
```rust
let page = allocate_pages(2).unwrap();
free_pages(page, 2);
```

**Implementation**: `kernel/src/subsystems/mm/phys.rs:200-250`

---

### 1.2 Virtual Memory

**mmap()** - Map virtual memory

```rust
pub fn mmap(addr: usize, size: usize, prot: VmProt, flags: MmapFlags) -> Result<usize, MmapError>
```

**Description**: Creates a new virtual memory mapping in the process address space.

**Parameters**:
- `addr`: Hint for mapping address (use 0 for any address)
- `size`: Size of mapping in bytes (must be page-aligned)
- `prot`: Protection flags (VmProt::READ, WRITE, EXECUTE)
- `flags`: Mapping flags (MAP_SHARED, MAP_PRIVATE, MAP_ANONYMOUS, MAP_FIXED)

**Returns**: Virtual address of mapping on success, or error

**Error Codes**:
- `MmapError::InvalidArgument`: Invalid size or alignment
- `MmapError::PermissionDenied`: Insufficient permissions
- `MmapError::OutOfMemory`: Cannot allocate page tables
- `MmapError::AddrInUse`: Address range conflicts (MAP_FIXED)

**Example**:
```rust
// Map 1MB anonymous memory
let addr = mmap(0, 1024 * 1024, VmProt::READ | VmProt::WRITE, MmapFlags::ANONYMOUS | MmapFlags::PRIVATE)?;
println!("Mapped at 0x{:x}", addr);
```

**Implementation**: `kernel/src/subsystems/mm/vm/mmap.rs:300-500`

---

**munmap()** - Unmap virtual memory

```rust
pub fn munmap(addr: usize, size: usize) -> Result<(), MmapError>
```

**Description**: Removes a virtual memory mapping.

**Parameters**:
- `addr`: Starting virtual address (must be page-aligned)
- `size`: Size of region to unmap (must be page-aligned)

**Returns**: Success or error

**Example**:
```rust
munmap(addr, 1024 * 1024)?;
```

**Implementation**: `kernel/src/subsystems/mm/vm/munmap.rs:150-300`

---

**mprotect()** - Change memory protection

```rust
pub fn mprotect(addr: usize, size: usize, prot: VmProt) -> Result<(), MprotectError>
```

**Description**: Changes protection flags for a memory region.

**Parameters**:
- `addr`: Starting virtual address
- `size`: Size of region
- `prot`: New protection flags

**Returns**: Success or error

**Example**:
```rust
// Make memory read-only and executable
mprotect(addr, 4096, VmProt::READ | VmProt::EXECUTE)?;
```

**Implementation**: `kernel/src/subsystems/mm/vm/mprotect.rs:120-280`

---

### 1.3 Heap Allocation

**kmalloc()** - Allocate kernel memory

```rust
pub fn kmalloc(size: usize, flags: AllocFlags) -> *mut u8
```

**Description**: Allocates memory from the kernel heap.

**Parameters**:
- `size`: Number of bytes to allocate
- `flags`: Allocation flags (GFP_KERNEL, GFP_ATOMIC, GFP_DMA)

**Returns**: Pointer to allocated memory, or null if out of memory

**Flags**:
- `GFP_KERNEL`: Normal allocation (can sleep)
- `GFP_ATOMIC`: Atomic allocation (cannot sleep)
- `GFP_DMA`: Allocate from DMA-capable memory
- `GFP_ZERO`: Zero allocated memory

**Example**:
```rust
// Allocate 1024 bytes (can sleep)
let ptr = kmalloc(1024, GFP_KERNEL);
if !ptr.is_null() {
    unsafe {
        *ptr = 42u8;
    }
    kfree(ptr);
}
```

**Implementation**: `kernel/src/subsystems/mm/slab.rs:400-600`

---

**kfree()** - Free kernel memory

```rust
pub fn kfree(ptr: *mut u8)
```

**Description**: Frees memory previously allocated with kmalloc.

**Parameters**:
- `ptr`: Pointer to memory (must be from kmalloc)

**Panics**: If ptr is null or invalid

**Example**:
```rust
kfree(ptr);
```

**Implementation**: `kernel/src/subsystems/mm/slab.rs:650-750`

---

**krealloc()** - Reallocate kernel memory

```rust
pub fn krealloc(ptr: *mut u8, new_size: usize, flags: AllocFlags) -> *mut u8
```

**Description**: Resizes a previously allocated memory block.

**Parameters**:
- `ptr`: Pointer to existing allocation (or null for new allocation)
- `new_size`: New size in bytes
- `flags`: Allocation flags

**Returns**: Pointer to resized memory, or null if failed

**Example**:
```rust
let ptr = kmalloc(256, GFP_KERNEL);
let ptr = krealloc(ptr, 512, GFP_KERNEL);
```

**Implementation**: `kernel/src/subsystems/mm/slab.rs:800-950`

---

### 1.4 NUMA APIs

**alloc_pages_node()** - Allocate pages from specific NUMA node

```rust
pub fn alloc_pages_node(node: u32, order: u8) -> Option<Page>
```

**Description**: Allocates pages from a specific NUMA node.

**Parameters**:
- `node`: NUMA node ID (0..num_numa_nodes)
- `order`: Log2 of number of pages

**Returns**: Physical page number, or null if out of memory

**Example**:
```rust
// Allocate from node 0 (local memory)
if let Some(page) = alloc_pages_node(0, 1) {
    // Use page
}
```

**Implementation**: `kernel/src/subsystems/mm/numa.rs:300-450`

---

**get_numa_node()** - Get NUMA node for address

```rust
pub fn get_numa_node(phys_addr: usize) -> u32
```

**Description**: Returns the NUMA node ID for a physical address.

**Parameters**:
- `phys_addr`: Physical address

**Returns**: NUMA node ID

**Example**:
```rust
let node = get_numa_node(phys_addr);
```

**Implementation**: `kernel/src/subsystems/mm/numa.rs:500-600`

---

## 2. Process Management APIs

### 2.1 Process Creation

**fork()** - Create child process

```rust
pub fn fork() -> Result<Pid, ForkError>
```

**Description**: Creates a new process (child) that is a copy of the calling process (parent).

**Returns**: Child's PID in parent, 0 in child, or error

**Error Codes**:
- `ForkError::OutOfMemory`: Cannot allocate process structures
- `ForkError::OutOfTasks`: Process table full
- `ForkError::NotAllowed`: Insufficient permissions

**Example**:
```rust
match fork() {
    Ok(0) => {
        // Child process
        println!("I am the child");
        execve("/bin/sh")?;
    }
    Ok(pid) => {
        // Parent process
        println!("Child PID: {}", pid);
        waitpid(pid, None)?;
    }
    Err(e) => {
        println!("Fork failed: {:?}", e);
    }
}
```

**Implementation**: `kernel/src/subsystems/process/fork.rs:150-400`

---

**vfork()** - Create child process with shared memory

```rust
pub fn vfork() -> Result<Pid, ForkError>
```

**Description**: Creates a child process that shares parent's memory until exec or exit.

**Returns**: Child's PID in parent, 0 in child, or error

**Note**: Parent is suspended until child execs or exits

**Example**:
```rust
let pid = vfork()?;
if pid == 0 {
    // Child
    execve("/bin/init")?;
    unreachable!();
}
// Parent continues after child execs
```

**Implementation**: `kernel/src/subsystems/process/vfork.rs:100-250`

---

**execve()** - Execute new program

```rust
pub fn execve(path: &str, args: &[&str], env: &[&str]) -> Result<!, ExecError>
```

**Description**: Replaces current process with a new program.

**Parameters**:
- `path`: Path to executable
- `args`: Arguments (argv[0] is program name)
- `env`: Environment variables (KEY=VALUE format)

**Returns**: Never returns on success, or error

**Error Codes**:
- `ExecError::NotFound`: Executable not found
- `ExecError::PermissionDenied`: Execute permission denied
- `ExecError::InvalidFormat`: Not a valid ELF file
- `ExecError::OutOfMemory`: Cannot allocate memory

**Example**:
```rust
execve(
    "/bin/ls",
    &["ls", "-l", "/tmp"],
    &["PATH=/bin:/usr/bin", "TERM=linux"]
)?;
```

**Implementation**: `kernel/src/subsystems/process/exec.rs:350-700`

---

### 2.2 Process Termination

**exit()** - Terminate current process

```rust
pub fn exit(status: i32) -> !
```

**Description**: Terminates the current process with the given exit status.

**Parameters**:
- `status`: Exit status (0 = success, non-zero = failure)

**Returns**: Never returns

**Example**:
```rust
if error_occurred {
    exit(1);
}
exit(0); // Success
```

**Implementation**: `kernel/src/subsystems/process/exit.rs:200-400`

---

**waitpid()** - Wait for process state change

```rust
pub fn waitpid(pid: Pid, options: WaitOptions) -> Result<WaitStatus, WaitError>
```

**Description**: Waits for state change in specified child process.

**Parameters**:
- `pid`: Process ID to wait for (-1 = any child)
- `options`: Wait options (WNOHANG, WUNTRACED, WCONTINUED)

**Returns**: Process status, or error

**Options**:
- `WNOHANG`: Return immediately if child hasn't exited
- `WUNTRACED`: Also wait for stopped children
- `WCONTINUED`: Also wait for continued children

**Example**:
```rust
// Wait for specific child
match waitpid(child_pid, WaitOptions::empty()) {
    Ok(WaitStatus::Exited(pid, status)) => {
        println!("Child {} exited with status {}", pid, status);
    }
    Ok(WaitStatus::Signaled(pid, signal, _)) => {
        println!("Child {} killed by signal {}", pid, signal);
    }
    Err(WaitError::NoChildProcesses) => {
        println!("No children to wait for");
    }
    _ => {}
}
```

**Implementation**: `kernel/src/subsystems/process/wait.rs:300-600`

---

### 2.3 Process Identification

**getpid()** - Get process ID

```rust
pub fn getpid() -> Pid
```

**Description**: Returns the PID of the current process.

**Returns**: Process ID

**Example**:
```rust
let pid = getpid();
println!("My PID: {}", pid);
```

**Implementation**: `kernel/src/subsystems/process/ids.rs:50-80`

---

**getppid()** - Get parent process ID

```rust
pub fn getppid() -> Pid
```

**Description**: Returns the PID of the parent process.

**Returns**: Parent process ID

**Example**:
```rust
let ppid = getppid();
println!("My parent PID: {}", ppid);
```

**Implementation**: `kernel/src/subsystems/process/ids.rs:100-130`

---

### 2.4 Thread Management

**clone()** - Create new thread

```rust
pub fn clone(flags: CloneFlags, stack: Option<usize>, tls: Option<usize>) -> Result<Tid, CloneError>
```

**Description**: Creates a new thread (or process, depending on flags).

**Parameters**:
- `flags`: Clone flags (CLONE_VM, CLONE_FS, CLONE_FILES, CLONE_THREAD, etc.)
- `stack`: Stack pointer for new thread (None = allocate new stack)
- `tls`: Thread-local storage address (None = allocate new TLS)

**Returns**: Thread ID, or error

**Flags**:
- `CLONE_VM`: Share memory space (creates thread)
- `CLONE_FS`: Share filesystem information
- `CLONE_FILES`: Share file descriptor table
- `CLONE_THREAD`: Add to caller's thread group
- `CLONE_PARENT`: Same parent as caller

**Example**:
```rust
// Create thread sharing memory and files
let stack = allocate_thread_stack();
let tid = clone(CloneFlags::VM | CloneFlags::FILES, Some(stack), None)?;
```

**Implementation**: `kernel/src/subsystems/process/thread.rs:450-750`

---

## 3. Filesystem APIs

### 3.1 File Operations

**open()** - Open file

```rust
pub fn open(pathname: &str, flags: OpenFlags) -> Result<Fd, FsError>
```

**Description**: Opens a file and returns a file descriptor.

**Parameters**:
- `pathname`: Path to file
- `flags`: Open flags (O_RDONLY, O_WRONLY, O_RDWR, O_CREAT, O_TRUNC, O_APPEND)

**Returns**: File descriptor, or error

**Flags**:
- `O_RDONLY`: Open for reading only
- `O_WRONLY`: Open for writing only
- `O_RDWR`: Open for reading and writing
- `O_CREAT`: Create file if it doesn't exist
- `O_TRUNC`: Truncate file to length 0
- `O_APPEND`: Append to end of file
- `O_NONBLOCK`: Non-blocking mode
- `O_DIRECTORY`: Fail if not directory

**Example**:
```rust
// Open file for writing (create if needed)
let fd = open("/tmp/test.txt", OpenFlags::WRONLY | OpenFlags::CREAT)?;
```

**Implementation**: `kernel/src/vfs/open.rs:200-500`

---

**close()** - Close file descriptor

```rust
pub fn close(fd: Fd) -> Result<(), FsError>
```

**Description**: Closes a file descriptor.

**Parameters**:
- `fd`: File descriptor to close

**Returns**: Success or error

**Example**:
```rust
close(fd)?;
```

**Implementation**: `kernel/src/vfs/file.rs:150-300`

---

**read()** - Read from file

```rust
pub fn read(fd: Fd, buf: &mut [u8]) -> Result<usize, FsError>
```

**Description**: Reads up to buf.len() bytes from file into buffer.

**Parameters**:
- `fd`: File descriptor
- `buf`: Destination buffer

**Returns**: Number of bytes read, or error

**Example**:
```rust
let mut buf = [0u8; 1024];
let n = read(fd, &mut buf)?;
println!("Read {} bytes", n);
```

**Implementation**: `kernel/src/vfs/read.rs:250-500`

---

**write()** - Write to file

```rust
pub fn write(fd: Fd, buf: &[u8]) -> Result<usize, FsError>
```

**Description**: Writes buf.len() bytes to file.

**Parameters**:
- `fd`: File descriptor
- `buf`: Data to write

**Returns**: Number of bytes written, or error

**Example**:
```rust
let data = b"Hello, world!";
let n = write(fd, data)?;
```

**Implementation**: `kernel/src/vfs/write.rs:300-550`

---

**lseek()** - Reposition file offset

```rust
pub fn lseek(fd: Fd, offset: isize, whence: SeekWhence) -> Result<usize, FsError>
```

**Description**: Sets the file position for the next read/write.

**Parameters**:
- `fd`: File descriptor
- `offset`: Offset in bytes
- `whence`: Reference point (SEEK_SET, SEEK_CUR, SEEK_END)

**Returns**: New file offset, or error

**Whence**:
- `SEEK_SET`: Set offset to offset bytes
- `SEEK_CUR`: Set offset to current + offset
- `SEEK_END`: Set offset to file size + offset

**Example**:
```rust
// Seek to end of file
let size = lseek(fd, 0, SeekWhence::End)?;
```

**Implementation**: `kernel/src/vfs/lseek.rs:120-350`

---

### 3.2 Directory Operations

**mkdir()** - Create directory

```rust
pub fn mkdir(pathname: &str, mode: FileMode) -> Result<(), FsError>
```

**Description**: Creates a new directory.

**Parameters**:
- `pathname`: Directory path
- `mode`: Permission mode (e.g., 0o755)

**Returns**: Success or error

**Example**:
```rust
mkdir("/tmp/newdir", 0o755)?;
```

**Implementation**: `kernel/src/vfs/mkdir.rs:100-280`

---

**rmdir()** - Remove directory

```rust
pub fn rmdir(pathname: &str) -> Result<(), FsError>
```

**Description**: Removes an empty directory.

**Parameters**:
- `pathname`: Directory path

**Returns**: Success or error

**Error**: Returns error if directory is not empty

**Example**:
```rust
rmdir("/tmp/olddir")?;
```

**Implementation**: `kernel/src/vfs/rmdir.rs:80-200`

---

**opendir()** - Open directory

```rust
pub fn opendir(pathname: &str) -> Result<DirHandle, FsError>
```

**Description**: Opens a directory for reading.

**Parameters**:
- `pathname`: Directory path

**Returns**: Directory handle, or error

**Example**:
```rust
let dir = opendir("/tmp")?;
```

**Implementation**: `kernel/src/vfs/readdir.rs:150-400`

---

**readdir()** - Read directory entry

```rust
pub fn readdir(dir: &mut DirHandle) -> Option<DirEntry>
```

**Description**: Reads next entry from directory.

**Parameters**:
- `dir`: Directory handle

**Returns**: Directory entry, or None if at end

**Example**:
```rust
while let Some(entry) = readdir(&mut dir) {
    println!("{} (inode {})", entry.name, entry.inode);
}
```

**Implementation**: `kernel/src/vfs/readdir.rs:450-650`

---

### 3.3 Filesystem Mounting

**mount()** - Mount filesystem

```rust
pub fn mount(source: &str, target: &str, fs_type: &str, flags: MountFlags, data: Option<&str>) -> Result<(), FsError>
```

**Description**: Mounts a filesystem at specified path.

**Parameters**:
- `source`: Source device (e.g., "/dev/sda1")
- `target`: Mount point directory
- `fs_type`: Filesystem type ("ext4", "proc", "sysfs", etc.)
- `flags`: Mount flags (MS_RDONLY, MS_NOSUID, MS_NOEXEC, etc.)
- `data`: Filesystem-specific data

**Returns**: Success or error

**Example**:
```rust
mount("/dev/sda1", "/mnt", "ext4", MountFlags::empty(), None)?;
```

**Implementation**: `kernel/src/vfs/mount.rs:400-800`

---

**umount()** - Unmount filesystem

```rust
pub fn umount(target: &str) -> Result<(), FsError>
```

**Description**: Unmounts a filesystem.

**Parameters**:
- `target`: Mount point to unmount

**Returns**: Success or error

**Example**:
```rust
umount("/mnt")?;
```

**Implementation**: `kernel/src/vfs/mount.rs:850-1000`

---

## 4. Network APIs

### 4.1 Socket API

**socket()** - Create socket

```rust
pub fn socket(domain: Domain, socket_type: Type, protocol: Protocol) -> Result<Fd, NetError>
```

**Description**: Creates an endpoint for communication.

**Parameters**:
- `domain`: Protocol family (AF_INET, AF_INET6, AF_UNIX)
- `socket_type`: Socket type (SOCK_STREAM, SOCK_DGRAM, SOCK_RAW)
- `protocol`: Protocol number (0 for default)

**Returns**: Socket file descriptor, or error

**Example**:
```rust
// Create TCP socket
let fd = socket(Domain::INET, Type::STREAM, Protocol::TCP)?;
```

**Implementation**: `kernel/src/subsystems/net/socket.rs:300-600`

---

**bind()** - Bind socket to address

```rust
pub fn bind(fd: Fd, addr: &SocketAddr) -> Result<(), NetError>
```

**Description**: Assigns a local address to the socket.

**Parameters**:
- `fd`: Socket file descriptor
- `addr`: Address to bind to

**Returns**: Success or error

**Example**:
```rust
let addr = SocketAddr::new(IpAddr::v4(127, 0, 0, 1), 8080);
bind(fd, &addr)?;
```

**Implementation**: `kernel/src/subsystems/net/socket.rs:650-850`

---

**listen()** - Listen for connections

```rust
pub fn listen(fd: Fd, backlog: i32) -> Result<(), NetError>
```

**Description**: Marks socket as passive, ready to accept incoming connections.

**Parameters**:
- `fd`: Socket file descriptor
- `backlog`: Maximum pending connection queue length

**Returns**: Success or error

**Example**:
```rust
listen(fd, 10)?;
```

**Implementation**: `kernel/src/subsystems/net/socket.rs:900-1050`

---

**accept()** - Accept connection

```rust
pub fn accept(fd: Fd) -> Result<(Fd, SocketAddr), NetError>
```

**Description**: Accepts first pending connection.

**Parameters**:
- `fd`: Listening socket file descriptor

**Returns**: (New socket fd, client address), or error

**Example**:
```rust
let (client_fd, client_addr) = accept(fd)?;
println!("Connection from {}", client_addr);
```

**Implementation**: `kernel/src/subsystems/net/socket.rs:1100-1300`

---

**connect()** - Connect to peer

```rust
pub fn connect(fd: Fd, addr: &SocketAddr) -> Result<(), NetError>
```

**Description**: Connects socket to specified address.

**Parameters**:
- `fd`: Socket file descriptor
- `addr`: Peer address

**Returns**: Success or error

**Example**:
```rust
let addr = SocketAddr::new(IpAddr::v4(192, 168, 1, 100), 80);
connect(fd, &addr)?;
```

**Implementation**: `kernel/src/subsystems/net/socket.rs:1350-1600`

---

**send()** - Send data

```rust
pub fn send(fd: Fd, buf: &[u8], flags: SendFlags) -> Result<usize, NetError>
```

**Description**: Sends data on connected socket.

**Parameters**:
- `fd`: Socket file descriptor
- `buf`: Data to send
- `flags`: Send flags (MSG_DONTWAIT, MSG_MORE, etc.)

**Returns**: Number of bytes sent, or error

**Example**:
```rust
let data = b"GET / HTTP/1.1\r\n\r\n";
let n = send(fd, data, SendFlags::empty())?;
```

**Implementation**: `kernel/src/subsystems/net/socket.rs:1650-1850`

---

**recv()** - Receive data

```rust
pub fn recv(fd: Fd, buf: &mut [u8], flags: RecvFlags) -> Result<usize, NetError>
```

**Description**: Receives data from connected socket.

**Parameters**:
- `fd`: Socket file descriptor
- `buf`: Destination buffer
- `flags`: Receive flags (MSG_DONTWAIT, MSG_PEEK, etc.)

**Returns**: Number of bytes received, or error

**Example**:
```rust
let mut buf = [0u8; 1024];
let n = recv(fd, &mut buf, RecvFlags::empty())?;
```

**Implementation**: `kernel/src/subsystems/net/socket.rs:1900-2100`

---

## 5. Synchronization APIs

### 5.1 Spinlock

**SpinLock** - Spin-based mutual exclusion

```rust
pub struct SpinLock<T> {
    data: UnsafeCell<T>,
    lock: AtomicBool,
}

impl<T> SpinLock<T> {
    pub fn new(data: T) -> Self;
    pub fn lock(&self) -> SpinLockGuard<T>;
    pub fn try_lock(&self) -> Option<SpinLockGuard<T>>;
}
```

**Description**: A lock that spins waiting for acquisition. Use only in contexts where sleeping is not allowed (interrupt handlers, etc.).

**Methods**:
- `new()`: Create new spinlock
- `lock()`: Acquire lock (spins until acquired)
- `try_lock()`: Try to acquire without blocking

**Example**:
```rust
let lock = SpinLock::new(0);

{
    let mut data = lock.lock();
    *data += 1;
} // Lock released here
```

**Implementation**: `kernel/src/subsystems/sync/spinlock.rs:100-300`

---

### 5.2 Mutex

**Mutex** - Sleep-based mutual exclusion

```rust
pub struct Mutex<T> {
    data: UnsafeCell<T>,
    lock: AtomicBool,
    wait_queue: WaitQueue,
}

impl<T> Mutex<T> {
    pub fn new(data: T) -> Self;
    pub fn lock(&self) -> MutexGuard<T>;
    pub fn try_lock(&self) -> Option<MutexGuard<T>>;
}
```

**Description**: A lock that sleeps waiting for acquisition. Use in process context where sleeping is allowed.

**Example**:
```rust
let mutex = Mutex::new(Vec::new());

{
    let mut list = mutex.lock();
    list.push(42);
} // Lock released here
```

**Implementation**: `kernel/src/subsystems/sync/mutex.rs:150-400`

---

### 5.3 Read-Write Lock

**RwLock** - Multiple readers or single writer

```rust
pub struct RwLock<T> {
    data: UnsafeCell<T>,
    readers: AtomicUsize,
    writer: AtomicBool,
}

impl<T> RwLock<T> {
    pub fn new(data: T) -> Self;
    pub fn read(&self) -> RwLockReadGuard<T>;
    pub fn write(&self) -> RwLockWriteGuard<T>;
}
```

**Description**: Allows multiple concurrent readers or exclusive writer access.

**Methods**:
- `read()`: Acquire read lock (shared access)
- `write()`: Acquire write lock (exclusive access)

**Example**:
```rust
let lock = RwLock::new(HashMap::new());

// Multiple readers can hold lock simultaneously
{
    let map = lock.read();
    let value = map.get(&key);
}

// Writer gets exclusive access
{
    let mut map = lock.write();
    map.insert(key, value);
}
```

**Implementation**: `kernel/src/subsystems/sync/rwlock.rs:200-500`

---

### 5.4 RCU (Read-Copy-Update)

**rcu_read_lock()** - Enter RCU read-side critical section

```rust
pub fn rcu_read_lock() -> RcuReadGuard
```

**Description**: Marks entry to RCU read-side critical section (no locks needed).

**Example**:
```rust
{
    let guard = rcu_read_lock();
    // Read data (no locks!)
    let value = data.read();
} // RCU grace period starts here
```

**Implementation**: `kernel/src/subsystems/sync/rcu.rs:150-350`

---

**rcu_call_old()** - Defer freeing until grace period

```rust
pub fn rcu_call_old<T>(old: T)
where
    T: 'static,
```

**Description**: Frees data after all pre-existing RCU read-side critical sections complete.

**Example**:
```rust
let new_data = Arc::new(new_value);
let old_data = data.swap(new_data);
rcu_call_old(old_data);
```

**Implementation**: `kernel/src/subsystems/sync/rcu.rs:400-600`

---

## 6. Security APIs

### 6.1 Capabilities

**has_capability()** - Check capability

```rust
pub fn has_capability(cap: Capability) -> bool
```

**Description**: Checks if current process has specified capability.

**Parameters**:
- `cap`: Capability to check (CAP_NET_RAW, CAP_SYS_ADMIN, etc.)

**Returns**: true if process has capability

**Example**:
```rust
if has_capability(Capability::NetRaw) {
    // Can create raw socket
} else {
    return Err(NetError::PermissionDenied);
}
```

**Implementation**: `kernel/src/security/capability.rs:100-250`

---

### 6.2 Permission Checks

**inode_permission()** - Check file access permissions

```rust
pub fn inode_permission(inode: &Inode, mask: AccessMode) -> Result<(), FsError>
```

**Description**: Checks if current process can access inode with specified mode.

**Parameters**:
- `inode`: Inode to check
- `mask`: Requested access (MAY_READ, MAY_WRITE, MAY_EXEC)

**Returns**: Success or permission denied error

**Example**:
```rust
inode_permission(&inode, AccessMode::READ | AccessMode::WRITE)?;
```

**Implementation**: `kernel/src/security/permission.rs:200-450`

---

## 7. System Call Reference

### 7.1 Complete System Call Listing

| Number | Name | Parameters | Description |
|--------|------|------------|-------------|
| 0 | read | (fd, buf, count) | Read from file descriptor |
| 1 | write | (fd, buf, count) | Write to file descriptor |
| 2 | open | (pathname, flags, mode) | Open file |
| 3 | close | (fd) | Close file descriptor |
| 4 | stat | (pathname, statbuf) | Get file status |
| 5 | fstat | (fd, statbuf) | Get file status |
| 6 | lseek | (fd, offset, whence) | Reposition file offset |
| 7 | mmap | (addr, length, prot, flags, fd, offset) | Map memory |
| 8 | munmap | (addr, length) | Unmap memory |
| 9 | mprotect | (addr, len, prot) | Change memory protection |
| 10 | brk | (addr) | Change data segment size |
| 11 | rt_sigaction | (signum, act, oldact) | Set signal handler |
| 12 | rt_sigprocmask | (how, set, oldset) | Set signal mask |
| 13 | rt_sigreturn | () | Return from signal handler |
| 14 | ioctl | (fd, cmd, arg) | Device control |
| 15 | pread64 | (fd, buf, count, offset) | Read from offset |
| 16 | pwrite64 | (fd, buf, count, offset) | Write at offset |
| 17 | readv | (fd, iov, iovcnt) | Read into vector |
| 18 | writev | (fd, iov, iovcnt) | Write from vector |
| 19 | access | (pathname, mode) | Check file accessibility |
| 20 | pipe | (pipefd) | Create pipe |
| 21 | select | (nfds, rfds, wfds, efds, timeout) | Synchronous I/O multiplexing |
| 22 | sched_yield | () | Yield CPU |
| 23 | mremap | (old_addr, old_size, new_size, flags, new_addr) | Remap memory |
| 24 | mincore | (addr, length, vec) | Determine page residency |
| 25 | madvise | (addr, length, advice) | Give advice about use of memory |
| 26 | shmget | (key, size, shmflg) | Create shared memory |
| 27 | shmat | (shmid, shmaddr, shmflg) | Attach shared memory |
| 28 | shmctl | (shmid, cmd, buf) | Control shared memory |
| 29 | dup | (oldfd) | Duplicate file descriptor |
| 30 | dup2 | (oldfd, newfd) | Duplicate to specific fd |
| 31 | pause | () | Wait for signal |
| 32 | nanosleep | (req, rem) | Sleep |
| 33 | getitimer | (which, value) | Get interval timer |
| 34 | setitimer | (which, value, ovalue) | Set interval timer |
| 35 | getpid | () | Get process ID |
| 36 | sendfile | (out_fd, in_fd, offset, count) | Transfer data between files |
| 37 | socket | (domain, type, protocol) | Create socket |
| 38 | connect | (sockfd, addr, addrlen) | Connect socket |
| 39 | accept | (sockfd, addr, addrlen) | Accept connection |
| 40 | sendto | (sockfd, buf, len, flags, addr, addrlen) | Send message |
| 41 | recvfrom | (sockfd, buf, len, flags, addr, addrlen) | Receive message |
| 42 | sendmsg | (sockfd, msg, flags) | Send message |
| 43 | recvmsg | (sockfd, msg, flags) | Receive message |
| 44 | shutdown | (sockfd, how) | Shutdown socket |
| 45 | bind | (sockfd, addr, addrlen) | Bind socket to address |
| 46 | listen | (sockfd, backlog) | Listen for connections |
| 47 | getsockname | (sockfd, addr, addrlen) | Get socket name |
| 48 | getpeername | (sockfd, addr, addrlen) | Get peer name |
| 49 | socketpair | (domain, type, protocol, sv) | Create socket pair |
| 50 | setsockopt | (sockfd, level, optname, optval, optlen) | Set socket options |
| 51 | getsockopt | (sockfd, level, optname, optval, optlen) | Get socket options |
| 52 | clone | (flags, stack, parent_tid, child_tid, tls) | Create child process |
| 53 | fork | () | Create child process |
| 54 | vfork | () | Create child process (shared memory) |
| 55 | execve | (pathname, argv, envp) | Execute program |
| 56 | exit | (status) | Terminate process |
| 57 | wait4 | (pid, status, options, rusage) | Wait for process |
| 58 | kill | (pid, sig) | Send signal |
| 59 | uname | (buf) | Get system information |
| 60 | semget | (key, nsems, semflg) | Create semaphore set |
| 61 | semop | (semid, sops, nsops) | Semaphore operations |
| 62 | semctl | (semid, semnum, cmd, arg) | Control semaphore |
| 63 | shmdt | (shmaddr) | Detach shared memory |
| 64 | msgget | (key, msgflg) | Create message queue |
| 65 | msgsnd | (msqid, msgp, msgsz, msgflg) | Send message |
| 66 | msgrcv | (msqid, msgp, msgsz, msgtyp, msgflg) | Receive message |
| 67 | msgctl | (msqid, cmd, buf) | Control message queue |
| 68 | fcntl | (fd, cmd, arg) | File descriptor control |
| 69 | flock | (fd, operation) | File lock |
| 70 | fsync | (fd) | Synchronize file |
| 71 | fdatasync | (fd) | Synchronize file data |
| 72 | truncate | (pathname, length) | Truncate file |
| 73 | ftruncate | (fd, length) | Truncate file |
| 74 | getdents | (fd, dirp, count) | Get directory entries |
| 75 | getcwd | (buf, size) | Get current working directory |
| 76 | chdir | (pathname) | Change working directory |
| 77 | fchdir | (fd) | Change working directory (via fd) |
| 78 | rename | (oldpath, newpath) | Rename file |
| 79 | mkdir | (pathname, mode) | Create directory |
| 80 | rmdir | (pathname) | Remove directory |
| 81 | creat | (pathname, mode) | Create file |
| 82 | link | (oldpath, newpath) | Create hard link |
| 83 | unlink | (pathname) | Remove link |
| 84 | symlink | (oldpath, newpath) | Create symbolic link |
| 85 | readlink | (pathname, buf, bufsiz) | Read symbolic link |
| 86 | chmod | (pathname, mode) | Change file permissions |
| 87 | fchmod | (fd, mode) | Change file permissions |
| 88 | chown | (pathname, owner, group) | Change file owner |
| 89 | fchown | (fd, owner, group) | Change file owner |
| 90 | lchown | (pathname, owner, group) | Change symlink owner |
| 91 | umask | (mask) | Set file mode creation mask |
| 92 | gettimeofday | (tv, tz) | Get time |
| 93 | settimeofday | (tv, tz) | Set time |
| 94 | getrlimit | (resource, rlim) | Get resource limit |
| 95 | setrlimit | (resource, rlim) | Set resource limit |
| 96 | getrusage | (who, usage) | Get resource usage |
| 97 | sysinfo | (info) | Get system information |
| 98 | times | (buf) | Get process times |
| 99 | ptrace | (request, pid, addr, data) | Process trace |
| 100 | getuid | () | Get user ID |
| 101 | geteuid | () | Get effective user ID |
| 102 | getgid | () | Get group ID |
| 103 | getegid | () | Get effective group ID |
| 104 | setuid | (uid) | Set user ID |
| 105 | setgid | (gid) | Set group ID |
| 106 | getppid | () | Get parent process ID |
| 107 | getpgrp | () | Get process group ID |
| 108 | setsid | () | Create session |
| 109 | setpgid | (pid, pgid) | Set process group ID |
| 110 | getpriority | (which, who) | Get scheduling priority |
| 111 | setpriority | (which, who, prio) | Set scheduling priority |
| 112 | sched_setaffinity | (pid, cpusetsize, mask) | Set CPU affinity |
| 113 | sched_getaffinity | (pid, cpusetsize, mask) | Get CPU affinity |
| 114 | sched_setparam | (pid, param) | Set scheduling parameters |
| 115 | sched_getparam | (pid, param) | Get scheduling parameters |
| 116 | sched_setscheduler | (pid, policy, param) | Set scheduling policy |
| 117 | sched_getscheduler | (pid) | Get scheduling policy |
| 118 | sched_get_priority_max | (policy) | Get max priority |
| 119 | sched_get_priority_min | (policy) | Get min priority |
| 120 | sched_rr_get_interval | (pid, tp) | Get RR interval |
| 121 | mlock | (addr, len) | Lock memory |
| 122 | munlock | (addr, len) | Unlock memory |
| 123 | mlockall | (flags) | Lock all memory |
| 124 | munlockall | () | Unlock all memory |
| 125 | vhangup | () | Virtual hangup |
| 126 | pivot_root | (new_root, put_old) | Change root filesystem |
| 127 | prctl | (option, arg2, arg3, arg4, arg5) | Process operations |
| 128 | arch_prctl | (code, addr) | Architecture-specific process control |
| 129 | adjtimex | (buf) | Adjust time |
| 130 | setrlimit | (resource, rlim) | Set resource limit |
| 131 | chroot | (pathname) | Change root directory |
| 132 | sync | () | Synchronize filesystem caches |
| 133 | acct | (pathname) | Enable process accounting |
| 134 | settimeofday | (tv, tz) | Set time |
| 135 | mount | (source, target, fstype, flags, data) | Mount filesystem |
| 136 | umount2 | (target, flags) | Unmount filesystem |
| 137 | swapon | (path, swapflags) | Add swap |
| 138 | swapoff | (path) | Remove swap |
| 139 | reboot | (magic, magic2, cmd, arg) | Reboot system |
| 140 | sethostname | (name, len) | Set hostname |
| 141 | setdomainname | (name, len) | Set domain name |
| 142 | iopl | (level) | Change I/O privilege level |
| 143 | ioperm | (from, num, turn_on) | Set I/O permission bits |
| 144 | init_module | (um, len, uargs) | Load kernel module |
| 145 | delete_module | (name) | Unload kernel module |
| 146 | quotactl | (cmd, special, id, addr) | Quota control |
| 147 | gettid | () | Get thread ID |
| 148 | readahead | (fd, offset, count) | Read ahead |
| 149 | setxattr | (pathname, name, value, size, flags) | Set extended attribute |
| 150 | lsetxattr | (pathname, name, value, size, flags) | Set extended attribute (link) |
| 151 | fsetxattr | (fd, name, value, size, flags) | Set extended attribute (fd) |
| 152 | getxattr | (pathname, name, value, size) | Get extended attribute |
| 153 | lgetxattr | (pathname, name, value, size) | Get extended attribute (link) |
| 154 | fgetxattr | (fd, name, value, size) | Get extended attribute (fd) |
| 155 | listxattr | (pathname, list, size) | List extended attributes |
| 156 | llistxattr | (pathname, list, size) | List extended attributes (link) |
| 157 | flistxattr | (fd, list, size) | List extended attributes (fd) |
| 158 | removexattr | (pathname, name) | Remove extended attribute |
| 159 | lremovexattr | (pathname, name) | Remove extended attribute (link) |
| 160 | fremovexattr | (fd, name) | Remove extended attribute (fd) |
| 161 | tkill | (tid, sig) | Send signal to thread |
| 162 | time | (tloc) | Get time |
| 163 | futex | (uaddr, op, val, timeout, uaddr2, val3) | Fast userspace mutex |
| 164 | sched_setaffinity | (pid, cpusetsize, mask) | Set CPU affinity |
| 165 | sched_getaffinity | (pid, cpusetsize, mask) | Get CPU affinity |
| 166 | set_thread_area | (user_info) | Set thread-local storage |
| 167 | get_thread_area | (user_info) | Get thread-local storage |
| 168 | io_setup | (nr_events, ctx_idp) | Setup async I/O |
| 169 | io_destroy | (ctx_id) | Destroy async I/O |
| 170 | io_getevents | (ctx_id, min_nr, nr, events, timeout) | Get async I/O events |
| 171 | io_submit | (ctx_id, nr, iocbpp) | Submit async I/O |
| 172 | io_cancel | (ctx_id, iocb, result) | Cancel async I/O |
| 173 | getdents64 | (fd, dirp, count) | Get directory entries (64-bit) |
| 174 | set_tid_address | (tidptr) | Set thread ID address |
| 175 | restart_syscall | () | Restart system call |
| 176 | semtimedop | (semid, sops, nsops, timeout) | Semaphore operations (timed) |
| 177 | fadvise64 | (fd, offset, len, advice) | File access advice |
| 178 | timer_create | (clockid, sevp, timer_id) | Create timer |
| 179 | timer_settime | (timer_id, flags, new, old) | Set timer |
| 180 | timer_gettime | (timer_id, setting) | Get timer |
| 181 | timer_getoverrun | (timer_id) | Get timer overrun |
| 182 | timer_delete | (timer_id) | Delete timer |
| 183 | clock_settime | (clock_id, tp) | Set clock |
| 184 | clock_gettime | (clock_id, tp) | Get clock |
| 185 | clock_getres | (clock_id, res) | Get clock resolution |
| 186 | clock_nanosleep | (clock_id, flags, request, remain) | Sleep |
| 187 | exit_group | (status) | Exit all threads |
| 188 | epoll_wait | (epfd, events, maxevents, timeout) | Wait for epoll event |
| 189 | epoll_ctl | (epfd, op, fd, event) | Control epoll |
| 190 | tgkill | (tgid, pid, sig) | Send signal to thread group |
| 191 | utimes | (filename, utimes) | Change file times |
| 192 | mbind | (start, len, mask, mode) | Set memory policy |
| 193 | get_mempolicy | (policy, nmask, maxnode, addr, flags) | Get memory policy |
| 194 | set_mempolicy | (mode, nmask, maxnode) | Set memory policy |
| 195 | mq_open | (name, oflag, mode, attr) | Open message queue |
| 196 | mq_unlink | (name) | Unlink message queue |
| 197 | mq_timedsend | (mqdes, msg_ptr, msg_len, msg_prio, abs_timeout) | Send message (timed) |
| 198 | mq_timedreceive | (mqdes, msg_ptr, msg_len, msg_prio, abs_timeout) | Receive message (timed) |
| 199 | mq_notify | (mqdes, sevp) | Notify message queue |
| 200 | mq_getsetattr | (mqdes, newattr, oldattr) | Get/set queue attributes |

### 7.2 Error Codes

| Code | Name | Description |
|------|------|-------------|
| 1 | EPERM | Operation not permitted |
| 2 | ENOENT | No such file or directory |
| 3 | ESRCH | No such process |
| 4 | EINTR | Interrupted system call |
| 5 | EIO | I/O error |
| 6 | ENXIO | No such device or address |
| 7 | E2BIG | Argument list too long |
| 8 | ENOEXEC | Exec format error |
| 9 | EBADF | Bad file number |
| 10 | ECHILD | No child processes |
| 11 | EAGAIN | Try again |
| 12 | ENOMEM | Out of memory |
| 13 | EACCES | Permission denied |
| 14 | EFAULT | Bad address |
| 15 | ENOTBLK | Block device required |
| 16 | EBUSY | Device or resource busy |
| 17 | EEXIST | File exists |
| 18 | EXDEV | Cross-device link |
| 19 | ENODEV | No such device |
| 20 | ENOTDIR | Not a directory |
| 21 | EISDIR | Is a directory |
| 22 | EINVAL | Invalid argument |
| 23 | ENFILE | File table overflow |
| 24 | EMFILE | Too many open files |
| 25 | ENOTTY | Not a typewriter |
| 26 | ETXTBSY | Text file busy |
| 27 | EFBIG | File too large |
| 28 | ENOSPC | No space left on device |
| 29 | ESPIPE | Illegal seek |
| 30 | EROFS | Read-only file system |
| 31 | EMLINK | Too many links |
| 32 | EPIPE | Broken pipe |
| 33 | EDOM | Math argument out of domain |
| 34 | ERANGE | Math result not representable |
| 35 | EDEADLK | Resource deadlock would occur |
| 36 | ENAMETOOLONG | File name too long |
| 37 | ENOLCK | No record locks available |
| 38 | ENOSYS | Function not implemented |
| 39 | ENOTEMPTY | Directory not empty |
| 40 | ELOOP | Too many symbolic links |
| 42 | ENOMSG | No message of desired type |
| 43 | EIDRM | Identifier removed |
| 44 | ECHRNG | Channel number out of range |
| 45 | EL2NSYNC | Level 2 not synchronized |
| 46 | EL3HLT | Level 3 halted |
| 47 | EL3RST | Level 3 reset |
| 48 | ELNRNG | Link number out of range |
| 49 | EUNATCH | Protocol driver not attached |
| 50 | ENOCSI | No CSI structure available |
| 51 | EL2HLT | Level 2 halted |
| 52 | EBADE | Invalid exchange |
| 53 | EBADR | Invalid request descriptor |
| 54 | EXFULL | Exchange full |
| 55 | ENOANO | No anode |
| 56 | EBADRQC | Invalid request code |
| 57 | EBADSLT | Invalid slot |
| 59 | EBFONT | Bad font file format |
| 60 | ENOSTR | Device not a stream |
| 61 | ENODATA | No data available |
| 62 | ETIME | Timer expired |
| 63 | ENOSR | Out of streams resources |
| 64 | ENONET | Machine is not on the network |
| 65 | ENOPKG | Package not installed |
| 66 | EREMOTE | Object is remote |
| 67 | ENOLINK | Link has been severed |
| 68 | EADV | Advertise error |
| 69 | ESRMNT | Srmount error |
| 70 | ECOMM | Communication error on send |
| 71 | EPROTO | Protocol error |
| 72 | EMULTIHOP | Multihop attempted |
| 73 | EDOTDOT | RFS specific error |
| 74 | EBADMSG | Not a data message |
| 75 | EOVERFLOW | Value too large for defined data type |
| 76 | ENOTUNIQ | Name not unique on network |
| 77 | EBADFD | File descriptor in bad state |
| 78 | EREMCHG | Remote address changed |
| 79 | ELIBACC | Cannot access a needed shared library |
| 80 | ELIBBAD | Accessing a corrupted shared library |
| 81 | ELIBSCN | .lib section in a.out corrupted |
| 82 | ELIBMAX | Attempting to link in too many shared libraries |
| 83 | ELIBEXEC | Cannot exec a shared library directly |
| 84 | EILSEQ | Illegal byte sequence |
| 85 | ERESTART | Interrupted system call should be restarted |
| 86 | ESTRPIPE | Streams pipe error |
| 87 | EUSERS | Too many users |
| 88 | ENOTSOCK | Socket operation on non-socket |
| 89 | EDESTADDRREQ | Destination address required |
| 90 | EMSGSIZE | Message too long |
| 91 | EPROTOTYPE | Protocol wrong type for socket |
| 92 | ENOPROTOOPT | Protocol not available |
| 93 | EPROTONOSUPPORT | Protocol not supported |
| 94 | ESOCKTNOSUPPORT | Socket type not supported |
| 95 | EOPNOTSUPP | Operation not supported |
| 96 | EPFNOSUPPORT | Protocol family not supported |
| 97 | EAFNOSUPPORT | Address family not supported |
| 98 | EADDRINUSE | Address already in use |
| 99 | EADDRNOTAVAIL | Cannot assign requested address |
| 100 | ENETDOWN | Network is down |
| 101 | ENETUNREACH | Network is unreachable |
| 102 | ENETRESET | Network dropped connection because of reset |
| 103 | ECONNABORTED | Software caused connection abort |
| 104 | ECONNRESET | Connection reset by peer |
| 105 | ENOBUFS | No buffer space available |
| 106 | EISCONN | Transport endpoint is already connected |
| 107 | ENOTCONN | Transport endpoint is not connected |
| 108 | ESHUTDOWN | Cannot send after socket shutdown |
| 109 | ETOOMANYREFS | Too many references |
| 110 | ETIMEDOUT | Connection timed out |
| 111 | ECONNREFUSED | Connection refused |
| 112 | EHOSTDOWN | Host is down |
| 113 | EHOSTUNREACH | No route to host |
| 114 | EALREADY | Operation already in progress |
| 115 | EINPROGRESS | Operation now in progress |
| 116 | ESTALE | Stale file handle |
| 117 | EUCLEAN | Structure needs cleaning |
| 118 | ENOTNAM | Not a XENIX named type file |
| 119 | ENAVAIL | No XENIX semaphores available |
| 120 | EISNAM | Is a named type file |
| 121 | EREMOTEIO | Remote I/O error |
| 122 | EDQUOT | Quota exceeded |
| 123 | ENOMEDIUM | No medium found |
| 124 | EMEDIUMTYPE | Wrong medium type |
| 125 | ECANCELED | Operation canceled |
| 126 | ENOKEY | Required key not available |
| 127 | EKEYEXPIRED | Key has expired |
| 128 | EKEYREVOKED | Key has been revoked |
| 129 | EKEYREJECTED | Key was rejected by service |
| 130 | EOWNERDEAD | Owner died |
| 131 | ENOTRECOVERABLE | State not recoverable |
| 132 | ERFKILL | Operation not possible due to RF-kill |

---

**End of Document**

Total Word Count: ~5,200 words
API Categories: 7 major categories
System Calls: 200+ syscalls documented
Error Codes: 132 error codes
Implementation References: 50+ file references
