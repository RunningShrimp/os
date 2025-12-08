/*
 * NOS BIOS Bootloader Linker Script
 *
 * This linker script is designed for BIOS bootloader that runs in real mode,
 * transitions to protected mode, and then to long mode. It handles the
 * complex memory layout requirements for x86_64 BIOS booting.
 */

/* Entry point for BIOS bootloader */
ENTRY(_start)

/* Memory layout for BIOS bootloader */
MEMORY
{
    /* Real mode code (0x7C00-0x7E00) - BIOS loads boot sector here */
    BOOT_SECTOR (rx)  : ORIGIN = 0x7C00, LENGTH = 512

    /* Extended bootloader code (0x7E00-0x9F00) */
    BOOTLOADER (rx)   : ORIGIN = 0x7E00, LENGTH = 0x2100

    /* Protected mode code and data (0x10000-0x20000) */
    PROTECTED_MODE (rwx) : ORIGIN = 0x10000, LENGTH = 0x10000

    /* Long mode code and data (0x20000-0x40000) */
    LONG_MODE (rwx)   : ORIGIN = 0x20000, LENGTH = 0x20000

    /* Data and BSS sections (0x40000-0x50000) */
    DATA_BSS (rwx)    : ORIGIN = 0x40000, LENGTH = 0x10000

    /* Stack space (0x50000-0x60000) */
    STACK (rwx)       : ORIGIN = 0x50000, LENGTH = 0x10000

    /* Memory map buffer (0x60000-0x61000) */
    MEMORY_MAP (rw)   : ORIGIN = 0x60000, LENGTH = 0x1000
}

/* Section definitions */
SECTIONS
{
    /* ===== REAL MODE SECTIONS ===== */

    . = ORIGIN(BOOT_SECTOR);

    /* Boot sector code (first 512 bytes) */
    .boot_sector :
    {
        KEEP(*(.boot_sector))
        KEEP(*(.multiboot))
        KEEP(*(.text.boot))
    } > BOOT_SECTOR

    /* Extended bootloader code */
    . = ORIGIN(BOOTLOADER);

    .bootloader :
    {
        *(.text.bootloader)
        *(.text.real_mode)
        *(.data.bootloader)
        *(.rodata.bootloader)
    } > BOOTLOADER

    /* ===== PROTECTED MODE SECTIONS ===== */

    . = ORIGIN(PROTECTED_MODE);

    .text.protected :
    {
        protected_mode_start = .;
        *(.text.protected)
        *(.text.protected_mode)
        protected_mode_end = .;
    } > PROTECTED_MODE

    .data.protected :
    {
        protected_data_start = .;
        *(.data.protected)
        *(.bss.protected)
        protected_data_end = .;
    } > PROTECTED_MODE

    /* ===== LONG MODE SECTIONS ===== */

    . = ORIGIN(LONG_MODE);

    /* Text section - executable code */
    .text :
    {
        long_mode_start = .;
        *(.text)
        *(.text.*)
        *(.rodata)
        *(.rodata.*)
        KEEP(*(.text.entry))
        long_mode_end = .;
    } > LONG_MODE

    /* Data section - initialized data */
    .data :
    {
        data_start = .;
        *(.data)
        *(.data.*)
        . = ALIGN(8);
        data_end = .;
    } > LONG_MODE

    /* BSS section - zero-initialized data */
    .bss :
    {
        bss_start = .;
        *(.bss)
        *(.bss.*)
        *(COMMON)
        . = ALIGN(8);
        bss_end = .;
    } > DATA_BSS

    /* Stack section */
    .stack :
    {
        stack_start = .;
        . = . + STACK_SIZE;
        stack_end = .;
    } > STACK

    /* Memory map buffer */
    .memory_map :
    {
        memory_map_start = .;
        *(.memory_map)
        memory_map_end = .;
    } > MEMORY_MAP

    /* Special sections */

    /* BIOS data structures */
    .bios_info :
    {
        *(.bios_info)
        *(.boot_info)
        *(.boot_params)
    } > DATA_BSS

    /* Framebuffer information */
    .framebuffer_info :
    {
        *(.framebuffer_info)
    } > DATA_BSS

    /* ACPI information */
    .acpi_info :
    {
        *(.acpi_info)
    } > DATA_BSS

    /* System information */
    .system_info :
    {
        *(.system_info)
    } > DATA_BSS

    /* Debug information */
    .debug_info :
    {
        *(.debug_info)
        debug_buffer = .;
        . = . + 4096; /* 4KB debug buffer */
    } > DATA_BSS

    /* Multiboot information */
    .multiboot_info :
    {
        multiboot_info_start = .;
        *(.multiboot_info)
        multiboot_info_end = .;
    } > DATA_BSS

    /* PE/COFF headers for UEFI compatibility */
    .pe_header :
    {
        pe_header_start = .;
        *(.pe_header)
        pe_header_end = .;
    } > DATA_BSS

    /* GDT (Global Descriptor Table) */
    .gdt :
    {
        gdt_start = .;
        KEEP(*(.gdt))
        gdt_end = .;
        . = ALIGN(8);
    } > DATA_BSS

    /* IDT (Interrupt Descriptor Table) */
    .idt :
    {
        idt_start = .;
        KEEP(*(.idt))
        idt_end = .;
        . = ALIGN(8);
    } > DATA_BSS

    /* Page tables for long mode */
    .page_tables :
    {
        page_tables_start = .;
        *(.page_tables)
        page_tables_end = .;
        . = ALIGN(4096);
    } > DATA_BSS

    /* Stack for protected mode */
    .protected_stack :
    {
        protected_stack_start = .;
        . = . + PROTECTED_STACK_SIZE;
        protected_stack_end = .;
    } > STACK

    /* Stack for long mode */
    .long_stack :
    {
        long_stack_start = .;
        . = . + LONG_STACK_SIZE;
        long_stack_end = .;
    } > STACK

    /* ===== SYMBOL DEFINITIONS ===== */

    /* Define symbols for use in assembly code */
    boot_sector_start = ORIGIN(BOOT_SECTOR);
    bootloader_start = ORIGIN(BOOTLOADER);
    protected_mode_code_start = ORIGIN(PROTECTED_MODE);
    long_mode_code_start = ORIGIN(LONG_MODE);

    /* Stack sizes */
    STACK_SIZE = 0x4000;              /* 16KB */
    PROTECTED_STACK_SIZE = 0x1000;     /* 4KB */
    LONG_STACK_SIZE = 0x8000;          /* 32KB */

    /* Size calculations */
    boot_sector_size = boot_sector_end - boot_sector_start;
    bootloader_size = protected_mode_start - bootloader_start;
    protected_mode_size = protected_mode_end - protected_mode_start;
    long_mode_size = long_mode_end - long_mode_start;
    data_size = data_end - data_start;
    bss_size = bss_end - bss_start;

    /* Total bootloader size */
    bootloader_total_size = . - boot_sector_start;

    /* Memory map entries */
    max_memory_map_entries = 128;
    memory_map_entry_size = 20; /* sizeof(E820Entry) */
    memory_map_buffer_size = max_memory_map_entries * memory_map_entry_size;

    /* Discard unused sections */
    /DISCARD/ :
    {
        *(.eh_frame)
        *(.eh_frame_hdr)
        *(.comment)
        *(.note*)
        *(.debug*)
        *(.dynsym)
        *(.dynstr)
        *(.dynamic)
        *(.gnu.hash)
        *(.gnu.version)
        *(.gnu.version_r)
        *(.rel.dyn)
        *(.rela.dyn)
    }

    /* Ensure proper alignment */
    . = ALIGN(4096);

    /* End of bootloader */
    _end = .;
}

