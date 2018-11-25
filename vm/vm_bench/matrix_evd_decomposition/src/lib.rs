extern crate rand;
extern crate mersenne_twister;

use mersenne_twister::MersenneTwister;
use rand::{Rng, SeedableRng};

mod settings;

#[no_mangle]
fn create_matrix(rows_number: u32, columns_count: u32, seed: i32) -> Matrix {
    let mut rng = SeedableRng::from_seed(seed);
    Matrix::from_fn(rows_number, columns_count, |_, _| rng.gen())
}

fn compute_matrix_hash(matrix: Matrix) -> i64 {

}

#[no_mangle]
pub extern "C" fn benchmark_entry() -> i64 {

    let matrix_a = create_matrix(MATRIX_ROWS, MATRIX_COLS, SEED);
    let matrix_b = create_matrix(MATRIX_ROWS, MATRIX_COLS, SEED);

    for _ in 1..ITERATIONS_COUNT {
        let matrix_c = matrix_a * matrix_b;
    }
    1
}
