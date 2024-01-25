mod peer_id;

use crate::peer_id::peer_id_serde;
use libp2p_identity::PeerId;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize, Debug)]
pub enum PeerScope {
    WorkerId(WorkerId),
    Host,
}

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug)]
pub struct WorkerId(PeerId);

impl From<PeerId> for WorkerId {
    fn from(value: PeerId) -> Self {
        WorkerId(value)
    }
}

impl From<WorkerId> for PeerId {
    fn from(value: WorkerId) -> Self {
        value.0
    }
}

impl fmt::Display for WorkerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.to_base58().fmt(f)
    }
}

impl Serialize for WorkerId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        Ok(peer_id_serde::serialize(&self.0, serializer)?)
    }
}

impl<'de> Deserialize<'de> for WorkerId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let peer_id: PeerId = peer_id_serde::deserialize(deserializer)?;
        Ok(WorkerId(peer_id))
    }
}

#[cfg(test)]
mod tests {}
