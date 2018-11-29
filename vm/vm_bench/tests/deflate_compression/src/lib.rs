mod settings;

extern crate deflate;
extern crate rand;
extern crate rand_isaac;

use settings::{SEED, ITERATIONS_COUNT, SEQUENCE_SIZE};
use rand::{Rng, SeedableRng};
use rand_isaac::IsaacRng;
use deflate::deflate_bytes;

type Sequence = Vec<u8>;

fn generate_sequence(seed : u64, size : u64) -> Sequence {
    let mut rng: IsaacRng = SeedableRng::seed_from_u64(seed);
    let mut result_sequence = Sequence::with_capacity(size as usize);

    for _ in 0..size {
        result_sequence.push(rng.gen::<u8>());
    }
    result_sequence
}

#[no_mangle]
pub extern "C" fn main() -> u64 {
    let seed : u64 = SEED.parse::<u64>().unwrap();
    let iterations_count : u64 = ITERATIONS_COUNT.parse::<u64>().unwrap();
    let sequence_size : u64 = SEQUENCE_SIZE.parse::<u64>().unwrap();

    let mut sequence = generate_sequence(seed, sequence_size);
    let mut compressed_sequence = deflate_bytes(&sequence);

    for _ in 1..iterations_count {
        let new_seed = compressed_sequence.len() +
            compressed_sequence.iter().fold(0u8, |x1, x2| x1 ^ x2) as usize;
        sequence = generate_sequence(new_seed as u64, sequence_size);
        compressed_sequence = deflate_bytes(&sequence);
    }

    compressed_sequence.iter().fold(0u8, |x1, x2| x1 ^ x2) as u64
}
