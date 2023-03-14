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
use fluence_libp2p::PeerId;
use fluence_spell_dtos::value::{ScriptValue, U32Value, UnitValue};
use serde_json::json;

use crate::error::SorcererError::{ParticleSigningFailed, ScopeKeypairMissing};
use now_millis::now_ms;
use particle_args::JError;
use particle_protocol::Particle;
use spell_event_bus::api::{TriggerEvent, TriggerInfoAqua};

use crate::utils::process_func_outcome;
use crate::Sorcerer;

impl Sorcerer {
    fn get_spell_counter(&self, spell_id: String, worker_id: PeerId) -> Result<u32, JError> {
        let func_outcome = self.services.call_function(
            worker_id,
            &spell_id,
            "get_u32",
            vec![json!("counter")],
            None,
            worker_id,
            self.spell_script_particle_ttl,
        );

        let res = process_func_outcome::<U32Value>(func_outcome, &spell_id, "get_u32");
        match res {
            // If the counter does not exist, consider it to be 0.
            // It will be incremented afterwards to 1 anyway.
            Ok(res) if res.absent => Ok(0u32),
            Ok(res) => Ok(res.num),
            Err(err) => {
                log::warn!("Error on get_u32 counter for spell {}: {}", spell_id, err);
                Err(err)
            }
        }
    }

    fn set_spell_next_counter(
        &self,
        spell_id: String,
        next_counter: u32,
        worker_id: PeerId,
    ) -> Result<(), JError> {
        let func_outcome = self.services.call_function(
            worker_id,
            &spell_id,
            "set_u32",
            vec![json!("counter"), json!(next_counter)],
            None,
            worker_id,
            self.spell_script_particle_ttl,
        );

        process_func_outcome::<UnitValue>(func_outcome, &spell_id, "set_u32").map(drop)
    }

    fn get_spell_script(&self, spell_id: String, worker_id: PeerId) -> Result<String, JError> {
        let func_outcome = self.services.call_function(
            worker_id,
            &spell_id,
            "get_script_source_from_file",
            vec![],
            None,
            worker_id,
            self.spell_script_particle_ttl,
        );

        Ok(process_func_outcome::<ScriptValue>(
            func_outcome,
            &spell_id,
            "get_script_source_from_file",
        )?
        .source_code)
    }

    pub(crate) fn make_spell_particle(
        &self,
        spell_id: String,
        worker_id: PeerId,
    ) -> Result<Particle, JError> {
        let spell_keypair = self
            .key_manager
            .get_worker_keypair(worker_id)
            .map_err(|err| ScopeKeypairMissing {
                err,
                spell_id: spell_id.clone(),
            })?;

        let spell_counter = self.get_spell_counter(spell_id.clone(), worker_id)?;
        self.set_spell_next_counter(spell_id.clone(), spell_counter + 1, worker_id)?;
        let spell_script = self.get_spell_script(spell_id.clone(), worker_id)?;

        let mut particle = Particle {
            id: f!("spell_{spell_id}_{spell_counter}"),
            init_peer_id: worker_id,
            timestamp: now_ms() as u64,
            ttl: self.spell_script_particle_ttl.as_millis() as u32,
            script: spell_script,
            signature: vec![],
            data: vec![],
        };
        particle
            .sign(&spell_keypair)
            .map_err(|err| ParticleSigningFailed { err, spell_id })?;

        Ok(particle)
    }

    pub(crate) fn store_trigger(
        &self,
        event: TriggerEvent,
        worker_id: PeerId,
    ) -> Result<(), JError> {
        let serialized_event = serde_json::to_string(&TriggerInfoAqua::from(event.info))?;

        let func_outcome = self.services.call_function(
            worker_id,
            &event.spell_id,
            "list_push_string",
            vec![json!("trigger_mailbox"), json!(serialized_event)],
            None,
            worker_id,
            self.spell_script_particle_ttl,
        );

        process_func_outcome::<UnitValue>(func_outcome, &event.spell_id, "list_push_string")
            .map(drop)
    }

    pub async fn execute_script(&self, event: TriggerEvent) {
        let error: Result<(), JError> = try {
            let worker_id = self
                .services
                .get_service_owner(event.spell_id.clone(), self.key_manager.get_host_peer_id())?;
            let particle = self.make_spell_particle(event.spell_id.clone(), worker_id)?;

            self.store_trigger(event.clone(), worker_id)?;
            self.aquamarine.clone().execute(particle, None).await?;
        };

        if let Err(err) = error {
            log::warn!(
                "Failed to execute spell script id: {}, event: {:?}, error: {:?}",
                event.spell_id,
                event.info,
                err
            );
        }
    }
}
