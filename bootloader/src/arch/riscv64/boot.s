// RISC-V 64 bootloader startup code
.section .text.boot
.global _start
.align 2

_start:
    // Disable interrupts (MIE in mstatus)
    csrci mstatus, 0x8
    
    // Set up stack (16KB at known location)
    li sp, 0x200000
    
    // Zero BSS section
    la x1, __bss_start
    la x2, __bss_end
    bge x1, x2, .bss_done
    
    xor x3, x3, x3
.bss_loop:
    sd x3, (x1)
    addi x1, x1, 8
    blt x1, x2, .bss_loop
.bss_done:
    
    // Memory barrier
    fence
    fence.i
    
    // Call Rust entry point
    call boot_main
    
    // Halt (should not return)
    wfi
    j .

    csrsi mstatus, 8  // Disable interrupts
    wfi
    j .

// Boot stack - 16KB
.section .bss
.align 4096
boot_stack:
    .space 4096 * 4
.global boot_stack_top
boot_stack_top:
