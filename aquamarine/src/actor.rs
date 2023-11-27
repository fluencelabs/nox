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

use std::sync::Arc;
use std::{
    collections::VecDeque,
    task::{Context, Poll, Waker},
};

use fluence_keypair::KeyPair;
use futures::future::BoxFuture;
use futures::FutureExt;
use tracing::{instrument, Instrument, Span};

use fluence_libp2p::PeerId;
use particle_execution::{ParticleFunctionStatic, ServiceFunction};
use particle_protocol::{ExtendedParticle, Particle};

use crate::deadline::Deadline;
use crate::particle_effects::RoutingEffects;
use crate::particle_executor::{FutResult, ParticleExecutor};
use crate::particle_functions::{Functions, SingleCallStat};
use crate::{AquaRuntime, InterpretationStats, ParticleEffects};

struct Reusables<RT> {
    vm_id: usize,
    vm: Option<RT>,
}

type ExecutionTask<RT> = Option<
    BoxFuture<
        'static,
        (
            Reusables<RT>,
            ParticleEffects,
            InterpretationStats,
            Arc<Span>,
        ),
    >,
>;
pub struct Actor<RT, F> {
    /// Particle of that actor is expired after that deadline
    deadline: Deadline,
    future: ExecutionTask<RT>,
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
    deal_id: Option<String>,
}

impl<RT, F> Actor<RT, F>
where
    RT: AquaRuntime,
    F: ParticleFunctionStatic,
{
    pub fn new(
        particle: &ExtendedParticle,
        functions: Functions<F>,
        current_peer_id: PeerId,
        key_pair: KeyPair,
        deal_id: Option<String>,
    ) -> Self {
        Self {
            deadline: Deadline::from(&particle.particle),
            functions,
            future: None,
            mailbox: <_>::default(),
            waker: None,
            // Clone particle without data
            particle: Particle {
                id: particle.particle.id.clone(),
                init_peer_id: particle.particle.init_peer_id,
                timestamp: particle.particle.timestamp,
                ttl: particle.particle.ttl,
                script: particle.particle.script.clone(),
                signature: particle.particle.signature.clone(),
                data: vec![],
            },
            current_peer_id,
            key_pair,
            deal_id,
        }
    }

    pub fn is_expired(&self, now_ms: u64) -> bool {
        self.deadline.is_expired(now_ms)
    }

    pub fn is_executing(&self) -> bool {
        self.future.is_some()
    }

    pub fn cleanup(&self, vm: &mut RT) {
        tracing::debug!(
            target: "particle_reap",
            particle_id = self.particle.id,
            worker_id = self.current_peer_id.to_string(),
            deal_id = self.deal_id,
            "Reaping particle's actor"
        );
        // TODO: remove dirs without using vm https://github.com/fluencelabs/fluence/issues/1216
        // TODO: ??? we don't have this issue anymore
        if let Err(err) = vm.cleanup(&self.particle.id, &self.current_peer_id.to_string()) {
            tracing::warn!(
                particle_id = self.particle.id,
                deal_id = self.deal_id,
                "Error cleaning up after particle {:?}",
                err
            );
        }
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
        use Poll::Ready;

        self.waker = Some(cx.waker().clone());

        self.functions.poll(cx);

        // Poll AquaVM future
        if let Some(Ready((reusables, effects, stats, span))) =
            self.future.as_mut().map(|f| f.poll_unpin(cx))
        {
            let local_span = tracing::info_span!(parent: span.as_ref(), "Poll AVM future");
            let _span_guard = local_span.enter();
            self.future.take();

            let waker = cx.waker().clone();
            // Schedule execution of functions
            self.functions.execute(
                self.particle.id.clone(),
                effects.call_requests,
                waker,
                span.clone(),
            );

            let effects = RoutingEffects {
                particle: ExtendedParticle {
                    particle: Particle {
                        data: effects.new_data,
                        ..self.particle.clone()
                    },
                    span,
                },
                next_peers: effects.next_peers,
            };
            return Ready(FutResult {
                runtime: (reusables.vm_id, reusables.vm),
                effects,
                stats,
            });
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
        let (calls, stats, spans) = self.functions.drain();

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
        // Take ownership of vm to process particle
        let peer_id = self.current_peer_id;
        // TODO: get rid of this clone by recovering key_pair after `vm.execute` (not trivial to implement)
        let key_pair = self.key_pair.clone();

        let async_span = if let Some(ext_particle) = ext_particle.as_ref() {
            tracing::info_span!(parent: ext_particle.span.as_ref(),"Actor: async AVM process particle & call results",particle_id = particle.id)
        } else {
            tracing::info_span!(
                "Actor: async AVM process call results",
                particle_id = particle.id
            )
        };
        for span in spans {
            async_span.follows_from(span.as_ref());
        }
        let linking_span = ext_particle
            .as_ref()
            .map(|p| p.span.clone())
            .unwrap_or_else(|| Arc::new(async_span.clone()));

        // TODO: add timeout for execution https://github.com/fluencelabs/fluence/issues/1212
        self.future = Some(
            async move {
                let res = vm
                    .execute((particle, calls), waker, peer_id, key_pair)
                    .in_current_span()
                    .await;

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
