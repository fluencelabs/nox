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
extern crate reikna;

use settings::FACTORIZED_NUMBER;
use reikna::prime;

/// This test simply factorizes given FACTORIZED_NUMBER by sieves from reikna library.
///
/// Returns a prime delimiter of FACTORIZED_NUMBER to prevent possible aggressive optimizations.
#[no_mangle]
pub extern "C" fn main() -> u64 {
    let factorized_number : u64 = FACTORIZED_NUMBER.parse::<u64>().unwrap();
    // reikna uses Atkin or Eratosthenes seive to factorize given number
    let factors = prime::factorize(factorized_number);

    factors[0]
}
