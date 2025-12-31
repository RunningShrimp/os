//! Software Performance Counters
//!
//! This module provides software performance counters for tracking system-level metrics
//! including syscalls, context switches, interrupts, scheduler statistics, and lock contention.

use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use crate::cpu::NCPU;

/// Maximum number of system call types to track
const MAX_SYSCALL_TYPES: usize = 256;

/// Maximum number of interrupt types to track
const MAX_INTERRUPT_TYPES: usize = 256;

/// Software counter categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SoftwareCounterType {
    // System Call Counters
    SyscallTotal,
    SyscallRead,
    SyscallWrite,
    SyscallOpen,
    SyscallClose,
    SyscallStat,
    SyscallFstat,
    SyscallPoll,
    SyscallMmap,
    SyscallMunmap,
    SyscallIoctl,
    SyscallSocket,
    SyscallConnect,
    SyscallAccept,
    SyscallSend,
    SyscallRecv,

    // Page Fault Counters
    PageFaultMajor,
    PageFaultMinor,
    PageFaultTotal,

    // Context Switch Counters
    ContextSwitchVoluntary,
    ContextSwitchInvoluntary,
    ContextSwitchTotal,

    // Interrupt Counters
    InterruptTotal,
    InterruptTimer,
    InterruptNetwork,
    InterruptDisk,
    InterruptKeyboard,

    // Scheduler Counters
    ScheduleCount,
    ScheduleLatencyTicks,
    RunqueueLength,
    IdleTimeTicks,
    RunTimeTicks,

    // Lock Counters
    LockSpinAcquisitions,
    LockSpinContentions,
    LockMutexAcquisitions,
    LockMutexContentions,
    LockRwlockReadAcquisitions,
    LockRwlockWriteAcquisitions,
    LockRwlockContentions,

    // Memory Counters
    MemoryAllocationCount,
    MemoryDeallocationCount,
    MemoryPageAllocations,
    MemoryPageDeallocations,

    // Network Counters
    NetworkPacketsReceived,
    NetworkPacketsSent,
    NetworkBytesReceived,
    NetworkBytesSent,

    // Filesystem Counters
    FileReads,
    FileWrites,
    FileOpens,
    FileCloses,
    FileSeeks,

    // Process Counters
    ProcessCreated,
    ProcessExited,
    ThreadCreated,
    ThreadExited,

    // Error Counters
    ErrorsSyscall,
    ErrorsMemory,
    ErrorsFilesystem,
    ErrorsNetwork,
}

impl SoftwareCounterType {
    /// Get all software counter types
    pub fn all() -> Vec<Self> {
        vec![
            Self::SyscallTotal,
            Self::SyscallRead,
            Self::SyscallWrite,
            Self::SyscallOpen,
            Self::SyscallClose,
            Self::SyscallStat,
            Self::SyscallFstat,
            Self::SyscallPoll,
            Self::SyscallMmap,
            Self::SyscallMunmap,
            Self::SyscallIoctl,
            Self::SyscallSocket,
            Self::SyscallConnect,
            Self::SyscallAccept,
            Self::SyscallSend,
            Self::SyscallRecv,
            Self::PageFaultMajor,
            Self::PageFaultMinor,
            Self::PageFaultTotal,
            Self::ContextSwitchVoluntary,
            Self::ContextSwitchInvoluntary,
            Self::ContextSwitchTotal,
            Self::InterruptTotal,
            Self::InterruptTimer,
            Self::InterruptNetwork,
            Self::InterruptDisk,
            Self::InterruptKeyboard,
            Self::ScheduleCount,
            Self::ScheduleLatencyTicks,
            Self::RunqueueLength,
            Self::IdleTimeTicks,
            Self::RunTimeTicks,
            Self::LockSpinAcquisitions,
            Self::LockSpinContentions,
            Self::LockMutexAcquisitions,
            Self::LockMutexContentions,
            Self::LockRwlockReadAcquisitions,
            Self::LockRwlockWriteAcquisitions,
            Self::LockRwlockContentions,
            Self::MemoryAllocationCount,
            Self::MemoryDeallocationCount,
            Self::MemoryPageAllocations,
            Self::MemoryPageDeallocations,
            Self::NetworkPacketsReceived,
            Self::NetworkPacketsSent,
            Self::NetworkBytesReceived,
            Self::NetworkBytesSent,
            Self::FileReads,
            Self::FileWrites,
            Self::FileOpens,
            Self::FileCloses,
            Self::FileSeeks,
            Self::ProcessCreated,
            Self::ProcessExited,
            Self::ThreadCreated,
            Self::ThreadExited,
            Self::ErrorsSyscall,
            Self::ErrorsMemory,
            Self::ErrorsFilesystem,
            Self::ErrorsNetwork,
        ]
    }

