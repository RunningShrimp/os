/* UEFI linker script for AArch64 */

ENTRY({{ARCH_STARTUP}})

SECTIONS
{
    . = {{KERNEL_BASE}};

    /* Text section */
    .text :
    {
        *(.text*)
        *(.rodata*)
        *(.rodata.str1.1)
    } = .;

    /* Data section */
    .data :
    {
        *(.data*)
    }

    /* BSS section */
    .bss :
    {
        *(.bss*)
        *(COMMON)
    }

    /* Section alignment */
    . = ALIGN(4096);

    /* Stack section */
    .stack :
    {
        . = . + {{STACK_SIZE}};
        _stack_top = .;
    }

    /* Heap section */
    .heap :
    {
        . = . + {{HEAP_SIZE}};
        _heap_end = .;
    }

    /* PE/COFF headers for UEFI */
    .efi_header :
    {
        SHORT(0x5A4D);      // MZ magic
        SHORT(0x90);
        LONG(0x3);        // Number of sections
        LONG(0x0);         // Timestamp
        LONG(0x4);         // Symbol table pointer
        LONG(0x200);       // Number of symbols
        SHORT(0);           // Optional header size
        SHORT(0x20B);       // PE32+ magic
        LONG(0x10B);        // Linker version
        LONG(0x0);          // Size of image
        LONG(0x1000);       // Size of headers
        LONG(0x200);        // Checksum
        SHORT(0);           // Subsystem
        SHORT(0);           // DLL characteristics
        LONG(0);            // Stack reserve size
        LONG(0);            // Stack commit size
        LONG(0);            // Heap reserve size
        LONG(0);            // Heap commit size
        LONG(0);            // Loader flags
        LONG(0x20);         // Number of RVAs and entries
        LONG(0);            // RVA of first entry
    }

    /* PE header */
    .pe_header :
    {
        LONG(0x20B);        // PE32+ magic
        SHORT(0);           // Machine type (AArch64)
        SHORT(1);           // Number of sections
        LONG(0);            // Timestamp
        LONG(0);            // Symbol table pointer
        LONG(0);            // Number of symbols
        SHORT(0x70);        // Optional header size
        SHORT(0x202);       // Characteristics
        SHORT(0);           // Magic number
        SHORT(0);           // Major linker version
        SHORT(0);           // Minor linker version
        LONG(0);            // Size of code
        LONG(0);            // Size of initialized data
        LONG(0);            // Size of uninitialized data
        LONG(0);            // Entry point RVA
        LONG(0);            // Base of code
        LONG(0);            // Base of data
    }

    /* Section headers */
    .section_headers :
    {
        LONG(0);            // Name
        LONG(0);            // Virtual size
        LONG(0);            // Virtual address
        LONG(0);            // Size of raw data
        LONG(0);            // Pointer to raw data
        LONG(0);            // Pointer to relocations
        LONG(0);            // Pointer to line numbers
        LONG(0);            // Number of relocations
        LONG(0);            // Number of line numbers
        LONG(0);            // Characteristics
    }
}