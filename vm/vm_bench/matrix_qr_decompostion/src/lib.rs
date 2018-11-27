mod settings;

extern crate rand;
extern crate rand_isaac;

use rand::{Rng, SeedableRng};
use rand_isaac::IsaacRng;
use settings::*;

fn create_matrix(rows_number: u32, columns_count: u32, seed: u64) -> Matrix {
    let mut rng: IsaacRng = SeedableRng::seed_from_u64(seed);
    Matrix::from_fn(rows_number as usize, columns_count as usize, |_, _| rng.gen_range(0f64, GENERATION_INTERVAL))
}

fn compute_matrix_hash(matrix: &Matrix) -> f64 {
    let mut trace : f64 = 0.0;
    for i in 0..matrix.ncols() {
        trace += matrix[(i,i)]
    }

    trace
}

#[no_mangle]
pub extern "C" fn main() -> u64 {
    let matrix_size : u32 = MATRIX_SIZE.parse::<u32>().unwrap();
    let seed : u64 = SEED.parse::<u64>().unwrap();
    let iterations_count : u64 = ITERATIONS_COUNT.parse::<u64>().unwrap();

    let mut matrix_hash : u64 = seed;
    for _ in 1..iterations_count {
        let matrix = create_matrix(matrix_size, matrix_size, matrix_hash);
        let qr = matrix.qr();
        matrix_hash = compute_matrix_hash(&qr.r()).to_bits();
    }

    matrix_hash
}
