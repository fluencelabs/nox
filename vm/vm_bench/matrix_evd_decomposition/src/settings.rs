// this seed is used for deterministic operation count on different launches
const SEED : i32 = 17;

// matrix size
const MATRIX_ROWS : i32 = 10;
const MATRIX_COLS : i32 = 10;

// count of test iterations
const ITERATIONS_COUNT : i32  = 1000;

// 1117 due to prevent overflow in matrix multiplication
const GENERATION_INTERVAL : i32 = 1117;

// exactly matrix type
type Matrix = na::DMatrix<i64>;
