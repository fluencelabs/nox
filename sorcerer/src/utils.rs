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

use fluence_spell_dtos::value::SpellValueT;
use serde::de::DeserializeOwned;

use particle_args::JError;
use particle_execution::FunctionOutcome;

// TODO: change function name to the better one
/// Return Ok(T) if result.success is true, return Err(T.error) otherwise
pub(crate) fn process_func_outcome<T>(
    func_outcome: FunctionOutcome,
    spell_id: &str,
    function_name: &str,
) -> Result<T, JError>
where
    T: DeserializeOwned + SpellValueT,
{
    match func_outcome {
        FunctionOutcome::NotDefined { args, .. } => Err(JError::new(format!(
            "Service with id '{}' not found (function {})",
            args.service_id, args.function_name
        ))),
        FunctionOutcome::Empty => Err(JError::new(f!(
            "Function {spell_id}.{function_name} has not returned any result"
        ))),
        FunctionOutcome::Err(err) => Err(JError::new(err.to_string())),
        FunctionOutcome::Ok(v) => {
            let result = serde_json::from_value::<T>(v)?;
            if result.is_success() {
                Ok(result)
            } else {
                Err(JError::new(format!(
                    "Result of a function {}.{} cannot be parsed to {}: {}",
                    spell_id,
                    function_name,
                    std::any::type_name::<T>(),
                    result.get_error()
                )))
            }
        }
    }
}

pub(crate) fn parse_spell_id_from(particle_id: &str) -> Result<&str, JError> {
    if particle_id.starts_with("spell_") {
        Ok(particle_id
            .split('_')
            .collect::<Vec<&str>>()
            .get(1)
            .ok_or(JError::new("Invalid particle id format"))?)
    } else {
        Err(JError::new(format!(
            "Expected spell particle id to start with 'spell_': {}",
            particle_id
        )))
    }
}
