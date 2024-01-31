pub mod peer_id;

use libp2p_identity::PeerId;
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize, Debug)]
#[serde(tag = "scope_type", content = "scope_value")]
pub enum PeerScope {
    WorkerId(WorkerId),
    Host,
}

#[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd, Debug, Serialize, Deserialize)]
pub struct WorkerId(
    #[serde(
        serialize_with = "peer_id::serde::serialize",
        deserialize_with = "peer_id::serde::deserialize"
    )]
    PeerId,
);

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

#[cfg(test)]
mod tests {}
