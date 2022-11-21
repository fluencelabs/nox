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
use fluence_libp2p::RandomPeerId;
use now_millis::now_ms;
use particle_args::Args;
use particle_execution::{FunctionOutcome, ParticleParams};
use particle_protocol::Particle;
use particle_services::ParticleAppServices;
use serde_json::json;
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

    if let FunctionOutcome::Ok(res) = func_outcome {
        if res["success"].as_bool().unwrap() {
            res["num"].as_u64().map(|n| n as u32).ok_or(eyre!(""))
        } else {
            Ok(0u32)
        }
    } else {
        Err(eyre!("get_spell_counter failed"))
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

    if let FunctionOutcome::Ok(res) = func_outcome {
        if res["success"].as_bool().unwrap() {
            Ok(())
        } else {
            Err(eyre!(res["error"].as_str().ok_or(eyre!(""))?))
        }
    } else {
        Err(eyre!("set_spell_next_counter failed"))
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

    if let FunctionOutcome::Ok(res) = func_outcome {
        if res["success"].as_bool().unwrap() {
            res["source_code"]
                .as_str()
                .ok_or(eyre!(""))
                .map(|s| s.to_string())
        } else {
            Err(eyre!(res["error"].as_str().ok_or(eyre!(""))?))
        }
    } else {
        Err(eyre!("get_spell_script failed"))
    }
}

fn get_spell_particle(services: ParticleAppServices, spell_id: String) -> eyre::Result<Particle> {
    let spell_counter = get_spell_counter(services.clone(), spell_id.clone())?;
    set_spell_next_counter(services.clone(), spell_id.clone(), spell_counter + 1)?;
    let spell_script = get_spell_script(services.clone(), spell_id.clone())?;

    // TODO: pass spell_id and spell peer id to data
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
fn execute_script() {
    // TODO: send particle to connection pool
}
