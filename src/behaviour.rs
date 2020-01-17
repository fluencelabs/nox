use libp2p_swarm::{NetworkBehaviour, OneShotHandler, NetworkBehaviourAction, PollParameters, ProtocolsHandler, NetworkBehaviourEventProcess};
use tokio::io::{AsyncWrite, AsyncRead};
use libp2p_swarm::protocols_handler::DummyProtocolsHandler;
use libp2p_core::{ConnectedPoint, Multiaddr};
use futures::Async;
use libp2p::PeerId;
use void::Void;
use libp2p_core::nodes::{Substream, ListenerId};
use std::marker::PhantomData;
use std::collections::VecDeque;
use std::error::Error;

#[derive(Debug, Clone)]
pub enum BehaviourEvent {
    Connected(PeerId),
    Disconnected(PeerId)
}

pub struct EventEmittingBehaviour<Substream> {
    events: VecDeque<NetworkBehaviourAction<Void, BehaviourEvent>>,
    marker: PhantomData<Substream>,
}

impl<Substream> EventEmittingBehaviour<Substream> {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
            marker: PhantomData,
        }
    }
}

impl<Substream: AsyncRead + AsyncWrite> NetworkBehaviour for EventEmittingBehaviour<Substream> {
    type ProtocolsHandler = DummyProtocolsHandler<Substream>;
    type OutEvent = BehaviourEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, peer_id: &PeerId) -> Vec<Multiaddr> {
        println!("addresses of peer: peer {:?}", peer_id);
        vec![]
    }

    fn inject_connected(&mut self, peer_id: PeerId, endpoint: ConnectedPoint) {
        println!("connected: peer {:?} endpoint {:?}", peer_id, endpoint);
        self.events.push_back(NetworkBehaviourAction::GenerateEvent(BehaviourEvent::Connected(peer_id)));
    }

    fn inject_disconnected(&mut self, peer_id: &PeerId, endpoint: ConnectedPoint) {
        println!("disconnected: peer {:?} endpoint {:?}", peer_id, endpoint);

        self.events.push_back(NetworkBehaviourAction::GenerateEvent(BehaviourEvent::Disconnected(peer_id.clone())));
    }

    fn inject_node_event(&mut self, peer_id: PeerId, _event: Void) {
        println!("inject_node_event: peer {:?}", peer_id)
    }

    fn poll(&mut self, _: &mut impl PollParameters) -> Async<NetworkBehaviourAction<
        <Self::ProtocolsHandler as ProtocolsHandler>::InEvent,
        Self::OutEvent>
    > {
        if let Some(event) = self.events.pop_front() {
            return Async::Ready(event);
        }

        Async::NotReady
    }

    fn inject_addr_reach_failure(&mut self, peer_id: Option<&PeerId>, addr: &Multiaddr, error: &Error) {
        println!("inject_addr_reach_failure peer_id: {:?}, addr: {:?}, error: {:?}", peer_id, addr, error)
    }

    fn inject_dial_failure(&mut self, peer_id: &PeerId) {
        println!("inject_dial_failure peer_id: {:?}", peer_id)
    }

    fn inject_new_listen_addr(&mut self, addr: &Multiaddr) {
        println!("inject_new_listen_addr addr: {:?}", addr)
    }

    fn inject_expired_listen_addr(&mut self, addr: &Multiaddr) {
        println!("inject_expired_listen_addr addr: {:?}", addr)
    }

    fn inject_new_external_addr(&mut self, addr: &Multiaddr) {
        println!("inject_new_external_addr addr: {:?}", addr)
    }

    fn inject_listener_error(&mut self, id: ListenerId, err: &Error) {
        println!("inject_listener_error id: {:?}, err: {:?}", id, err)
    }

    fn inject_listener_closed(&mut self, id: ListenerId) {
        println!("inject_listener_closed id: {:?}", id)
    }
}

//impl From<()> for InnerMessage {
//    #[inline]
//    fn from(_: ()) -> InnerMessage {
//        InnerMessage::Tx
//    }
//}