/*
 * Copyright 2021 Fluence Labs Limited
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

pub(crate) fn process_func_outcome(func_outcome: FunctionOutcome) -> Result<JValue, JError> {
    match func_outcome {
        FunctionOutcome::NotDefined { args, .. } => Err(JError::new(format!(
            "Service with id '{}' not found (function {})",
            args.service_id, args.function_name
        ))),
        FunctionOutcome::Empty => Err(JError::new("Function has not returned any result")),
        FunctionOutcome::Err(err) => Err(JError::new(err.to_string())),
        FunctionOutcome::Ok(v) => Ok(v),
    }
}

pub(crate) fn parse_spell_id_from(particle_id: String) -> eyre::Result<String, JError> {
    if particle_id.starts_with("spell_") {
        Ok(particle_id
            .split('_')
            .collect::<Vec<&str>>()
            .get(1)
            .ok_or(JError::new("Invalid particle id format"))?
            .to_string())
    } else {
        Err(JError::new("Invalid particle id format"))
    }
}
