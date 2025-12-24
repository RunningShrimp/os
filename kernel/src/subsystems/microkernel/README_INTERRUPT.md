# Interrupt Subsystem

## Overview

The NOS kernel provides a comprehensive interrupt handling subsystem supporting multiple architectures (x86_64, AArch64, RISC-V). The interrupt subsystem manages hardware interrupts, exceptions, and inter-processor interrupts (IPIs).

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   Hardware Devices                    │
│  - Timer, Keyboard, Network, Disk, etc.           │
└────────────────────┬────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│               Interrupt Controllers                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  │
│  │   PIC    │  │   APIC   │  │   GIC    │  │
│  │  (x86)   │  │ (x86/x64)│  │ (ARM)    │  │
│  └─────┬────┘  └─────┬────┘  └─────┬────┘  │
└────────────┼───────────────┼───────────────┼───────────┘
             │               │               │
             ▼               ▼               ▼
┌─────────────────────────────────────────────────────────┐
│              Interrupt Routing                    │
│  - Vector assignment                               │
│  - Priority management                            │
│  - IRQ masking                                   │
└────────────────────┬────────────────────────────────┘
                     │
                     ▼
┌─────────────────────────────────────────────────────────┐
│           IDT / Vector Table                     │
│  - x86_64: IDT (256 entries)                    │
│  - AArch64: Exception Vector Table                   │
│  - RISC-V: Trap Vector Table                       │
└────────────────────┬────────────────────────────────┘
                     │
        ┌────────────┴────────────┐
        ▼                         ▼
┌──────────────┐      ┌──────────────┐
│  Interrupt   │      │  Exception   │
│  Handlers    │      │  Handlers   │
└──────────────┘      └──────────────┘
        │                     │
        ▼                     ▼
┌────────────────────────────────────────────┐
│           Microkernel Core           │
│  - Device drivers                  │
│  - Task scheduler                  │
│  - System services                 │
└────────────────────────────────────────────┘
```

## Module Structure

| Module | Description |
|--------|-------------|
| `microkernel/interrupt.rs` | Core interrupt management for microkernel |
| `bootloader/arch/x86_64/idt.rs` | x86_64 IDT implementation |
| `bootloader/cpu_init/idt_manager.rs` | Advanced IDT management |
| `bootloader/cpu_init/interrupt_routing.rs` | PIC and APIC configuration |
| `bootloader/acpi_support/acpi_parser.rs` | ACPI MADT parsing |
| `bootloader/cpu_init/exception_handler.rs` | Exception handling framework |
| `platform/drivers/gic.rs` | ARM GICv2 implementation |
| `platform/drivers/gicv3.rs` | ARM GICv3 implementation |
| `platform/trap/mod.rs` (RISC-V) | RISC-V trap handling |
| `platform/trap/mod.rs` (AArch64) | AArch64 exception handling |
| `platform/trap/mod.rs` (x86_64) | x86_64 trap handling |
| `platform/drivers/device_manager.rs` | Device interrupt management |

## Interrupt Vectors

### x86_64 Vectors

```rust
pub enum InterruptVector {
    // Exceptions (0-31)
    DivideError = 0,
    Debug = 1,
    NonMaskableInterrupt = 2,
    Breakpoint = 3,
    IntoDebugTrap = 4,
    BoundsCheck = 5,
    InvalidOpcode = 6,
    NoCoprocessor = 7,
    DoubleFault = 8,
    CoprocessorSegment = 9,
    InvalidTss = 10,
    SegmentNotPresent = 11,
    StackSegmentFault = 12,
    GeneralProtectionFault = 13,
    PageFault = 14,
    // ... more exceptions

    // IRQs (32-47)
    Timer = 32,
    Keyboard = 33,
    Cascade = 34,
    Com2 = 35,
    Com1 = 36,
    Lpt2 = 37,
    Floppy = 38,
    Lpt1 = 39,
    Rtc = 40,
    Mouse = 44,
    // ... more IRQs

