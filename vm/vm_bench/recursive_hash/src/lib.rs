mod settings;
extern crate sha3;

use settings::{INITIAL_VALUE, ITERATIONS_COUNT};
use sha3::{Digest, Sha3_256};
use std::mem;

#[no_mangle]
pub extern "C" fn main() -> u8 {
    let iterations_count : i64 = ITERATIONS_COUNT.parse::<i64>().unwrap();
    let initial_value : i64 = INITIAL_VALUE.parse::<i64>().unwrap();

    let seed_as_byte_array: [u8; mem::size_of::<i64>()] = unsafe { mem::transmute(initial_value.to_le()) };
    let mut hash_result = Sha3_256::digest(&seed_as_byte_array);

    for _ in 0..iterations_count {
        hash_result = Sha3_256::digest(&hash_result);
    }

    hash_result.iter().fold(0, |x1 ,x2| x1 ^ x2)
}
