/*
 * Copyright 2020 Fluence Labs Limited
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

use crate::bootstrapper::event::BootstrapperEvent;
use fluence_libp2p::{event_polling, generate_swarm_event_type};
use libp2p::core::connection::{ConnectedPoint, ConnectionId};
use libp2p::swarm::{
    protocols_handler::DummyProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction,
};
use libp2p::PeerId;
use parity_multiaddr::Multiaddr;
use std::collections::{HashMap, HashSet, VecDeque};
use std::error::Error;
use std::mem;
use std::time::{Duration, Instant};

pub type SwarmEventType = generate_swarm_event_type!(Bootstrapper);
// TODO: make it exponential
static RECONNECT_DELAY: Duration = Duration::from_millis(1500);
static BOOTSTRAP_DELAY: Duration = Duration::from_millis(10000);
static BOOTSTRAP_MAX_DELAY: Duration = Duration::from_secs(60);

#[derive(Default, Debug)]
struct Backoff(u32);
impl Backoff {
    pub fn next_delay(&mut self) -> Duration {
        self.0 += 1;
        self.0 * RECONNECT_DELAY
    }
}

pub struct Bootstrapper {
    peer_id: PeerId,
    pub bootstrap_nodes: HashSet<Multiaddr>,
    delayed_events: Vec<(Option<Instant>, SwarmEventType)>,
    events: VecDeque<SwarmEventType>,
    bootstrap_scheduled: Option<(Instant, Duration)>,
    bootstrap_backoff: HashMap<Multiaddr, Backoff>,
}

impl Bootstrapper {
    pub fn new(peer_id: PeerId, bootstrap_nodes: Vec<Multiaddr>) -> Self {
        Self {
            peer_id,
            bootstrap_nodes: bootstrap_nodes.into_iter().collect(),
            delayed_events: Default::default(),
            events: Default::default(),
            bootstrap_scheduled: None,
            bootstrap_backoff: Default::default(),
        }
    }

    fn push_event(&mut self, event: BootstrapperEvent, delay: Option<Duration>) {
        let event = NetworkBehaviourAction::GenerateEvent(event);
        let deadline = delay.map(|d| Instant::now() + d);
        self.delayed_events.push((deadline, event));
    }

    fn reconnect_bootstrap<E, S>(&mut self, multiaddr: Multiaddr, error: E)
    where
        S: Into<String>,
        E: Into<Option<S>>,
    {
        let delay = self
            .bootstrap_backoff
            .entry(multiaddr.clone())
            .or_insert(Backoff::default())
            .next_delay();

        self.push_event(
            BootstrapperEvent::ReconnectToBootstrap {
                multiaddr,
                error: error.into().map(|e| e.into()),
            },
            Some(delay),
        );
    }

    /// Schedule sending of `RunBootstrap` event after a `BOOTSTRAP_DELAY`
    fn schedule_bootstrap(&mut self) {
        match self.bootstrap_scheduled {
            Some((scheduled, mut delay)) if delay < BOOTSTRAP_MAX_DELAY => {
                // Delay bootstrap by `elapsed`
                delay += scheduled.elapsed()
            }
            Some(_) => { /* maximum delay reached */ }
            mut empty => {
                empty.replace((Instant::now(), BOOTSTRAP_DELAY));
            }
        };
    }

    /// Send `RunBootstrap` if delay is reached
    fn trigger_bootstrap(&mut self, now: Instant) {
        if let Some(&(scheduled, delay)) = self.bootstrap_scheduled.as_ref() {
            if now >= scheduled + delay {
                self.push_event(BootstrapperEvent::RunBootstrap, None)
            }
        }
    }

    /// Send delayed events for which delay was reached
    fn complete_delayed(&mut self, now: Instant) {
        let delayed = mem::replace(&mut self.delayed_events, vec![]);

        let (ready, not_ready) = delayed.into_iter().partition(|(deadline, _)| {
            let ready = deadline.map(|d| d >= now).unwrap_or(true);
            ready
        });

        self.delayed_events = not_ready;
        self.events = ready.into_iter().map(|(_, e)| e).collect();
    }

    /// Called on each poll
    fn on_poll(&mut self) {
        let now = Instant::now();

        self.complete_delayed(now);
        self.trigger_bootstrap(now);
    }
}

impl NetworkBehaviour for Bootstrapper {
    type ProtocolsHandler = DummyProtocolsHandler;
    type OutEvent = BootstrapperEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        Default::default()
    }

    fn addresses_of_peer(&mut self, _: &PeerId) -> Vec<Multiaddr> {
        vec![]
    }

    fn inject_connected(&mut self, _: &PeerId) {}

    fn inject_disconnected(&mut self, _: &PeerId) {}

    fn inject_connection_established(
        &mut self,
        peer_id: &PeerId,
        _: &ConnectionId,
        cp: &ConnectedPoint,
    ) {
        let maddr = match cp {
            ConnectedPoint::Dialer { address } => address,
            ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr,
        };

        log::debug!(
            "{} connection established with {} {:?}",
            self.peer_id,
            peer_id,
            maddr
        );

        if self.bootstrap_nodes.contains(maddr) {
            self.schedule_bootstrap();
        }
    }

    fn inject_connection_closed(&mut self, _: &PeerId, _: &ConnectionId, cp: &ConnectedPoint) {
        let maddr = match cp {
            ConnectedPoint::Dialer { address } => address,
            ConnectedPoint::Listener { send_back_addr, .. } => send_back_addr,
        };

        if self.bootstrap_nodes.contains(maddr) {
            self.reconnect_bootstrap(maddr.clone(), "connection was closed");
        }
    }

    fn inject_event(&mut self, _: PeerId, _: ConnectionId, _: void::Void) {}

    fn inject_addr_reach_failure(
        &mut self,
        _: Option<&PeerId>,
        maddr: &Multiaddr,
        error: &dyn Error,
    ) {
        if self.bootstrap_nodes.contains(maddr) {
            self.reconnect_bootstrap(maddr.clone(), format!("{:?}", error));
        }
    }

    event_polling!(poll, events, SwarmEventType, on_poll);
}
