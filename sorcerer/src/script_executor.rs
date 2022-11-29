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

use eyre::eyre;
use serde::Deserialize;
use serde_json::json;

use crate::Sorcerer;
use now_millis::now_ms;
use particle_execution::FunctionOutcome;
use particle_protocol::Particle;

#[derive(Deserialize)]
struct UnitValue {
    pub success: bool,
    pub error: String,
}

#[derive(Deserialize)]
struct U32Value {
    pub num: u32,
    pub success: bool,
    pub error: String,
}

#[derive(Deserialize)]
pub struct Script {
    pub source_code: String,
    pub success: bool,
    pub error: String,
}

impl Sorcerer {
    fn get_spell_counter(&self, spell_id: String) -> eyre::Result<u32> {
        let func_outcome = self.services.call_function(
            spell_id,
            "get_u32",
            vec![json!("counter")],
            self.node_peer_id,
            self.spell_script_particle_ttl,
        );

        match func_outcome {
            FunctionOutcome::NotDefined { args, .. } => Err(eyre!(format!(
                "Service with id '{}' not found (function {})",
                args.service_id, args.function_name
            ))),
            FunctionOutcome::Empty => Err(eyre!("Function get_u32 has not returned any result")),
            FunctionOutcome::Err(err) => Err(eyre!(err.to_string())),
            FunctionOutcome::Ok(v) => {
                let result: U32Value = serde_json::from_value(v)?;
                if result.success {
                    Ok(result.num)
                } else {
                    // If key is not exists we will create it on the next step
                    Ok(0u32)
                }
            }
        }
    }

    fn set_spell_next_counter(&self, spell_id: String, next_counter: u32) -> eyre::Result<()> {
        let func_outcome = self.services.call_function(
            spell_id,
            "set_u32",
            vec![json!("counter"), json!(next_counter)],
            self.node_peer_id,
            self.spell_script_particle_ttl,
        );

        match func_outcome {
            FunctionOutcome::NotDefined { args, .. } => Err(eyre!(format!(
                "Service with id '{}' not found (function {})",
                args.service_id, args.function_name
            ))),
            FunctionOutcome::Empty => Err(eyre!("Function set_u32 has not returned any result")),
            FunctionOutcome::Err(err) => Err(eyre!(err.to_string())),
            FunctionOutcome::Ok(v) => {
                let result: UnitValue = serde_json::from_value(v)?;

                if result.success {
                    Ok(())
                } else {
                    Err(eyre!(result.error))
                }
            }
        }
    }

    fn get_spell_script(&self, spell_id: String) -> eyre::Result<String> {
        let func_outcome = self.services.call_function(
            spell_id,
            "get_script_source_from_file",
            vec![],
            self.node_peer_id,
            self.spell_script_particle_ttl,
        );

        match func_outcome {
            FunctionOutcome::NotDefined { args, .. } => Err(eyre!(format!(
                "Service with id '{}' not found (function {})",
                args.service_id, args.function_name
            ))),
            FunctionOutcome::Empty => Err(eyre!(
                "Function get_script_source_from_file has not returned any result"
            )),
            FunctionOutcome::Err(err) => Err(eyre!(err.to_string())),
            FunctionOutcome::Ok(v) => {
                let result: Script = serde_json::from_value(v)?;

                if result.success {
                    Ok(result.source_code)
                } else {
                    Err(eyre!(result.error))
                }
            }
        }
    }

    pub(crate) fn get_spell_particle(&self, spell_id: String) -> eyre::Result<Particle> {
        let spell_counter = self.get_spell_counter(spell_id.clone())?;
        self.set_spell_next_counter(spell_id.clone(), spell_counter + 1)?;
        let spell_script = self.get_spell_script(spell_id.clone())?;

        Ok(Particle {
            id: f!("spell_{spell_id}_{spell_counter}"),
            init_peer_id: self.node_peer_id.clone(),
            timestamp: now_ms() as u64,
            ttl: self.spell_script_particle_ttl.as_millis() as u32,
            script: spell_script,
            signature: vec![],
            data: vec![],
        })
    }

    pub async fn execute_script(&self, spell_id: String) {
        match self.get_spell_particle(spell_id) {
            Ok(particle) => {
                self.aquamarine
                    .clone()
                    .execute(particle, None)
                    // do not log errors: Aquamarine will log them fine
                    .await
                    .ok();
            }
            Err(err) => {
                log::warn!("Cannot obtain spell particle: {:?}", err);
            }
        }
    }
}
