# NOS Kernel

A modern, microkernel-based operating system written in Rust.

## Overview

NOS (New Operating System) is a research-grade microkernel designed for:
- High performance and low latency
- Strong memory safety guarantees via Rust
- Support for multiple architectures (x86_64, AArch64, RISC-V)
- Clean modular architecture
- Comprehensive security features

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│              Userspace Applications               │
└────────────────────┬────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│              System Call Interface             │
└────────────────────┬────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│               Microkernel Core                 │
│  - Process Manager                               │
│  - Thread Scheduler                             │
│  - Memory Manager                               │
│  - IPC (Inter-Process Communication)           │
└────────────────────┬────────────────────────────────┘
                     │
        ┌────────────┴────────────┐
        ▼                         ▼
┌──────────────┐      ┌──────────────┐
│   Drivers    │      │   Services   │
│  - Network   │      │  - VFS      │
│  - Storage   │      │  - IPC      │
│  - Devices   │      │  - Security  │
└──────────────┘      └──────────────┘
```

## Directory Structure

| Directory | Description |
|-----------|-------------|
| `arch/` | Architecture-specific code (x86_64, AArch64, RISC-V) |
| `bootloader/` | Bootloader and early initialization |
| `core/` | Core kernel initialization and startup |
| `subsystems/` | Major subsystems (process, memory, net, sync, syscalls) |
| `vfs/` | Virtual File System layer |
| `security/` | Security modules (permissions, ASLR, audit) |
| `platform/` | Platform-specific code |
| `libc/` | Libc interface for userspace |
| `posix/` | POSIX types and constants |

## Key Subsystems

### Process Management

Located in `subsystems/process/`:
- Process creation and termination
- Thread management
- Scheduling (CFS, priority-based)
- Process capabilities
- Signal handling

### Memory Management

Located in `subsystems/memory/` and `subsystems/mm/`:
- Physical memory management
- Virtual memory management
- Page allocation
- Memory regions
- KPTI (Kernel Page Table Isolation)

### Networking

Located in `subsystems/net/`:
- TCP/UDP/IP stack
- Socket implementation
- Device interfaces
- Packet filtering

### File System

Located in `vfs/` and `subsystems/fs/`:
- VFS abstraction layer
- Multiple filesystem support (Ext4, RamFS, TmpFS, SysFS, ProcFS)
- Dentry cache
- File permissions

### Security

Located in `security/`:
- Enhanced permissions system
- ASLR (Address Space Layout Randomization)
- Stack canaries
- Security auditing
- Memory security contexts

### Synchronization

Located in `subsystems/sync/`:
- Spinlocks
- Mutexes
- RWLocks
- Condition variables
- Per-CPU queues
- Lock-free data structures

## Supported Architectures

### x86_64

- **Bootloader**: Multiboot2 support
- **Memory**: 4-level page tables, PAE support
- **Interrupts**: IDT, APIC
- **Time**: RDTSC, TSC, HPET

### AArch64

- **Bootloader**: UEFI support
- **Memory**: 4-level page tables
- **Interrupts**: GICv2/GICv3
- **Time**: Generic timer

### RISC-V

- **Bootloader**: OpenSBI support
- **Memory**: Sv39 page tables
- **Interrupts**: PLIC
- **Time**: MTIME/MTIMECMP

## Building

```bash
# Build the kernel
cargo build --manifest-path=kernel/Cargo.toml

# Build with optimizations
cargo build --release --manifest-path=kernel/Cargo.toml

# Run tests
cargo test --manifest-path=kernel/Cargo.toml
```

## Documentation

- [VFS Module](./src/vfs/README.md)
- [File System Module](./src/subsystems/fs/README.md)
- [Driver Model](./src/subsystems/drivers/README.md)
- [Interrupt Subsystem](./src/subsystems/microkernel/README_INTERRUPT.md)

## Security Features

- **Memory Safety**: Rust ownership and borrowing
- **ASLR**: Randomized memory layout for code, data, heap, stack
- **KPTI**: Kernel page table isolation for Meltdown mitigation
- **Retpoline**: Spectre v2 mitigation
- **Stack Canaries**: Buffer overflow detection
- **Capabilities**: Fine-grained privilege control
- **Audit**: Security audit and compliance checking

## Performance Features

- **Lock-free data structures**: Per-CPU queues, work-stealing deques
- **Lockless scheduling**: O(1) task switching
- **Efficient memory**: Per-CPU allocators, slab allocators
- **Cache-friendly**: Optimized data structures and algorithms
- **Network optimizations**: Port bitmap allocator, hash tables

## System Calls

POSIX-compatible system calls:
- Process: `fork`, `execve`, `exit`, `waitpid`, `kill`
- File: `open`, `read`, `write`, `close`, `stat`, `lstat`
- Directory: `mkdir`, `rmdir`, `chdir`, `getcwd`
- Memory: `mmap`, `munmap`, `brk`, `mprotect`
- Socket: `socket`, `bind`, `listen`, `accept`, `connect`, `send`, `recv`
- IPC: `pipe`, `shmget`, `shmat`, `semop`, `msgsnd`

## Testing

Run tests:

```bash
# Unit tests
cargo test

# Integration tests
cargo test --test-threads=1

# Benchmarks
cargo bench
```

## Contributing

1. Follow Rust best practices
2. Use `#![no_std]` where applicable
3. Document public APIs with `///` comments
4. Write tests for new functionality
5. Run `cargo fmt` before committing
6. Run `cargo clippy` to check for warnings

## License

MIT License - see LICENSE file for details

## Roadmap

- [ ] Complete FUSE support
- [ ] Add NFS client
- [ ] Implement Btrfs filesystem
- [ ] Add NUMA-aware scheduling
- [ ] Implement live kernel patching
- [ ] Add eBPF support
- [ ] Implement container support
- [ ] Add hypervisor capabilities
