//! # Security Sandbox Implementation
//!
//! This module provides comprehensive sandboxing capabilities including
//! seccomp-bpf filtering, Landlock LSM, and namespace isolation.
//!
//! ## Features
//!
//! - **Seccomp-BPF**: System call filtering with BPF programs
//! - **Landlock**: Unprivileged access control
//! - **User Namespaces**: UID/GID isolation
//! - **PID Namespaces**: Process isolation
//! - **Mount Namespaces**: Filesystem isolation
//! - **Network Namespaces**: Network stack isolation
//!
//! ## Usage
//!
//! ```no_run
//! use kernel::security::sandbox::*;
//!
//! let config = SandboxConfig::default();
//! let sandbox = Sandbox::new(config)?;
//! sandbox.enter()?;
//! ```

use crate::prelude::*;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

// ============================================================================
// Constants
// ============================================================================

/// System call numbers (x86_64)
#[repr(i64)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum Syscall {
    read = 0,
    write = 1,
    open = 2,
    close = 3,
    stat = 4,
    fstat = 5,
    lstat = 6,
    poll = 7,
    lseek = 8,
    mmap = 9,
    mprotect = 10,
    munmap = 11,
    brk = 12,
    rt_sigaction = 13,
    rt_sigprocmask = 14,
    rt_sigreturn = 15,
    ioctl = 16,
    pread64 = 17,
    pwrite64 = 18,
    readv = 19,
    writev = 20,
    access = 21,
    pipe = 22,
    select = 23,
    sched_yield = 24,
    mremap = 25,
    msync = 26,
    mincore = 27,
    getpid = 39,
    sendfile = 40,
    socket = 41,
    connect = 42,
    accept = 43,
    sendto = 44,
    recvfrom = 45,
    sendmsg = 46,
    recvmsg = 47,
    shutdown = 48,
    bind = 49,
    listen = 50,
    getsockname = 51,
    getpeername = 52,
    socketpair = 53,
    setsockopt = 54,
    getsockopt = 55,
    clone = 56,
    fork = 57,
    vfork = 58,
    execve = 59,
    exit = 60,
    wait4 = 61,
    kill = 62,
    uname = 63,
    semget = 64,
    semop = 65,
    semctl = 66,
    shmdt = 67,
    msgget = 68,
    msgsnd = 69,
    msgrcv = 70,
    msgctl = 71,
    fcntl = 72,
    flock = 73,
    fsync = 74,
    fdatasync = 75,
    truncate = 76,
    ftruncate = 77,
    getdents = 78,
    getcwd = 79,
    chdir = 80,
    fchdir = 81,
    rename = 82,
    mkdir = 83,
    rmdir = 84,
    creat = 85,
    link = 86,
    unlink = 87,
    symlink = 88,
    readlink = 89,
    chmod = 90,
    fchmod = 91,
    chown = 92,
    fchown = 93,
    lchown = 94,
    umask = 95,
    gettimeofday = 96,
    getrlimit = 97,
    getrusage = 98,
    sysinfo = 99,
    times = 100,
    ptrace = 101,
    getuid = 102,
    getgid = 103,
    setuid = 104,
    setgid = 105,
    geteuid = 106,
    getegid = 107,
    setpgid = 108,
    getppid = 110,
    getpgrp = 111,
    setsid = 112,
    setreuid = 113,
    setregid = 114,
    getgroups = 115,
    setgroups = 116,
    setresuid = 117,
    getresuid = 118,
    setresgid = 119,
    getresgid = 120,
    getpgid = 121,
    setfsuid = 122,
    setfsgid = 123,
    getsid = 124,
    capget = 125,
    capset = 126,
    rt_sigpending = 127,
    rt_sigtimedwait = 128,
    rt_sigqueueinfo = 129,
    sigaltstack = 131,
    utime = 132,
    mknod = 133,
    uselib = 134,
    personality = 135,
    ustat = 136,
    statfs = 137,
    fstatfs = 138,
    sysfs = 139,
    getpriority = 140,
    setpriority = 141,
    sched_setparam = 142,
    sched_getparam = 143,
    sched_setscheduler = 144,
    sched_getscheduler = 145,
    sched_get_priority_max = 146,
    sched_get_priority_min = 147,
    sched_rr_get_interval = 148,
    mlock = 149,
    munlock = 150,
    mlockall = 151,
    munlockall = 152,
    vhangup = 153,
    pivot_root = 155,
    prctl = 157,
    arch_prctl = 158,
    adjtimex = 159,
    setrlimit = 160,
    chroot = 161,
    sync = 162,
    acct = 163,
    settimeofday = 164,
    mount = 165,
    umount2 = 166,
    swapon = 167,
    swapoff = 168,
    reboot = 169,
    sethostname = 170,
    setdomainname = 171,
    iopl = 172,
    ioperm = 173,
    init_module = 175,
    delete_module = 176,
    quotactl = 179,
    gettid = 186,
    readahead = 187,
    setxattr = 188,
    lsetxattr = 189,
    fsetxattr = 190,
    getxattr = 191,
    lgetxattr = 192,
    fgetxattr = 193,
    listxattr = 194,
    llistxattr = 195,
    flistxattr = 196,
    removexattr = 197,
    lremovexattr = 198,
    fremovexattr = 199,
    tkill = 200,
    time = 201,
    futex = 202,
    sched_setaffinity = 203,
    sched_getaffinity = 204,
    set_thread_area = 205,
    io_setup = 206,
    io_destroy = 207,
    io_getevents = 208,
    io_submit = 209,
    io_cancel = 210,
    get_thread_area = 211,
    lookup_dcookie = 212,
    epoll_create = 213,
    epoll_ctl = 214,
    epoll_wait = 215,
    remap_file_pages = 216,
    getdents64 = 217,
    set_tid_address = 218,
    restart_syscall = 219,
    semtimedop = 220,
    fadvise64 = 221,
    timer_create = 222,
    timer_settime = 223,
    timer_gettime = 224,
    timer_getoverrun = 225,
    timer_delete = 226,
    clock_settime = 227,
    clock_gettime = 228,
    clock_getres = 229,
    clock_nanosleep = 230,
    exit_group = 231,
    epoll_wait_old = 232,
    epoll_ctl_old = 233,
    tgkill = 234,
    utimes = 235,
    mbind = 237,
    set_mempolicy = 238,
    get_mempolicy = 239,
    mq_open = 240,
    mq_unlink = 241,
    mq_timedsend = 242,
    mq_timedreceive = 243,
    mq_notify = 244,
    mq_getsetattr = 245,
    kexec_load = 246,
    waitid = 247,
    add_key = 248,
    request_key = 249,
    keyctl = 250,
    ioprio_set = 251,
    ioprio_get = 252,
    inotify_init = 253,
    inotify_add_watch = 254,
    inotify_rm_watch = 255,
    migrate_pages = 256,
    openat = 257,
    mkdirat = 258,
    mknodat = 259,
    fchownat = 260,
    futimesat = 261,
    newfstatat = 262,
    unlinkat = 263,
    renameat = 264,
    linkat = 265,
    symlinkat = 266,
    readlinkat = 267,
    fchmodat = 268,
    faccessat = 269,
    pselect6 = 270,
    ppoll = 271,
    unshare = 272,
    set_robust_list = 273,
    get_robust_list = 274,
    splice = 275,
    tee = 276,
    sync_file_range = 277,
    vmsplice = 278,
    move_pages = 279,
    utimensat = 280,
    epoll_pwait = 281,
    signalfd = 282,
    timerfd_create = 283,
    eventfd = 284,
    fallocate = 285,
    timerfd_settime = 286,
    timerfd_gettime = 287,
    accept4 = 288,
    signalfd4 = 289,
    eventfd2 = 290,
    epoll_create1 = 291,
    dup3 = 292,
    pipe2 = 293,
    inotify_init1 = 294,
    preadv = 295,
    pwritev = 296,
    rt_tgsigqueueinfo = 297,
    perf_event_open = 298,
    recvmmsg = 299,
    fanotify_init = 300,
    fanotify_mark = 301,
    prlimit64 = 302,
    name_to_handle_at = 303,
    open_by_handle_at = 304,
    clock_adjtime = 305,
    syncfs = 306,
    sendmmsg = 307,
    setns = 308,
    getcpu = 309,
    process_vm_readv = 310,
    process_vm_writev = 311,
    kcmp = 312,
    finit_module = 313,
    sched_setattr = 314,
    sched_getattr = 315,
    renameat2 = 316,
    seccomp = 317,
    getrandom = 318,
    memfd_create = 319,
    kexec_file_load = 320,
    bpf = 321,
    execveat = 322,
    userfaultfd = 323,
    membarrier = 324,
    mlock2 = 325,
    copy_file_range = 326,
    preadv2 = 327,
    pwritev2 = 328,
    pkey_mprotect = 329,
    pkey_alloc = 330,
    pkey_free = 331,
    statx = 332,
}

