#![feature(core_intrinsics)]

mod settings;
extern crate sha3;
extern crate reduce;

use settings::{INITIAL_VALUE, ITERATIONS_COUNT};
use sha3::{Digest, Sha3_256};
use std::mem;
use reduce::Reduce;

// Now Rust doesn't have any const function that can
// determine size of const variable in compile-time
#[no_mangle]
const fn static_size_of_val<T>(_: &T) -> usize {
    mem::size_of::<T>()
}

#[no_mangle]
pub fn bench_test() -> u8 {
    let seed_as_byte_array: [u8; static_size_of_val(&INITIAL_VALUE)] =
        unsafe { mem::transmute(INITIAL_VALUE.to_le()) };
    let mut hash_result = Sha3_256::digest(&seed_as_byte_array);

    for _ in 1..ITERATIONS_COUNT {
        hash_result = Sha3_256::digest(&hash_result);
    }

    hash_result.into_iter().reduce(|x1 ,x2| x1 ^ x2).unwrap()
}
