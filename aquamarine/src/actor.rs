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

use avm_server::CallResults;
use std::sync::Arc;
use std::task::Context;
use std::{
    collections::VecDeque,
    task::{Poll, Waker},
};

use fluence_keypair::KeyPair;
use futures::future::BoxFuture;
use futures::FutureExt;
use tracing::{Instrument, Span};

use fluence_libp2p::PeerId;
use particle_execution::{ParticleFunctionStatic, ServiceFunction};
use particle_protocol::Particle;

use crate::deadline::Deadline;
use crate::particle_effects::RoutingEffects;
use crate::particle_executor::{FutResult, ParticleExecutor};
use crate::particle_functions::{Functions, SingleCallStat};
use crate::{AquaRuntime, InterpretationStats, ParticleDataStore, ParticleEffects};

struct Reusables<RT> {
    vm_id: usize,
    vm: Option<RT>,
}

pub struct Actor<RT, F> {
    /// Particle of that actor is expired after that deadline
    deadline: Deadline,
    future: Option<
        BoxFuture<'static, eyre::Result<(Reusables<RT>, ParticleEffects, InterpretationStats)>>,
    >,
    mailbox: VecDeque<Particle>,
    waker: Option<Waker>,
    functions: Functions<F>,
    /// Particle that's memoized on the actor creation.
    /// Used to execute CallRequests when mailbox is empty.
    /// Particle's data is empty.
    particle: Particle,
    /// Particles and call results will be processed in the security scope of this peer id
    /// It's either `host_peer_id` or local worker peer id
    current_peer_id: PeerId,
    key_pair: KeyPair,
    span: Span,
    data_store: Arc<ParticleDataStore>,
}

