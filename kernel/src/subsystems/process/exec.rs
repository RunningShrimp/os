// exec system call - load and execute ELF binaries
//
// This module implements the exec() system call which loads an ELF binary
// and replaces the current process's memory image with it.

extern crate alloc;

use alloc::{string::String as AString, vec::Vec};
use core::ptr;

use crate::{
    arch::memory_layout,
    subsystems::process::{
        PROC_TABLE, TrapFrame,
        dynamic_linker::DynamicLinker,
        elf::{AuxEntry, AuxType, ElfError, ElfLoader, PT_DYNAMIC, PT_INTERP},
        myproc,
    },
    reliability::errno::{ENOENT, errno_neg},
    security::aslr::{self, MemoryRegionType},
    subsystems::mm::{
        PAGE_SIZE, kalloc, kfree,
        vm::{activate, copyout, map_pages},
        page_table_isolation::{PageTable, ENTRIES_PER_TABLE},
    },
    types::stubs::VirtAddr,
};

/// Execution error types
#[derive(Debug, Clone, PartialEq, Eq)]
enum ExecError {
    ArgTooLong,
    FileTooLarge,
    NoProcess,
    OutOfMemory,
    InvalidArgument,
    // Add other variants as needed by the code
}

impl From<ExecError> for crate::reliability::errno::Errno {
    fn from(err: ExecError) -> Self {
        match err {
            ExecError::ArgTooLong => crate::reliability::errno::E2BIG,
            ExecError::FileTooLarge => crate::reliability::errno::EFBIG,
            ExecError::NoProcess => crate::reliability::errno::ESRCH,
            ExecError::OutOfMemory => crate::reliability::errno::ENOMEM,
            _ => crate::reliability::errno::EINVAL,
        }
    }
}

impl From<ElfError> for ExecError {
    fn from(err: ElfError) -> Self {
        match err {
            ElfError::InvalidMagic
            | ElfError::Not64Bit
            | ElfError::NotLittleEndian
            | ElfError::WrongArch
            | ElfError::NotExecutable
            | ElfError::InvalidPhentsize
            | ElfError::NoLoadSegments
            | ElfError::SegmentOverlap
            | ElfError::InvalidAddress => ExecError::InvalidArgument,
            ElfError::OutOfMemory => ExecError::OutOfMemory,
            ElfError::TooLarge => ExecError::FileTooLarge,
        }
    }
}

/// Maximum size of executable file (16 MB)
pub const MAX_EXEC_SIZE: usize = 16 * 1024 * 1024;

/// Maximum number of arguments
pub const MAX_ARGS: usize = 32;

/// Maximum argument string length
pub const MAX_ARG_LEN: usize = 4096;

/// User stack size (2 pages)
pub const USER_STACK_SIZE: usize = PAGE_SIZE * 2;

/// User stack top address
pub const USER_STACK_TOP: usize = 0x8000_0000;

// Page table entry flags
const PTE_U: usize = 0x001; // User
const PTE_R: usize = 0x002; // Read
const PTE_W: usize = 0x004; // Write
const PTE_X: usize = 0x008; // Execute

