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

use async_std::sync::{Arc, Mutex};
use async_std::task;
use connection_pool::{ConnectionPoolApi, ConnectionPoolT, Contact};
use fluence_libp2p::types::{Inlet, Outlet};
use futures::channel::mpsc::unbounded;
use futures::StreamExt;
use libp2p::PeerId;
use particle_protocol::Particle;
use std::collections::HashMap;
use std::ops::Deref;
use std::time::Duration;
use thiserror::Error;

struct Script {
    uuid: String,
    script: String,
}

pub struct ScriptStorage {
    inlet: Inlet<Script>,
    scripts: Arc<Mutex<HashMap<String, String>>>,
    connection_pool: ConnectionPoolApi,
    peer_id: PeerId,
    interval: Duration,
}

impl ScriptStorage {
    pub fn new(
        connection_pool: ConnectionPoolApi,
        peer_id: PeerId,
        interval: Duration,
    ) -> (ScriptStorageApi, Self) {
        let (outlet, inlet) = unbounded();
        let api = ScriptStorageApi { outlet };
        let this = ScriptStorage {
            inlet,
            scripts: <_>::default(),
            connection_pool,
            peer_id,
            interval,
        };
        (api, this)
    }

    pub fn start(self) {
        use futures::select;

        async fn execute_scripts(
            pool: &ConnectionPoolApi,
            scripts: HashMap<String, String>,
            peer_id: PeerId,
        ) {
            for (_, script) in scripts.into_iter() {
                let particle = Particle {
                    id: format!("auto_{}", uuid::Uuid::new_v4()),
                    init_peer_id: peer_id.clone(),
                    timestamp: chrono::Utc::now().timestamp() as u64,
                    ttl: 100000,
                    script,
                    signature: vec![],
                    data: vec![],
                };

                pool.send(Contact::new(peer_id, vec![]), particle).await;
            }
        }

        task::spawn(async move {
            let scripts = self.scripts;
            let pool = self.connection_pool;
            let peer_id = self.peer_id;

            let mut inlet = self.inlet.fuse();
            let mut timer = async_std::stream::interval(self.interval).fuse();

            loop {
                select! {
                    script = inlet.select_next_some() => {
                        scripts.lock().await.insert(script.uuid, script.script);
                    },
                    _ = timer.select_next_some() => {
                        let scripts = {
                            let lock = scripts.lock().await;
                            lock.deref().clone()
                        };

                        execute_scripts(&pool, scripts, peer_id.clone()).await;
                    }
                }
            }
        });
    }
}

#[derive(Clone)]
pub struct ScriptStorageApi {
    outlet: Outlet<Script>,
}

#[derive(Error, Debug)]
pub enum ScriptStorageError {
    #[error("can't send message to script storage")]
    OutletError,
}

impl ScriptStorageApi {
    pub fn add_script(&self, script: String) -> Result<String, ScriptStorageError> {
        let uuid = uuid::Uuid::new_v4().to_string();
        let script = Script {
            uuid: uuid.clone(),
            script,
        };
        self.outlet
            .unbounded_send(script)
            .map_err(|_| ScriptStorageError::OutletError)?;

        Ok(uuid)
    }
}
