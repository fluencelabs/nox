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
use fluence_spell_dtos::value::{ScriptValue, U32Value, UnitValue};
use serde_json::json;

use now_millis::now_ms;
use particle_args::JError;
use particle_protocol::Particle;
use spell_event_bus::api::{TriggerEvent, TriggerInfoAqua};

use crate::utils::process_func_outcome;
use crate::Sorcerer;

impl Sorcerer {
    fn get_spell_counter(&self, spell_id: String) -> Result<u32, JError> {
        let func_outcome = self.services.call_function(
            &spell_id,
            "get_u32",
            vec![json!("counter")],
            None,
            self.node_peer_id,
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

    fn set_spell_next_counter(&self, spell_id: String, next_counter: u32) -> Result<(), JError> {
        let func_outcome = self.services.call_function(
            &spell_id,
            "set_u32",
            vec![json!("counter"), json!(next_counter)],
            None,
            self.node_peer_id,
            self.spell_script_particle_ttl,
        );

        process_func_outcome::<UnitValue>(func_outcome, &spell_id, "set_u32").map(drop)
    }

    fn get_spell_script(&self, spell_id: String) -> Result<String, JError> {
        let func_outcome = self.services.call_function(
            &spell_id,
            "get_script_source_from_file",
            vec![],
            None,
            self.node_peer_id,
            self.spell_script_particle_ttl,
        );

        Ok(process_func_outcome::<ScriptValue>(
            func_outcome,
            &spell_id,
            "get_script_source_from_file",
        )?
        .source_code)
    }

    pub(crate) fn get_spell_particle(&self, spell_id: String) -> Result<Particle, JError> {
        let spell_counter = self.get_spell_counter(spell_id.clone())?;
        self.set_spell_next_counter(spell_id.clone(), spell_counter + 1)?;
        let spell_script = self.get_spell_script(spell_id.clone())?;

        Ok(Particle {
            id: f!("spell_{spell_id}_{spell_counter}"),
            init_peer_id: self.node_peer_id,
            timestamp: now_ms() as u64,
            ttl: self.spell_script_particle_ttl.as_millis() as u32,
            script: spell_script,
            signature: vec![],
            data: vec![],
        })
    }

    pub(crate) fn store_trigger(&self, event: TriggerEvent) -> Result<(), JError> {
        let serialized_event = serde_json::to_string(&TriggerInfoAqua::from(event.info))?;

        let func_outcome = self.services.call_function(
            &event.spell_id,
            "list_push_string",
            vec![json!("trigger_mailbox"), json!(serialized_event)],
            None,
            self.node_peer_id,
            self.spell_script_particle_ttl,
        );

        process_func_outcome::<UnitValue>(func_outcome, &event.spell_id, "list_push_string")
            .map(drop)
    }

    pub async fn execute_script(&self, event: TriggerEvent) {
        let error: Result<(), JError> = try {
            let particle = self.get_spell_particle(event.spell_id.clone())?;

            self.store_trigger(event.clone())?;
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
