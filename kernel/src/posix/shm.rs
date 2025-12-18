//! POSIX Shared Memory (sys/shm.h) Implementation
//!
//! Implements POSIX shared memory segments for inter-process communication.

extern crate alloc;

use alloc::sync::Arc;
use alloc::collections::BTreeMap;
use crate::sync::Mutex;
use crate::reliability::errno::{EOK, EINVAL, ENOENT};
use crate::posix::{ShmidDs, IpcPerm, Mode, Pid, Size};
use crate::mm::vm;

/// Shared memory segment
#[derive(Debug)]
struct SharedMemorySegment {
    /// Segment ID
    id: i32,
    /// Segment key
    key: i32,
    /// Segment size
    size: usize,
    /// Physical pages backing the segment
    pages: alloc::vec::Vec<vm::Page>,
    /// Permissions and ownership
    perm: IpcPerm,
    /// Number of current attaches
    nattch: u64,
    /// Creator process ID
    creator_pid: Pid,
    /// Last attach process ID
    last_attach_pid: Pid,
    /// Last detach time
    last_detach_time: crate::posix::Time,
    /// Creation time
    creation_time: crate::posix::Time,
    /// Segment is marked for removal
    remove_pending: bool,
}

/// Global shared memory registry
static SHM_SEGMENTS: Mutex<BTreeMap<i32, Arc<Mutex<SharedMemorySegment>>>> =
    Mutex::new(BTreeMap::new());

/// Next shared memory ID
static NEXT_SHM_ID: core::sync::atomic::AtomicI32 = core::sync::atomic::AtomicI32::new(1);

/// Maximum number of shared memory segments
const SHM_MAX: usize = 4096;

/// Maximum shared memory segment size
const SHMMAX: usize = 0x2000000; // 32MB

/// Minimum shared memory segment size
const SHMMIN: usize = 1;

/// Maximum shared memory attachments
const SHMSEG: usize = 4096;

// ============================================================================
// Shared Memory Functions
// ============================================================================

/// Get or create a shared memory segment
///
/// # Arguments
/// * `key` - IPC key
/// * `size` - Segment size (rounded up to page size)
/// * `shmflg` - Creation flags and permissions
///
/// # Returns
/// * Segment ID on success, -1 on failure
pub unsafe extern "C" fn shmget(key: i32, size: Size, shmflg: i32) -> i32 {
    if size < SHMMIN || size > SHMMAX as usize {
        return -1;
    }

    let create_flag = (shmflg & crate::posix::IPC_CREAT) != 0;
    let excl_flag = (shmflg & crate::posix::O_EXCL) != 0;
    let mode = (shmflg & 0o777) as Mode;

    let mut segments = SHM_SEGMENTS.lock();

    // Look for existing segment
    if let Some(segment) = segments.get(&key) {
        if excl_flag {
            return -1;
        }

        // Check permissions
        let seg_guard = segment.lock();
        if !check_permissions(&seg_guard.perm, mode) {
            return -1;
        }

        return seg_guard.id;
    }

    // Create new segment if requested
    if create_flag {
        if segments.len() >= SHM_MAX {
            return -1;
        }

        // Allocate segment ID
        let id = NEXT_SHM_ID.fetch_add(1, core::sync::atomic::Ordering::SeqCst);
        if id <= 0 {
            return -1;
        }

        // Round size up to page size
        let page_size = vm::PAGE_SIZE;
        let rounded_size = ((size + page_size - 1) / page_size) * page_size;
        let num_pages = rounded_size / page_size;

        // Allocate physical pages
        let mut pages = alloc::vec::Vec::with_capacity(num_pages);
        for _ in 0..num_pages {
            match vm::alloc_page() {
                Some(page_addr) => pages.push(vm::Page {
                    addr: page_addr,
                    size: page_size,
                    flags: 0,
                }),
                None => return -1,
            }
        }

        // Create permissions structure
        let perm = IpcPerm {
            uid: crate::process::getuid(),
            gid: crate::process::getgid(),
            cuid: crate::process::getuid(),
            cgid: crate::process::getgid(),
            mode,
            seq: 0,
            key,
        };

        // Create segment
        let segment = SharedMemorySegment {
            id,
            key,
            size: rounded_size,
            pages,
            perm,
            nattch: 0,
            creator_pid: crate::process::getpid(),
            last_attach_pid: 0,
            last_detach_time: 0,
            creation_time: 0, // TODO: Get current time
            remove_pending: false,
        };

        let segment = Arc::new(Mutex::new(segment));
        segments.insert(key, segment.clone());

        id
    } else {
        -1
    }
}

