use crate::KeyStorage;
use fluence_libp2p::PeerId;
use std::sync::Arc;

#[derive(Clone)]
pub struct ScopeHelper {
    host_peer_id: PeerId,
    management_peer_id: PeerId,
    builtins_management_peer_id: PeerId,
    key_storage: Arc<KeyStorage>,
}
impl ScopeHelper {
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
