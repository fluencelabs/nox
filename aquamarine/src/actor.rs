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

use futures::future::BoxFuture;
use futures::FutureExt;
use std::sync::Arc;
use std::{
    collections::VecDeque,
    task::{Context, Poll, Waker},
};
use tracing::{instrument, Instrument, Span};

use crate::deadline::Deadline;
use crate::particle_effects::RoutingEffects;
use crate::particle_executor::{FutResult, ParticleExecutor};
use crate::particle_functions::{Functions, SingleCallStat};
use crate::spawner::Spawner;
use crate::{AquaRuntime, InterpretationStats, ParticleDataStore, ParticleEffects};
use fluence_keypair::KeyPair;
use fluence_libp2p::PeerId;
use particle_execution::{ParticleFunctionStatic, ServiceFunction};
use particle_protocol::{ExtendedParticle, Particle};

struct Reusables<RT> {
    vm_id: usize,
    vm: Option<RT>,
}

type AVMCallResult<RT> = FutResult<(usize, Option<RT>), RoutingEffects, InterpretationStats>;
type AVMTask<RT> = BoxFuture<
    'static,
    (
        Reusables<RT>,
        ParticleEffects,
        InterpretationStats,
        Arc<Span>,
    ),
>;
pub struct Actor<RT, F> {
    /// Particle of that actor is expired after that deadline
    deadline: Deadline,
    future: Option<AVMTask<RT>>,
    mailbox: VecDeque<ExtendedParticle>,
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
    data_store: Arc<ParticleDataStore>,
    spawner: Spawner,
    deal_id: Option<String>,
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
        data_store: Arc<ParticleDataStore>,
        deal_id: Option<String>,
        spawner: Spawner,
    ) -> Self {
        Self {
            deadline: Deadline::from(particle),
            functions,
            future: None,
            mailbox: <_>::default(),
            waker: None,
            // Clone particle without data
            particle: Particle {
                data: vec![],
                ..particle.clone()
            },
            current_peer_id,
            key_pair,
            data_store,
            spawner,
            deal_id,
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

    #[instrument(level = tracing::Level::INFO, skip_all)]
    pub fn ingest(&mut self, particle: ExtendedParticle) {
        self.mailbox.push_back(particle);
        self.wake();
    }

    /// Polls actor for result on previously ingested particle
    pub fn poll_completed(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<FutResult<(usize, Option<RT>), RoutingEffects, InterpretationStats>> {
        self.waker = Some(cx.waker().clone());

        self.functions.poll(cx);

        // Poll AquaVM future
        if let Some(value) = self.poll_avm_future(cx) {
            return value;
        }

        Poll::Pending
    }

    fn poll_avm_future(&mut self, cx: &mut Context<'_>) -> Option<Poll<AVMCallResult<RT>>> {
        if let Some(Poll::Ready(res)) = self.future.as_mut().map(|f| f.poll_unpin(cx)) {
            let (reusables, effects, stats, parent_span) = res;
            let span = tracing::info_span!(
                parent: parent_span.as_ref(),
                "Actor::poll_avm_future::future_ready",
                particle_id= self.particle.id,
                deal_id = self.deal_id
            );
            let _span_guard = span.enter();

            self.future.take();

            let spawner = self.spawner.clone();
            let waker = cx.waker().clone();
            // Schedule execution of functions
            {
                self.functions.execute(
                    spawner,
                    self.particle.id.clone(),
                    effects.call_requests,
                    waker,
                    parent_span.clone(),
                );
            }

            let effects = RoutingEffects {
                particle: ExtendedParticle::linked(
                    Particle {
                        data: effects.new_data,
                        ..self.particle.clone()
                    },
                    parent_span,
                ),
                next_peers: effects.next_peers,
            };
            return Some(Poll::Ready(FutResult {
                runtime: (reusables.vm_id, reusables.vm),
                effects,
                stats,
            }));
        }
        None
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
        let (calls, stats, call_spans) = self.functions.drain();

        // Take the next particle
        let ext_particle = self.mailbox.pop_front();

        if ext_particle.is_none() && calls.is_empty() {
            debug_assert!(stats.is_empty(), "stats must be empty if calls are empty");
            // Nothing to execute, return vm
            return ActorPoll::Vm(vm_id, vm);
        }

        let particle = ext_particle
            .as_ref()
            .map(|p| p.particle.clone())
            .unwrap_or_else(|| {
                // If mailbox is empty, then take self.particle.
                // Its data is empty, so `vm` will process `calls` on the old (saved on disk) data
                self.particle.clone()
            });

        let waker = cx.waker().clone();
        let data_store = self.data_store.clone();
        let key_pair = self.key_pair.clone();
        let peer_id = self.current_peer_id;

        let (async_span, linking_span) =
            self.create_spans(call_spans, ext_particle, particle.id.as_str());

        let spawner = self.spawner.clone();
        self.future = Some(
            async move {
                let res = vm
                    .execute(
                        spawner,
                        data_store,
                        (particle.clone(), calls),
                        peer_id,
                        key_pair,
                    )
                    .in_current_span()
                    .await;

                waker.wake();

                let reusables = Reusables {
                    vm_id,
                    vm: res.runtime,
                };

                (reusables, res.effects, res.stats, linking_span)
            }
            .instrument(async_span)
            .boxed(),
        );
        self.wake();

        ActorPoll::Executing(stats)
    }

    fn create_spans(
        &self,
        call_spans: Vec<Arc<Span>>,
        ext_particle: Option<ExtendedParticle>,
        particle_id: &str,
    ) -> (Span, Arc<Span>) {
        let async_span = tracing::info_span!(
            "Actor: async AVM process particle & call results",
            particle_id = particle_id,
            deal_id = self.deal_id
        );
        if let Some(ext_particle) = ext_particle.as_ref() {
            async_span.follows_from(ext_particle.span.as_ref());
        }
        for span in call_spans {
            async_span.follows_from(span.as_ref());
        }
        let linking_span = Arc::new(async_span.clone());
        (async_span, linking_span)
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
