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

#[derive(Clone, Debug, PartialEq)]
pub struct FrankResult {
    pub outcome: Vec<u8>,
    pub spent_gas: i64,
    pub eic: i64,
}

impl FrankResult {
    pub fn new(outcome: Vec<u8>, spent_gas: i64, eic: i64) -> Self {
        Self {
            outcome,
            spent_gas,
            eic,
        }
    }
}

impl Default for FrankResult {
    fn default() -> Self {
        Self {
            outcome: Vec::new(),
            spent_gas: 0,
            eic: 0,
        }
    }
}
