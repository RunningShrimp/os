# Feature Flags

This document describes the feature flags available in the NOS operating system and their dependencies.

## Table of Contents

- [Overview](#overview)
- [Kernel Features](#kernel-features)
- [Workspace Crate Features](#workspace-crate-features)
- [Dependency Graph](#dependency-graph)
- [Usage Examples](#usage-examples)
- [Feature Combinations](#feature-combinations)

## Overview

NOS uses Cargo's feature flag system to provide modular compilation and conditional features. Features are organized hierarchically, with the kernel depending on several workspace crates, each with their own feature sets.

### Feature Design Principles

1. **Modular**: Each feature can be enabled independently
2. **Hierarchical**: Features depend on other features in a clear tree structure
3. **no_std First**: All features work without the standard library by default
4. **Testing Optional**: The `std` feature is only needed for testing, not production

## Kernel Features

The kernel package (`kernel/Cargo.toml`) is the main entry point and includes the following features:

### Default Features

```toml
default = ["posix_layer", "net_stack", "syscalls", "services", "error_handling"]
```

The default feature set includes:
- `posix_layer` - POSIX compatibility layer
- `net_stack` - Network protocol stack
- `syscalls` - System call interface (enables nos-syscalls crate)
- `services` - Service management framework (enables nos-services crate)
- `error_handling` - Unified error handling (enables nos-error-handling crate)

### Core Features

#### `baremetal`
Bare metal boot support with assembly paths for:
- Global assembly (`global_asm`)
- Trap handling
- Context switching
- Real target board or no-OS environments

**Dependencies**: None
**Use Case**: Production on real hardware

#### `kernel_tests`
Enables kernel self-test entry points and test cases:
- `run_tests()` entry point
- `test_*()` test functions
- Built-in test framework

**Dependencies**: None
**Use Case**: Development and testing (default: disabled)
**Note**: Disabled by default to avoid host compilation interference

#### `posix_layer`
POSIX compatibility layer providing:
- POSIX system call interfaces
- Standard Unix-like behavior
- Compatibility with existing POSIX software

**Dependencies**: None
**Use Case**: Running POSIX applications

#### `net_stack`
Network protocol stack implementation:
- TCP/IP networking
- Socket interfaces
- Network device drivers

**Dependencies**: None
**Use Case**: Network connectivity

### Module Features

#### `syscalls`
System call interface and dispatch mechanism.

**Dependencies**: Enables `nos-syscalls` crate
**Related Features**:
- `advanced_syscalls` - Advanced system call extensions (enables `nos-syscalls/advanced_syscalls`)

**Use Case**: User-kernel interface

#### `services`
Service management and discovery framework.

**Dependencies**: Enables `nos-services` crate
**Use Case**: Dynamic service loading and management

#### `error_handling`
Unified error handling and recovery framework.

**Dependencies**: Enables `nos-error-handling` crate
**Use Case**: Consistent error reporting across subsystems

### Performance Optimization Features

#### `fast_syscall`
Optimized system call fast path:
- Reduced overhead for common syscalls
- Inline optimizations
- Specialized dispatch paths

**Dependencies**: `syscalls`
**Use Case**: High-performance applications

#### `zero_copy`
Zero-copy I/O operations:
- Direct memory access between kernel and userspace
- Reduced memory copies
- Improved throughput

**Dependencies**: None
**Use Case**: High-performance I/O

#### `batch_syscalls`
Batch system call processing:
- Multiple syscalls in one transition
- Reduced context switches
- Improved throughput for bulk operations

**Dependencies**: `syscalls`
**Use Case**: Database, file systems

#### `net_opt`
Network stack optimizations:
- Advanced packet processing
- Zero-copy networking
- Optimized buffer management

**Dependencies**: `net_stack`
**Use Case**: High-performance networking

#### `sched_opt`
Scheduler optimizations:
- Advanced scheduling algorithms
- Reduced context switch overhead
- Improved CPU utilization

**Dependencies**: None
**Use Case**: Real-time and high-throughput scenarios

### Memory Features

#### `hpage_2mb`
Enable 2MB huge pages:
- Reduced TLB pressure
- Improved performance for large mappings
- Better memory utilization

**Dependencies**: None
**Use Case**: Database, large memory applications

#### `hpage_1gb`
Enable 1GB huge pages:
- Further reduced TLB pressure
- Maximum performance for very large mappings
- Specialized hardware support required

**Dependencies**: None
**Use Case**: Very large memory applications, databases

#### `lazy_init`
Lazy initialization support:
- Deferred component initialization
- Faster boot times
- Memory savings for unused features

**Dependencies**: None
**Use Case**: Embedded systems, fast boot requirements

### Subsystem Features

#### `graphics_subsystem`
Graphics and display support:
- Framebuffer management
- GPU integration
- Compositing

**Dependencies**: None
**Use Case**: Desktop, GUI applications

#### `web_engine`
Web rendering engine:
- HTML/CSS/JavaScript support
- Web standards compliance
- Browser capabilities

**Dependencies**: `graphics_subsystem`
**Use Case**: Web browser, web applications

#### `security_audit`
Security audit and logging:
- Comprehensive security event logging
- Audit trail generation
- Compliance support

**Dependencies**: None
**Use Case**: Security-critical environments

#### `formal_verification`
Formal verification annotations:
- Specification annotations
- Verification harnesses
- Proof-carrying code support

**Dependencies**: None
**Use Case**: Safety-critical systems

#### `observability`
Observability and monitoring:
- Metrics collection
- Performance tracing
- Debug instrumentation

**Dependencies**: None
**Use Case**: Production monitoring, debugging

#### `debug_subsystems`
Debug subsystems for development:
- Enhanced debugging output
- Development tools integration
- Detailed logging

**Dependencies**: None
**Use Case**: Development only

#### `cloud_native`
Cloud-native features:
- Container support
- Orchestration integration
- Microservices patterns

**Dependencies**: None
**Use Case**: Cloud deployments, containers

#### `realtime`
Real-time scheduling and timing:
- Deterministic scheduling
- Real-time guarantees
- Low-latency paths

**Dependencies**: None
**Use Case**: Real-time systems, audio/video

### Infrastructure Features

#### `alloc`
Heap allocation support:
- Dynamic memory allocation
- Standard collection types
- Managed memory

**Dependencies**: None
**Use Case**: Most kernel features (typically auto-enabled)

#### `strict_boot`
Strict boot validation:
- Comprehensive boot checks
- Validation at each stage
- Early error detection

**Dependencies**: None
**Use Case**: Production, safety-critical systems

#### `journaling_fs`
Journaling file system support:
- Data integrity guarantees
- Crash recovery
- Transactional metadata updates

**Dependencies**: None
**Use Case**: Reliable file storage

#### `link_phys_end`
Link physical address at end of image:
- Specific memory layout
- Bootloader integration

**Dependencies**: None
**Use Case**: Custom boot configurations

#### `log`
Logging framework support:
- Structured logging
- Multiple log levels
- Configurable output

**Dependencies**: None
**Use Case**: Debugging, monitoring

## Workspace Crate Features

### nos-api

Core interfaces and types for NOS.

**Features**:
- `std` - Standard library support (testing only)
- `log` - Logging support
- `debug_subsystems` - Debug instrumentation
- `formal_verification` - Verification annotations
- `security_audit` - Security annotations
- `minimal` - Minimal build configuration
- `embedded` - Embedded system optimizations
- `server` - Server configuration (with `log`)
- `desktop` - Desktop configuration (with `log`)
- `full` - Full configuration (`log` + `debug_subsystems`)
- `debug` - Debug configuration (`log` + `debug_subsystems`)
- `release` - Release configuration

**Default**: No default features (empty array)

### nos-syscalls

System call interface definitions.

**Features**:
- `advanced_syscalls` - Advanced system call extensions

**Default**: No default features

### nos-services

Service management framework.

**Features**:
- `std` - Standard library support (testing only)
- `log` - Logging support
- `debug_subsystems` - Debug instrumentation
- `formal_verification` - Verification annotations
- `security_audit` - Security annotations
- `minimal` - Minimal build configuration
- `embedded` - Embedded system optimizations
- `server` - Server configuration (with `log`)
- `desktop` - Desktop configuration (with `log`)

**Default**: No default features

### nos-error-handling

Error handling and recovery framework.

**Features**:
- `std` - Standard library support (testing only)
- `log` - Logging support
- `debug_subsystems` - Debug instrumentation
- `formal_verification` - Verification annotations
- `security_audit` - Security annotations
- `minimal` - Minimal build configuration
- `embedded` - Embedded system optimizations
- `server` - Server configuration (with `log`)
- `desktop` - Desktop configuration (with `log`)

**Default**: No default features

### nos-memory-management

Physical and virtual memory management.

**Features**:
- `std` - Standard library support (testing only)
- `log` - Logging support
- `debug_subsystems` - Debug instrumentation
- `formal_verification` - Verification annotations
- `security_audit` - Security annotations
- `minimal` - Minimal build configuration
- `embedded` - Embedded system optimizations
- `server` - Server configuration (with `log`)
- `desktop` - Desktop configuration (with `log`)

**Default**: No default features

## Dependency Graph

### External Dependencies

```
kernel
├── nos-api (no dependencies)
├── nos-memory-management
│   └── nos-api
├── nos-syscalls (optional)
│   └── nos-api
├── nos-services (optional)
│   ├── nos-api
│   └── log (optional)
└── nos-error-handling (optional)
    └── nos-api
```

### Feature Dependencies

```
default
├── posix_layer
├── net_stack
├── syscalls
│   └── advanced_syscalls (optional)
├── services
└── error_handling

Performance Features:
├── fast_syscall → syscalls
├── batch_syscalls → syscalls
├── net_opt → net_stack
└── sched_opt (standalone)

Memory Features:
├── hpage_2mb (standalone)
└── hpage_1gb (standalone)

Subsystem Features:
├── graphics_subsystem (standalone)
├── web_engine → graphics_subsystem
├── security_audit (standalone)
├── formal_verification (standalone)
├── observability (standalone)
├── debug_subsystems (standalone)
├── cloud_native (standalone)
└── realtime (standalone)

Infrastructure:
├── alloc (standalone)
├── strict_boot (standalone)
├── journaling_fs (standalone)
├── link_phys_end (standalone)
└── log (standalone)
```

### Feature Incompatibilities

Currently, there are no explicitly incompatible features. However, some features may conflict in practice:

- `baremetal` and `std` - Bare metal typically doesn't use std
- `minimal` and `debug_subsystems` - Minimal vs. debug features
- `embedded` and `full` - Different optimization goals

## Usage Examples

### Minimal Kernel

```toml
# For embedded or minimal deployments
[dependencies.kernel]
features = ["baremetal"]
default-features = false
```

### Standard Desktop

```toml
# Default features are suitable for most use cases
[dependencies.kernel]
# Uses default features
```

### High-Performance Server

```toml
[dependencies.kernel]
features = [
    "fast_syscall",
    "zero_copy",
    "net_opt",
    "hpage_2mb",
    "observability",
]
```

### Development Build

```toml
[dependencies.kernel]
features = [
    "kernel_tests",
    "debug_subsystems",
    "log",
    "observability",
]

[dependencies.nos-api]
features = ["std", "log", "debug_subsystems"]

[dependencies.nos-syscalls]
features = ["advanced_syscalls"]
```

### Real-Time System

```toml
[dependencies.kernel]
features = [
    "baremetal",
    "realtime",
    "sched_opt",
    "strict_boot",
]
default-features = false
```

### Cloud Native

```toml
[dependencies.kernel]
features = [
    "cloud_native",
    "observability",
    "net_stack",
    "security_audit",
]
```

### Graphics/Web

```toml
[dependencies.kernel]
features = [
    "graphics_subsystem",
    "web_engine",
    "posix_layer",
]
```

## Feature Combinations

### Recommended Combinations

#### Minimal Embedded
```toml
features = ["baremetal", "minimal", "embedded"]
```
- Smallest binary size
- No optional features
- Suitable for resource-constrained devices

#### Development
```toml
features = ["kernel_tests", "debug_subsystems", "log", "observability"]
```
- Full debugging support
- Test infrastructure
- Development tools

#### Production Server
```toml
features = [
    "fast_syscall",
    "zero_copy",
    "net_opt",
    "hpage_2mb",
    "observability",
    "security_audit",
]
```
- High performance
- Monitoring
- Security features

#### Desktop
```toml
features = [
    "posix_layer",
    "net_stack",
    "graphics_subsystem",
    "syscalls",
    "services",
]
```
- Full user experience
- GUI support
- Standard applications

#### Real-Time
```toml
features = ["baremetal", "realtime", "sched_opt", "strict_boot"]
```
- Deterministic behavior
- Low latency
- Safety guarantees

### Testing Combinations

#### Unit Tests
```toml
features = ["kernel_tests"]

[dev-dependencies]
# All crates with std feature
nos-api = { features = ["std"] }
nos-syscalls = { features = ["std"] }
nos-services = { features = ["std"] }
nos-error-handling = { features = ["std"] }
```

#### Integration Tests
```toml
features = [
    "kernel_tests",
    "posix_layer",
    "net_stack",
    "syscalls",
]
```

#### Benchmarks
```toml
features = [
    "kernel_tests",
    "fast_syscall",
    "zero_copy",
    "hpage_2mb",
]
```

## Feature Selection Guidelines

### When to Use `default-features = false`

Use this when:
1. Building for bare metal
2. Creating minimal embedded systems
3. Selecting specific features manually
4. Reducing binary size
5. Custom feature combinations

### When to Enable Specific Features

**Performance Features**:
- Enable `fast_syscall`, `zero_copy`, `batch_syscalls` for high-throughput applications
- Enable `hpage_2mb`, `hpage_1gb` for large memory workloads
- Enable `net_opt` for network-intensive applications

**Debugging Features**:
- Enable `debug_subsystems` for development only
- Enable `observability` for production monitoring
- Enable `kernel_tests` for test builds

**Safety Features**:
- Enable `strict_boot` for production
- Enable `security_audit` for security-critical systems
- Enable `formal_verification` for verified systems

### Feature Size Impact

Approximate binary size impact (relative to minimal):

| Feature | Size Impact | Notes |
|---------|-------------|-------|
| `baremetal` | +0% | Base configuration |
| `posix_layer` | +15% | Significant compatibility layer |
| `net_stack` | +20% | Full TCP/IP stack |
| `syscalls` | +5% | Dispatch infrastructure |
| `services` | +8% | Service framework |
| `error_handling` | +3% | Error types and handling |
| `graphics_subsystem` | +25% | GPU drivers, compositor |
| `web_engine` | +40% | Rendering, JS engine |
| `kernel_tests` | +10% | Test infrastructure |
| `debug_subsystems` | +5% | Debug code |
| `observability` | +7% | Metrics, tracing |

*Note: These are rough estimates and can vary significantly based on configuration.*

## Best Practices

1. **Start with defaults**: Use default features initially, then optimize
2. **Test feature combinations**: Some features may interact unexpectedly
3. **Profile before optimizing**: Measure performance impacts of features
4. **Document custom features**: If you add custom features, document them
5. **Keep dev features separate**: Use feature profiles for development vs. production
6. **Use workspace features**: Enable features consistently across workspace crates

## Migration Guide

### From Old Feature Names

If you're using an older version of NOS, some feature names may have changed:

| Old Feature | New Feature | Notes |
|-------------|-------------|-------|
| `std` | `kernel_tests` | For testing |
| `huge_pages` | `hpage_2mb` or `hpage_1gb` | More specific |
| `networking` | `net_stack` | Clarified naming |
| `graphics` | `graphics_subsystem` | Clarified naming |

## Further Reading

- [Cargo Features Documentation](https://doc.rust-lang.org/cargo/reference/features.html)
- [Kernel Architecture Documentation](./ARCHITECTURE.md)
- [Building NOS](../README.md#building)
- [Testing Guide](./TESTING.md)

## Contributing

When adding new features:

1. Update this document with the new feature
2. Document all dependencies
3. Add usage examples
4. Update the dependency graph
5. Note any incompatibilities
6. Include size/performance impact if known

## Feature Request Process

To request new features:

1. Open an issue describing the feature
2. Explain the use case and requirements
3. Discuss dependencies and potential conflicts
4. Propose feature flag name and configuration
5. Update documentation if approved
