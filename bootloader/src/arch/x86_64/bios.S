/*
 * NOS BIOS Bootloader - x86_64 Assembly Entry Points
 *
 * This file contains the assembly-level entry points for the BIOS bootloader,
 * including real mode initialization, protected mode transition, and
 * long mode setup for x86_64 systems.
 */

.section .text
.code16
.globl _start
.type _start, @function

/* BIOS Bootloader Entry Point */
_start:
    /* Save BIOS state */
    cli                     /* Disable interrupts */
    cld                     /* Clear direction flag */

    /* Save initial segment registers */
    mov %cs, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss

    /* Save stack pointer */
    mov %sp, saved_sp
    mov %ss, saved_ss

    /* Clear BSS */
    call clear_bss

    /* Check CPU features */
    call check_cpuid
    test %eax, %eax
    jz no_cpuid

    /* Check for long mode support */
    call check_long_mode
    test %eax, %eax
    jz no_long_mode

    /* Detect memory */
    call detect_memory

    /* Enable A20 gate */
    call enable_a20
    test %eax, %eax
    jz a20_failed

    /* Setup protected mode */
    call setup_protected_mode

    /* Jump to protected mode */
    ljmp $PROT_MODE_CS, $protected_mode_entry

no_cpuid:
    /* CPUID not supported - halt */
    mov $ERROR_NO_CPUID, %eax
    jmp bootloader_error

no_long_mode:
    /* Long mode not supported - halt */
    mov $ERROR_NO_LONG_MODE, %eax
    jmp bootloader_error

a20_failed:
    /* A20 gate failed - halt */
    mov $ERROR_A20_FAILED, %eax
    jmp bootloader_error

/* Clear BSS section */
clear_bss:
    mov $bss_start, %edi
    mov $bss_end, %ecx
    sub %edi, %ecx
    xor %eax, %eax
    rep stosb
    ret

/* Check if CPUID is supported */
check_cpuid:
    pushfl                  /* Save EFLAGS */
    pushfl                  /* Copy EFLAGS */
    mov (%esp), %eax
    xor $0x00200000, %eax   /* Flip ID bit */
    mov %eax, 4(%esp)
    popfl                   /* Restore modified EFLAGS */
    pushfl                  /* Save EFLAGS again */
    pop %ebx
    popfl                   /* Restore original EFLAGS */
    cmp %eax, %ebx
    sete %al                /* Set AL if ID bit didn't change */
    movzbl %al, %eax
    ret

/* Check for long mode support */
check_long_mode:
    /* Get highest extended function */
    mov $0x80000000, %eax
    cpuid
    cmp $0x80000001, %eax
    jb no_long_mode_support

    /* Get extended CPU features */
    mov $0x80000001, %eax
    cpuid
    test $(1 << 29), %edx   /* Test LM bit */
    jz no_long_mode_support
    test $(1 << 26), %edx   /* Test SSE2 bit */
    jz no_long_mode_support

    mov $1, %eax            /* Long mode supported */
    ret

no_long_mode_support:
    xor %eax, %eax
    ret

/* Detect system memory */
detect_memory:
    /* Call Rust memory detection code */
    push %ebp
    mov %esp, %ebp
    call rust_detect_memory
    pop %ebp
    ret

/* Enable A20 gate */
enable_a20:
    /* Try keyboard controller method */
    call enable_a20_keyboard
    test %eax, %eax
    jnz a20_enabled

    /* Try fast A20 gate */
    call enable_a20_fast
    test %eax, %eax
    jnz a20_enabled

    /* Try BIOS method */
    call enable_a20_bios
    test %eax, %eax
    jnz a20_enabled

    xor %eax, %eax          /* All methods failed */
    ret

a20_enabled:
    mov $1, %eax            /* Success */
    ret

/* Enable A20 via keyboard controller */
enable_a20_keyboard:
    /* Wait for input buffer empty */
    mov $0x64, %dx
.wait_input:
    in %dx, %al
    test $0x02, %al
    jnz .wait_input

    /* Write command to enable A20 */
    mov $0xD1, %al
    out %al, %dx

    /* Wait for input buffer empty again */
.wait_input2:
    in %dx, %al
    test $0x02, %al
    jnz .wait_input2

    /* Write data */
    mov $0xDF, %al
    mov $0x60, %dx
    out %al, %dx

    mov $1, %eax
    ret

/* Enable A20 via fast gate */
enable_a20_fast:
    mov $0x92, %dx
    in %dx, %al
    or $0x02, %al
    out %al, %dx
    mov $1, %eax
    ret