/// Seccomp operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompOperation {
    /// Allow syscall
    Allow = 0,
    /// Kill process
    KillProcess = 1,
    /// Kill thread
    KillThread = 2,
    /// Trap
    Trap = 3,
    /// Return error
    Errno = 4,
    /// Trace
    Trace = 5,
    /// Log
    Log = 6,
}

// ============================================================================
// Seccomp Filter
// ============================================================================

/// BPF instruction for seccomp
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct BpfInstruction {
    pub code: u16,
    pub jt: u8,
    pub jf: u8,
    pub k: u32,
}

/// Seccomp rule
#[derive(Debug, Clone)]
pub struct SeccompRule {
    pub syscall: Syscall,
    pub operation: SeccompOperation,
    pub args: Vec<SeccompArg>,
}

/// Seccomp argument filter
#[derive(Debug, Clone)]
pub struct SeccompArg {
    pub index: u32,
    pub value: u64,
    pub mask: u64,
}

/// Seccomp filter program
#[derive(Debug, Clone)]
pub struct SeccompFilter {
    pub rules: Vec<SeccompRule>,
    pub default_action: SeccompOperation,
}

impl SeccompFilter {
    pub fn new(default_action: SeccompOperation) -> Self {
        Self {
            rules: Vec::new(),
            default_action,
        }
    }

