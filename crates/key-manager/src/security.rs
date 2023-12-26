use crate::KeyStorage;
use fluence_keypair::KeyPair;
use fluence_libp2p::PeerId;

pub struct Security {
    host_peer_id: PeerId,
    pub root_key_pair: KeyPair,
    management_peer_id: PeerId,
    builtins_management_peer_id: PeerId,
    key_storage: KeyStorage,
}
impl Security {
    pub fn is_local(&self, peer_id: PeerId) -> bool {
        self.is_host(peer_id) || self.is_worker(peer_id)
    }

    pub fn is_host(&self, peer_id: PeerId) -> bool {
        self.host_peer_id == peer_id
    }

    pub fn is_worker(&self, peer_id: PeerId) -> bool {
        self.key_storage.get_keypair(peer_id).is_some()
    }

    pub fn is_management(&self, peer_id: PeerId) -> bool {
        self.management_peer_id == peer_id || self.builtins_management_peer_id == peer_id
    }

    pub fn get_host_peer_id(&self) -> PeerId {
        self.host_peer_id
    }
}