/* Enable A20 via BIOS */
enable_a20_bios:
    mov $0x2403, %ax        /* A20 gate support query */
    xor %bx, %bx
    xor %cx, %cx
    xor %dx, %dx
    int $0x15
    cmp $0x00, %ah
    jne .failed

    mov $0x2401, %ax        /* A20 gate enable */
    int $0x15
    cmp $0x00, %ah
    jne .failed

    mov $1, %eax
    ret
.failed:
    xor %eax, %eax
    ret

/* Setup Protected Mode GDT */
setup_protected_mode:
    lgdt gdtr               /* Load GDT */
    mov %cr0, %eax
    or $0x01, %eax          /* Set PE bit */
    mov %eax, %cr0          /* Enable protected mode */
    ret

/* Protected Mode Entry Point */
.section .text
.code32
.globl protected_mode_entry
.type protected_mode_entry, @function

protected_mode_entry:
    /* Set up segment registers */
    mov $PROT_MODE_DS, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss

    /* Set up new stack */
    mov $boot_stack_top, %esp

    /* Save multiboot information */
    push %ebx               /* Multiboot magic */
    push %eax               /* Multiboot info */

    /* Call Rust code to continue initialization */
    call rust_protected_mode_entry

    /* Should not return */
    cli
    hlt
    jmp .

/* Long Mode Entry Point */
.section .text
.code64
.globl long_mode_entry
.type long_mode_entry, @function

long_mode_entry:
    /* Set up 64-bit segment registers */
    mov $LONG_MODE_DS, %ax
    mov %ax, %ds
    mov %ax, %es
    mov %ax, %fs
    mov %ax, %gs
    mov %ax, %ss

    /* Set up stack */
    mov $long_mode_stack_top, %rsp

    /* Call Rust main function */
    call rust_main

    /* Should not return */
    cli
    hlt
    jmp .

/* Global Descriptor Table */
.section .rodata
.align 16

gdt:
    /* Null descriptor */
    .quad 0

    /* Code segment - 32-bit */
    .word 0xFFFF             /* Limit 15:0 */
    .word 0x0000             /* Base 15:0 */
    .byte 0x00               /* Base 23:16 */
    .byte 0x9A               /* Present, DPL=0, Code, Execute/Read */
    .byte 0xCF               /* Limit 19:16, 4KB gran, 32-bit */
    .byte 0x00               /* Base 31:24 */

    /* Data segment - 32-bit */
    .word 0xFFFF             /* Limit 15:0 */
    .word 0x0000             /* Base 15:0 */
    .byte 0x00               /* Base 23:16 */
    .byte 0x92               /* Present, DPL=0, Data, Read/Write */
    .byte 0xCF               /* Limit 19:16, 4KB gran, 32-bit */
    .byte 0x00               /* Base 31:24 */

    /* Code segment - 64-bit */
    .word 0xFFFF             /* Limit 15:0 */
    .word 0x0000             /* Base 15:0 */
    .byte 0x00               /* Base 23:16 */
    .byte 0x9A               /* Present, DPL=0, Code, Execute/Read */
    .byte 0xAF               /* Limit 19:16, 4KB gran, 64-bit */
    .byte 0x00               /* Base 31:24 */

    /* Data segment - 64-bit */
    .word 0xFFFF             /* Limit 15:0 */
    .word 0x0000             /* Base 15:0 */
    .byte 0x00               /* Base 23:16 */
    .byte 0x92               /* Present, DPL=0, Data, Read/Write */
    .byte 0xCF               /* Limit 19:16, 4KB gran, 64-bit */
    .byte 0x00               /* Base 31:24 */

gdt_end:

/* GDT Pointer */
gdtr:
    .word gdt_end - gdt - 1  /* GDT limit */
    .long gdt               /* GDT base */

/* Segment selector constants */
.equ PROT_MODE_CS, 0x08    /* Code segment selector */
.equ PROT_MODE_DS, 0x10    /* Data segment selector */
.equ LONG_MODE_CS, 0x18    /* 64-bit code segment */
.equ LONG_MODE_DS, 0x20    /* 64-bit data segment */

/* Error constants */
.equ ERROR_NO_CPUID, 0x01
.equ ERROR_NO_LONG_MODE, 0x02
.equ ERROR_A20_FAILED, 0x03

/* Stack areas */
.section .bss
.align 16

boot_stack:
    .skip 0x4000            /* 16KB boot stack */
boot_stack_top:

long_mode_stack:
    .skip 0x8000            /* 32KB long mode stack */
long_mode_stack_top:

/* Saved BIOS state */
saved_sp: .long 0
saved_ss: .long 0

/* External Rust function declarations */
.extern rust_detect_memory
.extern rust_protected_mode_entry
.extern rust_main

/* Error handler */
bootloader_error:
    /* Print error code */
    push %eax
    mov $ERROR_MSG, %si
    call bios_print_string
    pop %eax
    call bios_print_hex

    /* Halt system */
