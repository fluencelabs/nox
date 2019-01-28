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

mod settings {
    /// This seed is used for deterministic operation count on different launches.
    pub const SEED: &str = env!("SEED");

    /// Matrix size.
    pub const MATRIX_SIZE: &str = env!("MATRIX_SIZE");

    // count of test iterations
    pub const ITERATIONS_COUNT: &str = env!("ITERATIONS_COUNT");
}

// 1117 due to prevent overflow in matrix multiplication
const GENERATION_INTERVAL: u64 = 1117;

use nalgebra::DMatrix;
type Matrix = DMatrix<u64>;

use rand::{Rng, SeedableRng};
use rand_isaac::IsaacRng;

/// Generates random matrix with given size by IsaacRng.
fn generate_random_matrix(rows_number: usize, columns_count: usize, seed: u64) -> Matrix {
    let mut rng: IsaacRng = SeedableRng::seed_from_u64(seed);

    Matrix::from_fn(rows_number, columns_count, |_, _| {
        rng.gen_range(0u64, GENERATION_INTERVAL)
    })
}

/// Computes hash of a matrix as its trace.
fn compute_matrix_hash(matrix: &Matrix) -> u64 {
    let mut trace: u64 = 0;

    // it is assumed that matrix is squared
    for i in 0..matrix.ncols() {
        trace += matrix[(i, i)]
    }

    trace
}

pub fn bench() -> u64 {
    let matrix_size: usize = settings::MATRIX_SIZE.parse::<usize>().unwrap();
    let seed: u64 = settings::SEED.parse::<u64>().unwrap();
    let iterations_count: u64 = settings::ITERATIONS_COUNT.parse::<u64>().unwrap();

    let mut matrix_hash: u64 = seed;
    for _ in 1..iterations_count {
        let matrix_a = generate_random_matrix(matrix_size, matrix_size, matrix_hash);
        matrix_hash = compute_matrix_hash(&matrix_a);
        let matrix_b = generate_random_matrix(matrix_size, matrix_size, matrix_hash);

        let matrix_c = matrix_a * matrix_b;
        matrix_hash = compute_matrix_hash(&matrix_c);
    }

    matrix_hash
}
