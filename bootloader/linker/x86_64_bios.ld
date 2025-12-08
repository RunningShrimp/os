/* BIOS/Multiboot2 linker script for x86_64 */

ENTRY({{ARCH_STARTUP}})

SECTIONS
{
    . = 0x100000; /* 1MB */

    .text :
    {
        *(.text*)
        *(.rodata*)
    }

    .data :
    {
        *(.data*)
        *(.bss*)
        *(COMMON)
    }

    . = ALIGN(4096);

    .stack :
    {
        . = . + 64K;
        _stack_top = .;
    }

    .heap :
    {
        . = . + 512K;
        _heap_end = .;
    }
}