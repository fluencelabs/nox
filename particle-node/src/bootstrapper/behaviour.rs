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

use fluence_libp2p::{event_polling, generate_swarm_event_type, remote_multiaddr};
use server_config::BootstrapConfig;

use libp2p::{
    core::{
        connection::{ConnectedPoint, ConnectionId},
        Multiaddr,
    },
    swarm::{protocols_handler::DummyProtocolsHandler, NetworkBehaviour, NetworkBehaviourAction},
    PeerId,
};
use std::{
    collections::{HashMap, HashSet, VecDeque},
    error::Error,
    mem,
    time::{Duration, Instant},
};

pub type SwarmEventType = generate_swarm_event_type!(Bootstrapper);

#[derive(Default, Debug)]
struct Backoff(u32, Duration);
impl Backoff {
    pub fn new(delay: Duration) -> Self {
        Backoff(0, delay)
    }

    pub fn next_delay(&mut self) -> Duration {
        self.0 += 1;
        self.0 * self.1
    }
}

pub struct Bootstrapper {
    config: BootstrapConfig,
    peer_id: PeerId,
    pub bootstrap_nodes: HashSet<Multiaddr>,
    delayed_events: Vec<(Option<Instant>, SwarmEventType)>,
    events: VecDeque<SwarmEventType>,
    bootstrap_scheduled: Option<(Instant, Duration)>,
    bootstrap_backoff: HashMap<Multiaddr, Backoff>,
}

impl Bootstrapper {
    pub fn new(config: BootstrapConfig, peer_id: PeerId, bootstrap_nodes: Vec<Multiaddr>) -> Self {
        let mut this = Self {
            config,
            peer_id,
            bootstrap_nodes: bootstrap_nodes.into_iter().collect(),
            delayed_events: <_>::default(),
            events: <_>::default(),
            bootstrap_scheduled: None,
            bootstrap_backoff: <_>::default(),
        };

        this.dial_bootstrap_nodes();

        this
    }

    fn dial_bootstrap_nodes(&mut self) {
        for addr in self.bootstrap_nodes.iter() {
            self.events.push_back(NetworkBehaviourAction::DialAddress {
                address: addr.clone(),
            })
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
        let delay = self.config.reconnect_delay;
        let delay = self
            .bootstrap_backoff
            .entry(multiaddr.clone())
            .or_insert_with(|| Backoff::new(delay))
            .next_delay();

        self.push_event(
            BootstrapperEvent::ReconnectToBootstrap {
                multiaddr,
                error: error.into().map(|e| e.into()),
            },
            Some(delay),
        );
    }

    /// Schedule sending of `RunBootstrap` event after a `bootstrap_delay`
    fn schedule_bootstrap(&mut self) {
        match self.bootstrap_scheduled {
            Some((scheduled, mut delay)) if delay < self.config.bootstrap_max_delay => {
                // Delay bootstrap by `elapsed`
                delay += scheduled.elapsed()
            }
            Some(_) => { /* maximum delay reached */ }
            None => {
                self.bootstrap_scheduled
                    .replace((Instant::now(), self.config.bootstrap_delay));
            }
        };
    }

    /// Send `RunBootstrap` if delay is reached
    fn trigger_bootstrap(&mut self, now: Instant) {
        if let Some(&(scheduled, delay)) = self.bootstrap_scheduled.as_ref() {
            if now >= scheduled + delay {
                self.push_event(BootstrapperEvent::RunBootstrap, None);
                self.bootstrap_scheduled = None;
            }
        }
    }

    /// Send delayed events for which delay was reached
    fn complete_delayed(&mut self, now: Instant) {
        let delayed = mem::replace(&mut self.delayed_events, vec![]);

        let (ready, not_ready) = delayed
            .into_iter()
            .partition(|(deadline, _)| deadline.map(|d| deadline_reached(d, now)).unwrap_or(true));

        self.delayed_events = not_ready;
        self.events.extend(ready.into_iter().map(|(_, e)| e));
    }

    /// Called on each poll
    fn on_poll(&mut self) {
        let now = Instant::now();

        self.complete_delayed(now);
        self.trigger_bootstrap(now);
    }
}

fn deadline_reached(deadline: Instant, now: Instant) -> bool {
    now >= deadline
}

impl NetworkBehaviour for Bootstrapper {
    type ProtocolsHandler = DummyProtocolsHandler;
    type OutEvent = BootstrapperEvent;

