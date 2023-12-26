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
use std::collections::hash_map::Entry;
use std::sync::Arc;
use std::task::Poll::Ready;
use std::{
    collections::{HashMap, VecDeque},
    task::{Context, Poll},
};

use futures::task::Waker;
use tokio::task;
use tracing::instrument;

use fluence_libp2p::PeerId;
use key_manager::{ScopeHelper, WorkerRegistry};
/// For tests, mocked time is used
#[cfg(test)]
use mock_time::now_ms;
use particle_execution::{ParticleFunctionStatic, ParticleParams, ServiceFunction};
use particle_protocol::ExtendedParticle;
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
use crate::ParticleDataStore;

#[derive(PartialEq, Hash, Eq)]
struct ActorKey {
    signature: Vec<u8>,
    worker_id: PeerId,
}

const MAX_CLEANUP_KEYS_SIZE: usize = 1024;

pub struct Plumber<RT: AquaRuntime, F> {
    events: VecDeque<Result<RoutingEffects, AquamarineApiError>>,
    actors: HashMap<ActorKey, Actor<RT, F>>,
    vm_pool: VmPool<RT>,
    data_store: Arc<ParticleDataStore>,
    builtins: F,
    waker: Option<Waker>,
    metrics: Option<ParticleExecutorMetrics>,
    worker_registry: Arc<WorkerRegistry>,
    scope_helper: ScopeHelper,
    cleanup_future: Option<BoxFuture<'static, ()>>,
}

impl<RT: AquaRuntime, F: ParticleFunctionStatic> Plumber<RT, F> {
    pub fn new(
        vm_pool: VmPool<RT>,
        data_store: Arc<ParticleDataStore>,
        builtins: F,
        metrics: Option<ParticleExecutorMetrics>,
        worker_registry: Arc<WorkerRegistry>,
        scope_helper: ScopeHelper,
    ) -> Self {
        Self {
            vm_pool,
            data_store,
            builtins,
            events: <_>::default(),
            actors: <_>::default(),
            waker: <_>::default(),
            metrics,
            worker_registry,
            scope_helper,
            cleanup_future: None,
        }
    }

