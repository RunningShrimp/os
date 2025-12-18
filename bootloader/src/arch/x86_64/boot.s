// x86_64 bootloader startup code
.section .text.boot
.global _start
.code64

_start:
    cli
    
    // Set up stack (16KB at known location)
    mov rsp, 0x200000
    and rsp, -16
    
    // Clear BSS section
    lea rbx, [rel __bss_start]
    lea rcx, [rel __bss_end]
    cmp rbx, rcx
    jge bss_done
    
    xor eax, eax
.bss_loop:
    mov [rbx], eax
    add rbx, 4
    cmp rbx, rcx
    jl .bss_loop
.bss_done:
    
    // Call Rust entry point
    lea rax, [rel boot_main]
    call rax
    
    // Halt if we return
    hlt
    jmp .

    
bss_init_done:
    // Call boot_main (Rust entry point)
    call boot_main
    
    // If boot_main returns (shouldn't happen), halt
    cli
    hlt
    jmp .

// Boot stack - 16KB
.section .bss
.align 4096
boot_stack:
    .space 4096 * 4
.global boot_stack_top
boot_stack_top:
