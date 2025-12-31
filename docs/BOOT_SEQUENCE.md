# NOS Kernel Boot Sequence Documentation

## Executive Summary

This document provides a comprehensive technical overview of the NOS kernel boot sequence from power-on to userspace initialization. It covers the complete boot flow across all supported architectures (x86_64, ARM64, RISC-V), including timing analysis, critical dependencies, and implementation references.

**Document Version**: 1.0
**Last Updated**: 2025-01-01
**Total Word Count**: ~3,500 words

---

## Table of Contents

1. [Bootloader Phase](#1-bootloader-phase)
2. [Early Kernel Initialization](#2-early-kernel-initialization)
3. [Core Subsystem Initialization](#3-core-subsystem-initialization)
4. [Late Initialization](#4-late-initialization)
5. [Timing and Parallelization](#5-timing-and-parallelization)
6. [Architecture-Specific Details](#6-architecture-specific-details)
7. [Troubleshooting Boot Issues](#7-troubleshooting-boot-issues)

---

## 1. Bootloader Phase

The bootloader phase spans from system power-on to kernel entry point. This phase is responsible for hardware initialization, memory discovery, and constructing the boot information structure passed to the kernel.

### 1.1 BIOS/UEFI Initialization

**Duration**: 50-200ms (hardware dependent)

**Process Flow**:

1. **Power-On Self-Test (POST)**
   - CPU initialization and testing
   - Memory controller initialization
   - Basic I/O device detection
   - Firmware integrity verification

2. **Boot Device Selection**
   - Read boot order from NVRAM
   - Iterate through boot devices
   - Load first sector (MBR) or EFI boot loader
   - Transfer control to boot loader

**Implementation References**:
- MBR parsing: Not in kernel (bootloader responsibility)
- EFI handoff: See `kernel/src/platform/boot/validator.rs:120-180`

### 1.2 Multiboot2 Protocol

**Duration**: 5-10ms

For x86_64 systems using Multiboot2-compliant bootloaders (GRUB), the kernel implements the Multiboot2 specification for receiving boot information.

**Header Structure** (`kernel/src/platform/boot/multiboot2.rs:45-67`):

```rust
#[repr(C, packed)]
pub struct MultibootHeader {
    pub magic: u32,              // 0x36d76289
    pub architecture: u32,
    pub header_length: u32,
    pub checksum: u32,
    // ... tags follow
}
```

**Key Tags Parsed**:

1. **Memory Map Tag** (`MultibootTagType::Mmap`)
   - Provides physical memory layout
   - Available vs reserved memory regions
   - ACPI reclaimable regions
   - NVS memory regions

2. **Framebuffer Tag** (`MultibootTagType::Framebuffer`)
   - Physical address of framebuffer
   - Resolution and pitch
   - Pixel format (RGB/Indexed)

3. **ACPI Old/RSDP Tags**
   - Root System Description Pointer address
   - Used for ACPI table discovery

4. **Boot Command Line Tag**
   - Kernel command line arguments
   - Debug flags and configuration

**Implementation**: `kernel/src/platform/boot/multiboot2.rs:180-320`

### 1.3 UEFI Handoff Protocol

**Duration**: 10-20ms

For UEFI systems, the kernel receives control via the UEFI boot services.

**EFI System Table Access**:

The EFI system table pointer is passed in register `rsi` (x86_64) or `x0` (ARM64) at kernel entry.

**Key Information Retrieved**:

1. **EFI Memory Map** (`kernel/src/platform/boot/uefi.rs:89-145`)
   - Call `GetMemoryMap()` boot service
   - Parse memory types (EfiConventionalMemory, EfiLoaderCode, etc.)
   - Convert to kernel memory regions
   - Calculate required memory map size for ExitBootServices()

2. **Graphics Output Protocol** (`kernel/src/platform/boot/uefi.rs:200-250`)
   - Locate GOP protocol
   - Query available modes
   - Set preferred resolution
   - Obtain framebuffer address

3. **ACPI Tables** (`kernel/src/platform/boot/uefi.rs:300-340`)
   - Locate ACPI 2.0+ RSDP
   - Verify XSDT checksum
   - Pass to kernel for later parsing

**Implementation**: `kernel/src/platform/boot/mod.rs:150-280`

### 1.4 Memory Map Construction

**Duration**: 2-5ms

The kernel constructs a unified memory map from bootloader-provided information.

**Memory Region Types** (`kernel/src/memory/mod.rs:45-67`):

```rust
pub enum MemoryType {
    Available,      // Usable physical memory
    Reserved,       // Hardware reserved, do not use
    AcpiReclaim,    // ACPI tables (can reclaim after init)
    AcpiNvs,        // ACPI NVS (must preserve)
    Unusable,       // Reported as unusable by firmware
    Kernel,         // Kernel code and data
    Framebuffer,    // Graphics framebuffer
}
```

**Memory Map Processing Steps**:

1. **Merge Adjacent Regions**: Combine contiguous regions of same type
2. **Sort by Address**: Enable binary search for address lookup
3. **Calculate Statistics**:
   - Total physical memory
   - Available memory
   - Reserved memory
   - Maximum physical address

**Implementation**: `kernel/src/memory/map.rs:120-200`

### 1.5 ACPI Table Discovery

**Duration**: 5-15ms

The kernel locates and verifies ACPI tables for hardware configuration.

**Root Signature Verification** (`kernel/src/platform/acpi.rs:89-120`):

```rust
pub fn verify_rsdp(rsdp_addr: usize) -> Option<&'static Rsdp> {
    let rsdp = unsafe { &*(rsdp_addr as *const Rsdp) };

    // Verify signature "RSD PTR " (8 bytes)
    if &rsdp.signature != b"RSD PTR " {
        return None;
    }

    // Verify checksum
    if !verify_checksum(rsdp as *const _ as *const u8, 20) {
        return None;
    }

    Some(rsdp)
}
```

**Key ACPI Tables Parsed**:

1. **RSDT/XSDT**: Root System Description Table
   - Provides list of all other ACPI tables
   - XSDT uses 64-bit pointers (ACPI 2.0+)
   - RSDT uses 32-bit pointers (ACPI 1.0)

2. **FADT**: Fixed ACPI Description Table
   - PM1a/PM1b control registers
   - Hardware reduced flag
   - ACPI register space addresses

3. **MADT**: Multiple APIC Description Table
   - Local APIC addresses
   - I/O APIC addresses
   - Interrupt source overrides
   - NMI sources

4. **DSDT**: Differentiated System Description Table
   - AML bytecode for device enumeration
   - Compiled and executed during device init

5. **SSDT**: Secondary System Description Tables
   - Additional AML bytecode
   - Platform-specific extensions

**Implementation**: `kernel/src/platform/acpi.rs:150-450`

### 1.6 Boot Information Structure

**Duration**: 1-2ms

All bootloader information is packaged into a single structure for kernel consumption.

**BootInfo Structure** (`kernel/src/platform/boot/mod.rs:45-78`):

```rust
pub struct BootInfo {
    pub memory_map: &'static MemoryMap,
    pub rsdp_address: Option<usize>,
    pub framebuffer: Option<FramebufferInfo>,
    pub cmdline: Option<&'static str>,
    pub elf_sections: Option<&'static [ElfSection]>,
    pub mmap_addr: usize,
    pub mmap_size: usize,
}

pub struct FramebufferInfo {
    pub address: u64,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u8,
    pub format: PixelFormat,
}
```

**Implementation**: `kernel/src/platform/boot/mod.rs:80-150`

**Phase 1 Summary**: Total duration 73-352ms (hardware dependent)

---

## 2. Early Kernel Initialization

Early initialization runs in a minimal environment with basic paging enabled but most services unavailable.

### 2.1 Architecture-Specific Setup

**Duration**: 5-20ms per architecture

This phase sets up architecture-specific hardware and CPU features.

#### x86_64 Early Setup (`kernel/src/arch/x86_64/boot.rs:50-200`)

**Critical Steps**:

1. **GDT Setup** (Global Descriptor Table)
   ```rust
   // GDT entries: null, kernel code, kernel data, user code, user data, TSS
   static GDT: [GdtEntry; 6] = [
       GdtEntry::new(0, 0, 0),                    // Null
       GdtEntry::new(0, 0xFFFFF, GDT_CODE_KERNEL), // Kernel code
       GdtEntry::new(0, 0xFFFFF, GDT_DATA_KERNEL), // Kernel data
       GstEntry::new(0, 0xFFFFF, GDT_CODE_USER),   // User code
       GdtEntry::new(0, 0xFFFFF, GDT_DATA_USER),   // User data
   ];

   // Load GDT
   lgdt(&GDT_DESCRIPTOR);
   ```

   **GDT Entry Flags**:
   - Kernel code: Execute-only, ring 0, 64-bit
   - Kernel data: Read-write, ring 0
   - User code: Execute-only, ring 3, 64-bit
   - User data: Read-write, ring 3

2. **IDT Setup** (Interrupt Descriptor Table)
   ```rust
   static IDT: [IdtEntry; 256] = [IdtEntry::new(0); 256];

   // Set exception handlers
   IDT[0] = IdtEntry::new_exception(divide_error_handler);     // #DE
   IDT[1] = IdtEntry::new_exception(debug_handler);            // #DB
   IDT[2] = IdtEntry::new_interrupt(nmi_handler);              // NMI
   IDT[3] = IdtEntry::new_exception(breakpoint_handler);       // #BP
   IDT[6] = IdtEntry::new_exception(invalid_opcode_handler);   // #UD
   IDT[13] = IdtEntry::new_exception(general_protection_handler); // #GP
   IDT[14] = IdtEntry::new_exception(page_fault_handler);      // #PF

   // Load IDT
   lidt(&IDT_DESCRIPTOR);
   ```

   **ISR Stack Setup**:
   - Each ISR has its own stack (IST)
   - Prevents stack overflow on nested exceptions
   - Configured in TSS

3. **Paging Setup**
   ```rust
   // Create initial page table (identity map + kernel mapping)
   let pml4 = allocate_page() as *mut PageTable;

   // Identity map low 4MB
   map_page_4k(pml4, 0x0, 0x0, Flags::PRESENT | Flags::WRITABLE);
   map_page_4k(pml4, 0x1000, 0x1000, Flags::PRESENT | Flags::WRITABLE);
   // ... up to 0x40000

   // Map kernel at higher half (0xFFFFFFFF80000000)
   let kernel_virt_base = 0xFFFFFFFF80000000;
   for i in 0..kernel_size / PAGE_SIZE {
       map_page_4k(pml4, kernel_virt_base + i * PAGE_SIZE, kernel_phys + i * PAGE_SIZE,
                   Flags::PRESENT | Flags::WRITABLE | Flags::NO_EXECUTE);
   }

   // Load CR3
   let cr3 = pml4 as u64;
   unsafe { asm!("mov cr3, {}", in(reg) cr3); }
   ```

   **Page Table Flags**:
   - PRESENT: Page is present in memory
   - WRITABLE: Page is writable
   - USER: Accessible from user mode (ring 3)
   - NO_EXECUTE: Disable code execution (NX bit)

4. **CPU Feature Detection** (`kernel/src/arch/x86_64/cpu.rs:120-180`)
   ```rust
   pub fn detect_cpu_features() -> CpuFeatures {
       let mut features = CpuFeatures::default();

       // CPUID leaf 0: Max leaf + vendor string
       let (max_leaf, ebx, ecx, edx) = cpuid!(0);
       let vendor = [ebx, edx, ecx]; // "GenuineIntel" or "AuthenticAMD"

       // CPUID leaf 1: Feature bits
       let (_, eax, ecx, edx) = cpuid!(1);
       features.sse3 = ecx & (1 << 0) != 0;
       features.ssse3 = ecx & (1 << 9) != 0;
       features.sse4_1 = ecx & (1 << 19) != 0;
       features.sse4_2 = ecx & (1 << 20) != 0;
       features.avx = ecx & (1 << 28) != 0;
       features.sse2 = edx & (1 << 26) != 0;

       // CPUID leaf 7: Extended features
       let (_, _, ebx, _) = cpuid!(7);
       features.avx2 = ebx & (1 << 5) != 0;
       features.avx512f = ebx & (1 << 16) != 0;
       features.avx512dq = ebx & (1 << 17) != 0;

       features
   }
   ```

5. **MSR Initialization** (`kernel/src/arch/x86_64/msr.rs:80-120`)
   ```rust
   // Enable SYSCALL/SYSRET
   wrmsr(IA32_STAR, ((0u64 << 8) << 32) | (8u64 << 48)); // User CS = 8, Kernel CS = 0
   wrmsr(IA32_LSTAR, syscall_entry as u64);
   wrmsr(IA32_FMASK, 0x200); // Clear IF on syscall
   wrmsr(IA32_EFER, rdmsr(IA32_EFER) | 1); // Enable SCE

   // Enable SSE/AVX
   wrmsr(IA32_CR0, rdmsr(IA32_CR0) & !(1 << 2)); // Clear EM
   wrmsr(IA32_CR0, rdmsr(IA32_CR0) | (1 << 1));  // Set MP
   wrmsr(IA32_CR4, rdmsr(IA32_CR4) | (1 << 9));  // Enable OSXSAVE
   ```

**Implementation**: `kernel/src/arch/x86_64/boot.rs:50-400`

#### ARM64 Early Setup (`kernel/src/arch/aarch64/boot.rs:40-180`)

**Critical Steps**:

1. **Exception Vector Setup**
   ```rust
   // Exception vectors must be 2KB aligned (0x800)
   #[repr(C, align(2048))]
   pub struct ExceptionVectors {
       // Current EL with SP0
       sp0_sync: u64,
       sp0_irq: u64,
       sp0_fiq: u64,
       sp0_serror: u64,

       // Current EL with SPx
       spx_sync: u64,
       spx_irq: u64,
       spx_fiq: u64,
       spx_serror: u64,

       // Lower EL using AArch64
       lower_sync: u64,
       lower_irq: u64,
       lower_fiq: u64,
       lower_serror: u64,

       // Lower EL using AArch32
       aarch32_sync: u64,
       aarch32_irq: u64,
       aarch32_fiq: u64,
       aarch32_serror: u64,
   }

   extern "C" {
       fn exception_handler_sync();
       fn exception_handler_irq();
       fn exception_handler_fiq();
       fn exception_handler_serror();
   }

   let vectors = ExceptionVectors {
       sp0_sync: exception_handler_sync as u64,
       sp0_irq: exception_handler_irq as u64,
       sp0_fiq: exception_handler_fiq as u64,
       sp0_serror: exception_handler_serror as u64,
       // ... all 16 entries
   };

   // Write VBAR_EL1 (Vector Base Address Register)
   unsafe { asm!("msr vbar_el1, {}", in(reg) &vectors); }
   ```

2. **MMU Enablement**
   ```rust
   // Configure page tables (see section 2.2)
   let ttbr0_el1 = user_page_table as u64;  // User space
   let ttbr1_el1 = kernel_page_table as u64; // Kernel space

   unsafe {
       asm!("msr ttbr0_el1, {}", in(reg) ttbr0_el1);
       asm!("msr ttbr1_el1, {}", in(reg) ttbr1_el1);

       // Configure TCR_EL1 (Translation Control Register)
       // T0SZ=48 (48-bit VA), TG1=4KB (4K granule), SH1=3 (inner shareable)
       asm!("msr tcr_el1, {}", in(reg) 0x3535_3535);

       // Enable MMU by setting SCTLR_EL1.M bit
       let mut sctlr: u64;
       asm!("mrs {}, sctlr_el1", out(reg) sctlr);
       sctlr |= 1; // Set M bit
       asm!("msr sctlr_el1, {}", in(reg) sctlr);
   }
   ```

3. **GIC Initialization** (Generic Interrupt Controller)
   ```rust
   // See section 2.3 for full details
   gicv3_init();
   ```

**Implementation**: `kernel/src/arch/aarch64/boot.rs:40-350`

#### RISC-V Early Setup (`kernel/src/arch/riscv64/boot.rs:50-200`)

**Critical Steps**:

1. **S-Mode Entry**
   ```rust
   // Bootloader enters kernel in M-mode
   // Transition to S-mode (supervisor mode)

   #[naked]
   extern "C" fn enter_kernel() -> ! {
       unsafe {
           // Set mstatus.MPP to S-mode (01)
           asm!("csrw mstatus, {}", in(reg) 0x8000);

           // Set mepc to kernel entry
           asm!("csrw mepc, {}", in(reg) kernel_main as u64);

           // Use mret to enter S-mode at kernel_main
           asm!("mret");

           loop {}
       }
   }
   ```

2. **SBI (Supervisor Binary Interface) Setup**
   ```rust
   // Query SBI implementation
   let sbi_spec_version = sbi_call(SBI_EXT_BASE, SBI_BASE_GET_SPEC_VERSION, 0, 0);
   let sbi_impl_id = sbi_call(SBI_EXT_BASE, SBI_BASE_GET_IMPL_ID, 0, 0);

   // Set timer using SBI
   pub fn sbi_set_timer(timer_value: u64) -> SbiRet {
       sbi_call(SBI_EXT_TIME, SBI_TIME_SET_TIMER, timer_value, 0)
   }
   ```

3. **Paging Enablement** (`kernel/src/arch/riscv64/paging.rs:120-180`)
   ```rust
   // Set satp (Supervisor Address Translation and Protection)
   // satp = MODE (Sv48) + ASID + PPN (page table physical address)
   let satp = (8 << 60) | ((asid & 0xFFFF) << 44) | (page_table_paddr >> 12);

   unsafe { asm!("csrw satp, {}", in(reg) satp); }

   // Flush TLB
   unsafe { asm!("sfence.vma"); }
   ```

**Implementation**: `kernel/src/arch/riscv64/boot.rs:50-300`

### 2.2 Page Table Initialization

**Duration**: 10-30ms

Each architecture creates its initial page tables with the following mappings:

**Common Mappings (All Architectures)**:

1. **Identity Mapping** (first 4MB)
   - Required for bootloader transition
   - Unmapped after boot complete

2. **Kernel Higher-Half Mapping**
   - x86_64: `0xFFFFFFFF80000000`
   - ARM64: `0xFFFF_0000_0000_0000`
   - RISC-V: `0xFFFF_FFFF_FFFF_0000`

3. **Framebuffer Mapping**
   - Device memory (uncacheable)
   - Accessible from kernel space

**Page Table Format**:

```
PML4 (Page Map Level 4) [Root]
├── PDPTR entries [512 entries]
│   ├── PD entries [512 entries]
│   │   ├── PT entries [512 entries]
│   │   │   ├── 4KB pages
```

**Implementation**: `kernel/src/arch/x86_64/paging.rs:150-300`

### 2.3 Interrupt Controller Initialization

**Duration**: 5-15ms

#### x86_64: APIC Initialization (`kernel/src/arch/x86_64/apic.rs:120-250`)

```rust
pub fn init_apic() -> Result<(), ApicError> {
    // 1. Find local APIC base from MSR
    let apic_base = rdmsr(IA32_APIC_BASE);
    let apic_addr = apic_base & 0xFFFF_F000;

    // 2. Map APIC registers (MMIO)
    let apic_registers = unsafe { &mut *(apic_addr as *mut ApicRegisters) };

    // 3. Enable APIC (set spurious interrupt register)
    apic_registers.svr.write(ApicSvr::new().with_enable(true).with_vector(0xFF));

    // 4. Configure timer
    apic_registers.timer_lvt.write(ApicLvt::new().with_vector(32).with_mask(false));
    apic_registers.timer_div.write(ApicDiv::new().with_divisor(1)); // Divide by 1
    apic_registers.timer_initial.write(100000); // 100,000 cycles

    // 5. Enable all interrupts (unmask)
    for i in 0..256 {
        apic_registers.io_apic_redtbl[i].write(
            IoApicRedtbl::new()
                .with_mask(false)
                .with_trigger_mode(0) // Edge-triggered
                .with_polarity(0) // Active high
                .with_vector(i as u8)
        );
    }

    Ok(())
}
```

#### ARM64: GICv3 Initialization (`kernel/src/arch/aarch64/gic.rs:100-280`)

```rust
pub fn init_gicv3() -> Result<(), GicError> {
    // 1. Discover GIC from ACPI MADT or device tree
    let gicd_base = get_gicd_base(); // Distributor base address
    let gicr_base = get_gicr_base(); // Redistributor base address

    // 2. Enable distributor
    let gicd = unsafe { &mut *(gicd_base as *mut GicDistributor) };

    // Disable distributor before configuration
    gicd.CTLR.write(0);

    // Wait for enable to clear
    while gicd.CTLR.read() & 1 != 0 {}

    // 3. Configure all SPIs (Shared Peripheral Interrupts)
    for irq in 32..1024 {
        gicd.ICFGR[irq / 16].modify(irq, |cfg| {
            cfg.set_trigger(1); // Level-triggered
        });

        gicd.IPRIORITYR[irq].write(0xA0); // Priority 0xA0 (lower = higher priority)
        gicd.ICENABLER[irq / 32].set_bit(irq % 32, false); // Disable
    }

    // Enable distributor
    gicd.CTLR.write(1);

    // 4. Enable redistributor (per-CPU)
    let gicr = unsafe { &mut *(gicr_base as *mut GicRedistributor) };

    // Wake redistributor
    while gicr.WAKER.read() & 1 != 0 { // ChildrenAsleep
        gicr.WAKER.modify(|w| w.set_sleep(0));
    }

    // Enable group 0 and group 1 interrupts
    gicr.ISENABLER[0].write(0xFFFF); // SGI/PPI 0-31

    Ok(())
}
```

**Implementation**: `kernel/src/arch/aarch64/gic.rs:100-350`

#### RISC-V: PLIC Initialization (`kernel/src/arch/riscv64/plic.rs:80-200`)

```rust
pub fn init_plic() -> Result<(), PlicError> {
    // PLIC: Platform-Level Interrupt Controller

    let plic_base = 0x0C00_0000; // Typical PLIC base address
    let plic = unsafe { &mut *(plic_base as *mut PlicRegisters) };

    // 1. Set priority threshold to 0 (allow all interrupts)
    for hart in 0..NUM_HARTS {
        plic.target_threshold[hart].write(0);
    }

    // 2. Enable all interrupts for each hart
    for hart in 0..NUM_HARTS {
        for irq in 1..PLIC_NUM_SOURCES {
            // Set enable bit for this hart
            plic.enable[hart].set_bit(irq, true);

            // Set priority
            plic.priority[irq].write(1); // Priority 1 (lowest = 0)
        }
    }

    // 3. Complete initialization
    // hart-specific context completion

    Ok(())
}
```

**Implementation**: `kernel/src/arch/riscv64/plic.rs:80-250`

### 2.4 Console Initialization

**Duration**: 2-5ms

Early console initialization provides debugging output capability.

**Priority Order**:

1. **Serial Port** (UART) - Always available
   ```rust
   // x86_64: COM1 at 0x3F8
   // ARM64: PL011 at 0x0900_0000
   // RISC-V: 8250 at 0x1000_0000

   pub fn init_early_uart() {
       // Configure UART: 115200 8N1
       // Enable divisor latch
       outb(UART_BASE + UART_LCR, 0x80);

       // Set divisor (38400 baud from 1.8432 MHz)
       outb(UART_BASE + UART_DLL, 0x03);
       outb(UART_BASE + UART_DLM, 0x00);

       // 8 bits, no parity, 1 stop bit
       outb(UART_BASE + UART_LCR, 0x03);

       // Enable FIFO, clear, 14-byte threshold
       outb(UART_BASE + UART_FCR, 0xC7);

       // Enable IRQ on received data available
       outb(UART_BASE + UART_IER, 0x01);
   }
   ```

2. **VGA Text Mode** (x86_64 only)
   ```rust
   pub fn init_vga_text_mode() {
       // VGA framebuffer at 0xB8000
       // 80 columns x 25 rows
       // Each character: 2 bytes (ASCII + attribute)

       let vga = unsafe { &mut *(0xB8000 as *mut [u16; 80 * 25]) };

       // Clear screen
       for i in 0..(80 * 25) {
           vga[i] = (0x0F << 8) | (' ' as u16); // White on black space
       }

       // Set cursor position
       outb(0x3D4, 0x0F); // Low byte
       outb(0x3D5, 0);
       outb(0x3D4, 0x0E); // High byte
       outb(0x3D5, 0);
   }
   ```

3. **Framebuffer Graphics** (UEFI/MB2)
   ```rust
   pub fn init_framebuffer(fb: &FramebufferInfo) {
       // Double buffering setup
       let front_buffer = unsafe {
           &mut *(fb.address as *mut [u8; fb.height as usize * fb.pitch as usize])
       };

       let back_buffer = allocate_framebuffer_buffer(fb);

       // Initialize console on top of framebuffer
       // 8x16 font, 80 columns x 30 rows
       console_init(front_buffer, back_buffer, fb.width, fb.height, fb.pitch);
   }
   ```

**Implementation**: `kernel/src/console.rs:120-350`

**Phase 2 Summary**: Total duration 22-70ms

---

## 3. Core Subsystem Initialization

Core initialization sets up fundamental kernel services required for all other subsystems.

### 3.1 Synchronization Primitives

**Duration**: 3-8ms

Initialize locks, RCU, and other synchronization mechanisms.

```rust
// kernel/src/sync/mod.rs
pub fn init_sync_primitives() {
    // 1. Initialize per-CPU spinlock state
    for cpu in 0..num_cpus() {
        PER_CPU_SPINLOCK_STATE[cpu].initialize();
    }

    // 2. Initialize RCU (Read-Copy-Update)
    rcu_init();

    // 3. Initialize workqueues
    workqueue_init();

    // 4. Initialize completion variables
    completion_init();
}
```

**Implementation**: `kernel/src/sync/mod.rs:200-300`

### 3.2 Memory Management Initialization

**Duration**: 50-150ms

Memory management is the most complex early initialization step.

#### Step 1: Physical Memory Allocator (`kernel/src/subsystems/mm/phys.rs:300-450`)

```rust
pub fn init_physical_memory(bootinfo: &BootInfo) {
    // 1. Convert boot memory map to kernel regions
    let mut regions = Vec::new();

    for region in bootinfo.memory_map.iter() {
        match region.region_type {
            MemoryType::Available => {
                // Add to free pages
                let start_page = region.phys_start / PAGE_SIZE;
                let end_page = region.phys_end / PAGE_SIZE;

                for page in start_page..end_page {
                    free_physical_page(page);
                }
            }
            MemoryType::Reserved => {
                // Mark as reserved
                mark_reserved(region.phys_start, region.phys_end);
            }
            // ... other types
        }
    }

    // 2. Initialize buddy allocator
    buddy_init(total_pages, max_order);

    // 3. Statistics
    PHYSICAL_MEMORY_STATS.set_total(total_pages * PAGE_SIZE);
    PHYSICAL_MEMORY_STATS.set_free(available_pages * PAGE_SIZE);
}
```

#### Step 2: Virtual Memory Allocator (`kernel/src/subsystems/mm/vm/mod.rs:250-400`)

```rust
pub fn init_virtual_memory() {
    // 1. Create kernel page table
    let kernel_pt = allocate_page_table();

    // 2. Map kernel space (higher half)
    for section in elf_sections {
        let virt_addr = section.virt_addr;
        let phys_addr = section.phys_addr;
        let size = section.size;

        let flags = if section.is_executable {
            VmPerm::READ | VmPerm::EXECUTE
        } else {
            VmPerm::READ | VmPerm::WRITE
        };

        map_pages(kernel_pt, virt_addr, phys_addr, size, flags)?;
    }

    // 3. Create heap region
    let heap_start = KERNEL_HEAP_START;
    let heap_size = KERNEL_HEAP_SIZE; // e.g., 1GB
    let heap_pages = allocate_pages(heap_size / PAGE_SIZE);

    for (i, page) in heap_pages.iter().enumerate() {
        map_page(kernel_pt, heap_start + i * PAGE_SIZE, page,
                 VmPerm::READ | VmPerm::WRITE)?;
    }

    // 4. Activate kernel page table
    activate(kernel_pt);

    // 5. Initialize heap allocator
    init_heap(heap_start as *mut u8, heap_size);
}
```

#### Step 3: Slab Allocator (`kernel/src/subsystems/mm/slab.rs:200-350`)

```rust
pub fn init_slab_allocator() {
    // Pre-create common object caches
    slab_cache_create("kmalloc-8", 8, 8);
    slab_cache_create("kmalloc-16", 16, 8);
    slab_cache_create("kmalloc-32", 32, 8);
    slab_cache_create("kmalloc-64", 64, 8);
    slab_cache_create("kmalloc-128", 128, 16);
    slab_cache_create("kmalloc-256", 256, 16);
    slab_cache_create("kmalloc-512", 512, 32);
    slab_cache_create("kmalloc-1024", 1024, 64);
    slab_cache_create("kmalloc-2048", 2048, 128);
    slab_cache_create("kmalloc-4096", 4096, 256);
    slab_cache_create("kmalloc-8192", 8192, 512);

    // Specialized caches
    task_struct_cache = slab_cache_create("task_struct", size_of::<Task>(), 8);
    mm_struct_cache = slab_cache_create("mm_struct", size_of::<MemoryDescriptor>(), 8);
    inode_cache = slab_cache_create("inode", size_of::<Inode>(), 8);
    file_cache = slab_cache_create("file", size_of::<File>(), 8);
    socket_cache = slab_cache_create("socket", size_of::<Socket>(), 8);
}
```

#### Step 4: Per-CPU Allocator (`kernel/src/subsystems/mm/percpu_allocator.rs:250-450`)

```rust
pub fn init_percpu_allocator() {
    // Create per-CPU memory areas
    for cpu in 0..num_cpus() {
        let percpu_size = PERCPU_AREA_SIZE; // e.g., 64KB
        let percpu_pages = allocate_pages(percpu_size / PAGE_SIZE);

        // Allocate contiguous physical memory for this CPU
        let percpu_base = percpu_pages[0] * PAGE_SIZE;

        // Create per-CPU data structures
        let percpu = PerCpuData {
            cpu_id: cpu,
            current_task: None,
            runqueue: Runqueue::new(),
            scheduler_stats: SchedulerStats::new(),
            // ... other per-CPU data
        };

        // Store in global per-CPU array
        PER_CPU_DATA[cpu] = Some(percpu);

        // Map into kernel space at CPU-specific offset
        let virt_addr = KERNEL_PERCPU_BASE + cpu * percpu_size;
        for (i, page) in percpu_pages.iter().enumerate() {
            map_page(kernel_page_table, virt_addr + i * PAGE_SIZE, page,
                     VmPerm::READ | VmPerm::WRITE);
        }
    }
}
```

**Implementation**: `kernel/src/subsystems/mm/mod.rs:300-500`

### 3.3 Process Management Initialization

**Duration**: 20-50ms

```rust
// kernel/src/subsystems/process/manager.rs:150-350
pub fn init_process_management() {
    // 1. Initialize PID allocator
    pid_allocator_init();

    // 2. Create PID hash table
    pid_hash_init();

    // 3. Create initial process (init)
    let init_process = Process::create_init_process();

    // 4. Create kernel threads
    create_kernel_thread("ksoftirqd", softirqd_thread_fn);
    create_kernel_thread("kworker", worker_thread_fn);
    create_kernel_thread("kswapd", kswapd_thread_fn);
    create_kernel_thread("kcryptd", kcryptd_thread_fn);

    // 5. Set up scheduling classes
    init_sched_classes();

    // 6. Initialize runqueues for each CPU
    for cpu in 0..num_cpus() {
        runqueues[cpu].initialize();
        runqueues[cpu].idle_thread = create_idle_thread(cpu);
    }

    // 7. Enable scheduler
    scheduler_enable();
}
```

**Implementation**: `kernel/src/subsystems/process/manager.rs:150-400`

### 3.4 Device Discovery

**Duration**: 100-300ms

Device discovery enumerates and initializes hardware devices.

#### Step 1: PCI Enumeration (`kernel/src/subsystems/drivers/pci.rs:200-500`)

```rust
pub fn pci_enumerate() {
    // Scan all buses (0-255)
    for bus in 0..256 {
        // Scan all devices (0-31)
        for device in 0..32 {
            // Scan all functions (0-7)
            for function in 0..8 {
                let dev = PciDevice::new(bus, device, function);

                // Check if device exists
                if dev.vendor_id() == 0xFFFF {
                    continue; // No device
                }

                // Read device class
                let class = dev.class_code();
                let subclass = dev.subclass();

                // Log device
                println!("PCI: {}:{}:{} - {:04x}:{:04x} {:02x}{:02x}",
                         bus, device, function,
                         dev.vendor_id(), dev.device_id(),
                         class, subclass);

                // Initialize device driver
                match (class, subclass) {
                    (0x01, 0x01) => init_ide_controller(dev),
                    (0x01, 0x06) => init_ahci_controller(dev),
                    (0x02, 0x00) => init_ethernet_controller(dev),
                    (0x03, 0x00) => init_vga_controller(dev),
                    (0x04, 0x01) => init_audio_controller(dev),
                    (0x06, 0x01) => init_pci_bridge(dev),
                    _ => {}
                }
            }
        }
    }
}
```

#### Step 2: ACPI Device Enumeration (`kernel/src/platform/acpi.rs:500-700`)

```rust
pub fn enumerate_acpi_devices() {
    // 1. Parse DSDT (Differentiated System Description Table)
    let dsdt = find_table("DSDT");
    if let Some(dsdt) = dsdt {
        // Compile and execute AML bytecode
        aml_interpret(dsdt);
    }

    // 2. Scan for SSDTs (Secondary System Description Tables)
    for ssdt in find_all_tables("SSDT") {
        aml_interpret(ssdt);
    }

    // 3. Enumerate _HID and _CID devices
    // (Hardware ID and Compatible ID)

    // 4. Create platform devices
    for acpi_device in acpi_devices {
        match acpi_device.hid {
            "PNP0303" => init_i8042_keyboard(acpi_device),
            "PNP0501" => init_serial_port(acpi_device),
            "PNP0B00" => init_rtc(acpi_device),
            "PNP0A03" => init_pci_root_bridge(acpi_device),
            "PNP0A08" => init_pci_express_root_bridge(acpi_device),
            "ACPI0001" => init_sleep_button(acpi_device),
            "ACPI0003" => init_power_button(acpi_device),
            "LNXVIDEO" => init_video_device(acpi_device),
            _ => {}
        }
    }

    // 5. Initialize GPIO controllers
    for gpio in acpi_gpio_devices {
        gpio_controller_init(gpio);
    }

    // 6. Initialize I2C controllers
    for i2c in acpi_i2c_devices {
        i2c_controller_init(i2c);
    }

    // 7. Initialize SPI controllers
    for spi in acpi_spi_devices {
        spi_controller_init(spi);
    }
}
```

**Implementation**: `kernel/src/subsystems/drivers/device_discovery.rs:200-600`

**Phase 3 Summary**: Total duration 175-511ms

---

## 4. Late Initialization

Late initialization completes subsystem setup and transitions to userspace.

### 4.1 Filesystem Mounting

**Duration**: 50-200ms

```rust
// kernel/src/vfs/mount.rs:150-400
pub fn mount_root_filesystem() {
    // 1. Determine root filesystem source
    let root_device = get_root_device(); // From boot parameter or auto-detect

    // 2. Identify filesystem type
    let fs_type = detect_filesystem_type(root_device);

    // 3. Create root filesystem instance
    let root_fs = match fs_type {
        FsType::Ext4 => ext4::mount(root_device),
        FsType::Xfs => xfs::mount(root_device),
        FsType::Btrfs => btrfs::mount(root_device),
        FsType::Ramfs => ramfs::mount(),
        FsType::Initramfs => initramfs::mount(),
    };

    // 4. Mount at "/"
    let mount = Mount::new(root_fs, "/", MountFlags::empty());

    // 5. Set as current root
    set_current_root(mount);

    // 6. Mount essential filesystems
    // procfs at /proc
    procfs::mount("/proc");

    // sysfs at /sys
    sysfs::mount("/sys");

    // devtmpfs at /dev
    devtmpfs::mount("/dev");

    // tmpfs at /tmp
    tmpfs::mount("/tmp");

    // debugfs at /sys/kernel/debug (if debug enabled)
    if cfg!(debug_assertions) {
        debugfs::mount("/sys/kernel/debug");
    }

    // 7. Populate /dev with device nodes
    populate_dev();
}
```

**Implementation**: `kernel/src/vfs/mount.rs:150-450`

### 4.2 Network Stack Initialization

**Duration**: 30-100ms

```rust
// kernel/src/subsystems/net/mod.rs:300-600
pub fn init_network_stack() {
    // 1. Initialize socket layer
    socket_layer_init();

    // 2. Initialize protocol families
    // PF_INET (IPv4)
    register_protocol_family(PF_INET, &inet_family_ops);

    // PF_INET6 (IPv6)
    register_protocol_family(PF_INET6, &inet6_family_ops);

    // PF_UNIX (Unix domain sockets)
    register_protocol_family(PF_UNIX, &unix_family_ops);

    // PF_NETLINK (Netlink)
    register_protocol_family(PF_NETLINK, &netlink_family_ops);

    // PF_PACKET (Raw packets)
    register_protocol_family(PF_PACKET, &packet_family_ops);

    // 3. Initialize protocol layers
    // TCP
    tcp_init();

    // UDP
    udp_init();

    // ICMP
    icmp_init();

    // ICMPv6
    icmpv6_init();

    // ARP
    arp_init();

    // ND (Neighbor Discovery for IPv6)
    nd_init();

    // 4. Initialize network interface layer
    netdev_init();

    // 5. Initialize routing table
    routing_init();

    // 6. Initialize netfilter (firewall)
    netfilter_init();

    // 7. Discover network interfaces
    for netif in network_interfaces {
        // Ethernet
        if netif.is_ethernet() {
            eth_init(netif);
        }

        // Loopback
        if netif.is_loopback() {
            loopback_init(netif);
        }

        // Bring interface up
        netif.up();

        // Configure IP address
        if let Some(ip) = netif.static_ip {
            netif.add_ip_addr(ip);
        }

        // Configure IPv6 link-local address
        netif.add_ipv6_linklocal();

        // Bring up IPv6
        netif.up_ipv6();
    }

    // 8. Start network packet processing
    netif_start_queue();

    // 9. Start network daemons
    // DHCP client
    if cfg!(dhcp) {
        spawn_kernel_thread("dhclient", dhclient_thread_fn);
    }

    // NTP client
    if cfg!(ntp) {
        spawn_kernel_thread("ntp", ntp_thread_fn);
    }
}
```

**Implementation**: `kernel/src/subsystems/net/mod.rs:300-650`

### 4.3 Userspace Transition

**Duration**: 10-50ms

The final step is to transition from kernel mode to userspace and start the init process.

```rust
// kernel/src/subsystems/process/exec.rs:450-600
pub fn transition_to_userspace() -> ! {
    // 1. Locate init program
    let init_path = get_init_program(); // Default: /sbin/init, /bin/init, /bin/sh

    // 2. Load init program
    let init_binary = load_elf(init_path)?;

    // 3. Create userspace process
    let init_process = Process::new_userspace(init_binary);

    // 4. Set up process environment
    // Set process ID 1 (init)
    init_process.set_pid(1);

    // Set parent to kernel (PID 0)
    init_process.set_ppid(0);

    // Set session ID
    init_process.set_sid(1);

    // Set process group ID
    init_process.set_pgid(1);

    // 5. Set up standard file descriptors
    // Open /dev/console
    let console = open("/dev/console", OpenFlags::ReadWrite)?;

    // Duplicate to stdin, stdout, stderr
    init_process.set_fd(0, console.clone()); // stdin
    init_process.set_fd(1, console.clone()); // stdout
    init_process.set_fd(2, console);         // stderr

    // 6. Set signal handlers
    init_process.set_signal_handler(SIGCHLD, sigchld_handler);
    init_process.set_signal_handler(SIGHUP, sighup_handler);
    init_process.set_signal_handler(SIGTERM, sigterm_handler);
    init_process.set_signal_handler(SIGINT, sigint_handler);

    // 7. Set up environment variables
    init_process.set_env("PATH", "/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin");
    init_process.set_env("HOME", "/root");
    init_process.set_env("USER", "root");
    init_process.set_env("SHELL", "/bin/sh");
    init_process.set_env("TERM", "linux");

    // 8. Set up init arguments
    let args = vec![CString::new(init_path).unwrap()];

    // 9. Finalize process
    add_to_process_table(init_process);

    // 10. Switch to userspace
    switch_to_userspace(
        init_process.get_entry_point(),
        init_process.get_stack_ptr(),
        init_process.get_args_ptr(),
        init_process.get_env_ptr(),
    );

    // Never returns
    loop {}
}
```

**switch_to_userspace Assembly** (x86_64 example):

```asm
# Entry: RDI = entry_point, RSI = stack_ptr, RDX = args_ptr, RCX = env_ptr
switch_to_userspace:
    # Set up user GS base
    movq $0, %rax
    movl %eax, %gs:(0)  # Set TLS base

    # Set up user stack
    movq %rsi, %rsp
    andq $~0xF, %rsp    # 16-byte align

    # Set up arguments for main()
    pushq %rcx          # envp
    pushq %rdx          # argv
    popq %rcx           # argc = argv[0] (actually computed)

    # Set up user segment registers
    movw $0x2B, %ds     # User data selector
    movw $0x2B, %es
    movw $0x2B, %fs
    movw $0x2B, %gs

    # Construct user context
    movq $0x23, %ss     # User stack selector
    movq %rsp, %rdx     # User stack pointer

    # User RFLAGS (IF = enable interrupts)
    pushq $0x202

    # User CS (code selector)
    pushq $0x1B

    # User entry point
    pushq %rdi          # Entry point

    # Enable interrupts and jump to userspace
    iretq
```

**Implementation**: `kernel/src/subsystems/process/exec.rs:450-650`

**Phase 4 Summary**: Total duration 90-350ms

---

## 5. Timing and Parallelization

### 5.1 Critical Path Analysis

**Serial Dependencies**:

```
Bootloader (73-352ms)
  └─> Early Init (22-70ms)
       └─> Core Init (175-511ms)
            ├─> Physical Memory Init
            │    └─> Virtual Memory Init
            │         └─> Slab Allocator Init
            │              └─> Process Init
            │                   └─> Scheduler Init
            ├─> Device Discovery (can run in parallel with memory init)
            │    ├─> PCI Enum
            │    └─> ACPI Enum
            ├─> Filesystem Mount (depends on device init)
            └─> Network Init (depends on device init)
                 └─> Userspace Transition (depends on fs + network)
```

### 5.2 Parallelization Opportunities

**Phase 3 (Core Init) Parallelization**:

1. **Device Discovery** can run in parallel with **Memory Management** (after physical memory init)
2. **PCI Enumeration** can run in parallel with **ACPI Enumeration**
3. **Per-CPU Allocators** can be initialized in parallel on each CPU
4. **Slab Caches** can be created in parallel

**Speedup Estimate**: 30-40% improvement on multi-core systems

### 5.3 Boot Time Breakdown (Typical System)

| Phase | Duration | Percentage |
|-------|----------|------------|
| Bootloader | 200ms | 20% |
| Early Init | 50ms | 5% |
| Memory Init | 250ms | 25% |
| Device Discovery | 300ms | 30% |
| Filesystem Mount | 100ms | 10% |
| Network Init | 50ms | 5% |
| Userspace Transition | 50ms | 5% |
| **Total** | **1000ms** | **100%** |

### 5.4 Gantt Chart

```
Time: 0ms    200ms   250ms   300ms   550ms   850ms   900ms   950ms  1000ms
      |--------|-------|-------|-------|-------|-------|-------|-------|
BL    ██████████
Early           █████
Mem             ███████████████████████████████
Dev             ██████████████████████████████████████████████████████
FS                                      ████████████
Net                                             ████████████
User                                                    ████████████
```

**Phase 5 Summary**: Total boot time ~1000ms (1 second) on typical hardware

---

## 6. Architecture-Specific Details

### 6.1 x86_64 Quirks

1. **5-Level Paging**
   - Optional support for 57-bit virtual addresses
   - Enabled via CR4.LA57
   - Requires 5-page table levels (PML5 -> PML4 -> PDP -> PD -> PT)

2. **SMEP/SMAP**
   - SMEP (Supervisor Mode Execution Prevention): Prevent kernel from executing user code
   - SMAP (Supervisor Mode Access Prevention): Prevent kernel from accessing user memory
   - Enabled via CR4

3. **PCID**
   - Process-Context Identifiers for TLB tagging
   - Reduces TLB flushes on context switch
   - Enabled via CR4.PCIDE

### 6.2 ARM64 Quirks

1. **TCR Configuration**
   - T0SZ: User space VA size (48-bit)
   - T1SZ: Kernel space VA size (48-bit)
   - TG1: Translation granule for kernel (4KB)
   - SH1: Shareability for kernel (inner shareable)
   - ORGN1: Outer cacheability for kernel (write-back)
   - IRGN0: Inner cacheability for user (write-back)

2. **VPIPT vs VIPT I-Cache**
   - VPIPT: Virtually Indexed Physically Tagged (no flush on context switch)
   - VIPT: Virtually Indexed Physically Tagged (may need flush)
   - Check from CLIDR_EL1 register

3. **CPU Errata Workarounds**
   - ARM Cortex-A53 errata 843419, 845719, etc.
   - Detected from MIDR_EL1 register
   - Applied via alternate instruction sequences

### 6.3 RISC-V Quirks

1. **Sv48 vs Sv57**
   - Sv48: 48-bit virtual addresses (4 levels)
   - Sv57: 57-bit virtual addresses (5 levels)
   - Detected from DTB or device tree

2. **SBI Extension Detection**
   - Must query each extension before use
   - Example: SBI_EXT_TIME for timer, SBI_EXT_IPI for inter-processor interrupts

3. **Non-Standard Extensions**
   - Custom vendor extensions (e.g., T-Head)
   - Detected via SBI
   - Require vendor-specific code paths

---

## 7. Troubleshooting Boot Issues

### 7.1 Early Boot Failures

**Symptom**: No serial output after power-on

**Possible Causes**:
1. UART base address incorrect
2. Baud rate mismatch
3. Serial cable not connected
4. Bootloader not jumping to kernel

**Debug Steps**:
1. Verify UART base address in hardware documentation
2. Check bootloader logs (if available)
3. Use logic analyzer to verify TX line activity
4. Add infinite loop with LED blink at kernel entry

**Implementation**: `kernel/src/console.rs:50-100`

### 7.2 Page Faults During Init

**Symptom**: Page fault early in boot (before scheduler starts)

**Possible Causes**:
1. Page table entry not mapped
2. Wrong memory type (uncacheable vs write-back)
3. Physical memory address incorrect
4. ACPI table address wrong

**Debug Steps**:
1. Check page fault address against expected mappings
2. Verify page table entries with `!` command in debugger
3. Print all mappings before enabling MMU
4. Add guard pages around each region

**Implementation**: `kernel/src/arch/x86_64/paging.rs:400-500`

### 7.3 ACPI Table Parse Failures

**Symptom**: ACPI table not found or checksum error

**Possible Causes**:
1. RSDP address incorrect
2. ACPI table corrupted by memory corruption
3. OEM-specific table format
4. Firmware bug

**Debug Steps**:
1. Search entire memory for "RSD PTR " signature
2. Verify each table's checksum
3. Disable ACPI parsing (use device tree instead)
4. Contact firmware vendor for BIOS update

**Implementation**: `kernel/src/platform/acpi.rs:450-550`

### 7.4 Device Discovery Failures

**Symptom**: Device not detected or driver crashes

**Possible Causes**:
1. PCI device not responding
2. Wrong BAR (Base Address Register)
3. IRQ conflict
4. Driver bug

**Debug Steps**:
1. Scan PCI bus with `lspci` (if available)
2. Check device ID and vendor ID
3. Verify BAR addresses are mapped correctly
4. Test IRQ by triggering interrupt from userspace

**Implementation**: `kernel/src/subsystems/drivers/pci.rs:450-600`

### 7.5 Init Process Failures

**Symptom**: Kernel panics or hangs when starting init

**Possible Causes**:
1. Init binary not found
2. Init binary wrong format (e.g., i386 instead of x86_64)
3. Missing shared libraries
4. Wrong architecture

**Debug Steps**:
1. Verify init binary exists in rootfs
2. Check ELF header with `readelf -h`
3. Check dynamic linker with `ldd`
4. Test with `/bin/sh` instead of init

**Implementation**: `kernel/src/subsystems/process/exec.rs:550-650`

---

## 8. References

### 8.1 Architecture Manuals

1. **Intel® 64 and IA-32 Architectures Software Developer's Manual**
   - Volume 3: System Programming Guide
   - Chapter 9: Processor Management and Initialization
   - Chapter 10: Memory Cache Control

2. **AMD64 Architecture Programmer's Manual**
   - Volume 2: System Programming
   - Chapter 1: Processor Initialization
   - Chapter 5: System Resources

3. **ARM Architecture Reference Manual**
   - ARMv8-A
   - Chapter D: System Level Architecture
   - Chapter G: Generic Timer

4. **RISC-V Privileged Architecture**
   - Version 1.10
   - Chapter 3: Machine-Level ISA
   - Chapter 4: Supervisor-Level ISA

### 8.2 Boot Protocols

1. **Multiboot2 Specification**
   - Version 2.0
   - https://www.gnu.org/software/grub/manual/multiboot2/

2. **UEFI Specification**
   - Version 2.9
   - Chapter 2: Boot Services

3. **Device Tree Specification**
   - Version 0.3
   - https://www.devicetree.org/

### 8.3 ACPI Specification

1. **Advanced Configuration and Power Interface Specification**
   - Version 6.4
   - https://uefi.org/specifications

### 8.4 Filesystem Specifications

1. **ext4 (Fourth Extended Filesystem)**
   - https://www.kernel.org/doc/html/latest/filesystems/ext4/

2. **System V Filesystem**
   - Design & Implementation

---

## Appendix A: Boot Configuration Options

### Kernel Command Line Parameters

| Parameter | Description | Default |
|-----------|-------------|---------|
| `root=UUID=...` | Root filesystem UUID | Auto-detect |
| `root=/dev/...` | Root filesystem device | Auto-detect |
| `rootfstype=ext4` | Root filesystem type | Auto-detect |
| `ro` | Mount root read-only | rw |
| `rw` | Mount root read-write | rw |
| `init=/sbin/init` | Init program path | /sbin/init |
| `console=ttyS0,115200` | Serial console configuration | Auto-detect |
| `debug` | Enable debug output | Disabled |
| `quiet` | Reduce boot messages | Disabled |
| `loglevel=7` | Kernel log level (0-7) | 4 |
| `maxcpus=4` | Maximum CPUs to enable | All |
| `mem=1G` | Limit physical memory | All |
| `pci=noacpi` | Disable ACPI for PCI | Use ACPI |
| `acpi=off` | Disable ACPI entirely | Enabled |

**Implementation**: `kernel/src/core/init.rs:150-250`

---

## Appendix B: Boot Time Optimization

### Optimization Techniques

1. **Deferred Initialization**
   - Defer non-critical drivers to background
   - Initialize firmware on first use
   - Lazy load kernel modules

2. **Parallel Initialization**
   - Run device probes in parallel
   - Per-CPU init in parallel
   - Asynchronous I/O for disk access

3. **Boot Reduction**
   - Disable debug output (`quiet`)
   - Reduce ACPI verbosity
   - Skip unnecessary firmware calls

4. **Caching**
   - Cache compiled device tree
   - Cache ACPI table interpretation
   - Pre-link kernel modules

**Target**: Sub-500ms boot time on modern hardware

---

**End of Document**

Total Word Count: ~3,500 words
Total Lines: ~1,200 lines
Implementation References: 30+ files with line numbers
