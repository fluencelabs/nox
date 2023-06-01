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

use std::{
    collections::{HashMap, VecDeque},
    task::{Context, Poll},
};

use futures::task::Waker;
use tokio::task;

use fluence_libp2p::PeerId;
use key_manager::KeyManager;
/// For tests, mocked time is used
#[cfg(test)]
use mock_time::now_ms;
use particle_execution::{ParticleFunctionStatic, ParticleParams, ServiceFunction};
use particle_protocol::Particle;
use peer_metrics::ParticleExecutorMetrics;
/// Get current time from OS
#[cfg(not(test))]
use real_time::now_ms;

use crate::actor::{Actor, ActorPoll};
use crate::aqua_runtime::AquaRuntime;
use crate::deadline::Deadline;
use crate::error::AquamarineApiError;
use crate::particle_effects::RoutingEffects;
use crate::particle_functions::Functions;
use crate::vm_pool::VmPool;

type ParticleId = String;

pub struct Plumber<RT: AquaRuntime, F> {
    events: VecDeque<Result<RoutingEffects, AquamarineApiError>>,
    actors: HashMap<(ParticleId, PeerId), Actor<RT, F>>,
    vm_pool: VmPool<RT>,
    builtins: F,
    waker: Option<Waker>,
    metrics: Option<ParticleExecutorMetrics>,
    key_manager: KeyManager,
}

unsafe impl<RT: AquaRuntime, F: ParticleFunctionStatic> Send for Plumber<RT, F> {}

unsafe impl<RT: AquaRuntime, F: ParticleFunctionStatic> Sync for Plumber<RT, F> {}

impl<RT: AquaRuntime, F: ParticleFunctionStatic> Plumber<RT, F> {
    pub fn new(
        vm_pool: VmPool<RT>,
        builtins: F,
        metrics: Option<ParticleExecutorMetrics>,
        key_manager: KeyManager,
    ) -> Self {
        Self {
            vm_pool,
            builtins,
            events: <_>::default(),
            actors: <_>::default(),
            waker: <_>::default(),
            metrics,
            key_manager,
        }
    }

    /// Receives and ingests incoming particle: creates a new actor or forwards to the existing mailbox
    pub fn ingest(
        &mut self,
        particle: Particle,
        function: Option<ServiceFunction>,
        worker_id: PeerId,
    ) {
        self.wake();

        let deadline = Deadline::from(&particle);
        if deadline.is_expired(now_ms()) {
            tracing::info!(particle_id = particle.id, "Particle is expired, ignoring");
            self.events
                .push_back(Err(AquamarineApiError::ParticleExpired {
                    particle_id: particle.id,
                }));
            return;
        }

        let builtins = &self.builtins;
        let actor = self
            .actors
            .entry((particle.id.clone(), worker_id))
            .or_insert_with(|| {
                let params = ParticleParams::clone_from(&particle, worker_id);
                let functions = Functions::new(params, builtins.clone());
                Actor::new(&particle, functions, worker_id)
            });

        actor.ingest(particle);
        if let Some(function) = function {
            actor.set_function(function);
        }
    }

    pub fn add_service(
        &self,
        service: String,
        functions: HashMap<String, ServiceFunction>,
        fallback: Option<ServiceFunction>,
    ) {
        let builtins = self.builtins.clone();
        let task = async move {
            builtins.extend(service, functions, fallback).await;
        };
        task::Builder::new()
            .name("Add service")
            .spawn(task)
            .expect("Could not spawn add service task");
    }

    pub fn remove_service(&self, service: String) {
        let builtins = self.builtins.clone();
        let task = async move {
            builtins.remove(&service).await;
        };
        task::Builder::new()
            .name("Remove service")
            .spawn(task)
            .expect("Could not spawn remove service task");
    }

