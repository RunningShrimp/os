.globl _start
_start:
  call rust_main
1:
  hlt
  jmp 1b
