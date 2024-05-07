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
use std::sync::Arc;
use tracing::{instrument, Span};

use crate::error::SorcererError::{ParticleSigningFailed, ScopeKeypairMissing};
use crate::Sorcerer;
use fluence_libp2p::PeerId;
use now_millis::now_ms;
use particle_args::JError;
use particle_protocol::{ExtendedParticle, Particle};
use particle_services::PeerScope;
use spell_event_bus::api::{TriggerEvent, TriggerInfoAqua};
use spell_service_api::CallParams;

impl Sorcerer {
    async fn get_spell_counter(
        &self,
        peer_scope: PeerScope,
        spell_id: String,
    ) -> Result<u32, JError> {
        let init_peer_id = self.scopes.to_peer_id(peer_scope);
        let params = CallParams::local(
            peer_scope,
            spell_id,
            init_peer_id,
            self.spell_script_particle_ttl,
        );
        let counter = self.spell_service_api.get_counter(params).await?;
        // If the counter does not exist, consider it to be 0.
        // It will be incremented afterwards to 1 anyway.
        Ok(counter.unwrap_or(0u32))
    }

    async fn set_spell_next_counter(
        &self,
        peer_scope: PeerScope,
        spell_id: String,
        next_counter: u32,
    ) -> Result<(), JError> {
        let init_peer_id = self.scopes.to_peer_id(peer_scope);
        let params = CallParams::local(
            peer_scope,
            spell_id,
            init_peer_id,
            self.spell_script_particle_ttl,
        );
        self.spell_service_api
            .set_counter(params, next_counter)
            .await
            .map_err(|e| JError::new(e.to_string()))
    }

    async fn get_spell_script(
        &self,
        peer_scope: PeerScope,
        spell_id: String,
    ) -> Result<String, JError> {
        let init_peer_id = self.scopes.to_peer_id(peer_scope);
        let params = CallParams::local(
            peer_scope,
            spell_id,
            init_peer_id,
            self.spell_script_particle_ttl,
        );
        self.spell_service_api
            .get_script(params)
            .await
            .map_err(|e| JError::new(e.to_string()))
    }

    #[instrument(level = tracing::Level::INFO, skip_all)]
    pub(crate) async fn make_spell_particle(
        &self,
        peer_scope: PeerScope,
        spell_id: String,
    ) -> Result<Particle, JError> {
        let spell_keypair =
            self.key_storage
                .get_keypair(peer_scope)
                .ok_or(ScopeKeypairMissing {
                    spell_id: spell_id.clone(),
                    peer_scope,
                })?;

        let spell_counter = self.get_spell_counter(peer_scope, spell_id.clone()).await?;
        self.set_spell_next_counter(peer_scope, spell_id.clone(), spell_counter + 1)
            .await?;
        let spell_script = self.get_spell_script(peer_scope, spell_id.clone()).await?;
        let init_peer_id: PeerId = match peer_scope {
            PeerScope::WorkerId(worker_id) => worker_id.into(),
            PeerScope::Host => self.scopes.get_host_peer_id(),
        };

        let mut particle = Particle {
            id: f!("spell_{spell_id}_{spell_counter}"),
            init_peer_id,
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

    pub(crate) async fn store_trigger(
        &self,
        event: TriggerEvent,
        peer_scope: PeerScope,
    ) -> Result<(), JError> {
        let init_peer_id = self.scopes.to_peer_id(peer_scope);
        let serialized_event = serde_json::to_string(&TriggerInfoAqua::from(event.info))?;
        let params = CallParams::local(
            peer_scope,
            event.spell_id,
            init_peer_id,
            self.spell_script_particle_ttl,
        );
        self.spell_service_api
            .set_trigger_event(params, serialized_event)
            .await
            .map_err(|e| JError::new(e.to_string()))
    }

    #[instrument(level = tracing::Level::INFO, skip_all)]
    pub async fn execute_script(&self, event: TriggerEvent, span: Arc<Span>) {
        let error: Result<(), JError> = try {
            let peer_scope = self
                .spell_storage
                .get_scope(event.spell_id.clone())
                .expect("Scope not found");

            let particle = self
                .make_spell_particle(peer_scope, event.spell_id.clone())
                .await?;

            self.store_trigger(event.clone(), peer_scope).await?;
            if let Some(m) = &self.spell_metrics {
                m.observe_spell_cast();
            }

            self.aquamarine
                .clone()
                .execute(ExtendedParticle::linked(particle, span), None)
                .await?;
        };

        if let Err(err) = error {
            log::warn!(
                "Failed to execute spell script id: {spell_id}, event: {:?}, error: {:?}",
                event.info,
                err,
                spell_id = event.spell_id.to_string(),
            );
        }
    }
}
