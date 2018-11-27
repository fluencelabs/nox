mod settings;

extern crate snap;
extern crate rand;
extern crate rand_isaac;

use settings::{SEED, ITERATIONS_COUNT, SEQUENCE_SIZE};
use rand::{Rng, SeedableRng};
use rand_isaac::IsaacRng;
use std::mem;

type Sequence = Vec<u8>;

fn generate_sequence(seed : u64, size : u64) -> Sequence {
    let mut seed_as_vec : [u8; 32] = [0;32];
    let tt : [u8; 8] =  unsafe { mem::transmute(seed.to_le()) };
    // TODO : rewrite this ugly code
    for i in 0..8 {
        seed_as_vec[i] += tt[i];
    }

    let mut rng: IsaacRng = SeedableRng::from_seed(seed_as_vec);
    let mut result_sequence = Sequence::with_capacity(size as usize);

    for _ in 0..size {
        result_sequence.push(rng.gen::<u8>());
    }
    result_sequence
}

#[no_mangle]
pub extern "C" fn bench_test() -> u64 {
    let seed : u64 = SEED.parse::<u64>().unwrap();
    let iterations_count : u64 = ITERATIONS_COUNT.parse::<u64>().unwrap();
    let sequence_size : u64 = SEQUENCE_SIZE.parse::<u64>().unwrap();

    let mut sequence : Sequence = generate_sequence(seed, sequence_size);
    let mut compressed_sequence  = snap::Encoder::new().compress_vec(&sequence).unwrap();

    for _ in 1..iterations_count {
        let new_seed = compressed_sequence.len() +
            compressed_sequence.iter().fold(0u8, |x1, x2| x1 ^ x2) as usize;
        sequence = generate_sequence(new_seed as u64, sequence_size);
        compressed_sequence = snap::Encoder::new().compress_vec(&sequence).unwrap();
    }

    compressed_sequence.iter().fold(0u8, |x1, x2| x1 ^ x2) as u64
}
