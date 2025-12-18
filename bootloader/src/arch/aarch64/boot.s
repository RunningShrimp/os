// AArch64 bootloader startup code
.section .text.boot
.global _start
.align 3

_start:
    // Disable interrupts (all exception levels)
    msr daifset, #0xf
    
    // Set up stack (16KB at known location)
    mov sp, 0x200000
    
    // Zero BSS section (x0=start, x1=end, x2=value)
    adrp x0, __bss_start
    adrp x1, __bss_end
    add x0, x0, :lo12:__bss_start
    add x1, x1, :lo12:__bss_end
    cmp x0, x1
    bge .bss_done
    
    mov x2, xzr
.bss_loop:
    str x2, [x0], #8
    cmp x0, x1
    blt .bss_loop
.bss_done:
    
    // Enable FPU if needed (optional)
    mrs x0, cpacr_el1
    orr x0, x0, #(3 << 20)
    msr cpacr_el1, x0
    
    // Memory barrier
    dsb sy
    isb
    
    // Call Rust entry point
    adrp x0, boot_main
    add x0, x0, :lo12:boot_main
    blr x0
    
    // Halt if return
    b .

    bl boot_main
    
    // If boot_main returns (shouldn't happen), halt
    msr daifset, #0xf
    wfi
    b .

// Boot stack - 16KB
.section .bss
.align 4096
boot_stack:
    .space 4096 * 4
.global boot_stack_top
boot_stack_top:
