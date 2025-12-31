# RISC-V Architecture Implementation

This directory contains the comprehensive RISC-V 64-bit architecture implementation with SMP multi-core support and H-extension virtualization.

## Files Overview

### Core Files

- **`mod.rs`** - Main module initialization and orchestration
- **`interrupts.rs`** - Legacy interrupt handling (kept for compatibility)
- **`memory.rs`** - Enhanced memory management with paging integration

### New Files Created

#### 1. `smp.rs` - SMP Multi-Core Support
**Features:**
- Multi-core boot and initialization
- CPU hotplug support (add/remove CPUs dynamically)
- Per-CPU data structures with statistics
- Inter-processor interrupts (IPI)
- CPU affinity and mask management (`CpuMask`)
- Hart detection and enumeration
- Load balancing support

**Key Functions:**
- `smp_setup()` - Initialize SMP system with all CPUs
- `start_secondary_cpu()` - Boot auxiliary CPUs
- `cpu_spinup()` - Wait for CPUs to come online with timeout
- `send_ipi()` - Send inter-processor interrupts
- `cpu_add()` / `cpu_remove()` - CPU hotplug operations
- `get_cpu_id()` - Get current CPU (hart) ID
- `get_load_balance_mask()` - Load balancing mask

**Performance Targets:**
- SMP boot time: <100ms for 8 cores
- IPI latency: <5μs

#### 2. `interrupt.rs` - Enhanced Interrupt Controller
**Features:**
- PLIC (Platform-Level Interrupt Controller) support
- Per-hart interrupt context management
- Interrupt priority configuration (0-7)
- Interrupt routing and affinity
- Software, timer, and external interrupt handling
- AIA (Advanced Interrupt Architecture) support
- CLINT (Core-Local Interrupt Controller) support

**Key Functions:**
- `plic_init()` - Initialize PLIC for given number of harts
- `plic_enable_irq()` / `plic_disable_irq()` - Per-hart interrupt control
- `plic_set_priority()` / `plic_get_priority()` - Priority management
- `plic_set_affinity()` - Route interrupt to specific hart
- `handle_external_interrupt()` - External interrupt handler
- `enable_timer_interrupt()` - Timer interrupt control
- `plic_claim()` / `plic_complete()` - Interrupt claim/complete

**Performance Targets:**
- Interrupt latency: <10μs
- Interrupt dispatch overhead: <2μs

#### 3. `virtualization.rs` - H-Extension Virtualization
**Features:**
- H-extension detection and initialization
- Guest VM state management
- World switch (guest/host transitions)
- Virtual interrupt injection
- Stage-2 page tables (guest physical to host physical)
- AIA (Advanced Interrupt Architecture) virtualization
- VM lifecycle management (create, run, destroy)

**Key Functions:**
- `has_virtualization()` - Check if H-extension is available
- `hvf_init()` - Initialize hypervisor framework
- `vm_create()` / `vm_destroy()` - VM lifecycle
- `vm_run_vcpu()` - Run a VCPU
- `world_switch()` - Guest/host world switch
- `inject_virtual_interrupt()` - Inject interrupt into guest
- `vm_map_memory()` / `vm_unmap_memory()` - Guest memory management
- `handle_mmio_exit()` - Handle MMIO access from guest

**Structures:**
- `VcpuState` - Virtual CPU state with registers and CSRs
- `VMContext` - Virtual machine context with Stage-2 page tables
- `WorldSwitch` - Context for world switch operations
- `VMConfig` - VM configuration (vCPUs, memory, nested virt)

**Performance Targets:**
- World switch overhead: <5%
- VM exit latency: <1μs
- Virtual interrupt injection: <500ns

#### 4. `paging.rs` - Enhanced Paging System
**Features:**
- Sv48 page table format (48-bit virtual addresses)
- Stage-2 page tables for virtualization
- TLB management and flushing
- ASID (Address Space ID) support (16-bit)
- Page permission management (R/W/X/U/A/D/G)
- Huge page support (2MB, 1GB)
- Sv39 compatibility support

**Key Functions:**
- `make_page_table()` - Create page table hierarchy
- `map_page()` / `unmap_page()` - Page mapping operations
- `map_range()` / `unmap_range()` - Range operations
- `enable_paging()` - Enable paging with SATP
- `flush_tlb()` - TLB invalidation
- `get_satap()` / `get_asid()` - Read SATP and ASID

