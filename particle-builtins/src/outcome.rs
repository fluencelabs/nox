/*
 * Copyright 2024 Fluence DAO
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

use particle_args::JError;
use particle_execution::FunctionOutcome;
use serde_json::Value as JValue;

pub fn ok(v: JValue) -> FunctionOutcome {
    FunctionOutcome::Ok(v)
}

pub fn wrap(r: Result<JValue, JError>) -> FunctionOutcome {
    match r {
        Ok(v) => FunctionOutcome::Ok(v),
        Err(err) => FunctionOutcome::Err(err),
    }
}

pub fn wrap_unit(r: Result<(), JError>) -> FunctionOutcome {
    match r {
        Ok(_) => FunctionOutcome::Empty,
        Err(err) => FunctionOutcome::Err(err),
    }
}
