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

use crate::actor::{Actor, ActorEvent};
use crate::config::ActorConfig;

use particle_protocol::Particle;

use fluence_app_service::AppServiceError;

use async_std::task;
use futures::{future::BoxFuture, Future};
use libp2p::PeerId;
use parity_multiaddr::Multiaddr;
use std::{
    collections::{hash_map::Entry, HashMap, VecDeque},
    fmt::{Debug, Formatter},
    mem,
    pin::Pin,
    task::{Context, Poll, Waker},
};

#[derive(Debug)]
pub enum PeerKind {
    Client,
    Unknown,
}

#[derive(Debug)]
pub enum PlumberEvent {
    Forward {
        target: PeerId,
        particle: Particle,
        kind: PeerKind,
    },
}

type Fut = BoxFuture<'static, Result<Actor, AppServiceError>>;

pub enum ActorState {
    Creating { future: Fut, mailbox: Vec<Particle> },
    Created(Actor),
}

impl Debug for ActorState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ActorState::Creating { mailbox, .. } => write!(
                f,
                "ActorState::Creating {{ future: OPAQUE, mailbox: {:?} }}",
                mailbox
            ),
            ActorState::Created(actor) => write!(
                f,
                "ActorState::Created {{ actor.particle: {:?} }}",
                actor.particle()
            ),
        }
    }
}

#[derive(Debug)]
pub struct Plumber {
    clients: HashMap<PeerId, Option<Multiaddr>>,
    events: VecDeque<PlumberEvent>,
    actors: HashMap<String, ActorState>,
    config: ActorConfig,
    pub(super) waker: Option<Waker>,
}

impl Plumber {
    pub fn new(config: ActorConfig) -> Self {
        Self {
            clients: <_>::default(),
            events: <_>::default(),
            actors: <_>::default(),
            config,
            waker: <_>::default(),
        }
    }

    pub fn add_client(&mut self, client: PeerId) {
        self.clients.insert(client, None);
    }

    pub fn add_client_address(&mut self, client: &PeerId, address: Multiaddr) {
        if let Some(addr) = self.clients.get_mut(client) {
            if let Some(addr) = addr.replace(address) {
                log::info!("Replaced old addr {} for client {}", addr, client)
            }
        }
    }

    pub fn client_address(&self, client: &PeerId) -> &Option<Multiaddr> {
        self.clients.get(client).unwrap_or(&None)
    }

    pub fn remove_client(&mut self, client: &PeerId) {
        self.clients.remove(client);
    }

    /// Receives and ingests incoming particle: creates a new actor or forwards to the existing mailbox
    pub fn ingest(&mut self, particle: Particle) {
        match self.actors.entry(particle.id.clone()) {
            Entry::Vacant(entry) => {
                // Create new actor
                let config = self.config.clone();
                let future = Self::create_actor(config, particle);
                entry.insert(ActorState::Creating {
                    future,
                    mailbox: vec![],
                });
            }
            Entry::Occupied(mut entry) => match entry.get_mut() {
                // Forward to the mailbox
                ActorState::Created(actor) => actor.ingest(particle),
                // Actor is still creating, buffer particle until later
                ActorState::Creating { mailbox, .. } => mailbox.push(particle),
            },
        }
    }

    pub fn poll(&mut self, cx: &mut Context<'_>) -> Poll<PlumberEvent> {
        self.waker = Some(cx.waker().clone());

        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        // Vector of newly created actors
        let mut created = vec![];
        // Remove finished creation operations from hashmap
        self.actors.retain(|id, s| {
            if let ActorState::Creating { future, mailbox } = s {
                if let Poll::Ready(r) = Pin::new(future).poll(cx) {
                    // Take ownership of the mailbox
                    let mailbox = mem::replace(mailbox, vec![]);
                    created.push((id.clone(), r, mailbox));
                    return false;
                }
            }
            true
        });
        // We might have some work processed by newly created actors, so wake up!
        if !created.is_empty() {
            self.wake()
        }
        // Insert successfully created actors to hashmap
        for (id, actor, mailbox) in created.into_iter() {
            match actor {
                Ok(mut actor) => {
                    // Ingest buffered particles
                    for particle in mailbox.into_iter() {
                        actor.ingest(particle)
                    }
                    self.actors.insert(id, ActorState::Created(actor));
                }
                Err(err) => unimplemented!("error creating actor: {:?}", err),
            }
        }

        // Poll existing actors for results
        let effects = self
            .actors
            .values_mut()
            .flat_map(|mut actor| {
                if let ActorState::Created(ref mut actor) = &mut actor {
                    if let Poll::Ready(effects) = actor.poll(cx) {
                        return effects;
                    }
                }

                vec![]
            })
            .collect::<Vec<_>>();

        // Turn effects into events, and buffer them
        for effect in effects {
            let ActorEvent::Forward { particle, target } = effect;
            let kind = self.peer_kind(&target);
            self.events.push_back(PlumberEvent::Forward {
                particle,
                target,
                kind,
            });
        }

        // Return new event if there is some
        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        Poll::Pending
    }

    fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }
    }

    fn create_actor(config: ActorConfig, particle: Particle) -> Fut {
        Box::pin(task::spawn_blocking(move || Actor::new(config, particle)))
    }

    /// Returns whether peer is a directly connected client or not
    fn peer_kind(&self, peer: &PeerId) -> PeerKind {
        if let Some(addr) = self.clients.get(peer) {
            if addr.is_none() {
                log::warn!("Address of the peer {} is unknown", peer);
            }
            PeerKind::Client
        } else {
            PeerKind::Unknown
        }
    }
}