.halt:
    cli
    hlt
    jmp .halt

/* BIOS print string function */
bios_print_string:
    push %si
    push %ax
.loop:
    mov (%si), %al
    test %al, %al
    jz .done
    mov $0x0E, %ah
    int $0x10
    inc %si
    jmp .loop
.done:
    pop %ax
    pop %si
    ret

/* BIOS print hex function */
bios_print_hex:
    push %eax
    push %ebx
    push %ecx
    push %edx
    push %si

    mov $HEX_CHARS, %si
    mov %eax, %ebx
    mov $8, %ecx             /* 8 hex digits */

.hex_loop:
    rol $4, %ebx
    mov %bl, %al
    and $0x0F, %al
    xlatb
    mov $0x0E, %ah
    int $0x10
    loop .hex_loop

    pop %si
    pop %edx
    pop %ecx
    pop %ebx
    pop %eax
    ret

/* Data */
.section .rodata

ERROR_MSG:
    .asciz "BIOS Bootloader Error: 0x"

HEX_CHARS:
    .asciz "0123456789ABCDEF"

/* Multiboot2 header */
.section .multiboot
.align 8

multiboot2_header:
    .long 0x36d76289        /* Magic number */
    .long 0                 /* Architecture 0 = i386 */
    .long multiboot2_header_end - multiboot2_header /* Header length */
    .long -(0x36d76289 + 0 + (multiboot2_header_end - multiboot2_header)) /* Checksum */

    /* End tag */
    .short 0                /* Type */
    .short 0                /* Flags */
    .long 8                 /* Size */

multiboot2_header_end:

/* PE/COFF header for UEFI compatibility */
.section .pe_header
.align 8

pe_header:
    /* DOS header */
    .long 0x5A4D0000        /* e_magic, e_lfanew */
    .long 0x00900000        /* e_lfanew */

    /* PE signature */
    .long 0x00004550        /* PE signature */

    /* COFF header */
    .short 0x8664           /* Machine (x64) */
    .short 1                /* Number of sections */
    .long 0                 /* Timestamp */
    .long 0                 /* Symbol table pointer */
    .long 0                 /* Number of symbols */
    .short 0                /* Optional header size */
    .short 0x202            /* Characteristics */

    /* Optional header */
    .short 0x20B            /* Magic (PE32+) */
    .byte 1                 /* Major linker version */
    .byte 0                 /* Minor linker version */
    .long 0                 /* Size of code */
    .long 0                 /* Size of initialized data */
    .long 0                 /* Size of uninitialized data */
    .long _start            /* Entry point RVA */
    .long 0                 /* Base of code */

    /* Additional PE headers would go here for full UEFI support */

/* Bootloader information */
.section .boot_info
.align 8

boot_info:
    .quad 0                 /* Multiboot magic */
    .quad 0                 /* Multiboot info structure */
    .quad 0                 /* Memory map pointer */
    .quad 0                 /* Framebuffer info */

/* Boot parameters that can be modified by the boot menu */
.section .boot_params
.align 8

boot_cmdline:
    .skip 256               /* Kernel command line */
boot_cmdline_len: .long 0

kernel_image_path:
    .skip 256               /* Path to kernel image */
kernel_image_path_len: .long 0

boot_timeout: .long 5       /* Default timeout in seconds */
boot_flags: .long 0         /* Boot flags */

/* Memory map area */
.section .memory_map
.align 16

memory_map_buffer:
    .skip 4096              /* Buffer for E820 memory map */
memory_map_entries: .long 0
memory_map_size: .long 0

/* Framebuffer information */
.section .framebuffer_info
.align 16

framebuffer_info:
    .quad 0                 /* Framebuffer address */
    .long 0                 /* Framebuffer width */
    .long 0                 /* Framebuffer height */
    .long 0                 /* Bytes per pixel */
    .long 0                 /* Pitch */

/* ACPI information */
.section .acpi_info
.align 8

acpi_rsdp: .quad 0          /* ACPI RSDP address */
acpi_rsdt: .quad 0          /* ACPI RSDT address */
acpi_xsdt: .quad 0          /* ACPI XSDT address */

/* BIOS information */
.section .bios_info
.align 8

bios_version: .long 0
bios_segment: .long 0
bios_signature: .long 0

/* System information */
.section .system_info
.align 8

cpu_vendor: .skip 16        /* CPU vendor string */
cpu_features: .quad 0       /* CPU feature flags */
system_memory: .quad 0      /* Total system memory */
available_memory: .quad 0   /* Available memory */

/* Debug information */
.section .debug_info
.align 8

debug_enabled: .long 1
debug_buffer:
    .skip 4096              /* Debug output buffer */
debug_buffer_pos: .long 0