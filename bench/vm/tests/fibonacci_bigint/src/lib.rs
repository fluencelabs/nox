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
extern crate num_bigint;
extern crate num_traits;

use settings::FIB_NUMBER;
use num_bigint::BigUint;
use num_traits::One;
use std::ops::Sub;

/// Recursively computes a fibonacci number F_num for the given num.
fn fib(num: &BigUint) -> BigUint {
    if num.le(&BigUint::from(2u32)) {
        return One::one();
    }

    fib(&num.sub(1u32)) + fib(&num.sub(2u32))
}

#[no_mangle]
pub extern fn main() -> u8 {
    let fib_number : BigUint = BigUint::from(FIB_NUMBER.parse::<u64>().unwrap());

    fib(&fib_number).to_bytes_le().iter().fold(0u8, |x1, x2| x1 ^ x2)
}

