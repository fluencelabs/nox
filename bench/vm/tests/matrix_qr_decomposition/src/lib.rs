/*
 * Copyright 2018 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */
mod settings;

extern crate rand;
extern crate rand_isaac;

use rand::{Rng, SeedableRng};
use rand_isaac::IsaacRng;
use settings::*;

/// Generates random matrix with given size by IsaacRng.
fn generate_random_matrix(rows_number: u32, columns_count: u32, seed: u64) -> Matrix {
    let mut rng: IsaacRng = SeedableRng::seed_from_u64(seed);
    Matrix::from_fn(rows_number as usize, columns_count as usize, |_, _| rng.gen_range(0f64, GENERATION_INTERVAL))
}

/// Computes hash of a matrix as its trace.
fn compute_matrix_hash(matrix: &Matrix) -> f64 {
    let mut trace : f64 = 0.0;

    // it is assumed that matrix is squared
    for i in 0..matrix.ncols() {
        trace += matrix[(i,i)]
    }

    trace
}

#[no_mangle]
pub extern fn main() -> u64 {
    let matrix_size : u32 = MATRIX_SIZE.parse::<u32>().unwrap();
    let seed : u64 = SEED.parse::<u64>().unwrap();
    let iterations_count : u64 = ITERATIONS_COUNT.parse::<u64>().unwrap();

    let mut matrix_hash : u64 = seed;
    for _ in 1..iterations_count {
        let matrix = generate_random_matrix(matrix_size, matrix_size, matrix_hash);
        let qr = matrix.qr();
        matrix_hash = compute_matrix_hash(&qr.r()).to_bits();
    }

    matrix_hash
}
