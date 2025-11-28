ENTRY(rust_main)
SECTIONS {
  . = 0x00100000;
  .text : { *(.text*) }
  .rodata : { *(.rodata*) }
  .data : { *(.data*) }
  .bss : { *(.bss*) *(COMMON) }
  . = . + 0x10000;
  _stack_top = .;
  _heap_start = .;
  . = . + 0x80000;
  _heap_end = .;
  _phys_end = _heap_end;
}