/// Attach shared memory segment to process address space
///
/// # Arguments
/// * `shmid` - Segment ID
/// * `shmaddr` - Desired attach address (null for automatic)
/// * `shmflg` - Attach flags
///
/// # Returns
/// * Virtual address of attached segment, -1 on failure
pub unsafe extern "C" fn shmat(shmid: i32, shmaddr: *mut u8, shmflg: i32) -> *mut u8 {
    if shmid <= 0 {
        return core::ptr::null_mut();
    }

    let segments = SHM_SEGMENTS.lock();

    // Find segment by ID (linear search - could be optimized)
    let segment = segments.iter()
        .find(|(_, seg)| seg.lock().id == shmid)
        .map(|(_, seg)| seg.clone());

    let segment = match segment {
        Some(seg) => seg,
        None => return core::ptr::null_mut(),
    };

    drop(segments);

    let mut seg_guard = segment.lock();

    if seg_guard.remove_pending {
        return core::ptr::null_mut();
    }

    // Check permissions
    let required_mode = if (shmflg & crate::posix::SHM_RDONLY) != 0 {
        0o4 // Read permission
    } else {
        0o6 // Read/write permission
    };

    if !check_permissions(&seg_guard.perm, required_mode) {
        return core::ptr::null_mut();
    }

    // Check attach address requirements
    if !shmaddr.is_null() {
        // Address must be page-aligned unless SHM_RND flag is set
        if (shmaddr as usize) % vm::PAGE_SIZE != 0 {
            if (shmflg & crate::posix::SHM_RND) == 0 {
                return core::ptr::null_mut();
            }
        }
    }

    // Get current process and its page table
    let (current_pid, pagetable) = match crate::process::myproc() {
        Some(pid) => {
            let mut table = crate::process::manager::PROC_TABLE.lock();
            match table.find(pid) {
                Some(proc) => (pid, proc.pagetable),
                None => return core::ptr::null_mut(),
            }
        }
        None => return core::ptr::null_mut(),
    };

    if pagetable.is_null() {
        return core::ptr::null_mut();
    }

    // Map pages into process address space
    let virt_addr = if shmaddr.is_null() {
        // Find a suitable virtual address range
        match vm::find_free_range(seg_guard.size) {
            Some(addr) => addr,
            None => return core::ptr::null_mut(),
        }
    } else {
        // Round address down to page boundary
        shmaddr as usize & !(vm::PAGE_SIZE - 1)
    };

    // Map each page
    let num_pages = seg_guard.size / vm::PAGE_SIZE;
    for i in 0..num_pages {
        let page = &seg_guard.pages[i];
        let vaddr = virt_addr + (i * vm::PAGE_SIZE);
        let perm = vm::flags::PTE_R | vm::flags::PTE_W | vm::flags::PTE_U;

        unsafe {
            if vm::map_page(pagetable, vaddr, page.addr, perm).is_err() {
                // Rollback mappings on failure
                for j in 0..i {
                    let rollback_vaddr = virt_addr + (j * vm::PAGE_SIZE);
                    let _ = vm::unmap_page(pagetable, rollback_vaddr);
                }
                return core::ptr::null_mut();
            }
        }
    }

    // Update segment statistics
    seg_guard.nattch += 1;
    seg_guard.last_attach_pid = current_pid;

    virt_addr as *mut u8
}

