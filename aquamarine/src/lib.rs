/*
 * Copyright 2020 Fluence Labs Limited
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

#![recursion_limit = "512"]
#![warn(rust_2018_idioms)]
#![allow(
    dead_code,
    nonstandard_style,
    unused_imports,
    unused_mut,
    unused_variables,
    unused_unsafe,
    unreachable_patterns
)]

mod actor;
mod aqua_runtime;
mod aquamarine;
mod awaited_particle;
mod config;
mod error;
mod invoke;
mod outcome;
mod particle_executor;
mod plumber;
mod vm_pool;

pub use crate::aquamarine::{AquamarineApi, AquamarineBackend};
pub use aqua_runtime::AquaRuntime;
pub use awaited_particle::{AwaitedEffects, AwaitedParticle};
pub use config::{VmConfig, VmPoolConfig};
pub use outcome::{SendParticle, StepperEffects};
pub use plumber::Plumber;

// reexport
pub use aquamarine_vm::AquamarineVM;
pub use aquamarine_vm::AquamarineVMConfig;
pub use aquamarine_vm::InterpreterOutcome;

#[test]
fn deserialize() {
    let bytes = vec![
        123, 34, 97, 99, 116, 105, 111, 110, 34, 58, 34, 80, 97, 114, 116, 105, 99, 108, 101, 34,
        44, 34, 105, 100, 34, 58, 34, 49, 34, 44, 34, 105, 110, 105, 116, 95, 112, 101, 101, 114,
        95, 105, 100, 34, 58, 34, 49, 50, 68, 51, 75, 111, 111, 87, 67, 74, 104, 76, 98, 78, 51,
        118, 67, 101, 112, 109, 70, 106, 114, 87, 70, 53, 90, 70, 68, 71, 65, 117, 65, 89, 86, 121,
        78, 74, 51, 70, 49, 49, 101, 80, 99, 119, 76, 76, 82, 120, 86, 76, 34, 44, 34, 116, 105,
        109, 101, 115, 116, 97, 109, 112, 34, 58, 49, 54, 49, 55, 55, 51, 55, 48, 49, 54, 57, 51,
        49, 44, 34, 116, 116, 108, 34, 58, 54, 53, 53, 50, 53, 44, 34, 115, 99, 114, 105, 112, 116,
        34, 58, 34, 34, 44, 34, 115, 105, 103, 110, 97, 116, 117, 114, 101, 34, 58, 91, 93, 44, 34,
        100, 97, 116, 97, 34, 58, 34, 34, 125,
    ];
    let test_msg: Result<particle_protocol::ProtocolMessage, _> = serde_json::from_slice(&bytes);
    println!("{:?}", test_msg);
    test_msg.unwrap();
}
