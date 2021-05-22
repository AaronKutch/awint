/* from crate riscv_minimal_rt */

MEMORY
{
  FLASH : ORIGIN = 0, LENGTH = 16M
  RAM : ORIGIN = 1000000, LENGTH = 16K
}

_stack_start = 0;
