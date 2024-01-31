use crate::KeyStorage;
use derivative::Derivative;
use fluence_libp2p::PeerId;
use std::sync::Arc;
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

    pub fn scope(&self, peer_id: PeerId) -> Option<PeerScope> {
        //TODO: maybe a result ?
        if self.host_peer_id == peer_id {
            Some(PeerScope::Host)
        } else {
            let worker_id: WorkerId = peer_id.into();
            if self
                .key_storage
                .get_worker_key_pair(peer_id.into())
                .is_some()
            {
                Some(PeerScope::WorkerId(worker_id))
            } else {
                None
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
}
