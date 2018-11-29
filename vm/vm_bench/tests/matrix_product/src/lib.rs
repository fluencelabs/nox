mod settings;

extern crate rand;
extern crate rand_isaac;

use rand::{Rng, SeedableRng};
use rand_isaac::IsaacRng;
use settings::*;

fn generate_random_matrix(rows_number: u32, columns_count: u32, seed: u64) -> Matrix {
    let mut rng: IsaacRng = SeedableRng::seed_from_u64(seed);
    Matrix::from_fn(rows_number as usize, columns_count as usize, |_, _| rng.gen_range(0u64, GENERATION_INTERVAL))
}

// computes hash of a matrix as its trace
fn compute_matrix_hash(matrix: &Matrix) -> u64 {
    let mut trace : u64 = 0;

    // it is assumed that matrix is squared
    for i in 0..matrix.ncols() {
        trace += matrix[(i,i)]
    }

    trace
}

fn bench_test() -> u64 {
    let matrix_size : u32 = MATRIX_SIZE.parse::<u32>().unwrap();
    let seed : u64 = SEED.parse::<u64>().unwrap();
    let iterations_count : u64 = ITERATIONS_COUNT.parse::<u64>().unwrap();

    let mut matrix_hash : u64 = seed;
    for _ in 1..iterations_count {
        let matrix_a = generate_random_matrix(matrix_size, matrix_size, matrix_hash);
        matrix_hash = compute_matrix_hash(&matrix_a);
        let matrix_b = generate_random_matrix(matrix_size, matrix_size, matrix_hash);

        let matrix_c = matrix_a * matrix_b;
        matrix_hash = compute_matrix_hash(&matrix_c);
    }

    matrix_hash
}

#[no_mangle]
pub extern "C" fn app_main() -> u64 {
    bench_test()
}

#[no_mangle]
pub extern "C" fn main() -> u64 {
    bench_test()
}
