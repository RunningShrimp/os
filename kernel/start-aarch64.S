.globl _start
.section .text
_start:
  ldr x0, =_stack_top
  mov sp, x0
  bl rust_main
1:
  b 1b

