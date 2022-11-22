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
use fluence_libp2p::RandomPeerId;
use now_millis::now_ms;
use particle_args::Args;
use particle_execution::{FunctionOutcome, ParticleParams};
use particle_protocol::Particle;
use particle_services::ParticleAppServices;
use serde_json::json;
use connection_pool::ConnectionPoolApi;
use uuid_utils::uuid;

// TODO: use some meaningful ttl
pub static PARTICLE_TTL: u32 = 20000;

fn get_spell_counter(services: ParticleAppServices, spell_id: String) -> eyre::Result<u32> {
    let spell_args = Args {
        service_id: spell_id.clone(),
        function_name: "get_u32".to_string(),
        function_args: vec![json!("counter")],
        tetraplets: vec![],
    };

    // TODO: pass and use actual spell keypair
    let particle = ParticleParams {
        id: uuid(),
        init_peer_id: RandomPeerId::random(),
        timestamp: now_ms() as u64,
        ttl: PARTICLE_TTL,
        script: "".to_string(),
        signature: vec![],
    };
    let func_outcome = services.call_service(spell_args, particle);

    match func_outcome {
        FunctionOutcome::NotDefined { args, .. } => Err(eyre!(format!(
            "Service with id '{}' not found (function {})",
            args.service_id, args.function_name
        ))),
        FunctionOutcome::Empty => Err(eyre!("Function get_u32 has not returned any result")),
        FunctionOutcome::Err(err) => Err(eyre!(err.to_string())),
        FunctionOutcome::Ok(v) => {
            let success = v["success"].as_bool().ok_or(eyre!("Field success is missing in get_u32 result"))?;
            let num = v["num"].as_u64().ok_or(eyre!("Field num is missing in get_u32 result"))? as u32;

            if success {
                Ok(num)
            } else {
                // If key is not exists we will create it on the next step
                Ok(0u32)
            }
        }
    }
}

fn set_spell_next_counter(
    services: ParticleAppServices,
    spell_id: String,
    next_counter: u32,
) -> eyre::Result<()> {
    let spell_args = Args {
        service_id: spell_id.clone(),
        function_name: "set_u32".to_string(),
        function_args: vec![json!("counter"), json!(next_counter)],
        tetraplets: vec![],
    };

    // TODO: pass and use actual spell keypair
    let particle = ParticleParams {
        id: uuid(),
        init_peer_id: RandomPeerId::random(),
        timestamp: now_ms() as u64,
        ttl: PARTICLE_TTL,
        script: "".to_string(),
        signature: vec![],
    };
    let func_outcome = services.call_service(spell_args, particle);

    match func_outcome {
        FunctionOutcome::NotDefined { args, .. } => Err(eyre!(format!(
            "Service with id '{}' not found (function {})",
            args.service_id, args.function_name
        ))),
        FunctionOutcome::Empty => Err(eyre!("Function set_u32 has not returned any result")),
        FunctionOutcome::Err(err) => Err(eyre!(err.to_string())),
        FunctionOutcome::Ok(v) => {
            let success = v["success"].as_bool().ok_or(eyre!("Field success is missing in set_u32 result"))?;
            let error = v["error"].as_str().ok_or(eyre!("Field error is missing in set_u32 result"))?.to_string();

            if success {
                Ok(())
            } else {
                Err(eyre!(error))
            }
        }
    }
}

fn get_spell_script(services: ParticleAppServices, spell_id: String) -> eyre::Result<String> {
    let spell_args = Args {
        service_id: spell_id.clone(),
        function_name: "get_script_source_from_file".to_string(),
        function_args: vec![],
        tetraplets: vec![],
    };

    // TODO: pass and use actual spell keypair
    let particle = ParticleParams {
        id: uuid(),
        init_peer_id: RandomPeerId::random(),
        timestamp: now_ms() as u64,
        ttl: PARTICLE_TTL,
        script: "".to_string(),
        signature: vec![],
    };
    let func_outcome = services.call_service(spell_args, particle);

    match func_outcome {
        FunctionOutcome::NotDefined { args, .. } => Err(eyre!(format!(
            "Service with id '{}' not found (function {})",
            args.service_id, args.function_name
        ))),
        FunctionOutcome::Empty => Err(eyre!("Function get_script_source_from_file has not returned any result")),
        FunctionOutcome::Err(err) => Err(eyre!(err.to_string())),
        FunctionOutcome::Ok(v) => {
            let success = v["success"].as_bool().ok_or(eyre!("Field success is missing in get_script_source_from_file result"))?;
            let error = v["error"].as_str().ok_or(eyre!("Field error is missing in get_script_source_from_file result"))?.to_string();
            let script = v["script"].as_str().ok_or(eyre!("Field script is missing in get_script_source_from_file result"))?.to_string();

            if success {
                Ok(script)
            } else {
                Err(eyre!(error))
            }
        }
    }
}

fn get_spell_particle(services: ParticleAppServices, spell_id: String) -> eyre::Result<Particle> {
    let spell_counter = get_spell_counter(services.clone(), spell_id.clone())?;
    set_spell_next_counter(services.clone(), spell_id.clone(), spell_counter + 1)?;
    let spell_script = get_spell_script(services.clone(), spell_id.clone())?;


    Ok(Particle {
        id: f!("spell_{spell_id}_{spell_counter}"),
        init_peer_id: RandomPeerId::random(),
        timestamp: now_ms() as u64,
        ttl: PARTICLE_TTL,
        script: spell_script,
        signature: vec![],
        data: vec![],
    })
}

fn execute_script(pool: ConnectionPoolApi, services: ParticleAppServices, spell_id: String) -> eyre::Result<()> {
    let particle = get_spell_particle(services, spell_id)?;



    Ok(())
}