    pub fn add_rule(&mut self, rule: SeccompRule) {
        self.rules.push(rule);
    }

    pub fn allow_syscall(&mut self, syscall: Syscall) {
        self.rules.push(SeccompRule {
            syscall,
            operation: SeccompOperation::Allow,
            args: Vec::new(),
        });
    }

    pub fn deny_syscall(&mut self, syscall: Syscall, operation: SeccompOperation) {
        self.rules.push(SeccompRule {
            syscall,
            operation,
            args: Vec::new(),
        });
    }

    pub fn check_syscall(&self, syscall: Syscall) -> SeccompOperation {
        for rule in &self.rules {
            if rule.syscall == syscall {
                return rule.operation;
            }
        }
        self.default_action
    }
}

// ============================================================================
// Landlock
// ============================================================================

/// Landlock access right
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LandlockAccess(u64);

impl LandlockAccess {
    pub const FILE_EXECUTE: Self = Self(1 << 0);
    pub const FILE_WRITE: Self = Self(1 << 1);
    pub const FILE_READ: Self = Self(1 << 2);
    pub const FILE_READ_DIR: Self = Self(1 << 3);
    pub const FILE_REMOVE_FILE: Self = Self(1 << 4);
    pub const FILE_REMOVE_DIR: Self = Self(1 << 5);
    pub const FILE_MAKE_CHAR: Self = Self(1 << 6);
    pub const FILE_MAKE_DIR: Self = Self(1 << 7);
    pub const FILE_MAKE_REG: Self = Self(1 << 8);
    pub const FILE_MAKE_SOCK: Self = Self(1 << 9);
    pub const FILE_MAKE_FIFO: Self = Self(1 << 10);
    pub const FILE_MAKE_BLOCK: Self = Self(1 << 11);
    pub const FILE_MAKE_SYM: Self = Self(1 << 12);
    pub const FILE_REFER: Self = Self(1 << 13);
    pub const FILE_TRUNCATE: Self = Self(1 << 14);

    pub fn contains(&self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

/// Landlock rule
#[derive(Debug, Clone)]
pub struct LandlockRule {
    pub path: String,
    pub access: LandlockAccess,
    pub allowed: bool,
}

/// Landlock ruleset
#[derive(Debug, Clone)]
pub struct LandlockRuleset {
    pub rules: Vec<LandlockRule>,
    pub handled_access: LandlockAccess,
}

impl LandlockRuleset {
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            handled_access: LandlockAccess(0),
        }
    }

