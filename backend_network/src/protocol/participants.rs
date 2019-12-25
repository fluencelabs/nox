use libp2p::PeerId;

struct Node {
    peer_id: PeerId,
}

struct Peer {
    peer_id: PeerId,
    nodes: Vec<Node>,
}
