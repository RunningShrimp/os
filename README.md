# NOS - Modern Operating System Kernel

> A high-performance, cross-platform operating system kernel written in Rust

[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![Rust Version](https://img.shields.io/badge/rust-nightly-orange)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()

---

## 🎯 Project Status

NOS is currently undergoing a **comprehensive improvement initiative** to transform from a mid-level project to an excellent, production-ready operating system kernel.

### Current Metrics (as of 2025-12-09)
- **Maintainability**: 6.2/10 → Target: **8.5/10**
- **Performance**: Baseline → Target: **+100-300%**
- **Feature Completeness**: ~50% → Target: **80%+**
- **Technical Debt**: 261 TODOs → Target: **<50**

📋 **[View Full Improvement Roadmap](./NOS_IMPROVEMENT_ROADMAP.md)**

---

## 🚀 Quick Start

### For New Contributors
```bash
# Clone the repository
git clone https://github.com/RunningShrimp/os.git nos
cd nos

# Read the quick start guide
cat QUICK_START_GUIDE.md

# Build the kernel
cargo build --release

# Run tests
cargo test
```

📖 **[New Contributor Guide](./QUICK_START_GUIDE.md)**

### For Core Developers
- **[6-Month Roadmap](./NOS_IMPROVEMENT_ROADMAP.md)** - Complete improvement plan
- **[TODO Cleanup Plan](./docs/TODO_CLEANUP_PLAN.md)** - 261 tracked items
- **[Week 1 Guide](./docs/plans/WEEK1_DETAILED_GUIDE.md)** - Detailed first week tasks

---

## 📋 Features

### Currently Implemented
- ✅ Process management (basic operations)
- ✅ Memory management (page allocation, virtual memory)
- ✅ File system (VFS, basic operations)
- ✅ Multi-architecture support (x86_64, ARM64, RISC-V)
- ✅ System call interface
- ✅ Interrupt handling
- ✅ Basic device drivers

### In Development (First 2 Months)
- 🔄 TODO cleanup (261 → 180 items)
- 🔄 Syscalls module decoupling
- 🔄 Unified testing framework
- 🔄 Core process operations (fork, execve)
- 🔄 File system operations (complete POSIX interface)

### Planned (3-6 Months)
- 📅 O(1) process scheduler
- 📅 Per-CPU memory allocator
- 📅 VFS zero-copy optimization
- 📅 Network stack completion
- 📅 Advanced POSIX features
- 📅 Performance monitoring system

---

## 🏗️ Architecture

NOS employs a **hybrid kernel architecture** combining the benefits of both microkernel and monolithic designs:

```
┌─────────────────────────────────────┐
│    User Space Applications          │
├─────────────────────────────────────┤
│    System Call Interface            │
├─────────────────────────────────────┤
│  Kernel Core (Microkernel-like)     │
│  - Process Management               │
│  - Memory Management                │
│  - IPC                              │
├─────────────────────────────────────┤
│  Kernel Services (Hybrid)           │
│  - VFS (kernel space)               │
│  - Network Stack (user space)       │
│  - Device Drivers (mixed)           │
├─────────────────────────────────────┤
│  Hardware Abstraction Layer         │
├─────────────────────────────────────┤
│          Hardware                    │
└─────────────────────────────────────┘
```

📚 **[Architecture Documentation](./docs/PHASE4_LAYERED_ARCHITECTURE.md)**

---

## 📂 Project Structure

```
nos/
├── kernel/              # Kernel source code
│   ├── src/
│   │   ├── syscalls/    # System call implementations
│   │   ├── process/     # Process management
│   │   ├── mm/          # Memory management
│   │   ├── fs/          # File system
│   │   └── arch/        # Architecture-specific code
│   └── tests/           # Kernel tests
├── bootloader/          # Boot loader
├── user/                # User space programs
├── docs/                # Documentation
│   ├── plans/           # Planning documents
│   ├── reports/         # Assessment reports
│   └── README.md        # Documentation index
├── scripts/             # Build and utility scripts
└── targets/             # Target configurations
```

---

## 🔧 Development

### Prerequisites
- Rust nightly toolchain
- QEMU (for testing)
- Cross-compilation tools (for target architectures)

### Building
```bash
# Build for x86_64
cargo build --release

# Build for ARM64
cargo build --release --target aarch64-unknown-none

# Build for RISC-V
cargo build --release --target riscv64gc-unknown-none-elf
```

### Testing
```bash
# Run unit tests
cargo test --lib

# Run integration tests
cargo test --test integration

# Run with coverage
cargo tarpaulin
```

### Code Quality
```bash
# Format code
cargo fmt

# Lint code
cargo clippy -- -D warnings

# Check for common issues
cargo check
```

---

## 📊 Current Focus Areas

### Phase 1: Emergency Cleanup (Months 1-2)
- **Week 1**: Root directory cleanup + core process/file operations
- **Week 2-4**: TODO cleanup (process, file system, memory)
- **Week 3-6**: Syscalls module decoupling
- **Week 5-6**: Unified testing framework

**Progress**: Week 1 planning ✅ | Implementation starts now

### Key Metrics to Track
- TODO count: 261 → 180 (Week 4 target)
- Test coverage: 45% → 65% (Week 6 target)
- Module coupling: High → 60% reduction (Week 6 target)

---

## 📖 Documentation

- **[Quick Start Guide](./QUICK_START_GUIDE.md)** - Get started in 5 minutes
- **[Improvement Roadmap](./NOS_IMPROVEMENT_ROADMAP.md)** - 6-month plan
- **[TODO Cleanup Plan](./docs/TODO_CLEANUP_PLAN.md)** - Detailed TODO tracking
- **[Week 1 Guide](./docs/plans/WEEK1_DETAILED_GUIDE.md)** - First week tasks
- **[Documentation Index](./docs/README.md)** - All documentation
 - **[Architecture Overview](./docs/ARCHITECTURE_OVERVIEW.md)** - Layered architecture summary
 - **[Dependency Rules](./docs/DEPENDENCY_RULES.md)** - Module boundaries & CI rules
 - **[Syscalls Overview](./docs/SYSCALLS_OVERVIEW.md)** - Ranges, dispatch & feature gates
 - **[ProcFS Guide](./docs/PROCFS.md)** - Runtime observability nodes
 - **[Kernel Features](./docs/FEATURES.md)** - Feature flags and usage
 - **[6–12M Roadmap](./docs/ROADMAP_6_12M.md)** - Milestones
 - **[Implementation Checklist](./docs/IMPLEMENTATION_CHECKLIST.md)** - Current status & TODOs

---

## 🤝 Contributing

We welcome contributions! Here's how to get started:

1. **Read the docs**: Start with [QUICK_START_GUIDE.md](./QUICK_START_GUIDE.md)
2. **Choose a task**: Check [TODO_CLEANUP_PLAN.md](./docs/TODO_CLEANUP_PLAN.md)
3. **Follow standards**: See [MODULAR_DEVELOPMENT_STANDARDS.md](./docs/MODULAR_DEVELOPMENT_STANDARDS.md)
4. **Submit PR**: Follow our code review process

### Weekly Reports
We maintain weekly progress reports using this [template](./docs/templates/WEEKLY_REPORT_TEMPLATE.md).

---

## 🎯 Goals and Roadmap

### Short-term Goals (2 Months)
- ✅ Clean up technical debt (TODO reduction)
- ✅ Decouple syscalls module
- ✅ Implement core POSIX operations
- ✅ Unify testing framework

### Mid-term Goals (4 Months)
- 🎯 O(1) scheduler implementation
- 🎯 Per-CPU memory allocator
- 🎯 Complete POSIX interface (85%+)
- 🎯 Performance optimization (+100%)

### Long-term Goals (6 Months)
- 🎯 Architecture refactoring
- 🎯 Performance monitoring system
- 🎯 5+ platform support
- 🎯 Production-ready stability (99.9% uptime)

---

## 📜 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

---

## 👥 Team

- **Project Lead**: [Your Name]
- **Core Developers**: [Names]
- **Contributors**: [See CONTRIBUTORS.md]

---

## 📞 Contact

- **Issues**: [GitHub Issues](https://github.com/RunningShrimp/os/issues)
- **Discussions**: [GitHub Discussions](https://github.com/RunningShrimp/os/discussions)
- **Email**: [contact email]

---

## 🌟 Acknowledgments

- The Rust community for excellent tools and support
- Linux kernel for architecture inspiration
- All contributors who make this project possible

---

**Current Status**: 🔄 Active Development - Phase 1 Week 1  
**Last Updated**: 2025-12-09  
**Next Milestone**: Week 1 Completion (2025-12-15)

---

*Building a modern OS kernel, one commit at a time.* 🚀
