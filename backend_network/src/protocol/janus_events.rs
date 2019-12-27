use libp2p::{PeerId, gossipsub::MessageId};

/// Events of the Janus service
pub enum JanusEvent {
    /// A new peer has been dialed
    PeerDialed(PeerId),

    /// A peer has disconnected
    PeerAborted(PeerId),

    /// A new node has been dialed
    NodeDialed(PeerId),

    /// A node has disconnected
    NodeAborted(PeerId),

    /// A UTF-8 string message relaying from one node to other
    Message(String),

    /// PubSub message
    PubsubMessage {
        id: MessageId,
        source: PeerId,
        topics: Vec<String>,
        message: String,
    },
}
