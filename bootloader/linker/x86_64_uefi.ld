/* UEFI linker script for x86_64 */

ENTRY({{ARCH_STARTUP}})

SECTIONS
{
    . = {{KERNEL_BASE}};

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

    /* Stack and heap would be allocated here */
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