/// Execute an ELF binary
fn exec(
    elf_data: &[u8],
    argv: &[&[u8]],
    envp: &[&[u8]],
    execfn: Option<&[u8]>,
) -> Result<usize, ExecError> {
    for arg in argv {
        if arg.len() > MAX_ARG_LEN {
            return Err(ExecError::ArgTooLong);
        }
    }

    if elf_data.len() > MAX_EXEC_SIZE {
        return Err(ExecError::FileTooLarge);
    }

    // Parse and validate ELF
    let loader = ElfLoader::new(elf_data)?;

    // Check if this is a dynamically linked executable
    let has_interp = loader.program_headers().any(|ph| ph.p_type == PT_INTERP);
    let has_dynamic = loader.program_headers().any(|ph| ph.p_type == PT_DYNAMIC);

    // Get current process PID
    let pid = myproc().ok_or(ExecError::NoProcess)?;

    // Initialize ASLR for this process if not already initialized
    if aslr::is_aslr_enabled() {
        let _ = aslr::init_process_aslr_by_pid(pid as u64);
    }

    // Create new page table
    let new_pagetable = create_user_pagetable()?;

    // Initialize dynamic linker if needed
    let mut dynamic_linker = if has_interp || has_dynamic {
        Some(DynamicLinker::new())
    } else {
        None
    };

    // Load ELF using the ElfLoader API
    let elf_info = loader.load(|vaddr, readable, writable, executable| {
        let pa = kalloc();
        if pa.is_null() {
            return None;
        }
        unsafe {
            let mut perm = PTE_U;
            if readable {
                perm |= PTE_R;
            }
            if writable {
                perm |= PTE_W;
            }
            if executable {
                perm |= PTE_X;
            }
            if map_pages(vaddr, PAGE_SIZE, perm).is_err() {
                kfree(pa);
                return None;
            }
        }
        Some(pa)
    })?;

    // Randomize stack base address if ASLR is enabled
    let stack_top = if aslr::is_aslr_enabled() {
        // Get architecture-specific user stack top
        let base_stack_top = memory_layout::user_stack_top();
        // Randomize stack top address
        match aslr::randomize_memory_region(
            pid as u64,
            VirtAddr::new(base_stack_top),
            USER_STACK_SIZE,
            PAGE_SIZE,
            MemoryRegionType::Stack,
        ) {
            Ok(randomized_addr) => randomized_addr.as_usize(),
            Err(_) => base_stack_top, // Fallback to base address
        }
    } else {
        USER_STACK_TOP
    };

    // Set up user stack pages
    let stack_bottom = stack_top - USER_STACK_SIZE;
    for offset in (0..USER_STACK_SIZE).step_by(PAGE_SIZE) {
        let pa = kalloc();
        if pa.is_null() {
            return Err(ExecError::OutOfMemory);
        }
        unsafe {
            ptr::write_bytes(pa, 0, PAGE_SIZE);
            let perm = PTE_U | PTE_R | PTE_W;
            if map_pages(stack_bottom + offset, PAGE_SIZE, perm).is_err()
            {
                kfree(pa);
                return Err(ExecError::OutOfMemory);
            }
        }
    }

    // Load shared libraries if dynamically linked
    if let Some(ref mut linker) = dynamic_linker {
        // Find PT_DYNAMIC segment to get dependencies
        if let Some(dynamic_phdr) = loader.program_headers().find(|ph| ph.p_type == PT_DYNAMIC) {
            // Parse dynamic section to get DT_NEEDED entries
            let dynamic_addr = elf_info.base + dynamic_phdr.p_vaddr as usize;
            let dynamic_size = dynamic_phdr.p_memsz as usize;

            // Parse DT_NEEDED entries from dynamic section
            let needed_libs = parse_needed_libraries(
                &elf_data,
                dynamic_addr - elf_info.base,
                dynamic_size,
                elf_info.base,
            )?;

            // Load each needed library
            for lib_name in needed_libs {
                let _ = linker.load_library(
                    &lib_name,
                    0, // ASLR base
                    |vaddr, readable, writable, executable| {
                        let pa = kalloc();
                        if pa.is_null() {
                            return None;
                        }
                        unsafe {
                            let mut perm = PTE_U;
                            if readable {
                                perm |= PTE_R;
                            }
                            if writable {
                                perm |= PTE_W;
                            }
                            if executable {
                                perm |= PTE_X;
                            }
                            if map_pages(vaddr, PAGE_SIZE, perm).is_err()
                            {
                                kfree(pa);
                                return None;
                            }
                        }
                        Some(pa)
                    },
                );
            }
        }
    }

    // Build auxv with common entries
    let hdr = loader.header();
    // Map and write PHDR table into user VA
    let phdr_addr = hdr.e_phoff as usize; // base assumed 0 for static
    let phdr_size = hdr.e_phentsize as usize * hdr.e_phnum as usize;
    if phdr_size > 0 {
        let _ = map_pages(
            phdr_addr,
            ((phdr_size + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE,
            PTE_U | PTE_R,
        );
        // Copy PHDR bytes
        let dst = unsafe { core::slice::from_raw_parts_mut(phdr_addr as *mut u8, phdr_size) };
        unsafe {
            copyout(
                dst,
                elf_data.as_ptr().add(hdr.e_phoff as usize),
                phdr_size,
            )
            .map_err(|_| ExecError::OutOfMemory)?;
        }
    }
    // Randomize executable base address if ASLR is enabled and it's a PIE or dynamic executable
    let dynbase = if hdr.e_type as u16 == crate::process::elf::ET_DYN || elf_info.interp.is_some() {
        let base_addr = 0x400000usize; // Default base for PIE executables
        if aslr::is_aslr_enabled() {
            // Randomize PIE executable base address
            match aslr::randomize_memory_region(
                pid as u64,
                VirtAddr::new(base_addr),
                0x2000000, // 32MB range for randomization
                PAGE_SIZE,
                MemoryRegionType::Executable,
            ) {
                Ok(randomized_addr) => randomized_addr.as_usize(),
                Err(_) => base_addr, // Fallback to base address
            }
        } else {
            base_addr
        }
    } else {
        // Static executable - use original base
        elf_info.base
    };
    let mut auxv = [
        AuxEntry { a_type: AuxType::Null as usize, a_val: 0 }, // AT_PAGESZ (using custom value 65536)
        AuxEntry { a_type: AuxType::Entry as usize, a_val: hdr.e_entry as usize },
        AuxEntry { a_type: AuxType::Phnum as usize, a_val: hdr.e_phnum as usize },
        AuxEntry { a_type: AuxType::Phent as usize, a_val: hdr.e_phentsize as usize },
        AuxEntry { a_type: AuxType::Phdr as usize, a_val: phdr_addr },
        AuxEntry { a_type: AuxType::Base as usize, a_val: dynbase },
        AuxEntry { a_type: 17, a_val: crate::subsystems::time::TIMER_FREQ as usize }, // AT_CLKTCK
        AuxEntry { a_type: AuxType::Random as usize, a_val: 0 },
        AuxEntry { a_type: AuxType::Platform as usize, a_val: 0 },
        AuxEntry { a_type: 16, a_val: hwcap() }, // AT_HWCAP
        AuxEntry { a_type: 11, a_val: 0 }, // AT_UID
        AuxEntry { a_type: 12, a_val: 0 }, // AT_EUID
        AuxEntry { a_type: AuxType::Execfn as usize, a_val: 0 },
        AuxEntry::null(),
    ];
    // Push arguments onto actual user stack memory (use randomized stack_top)
    let (sp, argc, argv_ptr) = write_args_to_stack(
        new_pagetable,
        stack_top,
        argv,
        &mut auxv,
        envp,
        execfn,
        Some(platform_bytes()),
    )?;

    let entry = elf_info.entry;

    // Update process with new address space
    {
        let mut table = PROC_TABLE.lock();
        if let Some(proc) = table.find(pid) {
            // Free old page table
            let old_pagetable = proc.pagetable;
            if !old_pagetable.is_null() {
                free_user_pagetable(old_pagetable);
            }

            // Install new page table
            proc.pagetable = new_pagetable;
            proc.sz = stack_top;

            // Set up trapframe for return to user
            let tf = proc.trapframe;
            if !tf.is_null() {
                unsafe {
                    setup_trapframe(tf, entry, sp, argc, argv_ptr);
                }
            }

            // Activate new page table
            activate();
        } else {
            // Process not found, clean up
            free_user_pagetable(new_pagetable);
            return Err(ExecError::NoProcess);
        }
    }

    Ok(entry)
}

/// Set up trapframe for returning to user space
unsafe fn setup_trapframe(tf: *mut TrapFrame, entry: usize, sp: usize, argc: usize, argv: usize) {
    // Validate entry point and stack pointer
    if entry == 0 || sp == 0 {
        return; // Invalid setup, trapframe remains unchanged
    }

    #[cfg(target_arch = "riscv64")]
    {
        unsafe {
            (*tf).epc = entry;
            (*tf).sp = sp;
            (*tf).a0 = argc;
            (*tf).a1 = argv;
            // Clear other registers for security
            (*tf).a2 = 0;
            (*tf).a3 = 0;
            (*tf).a4 = 0;
            (*tf).a5 = 0;
            (*tf).a6 = 0;
            (*tf).a7 = 0;
        }
    }

    #[cfg(target_arch = "aarch64")]
    {
        unsafe {
            (*tf).elr = entry;
            (*tf).sp = sp;
            (*tf).regs[0] = argc;
            (*tf).regs[1] = argv;
            // Clear other registers for security
            for i in 2..31 {
                (*tf).regs[i] = 0;
            }
        }
    }

    #[cfg(target_arch = "x86_64")]
    {
        unsafe {
            (*tf).rip = entry;
            (*tf).rsp = sp;
            (*tf).rdi = argc;
            (*tf).rsi = argv;
            // Clear other registers for security
            (*tf).rdx = 0;
            (*tf).rcx = 0;
            (*tf).r8 = 0;
            (*tf).r9 = 0;
            (*tf).r10 = 0;
            (*tf).r11 = 0;
        }
    }
}

/// Create a new user page table with kernel mappings
fn create_user_pagetable() -> Result<*mut PageTable, ExecError> {
    let pagetable = kalloc();
    if pagetable.is_null() {
        return Err(ExecError::OutOfMemory);
    }
    let pagetable = pagetable as *mut PageTable;
    unsafe {
        ptr::write_bytes(pagetable as *mut u8, 0, PAGE_SIZE);
        // Kernel mappings would be shared via top-half or direct map; omitted in this minimal setup
    }
    Ok(pagetable)
}

/// Free a user page table and all its pages
fn free_user_pagetable(pagetable: *mut PageTable) {
    if pagetable.is_null() {
        return;
    }

    // Walk the page table and free all user pages
    unsafe {
        free_user_pages_recursive(pagetable, 0, 3); // Start with level 0 (root)
    }

    // Free the page table itself
    unsafe {
        kfree(pagetable as *mut u8);
    }
}

/// Recursively free user pages in the page table
unsafe fn free_user_pages_recursive(pt: *mut PageTable, level: usize, max_level: usize) {
    if pt.is_null() {
        return;
    }

    for i in 0..ENTRIES_PER_TABLE {
        unsafe {
            let entry = (*pt).get_entry(i);
            let pte = entry.to_u64() as usize;

            // Check if PTE is valid
            #[cfg(target_arch = "riscv64")]
            let valid = (pte & PTE_V) != 0;

            #[cfg(target_arch = "aarch64")]
            let valid = (pte & (1 << 0)) != 0; // DESC_VALID

            #[cfg(target_arch = "x86_64")]
            let valid = (pte & (1 << 0)) != 0; // PTE_P

            if !valid {
                continue;
            }

            // Check if this is a user page (not kernel)
            #[cfg(target_arch = "riscv64")]
            let is_user = (pte & PTE_U) != 0;

            #[cfg(target_arch = "aarch64")]
            let is_user = (pte & (1 << 6)) != 0; // DESC_AP_USER

            #[cfg(target_arch = "x86_64")]
            let is_user = (pte & (1 << 2)) != 0; // PTE_US

            if !is_user {
                continue;
            }

            if level == max_level {
                // This is a leaf PTE pointing to a page - free it
                let pa = pte & !0xFFF; // Extract physical address
                if pa != 0 {
                    kfree(pa as *mut u8);
                }
            } else {
                // This is an intermediate PTE - recurse
                let next_pt = (pte & !0xFFF) as *mut PageTable;
                free_user_pages_recursive(next_pt, level + 1, max_level);

                // Free the intermediate page table
                kfree(next_pt as *mut u8);
            }
        }
    }
}

/// Map a page in the page table
// Use vm::map_pages instead

/// Activate a page table (switch address space)
// Use vm::activate instead

/// Push arguments onto user stack


/// System call handler for exec
pub fn sys_exec(path: usize, argv: usize) -> isize {
    let path_slice = match read_user_string(path) {
        Some(s) => s,
        None => return -1,
    };

    let args = match read_user_argv(argv) {
        Some(a) => a,
        None => return -1,
    };

    let arg_slices: Vec<&[u8]> = args.iter().map(|a| a.as_slice()).collect();

    // Read ELF from VFS and execute
    let path_str = match alloc::string::String::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let abs_path = resolve_with_cwd(&path_str);

    // Try to open file using fs API
    let file_handle = match crate::subsystems::fs::api::file_ops::open(
        &abs_path,
        crate::posix::O_RDONLY as u32,
        0
    ) {
        Ok(handle) => handle,
        Err(_) => return errno_neg(ENOENT),
    };

    // Read file content
    let mut buf = alloc::vec::Vec::new();
    let mut tmp = [0u8; 512];
    let tmp_len = tmp.len();
    loop {
        let n = match crate::subsystems::fs::api::file_ops::read(
            file_handle,
            &mut tmp,
            0,
            tmp_len
        ) {
            Ok(n) => n,
            Err(_) => 0,
        };
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }

    match exec(&buf, &arg_slices, &[], Some(abs_path.as_bytes())) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

pub fn sys_execve(path: usize, argv: usize, envp: usize) -> isize {
    let path_slice = match read_user_string(path) {
        Some(s) => s,
        None => return -1,
    };
    let args = match read_user_argv(argv) {
        Some(a) => a,
        None => return -1,
    };
    let envs = match read_user_argv(envp) {
        Some(a) => a,
        None => return -1,
    };
    let arg_slices: Vec<&[u8]> = args.iter().map(|a| a.as_slice()).collect();
    let env_slices: Vec<&[u8]> = envs.iter().map(|a| a.as_slice()).collect();
    let path_str = match alloc::string::String::from_utf8(path_slice) {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let abs_path = resolve_with_cwd(&path_str);

    // Try to open file using fs API
    let file_handle = match crate::subsystems::fs::api::file_ops::open(
        &abs_path,
        crate::posix::O_RDONLY as u32,
        0
    ) {
        Ok(handle) => handle,
        Err(_) => return errno_neg(ENOENT),
    };

    // Read file content
    let mut buf = alloc::vec::Vec::new();
    let mut tmp = [0u8; 512];
    let tmp_len = tmp.len();
    loop {
        let n = match crate::subsystems::fs::api::file_ops::read(
            file_handle,
            &mut tmp,
            0,
            tmp_len
        ) {
            Ok(n) => n,
            Err(_) => 0,
        };
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
    }

    match exec(&buf, &arg_slices, &env_slices, Some(abs_path.as_bytes())) {
        Ok(_) => 0,
        Err(_) => -1,
    }
}

fn join_path(base: &str, rel: &str) -> AString {
    let mut out: alloc::vec::Vec<AString> = alloc::vec::Vec::new();
    let is_abs = rel.starts_with('/');
    if !is_abs {
        for p in base.split('/').filter(|s| !s.is_empty()) {
            out.push(AString::from(p));
        }
    }
    for p in rel.split('/').filter(|s| !s.is_empty()) {
        if p == "." {
            continue;
        }
        if p == ".." {
            if !out.is_empty() {
                out.pop();
            }
            continue;
        }
        out.push(AString::from(p));
    }
    let mut s = AString::from("/");
    for (i, seg) in out.iter().enumerate() {
        if i > 0 {
            s.push('/');
        }
        s.push_str(seg);
    }
    s
}

fn resolve_with_cwd(in_path: &str) -> AString {
    if in_path.starts_with('/') {
        return AString::from(in_path);
    }
    let mut ptable = PROC_TABLE.lock();
    let cur = match crate::process::myproc()
        .and_then(|pid| ptable.find(pid).and_then(|p| p.cwd_path.clone()))
    {
        Some(s) => s,
        None => AString::from("/"),
    };
    join_path(&cur, in_path)
}

/// Read a null-terminated string from user space
fn read_user_string(addr: usize) -> Option<Vec<u8>> {
    if addr == 0 {
        return None;
    }

    let mut result = Vec::new();
    let mut ptr = addr;

    unsafe {
        loop {
            let byte = *(ptr as *const u8);
            if byte == 0 {
                break;
            }
            result.push(byte);
            ptr += 1;

            if result.len() > MAX_ARG_LEN {
                return None;
            }
        }
    }

    Some(result)
}

/// Read argv array from user space
fn read_user_argv(addr: usize) -> Option<Vec<Vec<u8>>> {
    if addr == 0 {
        return None;
    }

    let mut args = Vec::new();
    let mut ptr = addr;

    unsafe {
        loop {
            let arg_ptr = *(ptr as *const usize);
            if arg_ptr == 0 {
                break;
            }

            match read_user_string(arg_ptr) {
                Some(s) => args.push(s),
                None => return None,
            }

            ptr += core::mem::size_of::<usize>();

            if args.len() > MAX_ARGS {
                return None;
            }
        }
    }

    Some(args)
}
/// Write argv onto user stack
fn write_args_to_stack(
    _pagetable: *mut PageTable,
    stack_top: usize,
    argv: &[&[u8]],
    aux_entries: &mut [AuxEntry],
    envp: &[&[u8]],
    execfn: Option<&[u8]>,
    platform: Option<&[u8]>,
) -> Result<(usize, usize, usize), ExecError> {
    let argc = argv.len();
    let usize_sz = core::mem::size_of::<usize>();
    let mut strings_size = 0usize;
    for a in argv {
        strings_size += a.len() + 1;
    }
    for e in envp {
        strings_size += e.len() + 1;
    }
    if let Some(s) = execfn {
        strings_size += s.len() + 1;
    }
    if let Some(p) = platform {
        strings_size += p.len() + 1;
    }
    let ptrs_size = (argc + 1) * usize_sz;
    let aux_size = aux_entries.len() * 2 * usize_sz;
    let env_ptrs_size = (envp.len() + 1) * usize_sz;
    let rand_size = 16;
    let total = strings_size + ptrs_size + env_ptrs_size + aux_size + usize_sz + rand_size + 16;
    let sp = (stack_top - total) & !0xF;
    let mut cursor = sp;
    let argv_ptr = cursor;
    cursor += ptrs_size;
    let envp_ptr = cursor;
    cursor += env_ptrs_size;
    let auxv_ptr = cursor;
    cursor += aux_size;
    let strings_base = cursor;

    let mut cur = strings_base;
    let mut ptrs = alloc::vec::Vec::with_capacity(argc + 1);
    for a in argv {
        let dst = unsafe { core::slice::from_raw_parts_mut(cur as *mut u8, a.len()) };
        copyout(dst, a.as_ptr(), a.len()).map_err(|_| ExecError::OutOfMemory)?;
        let nul: [u8; 1] = [0];
        let dst_nul = unsafe { core::slice::from_raw_parts_mut((cur + a.len()) as *mut u8, 1) };
        copyout(dst_nul, nul.as_ptr(), 1)
            .map_err(|_| ExecError::OutOfMemory)?;
        ptrs.push(cur);
        cur += a.len() + 1;
    }
    ptrs.push(0);

    for (i, p) in ptrs.iter().enumerate() {
        let bytes = (*p as usize).to_le_bytes();
        let dst = unsafe { core::slice::from_raw_parts_mut((argv_ptr + i * usize_sz) as *mut u8, usize_sz) };
        copyout(dst, bytes.as_ptr(), usize_sz)
            .map_err(|_| ExecError::OutOfMemory)?;
    }
    // envp strings and pointers
    let mut env_ptrs = alloc::vec::Vec::with_capacity(envp.len() + 1);
    for e in envp {
        let dst = unsafe { core::slice::from_raw_parts_mut(cur as *mut u8, e.len()) };
        copyout(dst, e.as_ptr(), e.len()).map_err(|_| ExecError::OutOfMemory)?;
        let nul: [u8; 1] = [0];
        let dst_nul = unsafe { core::slice::from_raw_parts_mut((cur + e.len()) as *mut u8, 1) };
        copyout(dst_nul, nul.as_ptr(), 1)
            .map_err(|_| ExecError::OutOfMemory)?;
        env_ptrs.push(cur);
        cur += e.len() + 1;
    }
    env_ptrs.push(0);
    for (i, p) in env_ptrs.iter().enumerate() {
        let bytes = (*p as usize).to_le_bytes();
        let dst = unsafe { core::slice::from_raw_parts_mut((envp_ptr + i * usize_sz) as *mut u8, usize_sz) };
        copyout(dst, bytes.as_ptr(), usize_sz)
            .map_err(|_| ExecError::OutOfMemory)?;
    }

    // AT_RANDOM: write 16 bytes and update auxv entry
    let rand_ptr = cur;
    let rnd = [0u8; 16];
    let dst_rand = unsafe { core::slice::from_raw_parts_mut(rand_ptr as *mut u8, rnd.len()) };
    copyout(dst_rand, rnd.as_ptr(), rnd.len())
            .map_err(|_| ExecError::OutOfMemory)?;
    for a in aux_entries.iter_mut() {
        if a.a_type == AuxType::Random as usize {
            a.a_val = rand_ptr;
        }
    }

    if let Some(s) = execfn {
        let base = cur;
        let dst_exec = unsafe { core::slice::from_raw_parts_mut(base as *mut u8, s.len()) };
        copyout(dst_exec, s.as_ptr(), s.len()).map_err(|_| ExecError::OutOfMemory)?;
        let nul: [u8; 1] = [0];
        let dst_nul_exec = unsafe { core::slice::from_raw_parts_mut((base + s.len()) as *mut u8, 1) };
        copyout(dst_nul_exec, nul.as_ptr(), 1)
            .map_err(|_| ExecError::OutOfMemory)?;
        for a in aux_entries.iter_mut() {
            if a.a_type == AuxType::Execfn as usize {
                a.a_val = base;
            }
        }
        cur += s.len() + 1;
    }

    if let Some(p) = platform {
        let base = cur;
        let dst_platform = unsafe { core::slice::from_raw_parts_mut(base as *mut u8, p.len()) };
        copyout(dst_platform, p.as_ptr(), p.len()).map_err(|_| ExecError::OutOfMemory)?;
        let nul: [u8; 1] = [0];
        let dst_nul_platform = unsafe { core::slice::from_raw_parts_mut((base + p.len()) as *mut u8, 1) };
        copyout(dst_nul_platform, nul.as_ptr(), 1)
            .map_err(|_| ExecError::OutOfMemory)?;
        for a in aux_entries.iter_mut() {
            if a.a_type == AuxType::Platform as usize {
                a.a_val = base;
            }
        }
        // Note: cur would be updated here, but it's no longer needed
    }

    for i in 0..aux_entries.len() {
        let ty = aux_entries[i].a_type.to_le_bytes();
        let val = aux_entries[i].a_val.to_le_bytes();
        let dst_ty = unsafe { core::slice::from_raw_parts_mut((auxv_ptr + i * 2 * usize_sz) as *mut u8, usize_sz) };
        let dst_val = unsafe { core::slice::from_raw_parts_mut((auxv_ptr + i * 2 * usize_sz + usize_sz) as *mut u8, usize_sz) };
        copyout(dst_ty, ty.as_ptr(), usize_sz)
            .map_err(|_| ExecError::OutOfMemory)?;
        copyout(dst_val, val.as_ptr(), usize_sz)
            .map_err(|_| ExecError::OutOfMemory)?;
    }

    Ok((sp, argc, argv_ptr))
}

#[inline]
fn platform_bytes() -> &'static [u8] {
    #[cfg(target_arch = "riscv64")]
    {
        b"riscv64"
    }
    #[cfg(target_arch = "aarch64")]
    {
        b"aarch64"
    }
    #[cfg(target_arch = "x86_64")]
    {
        b"x86_64"
    }
}

#[inline]
fn hwcap() -> usize {
    #[cfg(target_arch = "riscv64")]
    {
        let mut misa: usize = 0;
        unsafe {
            core::arch::asm!("csrr {}, misa", out(reg) misa);
        }
        let mut h: usize = 0;
        let has = |c: u8| -> bool { (misa & (1usize << (c as usize - b'A' as usize))) != 0 };
        if has(b'I') {
            h |= 1 << 0;
        }
        if has(b'M') {
            h |= 1 << 1;
        }
        if has(b'A') {
            h |= 1 << 2;
        }
        if has(b'F') {
            h |= 1 << 3;
        }
        if has(b'D') {
            h |= 1 << 4;
        }
        if has(b'C') {
            h |= 1 << 5;
        }
        if has(b'V') {
            h |= 1 << 6;
        }
        h
    }
    #[cfg(target_arch = "aarch64")]
    {
        let mut isar0: u64 = 0;
        let mut pfr0: u64 = 0;
        unsafe {
            core::arch::asm!("mrs {}, id_aa64isar0_el1", out(reg) isar0);
            core::arch::asm!("mrs {}, id_aa64pfr0_el1", out(reg) pfr0);
        }
        let mut h: usize = 0;
        let advsimd = ((pfr0 >> 20) & 0xF) as u64;
        let fp = ((pfr0 >> 16) & 0xF) as u64;
        let aes = ((isar0 >> 4) & 0xF) as u64;
        let sha1 = ((isar0 >> 8) & 0xF) as u64;
        let sha2 = ((isar0 >> 12) & 0xF) as u64;
        let crc32 = ((isar0 >> 16) & 0xF) as u64;
        let atomics = ((isar0 >> 20) & 0xF) as u64;
        let rdm = ((isar0 >> 28) & 0xF) as u64;
        if fp != 0 {
            h |= 1 << 16;
        }
        if advsimd != 0 {
            h |= 1 << 17;
        }
        if crc32 != 0 {
            h |= 1 << 18;
        }
        if aes != 0 {
            h |= 1 << 19;
        }
        if sha1 != 0 {
            h |= 1 << 20;
        }
        if sha2 != 0 {
            h |= 1 << 21;
        }
        if atomics != 0 {
            h |= 1 << 22;
        }
        if rdm != 0 {
            h |= 1 << 23;
        }
        h
    }
    #[cfg(target_arch = "x86_64")]
    {
        let mut eax: u32 = 1;
        let mut ebx: u32 = 0;
        let mut ecx: u32 = 0;
        let mut edx: u32 = 0;
        unsafe {
            core::arch::asm!(
                "cpuid",
                inlateout("eax") eax => eax,
                out("ebx") ebx,
                out("ecx") ecx,
                out("edx") edx,
            );
        }
        let mut h: usize = 0;
        if (edx & (1 << 25)) != 0 {
            h |= 1 << 32;
        }
        if (edx & (1 << 26)) != 0 {
            h |= 1 << 33;
        }
        if (ecx & (1 << 0)) != 0 {
            h |= 1 << 34;
        }
        if (ecx & (1 << 9)) != 0 {
            h |= 1 << 35;
        }
        if (ecx & (1 << 19)) != 0 {
            h |= 1 << 36;
        }
        if (ecx & (1 << 20)) != 0 {
            h |= 1 << 37;
        }
        if (ecx & (1 << 28)) != 0 {
            h |= 1 << 38;
        }
        if (ecx & (1 << 25)) != 0 {
            h |= 1 << 42;
        }
        eax = 7;
        let mut ecx2: u32 = 0;
        let mut ebx2: u32 = 0;
        let mut edx2: u32 = 0;
        unsafe {
            core::arch::asm!(
                "cpuid",
                inlateout("eax") eax => eax,
                inlateout("ecx") ecx2 => ecx2,
                out("ebx") ebx2,
                out("edx") edx2,
            );
        }
        if (ebx2 & (1 << 3)) != 0 {
            h |= 1 << 39;
        }
        if (ebx2 & (1 << 5)) != 0 {
            h |= 1 << 40;
        }
        if (ebx2 & (1 << 8)) != 0 {
            h |= 1 << 41;
        }
        if (ebx2 & (1 << 29)) != 0 {
            h |= 1 << 43;
        }
        h
    }
}

/// Parse DT_NEEDED entries from dynamic section
fn parse_needed_libraries(
    elf_data: &[u8],
    dynamic_offset: usize,
    dynamic_size: usize,
    _base: usize,
) -> Result<Vec<AString>, ExecError> {
    use crate::process::dynamic_linker::{DT_NEEDED, DT_NULL, DT_STRSZ, DT_STRTAB, Dyn};

    let mut needed_libs = Vec::new();
    let mut strtab_offset = 0;
    let mut strtab_size = 0;

    // First pass: find string table location
    let mut offset = dynamic_offset;
    while offset + core::mem::size_of::<Dyn>() <= dynamic_offset + dynamic_size {
        let dyn_entry = unsafe { &*(elf_data.as_ptr().add(offset) as *const Dyn) };

        match dyn_entry.d_tag {
            DT_NULL => break,
            DT_STRTAB => strtab_offset = dyn_entry.d_val as usize,
            DT_STRSZ => strtab_size = dyn_entry.d_val as usize,
            _ => {},
        }

        offset += core::mem::size_of::<Dyn>();
    }

    // Second pass: collect DT_NEEDED entries
    offset = dynamic_offset;
    while offset + core::mem::size_of::<Dyn>() <= dynamic_offset + dynamic_size {
        let dyn_entry = unsafe { &*(elf_data.as_ptr().add(offset) as *const Dyn) };

        match dyn_entry.d_tag {
            DT_NULL => break,
            DT_NEEDED => {
                let name_offset = strtab_offset + dyn_entry.d_val as usize;
                if name_offset < strtab_offset + strtab_size {
                    // Read null-terminated string
                    let mut name = AString::new();
                    let mut ptr = name_offset;
                    while ptr < strtab_offset + strtab_size {
                        let byte = elf_data[ptr];
                        if byte == 0 {
                            break;
                        }
                        name.push(byte as char);
                        ptr += 1;
                    }
                    if !name.is_empty() {
                        needed_libs.push(name);
                    }
                }
            },
            _ => {},
        }

        offset += core::mem::size_of::<Dyn>();
    }

    Ok(needed_libs)
}