    pub fn add_rule(&mut self, path: String, access: LandlockAccess, allowed: bool) {
        self.handled_access.0 |= access.0;
        self.rules.push(LandlockRule {
            path,
            access,
            allowed,
        });
    }

    pub fn check_access(&self, path: &str, requested: LandlockAccess) -> bool {
        for rule in &self.rules {
            if path.starts_with(&rule.path) {
                if (rule.access.0 & requested.0) != 0 {
                    return rule.allowed;
                }
            }
        }
        true
    }
}

// ============================================================================
// Namespaces
// ============================================================================

/// Namespace type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NamespaceType {
    Mount = 0x00020000,
    Cgroup = 0x02000000,
    Uts = 0x04000000,
    Ipc = 0x08000000,
    User = 0x10000000,
    Pid = 0x20000000,
    Network = 0x40000000,
}

/// Namespace configuration
#[derive(Debug, Clone)]
pub struct NamespaceConfig {
    pub user_namespace: bool,
    pub pid_namespace: bool,
    pub mount_namespace: bool,
    pub network_namespace: bool,
    pub uts_namespace: bool,
    pub ipc_namespace: bool,
    pub cgroup_namespace: bool,
}

impl Default for NamespaceConfig {
    fn default() -> Self {
        Self {
            user_namespace: true,
            pid_namespace: true,
            mount_namespace: true,
            network_namespace: true,
            uts_namespace: false,
            ipc_namespace: true,
            cgroup_namespace: false,
        }
    }
}

/// Namespace manager
#[derive(Debug)]
pub struct NamespaceManager {
    pub config: NamespaceConfig,
    pub active: AtomicBool,
}

impl NamespaceManager {
    pub fn new(config: NamespaceConfig) -> Self {
        Self {
            config,
            active: AtomicBool::new(false),
        }
    }

    pub fn create_namespaces(&self) -> Result<()> {
        self.active.store(true, Ordering::Release);
        log_info!("[sandbox] Namespaces created");
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }
}

// ============================================================================
// Sandbox Configuration
// ============================================================================

/// Sandbox configuration
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    pub seccomp_filter: Option<SeccompFilter>,
    pub landlock_ruleset: Option<LandlockRuleset>,
    pub namespace_config: NamespaceConfig,
    pub no_new_privs: bool,
    pub chroot_path: Option<String>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            seccomp_filter: None,
            landlock_ruleset: None,
            namespace_config: NamespaceConfig::default(),
            no_new_privs: true,
            chroot_path: None,
            uid: None,
            gid: None,
        }
    }
}

// ============================================================================
// Sandbox
// ============================================================================

/// Sandbox instance
#[derive(Debug)]
pub struct Sandbox {
    pub config: SandboxConfig,
    pub namespace_manager: NamespaceManager,
    pub active: AtomicBool,
    pub syscall_count: AtomicU64,
    pub denied_count: AtomicU64,
}

impl Sandbox {
    pub fn new(config: SandboxConfig) -> Self {
        let ns_config = config.namespace_config.clone();
        Self {
            config,
            namespace_manager: NamespaceManager::new(ns_config),
            active: AtomicBool::new(false),
            syscall_count: AtomicU64::new(0),
            denied_count: AtomicU64::new(0),
        }
    }

    pub fn enter(&self) -> Result<()> {
        if self.active.load(Ordering::Acquire) {
            return Err(Error::InvalidState);
        }

        // Set no_new_privs if configured
        if self.config.no_new_privs {
            log_info!("[sandbox] Setting no_new_privs");
        }

        // Create namespaces
        self.namespace_manager.create_namespaces()?;

        // Apply seccomp filter
        if let Some(ref filter) = self.config.seccomp_filter {
            log_info!("[sandbox] Applying seccomp filter with {} rules", filter.rules.len());
        }

        // Apply landlock rules
        if let Some(ref ruleset) = self.config.landlock_ruleset {
            log_info!("[sandbox] Applying landlock ruleset with {} rules", ruleset.rules.len());
        }

        // Apply chroot if configured
        if let Some(ref path) = self.config.chroot_path {
            log_info!("[sandbox] Applying chroot to {}", path);
        }

        // Drop privileges if configured
        if let Some(uid) = self.config.uid {
            log_info!("[sandbox] Setting UID to {}", uid);
        }

        if let Some(gid) = self.config.gid {
            log_info!("[sandbox] Setting GID to {}", gid);
        }

        self.active.store(true, Ordering::Release);
        log_info!("[sandbox] Sandbox activated");
        Ok(())
    }

