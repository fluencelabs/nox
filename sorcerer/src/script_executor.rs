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
use maplit::hashmap;
use serde_json::json;

use crate::Sorcerer;
use connection_pool::ConnectionPoolApi;
use kademlia::KademliaApi;
use now_millis::now_ms;
use particle_args::Args;
use particle_execution::{FunctionOutcome, ParticleParams};
use particle_protocol::Particle;
use particle_services::ParticleAppServices;
use uuid_utils::uuid;

// TODO: use some meaningful ttl
pub static PARTICLE_TTL: u32 = 20000;

impl<C> Sorcerer<C>
where
    C: Clone + Send + Sync + 'static + AsRef<KademliaApi> + AsRef<ConnectionPoolApi>,
{
    fn get_spell_counter(&self, spell_id: String) -> eyre::Result<u32> {
        let spell_args = Args {
            service_id: spell_id.clone(),
            function_name: "get_u32".to_string(),
            function_args: vec![json!("counter")],
            tetraplets: vec![],
        };

        // TODO: pass and use actual spell keypair
        let particle = ParticleParams {
            id: uuid(),
            init_peer_id: self.node_peer_id.clone(),
            timestamp: now_ms() as u64,
            ttl: PARTICLE_TTL,
            script: "".to_string(),
            signature: vec![],
        };
        let func_outcome = self.builtins.services.call_service(spell_args, particle);

        match func_outcome {
            FunctionOutcome::NotDefined { args, .. } => Err(eyre!(format!(
                "Service with id '{}' not found (function {})",
                args.service_id, args.function_name
            ))),
            FunctionOutcome::Empty => Err(eyre!("Function get_u32 has not returned any result")),
            FunctionOutcome::Err(err) => Err(eyre!(err.to_string())),
            FunctionOutcome::Ok(v) => {
                let success = v["success"]
                    .as_bool()
                    .ok_or(eyre!("Field success is missing in get_u32 result"))?;
                let num = v["num"]
                    .as_u64()
                    .ok_or(eyre!("Field num is missing in get_u32 result"))?
                    as u32;

                if success {
                    Ok(num)
                } else {
                    // If key is not exists we will create it on the next step
                    Ok(0u32)
                }
            }
        }
    }

    fn set_spell_next_counter(&self, spell_id: String, next_counter: u32) -> eyre::Result<()> {
        let spell_args = Args {
            service_id: spell_id.clone(),
            function_name: "set_u32".to_string(),
            function_args: vec![json!("counter"), json!(next_counter)],
            tetraplets: vec![],
        };

        // TODO: pass and use actual spell keypair
        let particle = ParticleParams {
            id: uuid(),
            init_peer_id: self.node_peer_id.clone(),
            timestamp: now_ms() as u64,
            ttl: PARTICLE_TTL,
            script: "".to_string(),
            signature: vec![],
        };
        let func_outcome = self.builtins.services.call_service(spell_args, particle);

        match func_outcome {
            FunctionOutcome::NotDefined { args, .. } => Err(eyre!(format!(
                "Service with id '{}' not found (function {})",
                args.service_id, args.function_name
            ))),
            FunctionOutcome::Empty => Err(eyre!("Function set_u32 has not returned any result")),
            FunctionOutcome::Err(err) => Err(eyre!(err.to_string())),
            FunctionOutcome::Ok(v) => {
                let success = v["success"]
                    .as_bool()
                    .ok_or(eyre!("Field success is missing in set_u32 result"))?;
                let error = v["error"]
                    .as_str()
                    .ok_or(eyre!("Field error is missing in set_u32 result"))?
                    .to_string();

                if success {
                    Ok(())
                } else {
                    Err(eyre!(error))
                }
            }
        }
    }

    fn get_spell_script(&self, spell_id: String) -> eyre::Result<String> {
        let spell_args = Args {
            service_id: spell_id.clone(),
            function_name: "get_script_source_from_file".to_string(),
            function_args: vec![],
            tetraplets: vec![],
        };

        // TODO: pass and use actual spell keypair
        let particle = ParticleParams {
            id: uuid(),
            init_peer_id: self.node_peer_id.clone(),
            timestamp: now_ms() as u64,
            ttl: PARTICLE_TTL,
            script: "".to_string(),
            signature: vec![],
        };
        let func_outcome = self.builtins.services.call_service(spell_args, particle);

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
                let success = v["success"].as_bool().ok_or(eyre!(
                    "Field success is missing in get_script_source_from_file result"
                ))?;
                let error = v["error"]
                    .as_str()
                    .ok_or(eyre!(
                        "Field error is missing in get_script_source_from_file result"
                    ))?
                    .to_string();
                let script = v["script"]
                    .as_str()
                    .ok_or(eyre!(
                        "Field script is missing in get_script_source_from_file result"
                    ))?
                    .to_string();

                if success {
                    Ok(script)
                } else {
                    Err(eyre!(error))
                }
            }
        }
    }

    fn get_spell_particle(&self, spell_id: String) -> eyre::Result<Particle> {
        let spell_counter = self.get_spell_counter(spell_id.clone())?;
        self.set_spell_next_counter(spell_id.clone(), spell_counter + 1)?;
        let spell_script = self.get_spell_script(spell_id.clone())?;

        Ok(Particle {
            id: f!("spell_{spell_id}_{spell_counter}"),
            init_peer_id: self.node_peer_id.clone(),
            timestamp: now_ms() as u64,
            ttl: PARTICLE_TTL,
            script: spell_script,
            signature: vec![],
            data: vec![],
        })
    }

    pub async fn execute_script(&self, spell_id: String) -> eyre::Result<()> {
        let particle = self.get_spell_particle(spell_id)?;

        // Should we store future? How can we handle errors?
        self.aquamarine.clone().execute(particle, None).await?;

        Ok(())
    }
}
