// the entry point for all programs is the first block
ENTRY:
  LOAD_INT32 400000000                     // this is the number of counter iterations
  STORE_REG r6
  JUMP #LOOP_CHECK

// this block returns from subroutine to the host executable
EXIT:
  RET

// this block checks before each iteration if we are still below the max iteration count
LOOP_CHECK:
  GET_LOCAL .0
  STORE_REG r7
  INCR                               // post-increments loop counter index 
  SET_LOCAL .0
  LOAD_REG r6
  LESS_THAN r7
  JUMP_EITHER #LOOP_BODY #EXIT   // if r7 < 1000 then jump to LOOP_BODY otherwise EXIT

// this block is run for every loop iteration
LOOP_BODY:
  LOAD_RAND 2                           // get a random number from the range 0=..2
  //LOAD_INT32 1
  STORE_REG r5
  LOAD_INT32 1
  LESS_THAN r5
  JUMP_EITHER #COIN_HEADS #COIN_TAILS   // if r5 < 1 then jump to COIN_HEADS otherwise COIN_TAILS

COIN_HEADS:
  GET_LOCAL .1
  INCR
  SET_LOCAL .1                       // sets the value of vm.local[1] to head flip count
  JUMP #LOOP_CHECK

COIN_TAILS:
  GET_LOCAL .2
  INCR
  SET_LOCAL .2                       // sets the value of vm.local[2] to tails flip count
  JUMP #LOOP_CHECK