/// Detach shared memory segment from process address space
///
/// # Arguments
/// * `shmaddr` - Virtual address of attached segment
///
/// # Returns
/// * 0 on success, -1 on failure
pub unsafe extern "C" fn shmdt(shmaddr: *mut u8) -> i32 {
    if shmaddr.is_null() {
        return EINVAL;
    }

    let vaddr = shmaddr as usize;

    // Get current process and its page table
    let (current_pid, pagetable) = match crate::process::myproc() {
        Some(pid) => {
            let table = crate::process::manager::PROC_TABLE.lock();
            match table.find_ref(pid) {
                Some(proc) => (pid, proc.pagetable),
                None => return EINVAL,
            }
        }
        None => return EINVAL,
    };

    if pagetable.is_null() {
        return EINVAL;
    }

    // Find which segment this address belongs to
    let segments = SHM_SEGMENTS.lock();
    let mut found_segment = None;

    for segment in segments.values() {
        let seg_guard = segment.lock();

        // Check if this segment could contain the address
        // This is a simplified check - in reality we'd need to track
        // each process's attachments separately
        if seg_guard.nattch > 0 {
            // Try to find a mapping for this address
            let table = crate::process::manager::PROC_TABLE.lock();
            if let Some(proc) = table.find_ref(current_pid) {
                if vm::get_page_mapping(proc as *const crate::process::manager::Proc, vaddr).is_some() {
                    found_segment = Some(segment.clone());
                    break;
                }
            }
        }
    }

    let segment = match found_segment {
        Some(seg) => seg,
        None => return EINVAL,
    };

    drop(segments);

    let mut seg_guard = segment.lock();

    // Unmap all pages in the segment
    let num_pages = seg_guard.size / vm::PAGE_SIZE;
    let mut unmapped_pages = 0;

    for i in 0..num_pages {
        let page_vaddr = vaddr + (i * vm::PAGE_SIZE);
        if vm::unmap_page(pagetable, page_vaddr).is_ok() {
            unmapped_pages += 1;
        }
    }

    if unmapped_pages == 0 {
        return EINVAL;
    }

    // Update segment statistics
    seg_guard.nattch = seg_guard.nattch.saturating_sub(1);
    seg_guard.last_detach_time = 0; // TODO: Get current time

    // If segment is marked for removal and has no more attachments, remove it
    if seg_guard.remove_pending && seg_guard.nattch == 0 {
        let mut segments = SHM_SEGMENTS.lock();
        segments.remove(&seg_guard.key);
    }

    EOK
}

/// Control shared memory segment operations
///
/// # Arguments
/// * `shmid` - Segment ID
/// * `cmd` - Control command
/// * `buf` - Buffer for data
///
/// # Returns
/// * 0 on success, -1 on failure
pub unsafe extern "C" fn shmctl(shmid: i32, cmd: i32, buf: *mut ShmidDs) -> i32 {
    if shmid <= 0 {
        return EINVAL;
    }

    let segments = SHM_SEGMENTS.lock();

    // Find segment by ID
    let segment = segments.iter()
        .find(|(_, seg)| seg.lock().id == shmid)
        .map(|(_, seg)| seg.clone());

    let segment = match segment {
        Some(seg) => seg,
        None => return EINVAL,
    };

    drop(segments);

    match cmd {
        crate::posix::IPC_STAT => {
            if buf.is_null() {
                return EINVAL;
            }

            let seg_guard = segment.lock();
            *buf = ShmidDs {
                shm_perm: seg_guard.perm,
                shm_segsz: seg_guard.size,
                shm_atime: 0, // TODO: Track attach time
                shm_dtime: seg_guard.last_detach_time,
                shm_ctime: seg_guard.creation_time,
                shm_cpid: seg_guard.creator_pid,
                shm_lpid: seg_guard.last_attach_pid,
                shm_nattch: seg_guard.nattch,
            };
            EOK
        }

        crate::posix::IPC_SET => {
            if buf.is_null() {
                return EINVAL;
            }

            let mut seg_guard = segment.lock();
            let new_buf = &*buf;

            // Only UID, GID, and mode can be changed
            seg_guard.perm.uid = new_buf.shm_perm.uid;
            seg_guard.perm.gid = new_buf.shm_perm.gid;
            seg_guard.perm.mode = new_buf.shm_perm.mode & 0o777;

            EOK
        }

        crate::posix::IPC_RMID => {
            let mut seg_guard = segment.lock();

            // Mark for removal
            seg_guard.remove_pending = true;

            // If no current attachments, remove immediately
            if seg_guard.nattch == 0 {
                let mut segments = SHM_SEGMENTS.lock();
                segments.remove(&seg_guard.key);
            }

            EOK
        }

        _ => EINVAL,
    }
}

/// Check if the calling process has required permissions for the IPC object
fn check_permissions(perm: &IpcPerm, required_mode: Mode) -> bool {
    let current_uid = crate::process::getuid();
    let current_gid = crate::process::getgid();
    let effective_gid = current_gid; // TODO: Support effective GID

    // Check owner permissions
    if current_uid == perm.uid {
        return (perm.mode >> 6) & required_mode == required_mode;
    }

    // Check group permissions
    if current_gid == perm.gid || effective_gid == perm.gid {
        return (perm.mode >> 3) & required_mode == required_mode;
    }

    // Check other permissions
    return perm.mode & required_mode == required_mode;
}