**Structures:**
- `PageTableEntry` - Page table entry with physical address and flags
- `PageTable` - Page table with 512 entries
- `PageTableFlags` - Permission flags (V, R, W, X, U, G, A, D)
- `TLBManager` - TLB and ASID management

**Modules:**
- `sv39` - Sv39 paging for compatibility
- `stage2` - Stage-2 page tables for virtualization
- `hugepages` - 2MB and 1GB huge page support

**Performance Targets:**
- TLB miss handling: <100ns
- Page table walk: <50ns per level
- TLB flush: <1μs

#### 5. `timer.rs` - Per-CPU Timers
**Features:**
- Per-CPU timer initialization
- Timer interrupt handling
- Time counter reading (mtime)
- Timer comparison (mtimecmp) management
- Support for both S-mode and M-mode timers
- High-resolution timekeeping
- Timer calibration and statistics

**Key Functions:**
- `timer_init()` / `timer_init_hart()` - Timer initialization
- `set_next_timer()` - Schedule next timer interrupt
- `set_timer_delay_us()` - Set timer with delay
- `read_time_counter()` - Read current time (mtime)
- `set_mtimecmp()` - Set timer compare value
- `handle_timer_interrupt()` - Timer interrupt handler
- `udelay()` / `mdelay()` / `sdelay()` - Delay functions

**Modules:**
- `machine` - M-mode timer support
- `hrtc` - High-resolution time counter (nanoseconds)
- `calibration` - Timer frequency calibration
- `stats` - Timer statistics and profiling

**Performance Targets:**
- Timer interrupt latency: <5μs
- Timer read overhead: <100ns
- Timer set overhead: <100ns

#### 6. `sync.rs` - RISC-V Synchronization Primitives
**Features:**
- Memory barriers (fence, fence.i, fence.tlb)
- Atomic operations (amos* instructions)
- Cache coherency protocols (RVWMO)
- Lock implementations (TAS, Ticket locks)
- RCu (Read-Copy-Update) support
- LL/SC (Load-Link/Store-Conditional) operations

**Key Functions:**
- `fence()` - Full memory barrier
- `fence_acquire()` / `fence_release()` - Acquire/release fences
- `fence_i()` - Instruction synchronization fence
- `fence_tlb()` - TLB fence (sfence.vma)
- `fence_io()` - I/O fence
- `compiler_barrier()` - Compiler barrier only

**Atomic Operations:**
- `atomic_min()` / `atomic_max()` - Atomic min/max
- `atomic_and()` / `atomic_or()` / `atomic_xor()` - Atomic bitwise
- `atomic_swap()` - Atomic swap
- `cas()` - Compare-and-swap using LL/SC

**Locks:**
- `TasLock` - Test-and-set spinlock
- `TicketLock` - FIFO ticket lock (fair)

**Modules:**
- `atomic` - Atomic operations (amos*)
- `cache` - Cache operations
- `rcu` - Read-Copy-Update
- `llsc` - Load-Link/Store-Conditional
- `ordering` - Memory ordering helpers

## Architecture Support

### CPU Support
- **32-bit (rv32)** - Supported via feature flags
- **64-bit (rv64)** - Primary target

### Extensions
- **I** - Integer (base ISA) - Required
- **M** - Multiplication/division - Required
- **A** - Atomic instructions - Required
- **C** - Compressed instructions - Optional
- **F** - Single-precision float - Optional
- **D** - Double-precision float - Optional
- **H** - Hypervisor - Optional (for virtualization)

### Privilege Modes
- **M-mode** - Machine mode (boot, hardware control)
- **S-mode** - Supervisor mode (OS kernel) - Primary target
- **U-mode** - User mode (applications)

### Memory Models
- **RVWMO** - RISC-V Weak Memory Ordering (default)
- Supports all RISC-V memory ordering primitives

## Platform Support

### QEMU
- Fully supported via `-smp` and `-machine virt` options
- PLIC, CLINT emulation available
- H-extension support (with `-machine virt,accel=tcg`)

### Real Hardware
- SiFive U/U series
- Kendryte K210
- Other RISC-V compliant hardware