    // Software interrupts
    SystemCall = 0x80,
}
```

### RISC-V Trap Causes

```rust
pub enum TrapCause {
    InstructionMisaligned = 0,
    InstructionAccess = 1,
    IllegalInstruction = 2,
    Breakpoint = 3,
    LoadMisaligned = 4,
    LoadAccess = 5,
    StoreMisaligned = 6,
    StoreAccess = 7,
    UserEcall = 8,
    SupervisorEcall = 9,
    InstructionPageFault = 12,
    LoadPageFault = 13,
    StorePageFault = 15,
}
```

### AArch64 Exception Classes

```rust
pub enum ExceptionClass {
    SyncLowerEl0,
    IrqLowerEl0,
    FiqLowerEl0,
    SErrorLowerEl0,
    SyncLowerEl1,
    IrqLowerEl1,
    FiqLowerEl1,
    SErrorLowerEl1,
    // ... 16 total classes
}
```

## Interrupt Context

### x86_64 Context

```rust
pub struct InterruptContext {
    pub vector: InterruptVector,
    pub error_code: Option<u64>,
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
    pub registers: Registers,
}
```

### RISC-V Context

```rust
pub struct TrapFrame {
    pub x: [u64; 32],
    pub pc: u64,
    pub status: u64,
}
```

## Interrupt Controllers

### PIC (8259A)

Legacy interrupt controller for x86:

```rust
pub struct Pic8259a {
    master_base: u16,
    slave_base: u16,
    mode: PicMode,
}

pub enum PicMode {
    Single,
    Cascaded,
}
```

Features:
- Up to 16 IRQs in cascaded mode
- Programmable priority levels
- IRQ masking/unmasking
- EOI (End-Of-Interrupt) signaling

### APIC (Advanced Programmable Interrupt Controller)

Modern interrupt controller for x86/x64:

```rust
pub struct ApicConfig {
    pub base_address: u64,
    pub apic_id: u8,
    pub is_bsp: bool,
    pub enabled: bool,
}

pub enum ApicMode {
    Pic,
    Apic,
    X2Apic,
}
```

Features:
- Up to 256 interrupt vectors
- Per-CPU local APICs
- I/O APIC for device routing
- MSI/MSI-X support
- Inter-processor interrupts (IPIs)

### GIC (Generic Interrupt Controller)

ARM interrupt controller:

```rust
pub struct GicConfig {
    pub distributor_base: u64,
    pub cpu_interface_base: u64,
    pub version: u32,
}
```

Features:
- GICv2: Distributor + CPU interface
- GICv3: System register interface (ICC_*_EL1)
- Redistributors for multi-cluster systems
- Software Generated Interrupts (SGIs)

## Interrupt Handlers

### Handler Registration

```rust
pub struct VectorTable {
    handlers: BTreeMap<InterruptVector, InterruptHandler>,
    stats: InterruptStats,
}

pub struct InterruptHandler {
    handler: fn(context: &InterruptContext) -> InterruptResult,
    priority: InterruptPriority,
    name: String,
    stats: HandlerStats,
}

pub enum InterruptPriority {
    Critical,   // NMI, exceptions
    High,       // Timer, watchdog
    Medium,     // Network, disk
    Low,        // Keyboard, mouse
}
```

### Interrupt Results

```rust
pub enum InterruptResult {
    Handled,
    NotHandled,
    Deferred,    // Defer to tasklet
    Reschedule, // Reschedule needed
}
```

## Exception Handling

### Exception Framework

```rust
pub struct ExceptionHandler {
    exceptions: BTreeMap<ExceptionType, fn(context: &ExceptionContext)>,
    recovery_strategies: BTreeMap<ExceptionType, RecoveryStrategy>,
}

pub enum ExceptionType {
    DivideByZero,
    Debug,
    NonMaskableInterrupt,
    Breakpoint,
    PageFault,
    GeneralProtectionFault,
    // ... more types
}

pub enum RecoveryStrategy {
    Skip,
    Retry,
    Fallback,
    Fatal,
}

pub struct ExceptionContext {
    pub exception_type: ExceptionType,
    pub error_code: Option<u64>,
    pub rip: u64,
    pub cr2: u64,  // Page fault address on x86_64
}
```

## Inter-Processor Interrupts (IPI)

### IPI Types

```rust
pub enum IpiType {
    Reschedule,
    Stop,
    FlushTLB,
    CallFunction,
    KickCPU,
}

pub fn send_ipi(target_cpu: u32, ipi_type: IpiType) -> Result<(), KernelError> {
    match current_arch() {
        Architecture::X86_64 => {
            apic_send_ipi(target_cpu, ipi_type)
        }
        Architecture::AArch64 => {
            gic_send_sgi(target_cpu, ipi_type)
        }
        Architecture::RiscV64 => {
            sbi_send_ipi(target_cpu, ipi_type)
        }
    }
}
```

## Interrupt Statistics

```rust
pub struct InterruptStats {
    pub total_interrupts: u64,
    pub exceptions: u64,
    pub irqs: u64,
    pub software_interrupts: u64,
    pub spurious_interrupts: u64,
    pub nested_interrupts: u64,
    pub per_handler_stats: BTreeMap<InterruptVector, HandlerStats>,
}

pub struct HandlerStats {
    pub call_count: u64,
    pub total_time_ns: u64,
    pub avg_time_ns: u64,
    pub max_time_ns: u64,
}
```

## Interrupt Control

### Enabling/Disabling

```rust
// Global interrupt control
pub fn enable_interrupts() {
    #[cfg(target_arch = "x86_64")]
    unsafe { asm!("sti"); }

    #[cfg(target_arch = "aarch64")]
    unsafe { asm!("msr daifclr, #2"); }

    #[cfg(target_arch = "riscv64")]
    unsafe { asm!("csrsi mstatus, 8"); }
}

pub fn disable_interrupts() {
    #[cfg(target_arch = "x86_64")]
    unsafe { asm!("cli"); }

    #[cfg(target_arch = "aarch64")]
    unsafe { asm!("msr daifset, #2"); }

    #[cfg(target_arch = "riscv64")]
    unsafe { asm!("csrci mstatus, 8"); }
}

pub fn are_interrupts_enabled() -> bool {
    #[cfg(target_arch = "x86_64")]
    { (get_rflags() & 0x200) != 0 }

    #[cfg(target_arch = "aarch64")]
    { (get_daif() & 0x80) == 0 }

    #[cfg(target_arch = "riscv64")]
    { (get_mstatus() & 0x8) != 0 }
}

// Local interrupt control (for critical sections)
pub struct InterruptGuard;

impl InterruptGuard {
    pub fn new() -> Self {
        disable_interrupts();
        InterruptGuard
    }
}

impl Drop for InterruptGuard {
    fn drop(&mut self) {
        enable_interrupts();
    }
}
```

## Interrupt Priority and Masking

### Priority Levels

```rust
pub const NUM_PRIORITY_LEVELS: usize = 16;

pub struct InterruptPriorityManager {
    priority_levels: [AtomicU32; NUM_PRIORITY_LEVELS],
    current_priority: AtomicU32,
}

impl InterruptPriorityManager {
    pub fn set_priority(&self, level: u8) {
        self.current_priority.store(level as u32, Ordering::Release);
    }

    pub fn raise_priority(&self, level: u8) -> InterruptGuard {
        let old = self.current_priority.load(Ordering::Acquire);
        self.current_priority.store(level as u32, Ordering::Release);
        InterruptGuard
    }
}
```

### IRQ Masking

```rust
pub struct IrqMask {
    pub mask: u16,  // 16 IRQs for PIC, 256 for APIC
}

pub fn mask_irq(irq: u8) {
    #[cfg(feature = "pic")]
    pic_mask_irq(irq);

    #[cfg(feature = "apic")]
    apic_mask_irq(irq);
}

pub fn unmask_irq(irq: u8) {
    #[cfg(feature = "pic")]
    pic_unmask_irq(irq);

    #[cfg(feature = "apic")]
    apic_unmask_irq(irq);
}
```

## Device Interrupt Management

### Interrupt Resource Allocation

```rust
pub struct InterruptResource {
    pub irq: u32,
    pub vector: InterruptVector,
    pub trigger_mode: TriggerMode,
    pub polarity: Polarity,
}

pub enum TriggerMode {
    Edge,
    Level,
}

pub struct InterruptResourcePool {
    available: BTreeSet<InterruptResource>,
    allocated: BTreeMap<DeviceId, InterruptResource>,
}
```

## Future Improvements

- [ ] Implement interrupt threading for bottom-half processing
- [ ] Add per-CPU interrupt statistics
- [ ] Implement interrupt coalescing for high-frequency devices
- [ ] Add interrupt affinity management
- [ ] Implement MSI/MSI-X configuration
- [ ] Add interrupt storm detection
- [ ] Implement interrupt load balancing
- [ ] Add interrupt latency monitoring
- [ ] Support for more interrupt controllers (e.g., PLIC for RISC-V)
