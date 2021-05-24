/* from crate riscv_minimal_rt */

MEMORY
{
  FLASH : ORIGIN = 0x0000000, LENGTH = 0x1000000
  RAM : ORIGIN = 0x1000000, LENGTH = 0x1000000
}

_stack_start = LENGTH(RAM);
