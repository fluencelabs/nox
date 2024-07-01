/*
 * Nox Fluence Peer
 *
 * Copyright (C) 2024 Fluence DAO
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation version 3 of the
 * License.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use futures::{stream::iter, StreamExt};
use tracing::instrument;

use aquamarine::RemoteRoutingEffects;
use particle_protocol::Particle;

use crate::connectivity::Connectivity;

#[derive(Clone)]
pub struct Effectors {
    pub connectivity: Connectivity,
}

impl Effectors {
    pub fn new(connectivity: Connectivity) -> Self {
        Self { connectivity }
    }

    /// Perform effects that Aquamarine instructed us to
    #[instrument(level = tracing::Level::INFO, skip_all)]
    pub async fn execute(self, effects: RemoteRoutingEffects) {
        let particle: &Particle = effects.particle.as_ref();
        if particle.is_expired() {
            tracing::info!(target: "expired", particle_id = particle.id, "Particle is expired");
            return;
        }

        // take every next peers, and try to send particle there concurrently
        let nps = iter(effects.next_peers);
        let particle = &effects.particle;
        let connectivity = self.connectivity.clone();
        nps.for_each_concurrent(None, move |target| {
            let connectivity = connectivity.clone();
            let particle = particle.clone();
            async move {
                // resolve contact
                if let Some(contact) = connectivity
                    .resolve_contact(target, particle.as_ref())
                    .await
                {
                    // forward particle
                    let sent = connectivity.send(contact, particle).await;
                    if sent {
                        // resolved and sent, exit
                    }
                }
            }
        })
        .await;
    }
}
