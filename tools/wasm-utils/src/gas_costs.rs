/*
 * Copyright 2019 Fluence Labs Limited
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

#[cfg(not(features = "std"))]
use std::collections::BTreeMap as Map;
#[cfg(features = "std")]
use std::collections::HashMap as Map;

use pwasm_utils::rules::{InstructionType, Metering};

// gas costs have been chosen according to
// https://github.com/ewasm/design/blob/master/determining_wasm_gas_costs.md
pub fn gas_cost_table() -> Map<InstructionType, Metering> {
    vec![
        (InstructionType::Add, Metering::Fixed(3)),
        (InstructionType::Bit, Metering::Fixed(3)),
        (InstructionType::Const, Metering::Fixed(3)),
        (InstructionType::ControlFlow, Metering::Fixed(2)),
        (InstructionType::Conversion, Metering::Fixed(3)),
        (InstructionType::CurrentMemory, Metering::Fixed(3)),
        (InstructionType::Div, Metering::Fixed(80)),
        (InstructionType::Float, Metering::Fixed(160)),
        (InstructionType::FloatComparsion, Metering::Fixed(200)),
        (InstructionType::FloatConst, Metering::Fixed(20)),
        (InstructionType::FloatConversion, Metering::Fixed(20)),
        (InstructionType::Global, Metering::Fixed(3)),
        (InstructionType::GrowMemory, Metering::Fixed(0)),
        (InstructionType::IntegerComparsion, Metering::Fixed(3)),
        (InstructionType::Load, Metering::Fixed(3)),
        (InstructionType::Local, Metering::Fixed(3)),
        (InstructionType::Mul, Metering::Fixed(3)),
        (InstructionType::Nop, Metering::Fixed(1)),
        (InstructionType::Reinterpretation, Metering::Fixed(1)),
        (InstructionType::Store, Metering::Fixed(3)),
        (InstructionType::Unreachable, Metering::Fixed(100)),
    ]
    .into_iter()
    .collect()
}
