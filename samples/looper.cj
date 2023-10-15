// the entry point for all programs is the first block
ENTRY:
  JUMP #LOOP_CHECK

// this block returns from subroutine to the host executable
EXIT:
  RET

// this block checks before each iteration if we are still below the max iteration count
LOOP_CHECK:
  GET_LOCAL .0
  STORE_REG r7
  LOAD_IMM 10000                     // this is the number of counter iterations
  LESS_THAN r7
  JUMP_EITHER #LOOP_BODY #LOOP_END   // if r7 < 1000 then jump to LOOP_BODY otherwise LOOP_END

// this block is run for every loop iteration
LOOP_BODY:
  GET_LOCAL .0
  INCR
  SET_LOCAL .0                       // sets the value of vm.local[0] to incremented value
  JUMP #LOOP_CHECK

// this block is run once we have reached the max iteration count
LOOP_END:
  LOAD_IMM 1
  STORE_REG r0                       // this sets r0 to 1 once we have finished looping
  JUMP #EXIT