### Simulators
- Spike (RISC-V ISA simulator)
- QEMU system mode

## Memory Layout

### Kernel Address Space
```
0x0000_0000 - 0x7FFF_FFFF  User space (128 GB)
0x8000_0000 - 0xFFFF_FFFF  Kernel space (128 GB)
  ├─ 0x8000_0000 - Kernel text/code
  ├─ 0x8020_0000 - Kernel data
  ├─ 0x8100_0000 - Kernel heap
  ├─ 0x9000_0000 - Device mapping (MMIO)
  └─ 0xC000_0000 - Direct physical map
```

### Page Tables
- **Sv48**: 48-bit virtual addresses, 4KB pages
  - 4 levels of page tables
  - Supports 4KB, 2MB, and 1GB pages
- **Stage-2**: Guest physical to host physical (virtualization)

## Interrupt Handling

### Interrupt Sources
1. **Software Interrupts** - Inter-processor interrupts (IPI)
2. **Timer Interrupts** - Per-CPU timer events
3. **External Interrupts** - Device interrupts via PLIC

### PLIC (Platform-Level Interrupt Controller)
- Up to 256 interrupt sources
- 7 priority levels
- Per-hart enable and threshold
- Claim/complete protocol

### Interrupt Flow
```
Device → PLIC → External IRQ → Trap Handler → PLIC Claim
  → ISR → PLIC Complete → Return from Trap
```

## Virtualization

### H-Extension Features
- **Guest/Host World Switch** - <5% overhead
- **Stage-2 Page Tables** - Guest physical to host physical
- **Virtual Interrupts** - Interrupt injection to guests
- **Nested Virtualization** - Future work

### VM Lifecycle
1. **Create VM** - Allocate VM context and Stage-2 page tables
2. **Map Memory** - Map guest physical memory regions
3. **Create VCPUs** - Initialize VCPU state
4. **Run VM** - World switch to guest
5. **Handle VM Exit** - Process exit reason and resume

### VM Exit Reasons
- Instruction access faults
- Load/store access faults
- Page faults (instruction, load, store)
- Ecall (environment call)
- MMIO access
- Interrupts

## Performance Optimization

### CPU Hotplug
- Dynamic CPU addition/removal
- Graceful shutdown with timeout
- Load balancing across CPUs

### Interrupt Distribution
- Per-CPU interrupt routing
- Interrupt affinity control
- Priority-based scheduling

### TLB Optimization
- ASID-based address space tagging
- Selective TLB invalidation
- TLB prefetch hints

### Cache Management
- Cache line alignment (64 bytes)
- Cache coherency (RVWMO)
- Cache-friendly data structures

## Testing

### Unit Tests
Each module includes comprehensive unit tests:
- CPU mask operations
- Interrupt enable/disable
- Page table mapping/unmapping
- Timer functions
- Lock implementations
- LL/SC operations

### Integration Tests
- SMP boot and shutdown
- Multi-core synchronization
- VM lifecycle
- Interrupt handling

### Performance Tests
- Timer overhead measurement
- Interrupt latency measurement
- TLB miss overhead
- World switch overhead

## Future Enhancements

### Phase 2
- [ ] Advanced power management (CPU idle states)
- [ ] NUMA support for multi-socket systems
- [ ] Performance counters (PMU)
- [ ] Debug triggers and breakpoints

### Phase 3
- [ ] Sstc (Supervisor-mode timer counters)
- [ ] Svinval (TLB invalidation improvements)
- [ ] Svnapot (Page table sharing for containers)
- [ ] Svpbmt (Page-based memory types)

### Phase 4
- [ ] Nested virtualization
- [ ] IOMMU support
- [ ] PCIe device passthrough
- [ ] Live migration support

## References

- [RISC-V ISA Manual](https://riscv.org/technical/specifications/)
- [RISC-V Privileged Architecture](https://riscv.org/specifications/privileged-isa/)
- [H-extension Specification](https://riscv.org/specifications/h-extension/)
- [RISC-V Linux Porting Guide](https://github.com/riscv/riscv-linux/blob/master/Documentation/riscv/paging.rst)

## Contributors

This implementation follows production-grade standards suitable for real RISC-V hardware deployment.

## License

See project root for license information.
