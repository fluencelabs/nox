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

//! Wrapper for counter (a test for Fluence network).
//!
//! Provides the public method `invoke` that increments a counter and returns its updated state.

mod counter;
use byteorder::{LittleEndian, WriteBytesExt};

//
// FFI for interaction with counter module
//

static mut COUNTER: counter::Counter = counter::Counter { counter: 0 };

#[no_mangle]
pub unsafe fn invoke(_ptr: *mut u8, _len: usize) -> usize {
    COUNTER.inc();

    let mut counter_value = vec![];
    counter_value.write_i64::<LittleEndian>(COUNTER.get()).unwrap();

    fluence::memory::write_result_to_mem(&counter_value)
        // returns error string instead of counter value in case of any error
        .expect("[Error] Writing counter value to memory was failed")
        .as_ptr() as usize
}
