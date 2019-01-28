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
    /// count of recursive sha3 that would be computed
    pub const ITERATIONS_COUNT: &str = env!("ITERATIONS_COUNT");

    /// an initial value for computed hash chain
    pub const INITIAL_VALUE: &str = env!("INITIAL_VALUE");
}

use sha3::{Digest, Sha3_256};
use std::mem;

pub fn bench() -> u8 {
    let iterations_count: i64 = settings::ITERATIONS_COUNT.parse::<i64>().unwrap();
    let initial_value: i64 = settings::INITIAL_VALUE.parse::<i64>().unwrap();

    let seed_as_byte_array: [u8; mem::size_of::<i64>()] =
        unsafe { mem::transmute(initial_value.to_le()) };
    let mut hash_result = Sha3_256::digest(&seed_as_byte_array);

    for _ in 1..iterations_count {
        hash_result = Sha3_256::digest(&hash_result);
    }

    hash_result.iter().fold(0, |x1, x2| x1 ^ x2)
}
