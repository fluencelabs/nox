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

    for _ in 1..iterations_count {
        hash_result = Sha3_256::digest(&hash_result);
    }

    hash_result.iter().fold(0, |x1 ,x2| x1 ^ x2)
}