impl<RT, F> Actor<RT, F>
where
    RT: AquaRuntime,
    F: ParticleFunctionStatic,
{
    pub fn new(
        particle: &Particle,
        functions: Functions<F>,
        current_peer_id: PeerId,
        key_pair: KeyPair,
        span: Span,
        data_store: Arc<ParticleDataStore>,
    ) -> Self {
        Self {
            deadline: Deadline::from(particle),
            functions,
            future: None,
            mailbox: <_>::default(),
            waker: None,
            // Clone particle without data
            particle: Particle {
                id: particle.id.clone(),
                init_peer_id: particle.init_peer_id,
                timestamp: particle.timestamp,
                ttl: particle.ttl,
                script: particle.script.clone(),
                signature: particle.signature.clone(),
                data: vec![],
            },
            current_peer_id,
            key_pair,
            span,
            data_store,
        }
    }

    pub fn is_expired(&self, now_ms: u64) -> bool {
        self.deadline.is_expired(now_ms)
    }

    pub fn is_executing(&self) -> bool {
        self.future.is_some()
    }

    pub fn cleanup_key(&self) -> (String, PeerId) {
        let particle_id = self.particle.id.clone();
        (particle_id, self.current_peer_id)
    }

    pub fn mailbox_size(&self) -> usize {
        self.mailbox.len()
    }

    pub fn set_function(&mut self, function: ServiceFunction) {
        self.functions.set_function(function)
    }

    pub fn ingest(&mut self, particle: Particle) {
        self.mailbox.push_back(particle);
        self.wake();
    }

    /// Polls actor for result on previously ingested particle
    pub fn poll_completed(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<FutResult<(usize, Option<RT>), RoutingEffects, InterpretationStats>> {
        use Poll::Ready;

        self.waker = Some(cx.waker().clone());

        self.functions.poll(cx);

        // Poll AquaVM future
        if let Some(Ready(res)) = self.future.as_mut().map(|f| f.poll_unpin(cx)) {
            match res {
                Ok((reusables, effects, stats)) => {
                    let _entered = self.span.enter();

                    self.future.take();

                    let waker = cx.waker().clone();
                    // Schedule execution of functions
                    self.functions
                        .execute(self.particle.id.clone(), effects.call_requests, waker);

                    let effects = RoutingEffects {
                        particle: Particle {
                            data: effects.new_data,
                            ..self.particle.clone()
                        },
                        next_peers: effects.next_peers,
                    };
                    return Ready(FutResult {
                        runtime: (reusables.vm_id, reusables.vm),
                        outcome: effects,
                        stats,
                    });
                }
                Err(err) => {
                    self.future.take();
                    tracing::error!(
                        particle_id = self.particle.id,
                        "Could not process particle: {}",
                        err
                    );
                    return Poll::Pending;
                }
            }
        }

        Poll::Pending
    }

    /// Provide actor with new `vm` to execute particles, if there are any.
    ///
    /// If actor is in the middle of executing previous particle, vm is returned
    /// If actor's mailbox is empty, vm is returned
    pub fn poll_next(&mut self, vm_id: usize, vm: RT, cx: &mut Context<'_>) -> ActorPoll<RT> {
        self.waker = Some(cx.waker().clone());

        self.functions.poll(cx);

        // Return vm if previous particle is still executing
        if self.is_executing() {
            return ActorPoll::Vm(vm_id, vm);
        }

        // Gather CallResults
        let (calls, stats) = self.functions.drain();

        // Take the next particle
        let particle = self.mailbox.pop_front();

        if particle.is_none() && calls.is_empty() {
            debug_assert!(stats.is_empty(), "stats must be empty if calls are empty");
            // Nothing to execute, return vm
            return ActorPoll::Vm(vm_id, vm);
        }

        let particle = particle.unwrap_or_else(|| {
            // If mailbox is empty, then take self.particle.
            // Its data is empty, so `vm` will process `calls` on the old (saved on disk) data
            self.particle.clone()
        });
        let waker = cx.waker().clone();
        let data_store = self.data_store.clone();
        let key_pair = self.key_pair.clone();
        let peer_id = self.current_peer_id.clone();
        self.future = Some(
            Actor::<RT, F>::process(
                vm_id, vm, waker, data_store, key_pair, peer_id, particle, calls,
            )
            .instrument(self.span.clone())
            .boxed(),
        );
        self.wake();

        ActorPoll::Executing(stats)
    }

    async fn process(
        vm_id: usize,
        vm: RT,
        waker: Waker,
        data_store: Arc<ParticleDataStore>,
        key_pair: KeyPair,
        peer_id: PeerId,
        particle: Particle,
        calls: CallResults,
    ) -> eyre::Result<(Reusables<RT>, ParticleEffects, InterpretationStats)> {
        let particle_id = particle.id.clone();

        let prev_data = data_store
            .read_data(particle.id.as_str(), peer_id.to_base58().as_str())
            .await?;

        let prev_data_len = prev_data.len();

        let res = vm.execute((particle, calls), prev_data, peer_id, key_pair);

        let interpretation_time = res.stats.interpretation_time;

        match &res.outcome {
            Ok(outcome) => {
                let len = outcome.data.len();
                tracing::trace!(
                    target: "execution", particle_id = particle_id,
                    "Particle interpreted in {} [{} bytes => {} bytes]",
                    humantime::format_duration(interpretation_time), prev_data_len, len
                );
                data_store
                    .store_data(
                        &outcome.data,
                        particle_id.as_str(),
                        peer_id.to_base58().as_str(),
                    )
                    .await?;
            }
            Err(err) => {
                tracing::warn!(
                    particle_id = particle_id,
                    "Error executing particle: {}",
                    err
                )
            }
        }

        let effects = RT::into_effects(res.outcome, particle_id);
        waker.wake();

        let reusables = Reusables {
            vm_id,
            vm: res.runtime,
        };

        Ok((reusables, effects, res.stats))
    }

    fn wake(&self) {
        if let Some(waker) = &self.waker {
            waker.wake_by_ref();
        }
    }
}

pub enum ActorPoll<RT> {
    Executing(Vec<SingleCallStat>),
    Vm(usize, RT),
}
