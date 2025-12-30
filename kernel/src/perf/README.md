//! Performance Counter System
//!
//! This module implements a comprehensive performance counter system with 50+ hardware
//! and software counters, providing detailed system performance metrics with minimal
//! overhead (<1% performance impact).
//!
//! # Architecture
//!
//! The performance counter system is organized into three main components:
//!
//! 1. **Hardware Counters** (`hardware.rs`)
//!    - CPU performance counters (cycles, instructions)
//!    - Cache performance counters (L1/L2/L3 hits/misses)
//!    - Branch prediction counters
//!    - TLB statistics
//!    - Memory access counters
//!    - Pipeline stall counters
//!
//! 2. **Software Counters** (`software.rs`)
//!    - System call counts (per-syscall type)
//!    - Context switch statistics
//!    - Interrupt counts (per-IRQ type)
//!    - Scheduler statistics
//!    - Lock contention metrics
//!    - Memory allocation tracking
//!    - Network I/O counters
//!    - Filesystem operation counters
//!    - Process/thread lifecycle events
//!    - Error counters
//!
//! 3. **Counter Manager** (`counter_manager.rs`)
//!    - Unified counter registration interface
//!    - Lock-free per-CPU counter arrays
//!    - Data collection and aggregation
//!    - Export to /proc or /sys/fs
//!    - Snapshot functionality
//!    - Custom counter support
//!
//! # Counter Breakdown
//!
//! ## Hardware Counters (22 total)
//!
//! ### CPU Counters
//! - `instructions_retired`: Number of instructions executed
//! - `cpu_cycles_unhalted`: CPU cycles while not halted
//! - `reference_cycles`: Reference clock cycles
//!
//! ### L1 Cache Counters
//! - `l1_cache_references`: Total L1 cache accesses
//! - `l1_cache_misses`: L1 cache misses
//! - `l1_instruction_cache_misses`: L1 instruction cache misses
//! - `l1_data_cache_misses`: L1 data cache misses
//!
//! ### L2 Cache Counters
//! - `l2_cache_references`: Total L2 cache accesses
//! - `l2_cache_misses`: L2 cache misses
//!
//! ### L3 Cache Counters
//! - `l3_cache_references`: Total L3 cache accesses
//! - `l3_cache_misses`: L3 cache misses
//!
//! ### Branch Prediction Counters
//! - `branch_instructions`: Branch instructions retired
//! - `branch_misses`: Branch mispredictions
//!
//! ### TLB Counters
//! - `instruction_tlb_hits`: Instruction TLB hits
//! - `instruction_tlb_misses`: Instruction TLB misses
//! - `data_tlb_hits`: Data TLB hits
//! - `data_tlb_misses`: Data TLB misses
//!
//! ### Memory Counters
//! - `memory_accesses`: Memory access operations
//! - `memory_cycles`: Cycles waiting for memory
//!
//! ### Pipeline Counters
//! - `stalled_cycles`: Total stalled cycles
//! - `stalled_cycles_frontend`: Frontend stalled cycles
//! - `stalled_cycles_backend`: Backend stalled cycles
//!
//! ## Software Counters (60 total)
//!
//! ### System Call Counters (16)
//! - `syscall_total`: Total system calls
//! - `syscall_read`: Read system calls
//! - `syscall_write`: Write system calls
//! - `syscall_open`: Open system calls
//! - `syscall_close`: Close system calls
//! - `syscall_stat`: Stat system calls
//! - `syscall_fstat`: Fstat system calls
//! - `syscall_poll`: Poll system calls
//! - `syscall_mmap`: Mmap system calls
//! - `syscall_munmap`: Munmap system calls
//! - `syscall_ioctl`: Ioctl system calls
//! - `syscall_socket`: Socket system calls
//! - `syscall_connect`: Connect system calls
//! - `syscall_accept`: Accept system calls
//! - `syscall_send`: Send system calls
//! - `syscall_recv`: Receive system calls
//!
//! ### Page Fault Counters (3)
//! - `page_fault_major`: Major page faults (disk I/O)
//! - `page_fault_minor`: Minor page faults (no disk I/O)
//! - `page_fault_total`: Total page faults
//!
//! ### Context Switch Counters (3)
//! - `context_switch_voluntary`: Voluntary context switches
//! - `context_switch_involuntary`: Involuntary context switches
//! - `context_switch_total`: Total context switches
//!
//! ### Interrupt Counters (5)
//! - `interrupt_total`: Total interrupts
//! - `interrupt_timer`: Timer interrupts
//! - `interrupt_network`: Network interrupts
//! - `interrupt_disk`: Disk interrupts
//! - `interrupt_keyboard`: Keyboard interrupts
//!
//! ### Scheduler Counters (5)
//! - `schedule_count`: Schedule operations
//! - `schedule_latency_ticks`: Total schedule latency
//! - `runqueue_length`: Current runqueue length
//! - `idle_time_ticks`: Total idle time
//! - `run_time_ticks`: Total run time
//!
//! ### Lock Counters (7)
//! - `lock_spin_acquisitions`: Spinlock acquisitions
//! - `lock_spin_contentions`: Spinlock contentions
//! - `lock_mutex_acquisitions`: Mutex acquisitions
//! - `lock_mutex_contentions`: Mutex contentions
//! - `lock_rwlock_read_acquisitions`: RWLock read acquisitions
//! - `lock_rwlock_write_acquisitions`: RWLock write acquisitions
//! - `lock_rwlock_contentions`: RWLock contentions
//!
//! ### Memory Counters (4)
//! - `memory_allocation_count`: Memory allocations
//! - `memory_deallocation_count`: Memory deallocations
//! - `memory_page_allocations`: Page allocations
//! - `memory_page_deallocations`: Page deallocations
//!
//! ### Network Counters (4)
//! - `network_packets_received`: Packets received
//! - `network_packets_sent`: Packets sent
//! - `network_bytes_received`: Bytes received
//! - `network_bytes_sent`: Bytes sent
//!
//! ### Filesystem Counters (5)
//! - `file_reads`: File read operations
//! - `file_writes`: File write operations
//! - `file_opens`: File open operations
//! - `file_closes`: File close operations
//! - `file_seeks`: File seek operations
//!
//! ### Process Counters (4)
//! - `process_created`: Processes created
//! - `process_exited`: Processes exited
//! - `thread_created`: Threads created
//! - `thread_exited`: Threads exited
//!
//! ### Error Counters (4)
//! - `errors_syscall`: System call errors
//! - `errors_memory`: Memory errors
//! - `errors_filesystem`: Filesystem errors
//! - `errors_network`: Network errors
//!
//! # Usage Examples
//!
//! ## Basic Usage
//!
//! ```rust
//! use kernel::perf::{init_all, get_counter_manager};
//!
//! // Initialize all performance counters
//! init_all();
//!
//! // Get counter manager
//! let manager = get_counter_manager();
//!
//! // Create a snapshot of all counters
//! let snapshot = manager.snapshot();
//!
//! // Export to /proc format
//! let proc_output = manager.export_to_proc();
//! println!("{}", proc_output);
//!
//! // Export to JSON format
//! let json_output = manager.export_to_json();
//! println!("{}", json_output);
//!
//! // Get summary
//! let summary = manager.get_summary();
//! println!("{:?}", summary);
//! ```
//!
//! ## Incrementing Counters
//!
//! ```rust
//! use kernel::perf::{
//!     increment_hw_counter,
//!     increment_sw_counter,
//!     HardwareCounterType,
//!     SoftwareCounterType,
//! };
//!
//! // Increment hardware counter (auto-detects CPU ID)
//! increment_hw_counter(HardwareCounterType::InstructionsRetired, 100);
//!
//! // Increment software counter
//! increment_sw_counter(SoftwareCounterType::SyscallTotal, 1);
//! ```
//!
//! ## Custom Counters
//!
//! ```rust
//! use kernel::perf::{get_counter_manager, SimpleCounter, CounterCategory};
//! use alloc::sync::Arc;
//!
//! let manager = get_counter_manager();
//!
//! // Create custom counter
//! let counter = Arc::new(SimpleCounter::new_custom(
//!     "my_metric".to_string(),
//!     "My custom metric".to_string(),
//! ));
//!
//! // Register counter
//! let counter_id = manager.register_counter(counter).unwrap();
//!
//! // Increment counter
//! manager.increment_custom("my_metric", 42);
//!
//! // Read value
//! let value = manager.get_custom_counter("my_metric").unwrap();
//! assert_eq!(value, 42);
//! ```
//!
//! ## Per-CPU Statistics
//!
//! ```rust
//! use kernel::perf::get_counter_manager;
//!
//! let manager = get_counter_manager();
//!
//! // Get counters for specific CPU
//! let cpu_id = 0;
//! let hw_counters = manager.hardware_manager().get_cpu_counters(cpu_id);
//! let sw_counters = manager.software_manager().get_cpu_counters(cpu_id);
//!
//! println!("CPU {} Hardware Counters: {:?}", cpu_id, hw_counters);
//! println!("CPU {} Software Counters: {:?}", cpu_id, sw_counters);
//! ```
//!
//! ## Performance Metrics
//!
//! ```rust
//! use kernel::perf::get_hw_counter_manager;
//!
//! let hw_manager = get_hw_counter_manager();
//!
//! // Calculate IPC (Instructions Per Cycle)
//! let cpu_id = 0;
//! let ipc = hw_manager.calculate_ipc(cpu_id);
//! println!("IPC: {:.2}", ipc);
//!
//! // Calculate cache miss rates
//! let l1_miss_rate = hw_manager.calculate_cache_miss_rate(cpu_id, 1);
//! let l2_miss_rate = hw_manager.calculate_cache_miss_rate(cpu_id, 2);
//! let l3_miss_rate = hw_manager.calculate_cache_miss_rate(cpu_id, 3);
//!
//! println!("L1 miss rate: {:.2}%", l1_miss_rate);
//! println!("L2 miss rate: {:.2}%", l2_miss_rate);
//! println!("L3 miss rate: {:.2}%", l3_miss_rate);
//!
//! // Calculate branch miss rate
//! let branch_miss_rate = hw_manager.calculate_branch_miss_rate(cpu_id);
//! println!("Branch miss rate: {:.2}%", branch_miss_rate);
//!
//! // Calculate TLB miss rate
//! let tlb_miss_rate = hw_manager.calculate_tlb_miss_rate(cpu_id);
//! println!("TLB miss rate: {:.2}%", tlb_miss_rate);
//! ```
//!
//! # Performance Considerations
//!
//! ## Lock-Free Access
//!
//! All counters use lock-free atomic operations (`Ordering::Relaxed`) for minimal
//! overhead. Per-CPU counter arrays eliminate cross-CPU cache line contention.
//!
//! ## Performance Impact
//!
//! - **Target overhead**: <1% performance impact
//! - **Sampling approach**: Counters can be enabled/disabled globally
//! - **Per-CPU aggregation**: Minimizes cache coherency traffic
//! - **Atomic operations**: Uses Relaxed ordering for best performance
//!
//! ## Memory Layout
//!
//! ```rust
//! #[repr(C)]
//! pub struct PerCpuHardwareCounters {
//!     // Each counter is aligned to avoid false sharing
//!     pub instructions_retired: AtomicU64,
//!     pub cpu_cycles_unhalted: AtomicU64,
//!     // ... more counters
//! }
//!
//! // Per-CPU array ensures each CPU has its own cache line
//! static mut COUNTERS: [PerCpuHardwareCounters; NCPU];
//! ```
//!
//! # Export Formats
//!
//! ## /proc Format
//!
//! The `/proc` format provides human-readable counter values:
//!
//! ```text
//! # Performance Counters
//! # Timestamp: 1234567890
//!
//! ## Hardware Counters (Aggregated)
//! hw_instructions_retired 12345678
//! hw_cpu_cycles_unhalted 12345678
//! ...
//!
//! ## Software Counters (Aggregated)
//! sw_syscall_total 12345
//! sw_context_switch_total 678
//! ...
//!
//! ## Per-CPU Breakdown
//! ### CPU 0
//! Hardware:
//!   hw_instructions_retired 1234567
//!   ...
//! Software:
//!   sw_syscall_total 1234
//!   ...
//! ```
//!
//! ## JSON Format
//!
//! The JSON format provides structured data for programmatic consumption:
//!
//! ```json
//! {
//!   "timestamp": 1234567890,
//!   "aggregated_hardware": {
//!     "instructions_retired": 12345678,
//!     "cpu_cycles_unhalted": 12345678
//!   },
//!   "aggregated_software": {
//!     "syscall_total": 12345,
//!     "context_switch_total": 678
//!   },
//!   "per_cpu": [
//!     {
//!       "cpu": 0,
//!       "hardware": {...},
//!       "software": {...}
//!     }
//!   ]
//! }
//! ```
//!
//! # Testing
//!
//! The module includes comprehensive tests covering:
//! - Counter creation and initialization
//! - Counter increment operations
//! - Per-CPU counter access
//! - Aggregated counter calculations
//! - Custom counter registration
//! - Snapshot creation
//! - Export functionality
//!
//! Run tests with:
//! ```bash
//! cargo test --package kernel --lib perf
//! ```
//!
//! # Future Enhancements
//!
//! Potential improvements for the performance counter system:
//!
//! 1. **Hardware PMU Integration**
//!    - Direct integration with x86_64 Perf Counters
//!    - ARM PMU support
//!    - RISC-V performance counters
//!
//! 2. **Advanced Metrics**
//!    - Rate-based counters (events/second)
//!    - Histogram support for latency distributions
//!    - Percentile calculations
//!
//! 3. **Dynamic Counter Registration**
//!    - Runtime counter addition/removal
//!    - Counter grouping and tagging
//!    - Conditional counter enabling
//!
//! 4. **Export Interfaces**
//!    - Actual /proc/perf_counters filesystem interface
//!    - sysfs integration
//!    - Real-time streaming support
//!
//! 5. **Integration**
//!    - Integration with tracing system
//!    - Performance alerting
//!    - Historical data storage
//!
//! # References
//!
//! - Intel Architecture Instruction Set Extensions Programming Reference
//! - ARM Performance Monitoring Unit Architecture
//! - Linux perf_events subsystem
//! - /proc/sys documentation