    /// Receives and ingests incoming particle: creates a new actor or forwards to the existing mailbox
    #[instrument(level = tracing::Level::INFO, skip_all)]
    pub fn ingest(
        &mut self,
        particle: ExtendedParticle,
        function: Option<ServiceFunction>,
        worker_id: PeerId,
    ) {
        self.wake();

        let deadline = Deadline::from(particle.as_ref());
        if deadline.is_expired(now_ms()) {
            tracing::info!(target: "expired", particle_id = particle.particle.id, "Particle is expired");
            self.events
                .push_back(Err(AquamarineApiError::ParticleExpired {
                    particle_id: particle.particle.id,
                }));
            return;
        }

        if let Err(err) = particle.particle.verify() {
            tracing::warn!(target: "signature", particle_id = particle.particle.id, "Particle signature verification failed: {err:?}");
            self.events
                .push_back(Err(AquamarineApiError::SignatureVerificationFailed {
                    particle_id: particle.particle.id,
                    err,
                }));
            return;
        }

        let is_active = self.worker_registry.is_worker_active(worker_id);
        let is_manager = self
            .scope_helper
            .is_management(particle.particle.init_peer_id);
        let is_host = self.scope_helper.is_host(particle.particle.init_peer_id);

        // Only a manager or the host itself is allowed to access deactivated workers
        if !is_active && !is_manager && !is_host {
            tracing::trace!(target: "worker_inactive", particle_id = particle.particle.id, worker_id = worker_id.to_string(), "Worker is not active");
            return;
        }

        let builtins = &self.builtins;
        let key = ActorKey {
            signature: particle.particle.signature.clone(),
            worker_id,
        };
        let entry = self.actors.entry(key);

        let actor = match entry {
            Entry::Occupied(actor) => Ok(actor.into_mut()),
            Entry::Vacant(entry) => {
                let params = ParticleParams::clone_from(particle.as_ref(), worker_id);
                let functions = Functions::new(params, builtins.clone());
                let key_pair = self.worker_registry.get_worker_keypair(worker_id);
                let deal_id = self.worker_registry.get_deal_id(worker_id).ok();
                let data_store = self.data_store.clone();
                key_pair.map(|kp| {
                    let actor = Actor::new(
                        particle.as_ref(),
                        functions,
                        worker_id,
                        kp,
                        data_store,
                        deal_id,
                    );
                    entry.insert(actor)
                })
            }
        };

        debug_assert!(actor.is_ok(), "no such worker: {:#?}", actor.err());

        match actor {
            Ok(actor) => {
                actor.ingest(particle);
                if let Some(function) = function {
                    actor.set_function(function);
                }
            }
            Err(err) => log::warn!(
                "No such worker {}, rejected particle {}: {:?}",
                worker_id,
                particle.particle.id,
                err
            ),
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
        for actor in self.actors.values_mut() {
            if let Poll::Ready(result) = actor.poll_completed(cx) {
                interpretation_stats.push(result.stats);
                let (local_peers, remote_peers): (Vec<_>, Vec<_>) = result
                    .effects
                    .next_peers
                    .into_iter()
                    .partition(|p| self.scope_helper.is_local(*p));

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

                let (vm_id, vm) = result.runtime;
                if let Some(vm) = vm {
                    self.vm_pool.put_vm(vm_id, vm);
                } else {
                    // if `result.vm` is None, then an AVM instance was lost due to
                    // panic or cancellation, and we must ask VmPool to recreate that AVM
                    // TODO: add a Count metric to count how often we call `recreate_avm`
                    self.vm_pool.recreate_avm(vm_id, cx);
                }
            }
            mailbox_size += actor.mailbox_size();
        }

        if let Some(Ready(())) = self.cleanup_future.as_mut().map(|f| f.poll_unpin(cx)) {
            // we remove clean up future if it is ready
            self.cleanup_future.take();
        }

        // do not schedule task if another in progress
        if self.cleanup_future.is_none() {
            // Remove expired actors
            let mut cleanup_keys: Vec<(String, PeerId)> = Vec::with_capacity(MAX_CLEANUP_KEYS_SIZE);
            let now = now_ms();
            self.actors.retain(|_, actor| {
                // TODO: this code isn't optimal we continue iterate over actors if cleanup keys is full
                // should be simpler to optimize it after fixing NET-632
                // also delete fn actor.cleanup_key()
                if cleanup_keys.len() >= MAX_CLEANUP_KEYS_SIZE {
                    return true;
                }
                // if actor hasn't yet expired or is still executing, keep it
                if !actor.is_expired(now) || actor.is_executing() {
                    return true; // keep actor
                }
                cleanup_keys.push(actor.cleanup_key());
                false // remove actor
            });

            if !cleanup_keys.is_empty() {
                let data_store = self.data_store.clone();
                self.cleanup_future =
                    Some(async move { data_store.batch_cleanup_data(cleanup_keys).await }.boxed())
            }
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

                let interpretation_time = stat.interpretation_time.as_secs_f64();
                m.interpretation_time_sec.observe(interpretation_time);
            }
            m.total_actors_mailbox.set(mailbox_size as i64);
            m.alive_actors.set(self.actors.len() as i64);

            for stat in &stats {
                m.service_call(stat.success, stat.kind, stat.call_time)
            }
        });

