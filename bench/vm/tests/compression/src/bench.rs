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
    /// This value is used as the first seed of pseudo-random generator for sequence generation.
    pub const SEED: &str = env!("SEED");

    /// Count of compression iterations.
    pub const ITERATIONS_COUNT: &str = env!("ITERATIONS_COUNT");

    /// Size of sequence that should be compressed on each iteration.
    pub const SEQUENCE_SIZE: &str = env!("SEQUENCE_SIZE");
}

use deflate::deflate_bytes;
use rand::{Rng, SeedableRng};
use rand_isaac::IsaacRng;

type Sequence = Vec<u8>;

/// Generates pseudo-random byte sequence by given seed and given size.
fn generate_sequence(seed: u64, size: usize) -> Sequence {
    let mut rng: IsaacRng = SeedableRng::seed_from_u64(seed);
    let mut result_sequence = Sequence::with_capacity(size);

    for _ in 0..size {
        result_sequence.push(rng.gen::<u8>());
    }
    result_sequence
}

/// Compresses provided sequence by deflate or snappy algorithm.
fn compress_sequence(sequence: &Sequence) -> Sequence {
    if cfg!(feature = "deflate_compression") {
        return deflate_bytes(&sequence);
    }

    snap::Encoder::new().compress_vec(&sequence).unwrap()
}

pub fn bench() -> u8 {
    let seed: u64 = settings::SEED.parse::<u64>().unwrap();
    let iterations_count: u64 = settings::ITERATIONS_COUNT.parse::<u64>().unwrap();
    let sequence_size: usize = settings::SEQUENCE_SIZE.parse::<usize>().unwrap();

    let mut compressed_sequence = generate_sequence(seed, sequence_size);

    for _ in 1..iterations_count {
        let new_seed: usize = compressed_sequence.len()
            + compressed_sequence.iter().fold(0u8, |x1, x2| x1 ^ x2) as usize;
        compressed_sequence = generate_sequence(new_seed as u64, sequence_size);
        compressed_sequence = compress_sequence(&compressed_sequence);
    }

    compressed_sequence.iter().fold(0u8, |x1, x2| x1 ^ x2)
}
