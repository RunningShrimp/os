/* NOS Kernel Linker Script for RISC-V 64
 * 
 * Memory Layout:
 * 0x80000000 (2GB) - Kernel code and data (loaded by bootloader)
 * 
 * Bootloader loads kernel ELF to this fixed address and jumps to rust_main()
 */

ENTRY(rust_main)

/* Physical memory constants */
KERNEL_LOAD_ADDR = 0x80000000;

SECTIONS {
    /* Kernel starts at 2GB */
    . = KERNEL_LOAD_ADDR;
    
    _kernel_start = .;
    
    /* Text section - read-only, executable */
    .text ALIGN(0x1000) : {
        *(.text .text.*)
        *(.gnu.linkonce.t.*)
    }
    
    /* Read-only data section */
    .rodata ALIGN(0x1000) : {
        *(.rodata .rodata.*)
        *(.gnu.linkonce.r.*)
        /* Include exception handling data */
        *(.eh_frame)
        *(.gcc_except_table)
    }
    
    /* Initialized data section */
    .data ALIGN(0x1000) : {
        *(.data .data.*)
        *(.gnu.linkonce.d.*)
    }
    
    /* Uninitialized data section */
    .bss ALIGN(0x1000) : {
        *(.bss .bss.*)
        *(COMMON)
        *(.gnu.linkonce.b.*)
    }
    
    _kernel_end = .;
    
    /* Kernel stack - placed after kernel sections */
    . = ALIGN(0x1000);
    _stack_end = .;
    . = . + 0x10000;  /* 64KB stack */
    _stack_top = .;
    
    /* Kernel heap */
    . = ALIGN(0x1000);
    _heap_start = .;
    . = . + 0x80000;  /* 512KB heap */
    _heap_end = .;
    
    /* Physical memory end marker */
    _phys_end = .;
    
    /* Discard unnecessary sections */
    /DISCARD/ : {
        *(.note)
        *(.note.*)
        *(.comment)
    }
}