        for effect in local_effects {
            for local_peer in effect.next_peers {
                let span = tracing::info_span!(parent: effect.particle.span.as_ref(), "Plumber: routing effect ingest");
                let _guard = span.enter();
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
    use std::path::PathBuf;

    use avm_server::{AVMMemoryStats, CallResults, ParticleParameters};
    use fluence_keypair::KeyPair;
    use fluence_libp2p::RandomPeerId;
    use futures::task::noop_waker_ref;
    use key_manager::{KeyStorage, ScopeHelper, WorkerRegistry};

    use particle_args::Args;
    use particle_execution::{FunctionOutcome, ParticleFunction, ParticleParams, ServiceFunction};
    use particle_protocol::{ExtendedParticle, Particle};

    use crate::deadline::Deadline;
    use crate::plumber::mock_time::set_mock_time;
    use crate::plumber::{now_ms, real_time};
    use crate::vm_pool::VmPool;
    use crate::AquamarineApiError::ParticleExpired;
    use crate::{AquaRuntime, ParticleDataStore, ParticleEffects, Plumber};
    use async_trait::async_trait;
    use avm_server::avm_runner::RawAVMOutcome;
    use tracing::Span;

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

        fn create_runtime(_config: Self::Config, _waker: Waker) -> Result<Self, Self::Error> {
            Ok(VMMock)
        }

        fn into_effects(
            _outcome: Result<RawAVMOutcome, Self::Error>,
            _particle_id: String,
        ) -> ParticleEffects {
            ParticleEffects {
                new_data: vec![],
                next_peers: vec![],
                call_requests: Default::default(),
            }
        }

        fn call(
            &mut self,
            _air: impl Into<String>,
            _prev_data: impl Into<Vec<u8>>,
            _current_data: impl Into<Vec<u8>>,
            _particle_params: ParticleParameters<'_>,
            _call_results: CallResults,
            _key_pair: &KeyPair,
        ) -> Result<RawAVMOutcome, Self::Error> {
            Ok(RawAVMOutcome {
                ret_code: 0,
                error_message: "".to_string(),
                data: vec![],
                call_requests: Default::default(),
                next_peer_pks: vec![],
            })
        }

        fn memory_stats(&self) -> AVMMemoryStats {
            AVMMemoryStats {
                memory_size: 0,
                max_memory_size: None,
            }
        }
    }

    async fn plumber() -> Plumber<VMMock, Arc<MockF>> {
        // Pool is of size 1 so it's easier to control tests
        let vm_pool = VmPool::new(1, (), None, None);
        let builtin_mock = Arc::new(MockF);

        let root_key_pair: KeyPair = KeyPair::generate_ed25519().into();
        let key_pair_path: PathBuf = "keypair".into();
        let workers_path: PathBuf = "workers".into();
        let key_storage = KeyStorage::from_path(key_pair_path.as_path(), root_key_pair.clone())
            .await
            .expect("Could not load key storage");

        let key_storage = Arc::new(key_storage);

        let scope_helper = ScopeHelper::new(
            root_key_pair.get_peer_id(),
            RandomPeerId::random(),
            RandomPeerId::random(),
            key_storage.clone(),
        );

        let worker_registry =
            WorkerRegistry::from_path(workers_path.as_path(), key_storage, scope_helper.clone())
                .await
                .expect("Could not load worker registry");

        let worker_registry = Arc::new(worker_registry);

        let tmp_dir = tempfile::tempdir().expect("Could not create temp dir");
        let tmp_path = tmp_dir.path();
        let data_store = ParticleDataStore::new(
            tmp_path.join("particles"),
            tmp_path.join("vault"),
            tmp_path.join("anomaly"),
        );
        data_store
            .initialize()
            .await
            .expect("Could not initialize datastore");
        let data_store = Arc::new(data_store);

        Plumber::new(vm_pool, data_store, builtin_mock, None, worker_registry.clone(), scope_helper.clone())
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
    #[tokio::test]
    async fn remove_expired() {
        set_mock_time(real_time::now_ms());

        let mut plumber = plumber().await;

        let particle = particle(now_ms(), 1);
        let deadline = Deadline::from(&particle);
        assert!(!deadline.is_expired(now_ms()));

        plumber.ingest(
            ExtendedParticle::new(particle, Span::none()),
            None,
            RandomPeerId::random(),
        );

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
    #[tokio::test]
    async fn ignore_expired() {
        set_mock_time(real_time::now_ms());
        // set_mock_time(1000);

        let mut plumber = plumber().await;
        let particle = particle(now_ms() - 100, 99);
        let deadline = Deadline::from(&particle);
        assert!(deadline.is_expired(now_ms()));

        plumber.ingest(
            ExtendedParticle::new(particle.clone(), Span::none()),
            None,
            RandomPeerId::random(),
        );

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