    /// Get counter name
    pub fn name(&self) -> &str {
        match self {
            Self::SyscallTotal => "syscall_total",
            Self::SyscallRead => "syscall_read",
            Self::SyscallWrite => "syscall_write",
            Self::SyscallOpen => "syscall_open",
            Self::SyscallClose => "syscall_close",
            Self::SyscallStat => "syscall_stat",
            Self::SyscallFstat => "syscall_fstat",
            Self::SyscallPoll => "syscall_poll",
            Self::SyscallMmap => "syscall_mmap",
            Self::SyscallMunmap => "syscall_munmap",
            Self::SyscallIoctl => "syscall_ioctl",
            Self::SyscallSocket => "syscall_socket",
            Self::SyscallConnect => "syscall_connect",
            Self::SyscallAccept => "syscall_accept",
            Self::SyscallSend => "syscall_send",
            Self::SyscallRecv => "syscall_recv",
            Self::PageFaultMajor => "page_fault_major",
            Self::PageFaultMinor => "page_fault_minor",
            Self::PageFaultTotal => "page_fault_total",
            Self::ContextSwitchVoluntary => "context_switch_voluntary",
            Self::ContextSwitchInvoluntary => "context_switch_involuntary",
            Self::ContextSwitchTotal => "context_switch_total",
            Self::InterruptTotal => "interrupt_total",
            Self::InterruptTimer => "interrupt_timer",
            Self::InterruptNetwork => "interrupt_network",
            Self::InterruptDisk => "interrupt_disk",
            Self::InterruptKeyboard => "interrupt_keyboard",
            Self::ScheduleCount => "schedule_count",
            Self::ScheduleLatencyTicks => "schedule_latency_ticks",
            Self::RunqueueLength => "runqueue_length",
            Self::IdleTimeTicks => "idle_time_ticks",
            Self::RunTimeTicks => "run_time_ticks",
            Self::LockSpinAcquisitions => "lock_spin_acquisitions",
            Self::LockSpinContentions => "lock_spin_contentions",
            Self::LockMutexAcquisitions => "lock_mutex_acquisitions",
            Self::LockMutexContentions => "lock_mutex_contentions",
            Self::LockRwlockReadAcquisitions => "lock_rwlock_read_acquisitions",
            Self::LockRwlockWriteAcquisitions => "lock_rwlock_write_acquisitions",
            Self::LockRwlockContentions => "lock_rwlock_contentions",
            Self::MemoryAllocationCount => "memory_allocation_count",
            Self::MemoryDeallocationCount => "memory_deallocation_count",
            Self::MemoryPageAllocations => "memory_page_allocations",
            Self::MemoryPageDeallocations => "memory_page_deallocations",
            Self::NetworkPacketsReceived => "network_packets_received",
            Self::NetworkPacketsSent => "network_packets_sent",
            Self::NetworkBytesReceived => "network_bytes_received",
            Self::NetworkBytesSent => "network_bytes_sent",
            Self::FileReads => "file_reads",
            Self::FileWrites => "file_writes",
            Self::FileOpens => "file_opens",
            Self::FileCloses => "file_closes",
            Self::FileSeeks => "file_seeks",
            Self::ProcessCreated => "process_created",
            Self::ProcessExited => "process_exited",
            Self::ThreadCreated => "thread_created",
            Self::ThreadExited => "thread_exited",
            Self::ErrorsSyscall => "errors_syscall",
            Self::ErrorsMemory => "errors_memory",
            Self::ErrorsFilesystem => "errors_filesystem",
            Self::ErrorsNetwork => "errors_network",
        }
    }

    /// Get counter description
    pub fn description(&self) -> &str {
        match self {
            Self::SyscallTotal => "Total system calls",
            Self::SyscallRead => "Read system calls",
            Self::SyscallWrite => "Write system calls",
            Self::SyscallOpen => "Open system calls",
            Self::SyscallClose => "Close system calls",
            Self::SyscallStat => "Stat system calls",
            Self::SyscallFstat => "Fstat system calls",
            Self::SyscallPoll => "Poll system calls",
            Self::SyscallMmap => "Mmap system calls",
            Self::SyscallMunmap => "Munmap system calls",
            Self::SyscallIoctl => "Ioctl system calls",
            Self::SyscallSocket => "Socket system calls",
            Self::SyscallConnect => "Connect system calls",
            Self::SyscallAccept => "Accept system calls",
            Self::SyscallSend => "Send system calls",
            Self::SyscallRecv => "Receive system calls",
            Self::PageFaultMajor => "Major page faults (disk I/O required)",
            Self::PageFaultMinor => "Minor page faults (no disk I/O)",
            Self::PageFaultTotal => "Total page faults",
            Self::ContextSwitchVoluntary => "Voluntary context switches",
            Self::ContextSwitchInvoluntary => "Involuntary context switches",
            Self::ContextSwitchTotal => "Total context switches",
            Self::InterruptTotal => "Total interrupts",
            Self::InterruptTimer => "Timer interrupts",
            Self::InterruptNetwork => "Network interrupts",
            Self::InterruptDisk => "Disk interrupts",
            Self::InterruptKeyboard => "Keyboard interrupts",
            Self::ScheduleCount => "Schedule operations",
            Self::ScheduleLatencyTicks => "Total schedule latency (ticks)",
            Self::RunqueueLength => "Current runqueue length",
            Self::IdleTimeTicks => "Total idle time (ticks)",
            Self::RunTimeTicks => "Total run time (ticks)",
            Self::LockSpinAcquisitions => "Spinlock acquisitions",
            Self::LockSpinContentions => "Spinlock contentions",
            Self::LockMutexAcquisitions => "Mutex acquisitions",
            Self::LockMutexContentions => "Mutex contentions",
            Self::LockRwlockReadAcquisitions => "RWLock read acquisitions",
            Self::LockRwlockWriteAcquisitions => "RWLock write acquisitions",
            Self::LockRwlockContentions => "RWLock contentions",
            Self::MemoryAllocationCount => "Memory allocations",
            Self::MemoryDeallocationCount => "Memory deallocations",
            Self::MemoryPageAllocations => "Page allocations",
            Self::MemoryPageDeallocations => "Page deallocations",
            Self::NetworkPacketsReceived => "Network packets received",
            Self::NetworkPacketsSent => "Network packets sent",
            Self::NetworkBytesReceived => "Network bytes received",
            Self::NetworkBytesSent => "Network bytes sent",
            Self::FileReads => "File read operations",
            Self::FileWrites => "File write operations",
            Self::FileOpens => "File open operations",
            Self::FileCloses => "File close operations",
            Self::FileSeeks => "File seek operations",
            Self::ProcessCreated => "Processes created",
            Self::ProcessExited => "Processes exited",
            Self::ThreadCreated => "Threads created",
            Self::ThreadExited => "Threads exited",
            Self::ErrorsSyscall => "System call errors",
            Self::ErrorsMemory => "Memory errors",
            Self::ErrorsFilesystem => "Filesystem errors",
            Self::ErrorsNetwork => "Network errors",
        }
    }

    /// Get counter unit
    pub fn unit(&self) -> &str {
        match self {
            Self::RunqueueLength => "processes",
            Self::IdleTimeTicks | Self::RunTimeTicks | Self::ScheduleLatencyTicks => "ticks",
            Self::NetworkBytesReceived | Self::NetworkBytesSent => "bytes",
            _ => "count",
        }
    }
}

/// Per-CPU software counters
#[repr(C)]
pub struct PerCpuSoftwareCounters {
    // System call counters
    pub syscall_total: AtomicU64,
    pub syscall_read: AtomicU64,
    pub syscall_write: AtomicU64,
    pub syscall_open: AtomicU64,
    pub syscall_close: AtomicU64,
    pub syscall_stat: AtomicU64,
    pub syscall_fstat: AtomicU64,
    pub syscall_poll: AtomicU64,
    pub syscall_mmap: AtomicU64,
    pub syscall_munmap: AtomicU64,
    pub syscall_ioctl: AtomicU64,
    pub syscall_socket: AtomicU64,
    pub syscall_connect: AtomicU64,
    pub syscall_accept: AtomicU64,
    pub syscall_send: AtomicU64,
    pub syscall_recv: AtomicU64,

    // Page fault counters
    pub page_fault_major: AtomicU64,
    pub page_fault_minor: AtomicU64,
    pub page_fault_total: AtomicU64,

    // Context switch counters
    pub context_switch_voluntary: AtomicU64,
    pub context_switch_involuntary: AtomicU64,
    pub context_switch_total: AtomicU64,

    // Interrupt counters
    pub interrupt_total: AtomicU64,
    pub interrupt_timer: AtomicU64,
    pub interrupt_network: AtomicU64,
    pub interrupt_disk: AtomicU64,
    pub interrupt_keyboard: AtomicU64,

    // Scheduler counters
    pub schedule_count: AtomicU64,
    pub schedule_latency_ticks: AtomicU64,
    pub runqueue_length: AtomicU64,
    pub idle_time_ticks: AtomicU64,
    pub run_time_ticks: AtomicU64,

    // Lock counters
    pub lock_spin_acquisitions: AtomicU64,
    pub lock_spin_contentions: AtomicU64,
    pub lock_mutex_acquisitions: AtomicU64,
    pub lock_mutex_contentions: AtomicU64,
    pub lock_rwlock_read_acquisitions: AtomicU64,
    pub lock_rwlock_write_acquisitions: AtomicU64,
    pub lock_rwlock_contentions: AtomicU64,

    // Memory counters
    pub memory_allocation_count: AtomicU64,
    pub memory_deallocation_count: AtomicU64,
    pub memory_page_allocations: AtomicU64,
    pub memory_page_deallocations: AtomicU64,

    // Network counters
    pub network_packets_received: AtomicU64,
    pub network_packets_sent: AtomicU64,
    pub network_bytes_received: AtomicU64,
    pub network_bytes_sent: AtomicU64,

    // Filesystem counters
    pub file_reads: AtomicU64,
    pub file_writes: AtomicU64,
    pub file_opens: AtomicU64,
    pub file_closes: AtomicU64,
    pub file_seeks: AtomicU64,

    // Process counters
    pub process_created: AtomicU64,
    pub process_exited: AtomicU64,
    pub thread_created: AtomicU64,
    pub thread_exited: AtomicU64,

    // Error counters
    pub errors_syscall: AtomicU64,
    pub errors_memory: AtomicU64,
    pub errors_filesystem: AtomicU64,
    pub errors_network: AtomicU64,
}

impl PerCpuSoftwareCounters {
    /// Create new per-CPU software counters
    pub const fn new() -> Self {
        Self {
            syscall_total: AtomicU64::new(0),
            syscall_read: AtomicU64::new(0),
            syscall_write: AtomicU64::new(0),
            syscall_open: AtomicU64::new(0),
            syscall_close: AtomicU64::new(0),
            syscall_stat: AtomicU64::new(0),
            syscall_fstat: AtomicU64::new(0),
            syscall_poll: AtomicU64::new(0),
            syscall_mmap: AtomicU64::new(0),
            syscall_munmap: AtomicU64::new(0),
            syscall_ioctl: AtomicU64::new(0),
            syscall_socket: AtomicU64::new(0),
            syscall_connect: AtomicU64::new(0),
            syscall_accept: AtomicU64::new(0),
            syscall_send: AtomicU64::new(0),
            syscall_recv: AtomicU64::new(0),
            page_fault_major: AtomicU64::new(0),
            page_fault_minor: AtomicU64::new(0),
            page_fault_total: AtomicU64::new(0),
            context_switch_voluntary: AtomicU64::new(0),
            context_switch_involuntary: AtomicU64::new(0),
            context_switch_total: AtomicU64::new(0),
            interrupt_total: AtomicU64::new(0),
            interrupt_timer: AtomicU64::new(0),
            interrupt_network: AtomicU64::new(0),
            interrupt_disk: AtomicU64::new(0),
            interrupt_keyboard: AtomicU64::new(0),
            schedule_count: AtomicU64::new(0),
            schedule_latency_ticks: AtomicU64::new(0),
            runqueue_length: AtomicU64::new(0),
            idle_time_ticks: AtomicU64::new(0),
            run_time_ticks: AtomicU64::new(0),
            lock_spin_acquisitions: AtomicU64::new(0),
            lock_spin_contentions: AtomicU64::new(0),
            lock_mutex_acquisitions: AtomicU64::new(0),
            lock_mutex_contentions: AtomicU64::new(0),
            lock_rwlock_read_acquisitions: AtomicU64::new(0),
            lock_rwlock_write_acquisitions: AtomicU64::new(0),
            lock_rwlock_contentions: AtomicU64::new(0),
            memory_allocation_count: AtomicU64::new(0),
            memory_deallocation_count: AtomicU64::new(0),
            memory_page_allocations: AtomicU64::new(0),
            memory_page_deallocations: AtomicU64::new(0),
            network_packets_received: AtomicU64::new(0),
            network_packets_sent: AtomicU64::new(0),
            network_bytes_received: AtomicU64::new(0),
            network_bytes_sent: AtomicU64::new(0),
            file_reads: AtomicU64::new(0),
            file_writes: AtomicU64::new(0),
            file_opens: AtomicU64::new(0),
            file_closes: AtomicU64::new(0),
            file_seeks: AtomicU64::new(0),
            process_created: AtomicU64::new(0),
            process_exited: AtomicU64::new(0),
            thread_created: AtomicU64::new(0),
            thread_exited: AtomicU64::new(0),
            errors_syscall: AtomicU64::new(0),
            errors_memory: AtomicU64::new(0),
            errors_filesystem: AtomicU64::new(0),
            errors_network: AtomicU64::new(0),
        }
    }

