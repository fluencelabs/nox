extern crate rand
extern crate mersenne_twister;

use mersenne_twister::MersenneTwister;
use rand::{Rng, SeedableRng};

// this seed is used for deterministic operation count on different launches
const SEED : i32 = 17
const MATRIX_ROWS : i32 = 10
const MATRIX_COLS : i32 = 10

// 1117 due to prevent overflow in matrix multiplication
const GENERATION_INTERVAL = 1117

// todo define matrix template for i64 and f64
type Matrix = na::DMatrix<i64>

#[no_mangle]
fn create_matrix(rows_number: u32, columns_count: u32, seed: i32) -> Matrix {
    let mut rng = SeedableRng::from_seed(seed);
    Matrix::from_fn(rows_number, columns_count, |_, _| rng.gen())
}

#[no_mangle]
pub extern "C" fn test() -> i64 {
    let matrix_A = create_matrix()
    let matrix_B = create_matrix()

    matrix_C = matrix_A.mul(matrix_B)
}
