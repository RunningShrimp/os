# x86_64 Bootloader Entry Assembly
# 
# This file provides the actual _start entry point that bootloaders jump to
# Handles GDT setup, stack initialization, and transition to Rust code

.section .text.start
.extern boot_main
.extern _kernel_stack_top

.global _start

_start:
    # Bootloader (GRUB/UEFI) jumps here
    # EAX = Multiboot magic or UEFI info
    # EBX = Multiboot info address
    # RSP = bootloader-provided stack (may be invalid)
    
    # Set up our own stack
    mov $(_kernel_stack_top), %rsp
    
    # Align stack to 16-byte boundary (required by ABI)
    and $0xFFFFFFFFFFFFFFF0, %rsp
    sub $8, %rsp
    
    # Clear flags
    xor %eax, %eax
    push %rax
    popf
    
    # Save boot parameters for Rust code
    # Move Multiboot magic and info pointer to callee-saved registers
    # (these must be preserved across function calls)
    push %rax                # Save EAX (magic)
    push %rbx                # Save RBX (info address)
    
    # Clear BSS section (uninitialized data)
    lea _bss_start(%rip), %rdi
    lea _bss_end(%rip), %rcx
    sub %rdi, %rcx
    xor %eax, %eax
    xor %edx, %edx
    
    # Use REP STOSD for faster clearing (8 bytes at a time on x86_64)
    mov %rcx, %rax
    shr $3, %rax           # Divide by 8
    mov %rax, %rcx
    xor %eax, %eax
    cld
    rep stosq              # Clear 8-byte chunks
    
    # Call Rust boot_main function
    # Restore boot parameters for Rust
    pop %rbx                # RBX = info address
    pop %rax                # RAX = magic
    
    # Call boot_main() which never returns
    # The bootloader expects us to never return
    call boot_main
    
    # If boot_main returns (which it shouldn't), halt
    hlt
    jmp _start

# Panic handler - called on panic! macro
.global rust_panic
.align 16
rust_panic:
    # Print error message if possible
    # For now, just halt the system
    hlt
    jmp rust_panic

# Stack space allocation (64KB)
.section .bss
.align 16
_kernel_stack_bottom:
    .space 65536
_kernel_stack_top:

.section .data
_bss_start:
.long 0
_bss_end:
.long 0
