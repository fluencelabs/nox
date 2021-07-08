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

use crate::ScriptStorageConfig;

use async_unlock::unlock;
use connection_pool::{ConnectionPoolApi, ConnectionPoolT};
use fluence_libp2p::types::{Inlet, OneshotOutlet, Outlet};
use fluence_libp2p::PeerId;
use particle_protocol::{Contact, Particle};

use async_std::{sync::Mutex, task, task::JoinHandle};
use futures::{
    channel::{mpsc::unbounded, oneshot},
    future::BoxFuture,
    FutureExt, StreamExt, TryFutureExt,
};
use now_millis::now_ms;
use std::{
    borrow::Borrow,
    collections::{hash_map::Entry, HashMap},
    convert::identity,
    sync::Arc,
    time::{Duration, Instant},
};
use thiserror::Error;

#[derive(Clone, Hash, Debug, PartialEq, Eq)]
pub struct ScriptId(Arc<String>);
impl Borrow<String> for ScriptId {
    fn borrow(&self) -> &String {
        self.0.borrow()
    }
}

#[derive(Clone, Debug)]
pub struct Script {
    pub src: String,
    pub failures: u8,
    /// Interval at which to execute this script.
    /// If None, that means the script will be executed only once
    pub interval: Option<Duration>,
    pub delay: Duration,
    pub executed_at: Option<Instant>,
    pub next_execution: Instant,
    pub owner: PeerId,
}

impl Script {
    pub fn new(src: String, interval: Option<Duration>, delay: Duration, owner: PeerId) -> Self {
        Self {
            src,
            interval,
            delay,
            failures: 0,
            executed_at: None,
            next_execution: Instant::now() + delay,
            owner,
        }
    }

    pub fn deadline(&self) -> Instant {
        self.next_execution
    }
}

type ParticleId = String;

struct SentParticle {
    pub script_id: ScriptId,
    pub deadline: Instant,
}

#[derive(Debug)]
pub enum Command {
    AddScript {
        uuid: String,
        script: String,
        interval: Option<Duration>,
        delay: Duration,
        owner: PeerId,
    },
    RemoveScript {
        uuid: String,
        outlet: OneshotOutlet<Result<bool, ScriptStorageError>>,
        actor: PeerId,
        force: bool,
    },
    ListScripts {
        outlet: OneshotOutlet<HashMap<ScriptId, Script>>,
    },
}

pub struct ScriptStorageBackend {
    inlet: Inlet<Command>,
    scripts: Mutex<HashMap<ScriptId, Script>>,
    sent_particles: Mutex<HashMap<ParticleId, SentParticle>>,
    failed_particles: Inlet<ParticleId>,
    connection_pool: ConnectionPoolApi,
    config: ScriptStorageConfig,
}

impl ScriptStorageBackend {
    pub fn new(
        connection_pool: ConnectionPoolApi,
        failed_particles: Inlet<ParticleId>,
        config: ScriptStorageConfig,
    ) -> (ScriptStorageApi, Self) {
        let (outlet, inlet) = unbounded();
        let api = ScriptStorageApi { outlet };
        let this = ScriptStorageBackend {
            inlet,
            scripts: <_>::default(),
            sent_particles: <_>::default(),
            failed_particles,
            connection_pool,
            config,
        };
        (api, this)
    }

    pub fn start(self) -> JoinHandle<()> {
        use futures::select;

        task::spawn(async move {
            let scripts = self.scripts;
            let sent_particles = self.sent_particles;
            let pool = self.connection_pool;
            let config = self.config;
            let max_failures = self.config.max_failures;

            let mut failed_particles = self.failed_particles.fuse();
            let mut inlet = self.inlet.fuse();
            let mut timer = async_std::stream::interval(self.config.timer_resolution).fuse();

            loop {
                select! {
                    command = inlet.select_next_some() => {
                        execute_command(command, &scripts).await;
                    },
                    failed = failed_particles.select_next_some() => {
                        remove_failed_scripts(failed, &sent_particles, &scripts, max_failures).await;
                    },
                    _ = timer.select_next_some() => {
                        execute_scripts(&pool, &scripts, &sent_particles, config).await;
                        cleanup(&sent_particles).await;
                    }
                }
            }
        })
    }
}

async fn execute_scripts(
    pool: &ConnectionPoolApi,
    scripts: &Mutex<HashMap<ScriptId, Script>>,
    sent_particles: &Mutex<HashMap<ParticleId, SentParticle>>,
    config: ScriptStorageConfig,
) {
    let now = Instant::now();
    let now_u64 = now_ms() as u64;

    // Remove all ready scripts without interval, they will be executing only once
    let ready_single_shots: Vec<_> = unlock(scripts, |scripts| {
        scripts
            .drain_filter(|_, s| s.interval.is_none() && s.deadline() <= now)
            .collect()
    })
    .await;

    // Take and clone all scripts that are ready to be executed
    let scripts: HashMap<ScriptId, Script> = unlock(scripts, |scripts| {
        scripts
            .iter_mut()
            .filter(|(_, script)| script.deadline() <= now)
            .map(|(id, s)| {
                // mark script as executed at the current timestamp and schedule next
                s.executed_at = Some(now);
                // SAFETY: safe to call unwrap because all scripts without interval already removed
                s.next_execution = now + s.interval.unwrap();
                (id.clone(), s.clone())
            })
            .collect()
    })
    .await;
    // concatenate single shots with other scripts
    let scripts = ready_single_shots.into_iter().chain(scripts);

    for (script_id, script) in scripts {
        let particle_id = format!("auto_{}", uuid::Uuid::new_v4());
        let execution_id = format!("auto_{}", uuid::Uuid::new_v4());

        // Save info about sent particle to account for failures
        let info = SentParticle {
            script_id,
            deadline: now + config.particle_ttl,
        };
        unlock(sent_particles, |sent| {
            sent.insert(particle_id.clone(), info)
        })
        .await;

        // Send particle to the current node
        let particle = Particle {
            id: particle_id,
            init_peer_id: config.peer_id,
            timestamp: now_u64,
            ttl: config.particle_ttl.as_millis() as u32,
            script: script.src,
            signature: vec![],
            data: vec![],
            execution_id,
        };
        let contact = Contact::new(config.peer_id, vec![]);
        pool.send(contact, particle).await;
    }
}

async fn execute_command(command: Command, scripts: &Mutex<HashMap<ScriptId, Script>>) {
    match command {
        Command::AddScript {
            uuid,
            script,
            interval,
            delay,
            owner,
        } => {
            let uuid = ScriptId(Arc::new(uuid));
            let script = Script::new(script, interval, delay, owner);
            unlock(scripts, |scripts| scripts.insert(uuid, script)).await;
        }
        Command::RemoveScript {
            uuid,
            outlet,
            actor,
            force,
        } => {
            let uuid = ScriptId(Arc::new(uuid));
            let removed = unlock(scripts, |scripts| match scripts.entry(uuid) {
                Entry::Vacant(_) => Ok(false),
                Entry::Occupied(e) if force || e.get().owner == actor => {
                    e.remove();
                    Ok(true)
                }
                Entry::Occupied(_) => Err(ScriptStorageError::PermissionDenied),
            })
            .await;
            outlet.send(removed).ok();
        }
        Command::ListScripts { outlet } => {
            let scripts = unlock(scripts, |scripts| scripts.clone()).await;
            outlet.send(scripts).ok();
        }
    }
}

async fn remove_failed_scripts(
    particle_id: String,
    sent_particles: &Mutex<HashMap<ParticleId, SentParticle>>,
    scripts: &Mutex<HashMap<ScriptId, Script>>,
    max_failures: u8,
) {
    let sent = unlock(sent_particles, |sent| sent.remove(&particle_id)).await;
    if let Some(SentParticle { script_id, .. }) = sent {
        unlock(scripts, |scripts| {
            if let Entry::Occupied(entry) = scripts.entry(script_id) {
                let failures = entry.get().failures;
                if failures + 1 < max_failures {
                    entry.into_mut().failures += 1;
                } else {
                    entry.remove();
                }
            }
        })
        .await;
    }
}

async fn cleanup(sent_particles: &Mutex<HashMap<ParticleId, SentParticle>>) {
    let now = Instant::now();
    unlock(sent_particles, |sent| {
        sent.retain(|_, SentParticle { deadline, .. }| *deadline < now)
    })
    .await
}

#[derive(Clone)]
pub struct ScriptStorageApi {
    pub outlet: Outlet<Command>,
}

#[derive(Error, Debug)]
pub enum ScriptStorageError {
    #[error("ScriptStorageError::OutletError: can't send message to script storage")]
    OutletError,
    #[error("ScriptStorageError::InletError: can't receive response from script storage")]
    InletError,
    #[error(
        "ScriptStorageError::PermissionDenied: only the owner (creator) of a script can remove it"
    )]
    PermissionDenied,
}

impl ScriptStorageApi {
    fn send(&self, command: Command) -> Result<(), ScriptStorageError> {
        self.outlet
            .unbounded_send(command)
            .map_err(|_| ScriptStorageError::OutletError)
    }

    pub fn add_script(
        &self,
        script: String,
        interval: Option<Duration>,
        delay: Duration,
        owner: PeerId,
    ) -> Result<String, ScriptStorageError> {
        let uuid = uuid::Uuid::new_v4().to_string();

        self.send(Command::AddScript {
            uuid: uuid.clone(),
            script,
            interval,
            delay,
            owner,
        })?;

        Ok(uuid)
    }

    pub fn remove_script(
        &self,
        uuid: String,
        actor: PeerId,
        force: bool,
    ) -> BoxFuture<'static, Result<bool, ScriptStorageError>> {
        use ScriptStorageError::InletError;

        let (outlet, inlet) = oneshot::channel();
        let command = Command::RemoveScript {
            uuid,
            outlet,
            actor,
            force,
        };
        if let Err(err) = self.send(command) {
            return futures::future::err(err).boxed();
        }
        inlet
            .map(|r| r.map_err(|_| InletError).and_then(identity))
            .boxed()
    }

    pub fn list_scripts(
        &self,
    ) -> BoxFuture<'static, Result<HashMap<ScriptId, Script>, ScriptStorageError>> {
        let (outlet, inlet) = oneshot::channel();
        if let Err(err) = self.send(Command::ListScripts { outlet }) {
            return futures::future::err(err).boxed();
        }
        inlet.map_err(|_| ScriptStorageError::InletError).boxed()
    }
}
