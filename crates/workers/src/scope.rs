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

use crate::KeyStorage;
use derivative::Derivative;
use fluence_libp2p::PeerId;
use std::sync::Arc;
use thiserror::Error;
use types::peer_scope::{PeerScope, WorkerId};

/// Represents information about various peer IDs.
#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct PeerScopes {
    host_peer_id: PeerId,
    management_peer_id: PeerId,
    builtins_management_peer_id: PeerId,
    #[derivative(Debug = "ignore")]
    key_storage: Arc<KeyStorage>,
}

#[derive(Debug, Error)]
#[error("Scope for peer id {peer_id} not found")]
pub struct ScopeNotFound {
    peer_id: PeerId,
}

impl PeerScopes {
    pub fn new(
        host_peer_id: PeerId,
        management_peer_id: PeerId,
        builtins_management_peer_id: PeerId,
        key_storage: Arc<KeyStorage>,
    ) -> Self {
        Self {
            host_peer_id,
            management_peer_id,
            builtins_management_peer_id,
            key_storage,
        }
    }

    pub fn scope(&self, peer_id: PeerId) -> Result<PeerScope, ScopeNotFound> {
        if self.host_peer_id == peer_id {
            Ok(PeerScope::Host)
        } else {
            let worker_id: WorkerId = peer_id.into();
            if self
                .key_storage
                .get_worker_key_pair(peer_id.into())
                .is_some()
            {
                Ok(PeerScope::WorkerId(worker_id))
            } else {
                Err(ScopeNotFound { peer_id })
            }
        }
    }

    pub fn is_host(&self, peer_id: PeerId) -> bool {
        self.host_peer_id == peer_id
    }

    pub fn is_management(&self, peer_id: PeerId) -> bool {
        self.management_peer_id == peer_id || self.builtins_management_peer_id == peer_id
    }

    pub fn get_host_peer_id(&self) -> PeerId {
        self.host_peer_id
    }

    pub fn to_peer_id(&self, peer_scope: PeerScope) -> PeerId {
        match peer_scope {
            PeerScope::WorkerId(worker_id) => worker_id.into(),
            PeerScope::Host => self.get_host_peer_id(),
        }
    }
}