    fn new_handler(&mut self) -> Self::ProtocolsHandler {
        <_>::default()
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
        let maddr = remote_multiaddr(cp);

        let is_bootstrap = self.bootstrap_nodes.contains(maddr);
        #[rustfmt::skip]
        log::debug!(
            "{} connection established with {} {:?}. Bootstrap? {}",
            self.peer_id, peer_id, maddr, is_bootstrap
        );

        if is_bootstrap {
            self.schedule_bootstrap();
        }
    }

    fn inject_connection_closed(&mut self, _: &PeerId, _: &ConnectionId, cp: &ConnectedPoint) {
        let maddr = remote_multiaddr(cp);

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrapper::event::BootstrapperEvent::RunBootstrap;
    use fluence_libp2p::RandomPeerId;
    use libp2p::swarm::{AddressRecord, PollParameters};
    use std::net::IpAddr;
    use std::task::{Context, Poll};
    use std::thread::sleep;

    struct PollParams {
        peer_id: PeerId,
    }
    impl PollParameters for PollParams {
        type SupportedProtocolsIter = std::iter::Empty<Vec<u8>>;
        type ListenedAddressesIter = std::iter::Empty<Multiaddr>;
        type ExternalAddressesIter = std::iter::Empty<AddressRecord>;

        fn supported_protocols(&self) -> Self::SupportedProtocolsIter {
            std::iter::empty()
        }

        fn listened_addresses(&self) -> Self::ListenedAddressesIter {
            std::iter::empty()
        }

        fn external_addresses(&self) -> Self::ExternalAddressesIter {
            std::iter::empty()
        }

        fn local_peer_id(&self) -> &PeerId {
            &self.peer_id
        }
    }

    #[test]
    fn run_bootstrap() {
        let delay = Duration::from_millis(100);
        let cfg = BootstrapConfig {
            reconnect_delay: delay,
            bootstrap_delay: delay,
            bootstrap_max_delay: delay * 100,
        };
        let peer_id = RandomPeerId::random();
        let addr: IpAddr = "127.0.0.1".parse().unwrap();
        let addr = Multiaddr::from(addr);
        let bs = vec![addr.clone()];
        let mut behaviour = Bootstrapper::new(cfg, peer_id.clone(), bs);

        let start = Instant::now();
        behaviour.inject_connection_established(
            &RandomPeerId::random(),
            &ConnectionId::new(0),
            &ConnectedPoint::Dialer { address: addr },
        );

        let noop_waker = futures_util::task::noop_waker();
        let mut cx = Context::from_waker(&noop_waker);

        let mut parameters = PollParams { peer_id };

        let event = loop {
            match behaviour.poll(&mut cx, &mut parameters) {
                Poll::Ready(event) => break event,
                Poll::Pending => {
                    sleep(Duration::from_millis(1));
                    continue;
                }
            }
        };

        match event {
            NetworkBehaviourAction::GenerateEvent(event) => match event {
                RunBootstrap => {
                    // Check that bootstrap didn't hang (completed in no more than twice of expected time)
                    assert!(start.elapsed() < delay * 2);
                    // Check that bootstrap was delayed for long enough
                    assert!(start.elapsed() >= delay)
                }
                other => unreachable!("{:?}", other),
            },
            NetworkBehaviourAction::DialAddress { .. } => {}
            other => unreachable!("{:?}", other),
        };
    }
}