    /// Increment software counter
    #[inline]
    pub fn increment(&self, counter_type: SoftwareCounterType, delta: u64) {
        match counter_type {
            SoftwareCounterType::SyscallTotal => self.syscall_total.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::SyscallRead => self.syscall_read.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::SyscallWrite => self.syscall_write.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::SyscallOpen => self.syscall_open.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::SyscallClose => self.syscall_close.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::SyscallStat => self.syscall_stat.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::SyscallFstat => self.syscall_fstat.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::SyscallPoll => self.syscall_poll.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::SyscallMmap => self.syscall_mmap.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::SyscallMunmap => self.syscall_munmap.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::SyscallIoctl => self.syscall_ioctl.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::SyscallSocket => self.syscall_socket.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::SyscallConnect => self.syscall_connect.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::SyscallAccept => self.syscall_accept.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::SyscallSend => self.syscall_send.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::SyscallRecv => self.syscall_recv.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::PageFaultMajor => self.page_fault_major.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::PageFaultMinor => self.page_fault_minor.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::PageFaultTotal => self.page_fault_total.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::ContextSwitchVoluntary => {
                self.context_switch_voluntary.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::ContextSwitchInvoluntary => {
                self.context_switch_involuntary.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::ContextSwitchTotal => {
                self.context_switch_total.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::InterruptTotal => self.interrupt_total.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::InterruptTimer => self.interrupt_timer.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::InterruptNetwork => {
                self.interrupt_network.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::InterruptDisk => self.interrupt_disk.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::InterruptKeyboard => {
                self.interrupt_keyboard.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::ScheduleCount => self.schedule_count.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::ScheduleLatencyTicks => {
                self.schedule_latency_ticks.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::RunqueueLength => self.runqueue_length.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::IdleTimeTicks => self.idle_time_ticks.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::RunTimeTicks => self.run_time_ticks.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::LockSpinAcquisitions => {
                self.lock_spin_acquisitions.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::LockSpinContentions => {
                self.lock_spin_contentions.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::LockMutexAcquisitions => {
                self.lock_mutex_acquisitions.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::LockMutexContentions => {
                self.lock_mutex_contentions.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::LockRwlockReadAcquisitions => {
                self.lock_rwlock_read_acquisitions.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::LockRwlockWriteAcquisitions => {
                self.lock_rwlock_write_acquisitions.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::LockRwlockContentions => {
                self.lock_rwlock_contentions.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::MemoryAllocationCount => {
                self.memory_allocation_count.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::MemoryDeallocationCount => {
                self.memory_deallocation_count.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::MemoryPageAllocations => {
                self.memory_page_allocations.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::MemoryPageDeallocations => {
                self.memory_page_deallocations.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::NetworkPacketsReceived => {
                self.network_packets_received.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::NetworkPacketsSent => {
                self.network_packets_sent.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::NetworkBytesReceived => {
                self.network_bytes_received.fetch_add(delta, Ordering::Relaxed)
            }
            SoftwareCounterType::NetworkBytesSent => self.network_bytes_sent.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::FileReads => self.file_reads.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::FileWrites => self.file_writes.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::FileOpens => self.file_opens.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::FileCloses => self.file_closes.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::FileSeeks => self.file_seeks.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::ProcessCreated => self.process_created.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::ProcessExited => self.process_exited.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::ThreadCreated => self.thread_created.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::ThreadExited => self.thread_exited.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::ErrorsSyscall => self.errors_syscall.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::ErrorsMemory => self.errors_memory.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::ErrorsFilesystem => self.errors_filesystem.fetch_add(delta, Ordering::Relaxed),
            SoftwareCounterType::ErrorsNetwork => self.errors_network.fetch_add(delta, Ordering::Relaxed),
        };
    }

    /// Get counter value
    #[inline]
    pub fn get(&self, counter_type: SoftwareCounterType) -> u64 {
        match counter_type {
            SoftwareCounterType::SyscallTotal => self.syscall_total.load(Ordering::Relaxed),
            SoftwareCounterType::SyscallRead => self.syscall_read.load(Ordering::Relaxed),
            SoftwareCounterType::SyscallWrite => self.syscall_write.load(Ordering::Relaxed),
            SoftwareCounterType::SyscallOpen => self.syscall_open.load(Ordering::Relaxed),
            SoftwareCounterType::SyscallClose => self.syscall_close.load(Ordering::Relaxed),
            SoftwareCounterType::SyscallStat => self.syscall_stat.load(Ordering::Relaxed),
            SoftwareCounterType::SyscallFstat => self.syscall_fstat.load(Ordering::Relaxed),
            SoftwareCounterType::SyscallPoll => self.syscall_poll.load(Ordering::Relaxed),
            SoftwareCounterType::SyscallMmap => self.syscall_mmap.load(Ordering::Relaxed),
            SoftwareCounterType::SyscallMunmap => self.syscall_munmap.load(Ordering::Relaxed),
            SoftwareCounterType::SyscallIoctl => self.syscall_ioctl.load(Ordering::Relaxed),
            SoftwareCounterType::SyscallSocket => self.syscall_socket.load(Ordering::Relaxed),
            SoftwareCounterType::SyscallConnect => self.syscall_connect.load(Ordering::Relaxed),
            SoftwareCounterType::SyscallAccept => self.syscall_accept.load(Ordering::Relaxed),
            SoftwareCounterType::SyscallSend => self.syscall_send.load(Ordering::Relaxed),
            SoftwareCounterType::SyscallRecv => self.syscall_recv.load(Ordering::Relaxed),
            SoftwareCounterType::PageFaultMajor => self.page_fault_major.load(Ordering::Relaxed),
            SoftwareCounterType::PageFaultMinor => self.page_fault_minor.load(Ordering::Relaxed),
            SoftwareCounterType::PageFaultTotal => self.page_fault_total.load(Ordering::Relaxed),
            SoftwareCounterType::ContextSwitchVoluntary => self.context_switch_voluntary.load(Ordering::Relaxed),
            SoftwareCounterType::ContextSwitchInvoluntary => {
                self.context_switch_involuntary.load(Ordering::Relaxed)
            }
            SoftwareCounterType::ContextSwitchTotal => self.context_switch_total.load(Ordering::Relaxed),
            SoftwareCounterType::InterruptTotal => self.interrupt_total.load(Ordering::Relaxed),
            SoftwareCounterType::InterruptTimer => self.interrupt_timer.load(Ordering::Relaxed),
            SoftwareCounterType::InterruptNetwork => self.interrupt_network.load(Ordering::Relaxed),
            SoftwareCounterType::InterruptDisk => self.interrupt_disk.load(Ordering::Relaxed),
            SoftwareCounterType::InterruptKeyboard => self.interrupt_keyboard.load(Ordering::Relaxed),
            SoftwareCounterType::ScheduleCount => self.schedule_count.load(Ordering::Relaxed),
            SoftwareCounterType::ScheduleLatencyTicks => self.schedule_latency_ticks.load(Ordering::Relaxed),
            SoftwareCounterType::RunqueueLength => self.runqueue_length.load(Ordering::Relaxed),
            SoftwareCounterType::IdleTimeTicks => self.idle_time_ticks.load(Ordering::Relaxed),
            SoftwareCounterType::RunTimeTicks => self.run_time_ticks.load(Ordering::Relaxed),
            SoftwareCounterType::LockSpinAcquisitions => self.lock_spin_acquisitions.load(Ordering::Relaxed),
            SoftwareCounterType::LockSpinContentions => self.lock_spin_contentions.load(Ordering::Relaxed),
            SoftwareCounterType::LockMutexAcquisitions => self.lock_mutex_acquisitions.load(Ordering::Relaxed),
            SoftwareCounterType::LockMutexContentions => self.lock_mutex_contentions.load(Ordering::Relaxed),
            SoftwareCounterType::LockRwlockReadAcquisitions => {
                self.lock_rwlock_read_acquisitions.load(Ordering::Relaxed)
            }
            SoftwareCounterType::LockRwlockWriteAcquisitions => {
                self.lock_rwlock_write_acquisitions.load(Ordering::Relaxed)
            }
            SoftwareCounterType::LockRwlockContentions => self.lock_rwlock_contentions.load(Ordering::Relaxed),
            SoftwareCounterType::MemoryAllocationCount => self.memory_allocation_count.load(Ordering::Relaxed),
            SoftwareCounterType::MemoryDeallocationCount => {
                self.memory_deallocation_count.load(Ordering::Relaxed)
            }
            SoftwareCounterType::MemoryPageAllocations => self.memory_page_allocations.load(Ordering::Relaxed),
            SoftwareCounterType::MemoryPageDeallocations => {
                self.memory_page_deallocations.load(Ordering::Relaxed)
            }
            SoftwareCounterType::NetworkPacketsReceived => self.network_packets_received.load(Ordering::Relaxed),
            SoftwareCounterType::NetworkPacketsSent => self.network_packets_sent.load(Ordering::Relaxed),
            SoftwareCounterType::NetworkBytesReceived => self.network_bytes_received.load(Ordering::Relaxed),
            SoftwareCounterType::NetworkBytesSent => self.network_bytes_sent.load(Ordering::Relaxed),
            SoftwareCounterType::FileReads => self.file_reads.load(Ordering::Relaxed),
            SoftwareCounterType::FileWrites => self.file_writes.load(Ordering::Relaxed),
            SoftwareCounterType::FileOpens => self.file_opens.load(Ordering::Relaxed),
            SoftwareCounterType::FileCloses => self.file_closes.load(Ordering::Relaxed),
            SoftwareCounterType::FileSeeks => self.file_seeks.load(Ordering::Relaxed),
            SoftwareCounterType::ProcessCreated => self.process_created.load(Ordering::Relaxed),
            SoftwareCounterType::ProcessExited => self.process_exited.load(Ordering::Relaxed),
            SoftwareCounterType::ThreadCreated => self.thread_created.load(Ordering::Relaxed),
            SoftwareCounterType::ThreadExited => self.thread_exited.load(Ordering::Relaxed),
            SoftwareCounterType::ErrorsSyscall => self.errors_syscall.load(Ordering::Relaxed),
            SoftwareCounterType::ErrorsMemory => self.errors_memory.load(Ordering::Relaxed),
            SoftwareCounterType::ErrorsFilesystem => self.errors_filesystem.load(Ordering::Relaxed),
            SoftwareCounterType::ErrorsNetwork => self.errors_network.load(Ordering::Relaxed),
        }
    }

    /// Reset all counters
    pub fn reset(&self) {
        self.syscall_total.store(0, Ordering::Relaxed);
        self.syscall_read.store(0, Ordering::Relaxed);
        self.syscall_write.store(0, Ordering::Relaxed);
        self.syscall_open.store(0, Ordering::Relaxed);
        self.syscall_close.store(0, Ordering::Relaxed);
        self.syscall_stat.store(0, Ordering::Relaxed);
        self.syscall_fstat.store(0, Ordering::Relaxed);
        self.syscall_poll.store(0, Ordering::Relaxed);
        self.syscall_mmap.store(0, Ordering::Relaxed);
        self.syscall_munmap.store(0, Ordering::Relaxed);
        self.syscall_ioctl.store(0, Ordering::Relaxed);
        self.syscall_socket.store(0, Ordering::Relaxed);
        self.syscall_connect.store(0, Ordering::Relaxed);
        self.syscall_accept.store(0, Ordering::Relaxed);
        self.syscall_send.store(0, Ordering::Relaxed);
        self.syscall_recv.store(0, Ordering::Relaxed);
        self.page_fault_major.store(0, Ordering::Relaxed);
        self.page_fault_minor.store(0, Ordering::Relaxed);
        self.page_fault_total.store(0, Ordering::Relaxed);
        self.context_switch_voluntary.store(0, Ordering::Relaxed);
        self.context_switch_involuntary.store(0, Ordering::Relaxed);
        self.context_switch_total.store(0, Ordering::Relaxed);
        self.interrupt_total.store(0, Ordering::Relaxed);
        self.interrupt_timer.store(0, Ordering::Relaxed);
        self.interrupt_network.store(0, Ordering::Relaxed);
        self.interrupt_disk.store(0, Ordering::Relaxed);
        self.interrupt_keyboard.store(0, Ordering::Relaxed);
        self.schedule_count.store(0, Ordering::Relaxed);
        self.schedule_latency_ticks.store(0, Ordering::Relaxed);
        self.runqueue_length.store(0, Ordering::Relaxed);
        self.idle_time_ticks.store(0, Ordering::Relaxed);
        self.run_time_ticks.store(0, Ordering::Relaxed);
        self.lock_spin_acquisitions.store(0, Ordering::Relaxed);
        self.lock_spin_contentions.store(0, Ordering::Relaxed);
        self.lock_mutex_acquisitions.store(0, Ordering::Relaxed);
        self.lock_mutex_contentions.store(0, Ordering::Relaxed);
        self.lock_rwlock_read_acquisitions.store(0, Ordering::Relaxed);
        self.lock_rwlock_write_acquisitions.store(0, Ordering::Relaxed);
        self.lock_rwlock_contentions.store(0, Ordering::Relaxed);
        self.memory_allocation_count.store(0, Ordering::Relaxed);
        self.memory_deallocation_count.store(0, Ordering::Relaxed);
        self.memory_page_allocations.store(0, Ordering::Relaxed);
        self.memory_page_deallocations.store(0, Ordering::Relaxed);
        self.network_packets_received.store(0, Ordering::Relaxed);
        self.network_packets_sent.store(0, Ordering::Relaxed);
        self.network_bytes_received.store(0, Ordering::Relaxed);
        self.network_bytes_sent.store(0, Ordering::Relaxed);
        self.file_reads.store(0, Ordering::Relaxed);
        self.file_writes.store(0, Ordering::Relaxed);
        self.file_opens.store(0, Ordering::Relaxed);
        self.file_closes.store(0, Ordering::Relaxed);
        self.file_seeks.store(0, Ordering::Relaxed);
        self.process_created.store(0, Ordering::Relaxed);
        self.process_exited.store(0, Ordering::Relaxed);
        self.thread_created.store(0, Ordering::Relaxed);
        self.thread_exited.store(0, Ordering::Relaxed);
        self.errors_syscall.store(0, Ordering::Relaxed);
        self.errors_memory.store(0, Ordering::Relaxed);
        self.errors_filesystem.store(0, Ordering::Relaxed);
        self.errors_network.store(0, Ordering::Relaxed);
    }

    /// Get all counters as a map
    pub fn as_map(&self) -> BTreeMap<String, u64> {
        let mut map = BTreeMap::new();
        for counter_type in SoftwareCounterType::all() {
            map.insert(counter_type.name().to_string(), self.get(counter_type));
        }
        map
    }
}

/// Software counter manager
pub struct SoftwareCounterManager {
    /// Per-CPU software counters
    per_cpu_counters: [PerCpuSoftwareCounters; NCPU],
}

impl SoftwareCounterManager {
    /// Create new software counter manager
    pub fn new() -> Self {
        Self {
            per_cpu_counters: [(); NCPU].map(|_| PerCpuSoftwareCounters::new()),
        }
    }

    /// Increment software counter
    #[inline]
    pub fn increment(&self, cpu_id: usize, counter_type: SoftwareCounterType, delta: u64) {
        if cpu_id < NCPU {
            self.per_cpu_counters[cpu_id].increment(counter_type, delta);
        }
    }

    /// Get counter value for specific CPU
    #[inline]
    pub fn get_cpu_counter(&self, cpu_id: usize, counter_type: SoftwareCounterType) -> u64 {
        if cpu_id < NCPU {
            self.per_cpu_counters[cpu_id].get(counter_type)
        } else {
            0
        }
    }

    /// Get aggregated counter value across all CPUs
    pub fn get_aggregated_counter(&self, counter_type: SoftwareCounterType) -> u64 {
        let mut total = 0u64;
        for cpu in 0..NCPU {
            total += self.per_cpu_counters[cpu].get(counter_type);
        }
        total
    }

    /// Get all counters for specific CPU
    pub fn get_cpu_counters(&self, cpu_id: usize) -> BTreeMap<String, u64> {
        if cpu_id < NCPU {
            self.per_cpu_counters[cpu_id].as_map()
        } else {
            BTreeMap::new()
        }
    }

    /// Get all counters aggregated across all CPUs
    pub fn get_all_counters(&self) -> BTreeMap<String, u64> {
        let mut aggregated = BTreeMap::new();
        for cpu in 0..NCPU {
            let cpu_counters = self.per_cpu_counters[cpu].as_map();
            for (key, value) in cpu_counters {
                *aggregated.entry(key).or_insert(0) += value;
            }
        }
        aggregated
    }

    /// Reset counters for specific CPU
    pub fn reset_cpu_counters(&self, cpu_id: usize) {
        if cpu_id < NCPU {
            self.per_cpu_counters[cpu_id].reset();
        }
    }

    /// Reset all counters
    pub fn reset_all_counters(&self) {
        for cpu in 0..NCPU {
            self.per_cpu_counters[cpu].reset();
        }
    }

    /// Calculate context switch rate
    pub fn calculate_context_switch_rate(&self) -> f64 {
        let total = self.get_aggregated_counter(SoftwareCounterType::ContextSwitchTotal);
        let run_time = self.get_aggregated_counter(SoftwareCounterType::RunTimeTicks);

        if run_time == 0 {
            0.0
        } else {
            (total as f64 / run_time as f64) * 1000.0
        }
    }

    /// Calculate lock contention rate
    pub fn calculate_lock_contention_rate(&self) -> f64 {
        let spin_contentions = self.get_aggregated_counter(SoftwareCounterType::LockSpinContentions);
        let mutex_contentions = self.get_aggregated_counter(SoftwareCounterType::LockMutexContentions);
        let rwlock_contentions = self.get_aggregated_counter(SoftwareCounterType::LockRwlockContentions);

        let spin_acquisitions = self.get_aggregated_counter(SoftwareCounterType::LockSpinAcquisitions);
        let mutex_acquisitions = self.get_aggregated_counter(SoftwareCounterType::LockMutexAcquisitions);
        let rwlock_read_acquisitions =
            self.get_aggregated_counter(SoftwareCounterType::LockRwlockReadAcquisitions);
        let rwlock_write_acquisitions =
            self.get_aggregated_counter(SoftwareCounterType::LockRwlockWriteAcquisitions);

        let total_contentions = spin_contentions + mutex_contentions + rwlock_contentions;
        let total_acquisitions = spin_acquisitions + mutex_acquisitions + rwlock_read_acquisitions + rwlock_write_acquisitions;

        if total_acquisitions == 0 {
            0.0
        } else {
            (total_contentions as f64 / total_acquisitions as f64) * 100.0
        }
    }
}

impl Default for SoftwareCounterManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Global software counter manager
static mut GLOBAL_SW_COUNTER_MANAGER: Option<SoftwareCounterManager> = None;
static SW_COUNTER_INIT: spin::Mutex<bool> = spin::Mutex::new(false);

/// Initialize software counter manager
pub fn init_software_counters() {
    let mut is_init = SW_COUNTER_INIT.lock();
    if *is_init {
        return;
    }

    unsafe {
        GLOBAL_SW_COUNTER_MANAGER = Some(SoftwareCounterManager::new());
    }

    *is_init = true;
    log::info!("Software performance counters initialized");
}

/// Get software counter manager
pub fn get_sw_counter_manager() -> &'static SoftwareCounterManager {
    unsafe {
        if GLOBAL_SW_COUNTER_MANAGER.is_none() {
            init_software_counters();
        }
        GLOBAL_SW_COUNTER_MANAGER.as_ref().unwrap()
    }
}

/// Convenience function to increment software counter
#[inline]
pub fn increment_sw_counter(counter_type: SoftwareCounterType, delta: u64) {
    let cpu_id = crate::cpu::cpuid();
    get_sw_counter_manager().increment(cpu_id, counter_type, delta);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_software_counter() {
        let manager = SoftwareCounterManager::new();
        manager.increment(0, SoftwareCounterType::SyscallTotal, 10);
        assert_eq!(manager.get_cpu_counter(0, SoftwareCounterType::SyscallTotal), 10);
    }

    #[test]
    fn test_aggregated_counter() {
        let manager = SoftwareCounterManager::new();
        manager.increment(0, SoftwareCounterType::SyscallTotal, 10);
        manager.increment(1, SoftwareCounterType::SyscallTotal, 5);
        assert_eq!(manager.get_aggregated_counter(SoftwareCounterType::SyscallTotal), 15);
    }

    #[test]
    fn test_counter_map() {
        let manager = SoftwareCounterManager::new();
        manager.increment(0, SoftwareCounterType::SyscallRead, 5);
        manager.increment(0, SoftwareCounterType::SyscallWrite, 3);

        let map = manager.get_cpu_counters(0);
        assert_eq!(map.get("syscall_read"), Some(&5));
        assert_eq!(map.get("syscall_write"), Some(&3));
    }
}
