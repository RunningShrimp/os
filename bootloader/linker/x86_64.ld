ENTRY(_start)

PHYS_LOAD_ADDR = 0x100000;
VIRT_BASE = 0xFFFF800000000000;

SECTIONS
{
    . = PHYS_LOAD_ADDR;

    .text : ALIGN(0x1000) {
        *(.multiboot2)
        *(.text .text.*)
        . = ALIGN(0x1000);
    }

    .rodata : ALIGN(0x1000) {
        *(.rodata .rodata.*)
        . = ALIGN(0x1000);
    }

    .data : ALIGN(0x1000) {
        *(.data .data.*)
        . = ALIGN(0x1000);
    }

    .bss : ALIGN(0x1000) {
        __bss_start = .;
        *(.bss .bss.*)
        *(COMMON)
        . = ALIGN(0x1000);
        __bss_end = .;
    }

    /DISCARD/ : {
        *(.note.GNU-stack)
        *(.gnu_debuglink)
        *(.gnu.lto_*)
    }
}