    pub fn poll(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<RoutingEffects, AquamarineApiError>> {
        self.waker = Some(cx.waker().clone());

        self.vm_pool.poll(cx);

        if let Some(event) = self.events.pop_front() {
            return Poll::Ready(event);
        }

        // Gather effects and put VMs back
        let mut remote_effects = vec![];
        let mut local_effects: Vec<RoutingEffects> = vec![];
        let mut interpretation_stats = vec![];
        let mut mailbox_size = 0;
        let key_manager = self.key_manager.clone();
        for actor in self.actors.values_mut() {
            if let Poll::Ready(result) = actor.poll_completed(cx) {
                interpretation_stats.push(result.stats);
                let (local_peers, remote_peers): (Vec<_>, Vec<_>) = result
                    .effects
                    .next_peers
                    .into_iter()
                    .partition(|p| key_manager.is_local(*p));

                if !remote_peers.is_empty() {
                    remote_effects.push(RoutingEffects {
                        particle: result.effects.particle.clone(),
                        next_peers: remote_peers,
                    })
                }

                if !local_peers.is_empty() {
                    local_effects.push(RoutingEffects {
                        particle: result.effects.particle,
                        next_peers: local_peers,
                    });
                }
                if let Some((vm_id, vm)) = result.vm {
                    self.vm_pool.put_vm(vm_id, vm)
                }
            }
            mailbox_size += actor.mailbox_size();
        }

        // Remove expired actors
        if let Some((vm_id, mut vm)) = self.vm_pool.get_vm() {
            let now = now_ms();

            self.actors.retain(|(particle_id, peer_id), actor| {
                // if actor hasn't yet expired or is still executing, keep it
                // TODO: if actor is expired, cancel execution and return VM back to pool
                //       https://github.com/fluencelabs/fluence/issues/1212
                if !actor.is_expired(now) || actor.is_executing() {
                    return true; // keep actor
                }
                tracing::debug!(
                    target: "particle_reap",
                    particle_id = particle_id,
                    "Reaping particle's actor peer_id={peer_id})"
                );
                // cleanup files and dirs after particle processing (vault & prev_data)
                // TODO: do not pass vm https://github.com/fluencelabs/fluence/issues/1216
                if let Err(err) = actor.cleanup(particle_id, &peer_id.to_string(), &mut vm) {
                    tracing::warn!(
                        particle_id = particle_id,
                        "Error cleaning up after particle {:?}",
                        err
                    )
                }
                false // remove actor
            });

            self.vm_pool.put_vm(vm_id, vm);
        }

        // Execute next messages
        let mut stats = vec![];
        for actor in self.actors.values_mut() {
            if let Some((vm_id, vm)) = self.vm_pool.get_vm() {
                match actor.poll_next(vm_id, vm, cx) {
                    ActorPoll::Vm(vm_id, vm) => self.vm_pool.put_vm(vm_id, vm),
                    ActorPoll::Executing(mut s) => stats.append(&mut s),
                }
            } else {
                // TODO: calculate deviations from normal mailbox_size
                if mailbox_size > 11 {
                    log::warn!(
                        "{} particles waiting in mailboxes, but all interpreters busy",
                        mailbox_size
                    );
                }
                break;
            }
        }
        self.meter(|m| {
            for stat in &interpretation_stats {
                // count particle interpretations
                if stat.success {
                    m.interpretation_successes.inc();
                } else {
                    m.interpretation_failures.inc();
                }

                let time = stat.interpretation_time.as_secs_f64();
                m.interpretation_time_sec.observe(time);
                m.total_actors_mailbox.set(mailbox_size as i64);
                m.alive_actors.set(self.actors.len() as i64);
            }

            for stat in &stats {
                m.service_call(stat.success, stat.kind, stat.run_time)
            }
        });

        for effect in local_effects {
            for local_peer in effect.next_peers {
                self.ingest(effect.particle.clone(), None, local_peer);
            }
        }

        // Turn effects into events, and buffer them
        self.events.extend(remote_effects.into_iter().map(Ok));

        // Return a new event if there is some
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

    fn meter<U, FF: Fn(&ParticleExecutorMetrics) -> U>(&self, f: FF) {
        self.metrics.as_ref().map(f);
    }
}

/// Implements `now` by taking number of non-leap seconds from `Utc::now()`
mod real_time {
    #[allow(dead_code)]
    pub fn now_ms() -> u64 {
        (chrono::Utc::now().timestamp() * 1000) as u64
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::convert::Infallible;
    use std::task::Waker;
    use std::{sync::Arc, task::Context};

    use avm_server::{AVMMemoryStats, AVMOutcome, CallResults, ParticleParameters};
    use fluence_keypair::KeyPair;
    use fluence_libp2p::RandomPeerId;
    use futures::future::BoxFuture;
    use futures::task::noop_waker_ref;
    use futures::FutureExt;
    use key_manager::KeyManager;

    use particle_args::Args;
    use particle_execution::{FunctionOutcome, ParticleFunction, ParticleParams, ServiceFunction};
    use particle_protocol::Particle;

    use crate::deadline::Deadline;
    use crate::plumber::mock_time::set_mock_time;
    use crate::plumber::{now_ms, real_time};
    use crate::vm_pool::VmPool;
    use crate::AquamarineApiError::ParticleExpired;
    use crate::{AquaRuntime, ParticleEffects, Plumber};
    use async_trait::async_trait;

    struct MockF;

    #[async_trait]
    impl ParticleFunction for MockF {
        async fn call(&self, _args: Args, _particle: ParticleParams) -> FunctionOutcome {
            panic!("no builtins in plumber tests!")
        }

        async fn extend(
            &self,
            _service: String,
            _functions: HashMap<String, ServiceFunction>,
            _fallback: Option<ServiceFunction>,
        ) {
            todo!()
        }

        async fn remove(&self, _service: &str) {
            todo!()
        }
    }

    struct VMMock;

    impl AquaRuntime for VMMock {
        type Config = ();
        type Error = Infallible;

        fn create_runtime(
            _config: Self::Config,
            _waker: Waker,
        ) -> BoxFuture<'static, Result<Self, Self::Error>> {
            async { Ok(VMMock) }.boxed()
        }

        fn into_effects(
            _outcome: Result<AVMOutcome, Self::Error>,
            _p: Particle,
        ) -> ParticleEffects {
            ParticleEffects {
                particle: Default::default(),
                next_peers: vec![],
                call_requests: Default::default(),
            }
        }

        fn call(
            &mut self,
            _aqua: String,
            _data: Vec<u8>,
            _particle: ParticleParameters<'_>,
            _call_results: CallResults,
        ) -> Result<AVMOutcome, Self::Error> {
            Ok(AVMOutcome {
                data: vec![],
                call_requests: Default::default(),
                next_peer_pks: vec![],
                memory_delta: 0,
                execution_time: Default::default(),
            })
        }

        fn cleanup(
            &mut self,
            _particle_id: &str,
            _current_peer_id: &str,
        ) -> Result<(), Self::Error> {
            Ok(())
        }

        fn memory_stats(&self) -> AVMMemoryStats {
            AVMMemoryStats {
                memory_size: 0,
                max_memory_size: None,
            }
        }
    }

    fn plumber() -> Plumber<VMMock, Arc<MockF>> {
        // Pool is of size 1 so it's easier to control tests
        let vm_pool = VmPool::new(1, (), None);
        let builtin_mock = Arc::new(MockF);
        let key_manager = KeyManager::new(
            "keypair".into(),
            KeyPair::generate_ed25519(),
            RandomPeerId::random(),
            RandomPeerId::random(),
        );
        Plumber::new(vm_pool, builtin_mock, None, key_manager)
    }

    fn particle(ts: u64, ttl: u32) -> Particle {
        let mut particle = Particle::default();
        particle.timestamp = ts;
        particle.ttl = ttl;

        particle
    }

    fn context() -> Context<'static> {
        Context::from_waker(noop_waker_ref())
    }

    /// Checks that expired actor will be removed
    #[ignore]
    #[test]
    fn remove_expired() {
        set_mock_time(real_time::now_ms());

        let mut plumber = plumber();

        let particle = particle(now_ms(), 1);
        let deadline = Deadline::from(&particle);
        assert!(!deadline.is_expired(now_ms()));

        plumber.ingest(particle, None, RandomPeerId::random());

        assert_eq!(plumber.actors.len(), 1);
        let mut cx = context();
        assert!(plumber.poll(&mut cx).is_pending());
        assert_eq!(plumber.actors.len(), 1);

        assert_eq!(plumber.vm_pool.free_vms(), 0);
        // pool is single VM, wait until VM is free
        loop {
            if plumber.vm_pool.free_vms() == 1 {
                break;
            };
            // 'is_pending' is used to suppress "must use" warning
            plumber.poll(&mut cx).is_pending();
        }

        set_mock_time(now_ms() + 2);
        assert!(plumber.poll(&mut cx).is_pending());
        assert_eq!(plumber.actors.len(), 0);
    }

    /// Checks that expired particle won't create an actor
    #[test]
    fn ignore_expired() {
        set_mock_time(real_time::now_ms());
        // set_mock_time(1000);

        let mut plumber = plumber();
        let particle = particle(now_ms() - 100, 99);
        let deadline = Deadline::from(&particle);
        assert!(deadline.is_expired(now_ms()));

        plumber.ingest(particle.clone(), None, RandomPeerId::random());

        assert_eq!(plumber.actors.len(), 0);

        // Check actor doesn't appear after poll somehow
        set_mock_time(now_ms() + 1000);
        let poll = plumber.poll(&mut context());
        assert!(poll.is_ready());
        match poll {
            std::task::Poll::Ready(Err(ParticleExpired { particle_id })) => {
                assert_eq!(particle_id, particle.id)
            }
            unexpected => panic!(
                "Expected Poll::Ready(Err(AquamarineApiError::ParticleExpired)), got {:?}",
                unexpected
            ),
        }
        assert_eq!(plumber.actors.len(), 0);
    }
}

/// Code taken from https://blog.iany.me/2019/03/how-to-mock-time-in-rust-tests-and-cargo-gotchas-we-met/
/// And then modified to use u64 instead of `SystemTime`
#[cfg(test)]
pub mod mock_time {
    #![allow(dead_code)]

    use std::cell::RefCell;

    thread_local! {
        static MOCK_TIME: RefCell<u64> = RefCell::new(0);
    }

    pub fn now_ms() -> u64 {
        MOCK_TIME.with(|cell| *cell.borrow())
    }

    pub fn set_mock_time(time: u64) {
        MOCK_TIME.with(|cell| *cell.borrow_mut() = time);
    }
}