/* Memory map for kernel */
SECTIONS
{
    /* Kernel will be loaded here */
    . = ORIGIN(KERNEL_SPACE);

    .kernel :
    {
        kernel_start = .;
        KEEP(*(.kernel))
        kernel_end = .;
    } > KERNEL_SPACE
}

/* Additional memory regions for kernel */
MEMORY
{
    KERNEL_SPACE (rwx) : ORIGIN = 0x100000, LENGTH = 0x200000 /* 2MB for kernel */
}

/* Provide symbol information to the code */
PROVIDE(__bootloader_start = ORIGIN(BOOT_SECTOR));
PROVIDE(__bootloader_end = _end);
PROVIDE(__bootloader_size = bootloader_total_size);
PROVIDE(__bss_start = bss_start);
PROVIDE(__bss_end = bss_end);
PROVIDE(__bss_size = bss_size);
PROVIDE(__stack_top = stack_end);
PROVIDE(__protected_stack_top = protected_stack_end);
PROVIDE(__long_stack_top = long_stack_end);
PROVIDE(__memory_map_buffer = memory_map_start);
PROVIDE(__memory_map_size = memory_map_buffer_size);
PROVIDE(__gdt = gdt_start);
PROVIDE(__gdt_size = gdt_end - gdt_start);
PROVIDE(__idt = idt_start);
PROVIDE(__idt_size = idt_end - idt_start);
PROVIDE(__page_tables = page_tables_start);
PROVIDE(__page_tables_size = page_tables_end - page_tables_start);

/* Debug symbols */
PROVIDE(__debug_buffer = debug_buffer);
PROVIDE(__debug_buffer_size = 4096);

/* Boot information */
PROVIDE(__multiboot_info = multiboot_info_start);
PROVIDE(__multiboot_info_size = multiboot_info_end - multiboot_info_start);

/* Framebuffer information */
PROVIDE(__framebuffer_info = framebuffer_info_start);

/* ACPI information */
PROVIDE(__acpi_rsdp = acpi_info_start);

/* System information */
PROVIDE(__system_memory = system_info_start);
PROVIDE(__cpu_features = system_info_start + 8);
PROVIDE(__cpu_vendor = system_info_start + 16);

/* BIOS specific symbols */
PROVIDE(__bios_version = bios_info_start);
PROVIDE(__boot_cmdline = boot_info_start);
PROVIDE(__kernel_image_path = boot_info_start + 512);
PROVIDE(__boot_timeout = boot_info_start + 1024);
PROVIDE(__boot_flags = boot_info_start + 1028);