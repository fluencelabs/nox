use crate::KeyStorage;
use derivative::Derivative;
use fluence_libp2p::PeerId;
use std::sync::Arc;

#[derive(Clone, Derivative)]
#[derivative(Debug)]
pub struct Scope {
    host_peer_id: PeerId,
    management_peer_id: PeerId,
    builtins_management_peer_id: PeerId,
    #[derivative(Debug = "ignore")]
    key_storage: Arc<KeyStorage>,
}
impl Scope {
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

    pub fn is_local(&self, peer_id: PeerId) -> bool {
        self.is_host(peer_id) || self.is_worker(peer_id)
    }

    pub fn is_host(&self, peer_id: PeerId) -> bool {
        self.host_peer_id == peer_id
    }

    pub fn is_worker(&self, peer_id: PeerId) -> bool {
        self.key_storage.get_key_pair(peer_id).is_some()
    }

    pub fn is_management(&self, peer_id: PeerId) -> bool {
        self.management_peer_id == peer_id || self.builtins_management_peer_id == peer_id
    }

    pub fn get_host_peer_id(&self) -> PeerId {
        self.host_peer_id
    }
}