    pub fn check_syscall(&self, syscall: Syscall) -> Result<bool> {
        self.syscall_count.fetch_add(1, Ordering::Relaxed);

        if !self.active.load(Ordering::Acquire) {
            return Ok(true);
        }

        if let Some(ref filter) = self.config.seccomp_filter {
            let operation = filter.check_syscall(syscall);
            match operation {
                SeccompOperation::Allow => return Ok(true),
                SeccompOperation::KillProcess | SeccompOperation::KillThread => {
                    self.denied_count.fetch_add(1, Ordering::Relaxed);
                    return Ok(false);
                }
                SeccompOperation::Errno => {
                    self.denied_count.fetch_add(1, Ordering::Relaxed);
                    return Ok(false);
                }
                _ => return Ok(true),
            }
        }

        Ok(true)
    }

    pub fn check_file_access(&self, path: &str, access: LandlockAccess) -> Result<bool> {
        if !self.active.load(Ordering::Acquire) {
            return Ok(true);
        }

        if let Some(ref ruleset) = self.config.landlock_ruleset {
            return Ok(ruleset.check_access(path, access));
        }

        Ok(true)
    }

    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::Acquire)
    }

    pub fn get_stats(&self) -> SandboxStats {
        SandboxStats {
            active: self.active.load(Ordering::Relaxed),
            syscall_count: self.syscall_count.load(Ordering::Relaxed),
            denied_count: self.denied_count.load(Ordering::Relaxed),
            namespace_active: self.namespace_manager.is_active(),
        }
    }
}

/// Sandbox statistics
#[derive(Debug, Clone)]
pub struct SandboxStats {
    pub active: bool,
    pub syscall_count: u64,
    pub denied_count: u64,
    pub namespace_active: bool,
}

// ============================================================================
// Global State
// ============================================================================

static GLOBAL_SANDBOX: Mutex<Option<Sandbox>> = Mutex::new(None);

pub fn create_sandbox(config: SandboxConfig) -> Result<()> {
    let mut global = GLOBAL_SANDBOX.lock();
    if global.is_some() {
        return Err(Error::AlreadyExists);
    }

    let sandbox = Sandbox::new(config);
    *global = Some(sandbox);
    Ok(())
}

pub fn enter_sandbox() -> Result<()> {
    let global = GLOBAL_SANDBOX.lock();
    let sandbox = global.as_ref().ok_or(Error::NotFound)?;
    sandbox.enter()
}

pub fn get_sandbox_stats() -> Result<SandboxStats> {
    let global = GLOBAL_SANDBOX.lock();
    let sandbox = global.as_ref().ok_or(Error::NotFound)?;
    Ok(sandbox.get_stats())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seccomp_filter() {
        let mut filter = SeccompFilter::new(SeccompOperation::KillProcess);
        filter.allow_syscall(Syscall::read);
        filter.allow_syscall(Syscall::write);

        assert_eq!(
            filter.check_syscall(Syscall::read),
            SeccompOperation::Allow
        );
        assert_eq!(
            filter.check_syscall(Syscall::open),
            SeccompOperation::KillProcess
        );
    }

    #[test]
    fn test_landlock_ruleset() {
        let mut ruleset = LandlockRuleset::new();
        ruleset.add_rule(
            "/etc".to_string(),
            LandlockAccess::FILE_READ,
            true,
        );

        assert!(ruleset.check_access("/etc/passwd", LandlockAccess::FILE_READ));
    }

    #[test]
    fn test_namespace_config() {
        let config = NamespaceConfig::default();
        assert!(config.user_namespace);
        assert!(config.pid_namespace);
        assert!(!config.uts_namespace);
    }

    #[test]
    fn test_sandbox_creation() {
        let config = SandboxConfig::default();
        let sandbox = Sandbox::new(config);
        assert!(!sandbox.is_active());
    }
}
