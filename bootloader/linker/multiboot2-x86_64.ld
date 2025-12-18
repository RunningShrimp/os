/* Multiboot2 bootloader linker script for x86_64 */

ENTRY(_start)

SECTIONS {
    /* Align to page boundary */
    . = ALIGN(4K);
    
    .boot_header : {
        LONG(0xE85250D6)                    /* Multiboot2 magic */
        LONG(0)                             /* architecture: i386 */
        LONG(multiboot2_header_end - multiboot2_header_start)  /* header length */
        LONG(-(0xE85250D6 + 0 + (multiboot2_header_end - multiboot2_header_start)))  /* checksum */
        
        /* End tag */
        WORD(0)                             /* type */
        WORD(0)                             /* flags */
        LONG(8)                             /* size */
        
        multiboot2_header_end = .;
    }
    
    multiboot2_header_start = multiboot2_header_end - 32;
    
    . = ALIGN(4K);
    
    .text : {
        KEEP(*(.text.boot))
        *(.text .text.*)
    }
    
    .rodata : {
        *(.rodata .rodata.*)
    }
    
    .data : {
        *(.data .data.*)
    }
    
    .bss : {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        __bss_end = .;
    }
}
